//! Immediate national open-market exchanges with transaction-size price impact.

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
}

impl EconomyWorld {
    /// Smaller exchanges receive better unit prices; there is no buy/sell arbitrage.
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
        let margin = config.open_market_spread.max(0.0)
            + quantity / config.open_market_depth[resource].max(1.0);
        let unit_price = match side {
            MarketSide::Buy => config.base_value[resource] * (1.0 + margin),
            MarketSide::Sell => config.base_value[resource] / (1.0 + margin),
        };
        let coin = quantity * unit_price;
        if !coin.is_finite() || coin <= 0.0 {
            return Err("The exchange is too large".into());
        }
        match side {
            MarketSide::Buy => {
                if wallet.coin + 1e-9 < coin {
                    return Err("Not enough Coin for this purchase".into());
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
            size_margin: margin,
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
        Ok(quote)
    }
}
