//! Named Roman regions over the Augustus geographic map.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use rand::random_range;
use serde::Deserialize;

use super::population::starting_total_for;
use super::production::{for_province, PRODUCTION_ICONS};
use super::terrain::paint_rivers;
use super::terrain_type::{for_province as terrain_for_province, TerrainType};
use super::water::{paint_lake, paint_water};
use super::{paint_marker_icon, Governance, MarkerIcon};
use crate::app::{
    map_hud_contains, GovernancePanelOpen, MapDetail, MapPanelCloseClick, ProvincePanelOpen,
};

pub(super) const LONGITUDE_SCALE: f32 = 0.766; // Equirectangular scale at roughly 40° north.
const MIN_ZOOM: f32 = 0.9;
const MAX_ZOOM: f32 = 8.0;
const LABEL_ZOOM_STEP: f32 = 0.25;
const LABEL_ZOOM_LEVELS: usize = ((MAX_ZOOM - MIN_ZOOM) / LABEL_ZOOM_STEP) as usize + 2;
const ZOOM_RESPONSE_RATE: f32 = 18.0;
const CITY_BLEND_START: f32 = 1.35;
const CITY_BLEND_END: f32 = 2.15;
const CITY_IMAGE_SCALE: f32 = 1.18;
const PAN_MARGIN_SCREEN_FRACTION: f32 = 0.025;
const PAN_MARGIN_FULL_ZOOM: f32 = 2.0;
const PAN_OVERSCROLL_SCREEN_FRACTION: f32 = 0.02;
const BOUNDS_RETURN_RATE: f32 = 14.0;
const DEEP_SEA: egui::Color32 = egui::Color32::from_rgb(37, 76, 112);
const CLOSE_SEA: egui::Color32 = egui::Color32::from_rgb(49, 105, 139);
const LAND: egui::Color32 = egui::Color32::from_rgb(189, 192, 171);
const BORDER: egui::Color32 = egui::Color32::from_rgb(145, 106, 82);
const INK: egui::Color32 = egui::Color32::from_rgb(51, 40, 33);
const LABEL_HORIZONTAL_MARGIN: f32 = 0.18;
const MIN_START_DISTANCE_KM: f32 = 1_000.0;
const TWO_PLAYER_MIN_START_DISTANCE_KM: f32 = 2_100.0;
const ROME_PROVINCE: &str = "Latium";
// Provinces containing the city overlays in CITIES.
const URBAN_PROVINCES: [&str; 8] = [
    "Latium",
    "Lugdunensis",
    "Tarraconensis",
    "Africa Proconsularis",
    "Achaia",
    "Aegyptus",
    "Asia",
    "Syria",
];

fn city_name_for_province(name: &str) -> Option<&'static str> {
    match name {
        "Latium" => Some("Rome"),
        "Lugdunensis" => Some("Lutetia"),
        "Tarraconensis" => Some("Tarraco"),
        "Africa Proconsularis" => Some("Carthage"),
        "Achaia" => Some("Athens"),
        "Aegyptus" => Some("Alexandria"),
        "Asia" => Some("Ephesus"),
        "Syria" => Some("Antioch"),
        _ => None,
    }
}

const INFLUENCE_PER_NOBLE: f64 = 1.0;
// Extra residents at a resource-poor start favor productive labor. The small
// upper-class share also gives a modest coin and influence compensation.
const START_BONUS_SHARES: [f64; 4] = [0.05, 0.10, 0.35, 0.50];

#[derive(Default, Resource)]
/// Province ownership and population for the current local game.
pub(crate) struct ProvinceOwnership {
    owners: Vec<Option<usize>>,
    player_colors: Vec<egui::Color32>,
    map_colors: Vec<egui::Color32>,
    populations: Vec<ProvincePopulation>,
    governance: Vec<Governance>,
}

/// Current values used by the selected province's overview card.
pub(crate) struct ProvinceOverview {
    pub name: &'static str,
    pub terrain: TerrainType,
    pub owner: Option<usize>,
    pub owner_color: Option<egui::Color32>,
    pub population: [f64; 4],
    pub base_resources: [i32; 3],
    pub production: [f64; 3],
    pub city_name: Option<&'static str>,
}

#[derive(Clone, Copy)]
struct ProvincePopulation {
    nobles: f64,
    citizens: f64,
    plebeians: f64,
    slaves: f64,
}

impl ProvincePopulation {
    fn starting(province: &Province) -> Self {
        let urban = URBAN_PROVINCES.contains(&province.name.as_str());
        let area: f64 = province
            .parts
            .iter()
            .flat_map(|part| {
                part.t.chunks_exact(3).map(|triangle| {
                    terrain_triangle_area(
                        part.v[triangle[0] as usize],
                        part.v[triangle[1] as usize],
                        part.v[triangle[2] as usize],
                    )
                })
            })
            .sum();
        let total = starting_total_for(area, urban);
        // Nobles, citizens, plebeians, slaves. City provinces shift a few
        // points toward the two upper classes; each game varies slightly.
        let shares = if urban {
            [0.12, 0.23, 0.37]
        } else {
            [0.10, 0.20, 0.40]
        };
        let nobles = total * (shares[0] + random_range(-0.02..=0.02));
        let citizens = total * (shares[1] + random_range(-0.02..=0.02));
        let plebeians = total * (shares[2] + random_range(-0.02..=0.02));
        Self {
            nobles,
            citizens,
            plebeians,
            slaves: total - nobles - citizens - plebeians,
        }
    }

    fn counts(self) -> [f64; 4] {
        [self.nobles, self.citizens, self.plebeians, self.slaves]
    }

    fn total(self) -> f64 {
        self.counts().into_iter().sum()
    }

    fn monthly_growth(self, rate: f64, slave_rate: f64) -> f64 {
        if rate == slave_rate {
            self.total() * rate
        } else {
            (self.total() - self.slaves) * rate + self.slaves * slave_rate
        }
    }

    fn grow(&mut self, rate: f64, slave_rate: f64) {
        let factor = 1.0 + rate;
        self.nobles *= factor;
        self.citizens *= factor;
        self.plebeians *= factor;
        self.slaves *= 1.0 + slave_rate;
    }

    fn lose(&mut self, amount: f64) -> f64 {
        let mut remaining = amount;
        for class in [&mut self.plebeians, &mut self.citizens, &mut self.slaves, &mut self.nobles] {
            let removed = remaining.min(*class);
            *class -= removed;
            remaining -= removed;
        }
        amount - remaining
    }

    fn weighted_workers(self) -> f64 {
        self.plebeians + self.slaves * 1.5
    }

    fn workers_under(self, governance: Governance) -> f64 {
        self.plebeians + self.slaves * 1.5 * governance.slave_labor_factor()
    }

    fn food_upkeep(self) -> f64 {
        self.total()
    }

    fn add_start_bonus(&mut self, amount: f64) {
        self.nobles += amount * START_BONUS_SHARES[0];
        self.citizens += amount * START_BONUS_SHARES[1];
        self.plebeians += amount * START_BONUS_SHARES[2];
        self.slaves += amount * START_BONUS_SHARES[3];
    }
}

fn monthly_output(base: [i32; 3], population: ProvincePopulation) -> [f64; 3] {
    let workers = population.weighted_workers();
    base.map(|yield_per_worker| f64::from(yield_per_worker) * workers)
}

/// One balance point per unit of monthly net food, metal, stone, coin, or
/// influence at default governance. This is only used to size opening bonuses;
/// the province's real resource yields and the economy rules remain intact.
fn starting_economy_score(base: [i32; 3], population: ProvincePopulation) -> f64 {
    monthly_output(base, population).into_iter().sum::<f64>() - population.food_upkeep()
        + population.plebeians
        + population.citizens * 1.5
        + population.nobles * (1.0 + INFLUENCE_PER_NOBLE)
}

fn balanced_starting_population(
    base: [i32; 3],
    mut population: ProvincePopulation,
    target_score: f64,
) -> ProvincePopulation {
    let gap = (target_score - starting_economy_score(base, population)).max(0.0);
    let mut one_bonus_resident = ProvincePopulation {
        nobles: 0.0,
        citizens: 0.0,
        plebeians: 0.0,
        slaves: 0.0,
    };
    one_bonus_resident.add_start_bonus(1.0);
    let bonus_value = starting_economy_score(base, one_bonus_resident);
    assert!(bonus_value > 0.0, "starting province cannot support a population bonus");
    population.add_start_bonus(gap / bonus_value);
    population
}

impl ProvinceOwnership {
    pub(crate) fn province_overview(&self, index: usize) -> Option<ProvinceOverview> {
        let province = atlas().provinces.get(index)?;
        let population = self.populations.get(index)?;
        Some(ProvinceOverview {
            name: province.name.as_str(),
            terrain: terrain_for_province(&province.name),
            owner: self.owners.get(index).copied().flatten(),
            owner_color: self.color(index),
            population: population.counts(),
            base_resources: province.production,
            production: self.output_for(index),
            city_name: city_name_for_province(&province.name),
        })
    }

    pub(crate) fn can_trade_with(&self, index: usize, player: usize) -> bool {
        self.owners.get(index).is_some_and(|owner| *owner != Some(player))
            && atlas().adjacency.get(index).is_some_and(|neighbors| {
                neighbors.iter().any(|&neighbor| self.owners.get(neighbor) == Some(&Some(player)))
            })
    }

    pub(crate) fn start_game(&mut self, player_colors: &[egui::Color32]) {
        self.owners = vec![None; atlas().provinces.len()];
        self.player_colors = player_colors.to_vec();
        self.map_colors = player_colors.to_vec();
        self.governance = vec![Governance::default(); player_colors.len()];
        self.populations = atlas().provinces.iter().map(ProvincePopulation::starting).collect();
        let target_score = starting_candidates()
            .into_iter()
            .map(|index| {
                starting_economy_score(atlas().provinces[index].production, self.populations[index])
            })
            .fold(0.0_f64, f64::max);
        for (player, province) in spread_out_starts(player_colors.len()).into_iter().enumerate() {
            self.owners[province] = Some(player);
            self.populations[province] = balanced_starting_population(
                atlas().provinces[province].production,
                self.populations[province],
                target_score,
            );
        }
    }

    pub(crate) fn governance_for(&self, player: usize) -> Governance {
        self.governance.get(player).copied().unwrap_or_default()
    }

    pub(crate) fn set_governance_for(&mut self, player: usize, governance: Governance) {
        if let Some(current) = self.governance.get_mut(player) {
            *current = governance;
        }
    }

    fn color(&self, province: usize) -> Option<egui::Color32> {
        self.owners
            .get(province)
            .and_then(|owner| owner.and_then(|player| self.player_colors.get(player).copied()))
    }

    pub(crate) fn set_map_colors(&mut self, map_colors: &[egui::Color32]) {
        assert_eq!(map_colors.len(), self.player_colors.len());
        self.map_colors.copy_from_slice(map_colors);
    }

    fn map_color(&self, province: usize) -> Option<egui::Color32> {
        self.owners
            .get(province)
            .and_then(|owner| owner.and_then(|player| self.map_colors.get(player).copied()))
    }

    pub(crate) fn production_for(&self, player: usize) -> [f64; 3] {
        let mut total = [0.0; 3];
        for (index, owner) in self.owners.iter().enumerate() {
            if *owner == Some(player) {
                for (resource, amount) in total.iter_mut().zip(self.output_for(index)) {
                    *resource += amount;
                }
            }
        }
        total
    }

    pub(crate) fn net_production_for(&self, player: usize) -> [f64; 3] {
        let mut total = self.production_for(player);
        let civilian_food = self
            .owners
            .iter()
            .enumerate()
            .filter(|(_, owner)| **owner == Some(player))
            .map(|(index, _)| self.food_upkeep_for(index))
            .sum::<f64>();
        total[0] -= civilian_food + self.military_food_upkeep_for(player);
        total
    }

    fn military_food_upkeep_for(&self, _player: usize) -> f64 {
        // No recruited units exist yet; the future roster will supply rations.
        0.0
    }

    pub(crate) fn output_for(&self, province: usize) -> [f64; 3] {
        let population = self
            .populations
            .get(province)
            .copied()
            .unwrap_or_else(|| ProvincePopulation::starting(&atlas().provinces[province]));
        let governance = self
            .owners
            .get(province)
            .and_then(|owner| *owner)
            .map(|player| self.governance_for(player))
            .unwrap_or_default();
        let base = atlas().provinces[province].production;
        if governance == Governance::default() {
            monthly_output(base, population)
        } else {
            let workers = population.workers_under(governance);
            base.map(|yield_per_worker| f64::from(yield_per_worker) * workers)
        }
    }

    fn food_upkeep_for(&self, province: usize) -> f64 {
        let player = self.owners[province];
        self.populations[province].food_upkeep()
            * player.map(|player| self.governance_for(player).food_per_person()).unwrap_or(1.0)
    }

    pub(crate) fn production_sources(
        &self,
        player: usize,
        resource: usize,
    ) -> Vec<(&str, f64, f64)> {
        self.owners
            .iter()
            .enumerate()
            .filter(|(_, owner)| **owner == Some(player))
            .map(|(index, _)| {
                let province = &atlas().provinces[index];
                (
                    province.name.as_str(),
                    self.output_for(index)[resource],
                    if resource == 0 {
                        self.food_upkeep_for(index)
                    } else {
                        0.0
                    },
                )
            })
            .collect()
    }

    pub(crate) fn population_for(&self, player: usize) -> [f64; 4] {
        let mut total = [0.0; 4];
        for (index, owner) in self.owners.iter().enumerate() {
            if *owner == Some(player) {
                for (sum, count) in total.iter_mut().zip(self.populations[index].counts()) {
                    *sum += count;
                }
            }
        }
        total
    }

    pub(crate) fn total_population_for(&self, player: usize) -> f64 {
        self.population_for(player).into_iter().sum()
    }

    pub(crate) fn population_sources(&self, player: usize, class: usize) -> Vec<(&str, f64)> {
        self.owners
            .iter()
            .enumerate()
            .filter(|(_, owner)| **owner == Some(player))
            .map(|(index, _)| {
                (atlas().provinces[index].name.as_str(), self.populations[index].counts()[class])
            })
            .collect()
    }

    pub(crate) fn coin_taxes_for(&self, player: usize) -> f64 {
        let population = self.population_for(player);
        population[2]
            + population[1] * 1.5
            + population[0] * self.governance_for(player).noble_tax_per_person()
    }

    pub(crate) fn coin_delta_for(&self, player: usize) -> f64 {
        self.coin_taxes_for(player) - self.military_wages_for(player)
    }

    pub(crate) fn military_wages_for(&self, player: usize) -> f64 {
        // The local map has no recruited units yet. The future military
        // roster will supply its base cost when recruitment is implemented.
        let base_cost = 0.0;
        base_cost * self.governance_for(player).army_wage_factor()
    }

    pub(crate) fn influence_delta_for(&self, player: usize) -> f64 {
        self.population_for(player)[0] * INFLUENCE_PER_NOBLE
    }

    pub(crate) fn population_change_for(
        &self,
        player: usize,
        food: f64,
        famine_months: u32,
    ) -> f64 {
        if self.owners.is_empty() {
            return 0.0;
        }
        let next_food = (food + self.net_production_for(player)[0]).max(0.0);
        let owned = self.owners.iter().enumerate().filter(|(_, owner)| **owner == Some(player));
        if next_food > 0.0 {
            let governance = self.governance_for(player);
            let rate = governance.population_growth_rate();
            let slave_rate = governance.slave_population_growth_rate();
            owned.map(|(index, _)| self.populations[index].monthly_growth(rate, slave_rate)).sum()
        } else {
            let next_famine_month = famine_months.saturating_add(1);
            let mortality = match next_famine_month {
                0..=3 => 0.0_f64,
                4..=8 => 2.0,
                _ => 5.0,
            };
            -mortality.min(owned.map(|(index, _)| self.populations[index].total()).sum())
        }
    }

    pub(crate) fn advance_population(&mut self, player: usize, change: f64) {
        let owned: Vec<_> = self
            .owners
            .iter()
            .enumerate()
            .filter_map(|(index, owner)| (*owner == Some(player)).then_some(index))
            .collect();
        if change > 0.0 {
            let governance = self.governance_for(player);
            let rate = governance.population_growth_rate();
            let slave_rate = governance.slave_population_growth_rate();
            for index in owned {
                self.populations[index].grow(rate, slave_rate);
            }
        } else {
            let mut remaining = -change;
            for index in owned {
                remaining -= self.populations[index].lose(remaining);
                if remaining <= 0.0 {
                    break;
                }
            }
        }
    }
}

fn starting_candidates() -> Vec<usize> {
    atlas()
        .provinces
        .iter()
        .enumerate()
        .filter_map(|(index, province)| {
            (URBAN_PROVINCES.contains(&province.name.as_str()) && province.name != ROME_PROVINCE)
                .then_some(index)
        })
        .collect()
}

fn spread_out_starts(count: usize) -> Vec<usize> {
    let candidates = starting_candidates();
    assert!((1..=4).contains(&count), "local practice supports one to four players");
    let mut selected = Vec::with_capacity(count);
    let mut chosen = Vec::new();
    let mut valid_sets = 0;
    let min_distance = if count == 2 {
        TWO_PLAYER_MIN_START_DISTANCE_KM
    } else {
        MIN_START_DISTANCE_KM
    };
    choose_start_set(
        &candidates,
        0,
        count,
        min_distance,
        &mut selected,
        &mut chosen,
        &mut valid_sets,
    );
    assert!(!chosen.is_empty(), "no viable starting provinces for {count} players");
    for index in (1..chosen.len()).rev() {
        chosen.swap(index, random_range(0..=index));
    }
    chosen
}

