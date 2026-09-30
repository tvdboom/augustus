//! Persistent individual loyalties: monthly realignment and immediate support-gated offices.
use super::{Currency, PlayerId, PoliticalError, PoliticalPlayer, PoliticalRank, PoliticalRng};

/// Public factions with distinct preferences and contiguous chamber sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bloc {
    /// Nobles and republican institutions.
    Aristocrats,
    /// Commerce and economic security.
    Merchants,
    /// Provincial welfare and voluntary vassal stability.
    Provincials,
    /// Citizens, plebeians and domestic policy.
    Populares,
    /// Army strength, service and victories.
    Military,
}
impl Bloc {
    /// Stable faction ordering for configuration and chamber sections.
    pub const ALL: [Self; 5] =
        [Self::Aristocrats, Self::Merchants, Self::Provincials, Self::Populares, Self::Military];
    /// Index into faction arrays.
    pub fn index(self) -> usize {
        self as usize
    }
    /// Full public faction name.
    pub fn label(self) -> &'static str {
        match self {
            Self::Aristocrats => "Aristocrats",
            Self::Merchants => "Merchants",
            Self::Provincials => "Provincials",
            Self::Populares => "Populares",
            Self::Military => "Military",
        }
    }
    /// Short explanation of the faction's actual structural preferences.
    pub fn preferences(self) -> &'static str {
        match self {
            Self::Aristocrats => "Happy nobles, political standing, Forums and wonders. Influence prestige has diminishing returns.",
            Self::Merchants => "Profitable income, reliable delivered trade, Markets and secure resources.",
            Self::Provincials => "Happy free populations, friendly stable vassals and provincial trade. High tribute drives them away.",
            Self::Populares => "Happy citizens and plebeians, generous food and low taxes. Famine and harsh labor drive them away.",
            Self::Military => "Strong trained armies, military career rank and recent victories. Defeats weaken support.",
        }
    }
}

/// Costs and loyalty rules; there is no election calendar.
#[derive(Debug, Clone)]
pub struct SenateConfig {
    /// Aedile, Praetor, Censor, Consul and Augustus appointment costs.
    pub promotion_costs: [f64; 5],
    /// Non-stacking monthly office income; Proconsul shares the Consul slot.
    pub rank_influence: [f64; 6],
    /// Number of senators in each contiguous faction section, totaling 100.
    pub bloc_sizes: [u8; 5],
    /// Length of a Consul term in months.
    pub consul_term: u32,
    /// Mandatory return cooldown after every Consul departure.
    pub consul_cooldown: u32,
    /// Consecutive reviews below retention support before forced resignation.
    pub loss_grace_months: u32,
    /// Influence price of faction outreach.
    pub court_cost: f64,
    /// Initial attraction points from non-stacking faction outreach.
    pub court_bonus: f64,
    /// Lifetime of outreach, fading linearly.
    pub court_months: u32,
    /// Coin price of the first temporary loyalty lease.
    pub bribe_base_coin: f64,
    /// Exclusive lifetime of a senator's bribery lease.
    pub bribe_months: u32,
    /// Maximum number of simultaneously bribed senators per player.
    pub bribe_cap: usize,
    /// Lifetime of exposed accusations, fading linearly.
    pub scandal_months: u32,
}
impl Default for SenateConfig {
    fn default() -> Self {
        Self {
            promotion_costs: [100.0, 180.0, 280.0, 400.0, 800.0],
            rank_influence: [0.0, 1.0, 2.0, 3.0, 4.0, 0.0],
            bloc_sizes: [20; 5],
            consul_term: 24,
            consul_cooldown: 12,
            loss_grace_months: 3,
            court_cost: 20.0,
            court_bonus: 8.0,
            court_months: 6,
            bribe_base_coin: 80.0,
            bribe_months: 6,
            bribe_cap: 10,
            scandal_months: 12,
        }
    }
}
impl SenateConfig {
    /// Enforce chamber, duration and cost invariants before simulation.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.bloc_sizes.iter().map(|n| usize::from(*n)).sum::<usize>() != 100 {
            return Err("The Senate must contain exactly 100 senators.");
        }
        if [
            self.consul_term,
            self.consul_cooldown,
            self.loss_grace_months,
            self.court_months,
            self.bribe_months,
            self.scandal_months,
        ]
        .contains(&0)
        {
            return Err("Political durations must be positive.");
        }
        if self
            .promotion_costs
            .iter()
            .chain([&self.court_cost, &self.court_bonus, &self.bribe_base_coin])
            .any(|n| !n.is_finite() || *n <= 0.0)
        {
            return Err("Political costs and bonuses must be finite and positive.");
        }
        Ok(())
    }
    /// Two-player baseline scales by sqrt(2 / players); victory always needs a majority.
    pub fn requirements(
        &self,
        rank: PoliticalRank,
        players: usize,
    ) -> Option<PromotionRequirement> {
        let next = match rank {
            PoliticalRank::Quaestor => PoliticalRank::Aedile,
            PoliticalRank::Aedile => PoliticalRank::Praetor,
            PoliticalRank::Praetor => PoliticalRank::Censor,
            PoliticalRank::Censor | PoliticalRank::Proconsul => PoliticalRank::Consul,
            PoliticalRank::Consul => PoliticalRank::Augustus,
            PoliticalRank::Augustus => return None,
        };
        let i = next.ladder_index() - 1;
        let scale = (2.0 / players.clamp(2, 8) as f64).sqrt();
        let senators = (([5.0, 12.0, 22.0, 40.0, 60.0][i] * scale).round() as usize).max(
            if next == PoliticalRank::Augustus {
                51
            } else {
                3
            },
        );
        Some(PromotionRequirement {
            rank: next,
            influence: self.promotion_costs[i],
            senators,
        })
    }
    /// Non-stacking monthly Influence supplied by the current office.
    pub fn rank_income(&self, rank: PoliticalRank) -> f64 {
        self.rank_influence[rank.ladder_index()]
    }
    /// Consuls retain office with at least 60% of appointment support, rounded up.
    pub fn retention_support(&self, players: usize) -> usize {
        (self.requirements(PoliticalRank::Censor, players).unwrap().senators * 3).div_ceil(5)
    }
}
#[derive(Debug, Clone, Copy)]
/// Requirements for the actor's next available office.
pub struct PromotionRequirement {
    /// Office to be awarded.
    pub rank: PoliticalRank,
    /// One-time Influence price.
    pub influence: f64,
    /// Number of currently loyal senators required.
    pub senators: usize,
}

