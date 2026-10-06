//! Relation is sentiment; control is political power. Neither substitutes for the other.

use super::{Currency, PlayerId, PoliticalError, PoliticalPlayer};
use std::collections::{BTreeSet, VecDeque};

/// Configurable diplomatic costs and diminishing-return curves.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiplomacyConfig {
    /// Coin price of one point of relation or political control before modifiers.
    pub coin_per_point: f64,
    /// Influence price of one point before modifiers.
    pub influence_per_point: f64,
    /// Recurring coin cost for one point of monthly support.
    pub monthly_coin: f64,
    /// Recurring Influence cost for one point of monthly support.
    pub monthly_influence: f64,
    /// Maximum hostile occupation control gain per month.
    pub garrison_cap: f64,
    /// Maximum peaceful foreign stationing control gain per month.
    pub invited_garrison_cap: f64,
    /// Effective military strength at half of the control cap.
    pub garrison_half_saturation: f64,
    /// Passive Influence generated per point of Vassal Control.
    pub vassal_influence_per_control: f64,
    /// Maximum independent control attainable through recurring trade alone.
    pub trade_control_ceiling: f64,
    /// Monthly maximum of trade-derived control per player and province.
    pub trade_control_monthly_cap: f64,
    /// Monthly occupation relation penalty, independent of control gain.
    pub occupation_relation_loss: f64,
    /// Base coin cost of opposition/dissidents/unrest before route and resistance.
    pub interference_coin_cost: f64,
    /// Base Influence cost of smear/agitation before route and resistance.
    pub interference_influence_cost: f64,
    /// Relation and independent-control reduction for targeted interference.
    pub interference_points: f64,
    /// Direct vassal-control reduction by Political Agitation.
    pub vassal_agitation_points: f64,
    /// Domestic happiness reduction for Agitate Population and Fund Unrest.
    pub unrest_happiness_points: f64,
}

impl Default for DiplomacyConfig {
    /// Initial values follow the spec's revised mechanics and economical action scales.
    fn default() -> Self {
        Self {
            coin_per_point: 20.0,
            influence_per_point: 2.0,
            monthly_coin: 10.0,
            monthly_influence: 2.0,
            garrison_cap: 5.0,
            invited_garrison_cap: 1.0,
            garrison_half_saturation: 20.0,
            vassal_influence_per_control: 0.05,
            trade_control_ceiling: 40.0,
            trade_control_monthly_cap: 1.0,
            occupation_relation_loss: 2.0,
            interference_coin_cost: 100.0,
            interference_influence_cost: 10.0,
            interference_points: 5.0,
            vassal_agitation_points: 3.0,
            unrest_happiness_points: 5.0,
        }
    }
}

/// The mutually exclusive political states of a province.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PoliticalState {
    /// The protected capital has no political Control or bilateral Relation.
    Rome,
    /// Local plus player shares always total 100.
    Independent {
        /// Control held by local government.
        local: f64,
        /// Control held by each player index.
        shares: Vec<f64>,
    },
    /// Rival independent shares no longer exist.
    Vassal {
        /// Controlling player.
        overlord: PlayerId,
        /// Stability and integration progress, clamped to 0..100.
        control: f64,
        /// Coin tribute policy.
        tribute: Tribute,
    },
    /// Direct ownership remains legal while rival players contest political control.
    Owned {
        /// Direct owner.
        owner: PlayerId,
    },
}

/// Tribute trades income against relation and therefore long-term stability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Tribute {
    /// Half normal tribute; +0.5 monthly relation.
    Low,
    /// Baseline tribute; no monthly relation effect.
    #[default]
    Normal,
    /// One and a half normal tribute; -1 monthly relation.
    High,
}

impl Tribute {
    /// Fraction of the NPC's available monthly coin output paid to the overlord.
    pub fn income_fraction(self) -> f64 {
        match self {
            Self::Low => 0.1,
            Self::Normal => 0.2,
            Self::High => 0.3,
        }
    }
    /// Monthly sentiment change toward the overlord.
    pub fn relation_delta(self) -> f64 {
        match self {
            Self::Low => 0.5,
            Self::Normal => 0.0,
            Self::High => -1.0,
        }
    }
}

/// Independent switches for relationship and government-support spending.
#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
pub struct MonthlySupport {
    /// Spend coin for relation.
    pub relation_coin: bool,
    /// Spend Influence for relation.
    pub relation_influence: bool,
    /// Spend coin for overlord control, only while a vassal.
    pub control_coin: bool,
    /// Spend Influence for overlord control, only while a vassal.
    pub control_influence: bool,
}

