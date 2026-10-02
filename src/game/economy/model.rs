//! Domain state and reports shared by the simulation and its UI adapter.

use super::{
    BuildingType, ConstructionProject, EconomyConfig, NpcTradeEconomy, TradeAgreement,
    TradePoliticalEffect,
};

/// Geographic capacity category. Marsh preserves the existing map's extra type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terrain {
    /// Productive agricultural land.
    Farmland,
    /// Open grassland.
    Plains,
    /// Wooded territory.
    Forest,
    /// Hilly territory.
    Hills,
    /// High mountains.
    Mountains,
    /// Arid country; river floodplains may use Farmland instead.
    Desert,
    /// Wetlands.
    Marsh,
}

/// Monthly ration policy; values index the matching configuration array.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FoodPolicy {
    /// Consume less food with lower happiness and higher mortality.
    Low,
    /// Ordinary rations.
    #[default]
    Normal,
    /// Consume extra food for wellbeing and births.
    High,
}

/// Monthly slave productivity and wellbeing tradeoff.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SlaveLabor {
    /// Lighter work, higher happiness, lower mortality.
    Light,
    /// Ordinary workload.
    #[default]
    Normal,
    /// Higher output, lower happiness, greater natural mortality.
    Harsh,
}

/// Province policy affecting automatic migration, never a manual pop transfer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MigrationPolicy {
    /// More attractive to migrants.
    Encourage,
    /// Ordinary mobility.
    #[default]
    Normal,
    /// Lower immigration and modestly lower emigration.
    Discourage,
    /// Strongly reduced, but never magically zero, mobility.
    Closed,
}

/// Labor allocation weights; every worker contributes to only one sector.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ResourceFocus {
    /// Weight by geographic potential alone.
    #[default]
    Balanced,
    /// Favor agriculture.
    Food,
    /// Favor mining.
    Metal,
    /// Favor quarrying.
    Stone,
}

/// Local construction speed versus productive workforce allocation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConstructionPace {
    /// Fewer workers and slower completion.
    Slow,
    /// Ordinary public works.
    #[default]
    Normal,
    /// Divert more productive labor to finish sooner.
    Urgent,
}

/// Monthly local Coin budget for free-class wellbeing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CivicSpending {
    /// No spending, with lower free-class happiness.
    Frugal,
    /// Modest spending, maintaining neutral free-class happiness.
    #[default]
    Normal,
    /// Greater spending, raising free-class happiness.
    Generous,
}

/// Recruitment speed, expense and local happiness while a cohort is being raised.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RecruitmentEffort {
    /// Slower recruitment with less pressure on residents.
    Low,
    /// Ordinary recruitment speed and cost without a happiness change.
    #[default]
    Normal,
    /// Pay for faster recruitment at a local happiness cost.
    High,
}

/// Local conversion between slaves and plebeians, with class happiness tradeoffs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ManumissionPolicy {
    /// Enslave a share of plebeians, reducing plebeian happiness.
    Enslave,
    /// No policy conversion or happiness effect.
    #[default]
    Normal,
    /// Free a share of slaves, reducing noble happiness.
    Free,
}

/// Nationwide rations/labor and six province-local policy values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProvincePolicies {
    /// Civilian food rations.
    pub food: FoodPolicy,
    /// Slave workload.
    pub slave_labor: SlaveLabor,
    /// Migration encouragement/restrictions.
    pub migration: MigrationPolicy,
    /// Physical resource labor allocation.
    pub focus: ResourceFocus,
    /// Local building and wonder pace.
    pub construction: ConstructionPace,
    /// Optional local wellbeing budget.
    pub civic_spending: CivicSpending,
    /// Effort applied only to an active recruitment project.
    pub recruitment: RecruitmentEffort,
    /// Local enslavement or freeing of residents and its happiness tradeoff.
    pub manumission: ManumissionPolicy,
}

/// Global player balances. Physical storage is never attached to owned provinces.
#[derive(Clone, Debug)]
pub struct PlayerEconomy {
    /// Food, Metal, Stone currently stored.
    pub resources: [f64; 3],
    /// Food, Metal, Stone storage maxima.
    pub storage: [f64; 3],
    /// Extra capacity granted by the local practice shortcut.
    pub practice_storage_bonus: [f64; 3],
    /// Currency without a physical storage limit.
    pub coin: f64,
    /// Scarce political currency without a physical storage limit.
    pub influence: f64,
}

impl PlayerEconomy {
    /// New-game balances in the current simulation's population scale.
    pub fn new(config: &EconomyConfig) -> Self {
        Self {
            resources: config.starting_stock,
            storage: config.base_storage,
            practice_storage_bonus: [0.0; 3],
            coin: 201.0,
            influence: 40.0,
        }
    }