/// Current empire aggregates used to explain structural bloc support.
#[derive(Debug, Clone)]
pub struct PoliticalProfile {
    /// Total Noble population, soft-capped in the support model.
    pub nobles: f64,
    /// Average Noble happiness, 0..100.
    pub noble_happiness: f64,
    /// Average Citizen happiness, 0..100.
    pub citizen_happiness: f64,
    /// Average Plebeian happiness, 0..100.
    pub plebeian_happiness: f64,
    /// Net recurring coin income.
    pub coin_income: f64,
    /// Recurring delivered trade value.
    pub trade_volume: f64,
    /// Fraction of recurring contracts fulfilled, 0..1.
    pub trade_reliability: f64,
    /// Average fraction of physical-resource needs available, 0..1.
    pub resource_security: f64,
    /// Number of completed wonders.
    pub wonders: f64,
    /// Completed Forum and other political-city building levels.
    pub political_buildings: f64,
    /// Urban Market levels.
    pub markets: f64,
    /// Mean relation among vassals, or neutral 50 when none.
    pub vassal_relation: f64,
    /// Mean Vassal Control multiplied by positive relation, 0..1.
    pub voluntary_vassal_stability: f64,
    /// Average free-pop happiness in the player's provinces.
    pub provincial_happiness: f64,
    /// Delivered recurring trade value to provinces and vassals.
    pub provincial_trade: f64,
    /// Share of vassals using high tribute, 0..1.
    pub high_tribute: f64,
    /// Average food-supply policy: -1 low, 0 normal, +1 high.
    pub food_policy: f64,
    /// Fraction of monthly food requested actually supplied, 0..1.
    pub food_security: f64,
    /// Fraction of owned population currently suffering famine, 0..1.
    pub famine: f64,
    /// Tax pressure above normal, 0..1.
    pub tax_pressure: f64,
    /// Fraction of provinces using harsh labor, 0..1.
    pub harsh_policies: f64,
    /// Effective military strength, soft-capped rather than a raw vote multiplier.
    pub military_strength: f64,
    /// Net recent victories and successful defenses minus defeats.
    pub recent_victories: f64,
    /// Military career rank index, 0..3.
    pub military_rank: f64,
}