fn choose_start_set(
    candidates: &[usize],
    offset: usize,
    count: usize,
    min_distance: f32,
    selected: &mut Vec<usize>,
    chosen: &mut Vec<usize>,
    valid_sets: &mut usize,
) {
    if selected.len() == count {
        // Reservoir sampling gives each geographically valid set the same chance.
        *valid_sets += 1;
        if random_range(0..*valid_sets) == 0 {
            chosen.clone_from(selected);
        }
        return;
    }
    let remaining = count - selected.len();
    if candidates.len() - offset < remaining {
        return;
    }
    for next in offset..=candidates.len() - remaining {
        let candidate = candidates[next];
        if selected.iter().all(|&other| {
            province_distance_km(atlas().provinces[candidate].label, atlas().provinces[other].label)
                >= min_distance
        }) {
            selected.push(candidate);
            choose_start_set(
                candidates,
                next + 1,
                count,
                min_distance,
                selected,
                chosen,
                valid_sets,
            );
            selected.pop();
        }
    }
}

fn province_distance_km([lon_a, lat_a]: [f32; 2], [lon_b, lat_b]: [f32; 2]) -> f32 {
    let lat_a = lat_a.to_radians();
    let lat_b = lat_b.to_radians();
    let delta_lat = lat_b - lat_a;
    let delta_lon = (lon_b - lon_a).to_radians();
    let haversine = (delta_lat * 0.5).sin().powi(2)
        + lat_a.cos() * lat_b.cos() * (delta_lon * 0.5).sin().powi(2);
    12_742.0 * haversine.sqrt().asin()
}

#[derive(Resource)]
/// Camera position and hovered region on the local map.
pub(crate) struct MapView {
    zoom: f32,
    target_zoom: f32,
    zoom_anchor: egui::Pos2,
    pan: Vec2,
    hovered: Option<usize>,
    atlas_ready: bool,
    wonder_textures: Vec<egui::TextureHandle>,
    city_textures: Vec<egui::TextureHandle>,
    environment_textures: Vec<egui::TextureHandle>,
    cloud_offset: [f32; 2],
    wildlife: Vec<WildlifeSighting>,
    wildlife_spawn_timer: f32,
    wildlife_rng: u64,
    animation_clock: f32,
    water_ripples: Vec<WaterRipple>,
    label_fit: f32,
    label_candidates: Vec<Vec<Vec<LabelPlacement>>>,
    label_anchors: Vec<Option<LabelPlacement>>,
    anchor_relocated: Vec<bool>,
    anchor_candidates: Vec<Vec<LabelPlacement>>,
    anchor_level: Option<usize>,
}

impl Default for MapView {
    fn default() -> Self {
        Self {
            zoom: MIN_ZOOM,
            target_zoom: MIN_ZOOM,
            zoom_anchor: egui::Pos2::ZERO,
            pan: Vec2::ZERO,
            hovered: None,
            atlas_ready: false,
            wonder_textures: Vec::new(),
            city_textures: Vec::new(),
            environment_textures: Vec::new(),
            cloud_offset: [0.0; 2],
            wildlife: Vec::new(),
            wildlife_spawn_timer: 2.0,
            wildlife_rng: random_seed(),
            animation_clock: 0.0,
            water_ripples: Vec::new(),
            label_fit: 0.0,
            label_candidates: Vec::new(),
            label_anchors: Vec::new(),
            anchor_relocated: Vec::new(),
            anchor_candidates: Vec::new(),
            anchor_level: None,
        }
    }
}

#[derive(Deserialize)]
struct MapAtlas {
    provinces: Vec<Province>,
    land: Vec<MapMesh>,
    marker_land: Vec<MapMesh>,
    #[serde(skip)]
    adjacency: Vec<Vec<usize>>,
}

#[derive(Clone)]
struct LabelPlacement {
    center: [f32; 2],
    angle: f32,
    font_size: f32,
    full_name: bool,
}

struct WildlifeSighting {
    texture: usize,
    position: [f32; 2],
    velocity: [f32; 2],
    age: f32,
    duration: f32,
    size: f32,
    rotation: f32,
    frame_offset: f32,
}

struct WaterRipple {
    position: [f32; 2],
    length: f32,
    bend: f32,
    tilt: f32,
    double_crest: bool,
    phase: f32,
    duration: f32,
    quiet: f32,
    seed: u32,
}

#[derive(Deserialize)]
struct Province {
    name: String,
    short: String,
    label: [f32; 2],
    bounds: [f32; 4],
    parts: Vec<MapMesh>,
    #[serde(skip)]
    terrain: Vec<TerrainMesh>,
    #[serde(skip)]
    visual_center: [f32; 2],
    #[serde(skip)]
    production: [i32; 3],
}

#[derive(Deserialize)]
pub(super) struct MapMesh {
    /// Longitude and latitude pairs.
    pub(super) v: Vec<[f32; 2]>,
    /// Exclusive end index of each ring; the first ring is the exterior.
    r: Vec<usize>,
    /// Earcut triangle indices.
    pub(super) t: Vec<u32>,
    #[serde(skip)]
    pub(super) bounds: [f32; 4],
}

struct TerrainMesh {
    tile: usize,
    geometry: MapMesh,
}

struct WonderAsset {
    name: &'static str,
    position: [f32; 2],
    png: &'static [u8],
}

struct CityAsset {
    province: &'static str,
    position: [f32; 2],
    // Where the site falls within the image; coastal scenes extend inland.
    hotspot: [f32; 2],
}

// Ancient names at present-day sites. Lutetia is included for the requested Paris location.
const CITIES: [CityAsset; 8] = [
    CityAsset {
        // Rome
        province: "Latium",
        position: [12.50, 41.90],
        hotspot: [0.5, 0.5],
    },
    CityAsset {
        // Lutetia (present-day Paris)
        province: "Lugdunensis",
        position: [2.35, 48.86],
        hotspot: [0.5, 0.5],
    },
    CityAsset {
        // Tarraco
        province: "Tarraconensis",
        position: [1.24, 41.12],
        hotspot: [0.5, 0.85],
    },
    CityAsset {
        // Carthage
        province: "Africa Proconsularis",
        position: [10.33, 36.85],
        hotspot: [0.5, 0.15],
    },
    CityAsset {
        // Athens
        province: "Achaia",
        position: [23.73, 37.98],
        hotspot: [0.8, 0.9],
    },
    CityAsset {
        // Alexandria
        province: "Aegyptus",
        position: [29.92, 31.20],
        hotspot: [0.5, 0.15],
    },
    CityAsset {
        // Ephesus
        province: "Asia",
        position: [27.36, 37.95],
        hotspot: [0.2, 0.5],
    },
    CityAsset {
        // Antioch
        province: "Syria",
        position: [36.16, 36.20],
        hotspot: [0.3, 0.5],
    },
];

const CITY_IMAGES: [(&str, &[u8]); 2] = [
    ("City illustration", include_bytes!("../../assets/images/cities/city.png")),
    ("Rome illustration", include_bytes!("../../assets/images/cities/rome.png")),
];

const ENVIRONMENT_IMAGES: [(&str, &[u8]); 14] = [
    ("Sea waves", include_bytes!("../../assets/images/map/waves.png")),
    ("Moving clouds", include_bytes!("../../assets/images/map/clouds.png")),
    ("Coastal gradient", include_bytes!("../../assets/images/map/coastal-gradient.png")),
    ("Sea ripple texture", include_bytes!("../../assets/images/map/ripples.png")),
    ("Dolphin animation", include_bytes!("../../assets/images/map/dolphin-jump-sheet.png")),
    ("Whale animation", include_bytes!("../../assets/images/map/whale-swim-sheet.png")),
    ("Seabird animation", include_bytes!("../../assets/images/map/seabirds-flight-sheet.png")),
    ("Northwest terrain", include_bytes!("../../assets/images/map/terrain-landcover-nw.png")),
    ("Northeast terrain", include_bytes!("../../assets/images/map/terrain-landcover-ne.png")),
    ("Southwest terrain", include_bytes!("../../assets/images/map/terrain-landcover-sw.png")),
    ("Southeast terrain", include_bytes!("../../assets/images/map/terrain-landcover-se.png")),
    ("Sea vortex animation", include_bytes!("../../assets/images/map/sea-vortex-sheet.png")),
    ("Eagle animation", include_bytes!("../../assets/images/map/eagle-flight-sheet.png")),
    ("Tern animation", include_bytes!("../../assets/images/map/tern-flight-sheet.png")),
];
const WAVE_TEXTURE: usize = 0;
const CLOUD_TEXTURE: usize = 1;
const COAST_TEXTURE: usize = 2;
const RIPPLE_TEXTURE: usize = 3;
// Stylized from the Natural Earth 1:50m northern and southern basins. The
// narrow connection and wider shores keep the full sea legible at map zoom.
// Each row is [latitude, western shore, eastern shore], north to south.
const DEAD_SEA_ROWS: &[[f32; 3]] = &[
    [31.80, 35.53, 35.56],
    [31.77, 35.48, 35.59],
    [31.73, 35.44, 35.61],
    [31.67, 35.40, 35.60],
    [31.60, 35.38, 35.59],
    [31.53, 35.36, 35.61],
    [31.46, 35.37, 35.60],
    [31.39, 35.38, 35.59],
    [31.33, 35.39, 35.57],
    [31.28, 35.41, 35.55],
    [31.24, 35.43, 35.52],
    [31.21, 35.42, 35.53],
    [31.17, 35.39, 35.56],
    [31.13, 35.38, 35.55],
    [31.09, 35.40, 35.54],
    [31.05, 35.43, 35.51],
    [31.01, 35.46, 35.49],
];
const DEAD_SEA_BOUNDS: [f32; 4] = [35.36, 31.01, 35.61, 31.80];
const DOLPHIN_TEXTURE: usize = 4;
const WHALE_TEXTURE: usize = 5;
const SEABIRDS_TEXTURE: usize = 6;
const TERRAIN_TEXTURE_START: usize = 7;
const VORTEX_TEXTURE: usize = TERRAIN_TEXTURE_START + TERRAIN_TILE_BOUNDS.len();
const EAGLE_TEXTURE: usize = VORTEX_TEXTURE + 1;
const TERN_TEXTURE: usize = EAGLE_TEXTURE + 1;
// Four 1920×1110 crops plus a one-pixel sampling gutter fit egui's default
// 2048px texture limit. Bounds are west, south, east, north in the source
// equirectangular projection; neighboring gutters avoid filtering seams.
const TERRAIN_TILE_BOUNDS: [[f32; 4]; 4] = [
    [-12.0, 40.5, 20.0, 59.0],
    [20.0, 40.5, 52.0, 59.0],
    [-12.0, 22.0, 20.0, 40.5],
    [20.0, 22.0, 52.0, 40.5],
];

const WONDERS: [WonderAsset; 11] = [
    WonderAsset {
        name: "Great Pyramid of Giza",
        position: [31.13, 29.98],
        png: include_bytes!("../../assets/images/wonders/great_pyramid.png"),
    },
    WonderAsset {
        name: "Oracle of Dodona",
        position: [20.78, 39.55],
        png: include_bytes!("../../assets/images/wonders/oracle_dodona.png"),
    },
    WonderAsset {
        name: "Stonehenge",
        position: [-1.83, 51.18],
        png: include_bytes!("../../assets/images/wonders/stonehenge.png"),
    },
    WonderAsset {
        name: "Acropolis of Pergamon",
        position: [27.18, 39.13],
        png: include_bytes!("../../assets/images/wonders/pergamon_acropolis.png"),
    },
    WonderAsset {
        name: "Temple of Zeus at Olympia",
        position: [21.63, 37.64],
        png: include_bytes!("../../assets/images/wonders/zeus_temple.png"),
    },
    WonderAsset {
        name: "Palace of the Argeads",
        position: [21.70, 40.44],
        png: include_bytes!("../../assets/images/wonders/argeads_palace.png"),
    },
    WonderAsset {
        name: "Ay Khanum",
        position: [69.42, 37.17],
        png: include_bytes!("../../assets/images/wonders/ay_khanum.png"),
    },
    WonderAsset {
        name: "Mausoleum at Halicarnassus",
        position: [27.42, 37.04],
        png: include_bytes!("../../assets/images/wonders/mausoleum_halicar.png"),
    },
    WonderAsset {
        name: "Colossus of Rhodes",
        // The coast atlas ends just short of Rhodes's ancient harbour. Keep
        // this marker on the island's northern end at every zoom level.
        position: [28.12, 36.34],
        png: include_bytes!("../../assets/images/wonders/rhodes_colossus.png"),
    },
    WonderAsset {
        name: "Aqueduct of Segovia",
        position: [-4.117, 40.948],
        png: include_bytes!("../../assets/images/wonders/segovia_aqueduct.png"),
    },
    WonderAsset {
        name: "Pont du Gard",
        position: [4.535, 43.948],
        png: include_bytes!("../../assets/images/wonders/pont_du_gard.png"),
    },
];

pub(crate) struct MapLoadProgress {
    pub completed: usize,
    pub total: usize,
    pub item: String,
}

impl MapView {
    pub(crate) fn is_loaded(&self) -> bool {
        self.atlas_ready
            && self.wonder_textures.len() == WONDERS.len()
            && self.city_textures.len() == CITY_IMAGES.len()
            && self.environment_textures.len() == ENVIRONMENT_IMAGES.len()
            && self.label_candidates.len() == LABEL_ZOOM_LEVELS
    }

    pub(crate) fn load_progress(&self) -> MapLoadProgress {
        let completed = usize::from(self.atlas_ready)
            + self.wonder_textures.len()
            + self.city_textures.len()
            + self.environment_textures.len()
            + self.label_candidates.len();
        let item = if !self.atlas_ready {
            "Loading map...".to_string()
        } else if let Some(wonder) = WONDERS.get(self.wonder_textures.len()) {
            format!("Loading wonder {}...", wonder.name)
        } else if let Some((name, _)) = CITY_IMAGES.get(self.city_textures.len()) {
            format!("Loading city {}...", name)
        } else if let Some((name, _)) = ENVIRONMENT_IMAGES.get(self.environment_textures.len()) {
            format!("Loading environment {}...", name)
        } else if self.label_candidates.len() < LABEL_ZOOM_LEVELS {
            "Loading historical provinces...".to_string()
        } else {
            "Game ready.".to_string()
        };
        MapLoadProgress {
            completed,
            total: WONDERS.len()
                + CITY_IMAGES.len()
                + ENVIRONMENT_IMAGES.len()
                + LABEL_ZOOM_LEVELS
                + 1,
            item,
        }
    }

    pub(crate) fn load_next(&mut self, ctx: &egui::Context) {
        if !self.atlas_ready {
            atlas();
            self.atlas_ready = true;
            return;
        }
        if let Some(wonder) = WONDERS.get(self.wonder_textures.len()) {
            self.wonder_textures.push(load_map_texture(ctx, wonder.name, wonder.png));
        } else if let Some((name, png)) = CITY_IMAGES.get(self.city_textures.len()) {
            let resolution = if self.city_textures.is_empty() {
                128
            } else {
                256
            };
            self.city_textures.push(load_city_texture(ctx, name, png, resolution));
        } else if let Some((name, png)) = ENVIRONMENT_IMAGES.get(self.environment_textures.len()) {
            self.environment_textures.push(load_map_texture(ctx, name, png));
        } else if self.label_candidates.len() < LABEL_ZOOM_LEVELS {
            let atlas = atlas();
            if self.label_candidates.is_empty() {
                self.label_fit = map_geometry(ctx.content_rect(), atlas).3;
            }
            let zoom = label_zoom(self.label_candidates.len());
            let projection = Projection {
                origin: egui::Pos2::ZERO,
                scale: self.label_fit * zoom,
                center: [0.0, 0.0],
            };
            let painter = ctx.layer_painter(egui::LayerId::background());
            self.label_candidates.push(
                atlas
                    .provinces
                    .iter()
                    .map(|province| label_candidates(&painter, province, &projection, zoom))
                    .collect(),
            );
        }
    }
}

fn load_map_texture(ctx: &egui::Context, name: &str, png: &[u8]) -> egui::TextureHandle {
    let rgba = image::load_from_memory(png).expect("map PNG assets must be valid").to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    let options = if matches!(name, "Sea waves" | "Sea ripple texture") {
        egui::TextureOptions::LINEAR_REPEAT
    } else {
        egui::TextureOptions::LINEAR
    };
    ctx.load_texture(name, color, options)
}

fn load_city_texture(
    ctx: &egui::Context,
    name: &str,
    png: &[u8],
    resolution: u32,
) -> egui::TextureHandle {
    let mut rgba = image::load_from_memory(png).expect("city PNG assets must be valid").to_rgba8();
    // Prefilter the detailed source for small map sizes. Premultiplied alpha
    // keeps the cutout edges clean against both land and sea colors.
    for pixel in rgba.pixels_mut() {
        let alpha = u16::from(pixel[3]);
        for channel in &mut pixel.0[..3] {
            *channel = ((u16::from(*channel) * alpha + 127) / 255) as u8;
        }
    }
    let mut reduced = image::imageops::resize(
        &rgba,
        resolution,
        resolution,
        image::imageops::FilterType::Lanczos3,
    );
    for pixel in reduced.pixels_mut() {
        let alpha = u32::from(pixel[3]);
        for channel in &mut pixel.0[..3] {
            if let Some(value) = (u32::from(*channel) * 255 + alpha / 2).checked_div(alpha) {
                *channel = value.min(255) as u8;
            }
        }
    }
    let size = [resolution as usize, resolution as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, reduced.as_raw());
    ctx.load_texture(name, color, egui::TextureOptions::LINEAR)
}

