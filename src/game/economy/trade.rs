//! Route-validated bilateral exchange and bounded NPC markets.

use super::{BuildingType, EconomyEvent, EconomyWorld, MonthlyInputs, ResourceFocus, TradeConfig};
use std::collections::{BTreeMap, VecDeque};

/// A bundle is a gross commitment; the recipient receives it after route losses.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TradeBundle {
    /// Food, Metal, Stone sent.
    pub resources: [f64; 3],
    /// Coin sent.
    pub coin: f64,
    /// Influence sent, only when explicitly enabled by configuration.
    pub influence: f64,
}

impl TradeBundle {
    /// Apply an identical fulfillment/transport fraction to every promised item.
    pub fn scaled(self, factor: f64) -> Self {
        Self {
            resources: self.resources.map(|value| value * factor),
            coin: self.coin * factor,
            influence: self.influence * factor,
        }
    }

    /// Local coin-equivalent value using the destination NPC's current scarcity.
    pub fn value(self, values: [f64; 3], influence_value: f64) -> f64 {
        self.resources.iter().zip(values).map(|(amount, price)| amount * price).sum::<f64>()
            + self.coin
            + self.influence * influence_value
    }

    /// Reject NaN, infinities, negative obligations, and disabled influence exchange.
    pub fn valid(self, config: &TradeConfig) -> bool {
        self.resources
            .into_iter()
            .chain([self.coin, self.influence])
            .all(|value| value.is_finite() && value >= 0.0)
            && (config.allow_influence || self.influence == 0.0)
    }
}

/// Players use global stores; NPC provinces have abstract physical markets and real coin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TradeParty {
    /// Human/local player index.
    Player(usize),
    /// Independent or vassal province index; owned provinces cannot be NPC parties.
    Npc(usize),
}

/// Immediate exchange or a repeatable monthly commitment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TradeFrequency {
    /// Executes once on acceptance and never produces control.
    OneTime,
    /// Executes once every monthly simulation, unless suspended/cancelled.
    Monthly,
}

/// Explicit agreement lifecycle retained for explanation in the trade UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TradeStatus {
    /// Awaiting the other human player's acceptance.
    Proposed,
    /// Both parties accepted.
    Active,
    /// Last monthly execution failed, but next month may resume.
    Suspended,
    /// A one-time agreement has already executed.
    Completed,
    /// Explicit cancellation or too many consecutive failures.
    Cancelled,
}

/// The same agreement structure supports player barter and NPC value-tested offers.
#[derive(Clone, Debug)]
pub struct TradeAgreement {
    /// Stable ID assigned by `propose_trade`.
    pub id: u64,
    /// Proposing party; currently must be a player.
    pub party_a: TradeParty,
    /// Counterparty, human player or NPC province.
    pub party_b: TradeParty,
    /// Immediate or repeated exchange.
    pub frequency: TradeFrequency,
    /// Gross outgoing commitment from A.
    pub a_gives: TradeBundle,
    /// Gross outgoing commitment from B.
    pub b_gives: TradeBundle,
    /// Lifecycle status.
    pub status: TradeStatus,
    /// Accepted flags for A and B; player trades need both.
    pub accepted: [bool; 2],
    /// Consecutive suspension count; success resets it.
    pub failure_months: u8,
    /// Most recent failure reason for a tooltip.
    pub last_failure: Option<String>,
    /// Actually delivered two-way value, in common base Coin equivalents, at the last execution.
    /// Failed recurring execution resets this to zero, including player-to-player trades.
    pub last_delivered_value: f64,
    /// Fraction of the promised gross bundles actually committed, before transport loss.
    pub last_fulfillment: f64,
    /// Month of the last successful transfer; political profiles can limit rewards to recent trade.
    pub last_executed_month: Option<u32>,
    /// Final monthly delivery after voluntary cancellation notice.
    pub cancellation_month: Option<u32>,
}

impl TradeAgreement {
    /// Construct an unaccepted proposal; no resource transfer happens until accepted.
    pub fn new(
        party_a: TradeParty,
        party_b: TradeParty,
        a_gives: TradeBundle,
        b_gives: TradeBundle,
        frequency: TradeFrequency,
    ) -> Self {
        Self {
            id: 0,
            party_a,
            party_b,
            frequency,
            a_gives,
            b_gives,
            status: TradeStatus::Proposed,
            accepted: [true, false],
            failure_months: 0,
            last_failure: None,
            last_delivered_value: 0.0,
            last_fulfillment: 0.0,
            last_executed_month: None,
            cancellation_month: None,
        }
    }
}

