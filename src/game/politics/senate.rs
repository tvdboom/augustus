//! Accumulating, contested faction confidence and actions on individual Senate seats.
use super::{Currency, PlayerId, PoliticalError, PoliticalPlayer, PoliticalRank, PoliticalRng};

/// Public factions with distinct preferences and contiguous chamber sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bloc {
    /// Nobles and republican institutions.
    Aristocrats,
    /// Commerce and economic security.
    Merchants,
    /// Vassal reach, provincial relations and rural trade.
    Provincials,
    /// Citizens, plebeians and domestic policy.
    Populares,
    /// Imperialists favor territorial control, army strength, service and victories.
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
            Self::Military => "Imperialists",
        }
    }
    /// Short explanation of the faction's actual structural preferences.
    pub fn preferences(self) -> &'static str {
        match self {
            Self::Aristocrats => "Happy nobles, larger happy noble populations, higher political rank, Forums and wonders build confidence each month.",
            Self::Merchants => "Positive net monthly coin income, fulfilled delivered trade and Markets build confidence. Larger incomes build confidence faster; shortages and wars weaken it.",
            Self::Provincials => "More vassals, good relations with other provinces and delivered trade with provinces without cities. High tribute and wars drive them away.",
            Self::Populares => "Happy citizens and plebeians, ample food reserves and supplied generous rations build confidence. Normal rations alone are neutral; shortages, taxes and harsh labor drive them away.",
            Self::Military => "Strong trained armies, higher military career rank, victories and control beyond the first province build confidence. Defeats weaken support.",
        }
    }
}

/// Costs and loyalty rules; there is no election calendar.
#[derive(Debug, Clone)]
pub struct SenateConfig {
    /// Aedile, Praetor, Censor, Consul and Augustus appointment costs.
    pub promotion_costs: [f64; 5],
    /// Non-stacking monthly office income; Proconsul shares the Consul slot, Augustus earns none.
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
    /// Monthly confidence from non-stacking faction outreach.
    pub court_bonus: f64,
    /// Lifetime of monthly outreach.
    pub court_months: u32,
    /// Lifetime of public scrutiny after a scandal's one-time confidence loss.
    pub scandal_months: u32,
    /// Confidence needed to fill one seat. Confidence is internal, never displayed.
    pub seat_confidence: f64,
    /// Costs in SenatorAction order; SenatorAction::currency selects the wallet.
    pub action_costs: [f64; 9],
    /// Detection probabilities in SenatorAction order.
    pub action_risks: [f64; 9],
    /// Influence paid each month while lobbying an individual senator.
    pub senator_outreach_upkeep: f64,
    /// Coin paid each month to maintain a senator's bribe.
    pub senator_bribe_upkeep: f64,
    /// Personal confidence that fades each month, without erasing structural support.
    pub personal_confidence_decay: f64,
}
impl Default for SenateConfig {
    fn default() -> Self {
        Self {
            promotion_costs: [500.0, 1000.0, 1500.0, 2000.0, 3000.0],
            rank_influence: [0.0, 5.0, 10.0, 15.0, 20.0, 0.0],
            bloc_sizes: [20; 5],
            consul_term: 24,
            consul_cooldown: 12,
            loss_grace_months: 3,
            court_cost: 20.0,
            court_bonus: 1.0,
            court_months: 6,
            scandal_months: 12,
            seat_confidence: 10.0,
            action_costs: [10.0, 40.0, 25.0, 80.0, 50.0, 180.0, 70.0, 35.0, 20.0],
            action_risks: [0.0, 0.0, 0.0, 0.20, 0.30, 0.40, 0.0, 0.20, 0.0],
            senator_outreach_upkeep: 4.0,
            senator_bribe_upkeep: 8.0,
            personal_confidence_decay: 0.5,
        }
    }
}
impl SenateConfig {
    /// Enforce chamber, duration and cost invariants before simulation.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.bloc_sizes.contains(&0)
            || self.bloc_sizes.iter().map(|n| usize::from(*n)).sum::<usize>() != 100
        {
            return Err("The Senate must contain exactly 100 senators.");
        }
        if !self.seat_confidence.is_finite()
            || self.seat_confidence <= 0.0
            || self.action_costs.iter().any(|n| !n.is_finite() || *n <= 0.0)
            || self.action_risks.iter().any(|n| !n.is_finite() || !(0.0..=1.0).contains(n))
        {
            return Err("Senator confidence, costs and risks must be valid.");
        }
        if [
            self.consul_term,
            self.consul_cooldown,
            self.loss_grace_months,
            self.court_months,
            self.scandal_months,
        ]
        .contains(&0)
        {
            return Err("Political durations must be positive.");
        }
        if self
            .promotion_costs
            .iter()
            .chain([
                &self.court_cost,
                &self.court_bonus,
                &self.senator_outreach_upkeep,
                &self.senator_bribe_upkeep,
                &self.personal_confidence_decay,
            ])
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
        let senators = (([10.0, 20.0, 30.0, 40.0, 60.0][i] * scale).round() as usize).max(
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
    /// Settled monthly net coin income after wages, upkeep and other monthly costs.
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
    /// Number of provinces that are this player's vassals.
    pub vassal_count: f64,
    /// Mean relation with provinces outside direct ownership, excluding Rome; neutral 50 when none.
    pub province_relation: f64,
    /// Delivered recurring trade value with provinces without cities.
    pub provincial_trade: f64,
    /// Share of vassals using high tribute, 0..1.
    pub high_tribute: f64,
    /// Average food-supply policy: -1 low, 0 normal, +1 high.
    pub food_policy: f64,
    /// Fraction of monthly food requested actually supplied, 0..1.
    pub food_security: f64,
    /// Stored food divided by monthly civilian and army demand; zero without owned residents.
    pub food_reserve_months: f64,
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
    /// Active, fulfilled recurring routes; each adds half a point per month.
    pub active_trade_routes: f64,
    /// Declared wars, including independent provinces.
    pub active_wars: f64,
    /// Province-equivalents of territorial control, including fractional vassal/independent Control.
    pub controlled_provinces: f64,
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
            vassal_count: 0.0,
            province_relation: 50.0,
            provincial_trade: 0.0,
            high_tribute: 0.0,
            food_policy: 0.0,
            food_security: 1.0,
            food_reserve_months: 0.0,
            famine: 0.0,
            tax_pressure: 0.0,
            harsh_policies: 0.0,
            military_strength: 0.0,
            recent_victories: 0.0,
            military_rank: 0.0,
            active_trade_routes: 0.0,
            active_wars: 0.0,
            controlled_provinces: 0.0,
        }
    }
}

