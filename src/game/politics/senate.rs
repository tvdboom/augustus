//! One shared sealed nomination auction, one campaign and one authoritative Senate vote.

use super::{Currency, PlayerId, PoliticalError, PoliticalPlayer, PoliticalRank, PoliticalRng};
use std::collections::{BTreeMap, BTreeSet};

/// Five public Senate blocs, with no persistent individual senator simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bloc {
    /// Nobility, prestige and political institutions.
    Aristocrats,
    /// Commerce, reliability and economic security.
    Merchants,
    /// Provincial welfare and non-coercive vassal stability.
    Provincials,
    /// Citizens, plebeians, food and domestic policies.
    Populares,
    /// Military strength, service and victories.
    Military,
}

impl Bloc {
    /// Stable order for profiles, scores, configuration and chamber seats.
    pub const ALL: [Self; 5] =
        [Self::Aristocrats, Self::Merchants, Self::Provincials, Self::Populares, Self::Military];
    /// Index into five-bloc arrays.
    pub fn index(self) -> usize {
        self as usize
    }
    /// Display label for tabs and tooltips.
    pub fn label(self) -> &'static str {
        match self {
            Self::Aristocrats => "Aristocrats",
            Self::Merchants => "Merchants",
            Self::Provincials => "Provincials",
            Self::Populares => "Populares",
            Self::Military => "Military",
        }
    }
}

/// Requested motion; all offices compete in the same auction without priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ballot {
    /// Promote an Aedile.
    Praetor,
    /// Promote a Praetor through the same nomination and voting mechanics.
    Censor,
    /// Elect a Censor or former Consul to an available seat.
    Consul,
    /// Elect a currently serving Consul to victory.
    Augustus,
    /// Remove the specified active Consul using evidence owned by the initiator.
    NoConfidence {
        /// Incumbent whose office is challenged.
        target: PlayerId,
        /// Evidence reserved for and consumed by the motion.
        scandal_id: u64,
    },
}

impl Ballot {
    /// Player-facing name of the vote.
    pub fn label(self) -> &'static str {
        match self {
            Self::Praetor => "Praetor",
            Self::Censor => "Censor",
            Self::Consul => "Consul",
            Self::Augustus => "Augustus",
            Self::NoConfidence {
                ..
            } => "No Confidence",
        }
    }
}

/// Central Senate costs, timing and support-to-vote conversion.
#[derive(Debug, Clone)]
pub struct SenateConfig {
    /// Aedile is bought without a ballot.
    pub aedile_cost: f64,
    /// Minimum Praetor, Censor, Consul, Augustus and removal nominations.
    pub minimum_bids: [f64; 5],
    /// Quaestor, Aedile, Praetor, Censor, Consul, Augustus income; Proconsul uses Consul.
    pub rank_influence: [f64; 6],
    /// Number of sealed monthly rounds before selecting a nominee.
    pub nomination_months: u32,
    /// Campaign duration after a unique nomination winner emerges.
    pub campaign_months: u32,
    /// Serving Consul term duration in months.
    pub consul_term: u32,
    /// Exact 100-vote chamber split by bloc.
    pub bloc_sizes: [u8; 5],
    /// Number of YES votes required to pass.
    pub yes_threshold: u8,
    /// Influence spending at half of its maximum campaign support.
    pub campaign_half_saturation: f64,
    /// Maximum probability contribution from each side's campaign.
    pub campaign_cap: f64,
    /// Coin price equivalent to one Influence in bribery's support formula.
    pub bribery_coin_per_influence: f64,
    /// Probability floor and ceiling prevent absolute certainty from raw empire size.
    pub probability_bounds: [f64; 2],
    /// Neutral Senate support before the explainable structural/active contributions.
    pub baseline_support: f64,
    /// Multipliers for each bloc's listed structural factors, in tooltip order.
    pub structural_weights: [[f64; 7]; 5],
    /// Certainty base, support-distance weight, campaign-engagement weight, and cap.
    pub certainty_curve: [f64; 4],
    /// Default button increment for legitimate campaigning and endorsements.
    pub campaign_action_influence: f64,
    /// Default button increment for scandalous coin bribery.
    pub bribery_action_coin: f64,
}

