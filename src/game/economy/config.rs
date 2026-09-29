//! All economy balance values live here instead of in UI code.

use super::{BuildingDefinition, BuildingType, WonderDefinition};

/// Policy modifiers, in consumption/happiness/birth/death order.
#[derive(Clone, Copy, Debug)]
pub struct FoodModifiers {
    /// Fraction of ordinary civilian food consumption.
    pub consumption: f64,
    /// Flat happiness change for every class.
    pub happiness: f64,
    /// Multiplier on births, before shortage.
    pub births: f64,
    /// Multiplier on normal mortality.
    pub deaths: f64,
}

/// Slave policy productivity, happiness, and mortality modifiers.
#[derive(Clone, Copy, Debug)]
pub struct LaborModifiers {
    /// Multiplier on slave labor only.
    pub productivity: f64,
    /// Flat slave happiness change.
    pub happiness: f64,
    /// Multiplier on normal slave deaths.
    pub deaths: f64,
}

/// Centralized population, resource, building, and trade tuning.
#[derive(Clone, Debug)]
pub struct EconomyConfig {
    /// Farmland, Plains, Forest, Hills, Mountains, Desert, Marsh.
    pub terrain_capacity: [f64; 7],
    /// Multiplier on the map's normalized area.
    pub area_to_capacity_scale: f64,
    /// Flat city capacity.
    pub city_capacity: f64,
    /// Monthly births per class at neutral happiness.
    pub birth_rates: [f64; 4],
    /// Monthly natural mortality per class.
    pub death_rates: [f64; 4],
    /// Monthly food demand per resident in each class.
    pub food_per_class: [f64; 4],
    /// Low, Normal, High ration settings.
    pub food_policy: [FoodModifiers; 3],
    /// Light, Normal, Harsh slave labor.
    pub slave_policy: [LaborModifiers; 3],
    /// Overcrowding percentage converted into happiness points.
    pub overcrowding_scale: f64,
    /// Upper bound on overcrowding penalty.
    pub overcrowding_cap: f64,
    /// Capacity ratio above which space independently suppresses births. This
    /// prevents unlimited happiness-building bonuses defeating natural equilibrium.
    pub crowding_birth_threshold: f64,
    /// Deaths per resident at completely missing food supply.
    pub max_famine_death_rate: f64,
    /// Happiness penalty at completely missing food supply.
    pub shortage_happiness_penalty: f64,
    /// Fraction of a temporary event/recruitment happiness modifier retained each month.
    pub temporary_happiness_decay: f64,
    /// Mobile class emigration rates; slaves must remain zero.
    pub migration_rates: [f64; 4],
    /// Additional emigration pressure from excess capacity.
    pub overpopulation_migration_scale: f64,
    /// Encourage, Normal, Discourage, Closed inbound multipliers.
    pub migration_in: [f64; 4],
    /// Encourage, Normal, Discourage, Closed outbound multipliers.
    pub migration_out: [f64; 4],
    /// Flat happiness change for Nobles, Citizens and Plebeians by migration policy.
    pub migration_happiness: [f64; 4],
    /// Capacity/happiness/city contributions to destination attractiveness.
    pub migration_weights: [f64; 3],
    /// Plebeian to citizen and citizen to noble monthly promotion rates.
    pub class_change_rates: [f64; 2],
    /// Free workers and slaves' baseline productivity.
    pub productivity: [f64; 2],
    /// Food, Metal, Stone output per allocated labor/potential.
    pub production_scale: [f64; 3],
    /// Diminishing-return level for provincial Food output; zero disables it.
    pub food_output_saturation: f64,
    /// Resource focus weights, ordered Balanced, Food, Metal, Stone.
    pub focus_weights: [[f64; 3]; 4],
    /// Slow, Normal, Urgent construction speed multipliers.
    pub construction_speed: [f64; 3],
    /// Fraction of the slave population diverted while a building or wonder is active.
    pub construction_labor: [f64; 3],
    /// Frugal, Normal, Generous monthly Coin per free resident, excluding slaves.
    pub civic_coin_per_free_resident: [f64; 3],
    /// Free-class happiness at full civic funding, recomposed every month.
    pub civic_happiness: [f64; 3],
    /// Low, Normal, High recruitment progress per month at full funding.
    pub recruitment_speed: [f64; 3],
    /// Monthly Coin per recruit, charged only for active projects.
    pub recruitment_coin_per_recruit: [f64; 3],
    /// Citizen and plebeian happiness effect while recruitment is active.
    pub recruitment_happiness: [f64; 3],
    /// Enslave, Normal, Free monthly rates: negative enslaves plebeians, positive frees slaves.
    pub manumission_rates: [f64; 3],
    /// Per-class happiness changes for Enslave, Normal and Free.
    pub manumission_happiness: [[f64; 4]; 3],
    /// Global physical storage without buildings.
    pub base_storage: [f64; 3],
    /// New player starting stocks; preserves a food buffer for specialization.
    pub starting_stock: [f64; 3],
    /// Monthly sestertii per Noble/Citizen/Plebeian/Slave; nobles and slaves are exempt.
    pub tax_rates: [f64; 4],
    /// Domestic influence per noble; political rank income is separate.
    pub influence_per_noble: f64,
    /// Maximum lost class output at zero happiness, shared by all four classes.
    pub max_unhappiness_output_loss: f64,
    /// Monthly slave revolt probability at 5% and 0% happiness.
    pub slave_revolt_chance: [f64; 2],
    /// Building definitions indexed by BuildingType.
    pub buildings: Vec<BuildingDefinition>,
    /// Canonical wonder IDs map to existing map sites, never duplicate positions.
    pub wonders: Vec<WonderDefinition>,
    /// Minimum assigned slaves and corresponding construction speed.
    pub wonder_slave_speeds: Vec<(f64, f64)>,
    /// Distance and scarcity parameters.
    pub trade: TradeConfig,
}

