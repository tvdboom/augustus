//! Class happiness, proportional famine, births/deaths, and conserved migration.

use super::{EconomicProvince, EconomyConfig, EconomyWorld, MonthlyInputs, ProvinceMonth};

/// Happiness below these class-specific values reduces the class's output.
pub const UNHAPPINESS_THRESHOLDS: [f64; 4] = [40.0, 30.0, 20.0, 10.0];

/// Output falls linearly from full at the threshold to half at zero happiness.
pub fn happiness_output_multiplier(class: usize, happiness: f64, config: &EconomyConfig) -> f64 {
    let threshold = UNHAPPINESS_THRESHOLDS[class];
    1.0 - config.max_unhappiness_output_loss.clamp(0.0, 1.0)
        * ((threshold - happiness) / threshold).clamp(0.0, 1.0)
}

/// Convert atlas square-degree geometry into the displayed resident scale.
/// The current map's starting population is `220 + 35 * sqrt(area)`,
/// before city/starting compensation. Capacity adds its own terrain/city modifiers.
pub fn normalized_capacity_area(raw_map_area: f64) -> f64 {
    220.0 + 35.0 * raw_map_area.max(0.0).sqrt()
}

/// Happiness strongly suppresses unhappy births and modestly rewards happy births.
pub fn birth_modifier(happiness: f64) -> f64 {
    let happiness = happiness.clamp(0.0, 100.0);
    if happiness < 50.0 {
        happiness / 50.0
    } else {
        1.0 + (happiness - 50.0) / 100.0
    }
}

impl EconomicProvince {
    /// A soft birth-only space limit preserves ordinary growth through 150%
    /// capacity, then tends toward zero even when buildings maximize happiness.
    /// It never deletes residents or increases natural/famine mortality.
    pub fn crowding_birth_modifier(&self, config: &EconomyConfig) -> f64 {
        let threshold = config.crowding_birth_threshold.max(1.0);
        let ratio = self.total_population() / self.capacity(config);
        (threshold / ratio.max(threshold)).clamp(0.0, 1.0)
    }

    /// Comfortable carrying capacity affects happiness; population is never truncated to it.
    pub fn capacity(&self, config: &EconomyConfig) -> f64 {
        (self.capacity_area
            * config.area_to_capacity_scale
            * config.terrain_capacity[self.terrain as usize]
            + if self.has_city {
                config.city_capacity
            } else {
                0.0
            }
            + self.building_effects(config).capacity
            + self.practice_capacity_bonus)
            .max(0.001)
    }

    /// Ration-adjusted civilian food request, including construction-assigned slaves.
    pub fn food_request(&self, config: &EconomyConfig) -> f64 {
        self.population
            .iter()
            .zip(config.food_per_class)
            .map(|(count, food)| count * food)
            .sum::<f64>()
            * config.food_policy[self.policies.food as usize].consumption
    }

    fn available_production_workers(&self, config: &EconomyConfig) -> [f64; 4] {
        let construction_slaves = if self.construction.is_some() {
            self.population[3]
                * config.construction_labor[self.policies.construction as usize].clamp(0.0, 1.0)
        } else {
            0.0
        };
        // Both construction assignments remove slaves from production, never free workers.
        let productive_slaves =
            (self.population[3] - self.assigned_slaves() - construction_slaves).max(0.0);
        [0.0, 0.0, self.population[2], productive_slaves]
    }

    /// People working in Food, Metal and Stone, ordered by population class.
    /// Construction assignments are excluded; productivity does not change headcounts.
    pub fn production_workers(&self, config: &EconomyConfig) -> [[f64; 4]; 3] {
        let workers = self.available_production_workers(config);
        let focus = config.focus_weights[self.policies.focus as usize];
        let weights: [f64; 3] = std::array::from_fn(|i| self.potential[i] * focus[i]);
        let denominator: f64 = weights.iter().sum();
        if denominator <= 0.0 {
            return [[0.0; 4]; 3];
        }
        weights.map(|weight| {
            let share = weight / denominator;
            workers.map(|count| count * share)
        })
    }