/// Direct interference actions are publicly attributed, not clandestine spy rolls.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum Interference {
    /// Influence reduces only the rival's relation.
    SmearCampaign,
    /// Coin reduces a rival's independent control, returning it to Local.
    FundOpposition,
    /// Influence reduces a rival's independent control, returning it to Local.
    UndermineRival,
    /// Influence immediately reduces an enemy overlord's vassal control.
    PoliticalAgitation,
    /// Coin reduces the vassal's relation toward its overlord.
    FundDissidents,
    /// Influence reduces Citizen and Plebeian happiness in enemy-owned land.
    AgitatePopulation,
    /// Coin reduces Plebeian happiness in enemy-owned land.
    FundUnrest,
}

/// A monthly control breakdown for explanatory tooltips.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ControlBreakdown {
    /// Poor relation causes decay; friendship does not grant free control.
    pub relation: f64,
    /// Diminishing contribution of physically stationed effective strength.
    pub military: f64,
    /// Actually paid government support.
    pub support: f64,
    /// Total change before clamping.
    pub total: f64,
}

/// Mutable provincial politics with hidden-in-progress, simultaneously resolved pressure.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProvincePolitics {
    /// Current legal relationship.
    pub state: PoliticalState,
    /// Sentiment toward each player; retained across state transitions.
    pub relations: Vec<f64>,
    /// Player Control in directly owned land; any remaining share belongs to local rebels.
    pub owned_shares: Vec<f64>,
    /// Independently configured recurring diplomatic programs per player.
    pub support: Vec<MonthlySupport>,
    /// Most recent vassal-control explanation.
    pub last_control_change: ControlBreakdown,
    pending_gains: Vec<f64>,
    pending_reductions: Vec<f64>,
    trade_this_month: Vec<f64>,
    used_interference: BTreeSet<(PlayerId, PlayerId, Interference)>,
}

impl ProvincePolitics {
    /// Rome has no provincial actions and cannot be politically acquired.
    pub fn rome(players: usize) -> Self {
        let mut province = Self::independent(players);
        province.state = PoliticalState::Rome;
        province
    }
    /// Start an independent province with all power local and neutral relations.
    pub fn independent(players: usize) -> Self {
        Self {
            state: PoliticalState::Independent {
                local: 100.0,
                shares: vec![0.0; players],
            },
            relations: vec![50.0; players],
            owned_shares: vec![0.0; players],
            support: vec![MonthlySupport::default(); players],
            last_control_change: ControlBreakdown::default(),
            pending_gains: vec![0.0; players],
            pending_reductions: vec![0.0; players],
            trade_this_month: vec![0.0; players],
            used_interference: BTreeSet::new(),
        }
    }

    /// Construct a directly owned province without independent control state.
    pub fn owned(players: usize, owner: PlayerId) -> Self {
        let mut province = Self::independent(players);
        province.state = PoliticalState::Owned {
            owner,
        };
        if let Some(share) = province.owned_shares.get_mut(owner) {
            *share = 100.0;
        }
        province
    }

    /// Look up sentiment safely; unknown foreign actors are neutral.
    pub fn relation(&self, player: PlayerId) -> f64 {
        self.relations.get(player).copied().unwrap_or(50.0)
    }

    /// Apply sentiment without ever changing political control.
    pub fn change_relation(&mut self, player: PlayerId, change: f64) {
        if self.state == PoliticalState::Rome {
            return;
        }
        if change.is_finite() {
            if let Some(relation) = self.relations.get_mut(player) {
                *relation = (*relation + change).clamp(0.0, 100.0);
            }
        }
    }

    /// Return independent control or zero when the province is no longer independent.
    pub fn independent_control(&self, player: PlayerId) -> f64 {
        match &self.state {
            PoliticalState::Independent {
                shares,
                ..
            } => shares.get(player).copied().unwrap_or(0.0),
            _ => 0.0,
        }
    }

    /// Political power held by a player in an NPC independent or vassal province.
    pub fn control(&self, player: PlayerId) -> f64 {
        match self.state {
            PoliticalState::Owned {
                owner,
            } => {
                if self.owned_shares.iter().all(|share| *share == 0.0) {
                    if owner == player {
                        100.0
                    } else {
                        0.0
                    }
                } else {
                    self.owned_shares.get(player).copied().unwrap_or(0.0)
                }
            },
            PoliticalState::Vassal {
                overlord,
                control,
                ..
            } if overlord == player => control,
            _ => self.independent_control(player),
        }
    }

