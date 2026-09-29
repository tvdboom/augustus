//! Province-based recruitment, formations, combat, and movement.
//!
//! The domain has no Bevy dependency and no persistent army objects. Civilian
//! population and resource stocks are supplied by the economy at the boundary.
//! All casualties are permanent; neither supply nor training restores manpower.

mod combat;
mod config;
mod formation;
mod movement;
mod world;

#[cfg(test)]
#[path = "../../../tests/unit/military.rs"]
mod tests;

pub use combat::*;
pub use config::*;
pub use formation::*;
pub use movement::*;
pub use world::*;

use std::collections::BTreeMap;

/// Stable map province index.
pub type ProvinceId = usize;
/// Stable player index.
pub type PlayerId = usize;
/// Stable, never reused unit identifier.
pub type UnitId = u64;

/// Internal combat-strength point to whole people; recruitment cost is separate.
pub const PEOPLE_PER_POPULATION: f64 = 100.0;

/// Military ownership is independent of territorial ownership.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ForceOwner {
    /// A player-controlled force.
    Player(PlayerId),
    /// The local defenders of the identified NPC province.
    Local(ProvinceId),
}

/// Complete initial roster; the order indexes the configuration matrices.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(usize)]
pub enum UnitType {
    /// Cheap civilian infantry.
    LightInfantry,
    /// Armored center infantry.
    HeavyInfantry,
    /// Ranged infantry support.
    Archers,
    /// Fast light mounted troops.
    LightCavalry,
    /// Armored mounted troops.
    HeavyCavalry,
    /// Fast mounted archers.
    HorseArchers,
    /// Chariot crews.
    WarChariots,
    /// Camel-mounted troops.
    WarCamels,
    /// Elephant crews and animals.
    WarElephants,
    /// Bolt-throwing artillery.
    Ballista,
    /// Stone-throwing artillery.
    Catapult,
}

impl UnitType {
    /// Every supported unit in stable UI/configuration order.
    pub const ALL: [Self; 11] = [
        Self::LightInfantry,
        Self::HeavyInfantry,
        Self::Archers,
        Self::LightCavalry,
        Self::HeavyCavalry,
        Self::HorseArchers,
        Self::WarChariots,
        Self::WarCamels,
        Self::WarElephants,
        Self::Ballista,
        Self::Catapult,
    ];
    /// Player-facing name.
    pub fn name(self) -> &'static str {
        [
            "Light Infantry",
            "Heavy Infantry",
            "Archers",
            "Light Cavalry",
            "Heavy Cavalry",
            "Horse Archers",
            "War Chariots",
            "War Camels",
            "War Elephants",
            "Ballista",
            "Catapult",
        ][self as usize]
    }
    /// Short label for a formation slot.
    pub fn abbreviation(self) -> &'static str {
        ["LI", "HI", "AR", "LC", "HC", "HA", "CH", "CA", "EL", "BA", "CT"][self as usize]
    }
    /// Whether a troop prefers the protected support row.
    pub fn is_support(self) -> bool {
        matches!(self, Self::Archers | Self::Ballista | Self::Catapult)
    }
    /// Whether a troop is mounted and receives cavalry terrain modifiers.
    pub fn is_cavalry(self) -> bool {
        matches!(
            self,
            Self::LightCavalry | Self::HeavyCavalry | Self::HorseArchers | Self::WarCamels
        )
    }
}

/// Explicit province recruitment capability; terrain never grants one implicitly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RecruitmentTag {
    /// Steppe horse archery.
    HorseArchers,
    /// Chariot culture.
    Chariots,
    /// Camel husbandry.
    Camels,
    /// Elephant husbandry.
    Elephants,
}

/// Shared terrain order for combat and travel configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum MilitaryTerrain {
    /// Open cultivated land.
    Farmland,
    /// Open grassland.
    Plains,
    /// Woodland.
    Forest,
    /// Rolling terrain.
    Hills,
    /// Mountain passes.
    Mountains,
    /// Arid terrain.
    Desert,
    /// Wetlands; uses restrictive frontage and slow travel.
    Marsh,
}

