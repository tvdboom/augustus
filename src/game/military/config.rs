//! Central balance data. Cohort size and recruitment population cost are separate.

use super::*;

/// All recruitment and combat data for a roster type.
#[derive(Clone, Debug)]
pub struct UnitDefinition {
    /// Index into Nobles, Citizens, Plebeians, Slaves; only 1 or 2 is used.
    pub manpower_class: usize,
    /// Nominal cohort strength in hundred-person blocks.
    pub manpower: f64,
    /// Economy population units removed from the source province on recruitment.
    pub population_cost: f64,
    /// One-time metal equipment expense.
    pub metal_cost: f64,
    /// Work required before the cohort appears.
    pub recruitment_months: f64,
    /// Monthly request for a full-strength cohort.
    pub food_per_month: f64,
    /// Monthly Coin wages for a full-strength cohort.
    pub coin_per_month: f64,
    /// Strength multiplier used for political occupation.
    pub political_strength: f64,
    /// Optional explicit province requirement.
    pub special_tag: Option<RecruitmentTag>,
    /// Relative attack strength.
    pub offense: f64,
    /// Relative resistance to damage.
    pub defense: f64,
    /// Travel speed; a group uses its slowest member.
    pub movement_speed: f64,
    /// Maximum sideways target reach.
    pub maneuver: usize,
    /// Fort suppression contributed by an active surviving cohort.
    pub siege_power: f64,
}

impl UnitDefinition {
    /// Whole people in a newly recruited cohort.
    pub fn cohort_people(&self) -> u64 {
        (self.manpower * PEOPLE_PER_POPULATION).round() as u64
    }
}

