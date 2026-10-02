//! Authoritative simultaneous combat with locked owner-specific battle plans.

use super::*;
use std::collections::{BTreeMap, BTreeSet};

/// One coalition sharing frontage while retaining each owner's tactic and rank.
#[derive(Clone, Debug)]
pub struct BattleSide {
    /// Surviving units, including troops routed in this battle.
    pub units: Vec<Unit>,
    /// Cohorts on engagement, retained for inspection after casualties remove survivors.
    pub initial_units: Vec<Unit>,
    /// Plans captured on engagement, immutable through public editing actions.
    pub plans: BTreeMap<ForceOwner, BattlePlan>,
    /// Military ranks captured for the engagement.
    pub ranks: BTreeMap<ForceOwner, MilitaryRank>,
    /// Current frontage, support, and reserves.
    pub formation: Formation,
    /// Troops which may not return to this battle.
    pub routed: BTreeSet<UnitId>,
    /// Units actually committed, used for post-battle training.
    pub participated: BTreeSet<UnitId>,
    /// Starting manpower per owner for casualty reports.
    pub initial_manpower: BTreeMap<ForceOwner, f64>,
    /// Starting political strength per owner for renown calibration.
    pub initial_strength: BTreeMap<ForceOwner, f64>,
}

impl BattleSide {
    /// Construct a coalition and deterministically deploy it.
    pub fn new(
        units: Vec<Unit>,
        mut plans: BTreeMap<ForceOwner, BattlePlan>,
        ranks: BTreeMap<ForceOwner, MilitaryRank>,
        width: usize,
        config: &MilitaryConfig,
    ) -> Self {
        let mut initial_manpower = BTreeMap::new();
        let mut initial_strength = BTreeMap::new();
        for unit in &units {
            *initial_manpower.entry(unit.owner).or_insert(0.) += unit.current_manpower;
            *initial_strength.entry(unit.owner).or_insert(0.) += unit.effective_strength(config);
        }
        for &owner in initial_manpower.keys() {
            if matches!(owner, ForceOwner::Local(_)) {
                plans.entry(owner).or_default().tactic = best_composition_tactic(
                    units.iter().filter(|unit| unit.owner == owner),
                    config,
                );
            }
        }
        let routed = BTreeSet::new();
        let formation = deploy_formation(&units, &plans, width, &routed, config);
        Self {
            initial_units: units.clone(),
            units,
            plans,
            ranks,
            formation,
            routed,
            participated: BTreeSet::new(),
            initial_manpower,
            initial_strength,
        }
    }
    /// Total currently surviving men, including routed survivors.
    pub fn manpower(&self) -> f64 {
        self.units.iter().map(|u| u.current_manpower).sum()
    }
    /// A side breaks once no living, non-routed combatant remains, including reserves.
    pub fn is_broken(&self) -> bool {
        self.units.iter().all(|u| u.current_manpower <= 0. || self.routed.contains(&u.id))
    }
    /// Owner-specific composition fit from surviving, non-routed troops.
    pub fn tactic_fit(&self, owner: ForceOwner, config: &MilitaryConfig) -> f64 {
        let tactic = self.plans.get(&owner).copied().unwrap_or_default().tactic;
        tactic_effectiveness(
            self.units.iter().filter(|u| u.owner == owner && !self.routed.contains(&u.id)),
            tactic,
            config,
        )
    }
}

/// Battlefield winner or mutual destruction. Timeout is an attacker loss.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BattleResult {
    /// Attacker victory.
    AttackerVictory,
    /// Defender victory, including attacker retreat or timeout.
    DefenderVictory,
    /// Both sides have no surviving non-routed troops.
    MutualRout,
}

