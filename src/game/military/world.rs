//! Military phase orchestration and transactional economy integration.

use super::*;
use std::collections::{BTreeMap, BTreeSet};

/// Observable domain events for notifications and territorial/political integration.
#[derive(Clone, Debug)]
pub enum MilitaryEvent {
    /// A fully paid cohort completed training.
    Recruited {
        /// Province receiving the cohort.
        province: ProvinceId,
        /// Actual new unit identity.
        unit: UnitId,
        /// Military owner.
        owner: ForceOwner,
        /// Type completed.
        unit_type: UnitType,
    },
    /// Troops finished an edge and joined a stationary province force.
    Arrived {
        /// Province entered.
        province: ProvinceId,
        /// Previous adjacent province.
        origin: ProvinceId,
        /// Military owner.
        owner: ForceOwner,
        /// Whether entry was hostile.
        invasion: bool,
        /// Locked movement plan to use on engagement.
        plan: BattlePlan,
    },
    /// Changed access stopped an order safely at its last legal province.
    MovementStopped {
        /// Province at which troops remain.
        province: ProvinceId,
        /// Owner affected.
        owner: ForceOwner,
    },
    /// Complete cohorts lost in combat or destroyed when no retreat is legal.
    UnitsDestroyed {
        /// Battlefield where these permanent losses occurred.
        province: ProvinceId,
        /// Owner receiving the loss report.
        owner: ForceOwner,
        /// Whole cohort records removed this month, not individual manpower.
        count: usize,
    },
    /// Battle completed; the integration layer applies territorial transfer immediately.
    BattleEnded {
        /// Province fought over.
        province: ProvinceId,
        /// Territorial owner at engagement.
        previous_owner: Option<PlayerId>,
        /// Principal surviving victor.
        winner: Option<ForceOwner>,
        /// Actual result.
        result: BattleResult,
        /// Battle identifier for history.
        battle: u64,
    },
    /// Renown automatically promoted a military career.
    RankIncreased {
        /// Owner promoted.
        owner: ForceOwner,
        /// New rank.
        rank: MilitaryRank,
    },
    /// An occupying victor established hostile control.
    OccupationEstablished {
        /// Province occupied.
        province: ProvinceId,
        /// Occupying military owner.
        owner: ForceOwner,
    },
}

/// Local authoritative military state; resource ownership stays with the economy.
#[derive(Clone, Debug)]
pub struct MilitaryWorld {
    /// Central editable balance data.
    pub config: MilitaryConfig,
    /// One state per map province, preserving the map's stable indices.
    pub provinces: Vec<ProvinceMilitaryState>,
    /// Units in transit, never duplicated in a province force.
    pub movements: Vec<MovementOrder>,
    /// Active battles, containing their engaged units.
    pub battles: Vec<Battle>,
    /// Cumulative global military renown per owner.
    pub renown: BTreeMap<ForceOwner, f64>,
    /// Latest completed battle records for inspection.
    pub history: Vec<BattleOutcome>,
    next_unit: u64,
    next_order: u64,
    next_battle: u64,
}

