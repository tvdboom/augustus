//! Deterministic automatic deployment and composition-based tactic explanations.

use super::*;
use std::collections::{BTreeMap, BTreeSet};

/// Universally available tactics; each counters two, loses to two, and is neutral to one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(usize)]
pub enum CombatTactic {
    /// Direct charge against a mobile line.
    #[default]
    ShockAction,
    /// Mobile wing attack.
    Envelopment,
    /// Ranged harassment and withdrawal.
    Skirmishing,
    /// Feints and flexible mobile forces.
    Deception,
    /// Strong defensive center.
    Bottleneck,
    /// Dense spear and shield formation.
    Phalanx,
}

impl CombatTactic {
    /// UI selection order.
    pub const ALL: [Self; 6] = [
        Self::ShockAction,
        Self::Envelopment,
        Self::Skirmishing,
        Self::Deception,
        Self::Bottleneck,
        Self::Phalanx,
    ];
    /// Player-facing label.
    pub fn name(self) -> &'static str {
        ["Shock Action", "Envelopment", "Skirmishing", "Deception", "Bottleneck", "Phalanx"]
            [self as usize]
    }
    /// Whether this tactic counters the opponent.
    pub fn counters(self, other: Self, config: &MilitaryConfig) -> bool {
        config.counters[self as usize].contains(&other)
    }
}

/// Saved preferences, snapshotted for movement and locked during a battle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BattlePlan {
    /// Preferred center type.
    pub primary_unit_type: UnitType,
    /// Preferred rear reserve and center replacement type.
    pub secondary_unit_type: UnitType,
    /// Preferred wing type.
    pub flank_unit_type: UnitType,
    /// Requested front slots on each wing: 1 through 5.
    pub flank_size: u8,
    /// Tactic for this force.
    pub tactic: CombatTactic,
}

impl Default for BattlePlan {
    fn default() -> Self {
        Self {
            primary_unit_type: UnitType::HeavyInfantry,
            secondary_unit_type: UnitType::LightInfantry,
            flank_unit_type: UnitType::LightCavalry,
            flank_size: 1,
            tactic: CombatTactic::ShockAction,
        }
    }
}

impl BattlePlan {
    /// Normalize a saved choice to the nearest supported wing size.
    pub fn normalized_flank_size(&self) -> u8 {
        [1_u8, 2, 3, 4, 5]
            .into_iter()
            .min_by_key(|&choice| (choice.abs_diff(self.flank_size), choice))
            .unwrap()
    }

    /// Narrow terrain limits each wing to one third of frontage.
    pub fn effective_flank_size(&self, width: usize) -> usize {
        usize::from(self.normalized_flank_size()).min(width / 3)
    }
}

/// Front and support slots contain stable unit identities, never duplicate units.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Formation {
    /// Left to right front row.
    pub front: Vec<Option<UnitId>>,
    /// Support protected only by an occupied aligned front slot.
    pub support: Vec<Option<UnitId>>,
    /// Uncommitted surviving units.
    pub reserves: Vec<UnitId>,
    /// Front slots on each wing.
    pub flank_size: usize,
}

/// Calculate fit from actual surviving fractional cohorts; preferred labels have no effect.
pub fn tactic_effectiveness<'a>(
    units: impl Iterator<Item = &'a Unit>,
    tactic: CombatTactic,
    config: &MilitaryConfig,
) -> f64 {
    let (fit, weight) =
        units.filter(|u| u.current_manpower > 0.).fold((0., 0.), |(fit, weight), unit| {
            let w = unit.manpower_ratio();
            (fit + w * config.tactic_fit[unit.unit_type as usize][tactic as usize], weight + w)
        });
    if weight > 0. {
        (fit / weight).clamp(0., 1.)
    } else {
        0.
    }
}