    /// Targeted spy losses join independent monthly pressure or weaken vassal stability.
    pub fn queue_control_loss(
        &mut self,
        player: PlayerId,
        amount: f64,
    ) -> Result<(), PoliticalError> {
        valid_amount(amount)?;
        match &mut self.state {
            PoliticalState::Independent {
                ..
            } => {
                *self.pending_reductions.get_mut(player).ok_or(PoliticalError::MissingTarget)? +=
                    amount;
            },
            PoliticalState::Vassal {
                overlord,
                control,
                ..
            } if *overlord == player => {
                *control = (*control - amount).max(0.0);
            },
            _ => return Err(PoliticalError::Ineligible),
        }
        Ok(())
    }

    /// Transfer an owner's Control to local rebels, releasing the province at zero.
    pub fn apply_rebellion_control_loss(&mut self, amount: f64) -> Result<(), PoliticalError> {
        valid_amount(amount)?;
        let PoliticalState::Owned {
            owner,
        } = self.state
        else {
            return Err(PoliticalError::Ineligible);
        };
        if self.owned_shares.iter().all(|share| *share == 0.0) {
            self.owned_shares[owner] = 100.0;
        }
        self.owned_shares[owner] = (self.owned_shares[owner] - amount).max(0.0);
        if self.owned_shares[owner] <= 1e-7 {
            self.owned_shares[owner] = 0.0;
            self.state = PoliticalState::Independent {
                local: (100.0 - self.owned_shares.iter().sum::<f64>()).max(0.0),
                shares: self.owned_shares.clone(),
            };
            self.support.fill(MonthlySupport::default());
            self.clear_pending();
        }
        Ok(())
    }

    /// Pay for immediate relation gain with diminishing returns at high friendship.
    pub fn improve_relation(
        &mut self,
        player: PlayerId,
        actor: &mut PoliticalPlayer,
        currency: Currency,
        amount: f64,
        distance: Option<usize>,
        config: &DiplomacyConfig,
    ) -> Result<(), PoliticalError> {
        if matches!(self.state, PoliticalState::Owned { owner } if owner == player)
            || self.state == PoliticalState::Rome
            || player >= self.relations.len()
        {
            return Err(PoliticalError::Ineligible);
        }
        valid_amount(amount)?;
        let cost = point_cost(currency, config)
            * amount
            * (1.0 + ((self.relation(player) - 50.0) / 50.0).max(0.0))
            * distance_multiplier(distance)?;
        actor.spend(currency, cost)?;
        self.change_relation(player, amount);
        Ok(())
    }

    /// Pay for independent political pressure, resolved with all rivals at month end.
    pub fn buy_control(
        &mut self,
        player: PlayerId,
        actor: &mut PoliticalPlayer,
        currency: Currency,
        amount: f64,
        distance: Option<usize>,
        config: &DiplomacyConfig,
    ) -> Result<(), PoliticalError> {
        valid_amount(amount)?;
        let (current, local, largest_rival) = match &self.state {
            PoliticalState::Independent {
                shares,
                local,
            } => (
                *shares.get(player).ok_or(PoliticalError::MissingTarget)?,
                *local,
                shares
                    .iter()
                    .enumerate()
                    .filter(|(id, _)| *id != player)
                    .map(|(_, share)| *share)
                    .fold(0.0, f64::max),
            ),
            PoliticalState::Owned {
                owner,
            } if *owner != player => (
                self.control(player),
                0.0,
                (0..self.relations.len())
                    .filter(|id| *id != player)
                    .map(|id| self.control(id))
                    .fold(0.0, f64::max),
            ),
            _ => return Err(PoliticalError::Ineligible),
        };
        let rival_fraction = if amount > 0.0 {
            ((amount - local).max(0.0) / amount).min(1.0)
        } else {
            0.0
        };
        let resistance = 1.0 + rival_fraction * (entrenchment(largest_rival) - 1.0);
        let expansion = if currency == Currency::Coin {
            1.0 + current / 25.0
        } else {
            1.0
        };
        let relation_cost = (1.5 - self.relation(player) / 100.0).clamp(0.5, 1.5);
        actor.spend(
            currency,
            point_cost(currency, config)
                * amount
                * expansion
                * relation_cost
                * distance_multiplier(distance)?
                * resistance,
        )?;
        self.queue_control_gain(player, amount)
    }