/// A recurring confidence modifier. The UI shows its direction, never its value.
#[derive(Debug, Clone)]
pub struct SupportReason {
    /// Concise explanation of the driver.
    pub label: &'static str,
    /// Signed monthly confidence contribution.
    pub points: f64,
}

/// One stable seat with shared confidence capacity, initially entirely neutral.
#[derive(Debug, Clone)]
pub struct Senator {
    /// Stable chamber identity, never changed during a match.
    pub id: usize,
    /// Faction determining structural preferences and chamber section.
    pub bloc: Bloc,
    /// Player currently supported, or neutral gray.
    pub allegiance: Option<PlayerId>,
    /// Changes when a murdered senator is replaced, while the seat ID stays stable.
    pub generation: u32,
    confidence: Vec<f64>,
    personal_confidence: Vec<f64>,
    arrangements: Vec<SenatorArrangement>,
    effects: Vec<SenatorEffect>,
}
/// A purchased, temporary effect on one senator; these can coexist with ongoing actions.
#[derive(Debug, Clone, Copy)]
struct SenatorEffect {
    player: PlayerId,
    action: SenatorAction,
    target: PlayerId,
    until: u32,
}
impl Senator {
    /// The viewer's own ongoing action; other players' actions stay private in the UI.
    pub fn arrangement(&self, player: PlayerId) -> Option<SenatorArrangement> {
        self.arrangements.iter().find(|a| a.player == player).copied()
    }
}
#[derive(Debug, Clone, Copy)]
/// A non-stacking personal relationship that adds confidence each month.
pub struct SenatorArrangement {
    /// Player who initiated the relationship.
    pub player: PlayerId,
    /// Patronage, coercion, paid bribery or lobbying.
    pub action: SenatorAction,
    /// Exclusive expiry month.
    pub until: u32,
    /// Paid actions grant confidence only after this month's upkeep was paid.
    pub paid_at: Option<u32>,
}
/// All options available from an individual seat's action panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SenatorAction {
    /// A temporary personal appeal paid with Influence.
    Petition,
    /// A temporary goodwill bonus paid with coin.
    Gift,
    /// A fixed six-month relationship paid with Influence.
    Patronage,
    /// Faster recurring confidence funded with coin, with a risk of exposure.
    Bribe,
    /// Six months of coercive confidence gains with political downsides.
    Threaten,
    /// Kill and replace the member, clearing all confidence in the seat.
    Assassinate,
    /// A public feast building goodwill with this senator over time.
    Banquet,
    /// Weaken this senator's rival patron.
    Discredit,
    /// Gradual lobbying funded by upfront and monthly Influence.
    Lobby,
}
impl SenatorAction {
    /// Stable ordering shared by prices, risks and the individual action panel.
    pub const ALL: [Self; 9] = [
        Self::Petition,
        Self::Gift,
        Self::Patronage,
        Self::Bribe,
        Self::Threaten,
        Self::Assassinate,
        Self::Banquet,
        Self::Discredit,
        Self::Lobby,
    ];
    /// Public action name.
    pub fn label(self) -> &'static str {
        match self {
            Self::Petition => "Petition",
            Self::Gift => "Send a gift",
            Self::Patronage => "Offer patronage",
            Self::Bribe => "Bribe",
            Self::Threaten => "Threaten",
            Self::Assassinate => "Murder",
            Self::Banquet => "Host a public banquet",
            Self::Discredit => "Discredit their patron",
            Self::Lobby => "Lobby senator",
        }
    }
    /// Explain the action without revealing internal confidence amounts.
    pub fn description(self) -> &'static str {
        match self {
            Self::Petition => "Build modest goodwill with this senator over three months. The confidence you earn gradually fades.",
            Self::Gift => "Build goodwill with this senator over four months with a lawful gift. The confidence you earn gradually fades.",
            Self::Patronage => "Build this senator's confidence in you each month for six months. Confidence gradually fades without continued support.",
            Self::Bribe => "Pay this senator each month to build confidence faster than lobbying. Unpaid months grant no favor, and confidence gradually fades. Each payment risks exposure and faction confidence losses. You can cancel at any time.",
            Self::Threaten => "Build this senator's confidence in you each month for six months. Confidence gradually fades. Coercion hurts your standing with Aristocrats and Populares throughout the arrangement.",
            Self::Assassinate => "Remove every player's confidence in this seat. A neutral replacement arrives immediately. Exposure creates a severity III scandal.",
            Self::Banquet => "Build goodwill with this senator over four months through a public banquet. The confidence you earn gradually fades.",
            Self::Discredit => "Undermine this senator's current rival patron over three months. Exposure creates a severity II scandal.",
            Self::Lobby => "Pay Influence each month to gradually build and maintain this senator's confidence in you. Unpaid months grant no favor, and confidence gradually fades. You can cancel at any time.",
        }
    }
    /// Currency spent by this action.
    pub fn currency(self) -> Currency {
        if matches!(self, Self::Petition | Self::Patronage | Self::Discredit | Self::Lobby) {
            Currency::Influence
        } else {
            Currency::Coin
        }
    }
    /// Each player can maintain one of these actions on a senator at a time.
    pub fn is_ongoing(self) -> bool {
        matches!(self, Self::Patronage | Self::Bribe | Self::Threaten | Self::Lobby)
    }
    /// Monthly upkeep for actions maintained by recurring payments.
    pub fn upkeep(self, config: &SenateConfig) -> Option<f64> {
        match self {
            Self::Lobby => Some(config.senator_outreach_upkeep),
            Self::Bribe => Some(config.senator_bribe_upkeep),
            _ => None,
        }
    }
    fn monthly_confidence(self) -> f64 {
        match self {
            Self::Gift => 1.25,
            Self::Bribe | Self::Threaten => 2.0,
            Self::Banquet => 1.5,
            Self::Discredit => -1.0,
            Self::Assassinate => 0.0,
            _ => 1.0,
        }
    }
    fn duration(self) -> u32 {
        match self {
            Self::Petition | Self::Discredit => 3,
            Self::Gift | Self::Banquet => 4,
            Self::Patronage | Self::Threaten => 6,
            Self::Bribe | Self::Lobby => u32::MAX,
            Self::Assassinate => 0,
        }
    }
}
#[derive(Debug, Clone, Copy)]
/// Recorded outcome of a paid personal action.
pub struct SenatorActionOutcome {
    /// Whether the misconduct was publicly exposed immediately.
    pub caught: bool,
    /// Even undiscovered misconduct can later be found by a spy network.
    pub misconduct: Option<(super::espionage::ScandalKind, super::espionage::Severity)>,
}
/// A recurring bribe payment, retained as discoverable misconduct by the campaign.
#[derive(Debug, Clone, Copy)]
pub struct SenatorPaymentOutcome {
    /// Player who paid the bribe.
    pub player: PlayerId,
    /// Senator receiving the payment.
    pub senator: usize,
    /// Whether this month's payment was exposed.
    pub caught: bool,
}
#[derive(Debug, Clone)]
struct Outreach {
    player: PlayerId,
    bloc: Bloc,
    until: u32,
}
#[derive(Debug, Clone)]
/// Public scrutiny retained after its immediate confidence loss has been applied.
pub struct Accusation {
    /// Player whose misconduct was exposed.
    pub target: PlayerId,
    /// Immediate confidence losses in faction order; never applied again monthly.
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
    rng: PoliticalRng,
    seat_confidence: f64,
    action_spending: Vec<(PlayerId, SenatorAction, f64)>,
}
impl SenateState {
    /// A public invasion without casus belli damages faction support immediately.
    pub fn record_unjustified_attack(&mut self, player: PlayerId, config: &SenateConfig) {
        use super::espionage::{ScandalKind, Severity};
        self.apply_scandal(player, ScandalKind::FriendlyAttack, Severity::Major, config);
    }