    /// Divide productive labor once, then calculate sector production from allocated shares.
    pub fn production(&self, config: &EconomyConfig) -> ([f64; 3], [f64; 3]) {
        let workers = self.available_production_workers(config);
        let labor = workers[2]
            * config.productivity[0]
            * happiness_output_multiplier(2, self.happiness[2], config)
            + workers[3]
                * config.productivity[1]
                * config.slave_policy[self.policies.slave_labor as usize].productivity
                * happiness_output_multiplier(3, self.happiness[3], config);
        let focus = config.focus_weights[self.policies.focus as usize];
        let weights: [f64; 3] = std::array::from_fn(|i| self.potential[i] * focus[i]);
        let denominator: f64 = weights.iter().sum();
        if denominator <= 0.0 {
            return ([0.0; 3], [0.0; 3]);
        }
        let allocation = weights.map(|weight| labor * weight / denominator);
        let effects = self.building_effects(config);
        let production = std::array::from_fn(|i| {
            allocation[i]
                * self.potential[i]
                * (1.0 + effects.production[i]).max(0.0)
                * config.production_scale[i]
        });
        (allocation, production)
    }

    /// Final happiness combines standing modifiers with accumulated hardship.
    pub fn calculate_happiness(&self, config: &EconomyConfig) -> [f64; 4] {
        let food = config.food_policy[self.policies.food as usize];
        let slave = config.slave_policy[self.policies.slave_labor as usize];
        let buildings = self.building_effects(config);
        std::array::from_fn(|class| {
            (50.0
                + food.happiness
                + buildings.happiness[class]
                + config.manumission_happiness[self.policies.manumission as usize][class]
                + self.happiness_modifiers[class]
                + self.temporary_happiness[class]
                + if class == 0 {
                    self.noble_wage_happiness
                } else {
                    0.0
                }
                - if class == 0 {
                    self.insolvency_unhappiness
                } else {
                    0.0
                }
                + if class == 1 || class == 2 {
                    self.recruitment_happiness
                } else {
                    0.0
                }
                + if class == 3 {
                    slave.happiness
                } else {
                    config.migration_happiness[self.policies.migration as usize]
                        + self.civic_happiness
                }
                - self.overcrowding_unhappiness
                - self.shortage_unhappiness)
                .clamp(0.0, 100.0)
        })
    }

    /// Only citizens and plebeians pay taxes; nobles and slaves are exempt.
    pub fn tax_income(&self, config: &EconomyConfig) -> f64 {
        (1..=2)
            .map(|class| {
                self.population[class]
                    * config.tax_rates[class]
                    * if class == 1 {
                        happiness_output_multiplier(class, self.happiness[class], config)
                    } else {
                        1.0
                    }
            })
            .sum::<f64>()
            * (1.0 + self.building_effects(config).tax).max(0.0)
    }

    /// Requested monthly civic budget, based on free residents before demographic changes.
    pub fn civic_spending_cost(&self, config: &EconomyConfig) -> f64 {
        self.population[..3].iter().sum::<f64>()
            * config.civic_coin_per_free_resident[self.policies.civic_spending as usize].max(0.0)
    }