    /// Add valid occupation, trade, scandal or paid pressure to the simultaneous pool.
    pub fn queue_control_gain(
        &mut self,
        player: PlayerId,
        amount: f64,
    ) -> Result<(), PoliticalError> {
        valid_amount(amount)?;
        if !matches!(self.state, PoliticalState::Independent { .. } | PoliticalState::Owned { .. })
        {
            return Err(PoliticalError::Ineligible);
        }
        if matches!(self.state, PoliticalState::Owned { owner } if owner == player) {
            return Err(PoliticalError::Ineligible);
        }
        *self.pending_gains.get_mut(player).ok_or(PoliticalError::MissingTarget)? += amount;
        Ok(())
    }

    /// Apply a one-time political gain now using the same bounded share transfer as monthly pressure.
    pub fn gain_control_now(
        &mut self,
        player: PlayerId,
        amount: f64,
    ) -> Result<(), PoliticalError> {
        valid_amount(amount)?;
        if player >= self.relations.len() {
            return Err(PoliticalError::MissingTarget);
        }
        let mut gains = vec![0.0; self.relations.len()];
        let reductions = vec![0.0; self.relations.len()];
        gains[player] = amount;
        match &mut self.state {
            PoliticalState::Independent {
                local,
                shares,
            } => {
                resolve_control(local, shares, &gains, &reductions);
            },
            PoliticalState::Owned {
                owner,
            } if *owner != player => {
                if self.owned_shares.iter().all(|share| *share == 0.0) {
                    self.owned_shares[*owner] = 100.0;
                }
                let mut local = (100.0 - self.owned_shares.iter().sum::<f64>()).max(0.0);
                resolve_control(&mut local, &mut self.owned_shares, &gains, &reductions);
            },
            _ => return Err(PoliticalError::Ineligible),
        }
        Ok(())
    }

    /// Unresolved positive pressure requested by this player for the next monthly
    /// pool. It is a request, not a promise of the final gain against rival pressure.
    pub fn queued_control_pressure(&self, player: PlayerId) -> f64 {
        self.pending_gains.get(player).copied().unwrap_or(0.0)
    }

    /// Apply trade sentiment and capped monthly control; caller excludes one-time control.
    pub fn apply_trade(
        &mut self,
        player: PlayerId,
        relation_gain: f64,
        control_gain: f64,
        config: &DiplomacyConfig,
    ) {
        if self.state == PoliticalState::Rome {
            return;
        }
        self.change_relation(player, relation_gain.max(0.0));
        if matches!(self.state, PoliticalState::Owned { .. }) {
            return;
        }
        if player >= self.trade_this_month.len() || !control_gain.is_finite() {
            return;
        }
        let allowed = control_gain
            .max(0.0)
            .min((config.trade_control_monthly_cap - self.trade_this_month[player]).max(0.0))
            .min(
                (config.trade_control_ceiling
                    - self.independent_control(player)
                    - self.pending_gains[player])
                    .max(0.0),
            );
        if self.queue_control_gain(player, allowed).is_ok() {
            self.trade_this_month[player] += allowed;
        }
    }