fn atlas() -> &'static MapAtlas {
    static ATLAS: OnceLock<MapAtlas> = OnceLock::new();
    ATLAS.get_or_init(|| {
        let mut atlas: MapAtlas = serde_json::from_str(include_str!("../../assets/map/atlas.json"))
            .expect("generated map atlas must be valid");
        for part in &mut atlas.land {
            part.cache_bounds();
        }
        for province in &mut atlas.provinces {
            for part in &mut province.parts {
                part.cache_bounds();
            }
            province.terrain = build_terrain_tiles(&province.parts);
            province.visual_center = province_center(province);
            province.production = for_province(&province.name);
        }
        atlas.adjacency = province_adjacency(&atlas.provinces);
        atlas
    })
}

fn province_adjacency(provinces: &[Province]) -> Vec<Vec<usize>> {
    type Point = [u32; 2];
    let mut edges: HashMap<(Point, Point), usize> = HashMap::new();
    let mut adjacency = vec![Vec::new(); provinces.len()];
    for (index, province) in provinces.iter().enumerate() {
        for part in &province.parts {
            let mut start = 0;
            for &end in &part.r {
                let ring = &part.v[start..end];
                for edge in
                    (0..ring.len()).map(|vertex| (ring[vertex], ring[(vertex + 1) % ring.len()]))
                {
                    let a = edge.0.map(f32::to_bits);
                    let b = edge.1.map(f32::to_bits);
                    let key = if a < b {
                        (a, b)
                    } else {
                        (b, a)
                    };
                    if let Some(&other) = edges.get(&key) {
                        if other != index {
                            adjacency[index].push(other);
                            adjacency[other].push(index);
                        }
                    } else {
                        edges.insert(key, index);
                    }
                }
                start = end;
            }
        }
    }
    for neighbors in &mut adjacency {
        neighbors.sort_unstable();
        neighbors.dedup();
    }
    adjacency
}

fn province_center(province: &Province) -> [f32; 2] {
    let mut largest = (0.0_f64, province.label);
    for part in &province.parts {
        let mut area = 0.0_f64;
        let mut weighted = [0.0_f64; 2];
        for triangle in part.t.chunks_exact(3) {
            let a = part.v[triangle[0] as usize];
            let b = part.v[triangle[1] as usize];
            let c = part.v[triangle[2] as usize];
            let size = (((b[0] - a[0]) as f64 * (c[1] - a[1]) as f64)
                - ((c[0] - a[0]) as f64 * (b[1] - a[1]) as f64))
                .abs();
            area += size;
            weighted[0] += size * (a[0] as f64 + b[0] as f64 + c[0] as f64) / 3.0;
            weighted[1] += size * (a[1] as f64 + b[1] as f64 + c[1] as f64) / 3.0;
        }
        if area > largest.0 {
            largest = (area, [(weighted[0] / area) as f32, (weighted[1] / area) as f32]);
        }
    }
    largest.1
}

struct Projection {
    origin: egui::Pos2,
    scale: f32,
    center: [f32; 2],
}

impl Projection {
    fn point(&self, [lon, lat]: [f32; 2]) -> egui::Pos2 {
        self.origin
            + egui::vec2(
                (lon - self.center[0]) * LONGITUDE_SCALE * self.scale,
                (self.center[1] - lat) * self.scale,
            )
    }

    fn inverse(&self, point: egui::Pos2) -> [f32; 2] {
        [
            self.center[0] + (point.x - self.origin.x) / (LONGITUDE_SCALE * self.scale),
            self.center[1] - (point.y - self.origin.y) / self.scale,
        ]
    }

    fn bounds_rect(&self, [west, south, east, north]: [f32; 4]) -> egui::Rect {
        egui::Rect::from_min_max(self.point([west, north]), self.point([east, south]))
    }
}

fn settle_position(position: Vec2, target: Vec2, dt: f32) -> Vec2 {
    if position.distance_squared(target) < 0.01 {
        target
    } else {
        let fraction = 1.0 - (-BOUNDS_RETURN_RATE * dt.max(0.0)).exp();
        position.lerp(target, fraction.clamp(0.0, 1.0))
    }
}

fn pan_margin_screen_fraction(zoom: f32) -> f32 {
    let close = ((zoom - MIN_ZOOM) / (PAN_MARGIN_FULL_ZOOM - MIN_ZOOM)).clamp(0.0, 1.0);
    PAN_MARGIN_SCREEN_FRACTION + (0.5 - PAN_MARGIN_SCREEN_FRACTION) * close
}

fn pan_limit(map_span: f32, viewport_span: f32, zoom: f32) -> f32 {
    ((map_span - viewport_span) * 0.5).max(0.0) + viewport_span * pan_margin_screen_fraction(zoom)
}

fn map_geometry(rect: egui::Rect, atlas: &MapAtlas) -> ([f32; 2], f32, f32, f32) {
    let bounds = atlas.provinces.iter().fold(
        [f32::MAX, f32::MAX, f32::MIN, f32::MIN],
        |mut all, province| {
            all[0] = all[0].min(province.bounds[0]);
            all[1] = all[1].min(province.bounds[1]);
            all[2] = all[2].max(province.bounds[2]);
            all[3] = all[3].max(province.bounds[3]);
            all
        },
    );
    let center = [(bounds[0] + bounds[2]) * 0.5, (bounds[1] + bounds[3]) * 0.5];
    let width = (bounds[2] - bounds[0]) * LONGITUDE_SCALE;
    let height = bounds[3] - bounds[1];
    let fit = (rect.width() / (width + 8.0)).min(rect.height() / (height + 5.0));
    (center, width, height, fit)
}

fn label_zoom(level: usize) -> f32 {
    (MIN_ZOOM + level as f32 * LABEL_ZOOM_STEP).min(MAX_ZOOM)
}

fn label_level(zoom: f32, fit: f32, prepared_fit: f32) -> usize {
    let effective_zoom = zoom.min(zoom * fit / prepared_fit);
    if effective_zoom >= MAX_ZOOM {
        return LABEL_ZOOM_LEVELS - 1;
    }
    (((effective_zoom - MIN_ZOOM) / LABEL_ZOOM_STEP).floor().max(0.0) as usize)
        .min(LABEL_ZOOM_LEVELS - 1)
}

fn stable_label_anchors(
    prepared: &[Vec<Vec<LabelPlacement>>],
    province_count: usize,
) -> Vec<Option<LabelPlacement>> {
    (0..province_count)
        .map(|index| prepared.iter().rev().find_map(|level| level[index].first().cloned()))
        .collect()
}

/// The map fills the screen. Escape opens the in-game menu through the app's shared handler.
pub(crate) fn draw_map(
    mut contexts: EguiContexts,
    mut view: ResMut<MapView>,
    ownership: Res<ProvinceOwnership>,
    mut governance_open: ResMut<GovernancePanelOpen>,
    mut province_open: ResMut<ProvincePanelOpen>,
    panel_close_click: Res<MapPanelCloseClick>,
    mut production_icons: Local<Option<[egui::TextureHandle; 3]>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    state: Res<State<crate::app::AppState>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    while !view.is_loaded() {
        view.load_next(ctx);
    }
    let icons = production_icons.get_or_insert_with(|| {
        std::array::from_fn(|index| {
            let image = image::load_from_memory(PRODUCTION_ICONS[index])
                .expect("production icon PNG must be valid")
                .resize_exact(64, 64, image::imageops::FilterType::Lanczos3)
                .to_rgba8();
            ctx.load_texture(
                format!("province-production-{index}"),
                egui::ColorImage::from_rgba_unmultiplied(
                    [image.width() as usize, image.height() as usize],
                    image.as_raw(),
                ),
                egui::TextureOptions::LINEAR,
            )
        })
    });
    let rect = ctx.content_rect();
    let interactions_enabled = *state.get() == crate::app::AppState::Map;
    egui::Area::new(egui::Id::new("augustus_province_map"))
        .order(egui::Order::Background)
        .fixed_pos(rect.min)
        .show(ctx, |ui| {
            ui.set_min_size(rect.size());
            ui.set_max_size(rect.size());
            let sense = if interactions_enabled {
                egui::Sense::click_and_drag()
            } else {
                egui::Sense::hover()
            };
            let (map_rect, response) = ui.allocate_exact_size(rect.size(), sense);
            let painter = ui.painter_at(map_rect);
            let clicked_detail = paint_map(
                &painter,
                map_rect,
                &mut view,
                &ownership,
                icons,
                &response,
                &keyboard,
                time.delta_secs(),
                interactions_enabled,
            );
            if let Some(detail) = clicked_detail.filter(|_| !panel_close_click.0) {
                province_open.0 = Some(detail);
                governance_open.0 = false;
            }
            response
        });
    ctx.request_repaint_after(std::time::Duration::from_millis(33));
}

fn paint_map(
    painter: &egui::Painter,
    rect: egui::Rect,
    view: &mut MapView,
    ownership: &ProvinceOwnership,
    production_icons: &[egui::TextureHandle; 3],
    response: &egui::Response,
    keyboard: &ButtonInput<KeyCode>,
    dt: f32,
    interactions_enabled: bool,
) -> Option<MapDetail> {
    let atlas = atlas();
    let (center, width, height, fit) = map_geometry(rect, atlas);

    let (pointer_over_menu, drag_started_on_menu) = painter.ctx().input(|input| {
        (
            input.pointer.hover_pos().is_some_and(|point| map_hud_contains(rect, point)),
            input.pointer.press_origin().is_some_and(|point| map_hud_contains(rect, point)),
        )
    });

    if interactions_enabled && !pointer_over_menu {
        if let Some(cursor) = response.hover_pos() {
            let scroll = painter.ctx().input(|input| input.smooth_scroll_delta.y);
            if scroll.abs() > 0.0 {
                view.target_zoom =
                    (view.target_zoom * (scroll * 0.0022).exp()).clamp(MIN_ZOOM, MAX_ZOOM);
                view.zoom_anchor = cursor;
            }
        }
    }
    if (view.target_zoom - view.zoom).abs() > 0.0001 {
        painter.ctx().request_repaint();
        let fraction = 1.0 - (-ZOOM_RESPONSE_RATE * dt.max(0.0)).exp();
        let next_zoom = view.zoom + (view.target_zoom - view.zoom) * fraction.clamp(0.0, 1.0);
        let next_zoom = if (view.target_zoom - next_zoom).abs() < 0.001 {
            view.target_zoom
        } else {
            next_zoom
        };
        let old_origin = rect.center() + egui::vec2(view.pan.x, view.pan.y);
        let new_origin =
            view.zoom_anchor - (view.zoom_anchor - old_origin) * (next_zoom / view.zoom);
        let pan = new_origin - rect.center();
        view.pan = Vec2::new(pan.x, pan.y);
        view.zoom = next_zoom;
    }
    let dragging = interactions_enabled
        && !drag_started_on_menu
        && response.dragged_by(egui::PointerButton::Primary);
    let grabbing = dragging
        || (interactions_enabled
            && !drag_started_on_menu
            && response.hovered()
            && painter.ctx().input(|input| input.pointer.primary_down()));
    if dragging {
        let delta = painter.ctx().input(|input| input.pointer.delta());
        view.pan += Vec2::new(delta.x, delta.y);
    }
    let speed = 600.0 * dt.clamp(0.0, 0.1);
    if interactions_enabled && keyboard.pressed(KeyCode::KeyA) {
        view.pan.x += speed;
    }
    if interactions_enabled && keyboard.pressed(KeyCode::KeyD) {
        view.pan.x -= speed;
    }
    if interactions_enabled && keyboard.pressed(KeyCode::KeyW) {
        view.pan.y += speed;
    }
    if interactions_enabled && keyboard.pressed(KeyCode::KeyS) {
        view.pan.y -= speed;
    }
    let scale = fit * view.zoom;
    let max_pan = Vec2::new(
        pan_limit(width * scale, rect.width(), view.zoom),
        pan_limit(height * scale, rect.height(), view.zoom),
    );
    let target_pan = view.pan.clamp(-max_pan, max_pan);
    let overscroll = Vec2::new(rect.width(), rect.height()) * PAN_OVERSCROLL_SCREEN_FRACTION;
    let limited = target_pan + (view.pan - target_pan).clamp(-overscroll, overscroll);
    view.pan = if !interactions_enabled {
        view.pan
    } else if grabbing {
        limited
    } else {
        settle_position(limited, target_pan, dt)
    };
    let projection = Projection {
        origin: rect.center() + egui::vec2(view.pan.x, view.pan.y),
        scale,
        center,
    };
    view.animation_clock += dt.clamp(0.0, 0.1);
    if view.zoom >= 2.6 {
        if view.water_ripples.is_empty() {
            view.water_ripples = generate_water_ripples(&atlas.land, &mut view.wildlife_rng);
        }
        update_wildlife(view, &atlas.land, &projection, rect, dt);
    } else {
        view.wildlife.clear();
        view.wildlife_spawn_timer = 2.0;
    }
    advance_texture_offset(&mut view.cloud_offset, [4.5, 0.7], [84.0, 32.0], scale, dt);
    let city_markers = layout_cities(&projection, rect, view.zoom);
    let wonder_markers = layout_wonders(&projection, rect, view.zoom);

    let close = smoothstep((view.zoom - 1.0) / 2.5);
    let sea_color = blend_color(DEEP_SEA, CLOSE_SEA, close);
    painter.rect_filled(rect, 0.0, sea_color);
    if let Some(coast) = view.environment_textures.get(COAST_TEXTURE) {
        let gradient_rect = egui::Rect::from_min_max(
            projection.point([-18.0, 61.0]),
            projection.point([52.0, 17.0]),
        );
        painter.image(
            coast.id(),
            gradient_rect,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            egui::Color32::from_white_alpha((100.0 + close * 155.0) as u8),
        );
    }
    let northwest = projection.inverse(rect.min);
    let southeast = projection.inverse(rect.max);
    paint_water(
        painter,
        rect,
        |point| projection.point(point),
        [northwest[0], southeast[1], southeast[0], northwest[1]],
        view.environment_textures.get(WAVE_TEXTURE),
        view.environment_textures.get(RIPPLE_TEXTURE),
        view.animation_clock,
        view.zoom,
    );
    if view.zoom >= 2.6 {
        paint_water_ripples(
            painter,
            &view.water_ripples,
            &projection,
            view.animation_clock,
            smoothstep((view.zoom - 2.6) / 1.2),
        );
    }
    paint_meshes(painter, &atlas.land, &projection, LAND);

    let pointer = if interactions_enabled && !pointer_over_menu {
        response.hover_pos().map(|position| projection.inverse(position))
    } else {
        None
    };
    view.hovered = pointer
        .and_then(|point| atlas.provinces.iter().position(|province| province.contains(point)));
    // Foreground controls choose their own cursor. The HUD bounds also cover
    // controls that may be drawn after the map in this pass.
    let foreground_control = painter
        .ctx()
        .input(|input| input.pointer.hover_pos())
        .and_then(|point| painter.ctx().layer_id_at(point))
        .is_some_and(|layer| layer.order > egui::Order::Background);
    let clicked_detail = (interactions_enabled
        && !pointer_over_menu
        && !drag_started_on_menu
        && !foreground_control
        && response.clicked_by(egui::PointerButton::Primary))
    .then(|| response.interact_pointer_pos())
    .flatten()
    .and_then(|position| {
        city_markers
            .iter()
            .rev()
            .find(|marker| marker.bounds.contains(position))
            .and_then(|marker| {
                atlas
                    .provinces
                    .iter()
                    .position(|province| province.name == CITIES[marker.city_index].province)
            })
            .map(MapDetail::City)
            .or_else(|| {
                let point = projection.inverse(position);
                atlas
                    .provinces
                    .iter()
                    .position(|province| province.contains(point))
                    .map(MapDetail::Province)
            })
    });
    if interactions_enabled && !pointer_over_menu && !foreground_control {
        painter.ctx().set_cursor_icon(if dragging {
            egui::CursorIcon::Grabbing
        } else if view.hovered.is_some()
            || response.hover_pos().is_some_and(|position| {
                city_markers.iter().any(|marker| marker.bounds.contains(position))
            })
        {
            egui::CursorIcon::PointingHand
        } else {
            egui::CursorIcon::Default
        });
    }
    for (index, province) in atlas.provinces.iter().enumerate() {
        if !projection.bounds_rect(province.bounds).intersects(rect) {
            continue;
        }
        let hovered = view.hovered == Some(index);
        let owned_color = ownership.map_color(index);
        if view.environment_textures.len() >= TERRAIN_TEXTURE_START + TERRAIN_TILE_BOUNDS.len() {
            for tile in &province.terrain {
                let texture = &view.environment_textures[TERRAIN_TEXTURE_START + tile.tile];
                let mesh = province_terrain_mesh(tile, &projection, rect, texture.id());
                if !mesh.indices.is_empty() {
                    painter.add(egui::Shape::mesh(mesh));
                }
            }
            if let Some(color) = owned_color {
                paint_meshes(
                    painter,
                    &province.parts,
                    &projection,
                    egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 168),
                );
            }
            if hovered {
                paint_meshes(
                    painter,
                    &province.parts,
                    &projection,
                    egui::Color32::from_white_alpha(48),
                );
            }
        } else {
            let color = match (owned_color, hovered) {
                (Some(color), true) => blend_color(color, egui::Color32::WHITE, 0.28),
                (Some(color), false) => color,
                (None, true) => blend_color(province_color(index), egui::Color32::WHITE, 0.28),
                (None, false) => province_color(index),
            };
            paint_meshes(painter, &province.parts, &projection, color);
        }
    }

    for (index, province) in atlas.provinces.iter().enumerate() {
        if !projection.bounds_rect(province.bounds).intersects(rect) {
            continue;
        }
        let hovered = view.hovered == Some(index);
        for part in &province.parts {
            paint_rings(
                painter,
                part,
                &projection,
                egui::Stroke::new(
                    if hovered {
                        1.3
                    } else {
                        0.55
                    },
                    BORDER,
                ),
            );
        }
    }

    paint_dead_sea(painter, view, &projection, sea_color);
    paint_rivers(painter, |point| projection.point(point), view.zoom, sea_color);

    paint_wildlife(painter, &view.wildlife, &view.environment_textures, &projection);

    if let Some(clouds) = view.environment_textures.get(CLOUD_TEXTURE) {
        let opacity = (45.0 + close * 63.0) as u8;
        paint_scrolling_texture(
            painter,
            rect,
            &projection,
            clouds,
            [84.0, 32.0],
            view.cloud_offset,
            egui::Color32::from_white_alpha(opacity),
        );
    }

    // Draw ownership boundaries above clouds and every neutral border. The
    // dark casing keeps all player colors legible over both grass and desert.
    for (index, province) in atlas.provinces.iter().enumerate() {
        let Some(color) = ownership.map_color(index) else {
            continue;
        };
        if !projection.bounds_rect(province.bounds).intersects(rect) {
            continue;
        }
        let width = if view.hovered == Some(index) {
            2.0
        } else {
            1.4
        };
        for part in &province.parts {
            paint_rings(
                painter,
                part,
                &projection,
                egui::Stroke::new(width + 0.9, egui::Color32::from_rgb(47, 39, 31)),
            );
            paint_rings(painter, part, &projection, egui::Stroke::new(width, color));
        }
    }

    paint_cities(painter, &city_markers, view.zoom, &view.city_textures);
    paint_wonders(painter, &wonder_markers, view.zoom, &view.wonder_textures);
    let mut occupied =
        Vec::with_capacity(wonder_markers.len() + city_markers.len() + atlas.provinces.len());
    occupied.extend(wonder_markers.iter().map(|marker| marker.bounds(view.zoom).expand(1.0)));
    occupied.extend(city_markers.iter().map(|marker| marker.bounds));
    let marker_count = occupied.len();

    // Choose one geographic position and angle from the detailed map, then
    // refit only the text at each zoom level. A marker may force one lasting
    // relocation; ordinary zoom and label collisions never rotate the name.
    let label_level = label_level(view.zoom, fit, view.label_fit);
    if view.label_anchors.len() != atlas.provinces.len() {
        view.label_anchors = stable_label_anchors(&view.label_candidates, atlas.provinces.len());
        view.anchor_relocated = vec![false; atlas.provinces.len()];
        view.anchor_level = None;
    }
    if view.anchor_level != Some(label_level) {
        let prepared_projection = Projection {
            origin: egui::Pos2::ZERO,
            scale: view.label_fit * label_zoom(label_level),
            center: [0.0, 0.0],
        };
        view.anchor_candidates = atlas
            .provinces
            .iter()
            .zip(&view.label_anchors)
            .map(|(province, anchor)| {
                anchor.as_ref().map_or_else(Vec::new, |anchor| {
                    anchored_label_candidates(
                        painter,
                        province,
                        &prepared_projection,
                        label_zoom(label_level),
                        anchor,
                    )
                })
            })
            .collect();
        view.anchor_level = Some(label_level);
    }
    let mut visible_labels = vec![None; atlas.provinces.len()];
    for index in
        (0..atlas.provinces.len()).filter(|&index| view.hovered != Some(index)).chain(view.hovered)
    {
        let province = &atlas.provinces[index];
        let anchored = &view.anchor_candidates[index];
        let regular = &view.label_candidates[label_level][index];
        let has_anchor = view.label_anchors[index].is_some();
        if has_anchor && anchored.is_empty() {
            continue;
        }
        if let Some(anchor) = anchored.first() {
            let name = if anchor.full_name {
                &province.name
            } else {
                &province.short
            };
            let galley = painter.layout_no_wrap(
                name.clone(),
                egui::FontId::proportional(anchor.font_size),
                INK,
            );
            let bounds =
                rotated_bounds(projection.point(anchor.center), galley.size(), anchor.angle)
                    .expand(2.0);
            if !rect.contains_rect(bounds) {
                continue;
            }
        }
        let allow_relocation = has_anchor
            && !view.anchor_relocated[index]
            && marker_blocks_readable_anchor(
                painter,
                province,
                &projection,
                anchored,
                regular,
                &occupied[..marker_count],
            );
        let mut selected = None;
        for (candidate, relocated) in
            label_choice_order(anchored, regular, has_anchor, allow_relocation)
        {
            let name = if candidate.full_name {
                &province.name
            } else {
                &province.short
            };
            let galley = painter.layout_no_wrap(
                name.clone(),
                egui::FontId::proportional(candidate.font_size),
                INK,
            );
            let center = projection.point(candidate.center);
            let label_rect = rotated_bounds(center, galley.size(), candidate.angle).expand(2.0);
            if occupied.iter().any(|other| other.intersects(label_rect)) {
                continue;
            }
            // The viewport may hide a label, but must not make it relocate.
            if !rect.contains_rect(label_rect) {
                break;
            }
            selected = Some((candidate.clone(), galley, center, label_rect, relocated));
            break;
        }
        if let Some((candidate, galley, center, label_rect, relocated)) = selected {
            let origin = center - galley.size() * 0.5;
            let shape = egui::epaint::TextShape::new(origin, galley, INK)
                .with_angle_and_anchor(candidate.angle, egui::Align2::CENTER_CENTER);
            painter.add(egui::Shape::Text(shape));
            occupied.push(label_rect);
            visible_labels[index] = Some((label_rect, candidate.font_size));
            if !has_anchor || relocated {
                view.label_anchors[index] = Some(candidate);
                view.anchor_relocated[index] = relocated;
                view.anchor_level = None;
            }
        }
    }
    if view.zoom >= 2.8 {
        for (index, province) in atlas.provinces.iter().enumerate() {
            if !projection.bounds_rect(province.bounds).intersects(rect) {
                continue;
            }
            let Some((label_rect, font_size)) = visible_labels[index] else {
                continue;
            };
            let entries: Vec<_> = province
                .production
                .iter()
                .copied()
                .enumerate()
                .filter(|(_, amount)| *amount != 0)
                .collect();
            if entries.is_empty() {
                continue;
            }
            let scale = font_size / 12.0;
            let text_width: f32 = entries
                .iter()
                .map(|(_, amount)| {
                    painter
                        .layout_no_wrap(
                            amount.to_string(),
                            egui::FontId::proportional(font_size),
                            INK,
                        )
                        .size()
                        .x
                })
                .sum();
            let width = 8.0 * scale
                + text_width
                + entries.len() as f32 * 14.0 * scale
                + entries.len().saturating_sub(1) as f32 * 5.0 * scale;
            let size = egui::vec2(width, 20.0 * scale);
            let badge = resource_badge_below(label_rect, size, rect).filter(|candidate| {
                !occupied.iter().any(|other| other.intersects(*candidate))
                    && !map_hud_contains(rect, candidate.center())
            });
            let Some(badge) = badge else {
                continue;
            };
            painter.rect(
                badge,
                4.0 * scale,
                egui::Color32::from_rgba_unmultiplied(244, 232, 206, 235),
                egui::Stroke::new(0.7 * scale, BORDER),
                egui::StrokeKind::Inside,
            );
            let mut x = badge.left() + 4.0 * scale;
            for (resource, amount) in entries {
                let icon_size = 12.0 * scale;
                painter.image(
                    production_icons[resource].id(),
                    egui::Rect::from_min_size(
                        egui::pos2(x, badge.center().y - icon_size * 0.5),
                        egui::vec2(icon_size, icon_size),
                    ),
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                x += 14.0 * scale;
                let text = amount.to_string();
                let galley =
                    painter.layout_no_wrap(text, egui::FontId::proportional(font_size), INK);
                let text_size = galley.size();
                painter.galley(egui::pos2(x, badge.center().y - text_size.y * 0.5), galley, INK);
                x += text_size.x + 5.0 * scale;
            }
            occupied.push(badge);
        }
    }
    clicked_detail
}