impl Default for SenateConfig {
    /// Initial configuration preserves the Censor between the spec's Praetor and Consul.
    fn default() -> Self {
        Self {
            aedile_cost: 100.0,
            minimum_bids: [100.0, 150.0, 200.0, 400.0, 200.0],
            rank_influence: [0.0, 1.0, 2.0, 3.0, 4.0, 0.0],
            nomination_months: 12,
            campaign_months: 12,
            consul_term: 48,
            bloc_sizes: [30, 20, 20, 20, 10],
            yes_threshold: 51,
            campaign_half_saturation: 60.0,
            campaign_cap: 0.25,
            bribery_coin_per_influence: 5.0,
            probability_bounds: [0.02, 0.98],
            baseline_support: 0.45,
            structural_weights: [[1.0; 7]; 5],
            certainty_curve: [0.4, 0.8, 0.25, 0.95],
            campaign_action_influence: 10.0,
            bribery_action_coin: 50.0,
        }
    }
}

impl SenateConfig {
    /// Enforce the invariant of exactly 100 seats before the simulation starts.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.bloc_sizes.iter().map(|v| *v as u16).sum::<u16>() != 100 {
            return Err("The Senate must contain exactly 100 votes.");
        }
        if self.nomination_months == 0 || self.campaign_months == 0 || self.consul_term == 0 {
            return Err("Political durations must be positive.");
        }
        if self.yes_threshold != 51 {
            return Err("The Senate requires an absolute majority of 51 votes.");
        }
        Ok(())
    }
    /// Nomination minimum shared by model and UI.
    pub fn minimum_bid(&self, ballot: Ballot) -> f64 {
        self.minimum_bids[match ballot {
            Ballot::Praetor => 0,
            Ballot::Censor => 1,
            Ballot::Consul => 2,
            Ballot::Augustus => 3,
            Ballot::NoConfidence {
                ..
            } => 4,
        }]
    }
    /// Non-stacking monthly office income; callers add it during normal economy income.
    pub fn rank_income(&self, rank: PoliticalRank) -> f64 {
        self.rank_influence[rank.ladder_index()]
    }
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

/// A bloc's visible uncertainty and final underlying probability.
#[derive(Debug, Clone)]
pub struct BlocProjection {
    /// Senate faction represented.
    pub bloc: Bloc,
    /// Seats fixed to YES before the final vote.
    pub yes: u8,
    /// Seats fixed to NO before the final vote.
    pub no: u8,
    /// Gray seats independently resolved at the final probability.
    pub undecided: u8,
    /// Chance that each undecided seat will choose YES.
    pub support_probability: f64,
    /// Neutral support before the listed modifiers, for faithful tooltips.
    pub baseline_support: f64,
    /// Structural and active political effects for tooltips.
    pub reasons: Vec<SupportReason>,
}

/// Public totals from resolved sealed nomination rounds.
#[derive(Debug, Clone)]
pub struct Nomination {
    /// Player requesting the next ballot.
    pub player: PlayerId,
    /// Office or motion requested.
    pub ballot: Ballot,
    /// Permanently spent, publicly revealed Influence.
    pub committed: f64,
}

/// Active campaign; spending has diminishing returns and the empire profile stays live.
#[derive(Debug, Clone)]
pub struct Campaign {
    /// Player who won access to the single ballot.
    pub candidate: PlayerId,
    /// Promotion or removal motion.
    pub ballot: Ballot,
    /// Completed months of the campaign.
    pub elapsed: u32,
    /// Candidate and endorsement Influence by bloc.
    pub support_spending: [f64; 5],
    /// Opponents' Influence by bloc.
    pub opposition_spending: [f64; 5],
    /// Coin bribery, reported separately because it creates evidence.
    pub bribery: [f64; 5],
    /// Accumulated exposed-scandal probability penalties.
    pub scandal_penalties: [f64; 5],
}

/// Authoritative saved results; the UI animates these instead of rerolling votes.
#[derive(Debug, Clone)]
pub struct SenateResult {
    /// Absolute resolution month.
    pub month: u32,
    /// Player who sought the ballot.
    pub candidate: PlayerId,
    /// Requested office or motion.
    pub ballot: Ballot,
    /// Final YES count.
    pub yes: u8,
    /// Final NO count.
    pub no: u8,
    /// Whether the majority and final eligibility checks both passed.
    pub passed: bool,
    /// Stable bloc-order sequence of all 100 final votes.
    pub votes: Vec<bool>,
    /// Identifies circles that were gray and should animate.
    pub was_undecided: Vec<bool>,
}

