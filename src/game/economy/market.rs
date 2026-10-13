//! Immediate national open-market exchanges with cumulative monthly price impact.

use super::EconomyWorld;

/// Which side of a resource/Coin exchange the player takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MarketSide {
    /// Pay Coin for physical resources.
    Buy,
    /// Exchange resources for Coin.
    Sell,
}

/// A live quote includes affordability and storage checks; quoting does not transfer anything.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct OpenMarketQuote {
    /// Requested resource units.
    pub quantity: f64,
    /// Coin paid/received for the complete transaction.
    pub coin: f64,
    /// Coin per physical resource unit at this transaction size.
    pub unit_price: f64,
    /// Price margin from the spread and transaction size.
    pub size_margin: f64,
    /// Same-side units of this resource already traded this month.
    pub prior_volume: f64,
}

/// One immediate order can include any combination of Food, Metal, and Stone.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct OpenMarketOrderQuote {
    /// Per-resource prices; zero quantities are omitted.
    pub resources: [Option<OpenMarketQuote>; 3],
    /// Combined cost or payment for the selected resources.
    pub coin: f64,
}

impl EconomyWorld {
    /// Integrate the marginal price over cumulative monthly volume so splitting
    /// a transaction cannot improve its total proceeds or cost.
    pub fn quote_open_market(
        &self,
        player: usize,
        resource: usize,
        side: MarketSide,
        quantity: f64,
    ) -> Result<OpenMarketQuote, String> {
        let quote = self.open_market_price(player, resource, side, quantity)?;
        let mut quantities = [0.0; 3];
        quantities[resource] = quantity;
        self.check_open_market_order(player, side, quantities, quote.coin)?;
        Ok(quote)
    }

    /// Price the whole order without affordability/storage checks so the UI always shows cost.
    pub fn price_open_market_order(
        &self,
        player: usize,
        side: MarketSide,
        quantities: [f64; 3],
    ) -> Result<OpenMarketOrderQuote, String> {
        self.players.get(player).ok_or("Unknown player")?;
        if quantities.iter().any(|quantity| !quantity.is_finite() || *quantity < 0.0) {
            return Err("Choose finite, nonnegative quantities".into());
        }
        let mut resources = [None; 3];
        let mut coin = 0.0;
        for (resource, quantity) in quantities.into_iter().enumerate() {
            if quantity > 0.0 {
                let quote = self.open_market_price(player, resource, side, quantity)?;
                coin += quote.coin;
                resources[resource] = Some(quote);
            }
        }
        if !coin.is_finite() {
            return Err("The exchange is too large".into());
        }
        Ok(OpenMarketOrderQuote {
            resources,
            coin,
        })
    }

    /// Validate every selected resource and their combined cost before any transfer.
    pub fn quote_open_market_order(
        &self,
        player: usize,
        side: MarketSide,
        quantities: [f64; 3],
    ) -> Result<OpenMarketOrderQuote, String> {
        let quote = self.price_open_market_order(player, side, quantities)?;
        if quote.coin <= 0.0 {
            return Err("Choose a positive quantity for at least one resource".into());
        }
        self.check_open_market_order(player, side, quantities, quote.coin)?;
        Ok(quote)
    }

    fn check_open_market_order(
        &self,
        player: usize,
        side: MarketSide,
        quantities: [f64; 3],
        coin: f64,
    ) -> Result<(), String> {
        let wallet = self.players.get(player).ok_or("Unknown player")?;
        match side {
            MarketSide::Buy => {
                if wallet.coin + 1e-9 < coin {
                    return Err("Not enough sestertii for this purchase".into());
                }
                if (0..3).any(|resource| {
                    quantities[resource] > 0.0
                        && wallet.resources[resource] + quantities[resource]
                            > wallet.storage[resource] + 1e-9
                }) {
                    return Err("Not enough storage for this purchase".into());
                }
            },
            MarketSide::Sell
                if (0..3)
                    .any(|resource| wallet.resources[resource] + 1e-9 < quantities[resource]) =>
            {
                return Err("Not enough stock to sell this quantity".into());
            },
            MarketSide::Sell => {},
        }
        Ok(())
    }

    fn open_market_price(
        &self,
        player: usize,
        resource: usize,
        side: MarketSide,
        quantity: f64,
    ) -> Result<OpenMarketQuote, String> {
        if resource >= 3 || !quantity.is_finite() || quantity <= 0.0 {
            return Err("Choose a resource and a finite positive quantity".into());
        }
        self.players.get(player).ok_or("Unknown player")?;
        let config = &self.config.trade;
        let side_index = match side {
            MarketSide::Buy => 0,
            MarketSide::Sell => 1,
        };
        let prior_volume = self.open_market_volume[player][side_index][resource];
        let depth = config.open_market_depth[resource].max(1.0);
        let spread = config.open_market_spread.max(0.0);
        let base = config.base_value[resource]
            * if side == MarketSide::Sell {
                config.open_market_sell_multiplier
            } else {
                1.0
            };
        let coin = match side {
            MarketSide::Buy => {
                base * (quantity * (1.0 + spread)
                    + (prior_volume * quantity + quantity * quantity * 0.5) / depth)
            },
            MarketSide::Sell => {
                let start = 1.0 + spread + prior_volume / depth;
                base * depth * (quantity / (depth * start)).ln_1p()
            },
        };
        let unit_price = coin / quantity;
        let size_margin = match side {
            MarketSide::Buy => unit_price / base - 1.0,
            MarketSide::Sell => base / unit_price - 1.0,
        };
        if !coin.is_finite() || coin <= 0.0 || !size_margin.is_finite() {
            return Err("The exchange is too large".into());
        }
        Ok(OpenMarketQuote {
            quantity,
            coin,
            unit_price,
            size_margin,
            prior_volume,
        })
    }

    /// Quote again before committing, so stale UI prices/balances cannot overspend.
    pub fn exchange_open_market(
        &mut self,
        player: usize,
        resource: usize,
        side: MarketSide,
        quantity: f64,
    ) -> Result<OpenMarketQuote, String> {
        let quote = self.quote_open_market(player, resource, side, quantity)?;
        let wallet = &mut self.players[player];
        let direction = if side == MarketSide::Buy {
            1.0
        } else {
            -1.0
        };
        wallet.resources[resource] = (wallet.resources[resource] + direction * quantity).max(0.0);
        wallet.coin = (wallet.coin - direction * quote.coin).max(0.0);
        let side_index = match side {
            MarketSide::Buy => 0,
            MarketSide::Sell => 1,
        };
        self.open_market_volume[player][side_index][resource] += quantity;
        Ok(quote)
    }

    /// Requote and commit the complete order atomically, including monthly price reservations.
    pub fn exchange_open_market_order(
        &mut self,
        player: usize,
        side: MarketSide,
        quantities: [f64; 3],
    ) -> Result<OpenMarketOrderQuote, String> {
        let quote = self.quote_open_market_order(player, side, quantities)?;
        let direction = if side == MarketSide::Buy {
            1.0
        } else {
            -1.0
        };
        let side_index = if side == MarketSide::Buy {
            0
        } else {
            1
        };
        let wallet = &mut self.players[player];
        for (resource, quantity) in quantities.into_iter().enumerate() {
            wallet.resources[resource] =
                (wallet.resources[resource] + direction * quantity).max(0.0);
            self.open_market_volume[player][side_index][resource] += quantity;
        }
        wallet.coin = (wallet.coin - direction * quote.coin).max(0.0);
        Ok(quote)
    }
}
