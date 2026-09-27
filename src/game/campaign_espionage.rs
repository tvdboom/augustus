//! Adapts real campaign conditions into espionage opportunities and notification events.

use super::campaign::Campaign;
use super::campaign_notifications::{CampaignNotice, NoticeAction, NoticeKind, NoticeSeverity};
use crate::game::economy::{FoodPolicy, SlaveLabor};
use crate::game::politics::diplomacy::{PoliticalState, Tribute};
use crate::game::politics::espionage::{
    EspionageEvent, ScandalKind, ScandalTarget, Severity, SpyProvince,
};
use crate::game::politics::senate::{Ballot, SenateEvent};

impl Campaign {
    /// Resolve networks from actual post-demographic happiness, food supply and policy.
    /// This adapter never manufactures a human player's misconduct.
    pub fn advance_espionage(&mut self) {
        let provinces: Vec<_> = self
            .economy
            .provinces
            .iter()
            .enumerate()
            .map(|(id, province)| {
                let supply = self
                    .economy
                    .last_report
                    .province_reports
                    .get(id)
                    .map_or(1.0, |p| p.food_supply_ratio);
                let mut conditions = Vec::new();
                if province.owner.is_some() {
                    if province.policies.food == FoodPolicy::Low {
                        conditions.push((ScandalKind::LowFood, Severity::Minor));
                    }
                    if province.policies.slave_labor == SlaveLabor::Harsh {
                        conditions.push((ScandalKind::HarshLabor, Severity::Medium));
                    }
                    if province.happiness[1] < 25.0 {
                        conditions.push((ScandalKind::UnhappyCitizens, Severity::Medium));
                    }
                    if province.happiness[2] < 25.0 {
                        conditions.push((ScandalKind::UnhappyPlebeians, Severity::Medium));
                    }
                    if supply < 0.99 {
                        conditions.push((
                            ScandalKind::Famine,
                            if supply < 0.5 {
                                Severity::Major
                            } else {
                                Severity::Medium
                            },
                        ));
                    }
                    if supply < 0.25 {
                        conditions.push((ScandalKind::MassStarvation, Severity::Major));
                    }
                }
                SpyProvince {
                    owner: province.owner,
                    noble_happiness: province.happiness[0],
                    conditions,
                }
            })
            .collect();
        // Vassal decisions belong to the overlord. Their opportunities can be found
        // through any network in that player's directly owned provinces.
        for player in 0..self.actors.len() {
            let high_tribute = self.politics.iter().any(|p| matches!(p.state, PoliticalState::Vassal { overlord, tribute: Tribute::High, .. } if overlord == player));
            let hostile_occupation = self.politics.iter().enumerate().any(|(id, p)| {
                matches!(p.state, PoliticalState::Vassal { overlord, .. } if overlord == player)
                    && p.relation(player) < 40.0
                    && self
                        .military
                        .stationed_strength(id, crate::game::military::ForceOwner::Player(player))
                        > 0.0
            });
            self.espionage.set_global_condition(
                player,
                ScandalKind::HighTribute,
                high_tribute,
                Severity::Medium,
            );
            self.espionage.set_global_condition(
                player,
                ScandalKind::HostileOccupation,
                hostile_occupation,
                Severity::Medium,
            );
        }
        let events = self.espionage.advance_month(
            self.economy.month,
            &mut self.actors,
            &provinces,
            &mut self.politics,
            &self.espionage_config,
        );
        for event in events {
            match event {
                EspionageEvent::Withdrawn(player, province) => self.notifications.province_notice(player, province, self.economy.month, NoticeSeverity::Warning, NoticeKind::SpyWithdrawn,
                    format!("Spy withdrawn from {}", self.economy.provinces[province].name), "The network could not pay maintenance or the province was no longer a foreign target."),
                EspionageEvent::Detected(player, province) => {
                    let consequence = provinces[province].owner.map_or_else(|| format!("Relation toward us decreased by {:.0}.", self.espionage_config.npc_detection_relation_loss), |victim| format!("Player {} gained an espionage scandal against us.", victim + 1));
                    self.notifications.province_notice(player, province, self.economy.month, NoticeSeverity::Warning, NoticeKind::SpyDetected,
                        format!("Spy uncovered in {}", self.economy.provinces[province].name), consequence);
                },
                EspionageEvent::EvidenceDiscovered(player, id) => {
                    let scandal = self.espionage.scandals.iter().find(|s| s.id == id);
                    if let Some(scandal) = scandal {
                        self.notifications.push(CampaignNotice { id: 0, recipient: player, severity: NoticeSeverity::Info,
                            title: "Scandal discovered".into(), body: format!("Our spies uncovered {}.", scandal.kind.label()),
                            kind: NoticeKind::ScandalDiscovered, province: scandal.province, wonder: None, scandal: Some(id),
                            month: self.economy.month, action: NoticeAction::OpenScandal { scandal: id, province: match scandal.target { ScandalTarget::Province(province) => Some(province), ScandalTarget::Player(_) => None } } });
                    }
                },
            }
        }
        // Blackmail affects valuation only for its actual owner and target; economics
        // reads these multipliers at the next proposal/recurring evaluation.
    }

    /// Keep evidence backing the active removal motion valid and consume it at its vote.
    pub fn handle_senate_evidence(&mut self, event: &SenateEvent) {
        match *event {
            SenateEvent::CampaignStarted(
                holder,
                Ballot::NoConfidence {
                    scandal_id,
                    ..
                },
            ) => {
                if self.espionage.reserve_motion(holder, scandal_id, self.economy.month).is_err() {
                    self.senate.campaign = None;
                    self.senate.nominations.clear();
                    self.senate.nomination_elapsed = 0;
                    self.messages.push(("The removal nomination lapsed because its supporting evidence expired. A new Nomination Year begins; committed Influence remains spent.".into(), None));
                }
            },
            SenateEvent::ConsumeScandal(id) => {
                let _ = self.espionage.consume_motion(id);
            },
            _ => {},
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/campaign_espionage.rs"]
mod tests;
