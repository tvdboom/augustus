//! Persistent spy assignments and evidence derived from actual player decisions.

use super::diplomacy::{PoliticalState, ProvincePolitics};
use super::{Currency, PlayerId, PoliticalError, PoliticalPlayer, PoliticalRng, ProvinceId};
use std::collections::{BTreeMap, BTreeSet};

/// Central spy economics, random probabilities and evidence lifetime.
#[derive(Debug, Clone)]
pub struct EspionageConfig {
    /// One-time Influence price to deploy a network.
    pub deployment_influence: f64,
    /// Coin maintenance paid before each detection check.
    pub monthly_coin: f64,
    /// Detection chance with zero and fully happy Nobles.
    pub detection_range: [f64; 2],
    /// Relation penalty when an NPC catches a spy.
    pub npc_detection_relation_loss: f64,
    /// Maximum lifetime of ordinary evidence in months.
    pub evidence_lifetime: u32,
    /// Monthly chance to create an NPC's hidden scandal.
    pub npc_generation_chance: f64,
    /// Maximum hidden scandals per NPC province.
    pub npc_pool_cap: usize,
    /// Chance for a surviving network to uncover an NPC scandal.
    pub npc_discovery_chance: f64,
    /// Duration of blackmailed favorable trading terms.
    pub favorable_trade_months: u32,
    /// Multiplier applied to the NPC required-value ratio under blackmail.
    pub favorable_trade_ratio: f64,
}

impl Default for EspionageConfig {
    /// Recommended specification defaults, with a bounded NPC discovery rate.
    fn default() -> Self {
        Self {
            deployment_influence: 10.0,
            monthly_coin: 5.0,
            detection_range: [0.02, 0.10],
            npc_detection_relation_loss: 10.0,
            evidence_lifetime: 24,
            npc_generation_chance: 0.05,
            npc_pool_cap: 3,
            npc_discovery_chance: 0.15,
            favorable_trade_months: 6,
            favorable_trade_ratio: 0.75,
        }
    }
}

/// Mechanically meaningful evidence, with distinct affected Senate blocs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScandalKind {
    /// Low ration policy, discovered only while active.
    LowFood,
    /// High tax policy.
    HighTaxes,
    /// Harsh labor policy.
    HarshLabor,
    /// Citizen happiness below the configured threshold.
    UnhappyCitizens,
    /// Plebeian happiness below the configured threshold.
    UnhappyPlebeians,
    /// Inadequate food supply causing famine.
    Famine,
    /// Severe starvation.
    MassStarvation,
    /// High vassal tribute.
    HighTribute,
    /// Coercive occupation of a hostile vassal.
    HostileOccupation,
    /// Actual broken agreement, retained as a completed action.
    TreatyViolation,
    /// Actual attack on a friendly province.
    FriendlyAttack,
    /// A detected spy network against another player.
    Espionage,
    /// Actual coin spending on Senate bribery.
    PoliticalBribery,
    /// NPC corruption.
    CorruptGovernor,
    /// NPC secret payments.
    SecretPayments,
    /// NPC citizen abuse.
    CitizenAbuse,
    /// NPC illegal taxes.
    IllegalTaxes,
    /// NPC military failures.
    MilitaryIncompetence,
    /// NPC feuding nobles.
    EliteFeud,
    /// NPC illicit commerce.
    Smuggling,
}