/// Resolved, authoritative dice and damage factors for one simultaneous round.
#[derive(Clone, Debug)]
pub struct BattleRound {
    /// Completed round number, starting at one.
    pub number: usize,
    /// One six-sided die per coalition; every cohort shares its side's roll.
    pub dice: [u8; 2],
    /// Damage multipliers corresponding to the dice, attacker first.
    pub dice_multipliers: [f64; 2],
    /// Actual manpower-weighted tactic multipliers against engaged targets.
    pub tactics: [BTreeMap<ForceOwner, f64>; 2],
    /// Whole people lost by each side, attacker first.
    pub casualties: [u64; 2],
}

/// Map a d6 roll to the configured damage range; equal bounds disable luck in tests.
pub fn dice_multiplier(roll: u8, config: &MilitaryConfig) -> f64 {
    let fraction = f64::from(roll.clamp(1, 6) - 1) / 5.;
    config.random_range[0] + (config.random_range[1] - config.random_range[0]) * fraction
}

/// Active province battle; tactics are locked until the record is resolved.
#[derive(Clone, Debug)]
pub struct Battle {
    /// Stable battle identity.
    pub id: u64,
    /// Province where combat occurs.
    pub province: ProvinceId,
    /// Territorial owner before battle, used for conquest integration.
    pub territorial_owner: Option<PlayerId>,
    /// Attacker's previous province for preferred retreat.
    pub attacker_origin: Option<ProvinceId>,
    /// Terrain captured at battle start.
    pub terrain: MilitaryTerrain,
    /// Combined defensive fort and city-wall levels.
    pub fortification_level: u32,
    /// Attacking coalition.
    pub attackers: BattleSide,
    /// Defending coalition.
    pub defenders: BattleSide,
    /// Number of completed combat rounds.
    pub round: usize,
    /// Number of completed monthly battle phases.
    pub months: usize,
    /// Completed rounds retained for the battle panel and outcome inspection.
    pub rounds: Vec<BattleRound>,
    /// Result, assigned only once.
    pub result: Option<BattleResult>,
    /// Pending retreat at the next round boundary (true means attacker).
    retreat_requested: Option<bool>,
    /// Owners without an adjacent province where their troops may station.
    pub trapped: BTreeSet<ForceOwner>,
    /// Rome's local defenders fight until destroyed, regardless of morale.
    rome_defense: bool,
    /// Deterministic authoritative RNG state; clients consume resolved state.
    random_state: u64,
}

impl Battle {
    /// Begin a battle using locked plans from both owner groups.
    pub fn new(
        id: u64,
        province: ProvinceId,
        territorial_owner: Option<PlayerId>,
        attacker_origin: Option<ProvinceId>,
        terrain: MilitaryTerrain,
        fortification_level: u32,
        attackers: BattleSide,
        defenders: BattleSide,
        seed: u64,
    ) -> Self {
        Self {
            id,
            province,
            territorial_owner,
            attacker_origin,
            terrain,
            fortification_level,
            attackers,
            defenders,
            round: 0,
            months: 0,
            rounds: Vec::new(),
            result: None,
            retreat_requested: None,
            trapped: BTreeSet::new(),
            rome_defense: false,
            random_state: seed.max(1),
        }
    }
    /// The capital's local defenders cannot rout or lose through the battle deadline.
    pub fn protect_rome_defenders(&mut self) {
        if self.defenders.units.iter().any(|unit| unit.owner == ForceOwner::Local(self.province)) {
            self.rome_defense = true;
            self.trapped.insert(ForceOwner::Local(self.province));
            self.restore_rome_morale();
        }
    }

