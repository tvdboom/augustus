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
    /// A paid and earned promotion advanced a military career.
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
    /// Map province states followed by the separate capital, preserving atlas indices.
    pub provinces: Vec<ProvinceMilitaryState>,
    /// Units in transit, never duplicated in a province force.
    pub movements: Vec<MovementOrder>,
    /// Active battles, containing their engaged units.
    pub battles: Vec<Battle>,
    /// Cumulative global military renown per owner.
    pub renown: BTreeMap<ForceOwner, f64>,
    /// Explicit paid career ranks, independent of battle renown.
    pub ranks: BTreeMap<ForceOwner, MilitaryRank>,
    /// Highest combined surviving manpower the owner has fielded at one time.
    pub peak_manpower: BTreeMap<ForceOwner, f64>,
    /// Completed battles won by this owner's participating forces.
    pub victories: BTreeMap<ForceOwner, u32>,
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
            ranks: BTreeMap::new(),
            peak_manpower: BTreeMap::new(),
            victories: BTreeMap::new(),
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
        self.insert_units(province, vec![unit]);
        self.provinces[province].plans.entry(owner).or_default();
        Ok(id)
    }
    /// Raise full and partial cohorts from a population using the roster's draft cost.
    /// The caller removes those residents from the economic population.
    pub fn seed_population_force(
        &mut self,
        province: ProvinceId,
        owner: ForceOwner,
        unit_type: UnitType,
        population: f64,
    ) -> Result<usize, MilitaryError> {
        if province >= self.provinces.len() {
            return Err(MilitaryError::UnknownProvince);
        }
        let definition = self.config.unit(unit_type);
        let equivalents = population / definition.population_cost;
        if !population.is_finite()
            || population <= 0.0
            || !definition.population_cost.is_finite()
            || definition.population_cost <= 0.0
            || !equivalents.is_finite()
            || equivalents.ceil() >= usize::MAX as f64
        {
            return Err(MilitaryError::InvalidUnits);
        }
        let manpower = definition.manpower;
        let units: Vec<_> = (0..equivalents.ceil() as usize)
            .map(|index| {
                let mut unit = self.make_unit(owner, unit_type, manpower);
                let fraction = (equivalents - index as f64).clamp(0.0, 1.0);
                unit.current_manpower =
                    (manpower * fraction * PEOPLE_PER_POPULATION).round() / PEOPLE_PER_POPULATION;
                unit
            })
            .filter(|unit| unit.current_manpower > 0.0)
            .collect();
        let cohorts = units.len();
        if cohorts == 0 {
            return Err(MilitaryError::InvalidUnits);
        }
        // Insert the whole force once so large practice populations do not repeatedly
        // scan every previously raised cohort to update the manpower peak.
        self.insert_units(province, units);
        Ok(cohorts)
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
    /// Current explicitly earned military rank.
    pub fn rank(&self, owner: ForceOwner) -> MilitaryRank {
        self.ranks.get(&owner).copied().unwrap_or_default()
    }
    /// The same career requirements drive purchases, buttons, and opportunity notices.
    pub fn promotion_eligibility(
        &self,
        owner: ForceOwner,
        target: MilitaryRank,
        influence: f64,
    ) -> Result<MilitaryPromotionRequirements, String> {
        let requirements = target
            .promotion_requirements()
            .ok_or_else(|| "Centurion is the starting rank.".to_owned())?;
        if self.rank(owner) != requirements.previous {
            return Err(format!("Become {} first.", requirements.previous.name()));
        }
        let peak =
            self.peak_manpower.get(&owner).copied().unwrap_or(0.0).max(self.total_manpower(owner));
        if peak < requirements.peak_manpower {
            return Err("The army strength milestone has not been reached.".into());
        }
        if self.victories.get(&owner).copied().unwrap_or(0) < requirements.victories {
            return Err("More battle victories are required.".into());
        }
        if !influence.is_finite() || influence + 1e-9 < requirements.influence {
            return Err("Not enough Influence for this promotion.".into());
        }
        Ok(requirements)
    }
    /// Current combined manpower across stationed, traveling, and fighting forces.
    pub fn total_manpower(&self, owner: ForceOwner) -> f64 {
        self.provinces
            .iter()
            .flat_map(|state| state.forces.values().flatten())
            .chain(self.movements.iter().flat_map(|movement| &movement.units))
            .chain(
                self.battles.iter().flat_map(|battle| {
                    battle.attackers.units.iter().chain(&battle.defenders.units)
                }),
            )
            .filter(|unit| unit.owner == owner)
            .map(|unit| unit.current_manpower.max(0.0))
            .sum()
    }
    /// Remember a historical peak even if units are later lost or disbanded.
    pub fn observe_peak_manpower(&mut self, owner: ForceOwner) {
        let current = self.total_manpower(owner);
        let peak = self.peak_manpower.entry(owner).or_default();
        *peak = peak.max(current);
    }
    /// Whether any units in a province are engaged, locking disband and plan actions.
    pub fn province_in_battle(&self, province: ProvinceId) -> bool {
        self.battles.iter().any(|b| b.province == province)
    }
    /// Established occupation blocks the displaced owner's province actions, even during relief combat.
    pub fn province_occupied_by_enemy(&self, province: ProvinceId, owner: ForceOwner) -> bool {
        self.provinces
            .get(province)
            .is_some_and(|state| state.occupation.is_some_and(|occupier| occupier != owner))
    }
    /// Save an owner-specific province default only outside battle.
    pub fn set_plan(
        &mut self,
        province: ProvinceId,
        owner: ForceOwner,
        mut plan: BattlePlan,
    ) -> Result<(), MilitaryError> {
        if self.province_occupied_by_enemy(province, owner) {
            return Err(MilitaryError::Occupied);
        }
        if self.province_in_battle(province) {
            return Err(MilitaryError::InBattle);
        }
        let state = self.provinces.get_mut(province).ok_or(MilitaryError::UnknownProvince)?;
        plan.flank_size = plan.normalized_flank_size();
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
        plan.flank_size = plan.normalized_flank_size();
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
        if self.province_occupied_by_enemy(province, ForceOwner::Player(player)) {
            return Err(MilitaryError::Occupied);
        }
        let state = self.provinces.get_mut(province).ok_or(MilitaryError::UnknownProvince)?;
        let def = self.config.unit(kind);
        if state.recruitment_queue_full() {
            return Err(MilitaryError::RecruitmentBusy);
        }
        if def.special_tag.is_some_and(|tag| !tags.contains(&tag)) {
            return Err(MilitaryError::MissingRecruitmentTag);
        }
        if !population[def.manpower_class].is_finite()
            || population[def.manpower_class] < def.population_cost
        {
            return Err(MilitaryError::InsufficientPopulation);
        }
        if !metal.is_finite() || *metal < def.metal_cost {
            return Err(MilitaryError::InsufficientMetal);
        }
        let fraction = def.population_cost / population[def.manpower_class].max(0.001);
        population[def.manpower_class] -= def.population_cost;
        *metal -= def.metal_cost;
        state.draft_penalties[def.manpower_class] = (state.draft_penalties[def.manpower_class]
            + (self.config.base_draft_penalty + fraction * self.config.draft_happiness_scale)
                .min(self.config.maximum_draft_penalty))
        .min(50.);
        let project = RecruitmentProject {
            owner: ForceOwner::Player(player),
            unit_type: kind,
            progress: 0.,
            required_progress: def.recruitment_months,
            population_cost: def.population_cost,
            cohort_manpower: def.manpower,
            manpower_class: def.manpower_class,
            paid_metal: def.metal_cost,
        };
        if state.recruitment.is_none() {
            state.recruitment = Some(project);
        } else {
            state.recruitment_queue.push_back(project);
        }
        Ok(())
    }
    /// Cancel a project with no population or equipment refund.
    pub fn cancel_recruitment(
        &mut self,
        province: ProvinceId,
        owner: ForceOwner,
    ) -> Result<(), MilitaryError> {
        if self.province_occupied_by_enemy(province, owner) {
            return Err(MilitaryError::Occupied);
        }
        let state = self.provinces.get_mut(province).ok_or(MilitaryError::UnknownProvince)?;
        if state.recruitment.as_ref().is_none_or(|r| r.owner != owner) {
            return Err(MilitaryError::InvalidUnits);
        }
        state.recruitment = state.recruitment_queue.pop_front();
        Ok(())
    }
    /// Refund a waiting cohort's original population and equipment payment atomically.
    pub fn cancel_queued_recruitment(
        &mut self,
        province: ProvinceId,
        player: PlayerId,
        index: usize,
        directly_owned: bool,
        population: &mut [f64; 4],
        metal: &mut f64,
    ) -> Result<(), MilitaryError> {
        if !directly_owned {
            return Err(MilitaryError::NotDirectlyOwned);
        }
        if self.province_occupied_by_enemy(province, ForceOwner::Player(player)) {
            return Err(MilitaryError::Occupied);
        }
        let state = self.provinces.get_mut(province).ok_or(MilitaryError::UnknownProvince)?;
        let project = state.recruitment_queue.get(index).ok_or(MilitaryError::InvalidUnits)?;
        if project.owner != ForceOwner::Player(player) || project.manpower_class >= population.len()
        {
            return Err(MilitaryError::InvalidUnits);
        }
        let project = state.recruitment_queue.remove(index).ok_or(MilitaryError::InvalidUnits)?;
        population[project.manpower_class] += project.population_cost;
        *metal += project.paid_metal;
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
        if self.province_occupied_by_enemy(province, ForceOwner::Player(player)) {
            return Err(MilitaryError::Occupied);
        }
        if self.province_in_battle(province) {
            return Err(MilitaryError::InBattle);
        }
        let state = self.provinces.get_mut(province).ok_or(MilitaryError::UnknownProvince)?;
        let units =
            state.forces.get_mut(&ForceOwner::Player(player)).ok_or(MilitaryError::InvalidUnits)?;
        let index = units.iter().position(|u| u.id == id).ok_or(MilitaryError::InvalidUnits)?;
        let unit = units.remove(index);
        let definition = self.config.unit(unit.unit_type);
        population[definition.manpower_class] += definition.population_cost * unit.manpower_ratio();
        Ok(())
    }
    /// Disband the player's whole stationary army atomically; guests remain untouched.
    pub fn disband_army(
        &mut self,
        province: ProvinceId,
        player: PlayerId,
        directly_owned: bool,
        population: &mut [f64; 4],
    ) -> Result<(), MilitaryError> {
        if !directly_owned {
            return Err(MilitaryError::NotDirectlyOwned);
        }
        if self.province_occupied_by_enemy(province, ForceOwner::Player(player)) {
            return Err(MilitaryError::Occupied);
        }
        if self.province_in_battle(province) {
            return Err(MilitaryError::InBattle);
        }
        let state = self.provinces.get_mut(province).ok_or(MilitaryError::UnknownProvince)?;
        let units = state
            .forces
            .get_mut(&ForceOwner::Player(player))
            .filter(|units| !units.is_empty())
            .ok_or(MilitaryError::InvalidUnits)?;
        for unit in units.drain(..) {
            let definition = self.config.unit(unit.unit_type);
            population[definition.manpower_class] +=
                definition.population_cost * unit.manpower_ratio();
        }
        Ok(())
    }
    /// Progress one recruitment month; a changed owner cancels without refunds.
    pub fn advance_recruitment(
        &mut self,
        owner: impl Fn(ProvinceId) -> Option<PlayerId>,
    ) -> Vec<MilitaryEvent> {
        self.advance_recruitment_with_speed(owner, |_| 1.0)
    }

    /// Apply a funded local work rate; ownership changes still cancel without refunds.
    pub fn advance_recruitment_with_speed(
        &mut self,
        owner: impl Fn(ProvinceId) -> Option<PlayerId>,
        speed: impl Fn(ProvinceId) -> f64,
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
                state.recruitment_queue.clear();
            }
            if let Some(project) = &mut state.recruitment {
                if state.occupation.is_some_and(|occupier| occupier != project.owner) {
                    continue;
                }
                project.progress += speed(province).max(0.0);
                if project.progress >= project.required_progress {
                    complete.push((province, state.recruitment.take().unwrap()));
                    state.recruitment = state.recruitment_queue.pop_front();
                }
            }
        }
        let mut events = vec![];
        for (province, project) in complete {
            let unit = self.make_unit(project.owner, project.unit_type, project.cohort_manpower);
            let id = unit.id;
            self.insert_units(province, vec![unit]);
            if let Some(attacker) = self
                .battles
                .iter()
                .find(|battle| battle.province == province && battle.result.is_none())
                .and_then(|battle| {
                    if battle.attackers.plans.contains_key(&project.owner) {
                        Some(true)
                    } else if battle.defenders.plans.contains_key(&project.owner) {
                        Some(false)
                    } else {
                        None
                    }
                })
            {
                // The freshly inserted owner group and its coalition are known to exist.
                let _ = self.join_battle(province, project.owner, attacker);
            }
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
        self.units_with_province().map(|(_, unit)| unit)
    }

    /// Stationed and fighting troops have a province; marching troops are provisioned by their owner.
    pub fn units_with_province(&self) -> impl Iterator<Item = (Option<ProvinceId>, &Unit)> {
        self.provinces
            .iter()
            .enumerate()
            .flat_map(|(id, p)| p.forces.values().flatten().map(move |unit| (Some(id), unit)))
            .chain(self.movements.iter().flat_map(|m| m.units.iter().map(|unit| (None, unit))))
            .chain(self.battles.iter().flat_map(|b| {
                b.attackers
                    .units
                    .iter()
                    .chain(&b.defenders.units)
                    .map(move |unit| (Some(b.province), unit))
            }))
    }
    /// Apply proportional food shortage and rank-based monthly recovery.
    pub fn apply_supply(&mut self, owner: ForceOwner, supply_ratio: f64) {
        self.apply_supply_with_wages(owner, supply_ratio, 0.0);
    }

    /// Nationwide wage morale adjusts recovery targets without accumulating policy drift.
    pub fn apply_supply_with_wages(
        &mut self,
        owner: ForceOwner,
        supply_ratio: f64,
        wage_morale: f64,
    ) {
        self.apply_supply_with_provisioning(owner, |_| supply_ratio, wage_morale);
    }

    /// Food comes from the province host, while wages remain the army owner's responsibility.
    pub fn apply_supply_with_provisioning(
        &mut self,
        owner: ForceOwner,
        supply_ratio: impl Fn(Option<ProvinceId>) -> f64,
        wage_morale: f64,
    ) {
        let config = &self.config;
        let recovery = config.rank_recovery[self.rank(owner) as usize];
        let update = |unit: &mut Unit, in_battle: bool, province: Option<ProvinceId>| {
            if unit.owner != owner {
                return;
            }
            let supply = supply_ratio(province).clamp(0., 1.);
            unit.morale =
                (unit.morale - config.shortage_morale_penalty * (1. - supply)).clamp(0., 100.);
            if unit.morale <= 0.0 {
                return;
            }
            if supply >= config.training_supply_threshold {
                unit.training = (unit.training + config.passive_training).min(100.);
            }
            if supply >= config.training_supply_threshold && !in_battle {
                let target = (config.base_morale + wage_morale).clamp(0., 100.);
                if unit.morale < target {
                    unit.morale = (unit.morale + recovery).min(target);
                } else {
                    unit.morale = (unit.morale - 1.).max(target);
                }
                let restored = (unit.max_people() as f64 * recovery / 100.).round() as u64;
                unit.current_manpower = (unit.people() + restored).min(unit.max_people()) as f64
                    / PEOPLE_PER_POPULATION;
            }
        };
        for (id, state) in self.provinces.iter_mut().enumerate() {
            for unit in state.forces.values_mut().flatten() {
                update(unit, false, Some(id));
            }
        }
        for unit in self.movements.iter_mut().flat_map(|m| m.units.iter_mut()) {
            update(unit, false, None);
        }
        for battle in &mut self.battles {
            for unit in battle.attackers.units.iter_mut().chain(&mut battle.defenders.units) {
                update(unit, true, Some(battle.province));
            }
        }
        self.observe_peak_manpower(owner);
    }

    /// Unpaid armies lose the full penalty without ordinary wage recovery offsetting it.
    pub fn reduce_morale(
        &mut self,
        owner: ForceOwner,
        points: f64,
        before_wages: Option<&BTreeMap<UnitId, f64>>,
    ) {
        let lower = |unit: &mut Unit| {
            if unit.owner == owner {
                let before = before_wages
                    .and_then(|values| values.get(&unit.id))
                    .copied()
                    .unwrap_or(unit.morale);
                unit.morale = (unit.morale.min(before) - points).max(0.0);
            }
        };
        for unit in self.provinces.iter_mut().flat_map(|state| state.forces.values_mut().flatten())
        {
            lower(unit);
        }
        for unit in self.movements.iter_mut().flat_map(|order| order.units.iter_mut()) {
            lower(unit);
        }
        for battle in &mut self.battles {
            for unit in battle.attackers.units.iter_mut().chain(&mut battle.defenders.units) {
                lower(unit);
            }
        }
    }

    /// Zero-morale cohorts leave every force, including marching and fighting armies.
    pub fn disband_zero_morale(&mut self) -> Vec<(Option<ProvinceId>, Unit)> {
        let mut disbanded = Vec::new();
        for (province, state) in self.provinces.iter_mut().enumerate() {
            for units in state.forces.values_mut() {
                units.retain(|unit| {
                    if unit.morale > 0.0 {
                        true
                    } else {
                        disbanded.push((Some(province), unit.clone()));
                        false
                    }
                });
            }
        }
        for order in &mut self.movements {
            order.units.retain(|unit| {
                if unit.morale > 0.0 {
                    true
                } else {
                    disbanded.push((None, unit.clone()));
                    false
                }
            });
        }
        self.movements.retain(|order| !order.units.is_empty());
        for battle in &mut self.battles {
            for side in [&mut battle.attackers, &mut battle.defenders] {
                side.units.retain(|unit| {
                    if unit.morale > 0.0 {
                        true
                    } else {
                        disbanded.push((Some(battle.province), unit.clone()));
                        false
                    }
                });
            }
        }
        disbanded
    }
    /// Merge only the selected owner's stationary, understrength cohorts.
    pub fn merge_army(
        &mut self,
        province: ProvinceId,
        owner: ForceOwner,
    ) -> Result<usize, MilitaryError> {
        if self.province_in_battle(province) {
            return Err(MilitaryError::InBattle);
        }
        let state = self.provinces.get_mut(province).ok_or(MilitaryError::UnknownProvince)?;
        let units = state.forces.get_mut(&owner).ok_or(MilitaryError::InvalidUnits)?;
        Ok(merge_understrength_cohorts(units))
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
            withdrawing: false,
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
            // A forced peaceful withdrawal waits for a battle rather than joining it.
            if movement.withdrawing && self.province_in_battle(destination) {
                continuing.push(movement);
                continue;
            }
            // Passage never becomes stationing when a battle interrupts a crossing.
            if destination < graph.len()
                && access(movement.owner, destination) == MilitaryAccess::Transit
                && self.province_in_battle(destination)
            {
                continuing.push(movement);
                continue;
            }
            if destination >= graph.len()
                || !graph[movement.origin].neighbors.contains(&destination)
                || (!movement.withdrawing
                    && (access(movement.owner, destination) == MilitaryAccess::Blocked
                        || (movement.route.len() == 1
                            && access(movement.owner, destination) == MilitaryAccess::Transit)))
            {
                // If permission changes during passage, seek a legal stationing province.
                // Keep troops in transit if temporarily stranded instead of creating a garrison.
                if access(movement.owner, movement.origin) == MilitaryAccess::Transit {
                    let escape = (0..graph.len())
                        .filter(|&id| {
                            access(movement.owner, id) == MilitaryAccess::Peaceful
                                && !self.province_in_battle(id)
                        })
                        .filter_map(|id| {
                            fastest_route(
                                graph,
                                movement.origin,
                                id,
                                movement.owner,
                                &movement.units,
                                |owner, next| {
                                    if self.province_in_battle(next) {
                                        MilitaryAccess::Blocked
                                    } else {
                                        access(owner, next)
                                    }
                                },
                                &self.config,
                            )
                            .ok()
                        })
                        .min_by_key(|route| route.len());
                    if let Some(route) = escape {
                        movement.required_progress = edge_travel_months(
                            &graph[movement.origin],
                            &graph[route[0]],
                            median,
                            movement.speed(&self.config),
                            &self.config,
                        );
                        movement.route = route;
                        movement.progress = 0.;
                    }
                    continuing.push(movement);
                    continue;
                }
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
            let invasion = !movement.withdrawing
                && access(movement.owner, destination) == MilitaryAccess::Invasion;
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
                if access(movement.owner, destination) != MilitaryAccess::Transit
                    && (next >= graph.len()
                        || !graph[destination].neighbors.contains(&next)
                        || (!movement.withdrawing
                            && access(movement.owner, next) == MilitaryAccess::Blocked))
                {
                    self.insert_units(destination, movement.units);
                    events.push(MilitaryEvent::MovementStopped {
                        province: destination,
                        owner: movement.owner,
                    });
                } else {
                    movement.origin = destination;
                    movement.progress = 0.;
                    movement.required_progress = if next < graph.len() {
                        edge_travel_months(
                            &graph[destination],
                            &graph[next],
                            median,
                            movement.speed(&self.config),
                            &self.config,
                        )
                    } else {
                        1.
                    };
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

    /// Join fresh cohorts to an existing battle immediately, ready for the next round.
    /// Existing owners retain their locked plan; a newly allied owner snapshots
    /// its own plan and rank.
    pub fn join_battle(
        &mut self,
        province: ProvinceId,
        owner: ForceOwner,
        attacker: bool,
    ) -> Result<(), MilitaryError> {
        let battle_index = self
            .battles
            .iter()
            .position(|b| b.province == province && b.result.is_none())
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
        let mut plan = state.plans.get(&owner).copied().unwrap_or_default();
        if matches!(owner, ForceOwner::Local(_)) {
            plan.tactic = best_composition_tactic(state.forces[&owner].iter(), &self.config);
        }
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
            side.initial_units.push(unit.clone());
            side.units.push(unit);
        }
        refill_formation(&mut side.formation, &side.units, &side.plans, &side.routed, &self.config);
        Ok(())
    }
    /// Resolve battles, retreat survivors, award renown, and return conquest events.
    pub fn advance_battles(
        &mut self,
        graph: &[MilitaryProvince],
        access: impl Fn(ForceOwner, ProvinceId) -> MilitaryAccess,
    ) -> Vec<MilitaryEvent> {
        self.resolve_battles(graph, access, Battle::advance_month)
    }
    /// Resolve one round per active battle for the running game clock.
    pub fn advance_battle_rounds(
        &mut self,
        graph: &[MilitaryProvince],
        access: impl Fn(ForceOwner, ProvinceId) -> MilitaryAccess,
    ) -> Vec<MilitaryEvent> {
        self.resolve_battles(graph, access, Battle::advance_timed_round)
    }
    fn resolve_battles(
        &mut self,
        graph: &[MilitaryProvince],
        access: impl Fn(ForceOwner, ProvinceId) -> MilitaryAccess,
        advance: impl Fn(&mut Battle, &MilitaryConfig),
    ) -> Vec<MilitaryEvent> {
        let mut events = vec![];
        let mut active = vec![];
        for mut battle in std::mem::take(&mut self.battles) {
            battle.trapped.clear();
            for owner in
                battle.attackers.units.iter().chain(&battle.defenders.units).map(|unit| unit.owner)
            {
                if retreat_destination(
                    graph,
                    battle.province,
                    owner,
                    battle.attacker_origin,
                    &access,
                )
                .is_none()
                {
                    battle.trapped.insert(owner);
                }
            }
            let mut original_counts = BTreeMap::<ForceOwner, usize>::new();
            for unit in battle.attackers.units.iter().chain(&battle.defenders.units) {
                *original_counts.entry(unit.owner).or_default() += 1;
            }
            advance(&mut battle, &self.config);
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
                *self.renown.entry(owner).or_insert(0.) += gain;
            }
            let winning_side = match outcome.result {
                BattleResult::AttackerVictory => Some(&outcome.attackers),
                BattleResult::DefenderVictory => Some(&outcome.defenders),
                BattleResult::MutualRout => None,
            };
            if let Some(side) = winning_side {
                for &owner in side.initial_strength.keys() {
                    *self.victories.entry(owner).or_default() += 1;
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
                    let routed = side.routed.contains(&unit.id);
                    if routed || !wins {
                        if let Some(destination) = retreat_destination(
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
                    } else {
                        *survivors.entry(unit.owner).or_default() += 1;
                        self.insert_units(outcome.province, vec![unit.clone()]);
                    }
                    // No legal retreat: survivors are destroyed, never teleported or healed.
                }
            }
            record_destroyed_cohorts(&mut events, outcome.province, &original_counts, &survivors);
            if outcome.result != BattleResult::MutualRout {
                let local_defender_remains = self.provinces[outcome.province]
                    .forces
                    .get(&ForceOwner::Local(outcome.province))
                    .is_some_and(|units| !units.is_empty());
                // A victory establishes occupation, never immediate ownership.
                // Neutral local forces that did not join still block occupation.
                let occupation = if !local_defender_remains {
                    match winner {
                        Some(ForceOwner::Player(player))
                            if outcome.territorial_owner != Some(player) =>
                        {
                            winner
                        },
                        Some(ForceOwner::Local(_)) if outcome.territorial_owner.is_none() => winner,
                        _ => None,
                    }
                } else {
                    None
                };
                let previous_occupation = self.provinces[outcome.province].occupation;
                self.provinces[outcome.province].occupation = occupation;
                if let Some(owner) = occupation.filter(|owner| Some(*owner) != previous_occupation)
                {
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
        let owners: BTreeSet<_> = units.iter().map(|unit| unit.owner).collect();
        if let Some(state) = self.provinces.get_mut(province) {
            for unit in units {
                if unit.current_manpower > 0. {
                    state.plans.entry(unit.owner).or_default();
                    state.forces.entry(unit.owner).or_default().push(unit);
                }
            }
        }
        for owner in owners {
            self.observe_peak_manpower(owner);
        }
    }
}

/// Emit one owner-scoped destroyed-cohort fact per battle or combat month.
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
        "Rome" => {
            let mut guards = vec![HeavyInfantry; 26];
            guards.extend([Archers; 10]);
            guards.extend([HeavyCavalry; 8]);
            guards.extend([LightCavalry; 6]);
            guards
        },
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
        "Numidia" | "Cyrenaica" => vec![Camels, Elephants],
        "Arabia" | "Aegyptus" | "Mauretania Caesariensis" | "Mauretania Tingitana" => {
            vec![Camels]
        },
        "Africa Proconsularis" => vec![Elephants],
        _ => vec![],
    }
}
