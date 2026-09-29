//! Immediate national open-market exchanges with cumulative monthly price impact.

use super::EconomyWorld;

/// Which side of a resource/Coin exchange the player takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarketSide {
    /// Pay Coin for physical resources.
    Buy,
    /// Exchange resources for Coin.
    Sell,
}

/// A live quote includes affordability and storage checks; quoting does not transfer anything.
#[derive(Clone, Copy, Debug)]
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
        if resource >= 3 || !quantity.is_finite() || quantity <= 0.0 {
            return Err("Choose a resource and a finite positive quantity".into());
        }
        let wallet = self.players.get(player).ok_or("Unknown player")?;
        let config = &self.config.trade;
        let side_index = match side {
            MarketSide::Buy => 0,
            MarketSide::Sell => 1,
        };
        let prior_volume = self.open_market_volume[player][side_index][resource];
        let depth = config.open_market_depth[resource].max(1.0);
        let spread = config.open_market_spread.max(0.0);
        let base = config.base_value[resource];
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
        match side {
            MarketSide::Buy => {
                if wallet.coin + 1e-9 < coin {
                    return Err("Not enough sestertii for this purchase".into());
                }
                if wallet.resources[resource] + quantity > wallet.storage[resource] + 1e-9 {
                    return Err("Not enough storage for this purchase".into());
                }
            },
            MarketSide::Sell if wallet.resources[resource] + 1e-9 < quantity => {
                return Err("Not enough stock to sell this quantity".into())
            },
            MarketSide::Sell => {},
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
}
