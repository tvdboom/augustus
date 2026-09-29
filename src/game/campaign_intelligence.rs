//! Public province troop composition and morale, with private training and plans redacted.

use super::campaign::Campaign;
use crate::game::military::{ForceOwner, MilitaryWorld, Unit};

impl Campaign {
    /// Administration grants access to local treasury controls, independently of public facts.
    pub(crate) fn administers_province(&self, player: usize, province: usize) -> bool {
        self.politics.get(province).is_some_and(|p| match p.state {
            crate::game::politics::diplomacy::PoliticalState::Owned {
                owner,
            } => owner == player,
            crate::game::politics::diplomacy::PoliticalState::Vassal {
                overlord,
                ..
            } => overlord == player,
            _ => false,
        })
    }

    /// Stationed and fighting troop composition is public in every province.
    pub(crate) fn observes_military(&self, _player: usize, province: usize) -> bool {
        province < self.military.provinces.len()
    }

    /// Current public troops and army morale with foreign orders and training removed.
    pub(crate) fn military_view(&self, player: usize, _include_reports: bool) -> MilitaryWorld {
        let owner = ForceOwner::Player(player);
        let mut view = self.military.clone();
        let redact = |unit: &mut Unit| {
            if unit.owner != owner {
                unit.training = self.military.config.starting_training;
            }
        };
        for state in &mut view.provinces {
            for unit in state.forces.values_mut().flatten() {
                redact(unit);
            }
            state.plans.retain(|&p, _| p == owner);
            state.draft_penalties = [0.0; 4];
        }
        view.movements.retain(|m| m.owner == owner);
        for battle in &mut view.battles {
            for side in [&mut battle.attackers, &mut battle.defenders] {
                side.plans.retain(|&p, _| p == owner);
                let own_ids: Vec<_> =
                    side.units.iter().filter(|u| u.owner == owner).map(|u| u.id).collect();
                for unit in &mut side.units {
                    redact(unit);
                }
                for slot in side.formation.front.iter_mut().chain(&mut side.formation.support) {
                    if slot.is_some_and(|id| !own_ids.contains(&id)) {
                        *slot = None;
                    }
                }
                side.formation.reserves.retain(|id| own_ids.contains(id));
                side.routed.retain(|id| own_ids.contains(id));
                // A foreign flank width is itself a saved deployment preference.
                if own_ids.is_empty() {
                    side.formation.flank_size = 0;
                }
            }
        }
        view
    }
}

#[cfg(test)]
#[path = "../../tests/unit/intelligence.rs"]
mod tests;