    fn restore_rome_morale(&mut self) {
        if !self.rome_defense {
            return;
        }
        for unit in &mut self.defenders.units {
            if unit.owner == ForceOwner::Local(self.province) && unit.current_manpower > 0. {
                unit.morale = unit.morale.max(1.);
                self.defenders.routed.remove(&unit.id);
            }
        }
    }
    /// Request retreat without changing the already committed formation.
    pub fn request_retreat(
        &mut self,
        attacker: bool,
        config: &MilitaryConfig,
    ) -> Result<(), MilitaryError> {
        if self.result.is_some() {
            return Err(MilitaryError::InvalidBattle);
        }
        if self.rome_defense && !attacker {
            return Err(MilitaryError::InvalidBattle);
        }
        let side = if attacker {
            &self.attackers
        } else {
            &self.defenders
        };
        if side.units.iter().any(|unit| self.trapped.contains(&unit.owner)) {
            return Err(MilitaryError::NoLegalRoute);
        }
        if self.months < config.minimum_retreat_months {
            return Err(MilitaryError::RetreatTooEarly);
        }
        self.retreat_requested = Some(attacker);
        Ok(())
    }
    /// Resolve up to the configured number of simultaneous rounds this month.
    pub fn advance_month(&mut self, config: &MilitaryConfig) {
        if self.result.is_some() {
            return;
        }
        for _ in 0..config.rounds_per_month {
            self.advance_round(config);
            if self.result.is_some() {
                break;
            }
        }
        self.complete_month(config);
    }
    /// Advance one visible round on the game clock, completing a month every
    /// configured number of rounds. Pausing the clock pauses these rounds too.
    pub fn advance_timed_round(&mut self, config: &MilitaryConfig) {
        if self.result.is_some() {
            return;
        }
        self.advance_round(config);
        if self.result.is_some() || self.round.is_multiple_of(config.rounds_per_month.max(1)) {
            self.complete_month(config);
        }
    }
    fn complete_month(&mut self, config: &MilitaryConfig) {
        self.months += 1;
        if self.result.is_none()
            && !self.rome_defense
            && !self.attackers.units.iter().any(|unit| self.trapped.contains(&unit.owner))
            && self.months >= config.maximum_battle_months
        {
            self.result = Some(BattleResult::DefenderVictory);
        }
    }
    /// Resolve one round from a shared pre-round state, eliminating first-strike bias.
    pub fn advance_round(&mut self, config: &MilitaryConfig) {
        if self.result.is_some() {
            return;
        }
        if self.rome_defense {
            self.trapped.insert(ForceOwner::Local(self.province));
        }
        self.restore_rome_morale();
        for side in [&mut self.attackers, &mut self.defenders] {
            side.routed.retain(|id| {
                !side.units.iter().any(|unit| unit.id == *id && self.trapped.contains(&unit.owner))
            });
            for unit in &side.units {
                if unit.morale < 20. && !self.trapped.contains(&unit.owner) {
                    side.routed.insert(unit.id);
                }
            }
        }
        if let Some(attacker) = self.retreat_requested.take() {
            if (if attacker {
                &self.attackers
            } else {
                &self.defenders
            })
            .units
            .iter()
            .any(|unit| self.trapped.contains(&unit.owner))
            {
                return;
            }
            self.result = Some(if attacker {
                BattleResult::DefenderVictory
            } else {
                BattleResult::AttackerVictory
            });
            return;
        }
        refill_formation(
            &mut self.attackers.formation,
            &self.attackers.units,
            &self.attackers.plans,
            &self.attackers.routed,
            config,
        );
        refill_formation(
            &mut self.defenders.formation,
            &self.defenders.units,
            &self.defenders.plans,
            &self.defenders.routed,
            config,
        );
        if let Some(result) = broken_result(&self.attackers, &self.defenders) {
            self.result = Some(result);
            return;
        }
        let dice = [roll_die(&mut self.random_state), roll_die(&mut self.random_state)];
        let dice_multipliers = dice.map(|roll| dice_multiplier(roll, config));
        let before = [
            self.attackers.units.iter().map(Unit::people).sum::<u64>(),
            self.defenders.units.iter().map(Unit::people).sum::<u64>(),
        ];
        let (defender_losses, attacker_tactics) = attacks(
            &self.attackers,
            &self.defenders,
            self.fortification_level,
            true,
            dice_multipliers[0],
            config,
        );
        let (attacker_losses, defender_tactics) =
            attacks(&self.defenders, &self.attackers, 0, false, dice_multipliers[1], config);
        mark_participating(&mut self.attackers);
        mark_participating(&mut self.defenders);
        apply_losses(&mut self.attackers, attacker_losses, &self.trapped);
        apply_losses(&mut self.defenders, defender_losses, &self.trapped);
        self.restore_rome_morale();
        self.round += 1;
        self.rounds.push(BattleRound {
            number: self.round,
            dice,
            dice_multipliers,
            tactics: [attacker_tactics, defender_tactics],
            casualties: [
                before[0].saturating_sub(self.attackers.units.iter().map(Unit::people).sum()),
                before[1].saturating_sub(self.defenders.units.iter().map(Unit::people).sum()),
            ],
        });
        // Remove destroyed and routed icons immediately, preserving living positions.
        for side in [&mut self.attackers, &mut self.defenders] {
            for slot in side.formation.front.iter_mut().chain(&mut side.formation.support) {
                if slot.is_some_and(|id| {
                    side.routed.contains(&id) || !side.units.iter().any(|unit| unit.id == id)
                }) {
                    *slot = None;
                }
            }
            side.formation.reserves.retain(|id| {
                !side.routed.contains(id) && side.units.iter().any(|unit| unit.id == *id)
            });
        }
        self.result = broken_result(&self.attackers, &self.defenders);
    }
    /// Apply experience/morale rewards once when consuming a finished battle.
    pub fn into_outcome(mut self, config: &MilitaryConfig) -> Option<BattleOutcome> {
        let result = self.result?;
        let attacker_wins = result == BattleResult::AttackerVictory;
        let defender_wins = result == BattleResult::DefenderVictory;
        reward_survivors(&mut self.attackers, attacker_wins, config);
        reward_survivors(&mut self.defenders, defender_wins, config);
        let mut renown = BTreeMap::new();
        let winners = if attacker_wins {
            Some((&self.attackers, &self.defenders))
        } else if defender_wins {
            Some((&self.defenders, &self.attackers))
        } else {
            None
        };
        if let Some((winner, loser)) = winners {
            let own: f64 = winner.initial_strength.values().sum();
            let enemy: f64 = loser.initial_strength.values().sum();
            // Ordinary peer battle (~3 cohorts) gives ~25; tiny opponents never farm a flat award.
            let relative = (enemy / own.max(0.01)).sqrt().clamp(0.25, 2.5);
            let scale = (enemy / 25.).sqrt().clamp(0., 3.);
            let award = (25. * relative * scale).min(100.);
            for (&owner, &strength) in &winner.initial_strength {
                renown.insert(owner, award * strength / own.max(0.01));
            }
        }
        Some(BattleOutcome {
            id: self.id,
            province: self.province,
            territorial_owner: self.territorial_owner,
            attacker_origin: self.attacker_origin,
            result,
            attackers: self.attackers,
            defenders: self.defenders,
            renown,
            rounds: self.round,
            months: self.months,
            round_history: self.rounds,
            terrain: self.terrain,
            fortification_level: self.fortification_level,
        })
    }
}