/// Model events passed to toasts and evidence inventory without UI coupling.
#[derive(Debug, Clone)]
pub enum SenateEvent {
    /// Unique winner begins their campaign.
    CampaignStarted(PlayerId, Ballot),
    /// A tied final auction needs another sealed month.
    SuddenDeath(Vec<PlayerId>),
    /// Senate reached its authoritative result.
    VoteResolved(SenateResult),
    /// Office ended and a seat became vacant.
    ConsulExpired(PlayerId),
    /// Evidence is consumed at the end of a No Confidence campaign, whether it wins or loses.
    ConsumeScandal(u64),
    /// Elected Augustus; the session can enter its victory state.
    Victory(PlayerId),
}

/// Shared Roman political calendar. Private bids are intentionally not public fields.
#[derive(Debug, Clone)]
pub struct SenateState {
    /// Absolute number of resolved months.
    pub month: u32,
    /// Number of nomination months already completed.
    pub nomination_elapsed: u32,
    /// Public cumulative, already revealed nominations.
    pub nominations: BTreeMap<PlayerId, Nomination>,
    /// None during nomination; exactly one shared campaign otherwise.
    pub campaign: Option<Campaign>,
    /// Most recent authoritative outcome.
    pub last_result: Option<SenateResult>,
    /// Winning player, set only through an Augustus vote.
    pub winner: Option<PlayerId>,
    /// When tied, only these players may bid in the additional sealed rounds.
    pub sudden_death: BTreeSet<PlayerId>,
    pending_bids: BTreeMap<PlayerId, (Ballot, f64)>,
    rng: PoliticalRng,
}

impl SenateState {
    /// Initialize a match-specific random stream and the first Nomination Year.
    pub fn new(seed: u64) -> Self {
        Self {
            month: 0,
            nomination_elapsed: 0,
            nominations: BTreeMap::new(),
            campaign: None,
            last_result: None,
            winner: None,
            sudden_death: BTreeSet::new(),
            pending_bids: BTreeMap::new(),
            rng: PoliticalRng::new(seed),
        }
    }

    /// Fixed-cost first promotion has no effect on the shared Senate calendar.
    pub fn buy_aedile(
        &self,
        player: &mut PoliticalPlayer,
        config: &SenateConfig,
    ) -> Result<(), PoliticalError> {
        if player.rank != PoliticalRank::Quaestor {
            return Err(PoliticalError::Ineligible);
        }
        player.spend(Currency::Influence, config.aedile_cost)?;
        player.rank = PoliticalRank::Aedile;
        Ok(())
    }

    /// Expected ballot month; sudden-death months extend the schedule explicitly.
    pub fn expected_vote_month(&self, config: &SenateConfig) -> u32 {
        self.month
            + self.campaign.as_ref().map_or(
                config
                    .nomination_months
                    .saturating_sub(self.nomination_elapsed)
                    .max(u32::from(!self.sudden_death.is_empty()))
                    + config.campaign_months,
                |campaign| config.campaign_months.saturating_sub(campaign.elapsed),
            )
    }

    /// Check rank, term and seat restrictions without spending or changing state.
    pub fn eligibility(
        &self,
        player: PlayerId,
        ballot: Ballot,
        players: &[PoliticalPlayer],
        has_scandal: bool,
        config: &SenateConfig,
    ) -> Result<(), PoliticalError> {
        let actor = players.get(player).ok_or(PoliticalError::MissingTarget)?;
        let vote_month = self.expected_vote_month(config);
        match ballot {
            Ballot::Praetor if actor.rank == PoliticalRank::Aedile => Ok(()),
            Ballot::Censor if actor.rank == PoliticalRank::Praetor => Ok(()),
            Ballot::Consul
                if matches!(actor.rank, PoliticalRank::Censor | PoliticalRank::Proconsul) =>
            {
                if players
                    .iter()
                    .filter(|p| {
                        p.rank == PoliticalRank::Consul
                            && p.consul_until.is_some_and(|until| until > vote_month)
                    })
                    .count()
                    < 2
                {
                    Ok(())
                } else {
                    Err(PoliticalError::NoConsulSeat)
                }
            },
            Ballot::Augustus
                if actor.rank == PoliticalRank::Consul
                    && actor.consul_until.is_some_and(|until| until >= vote_month) =>
            {
                Ok(())
            },
            Ballot::NoConfidence {
                target,
                ..
            } if target != player
                && players.get(target).is_some_and(|p| {
                    p.rank == PoliticalRank::Consul
                        && p.consul_until.is_some_and(|until| until >= vote_month)
                }) =>
            {
                if has_scandal {
                    Ok(())
                } else {
                    Err(PoliticalError::ScandalRequired)
                }
            },
            _ => Err(PoliticalError::Ineligible),
        }
    }