    /// Resource vector for monthly change reports: Food, Metal, Stone, Coin, Influence.
    pub fn balances(&self) -> [f64; 5] {
        [self.resources[0], self.resources[1], self.resources[2], self.coin, self.influence]
    }

    /// Clamp physical reserves after same-month production, trade, and consumption.
    pub fn clamp_storage(&mut self) {
        for index in 0..3 {
            self.resources[index] = self.resources[index].clamp(0.0, self.storage[index].max(0.0));
        }
        self.coin = self.coin.max(0.0);
        self.influence = self.influence.max(0.0);
    }
}

/// Economic state attached to a map province, including independent NPCs.
#[derive(Clone, Debug)]
pub struct EconomicProvince {
    /// Display name supplied by the canonical map.
    pub name: String,
    /// Normalized geometric area; use `normalized_capacity_area` for the current atlas.
    pub capacity_area: f64,
    /// Geographic capacity modifier category.
    pub terrain: Terrain,
    /// Whether the authoritative map defines a city here.
    pub has_city: bool,
    /// Current direct player owner; absent for independent/vassal NPCs.
    pub owner: Option<usize>,
    /// Enemy military occupation blocks administration and provincial trade without changing ownership.
    pub occupied: bool,
    /// Current NPC overlord, if any. Does not receive direct province output.
    pub overlord: Option<usize>,
    /// View of each player, synchronized from the political subsystem.
    pub relation_by_player: Vec<f64>,
    /// Temporary political/blackmail modifier on NPC required trade-value ratio.
    /// Politics supplies expiration; 1.0 is ordinary trade and 0.75 is favorable terms.
    pub trade_ratio_by_player: Vec<f64>,
    /// Aggregate class populations: Noble, Citizen, Plebeian, Slave.
    pub population: [f64; 4],
    /// Happiness per class, clamped to 0..100 every month.
    pub happiness: [f64; 4],
    /// Persistent happiness lost to consecutive overcrowded months.
    pub overcrowding_unhappiness: f64,
    /// Persistent happiness lost to consecutive food shortages.
    pub shortage_unhappiness: f64,
    /// Food, Metal, Stone geographic potential from existing map data.
    pub potential: [f64; 3],
    /// Monthly local decisions.
    pub policies: ProvincePolicies,
    /// Building levels indexed by BuildingType; no gameplay maximum.
    pub buildings: [u32; BuildingType::COUNT],
    /// The single local construction slot.
    pub construction: Option<ConstructionProject>,
    /// Fully paid upgrades waiting for construction.
    pub construction_queue: std::collections::VecDeque<ConstructionProject>,
    /// Canonical wonder indexes whose existing coordinates lie in this province.
    pub wonder_sites: Vec<usize>,
    /// The province's one completed wonder, if any.
    pub completed_wonder: Option<usize>,
    /// Persistent externally managed class happiness modifiers, e.g. unrest.
    pub happiness_modifiers: [f64; 4],
    /// Happiness support actually funded in the latest economic month.
    pub civic_happiness: f64,
    /// Non-accumulating noble wage policy happiness for the latest month.
    pub noble_wage_happiness: f64,
    /// Non-accumulating recruitment-effort effect for the current month.
    pub recruitment_happiness: f64,
    /// Additional per-class happiness effect that decays monthly.
    pub temporary_happiness: [f64; 4],
    /// Cumulative noble unhappiness while the owner's treasury cannot cover outflow.
    pub insolvency_unhappiness: f64,
    /// Local market with persistent NPC coin treasury.
    pub market: NpcTradeEconomy,
}

impl EconomicProvince {
    /// Maximum construction orders per province, including the active project.
    pub const MAX_CONSTRUCTION_ORDERS: usize = 10;

    /// Whether the active project and waiting orders fill all construction slots.
    pub fn construction_queue_full(&self) -> bool {
        self.construction_queue.len() + usize::from(self.construction.is_some())
            >= Self::MAX_CONSTRUCTION_ORDERS
    }

    /// Build economic state from existing map/population data without changing its art.
    pub fn new(
        name: impl Into<String>,
        capacity_area: f64,
        terrain: Terrain,
        has_city: bool,
        potential: [f64; 3],
        population: [f64; 4],
        player_count: usize,
    ) -> Self {
        Self {
            name: name.into(),
            capacity_area: capacity_area.max(0.0),
            terrain,
            has_city,
            owner: None,
            occupied: false,
            overlord: None,
            relation_by_player: vec![50.0; player_count],
            trade_ratio_by_player: vec![1.0; player_count],
            population: population.map(|value| value.max(0.0)),
            happiness: [50.0; 4],
            overcrowding_unhappiness: 0.0,
            shortage_unhappiness: 0.0,
            potential: potential.map(|value| value.max(0.0)),
            policies: ProvincePolicies::default(),
            buildings: [0; BuildingType::COUNT],
            construction: None,
            construction_queue: Default::default(),
            wonder_sites: Vec::new(),
            completed_wonder: None,
            happiness_modifiers: [0.0; 4],
            civic_happiness: 0.0,
            noble_wage_happiness: 0.0,
            recruitment_happiness: 0.0,
            temporary_happiness: [0.0; 4],
            insolvency_unhappiness: 0.0,
            market: NpcTradeEconomy::default(),
        }
    }

