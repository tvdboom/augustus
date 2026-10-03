//! Adapts real campaign conditions into espionage opportunities and notification events.

use super::campaign::Campaign;
use super::campaign_notifications::{CampaignNotice, NoticeAction, NoticeKind, NoticeSeverity};
use crate::game::economy::{FoodPolicy, SlaveLabor};
use crate::game::politics::diplomacy::{PoliticalState, Tribute};
use crate::game::politics::espionage::{
    EspionageEvent, ScandalKind, ScandalTarget, ScandalUse, Severity, SpyProvince,
};
use crate::game::politics::PoliticalError;

impl Campaign {
    /// Expose exactly this player's evidence through the existing Senate penalty rules.
    pub fn expose_scandal(&mut self, player: usize, id: u64) -> Result<usize, PoliticalError> {
        if self.defeated.get(player).copied().unwrap_or(false) {
            return Err(PoliticalError::Ineligible);
        }
        let month = self.economy.month.max(self.senate.month);
        if !self.espionage.scandals.iter().any(|evidence| {
            evidence.id == id
                && evidence.holder == player
                && evidence.is_current(month)
                && !evidence.reserved_for_motion
        }) {
            return Err(PoliticalError::ScandalRequired);
        }
        self.senate.expose_scandal(
            player,
            id,
            &self.actors,
            &mut self.espionage,
            &self.senate_config,
        )
    }

    /// Quote only benefits this holder can still obtain against the current government.
    pub fn scandal_use_quote(
        &self,
        player: usize,
        id: u64,
        province: usize,
        usage: ScandalUse,
    ) -> Result<f64, PoliticalError> {
        if self.senate.winner.is_some() {
            return Err(PoliticalError::CampaignFinished);
        }
        if self.defeated.get(player).copied().unwrap_or(false) {
            return Err(PoliticalError::Ineligible);
        }
        let month = self.economy.month.max(self.senate.month);
        let evidence = self
            .espionage
            .scandals
            .iter()
            .find(|evidence| {
                evidence.id == id
                    && evidence.holder == player
                    && evidence.is_current(month)
                    && !evidence.reserved_for_motion
            })
            .ok_or(PoliticalError::ScandalRequired)?;
        let politics = self.politics.get(province).ok_or(PoliticalError::MissingTarget)?;
        if player >= self.actors.len()
            || player >= politics.relations.len()
            || province >= self.economy.provinces.len()
        {
            return Err(PoliticalError::MissingTarget);
        }
        let matches_target = match evidence.target {
            ScandalTarget::Province(target) => {
                target == province
                    && matches!(
                        politics.state,
                        PoliticalState::Independent { .. } | PoliticalState::Vassal { .. }
                    )
            },
            ScandalTarget::Player(target) => {
                target != player
                    && match politics.state {
                        PoliticalState::Owned {
                            owner,
                        } => owner == target,
                        PoliticalState::Vassal {
                            overlord,
                            ..
                        } => overlord == target,
                        _ => false,
                    }
            },
        };
        if !matches_target {
            return Err(PoliticalError::Ineligible);
        }
        let gain = evidence.severity.control_gain();
        let amount = match usage {
            ScandalUse::Control => {
                if !evidence.kind.grants_provincial_control()
                    || !matches!(
                        politics.state,
                        PoliticalState::Independent { .. } | PoliticalState::Owned { .. }
                    )
                {
                    return Err(PoliticalError::Ineligible);
                }
                gain.min((100.0 - politics.control(player)).max(0.0))
            },
            ScandalUse::Relation => gain.min((100.0 - politics.relation(player)).max(0.0)),
            ScandalUse::Trade => {
                if !matches!(evidence.target, ScandalTarget::Province(_))
                    || !matches!(politics.state, PoliticalState::Independent { .. })
                {
                    return Err(PoliticalError::Ineligible);
                }
                (1.0 - self.espionage_config.favorable_trade_ratio) * 100.0
            },
        };
        if amount <= 1e-9 || !amount.is_finite() {
            return Err(PoliticalError::Ineligible);
        }
        Ok(amount)
    }