    /// Perform transparent foreign interference and return pop happiness deltas.
    /// Returned class order is Nobles, Citizens, Plebeians, Slaves.
    pub fn interfere(
        &mut self,
        actor_id: PlayerId,
        rival: PlayerId,
        actor: &mut PoliticalPlayer,
        action: Interference,
        distance: Option<usize>,
        config: &DiplomacyConfig,
    ) -> Result<[f64; 4], PoliticalError> {
        if actor_id == rival || actor_id >= self.relations.len() || rival >= self.relations.len() {
            return Err(PoliticalError::Ineligible);
        }
        if action == Interference::SmearCampaign
            && self.used_interference.contains(&(actor_id, rival, action))
        {
            return Err(PoliticalError::AlreadyUsed);
        }
        let relation_resistance = relation_resistance(self.relation(rival));
        let (currency, mut cost, allowed) = match action {
            Interference::SmearCampaign => (
                Currency::Influence,
                config.interference_influence_cost * relation_resistance,
                matches!(self.state, PoliticalState::Independent { .. })
                    || matches!(self.state, PoliticalState::Vassal { overlord, .. } if overlord == rival),
            ),
            Interference::FundOpposition | Interference::UndermineRival => {
                let coin = action == Interference::FundOpposition;
                (
                    if coin {
                        Currency::Coin
                    } else {
                        Currency::Influence
                    },
                    if coin {
                        config.interference_coin_cost
                    } else {
                        config.interference_influence_cost
                    } * entrenchment(self.independent_control(rival))
                        * relation_resistance,
                    self.independent_control(rival) > 0.0,
                )
            },
            Interference::PoliticalAgitation => (
                Currency::Influence,
                config.interference_influence_cost
                    * match self.state {
                        PoliticalState::Vassal {
                            control,
                            ..
                        } => entrenchment(control),
                        _ => 1.0,
                    },
                matches!(self.state, PoliticalState::Vassal { overlord, .. } if overlord == rival),
            ),
            Interference::FundDissidents => (
                Currency::Coin,
                config.interference_coin_cost * relation_resistance,
                matches!(self.state, PoliticalState::Vassal { overlord, .. } if overlord == rival),
            ),
            Interference::AgitatePopulation => (
                Currency::Influence,
                config.interference_influence_cost,
                matches!(self.state, PoliticalState::Owned { owner } if owner == rival),
            ),
            Interference::FundUnrest => (
                Currency::Coin,
                config.interference_coin_cost,
                matches!(self.state, PoliticalState::Owned { owner } if owner == rival),
            ),
        };
        if !allowed {
            return Err(PoliticalError::Ineligible);
        }
        cost *= distance_multiplier(distance)?;
        actor.spend(currency, cost)?;
        if action == Interference::SmearCampaign {
            self.used_interference.insert((actor_id, rival, action));
        }
        let mut happiness = [0.0; 4];
        match action {
            Interference::SmearCampaign | Interference::FundDissidents => {
                self.change_relation(rival, -config.interference_points)
            },
            Interference::FundOpposition | Interference::UndermineRival => {
                self.pending_reductions[rival] += config.interference_points
            },
            Interference::PoliticalAgitation => {
                if let PoliticalState::Vassal {
                    control,
                    ..
                } = &mut self.state
                {
                    *control = (*control - config.vassal_agitation_points).max(0.0);
                }
                self.release_if_unstable();
            },
            Interference::AgitatePopulation => {
                happiness[1] = -config.unrest_happiness_points;
                happiness[2] = -config.unrest_happiness_points;
            },
            Interference::FundUnrest => happiness[2] = -config.unrest_happiness_points,
        }
        Ok(happiness)
    }

    /// Elective vassalization consumes the first fifty control and discards rival shares.
    pub fn vassalize(&mut self, player: PlayerId) -> Result<(), PoliticalError> {
        let shares = match &self.state {
            PoliticalState::Independent {
                shares,
                ..
            } => shares,
            PoliticalState::Owned {
                owner,
            } if *owner != player => &self.owned_shares,
            _ => return Err(PoliticalError::Ineligible),
        };
        let control = *shares.get(player).ok_or(PoliticalError::MissingTarget)?;
        if control <= 50.0
            || shares.iter().enumerate().any(|(id, value)| id != player && *value >= control)
        {
            return Err(PoliticalError::Ineligible);
        }
        self.state = PoliticalState::Vassal {
            overlord: player,
            control: control - 50.0,
            tribute: Tribute::Normal,
        };
        self.clear_pending();
        Ok(())
    }

    /// Elective ownership at 100 control; return the one-time happiness shift.
    pub fn take_ownership(&mut self, player: PlayerId) -> Result<f64, PoliticalError> {
        let allowed = match &self.state {
            PoliticalState::Independent {
                shares,
                ..
            } => shares.get(player).is_some_and(|v| *v >= 100.0 - 1e-7),
            PoliticalState::Owned {
                owner,
            } if *owner != player => {
                self.owned_shares.get(player).is_some_and(|v| *v >= 100.0 - 1e-7)
            },
            PoliticalState::Vassal {
                overlord,
                control,
                ..
            } => *overlord == player && *control >= 100.0 - 1e-7,
            _ => false,
        };
        if !allowed {
            return Err(PoliticalError::Ineligible);
        }
        let shift = self.relation(player) - 50.0;
        self.state = PoliticalState::Owned {
            owner: player,
        };
        self.owned_shares.fill(0.0);
        self.owned_shares[player] = 100.0;
        self.support.fill(MonthlySupport::default());
        self.clear_pending();
        Ok(shift)
    }

    /// Transfer an enemy-owned province after an authoritative military victory.
    /// Independent NPC victories instead establish occupation and accumulate control.
    pub fn capture_owned(&mut self, victor: PlayerId) -> Result<(), PoliticalError> {
        if victor >= self.relations.len()
            || !matches!(self.state, PoliticalState::Owned { owner } if owner != victor)
        {
            return Err(PoliticalError::Ineligible);
        }
        self.state = PoliticalState::Owned {
            owner: victor,
        };
        self.owned_shares.fill(0.0);
        self.owned_shares[victor] = 100.0;
        self.support.fill(MonthlySupport::default());
        self.clear_pending();
        Ok(())
    }