/// Choose the highest composition fit, keeping the first tactic when fits tie.
pub fn best_composition_tactic<'a>(
    units: impl Iterator<Item = &'a Unit> + Clone,
    config: &MilitaryConfig,
) -> CombatTactic {
    CombatTactic::ALL
        .into_iter()
        .map(|tactic| (tactic, tactic_effectiveness(units.clone(), tactic, config)))
        .reduce(|best, candidate| {
            if candidate.1 > best.1 {
                candidate
            } else {
                best
            }
        })
        .unwrap()
        .0
}

/// Deterministic deployment with per-owner plans and strongest same-type cohorts first.
/// `routed` troops cannot return to the current battle.
pub fn deploy_formation(
    units: &[Unit],
    plans: &BTreeMap<ForceOwner, BattlePlan>,
    width: usize,
    routed: &BTreeSet<UnitId>,
    config: &MilitaryConfig,
) -> Formation {
    let flank_size =
        plans.values().map(|p| p.effective_flank_size(width)).max().unwrap_or(1).min(width / 3);
    let mut formation = Formation {
        front: vec![None; width],
        support: vec![None; width],
        reserves: vec![],
        flank_size,
    };
    let mut available: Vec<&Unit> =
        units.iter().filter(|u| u.current_manpower > 0. && !routed.contains(&u.id)).collect();
    // Undersized forces occupy a centered contiguous frontage. Otherwise one
    // lone infantry cohort would be stranded in an outer wing unable to engage
    // another small force with a different requested flank size.
    let occupied_width = available.len().min(width);
    let left = width.saturating_sub(occupied_width) / 2;
    let right = left + occupied_width;
    let occupied_flanks = flank_size.min(occupied_width / 3);
    // Keep the selected rear-line type out of the front when other combat
    // cohorts can fill it. Otherwise a preferred flank can consume the only
    // rear-line cohort before the rear row is considered at all.
    let rear_capacity = available
        .iter()
        .filter(|unit| !unit.unit_type.is_support())
        .count()
        .saturating_sub(occupied_width)
        .min(width.saturating_sub(2 * flank_size));
    let mut rear_candidates: Vec<_> = available
        .iter()
        .copied()
        .filter(|unit| {
            let plan = plans.get(&unit.owner).copied().unwrap_or_default();
            !unit.unit_type.is_support() && unit.unit_type == plan.secondary_unit_type
        })
        .collect();
    rear_candidates.sort_by(|a, b| {
        b.manpower_ratio()
            .total_cmp(&a.manpower_ratio())
            .then_with(|| b.training.total_cmp(&a.training))
            .then_with(|| b.morale.total_cmp(&a.morale))
            .then(a.id.cmp(&b.id))
    });
    let reserved: Vec<_> = rear_candidates.into_iter().take(rear_capacity).collect();
    let reserved_ids: BTreeSet<_> = reserved.iter().map(|unit| unit.id).collect();
    available.retain(|unit| !reserved_ids.contains(&unit.id));
    // Wings receive their preferred mobile type before the center is populated.
    for slot in (left..left + occupied_flanks).chain(right.saturating_sub(occupied_flanks)..right) {
        formation.front[slot] = take_best(&mut available, plans, true, false, false, config);
    }
    for slot in left + occupied_flanks..right.saturating_sub(occupied_flanks) {
        formation.front[slot] = take_best(&mut available, plans, false, false, false, config);
    }
    available.extend(reserved);
    let mut support_slots: Vec<_> = (0..width).collect();
    support_slots.sort_by_key(|&slot| (formation.front[slot].is_none(), slot));
    for slot in support_slots {
        formation.support[slot] = take_best(&mut available, plans, false, true, false, config);
    }
    available.sort_by_key(|u| u.id);
    formation.reserves = available.iter().map(|u| u.id).collect();
    formation
}