/// Public scarcity band, computed from production relative to internal need.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DemandBand {
    /// Major need relative to local supply.
    SevereShortage,
    /// More demand than output.
    Shortage,
    /// Supply near need.
    #[default]
    Balanced,
    /// Positive exportable production.
    Surplus,
    /// Large excess relative to need.
    LargeSurplus,
}

impl DemandBand {
    /// Clear label used in the NPC economy panel.
    pub fn name(self) -> &'static str {
        match self {
            Self::SevereShortage => "Severe Shortage",
            Self::Shortage => "Shortage",
            Self::Balanced => "Balanced",
            Self::Surplus => "Surplus",
            Self::LargeSurplus => "Large Surplus",
        }
    }
}

/// Abstract physical supply/demand with per-month reservations preventing double sale.
#[derive(Clone, Copy, Debug, Default)]
pub struct NpcResourceMarket {
    /// This month's estimated production.
    pub production_potential: f64,
    /// Civilian/military/development demand.
    pub internal_need: f64,
    /// Sustainable gross exports for this month.
    pub export_capacity: f64,
    /// Imports currently desired before any trade.
    pub import_demand: f64,
    /// Current scarcity-adjusted coin value per unit.
    pub local_unit_value: f64,
    /// Display scarcity classification.
    pub band: DemandBand,
    /// Actual goods received after transport loss, accumulated this month.
    pub imported: f64,
    /// Actual gross goods already sent, accumulated this month.
    pub exported: f64,
}

/// NPC cash persists while physical resources remain a monthly flow abstraction.
#[derive(Clone, Debug)]
pub struct NpcTradeEconomy {
    /// Food, Metal, Stone markets.
    pub resources: [NpcResourceMarket; 3],
    /// Actual coin available, never negative.
    pub coin_treasury: f64,
    /// Optional influence balance when influence trade is enabled.
    pub influence_treasury: f64,
    /// Abstract taxation and urban income generated this month.
    pub monthly_coin_income: f64,
    /// Ordinary internal expenses generated this month.
    pub monthly_coin_expenses: f64,
    /// Sustainable monthly coin imports budget before reservations.
    pub trade_budget: f64,
    /// Coin already promised/paid through this month's executed recurring trades.
    pub coin_spent: f64,
}

impl Default for NpcTradeEconomy {
    /// Small opening treasury supports trade without an unlimited NPC money faucet.
    fn default() -> Self {
        Self {
            resources: [NpcResourceMarket::default(); 3],
            coin_treasury: 200.0,
            influence_treasury: 0.0,
            monthly_coin_income: 0.0,
            monthly_coin_expenses: 0.0,
            trade_budget: 0.0,
            coin_spent: 0.0,
        }
    }
}

/// Political consequences from actual delivered trade; politics applies ownership rules.
#[derive(Clone, Debug, Default)]
pub struct TradePoliticalEffect {
    /// NPC province receiving the relationship/control pressure.
    pub province: usize,
    /// Foreign trading player.
    pub player: usize,
    /// Already capped recurring or small one-time relationship improvement.
    pub relation_gain: f64,
    /// Monthly control pressure; politics additionally enforces the 40-control ceiling.
    pub control_gain: f64,
    /// NPC-valued goods actually delivered across both directions.
    pub delivered_value: f64,
}

/// Economy-level NPC sentiment effect; the campaign also applies political penalties.
#[derive(Clone, Copy, Debug)]
pub struct TradeCancellationEffect {
    /// Province whose agreement was ended.
    pub province: usize,
    /// Cancelling participant.
    pub player: usize,
    /// Positive relation loss applied once.
    pub relation_loss: f64,
}

/// Read-only preflight data for the trade proposal panel.
#[derive(Clone, Debug)]
pub struct TradeQuote {
    /// Shortest valid sequence of province IDs.
    pub route: Vec<usize>,
    /// Shared delivered fraction after distance and road effects.
    pub efficiency: f64,
    /// Actual received bundles, before any NPC supply reduction.
    pub a_receives: TradeBundle,
    /// Actual received bundles, before any NPC supply reduction.
    pub b_receives: TradeBundle,
    /// NPC receive-value requirement divided by its given-value cost, when present.
    pub required_value_ratio: Option<f64>,
    /// Largest common fulfillment fraction supported by current NPC supply/budget/demand.
    pub fulfillment: f64,
}

