//! Shared illustrated widgets reuse the existing province/city artwork and typography.

use super::province_panel;
use crate::game::{economy::BuildingType, military::UnitType};
use bevy_egui::egui;

/// Set map popup typography before any map UI; restore the original menu style on exit.
pub(in crate::app) fn configure_style(
    mut contexts: bevy_egui::EguiContexts,
    state: bevy::prelude::Res<bevy::prelude::State<super::AppState>>,
    mut previous: bevy::prelude::Local<Option<(bool, u32)>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let map = matches!(*state.get(), super::AppState::Map | super::AppState::EmptyScreen);
    let scale = super::viewport_ui_scale(ctx.content_rect().size());
    let key = (map, scale.to_bits());
    if *previous != Some(key) {
        ctx.set_global_style(if map {
            map_style(scale)
        } else {
            super::augustus_ui_style()
        });
        *previous = Some(key);
    }
}

/// Semantic artwork shared by tables, action controls and compact statistics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::app) enum Icon {
    Food,
    Metal,
    Stone,
    Coin,
    Influence,
    Population,
    Happiness,
    Nobles,
    Citizens,
    Plebeians,
    Slaves,
    Spy,
    Trade,
    Control,
    Attack,
    Province,
    Morale,
    MilitaryPower,
    Eagle,
    Building(BuildingType),
    Unit(UnitType),
    Wonder(usize),
}

/// Cache prepared icons by semantic identity; no file IO or resampling on hover.
fn texture(ctx: &egui::Context, kind: Icon) -> egui::TextureId {
    if let Icon::Unit(unit) = kind {
        return crate::map::military_unit_icon(ctx, unit);
    }
    let key = egui::Id::new(("campaign-illustration", kind));
    if let Some(handle) = ctx.data(|data| data.get_temp::<egui::TextureHandle>(key)) {
        return handle.id();
    }
    macro_rules! prepared {
        ($name:literal) => {
            include_bytes!(concat!(env!("OUT_DIR"), "/panel-icons/", $name, ".png")) as &[u8]
        };
    }
    let bytes = match kind {
        Icon::Food => prepared!("food"),
        Icon::Metal => prepared!("metal"),
        Icon::Stone => prepared!("stone"),
        Icon::Coin => prepared!("coin"),
        Icon::Influence => prepared!("influence"),
        Icon::Population => prepared!("population"),
        Icon::Happiness => prepared!("happiness"),
        Icon::Nobles => prepared!("nobles"),
        Icon::Citizens => prepared!("civilians"),
        Icon::Plebeians => prepared!("plebeians"),
        Icon::Slaves => prepared!("slaves"),
        Icon::Spy => prepared!("spy"),
        Icon::Trade => prepared!("trade"),
        Icon::Control => prepared!("court-nobles"),
        Icon::Attack => prepared!("attack"),
        Icon::Province => prepared!("province"),
        Icon::Morale => prepared!("morale"),
        Icon::MilitaryPower => prepared!("military-power"),
        Icon::Eagle => prepared!("spqr-eagle-gold"),
        Icon::Building(building) => match building {
            BuildingType::Granary | BuildingType::Warehouse | BuildingType::Farm => {
                prepared!("granary")
            },
            BuildingType::Aqueduct | BuildingType::Baths | BuildingType::Road => {
                prepared!("aqueduct")
            },
            BuildingType::Mine
            | BuildingType::Quarry
            | BuildingType::Armory
            | BuildingType::StoneYard => prepared!("foundry"),
            BuildingType::Fort | BuildingType::CityWalls => prepared!("province"),
            BuildingType::Forum => prepared!("forum"),
            BuildingType::Temple => prepared!("great-temple"),
            BuildingType::Arena => prepared!("grand-theater"),
            BuildingType::UrbanMarket => prepared!("marketplace"),
        },
        Icon::Wonder(id) => crate::map::wonder_image(id).unwrap_or(prepared!("great-temple")),
        Icon::Unit(_) => unreachable!(),
    };
    let image = image::load_from_memory(bytes).expect("valid campaign illustration").to_rgba8();
    let handle = ctx.load_texture(
        format!("campaign-{kind:?}"),
        egui::ColorImage::from_rgba_unmultiplied(
            [image.width() as usize, image.height() as usize],
            image.as_raw(),
        ),
        egui::TextureOptions::LINEAR,
    );
    let id = handle.id();
    ctx.data_mut(|data| data.insert_temp(key, handle));
    id
}

/// Draw an image at the requested logical size; unit icons select the first atlas cell.
pub(in crate::app) fn icon(ui: &mut egui::Ui, kind: Icon, size: f32) -> egui::Response {
    let uv = if matches!(kind, Icon::Unit(_)) {
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(0.25, 0.25))
    } else {
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0))
    };
    ui.add(egui::Image::new((texture(ui.ctx(), kind), egui::vec2(size, size))).uv(uv))
}