    /// Breaking a trade route immediately removes ten Merchant confidence points.
    pub fn record_trade_breach(&mut self, player: PlayerId) {
        self.remove_confidence(player, Bloc::Merchants, 10.0);
        self.accusations.push(Accusation {
            target: player,
            penalties: [0.0, 10.0, 0.0, 0.0, 0.0],
            until: self.month.saturating_add(6),
        });
        self.review_allegiances();
    }

    /// Initialize a default chamber with zero confidence for every player.
    pub fn new(seed: u64) -> Self {
        Self::with_config(seed, &SenateConfig::default())
    }
    /// Initialize a chamber from the authoritative match configuration.
    pub fn with_config(seed: u64, config: &SenateConfig) -> Self {
        assert!(config.validate().is_ok(), "invalid Senate configuration");
        let mut senators = Vec::with_capacity(100);
        for bloc in Bloc::ALL {
            for _ in 0..config.bloc_sizes[bloc.index()] {
                senators.push(Senator {
                    id: senators.len(),
                    bloc,
                    allegiance: None,
                    generation: 0,
                    confidence: vec![],
                    personal_confidence: vec![],
                    arrangements: vec![],
                    effects: vec![],
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
            rng: PoliticalRng::new(seed),
            seat_confidence: config.seat_confidence,
            action_spending: vec![],
        }
    }
    /// Count all senators currently supporting a player.
    pub fn support(&self, player: PlayerId) -> usize {
        self.senators.iter().filter(|s| s.allegiance == Some(player)).count()
    }
    /// Fill the local practice player's missing support, taking neutral seats first.
    pub(crate) fn grant_practice_support(&mut self, player: PlayerId, required: usize) {
        let missing = required.saturating_sub(self.support(player));
        if missing == 0 {
            return;
        }
        self.ensure_players(player + 1);
        let mut seats: Vec<_> = self
            .senators
            .iter()
            .filter(|seat| seat.allegiance != Some(player))
            .map(|seat| (seat.allegiance.is_some(), seat.id))
            .collect();
        seats.sort_unstable();
        for (_, id) in seats.into_iter().take(missing) {
            self.add_to_seat(id, player, self.seat_confidence);
        }
        self.review_allegiances();
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
    /// Legitimate monthly outreach. Refreshing replaces the campaign; it never stacks.
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
    /// Validate an individual action without spending money or drawing randomness.
    pub fn senator_action_quote(
        &self,
        player: PlayerId,
        id: usize,
        action: SenatorAction,
        players: &[PoliticalPlayer],
        config: &SenateConfig,
    ) -> Result<(Currency, f64, f64), PoliticalError> {
        let actor = players.get(player).ok_or(PoliticalError::MissingTarget)?;
        let seat = self.senators.get(id).ok_or(PoliticalError::MissingTarget)?;
        if self.winner.is_some() {
            return Err(PoliticalError::CampaignFinished);
        }
        if action == SenatorAction::Discredit {
            match seat.allegiance {
                None => return Err(PoliticalError::SenatorIsNeutral),
                Some(patron) if patron == player => {
                    return Err(PoliticalError::RivalPatronRequired)
                },
                _ => {},
            }
        }
        if action.is_ongoing() {
            if let Some(arrangement) = seat.arrangement(player).filter(|a| a.until > self.month) {
                return Err(PoliticalError::SenatorArrangementActive {
                    action: arrangement.action,
                    months_remaining: arrangement.until.saturating_sub(self.month),
                });
            }
        }
        let currency = action.currency();
        let cost = config.action_costs[action as usize];
        let balance = if currency == Currency::Coin {
            actor.coin
        } else {
            actor.influence
        };
        if balance + 1e-9 < cost {
            return Err(PoliticalError::InsufficientFunds);
        }
        Ok((currency, cost, config.action_risks[action as usize]))
    }

    /// Spend once, apply a targeted effect and resolve its seeded detection risk.
    pub fn act_on_senator(
        &mut self,
        player: PlayerId,
        id: usize,
        action: SenatorAction,
        players: &mut [PoliticalPlayer],
        config: &SenateConfig,
    ) -> Result<SenatorActionOutcome, PoliticalError> {
        use super::espionage::{ScandalKind, Severity};
        let (currency, cost, risk) =
            self.senator_action_quote(player, id, action, players, config)?;
        players[player].spend(currency, cost)?;
        self.action_spending.push((player, action, cost));
        self.ensure_players(players.len());
        let bloc = self.senators[id].bloc;
        let misconduct = match action {
            SenatorAction::Bribe => Some((ScandalKind::PoliticalBribery, Severity::Medium)),
            SenatorAction::Threaten => Some((ScandalKind::SenatorCoercion, Severity::Medium)),
            SenatorAction::Assassinate => Some((ScandalKind::SenatorMurder, Severity::Major)),
            SenatorAction::Discredit => Some((ScandalKind::PoliticalSmear, Severity::Medium)),
            _ => None,
        };
        match action {
            SenatorAction::Patronage
            | SenatorAction::Bribe
            | SenatorAction::Threaten
            | SenatorAction::Lobby => {
                self.senators[id].arrangements.push(SenatorArrangement {
                    player,
                    action,
                    until: self.month.saturating_add(action.duration()),
                    paid_at: None,
                });
            },
            SenatorAction::Assassinate => {
                let seat = &mut self.senators[id];
                seat.confidence.fill(0.0);
                seat.personal_confidence.fill(0.0);
                seat.allegiance = None;
                seat.arrangements.clear();
                seat.effects.clear();
                seat.generation += 1;
            },
            _ => {
                let target = if action == SenatorAction::Discredit {
                    self.senators[id].allegiance.unwrap()
                } else {
                    player
                };
                self.senators[id].effects.push(SenatorEffect {
                    player,
                    action,
                    target,
                    until: self.month.saturating_add(action.duration()),
                });
            },
        }
        let caught = misconduct.is_some() && risk > 0.0 && self.rng.unit() < risk;
        if caught {
            // The public penalty is larger than the action's gain, and applies now.
            let (kind, severity) = misconduct.unwrap();
            self.apply_scandal(player, kind, severity, config);
            self.remove_confidence(
                player,
                bloc,
                if action == SenatorAction::Assassinate {
                    50.0
                } else {
                    30.0
                },
            );
            self.senators[id].arrangements.retain(|a| a.player != player);
            if action == SenatorAction::Discredit {
                self.senators[id].effects.pop();
            }
        }
        self.review_allegiances();
        Ok(SenatorActionOutcome {
            caught,
            misconduct,
        })
    }

    /// Upfront expenses incurred by this action in the current campaign month.
    pub fn spent_on(&self, player: PlayerId, action: SenatorAction) -> f64 {
        self.action_spending
            .iter()
            .filter(|(p, a, _)| *p == player && *a == action)
            .map(|(_, _, cost)| cost)
            .sum()
    }

    /// Scheduled Influence outflow for personal senator lobbying.
    pub fn outreach_upkeep(&self, player: PlayerId, config: &SenateConfig) -> f64 {
        self.action_upkeep(player, SenatorAction::Lobby, config)
    }

    /// Scheduled outflow for one kind of recurring personal action.
    pub fn action_upkeep(
        &self,
        player: PlayerId,
        action: SenatorAction,
        config: &SenateConfig,
    ) -> f64 {
        if self.winner.is_some() {
            return 0.0;
        }
        self.senators
            .iter()
            .filter(|s| {
                s.arrangement(player).is_some_and(|a| a.action == action && a.until > self.month)
            })
            .count() as f64
            * action.upkeep(config).unwrap_or(0.0)
    }

    /// Pay before new monthly income arrives. Unpaid arrangements pause rather than creating debt.
    pub fn pay_outreach(
        &mut self,
        players: &mut [PoliticalPlayer],
        config: &SenateConfig,
    ) -> Vec<SenatorPaymentOutcome> {
        let mut payments = Vec::new();
        if self.winner.is_some() {
            return payments;
        }
        self.ensure_players(players.len());
        for seat in &mut self.senators {
            seat.arrangements.retain_mut(|a| {
                let Some(upkeep) = a.action.upkeep(config) else {
                    return true;
                };
                let Some(actor) = players.get_mut(a.player) else {
                    return false;
                };
                if a.paid_at != Some(self.month) && actor.spend(a.action.currency(), upkeep).is_ok()
                {
                    a.paid_at = Some(self.month);
                    if a.action == SenatorAction::Bribe {
                        let risk = config.action_risks[a.action as usize];
                        payments.push(SenatorPaymentOutcome {
                            player: a.player,
                            senator: seat.id,
                            caught: risk > 0.0 && self.rng.unit() < risk,
                        });
                    }
                }
                true
            });
        }
        for payment in &payments {
            if payment.caught {
                use super::espionage::{ScandalKind, Severity};
                self.apply_scandal(
                    payment.player,
                    ScandalKind::PoliticalBribery,
                    Severity::Medium,
                    config,
                );
                let bloc = self.senators[payment.senator].bloc;
                self.remove_confidence(payment.player, bloc, 30.0);
                self.senators[payment.senator].arrangements.retain(|a| a.player != payment.player);
            }
        }
        self.review_allegiances();
        payments
    }

    /// Cancel only the player's own ongoing action, without another charge.
    pub fn cancel_senator_action(
        &mut self,
        player: PlayerId,
        id: usize,
    ) -> Result<(), PoliticalError> {
        if self.winner.is_some() {
            return Err(PoliticalError::CampaignFinished);
        }
        let seat = self.senators.get_mut(id).ok_or(PoliticalError::MissingTarget)?;
        if seat.arrangement(player).is_none() {
            return Err(PoliticalError::Ineligible);
        }
        seat.arrangements.retain(|a| a.player != player);
        Ok(())
    }

    fn ensure_players(&mut self, count: usize) {
        for seat in &mut self.senators {
            if seat.confidence.len() < count {
                seat.confidence.resize(count, 0.0);
                seat.personal_confidence.resize(count, 0.0);
            }
        }
    }

    /// Shared capacity: fill neutral confidence first, then take it from rivals.
    fn add_to_seat(&mut self, id: usize, player: PlayerId, amount: f64) {
        let seat = &mut self.senators[id];
        let gain = amount.max(0.0).min(self.seat_confidence - seat.confidence[player]);
        let used: f64 = seat.confidence.iter().sum();
        let neutral = (self.seat_confidence - used).max(0.0);
        let transfer = (gain - neutral).max(0.0);
        let rivals = used - seat.confidence[player];
        if transfer > 0.0 && rivals > 0.0 {
            for (p, points) in seat.confidence.iter_mut().enumerate() {
                if p != player {
                    *points = (*points * (1.0 - transfer / rivals)).max(0.0);
                    seat.personal_confidence[p] *= 1.0 - transfer / rivals;
                }
            }
        }
        seat.confidence[player] += gain;
    }

    fn add_personal_confidence(&mut self, id: usize, player: PlayerId, amount: f64) {
        let before = self.senators[id].confidence[player];
        self.add_to_seat(id, player, amount);
        let seat = &mut self.senators[id];
        seat.personal_confidence[player] += seat.confidence[player] - before;
    }

    fn remove_from_seat(&mut self, id: usize, player: PlayerId, amount: f64) {
        let seat = &mut self.senators[id];
        let loss = amount.max(0.0).min(seat.confidence[player]);
        seat.confidence[player] -= loss;
        seat.personal_confidence[player] = (seat.personal_confidence[player] - loss).max(0.0);
    }

    /// A victory or defeat changes military confidence immediately; seats remain contested.
    pub fn record_battle_result(&mut self, player: PlayerId, won: bool) {
        if self.winner.is_some() {
            return;
        }
        self.ensure_players(player + 1);
        if won {
            self.gain_confidence(player, Bloc::Military, 3.);
        } else {
            self.remove_confidence(player, Bloc::Military, 2.);
        }
        self.review_allegiances();
    }

    /// Pack gains into one's unfinished seat, then empty seats, then rival seats.
    fn gain_confidence(&mut self, player: PlayerId, bloc: Bloc, mut amount: f64) {
        while amount > 1e-9 {
            let neutral_available = self.senators.iter().any(|s| {
                s.bloc == bloc && s.confidence.iter().sum::<f64>() < self.seat_confidence - 1e-9
            });
            let candidate = self
                .senators
                .iter()
                .filter(|s| s.bloc == bloc && s.confidence[player] < self.seat_confidence - 1e-9)
                .filter(|s| {
                    !neutral_available
                        || s.confidence.iter().sum::<f64>() < self.seat_confidence - 1e-9
                })
                .max_by(|a, b| {
                    let priority = |s: &Senator| {
                        let own = s.confidence[player];
                        let total: f64 = s.confidence.iter().sum();
                        if own > 0.0 {
                            (3, own)
                        } else if total < 1e-9 {
                            (2, 0.0)
                        } else {
                            (1, -total)
                        }
                    };
                    let (ac, av) = priority(a);
                    let (bc, bv) = priority(b);
                    ac.cmp(&bc).then_with(|| av.total_cmp(&bv)).then_with(|| b.id.cmp(&a.id))
                })
                .map(|s| s.id);
            let Some(id) = candidate else {
                break;
            };
            let available = self.seat_confidence
                - if neutral_available {
                    self.senators[id].confidence.iter().sum::<f64>()
                } else {
                    self.senators[id].confidence[player]
                };
            let gain = amount.min(available);
            self.add_to_seat(id, player, gain);
            amount -= gain;
        }
    }

    /// Losses drain unfinished confidence before releasing fully supporting seats.
    fn remove_confidence(&mut self, player: PlayerId, bloc: Bloc, mut amount: f64) {
        let mut seats: Vec<_> = self
            .senators
            .iter()
            .filter(|s| s.bloc == bloc)
            .filter_map(|s| s.confidence.get(player).filter(|v| **v > 0.0).map(|v| (s.id, *v)))
            .collect();
        seats.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        for (id, _) in seats {
            let loss = amount.min(self.senators[id].confidence[player]);
            self.remove_from_seat(id, player, loss);
            amount -= loss;
            if amount <= 1e-9 {
                break;
            }
        }
    }

    fn review_allegiances(&mut self) {
        for seat in &mut self.senators {
            seat.allegiance =
                seat.confidence.iter().position(|v| *v >= self.seat_confidence - 1e-9);
        }
    }

    fn apply_scandal(
        &mut self,
        target: PlayerId,
        kind: super::espionage::ScandalKind,
        severity: super::espionage::Severity,
        config: &SenateConfig,
    ) {
        let penalties = kind.senate_losses(severity);
        for bloc in Bloc::ALL {
            self.remove_confidence(target, bloc, penalties[bloc.index()]);
        }
        for seat in &mut self.senators {
            seat.arrangements.retain(|a| a.player != target || a.action != SenatorAction::Threaten);
        }
        self.accusations.push(Accusation {
            target,
            penalties,
            until: self.month.saturating_add(config.scandal_months),
        });
        self.review_allegiances();
    }
    /// Real unexpired evidence is consumed atomically, with immediate faction losses.
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
                s.id == id
                    && s.holder == holder
                    && s.is_current(self.month)
                    && !s.reserved_for_motion
            })
            .ok_or(PoliticalError::ScandalRequired)?;
        let ScandalTarget::Player(target) = evidence.target else {
            return Err(PoliticalError::Ineligible);
        };
        if target == holder || holder >= players.len() || target >= players.len() {
            return Err(PoliticalError::Ineligible);
        }
        let kind = evidence.kind;
        let severity = evidence.severity;
        espionage.consume(holder, id, self.month)?;
        self.ensure_players(players.len());
        self.apply_scandal(target, kind, severity, config);
        Ok(target)
    }
    /// Explain actual recurring drivers, separate from one-time actions and scandals.
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
            .count() as f64
            * config.court_bonus;
        reasons.push(SupportReason {
            label: "Faction outreach",
            points: outreach,
        });
        if matches!(bloc, Bloc::Aristocrats | Bloc::Populares) {
            let coercion = self
                .senators
                .iter()
                .filter(|s| {
                    s.arrangement(player).is_some_and(|a| {
                        a.action == SenatorAction::Threaten && a.until > self.month
                    })
                })
                .count() as f64;
            reasons.push(SupportReason {
                label: "Coercion of senators",
                points: -0.5 * coercion,
            });
        }
        reasons
    }