impl EconomyWorld {
    /// Rebuild NPC markets once at month start, preserving cash and resetting reservations.
    pub fn refresh_npc_markets(&mut self, inputs: &MonthlyInputs) {
        let config = &self.config;
        let trade = &config.trade;
        for (index, province) in self.provinces.iter_mut().enumerate() {
            if province.owner.is_some() {
                continue;
            }
            let military_food = inputs.npc_army_food.get(index).copied().unwrap_or(0.0).max(0.0);
            let civilian_food = province.food_request(config);
            // Re-evaluate food using a balanced baseline, avoiding focus oscillation.
            province.policies.focus = ResourceFocus::Balanced;
            let (_, balanced) = province.production(config);
            if balanced[0] < (civilian_food + military_food) * 0.9 {
                province.policies.focus = ResourceFocus::Food;
            } else {
                let best = (0..3)
                    .max_by(|&a, &b| {
                        (province.potential[a] * trade.base_value[a])
                            .total_cmp(&(province.potential[b] * trade.base_value[b]))
                    })
                    .unwrap_or(0);
                province.policies.focus =
                    [ResourceFocus::Food, ResourceFocus::Metal, ResourceFocus::Stone][best];
                if province.production(config).1[0] < civilian_food + military_food {
                    province.policies.focus = ResourceFocus::Balanced;
                }
            }
            let (_, output) = province.production(config);
            let fort = f64::from(province.level(BuildingType::CityWalls));
            let needs = [
                civilian_food + military_food,
                trade.npc_base_need[1] + military_food * trade.npc_military_metal_need + fort,
                trade.npc_base_need[2]
                    + if province.has_city {
                        trade.npc_city_stone_need
                    } else {
                        0.0
                    }
                    + if province.construction.is_some() {
                        trade.npc_development_stone_need
                    } else {
                        0.0
                    }
                    + fort,
            ];
            for resource in 0..3 {
                let net = output[resource] - needs[resource];
                let ratio = net / needs[resource].max(1.0);
                let [minor, major] = trade.demand_band_thresholds;
                let band = if ratio < -major {
                    DemandBand::SevereShortage
                } else if ratio < -minor {
                    DemandBand::Shortage
                } else if ratio <= minor {
                    DemandBand::Balanced
                } else if ratio <= major {
                    DemandBand::Surplus
                } else {
                    DemandBand::LargeSurplus
                };
                province.market.resources[resource] = NpcResourceMarket {
                    production_potential: output[resource],
                    internal_need: needs[resource],
                    export_capacity: net.max(0.0) * trade.export_share,
                    import_demand: (-net).max(0.0) * trade.import_share,
                    local_unit_value: trade.base_value[resource]
                        * trade.scarcity_multipliers[band as usize],
                    band,
                    imported: 0.0,
                    exported: 0.0,
                };
            }
            let income = province.tax_income(config)
                + if province.has_city {
                    trade.npc_city_income
                } else {
                    0.0
                };
            province.market.monthly_coin_income = income;
            province.market.monthly_coin_expenses = income * trade.npc_expense_share;
            province.market.trade_budget = income * trade.income_budget_share
                + (province.market.coin_treasury * trade.treasury_budget_share)
                    .min(trade.maximum_treasury_draw);
            province.market.coin_spent = 0.0;
        }
    }