impl MilitaryWorld {
    /// Allocate empty province forces; configured NPC troops are seeded separately.
    pub fn new(province_count: usize) -> Self {
        Self {
            config: MilitaryConfig::default(),
            provinces: vec![ProvinceMilitaryState::default(); province_count],
            movements: vec![],
            battles: vec![],
            renown: BTreeMap::new(),
            history: vec![],
            next_unit: 1,
            next_order: 1,
            next_battle: 1,
        }
    }
    /// Create a configured initial cohort; this does not draft existing civilians.
    pub fn seed_unit(
        &mut self,
        province: ProvinceId,
        owner: ForceOwner,
        unit_type: UnitType,
    ) -> Result<UnitId, MilitaryError> {
        if province >= self.provinces.len() {
            return Err(MilitaryError::UnknownProvince);
        }
        let unit = self.make_unit(owner, unit_type, self.config.unit(unit_type).manpower);
        let id = unit.id;
        self.provinces[province].forces.entry(owner).or_default().push(unit);
        self.provinces[province].plans.entry(owner).or_default();
        Ok(id)
    }
    /// Populate the initial local defenders from explicit province setup data.
    pub fn seed_local_defenders(
        &mut self,
        province: ProvinceId,
        name: &str,
    ) -> Result<(), MilitaryError> {
        for kind in initial_defenders(name) {
            self.seed_unit(province, ForceOwner::Local(province), kind)?;
        }
        Ok(())
    }
    /// Current automatically earned military rank.
    pub fn rank(&self, owner: ForceOwner) -> MilitaryRank {
        MilitaryRank::from_renown(self.renown.get(&owner).copied().unwrap_or(0.), &self.config)
    }
    /// Whether any units in a province are engaged, locking recruitment/disband/plan actions.
    pub fn province_in_battle(&self, province: ProvinceId) -> bool {
        self.battles.iter().any(|b| b.province == province)
    }
    /// Save an owner-specific province default only outside battle.
    pub fn set_plan(
        &mut self,
        province: ProvinceId,
        owner: ForceOwner,
        mut plan: BattlePlan,
    ) -> Result<(), MilitaryError> {
        if self.province_in_battle(province) {
            return Err(MilitaryError::InBattle);
        }
        let state = self.provinces.get_mut(province).ok_or(MilitaryError::UnknownProvince)?;
        plan.flank_size = plan.flank_size.clamp(1, 3);
        state.plans.insert(owner, plan);
        Ok(())
    }
    /// Update an in-transit plan before it crosses into combat.
    pub fn set_movement_plan(
        &mut self,
        order: u64,
        owner: ForceOwner,
        mut plan: BattlePlan,
    ) -> Result<(), MilitaryError> {
        let movement = self
            .movements
            .iter_mut()
            .find(|m| m.id == order && m.owner == owner)
            .ok_or(MilitaryError::InvalidUnits)?;
        plan.flank_size = plan.flank_size.clamp(1, 3);
        movement.plan = plan;
        Ok(())
    }
    /// Atomically draft civilians and pay global Metal before creating a project.
    pub fn recruit(
        &mut self,
        province: ProvinceId,
        player: PlayerId,
        kind: UnitType,
        directly_owned: bool,
        tags: &[RecruitmentTag],
        population: &mut [f64; 4],
        metal: &mut f64,
    ) -> Result<(), MilitaryError> {
        if !directly_owned {
            return Err(MilitaryError::NotDirectlyOwned);
        }
        if self.province_in_battle(province) {
            return Err(MilitaryError::InBattle);
        }
        let state = self.provinces.get_mut(province).ok_or(MilitaryError::UnknownProvince)?;
        if state.recruitment.is_some() {
            return Err(MilitaryError::RecruitmentBusy);
        }
        let def = self.config.unit(kind);
        if def.special_tag.is_some_and(|tag| !tags.contains(&tag)) {
            return Err(MilitaryError::MissingRecruitmentTag);
        }
        if !population[def.manpower_class].is_finite()
            || population[def.manpower_class] < def.manpower
        {
            return Err(MilitaryError::InsufficientPopulation);
        }
        if !metal.is_finite() || *metal < def.metal_cost {
            return Err(MilitaryError::InsufficientMetal);
        }
        let fraction = def.manpower / population[def.manpower_class].max(0.001);
        population[def.manpower_class] -= def.manpower;
        *metal -= def.metal_cost;
        state.draft_penalties[def.manpower_class] = (state.draft_penalties[def.manpower_class]
            + (fraction * self.config.draft_happiness_scale)
                .min(self.config.maximum_draft_penalty))
        .min(50.);
        state.recruitment = Some(RecruitmentProject {
            owner: ForceOwner::Player(player),
            unit_type: kind,
            progress: 0.,
            required_progress: def.recruitment_months,
            manpower: def.manpower,
        });
        Ok(())
    }
    /// Cancel a project with no population or equipment refund.
    pub fn cancel_recruitment(
        &mut self,
        province: ProvinceId,
        owner: ForceOwner,
    ) -> Result<(), MilitaryError> {
        let state = self.provinces.get_mut(province).ok_or(MilitaryError::UnknownProvince)?;
        if state.recruitment.as_ref().is_none_or(|r| r.owner != owner) {
            return Err(MilitaryError::InvalidUnits);
        }
        state.recruitment = None;
        Ok(())
    }
    /// Disband only surviving soldiers into their original class in a directly owned province.
    pub fn disband(
        &mut self,
        province: ProvinceId,
        player: PlayerId,
        id: UnitId,
        directly_owned: bool,
        population: &mut [f64; 4],
    ) -> Result<(), MilitaryError> {
        if !directly_owned {
            return Err(MilitaryError::NotDirectlyOwned);
        }
        if self.province_in_battle(province) {
            return Err(MilitaryError::InBattle);
        }
        let state = self.provinces.get_mut(province).ok_or(MilitaryError::UnknownProvince)?;
        let units =
            state.forces.get_mut(&ForceOwner::Player(player)).ok_or(MilitaryError::InvalidUnits)?;
        let index = units.iter().position(|u| u.id == id).ok_or(MilitaryError::InvalidUnits)?;
        let unit = units.remove(index);
        population[self.config.unit(unit.unit_type).manpower_class] += unit.current_manpower;
        Ok(())
    }
    /// Progress one recruitment month; a changed owner cancels without refunds.
    pub fn advance_recruitment(
        &mut self,
        owner: impl Fn(ProvinceId) -> Option<PlayerId>,
    ) -> Vec<MilitaryEvent> {
        let mut complete = vec![];
        for (province, state) in self.provinces.iter_mut().enumerate() {
            for penalty in &mut state.draft_penalties {
                *penalty = (*penalty - self.config.draft_penalty_decay).max(0.);
            }
            if state
                .recruitment
                .as_ref()
                .is_some_and(|r| Some(r.owner) != owner(province).map(ForceOwner::Player))
            {
                state.recruitment = None;
            }
            if let Some(project) = &mut state.recruitment {
                project.progress += 1.;
                if project.progress >= project.required_progress {
                    complete.push((province, state.recruitment.take().unwrap()));
                }
            }
        }
        let mut events = vec![];
        for (province, project) in complete {
            let unit = self.make_unit(project.owner, project.unit_type, project.manpower);
            let id = unit.id;
            self.provinces[province].forces.entry(project.owner).or_default().push(unit);
            events.push(MilitaryEvent::Recruited {
                province,
                unit: id,
                owner: project.owner,
                unit_type: project.unit_type,
            });
        }
        events
    }
    /// Food demand includes stationary, moving, and fighting troops exactly once.
    pub fn food_demand(&self, owner: ForceOwner) -> f64 {
        self.all_units().filter(|u| u.owner == owner).map(|u| u.food_demand(&self.config)).sum()
    }
    /// Inspect every existing unit without introducing an Army abstraction.
    pub fn all_units(&self) -> impl Iterator<Item = &Unit> {
        self.provinces
            .iter()
            .flat_map(|p| p.forces.values().flatten())
            .chain(self.movements.iter().flat_map(|m| m.units.iter()))
            .chain(
                self.battles
                    .iter()
                    .flat_map(|b| b.attackers.units.iter().chain(&b.defenders.units)),
            )
    }
    /// Apply proportional food shortage and peaceful morale/training without healing men.
    pub fn apply_supply(&mut self, owner: ForceOwner, supply_ratio: f64) {
        let supply = supply_ratio.clamp(0., 1.);
        let config = &self.config;
        let update = |unit: &mut Unit, in_battle: bool| {
            if unit.owner != owner {
                return;
            }
            unit.morale =
                (unit.morale - config.shortage_morale_penalty * (1. - supply)).clamp(0., 100.);
            if supply >= config.training_supply_threshold {
                unit.training = (unit.training + config.passive_training).min(100.);
            }
            if supply >= config.training_supply_threshold && !in_battle {
                let target = (config.base_morale + unit.training * 0.1).min(100.);
                if unit.morale < target {
                    unit.morale = (unit.morale + config.morale_recovery).min(target);
                } else {
                    unit.morale = (unit.morale - 1.).max(target);
                }
            }
        };
        for unit in self.provinces.iter_mut().flat_map(|p| p.forces.values_mut().flatten()) {
            update(unit, false);
        }
        for unit in self.movements.iter_mut().flat_map(|m| m.units.iter_mut()) {
            update(unit, false);
        }
        for battle in &mut self.battles {
            for unit in battle.attackers.units.iter_mut().chain(&mut battle.defenders.units) {
                update(unit, true);
            }
        }
    }
    /// Validate and remove a unique selection atomically into a new transient route.
    pub fn order_movement(
        &mut self,
        province: ProvinceId,
        destination: ProvinceId,
        owner: ForceOwner,
        ids: &[UnitId],
        override_plan: Option<BattlePlan>,
        graph: &[MilitaryProvince],
        access: impl Fn(ForceOwner, ProvinceId) -> MilitaryAccess,
    ) -> Result<u64, MilitaryError> {
        let force = self
            .provinces
            .get(province)
            .and_then(|p| p.forces.get(&owner))
            .ok_or(MilitaryError::InvalidUnits)?;
        let units: Vec<_> = force.iter().filter(|u| ids.contains(&u.id)).cloned().collect();
        let route =
            fastest_route(graph, province, destination, owner, &units, &access, &self.config)?;
        self.order_movement_route(province, owner, ids, override_plan, &route, graph, access)
    }