impl ScandalKind {
    /// Readable evidence label.
    pub fn label(self) -> &'static str {
        match self {
            Self::LowFood => "Low Food Supply",
            Self::HighTaxes => "High Taxes",
            Self::HarshLabor => "Harsh Slave Labor",
            Self::UnhappyCitizens => "Unhappy Citizens",
            Self::UnhappyPlebeians => "Unhappy Plebeians",
            Self::Famine => "Active Famine",
            Self::MassStarvation => "Mass Starvation",
            Self::HighTribute => "High Vassal Tribute",
            Self::HostileOccupation => "Hostile Vassal Occupation",
            Self::TreatyViolation => "Broken Agreement",
            Self::FriendlyAttack => "Attack on a Friendly Province",
            Self::Espionage => "Detected Espionage",
            Self::PoliticalBribery => "Political Bribery",
            Self::CorruptGovernor => "Corrupt Governor",
            Self::SecretPayments => "Secret Payments",
            Self::CitizenAbuse => "Abuse of Citizens",
            Self::IllegalTaxes => "Illegal Taxes",
            Self::MilitaryIncompetence => "Military Incompetence",
            Self::EliteFeud => "Elite Feud",
            Self::Smuggling => "Smuggling",
        }
    }

    /// Base monthly discovery probability, independent of Noble happiness.
    pub fn discovery_chance(self) -> f64 {
        match self {
            Self::LowFood | Self::UnhappyCitizens | Self::UnhappyPlebeians => 0.10,
            Self::HarshLabor => 0.08,
            Self::HighTaxes => 0.06,
            Self::Famine | Self::MassStarvation => 0.20,
            Self::TreatyViolation | Self::FriendlyAttack => 0.25,
            Self::PoliticalBribery => 0.15,
            _ => 0.10,
        }
    }

    /// Probability penalties for Aristocrats, Merchants, Provincials, Populares, Military.
    pub fn bloc_penalties(self, severity: Severity) -> [f64; 5] {
        let base = match self {
            Self::LowFood | Self::Famine | Self::MassStarvation => [0.02, 0.03, 0.08, 0.14, 0.03],
            Self::HarshLabor | Self::CitizenAbuse => [0.01, 0.02, 0.08, 0.09, 0.01],
            Self::HighTribute | Self::HostileOccupation => [0.02, 0.03, 0.14, 0.05, 0.01],
            Self::PoliticalBribery | Self::CorruptGovernor | Self::SecretPayments => {
                [0.12, 0.10, 0.03, 0.04, 0.02]
            },
            Self::Espionage | Self::TreatyViolation | Self::FriendlyAttack => {
                [0.07, 0.07, 0.08, 0.07, 0.05]
            },
            Self::MilitaryIncompetence => [0.03, 0.02, 0.04, 0.02, 0.14],
            Self::HighTaxes | Self::IllegalTaxes | Self::Smuggling => {
                [0.03, 0.10, 0.04, 0.08, 0.01]
            },
            Self::EliteFeud => [0.12, 0.03, 0.02, 0.02, 0.01],
            Self::UnhappyCitizens | Self::UnhappyPlebeians => [0.02, 0.03, 0.06, 0.12, 0.02],
        };
        base.map(|effect| effect * severity.multiplier())
    }
}

/// Severity scales discoverability, Senate penalties, and NPC political leverage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Small indiscretion.
    Minor,
    /// Significant misconduct, including detected espionage.
    Medium,
    /// Serious misconduct or mass suffering.
    Major,
}

impl Severity {
    /// Shared severity multiplier for discovery and Senate support effects.
    pub fn multiplier(self) -> f64 {
        match self {
            Self::Minor => 0.75,
            Self::Medium => 1.0,
            Self::Major => 1.5,
        }
    }
    /// Scandal leverage against independent NPC control.
    pub fn control_gain(self) -> f64 {
        match self {
            Self::Minor => 5.0,
            Self::Medium => 7.5,
            Self::Major => 10.0,
        }
    }
}

/// Real player action or an NPC province whose government can be blackmailed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScandalTarget {
    /// Player evidence is only used for political leverage.
    Player(PlayerId),
    /// NPC evidence may grant control or favorable trade.
    Province(ProvinceId),
}