/// Editable balance configuration; matrices are indexed by enum discriminants.
#[derive(Clone, Debug)]
pub struct MilitaryConfig {
    /// Complete definitions in `UnitType::ALL` order.
    pub units: [UnitDefinition; 11],
    /// Asymmetric attack multipliers, including siege weapons.
    pub matchups: [[f64; 11]; 11],
    /// Fit for Balanced, Shock, Bottleneck, Envelopment, Skirmishing, Deception.
    pub tactic_fit: [[f64; 6]; 11],
    /// Counter target of each tactic, or None for Balanced.
    pub counters: [Option<CombatTactic>; 6],
    /// Maximum bonus when countering with perfect composition.
    pub tactic_bonus: f64,
    /// Damage multiplier when countered.
    pub countered_multiplier: f64,
    /// Both sides' casualty intensity from each selected tactic.
    pub casualty_intensity: [f64; 6],
    /// Front/support width by terrain.
    pub combat_widths: [usize; 7],
    /// Fallback center deployment, highest priority first.
    pub center_priority: [UnitType; 11],
    /// Fallback flank deployment, highest priority first.
    pub flank_priority: [UnitType; 11],
    /// Support deployment, highest priority first.
    pub support_priority: [UnitType; 3],
    /// Terrain attack modifiers indexed terrain then unit type.
    pub terrain_attack: [[f64; 11]; 7],
    /// Defender-only terrain resistance.
    pub terrain_defense: [f64; 7],
    /// Attack multiplier while protected behind the front.
    pub support_effectiveness: f64,
    /// Combat pressure multiplier when support is exposed.
    pub exposed_support_casualties: f64,
    /// Defense bonus per fort/wall level.
    pub fort_defense_per_level: f64,
    /// Maximum additive fort defense.
    pub fort_defense_cap: f64,
    /// Fraction of fort bonus suppressed per siege point.
    pub siege_suppression_per_point: f64,
    /// Maximum fraction of fort defense suppressed.
    pub siege_suppression_cap: f64,
    /// Training attack gain at 100 experience.
    pub training_attack: f64,
    /// Training defensive gain at 100 experience.
    pub training_defense: f64,
    /// Attack output at zero Morale.
    pub strength_morale_base: f64,
    /// Additional attack output at 100 Morale.
    pub strength_morale_scale: f64,
    /// Inclusive random multiplier bounds.
    pub random_range: [f64; 2],
    /// Fraction of target nominal manpower lost per equal-strength attack.
    pub base_manpower_damage: f64,
    /// Morale points lost per equal-strength attack.
    pub base_morale_damage: f64,
    /// Simultaneous rounds each month.
    pub rounds_per_month: usize,
    /// Attacker withdrawal deadline.
    pub maximum_battle_months: usize,
    /// Minimum completed months before voluntary retreat.
    pub minimum_retreat_months: usize,
    /// Starting experience of recruited troops.
    pub starting_training: f64,
    /// Monthly supplied experience gain for every living cohort.
    pub passive_training: f64,
    /// Supply ratio required for passive training.
    pub training_supply_threshold: f64,
    /// Morale baseline before rank/training effects.
    pub base_morale: f64,
    /// Maximum recovery toward baseline per peaceful month.
    pub morale_recovery: f64,
    /// Morale lost for a fully unsupplied month.
    pub shortage_morale_penalty: f64,
    /// Participation experience gain after battle.
    pub participation_training: f64,
    /// Additional experience for a winner.
    pub victory_training: f64,
    /// Maximum post-battle experience gain.
    pub maximum_battle_training: f64,
    /// Winner morale recovery after battle.
    pub victory_morale: f64,
    /// Draft penalty proportionality constant.
    pub draft_happiness_scale: f64,
    /// Flat local happiness cost of raising one cohort.
    pub base_draft_penalty: f64,
    /// Maximum penalty from one draft.
    pub maximum_draft_penalty: f64,
    /// Draft penalty points removed per month.
    pub draft_penalty_decay: f64,
    /// Minimum NPC relation for peaceful troop passage.
    pub npc_access_relation: f64,
    /// Minimum NPC relation for peaceful troop stationing.
    pub npc_stationing_relation: f64,
    /// Rank renown thresholds.
    pub rank_thresholds: [f64; 4],
    /// Rank bonuses to combat morale.
    pub rank_morale: [f64; 4],
    /// Rank multipliers to occupation/garrison strength.
    pub rank_control: [f64; 4],
    /// Structural Military Senate-bloc score, never fixed votes.
    pub rank_senate: [f64; 4],
    /// Diminishing-return control maximum.
    pub maximum_garrison_control: f64,
    /// Effective power required for half the maximum garrison benefit.
    pub garrison_half_saturation: f64,
    /// Monthly hostile occupation relation loss.
    pub occupation_relation_loss: f64,
    /// Province crossing cost by terrain.
    pub terrain_movement: [f64; 7],
    /// Additional road speed per level.
    pub road_speed_bonus: f64,
    /// Minimum road-adjusted crossing cost multiplier.
    pub minimum_road_cost: f64,
    /// Reference speed used to normalize monthly travel.
    pub reference_speed: f64,
    /// Overall movement duration scale.
    pub movement_scale: f64,
    /// Camera scale below which representative troop sprites are hidden.
    pub sprite_zoom_threshold: f64,
    /// Minimum manpower share for secondary representative types.
    pub representative_share_threshold: f64,
    /// Maximum representative types per owner.
    pub maximum_representatives: usize,
}

impl MilitaryConfig {
    /// Retrieve immutable roster data.
    pub fn unit(&self, kind: UnitType) -> &UnitDefinition {
        &self.units[kind as usize]
    }
    /// Political control pressure, bounded and with diminishing returns.
    pub fn garrison_control(&self, effective_strength: f64, rank: MilitaryRank) -> f64 {
        let power = effective_strength.max(0.0) * self.rank_control[rank as usize];
        self.maximum_garrison_control * power / (power + self.garrison_half_saturation.max(0.001))
    }
}

