//! Province construction slots, unlimited upgrades, and canonical wonder projects.

use super::{ConstructionPace, EconomicProvince, EconomyConfig, EconomyEvent, EconomyWorld};

/// Standard improvements, with explicit stable indexes for save/UI arrays.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum BuildingType {
    /// Food storage.
    Granary,
    /// Metal and stone storage.
    Warehouse,
    /// Faster army travel and more efficient trade routes.
    Road,
    /// Comfortable population capacity.
    Aqueduct,
    /// City influence source.
    Forum,
    /// City capacity and happiness.
    Baths,
    /// City taxes and trade.
    UrbanMarket,
    /// City happiness and influence.
    Temple,
    /// City happiness.
    Arena,
    /// City defenses.
    CityWalls,
    /// City influence and citizen happiness.
    Academy,
    /// City metal production.
    Foundry,
}

impl BuildingType {
    /// Number of supported normal buildings.
    pub const COUNT: usize = 12;
    /// Stable UI order, with city improvements grouped together.
    pub const ALL: [Self; Self::COUNT] = [
        Self::Granary,
        Self::Warehouse,
        Self::Road,
        Self::Aqueduct,
        Self::Forum,
        Self::Baths,
        Self::UrbanMarket,
        Self::Temple,
        Self::Arena,
        Self::CityWalls,
        Self::Academy,
        Self::Foundry,
    ];

    /// Player-facing building name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Granary => "Granary",
            Self::Warehouse => "Warehouse",
            Self::Aqueduct => "Aqueduct",
            Self::Road => "Road",
            Self::Forum => "Forum",
            Self::Baths => "Baths",
            Self::Temple => "Temple",
            Self::Arena => "Arena",
            Self::UrbanMarket => "Market",
            Self::CityWalls => "Walls",
            Self::Academy => "Academy",
            Self::Foundry => "Foundry",
        }
    }
}

/// Linear benefits per completed level; costs increase exponentially instead.
#[derive(Clone, Copy, Debug, Default)]
pub struct BuildingEffects {
    /// Contribution to the direct owner's global Food/Metal/Stone storage.
    pub storage: [f64; 3],
    /// Additional province capacity.
    pub capacity: f64,
    /// Additive production multiplier per resource: 0.1 means +10% per level.
    pub production: [f64; 3],
    /// Flat class happiness change per level.
    pub happiness: [f64; 4],
    /// Monthly influence per level.
    pub influence: f64,
    /// Additive tax multiplier per level.
    pub tax: f64,
    /// Local military defense contribution per level.
    pub defense: f64,
    /// Extra inbound migration attractiveness.
    pub migration: f64,
    /// Added route efficiency per completed level.
    pub trade: f64,
}

/// Cost/time/effects definition for a normal building.
#[derive(Clone, Debug)]
pub struct BuildingDefinition {
    /// Improvement whose completed levels receive these effects.
    pub building: BuildingType,
    /// Base first-level Stone cost.
    pub stone_cost: f64,
    /// Base first-level Metal cost; no Coin cost exists.
    pub metal_cost: f64,
    /// First-level progress requirement.
    pub build_months: f64,
    /// Exponential cost growth for successive upgrades.
    pub cost_growth: f64,
    /// Slower exponential construction-time growth.
    pub time_growth: f64,
    /// Only canonical city provinces may build this improvement.
    pub requires_city: bool,
    /// Benefits from each completed level.
    pub effects: BuildingEffects,
}