    /// Escrow a sealed additional bid. Funds become unavailable immediately, but the
    /// additional amount is revealed and committed only at the simultaneous monthly step.
    /// `has_scandal` is supplied by the authoritative evidence inventory, never by the UI.
    pub fn submit_bid(
        &mut self,
        player: PlayerId,
        ballot: Ballot,
        amount: f64,
        players: &mut [PoliticalPlayer],
        has_scandal: bool,
        config: &SenateConfig,
    ) -> Result<(), PoliticalError> {
        self.bid_eligibility(player, ballot, amount, players, has_scandal, config)?;
        players[player].spend(Currency::Influence, amount)?;
        self.pending_bids
            .entry(player)
            .and_modify(|entry| entry.1 += amount)
            .or_insert((ballot, amount));
        Ok(())
    }

    /// Validate the complete bid, including a prior sealed choice and sudden-death
    /// participation, without spending funds or revealing another player's bid.
    pub fn bid_eligibility(
        &self,
        player: PlayerId,
        ballot: Ballot,
        amount: f64,
        players: &[PoliticalPlayer],
        has_scandal: bool,
        config: &SenateConfig,
    ) -> Result<(), PoliticalError> {
        if self.campaign.is_some() || self.winner.is_some() {
            return Err(PoliticalError::WrongPhase);
        }
        if !amount.is_finite() || amount <= 0.0 {
            return Err(PoliticalError::InvalidAmount);
        }
        if !self.sudden_death.is_empty() && !self.sudden_death.contains(&player) {
            return Err(PoliticalError::Ineligible);
        }
        self.eligibility(player, ballot, players, has_scandal, config)?;
        if self.nominations.get(&player).is_some_and(|n| n.ballot != ballot)
            || self.pending_bids.get(&player).is_some_and(|(b, _)| *b != ballot)
        {
            return Err(PoliticalError::Ineligible);
        }
        let committed = self.nominations.get(&player).map_or(0.0, |n| n.committed);
        let pending = self.pending_bids.get(&player).map_or(0.0, |(_, value)| *value);
        if committed + pending + amount < config.minimum_bid(ballot) {
            return Err(PoliticalError::InvalidAmount);
        }
        if players[player].influence + 1e-9 < amount {
            return Err(PoliticalError::InsufficientFunds);
        }
        Ok(())
    }

    /// Read only the viewing player's private additional commitment.
    pub fn private_bid(&self, viewer: PlayerId) -> f64 {
        self.pending_bids.get(&viewer).map_or(0.0, |(_, amount)| *amount)
    }

    /// Return only this viewer's already chosen motion, including an unrevealed bid.
    pub fn chosen_ballot(&self, viewer: PlayerId) -> Option<Ballot> {
        self.nominations
            .get(&viewer)
            .map(|n| n.ballot)
            .or_else(|| self.pending_bids.get(&viewer).map(|(ballot, _)| *ballot))
    }

    /// Inspect current structural attitudes before nomination without starting a
    /// campaign, committing seats, modifying bids, or advancing the random stream.
    pub fn preview_projection(
        &self,
        candidate: PlayerId,
        ballot: Ballot,
        players: &[PoliticalPlayer],
        profiles: &[PoliticalProfile],
        config: &SenateConfig,
    ) -> Vec<BlocProjection> {
        let mut preview = self.clone();
        preview.campaign = Some(Campaign {
            candidate,
            ballot,
            elapsed: 0,
            support_spending: [0.0; 5],
            opposition_spending: [0.0; 5],
            bribery: [0.0; 5],
            scandal_penalties: [0.0; 5],
        });
        preview.projection(players, profiles, config)
    }