/// Resolved battle, containing survivors for deterministic reinsertion/retreat.
#[derive(Clone, Debug)]
pub struct BattleOutcome {
    /// Battle identity.
    pub id: u64,
    /// Province where combat took place.
    pub province: ProvinceId,
    /// Previous directly owning player, if any.
    pub territorial_owner: Option<PlayerId>,
    /// Preferred attacking retreat location.
    pub attacker_origin: Option<ProvinceId>,
    /// Outcome classification.
    pub result: BattleResult,
    /// Attacker survivors and casualty baseline.
    pub attackers: BattleSide,
    /// Defender survivors and casualty baseline.
    pub defenders: BattleSide,
    /// Renown distributed by participating strength.
    pub renown: BTreeMap<ForceOwner, f64>,
    /// Rounds fought.
    pub rounds: usize,
    /// Monthly battle phases elapsed.
    pub months: usize,
    /// Dice, bonuses and casualties for each completed round.
    pub round_history: Vec<BattleRound>,
    /// Original battlefield landscape for the result panel.
    pub terrain: MilitaryTerrain,
    /// Defenses present on engagement, retained for the result panel.
    pub fortification_level: u32,
}

impl BattleOutcome {
    /// Principal surviving winner, with stable owner tie-break for allied victories.
    pub fn winner(&self) -> Option<ForceOwner> {
        let units = match self.result {
            BattleResult::AttackerVictory => &self.attackers.units,
            BattleResult::DefenderVictory => &self.defenders.units,
            BattleResult::MutualRout => return None,
        };
        let mut manpower = BTreeMap::new();
        for unit in units {
            *manpower.entry(unit.owner).or_insert(0.) += unit.current_manpower;
        }
        manpower
            .into_iter()
            .max_by(|(a, av), (b, bv)| av.total_cmp(bv).then(b.cmp(a)))
            .map(|(owner, _)| owner)
    }
}