    /// Spend once after validation, then publish the same immediate gain the row quoted.
    pub fn use_scandal(
        &mut self,
        player: usize,
        id: u64,
        province: usize,
        usage: ScandalUse,
    ) -> Result<f64, PoliticalError> {
        let amount = self.scandal_use_quote(player, id, province, usage)?;
        let month = self.economy.month.max(self.senate.month);
        if usage == ScandalUse::Trade {
            self.espionage.blackmail_trade(player, id, month, &self.espionage_config)?;
        } else {
            let mut next = self.politics[province].clone();
            match usage {
                ScandalUse::Control => next.gain_control_now(player, amount)?,
                ScandalUse::Relation => next.change_relation(player, amount),
                ScandalUse::Trade => unreachable!(),
            }
            self.espionage.consume(player, id, month)?;
            self.politics[province] = next;
        }
        self.reconcile_provinces();
        Ok(amount)
    }

    pub fn flee_spy(&mut self, player: usize, province: usize) -> Result<bool, String> {
        let target = self.economy.provinces.get(province).ok_or("Unknown province")?;
        let snapshot = SpyProvince {
            owner: target.owner,
            noble_happiness: target.happiness[0],
            conditions: Vec::new(),
        };
        let events = self
            .espionage
            .flee(
                player,
                province,
                &snapshot,
                &mut self.politics,
                self.economy.month,
                &self.espionage_config,
            )
            .map_err(|error| format!("{error:?}"))?;
        let detected = events.iter().any(|event| matches!(event, EspionageEvent::Detected(..)));
        self.report_espionage_events(events);
        self.reconcile_provinces();
        Ok(detected)
    }

    /// Monthly coin obligation for all of this player's currently deployed spies.
    pub fn spy_upkeep(&self, player: usize) -> f64 {
        self.espionage
            .missions
            .iter()
            .filter(|mission| mission.owner == player)
            .filter_map(|mission| {
                self.espionage_config.monthly_cost_at(self.distance(player, mission.province)).ok()
            })
            .sum()
    }

    /// Resolve networks from actual post-demographic happiness, food supply and policy.
    /// This adapter never manufactures a human player's misconduct.
    pub fn advance_espionage(&mut self) {
        if self.espionage.last_resolution_month().is_some_and(|last| self.economy.month <= last) {
            return;
        }
        let distances: Vec<_> = self
            .espionage
            .missions
            .iter()
            .map(|mission| self.distance(mission.owner, mission.province))
            .collect();
        for (mission, distance) in self.espionage.missions.iter_mut().zip(distances) {
            if let Some(distance) = distance {
                mission.distance = distance;
            }
        }
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
                        conditions.push((ScandalKind::HarshLabor, Severity::Minor));
                    }
                    if province.happiness[1] < 25.0 {
                        conditions.push((ScandalKind::UnhappyCitizens, Severity::Medium));
                    }
                    if province.happiness[2] < 25.0 {
                        conditions.push((ScandalKind::UnhappyPlebeians, Severity::Medium));
                    }
                    if supply < 0.99 {
                        conditions.push((ScandalKind::Famine, Severity::Major));
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
        self.report_espionage_events(events);
    }