    /// Total aggregate population, including construction-assigned slaves.
    pub fn total_population(&self) -> f64 {
        self.population.iter().sum()
    }

    /// Only the direct owner of an unoccupied province can administer it.
    pub fn can_administer(&self, player: usize) -> bool {
        self.owner == Some(player) && !self.occupied
    }

    /// Current completed building level.
    pub fn level(&self, building: BuildingType) -> u32 {
        self.buildings[building as usize]
    }

    /// Quote the next level after all already paid upgrades.
    pub fn planned_building_level(&self, building: BuildingType) -> u32 {
        self.construction
            .iter()
            .chain(self.construction_queue.iter())
            .filter_map(|project| {
                if let ConstructionProject::Building(p) = project {
                    (p.building == building).then_some(p.target_level)
                } else {
                    None
                }
            })
            .max()
            .unwrap_or(self.level(building))
    }

    /// Preserve buildings/projects on capture while resetting forced labor assignment.
    pub fn change_owner(&mut self, owner: Option<usize>, overlord: Option<usize>) {
        if self.owner != owner || self.overlord != overlord {
            self.construction_queue.clear();
            self.civic_happiness = 0.0;
            self.noble_wage_happiness = 0.0;
            self.recruitment_happiness = 0.0;
            self.insolvency_unhappiness = 0.0;
            if let Some(ConstructionProject::Wonder(project)) = &mut self.construction {
                project.assigned_slaves = 0.0;
            }
        }
        self.owner = owner;
        self.overlord = if owner.is_some() {
            None
        } else {
            overlord
        };
    }

    /// Average happiness weighted by residents, with neutral empty provinces.
    pub fn mean_happiness(&self) -> f64 {
        let total = self.total_population();
        if total <= 0.0 {
            return 50.0;
        }
        self.population
            .iter()
            .zip(self.happiness)
            .map(|(count, happiness)| count * happiness)
            .sum::<f64>()
            / total
    }
}

/// External military/diplomatic facts consumed during this economic month.
#[derive(Clone, Debug, Default)]
pub struct MonthlyInputs {
    /// Military food demand charged to each player: all hosted armies plus their marching troops.
    pub army_food: Vec<f64>,
    /// Local NPC military food demand by province.
    pub npc_army_food: Vec<f64>,
    /// Symmetric player hostility matrix; an absent entry means peaceful transit.
    pub player_hostility: Vec<Vec<bool>>,
    /// Player-by-province hostility against NPCs, independent of current relation.
    pub npc_hostility: Vec<Vec<bool>>,
}

impl MonthlyInputs {
    /// Read either direction defensively so one-sided declarations still block trade.
    pub fn hostile(&self, a: usize, b: usize) -> bool {
        self.player_hostility.get(a).and_then(|row| row.get(b)).copied().unwrap_or(false)
            || self.player_hostility.get(b).and_then(|row| row.get(a)).copied().unwrap_or(false)
    }

    /// War blocks NPC transit/trade even if gifts have raised relation during hostilities.
    pub fn hostile_npc(&self, player: usize, province: usize) -> bool {
        self.npc_hostility.get(player).and_then(|row| row.get(province)).copied().unwrap_or(false)
    }
}

/// Food, population, and production explanations for a province's latest month.
#[derive(Clone, Debug, Default)]
pub struct ProvinceMonth {
    /// Effective comfortable capacity; never a hard population cap.
    pub capacity: f64,
    /// Allocated workers by physical resource, summing to productive labor.
    pub labor: [f64; 3],
    /// Gross production before consumption/trade.
    pub production: [f64; 3],
    /// Civilian food requested before global proportional allocation.
    pub food_requested: f64,
    /// Fraction of requested food delivered.
    pub food_supply_ratio: f64,
    /// Births per class before class changes and migration.
    pub births: [f64; 4],
    /// Natural deaths per class.
    pub normal_deaths: [f64; 4],
    /// Proportional famine mortality per class.
    pub famine_deaths: [f64; 4],
    /// Change in class happiness from before this month's demographic calculation.
    pub happiness_delta: [f64; 4],
    /// Happiness lost to overcrowding this month, before recovery or the happiness floor.
    pub overcrowding_penalty: f64,
    /// Happiness lost to food shortage this month, before recovery or the happiness floor.
    pub shortage_penalty: f64,
    /// Net immigrants minus emigrants by class.
    pub migration: [f64; 4],
    /// Enslaved residents who left this province in a revolt after demographics.
    pub slave_revolt_loss: f64,
    /// Tax income paid to the direct owner.
    pub tax_income: f64,
    /// Coin actually paid for local civic spending before taxes arrive.
    pub civic_spending: f64,
    /// Domestic/building/wonder passive influence, excluding political rank/vassals.
    pub influence_income: f64,
    /// Total population change, all demographic effects included.
    pub population_delta: f64,
}