    /// Find the shortest permitted route using a deterministic multi-source breadth-first search.
    pub fn trade_route(
        &self,
        a: TradeParty,
        b: TradeParty,
        inputs: &MonthlyInputs,
    ) -> Result<Vec<usize>, String> {
        let players: Vec<usize> = [a, b]
            .into_iter()
            .filter_map(|party| match party {
                TradeParty::Player(p) => Some(p),
                _ => None,
            })
            .collect();
        if players.len() == 2 && inputs.hostile(players[0], players[1]) {
            return Err("The players are at war".into());
        }
        let starts = self.trade_endpoints(a)?;
        let ends = self.trade_endpoints(b)?;
        let mut previous = vec![None; self.provinces.len()];
        let mut visited = vec![false; self.provinces.len()];
        let mut queue = VecDeque::new();
        for start in starts {
            if self.trade_transit_allowed(start, &players, inputs) {
                visited[start] = true;
                queue.push_back(start);
            }
        }
        while let Some(province) = queue.pop_front() {
            if ends.contains(&province) {
                let mut route = vec![province];
                let mut cursor = province;
                while let Some(parent) = previous[cursor] {
                    route.push(parent);
                    cursor = parent;
                }
                route.reverse();
                return Ok(route);
            }
            let mut adjacent = self.adjacency.get(province).cloned().unwrap_or_default();
            adjacent.sort_unstable();
            adjacent.dedup();
            for next in adjacent {
                if next < visited.len()
                    && !visited[next]
                    && self.trade_transit_allowed(next, &players, inputs)
                {
                    visited[next] = true;
                    previous[next] = Some(province);
                    queue.push_back(next);
                }
            }
        }
        Err("No route avoids hostile or closed territory".into())
    }

    /// Route loss applies to both directions and every resource, including currency.
    pub fn route_efficiency(&self, route: &[usize]) -> f64 {
        let extra_steps = route.len().saturating_sub(2) as f64;
        (1.0 - extra_steps * self.config.trade.loss_per_step)
            .clamp(self.config.trade.minimum_efficiency, 1.0)
    }

    /// Validate and quote a proposal; this read-only method never reserves or transfers anything.
    pub fn quote_trade(
        &self,
        agreement: &TradeAgreement,
        inputs: &MonthlyInputs,
    ) -> Result<TradeQuote, String> {
        if !matches!(agreement.party_a, TradeParty::Player(_)) {
            return Err("A player must propose the trade".into());
        }
        if agreement.party_a == agreement.party_b {
            return Err("A party cannot trade with itself".into());
        }
        if !agreement.a_gives.valid(&self.config.trade)
            || !agreement.b_gives.valid(&self.config.trade)
        {
            return Err(
                "Trade amounts must be finite, nonnegative, and use enabled resources".into()
            );
        }
        if agreement.a_gives == TradeBundle::default()
            || agreement.b_gives == TradeBundle::default()
        {
            return Err(
                "Both sides must give something; use a diplomatic gift for donations".into()
            );
        }
        let route = self.trade_route(agreement.party_a, agreement.party_b, inputs)?;
        let efficiency = self.route_efficiency(&route);
        let mut quote = TradeQuote {
            route,
            efficiency,
            a_receives: agreement.b_gives.scaled(efficiency),
            b_receives: agreement.a_gives.scaled(efficiency),
            required_value_ratio: None,
            fulfillment: 1.0,
        };
        if let TradeParty::Npc(npc) = agreement.party_b {
            let TradeParty::Player(player) = agreement.party_a else {
                unreachable!()
            };
            let province = &self.provinces[npc];
            let relation = province.relation_by_player.get(player).copied().unwrap_or(50.0);
            if relation < self.config.trade.minimum_relation {
                return Err("The NPC's relation is too hostile for trade".into());
            }
            let band = ((relation.clamp(0.0, 100.0) / 20.0).floor() as usize).min(4);
            let required = self.config.trade.required_value_ratios[band]
                * if agreement.frequency == TradeFrequency::OneTime {
                    self.config.trade.one_time_margin
                } else {
                    1.0
                }
                * province
                    .trade_ratio_by_player
                    .get(player)
                    .copied()
                    .unwrap_or(1.0)
                    .clamp(0.1, 10.0);
            let values = province.market.resources.map(|market| market.local_unit_value);
            let received = quote.b_receives.value(values, self.config.trade.influence_value);
            // Cost is the NPC's gross export, not what survives at the other end.
            // This prevents transport loss cancelling out of the acceptance test.
            let given = agreement.b_gives.value(values, self.config.trade.influence_value);
            if received + 1e-9 < given * required {
                return Err(format!("NPC requires {required:.2}× its offered value after transport loss ({received:.1} received / {given:.1} given)"));
            }
            quote.required_value_ratio = Some(required);
            quote.fulfillment = self.npc_fulfillment(npc, agreement, efficiency);
            if quote.fulfillment + 1e-9 < self.config.trade.minimum_fulfillment {
                return Err(format!(
                    "NPC supply, import demand, or sestertii budget supports only {:.0}% of this deal",
                    quote.fulfillment * 100.0
                ));
            }
        }
        Ok(quote)
    }