/// Persistent unit state. A unit appears in exactly one province, movement, or battle.
#[derive(Clone, Debug, PartialEq)]
pub struct Unit {
    /// Unique identity used as the final deployment tie-break.
    pub id: UnitId,
    /// Military owner.
    pub owner: ForceOwner,
    /// Roster type.
    pub unit_type: UnitType,
    /// Surviving people in economy population units, quantized to whole people.
    pub current_manpower: f64,
    /// Original recruited people in economy population units.
    pub max_manpower: f64,
    /// Experience in 0..100, shared by this owner's cohorts of the same type in the army.
    pub training: f64,
    /// Army-wide morale in 0..100, mirrored on cohorts for combat calculations.
    pub morale: f64,
}

impl Unit {
    /// Whole surviving people represented by this cohort.
    pub fn people(&self) -> u64 {
        (self.current_manpower.max(0.0) * PEOPLE_PER_POPULATION).round() as u64
    }
    /// Whole people in a fresh cohort of this type.
    pub fn max_people(&self) -> u64 {
        (self.max_manpower.max(0.0) * PEOPLE_PER_POPULATION).round() as u64
    }
    /// Cohort strength as a ratio, robust to invalid saved maximums.
    pub fn manpower_ratio(&self) -> f64 {
        if self.max_manpower > 0.0 {
            (self.current_manpower / self.max_manpower).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
    /// Monthly food request scales with surviving men and animals.
    pub fn food_demand(&self, config: &MilitaryConfig) -> f64 {
        config.unit(self.unit_type).food_per_month * self.manpower_ratio()
    }
    /// Monthly Coin wages scale with surviving personnel.
    pub fn coin_demand(&self, config: &MilitaryConfig) -> f64 {
        config.unit(self.unit_type).coin_per_month * self.manpower_ratio()
    }
    /// Strength used by garrisons and occupation, independent of combat targeting.
    pub fn effective_strength(&self, config: &MilitaryConfig) -> f64 {
        self.current_manpower.max(0.0)
            * config.unit(self.unit_type).political_strength
            * (1.0 + config.training_attack * self.training.clamp(0.0, 100.0) / 100.0)
            * (config.strength_morale_base
                + config.strength_morale_scale * self.morale.clamp(0.0, 100.0) / 100.0)
    }
}

/// Keep cohort identities/manpower while sharing army morale and type-specific training.
/// Merging uses surviving manpower, so equal-sized types trained at 10 and 20 become 15.
pub fn consolidate_army_condition(units: &mut [Unit]) {
    let mut morale = BTreeMap::<ForceOwner, (f64, f64)>::new();
    let mut training = BTreeMap::<(ForceOwner, UnitType), (f64, f64)>::new();
    for unit in units.iter().filter(|unit| unit.current_manpower > 0.) {
        let weight = unit.current_manpower;
        let army = morale.entry(unit.owner).or_default();
        army.0 += unit.morale.clamp(0., 100.) * weight;
        army.1 += weight;
        let kind = training.entry((unit.owner, unit.unit_type)).or_default();
        kind.0 += unit.training.clamp(0., 100.) * weight;
        kind.1 += weight;
    }
    for unit in units {
        if let Some(&(sum, weight)) = morale.get(&unit.owner) {
            unit.morale = sum / weight;
        }
        if let Some(&(sum, weight)) = training.get(&(unit.owner, unit.unit_type)) {
            unit.training = sum / weight;
        }
    }
}

/// Political rank never appears in this independent military ladder.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum MilitaryRank {
    /// Initial military rank.
    #[default]
    Centurion,
    /// First renown promotion.
    MilitaryTribune,
    /// Experienced commander.
    Legate,
    /// Highest military rank.
    Imperator,
}

impl MilitaryRank {
    /// Display label.
    pub fn name(self) -> &'static str {
        ["Centurion", "Military Tribune", "Legate", "Imperator"][self as usize]
    }
    /// Automatic promotion based on configured renown thresholds.
    pub fn from_renown(renown: f64, config: &MilitaryConfig) -> Self {
        let ranks = [Self::Centurion, Self::MilitaryTribune, Self::Legate, Self::Imperator];
        ranks
            .into_iter()
            .rev()
            .find(|rank| renown >= config.rank_thresholds[*rank as usize])
            .unwrap_or_default()
    }
}