/// Stored evidence persists after its underlying condition has ended.
#[derive(Debug, Clone)]
pub struct Scandal {
    /// Globally unique evidence item.
    pub id: u64,
    /// Owning player whose spies uncovered it.
    pub holder: PlayerId,
    /// Subject of the evidence.
    pub target: ScandalTarget,
    /// Actual misconduct.
    pub kind: ScandalKind,
    /// Strength of the evidence.
    pub severity: Severity,
    /// Location where evidence originated.
    pub province: Option<ProvinceId>,
    /// Activation identity; a holder never discovers this source twice.
    pub source_id: u64,
    /// Discovery month.
    pub acquired: u32,
    /// Exclusive expiry month.
    pub expires: u32,
    /// Evidence supporting the active removal motion survives expiry until its vote.
    pub reserved_for_motion: bool,
}

/// One active policy/condition activation or a retained completed action.
#[derive(Debug, Clone)]
pub struct ScandalOpportunity {
    /// Unique activation identity; never reuse after the condition is disabled.
    pub source_id: u64,
    /// Subject of the real misconduct.
    pub player: PlayerId,
    /// Province where the underlying condition occurred.
    pub province: Option<ProvinceId>,
    /// Condition or action.
    pub kind: ScandalKind,
    /// Severity at the last snapshot.
    pub severity: Severity,
    /// Completed action remains discoverable until this optional date.
    pub expires: Option<u32>,
}

/// Persistent, single-province spy network.
#[derive(Debug, Clone)]
pub struct SpyMission {
    /// Player paying and benefiting from the network.
    pub owner: PlayerId,
    /// Target province index.
    pub province: ProvinceId,
    /// Number of paid, resolved operational months.
    pub months_active: u32,
}

/// Province input snapshot; contains real policy flags, not invented human scandals.
#[derive(Debug, Clone)]
pub struct SpyProvince {
    /// Actual owning player; None means an NPC independent or vassal government.
    pub owner: Option<PlayerId>,
    /// Noble happiness controls detection, not evidence discovery.
    pub noble_happiness: f64,
    /// Currently active, real conditions with their severity.
    pub conditions: Vec<(ScandalKind, Severity)>,
}

/// Expiring favorable trade applies only to one player–NPC relationship.
#[derive(Debug, Clone)]
pub struct TradeLeverage {
    /// Benefiting player.
    pub player: PlayerId,
    /// NPC target.
    pub province: ProvinceId,
    /// Exclusive expiration month.
    pub expires: u32,
    /// Required NPC value multiplier.
    pub ratio: f64,
}

/// Events for notifications and political relation updates.
#[derive(Debug, Clone)]
pub enum EspionageEvent {
    /// Network could no longer pay maintenance.
    Withdrawn(PlayerId, ProvinceId),
    /// Target uncovered and removed the foreign network.
    Detected(PlayerId, ProvinceId),
    /// Holder acquired an evidence item.
    EvidenceDiscovered(PlayerId, u64),
}

/// Authoritative spy and evidence inventory, using its own deterministic random stream.
#[derive(Debug, Clone)]
pub struct EspionageState {
    /// Active missions; duplicate owner/target pairs are rejected.
    pub missions: Vec<SpyMission>,
    /// Discovered evidence visible only to its holder until exposed.
    pub scandals: Vec<Scandal>,
    /// Ongoing actual game-state conditions, plus completed actions.
    pub opportunities: Vec<ScandalOpportunity>,
    /// Player-specific blackmail modifiers.
    pub favorable_trade: Vec<TradeLeverage>,
    hidden_npc: BTreeMap<ProvinceId, Vec<(u64, ScandalKind, Severity)>>,
    discovered_sources: BTreeSet<(PlayerId, u64)>,
    global_conditions: BTreeMap<(PlayerId, ScandalKind), Severity>,
    next_id: u64,
    last_resolution_month: Option<u32>,
    rng: PoliticalRng,
}