    /// Quote an immediate execution including both parties' current balances.
    /// Monthly proposals use `quote_trade` because future production can fund them.
    pub fn quote_trade_execution(
        &self,
        agreement: &TradeAgreement,
        inputs: &MonthlyInputs,
    ) -> Result<TradeQuote, String> {
        let quote = self.quote_trade(agreement, inputs)?;
        self.check_party_supply(agreement.party_a, agreement.a_gives.scaled(quote.fulfillment))?;
        self.check_party_supply(agreement.party_b, agreement.b_gives.scaled(quote.fulfillment))?;
        Ok(quote)
    }

    /// Record a proposal. NPC acceptance uses the live route, prices, and capacity.
    /// Returns a political effect only if an accepted one-time NPC trade executes now.
    pub fn propose_trade(
        &mut self,
        mut agreement: TradeAgreement,
        inputs: &MonthlyInputs,
    ) -> Result<(u64, Option<TradePoliticalEffect>), String> {
        let quote = self.quote_trade(&agreement, inputs)?;
        if quote.fulfillment + 1e-9 < 1.0 {
            return Err("The NPC cannot sustain the full proposed amounts; reduce the offer before accepting. The 80% tolerance applies only to an existing route.".into());
        }
        agreement.id = self.next_trade_id;
        agreement.accepted = [true, matches!(agreement.party_b, TradeParty::Npc(_))];
        agreement.status = if agreement.accepted[1] {
            TradeStatus::Active
        } else {
            TradeStatus::Proposed
        };
        let effect = if agreement.frequency == TradeFrequency::OneTime && agreement.accepted[1] {
            let effect = self.execute_trade(&mut agreement, inputs)?;
            agreement.status = TradeStatus::Completed;
            effect
        } else {
            None
        };
        self.next_trade_id = self.next_trade_id.saturating_add(1);
        let id = agreement.id;
        self.trades.push(agreement);
        Ok((id, effect))
    }

    /// The named human counterparty must explicitly accept; arbitrary player barter is not price-tested.
    pub fn accept_trade(
        &mut self,
        id: u64,
        player: usize,
        inputs: &MonthlyInputs,
    ) -> Result<(), String> {
        let index =
            self.trades.iter().position(|trade| trade.id == id).ok_or("Unknown agreement")?;
        let mut agreement = self.trades[index].clone();
        if agreement.party_b != TradeParty::Player(player)
            || agreement.status != TradeStatus::Proposed
        {
            return Err("Only the invited player may accept a pending proposal".into());
        }
        self.quote_trade(&agreement, inputs)?;
        agreement.accepted[1] = true;
        agreement.status = TradeStatus::Active;
        if agreement.frequency == TradeFrequency::OneTime {
            self.execute_trade(&mut agreement, inputs)?;
            agreement.status = TradeStatus::Completed;
        }
        self.trades[index] = agreement;
        Ok(())
    }

    /// Either participating human may cancel a recurring route or pending proposal.
    pub fn cancel_trade(
        &mut self,
        id: u64,
        player: usize,
    ) -> Result<Option<TradeCancellationEffect>, String> {
        let trade =
            self.trades.iter_mut().find(|trade| trade.id == id).ok_or("Unknown agreement")?;
        if trade.party_a != TradeParty::Player(player)
            && trade.party_b != TradeParty::Player(player)
        {
            return Err("Only a participant can cancel this trade".into());
        }
        if !matches!(
            trade.status,
            TradeStatus::Proposed | TradeStatus::Active | TradeStatus::Suspended
        ) {
            return Err("This agreement has already ended".into());
        }
        let npc = match (trade.party_a, trade.party_b) {
            (TradeParty::Npc(id), _) | (_, TradeParty::Npc(id)) => Some(id),
            _ => None,
        };
        let effect = npc.filter(|_| trade.status != TradeStatus::Proposed).map(|province| {
            TradeCancellationEffect {
                province,
                player,
                relation_loss: self.config.trade.cancellation_relation_penalty,
            }
        });
        trade.status = TradeStatus::Cancelled;
        if let Some(effect) = effect {
            if let Some(relation) =
                self.provinces[effect.province].relation_by_player.get_mut(player)
            {
                *relation = (*relation - effect.relation_loss).clamp(0.0, 100.0);
            }
        }
        Ok(effect)
    }

