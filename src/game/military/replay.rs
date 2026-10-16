//! Compact, authoritative visual recordings; playback never runs combat again.
use super::*;
use std::collections::BTreeSet;

/// One coalition's changing visual state; immutable unit identities stay in initial_units.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BattleReplaySide {
    /// Cohort ID, surviving manpower, morale, and training, in original unit order.
    pub conditions: Vec<(UnitId, f64, f64, f64)>,
    /// Deployment after replacements have resolved.
    pub formation: Formation,
    /// Routed cohorts at this moment.
    pub routed: BTreeSet<UnitId>,
    /// Engagement baseline so later reinforcements do not appear before arrival.
    #[serde(with = "crate::multiplayer::patch::pairs")]
    pub initial_manpower: BTreeMap<ForceOwner, f64>,
}

impl BattleReplaySide {
    fn capture(side: &BattleSide) -> Self {
        Self {
            conditions: side
                .units
                .iter()
                .map(|u| (u.id, u.current_manpower, u.morale, u.training))
                .collect(),
            formation: side.formation.clone(),
            routed: side.routed.clone(),
            initial_manpower: side.initial_manpower.clone(),
        }
    }

    /// Restore a disposable display side using the saved immutable cohort identities.
    pub fn restore(&self, final_side: &BattleSide) -> BattleSide {
        let originals: BTreeMap<_, _> =
            final_side.initial_units.iter().map(|unit| (unit.id, unit)).collect();
        let mut side = final_side.clone();
        side.units = self
            .conditions
            .iter()
            .filter_map(|&(id, manpower, morale, training)| {
                let mut unit = (*originals.get(&id)?).clone();
                unit.current_manpower = manpower;
                unit.morale = morale;
                unit.training = training;
                Some(unit)
            })
            .collect();
        side.formation = self.formation.clone();
        side.routed = self.routed.clone();
        side.initial_manpower = self.initial_manpower.clone();
        side.plans.retain(|owner, _| self.initial_manpower.contains_key(owner));
        side.ranks.retain(|owner, _| self.initial_manpower.contains_key(owner));
        side
    }
}

/// Both armies at the start of combat or immediately after a recorded round.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BattleReplayFrame {
    /// Attacker then defender, preserving battlefield orientation.
    pub sides: [BattleReplaySide; 2],
}

impl BattleReplayFrame {
    pub(super) fn capture(attackers: &BattleSide, defenders: &BattleSide) -> Self {
        Self {
            sides: [BattleReplaySide::capture(attackers), BattleReplaySide::capture(defenders)],
        }
    }
}