/// Human-readable pre-battle estimate that intentionally does not roll hidden tactics.
#[derive(Clone, Debug)]
pub struct BattleEstimate {
    /// Qualitative assessment, never a guaranteed percentage.
    pub assessment: &'static str,
    /// Known factors used by the estimate.
    pub reasons: Vec<String>,
    /// Actual expected deployed formation including fallbacks.
    pub formation: Formation,
    /// Fit from current real units.
    pub tactic_fit: f64,
}

/// Compare visible troops, training, morale, terrain, and fortification only.
pub fn estimate_battle(
    attackers: &[Unit],
    defenders: &[Unit],
    plan: BattlePlan,
    terrain: MilitaryTerrain,
    fortification: u32,
    attacker_rank: MilitaryRank,
    defender_rank: MilitaryRank,
    config: &MilitaryConfig,
) -> BattleEstimate {
    let width = config.combat_widths[terrain as usize];
    let mut plans = BTreeMap::new();
    for u in attackers {
        plans.insert(u.owner, plan);
    }
    let formation = deploy_formation(attackers, &plans, width, &BTreeSet::new(), config);
    let fit = tactic_effectiveness(attackers.iter(), plan.tactic, config);
    let score = |units: &[Unit], opponents: &[Unit], rank: MilitaryRank, defender: bool| -> f64 {
        units
            .iter()
            .map(|u| {
                let stats = config.unit(u.unit_type);
                let morale = (u.morale + config.rank_morale[rank as usize]).clamp(0., 100.);
                let enemy_weight: f64 = opponents.iter().map(Unit::manpower_ratio).sum();
                let matchup = if enemy_weight > 0. {
                    opponents
                        .iter()
                        .map(|enemy| {
                            enemy.manpower_ratio()
                                * config.matchups[u.unit_type as usize][enemy.unit_type as usize]
                        })
                        .sum::<f64>()
                        / enemy_weight
                } else {
                    1.
                };
                u.manpower_ratio()
                    * stats.offense
                    * matchup
                    * (stats.defense * (1. + config.training_defense * u.training / 100.)).sqrt()
                    * (1. + config.training_attack * u.training / 100.)
                    * (config.strength_morale_base + config.strength_morale_scale * morale / 100.)
                    * if defender {
                        1. + (f64::from(fortification) * config.fort_defense_per_level)
                            .min(config.fort_defense_cap)
                    } else {
                        1.
                    }
            })
            .sum()
    };
    let ratio = score(attackers, defenders, attacker_rank, false)
        / score(defenders, attackers, defender_rank, true).max(0.001);
    let assessment = if ratio > 1.25 {
        "Favorable"
    } else if ratio < 0.8 {
        "Unfavorable"
    } else {
        "Even"
    };
    let mut reasons = vec![
        format!("{} frontage; {} flank slots per side", width, formation.flank_size),
        format!("{} composition fit: {:.0}%", plan.tactic.name(), fit * 100.),
        "Enemy tactic: Unknown; no counter bonus assumed".to_owned(),
    ];
    if fortification > 0 {
        reasons.push(format!("Defender fortification level {fortification}"));
    }
    if let (Some(a), Some(d)) = (
        representative_types(attackers, config).first(),
        representative_types(defenders, config).first(),
    ) {
        reasons.push(format!(
            "{} against {}: {:+.0}% matchup effectiveness",
            a.name(),
            d.name(),
            (config.matchups[*a as usize][*d as usize] - 1.) * 100.
        ));
    }
    BattleEstimate {
        assessment,
        reasons,
        formation,
        tactic_fit: fit,
    }
}