    /// Apply a one-time paid government boost without modifying relation.
    pub fn support_government(
        &mut self,
        player: PlayerId,
        actor: &mut PoliticalPlayer,
        currency: Currency,
        amount: f64,
        distance: Option<usize>,
        config: &DiplomacyConfig,
    ) -> Result<(), PoliticalError> {
        valid_amount(amount)?;
        let PoliticalState::Vassal {
            overlord,
            control,
            ..
        } = &mut self.state
        else {
            return Err(PoliticalError::Ineligible);
        };
        if *overlord != player {
            return Err(PoliticalError::Ineligible);
        }
        actor.spend(
            currency,
            amount * point_cost(currency, config) * distance_multiplier(distance)?,
        )?;
        *control = (*control + amount).min(100.0);
        Ok(())
    }

    /// Resolve recurring programs and simultaneous control. Military power must include
    /// the owner's military rank and only units physically in this province.
    pub fn advance_month(
        &mut self,
        players: &mut [PoliticalPlayer],
        distances: &[Option<usize>],
        stationed_power: &[f64],
        occupation: Option<PlayerId>,
        config: &DiplomacyConfig,
    ) {
        let paid_control_support = self.spend_recurring_support(players, distances, config);
        self.resolve_month(stationed_power, occupation, paid_control_support, config);
    }

    /// Debit recurring programs before new income or trades are applied. Relation
    /// programs take effect immediately; return paid government-support points for
    /// the later political resolution so battle outcomes still resolve first.
    pub fn spend_recurring_support(
        &mut self,
        players: &mut [PoliticalPlayer],
        distances: &[Option<usize>],
        config: &DiplomacyConfig,
    ) -> f64 {
        let mut control_support = 0.0;
        if self.state != PoliticalState::Rome {
            for player in 0..players.len().min(self.support.len()) {
                if matches!(self.state, PoliticalState::Owned { owner } if owner == player) {
                    continue;
                }
                let Ok(distance) = distance_multiplier(distances.get(player).copied().flatten())
                else {
                    continue;
                };
                let support = self.support[player];
                for (enabled, currency, cost, for_control) in [
                    (support.relation_coin, Currency::Coin, config.monthly_coin, false),
                    (
                        support.relation_influence,
                        Currency::Influence,
                        config.monthly_influence,
                        false,
                    ),
                    (support.control_coin, Currency::Coin, config.monthly_coin, true),
                    (
                        support.control_influence,
                        Currency::Influence,
                        config.monthly_influence,
                        true,
                    ),
                ] {
                    let valid = !for_control
                        || matches!(self.state, PoliticalState::Vassal { overlord, .. } if overlord == player);
                    if enabled && valid && players[player].spend(currency, cost * distance).is_ok()
                    {
                        if for_control {
                            control_support += 1.0;
                        } else {
                            self.change_relation(player, 1.0);
                        }
                    }
                }
            }
        }
        control_support
    }

