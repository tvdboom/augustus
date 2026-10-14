//! National trade commands keep political relations and shared wallets authoritative.

use super::campaign::Campaign;
use super::campaign_notifications::{CampaignNotice, NoticeAction, NoticeKind, NoticeSeverity};
use crate::game::economy::*;

impl Campaign {
    /// Retain a brief outcome from a successful open-market purchase or sale.
    pub(crate) fn notify_market_trade(&mut self, player: usize, side: MarketSide) {
        self.notifications.push(CampaignNotice {
            id: 0,
            recipient: player,
            severity: NoticeSeverity::Info,
            title: "Market exchange completed".into(),
            body: match side {
                MarketSide::Buy => "Goods purchased.",
                MarketSide::Sell => "Goods sold.",
            }
            .into(),
            kind: NoticeKind::TradeUpdated,
            province: None,
            building: None,
            wonder: None,
            scandal: None,
            month: self.economy.month,
            action: NoticeAction::OpenMarket,
        });
    }

    /// Store the same trade update for each participant before its popup is displayed.
    fn notify_trade(
        &mut self,
        id: u64,
        month: u32,
        severity: NoticeSeverity,
        kind: NoticeKind,
        title: &str,
        body: &str,
    ) {
        let Some(trade) = self.economy.trades.iter().find(|trade| trade.id == id) else {
            return;
        };
        for (party, partner) in [(trade.party_a, trade.party_b), (trade.party_b, trade.party_a)] {
            let TradeParty::Player(player) = party else {
                continue;
            };
            let (province, partner_name) = match partner {
                TradeParty::Npc(province) => {
                    (Some(province), self.economy.provinces[province].name.clone())
                },
                TradeParty::Player(partner) => (None, format!("Player {}", partner + 1)),
            };
            self.notifications.push(CampaignNotice {
                id: 0,
                recipient: player,
                severity,
                title: title.into(),
                body: format!("{partner_name}: {body}"),
                kind,
                province,
                building: None,
                wonder: None,
                scandal: None,
                month,
                action: NoticeAction::OpenTrade(id),
            });
        }
    }

    /// Retain brief trade outcomes with the agreement as their navigation destination.
    pub(super) fn record_trade_event(&mut self, event: &EconomyEvent, month: u32) {
        let (id, severity, kind, title, body) = match event {
            EconomyEvent::TradeSuspended {
                agreement,
                ..
            } => (
                *agreement,
                NoticeSeverity::Warning,
                NoticeKind::TradeInterrupted,
                "Trade interrupted",
                "Trade route suspended.",
            ),
            EconomyEvent::TradeCancelled {
                agreement,
            } => (
                *agreement,
                NoticeSeverity::Warning,
                NoticeKind::TradeInterrupted,
                "Trade cancelled",
                "Trade route cancelled.",
            ),
            EconomyEvent::TradeNoticeCompleted {
                agreement,
            } => (
                *agreement,
                NoticeSeverity::Info,
                NoticeKind::TradeUpdated,
                "Trade route ended",
                "Trade route closed.",
            ),
            _ => return,
        };
        self.notify_trade(id, month, severity, kind, title, body);
    }

    pub(crate) fn end_trade(
        &mut self,
        player: usize,
        id: u64,
        notice: bool,
    ) -> Result<String, String> {
        if notice {
            let due = self.economy.schedule_trade_cancellation(id, player)?;
            let message =
                format!("Trade route closes in {} months.", due.saturating_sub(self.economy.month));
            self.notify_trade(
                id,
                self.economy.month,
                NoticeSeverity::Info,
                NoticeKind::TradeUpdated,
                "Trade closure scheduled",
                &message,
            );
            Ok(message)
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
            if let Some(effect) = effect {
                self.politics[effect.province]
                    .change_relation(effect.player, -effect.relation_loss);
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
                }
            }
            let message = if active {
                let influence_loss = self.actors[player]
                    .influence
                    .min(self.economy.config.trade.cancellation_influence_penalty);
                self.actors[player].influence -= influence_loss;
                self.senate.record_trade_breach(player);
                self.push_wallets();
                self.reconcile_provinces();
                "Trade route cancelled.".to_owned()
            } else {
                "Trade offer cancelled.".to_owned()
            };
            self.notify_trade(
                id,
                self.economy.month,
                if active {
                    NoticeSeverity::Warning
                } else {
                    NoticeSeverity::Info
                },
                NoticeKind::TradeUpdated,
                if active {
                    "Trade stopped"
                } else {
                    "Trade offer cancelled"
                },
                &message,
            );
            Ok(message)
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
        let status = self.economy.trades.iter().find(|trade| trade.id == id).unwrap().status;
        let (title, outcome) = match status {
            TradeStatus::Proposed => ("Trade offer sent", "Trade offer sent."),
            TradeStatus::Completed => ("Trade completed", "Exchange completed."),
            _ => ("Trade route opened", "Trade route opened."),
        };
        self.notify_trade(
            id,
            self.economy.month,
            NoticeSeverity::Info,
            NoticeKind::TradeUpdated,
            title,
            outcome,
        );
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
        let completed = self
            .economy
            .trades
            .iter()
            .any(|trade| trade.id == id && trade.status == TradeStatus::Completed);
        let message = if completed {
            "Exchange completed."
        } else {
            "Trade offer accepted."
        };
        self.notify_trade(
            id,
            self.economy.month,
            NoticeSeverity::Info,
            NoticeKind::TradeUpdated,
            if completed {
                "Trade completed"
            } else {
                "Trade accepted"
            },
            message,
        );
        Ok(message.into())
    }
}