impl Default for MilitaryConfig {
    fn default() -> Self {
        use RecruitmentTag as Tag;
        use UnitType::*;
        let manpower = [10., 10., 8., 5., 4., 5., 3., 4., 2., 3., 3.];
        let population_cost = [1.; 11];
        let metal = [12., 40., 16., 28., 48., 40., 24., 36., 100., 60., 80.];
        let months = [2., 3., 2., 3., 4., 3., 3., 3., 5., 4., 5.];
        // Aggregate population scale: infantry < cavalry < elephants.
        let food = [1.5, 2.5, 1.5, 3., 4., 3., 3., 2.5, 6., 1.5, 2.];
        let wages = [1., 2., 1., 2., 3., 2.5, 2.5, 2.5, 5., 2., 2.];
        let stats: [[f64; 4]; 11] = [
            [0.80, 0.85, 2.5, 1.],
            [1.15, 1.25, 2.5, 1.],
            [1.00, 0.75, 2.5, 2.],
            [1.00, 0.90, 4.0, 3.],
            [1.30, 1.20, 3.5, 2.],
            [1.15, 0.90, 4.0, 5.],
            [1.00, 0.90, 2.5, 1.],
            [1.05, 0.95, 3.5, 4.],
            [1.60, 1.50, 2.5, 0.],
            [0.70, 0.30, 2.0, 0.],
            [0.80, 0.25, 1.5, 0.],
        ];
        let units = std::array::from_fn(|i| UnitDefinition {
            manpower_class: if matches!(
                UnitType::ALL[i],
                LightCavalry | HeavyCavalry | HorseArchers | WarChariots | WarCamels | WarElephants
            ) {
                1
            } else {
                2
            },
            manpower: manpower[i],
            population_cost: population_cost[i],
            metal_cost: metal[i],
            recruitment_months: months[i],
            food_per_month: food[i],
            coin_per_month: wages[i],
            political_strength: [0.8, 1.25, 0.8, 1.4, 1.8, 1.7, 1.5, 1.6, 4., 0.4, 0.4][i],
            special_tag: match UnitType::ALL[i] {
                HorseArchers => Some(Tag::HorseArchers),
                WarChariots => Some(Tag::Chariots),
                WarCamels => Some(Tag::Camels),
                WarElephants => Some(Tag::Elephants),
                _ => None,
            },
            offense: stats[i][0],
            defense: stats[i][1],
            movement_speed: stats[i][2],
            maneuver: stats[i][3] as usize,
            siege_power: if i == 9 {
                1.
            } else if i == 10 {
                2.
            } else {
                0.
            },
        });
        let basic = [
            [1., 0.70, 0.95, 0.80, 0.65, 0.75, 0.75, 0.85, 0.55],
            [1.35, 1., 1.05, 1.25, 1.20, 0.85, 1.40, 1.10, 0.80],
            [1.25, 1.10, 1., 0.65, 0.55, 0.70, 1., 0.70, 0.80],
            [1.20, 0.60, 1.50, 1., 0.75, 1.35, 1.15, 0.85, 0.50],
            [1.35, 0.75, 1.50, 1.30, 1., 1.20, 1.35, 1.10, 0.65],
            [1.45, 1.25, 1.30, 0.75, 0.70, 1., 1.30, 0.90, 1.10],
            [1.50, 0.60, 1.20, 0.75, 0.60, 0.65, 1., 0.85, 0.50],
            [1.30, 0.80, 1.20, 1.20, 0.90, 1.15, 1.25, 1., 0.60],
            [1.60, 1.40, 1.25, 1.60, 1.50, 0.90, 1.60, 1.40, 1.],
        ];
        let mut matchups = [[1.; 11]; 11];
        for a in 0..9 {
            for d in 0..9 {
                matchups[a][d] = basic[a][d];
            }
        }
        matchups[9][1] = 1.35;
        matchups[9][8] = 1.50;
        matchups[9][4] = 1.10;
        matchups[10][0] = 1.10;
        matchups[10][1] = 1.20;
        matchups[10][8] = 1.30;
        for row in &mut matchups[9..] {
            for fast in [3, 5, 7] {
                row[fast] = 0.60;
            }
        }
        let mut terrain_attack = [[1.; 11]; 7];
        for t in [0, 1] {
            for u in [3, 4, 6] {
                terrain_attack[t][u] = 1.10;
            }
        }
        for t in [2, 3] {
            terrain_attack[t][2] = 1.10;
            for u in [3, 4, 5, 7] {
                terrain_attack[t][u] = 0.90;
            }
        }
        terrain_attack[2][0] = 1.10;
        terrain_attack[2][6] = 0.80;
        terrain_attack[3][6] = 0.85;
        for u in [3, 4, 5, 7] {
            terrain_attack[4][u] = 0.75;
            terrain_attack[6][u] = 0.75;
        }
        terrain_attack[4][6] = 0.60;
        terrain_attack[4][8] = 0.90;
        terrain_attack[6][6] = 0.60;
        terrain_attack[5][7] = 1.20;
        terrain_attack[5][3] = 1.05;
        terrain_attack[5][1] = 0.95;
        Self {
            units,
            matchups,
            terrain_attack,
            tactic_fit: [
                [1., 0.50, 0.80, 0.30, 0.80, 0.60],
                [1., 1., 1., 0.20, 0.20, 0.60],
                [1., 0.20, 0.80, 0.30, 1., 0.70],
                [1., 0.50, 0.20, 1., 0.70, 0.90],
                [1., 1., 0.30, 0.90, 0.30, 0.70],
                [1., 0.25, 0.20, 1., 1., 1.],
                [1., 0.75, 0.30, 0.80, 0.50, 1.],
                [1., 0.50, 0.20, 1., 0.60, 0.90],
                [1., 1., 0.75, 0.10, 0.10, 0.30],
                [1., 0., 0.50, 0., 0.50, 0.20],
                [1., 0., 0.50, 0., 0.40, 0.20],
            ],
            counters: [
                None,
                Some(CombatTactic::Deception),
                Some(CombatTactic::ShockAction),
                Some(CombatTactic::Bottleneck),
                Some(CombatTactic::Envelopment),
                Some(CombatTactic::Skirmishing),
            ],
            tactic_bonus: 0.20,
            countered_multiplier: 0.90,
            casualty_intensity: [1., 1.10, 1., 1., 0.90, 1.],
            combat_widths: [16, 16, 12, 12, 10, 16, 10],
            center_priority: [
                WarElephants,
                HeavyInfantry,
                LightInfantry,
                WarChariots,
                HeavyCavalry,
                WarCamels,
                LightCavalry,
                HorseArchers,
                Archers,
                Ballista,
                Catapult,
            ],
            flank_priority: [
                HorseArchers,
                WarCamels,
                LightCavalry,
                HeavyCavalry,
                WarChariots,
                LightInfantry,
                HeavyInfantry,
                WarElephants,
                Archers,
                Ballista,
                Catapult,
            ],
            support_priority: [Archers, Ballista, Catapult],
            terrain_defense: [1., 1., 1.10, 1.10, 1.20, 1., 1.10],
            support_effectiveness: 0.50,
            exposed_support_casualties: 1.50,
            fort_defense_per_level: 0.05,
            fort_defense_cap: 0.30,
            siege_suppression_per_point: 0.05,
            siege_suppression_cap: 0.80,
            training_attack: 0.15,
            training_defense: 0.25,
            strength_morale_base: 0.75,
            strength_morale_scale: 0.50,
            random_range: [0.90, 1.10],
            base_manpower_damage: 0.055,
            base_morale_damage: 8.0,
            rounds_per_month: 4,
            maximum_battle_months: 6,
            minimum_retreat_months: 1,
            starting_training: 10.,
            passive_training: 1.,
            training_supply_threshold: 0.95,
            base_morale: 50.,
            morale_recovery: 5.,
            shortage_morale_penalty: 25.,
            participation_training: 2.,
            victory_training: 1.,
            maximum_battle_training: 6.,
            victory_morale: 10.,
            draft_happiness_scale: 100.,
            base_draft_penalty: 2.,
            maximum_draft_penalty: 20.,
            draft_penalty_decay: 1.,
            npc_access_relation: 60.,
            npc_stationing_relation: 80.,
            rank_thresholds: [0., 150., 400., 900.],
            rank_morale: [0., 5., 10., 15.],
            rank_control: [1., 1.10, 1.20, 1.30],
            rank_senate: [0., 8., 16., 25.],
            maximum_garrison_control: 4.,
            garrison_half_saturation: 20.,
            occupation_relation_loss: 2.,
            terrain_movement: [0.90, 1., 1.20, 1.25, 1.60, 1.30, 1.50],
            road_speed_bonus: 0.15,
            minimum_road_cost: 0.50,
            reference_speed: 2.5,
            movement_scale: 1.,
            sprite_zoom_threshold: 2.3,
            representative_share_threshold: 0.25,
            maximum_representatives: 3,
        }
    }
}