fn resource_badge_below(
    label: egui::Rect,
    size: egui::Vec2,
    viewport: egui::Rect,
) -> Option<egui::Rect> {
    let center = egui::pos2(label.center().x, label.bottom() + 3.0 + size.y * 0.5);
    let badge = egui::Rect::from_center_size(center, size);
    viewport.contains_rect(badge).then_some(badge)
}

fn rotated_bounds(center: egui::Pos2, size: egui::Vec2, angle: f32) -> egui::Rect {
    let (sin, cos) = angle.sin_cos();
    let half = egui::vec2(
        (size.x * cos.abs() + size.y * sin.abs()) * 0.5,
        (size.x * sin.abs() + size.y * cos.abs()) * 0.5,
    );
    egui::Rect::from_min_max(center - half, center + half)
}

fn label_candidates(
    painter: &egui::Painter,
    province: &Province,
    projection: &Projection,
    zoom: f32,
) -> Vec<LabelPlacement> {
    let mut centers = Vec::new();
    if province.contains(province.visual_center) {
        centers.push(province.visual_center);
    }
    if province.contains(province.label) && province.label != province.visual_center {
        centers.push(province.label);
    }
    for row in 0..5 {
        for column in 0..5 {
            let center = [
                province.bounds[0]
                    + (province.bounds[2] - province.bounds[0]) * (column as f32 + 0.5) / 5.0,
                province.bounds[1]
                    + (province.bounds[3] - province.bounds[1]) * (row as f32 + 0.5) / 5.0,
            ];
            if province.contains(center) {
                centers.push(center);
            }
        }
    }
    centers.sort_by(|a, b| {
        let distance = |point: &[f32; 2]| {
            let dx = (point[0] - province.visual_center[0]) * LONGITUDE_SCALE;
            let dy = point[1] - province.visual_center[1];
            dx * dx + dy * dy
        };
        distance(a).total_cmp(&distance(b))
    });

    let angles = [
        0.0,
        0.35,
        -0.35,
        0.7,
        -0.7,
        1.05,
        -1.05,
        std::f32::consts::FRAC_PI_2,
        -std::f32::consts::FRAC_PI_2,
    ];
    let preferred_size = (10.5 * zoom.sqrt()).clamp(9.0, 18.0);
    let mut choices = Vec::new();
    let names = if province.name == province.short {
        vec![true]
    } else {
        vec![true, false]
    };
    'sizes: for step in 0..12 {
        let font_size = preferred_size - step as f32 * 1.25;
        if font_size < 6.5 {
            break;
        }
        for &full_name in &names {
            let name = if full_name {
                &province.name
            } else {
                &province.short
            };
            let galley =
                painter.layout_no_wrap(name.clone(), egui::FontId::proportional(font_size), INK);
            let size = label_fit_size(galley.size());
            for &center in &centers {
                for &angle in &angles {
                    if label_fits_province(province, projection, center, size, angle) {
                        choices.push(LabelPlacement {
                            center,
                            angle,
                            font_size,
                            full_name,
                        });
                        if choices.len() >= 16 {
                            break 'sizes;
                        }
                        break;
                    }
                }
            }
        }
    }
    // A slight size reduction is preferable to pushing a name to the edge of
    // its province. Keep widely smaller options behind all readable ones.
    let steps_per_band = if preferred_size >= 13.0 {
        3.0
    } else {
        2.0
    };
    let band = |candidate: &LabelPlacement| {
        ((preferred_size - candidate.font_size) / (1.25 * steps_per_band)).floor() as i32
    };
    let center_distance = |candidate: &LabelPlacement| {
        let dx = (candidate.center[0] - province.visual_center[0]) * LONGITUDE_SCALE;
        let dy = candidate.center[1] - province.visual_center[1];
        dx * dx + dy * dy
    };
    choices.sort_by(|a, b| {
        band(a)
            .cmp(&band(b))
            .then_with(|| center_distance(a).total_cmp(&center_distance(b)))
            .then_with(|| b.full_name.cmp(&a.full_name))
            .then_with(|| b.font_size.total_cmp(&a.font_size))
            .then_with(|| a.angle.abs().total_cmp(&b.angle.abs()))
    });
    choices
}

fn anchored_label_candidates(
    painter: &egui::Painter,
    province: &Province,
    projection: &Projection,
    zoom: f32,
    anchor: &LabelPlacement,
) -> Vec<LabelPlacement> {
    let preferred_size = (10.5 * zoom.sqrt()).clamp(9.0, 18.0);
    let mut choices = Vec::new();
    for step in 0..12 {
        let font_size = preferred_size - step as f32 * 1.25;
        if font_size < 6.5 {
            break;
        }
        for full_name in [true, false] {
            if !full_name && province.name == province.short {
                continue;
            }
            let name = if full_name {
                &province.name
            } else {
                &province.short
            };
            let galley =
                painter.layout_no_wrap(name.clone(), egui::FontId::proportional(font_size), INK);
            if label_fits_province(
                province,
                projection,
                anchor.center,
                label_fit_size(galley.size()),
                anchor.angle,
            ) {
                choices.push(LabelPlacement {
                    center: anchor.center,
                    angle: anchor.angle,
                    font_size,
                    full_name,
                });
            }
        }
    }
    choices
}

fn readable_label_floor(regular: &[LabelPlacement]) -> f32 {
    regular.iter().map(|candidate| candidate.font_size).fold(6.5_f32, f32::max) - 2.5
}

fn marker_blocks_readable_anchor(
    painter: &egui::Painter,
    province: &Province,
    projection: &Projection,
    anchored: &[LabelPlacement],
    regular: &[LabelPlacement],
    markers: &[egui::Rect],
) -> bool {
    let mut readable = false;
    for candidate in
        anchored.iter().filter(|candidate| candidate.font_size >= readable_label_floor(regular))
    {
        readable = true;
        let name = if candidate.full_name {
            &province.name
        } else {
            &province.short
        };
        let galley = painter.layout_no_wrap(
            name.clone(),
            egui::FontId::proportional(candidate.font_size),
            INK,
        );
        let bounds =
            rotated_bounds(projection.point(candidate.center), galley.size(), candidate.angle)
                .expand(2.0);
        if markers.iter().all(|marker| !marker.intersects(bounds)) {
            return false;
        }
    }
    readable
}

fn label_choice_order<'a>(
    anchored: &'a [LabelPlacement],
    regular: &'a [LabelPlacement],
    has_anchor: bool,
    allow_relocation: bool,
) -> Vec<(&'a LabelPlacement, bool)> {
    if !has_anchor {
        regular.iter().map(|candidate| (candidate, false)).collect()
    } else if allow_relocation {
        regular
            .iter()
            .filter(|candidate| candidate.font_size >= readable_label_floor(regular))
            .map(|candidate| (candidate, true))
            .chain(anchored.iter().map(|candidate| (candidate, false)))
            .collect()
    } else {
        anchored.iter().map(|candidate| (candidate, false)).collect()
    }
}

fn label_fit_size(text_size: egui::Vec2) -> egui::Vec2 {
    text_size + egui::vec2((text_size.x * LABEL_HORIZONTAL_MARGIN).max(8.0), 6.0)
}

fn label_fits_province(
    province: &Province,
    projection: &Projection,
    center: [f32; 2],
    size: egui::Vec2,
    angle: f32,
) -> bool {
    let (sin, cos) = angle.sin_cos();
    let screen_center = projection.point(center);
    let screen_bounds = rotated_bounds(screen_center, size, angle);
    let lower_left = projection.inverse(screen_bounds.left_bottom());
    let upper_right = projection.inverse(screen_bounds.right_top());
    if lower_left[0] < province.bounds[0]
        || lower_left[1] < province.bounds[1]
        || upper_right[0] > province.bounds[2]
        || upper_right[1] > province.bounds[3]
    {
        return false;
    }
    let columns = ((size.x / 6.0).ceil() as usize).clamp(4, 32);
    for column in 0..=columns {
        let x = size.x * (column as f32 / columns as f32 - 0.5);
        for row in 0..=2 {
            let y = size.y * (row as f32 * 0.5 - 0.5);
            let point = screen_center + egui::vec2(x * cos - y * sin, x * sin + y * cos);
            if !province.contains(projection.inverse(point)) {
                return false;
            }
        }
    }
    true
}

struct CityMarker {
    city_index: usize,
    icon: egui::Rect,
    image: egui::Rect,
    bounds: egui::Rect,
    is_rome: bool,
}