    /// Births/deaths are independent flows; class upgrades preserve the remaining total.
    pub(super) fn advance_demographics(
        &mut self,
        report: &mut ProvinceMonth,
        config: &EconomyConfig,
    ) {
        let food = config.food_policy[self.policies.food as usize];
        let slave = config.slave_policy[self.policies.slave_labor as usize];
        let previous_happiness = self.happiness;
        report.overcrowding_penalty = ((self.total_population() / self.capacity(config) - 1.0)
            .max(0.0)
            * config.overcrowding_scale)
            .min(config.overcrowding_cap);
        report.shortage_penalty =
            (1.0 - report.food_supply_ratio.clamp(0.0, 1.0)) * config.shortage_happiness_penalty;
        self.overcrowding_unhappiness = if report.overcrowding_penalty > 0.0 {
            (self.overcrowding_unhappiness + report.overcrowding_penalty).min(100.0)
        } else {
            (self.overcrowding_unhappiness - config.overcrowding_cap).max(0.0)
        };
        self.shortage_unhappiness = if report.shortage_penalty > 0.0 {
            (self.shortage_unhappiness + report.shortage_penalty).min(100.0)
        } else {
            (self.shortage_unhappiness - config.shortage_happiness_penalty).max(0.0)
        };
        self.happiness = self.calculate_happiness(config);
        report.happiness_delta =
            std::array::from_fn(|class| self.happiness[class] - previous_happiness[class]);
        let crowding_births = self.crowding_birth_modifier(config);
        for class in 0..4 {
            let count = self.population[class];
            report.births[class] = count
                * config.birth_rates[class]
                * birth_modifier(self.happiness[class])
                * crowding_births
                * food.births
                * report.food_supply_ratio;
            // Low happiness never directly increases natural mortality.
            report.normal_deaths[class] = count
                * config.death_rates[class]
                * food.deaths
                * if class == 3 {
                    slave.deaths
                } else {
                    1.0
                };
            let shortage = 1.0 - report.food_supply_ratio.clamp(0.0, 1.0);
            report.famine_deaths[class] = count * shortage.powf(1.5) * config.max_famine_death_rate;
            self.population[class] = (count + report.births[class]
                - report.normal_deaths[class]
                - report.famine_deaths[class])
                .max(0.0);
        }
        let before = self.population;
        // Rates use one immutable pre-conversion snapshot: no same-month cascading promotion.
        let rate = config.manumission_rates[self.policies.manumission as usize].clamp(-1.0, 1.0);
        let manumission = if rate >= 0.0 {
            before[3] * rate
        } else {
            before[2] * rate
        };
        // Enslaved plebeians cannot also gain citizenship; newly freed slaves wait a month.
        let citizenship = (before[2] + manumission.min(0.0)).max(0.0)
            * config.class_change_rates[0].clamp(0.0, 1.0)
            * if self.has_city {
                1.0
            } else {
                0.5
            };
        let nobility = before[1]
            * config.class_change_rates[1].clamp(0.0, 1.0)
            * if self.has_city {
                1.0
            } else {
                0.25
            };
        self.population[3] -= manumission;
        self.population[2] += manumission - citizenship;
        self.population[1] += citizenship - nobility;
        self.population[0] += nobility;
        self.validate_slave_assignment();
    }
}

impl EconomyWorld {
    /// Civilian total, comfortable capacity and last month's net growth across
    /// directly owned provinces. Drafted soldiers are no longer residents.
    pub fn player_population(&self, player: usize) -> (f64, f64, f64) {
        let mut total = 0.0;
        let mut capacity = 0.0;
        let mut growth = 0.0;
        for (id, province) in self.provinces.iter().enumerate() {
            if province.owner == Some(player) {
                total += province.total_population();
                capacity += province.capacity(&self.config);
                growth +=
                    self.last_report.province_reports.get(id).map_or(0.0, |r| r.population_delta);
            }
        }
        (total, capacity, growth)
    }

    /// Current class happiness and last monthly change, weighted by residents in
    /// directly owned provinces. Empty classes have neutral happiness and no change.
    pub fn player_happiness(&self, player: usize, class: usize) -> (f64, f64) {
        let mut count = 0.0;
        let mut happiness = 0.0;
        let mut change = 0.0;
        for (id, province) in self.provinces.iter().enumerate() {
            if province.owner != Some(player) {
                continue;
            }
            let residents = province.population[class];
            count += residents;
            happiness += residents * province.happiness[class];
            change += residents
                * self
                    .last_report
                    .province_reports
                    .get(id)
                    .map_or(0.0, |r| r.happiness_delta[class]);
        }
        if count > 0.0 {
            (happiness / count, change / count)
        } else {
            (50.0, 0.0)
        }
    }