impl EspionageState {
    /// Initialize the match's authoritative evidence stream.
    pub fn new(seed: u64) -> Self {
        Self {
            missions: Vec::new(),
            scandals: Vec::new(),
            opportunities: Vec::new(),
            favorable_trade: Vec::new(),
            hidden_npc: BTreeMap::new(),
            discovered_sources: BTreeSet::new(),
            global_conditions: BTreeMap::new(),
            next_id: 1,
            last_resolution_month: None,
            rng: PoliticalRng::new(seed),
        }
    }

    /// Last resolved month; duplicate ticks cannot charge upkeep or refresh intelligence.
    pub fn last_resolution_month(&self) -> Option<u32> {
        self.last_resolution_month
    }

    /// Deploy one network per actor and target, charging Influence exactly once.
    pub fn deploy(
        &mut self,
        player: PlayerId,
        province: ProvinceId,
        players: &mut [PoliticalPlayer],
        politics: &[ProvincePolitics],
        config: &EspionageConfig,
    ) -> Result<(), PoliticalError> {
        let target = politics.get(province).ok_or(PoliticalError::MissingTarget)?;
        if target.state == PoliticalState::Rome
            || matches!(target.state, PoliticalState::Owned { owner } if owner == player)
            || matches!(target.state, PoliticalState::Vassal { overlord, .. } if overlord == player)
        {
            return Err(PoliticalError::Ineligible);
        }
        if self.missions.iter().any(|m| m.owner == player && m.province == province) {
            return Err(PoliticalError::AlreadyUsed);
        }
        players
            .get_mut(player)
            .ok_or(PoliticalError::MissingTarget)?
            .spend(Currency::Influence, config.deployment_influence)?;
        self.missions.push(SpyMission {
            owner: player,
            province,
            months_active: 0,
        });
        self.missions.sort_by_key(|m| (m.owner, m.province));
        Ok(())
    }

    /// Voluntary withdrawal stops maintenance and performs no detection roll.
    pub fn withdraw(&mut self, player: PlayerId, province: ProvinceId) {
        self.missions.retain(|m| m.owner != player || m.province != province);
    }

    /// Record a genuine completed action, such as bribery or treaty breaking.
    pub fn record_action(
        &mut self,
        player: PlayerId,
        province: Option<ProvinceId>,
        kind: ScandalKind,
        severity: Severity,
        month: u32,
        config: &EspionageConfig,
    ) -> u64 {
        let id = self.allocate_id();
        self.opportunities.push(ScandalOpportunity {
            source_id: id,
            player,
            province,
            kind,
            severity,
            expires: Some(month + config.evidence_lifetime),
        });
        id
    }

    /// Reconcile a real player-wide policy such as high tribute. Deactivation prevents
    /// further discovery; restarting later receives a fresh source identity.
    pub fn set_global_condition(
        &mut self,
        player: PlayerId,
        kind: ScandalKind,
        active: bool,
        severity: Severity,
    ) {
        if !active {
            self.global_conditions.remove(&(player, kind));
            self.opportunities.retain(|o| {
                !(o.player == player
                    && o.kind == kind
                    && o.province.is_none()
                    && o.expires.is_none())
            });
            return;
        }
        self.global_conditions.insert((player, kind), severity);
        if let Some(opportunity) = self.opportunities.iter_mut().find(|o| {
            o.player == player && o.kind == kind && o.province.is_none() && o.expires.is_none()
        }) {
            opportunity.severity = severity;
        } else {
            let source_id = self.allocate_id();
            self.opportunities.push(ScandalOpportunity {
                source_id,
                player,
                province: None,
                kind,
                severity,
                expires: None,
            });
        }
    }

    /// Check possession for a motion using only unexpired matching evidence.
    pub fn holds_evidence(&self, holder: PlayerId, id: u64, target: PlayerId, month: u32) -> bool {
        self.scandals.iter().any(|s| {
            s.id == id
                && s.holder == holder
                && s.target == ScandalTarget::Player(target)
                && s.expires > month
                && !s.reserved_for_motion
        })
    }