impl BuildingDefinition {
    /// Central default catalog; names and geometry remain in their existing map data.
    pub fn for_type(building: BuildingType) -> Self {
        use BuildingType::*;
        let mut effects = BuildingEffects::default();
        let (stone_cost, metal_cost, build_months, requires_city) = match building {
            Granary => {
                effects.storage[0] = 600.0;
                (100.0, 10.0, 3.0, false)
            },
            Warehouse => {
                effects.storage = [0.0, 600.0, 1000.0];
                (140.0, 20.0, 4.0, false)
            },
            Aqueduct => {
                effects.capacity = 20.0;
                (160.0, 20.0, 4.0, false)
            },
            Road => (100.0, 10.0, 3.0, false),
            Forum => {
                effects.influence = 0.5;
                (180.0, 20.0, 4.0, true)
            },
            Baths => {
                effects.capacity = 15.0;
                effects.happiness = [2.0; 4];
                (180.0, 20.0, 4.0, true)
            },
            Temple => {
                effects.happiness = [2.0; 4];
                effects.influence = 0.25;
                (160.0, 15.0, 4.0, true)
            },
            Arena => {
                effects.happiness = [1.0, 2.0, 3.0, 2.0];
                (220.0, 30.0, 5.0, true)
            },
            UrbanMarket => {
                effects.tax = 0.1;
                effects.trade = 0.005;
                (150.0, 20.0, 4.0, true)
            },
            CityWalls => {
                effects.defense = 0.1;
                (260.0, 60.0, 6.0, true)
            },
            Academy => {
                effects.influence = 0.25;
                effects.happiness[1] = 3.0;
                (200.0, 20.0, 5.0, true)
            },
            Foundry => {
                effects.production[1] = 0.15;
                (150.0, 40.0, 4.0, true)
            },
        };
        Self {
            building,
            stone_cost,
            metal_cost,
            build_months,
            cost_growth: 1.6,
            time_growth: 1.2,
            requires_city,
            effects,
        }
    }

    /// Quote upgrading current level to current+1, using identical display/payment rounding.
    pub fn quote(&self, current_level: u32) -> ConstructionQuote {
        let cost_factor = self.cost_growth.powf(f64::from(current_level));
        ConstructionQuote {
            stone: (self.stone_cost * cost_factor).round(),
            metal: (self.metal_cost * cost_factor).round(),
            required_progress: self.build_months * self.time_growth.powf(f64::from(current_level)),
        }
    }
}

/// Stone/Metal payment and progress requirement shown before starting construction.
#[derive(Clone, Copy, Debug)]
pub struct ConstructionQuote {
    /// Payable rounded Stone cost.
    pub stone: f64,
    /// Payable rounded Metal cost.
    pub metal: f64,
    /// Progress required; a normal building adds one each month.
    pub required_progress: f64,
}

/// A canonical existing map wonder's gameplay settings, without copied coordinates/art.
#[derive(Clone, Debug)]
pub struct WonderDefinition {
    /// Index of the existing WONDERS site.
    pub wonder_id: usize,
    /// Stone paid when construction starts.
    pub stone_cost: f64,
    /// Metal paid when construction starts.
    pub metal_cost: f64,
    /// Work required, affected by slave assignment each month.
    pub required_progress: f64,
    /// One-time reward to direct owner on completion.
    pub completion_influence: f64,
    /// Passive reward to current direct owner thereafter.
    pub monthly_influence: f64,
}

impl WonderDefinition {
    /// Default monument scale; map still supplies the authoritative identity/location.
    pub fn for_site(wonder_id: usize) -> Self {
        Self {
            wonder_id,
            stone_cost: 2500.0,
            metal_cost: 500.0,
            required_progress: 36.0,
            completion_influence: 250.0,
            monthly_influence: 3.0,
        }
    }
}

/// Ordinary building upgrade in the province's one construction slot.
#[derive(Clone, Debug)]
pub struct BuildingProject {
    /// Improvement being built.
    pub building: BuildingType,
    /// Next level, captured when payment succeeds.
    pub target_level: u32,
    /// Accumulated monthly progress.
    pub progress: f64,
    /// Total work requirement.
    pub required_progress: f64,
}

/// Monument construction attached permanently to its canonical province.
#[derive(Clone, Debug)]
pub struct WonderProject {
    /// Canonical site ID.
    pub wonder_id: usize,
    /// Accumulated monthly work.
    pub progress: f64,
    /// Total work requirement.
    pub required_progress: f64,
    /// Slaves retained in population/food accounting but excluded from production.
    pub assigned_slaves: f64,
}

/// Mutually exclusive construction slot contents.
#[derive(Clone, Debug)]
pub enum ConstructionProject {
    /// Standard improvement/upgrade.
    Building(BuildingProject),
    /// Canonical monument.
    Wonder(WonderProject),
}

impl ConstructionProject {
    /// Current work and total work for progress bars.
    pub fn progress(&self) -> (f64, f64) {
        match self {
            Self::Building(p) => (p.progress, p.required_progress),
            Self::Wonder(p) => (p.progress, p.required_progress),
        }
    }

