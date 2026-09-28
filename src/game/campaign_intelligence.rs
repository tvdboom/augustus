//! Player-scoped, exact provincial intelligence. Reports never contain national wallets,
//! battle plans, movement orders, or information from another province.

use super::campaign::Campaign;
use crate::game::economy::{ConstructionPace, ConstructionProject, ProvincePolicies};
use crate::game::military::{ForceOwner, MilitaryWorld, RecruitmentProject, Unit};
use crate::game::politics::diplomacy::PoliticalState;
use std::collections::BTreeMap;

pub(crate) type IntelligenceReports = BTreeMap<(usize, usize), ProvinceIntelligence>;

#[derive(Clone, Debug)]
pub(crate) struct DemographicReport {
    pub month: u32,
    pub population: [f64; 4],
    pub population_change: Option<[f64; 4]>,
    pub happiness: [f64; 4],
}

#[derive(Clone, Debug)]
pub(crate) struct EconomicReport {
    pub month: u32,
    pub production: [f64; 3],
    pub food_requested: f64,
    pub food_supply_ratio: Option<f64>,
    pub policies: ProvincePolicies,
}

#[derive(Clone, Debug)]
pub(crate) struct OperationalReport {
    pub month: u32,
    pub construction: Option<ConstructionProject>,
    pub construction_pace: ConstructionPace,
    pub forces: BTreeMap<ForceOwner, Vec<Unit>>,
    pub recruitment: Option<RecruitmentProject>,
}

#[derive(Clone, Debug)]
pub(crate) struct ProvinceIntelligence {
    pub demographics: DemographicReport,
    pub economy: Option<EconomicReport>,
    pub operations: Option<OperationalReport>,
}

impl Campaign {
    /// Ownership and overlordship grant current local information, independently of spies.
    pub(crate) fn administers_province(&self, player: usize, province: usize) -> bool {
        self.politics.get(province).is_some_and(|p| match p.state {
            PoliticalState::Rome => false,
            PoliticalState::Owned {
                owner,
            } => owner == player,
            PoliticalState::Vassal {
                overlord,
                ..
            } => overlord == player,
            PoliticalState::Independent {
                ..
            } => false,
        })
    }

    /// One report per player/target, with separate dates for tiers not yet refreshed
    /// by a newly deployed network. Withdrawal/detection never erase historical reports.
    pub(crate) fn refresh_intelligence_reports(&mut self) {
        for mission in &self.espionage.missions {
            if mission.months_active == 0 {
                continue;
            }
            let Some(province) = self.economy.provinces.get(mission.province) else {
                continue;
            };
            let month = self.economy.month;
            let monthly = self
                .economy
                .last_report
                .province_reports
                .get(mission.province)
                .filter(|_| self.economy.last_report.month == month);
            let demographics = DemographicReport {
                month,
                population: province.population,
                population_change: monthly.map(|p| {
                    std::array::from_fn(|class| {
                        p.births[class] - p.normal_deaths[class] - p.famine_deaths[class]
                            + p.migration[class]
                    })
                }),
                happiness: province.happiness,
            };
            let report =
                self.intelligence.entry((mission.owner, mission.province)).or_insert_with(|| {
                    ProvinceIntelligence {
                        demographics: demographics.clone(),
                        economy: None,
                        operations: None,
                    }
                });
            report.demographics = demographics;
            if mission.months_active >= 2 {
                report.economy = Some(EconomicReport {
                    month,
                    production: monthly.map_or_else(
                        || province.production(&self.economy.config).1,
                        |p| p.production,
                    ),
                    food_requested: monthly.map_or_else(
                        || province.food_request(&self.economy.config),
                        |p| p.food_requested,
                    ),
                    food_supply_ratio: monthly.map(|p| p.food_supply_ratio),
                    policies: province.policies,
                });
            }
            if mission.months_active >= 3 {
                let Some(state) = self.military.provinces.get(mission.province) else {
                    continue;
                };
                let mut forces = state.forces.clone();
                for battle in
                    self.military.battles.iter().filter(|b| b.province == mission.province)
                {
                    for unit in battle.attackers.units.iter().chain(&battle.defenders.units) {
                        if unit.current_manpower > 0.0 {
                            forces.entry(unit.owner).or_default().push(unit.clone());
                        }
                    }
                }
                report.operations = Some(OperationalReport {
                    month,
                    construction: province.construction.clone(),
                    construction_pace: province.policies.construction,
                    forces,
                    recruitment: state.recruitment.clone(),
                });
            }
        }
    }

    /// Troops observe their province and its immediate neighbors. Battle participants
    /// observe the battlefield; owning a border alone does not scout foreign armies.
    pub(crate) fn observes_military(&self, player: usize, province: usize) -> bool {
        if self.administers_province(player, province) {
            return true;
        }
        let owner = ForceOwner::Player(player);
        let present = |id: usize| {
            self.military.provinces.get(id).is_some_and(|p| {
                p.forces
                    .get(&owner)
                    .is_some_and(|units| units.iter().any(|u| u.current_manpower > 0.0))
            }) || self.military.battles.iter().any(|b| {
                b.province == id
                    && b.attackers
                        .units
                        .iter()
                        .chain(&b.defenders.units)
                        .any(|u| u.owner == owner && u.current_manpower > 0.0)
            }) || self.military.movements.iter().any(|m| {
                m.owner == owner
                    && m.origin == id
                    && m.units.iter().any(|u| u.current_manpower > 0.0)
            })
        };
        present(province)
            || self
                .economy
                .adjacency
                .get(province)
                .is_some_and(|neighbors| neighbors.iter().any(|&id| present(id)))
    }

    /// A renderer receives only observed troops or dated local reports. Own forces
    /// always remain visible; all foreign movement destinations and saved plans are removed.
    pub(crate) fn military_view(&self, player: usize, include_reports: bool) -> MilitaryWorld {
        let owner = ForceOwner::Player(player);
        let mut view = self.military.clone();
        for (id, state) in view.provinces.iter_mut().enumerate() {
            let administrative = self.administers_province(player, id);
            let observed = self.observes_military(player, id);
            let report = include_reports
                .then(|| self.intelligence.get(&(player, id)))
                .flatten()
                .and_then(|r| r.operations.as_ref());
            if !observed {
                let own = state.forces.get(&owner).cloned();
                state.forces = report.map_or_else(BTreeMap::new, |r| r.forces.clone());
                // Historical reports must not duplicate our live troops or units now in battle/transit.
                state.forces.remove(&owner);
                if let Some(own) = own {
                    state.forces.insert(owner, own);
                }
                state.occupation = None;
            }
            state.plans.retain(|&p, _| p == owner);
            state.draft_penalties = [0.0; 4];
            if !administrative {
                state.recruitment = report.and_then(|r| r.recruitment.clone());
            }
        }
        view.movements.retain(|m| m.owner == owner);
        view.battles.retain(|b| self.observes_military(player, b.province));
        for battle in &mut view.battles {
            battle.attackers.plans.retain(|&p, _| p == owner);
            battle.defenders.plans.retain(|&p, _| p == owner);
        }
        view
    }
}

#[cfg(test)]
#[path = "../../tests/unit/intelligence.rs"]
mod tests;