/// NPC valuation, budgets, transport losses, and agreement failure rules.
#[derive(Clone, Debug)]
pub struct TradeConfig {
    /// Enabling influence exchanges needs explicit game configuration.
    pub allow_influence: bool,
    /// Transport loss per edge after the first.
    pub loss_per_step: f64,
    /// Lowest possible non-blocked route efficiency.
    pub minimum_efficiency: f64,
    /// Added efficiency per Road level along a route.
    pub road_efficiency: f64,
    /// Minimum relation required for transit and NPC trade.
    pub minimum_relation: f64,
    /// Coin-equivalent base value of Food, Metal, Stone.
    pub base_value: [f64; 3],
    /// Scarcity value multipliers from Severe Shortage through Large Surplus.
    pub scarcity_multipliers: [f64; 5],
    /// Relative surplus/shortage thresholds around the balanced band.
    pub demand_band_thresholds: [f64; 2],
    /// Relation bands from hostile to friendly.
    pub required_value_ratios: [f64; 5],
    /// Additional one-time transaction margin.
    pub one_time_margin: f64,
    /// Relation lost when an accepted NPC route is ended without notice.
    pub cancellation_relation_penalty: f64,
    /// Monthly deliveries retained after giving notice to an NPC.
    pub cancellation_notice_months: u32,
    /// Open-market spread above/below base resource values.
    pub open_market_spread: f64,
    /// Quantity at which transaction size adds one full base-value margin.
    pub open_market_depth: [f64; 3],
    /// Coin value of one influence when exchanges are enabled.
    pub influence_value: f64,
    /// NPC resource needs without military or development modifiers.
    pub npc_base_need: [f64; 3],
    /// Metal requirement per food unit required by local forces.
    pub npc_military_metal_need: f64,
    /// Additional stone need for urban development.
    pub npc_city_stone_need: f64,
    /// Extra stone need while actively constructing.
    pub npc_development_stone_need: f64,
    /// Share of NPC surplus available for trade.
    pub export_share: f64,
    /// Share of NPC shortage requested as imports.
    pub import_share: f64,
    /// Maximum accepted imports as a multiple of current demand.
    pub maximum_import_multiple: f64,
    /// Extra physical export allowance for a one-time trade.
    pub one_time_export_multiplier: f64,
    /// Share of recurring income budgeted for imports.
    pub income_budget_share: f64,
    /// Share of treasury budgeted for recurring imports.
    pub treasury_budget_share: f64,
    /// Maximum recurring treasury draw.
    pub maximum_treasury_draw: f64,
    /// Share of treasury available in a one-time purchase.
    pub one_time_treasury_share: f64,
    /// NPC city coin income bonus.
    pub npc_city_income: f64,
    /// NPC routine expenses as a fraction of income.
    pub npc_expense_share: f64,
    /// Fraction of promised NPC supply below which a month is suspended.
    pub minimum_fulfillment: f64,
    /// Consecutive failures needed for automatic cancellation.
    pub failure_limit: u8,
    /// Delivered value needed for one relation point.
    pub value_per_relation: f64,
    /// Maximum recurring relation gain per province/player/month.
    pub relation_cap: f64,
    /// Fraction of regular relation gain awarded by one-time trades.
    pub one_time_relation_factor: f64,
    /// Control gain per delivered value, before foreign-trade share.
    pub control_scale: f64,
    /// Maximum passive trade control per province/player/month.
    pub control_cap: f64,
}