    /// Keep any recurring route running for six monthly deliveries before ending without a penalty.
    pub fn schedule_trade_cancellation(&mut self, id: u64, player: usize) -> Result<u32, String> {
        let trade =
            self.trades.iter_mut().find(|trade| trade.id == id).ok_or("Unknown agreement")?;
        if trade.party_a != TradeParty::Player(player)
            && trade.party_b != TradeParty::Player(player)
        {
            return Err("Only a participant can give notice".into());
        }
        if trade.frequency != TradeFrequency::Monthly
            || !matches!(trade.status, TradeStatus::Active | TradeStatus::Suspended)
        {
            return Err("Notice requires an open monthly route".into());
        }
        if trade.cancellation_month.is_some() {
            return Err("This route already has cancellation notice".into());
        }
        let due = self.month.saturating_add(self.config.trade.cancellation_notice_months);
        trade.cancellation_month = Some(due);
        Ok(due)
    }

    /// Recurring agreements execute in stable creation order; reservations prevent overcommitment.
    pub(super) fn advance_trade(
        &mut self,
        inputs: &MonthlyInputs,
        events: &mut Vec<EconomyEvent>,
    ) -> Vec<TradePoliticalEffect> {
        let mut values: BTreeMap<(usize, usize), f64> = BTreeMap::new();
        for index in 0..self.trades.len() {
            let mut trade = self.trades[index].clone();
            if trade.frequency != TradeFrequency::Monthly
                || !matches!(trade.status, TradeStatus::Active | TradeStatus::Suspended)
            {
                continue;
            }
            let next_month = self.month.saturating_add(1);
            if trade.cancellation_month.is_some_and(|due| next_month > due) {
                self.trades[index].status = TradeStatus::Cancelled;
                continue;
            }
            match self.execute_trade(&mut trade, inputs) {
                Ok(effect) => {
                    self.trades[index].last_delivered_value = trade.last_delivered_value;
                    self.trades[index].last_fulfillment = trade.last_fulfillment;
                    self.trades[index].last_executed_month = trade.last_executed_month;
                    self.trades[index].failure_months = 0;
                    self.trades[index].status = TradeStatus::Active;
                    self.trades[index].last_failure = None;
                    if let Some(effect) = effect {
                        *values.entry((effect.province, effect.player)).or_default() +=
                            effect.delivered_value;
                    }
                },
                Err(reason) => {
                    let agreement = &mut self.trades[index];
                    agreement.failure_months = agreement.failure_months.saturating_add(1);
                    agreement.last_failure = Some(reason.clone());
                    agreement.last_delivered_value = 0.0;
                    agreement.last_fulfillment = 0.0;
                    if agreement.failure_months >= self.config.trade.failure_limit {
                        agreement.status = TradeStatus::Cancelled;
                        events.push(EconomyEvent::TradeCancelled {
                            agreement: agreement.id,
                        });
                    } else {
                        agreement.status = TradeStatus::Suspended;
                        events.push(EconomyEvent::TradeSuspended {
                            agreement: agreement.id,
                            reason,
                        });
                    }
                },
            }
            if trade.cancellation_month.is_some_and(|due| next_month >= due)
                && matches!(self.trades[index].status, TradeStatus::Active | TradeStatus::Suspended)
            {
                self.trades[index].status = TradeStatus::Cancelled;
                events.push(EconomyEvent::TradeNoticeCompleted {
                    agreement: trade.id,
                });
            }
        }
        let mut total_by_province = BTreeMap::<usize, f64>::new();
        for ((province, _), value) in &values {
            *total_by_province.entry(*province).or_default() += value;
        }
        values
            .into_iter()
            .map(|((province, player), delivered_value)| {
                let foreign_share =
                    delivered_value / total_by_province[&province].max(f64::EPSILON);
                TradePoliticalEffect {
                    province,
                    player,
                    delivered_value,
                    relation_gain: (delivered_value / self.config.trade.value_per_relation)
                        .min(self.config.trade.relation_cap),
                    control_gain: if self.provinces[province].overlord.is_none() {
                        (delivered_value * self.config.trade.control_scale)
                            .min(self.config.trade.control_cap)
                            * foreign_share
                    } else {
                        0.0
                    },
                }
            })
            .collect()
    }

