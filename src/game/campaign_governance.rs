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
        }
    }

    pub(crate) fn army_wages(&self, player: usize) -> f64 {
        self.military
            .all_units()
            .filter(|unit| unit.owner == ForceOwner::Player(player))
            .map(|unit| unit.coin_demand(&self.military.config))
            .sum::<f64>()
            * self.governance_for(player).army_wage_factor()
    }

    pub(crate) fn noble_wages(&self, player: usize) -> f64 {
        self.economy
            .provinces
            .iter()
            .filter(|province| province.owner == Some(player))
            .map(|province| province.population[0])
            .sum::<f64>()
            * self.economy.config.noble_wage_per_noble
            * self.governance_for(player).noble_wage_factor()
    }

    pub(crate) fn pay_noble_wages(&mut self, player: usize) {
        let requested = self.noble_wages(player);
        let wallet = &mut self.economy.players[player];
        let paid = wallet.coin.max(0.0).min(requested);
        wallet.coin -= paid;
        let happiness = self.governance_for(player).noble_wage_happiness();
        for (id, province) in self.economy.provinces.iter_mut().enumerate() {
            if province.owner != Some(player) {
                continue;
            }
            let previous = province.happiness[0];
            province.noble_wage_happiness = happiness;
            province.happiness[0] = province.calculate_happiness(&self.economy.config)[0];
            if let Some(report) = self.economy.last_report.province_reports.get_mut(id) {
                report.happiness_delta[0] += province.happiness[0] - previous;
            }
        }
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
        let economy = &self.economy;
        self.military.apply_supply_with_provisioning(
            ForceOwner::Player(player),
            |province| {
                province
                    .and_then(|id| economy.provinces.get(id))
                    .and_then(|province| province.owner)
                    .and_then(|host| {
                        if host == player {
                            Some(supply)
                        } else {
                            economy.last_report.food_supply_ratio.get(host).copied()
                        }
                    })
                    .unwrap_or(supply)
            },
            bonus * coverage - 2.0 * (1.0 - coverage),
        );
    }
}