    /// Monthly snapshots change accumulated confidence, never replace it.
    /// Losses resolve first; the first recipient rotates to avoid persistent order bias.
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
        self.ensure_players(players.len());
        for id in 0..self.senators.len() {
            for player in 0..players.len() {
                let decay = self.senators[id].personal_confidence[player]
                    .min(config.personal_confidence_decay);
                self.remove_from_seat(id, player, decay);
            }
        }
        let rates: Vec<_> = players
            .iter()
            .enumerate()
            .map(|(id, actor)| {
                let profile = profiles.get(id).cloned().unwrap_or_default();
                Bloc::ALL.map(|bloc| {
                    self.reasons(id, bloc, actor, &profile, config)
                        .iter()
                        .map(|r| r.points)
                        .sum::<f64>()
                })
            })
            .collect();
        for (player, rate) in rates.iter().enumerate() {
            for bloc in Bloc::ALL {
                self.remove_confidence(player, bloc, (-rate[bloc.index()]).max(0.0));
            }
        }
        for turn in 0..players.len() {
            let player = (turn + self.month as usize) % players.len();
            for bloc in Bloc::ALL {
                self.gain_confidence(player, bloc, rates[player][bloc.index()].max(0.0));
            }
        }
        for turn in 0..players.len() {
            let player = (turn + self.month as usize) % players.len();
            for id in 0..self.senators.len() {
                if let Some(a) = self.senators[id].arrangement(player) {
                    if a.until > self.month
                        && (a.action.upkeep(config).is_none() || a.paid_at == Some(self.month))
                    {
                        self.add_personal_confidence(id, player, a.action.monthly_confidence());
                    }
                }
                let effects = self.senators[id].effects.clone();
                for effect in effects {
                    if effect.player != player || effect.until <= self.month {
                        continue;
                    }
                    let points = effect.action.monthly_confidence();
                    if points < 0.0 {
                        self.remove_from_seat(id, effect.target, -points);
                    } else {
                        self.add_personal_confidence(id, player, points);
                    }
                }
            }
        }
        self.month += 1;
        self.action_spending.clear();
        self.used_actions.clear();
        self.outreach.retain(|o| o.until > self.month);
        self.accusations.retain(|a| a.until > self.month);
        for seat in &mut self.senators {
            seat.arrangements.retain(|a| a.player < players.len() && a.until > self.month);
            seat.effects
                .retain(|effect| effect.player < players.len() && effect.until > self.month);
        }
        self.review_allegiances();
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
/// Bounded returns improve monthly progress without runaway growth.
fn diminishing(amount: f64, half: f64) -> f64 {
    amount.max(0.0) / (amount.max(0.0) + half.max(0.001))
}

fn structural_reasons(
    bloc: Bloc,
    p: &PoliticalProfile,
    actor: &PoliticalPlayer,
) -> Vec<SupportReason> {
    let governed = p.controlled_provinces > 0.0;
    let happy_nobles = ((p.noble_happiness - 50.0) / 50.0).clamp(0.0, 1.0);
    let food_policy = p.food_policy.clamp(-1.0, 1.0);
    let rows: Vec<(&'static str, f64)> = match bloc {
        Bloc::Aristocrats => vec![
            (
                "Happy noble population",
                0.6 * diminishing((p.nobles.max(0.0) / crate::map::POPULATION_SCALE).sqrt(), 8.0)
                    * happy_nobles,
            ),
            ("Noble happiness", (p.noble_happiness - 50.0) * 0.025),
            ("Political office", actor.rank.ladder_index() as f64 * 0.15),
            ("Completed wonders", 0.8 * diminishing(p.wonders, 2.0)),
            ("Forums", 0.6 * diminishing(p.political_buildings, 4.0)),
        ],
        Bloc::Merchants => vec![
            (
                "Net monthly coin income",
                // Every profit contributes. Logarithmic growth slows the return
                // without capping large incomes below ordinary monthly penalties.
                0.8 * (p.coin_income.max(0.0) / 30.0).ln_1p(),
            ),
            ("Active fulfilled trade routes", 0.5 * p.active_trade_routes.clamp(0.0, 10.0)),
            ("Unfulfilled trade commitments", -(1.0 - p.trade_reliability).clamp(0.0, 1.0)),
            ("Urban Markets", 0.6 * diminishing(p.markets, 5.0)),
            ("Resource shortages", -(1.0 - p.resource_security).clamp(0.0, 1.0)),
            ("Active wars", -0.5 * p.active_wars.clamp(0.0, 4.0)),
            (
                "Loss-making economy",
                if p.coin_income < 0.0 {
                    -0.5
                } else {
                    0.0
                },
            ),
        ],
        Bloc::Provincials => vec![
            ("Vassal provinces", diminishing(p.vassal_count, 4.0)),
            ("Relations with other provinces", (p.province_relation - 50.0) * 0.02),
            ("Trade with provinces without cities", 0.4 * diminishing(p.provincial_trade, 40.0)),
            ("High tribute", -1.5 * p.high_tribute),
            ("Active wars", -0.25 * p.active_wars.clamp(0.0, 4.0)),
        ],
        Bloc::Populares => vec![
            ("Citizen happiness", (p.citizen_happiness - 50.0) * 0.03),
            ("Plebeian happiness", (p.plebeian_happiness - 50.0) * 0.025),
            (
                "Ample food reserves",
                if governed {
                    0.8 * diminishing(p.food_reserve_months, 3.0) * p.food_security.clamp(0.0, 1.0)
                } else {
                    0.0
                },
            ),
            (
                "Generous or restricted food policy",
                if governed {
                    food_policy
                        * 0.5
                        * if food_policy > 0.0 {
                            p.food_security.clamp(0.0, 1.0)
                        } else {
                            1.0
                        }
                } else {
                    0.0
                },
            ),
            (
                "Food shortages",
                if governed {
                    -(1.0 - p.food_security).clamp(0.0, 1.0) * 1.25
                } else {
                    0.0
                },
            ),
            ("Active famine", -3.0 * p.famine),
            ("Tax pressure", -p.tax_pressure),
            ("Harsh slave labor", -p.harsh_policies),
        ],
        Bloc::Military => vec![
            ("Effective army strength", 1.5 * diminishing(p.military_strength, 500.0)),
            ("Recent victories or defeats", p.recent_victories.clamp(-4.0, 4.0) * 0.3),
            ("Military rank", p.military_rank.clamp(0.0, 3.0) * 0.5),
            (
                "Control beyond the first province",
                0.15 * (p.controlled_provinces - 1.0).clamp(0.0, 10.0),
            ),
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