/// Evaluate both broken states together to preserve simultaneous mutual routing.
fn broken_result(a: &BattleSide, d: &BattleSide) -> Option<BattleResult> {
    match (a.is_broken(), d.is_broken()) {
        (true, true) => Some(BattleResult::MutualRout),
        (true, false) => Some(BattleResult::DefenderVictory),
        (false, true) => Some(BattleResult::AttackerVictory),
        _ => None,
    }
}

/// Record only actively deployed participants, excluding uncommitted reserves.
fn mark_participating(side: &mut BattleSide) {
    side.participated
        .extend(side.formation.front.iter().chain(&side.formation.support).flatten().copied());
}

/// Accumulate attacks using immutable pre-round state and owner-specific tactics.
fn attacks(
    source: &BattleSide,
    target: &BattleSide,
    fortification: u32,
    attacking: bool,
    dice: f64,
    config: &MilitaryConfig,
) -> (BTreeMap<UnitId, (f64, f64)>, BTreeMap<ForceOwner, f64>) {
    let mut losses = BTreeMap::new();
    let mut tactics = BTreeMap::<ForceOwner, (f64, f64)>::new();
    let by_id: BTreeMap<_, _> = source.units.iter().map(|u| (u.id, u)).collect();
    let enemy: BTreeMap<_, _> = target.units.iter().map(|u| (u.id, u)).collect();
    let siege: f64 = source
        .formation
        .front
        .iter()
        .chain(&source.formation.support)
        .flatten()
        .filter_map(|id| by_id.get(id))
        .map(|u| config.unit(u.unit_type).siege_power * u.manpower_ratio())
        .sum();
    let fort_bonus = (f64::from(fortification) * config.fort_defense_per_level)
        .min(config.fort_defense_cap)
        * (1. - (siege * config.siege_suppression_per_point).min(config.siege_suppression_cap));
    for (support, row) in [(false, &source.formation.front), (true, &source.formation.support)] {
        for (slot, id) in row.iter().enumerate() {
            let Some(unit) = id.and_then(|id| by_id.get(&id).copied()) else {
                continue;
            };
            let stats = config.unit(unit.unit_type);
            let Some((target_id, exposed)) = find_target(slot, stats.maneuver, &target.formation)
            else {
                continue;
            };
            let Some(victim) = enemy.get(&target_id) else {
                continue;
            };
            // Siege crews used as emergency front-line troops have no screen either.
            let exposed = exposed
                || (victim.unit_type.is_support()
                    && target.formation.front.contains(&Some(victim.id)));
            let target_stats = config.unit(victim.unit_type);
            let tactic = source.plans.get(&unit.owner).copied().unwrap_or_default().tactic;
            let opposing_tactic =
                target.plans.get(&victim.owner).copied().unwrap_or_default().tactic;
            let tactic_multiplier = if tactic.counters(opposing_tactic, config) {
                1. + config.tactic_bonus[tactic as usize] * source.tactic_fit(unit.owner, config)
            } else if opposing_tactic.counters(tactic, config) {
                config.countered_multiplier
            } else {
                1.
            };
            let rank = source.ranks.get(&unit.owner).copied().unwrap_or_default();
            let entry = tactics.entry(unit.owner).or_default();
            entry.0 += tactic_multiplier * unit.current_manpower;
            entry.1 += unit.current_manpower;
            let morale = (unit.morale + config.rank_morale[rank as usize]).clamp(0., 100.);
            let protected =
                support && source.formation.front.get(slot).is_some_and(Option::is_some);
            let attack = stats.offense
                * unit.manpower_ratio()
                * config.matchups[unit.unit_type as usize][victim.unit_type as usize]
                * tactic_multiplier
                * (1. + config.training_attack * unit.training / 100.)
                * (config.strength_morale_base + config.strength_morale_scale * morale / 100.)
                * dice
                * if protected {
                    config.support_effectiveness
                } else {
                    1.
                };
            let defense = target_stats.defense
                * (1. + config.training_defense * victim.training / 100.)
                * if attacking {
                    1. + fort_bonus
                } else {
                    1.
                };
            let ratio = attack / defense.max(0.001);
            // Morale already modifies attack above; defense resists both losses.
            let pressure = ratio
                * if exposed {
                    config.exposed_support_casualties
                } else {
                    1.
                };
            let manpower = config.base_manpower_damage * victim.max_manpower * pressure;
            let morale_loss = config.base_morale_damage * pressure;
            let entry = losses.entry(target_id).or_insert((0., 0.));
            entry.0 += manpower;
            entry.1 += morale_loss;
        }
    }
    (
        losses,
        tactics
            .into_iter()
            .map(|(owner, (sum, weight))| (owner, sum / weight.max(0.001)))
            .collect(),
    )
}

