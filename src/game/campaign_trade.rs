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
            let effect = self.economy.cancel_trade(id, player)?;
            if let Some(effect) = effect {
                self.politics[effect.province]
                    .change_relation(effect.player, -effect.relation_loss);
                self.reconcile_provinces();
                Ok(format!(
                    "Route #{id} cancelled. Relation with {} −{:.0}.",
                    self.economy.provinces[effect.province].name, effect.relation_loss
                ))
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
        self.pull_wallets();
        Ok(format!("Agreement #{id} accepted."))
    }
}