    /// Spend Influence to support or oppose a bloc. Candidates cannot oppose their own promotion.
    pub fn campaign_spend(
        &mut self,
        player: PlayerId,
        bloc: Bloc,
        support: bool,
        amount: f64,
        players: &mut [PoliticalPlayer],
    ) -> Result<(), PoliticalError> {
        let campaign = self.campaign.as_mut().ok_or(PoliticalError::WrongPhase)?;
        if player == campaign.candidate
            && !support
            && !matches!(campaign.ballot, Ballot::NoConfidence { .. })
        {
            return Err(PoliticalError::Ineligible);
        }
        players
            .get_mut(player)
            .ok_or(PoliticalError::MissingTarget)?
            .spend(Currency::Influence, amount)?;
        if support {
            campaign.support_spending[bloc.index()] += amount;
        } else {
            campaign.opposition_spending[bloc.index()] += amount;
        }
        Ok(())
    }

    /// Spend candidate coin for a stronger campaign; caller records Political Bribery
    /// as an actual scandal opportunity whenever this succeeds with a positive amount.
    pub fn bribe(
        &mut self,
        player: PlayerId,
        bloc: Bloc,
        amount: f64,
        players: &mut [PoliticalPlayer],
    ) -> Result<(), PoliticalError> {
        let campaign = self.campaign.as_mut().ok_or(PoliticalError::WrongPhase)?;
        if player != campaign.candidate {
            return Err(PoliticalError::Ineligible);
        }
        players
            .get_mut(player)
            .ok_or(PoliticalError::MissingTarget)?
            .spend(Currency::Coin, amount)?;
        campaign.bribery[bloc.index()] += amount;
        Ok(())
    }

    /// Add already validated evidence effects, differentiated by bloc and severity.
    pub fn expose_scandal(
        &mut self,
        target: PlayerId,
        penalties: [f64; 5],
    ) -> Result<(), PoliticalError> {
        let campaign = self.campaign.as_mut().ok_or(PoliticalError::WrongPhase)?;
        let expected = match campaign.ballot {
            Ballot::NoConfidence {
                target,
                ..
            } => target,
            _ => campaign.candidate,
        };
        if target != expected {
            return Err(PoliticalError::Ineligible);
        }
        for (current, penalty) in campaign.scandal_penalties.iter_mut().zip(penalties) {
            *current += penalty.max(0.0);
        }
        Ok(())
    }

    /// Explain live support and visible uncertainty from the current campaign/profile.
    pub fn projection(
        &self,
        players: &[PoliticalPlayer],
        profiles: &[PoliticalProfile],
        config: &SenateConfig,
    ) -> Vec<BlocProjection> {
        let Some(campaign) = &self.campaign else {
            return Vec::new();
        };
        let profile_id = match campaign.ballot {
            Ballot::NoConfidence {
                target,
                ..
            } => target,
            _ => campaign.candidate,
        };
        let fallback = PoliticalProfile::default();
        let profile = profiles.get(profile_id).unwrap_or(&fallback);
        let player = players.get(profile_id).cloned().unwrap_or_default();
        Bloc::ALL
            .into_iter()
            .map(|bloc| {
                let i = bloc.index();
                let mut reasons = structural_reasons(bloc, profile, &player);
                for (reason, weight) in reasons.iter_mut().zip(config.structural_weights[i]) {
                    reason.points *= weight;
                }
                if matches!(campaign.ballot, Ballot::NoConfidence { .. }) {
                    for reason in &mut reasons {
                        reason.points = -reason.points;
                    }
                }
                let support =
                    diminishing(campaign.support_spending[i], config.campaign_half_saturation)
                        * config.campaign_cap;
                let opposition =
                    diminishing(campaign.opposition_spending[i], config.campaign_half_saturation)
                        * config.campaign_cap;
                let bribery = diminishing(
                    campaign.bribery[i] / config.bribery_coin_per_influence.max(0.01),
                    config.campaign_half_saturation,
                ) * config.campaign_cap;
                let scandal_sign = if matches!(campaign.ballot, Ballot::NoConfidence { .. }) {
                    1.0
                } else {
                    -1.0
                };
                reasons.extend([
                    SupportReason {
                        label: "Campaign and endorsements",
                        points: support * 100.0,
                    },
                    SupportReason {
                        label: "Opposition campaign",
                        points: -opposition * 100.0,
                    },
                    SupportReason {
                        label: "Coin bribery",
                        points: bribery * 100.0,
                    },
                    SupportReason {
                        label: "Exposed scandals",
                        points: campaign.scandal_penalties[i] * scandal_sign * 100.0,
                    },
                ]);
                let probability = (config.baseline_support
                    + reasons.iter().map(|r| r.points / 100.0).sum::<f64>())
                .clamp(config.probability_bounds[0], config.probability_bounds[1]);
                let engagement = diminishing(
                    campaign.support_spending[i]
                        + campaign.opposition_spending[i]
                        + campaign.bribery[i] / config.bribery_coin_per_influence.max(0.01),
                    config.campaign_half_saturation,
                );
                let certainty = (config.certainty_curve[0]
                    + (probability - 0.5).abs() * config.certainty_curve[1]
                    + engagement * config.certainty_curve[2])
                    .clamp(0.0, config.certainty_curve[3]);
                let size = config.bloc_sizes[i];
                let committed = (f64::from(size) * certainty).round() as u8;
                let yes = (f64::from(committed) * probability).round() as u8;
                BlocProjection {
                    bloc,
                    yes,
                    no: committed - yes,
                    undecided: size - committed,
                    support_probability: probability,
                    baseline_support: config.baseline_support,
                    reasons,
                }
            })
            .collect()
    }