/// A compact icon-value pair with the complete explanation on hover.
pub(in crate::app) fn stat(
    ui: &mut egui::Ui,
    kind: Icon,
    value: &str,
    tooltip: &str,
) -> egui::Response {
    ui.horizontal(|ui| {
        icon(ui, kind, 20.0);
        ui.label(value);
    })
    .response
    .on_hover_text(tooltip)
}

/// Existing landscape artwork anchors the overview visually; city views use the city banner.
pub(in crate::app) fn portrait(
    ui: &mut egui::Ui,
    terrain: crate::game::economy::Terrain,
    city: bool,
    height: f32,
) {
    use crate::game::economy::Terrain;
    let key = egui::Id::new(("campaign-landscape", terrain as usize, city));
    let image = ui.ctx().data(|d| d.get_temp::<egui::TextureHandle>(key)).unwrap_or_else(|| {
        let (name, bytes): (&str, &[u8]) = if city {
            ("city", include_bytes!("../../assets/images/cities/city-panel-banner.png"))
        } else {
            match terrain {
                Terrain::Desert => {
                    ("desert", include_bytes!("../../assets/images/map/terrain/desert.png"))
                },
                Terrain::Farmland => {
                    ("farmland", include_bytes!("../../assets/images/map/terrain/farmland.png"))
                },
                Terrain::Forest => {
                    ("forest", include_bytes!("../../assets/images/map/terrain/forest.png"))
                },
                Terrain::Hills => {
                    ("hills", include_bytes!("../../assets/images/map/terrain/hills.png"))
                },
                Terrain::Mountains => {
                    ("mountain", include_bytes!("../../assets/images/map/terrain/mountain.png"))
                },
                Terrain::Marsh => {
                    ("marsh", include_bytes!("../../assets/images/map/terrain/marsh.png"))
                },
                Terrain::Plains => {
                    ("plains", include_bytes!("../../assets/images/map/terrain/plains.png"))
                },
            }
        };
        let handle = province_panel::load_image(ui.ctx(), (name, bytes), "campaign-portrait");
        ui.ctx().data_mut(|d| d.insert_temp(key, handle.clone()));
        handle
    });
    let rect =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), height), egui::Sense::hover()).0;
    let aspect = image.size()[0] as f32 / image.size()[1] as f32;
    let target = rect.width() / rect.height();
    let (x, y) = if aspect > target {
        ((1.0 - target / aspect) * 0.5, 0.0)
    } else {
        (0.0, (1.0 - aspect / target) * 0.5)
    };
    ui.painter().image(
        image.id(),
        rect,
        egui::Rect::from_min_max(egui::pos2(x, y), egui::pos2(1.0 - x, 1.0 - y)),
        egui::Color32::WHITE,
    );
    ui.painter().rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0, province_panel::RULE),
        egui::StrokeKind::Inside,
    );
}

/// The map's global popup style must match its compact widgets, not the main menu's 23px body.
pub(in crate::app) fn map_style(scale: f32) -> egui::Style {
    let mut style = super::augustus_ui_style();
    style.text_styles = [
        (egui::TextStyle::Small, 12.0),
        (egui::TextStyle::Body, 14.0),
        (egui::TextStyle::Button, 14.0),
        (egui::TextStyle::Heading, 20.0),
        (egui::TextStyle::Monospace, 13.0),
    ]
    .into_iter()
    .map(|(style, size)| (style, egui::FontId::proportional(size * scale)))
    .collect();
    style.spacing.item_spacing = egui::vec2(7.0, 5.0) * scale;
    style.spacing.button_padding = egui::vec2(8.0, 4.0) * scale;
    style.spacing.menu_margin = egui::Margin::same(8);
    style.spacing.tooltip_width = 340.0 * scale;
    style.spacing.interact_size.y = 25.0 * scale;
    style.visuals.override_text_color = Some(province_panel::INK);
    style.visuals.window_fill = province_panel::PAPER;
    style.visuals.panel_fill = province_panel::PAPER;
    style.visuals.extreme_bg_color = province_panel::TABLE_STRIPE;
    style.visuals.faint_bg_color = province_panel::TABLE_STRIPE;
    style.visuals.window_stroke = egui::Stroke::new(1.0, province_panel::RULE);
    style.visuals.selection.bg_fill = egui::Color32::from_rgb(202, 177, 137);
    style.visuals.selection.stroke.color = province_panel::INK;
    for widget in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.noninteractive,
    ] {
        widget.fg_stroke.color = province_panel::INK;
        widget.bg_stroke = egui::Stroke::new(1.0, province_panel::RULE);
    }
    style.visuals.widgets.inactive.weak_bg_fill = province_panel::TABLE_STRIPE;
    style.visuals.widgets.inactive.bg_fill = province_panel::TABLE_STRIPE;
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(226, 210, 180);
    style.visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(226, 210, 180);
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(195, 145, 87);
    style
}