    /// Reserve evidence when its removal motion wins nomination. It cannot also be
    /// exposed, and survives expiration until that single campaign has resolved.
    pub fn reserve_motion(
        &mut self,
        holder: PlayerId,
        id: u64,
        month: u32,
    ) -> Result<(), PoliticalError> {
        let scandal = self
            .scandals
            .iter_mut()
            .find(|s| {
                s.id == id && s.holder == holder && s.expires > month && !s.reserved_for_motion
            })
            .ok_or(PoliticalError::ScandalRequired)?;
        scandal.reserved_for_motion = true;
        Ok(())
    }

    /// Consume the evidence backing a completed removal vote, successful or not.
    pub fn consume_motion(&mut self, id: u64) -> Result<Scandal, PoliticalError> {
        let index = self
            .scandals
            .iter()
            .position(|s| s.id == id && s.reserved_for_motion)
            .ok_or(PoliticalError::ScandalRequired)?;
        Ok(self.scandals.remove(index))
    }

    /// Consume an unexpired scandal atomically and remove its underlying NPC pool entry.
    pub fn consume(
        &mut self,
        holder: PlayerId,
        id: u64,
        month: u32,
    ) -> Result<Scandal, PoliticalError> {
        let index = self
            .scandals
            .iter()
            .position(|s| {
                s.id == id && s.holder == holder && s.expires > month && !s.reserved_for_motion
            })
            .ok_or(PoliticalError::ScandalRequired)?;
        let scandal = self.scandals.remove(index);
        if let ScandalTarget::Province(province) = scandal.target {
            if let Some(pool) = self.hidden_npc.get_mut(&province) {
                pool.retain(|(source, _, _)| *source != scandal.source_id);
            }
            self.scandals.retain(|other| other.source_id != scandal.source_id);
        }
        Ok(scandal)
    }

    /// Use NPC evidence for simultaneous control pressure and the specified relation loss.
    pub fn blackmail_control(
        &mut self,
        player: PlayerId,
        id: u64,
        month: u32,
        politics: &mut [ProvincePolitics],
    ) -> Result<(), PoliticalError> {
        let evidence = self
            .scandals
            .iter()
            .find(|s| s.id == id && s.holder == player && s.expires > month)
            .ok_or(PoliticalError::ScandalRequired)?;
        let ScandalTarget::Province(province) = evidence.target else {
            return Err(PoliticalError::Ineligible);
        };
        let target = politics.get_mut(province).ok_or(PoliticalError::MissingTarget)?;
        if !matches!(target.state, PoliticalState::Independent { .. }) {
            return Err(PoliticalError::Ineligible);
        }
        let gain = evidence.severity.control_gain();
        target.queue_control_gain(player, gain)?;
        target.change_relation(player, -5.0);
        self.consume(player, id, month)?;
        Ok(())
    }

    /// Consume NPC evidence for six months of favorable, player-specific trade terms.
    pub fn blackmail_trade(
        &mut self,
        player: PlayerId,
        id: u64,
        month: u32,
        config: &EspionageConfig,
    ) -> Result<(), PoliticalError> {
        let evidence = self
            .scandals
            .iter()
            .find(|s| s.id == id && s.holder == player && s.expires > month)
            .ok_or(PoliticalError::ScandalRequired)?;
        let ScandalTarget::Province(province) = evidence.target else {
            return Err(PoliticalError::Ineligible);
        };
        self.consume(player, id, month)?;
        self.favorable_trade.retain(|b| b.player != player || b.province != province);
        self.favorable_trade.push(TradeLeverage {
            player,
            province,
            expires: month + config.favorable_trade_months,
            ratio: config.favorable_trade_ratio,
        });
        Ok(())
    }

    /// Return a live NPC valuation modifier; never affects another player's agreement.
    pub fn trade_ratio(&self, player: PlayerId, province: ProvinceId, month: u32) -> f64 {
        self.favorable_trade
            .iter()
            .find(|b| b.player == player && b.province == province && b.expires > month)
            .map_or(1.0, |b| b.ratio)
    }