    /// Advance the single shared calendar once after the month's economic/political
    /// changes. Undecided votes are rolled here once and saved for presentation.
    pub fn advance_month(
        &mut self,
        players: &mut [PoliticalPlayer],
        profiles: &[PoliticalProfile],
        config: &SenateConfig,
    ) -> Vec<SenateEvent> {
        assert!(config.validate().is_ok(), "invalid Senate configuration");
        if self.winner.is_some() {
            return Vec::new();
        }
        self.month += 1;
        let mut events = Vec::new();
        if self.campaign.is_some() {
            self.campaign.as_mut().expect("campaign present").elapsed += 1;
            if self.campaign.as_ref().is_some_and(|c| c.elapsed >= config.campaign_months) {
                let projection = self.projection(players, profiles, config);
                let campaign = self.campaign.take().expect("campaign present");
                let mut votes = Vec::with_capacity(100);
                let mut was_undecided = Vec::with_capacity(100);
                for bloc in projection {
                    for _ in 0..bloc.yes {
                        votes.push(true);
                        was_undecided.push(false);
                    }
                    for _ in 0..bloc.no {
                        votes.push(false);
                        was_undecided.push(false);
                    }
                    for _ in 0..bloc.undecided {
                        votes.push(self.rng.unit() < bloc.support_probability);
                        was_undecided.push(true);
                    }
                }
                let yes = votes.iter().filter(|yes| **yes).count() as u8;
                let passed =
                    yes >= config.yes_threshold && self.apply_result(&campaign, players, config);
                let result = SenateResult {
                    month: self.month,
                    candidate: campaign.candidate,
                    ballot: campaign.ballot,
                    yes,
                    no: 100 - yes,
                    passed,
                    votes,
                    was_undecided,
                };
                if let Ballot::NoConfidence {
                    scandal_id,
                    ..
                } = campaign.ballot
                {
                    events.push(SenateEvent::ConsumeScandal(scandal_id));
                }
                if passed && campaign.ballot == Ballot::Augustus {
                    self.winner = Some(campaign.candidate);
                    events.push(SenateEvent::Victory(campaign.candidate));
                }
                self.last_result = Some(result.clone());
                events.push(SenateEvent::VoteResolved(result));
                self.nomination_elapsed = 0;
                self.nominations.clear();
                self.sudden_death.clear();
            }
        } else {
            self.nomination_elapsed += 1;
            for (player, (ballot, amount)) in std::mem::take(&mut self.pending_bids) {
                self.nominations.entry(player).and_modify(|n| n.committed += amount).or_insert(
                    Nomination {
                        player,
                        ballot,
                        committed: amount,
                    },
                );
            }
            if self.nomination_elapsed >= config.nomination_months {
                let best = self.nominations.values().map(|n| n.committed).fold(0.0, f64::max);
                let leaders: Vec<_> = self
                    .nominations
                    .values()
                    .filter(|n| (n.committed - best).abs() < 1e-9)
                    .cloned()
                    .collect();
                match leaders.as_slice() {
                    [] => {
                        self.nomination_elapsed = 0;
                    },
                    [leader] => {
                        self.campaign = Some(Campaign {
                            candidate: leader.player,
                            ballot: leader.ballot,
                            elapsed: 0,
                            support_spending: [0.0; 5],
                            opposition_spending: [0.0; 5],
                            bribery: [0.0; 5],
                            scandal_penalties: [0.0; 5],
                        });
                        self.sudden_death.clear();
                        events.push(SenateEvent::CampaignStarted(leader.player, leader.ballot));
                    },
                    _ => {
                        self.sudden_death = leaders.iter().map(|n| n.player).collect();
                        events.push(SenateEvent::SuddenDeath(
                            self.sudden_death.iter().copied().collect(),
                        ));
                    },
                }
            }
        }
        // Augustus gets its same-month ballot before the Consul term expires.
        for (id, player) in players.iter_mut().enumerate() {
            if player.rank == PoliticalRank::Consul
                && player.consul_until.is_some_and(|until| until <= self.month)
            {
                player.rank = PoliticalRank::Proconsul;
                player.consul_until = None;
                events.push(SenateEvent::ConsulExpired(id));
            }
        }
        events
    }