/// A single province's forces and saved owner-specific plans.
#[derive(Clone, Debug, Default)]
pub struct ProvinceMilitaryState {
    /// Stationary troops grouped by their actual owner.
    pub forces: BTreeMap<ForceOwner, Vec<Unit>>,
    /// Local troops raised by an enslaved-population revolt, for map and panel labels.
    pub slave_rebellion: bool,
    /// Defaults inherited by a new movement order.
    pub plans: BTreeMap<ForceOwner, BattlePlan>,
    /// The only active recruitment slot.
    pub recruitment: Option<RecruitmentProject>,
    /// Paid waiting cohorts in click order.
    pub recruitment_queue: std::collections::VecDeque<RecruitmentProject>,
    /// Hostile occupation established only by defeating local defenders.
    pub occupation: Option<ForceOwner>,
    /// Remaining temporary happiness penalty for each civilian class.
    pub draft_penalties: [f64; 4],
}

impl ProvinceMilitaryState {
    /// Maximum recruitment orders per province, including the active project.
    pub const MAX_RECRUITMENT_ORDERS: usize = 50;

    /// Whether the active project and waiting orders fill all recruitment slots.
    pub fn recruitment_queue_full(&self) -> bool {
        self.recruitment_queue.len() + usize::from(self.recruitment.is_some())
            >= Self::MAX_RECRUITMENT_ORDERS
    }
}

/// Fully paid recruitment; waiting orders can be refunded until they start.
#[derive(Clone, Debug)]
pub struct RecruitmentProject {
    /// Owner at time of drafting.
    pub owner: ForceOwner,
    /// Roster type being equipped.
    pub unit_type: UnitType,
    /// Completed monthly work.
    pub progress: f64,
    /// Required monthly work, captured when recruitment begins.
    pub required_progress: f64,
    /// Economy population already removed from the source class.
    pub population_cost: f64,
    /// Full cohort strength to create when training completes.
    pub cohort_manpower: f64,
    /// Original civilian class to restore when removing a waiting order.
    pub manpower_class: usize,
    /// Metal paid at click time, retained independently of later config changes.
    pub paid_metal: f64,
}

/// Domain errors suitable for action-button disabled explanations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MilitaryError {
    /// Province index does not exist.
    UnknownProvince,
    /// Must own the province directly.
    NotDirectlyOwned,
    /// Province is engaged in combat.
    InBattle,
    /// Recruitment has reached its province order limit.
    RecruitmentBusy,
    /// Specialized recruitment tag is absent.
    MissingRecruitmentTag,
    /// Source civilian class is too small.
    InsufficientPopulation,
    /// Global metal stock is insufficient.
    InsufficientMetal,
    /// Selected troops are empty, duplicated, foreign, or absent.
    InvalidUnits,
    /// No legal route or edge remains.
    NoLegalRoute,
    /// The battle has not completed a full month yet.
    RetreatTooEarly,
    /// The force is already committed to another battle/order.
    InvalidBattle,
}

impl std::fmt::Display for MilitaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::UnknownProvince => "Province does not exist",
            Self::NotDirectlyOwned => "Requires a directly owned province",
            Self::InBattle => "Force is locked in battle",
            Self::RecruitmentBusy => "Recruitment queue is full",
            Self::MissingRecruitmentTag => "Province lacks the required recruitment tradition",
            Self::InsufficientPopulation => "Not enough people in the recruitment class",
            Self::InsufficientMetal => "Not enough global Metal",
            Self::InvalidUnits => "Choose available units belonging to this force",
            Self::NoLegalRoute => "No legal military route",
            Self::RetreatTooEarly => "Retreat requires one completed battle month",
            Self::InvalidBattle => "Forces cannot begin this battle",
        })
    }
}

impl std::error::Error for MilitaryError {}