    /// Commit only after both parties pass affordability and supply checks.
    fn execute_trade(
        &mut self,
        agreement: &mut TradeAgreement,
        inputs: &MonthlyInputs,
    ) -> Result<Option<TradePoliticalEffect>, String> {
        let quote = self.quote_trade_execution(agreement, inputs)?;
        let a_gives = agreement.a_gives.scaled(quote.fulfillment);
        let b_gives = agreement.b_gives.scaled(quote.fulfillment);
        self.transfer_from(agreement.party_a, a_gives, agreement.frequency);
        self.transfer_from(agreement.party_b, b_gives, agreement.frequency);
        self.transfer_to(agreement.party_a, b_gives.scaled(quote.efficiency));
        self.transfer_to(agreement.party_b, a_gives.scaled(quote.efficiency));
        // Immediate trades occur outside the monthly production/consumption stage.
        // Discard overflow now so a paused player cannot bypass storage by trading
        // above capacity and spending the excess before the next monthly clamp.
        if agreement.frequency == TradeFrequency::OneTime {
            for party in [agreement.party_a, agreement.party_b] {
                if let TradeParty::Player(player) = party {
                    self.players[player].clamp_storage();
                }
            }
        }
        agreement.last_delivered_value = (a_gives
            .value(self.config.trade.base_value, self.config.trade.influence_value)
            + b_gives.value(self.config.trade.base_value, self.config.trade.influence_value))
            * quote.efficiency;
        agreement.last_fulfillment = quote.fulfillment;
        agreement.last_executed_month = Some(if agreement.frequency == TradeFrequency::Monthly {
            self.month.saturating_add(1)
        } else {
            self.month
        });
        if let (TradeParty::Player(player), TradeParty::Npc(province)) =
            (agreement.party_a, agreement.party_b)
        {
            let values =
                self.provinces[province].market.resources.map(|market| market.local_unit_value);
            let value = (a_gives.value(values, self.config.trade.influence_value)
                + b_gives.value(values, self.config.trade.influence_value))
                * quote.efficiency;
            let mut relation_gain =
                (value / self.config.trade.value_per_relation).min(self.config.trade.relation_cap);
            if agreement.frequency == TradeFrequency::OneTime {
                let cap =
                    self.config.trade.relation_cap * self.config.trade.one_time_relation_factor;
                let awarded = self.one_time_relation_awarded.entry((province, player)).or_default();
                relation_gain = (relation_gain * self.config.trade.one_time_relation_factor)
                    .min((cap - *awarded).max(0.0));
                *awarded += relation_gain;
            }
            Ok(Some(TradePoliticalEffect {
                province,
                player,
                delivered_value: value,
                relation_gain,
                control_gain: 0.0,
            })) // Monthly control is aggregated across all foreign trading partners.
        } else {
            Ok(None)
        }
    }