/// Semantic events for the shared toast system; no UI dependency in the economy.
#[derive(Clone, Debug, PartialEq)]
pub enum EconomyEvent {
    /// A province's normal building finished.
    BuildingCompleted {
        /// Province that completed the improvement.
        province: usize,
        /// Completed improvement type.
        building: BuildingType,
        /// New completed level.
        level: u32,
    },
    /// A wonder finished; one-time reward has already been credited exactly once.
    WonderCompleted {
        /// Province containing the canonical site.
        province: usize,
        /// Canonical wonder ID.
        wonder: usize,
        /// Direct owner receiving the completion award, if any.
        owner: Option<usize>,
    },
    /// A recurring agreement could not run this month.
    TradeSuspended {
        /// Stable agreement identifier.
        agreement: u64,
        /// Inspectable reason the monthly execution could not proceed.
        reason: String,
    },
    /// A recurring agreement hit its consecutive-failure limit.
    TradeCancelled {
        /// Stable agreement identifier.
        agreement: u64,
    },
    /// Voluntary NPC notice elapsed after its last scheduled delivery.
    TradeNoticeCompleted {
        /// Stable agreement identifier.
        agreement: u64,
    },
    /// A player received less than 90% of required civilian/military food.
    FoodShortage {
        /// Affected player.
        player: usize,
        /// Fraction of the player's requested food delivered.
        supplied: f64,
    },
}

/// Month result exposed to HUD, notifications, military, and politics.
#[derive(Clone, Debug, Default)]
pub struct MonthlyReport {
    /// New month number, starting at one.
    pub month: u32,
    /// Player deltas for Food, Metal, Stone, Coin, Influence.
    pub player_delta: Vec<[f64; 5]>,
    /// Player-wide proportional food allocation; military uses this same fraction.
    pub food_supply_ratio: Vec<f64>,
    /// Province-indexed explanations for the UI.
    pub province_reports: Vec<ProvinceMonth>,
    /// Trade effects to apply through the political system's simultaneous resolver.
    pub trade_effects: Vec<TradePoliticalEffect>,
    /// Events for notifications and audit logging.
    pub events: Vec<EconomyEvent>,
}

/// Authoritative economic simulation, with no renderer, clock, or random dependency.
#[derive(Clone, Debug)]
pub struct EconomyWorld {
    /// Centralized game balance constants.
    pub config: EconomyConfig,
    /// Global accounts indexed by player ID.
    pub players: Vec<PlayerEconomy>,
    /// Province economic states indexed by canonical province ID.
    pub provinces: Vec<EconomicProvince>,
    /// Province graph including visible legal sea-crossing edges.
    pub adjacency: Vec<Vec<usize>>,
    /// Accepted/proposed/completed trade agreements.
    pub trades: Vec<TradeAgreement>,
    /// Number of completed economic months.
    pub month: u32,
    /// Latest month retained for inspectable UI explanations.
    pub last_report: MonthlyReport,
    pub(super) next_trade_id: u64,
    /// Per-turn one-time relation rewards, preventing split-deal relation farming.
    pub(super) one_time_relation_awarded: std::collections::BTreeMap<(usize, usize), f64>,
    /// This month's open-market Buy/Sell volume by player and resource.
    pub(super) open_market_volume: Vec<[[f64; 3]; 2]>,
}

impl EconomyWorld {
    /// Initialize from canonical map state and a validated province adjacency graph.
    pub fn new(
        player_count: usize,
        provinces: Vec<EconomicProvince>,
        adjacency: Vec<Vec<usize>>,
    ) -> Self {
        let config = EconomyConfig::default();
        let players = (0..player_count).map(|_| PlayerEconomy::new(&config)).collect();
        let mut world = Self {
            config,
            players,
            provinces,
            adjacency,
            trades: Vec::new(),
            month: 0,
            last_report: MonthlyReport::default(),
            next_trade_id: 1,
            one_time_relation_awarded: std::collections::BTreeMap::new(),
            open_market_volume: vec![[[0.0; 3]; 2]; player_count],
        };
        world.refresh_npc_markets(&MonthlyInputs::default());
        world
    }
}
