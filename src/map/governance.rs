//! Empire-wide edicts for the local monthly economy.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum EdictLevel {
    Low,
    #[default]
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Governance {
    pub food_rations: EdictLevel,
    pub slave_labor: EdictLevel,
    pub noble_taxes: EdictLevel,
    pub army_wages: EdictLevel,
}

impl Governance {
    pub(crate) fn population_growth_rate(self) -> f64 {
        match self.food_rations {
            EdictLevel::Low => 0.005,
            EdictLevel::Medium => 0.01,
            EdictLevel::High => 0.015,
        }
    }

    pub(crate) fn slave_population_growth_rate(self) -> f64 {
        (self.population_growth_rate()
            - if self.slave_labor == EdictLevel::High {
                0.005
            } else {
                0.0
            })
        .max(0.0)
    }

    pub(crate) fn food_per_person(self) -> f64 {
        match self.food_rations {
            EdictLevel::Low => 0.8,
            EdictLevel::Medium => 1.0,
            EdictLevel::High => 1.2,
        }
    }

    pub(crate) fn slave_labor_factor(self) -> f64 {
        match self.slave_labor {
            EdictLevel::Low => 0.5,
            EdictLevel::Medium => 1.0,
            EdictLevel::High => 1.5,
        }
    }

    pub(crate) fn noble_tax_per_person(self) -> f64 {
        match self.noble_taxes {
            EdictLevel::Low => 0.5,
            EdictLevel::Medium => 1.0,
            EdictLevel::High => 1.5,
        }
    }

    pub(crate) fn army_wage_factor(self) -> f64 {
        match self.army_wages {
            EdictLevel::Low => 0.75,
            EdictLevel::Medium => 1.0,
            EdictLevel::High => 1.25,
        }
    }

    pub(crate) fn happiness_deltas(self) -> [f64; 4] {
        let ration = match self.food_rations {
            EdictLevel::Low => -1.0,
            EdictLevel::Medium => 0.0,
            EdictLevel::High => 1.0,
        };
        let slave_labor = match self.slave_labor {
            EdictLevel::Low => 2.0,
            EdictLevel::Medium => 0.0,
            EdictLevel::High => -2.0,
        };
        let noble_tax = match self.noble_taxes {
            EdictLevel::Low => 1.0,
            EdictLevel::Medium => 0.0,
            EdictLevel::High => -1.0,
        };
        [ration + noble_tax, ration, ration, ration + slave_labor]
    }
}