impl Default for EconomyConfig {
    /// Defaults target the existing map's aggregate population units, not persons.
    fn default() -> Self {
        Self {
            terrain_capacity: [1.5, 1.0, 0.75, 0.7, 0.35, 0.2, 0.6],
            area_to_capacity_scale: 2.0,
            // Urban starts can receive population compensation; house it before
            // overcrowding suppresses all labor and triggers an idle revolt.
            city_capacity: 90.0,
            birth_rates: [0.0035, 0.004, 0.0045, 0.004],
            death_rates: [0.002; 4],
            food_per_class: [1.0; 4],
            food_policy: [
                FoodModifiers {
                    consumption: 0.8,
                    happiness: -10.0,
                    births: 0.75,
                    deaths: 1.15,
                },
                FoodModifiers {
                    consumption: 1.0,
                    happiness: 0.0,
                    births: 1.0,
                    deaths: 1.0,
                },
                FoodModifiers {
                    consumption: 1.2,
                    happiness: 10.0,
                    births: 1.1,
                    deaths: 0.95,
                },
            ],
            slave_policy: [
                LaborModifiers {
                    productivity: 0.8,
                    happiness: 10.0,
                    deaths: 0.9,
                },
                LaborModifiers {
                    productivity: 1.0,
                    happiness: 0.0,
                    deaths: 1.0,
                },
                LaborModifiers {
                    productivity: 1.25,
                    happiness: -15.0,
                    deaths: 1.2,
                },
            ],
            overcrowding_scale: 100.0,
            overcrowding_cap: 50.0,
            crowding_birth_threshold: 1.5,
            max_famine_death_rate: 0.08,
            shortage_happiness_penalty: 50.0,
            temporary_happiness_decay: 0.85,
            migration_rates: [0.002, 0.01, 0.015, 0.0],
            overpopulation_migration_scale: 2.0,
            migration_in: [1.5, 1.0, 0.5, 0.1],
            migration_out: [1.0, 1.0, 0.7, 0.1],
            migration_happiness: [0.0, 0.0, -1.0, -2.0],
            migration_weights: [2.0, 1.0, 0.25],
            class_change_rates: [0.0008, 0.00025],
            productivity: [1.0, 1.5],
            // Only plebs/slaves produce; Food supports all four classes.
            production_scale: [3.3, 0.9, 1.2],
            food_output_saturation: 400.0,
            focus_weights: [[1.0; 3], [3.0, 1.0, 1.0], [1.0, 3.0, 1.0], [1.0, 1.0, 3.0]],
            construction_speed: [0.75, 1.0, 1.25],
            construction_labor: [0.05, 0.10, 0.20],
            civic_coin_per_free_resident: [0.0, 0.1, 0.2],
            civic_happiness: [-1.0, 0.0, 1.0],
            recruitment_speed: [0.75, 1.0, 1.25],
            recruitment_coin_per_recruit: [0.1, 0.2, 0.3],
            recruitment_happiness: [1.0, 0.0, -1.0],
            manumission_rates: [-0.002, 0.0, 0.002],
            manumission_happiness: [[0.0, 0.0, -1.0, 0.0], [0.0; 4], [-1.0, 0.0, 0.0, 0.0]],
            base_storage: [2400.0, 1600.0, 4000.0],
            starting_stock: [450.0, 120.0, 200.0],
            tax_rates: [0.0, 0.5, 0.2, 0.0],
            influence_per_noble: 0.25,
            max_unhappiness_output_loss: 0.5,
            slave_revolt_chance: [0.10, 0.50],
            buildings: BuildingType::ALL.into_iter().map(BuildingDefinition::for_type).collect(),
            wonders: (0..crate::map::WONDER_COUNT).map(WonderDefinition::for_site).collect(),
            wonder_slave_speeds: vec![
                (0.0, 1.0),
                (25.0, 1.25),
                (50.0, 1.5),
                (100.0, 1.75),
                (200.0, 2.0),
            ],
            trade: TradeConfig::default(),
        }
    }
}

impl Default for TradeConfig {
    /// Trade losses and suspension rules follow the specification's examples.
    fn default() -> Self {
        Self {
            allow_influence: false,
            loss_per_step: 0.05,
            minimum_efficiency: 0.5,
            road_efficiency: 0.005,
            minimum_relation: 20.0,
            base_value: [1.0, 2.5, 1.5],
            scarcity_multipliers: [1.75, 1.35, 1.0, 0.85, 0.7],
            demand_band_thresholds: [0.1, 0.5],
            required_value_ratios: [1.5, 1.25, 1.1, 1.05, 1.0],
            one_time_margin: 1.1,
            cancellation_relation_penalty: 10.0,
            cancellation_notice_months: 6,
            open_market_spread: 0.1,
            open_market_depth: [1000.0, 400.0, 600.0],
            influence_value: 8.0,
            npc_base_need: [0.0, 2.0, 2.0],
            npc_military_metal_need: 0.1,
            npc_city_stone_need: 4.0,
            npc_development_stone_need: 6.0,
            export_share: 1.0,
            import_share: 1.0,
            maximum_import_multiple: 1.5,
            one_time_export_multiplier: 1.5,
            income_budget_share: 0.4,
            treasury_budget_share: 0.05,
            maximum_treasury_draw: 25.0,
            one_time_treasury_share: 0.5,
            npc_city_income: 5.0,
            npc_expense_share: 0.3,
            minimum_fulfillment: 0.8,
            failure_limit: 3,
            value_per_relation: 100.0,
            relation_cap: 1.0,
            one_time_relation_factor: 0.1,
            control_scale: 0.01,
            control_cap: 1.0,
        }
    }
}