fn city_blend(zoom: f32) -> f32 {
    let t = ((zoom - CITY_BLEND_START) / (CITY_BLEND_END - CITY_BLEND_START)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn city_rect(anchor: egui::Pos2, size: f32, hotspot: [f32; 2]) -> egui::Rect {
    egui::Rect::from_min_size(
        anchor - egui::vec2(size * hotspot[0], size * hotspot[1]),
        egui::vec2(size, size),
    )
}

fn layout_cities(projection: &Projection, map_rect: egui::Rect, zoom: f32) -> Vec<CityMarker> {
    let blend = city_blend(zoom);
    let icon_size = 24.0 + 1.2 * (zoom - MIN_ZOOM).max(0.0);
    let image_size = (30.0 + 8.0 * (zoom - 2.0)).max(24.0) * CITY_IMAGE_SCALE;
    CITIES
        .iter()
        .enumerate()
        .filter_map(|(index, city)| {
            let is_rome = index == 0;
            let anchor = projection.point(city.position);
            let icon_scale = if is_rome {
                1.2
            } else {
                1.0
            };
            let image_scale = if is_rome {
                1.45
            } else {
                1.0
            };
            let icon = city_rect(anchor, icon_size * icon_scale, city.hotspot);
            let image = city_rect(anchor, image_size * image_scale, city.hotspot);
            let bounds = if blend <= 0.0 {
                icon
            } else if blend >= 1.0 {
                image
            } else {
                icon.union(image)
            }
            .expand(2.0);
            map_rect.intersects(bounds).then_some(CityMarker {
                city_index: index,
                icon,
                image,
                bounds,
                is_rome,
            })
        })
        .collect()
}

fn paint_cities(
    painter: &egui::Painter,
    markers: &[CityMarker],
    zoom: f32,
    textures: &[egui::TextureHandle],
) {
    let (Some(image), Some(rome_image)) = (textures.first(), textures.get(1)) else {
        return;
    };
    let blend = city_blend(zoom);
    let icon_tint = egui::Color32::from_white_alpha(((1.0 - blend) * 255.0).round() as u8);
    let image_tint = egui::Color32::from_white_alpha((blend * 255.0).round() as u8);
    for marker in markers {
        if blend < 1.0 {
            paint_marker_icon(painter, marker.icon, MarkerIcon::City, icon_tint);
        }
        if blend > 0.0 {
            let city_image = if marker.is_rome {
                rome_image
            } else {
                image
            };
            painter.image(
                city_image.id(),
                marker.image,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                image_tint,
            );
        }
    }
}

struct WonderMarker {
    index: usize,
    icon: egui::Rect,
    image: Option<egui::Rect>,
}

struct WonderArt {
    uv: egui::Rect,
    width_to_height: f32,
}

fn wonder_art() -> &'static [WonderArt] {
    static ART: OnceLock<Vec<WonderArt>> = OnceLock::new();
    ART.get_or_init(|| {
        WONDERS
            .iter()
            .map(|wonder| {
                let rgba = image::load_from_memory(wonder.png)
                    .expect("wonder PNG assets must be valid")
                    .to_rgba8();
                let (width, height) = rgba.dimensions();
                let (mut left, mut top, mut right, mut bottom) = (width, height, 0, 0);
                for (x, y, pixel) in rgba.enumerate_pixels() {
                    if pixel[3] > 0 {
                        left = left.min(x);
                        top = top.min(y);
                        right = right.max(x + 1);
                        bottom = bottom.max(y + 1);
                    }
                }
                assert!(right > left && bottom > top, "{} has no visible pixels", wonder.name);
                WonderArt {
                    uv: egui::Rect::from_min_max(
                        egui::pos2(left as f32 / width as f32, top as f32 / height as f32),
                        egui::pos2(right as f32 / width as f32, bottom as f32 / height as f32),
                    ),
                    width_to_height: (right - left) as f32 / (bottom - top) as f32,
                }
            })
            .collect()
    })
}

impl WonderMarker {
    fn bounds(&self, zoom: f32) -> egui::Rect {
        let blend = city_blend(zoom);
        if let Some(image) = self.image {
            if blend >= 1.0 {
                return image;
            }
            if blend > 0.0 {
                return self.icon.union(image);
            }
        }
        self.icon
    }
}

fn layout_wonders(projection: &Projection, map_rect: egui::Rect, zoom: f32) -> Vec<WonderMarker> {
    let mut markers: Vec<WonderMarker> = Vec::with_capacity(WONDERS.len());
    for (index, wonder) in WONDERS.iter().enumerate() {
        let anchor = projection.point(wonder.position);
        if !map_rect.contains(anchor) {
            continue;
        }
        // The overview symbol and detailed art share the exact map coordinate.
        let icon_size = 22.0 + (zoom - MIN_ZOOM).clamp(0.0, 2.0);
        let icon = egui::Rect::from_center_size(anchor, egui::vec2(icon_size, icon_size));
        // Size the visible artwork, not the padded PNG canvas. Land and nearby
        // markers do not shrink a wonder; coastal art may extend over water.
        let art = &wonder_art()[index];
        let height = (18.0 * zoom).min(110.0);
        let image_rect =
            egui::Rect::from_center_size(anchor, egui::vec2(height * art.width_to_height, height));
        let on_land = atlas()
            .marker_land
            .iter()
            .chain(&atlas().land)
            .any(|part| part.contains(wonder.position));
        let image = on_land.then_some(image_rect);
        markers.push(WonderMarker {
            index,
            icon,
            image,
        });
    }
    markers
}

fn paint_wonders(
    painter: &egui::Painter,
    markers: &[WonderMarker],
    zoom: f32,
    textures: &[egui::TextureHandle],
) {
    for marker in markers {
        let blend = if marker.image.is_some() {
            city_blend(zoom)
        } else {
            0.0
        };
        if blend < 1.0 {
            paint_marker_icon(
                painter,
                marker.icon,
                MarkerIcon::Wonder,
                egui::Color32::from_white_alpha(((1.0 - blend) * 255.0).round() as u8),
            );
        }
        if let (Some(image), Some(texture)) = (marker.image, textures.get(marker.index)) {
            if blend > 0.0 {
                painter.image(
                    texture.id(),
                    image,
                    wonder_art()[marker.index].uv,
                    egui::Color32::from_white_alpha((blend * 255.0).round() as u8),
                );
            }
        }
    }
}

impl Province {
    fn contains(&self, [lon, lat]: [f32; 2]) -> bool {
        if lon < self.bounds[0]
            || lon > self.bounds[2]
            || lat < self.bounds[1]
            || lat > self.bounds[3]
        {
            return false;
        }
        self.parts.iter().any(|part| part.contains([lon, lat]))
    }
}

impl MapMesh {
    fn cache_bounds(&mut self) {
        self.bounds = self.v.iter().fold(
            [f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY],
            |mut bounds, &[lon, lat]| {
                bounds[0] = bounds[0].min(lon);
                bounds[1] = bounds[1].min(lat);
                bounds[2] = bounds[2].max(lon);
                bounds[3] = bounds[3].max(lat);
                bounds
            },
        );
    }

    fn contains(&self, point: [f32; 2]) -> bool {
        let mut start = 0;
        let mut inside = false;
        for &end in &self.r {
            if point_in_ring(point, &self.v[start..end]) {
                inside = !inside;
            }
            start = end;
        }
        inside
    }
}

fn point_in_ring([x, y]: [f32; 2], ring: &[[f32; 2]]) -> bool {
    let mut inside = false;
    for index in 0..ring.len() {
        let a = ring[index];
        let b = ring[(index + ring.len() - 1) % ring.len()];
        if (a[1] > y) != (b[1] > y) && x < (b[0] - a[0]) * (y - a[1]) / (b[1] - a[1]) + a[0] {
            inside = !inside;
        }
    }
    inside
}

fn smoothstep(value: f32) -> f32 {
    let t = value.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn blend_color(a: egui::Color32, b: egui::Color32, amount: f32) -> egui::Color32 {
    let mix =
        |left: u8, right: u8| (f32::from(left) * (1.0 - amount) + f32::from(right) * amount) as u8;
    egui::Color32::from_rgb(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()))
}

fn random_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|time| time.as_nanos() as u64)
        .unwrap_or(0x9e37_79b9_7f4a_7c15)
}

fn next_random(random: &mut u64) -> f32 {
    let mut value = *random;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    *random = value.max(1);
    ((*random >> 40) as f32) / ((1_u32 << 24) as f32)
}

pub(super) fn point_on_land(land: &[MapMesh], point: [f32; 2]) -> bool {
    land.iter().any(|part| {
        point[0] >= part.bounds[0]
            && point[0] <= part.bounds[2]
            && point[1] >= part.bounds[1]
            && point[1] <= part.bounds[3]
            && part.contains(point)
    })
}

fn generate_water_ripples(land: &[MapMesh], random: &mut u64) -> Vec<WaterRipple> {
    let mut ripples = Vec::with_capacity(3_500);
    for _ in 0..35_000 {
        if ripples.len() == 3_500 {
            break;
        }
        let position = [-18.0 + next_random(random) * 70.0, 17.0 + next_random(random) * 44.0];
        if point_on_land(land, position) {
            continue;
        }
        ripples.push(WaterRipple {
            position,
            length: 9.0 + next_random(random).powi(2) * 17.0,
            bend: (0.8 + next_random(random) * 2.0)
                * if next_random(random) < 0.5 {
                    -1.0
                } else {
                    1.0
                },
            tilt: (next_random(random) - 0.5) * 0.3,
            double_crest: next_random(random) > 0.32,
            phase: next_random(random) * 24.0,
            duration: 3.5 + next_random(random) * 4.0,
            quiet: 4.0 + next_random(random) * 8.0,
            seed: (next_random(random) * u32::MAX as f32) as u32,
        });
    }
    ripples
}

fn crest_life(time: f32, phase: f32, duration: f32, quiet: f32) -> (u32, f32, f32) {
    let elapsed = time + phase;
    let period = duration + quiet;
    let cycle = (elapsed / period).floor() as u32;
    let age = elapsed.rem_euclid(period);
    let life = (age / duration).min(1.0);
    let envelope = if age < duration {
        (std::f32::consts::PI * life).sin().powi(2)
    } else {
        0.0
    };
    (cycle, life, envelope)
}

fn crest_variation(seed: u32, cycle: u32) -> [f32; 3] {
    std::array::from_fn(|channel| {
        let mut value =
            seed ^ cycle.wrapping_mul(0x9e37_79b9) ^ (channel as u32 + 1).wrapping_mul(0x85eb_ca6b);
        value ^= value >> 16;
        value = value.wrapping_mul(0x7feb_352d);
        value ^= value >> 15;
        value = value.wrapping_mul(0x846c_a68b);
        value ^= value >> 16;
        (value >> 8) as f32 / 16_777_216.0
    })
}

fn paint_water_ripples(
    painter: &egui::Painter,
    ripples: &[WaterRipple],
    projection: &Projection,
    time: f32,
    zoom_fade: f32,
) {
    let clip = painter.clip_rect().expand(30.0 + projection.scale * 0.2);
    for ripple in ripples {
        let base_center = projection.point(ripple.position);
        if !clip.contains(base_center) {
            continue;
        }
        let (cycle, life, envelope) = crest_life(time, ripple.phase, ripple.duration, ripple.quiet);
        if envelope < 0.01 {
            continue;
        }
        // Each appearance gets a different position, length, and angle. Its
        // quiet interval lets the old crest disappear before a new one forms.
        let variation = crest_variation(ripple.seed, cycle);
        let center = projection.point([
            ripple.position[0] + (variation[0] - 0.5) * 0.30,
            ripple.position[1] + (variation[1] - 0.5) * 0.20,
        ]) + egui::vec2(life * (3.0 + variation[2] * 4.0), -life * 2.0);
        let tilt = ripple.tilt + (variation[1] - 0.5) * 0.65;
        for (line, width_fraction) in [(0.0, 1.0), (1.0, 0.62)] {
            if line > 0.0 && !ripple.double_crest {
                continue;
            }
            let width =
                ripple.length * width_fraction * (0.65 + variation[2] * 0.55) * (0.7 + life * 0.3);
            let origin = center + egui::vec2(line * ripple.length * 0.14, line * 3.2);
            let points = [-0.5_f32, -0.18, 0.14, 0.5]
                .map(|x| {
                    origin
                        + egui::vec2(
                            x * width,
                            x * width * tilt + (1.0 - (x * 2.0).abs()) * ripple.bend,
                        )
                })
                .to_vec();
            let alpha = (envelope * 84.0 * zoom_fade * (1.0 - line * 0.40)) as u8;
            painter.add(egui::Shape::line(
                points,
                egui::Stroke::new(
                    0.75 + envelope * 0.25,
                    egui::Color32::from_rgba_unmultiplied(177, 216, 227, alpha),
                ),
            ));
        }
    }
}

fn update_wildlife(
    view: &mut MapView,
    land: &[MapMesh],
    projection: &Projection,
    rect: egui::Rect,
    dt: f32,
) {
    let dt = dt.clamp(0.0, 0.1);
    for sighting in &mut view.wildlife {
        sighting.age += dt;
        sighting.position[0] += sighting.velocity[0] * dt;
        sighting.position[1] += sighting.velocity[1] * dt;
    }
    view.wildlife.retain(|sighting| sighting.age < sighting.duration);
    // Let each sighting finish before starting the next quiet interval.
    if !view.wildlife.is_empty() {
        return;
    }
    view.wildlife_spawn_timer -= dt;
    if view.wildlife_spawn_timer > 0.0 {
        return;
    }

    view.wildlife_spawn_timer = 2.0 + next_random(&mut view.wildlife_rng) * 6.0;
    let roll = next_random(&mut view.wildlife_rng);
    if roll >= 0.42 {
        spawn_birds(view, projection, rect);
        return;
    }

    let top_left = projection.inverse(rect.min);
    let bottom_right = projection.inverse(rect.max);
    let west = top_left[0].min(bottom_right[0]);
    let east = top_left[0].max(bottom_right[0]);
    let south = top_left[1].min(bottom_right[1]);
    let north = top_left[1].max(bottom_right[1]);
    let margin_x = (east - west) * 0.08;
    let margin_y = (north - south) * 0.08;
    let mut water_position = None;
    for _ in 0..48 {
        let position = [
            west + margin_x + next_random(&mut view.wildlife_rng) * (east - west - margin_x * 2.0),
            south
                + margin_y
                + next_random(&mut view.wildlife_rng) * (north - south - margin_y * 2.0),
        ];
        if !point_on_land(land, position) {
            water_position = Some(position);
            break;
        }
    }
    let Some(position) = water_position else {
        return;
    };
    let (texture, duration, velocity, size) = if roll < 0.10 {
        (WHALE_TEXTURE, 5.0, [0.004, 0.002], 40.0)
    } else if roll < 0.27 {
        (DOLPHIN_TEXTURE, 3.6, [0.012, 0.005], 38.0)
    } else {
        (VORTEX_TEXTURE, 4.8, [0.0, 0.0], 52.0)
    };
    view.wildlife.push(WildlifeSighting {
        texture,
        position,
        velocity,
        age: 0.0,
        duration,
        size,
        rotation: 0.0,
        frame_offset: 0.0,
    });
}

fn bird_texture(roll: f32) -> usize {
    if roll < 0.42 {
        SEABIRDS_TEXTURE
    } else if roll < 0.72 {
        EAGLE_TEXTURE
    } else {
        TERN_TEXTURE
    }
}

fn is_bird_texture(texture: usize) -> bool {
    matches!(texture, SEABIRDS_TEXTURE | EAGLE_TEXTURE | TERN_TEXTURE)
}

fn spawn_birds(view: &mut MapView, projection: &Projection, rect: egui::Rect) {
    let texture = bird_texture(next_random(&mut view.wildlife_rng));
    let count_roll = next_random(&mut view.wildlife_rng);
    let requested = if count_roll < 0.48 {
        1
    } else if count_roll < 0.84 {
        2
    } else {
        3
    };
    let count = if texture == EAGLE_TEXTURE {
        1
    } else {
        requested
    };
    let side_by_side = next_random(&mut view.wildlife_rng) < 0.5;
    let duration = 6.0 + next_random(&mut view.wildlife_rng) * 3.0;
    let angle = next_random(&mut view.wildlife_rng) * std::f32::consts::TAU;
    let direction = egui::vec2(angle.cos(), angle.sin());
    let min_side = rect.width().min(rect.height());
    let start_screen = egui::pos2(
        rect.left() + rect.width() * (0.2 + next_random(&mut view.wildlife_rng) * 0.6),
        rect.top() + rect.height() * (0.2 + next_random(&mut view.wildlife_rng) * 0.6),
    );
    let margin = min_side * 0.06
        + if count > 1 {
            30.0
        } else {
            0.0
        };
    let room_x = if direction.x >= 0.0 {
        (rect.right() - margin - start_screen.x) / direction.x.max(0.0001)
    } else {
        (start_screen.x - rect.left() - margin) / (-direction.x).max(0.0001)
    };
    let room_y = if direction.y >= 0.0 {
        (rect.bottom() - margin - start_screen.y) / direction.y.max(0.0001)
    } else {
        (start_screen.y - rect.top() - margin) / (-direction.y).max(0.0001)
    };
    let desired_distance = min_side * (0.12 + next_random(&mut view.wildlife_rng) * 0.18);
    let distance = desired_distance.min(room_x.min(room_y).max(min_side * 0.08));
    let end_screen = start_screen + direction * distance;
    let start_map = projection.inverse(start_screen);
    let end_map = projection.inverse(end_screen);
    let velocity = [(end_map[0] - start_map[0]) / duration, (end_map[1] - start_map[1]) / duration];
    // All sheet birds face screen-up; rotate each one to the flight heading.
    let rotation = angle + std::f32::consts::FRAC_PI_2;
    for index in 0..count {
        let rank = index as f32 - (count as f32 - 1.0) * 0.5;
        let offset = if side_by_side {
            egui::vec2(-direction.y, direction.x) * rank * 28.0 - direction * rank.abs() * 11.0
        } else {
            direction * rank * 32.0
        };
        let position = projection.inverse(start_screen + offset);
        view.wildlife.push(WildlifeSighting {
            texture,
            position,
            velocity,
            age: 0.0,
            duration: duration + (next_random(&mut view.wildlife_rng) - 0.5) * 0.7,
            size: if texture == EAGLE_TEXTURE {
                28.0
            } else {
                23.0
            },
            rotation,
            frame_offset: next_random(&mut view.wildlife_rng) * 16.0,
        });
    }
}