    /// Sources and destinations are territorial endpoints, never arbitrary coordinates.
    fn trade_endpoints(&self, party: TradeParty) -> Result<Vec<usize>, String> {
        match party {
            TradeParty::Player(player) => {
                if player >= self.players.len() {
                    return Err("Unknown player".into());
                }
                let points: Vec<_> = self
                    .provinces
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p.owner == Some(player))
                    .map(|(index, _)| index)
                    .collect();
                if points.is_empty() {
                    Err("Player has no province connected to trade".into())
                } else {
                    Ok(points)
                }
            },
            TradeParty::Npc(id) => {
                let p = self.provinces.get(id).ok_or("Unknown NPC province")?;
                if p.name == "Latium" {
                    return Err("Rome does not permit provincial trade".into());
                }
                if p.owner.is_some() {
                    Err("Owned provinces use their player's global economy".into())
                } else {
                    Ok(vec![id])
                }
            },
        }
    }

    /// Hostile owners/overlords and hostile local NPC relations block transit.
    fn trade_transit_allowed(
        &self,
        province: usize,
        players: &[usize],
        inputs: &MonthlyInputs,
    ) -> bool {
        let Some(p) = self.provinces.get(province) else {
            return false;
        };
        players.iter().all(|&player| {
            if p.owner.is_none() && inputs.hostile_npc(player, province) {
                return false;
            }
            if let Some(other) = p.owner.or(p.overlord) {
                if inputs.hostile(player, other) {
                    return false;
                }
                if p.owner.is_some() || other == player {
                    return true;
                }
            }
            p.relation_by_player.get(player).copied().unwrap_or(50.0)
                >= self.config.trade.minimum_relation
        })
    }

    /// Combine export, import, and cash limitations into one bilateral scaling fraction.
    fn npc_fulfillment(&self, npc: usize, agreement: &TradeAgreement, efficiency: f64) -> f64 {
        let market = &self.provinces[npc].market;
        let config = &self.config.trade;
        let one_time = agreement.frequency == TradeFrequency::OneTime;
        let mut fraction: f64 = 1.0;
        for resource in 0..3 {
            let state = market.resources[resource];
            let sent = agreement.b_gives.resources[resource];
            if sent > 0.0 {
                let allowance = state.export_capacity
                    * if one_time {
                        config.one_time_export_multiplier
                    } else {
                        1.0
                    };
                fraction = fraction.min((allowance - state.exported).max(0.0) / sent);
            }
            let received = agreement.a_gives.resources[resource] * efficiency;
            if received > 0.0 {
                let allowance = state.import_demand * config.maximum_import_multiple;
                fraction = fraction.min((allowance - state.imported).max(0.0) / received);
            }
        }
        if agreement.b_gives.coin > 0.0 {
            let allowance = if one_time {
                market.coin_treasury * config.one_time_treasury_share
            } else {
                (market.trade_budget - market.coin_spent).max(0.0).min(market.coin_treasury)
            };
            fraction = fraction.min(allowance / agreement.b_gives.coin);
        }
        if agreement.b_gives.influence > 0.0 {
            fraction = fraction.min(market.influence_treasury / agreement.b_gives.influence);
        }
        fraction.clamp(0.0, 1.0)
    }

    /// Neither side may fund its gross obligation with goods only received later in the same deal.
    fn check_party_supply(&self, party: TradeParty, bundle: TradeBundle) -> Result<(), String> {
        match party {
            TradeParty::Player(id) => {
                let account = self.players.get(id).ok_or("Unknown player")?;
                if (0..3).any(|i| account.resources[i] + 1e-9 < bundle.resources[i])
                    || account.coin + 1e-9 < bundle.coin
                    || account.influence + 1e-9 < bundle.influence
                {
                    return Err(format!("Player {} cannot fulfill the promised bundle", id + 1));
                }
            },
            TradeParty::Npc(id) => {
                let market = &self.provinces[id].market;
                if market.coin_treasury + 1e-9 < bundle.coin
                    || market.influence_treasury + 1e-9 < bundle.influence
                {
                    return Err("NPC treasury cannot fulfill the promised payment".into());
                }
            },
        }
        Ok(())
    }

    /// Debit real wallets or reserve the NPC's abstract monthly export capacity.
    fn transfer_from(&mut self, party: TradeParty, bundle: TradeBundle, frequency: TradeFrequency) {
        match party {
            TradeParty::Player(id) => {
                for (stock, amount) in self.players[id].resources.iter_mut().zip(bundle.resources) {
                    *stock = (*stock - amount).max(0.0);
                }
                self.players[id].coin = (self.players[id].coin - bundle.coin).max(0.0);
                self.players[id].influence =
                    (self.players[id].influence - bundle.influence).max(0.0);
            },
            TradeParty::Npc(id) => {
                let market = &mut self.provinces[id].market;
                for (resource, amount) in market.resources.iter_mut().zip(bundle.resources) {
                    resource.exported += amount;
                }
                market.coin_treasury = (market.coin_treasury - bundle.coin).max(0.0);
                market.influence_treasury = (market.influence_treasury - bundle.influence).max(0.0);
                if frequency == TradeFrequency::Monthly {
                    market.coin_spent += bundle.coin;
                }
            },
        }
    }

    /// Deliver post-transport goods; owned physical reserves clamp at the consistent month boundary.
    fn transfer_to(&mut self, party: TradeParty, bundle: TradeBundle) {
        match party {
            TradeParty::Player(id) => {
                for (stock, amount) in self.players[id].resources.iter_mut().zip(bundle.resources) {
                    *stock += amount;
                }
                self.players[id].coin += bundle.coin;
                self.players[id].influence += bundle.influence;
            },
            TradeParty::Npc(id) => {
                let market = &mut self.provinces[id].market;
                for (resource, amount) in market.resources.iter_mut().zip(bundle.resources) {
                    resource.imported += amount;
                }
                market.coin_treasury += bundle.coin;
                market.influence_treasury += bundle.influence;
            },
        }
    }
}