/// Nearest reachable front target, then uncovered support; aligned slots win ties.
fn find_target(slot: usize, maneuver: usize, formation: &Formation) -> Option<(UnitId, bool)> {
    let mut positions: Vec<_> =
        (0..formation.front.len()).filter(|&p| p.abs_diff(slot) <= maneuver).collect();
    positions.sort_by_key(|&p| (p.abs_diff(slot), p));
    for &p in &positions {
        if let Some(id) = formation.front[p] {
            return Some((id, false));
        }
    }
    for p in positions {
        if formation.front[p].is_none() {
            if let Some(id) = formation.support[p] {
                return Some((id, true));
            }
        }
    }
    None
}

/// Advance a small stable server-owned PRNG; no thread-local client randomness is used.
fn roll_die(state: &mut u64) -> u8 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state % 6 + 1) as u8
}

/// Commit summed losses simultaneously and remove only destroyed units.
fn apply_losses(
    side: &mut BattleSide,
    losses: BTreeMap<UnitId, (f64, f64)>,
    trapped: &BTreeSet<ForceOwner>,
) {
    for unit in &mut side.units {
        if let Some(&(men, morale)) = losses.get(&unit.id) {
            unit.morale = (unit.morale - morale).clamp(0., 100.);
            let casualties = (men.max(0.) * PEOPLE_PER_POPULATION).round();
            let survivors = (unit.people() as f64 - casualties).max(0.);
            unit.current_manpower = survivors / PEOPLE_PER_POPULATION;
        }
        if unit.morale < 20. && !trapped.contains(&unit.owner) {
            side.routed.insert(unit.id);
        }
    }
    side.units.retain(|u| u.current_manpower > 0.);
}

/// Award bounded experience to living participants and preserve defeat morale.
fn reward_survivors(side: &mut BattleSide, winner: bool, config: &MilitaryConfig) {
    let mut surviving = BTreeMap::new();
    for unit in &side.units {
        *surviving.entry(unit.owner).or_insert(0.) += unit.current_manpower;
    }
    for unit in &mut side.units {
        if !side.participated.contains(&unit.id) {
            continue;
        }
        let initial = side.initial_manpower.get(&unit.owner).copied().unwrap_or(1.).max(0.001);
        let losses = 1. - surviving.get(&unit.owner).copied().unwrap_or(0.) / initial;
        let extra = if losses >= 0.5 {
            3.
        } else if losses >= 0.25 {
            2.
        } else if losses >= 0.1 {
            1.
        } else {
            0.
        };
        let gain = (config.participation_training
            + extra
            + if winner {
                config.victory_training
            } else {
                0.
            })
        .min(config.maximum_battle_training);
        unit.training = (unit.training + gain).clamp(0., 100.);
    }
}
