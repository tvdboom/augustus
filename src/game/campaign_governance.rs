//! Nationwide edicts project into owned provinces and paid player troops.

use super::campaign::Campaign;
use crate::game::economy::{FoodPolicy, SlaveLabor};
use crate::game::military::ForceOwner;
use crate::map::{EdictLevel, Governance};

impl Campaign {
    pub(crate) fn governance_for(&self, player: usize) -> Governance {
        self.governance.get(player).copied().unwrap_or_default()
    }

    pub(crate) fn set_governance(&mut self, player: usize, governance: Governance) {
        self.governance.resize(self.actors.len(), Governance::default());
        if let Some(current) = self.governance.get_mut(player) {
            *current = governance;
        }
        self.sync_governance();
    }

    /// Reapply after conquest/integration so an owner's edicts always cover their nation.
    pub(crate) fn sync_governance(&mut self) {
        for province in &mut self.economy.provinces {
            province.noble_tax_multiplier = 1.0;
            province.noble_tax_happiness = 0.0;
            let Some(owner) = province.owner else {
                continue;
            };
            let governance = self.governance.get(owner).copied().unwrap_or_default();
            province.policies.food = match governance.food_rations {
                EdictLevel::Low => FoodPolicy::Low,
                EdictLevel::Medium => FoodPolicy::Normal,
                EdictLevel::High => FoodPolicy::High,
            };
            province.policies.slave_labor = match governance.slave_labor {
                EdictLevel::Low => SlaveLabor::Light,
                EdictLevel::Medium => SlaveLabor::Normal,
                EdictLevel::High => SlaveLabor::Harsh,
            };
            province.noble_tax_multiplier = governance.noble_tax_per_person();
            province.noble_tax_happiness = match governance.noble_taxes {
                EdictLevel::Low => 1.0,
                EdictLevel::Medium => 0.0,
                EdictLevel::High => -1.0,
            };
        }
    }

    pub(crate) fn army_wages(&self, player: usize) -> f64 {
        self.military
            .all_units()
            .filter(|unit| unit.owner == ForceOwner::Player(player))
            .map(|unit| unit.current_manpower)
            .sum::<f64>()
            * self.military.config.coin_per_manpower
            * self.governance_for(player).army_wage_factor()
    }

    pub(crate) fn pay_army_wages(&mut self, player: usize, supply: f64) {
        let requested = self.army_wages(player);
        let wallet = &mut self.economy.players[player];
        let paid = wallet.coin.min(requested);
        wallet.coin -= paid;
        let coverage = if requested > 0.0 {
            paid / requested
        } else {
            1.0
        };
        let bonus = match self.governance_for(player).army_wages {
            EdictLevel::Low => -2.0,
            EdictLevel::Medium => 0.0,
            EdictLevel::High => 1.0,
        };
        self.military.apply_supply_with_wages(
            ForceOwner::Player(player),
            supply,
            bonus * coverage - 2.0 * (1.0 - coverage),
        );
    }
}