fn paint_wildlife(
    painter: &egui::Painter,
    sightings: &[WildlifeSighting],
    textures: &[egui::TextureHandle],
    projection: &Projection,
) {
    for sighting in sightings {
        let Some(texture) = textures.get(sighting.texture) else {
            continue;
        };
        let frame = if is_bird_texture(sighting.texture) {
            (sighting.age * 12.0 + sighting.frame_offset) as usize % 16
        } else {
            (((sighting.age / sighting.duration) * 16.0).floor() as usize).min(15)
        };
        let column = frame % 4;
        let row = frame / 4;
        let sprite_uv = egui::Rect::from_min_max(
            egui::pos2(column as f32 * 0.25, row as f32 * 0.25),
            egui::pos2((column + 1) as f32 * 0.25, (row + 1) as f32 * 0.25),
        );
        let center = projection.point(sighting.position);
        let image_rect =
            egui::Rect::from_center_size(center, egui::vec2(sighting.size, sighting.size));
        if !image_rect.intersects(painter.clip_rect()) {
            continue;
        }
        let fade = if is_bird_texture(sighting.texture) {
            smoothstep(sighting.age / 2.0) * smoothstep((sighting.duration - sighting.age) / 2.8)
        } else if sighting.texture == VORTEX_TEXTURE {
            smoothstep(sighting.age / 0.7) * smoothstep((sighting.duration - sighting.age) / 0.7)
        } else {
            1.0
        };
        let tint = egui::Color32::from_white_alpha((fade * 255.0) as u8);
        if is_bird_texture(sighting.texture) {
            let half = sighting.size * 0.5;
            let corners = [
                egui::vec2(-half, -half),
                egui::vec2(half, -half),
                egui::vec2(half, half),
                egui::vec2(-half, half),
            ];
            let (sin, cos) = sighting.rotation.sin_cos();
            let mut mesh = egui::Mesh::with_texture(texture.id());
            for (corner, uv) in corners.into_iter().zip([
                sprite_uv.left_top(),
                sprite_uv.right_top(),
                sprite_uv.right_bottom(),
                sprite_uv.left_bottom(),
            ]) {
                let rotated =
                    egui::vec2(corner.x * cos - corner.y * sin, corner.x * sin + corner.y * cos);
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: center + rotated,
                    uv,
                    color: tint,
                });
            }
            mesh.indices.extend([0, 1, 2, 0, 2, 3]);
            painter.add(egui::Shape::mesh(mesh));
        } else {
            painter.image(texture.id(), image_rect, sprite_uv, tint);
        }
    }
}

fn advance_texture_offset(
    offset: &mut [f32; 2],
    screen_pixels_per_second: [f32; 2],
    tile_degrees: [f32; 2],
    scale: f32,
    dt: f32,
) {
    // Accumulate motion in map coordinates so zooming cannot jump the clouds.
    // Dividing by the current scale keeps their screen speed easy to see.
    let dt = dt.clamp(0.0, 0.1);
    offset[0] = (offset[0] + screen_pixels_per_second[0] * dt / (LONGITUDE_SCALE * scale))
        .rem_euclid(tile_degrees[0]);
    offset[1] = (offset[1] - screen_pixels_per_second[1] * dt / scale).rem_euclid(tile_degrees[1]);
}

fn paint_scrolling_texture(
    painter: &egui::Painter,
    rect: egui::Rect,
    projection: &Projection,
    texture: &egui::TextureHandle,
    tile_degrees: [f32; 2],
    offset: [f32; 2],
    tint: egui::Color32,
) {
    // Place tiles in geographic space so panning and zooming do not make the
    // waves or clouds slide against the coast. Only the deliberate drift moves.
    let top_left = projection.inverse(rect.min);
    let bottom_right = projection.inverse(rect.max);
    let shift_lon = offset[0];
    let shift_lat = offset[1];
    let first_x = ((top_left[0] - shift_lon) / tile_degrees[0]).floor() as i32;
    let last_x = ((bottom_right[0] - shift_lon) / tile_degrees[0]).ceil() as i32;
    let first_y = ((bottom_right[1] - shift_lat) / tile_degrees[1]).floor() as i32;
    let last_y = ((top_left[1] - shift_lat) / tile_degrees[1]).ceil() as i32;
    let uv = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0));
    for y in first_y..last_y {
        for x in first_x..last_x {
            let lon = x as f32 * tile_degrees[0] + shift_lon;
            let lat = y as f32 * tile_degrees[1] + shift_lat;
            let image_rect = egui::Rect::from_min_max(
                projection.point([lon, lat + tile_degrees[1]]),
                projection.point([lon + tile_degrees[0], lat]),
            );
            if image_rect.intersects(rect) {
                painter.image(texture.id(), image_rect, uv, tint);
            }
        }
    }
}

fn paint_dead_sea(
    painter: &egui::Painter,
    view: &MapView,
    projection: &Projection,
    sea_color: egui::Color32,
) {
    if !projection.bounds_rect(DEAD_SEA_BOUNDS).intersects(painter.clip_rect()) {
        return;
    }

    let mut water = egui::Mesh::default();
    for rows in DEAD_SEA_ROWS.windows(2) {
        let base = water.vertices.len() as u32;
        for point in lake_strip(rows) {
            water.vertices.push(egui::epaint::Vertex {
                pos: projection.point(point),
                uv: egui::Pos2::ZERO,
                color: sea_color,
            });
        }
        water.indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    painter.add(egui::Shape::mesh(water));

    let close = smoothstep((view.zoom - 1.0) / 2.5);
    let strips: Vec<_> = DEAD_SEA_ROWS.windows(2).map(lake_strip).collect();
    paint_lake(
        painter,
        |point| projection.point(point),
        &strips,
        view.environment_textures.get(WAVE_TEXTURE),
        view.environment_textures.get(RIPPLE_TEXTURE),
        view.animation_clock,
        view.zoom,
    );

    if view.zoom >= 2.6 {
        paint_dead_sea_crests(painter, projection, view.animation_clock, view.zoom);
    }

    let mut shore = Vec::with_capacity(DEAD_SEA_ROWS.len() * 2);
    shore.extend(DEAD_SEA_ROWS.iter().map(|row| projection.point([row[1], row[0]])));
    shore.extend(DEAD_SEA_ROWS.iter().rev().map(|row| projection.point([row[2], row[0]])));
    painter.add(egui::Shape::closed_line(
        shore,
        egui::Stroke::new(
            0.6 + close * 0.5,
            egui::Color32::from_rgba_unmultiplied(102, 164, 170, 160),
        ),
    ));
}

fn lake_strip(rows: &[[f32; 3]]) -> [[f32; 2]; 4] {
    [
        [rows[0][1], rows[0][0]],
        [rows[0][2], rows[0][0]],
        [rows[1][2], rows[1][0]],
        [rows[1][1], rows[1][0]],
    ]
}

fn paint_dead_sea_crests(painter: &egui::Painter, projection: &Projection, time: f32, zoom: f32) {
    let fade = smoothstep((zoom - 2.6) / 1.2);
    for (index, row_index) in [4, 7, 13].into_iter().enumerate() {
        let row = DEAD_SEA_ROWS[row_index];
        let (cycle, life, envelope) =
            crest_life(time, index as f32 * 4.7, 4.1 + index as f32, 5.3 + index as f32 * 1.3);
        if envelope < 0.01 {
            continue;
        }
        let variation = crest_variation(index as u32 + 79, cycle);
        let center = projection.point([(row[1] + row[2]) * 0.5, row[0]])
            + egui::vec2((variation[0] - 0.5) * 1.6, life - 0.5);
        let width =
            (row[2] - row[1]) * LONGITUDE_SCALE * projection.scale * (0.35 + variation[1] * 0.26);
        let points = [-0.5_f32, -0.18, 0.14, 0.5]
            .map(|x| center + egui::vec2(x * width, (1.0 - (x * 2.0).abs()) * 1.3))
            .to_vec();
        let alpha = (envelope * 84.0 * fade) as u8;
        painter.add(egui::Shape::line(
            points,
            egui::Stroke::new(
                0.75 + envelope * 0.25,
                egui::Color32::from_rgba_unmultiplied(177, 216, 227, alpha),
            ),
        ));
    }
}

fn terrain_uv([lon, lat]: [f32; 2], [west, south, east, north]: [f32; 4]) -> egui::Pos2 {
    egui::pos2(
        (1.0 + (lon - west) / (east - west) * 1920.0) / 1922.0,
        (1.0 + (north - lat) / (north - south) * 1110.0) / 1112.0,
    )
}

fn clip_terrain_polygon(mut polygon: Vec<[f32; 2]>, bounds: [f32; 4]) -> Vec<[f32; 2]> {
    for (axis, boundary, keep_greater) in
        [(0, bounds[0], true), (0, bounds[2], false), (1, bounds[1], true), (1, bounds[3], false)]
    {
        if polygon.len() < 3 {
            return Vec::new();
        }
        let mut clipped = Vec::new();
        for (start, end) in
            polygon.iter().copied().zip(polygon.iter().copied().cycle().skip(1)).take(polygon.len())
        {
            let start_inside = if keep_greater {
                start[axis] >= boundary
            } else {
                start[axis] <= boundary
            };
            let end_inside = if keep_greater {
                end[axis] >= boundary
            } else {
                end[axis] <= boundary
            };
            if start_inside != end_inside {
                let fraction = (boundary - start[axis]) / (end[axis] - start[axis]);
                let mut point = [
                    start[0] + (end[0] - start[0]) * fraction,
                    start[1] + (end[1] - start[1]) * fraction,
                ];
                point[axis] = boundary;
                clipped.push(point);
            }
            if end_inside {
                clipped.push(end);
            }
        }
        polygon = clipped;
    }
    polygon
}

fn build_terrain_tiles(parts: &[MapMesh]) -> Vec<TerrainMesh> {
    // Clip only the selectable province triangles, including their holes.
    // This one-time geographic split never adds neutral land or sea pixels.
    TERRAIN_TILE_BOUNDS
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(tile, bounds)| {
            let mut geometry = MapMesh {
                v: Vec::new(),
                r: Vec::new(),
                t: Vec::new(),
                bounds,
            };
            for part in parts {
                if part.bounds[0] > bounds[2]
                    || part.bounds[2] < bounds[0]
                    || part.bounds[1] > bounds[3]
                    || part.bounds[3] < bounds[1]
                {
                    continue;
                }
                for triangle in part.t.chunks_exact(3) {
                    let polygon = clip_terrain_polygon(
                        triangle.iter().map(|index| part.v[*index as usize]).collect(),
                        bounds,
                    );
                    if polygon.len() < 3 {
                        continue;
                    }
                    let offset = geometry.v.len() as u32;
                    geometry.v.extend(&polygon);
                    for index in 1..polygon.len() - 1 {
                        if terrain_triangle_area(polygon[0], polygon[index], polygon[index + 1])
                            > 0.0
                        {
                            geometry.t.extend([
                                offset,
                                offset + index as u32,
                                offset + index as u32 + 1,
                            ]);
                        }
                    }
                }
            }
            if geometry.t.is_empty() {
                None
            } else {
                geometry.cache_bounds();
                Some(TerrainMesh {
                    tile,
                    geometry,
                })
            }
        })
        .collect()
}

