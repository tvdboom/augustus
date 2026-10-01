//! National trade commands keep political relations and shared wallets authoritative.

use super::campaign::Campaign;
use crate::game::economy::*;

impl Campaign {
    pub(crate) fn end_trade(
        &mut self,
        player: usize,
        id: u64,
        notice: bool,
    ) -> Result<String, String> {
        if notice {
            let due = self.economy.schedule_trade_cancellation(id, player)?;
            Ok(format!(
                "Notice given. Route #{id} ends after {} more months, with no relation penalty.",
                due.saturating_sub(self.economy.month)
            ))
        } else {
            let trade = self
                .economy
                .trades
                .iter()
                .find(|trade| trade.id == id)
                .ok_or("Unknown agreement")?
                .clone();
            let active = matches!(trade.status, TradeStatus::Active | TradeStatus::Suspended);
            let effect = self.economy.cancel_trade(id, player)?;
            let mut consequence = String::new();
            if let Some(effect) = effect {
                self.politics[effect.province]
                    .change_relation(effect.player, -effect.relation_loss);
                consequence = format!(
                    "Relation with {} −{:.0}. ",
                    self.economy.provinces[effect.province].name, effect.relation_loss
                );
            } else if active {
                let other = if trade.party_a == TradeParty::Player(player) {
                    trade.party_b
                } else {
                    trade.party_a
                };
                if let TradeParty::Player(partner) = other {
                    for (province, politics) in
                        self.economy.provinces.iter().zip(&mut self.politics)
                    {
                        if province.owner == Some(partner) {
                            politics.change_relation(
                                player,
                                -self.economy.config.trade.cancellation_relation_penalty,
                            );
                        }
                    }
                    consequence = format!(
                        "Relation with Player {} −{:.0}. ",
                        partner + 1,
                        self.economy.config.trade.cancellation_relation_penalty
                    );
                }
            }
            if active {
                let influence_loss = self.actors[player]
                    .influence
                    .min(self.economy.config.trade.cancellation_influence_penalty);
                self.actors[player].influence -= influence_loss;
                self.senate.record_trade_breach(player);
                consequence.push_str(&format!(
                    "Influence −{influence_loss:.0}; Merchant confidence drops immediately."
                ));
                self.push_wallets();
                self.reconcile_provinces();
                Ok(format!("Route #{id} stopped immediately. {consequence}"))
            } else {
                Ok(format!("Agreement #{id} cancelled."))
            }
        }
    }

    pub(crate) fn propose_national_trade(
        &mut self,
        agreement: TradeAgreement,
    ) -> Result<u64, String> {
        let inputs = self.inputs();
        let (id, effect) = self.economy.propose_trade(agreement, &inputs)?;
        if let Some(effect) = effect {
            self.politics[effect.province].apply_trade(
                effect.player,
                effect.relation_gain,
                effect.control_gain,
                &self.diplomacy_config,
            );
        }
        self.pull_wallets();
        self.reconcile_provinces();
        Ok(id)
    }

    pub(crate) fn accept_national_trade(
        &mut self,
        player: usize,
        id: u64,
    ) -> Result<String, String> {
        let inputs = self.inputs();
        self.economy.accept_trade(id, player, &inputs)?;
        if let Some(trade) = self.economy.trades.iter().find(|trade| trade.id == id) {
            if trade.frequency == TradeFrequency::OneTime {
                if let (TradeParty::Player(a), TradeParty::Player(b)) =
                    (trade.party_a, trade.party_b)
                {
                    let gain = (trade.last_delivered_value
                        / self.economy.config.trade.value_per_relation)
                        .min(self.economy.config.trade.relation_cap)
                        * self.economy.config.trade.one_time_relation_factor;
                    for (politics, province) in
                        self.politics.iter_mut().zip(&self.economy.provinces)
                    {
                        match province.owner {
                            Some(owner) if owner == a => politics.change_relation(b, gain),
                            Some(owner) if owner == b => politics.change_relation(a, gain),
                            _ => {},
                        }
                    }
                    self.reconcile_provinces();
                }
            }
        }
        self.pull_wallets();
        Ok(format!("Agreement #{id} accepted."))
    }
}