    fn report_espionage_events(&mut self, events: Vec<EspionageEvent>) {
        for event in events {
            match event {
                EspionageEvent::PopulationUndermined(player, province, class, points) => {
                    let target = &mut self.economy.provinces[province];
                    let mut changes = [0.0; 4];
                    let happiness = &mut target.happiness[class];
                    let reduced = (*happiness - points).clamp(0.0, 100.0);
                    changes[class] = reduced - *happiness;
                    target.temporary_happiness[class] += changes[class];
                    *happiness = reduced;
                    if let Some(mission) = self
                        .espionage
                        .missions
                        .iter_mut()
                        .find(|mission| mission.owner == player && mission.province == province)
                    {
                        mission.totals.happiness_reduced -= changes[class];
                    }
                    if let Some(owner) = target.owner {
                        self.notifications
                            .record_foreign_happiness(player, owner, province, changes);
                    }
                },
                EspionageEvent::Withdrawn(player, province) => self.notifications.province_notice(
                    player,
                    province,
                    self.economy.month,
                    NoticeSeverity::Warning,
                    NoticeKind::SpyWithdrawn,
                    "Spy withdrawn",
                    format!(
                        "Upkeep failed or {} is no longer a foreign target.",
                        self.economy.provinces[province].name
                    ),
                ),
                EspionageEvent::Recalled(player, province) => self.notifications.province_notice(
                    player,
                    province,
                    self.economy.month,
                    NoticeSeverity::Info,
                    NoticeKind::SpyWithdrawn,
                    "Spy recalled",
                    format!("Your spy has left {}.", self.economy.provinces[province].name),
                ),
                EspionageEvent::Detected(player, province) => {
                    let owner = self.economy.provinces[province].owner;
                    let consequence = owner.map_or_else(
                        || {
                            format!(
                                "Relation fell by {:.0}.",
                                self.espionage_config.npc_detection_relation_loss
                            )
                        },
                        |victim| format!("Player {} gained an espionage scandal.", victim + 1),
                    );
                    self.notifications.province_notice(
                        player,
                        province,
                        self.economy.month,
                        NoticeSeverity::Warning,
                        NoticeKind::SpyDetected,
                        "Spy uncovered",
                        format!(
                            "Spy lost in {}. {consequence}",
                            self.economy.provinces[province].name
                        ),
                    );
                },
                EspionageEvent::EvidenceDiscovered(player, id) => {
                    let scandal = self.espionage.scandals.iter().find(|s| s.id == id);
                    if let Some(scandal) = scandal {
                        self.notifications.remember_scandal_target(id, scandal.target);
                        self.notifications.push(CampaignNotice {
                            id: 0,
                            recipient: player,
                            severity: NoticeSeverity::Info,
                            title: format!("Scandal discovered · {}", scandal.kind.label()),
                            body: format!(
                                "Against {} · {} · {} at discovery.",
                                match scandal.target {
                                    ScandalTarget::Player(target) =>
                                        format!("Player {}", target + 1),
                                    ScandalTarget::Province(province) =>
                                        self.economy.provinces[province].name.clone(),
                                },
                                scandal.severity.label(),
                                scandal.validity_label(self.economy.month)
                            ),
                            kind: NoticeKind::ScandalDiscovered,
                            province: scandal.province,
                            building: None,
                            wonder: None,
                            scandal: Some(id),
                            month: self.economy.month,
                            action: NoticeAction::OpenScandal {
                                scandal: id,
                                province: match scandal.target {
                                    ScandalTarget::Province(province) => Some(province),
                                    ScandalTarget::Player(_) => None,
                                },
                            },
                        });
                    }
                },
                EspionageEvent::EvidenceExpired(scandal) => {
                    self.notifications.remember_scandal_target(scandal.id, scandal.target);
                    self.notifications.push(CampaignNotice {
                        id: 0,
                        recipient: scandal.holder,
                        severity: NoticeSeverity::Info,
                        title: format!("Scandal expired · {}", scandal.kind.label()),
                        body: format!(
                            "{} · Valid for {} months.",
                            scandal.severity.label(),
                            scandal.expires.saturating_sub(scandal.acquired)
                        ),
                        kind: NoticeKind::ScandalExpired,
                        province: scandal.province,
                        building: None,
                        wonder: None,
                        scandal: Some(scandal.id),
                        month: self.economy.month,
                        action: NoticeAction::OpenScandal {
                            scandal: scandal.id,
                            province: scandal.province,
                        },
                    });
                },
            }
        }
        // Blackmail affects valuation only for its actual owner and target; economics
        // reads these multipliers at the next proposal/recurring evaluation.
    }
}

#[cfg(test)]
#[path = "../../tests/unit/campaign_espionage.rs"]
mod tests;