    /// Maintenance → detection → actual opportunities → NPC pool → discovery → expiry.
    pub fn advance_month(
        &mut self,
        month: u32,
        players: &mut [PoliticalPlayer],
        provinces: &[SpyProvince],
        politics: &mut [ProvincePolitics],
        config: &EspionageConfig,
    ) -> Vec<EspionageEvent> {
        if self.last_resolution_month.is_some_and(|last| month <= last) {
            return Vec::new();
        }
        self.last_resolution_month = Some(month);
        let mut events = Vec::new();
        let mut surviving = Vec::new();
        for mut mission in std::mem::take(&mut self.missions) {
            let Some(province) = provinces.get(mission.province) else {
                continue;
            };
            let own_vassal = politics.get(mission.province).is_some_and(|target| matches!(target.state, PoliticalState::Vassal { overlord, .. } if overlord == mission.owner));
            let capital = politics
                .get(mission.province)
                .is_some_and(|target| target.state == PoliticalState::Rome);
            if province.owner == Some(mission.owner) || own_vassal || capital {
                events.push(EspionageEvent::Withdrawn(mission.owner, mission.province));
                continue;
            }
            let paid = players
                .get_mut(mission.owner)
                .is_some_and(|actor| actor.spend(Currency::Coin, config.monthly_coin).is_ok());
            if !paid {
                events.push(EspionageEvent::Withdrawn(mission.owner, mission.province));
                continue;
            }
            let detection = detection_chance(province.noble_happiness, config);
            if self.rng.unit() < detection {
                events.push(EspionageEvent::Detected(mission.owner, mission.province));
                if let Some(victim) = province.owner {
                    let source = self.allocate_id();
                    let id = self.grant(
                        victim,
                        ScandalTarget::Player(mission.owner),
                        ScandalKind::Espionage,
                        Severity::Medium,
                        Some(mission.province),
                        source,
                        month,
                        config,
                    );
                    events.push(EspionageEvent::EvidenceDiscovered(victim, id));
                } else if let Some(politics) = politics.get_mut(mission.province) {
                    politics.change_relation(mission.owner, -config.npc_detection_relation_loss);
                }
            } else {
                mission.months_active = mission.months_active.saturating_add(1);
                surviving.push(mission);
            }
        }
        self.missions = surviving;
        self.sync_opportunities(provinces, month);
        let npc_kinds = [
            ScandalKind::CorruptGovernor,
            ScandalKind::SecretPayments,
            ScandalKind::CitizenAbuse,
            ScandalKind::IllegalTaxes,
            ScandalKind::MilitaryIncompetence,
            ScandalKind::EliteFeud,
            ScandalKind::Smuggling,
        ];
        for (province, snapshot) in provinces.iter().enumerate() {
            if snapshot.owner.is_some() {
                self.hidden_npc.remove(&province);
                continue;
            }
            let count = self.hidden_npc.get(&province).map_or(0, Vec::len);
            if count < config.npc_pool_cap && self.rng.unit() < config.npc_generation_chance {
                let kind = npc_kinds[(self.rng.unit() * npc_kinds.len() as f64) as usize];
                let severity = if self.rng.unit() < 0.25 {
                    Severity::Major
                } else {
                    Severity::Minor
                };
                let source = self.allocate_id();
                self.hidden_npc.entry(province).or_default().push((source, kind, severity));
            }
        }
        for mission in self.missions.clone() {
            let snapshot = &provinces[mission.province];
            if let Some(target_player) = snapshot.owner {
                let opportunities: Vec<_> = self
                    .opportunities
                    .iter()
                    .filter(|o| {
                        o.player == target_player
                            // Completed foreign actions are evidence about the perpetrator.
                            // Their origin may never be one of that player's holdings, so
                            // a network in any of their provinces can uncover the act.
                            && (o.expires.is_some()
                                || o.province.is_none()
                                || o.province == Some(mission.province))
                            && !self.discovered_sources.contains(&(mission.owner, o.source_id))
                    })
                    .cloned()
                    .collect();
                for opportunity in opportunities {
                    if self.rng.unit()
                        < (opportunity.kind.discovery_chance() * opportunity.severity.multiplier())
                            .min(1.0)
                    {
                        let id = self.grant(
                            mission.owner,
                            ScandalTarget::Player(target_player),
                            opportunity.kind,
                            opportunity.severity,
                            opportunity.province,
                            opportunity.source_id,
                            month,
                            config,
                        );
                        events.push(EspionageEvent::EvidenceDiscovered(mission.owner, id));
                        break;
                    }
                }
            } else {
                let hidden = self.hidden_npc.get(&mission.province).and_then(|pool| {
                    pool.iter()
                        .find(|(id, _, _)| !self.discovered_sources.contains(&(mission.owner, *id)))
                        .copied()
                });
                if let Some((source, kind, severity)) = hidden {
                    if self.rng.unit() < config.npc_discovery_chance * severity.multiplier() {
                        let id = self.grant(
                            mission.owner,
                            ScandalTarget::Province(mission.province),
                            kind,
                            severity,
                            Some(mission.province),
                            source,
                            month,
                            config,
                        );
                        events.push(EspionageEvent::EvidenceDiscovered(mission.owner, id));
                    }
                }
            }
        }
        self.scandals.retain(|s| s.expires > month || s.reserved_for_motion);
        self.favorable_trade.retain(|b| b.expires > month);
        events
    }