    /// Issue an explicit route after validating every edge with ordinary access rules.
    /// The UI can supply a fastest route, a waypoint route, or a complete hand-picked path.
    pub fn order_movement_route(
        &mut self,
        province: ProvinceId,
        owner: ForceOwner,
        ids: &[UnitId],
        override_plan: Option<BattlePlan>,
        route: &[ProvinceId],
        graph: &[MilitaryProvince],
        access: impl Fn(ForceOwner, ProvinceId) -> MilitaryAccess,
    ) -> Result<u64, MilitaryError> {
        if self.province_in_battle(province) {
            return Err(MilitaryError::InBattle);
        }
        let state = self.provinces.get(province).ok_or(MilitaryError::UnknownProvince)?;
        let force = state.forces.get(&owner).ok_or(MilitaryError::InvalidUnits)?;
        let selected: BTreeSet<_> = ids.iter().copied().collect();
        if selected.is_empty() || selected.len() != ids.len() {
            return Err(MilitaryError::InvalidUnits);
        }
        let units: Vec<_> = force.iter().filter(|u| selected.contains(&u.id)).cloned().collect();
        if units.len() != ids.len() {
            return Err(MilitaryError::InvalidUnits);
        }
        validate_route(graph, province, route, owner, access)?;
        let required = edge_travel_months(
            &graph[province],
            &graph[route[0]],
            median_province_area(graph),
            force_speed(&units, &self.config),
            &self.config,
        );
        let plan =
            override_plan.unwrap_or_else(|| state.plans.get(&owner).copied().unwrap_or_default());
        self.provinces[province]
            .forces
            .get_mut(&owner)
            .unwrap()
            .retain(|u| !selected.contains(&u.id));
        if self.provinces[province].forces.get(&owner).is_some_and(Vec::is_empty)
            && self.provinces[province].occupation == Some(owner)
        {
            self.provinces[province].occupation = None;
        }
        let id = self.next_order;
        self.next_order += 1;
        self.movements.push(MovementOrder {
            id,
            owner,
            units,
            origin: province,
            route: route.to_vec(),
            progress: 0.,
            required_progress: required,
            plan,
        });
        Ok(id)
    }
    /// Advance at most one edge per tick and revalidate diplomatic permissions on every edge.
    pub fn advance_movement(
        &mut self,
        graph: &[MilitaryProvince],
        access: impl Fn(ForceOwner, ProvinceId) -> MilitaryAccess,
    ) -> Vec<MilitaryEvent> {
        let mut events = vec![];
        let mut continuing = vec![];
        let median = median_province_area(graph);
        for mut movement in std::mem::take(&mut self.movements) {
            let Some(destination) = movement.destination() else {
                self.insert_units(movement.origin, movement.units);
                continue;
            };
            if destination >= graph.len()
                || !graph[movement.origin].neighbors.contains(&destination)
                || access(movement.owner, destination) == MilitaryAccess::Blocked
            {
                self.insert_units(movement.origin, movement.units);
                events.push(MilitaryEvent::MovementStopped {
                    province: movement.origin,
                    owner: movement.owner,
                });
                continue;
            }
            movement.progress += 1.;
            if movement.progress + f64::EPSILON < movement.required_progress {
                continuing.push(movement);
                continue;
            }
            let origin = movement.origin;
            let invasion = access(movement.owner, destination) == MilitaryAccess::Invasion;
            movement.route.remove(0);
            // Any hostile arrival stops the order so the caller can start combat before politics.
            if invasion || movement.route.is_empty() || self.province_in_battle(destination) {
                self.insert_units(destination, movement.units);
                self.provinces[destination].plans.insert(movement.owner, movement.plan);
                events.push(MilitaryEvent::Arrived {
                    province: destination,
                    origin,
                    owner: movement.owner,
                    invasion,
                    plan: movement.plan,
                });
            } else {
                let next = movement.route[0];
                if next >= graph.len()
                    || !graph[destination].neighbors.contains(&next)
                    || access(movement.owner, next) == MilitaryAccess::Blocked
                {
                    self.insert_units(destination, movement.units);
                    events.push(MilitaryEvent::MovementStopped {
                        province: destination,
                        owner: movement.owner,
                    });
                } else {
                    movement.origin = destination;
                    movement.progress = 0.;
                    movement.required_progress = edge_travel_months(
                        &graph[destination],
                        &graph[next],
                        median,
                        movement.speed(&self.config),
                        &self.config,
                    );
                    continuing.push(movement);
                }
            }
        }
        self.movements = continuing;
        events
    }
    /// Start opposing owner groups; plans lock and troops are removed from peaceful province lists.
    pub fn start_battle(
        &mut self,
        province: ProvinceId,
        attacking: &[ForceOwner],
        defending: &[ForceOwner],
        territorial_owner: Option<PlayerId>,
        origin: Option<ProvinceId>,
        terrain: MilitaryTerrain,
        fortification: u32,
        seed: u64,
    ) -> Result<u64, MilitaryError> {
        if self.province_in_battle(province) {
            return Err(MilitaryError::InBattle);
        }
        let state = self.provinces.get(province).ok_or(MilitaryError::UnknownProvince)?;
        let a: BTreeSet<_> = attacking.iter().copied().collect();
        let d: BTreeSet<_> = defending.iter().copied().collect();
        if a.is_empty()
            || d.is_empty()
            || !a.is_disjoint(&d)
            || a.iter().chain(&d).any(|o| state.forces.get(o).is_none_or(Vec::is_empty))
        {
            return Err(MilitaryError::InvalidBattle);
        }
        let width = self.config.combat_widths[terrain as usize];
        let gather = |owners: &BTreeSet<ForceOwner>| {
            let units = owners.iter().flat_map(|o| state.forces[o].iter().cloned()).collect();
            let plans = owners
                .iter()
                .map(|&o| (o, state.plans.get(&o).copied().unwrap_or_default()))
                .collect();
            let ranks = owners.iter().map(|&o| (o, self.rank(o))).collect();
            BattleSide::new(units, plans, ranks, width, &self.config)
        };
        let attackers = gather(&a);
        let defenders = gather(&d);
        for owner in a.iter().chain(&d) {
            self.provinces[province].forces.remove(owner);
        }
        let id = self.next_battle;
        self.next_battle += 1;
        self.battles.push(Battle::new(
            id,
            province,
            territorial_owner,
            origin,
            terrain,
            fortification,
            attackers,
            defenders,
            seed,
        ));
        Ok(id)
    }