    /// Resolve pooled control after military outcomes, using only the support that
    /// was actually paid earlier in the month. Captured owned provinces ignore it.
    pub fn resolve_month(
        &mut self,
        stationed_power: &[f64],
        occupation: Option<PlayerId>,
        paid_control_support: f64,
        config: &DiplomacyConfig,
    ) {
        match self.state.clone() {
            PoliticalState::Independent {
                ..
            } => {
                for player in 0..self.relations.len() {
                    let power = stationed_power.get(player).copied().unwrap_or(0.0);
                    if power <= 0.0 {
                        continue;
                    }
                    let hostile = occupation == Some(player);
                    let cap = if hostile {
                        config.garrison_cap
                    } else {
                        config.invited_garrison_cap
                    };
                    let gain = garrison_bonus_with_cap(power, cap, config.garrison_half_saturation);
                    let _ = self.queue_control_gain(player, gain);
                    if hostile {
                        self.change_relation(player, -config.occupation_relation_loss);
                    }
                }
                if let PoliticalState::Independent {
                    local,
                    shares,
                } = &mut self.state
                {
                    resolve_control(local, shares, &self.pending_gains, &self.pending_reductions);
                }
            },
            PoliticalState::Vassal {
                overlord,
                tribute,
                ..
            } => {
                self.change_relation(overlord, tribute.relation_delta());
                let relation = ((self.relation(overlord) - 50.0) / 10.0).min(0.0);
                let military = if let Some(occupier) =
                    occupation.filter(|player| *player != overlord)
                {
                    self.change_relation(occupier, -config.occupation_relation_loss);
                    -garrison_bonus(stationed_power.get(occupier).copied().unwrap_or(0.0), config)
                } else {
                    garrison_bonus(stationed_power.get(overlord).copied().unwrap_or(0.0), config)
                };
                let control_support = paid_control_support.max(0.0);
                let total = relation + military + control_support;
                self.last_control_change = ControlBreakdown {
                    relation,
                    military,
                    support: control_support,
                    total,
                };
                if let PoliticalState::Vassal {
                    control,
                    ..
                } = &mut self.state
                {
                    *control = (*control + total).clamp(0.0, 100.0);
                }
                self.release_if_unstable();
            },
            PoliticalState::Owned {
                owner,
            } => {
                if self.owned_shares.iter().all(|share| *share == 0.0) {
                    self.owned_shares[owner] = 100.0;
                }
                for player in 0..self.relations.len() {
                    if player == owner && occupation.is_some() {
                        continue;
                    }
                    let power = stationed_power.get(player).copied().unwrap_or(0.0);
                    if power <= 0.0 {
                        continue;
                    }
                    let hostile = occupation == Some(player);
                    let cap = if hostile || player == owner {
                        config.garrison_cap
                    } else {
                        config.invited_garrison_cap
                    };
                    let gain = garrison_bonus_with_cap(power, cap, config.garrison_half_saturation);
                    if player == owner {
                        // A liberating garrison restores the owner's share; public political
                        // pressure remains restricted to foreign claimants.
                        self.pending_gains[player] += gain;
                    } else {
                        let _ = self.queue_control_gain(player, gain);
                    }
                    if hostile {
                        self.change_relation(player, -config.occupation_relation_loss);
                    }
                }
                let mut local = (100.0 - self.owned_shares.iter().sum::<f64>()).max(0.0);
                resolve_control(
                    &mut local,
                    &mut self.owned_shares,
                    &self.pending_gains,
                    &self.pending_reductions,
                );
                // Local rebel Control persists until a player gains that share.
            },
            PoliticalState::Rome => {},
        }
        self.clear_pending();
        self.used_interference.clear();
    }

    /// Return passive vassal income for the economy's normal generation stage.
    pub fn vassal_income(&self, config: &DiplomacyConfig) -> Option<(PlayerId, f64)> {
        match self.state {
            PoliticalState::Vassal {
                overlord,
                control,
                ..
            } => Some((overlord, control * config.vassal_influence_per_control)),
            _ => None,
        }
    }

    /// Return tribute owed from actual available output; callers debit the NPC treasury.
    pub fn tribute_due(&self, available_coin: f64) -> Option<(PlayerId, f64)> {
        match self.state {
            PoliticalState::Vassal {
                overlord,
                tribute,
                ..
            } => Some((overlord, available_coin.max(0.0) * tribute.income_fraction())),
            _ => None,
        }
    }

    /// Drop an unstable vassal to local independence while retaining its relations.
    fn release_if_unstable(&mut self) {
        if matches!(self.state, PoliticalState::Vassal { control, .. } if control <= 0.0) {
            self.state = PoliticalState::Independent {
                local: 100.0,
                shares: vec![0.0; self.relations.len()],
            };
            for support in &mut self.support {
                support.control_coin = false;
                support.control_influence = false;
            }
            self.clear_pending();
        }
    }

    /// Discard the resolved month or invalidated independent-state requests.
    fn clear_pending(&mut self) {
        self.pending_gains.fill(0.0);
        self.pending_reductions.fill(0.0);
        self.trade_this_month.fill(0.0);
    }
}

/// Price one point in the selected currency.
fn point_cost(currency: Currency, config: &DiplomacyConfig) -> f64 {
    match currency {
        Currency::Coin => config.coin_per_point,
        Currency::Influence => config.influence_per_point,
    }
}

/// Reject malformed values before debiting any wallet.
fn valid_amount(amount: f64) -> Result<(), PoliticalError> {
    if amount.is_finite() && amount >= 0.0 {
        Ok(())
    } else {
        Err(PoliticalError::InvalidAmount)
    }
}

/// Political distance tiers; zero steps cost the same as one adjacent step.
pub fn distance_multiplier(steps: Option<usize>) -> Result<f64, PoliticalError> {
    match steps {
        Some(0..=1) => Ok(1.0),
        Some(2..=3) => Ok(1.25),
        Some(4..=5) => Ok(1.5),
        Some(6..=7) => Ok(1.75),
        Some(8..=9) => Ok(2.0),
        Some(10..=11) => Ok(2.25),
        Some(12..=13) => Ok(2.5),
        Some(14..=15) => Ok(2.75),
        Some(_) | None => Ok(3.0),
    }
}