impl Default for PoliticalProfile {
    /// Neutral profile for players with no provinces or unavailable observations.
    fn default() -> Self {
        Self {
            nobles: 0.0,
            noble_happiness: 50.0,
            citizen_happiness: 50.0,
            plebeian_happiness: 50.0,
            coin_income: 0.0,
            trade_volume: 0.0,
            trade_reliability: 1.0,
            resource_security: 1.0,
            wonders: 0.0,
            political_buildings: 0.0,
            markets: 0.0,
            vassal_relation: 50.0,
            voluntary_vassal_stability: 0.5,
            provincial_happiness: 50.0,
            provincial_trade: 0.0,
            high_tribute: 0.0,
            food_policy: 0.0,
            food_security: 1.0,
            famine: 0.0,
            tax_pressure: 0.0,
            harsh_policies: 0.0,
            military_strength: 0.0,
            recent_victories: 0.0,
            military_rank: 0.0,
        }
    }
}

/// Named contribution in percentage points for transparent hover explanations.
#[derive(Debug, Clone)]
pub struct SupportReason {
    /// Concise explanation of the driver.
    pub label: &'static str,
    /// Signed percentage-point support contribution.
    pub points: f64,
}

/// One stable senator with individual emphasis within a public faction.
#[derive(Debug, Clone)]
pub struct Senator {
    /// Stable chamber identity, never changed during a match.
    pub id: usize,
    /// Faction determining structural preferences and chamber section.
    pub bloc: Bloc,
    /// Player currently supported, or neutral gray.
    pub allegiance: Option<PlayerId>,
    /// Temporary loyalty override, visibly marked by a gold ring.
    pub bribe: Option<SenatorBribe>,
    preferences: [f64; 7],
    threshold: f64,
}
#[derive(Debug, Clone, Copy)]
/// An exclusive temporary loyalty lease.
pub struct SenatorBribe {
    /// Player who paid for the senator's support.
    pub player: PlayerId,
    /// Exclusive expiry month.
    pub until: u32,
}
#[derive(Debug, Clone)]
struct Outreach {
    player: PlayerId,
    bloc: Bloc,
    until: u32,
}
#[derive(Debug, Clone)]
/// An exposed real scandal with faction-specific, fading attraction penalties.
pub struct Accusation {
    /// Player whose misconduct was exposed.
    pub target: PlayerId,
    /// Attraction-point penalties in faction order.
    pub penalties: [f64; 5],
    /// Exclusive expiry month.
    pub until: u32,
}
#[derive(Debug, Clone)]
/// Office transitions for player-scoped notifications and victory handling.
pub enum SenateEvent {
    /// Player paid Influence and met the support requirement.
    RankAdvanced(PlayerId, PoliticalRank),
    /// A Consul completed their term and became Proconsul.
    ConsulExpired(PlayerId),
    /// Lost support, possibly accelerated by a scandal, forced resignation.
    ConsulRemoved(PlayerId),
    /// A serving Consul secured Augustus support and won.
    Victory(PlayerId),
}

/// The authoritative chamber. No bids, vote rolls, nominee or global ballot remains.
#[derive(Debug, Clone)]
pub struct SenateState {
    /// Number of completed monthly loyalty reviews.
    pub month: u32,
    /// All 100 persistent public allegiances.
    pub senators: Vec<Senator>,
    /// Winning player, set by Augustus appointment or Rome's conquest.
    pub winner: Option<PlayerId>,
    /// Active publicly exposed scandals.
    pub accusations: Vec<Accusation>,
    outreach: Vec<Outreach>,
    low_support: Vec<u32>,
    used_actions: Vec<(PlayerId, Bloc, bool)>,
}
impl SenateState {
    /// A public invasion without casus belli damages faction support immediately.
    pub fn record_unjustified_attack(&mut self, player: PlayerId, config: &SenateConfig) {
        use super::espionage::{ScandalKind, Severity};
        self.accusations.push(Accusation {
            target: player,
            penalties: ScandalKind::FriendlyAttack
                .bloc_penalties(Severity::Major)
                .map(|value| value * 100.0),
            until: self.month.saturating_add(config.scandal_months),
        });
    }