    /// Reconcile policy activations so ending and restarting a source gets a new identity.
    fn sync_opportunities(&mut self, provinces: &[SpyProvince], month: u32) {
        self.opportunities.retain(|o| match o.expires {
            Some(expiry) => expiry > month,
            None => match o.province {
                Some(id) => provinces.get(id).is_some_and(|p| {
                    p.owner == Some(o.player)
                        && p.conditions.iter().any(|(kind, _)| *kind == o.kind)
                }),
                None => self.global_conditions.contains_key(&(o.player, o.kind)),
            },
        });
        for (province, snapshot) in provinces.iter().enumerate() {
            let Some(player) = snapshot.owner else {
                continue;
            };
            for &(kind, severity) in &snapshot.conditions {
                if let Some(existing) = self.opportunities.iter_mut().find(|o| {
                    o.player == player
                        && o.province == Some(province)
                        && o.kind == kind
                        && o.expires.is_none()
                }) {
                    existing.severity = severity;
                } else {
                    let id = self.allocate_id();
                    self.opportunities.push(ScandalOpportunity {
                        source_id: id,
                        player,
                        province: Some(province),
                        kind,
                        severity,
                        expires: None,
                    });
                }
            }
        }
    }

    /// Allocate a never-reused source or evidence identity within this match.
    fn allocate_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Store discovered evidence and remember the source even after consumption/expiry.
    fn grant(
        &mut self,
        holder: PlayerId,
        target: ScandalTarget,
        kind: ScandalKind,
        severity: Severity,
        province: Option<ProvinceId>,
        source_id: u64,
        month: u32,
        config: &EspionageConfig,
    ) -> u64 {
        let id = self.allocate_id();
        self.discovered_sources.insert((holder, source_id));
        self.scandals.push(Scandal {
            id,
            holder,
            target,
            kind,
            severity,
            province,
            source_id,
            acquired: month,
            expires: month + config.evidence_lifetime,
            reserved_for_motion: false,
        });
        id
    }
}

/// Noble happiness affects getting caught independently of scandal discovery.
pub fn detection_chance(noble_happiness: f64, config: &EspionageConfig) -> f64 {
    config.detection_range[0]
        + noble_happiness.clamp(0.0, 100.0) / 100.0
            * (config.detection_range[1] - config.detection_range[0])
}

#[cfg(test)]
#[path = "../../../tests/unit/espionage.rs"]
mod tests;