    /// Work rate using current assignment, so changing workers updates estimates.
    pub fn speed(&self, config: &EconomyConfig, pace: ConstructionPace) -> f64 {
        let base = match self {
            Self::Building(_) => 1.0,
            Self::Wonder(p) => config
                .wonder_slave_speeds
                .iter()
                .filter(|(minimum, _)| p.assigned_slaves >= *minimum)
                .map(|(_, speed)| *speed)
                .next_back()
                .unwrap_or(1.0),
        };
        base * config.construction_speed[pace as usize].max(0.0)
    }

    /// Whole monthly ticks remaining at current speed, recalculated for the UI.
    pub fn months_remaining(&self, config: &EconomyConfig, pace: ConstructionPace) -> u32 {
        let (progress, required) = self.progress();
        ((required - progress).max(0.0) / self.speed(config, pace).max(0.001)).ceil() as u32
    }
}

impl EconomicProvince {
    /// Sum linear building effects for capacity, production, UI explanations, and storage.
    pub fn building_effects(&self, config: &EconomyConfig) -> BuildingEffects {
        let mut result = BuildingEffects::default();
        for definition in &config.buildings {
            let level = f64::from(self.level(definition.building));
            for index in 0..3 {
                result.storage[index] += definition.effects.storage[index] * level;
                result.production[index] += definition.effects.production[index] * level;
            }
            for index in 0..4 {
                result.happiness[index] += definition.effects.happiness[index] * level;
            }
            result.capacity += definition.effects.capacity * level;
            result.influence += definition.effects.influence * level;
            result.tax += definition.effects.tax * level;
            result.defense += definition.effects.defense * level;
            result.migration += definition.effects.migration * level;
            result.trade += definition.effects.trade * level;
        }
        result
    }

    /// Clamp a wonder's active workers before production and construction updates.
    pub fn validate_slave_assignment(&mut self) {
        if let Some(ConstructionProject::Wonder(project)) = &mut self.construction {
            project.assigned_slaves =
                project.assigned_slaves.clamp(0.0, self.population[3].max(0.0));
        }
    }

    /// Slaves working on a monument cannot simultaneously produce resources.
    pub fn assigned_slaves(&self) -> f64 {
        match &self.construction {
            Some(ConstructionProject::Wonder(project)) => {
                project.assigned_slaves.clamp(0.0, self.population[3].max(0.0))
            },
            _ => 0.0,
        }
    }
}

impl EconomyWorld {
    /// Pay both costs atomically and occupy the province's single building slot.
    pub fn start_building(
        &mut self,
        player: usize,
        province: usize,
        building: BuildingType,
    ) -> Result<(), String> {
        let p = self.provinces.get(province).ok_or("Unknown province")?;
        if p.owner != Some(player) {
            return Err("Only the direct owner can construct buildings".into());
        }
        if p.construction.is_some() {
            return Err("The province's construction slot is occupied".into());
        }
        let definition = self
            .config
            .buildings
            .iter()
            .find(|d| d.building == building)
            .ok_or("Missing building definition")?;
        if definition.requires_city && !p.has_city {
            return Err("This building requires a city".into());
        }
        let current = p.level(building);
        let target =
            current.checked_add(1).ok_or("Building level exceeds numeric representation")?;
        let quote = definition.quote(current);
        self.pay_construction(player, quote.stone, quote.metal)?;
        self.provinces[province].construction =
            Some(ConstructionProject::Building(BuildingProject {
                building,
                target_level: target,
                progress: 0.0,
                required_progress: quote.required_progress,
            }));
        Ok(())
    }