    /// Breaking a trade route immediately damages Merchant support for six months.
    pub fn record_trade_breach(&mut self, player: PlayerId) {
        self.accusations.push(Accusation {
            target: player,
            penalties: [0.0, 8.0, 0.0, 0.0, 0.0],
            until: self.month.saturating_add(6),
        });
        if let Some(senator) = self
            .senators
            .iter_mut()
            .find(|senator| senator.bloc == Bloc::Merchants && senator.allegiance == Some(player))
        {
            senator.allegiance = None;
        }
    }

    /// Initialize a default chamber with neutral loyalties and seeded preferences.
    pub fn new(seed: u64) -> Self {
        Self::with_config(seed, &SenateConfig::default())
    }
    /// Initialize a chamber from the authoritative match configuration.
    pub fn with_config(seed: u64, config: &SenateConfig) -> Self {
        assert!(config.validate().is_ok(), "invalid Senate configuration");
        let mut rng = PoliticalRng::new(seed);
        let mut senators = Vec::with_capacity(100);
        for bloc in Bloc::ALL {
            for _ in 0..config.bloc_sizes[bloc.index()] {
                senators.push(Senator {
                    id: senators.len(),
                    bloc,
                    allegiance: None,
                    bribe: None,
                    preferences: std::array::from_fn(|_| 0.65 + rng.unit() * 0.7),
                    threshold: 3.0 + rng.unit() * 10.0,
                });
            }
        }
        Self {
            month: 0,
            senators,
            winner: None,
            accusations: vec![],
            outreach: vec![],
            low_support: vec![],
            used_actions: vec![],
        }
    }
    /// Count all senators currently supporting a player.
    pub fn support(&self, player: PlayerId) -> usize {
        self.senators.iter().filter(|s| s.allegiance == Some(player)).count()
    }
    /// Count the player's loyal senators within one faction.
    pub fn bloc_support(&self, player: PlayerId, bloc: Bloc) -> usize {
        self.senators.iter().filter(|s| s.bloc == bloc && s.allegiance == Some(player)).count()
    }
    /// Validate rank, cooldown, seats, support and funds without mutating anything.
    pub fn promotion_eligibility(
        &self,
        player: PlayerId,
        players: &[PoliticalPlayer],
        config: &SenateConfig,
    ) -> Result<PromotionRequirement, PoliticalError> {
        if self.winner.is_some() {
            return Err(PoliticalError::Ineligible);
        }
        let actor = players.get(player).ok_or(PoliticalError::MissingTarget)?;
        let requirement =
            config.requirements(actor.rank, players.len()).ok_or(PoliticalError::Ineligible)?;
        if requirement.rank == PoliticalRank::Consul {
            if actor.consul_again_at > self.month {
                return Err(PoliticalError::ConsulCooldown);
            }
            if players.iter().filter(|p| p.rank == PoliticalRank::Consul).count() >= 2 {
                return Err(PoliticalError::NoConsulSeat);
            }
        }
        if requirement.rank == PoliticalRank::Augustus
            && actor.consul_until.is_none_or(|end| end <= self.month)
        {
            return Err(PoliticalError::Ineligible);
        }
        if actor.promoted_at == Some(self.month) {
            return Err(PoliticalError::AlreadyUsed);
        }
        if self.support(player) < requirement.senators {
            return Err(PoliticalError::InsufficientSupport);
        }
        if actor.influence + 1e-9 < requirement.influence {
            return Err(PoliticalError::InsufficientFunds);
        }
        Ok(requirement)
    }
    /// Appoint immediately; supporters stay with their player and are never consumed.
    pub fn promote(
        &mut self,
        player: PlayerId,
        players: &mut [PoliticalPlayer],
        config: &SenateConfig,
    ) -> Result<SenateEvent, PoliticalError> {
        let requirement = self.promotion_eligibility(player, players, config)?;
        players[player].spend(Currency::Influence, requirement.influence)?;
        let actor = &mut players[player];
        actor.rank = requirement.rank;
        actor.promoted_at = Some(self.month);
        if actor.rank == PoliticalRank::Consul {
            actor.consul_until = Some(self.month + config.consul_term);
            self.low_support.resize(players.len(), 0);
            self.low_support[player] = 0;
        }
        if players[player].rank == PoliticalRank::Augustus {
            players[player].consul_until = None;
            self.winner = Some(player);
            Ok(SenateEvent::Victory(player))
        } else {
            Ok(SenateEvent::RankAdvanced(player, requirement.rank))
        }
    }
    /// Legitimate short-lived outreach. Refreshing replaces the bonus; it never stacks.
    pub fn court(
        &mut self,
        player: PlayerId,
        bloc: Bloc,
        players: &mut [PoliticalPlayer],
        config: &SenateConfig,
    ) -> Result<(), PoliticalError> {
        self.action_eligibility(player, bloc, false, players)?;
        players[player].spend(Currency::Influence, config.court_cost)?;
        self.outreach.retain(|o| o.player != player || o.bloc != bloc);
        self.outreach.push(Outreach {
            player,
            bloc,
            until: self.month + config.court_months,
        });
        self.used_actions.push((player, bloc, false));
        Ok(())
    }
    fn action_eligibility(
        &self,
        player: PlayerId,
        bloc: Bloc,
        bribe: bool,
        players: &[PoliticalPlayer],
    ) -> Result<(), PoliticalError> {
        if self.winner.is_some() {
            return Err(PoliticalError::Ineligible);
        }
        if player >= players.len() {
            return Err(PoliticalError::MissingTarget);
        }
        if self.used_actions.contains(&(player, bloc, bribe)) {
            return Err(PoliticalError::AlreadyUsed);
        }
        Ok(())
    }
    /// Count unexpired leases against this player's bribery cap.
    pub fn active_bribes(&self, player: PlayerId) -> usize {
        self.senators
            .iter()
            .filter(|s| s.bribe.is_some_and(|b| b.player == player && b.until > self.month))
            .count()
    }
    /// Quote the validated escalating price without purchasing a loyalty lease.
    pub fn bribe_quote(
        &self,
        player: PlayerId,
        bloc: Bloc,
        players: &[PoliticalPlayer],
        config: &SenateConfig,
    ) -> Result<f64, PoliticalError> {
        self.action_eligibility(player, bloc, true, players)?;
        let count = self.active_bribes(player);
        if count >= config.bribe_cap
            || !self
                .senators
                .iter()
                .any(|s| s.bloc == bloc && s.allegiance != Some(player) && s.bribe.is_none())
        {
            return Err(PoliticalError::Ineligible);
        }
        Ok(config.bribe_base_coin + 20.0 * count as f64)
    }
    /// One senator per payment, at most ten active leases. Caller records real misconduct.
    pub fn bribe(
        &mut self,
        player: PlayerId,
        bloc: Bloc,
        players: &mut [PoliticalPlayer],
        config: &SenateConfig,
    ) -> Result<usize, PoliticalError> {
        let cost = self.bribe_quote(player, bloc, players, config)?;
        let index = self
            .senators
            .iter()
            .position(|s| s.bloc == bloc && s.allegiance != Some(player) && s.bribe.is_none())
            .ok_or(PoliticalError::Ineligible)?;
        players[player].spend(Currency::Coin, cost)?;
        let senator = &mut self.senators[index];
        senator.bribe = Some(SenatorBribe {
            player,
            until: self.month + config.bribe_months,
        });
        senator.allegiance = Some(player);
        self.used_actions.push((player, bloc, true));
        Ok(senator.id)
    }
    /// Accusations need real unexpired evidence, consumed atomically here. They damage
    /// relevant factions for a year and release that target's compromised bribery leases.
    pub fn expose_scandal(
        &mut self,
        holder: PlayerId,
        id: u64,
        players: &[PoliticalPlayer],
        espionage: &mut super::espionage::EspionageState,
        config: &SenateConfig,
    ) -> Result<PlayerId, PoliticalError> {
        use super::espionage::ScandalTarget;
        if self.winner.is_some() {
            return Err(PoliticalError::Ineligible);
        }
        let evidence = espionage
            .scandals
            .iter()
            .find(|s| {
                s.id == id && s.holder == holder && s.expires > self.month && !s.reserved_for_motion
            })
            .ok_or(PoliticalError::ScandalRequired)?;
        let ScandalTarget::Player(target) = evidence.target else {
            return Err(PoliticalError::Ineligible);
        };
        if target == holder || holder >= players.len() || target >= players.len() {
            return Err(PoliticalError::Ineligible);
        }
        let penalties = evidence.kind.bloc_penalties(evidence.severity).map(|n| n * 100.0);
        espionage.consume(holder, id, self.month)?;
        self.accusations.push(Accusation {
            target,
            penalties,
            until: self.month + config.scandal_months,
        });
        for senator in &mut self.senators {
            if senator.bribe.is_some_and(|b| b.player == target) {
                senator.bribe = None;
            }
        }
        Ok(target)
    }
    /// Explain structural attraction plus the current fading outreach and scandals.
    pub fn reasons(
        &self,
        player: PlayerId,
        bloc: Bloc,
        actor: &PoliticalPlayer,
        profile: &PoliticalProfile,
        config: &SenateConfig,
    ) -> Vec<SupportReason> {
        let mut reasons = structural_reasons(bloc, profile, actor);
        let outreach = self
            .outreach
            .iter()
            .filter(|o| o.player == player && o.bloc == bloc && o.until > self.month)
            .map(|o| {
                config.court_bonus * (o.until - self.month) as f64 / config.court_months as f64
            })
            .sum();
        let penalty = self
            .accusations
            .iter()
            .filter(|a| a.target == player && a.until > self.month)
            .map(|a| {
                a.penalties[bloc.index()] * (a.until - self.month) as f64
                    / config.scandal_months as f64
            })
            .sum::<f64>()
            .min(30.0);
        reasons.push(SupportReason {
            label: "Faction outreach (fades)",
            points: outreach,
        });
        reasons.push(SupportReason {
            label: "Exposed scandals (fades)",
            points: -penalty,
        });
        reasons
    }
    /// All senators compare all players against the same snapshot once each month.
    /// Exact ties remain neutral; a two-point incumbent margin prevents jitter.
    pub fn advance_month(
        &mut self,
        players: &mut [PoliticalPlayer],
        profiles: &[PoliticalProfile],
        config: &SenateConfig,
    ) -> Vec<SenateEvent> {
        assert!(config.validate().is_ok(), "invalid Senate configuration");
        if self.winner.is_some() {
            return vec![];
        }
        self.month += 1;
        self.used_actions.clear();
        self.outreach.retain(|o| o.until > self.month);
        self.accusations.retain(|a| a.until > self.month);
        let factors: Vec<_> = players
            .iter()
            .enumerate()
            .map(|(id, actor)| {
                let p = profiles.get(id).cloned().unwrap_or_default();
                Bloc::ALL.map(|bloc| self.reasons(id, bloc, actor, &p, config))
            })
            .collect();
        for senator in &mut self.senators {
            if let Some(bribe) = senator.bribe {
                if bribe.until > self.month && bribe.player < players.len() {
                    senator.allegiance = Some(bribe.player);
                    continue;
                }
                senator.bribe = None;
            }
            let scores: Vec<f64> = factors
                .iter()
                .map(|factions| {
                    factions[senator.bloc.index()]
                        .iter()
                        .enumerate()
                        .map(|(i, r)| {
                            // Outreach and scandal effects are public and identical within a faction.
                            r.points
                                * if i < structural_reasons_len(senator.bloc) {
                                    senator.preferences[i]
                                } else {
                                    1.0
                                }
                        })
                        .sum()
                })
                .collect();
            let mut best = None;
            let mut best_score = senator.threshold;
            let mut tied = false;
            for (player, &score) in scores.iter().enumerate() {
                if score > best_score + 0.000_001 {
                    best = Some(player);
                    best_score = score;
                    tied = false;
                } else if best.is_some() && (score - best_score).abs() <= 0.000_001 {
                    tied = true;
                }
            }
            if let Some(incumbent) = senator.allegiance.filter(|&p| p < scores.len()) {
                let score = scores[incumbent];
                if score >= senator.threshold - 2.0 && best_score <= score + 2.0 {
                    continue;
                }
            }
            senator.allegiance = if tied {
                None
            } else {
                best
            };
        }
        self.low_support.resize(players.len(), 0);
        let mut events = Vec::new();
        for id in 0..players.len() {
            if players[id].rank != PoliticalRank::Consul {
                self.low_support[id] = 0;
                continue;
            }
            if players[id].consul_until.is_none_or(|end| end <= self.month) {
                self.end_consul(id, players, config);
                events.push(SenateEvent::ConsulExpired(id));
                continue;
            }
            let supporters = self.support(id);
            let minimum = config.retention_support(players.len());
            self.low_support[id] = if supporters < minimum {
                self.low_support[id] + 1
            } else {
                0
            };
            let scandal_removal =
                self.accusations.iter().any(|a| a.target == id && a.until > self.month)
                    && supporters < minimum;
            if scandal_removal || self.low_support[id] >= config.loss_grace_months {
                self.end_consul(id, players, config);
                events.push(SenateEvent::ConsulRemoved(id));
            }
        }
        events
    }
    fn end_consul(&mut self, id: PlayerId, players: &mut [PoliticalPlayer], config: &SenateConfig) {
        players[id].rank = PoliticalRank::Proconsul;
        players[id].consul_until = None;
        players[id].consul_again_at = self.month + config.consul_cooldown;
        self.low_support[id] = 0;
    }
}
fn structural_reasons_len(bloc: Bloc) -> usize {
    match bloc {
        Bloc::Aristocrats => 6,
        Bloc::Merchants | Bloc::Provincials => 5,
        Bloc::Populares => 7,
        Bloc::Military => 3,
    }
}