    /// Recheck final eligibility and apply a passed motion; never create a third Consul.
    fn apply_result(
        &self,
        campaign: &Campaign,
        players: &mut [PoliticalPlayer],
        config: &SenateConfig,
    ) -> bool {
        let Some(actor) = players.get(campaign.candidate) else {
            return false;
        };
        let valid = match campaign.ballot {
            Ballot::Praetor => actor.rank == PoliticalRank::Aedile,
            Ballot::Censor => actor.rank == PoliticalRank::Praetor,
            Ballot::Consul => {
                matches!(actor.rank, PoliticalRank::Censor | PoliticalRank::Proconsul)
                    && players
                        .iter()
                        .filter(|p| {
                            p.rank == PoliticalRank::Consul
                                && p.consul_until.is_some_and(|until| until > self.month)
                        })
                        .count()
                        < 2
            },
            Ballot::Augustus => {
                actor.rank == PoliticalRank::Consul
                    && actor.consul_until.is_some_and(|until| until >= self.month)
            },
            Ballot::NoConfidence {
                target,
                ..
            } => players.get(target).is_some_and(|p| p.rank == PoliticalRank::Consul),
        };
        if !valid {
            return false;
        }
        match campaign.ballot {
            Ballot::Praetor => players[campaign.candidate].rank = PoliticalRank::Praetor,
            Ballot::Censor => players[campaign.candidate].rank = PoliticalRank::Censor,
            Ballot::Consul => {
                // Expiring incumbents vacate before installing a newly elected Consul.
                for player in players.iter_mut() {
                    if player.rank == PoliticalRank::Consul
                        && player.consul_until.is_some_and(|until| until <= self.month)
                    {
                        player.rank = PoliticalRank::Proconsul;
                        player.consul_until = None;
                    }
                }
                players[campaign.candidate].rank = PoliticalRank::Consul;
                players[campaign.candidate].consul_until = Some(self.month + config.consul_term);
            },
            Ballot::Augustus => {
                players[campaign.candidate].rank = PoliticalRank::Augustus;
                players[campaign.candidate].consul_until = None;
            },
            Ballot::NoConfidence {
                target,
                ..
            } => {
                players[target].rank = PoliticalRank::Proconsul;
                players[target].consul_until = None;
            },
        }
        true
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
            ("Noble population (diminishing)", 8.0 * diminishing(p.nobles.sqrt(), 8.0)),
            ("Noble happiness", (p.noble_happiness - 50.0) * 0.16),
            ("Political office", actor.rank.ladder_index() as f64 * 1.5),
            ("Influence prestige", 4.0 * diminishing(actor.influence, 200.0)),
            ("Completed wonders", 5.0 * diminishing(p.wonders, 2.0)),
            ("Political city buildings", 4.0 * diminishing(p.political_buildings, 4.0)),
        ],
        Bloc::Merchants => vec![
            ("Coin income", 8.0 * diminishing(p.coin_income, 30.0)),
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
