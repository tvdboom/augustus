//! Historical atlas, terrain, map art, and campaign presentation adapter.

mod crossings;
mod governance;
mod marker_icon;
mod population;
mod production;
mod terrain;
mod terrain_type;
#[path = "map.rs"]
mod view;
mod water;
pub(crate) use governance::{EdictLevel, Governance};
pub(crate) use marker_icon::{paint_marker_icon, MarkerIcon};
pub(crate) use view::city_name_for_province;
pub(crate) use view::{draw_map, military_unit_icon, MapView, ProvinceOverview, ProvinceOwnership};
pub(crate) use view::{wonder_image, wonder_name, WONDER_COUNT};
