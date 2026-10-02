//! Historical atlas, terrain, map art, and campaign presentation adapter.

mod crossings;
mod governance;
mod marker_icon;
mod population;
pub(crate) use population::POPULATION_SCALE;
mod production;
mod terrain;
mod terrain_type;
#[path = "map.rs"]
mod view;
mod water;
mod wonder_frames;
pub(crate) use governance::{EdictLevel, Governance};
pub(crate) use marker_icon::{paint_marker_icon, MarkerIcon};
pub(crate) use view::city_name_for_province;
pub(crate) use view::take_army_click;
pub(crate) use view::{audible_battles, AudibleBattle};
pub(crate) use view::{
    draw_map, military_unit_icon, movement_visual_progress, set_army_order_selection, MapView,
    ProvinceOverview, ProvinceOwnership,
};
pub(crate) use view::{take_battle_click, take_province_order_click};
pub(crate) use view::{wonder_image, wonder_name, WONDER_COUNT};