    /// Join fresh cohorts to an existing battle at the next round boundary.
    /// Existing owners retain their locked plan; a newly allied owner snapshots
    /// its own plan and rank. No existing troop regains lost manpower.
    pub fn join_battle(
        &mut self,
        province: ProvinceId,
        owner: ForceOwner,
        attacker: bool,
    ) -> Result<(), MilitaryError> {
        let battle_index = self
            .battles
            .iter()
            .position(|b| b.province == province)
            .ok_or(MilitaryError::InvalidBattle)?;
        let state = self.provinces.get(province).ok_or(MilitaryError::UnknownProvince)?;
        if state.forces.get(&owner).is_none_or(Vec::is_empty) {
            return Err(MilitaryError::InvalidUnits);
        }
        let battle = &self.battles[battle_index];
        let opposing = if attacker {
            &battle.defenders
        } else {
            &battle.attackers
        };
        if opposing.plans.contains_key(&owner) {
            return Err(MilitaryError::InvalidBattle);
        }
        let rank = self.rank(owner);
        let plan = state.plans.get(&owner).copied().unwrap_or_default();
        let units = self.provinces[province].forces.remove(&owner).unwrap();
        let battle = &mut self.battles[battle_index];
        let side = if attacker {
            &mut battle.attackers
        } else {
            &mut battle.defenders
        };
        side.plans.entry(owner).or_insert(plan);
        side.ranks.entry(owner).or_insert(rank);
        for unit in units {
            *side.initial_manpower.entry(owner).or_insert(0.) += unit.current_manpower;
            *side.initial_strength.entry(owner).or_insert(0.) +=
                unit.effective_strength(&self.config);
            side.formation.reserves.push(unit.id);
            side.units.push(unit);
        }
        Ok(())
    }
    /// Resolve battles, retreat survivors, award renown, and return conquest events.
    pub fn advance_battles(
        &mut self,
        graph: &[MilitaryProvince],
        access: impl Fn(ForceOwner, ProvinceId) -> MilitaryAccess,
    ) -> Vec<MilitaryEvent> {
        let mut events = vec![];
        let mut active = vec![];
        for mut battle in std::mem::take(&mut self.battles) {
            let mut original_counts = BTreeMap::<ForceOwner, usize>::new();
            for unit in battle.attackers.units.iter().chain(&battle.defenders.units) {
                *original_counts.entry(unit.owner).or_default() += 1;
            }
            battle.advance_month(&self.config);
            if battle.result.is_none() {
                let mut survivors = BTreeMap::<ForceOwner, usize>::new();
                for unit in battle.attackers.units.iter().chain(&battle.defenders.units) {
                    *survivors.entry(unit.owner).or_default() += 1;
                }
                record_destroyed_cohorts(
                    &mut events,
                    battle.province,
                    &original_counts,
                    &survivors,
                );
                active.push(battle);
                continue;
            }
            let outcome = battle.into_outcome(&self.config).unwrap();
            let winner = outcome.winner();
            for (&owner, &gain) in &outcome.renown {
                let previous = self.rank(owner);
                *self.renown.entry(owner).or_insert(0.) += gain;
                let rank = self.rank(owner);
                if rank != previous {
                    events.push(MilitaryEvent::RankIncreased {
                        owner,
                        rank,
                    });
                }
            }
            let mut survivors = BTreeMap::<ForceOwner, usize>::new();
            for (attacker, side) in [(true, &outcome.attackers), (false, &outcome.defenders)] {
                let wins = if attacker {
                    outcome.result == BattleResult::AttackerVictory
                } else {
                    outcome.result == BattleResult::DefenderVictory
                };
                for unit in &side.units {
                    if wins {
                        *survivors.entry(unit.owner).or_default() += 1;
                        self.insert_units(outcome.province, vec![unit.clone()]);
                    } else if let Some(destination) = retreat_destination(
                        graph,
                        outcome.province,
                        unit.owner,
                        if attacker {
                            outcome.attacker_origin
                        } else {
                            None
                        },
                        &access,
                    ) {
                        *survivors.entry(unit.owner).or_default() += 1;
                        self.insert_units(destination, vec![unit.clone()]);
                    }
                    // No legal retreat: survivors are destroyed, never teleported or healed.
                }
            }
            record_destroyed_cohorts(&mut events, outcome.province, &original_counts, &survivors);
            if outcome.result == BattleResult::AttackerVictory {
                let local_defender_remains = self.provinces[outcome.province]
                    .forces
                    .get(&ForceOwner::Local(outcome.province))
                    .is_some_and(|units| !units.is_empty());
                // Direct conquest is handled by the ownership event. A neutral
                // local force must never be silently conquered by a player-vs-
                // player battle which it did not join.
                let occupation = if outcome.territorial_owner.is_none() && !local_defender_remains {
                    winner
                } else {
                    None
                };
                self.provinces[outcome.province].occupation = occupation;
                if let Some(owner) = occupation {
                    events.push(MilitaryEvent::OccupationEstablished {
                        province: outcome.province,
                        owner,
                    });
                }
            } else if self.provinces[outcome.province].occupation.is_some_and(|o| {
                self.provinces[outcome.province].forces.get(&o).is_none_or(Vec::is_empty)
            }) {
                self.provinces[outcome.province].occupation = None;
            }
            events.push(MilitaryEvent::BattleEnded {
                province: outcome.province,
                previous_owner: outcome.territorial_owner,
                winner,
                result: outcome.result,
                battle: outcome.id,
            });
            self.history.push(outcome);
            if self.history.len() > 32 {
                self.history.remove(0);
            }
        }
        self.battles = active;
        events
    }
    /// Current effective stationed power excludes troops traveling or fighting.
    pub fn stationed_strength(&self, province: ProvinceId, owner: ForceOwner) -> f64 {
        self.provinces
            .get(province)
            .and_then(|p| p.forces.get(&owner))
            .map(|units| units.iter().map(|u| u.effective_strength(&self.config)).sum())
            .unwrap_or(0.)
    }
    /// Hostile occupation pressure; peaceful foreign presence is always zero.
    pub fn occupation_control(&self, province: ProvinceId, owner: ForceOwner) -> f64 {
        if self.provinces.get(province).is_some_and(|p| p.occupation == Some(owner))
            && !self.province_in_battle(province)
        {
            self.config.garrison_control(self.stationed_strength(province, owner), self.rank(owner))
        } else {
            0.
        }
    }
    /// Overlord garrison contribution for the economy's vassal-control phase.
    pub fn vassal_garrison_control(&self, province: ProvinceId, overlord: PlayerId) -> f64 {
        let owner = ForceOwner::Player(overlord);
        self.config.garrison_control(self.stationed_strength(province, owner), self.rank(owner))
    }
    /// Build a fresh identity and bounded starting state.
    fn make_unit(&mut self, owner: ForceOwner, unit_type: UnitType, manpower: f64) -> Unit {
        let id = self.next_unit;
        self.next_unit += 1;
        Unit {
            id,
            owner,
            unit_type,
            current_manpower: manpower,
            max_manpower: manpower,
            training: self.config.starting_training,
            morale: self.config.base_morale,
        }
    }
    /// Return survivors to their owner groups without losing saved force plans.
    fn insert_units(&mut self, province: ProvinceId, units: Vec<Unit>) {
        if let Some(state) = self.provinces.get_mut(province) {
            for unit in units {
                if unit.current_manpower > 0. {
                    state.plans.entry(unit.owner).or_default();
                    state.forces.entry(unit.owner).or_default().push(unit);
                }
            }
        }
    }
}