    /// Simultaneous migration deltas prevent province iteration order from moving a pop twice.
    pub(super) fn migrate(&mut self, inputs: &MonthlyInputs, reports: &mut [ProvinceMonth]) {
        let mut deltas = vec![[0.0; 4]; self.provinces.len()];
        for (source_id, source) in self.provinces.iter().enumerate() {
            let population_ratio = source.total_population() / source.capacity(&self.config);
            for (class, &population) in source.population.iter().take(3).enumerate() {
                // Slaves have no free migration regardless of malformed config.
                let unhappiness = ((50.0 - source.happiness[class]) / 50.0).max(0.0);
                let rate = self.config.migration_rates[class]
                    * (self.config.baseline_migration_pressure.max(0.0)
                        + unhappiness
                        + (population_ratio - 1.0).max(0.0)
                            * self.config.overpopulation_migration_scale)
                    * self.config.migration_out[source.policies.migration as usize];
                let emigrants = population * rate.clamp(0.0, 1.0);
                if emigrants <= 0.0 {
                    continue;
                }
                let mut destinations = Vec::new();
                if let Some(adjacent) = self.adjacency.get(source_id) {
                    for &target_id in adjacent {
                        if target_id == source_id {
                            continue;
                        }
                        let Some(target) = self.provinces.get(target_id) else {
                            continue;
                        };
                        let relationship =
                            migration_relationship(source_id, source, target_id, target, inputs);
                        if relationship <= 0.0 {
                            continue;
                        }
                        let free = (1.0
                            - target.total_population() / target.capacity(&self.config))
                        .max(0.0);
                        let weights = self.config.migration_weights;
                        let score = (weights[0] * free
                            + weights[1] * target.happiness[class] / 100.0
                            + if target.has_city {
                                weights[2]
                            } else {
                                0.0
                            })
                            * (1.0 + target.building_effects(&self.config).migration).max(0.0)
                            * self.config.migration_in[target.policies.migration as usize]
                            * relationship;
                        if score > 0.0 && !destinations.iter().any(|(id, _)| *id == target_id) {
                            destinations.push((target_id, score));
                        }
                    }
                }
                let total_score: f64 = destinations.iter().map(|(_, score)| score).sum();
                if total_score <= 0.0 {
                    continue;
                } // No valid destination means no lost population.
                deltas[source_id][class] -= emigrants;
                for (target, score) in destinations {
                    deltas[target][class] += emigrants * score / total_score;
                }
            }
        }
        for ((province, report), delta) in self.provinces.iter_mut().zip(reports).zip(deltas) {
            report.migration = delta;
            for (population, change) in province.population.iter_mut().zip(delta) {
                *population += change;
            }
        }
    }
}

/// Relationship affects destination weight but never changes the conserved migrant total.
fn migration_relationship(
    source_id: usize,
    source: &EconomicProvince,
    target_id: usize,
    target: &EconomicProvince,
    inputs: &MonthlyInputs,
) -> f64 {
    let source_player = source.owner.or(source.overlord);
    let target_player = target.owner.or(target.overlord);
    if source_player
        .is_some_and(|player| target.owner.is_none() && inputs.hostile_npc(player, target_id))
        || target_player
            .is_some_and(|player| source.owner.is_none() && inputs.hostile_npc(player, source_id))
    {
        return 0.0;
    }
    if let (Some(a), Some(b)) = (source_player, target_player) {
        if inputs.hostile(a, b) {
            return 0.0;
        }
        if a == b {
            return if source.owner == target.owner && source.owner.is_some() {
                1.0
            } else {
                0.9
            };
        }
    }
    let relation = if let Some(player) = source_player {
        target.relation_by_player.get(player).copied().unwrap_or(50.0)
    } else if let Some(player) = target_player {
        source.relation_by_player.get(player).copied().unwrap_or(50.0)
    } else {
        50.0
    };
    if relation < 40.0 {
        0.0
    } else if relation >= 60.0 {
        0.6
    } else {
        0.25
    }
}