    /// Begin a canonical-site wonder, paying all costs without a cancellation refund.
    pub fn start_wonder(
        &mut self,
        player: usize,
        province: usize,
        wonder: usize,
    ) -> Result<(), String> {
        let p = self.provinces.get(province).ok_or("Unknown province")?;
        if p.owner != Some(player) {
            return Err("Only the direct owner can begin a wonder".into());
        }
        if p.construction.is_some() {
            return Err("The province's construction slot is occupied".into());
        }
        if p.completed_wonder.is_some() {
            return Err("Only one completed wonder is allowed per province".into());
        }
        if !p.wonder_sites.contains(&wonder) {
            return Err("The canonical wonder site is in another province".into());
        }
        let definition = self
            .config
            .wonders
            .iter()
            .find(|d| d.wonder_id == wonder)
            .ok_or("Unknown canonical wonder")?
            .clone();
        self.pay_construction(player, definition.stone_cost, definition.metal_cost)?;
        self.provinces[province].construction = Some(ConstructionProject::Wonder(WonderProject {
            wonder_id: wonder,
            progress: 0.0,
            required_progress: definition.required_progress,
            assigned_slaves: 0.0,
        }));
        Ok(())
    }

    /// Change labor without moving or removing any population.
    pub fn assign_wonder_slaves(
        &mut self,
        player: usize,
        province: usize,
        count: f64,
    ) -> Result<(), String> {
        let p = self.provinces.get_mut(province).ok_or("Unknown province")?;
        if p.owner != Some(player) {
            return Err("Only the direct owner can assign construction labor".into());
        }
        if !count.is_finite() || count < 0.0 || count > p.population[3] {
            return Err(
                "Slave assignment must be between zero and the province's slave population".into(),
            );
        }
        match &mut p.construction {
            Some(ConstructionProject::Wonder(project)) => {
                project.assigned_slaves = count;
                Ok(())
            },
            _ => Err("No wonder is under construction".into()),
        }
    }

    /// Cancel the selected owner's project; sunk costs are intentionally not refunded.
    pub fn cancel_construction(&mut self, player: usize, province: usize) -> Result<(), String> {
        let p = self.provinces.get_mut(province).ok_or("Unknown province")?;
        if p.owner != Some(player) {
            return Err("Only the direct owner can cancel construction".into());
        }
        p.construction = None;
        Ok(())
    }

    /// Resource validation precedes mutation to prevent partial construction payments.
    fn pay_construction(&mut self, player: usize, stone: f64, metal: f64) -> Result<(), String> {
        if !stone.is_finite() || !metal.is_finite() || stone < 0.0 || metal < 0.0 {
            return Err("Construction cost is outside the supported numeric range".into());
        }
        let wallet = self.players.get_mut(player).ok_or("Unknown player")?;
        if wallet.resources[2] < stone || wallet.resources[1] < metal {
            return Err(format!("Requires {stone:.0} Stone and {metal:.0} Metal"));
        }
        wallet.resources[2] -= stone;
        wallet.resources[1] -= metal;
        Ok(())
    }

    /// Complete construction after production; labor remains excluded for the whole month.
    pub(super) fn advance_construction(&mut self, events: &mut Vec<EconomyEvent>) {
        for (province, state) in self.provinces.iter_mut().enumerate() {
            state.validate_slave_assignment();
            let Some(mut project) = state.construction.take() else {
                continue;
            };
            let speed = project.speed(&self.config, state.policies.construction);
            match &mut project {
                ConstructionProject::Building(p) => p.progress += speed,
                ConstructionProject::Wonder(p) => p.progress += speed,
            }
            let (progress, required) = project.progress();
            if progress + 1e-9 < required {
                state.construction = Some(project);
                continue;
            }
            match project {
                ConstructionProject::Building(p) => {
                    state.buildings[p.building as usize] = p.target_level;
                    events.push(EconomyEvent::BuildingCompleted {
                        province,
                        building: p.building,
                        level: p.target_level,
                    });
                },
                ConstructionProject::Wonder(p) => {
                    // No repeated reward, even if an externally loaded invalid project duplicates a monument.
                    if state.completed_wonder.is_some() {
                        continue;
                    }
                    state.completed_wonder = Some(p.wonder_id);
                    if let Some(owner) = state.owner.and_then(|owner| self.players.get_mut(owner)) {
                        if let Some(definition) =
                            self.config.wonders.iter().find(|d| d.wonder_id == p.wonder_id)
                        {
                            owner.influence += definition.completion_influence;
                        }
                    }
                    events.push(EconomyEvent::WonderCompleted {
                        province,
                        wonder: p.wonder_id,
                        owner: state.owner,
                    });
                },
            }
        }
    }
}