fn terrain_triangle_area(a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> f64 {
    let ab = [f64::from(b[0]) - f64::from(a[0]), f64::from(b[1]) - f64::from(a[1])];
    let ac = [f64::from(c[0]) - f64::from(a[0]), f64::from(c[1]) - f64::from(a[1])];
    (ab[0] * ac[1] - ab[1] * ac[0]).abs() * 0.5
}

fn province_terrain_mesh(
    tile: &TerrainMesh,
    projection: &Projection,
    clip: egui::Rect,
    texture: egui::TextureId,
) -> egui::Mesh {
    let mut mesh = egui::Mesh::with_texture(texture);
    let part = &tile.geometry;
    if projection.bounds_rect(part.bounds).intersects(clip) {
        mesh.vertices.extend(part.v.iter().map(|&point| egui::epaint::Vertex {
            pos: projection.point(point),
            uv: terrain_uv(point, TERRAIN_TILE_BOUNDS[tile.tile]),
            color: egui::Color32::WHITE,
        }));
        mesh.indices.extend(&part.t);
    }
    mesh
}

fn paint_meshes(
    painter: &egui::Painter,
    parts: &[MapMesh],
    projection: &Projection,
    color: egui::Color32,
) {
    let mut mesh = egui::Mesh::default();
    for part in parts {
        if !projection.bounds_rect(part.bounds).intersects(painter.clip_rect()) {
            continue;
        }
        let offset = mesh.vertices.len() as u32;
        mesh.vertices.extend(part.v.iter().map(|&point| egui::epaint::Vertex {
            pos: projection.point(point),
            uv: egui::Pos2::ZERO,
            color,
        }));
        mesh.indices.extend(part.t.iter().map(|index| index + offset));
    }
    if !mesh.indices.is_empty() {
        painter.add(egui::Shape::Mesh(mesh.into()));
    }
}

fn paint_rings(
    painter: &egui::Painter,
    mesh: &MapMesh,
    projection: &Projection,
    stroke: egui::Stroke,
) {
    if !projection.bounds_rect(mesh.bounds).intersects(painter.clip_rect().expand(stroke.width)) {
        return;
    }
    let mut start = 0;
    for &end in &mesh.r {
        if end - start >= 3 {
            painter.add(egui::Shape::closed_line(
                mesh.v[start..end].iter().map(|&point| projection.point(point)).collect(),
                stroke,
            ));
        }
        start = end;
    }
}

fn province_color(index: usize) -> egui::Color32 {
    const COLORS: [(u8, u8, u8); 6] = [
        (186, 182, 168),
        (178, 181, 169),
        (190, 185, 172),
        (180, 176, 162),
        (183, 187, 173),
        (176, 183, 171),
    ];
    let (r, g, b) = COLORS[index % COLORS.len()];
    egui::Color32::from_rgb(r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wave_crests_disappear_before_reforming_with_a_new_shape() {
        let duration = 5.0;
        let quiet = 7.0;
        assert_eq!(crest_life(0.0, 0.0, duration, quiet).2, 0.0);
        assert!(crest_life(2.5, 0.0, duration, quiet).2 > 0.99);
        for time in [5.0, 8.0, 11.999, 12.0] {
            assert_eq!(crest_life(time, 0.0, duration, quiet).2, 0.0);
        }
        assert!(crest_life(12.001, 0.0, duration, quiet).2 < 0.00001);
        let first = crest_variation(1289, 0);
        let second = crest_variation(1289, 1);
        assert!(first.iter().zip(second).any(|(a, b)| (a - b).abs() > 0.2));
        assert!(first.iter().chain(second.iter()).all(|value| (0.0..1.0).contains(value)));
    }

    #[test]
    fn terrain_uvs_face_north_and_cover_every_playable_vertex() {
        for bounds in TERRAIN_TILE_BOUNDS {
            let [west, south, east, north] = bounds;
            assert_eq!(terrain_uv([west, north], bounds), egui::pos2(1.0 / 1922.0, 1.0 / 1112.0));
            assert_eq!(
                terrain_uv([east, south], bounds),
                egui::pos2(1921.0 / 1922.0, 1111.0 / 1112.0)
            );
            assert_eq!(
                terrain_uv([(west + east) * 0.5, (south + north) * 0.5], bounds),
                egui::pos2(0.5, 0.5)
            );
        }
        for province in &atlas().provinces {
            for point in province.parts.iter().flat_map(|part| &part.v) {
                assert!(
                    TERRAIN_TILE_BOUNDS.iter().any(|bounds| {
                        point[0] >= bounds[0]
                            && point[0] <= bounds[2]
                            && point[1] >= bounds[1]
                            && point[1] <= bounds[3]
                    }),
                    "terrain raster misses {} at {point:?}",
                    province.name
                );
            }
            for tile in &province.terrain {
                let bounds = TERRAIN_TILE_BOUNDS[tile.tile];
                for point in &tile.geometry.v {
                    assert!(point[0] >= bounds[0] && point[0] <= bounds[2]);
                    assert!(point[1] >= bounds[1] && point[1] <= bounds[3]);
                    let uv = terrain_uv(*point, bounds);
                    assert!((0.0..=1.0).contains(&uv.x) && (0.0..=1.0).contains(&uv.y));
                }
            }
        }
    }

    #[test]
    fn terrain_tile_seams_preserve_every_provinces_area() {
        let projection = Projection {
            origin: egui::Pos2::ZERO,
            scale: 30.0,
            center: [20.0, 40.0],
        };
        let area = |part: &MapMesh| -> f64 {
            part.t
                .chunks_exact(3)
                .map(|face| {
                    terrain_triangle_area(
                        part.v[face[0] as usize],
                        part.v[face[1] as usize],
                        part.v[face[2] as usize],
                    )
                })
                .sum()
        };
        for province in &atlas().provinces {
            let original: f64 = province.parts.iter().map(&area).sum();
            let tiled: f64 = province.terrain.iter().map(|tile| area(&tile.geometry)).sum();
            assert!(
                (original - tiled).abs() <= original * 0.00001,
                "terrain loses or overlaps {} at a seam: {original} versus {tiled}",
                province.name
            );
            for tile in &province.terrain {
                let mesh = province_terrain_mesh(
                    tile,
                    &projection,
                    egui::Rect::EVERYTHING,
                    egui::TextureId::Managed(12),
                );
                assert!(mesh.is_valid(), "invalid terrain mesh for {}", province.name);
            }
        }
    }

    #[test]
    fn terrain_preserves_a_province_hole_and_excludes_the_backdrop() {
        let mut province = Province {
            name: "Playable ring".into(),
            short: "Ring".into(),
            label: [0.5, 0.5],
            bounds: [0.0, 0.0, 4.0, 4.0],
            parts: vec![MapMesh {
                v: vec![
                    [0.0, 0.0],
                    [4.0, 0.0],
                    [4.0, 4.0],
                    [0.0, 4.0],
                    [1.0, 1.0],
                    [3.0, 1.0],
                    [3.0, 3.0],
                    [1.0, 3.0],
                ],
                r: vec![4, 8],
                t: vec![0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7, 6, 3, 0, 4, 3, 4, 7],
                bounds: [0.0, 0.0, 4.0, 4.0],
            }],
            terrain: Vec::new(),
            visual_center: [0.5, 0.5],
            production: [0; 3],
        };
        // Position the playable ring across both tile seams.
        for part in &mut province.parts {
            for point in &mut part.v {
                point[0] += 18.0;
                point[1] += 38.5;
            }
            part.cache_bounds();
        }
        province.bounds = [18.0, 38.5, 22.0, 42.5];
        province.terrain = build_terrain_tiles(&province.parts);
        assert_eq!(province.terrain.len(), 4);
        let projection = Projection {
            origin: egui::pos2(80.0, 50.0),
            scale: 20.0,
            center: [20.0, 40.5],
        };
        let texture = egui::TextureId::Managed(12);
        let mut area = 0.0;
        for tile in &province.terrain {
            let mesh = province_terrain_mesh(tile, &projection, egui::Rect::EVERYTHING, texture);
            assert_eq!(mesh.texture_id, texture);
            for face in mesh.indices.chunks_exact(3) {
                let [a, b, c] = [face[0], face[1], face[2]]
                    .map(|index| projection.inverse(mesh.vertices[index as usize].pos));
                let center = [(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0];
                assert!(province.contains(center), "terrain face escaped the playable ring");
                area += ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() * 0.5;
            }
        }
        assert!((area - 12.0).abs() < 0.0001, "terrain filled the non-playable hole");
        let outside = projection.bounds_rect([25.0, 45.0, 26.0, 46.0]);
        assert!(province.terrain.iter().all(|tile| province_terrain_mesh(
            tile,
            &projection,
            outside,
            texture
        )
        .indices
        .is_empty()));
    }

    #[test]
    fn terrain_tiles_load_with_eguis_default_texture_limit() {
        let context = egui::Context::default();
        context.begin_pass(Default::default());
        let limit = context.input(|input| input.max_texture_side);
        for (name, png) in &ENVIRONMENT_IMAGES
            [TERRAIN_TEXTURE_START..TERRAIN_TEXTURE_START + TERRAIN_TILE_BOUNDS.len()]
        {
            let texture = load_map_texture(&context, name, png);
            assert!(texture.size().into_iter().all(|side| side <= limit));
            assert_eq!(texture.size(), [1922, 1112]);
        }
        let mut output = context.end_pass();
        output.textures_delta.clear();
    }

    #[test]
    fn resource_badges_have_only_a_below_name_position() {
        let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(300.0, 200.0));
        let name = egui::Rect::from_min_size(egui::pos2(90.0, 60.0), egui::vec2(80.0, 24.0));
        let badge = resource_badge_below(name, egui::vec2(64.0, 22.0), viewport).unwrap();
        assert_eq!(badge.top(), name.bottom() + 3.0);
        assert_eq!(badge.center().x, name.center().x);
        let bottom_name = name.translate(egui::vec2(0.0, 100.0));
        assert!(resource_badge_below(bottom_name, egui::vec2(64.0, 22.0), viewport).is_none());
    }

    #[test]
    fn zoomed_pan_can_center_edge_provinces() {
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1440.0, 900.0));
        let (center, width, height, fit) = map_geometry(rect, atlas());
        let zoom = PAN_MARGIN_FULL_ZOOM;
        let x_limit = pan_limit(width * fit * zoom, rect.width(), zoom);
        let y_limit = pan_limit(height * fit * zoom, rect.height(), zoom);
        for province in &atlas().provinces {
            let x_offset = (province.label[0] - center[0]).abs() * LONGITUDE_SCALE * fit * zoom;
            let y_offset = (province.label[1] - center[1]).abs() * fit * zoom;
            assert!(x_offset <= x_limit && y_offset <= y_limit, "{}", province.name);
        }

        let spain = atlas().provinces.iter().find(|province| province.name == "Lusitania").unwrap();
        let overview_offset = (spain.label[0] - center[0]).abs() * LONGITUDE_SCALE * fit * MIN_ZOOM;
        let overview_limit = pan_limit(width * fit * MIN_ZOOM, rect.width(), MIN_ZOOM);
        assert!(overview_offset > overview_limit);
    }

    #[test]
    fn every_mapped_province_has_plausible_nonnegative_production() {
        let atlas = atlas();
        assert_eq!(atlas.provinces.len(), super::super::production::OUTPUT.len());
        for province in &atlas.provinces {
            assert!(province.production.iter().all(|amount| *amount >= 0), "{}", province.name);
            assert!(province.production.iter().any(|amount| *amount > 0), "{}", province.name);
        }
        assert!(for_province("Aegyptus")[0] > for_province("Arabia")[0]);
        assert!(for_province("Noricum")[1] > for_province("Latium")[1]);
        assert!(for_province("Achaia")[2] > for_province("Picenum")[2]);
    }

    #[test]
    fn starting_population_uses_area_city_bias_and_four_near_standard_classes() {
        let provinces = &atlas().provinces;
        for province in provinces {
            let population = ProvincePopulation::starting(province);
            assert!(population.counts().into_iter().all(|count| count > 0.0), "{}", province.name);
            assert!((20.0..=80.0).contains(&population.total()), "{}", province.name);
            let target_percentages = if URBAN_PROVINCES.contains(&province.name.as_str()) {
                [12.0, 23.0, 37.0, 28.0]
            } else {
                [10.0, 20.0, 40.0, 30.0]
            };
            for (class, (count, percentage)) in
                population.counts().into_iter().zip(target_percentages).enumerate()
            {
                assert!(
                    (count * 100.0 - population.total() * percentage).abs()
                        <= population.total()
                            * if class == 3 {
                                7.0
                            } else {
                                3.0
                            },
                    "{} class share differs from the province type",
                    province.name
                );
            }
        }
        assert!(starting_total_for(20.0, false) > starting_total_for(1.0, false));
        assert!(starting_total_for(1.0, true) > starting_total_for(20.0, false));
    }

    #[test]
    fn resource_poor_city_starts_gain_population_without_changing_yields() {
        let mut ownership = ProvinceOwnership::default();
        ownership.start_game(&[
            egui::Color32::RED,
            egui::Color32::BLUE,
            egui::Color32::GREEN,
            egui::Color32::YELLOW,
        ]);
        let candidates = starting_candidates();
        let scores: Vec<_> = candidates
            .iter()
            .map(|&index| {
                starting_economy_score(
                    atlas().provinces[index].production,
                    ownership.populations[index],
                )
            })
            .collect();
        let target = scores.iter().copied().fold(0.0_f64, f64::max);
        for (index, &owner) in ownership.owners.iter().enumerate() {
            if let Some(player) = owner {
                let province = &atlas().provinces[index];
                let population = ownership.populations[index];
                assert!(
                    (starting_economy_score(province.production, population) - target).abs() < 1e-8,
                    "{} has an unbalanced opening economy",
                    province.name
                );
                assert!(ownership.net_production_for(player)[0] > 0.0);
            }
        }

        let ordinary = ProvincePopulation {
            nobles: 6.0,
            citizens: 12.0,
            plebeians: 20.0,
            slaves: 16.0,
        };
        let rich_yields = [6, 1, 4];
        let poor_yields = [3, 1, 1];
        let target = starting_economy_score(rich_yields, ordinary);
        let balanced = balanced_starting_population(poor_yields, ordinary, target);
        assert!(balanced.total() > ordinary.total());
        assert!(balanced.slaves - ordinary.slaves > balanced.nobles - ordinary.nobles);
        assert!((starting_economy_score(poor_yields, balanced) - target).abs() < 1e-8);
        assert_eq!(
            balanced_starting_population(rich_yields, ordinary, target).total(),
            ordinary.total()
        );
    }

    #[test]
    fn owned_population_reconciles_with_class_sources_and_famine_trend() {
        let mut ownership = ProvinceOwnership::default();
        ownership.start_game(&[egui::Color32::RED, egui::Color32::BLUE]);
        for player in 0..2 {
            let totals = ownership.population_for(player);
            for (class, total) in totals.into_iter().enumerate() {
                let sources = ownership.population_sources(player, class);
                assert_eq!(sources.iter().map(|(_, count)| count).sum::<f64>(), total);
                assert_eq!(sources.len(), 1);
            }
            let expected_growth = ownership.total_population_for(player) * 0.01;
            assert!(expected_growth > 0.0);
            assert_eq!(ownership.population_change_for(player, f64::MAX / 2.0, 0), expected_growth);
        }
        let owned = ownership.owners.iter().position(|owner| *owner == Some(0)).unwrap();
        ownership.populations[owned] = ProvincePopulation {
            nobles: 40.0,
            citizens: 0.0,
            plebeians: 0.0,
            slaves: 0.0,
        };
        assert_eq!(ownership.population_change_for(0, 0.0, 0), 0.0);
        assert_eq!(ownership.population_change_for(0, 0.0, 3), -2.0);
        assert_eq!(ownership.population_change_for(0, 0.0, 8), -5.0);
        ownership.advance_population(0, -5.0);
        assert_eq!(ownership.population_for(0).into_iter().sum::<f64>(), 35.0);
    }

    #[test]
    fn starting_provinces_are_non_rome_cities_varied_and_well_separated() {
        let atlas = atlas();
        let candidates = starting_candidates();
        assert_eq!(candidates.len(), URBAN_PROVINCES.len() - 1);
        assert!(candidates.iter().all(|&index| {
            let name = atlas.provinces[index].name.as_str();
            URBAN_PROVINCES.contains(&name) && name != ROME_PROVINCE
        }));
        for count in 1..=4 {
            let mut seen = std::collections::HashSet::new();
            for _ in 0..12 {
                let selected = spread_out_starts(count);
                assert_eq!(selected.len(), count);
                for (offset, &left) in selected.iter().enumerate() {
                    let province = &atlas.provinces[left];
                    assert!(candidates.contains(&left));
                    for &right in &selected[offset + 1..] {
                        assert_ne!(left, right);
                        assert!(
                            province_distance_km(province.label, atlas.provinces[right].label)
                                >= if count == 2 {
                                    TWO_PLAYER_MIN_START_DISTANCE_KM
                                } else {
                                    MIN_START_DISTANCE_KM
                                }
                        );
                    }
                }
                let mut set = selected;
                set.sort_unstable();
                seen.insert(set);
            }
            assert!(seen.len() > 1, "{count} players always receive the same province set");
        }
    }

    #[test]
    fn every_urban_province_has_its_city_name() {
        for name in URBAN_PROVINCES {
            assert!(city_name_for_province(name).is_some(), "{name}");
        }
        assert_eq!(city_name_for_province("Britannia"), None);
    }

    #[test]
    fn city_markers_target_their_provinces() {
        let atlas = atlas();
        assert_eq!(CITIES.len(), URBAN_PROVINCES.len());
        for city in CITIES {
            assert!(URBAN_PROVINCES.contains(&city.province));
            assert!(city_name_for_province(city.province).is_some());
            assert!(atlas.provinces.iter().any(|province| province.name == city.province));
        }
    }

    #[test]
    fn ownership_uses_each_players_color_and_only_owned_production() {
        let colors =
            [egui::Color32::RED, egui::Color32::BLUE, egui::Color32::GREEN, egui::Color32::YELLOW];
        let mut ownership = ProvinceOwnership::default();
        ownership.start_game(&colors);
        for (player, color) in colors.into_iter().enumerate() {
            let owned: Vec<_> = ownership
                .owners
                .iter()
                .enumerate()
                .filter(|(_, owner)| **owner == Some(player))
                .collect();
            assert_eq!(owned.len(), 1);
            assert_eq!(ownership.color(owned[0].0), Some(color));
            assert_eq!(ownership.production_for(player), ownership.output_for(owned[0].0));
        }
        ownership.start_game(&colors[..1]);
        assert_eq!(ownership.owners.iter().filter(|owner| owner.is_some()).count(), 1);
    }

    #[test]
    fn map_tint_can_differ_from_exact_banner_color() {
        let banner = [egui::Color32::from_rgb(109, 36, 55)];
        let tint = [egui::Color32::from_rgb(225, 76, 158)];
        let mut ownership = ProvinceOwnership::default();
        ownership.start_game(&banner);
        ownership.set_map_colors(&tint);
        let province = ownership.owners.iter().position(|owner| *owner == Some(0)).unwrap();
        assert_eq!(ownership.map_color(province), Some(tint[0]));
        assert_eq!(ownership.color(province), Some(banner[0]));
        assert_eq!(ownership.province_overview(province).unwrap().owner_color, Some(banner[0]));
    }

    #[test]
    fn trade_requires_a_shared_land_border_with_an_owned_province() {
        let provinces = &atlas().provinces;
        let find = |name| provinces.iter().position(|province| province.name == name).unwrap();
        let aegyptus = find("Aegyptus");
        let cyrenaica = find("Cyrenaica");
        let syria = find("Syria");
        let mut ownership = ProvinceOwnership::default();
        ownership.owners = vec![None; provinces.len()];
        ownership.owners[aegyptus] = Some(0);
        assert!(ownership.can_trade_with(cyrenaica, 0));
        assert!(!ownership.can_trade_with(syria, 0));
        assert!(!ownership.can_trade_with(aegyptus, 0));
        assert!(!ownership.can_trade_with(cyrenaica, 1));
    }

    #[test]
    fn slaves_add_more_output_than_plebeians_and_every_class_eats() {
        let population = ProvincePopulation {
            nobles: 2.0,
            citizens: 3.0,
            plebeians: 4.0,
            slaves: 2.0,
        };
        assert_eq!(monthly_output([3, 2, 4], population), [21.0, 14.0, 28.0]);
        assert_eq!(
            monthly_output(
                [1, 1, 1],
                ProvincePopulation {
                    nobles: 0.0,
                    citizens: 0.0,
                    plebeians: 0.0,
                    slaves: 1.0,
                }
            ),
            [1.5, 1.5, 1.5]
        );
        assert_eq!(population.food_upkeep(), 11.0);

        let mut ownership = ProvinceOwnership::default();
        ownership.start_game(&[egui::Color32::RED]);
        let province =
            atlas().provinces.iter().position(|province| province.name == "Aegyptus").unwrap();
        ownership.populations[province] = ProvincePopulation {
            nobles: 10.0,
            citizens: 18.0,
            plebeians: 12.0,
            slaves: 0.0,
        };
        let plebeian_output = ownership.output_for(province);
        ownership.populations[province] = ProvincePopulation {
            nobles: 10.0,
            citizens: 18.0,
            plebeians: 0.0,
            slaves: 12.0,
        };
        let slave_output = ownership.output_for(province);
        assert!(slave_output[0] > plebeian_output[0]);
        assert_eq!(ownership.food_upkeep_for(province), 40.0);
        assert_eq!(
            ProvincePopulation {
                nobles: 1.0,
                citizens: 1.0,
                plebeians: 1.0,
                slaves: 1.0
            }
            .food_upkeep(),
            4.0
        );
        assert_eq!(
            ProvincePopulation {
                nobles: 0.0,
                citizens: 0.0,
                plebeians: 0.0,
                slaves: 0.0
            }
            .food_upkeep(),
            0.0
        );
    }

    #[test]
    fn each_pop_class_grows_one_percent_per_fed_month() {
        let mut population = ProvincePopulation {
            nobles: 200.0,
            citizens: 400.0,
            plebeians: 800.0,
            slaves: 600.0,
        };
        assert_eq!(population.monthly_growth(0.01, 0.01), 20.0);
        population.grow(0.01, 0.01);
        assert_eq!(population.counts(), [202.0, 404.0, 808.0, 606.0]);
        assert_eq!(population.food_upkeep(), 2_020.0);

        let mut small_population = ProvincePopulation {
            nobles: 0.0,
            citizens: 0.0,
            plebeians: 1.0,
            slaves: 0.0,
        };
        small_population.grow(0.01, 0.01);
        assert_eq!(small_population.plebeians, 1.01);
        assert_eq!(monthly_output([1, 0, 0], small_population)[0], 1.01);
    }

    #[test]
    fn province_sources_reconcile_with_all_resource_deltas() {
        let mut ownership = ProvinceOwnership::default();
        ownership.start_game(&[egui::Color32::RED]);
        for resource in 0..3 {
            let sources = ownership.production_sources(0, resource);
            assert_eq!(sources.len(), 1);
            let (_, produced, consumed) = sources[0];
            assert_eq!(ownership.production_for(0)[resource], produced);
            if resource == 0 {
                assert_eq!(consumed, ownership.total_population_for(0));
                assert_eq!(ownership.net_production_for(0)[resource], produced - consumed);
            } else {
                assert_eq!(consumed, 0.0);
                assert_eq!(ownership.net_production_for(0)[resource], produced);
            }
        }
    }

    #[test]
    fn coin_taxes_use_owned_population_and_reconcile_with_monthly_delta() {
        let mut ownership = ProvinceOwnership::default();
        ownership.start_game(&[egui::Color32::RED, egui::Color32::BLUE]);
        let aegyptus =
            atlas().provinces.iter().position(|province| province.name == "Aegyptus").unwrap();
        ownership.owners.fill(None);
        ownership.owners[aegyptus] = Some(0);
        ownership.populations[aegyptus] = ProvincePopulation {
            nobles: 3.0,
            citizens: 20.0,
            plebeians: 5.0,
            slaves: 4.0,
        };

        let opening_taxes = ownership.coin_taxes_for(0);
        assert_eq!(opening_taxes, 38.0);
        assert_eq!(ownership.coin_delta_for(0), opening_taxes);
        assert_eq!(ownership.influence_delta_for(0), 3.0);
        assert_eq!(ownership.coin_taxes_for(1), 0.0);
        assert_eq!(ownership.coin_delta_for(1), 0.0);
        assert_eq!(ownership.influence_delta_for(1), 0.0);

        ownership.advance_population(0, 1.0);
        assert!((ownership.coin_delta_for(0) - opening_taxes * 1.01).abs() < 1e-9);
    }

    #[test]
    fn governance_edicts_change_only_the_owning_players_monthly_rates() {
        use super::super::EdictLevel;

        let mut ownership = ProvinceOwnership::default();
        ownership.start_game(&[egui::Color32::RED, egui::Color32::BLUE]);
        let province =
            atlas().provinces.iter().position(|province| province.name == "Aegyptus").unwrap();
        ownership.owners.fill(None);
        ownership.owners[province] = Some(0);
        ownership.populations[province] = ProvincePopulation {
            nobles: 10.0,
            citizens: 20.0,
            plebeians: 10.0,
            slaves: 10.0,
        };
        let baseline_output = ownership.output_for(province);
        assert_eq!(baseline_output[0], 150.0);
        assert_eq!(ownership.net_production_for(0)[0], 100.0);
        assert_eq!(ownership.coin_taxes_for(0), 50.0);
        assert_eq!(ownership.influence_delta_for(0), 10.0);

        let mut edicts = Governance {
            food_rations: EdictLevel::Low,
            ..Default::default()
        };
        ownership.set_governance_for(0, edicts);
        assert_eq!(ownership.net_production_for(0)[0], 110.0);
        assert_eq!(ownership.food_upkeep_for(province), 40.0);
        assert_eq!(ownership.governance_for(1), Governance::default());

        edicts.food_rations = EdictLevel::High;
        ownership.set_governance_for(0, edicts);
        assert_eq!(ownership.output_for(province), baseline_output);
        assert_eq!(ownership.food_upkeep_for(province), 60.0);
        assert_eq!(ownership.net_production_for(0)[0], 90.0);
        assert_eq!(ownership.influence_delta_for(0), 10.0);

        edicts.food_rations = EdictLevel::Medium;
        edicts.slave_labor = EdictLevel::Low;
        ownership.set_governance_for(0, edicts);
        assert_eq!(ownership.output_for(province)[0], 105.0);
        edicts.slave_labor = EdictLevel::High;
        ownership.set_governance_for(0, edicts);
        assert_eq!(ownership.output_for(province)[0], 195.0);

        edicts.noble_taxes = EdictLevel::Low;
        ownership.set_governance_for(0, edicts);
        assert_eq!(ownership.coin_taxes_for(0), 45.0);
        edicts.noble_taxes = EdictLevel::High;
        ownership.set_governance_for(0, edicts);
        assert_eq!(ownership.coin_taxes_for(0), 55.0);

        edicts.army_wages = EdictLevel::High;
        ownership.set_governance_for(0, edicts);
        assert_eq!(ownership.military_wages_for(0), 0.0);
        assert_eq!(ownership.coin_delta_for(0), 55.0);
    }

    #[test]
    fn food_rations_and_hard_labor_set_class_growth_rates() {
        use super::super::EdictLevel;

        let mut ownership = ProvinceOwnership::default();
        ownership.start_game(&[egui::Color32::RED]);
        let province =
            atlas().provinces.iter().position(|province| province.name == "Aegyptus").unwrap();
        ownership.owners.fill(None);
        ownership.owners[province] = Some(0);
        let baseline = ProvincePopulation {
            nobles: 100.0,
            citizens: 200.0,
            plebeians: 400.0,
            slaves: 300.0,
        };
        for (level, expected_growth) in
            [(EdictLevel::Low, 5.0), (EdictLevel::Medium, 10.0), (EdictLevel::High, 15.0)]
        {
            ownership.populations[province] = baseline;
            ownership.set_governance_for(
                0,
                Governance {
                    food_rations: level,
                    ..Default::default()
                },
            );
            let change = ownership.population_change_for(0, 100.0, 0);
            assert!((change - expected_growth).abs() < 1e-9);
            ownership.advance_population(0, change);
            assert!((ownership.total_population_for(0) - (1_000.0 + expected_growth)).abs() < 1e-9);

            ownership.populations[province] = baseline;
            ownership.set_governance_for(
                0,
                Governance {
                    food_rations: level,
                    slave_labor: EdictLevel::High,
                    ..Default::default()
                },
            );
            let hard_labor_change = ownership.population_change_for(0, 100.0, 0);
            assert!((hard_labor_change - (expected_growth - 1.5)).abs() < 1e-9);
            ownership.advance_population(0, hard_labor_change);
            let expected_slave_growth =
                300.0 * (ownership.governance_for(0).slave_population_growth_rate());
            assert!(
                (ownership.populations[province].slaves - 300.0 - expected_slave_growth).abs()
                    < 1e-9
            );
        }
    }

    #[test]
    fn rotated_label_fits_a_narrow_province() {
        let province = Province {
            name: "Narrow".into(),
            short: "Narrow".into(),
            label: [0.5, 2.0],
            bounds: [0.0, 0.0, 1.0, 4.0],
            terrain: Vec::new(),
            parts: vec![MapMesh {
                v: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 4.0], [0.0, 4.0]],
                r: vec![4],
                t: Vec::new(),
                bounds: [0.0, 0.0, 1.0, 4.0],
            }],
            visual_center: [0.5, 2.0],
            production: [0; 3],
        };
        let projection = Projection {
            origin: egui::Pos2::ZERO,
            scale: 40.0,
            center: [0.5, 2.0],
        };
        let text_size = egui::vec2(50.0, 12.0);
        assert!(!label_fits_province(&province, &projection, province.label, text_size, 0.0));
        assert!(label_fits_province(
            &province,
            &projection,
            province.label,
            text_size,
            std::f32::consts::FRAC_PI_2,
        ));
    }

    #[test]
    fn label_candidates_leave_room_and_offer_marker_fallbacks() {
        let province = Province {
            name: "Tarraconensis".into(),
            short: "Tarraconensis".into(),
            label: [10.0, 5.0],
            bounds: [0.0, 0.0, 20.0, 10.0],
            terrain: Vec::new(),
            parts: vec![MapMesh {
                v: vec![[0.0, 0.0], [20.0, 0.0], [20.0, 10.0], [0.0, 10.0]],
                r: vec![4],
                t: vec![0, 1, 2, 0, 2, 3],
                bounds: [0.0, 0.0, 20.0, 10.0],
            }],
            visual_center: [10.0, 5.0],
            production: [0; 3],
        };
        let projection = Projection {
            origin: egui::Pos2::ZERO,
            scale: 10.0,
            center: [10.0, 5.0],
        };
        let context = egui::Context::default();
        context.begin_pass(Default::default());
        let painter = context.layer_painter(egui::LayerId::background());
        let candidates = label_candidates(&painter, &province, &projection, 2.0);
        assert!(candidates.len() > 1);
        let first = &candidates[0];
        assert_eq!(first.center, province.visual_center);
        let alternate = candidates.iter().find(|candidate| {
            candidate.center != first.center && candidate.font_size == first.font_size
        });
        assert!(alternate.is_some(), "a blocked center must have a readable fallback");
        let text_size = painter
            .layout_no_wrap(province.name.clone(), egui::FontId::proportional(first.font_size), INK)
            .size();
        assert!(label_fit_size(text_size).x >= text_size.x * 1.18);
        for zoom in [1.0, 4.0] {
            let fitted = anchored_label_candidates(&painter, &province, &projection, zoom, first);
            assert!(!fitted.is_empty());
            assert!(fitted.iter().all(|candidate| {
                candidate.center == first.center && candidate.angle == first.angle
            }));
        }
        let mut output = context.end_pass();
        output.textures_delta.clear();
    }

    #[test]
    fn achaia_keeps_its_anchor_across_zoom_levels() {
        let province = atlas().provinces.iter().find(|province| province.name == "Achaia").unwrap();
        let context = egui::Context::default();
        context.begin_pass(Default::default());
        let painter = context.layer_painter(egui::LayerId::background());
        let initial_projection = Projection {
            origin: egui::Pos2::ZERO,
            scale: 40.0,
            center: province.label,
        };
        let overview_projection = Projection {
            origin: egui::Pos2::ZERO,
            scale: 13.0 * 1.5,
            center: province.label,
        };
        let prepared = vec![
            vec![label_candidates(&painter, province, &overview_projection, 1.5)],
            vec![label_candidates(&painter, province, &initial_projection, MAX_ZOOM)],
        ];
        assert_ne!(prepared[0][0][0].angle, 0.0);
        let anchor = stable_label_anchors(&prepared, 1)
            .remove(0)
            .expect("Achaia should have a label placement");
        assert_eq!(anchor.angle, 0.0);
        for zoom in [1.5, 2.5, 3.7, 5.0, 8.0] {
            let projection = Projection {
                origin: egui::Pos2::ZERO,
                scale: 13.0 * zoom,
                center: province.label,
            };
            let anchored =
                anchored_label_candidates(&painter, province, &projection, zoom, &anchor);
            let regular = label_candidates(&painter, province, &projection, zoom);
            assert!(label_choice_order(&anchored, &regular, true, false).iter().all(
                |(candidate, relocated)| {
                    !relocated
                        && candidate.center == anchor.center
                        && candidate.angle == anchor.angle
                }
            ));
        }
        let mut output = context.end_pass();
        output.textures_delta.clear();
    }

    #[test]
    fn tarraconensis_has_readable_placements_away_from_its_marker() {
        let province =
            atlas().provinces.iter().find(|province| province.name == "Tarraconensis").unwrap();
        let projection = Projection {
            origin: egui::Pos2::ZERO,
            scale: 48.0,
            center: province.label,
        };
        let context = egui::Context::default();
        context.begin_pass(Default::default());
        let painter = context.layer_painter(egui::LayerId::background());
        let candidates = label_candidates(&painter, province, &projection, 3.7);
        let preferred_size = candidates[0].font_size;
        let marker = egui::Rect::from_center_size(
            projection.point([-4.117, 40.948]),
            egui::vec2(42.0, 42.0),
        );
        assert!(candidates.iter().any(|candidate| {
            let galley = painter.layout_no_wrap(
                province.name.clone(),
                egui::FontId::proportional(candidate.font_size),
                INK,
            );
            let bounds =
                rotated_bounds(projection.point(candidate.center), galley.size(), candidate.angle);
            candidate.full_name
                && candidate.font_size == preferred_size
                && !marker.intersects(bounds)
        }));
        let anchor = LabelPlacement {
            center: province.label,
            angle: 0.0,
            font_size: preferred_size,
            full_name: true,
        };
        let anchored = anchored_label_candidates(&painter, province, &projection, 3.7, &anchor);
        let blocked_center =
            egui::Rect::from_center_size(projection.point(anchor.center), egui::vec2(42.0, 42.0));
        let readable_floor = readable_label_floor(&candidates);
        assert!(marker_blocks_readable_anchor(
            &painter,
            province,
            &projection,
            &anchored,
            &candidates,
            &[blocked_center],
        ));
        let (selected, relocated) = label_choice_order(&anchored, &candidates, true, true)
            .into_iter()
            .find(|(candidate, _)| {
                let name = if candidate.full_name {
                    &province.name
                } else {
                    &province.short
                };
                let galley = painter.layout_no_wrap(
                    name.clone(),
                    egui::FontId::proportional(candidate.font_size),
                    INK,
                );
                let bounds = rotated_bounds(
                    projection.point(candidate.center),
                    galley.size(),
                    candidate.angle,
                );
                !blocked_center.intersects(bounds)
            })
            .expect("a blocked Tarraconensis label needs a fallback");
        assert!(relocated);
        assert!(selected.font_size >= readable_floor);
        assert!(label_choice_order(&anchored, &candidates, true, false).iter().all(
            |(candidate, relocated)| {
                !relocated && candidate.center == anchor.center && candidate.angle == anchor.angle
            }
        ));
        let mut output = context.end_pass();
        output.textures_delta.clear();
    }

    #[test]
    fn rhodes_wonder_anchor_is_on_the_island() {
        let rhodes = WONDERS.iter().find(|wonder| wonder.name == "Colossus of Rhodes").unwrap();
        let province = atlas()
            .provinces
            .iter()
            .find(|province| province.contains(rhodes.position))
            .expect("the Colossus should be on Rhodes, within Asia's island geometry");
        assert_eq!(province.name, "Asia");
        assert!(atlas().land.iter().any(|part| part.contains(rhodes.position)));
    }

    #[test]
    fn mismatched_coastal_backdrop_does_not_tint_sea() {
        for point in [[-6.235, 37.013], [-1.273, 44.175], [4.521, 51.682]] {
            assert!(
                !atlas().land.iter().any(|part| part.contains(point)),
                "coastal water at {point:?} should have the sea color"
            );
        }
        assert!(atlas().land.iter().any(|part| part.contains([10.0, 60.0])));
    }

    #[test]
    fn colossus_grows_with_zoom_without_leaving_rhodes() {
        let colossus =
            WONDERS.iter().position(|wonder| wonder.name == "Colossus of Rhodes").unwrap();
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 545.0));
        let marker_at_zoom = |zoom| {
            let projection = Projection {
                origin: rect.center(),
                scale: 14.7 * zoom,
                center: WONDERS[colossus].position,
            };
            let marker = layout_wonders(&projection, rect, zoom)
                .into_iter()
                .find(|marker| marker.index == colossus)
                .expect("the Colossus should appear on Rhodes at close zoom");
            let image = marker.image.expect("the close view should show the Colossus art");
            assert_eq!(image.center(), projection.point(WONDERS[colossus].position));
            image.width()
        };
        assert!(marker_at_zoom(8.0) > marker_at_zoom(3.6));
    }

    #[test]
    fn overview_wonders_stay_at_their_sites() {
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 545.0));
        let projection = Projection {
            origin: rect.center(),
            scale: 12.0,
            center: [20.0, 40.0],
        };
        let wonders = layout_wonders(&projection, rect, MIN_ZOOM);
        assert_eq!(wonders.len(), WONDERS.len());
        assert_eq!(city_blend(MIN_ZOOM), 0.0);
        for wonder in &wonders {
            let site = projection.point(WONDERS[wonder.index].position);
            assert!(wonder.icon.center().distance(site) < 0.001);
        }
    }

    #[test]
    fn every_wonder_has_illustration_at_close_zoom() {
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 545.0));
        for zoom in [CITY_BLEND_END, 4.0, MAX_ZOOM] {
            for (index, wonder) in WONDERS.iter().enumerate() {
                let projection = Projection {
                    origin: rect.center(),
                    scale: 14.7 * zoom,
                    center: wonder.position,
                };
                let marker = layout_wonders(&projection, rect, zoom)
                    .into_iter()
                    .find(|marker| marker.index == index)
                    .expect("the wonder site should remain on screen");
                let image = marker.image.unwrap_or_else(|| {
                    panic!("{} needs its illustration at zoom {zoom}", wonder.name)
                });
                let site = projection.point(wonder.position);
                assert!(marker.icon.center().distance(site) < 0.001, "{} icon moved", wonder.name);
                assert!(image.center().distance(site) < 0.001, "{} art moved", wonder.name);
                assert!(
                    (image.height() - (18.0 * zoom).min(110.0)).abs() < 0.001,
                    "{} art is smaller than the Colossus at zoom {zoom}",
                    wonder.name,
                );
            }
        }
    }

    #[test]
    fn wonders_do_not_share_city_sites() {
        for wonder in &WONDERS {
            for city in &CITIES {
                assert!(
                    (wonder.position[0] - city.position[0]).abs() >= 0.05
                        || (wonder.position[1] - city.position[1]).abs() >= 0.05,
                    "{} overlaps a city site",
                    wonder.name,
                );
            }
        }
    }

    #[test]
    fn new_wonders_are_on_land_and_cities_stay_at_sites() {
        for name in ["Aqueduct of Segovia", "Pont du Gard"] {
            let wonder = WONDERS.iter().find(|wonder| wonder.name == name).unwrap();
            assert!(
                atlas().land.iter().any(|part| part.contains(wonder.position)),
                "{} must be on the map's land geometry",
                wonder.name,
            );
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 545.0));
            let projection = Projection {
                origin: rect.center(),
                scale: 14.7 * 8.0,
                center: wonder.position,
            };
            assert!(
                layout_wonders(&projection, rect, 8.0)
                    .iter()
                    .any(|marker| WONDERS[marker.index].name == wonder.name
                        && marker.image.is_some()),
                "{} should be visible at close zoom",
                wonder.name,
            );
        }
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 545.0));
        let projection = Projection {
            origin: rect.center(),
            scale: 12.0,
            center: [20.0, 40.0],
        };
        let overview = layout_cities(&projection, rect, MIN_ZOOM);
        let close = layout_cities(&projection, rect, MAX_ZOOM);
        assert_eq!(overview.len(), CITIES.len());
        assert_eq!(close.len(), CITIES.len());
        for ((city, icon), illustration) in CITIES.iter().zip(&overview).zip(&close) {
            let anchor = projection.point(city.position);
            for image in [icon.icon, illustration.image] {
                let hotspot = image.min
                    + egui::vec2(image.width() * city.hotspot[0], image.height() * city.hotspot[1]);
                assert!(hotspot.distance(anchor) < 0.001);
            }
            assert!(illustration.image.width() > icon.image.width());
        }
        assert!(overview[0].is_rome && close[0].is_rome);
        assert!(overview[0].icon.width() > overview[1].icon.width());
        assert!(close[0].image.width() > close[1].image.width());
        assert_eq!(city_blend(MIN_ZOOM), 0.0);
        assert_eq!(city_blend(MAX_ZOOM), 1.0);
    }
}