/// Smooth, bounded return curve shared by Influence and coin campaign actions.
fn diminishing(amount: f64, half: f64) -> f64 {
    amount.max(0.0) / (amount.max(0.0) + half.max(0.001))
}

/// Explain structural bloc preferences in percentage points. Population and wealth
/// use soft caps; even huge empires cannot purchase permanent Senate dominance.
fn structural_reasons(
    bloc: Bloc,
    profile: &PoliticalProfile,
    actor: &PoliticalPlayer,
) -> Vec<SupportReason> {
    let p = profile;
    let rows: Vec<(&'static str, f64)> = match bloc {
        Bloc::Aristocrats => vec![
            (
                "Noble population (diminishing)",
                8.0 * diminishing((p.nobles / crate::map::POPULATION_SCALE).sqrt(), 8.0),
            ),
            ("Noble happiness", (p.noble_happiness - 50.0) * 0.16),
            ("Political office", actor.rank.ladder_index() as f64 * 1.5),
            ("Influence prestige", 4.0 * diminishing(actor.influence, 200.0)),
            ("Completed wonders", 5.0 * diminishing(p.wonders, 2.0)),
            ("Political city buildings", 4.0 * diminishing(p.political_buildings, 4.0)),
        ],
        Bloc::Merchants => vec![
            ("Sestertius income", 8.0 * diminishing(p.coin_income, 30.0)),
            ("Recurring trade", 7.0 * diminishing(p.trade_volume, 60.0)),
            ("Trade reliability", (p.trade_reliability - 0.8) * 15.0),
            ("Urban Markets", 4.0 * diminishing(p.markets, 5.0)),
            ("Resource security", (p.resource_security - 0.8) * 20.0),
        ],
        Bloc::Provincials => vec![
            ("Vassal relations", (p.vassal_relation - 50.0) * 0.14),
            ("Voluntary vassal stability", (p.voluntary_vassal_stability - 0.5) * 10.0),
            ("Free-pop happiness", (p.provincial_happiness - 50.0) * 0.18),
            ("Provincial trade", 5.0 * diminishing(p.provincial_trade, 40.0)),
            ("High tribute", -12.0 * p.high_tribute),
        ],
        Bloc::Populares => vec![
            ("Citizen happiness", (p.citizen_happiness - 50.0) * 0.24),
            ("Plebeian happiness", (p.plebeian_happiness - 50.0) * 0.16),
            ("Food policy", p.food_policy * 4.0),
            ("Food security", (p.food_security - 0.8) * 15.0),
            ("Active famine", -18.0 * p.famine),
            ("Tax pressure", -8.0 * p.tax_pressure),
            ("Harsh domestic policies", -7.0 * p.harsh_policies),
        ],
        Bloc::Military => vec![
            ("Effective military strength", 10.0 * diminishing(p.military_strength, 500.0)),
            ("Recent victories and defeats", p.recent_victories.clamp(-4.0, 4.0) * 2.0),
            ("Military rank", p.military_rank.clamp(0.0, 3.0) * 3.0),
        ],
    };
    rows.into_iter()
        .map(|(label, points)| SupportReason {
            label,
            points,
        })
        .collect()
}

#[cfg(test)]
#[path = "../../../tests/unit/senate.rs"]
mod tests;