/// Emit one owner-scoped permanent-loss fact per battle/month, including trapped survivors.
fn record_destroyed_cohorts(
    events: &mut Vec<MilitaryEvent>,
    province: ProvinceId,
    before: &BTreeMap<ForceOwner, usize>,
    after: &BTreeMap<ForceOwner, usize>,
) {
    for (&owner, &original) in before {
        let count = original.saturating_sub(after.get(&owner).copied().unwrap_or(0));
        if count > 0 {
            events.push(MilitaryEvent::UnitsDestroyed {
                province,
                owner,
                count,
            });
        }
    }
}

/// Explicit initial province defenses, independent of current civilian population.
pub fn initial_defenders(name: &str) -> Vec<UnitType> {
    use UnitType::*;
    match name {
        "Latium" => vec![HeavyInfantry, HeavyInfantry, LightInfantry, Archers, LightCavalry],
        "Achaia" | "Asia" | "Africa Proconsularis" | "Aegyptus" | "Syria" => {
            vec![LightInfantry, LightInfantry, HeavyInfantry, Archers]
        },
        "Britannia" | "Belgica" => vec![LightInfantry, LightInfantry, WarChariots],
        "Arabia" | "Numidia" | "Mauretania Caesariensis" | "Mauretania Tingitana" => {
            vec![LightInfantry, Archers, WarCamels]
        },
        "Armenia Mesopotamia" | "Dacia" => vec![LightInfantry, HorseArchers],
        _ => vec![LightInfantry, Archers],
    }
}

/// Historical recruitment capabilities are explicit data, never inferred from terrain.
pub fn recruitment_tags(name: &str) -> Vec<RecruitmentTag> {
    use RecruitmentTag::*;
    match name {
        "Armenia Mesopotamia" | "Dacia" | "Moesia Inferior" => vec![HorseArchers],
        "Britannia" | "Belgica" | "Lugdunensis" => vec![Chariots],
        "Arabia" | "Aegyptus" | "Numidia" | "Mauretania Caesariensis" | "Mauretania Tingitana" => {
            vec![Camels]
        },
        "Africa Proconsularis" => vec![Elephants],
        _ => vec![],
    }
}