/// Cost resistance at increasing control levels.
pub fn entrenchment(control: f64) -> f64 {
    if control >= 90.0 {
        2.0
    } else if control >= 75.0 {
        1.5
    } else if control >= 50.0 {
        1.25
    } else {
        1.0
    }
}

/// Cost resistance when attacking another player's relationship.
pub fn relation_resistance(relation: f64) -> f64 {
    if relation >= 80.0 {
        2.0
    } else if relation >= 60.0 {
        1.5
    } else if relation >= 40.0 {
        1.25
    } else {
        1.0
    }
}

/// Strength, not unit count, supplies smoothly capped coercive political power.
pub fn garrison_bonus(power: f64, config: &DiplomacyConfig) -> f64 {
    garrison_bonus_with_cap(power, config.garrison_cap, config.garrison_half_saturation)
}

/// Shared saturation curve for peaceful and hostile stationed troops.
pub fn garrison_bonus_with_cap(power: f64, cap: f64, half_saturation: f64) -> f64 {
    let power = power.max(0.0);
    if power == 0.0 {
        0.0
    } else {
        cap * power / (power + half_saturation.max(0.001))
    }
}

/// Shortest province-graph distance from any owned/vassal source, including sea edges.
pub fn political_distance(graph: &[Vec<usize>], sources: &[usize], target: usize) -> Option<usize> {
    if target >= graph.len() {
        return None;
    }
    let mut distance = vec![usize::MAX; graph.len()];
    let mut queue = VecDeque::new();
    for &source in sources {
        if source < graph.len() {
            distance[source] = 0;
            queue.push_back(source);
        }
    }
    while let Some(node) = queue.pop_front() {
        if node == target {
            return Some(distance[node]);
        }
        for &next in &graph[node] {
            if next < graph.len() && distance[next] == usize::MAX {
                distance[next] = distance[node] + 1;
                queue.push_back(next);
            }
        }
    }
    None
}

/// Resolve all positive pressure and explicitly targeted losses from the same month.
/// Transfers in each iteration use a shared snapshot; opposing flows cancel naturally.
pub fn resolve_control(local: &mut f64, shares: &mut [f64], gains: &[f64], reductions: &[f64]) {
    let count = shares.len();
    let mut pressure: Vec<f64> =
        (0..count).map(|id| gains.get(id).copied().unwrap_or(0.0).max(0.0)).collect();
    let total: f64 = pressure.iter().sum();
    let allocated = total.min(*local);
    if total > 0.0 {
        for id in 0..count {
            let gain = allocated * pressure[id] / total;
            shares[id] += gain;
            pressure[id] -= gain;
        }
        *local -= allocated;
    }
    for (id, share) in shares.iter_mut().enumerate() {
        let removed = share.min(reductions.get(id).copied().unwrap_or(0.0).max(0.0));
        *share -= removed;
        *local += removed;
    }
    for _ in 0..count.saturating_mul(count).max(1) + 1 {
        let snapshot = shares.to_vec();
        let mut requests = vec![vec![0.0; count]; count];
        for actor in 0..count {
            if pressure[actor] <= 1e-9 {
                continue;
            }
            let target = (0..count)
                .filter(|id| *id != actor && snapshot[*id] > 1e-9)
                .max_by(|a, b| snapshot[*a].total_cmp(&snapshot[*b]).then_with(|| b.cmp(a)));
            if let Some(target) = target {
                requests[actor][target] = pressure[actor].min(snapshot[target]);
            }
        }
        let mut moved = 0.0;
        for target in 0..count {
            let incoming: f64 = requests.iter().map(|row| row[target]).sum();
            let factor = if incoming > snapshot[target] {
                snapshot[target] / incoming
            } else {
                1.0
            };
            for actor in 0..count {
                let amount = requests[actor][target] * factor;
                shares[actor] += amount;
                shares[target] -= amount;
                pressure[actor] -= amount;
                moved += amount;
            }
        }
        if moved <= 1e-9 || pressure.iter().all(|value| *value <= 1e-9) {
            break;
        }
    }
    for share in shares.iter_mut() {
        *share = share.max(0.0);
    }
    *local = local.max(0.0);
    let total = *local + shares.iter().sum::<f64>();
    if total > 0.0 {
        let factor = 100.0 / total;
        *local *= factor;
        for share in shares {
            *share *= factor;
        }
    } else {
        *local = 100.0;
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/diplomacy.rs"]
mod tests;