/// Refill casualties from reserves without shifting surviving active troops.
pub fn refill_formation(
    formation: &mut Formation,
    units: &[Unit],
    plans: &BTreeMap<ForceOwner, BattlePlan>,
    routed: &BTreeSet<UnitId>,
    config: &MilitaryConfig,
) {
    let alive: BTreeSet<UnitId> = units
        .iter()
        .filter(|u| u.current_manpower > 0. && !routed.contains(&u.id))
        .map(|u| u.id)
        .collect();
    for slot in formation.front.iter_mut().chain(formation.support.iter_mut()) {
        if slot.is_some_and(|id| !alive.contains(&id)) {
            *slot = None;
        }
    }
    let active: BTreeSet<_> =
        formation.front.iter().chain(&formation.support).flatten().copied().collect();
    let mut available: Vec<_> =
        units.iter().filter(|u| alive.contains(&u.id) && !active.contains(&u.id)).collect();
    let width = formation.front.len();
    let mut slots: Vec<_> = (0..width).collect();
    // New arriving cohorts fill close to the occupied center before far empty
    // wings, while replacement types still follow each slot's original role.
    slots.sort_by_key(|&slot| ((slot * 2 + 1).abs_diff(width), slot));
    for slot in slots {
        if formation.front[slot].is_none() {
            let flank =
                slot < formation.flank_size || slot >= width.saturating_sub(formation.flank_size);
            formation.front[slot] = take_best(&mut available, plans, flank, false, true, config);
        }
    }
    for slot in &mut formation.support {
        if slot.is_none() {
            *slot = take_best(&mut available, plans, false, true, true, config);
        }
    }
    available.sort_by_key(|u| u.id);
    formation.reserves = available.iter().map(|u| u.id).collect();
}

/// Select a type by role preference, then manpower ratio/training/morale/id.
fn take_best(
    available: &mut Vec<&Unit>,
    plans: &BTreeMap<ForceOwner, BattlePlan>,
    flank: bool,
    support: bool,
    replacement: bool,
    config: &MilitaryConfig,
) -> Option<UnitId> {
    let score = |u: &Unit| {
        let plan = plans.get(&u.owner).copied().unwrap_or_default();
        if support {
            config.support_priority.iter().position(|&k| k == u.unit_type).unwrap_or(usize::MAX)
        } else if flank {
            if u.unit_type == plan.flank_unit_type {
                0
            } else {
                1 + config.flank_priority.iter().position(|&k| k == u.unit_type).unwrap_or(11)
            }
        } else if replacement && u.unit_type == plan.secondary_unit_type {
            0
        } else if replacement && u.unit_type == plan.primary_unit_type {
            1
        } else if u.unit_type == plan.primary_unit_type {
            0
        } else if u.unit_type == plan.secondary_unit_type {
            1
        } else {
            2 + config.center_priority.iter().position(|&k| k == u.unit_type).unwrap_or(11)
        }
    };
    let best = available
        .iter()
        .enumerate()
        .filter(|(_, u)| !support || u.unit_type.is_support())
        .min_by(|(_, a), (_, b)| {
            score(a)
                .cmp(&score(b))
                .then_with(|| b.manpower_ratio().total_cmp(&a.manpower_ratio()))
                .then_with(|| b.training.total_cmp(&a.training))
                .then_with(|| b.morale.total_cmp(&a.morale))
                .then(a.id.cmp(&b.id))
        })
        .map(|(i, _)| i)?;
    Some(available.remove(best).id)
}

/// Select dominant and significant secondary types for a close-zoom map cluster.
pub fn representative_types(units: &[Unit], config: &MilitaryConfig) -> Vec<UnitType> {
    let mut totals = [0.; 11];
    for unit in units {
        totals[unit.unit_type as usize] += unit.current_manpower.max(0.);
    }
    let total: f64 = totals.iter().sum();
    if total <= 0. {
        return vec![];
    }
    let mut kinds = UnitType::ALL.to_vec();
    kinds.sort_by(|a, b| totals[*b as usize].total_cmp(&totals[*a as usize]).then(a.cmp(b)));
    kinds
        .into_iter()
        .enumerate()
        .filter(|(i, k)| {
            *i == 0 || totals[*k as usize] / total >= config.representative_share_threshold
        })
        .take(config.maximum_representatives)
        .map(|(_, k)| k)
        .collect()
}
