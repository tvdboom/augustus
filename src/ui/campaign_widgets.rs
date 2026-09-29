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
    if previous.is_none() {
        configure_cursor(ctx);
    }
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

pub(super) fn configure_cursor(ctx: &egui::Context) {
    // egui retains the output cursor between passes. Start fresh so leaving a
    // clickable control restores the arrow even over a noninteractive panel.
    ctx.on_begin_pass(
        "augustus-reset-cursor",
        std::sync::Arc::new(|ui| ui.ctx().set_cursor_icon(egui::CursorIcon::Default)),
    );
    ctx.on_end_pass("augustus-click-cursor", std::sync::Arc::new(|ui| click_cursor(ui.ctx())));
}

/// Cover standard widgets and custom click targets while preserving editing and drag cursors.
fn click_cursor(ctx: &egui::Context) {
    if ctx.output(|output| output.cursor_icon) != egui::CursorIcon::Default {
        return;
    }
    let hovered =
        ctx.interaction_snapshot(|snapshot| snapshot.hovered.iter().copied().collect::<Vec<_>>());
    if hovered.into_iter().any(|id| {
        ctx.read_response(id).is_some_and(|response| {
            response.enabled()
                && response.hovered()
                && response.sense.senses_click()
                && !response.sense.senses_drag()
        })
    }) {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
}

#[cfg(test)]
#[path = "../../tests/unit/cursor_ui.rs"]
mod cursor_tests;

/// Currency unit for an amount, including fractional amounts.
pub(super) fn sestertius_unit(amount: f64) -> &'static str {
    if amount.abs() == 1.0 {
        "sestertius"
    } else {
        "sestertii"
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
    Amount,
    BaseAmount,
    Delta,
    Change,
    Notifications,
    Cancel,
    Notice,
    Confirm,
    Nobles,
    Citizens,
    Plebeians,
    Slaves,
    Spy,
    SpyBuildControl,
    SpyImproveRelations,
    SpyUncoverScandals,
    SpyUndermineOpponents,
    Trade,
    Control,
    Relation,
    Diplomacy,
    PoliticalDistance,
    Vassalize,
    Integrate,
    Policies,
    Construction,
    Recruitment,
    Attack,
    Offense,
    Defense,
    Speed,
    Maneuver,
    Orders,
    Province,
    Morale,
    Terrain,
    MilitaryPower,
    MilitaryAccess,
    Eagle,
    Duration,
    MilitaryRank(crate::game::military::MilitaryRank),
    Building(BuildingType),
    Unit(UnitType),
    Wonder(usize),
}

/// Cache prepared icons by semantic identity; no file IO or resampling on hover.
pub(super) fn texture(ctx: &egui::Context, kind: Icon) -> egui::TextureId {
    if let Icon::Unit(unit) = kind {
        if !matches!(unit, UnitType::Catapult | UnitType::WarChariots) {
            return crate::map::military_unit_icon(ctx, unit);
        }
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
    macro_rules! building_art {
        ($name:literal) => {
            include_bytes!(concat!(env!("OUT_DIR"), "/building-icons/", $name, ".png")) as &[u8]
        };
    }
    let bytes = match kind {
        Icon::PoliticalDistance | Icon::Vassalize | Icon::Integrate => {
            unreachable!("vector diplomacy symbols are painted directly")
        },
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
        Icon::SpyBuildControl => prepared!("spy-build-control"),
        Icon::SpyImproveRelations => prepared!("spy-improve-relations"),
        Icon::SpyUncoverScandals => prepared!("spy-uncover-scandals"),
        Icon::SpyUndermineOpponents => prepared!("spy-undermine-opponents"),
        Icon::Trade => prepared!("trade"),
        Icon::Control => prepared!("control"),
        Icon::Relation => prepared!("relation"),
        Icon::Diplomacy => prepared!("diplomacy"),
        Icon::Policies => prepared!("policies"),
        Icon::Construction => prepared!("construction"),
        Icon::Recruitment => prepared!("recruitment"),
        Icon::MilitaryAccess => prepared!("military-access"),
        Icon::Unit(UnitType::Catapult) => prepared!("catapult"),
        Icon::Unit(UnitType::WarChariots) => prepared!("war-chariots"),
        Icon::Amount => prepared!("amount"),
        Icon::BaseAmount => prepared!("base-amount"),
        Icon::Delta => prepared!("delta"),
        Icon::Change => prepared!("change"),
        Icon::Notifications => prepared!("notifications"),
        Icon::Cancel => prepared!("cancel"),
        Icon::Notice => prepared!("notice"),
        Icon::Confirm => prepared!("confirm"),
        Icon::Attack => prepared!("attack"),
        Icon::Offense => prepared!("offense"),
        Icon::Defense => prepared!("defense"),
        Icon::Speed => prepared!("speed"),
        Icon::Maneuver => prepared!("maneuver"),
        Icon::Orders => prepared!("orders"),
        Icon::Duration => prepared!("recruitment-time"),
        Icon::Province => prepared!("province"),
        Icon::Morale => prepared!("morale"),
        Icon::Terrain => prepared!("terrain"),
        Icon::MilitaryPower => prepared!("military-power"),
        Icon::Eagle => prepared!("spqr-eagle-gold"),
        Icon::Building(building) => match building {
            BuildingType::Granary => building_art!("granary-rural"),
            BuildingType::Warehouse => building_art!("granary"),
            BuildingType::Aqueduct => building_art!("aqueduct"),
            BuildingType::Baths => building_art!("baths"),
            BuildingType::Road => building_art!("road"),
            BuildingType::Foundry => building_art!("foundry"),
            BuildingType::Academy => building_art!("academy"),
            BuildingType::CityWalls => building_art!("walls"),
            BuildingType::Forum => building_art!("forum"),
            BuildingType::Temple => building_art!("great-temple"),
            BuildingType::Arena => building_art!("grand-theater"),
            BuildingType::UrbanMarket => building_art!("marketplace"),
        },
        Icon::Wonder(id) => crate::map::wonder_image(id).unwrap_or(prepared!("great-temple")),
        Icon::Unit(_) | Icon::MilitaryRank(_) => {
            unreachable!()
        },
    };
    let mut image = image::load_from_memory(bytes).expect("valid campaign illustration").to_rgba8();
    if matches!(kind, Icon::Wonder(_)) {
        // Map artwork includes transparent margins. Center the visible wonder in
        // a square so it fills the same icon area as buildings without stretching.
        let (width, height) = image.dimensions();
        let (mut left, mut top, mut right, mut bottom) = (width, height, 0, 0);
        for (x, y, pixel) in image.enumerate_pixels() {
            if pixel[3] > 0 {
                left = left.min(x);
                top = top.min(y);
                right = right.max(x + 1);
                bottom = bottom.max(y + 1);
            }
        }
        if right > left && bottom > top {
            let art =
                image::imageops::crop_imm(&image, left, top, right - left, bottom - top).to_image();
            let side = art.width().max(art.height());
            let padding = side.div_ceil(50);
            image = image::RgbaImage::new(side + 2 * padding, side + 2 * padding);
            image::imageops::overlay(
                &mut image,
                &art,
                i64::from(padding + (side - art.width()) / 2),
                i64::from(padding + (side - art.height()) / 2),
            );
        }
    }
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

/// Draw semantic artwork; catapults and chariots use standalone art, other units use sprites.
pub(in crate::app) fn icon(ui: &mut egui::Ui, kind: Icon, size: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    paint_icon(ui, kind, rect);
    response
}

/// A searchable panel title option. The label may distinguish armies sharing a province.
pub(super) struct TitleSearchOption<'a> {
    pub name: &'a str,
    pub label: String,
    pub selected: bool,
}

/// Use the same centered title field and aligned dropdown in province and army panels.
pub(super) fn title_search_selector(
    ui: &mut egui::Ui,
    size: egui::Vec2,
    ink: egui::Color32,
    scale: f32,
    selected_name: &str,
    search_id: egui::Id,
    open: &mut bool,
    query: &mut String,
    options: &[TitleSearchOption<'_>],
    empty_message: &str,
) -> Option<usize> {
    let editing = *open;
    let response = ui
        .scope(|ui| {
            ui.visuals_mut().weak_text_color = Some(if editing {
                ink.gamma_multiply(0.5)
            } else {
                ink
            });
            ui.visuals_mut().text_cursor.stroke.color = ink;
            ui.add_sized(
                size,
                egui::TextEdit::singleline(query)
                    .id(search_id)
                    .frame(
                        egui::Frame::new()
                            .fill(egui::Color32::from_black_alpha(if editing {
                                18
                            } else {
                                8
                            }))
                            .stroke(egui::Stroke::new(scale, egui::Color32::TRANSPARENT))
                            .corner_radius(3.0 * scale)
                            .inner_margin(egui::vec2(8.0 * scale, 0.0)),
                    )
                    .min_size(size)
                    .horizontal_align(egui::Align::Center)
                    .vertical_align(egui::Align::Center)
                    .font(egui::FontId::proportional(20.0 * scale))
                    .text_color(ink)
                    .hint_text(egui::RichText::new(selected_name).size(20.0 * scale)),
            )
        })
        .inner;
    if (response.clicked() || response.gained_focus()) && !*open {
        *open = true;
        response.request_focus();
    }
    let needle = query.trim().to_lowercase();
    let mut matches: Vec<_> = options
        .iter()
        .enumerate()
        .filter(|(_, option)| option.name.to_lowercase().contains(&needle))
        .collect();
    matches.sort_unstable_by(|(_, a), (_, b)| a.name.cmp(b.name).then(a.label.cmp(&b.label)));
    let mut selected = None;
    let popup_frame = egui::Frame::popup(ui.style()).inner_margin(4.0 * scale);
    let dropdown = egui::Popup::from_response(&response)
        .id(search_id.with("dropdown"))
        .open_bool(open)
        .close_behavior(if response.contains_pointer() {
            egui::PopupCloseBehavior::IgnoreClicks
        } else {
            egui::PopupCloseBehavior::CloseOnClickOutside
        })
        .align(egui::RectAlign::BOTTOM_START)
        .align_alternatives(&[])
        .gap(0.0)
        .width(response.rect.width())
        .frame(popup_frame)
        .show(|ui| {
            ui.set_width((response.rect.width() - popup_frame.total_margin().sum().x).max(1.0));
            if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
                selected = matches.first().map(|(index, _)| *index);
            }
            egui::ScrollArea::vertical()
                .id_salt("title-search-results")
                .max_height((ui.ctx().content_rect().height() * 0.4).min(280.0 * scale))
                .show(ui, |ui| {
                    if matches.is_empty() {
                        ui.label(empty_message);
                    }
                    for (index, option) in matches {
                        if ui
                            .add(
                                egui::Button::selectable(option.selected, ())
                                    .left_text(option.label.as_str())
                                    .truncate()
                                    .min_size(egui::vec2(ui.available_width(), 28.0 * scale)),
                            )
                            .clicked()
                        {
                            selected = Some(index);
                        }
                    }
                });
        });
    if let Some(dropdown) = dropdown {
        let popup_layer = dropdown.response.layer_id;
        if response.layer_id.order == popup_layer.order {
            ui.ctx().set_sublayer(response.layer_id, popup_layer);
        }
    }
    if selected.is_some() {
        *open = false;
    }
    if !*open {
        query.clear();
        ui.memory_mut(|memory| memory.surrender_focus(search_id));
    }
    if response.hovered() || response.has_focus() {
        ui.painter().rect_stroke(
            response.rect,
            3.0 * scale,
            egui::Stroke::new(scale, ink.gamma_multiply(0.7)),
            egui::StrokeKind::Inside,
        );
    }
    selected
}

/// Keep unavailable purchases warm; diagonal lines distinguish them from actionable cards.
pub(in crate::app) const UNAVAILABLE_PURCHASE_FILL: egui::Color32 = province_panel::TABLE_STRIPE;
/// Muted labels distinguish unavailable purchases while preserving contrast.
pub(in crate::app) const UNAVAILABLE_PURCHASE_INK: egui::Color32 =
    egui::Color32::from_rgb(108, 105, 96);

/// Paint a shared purchase state without blocking availability tooltips.
pub(in crate::app) fn paint_purchase_background(
    ui: &egui::Ui,
    rect: egui::Rect,
    enabled: bool,
    hovered: bool,
    pressed: bool,
    rounding: f32,
    scale: f32,
) -> egui::Color32 {
    let fill = if !enabled {
        UNAVAILABLE_PURCHASE_FILL
    } else if pressed {
        egui::Color32::from_rgb(207, 180, 137)
    } else if hovered {
        egui::Color32::from_rgb(224, 210, 181)
    } else {
        province_panel::TABLE_STRIPE
    };
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    painter.rect_filled(rect, rounding, fill);
    if !enabled {
        let bounds = rect.shrink(3.0 * scale);
        let mut offset = 0.0;
        while offset < bounds.width() + bounds.height() {
            let start = bounds.min
                + egui::vec2(offset.min(bounds.width()), (offset - bounds.width()).max(0.0));
            let end = bounds.min
                + egui::vec2((offset - bounds.height()).max(0.0), offset.min(bounds.height()));
            painter.line_segment(
                [start, end],
                egui::Stroke::new(scale, egui::Color32::from_rgba_unmultiplied(108, 105, 96, 55)),
            );
            offset += (10.0 * scale).max(4.0);
        }
    }
    fill
}

/// Cancellation distinguishes active work from refundable waiting orders.
#[derive(Clone, Copy)]
pub(in crate::app) enum WorkQueueAction {
    /// Active work has started, so its payment is lost.
    CancelActive,
    /// Only this waiting order is removed, with a full refund.
    CancelQueued(usize),
}

/// Shared compact work strip: active icon, percentage, cancellation, then waiting icons.
pub(in crate::app) fn work_queue(
    ui: &mut egui::Ui,
    id: egui::Id,
    title: &str,
    active: Option<(Icon, f32, String)>,
    queued: &[(usize, Icon, String)],
    can_cancel: bool,
    scale: f32,
) -> Option<WorkQueueAction> {
    use province_panel::{INK, RULE};
    let label_ink = ui.visuals().override_text_color.unwrap_or(INK);
    let mut action = None;
    ui.push_id(id, |ui| {
        let icon_size = 28.0 * scale;
        let label_font = egui::TextStyle::Small.resolve(ui.style());
        let label_width = [title, "Queue"]
            .iter()
            .map(|text| {
                ui.painter()
                    .layout_no_wrap((*text).to_owned(), label_font.clone(), label_ink)
                    .size()
                    .x
            })
            .fold(0.0_f32, f32::max);
        let label = |ui: &mut egui::Ui, text: &str| {
            let (rect, _) =
                ui.allocate_exact_size(egui::vec2(label_width, icon_size), egui::Sense::hover());
            ui.painter().text(
                rect.left_center(),
                egui::Align2::LEFT_CENTER,
                text,
                label_font.clone(),
                label_ink,
            );
            // Scroll areas round their origin to pixels; keep both icon rows on that grid.
            let pixels_per_point = ui.ctx().pixels_per_point();
            let next_x = ui.next_widget_position().x;
            ui.add_space((next_x * pixels_per_point).ceil() / pixels_per_point - next_x);
        };
        if let Some((art, progress, tooltip)) = active {
            ui.horizontal(|ui| {
                label(ui, title);
                let (image, response) =
                    ui.allocate_exact_size(egui::Vec2::splat(icon_size), egui::Sense::hover());
                paint_icon(ui, art, image);
                response.on_hover_text(tooltip);
                let gap = ui.spacing().item_spacing.x;
                let width = (ui.available_width() - 22.0 * scale - gap).max(1.0);
                let (track, _) =
                    ui.allocate_exact_size(egui::vec2(width, 18.0 * scale), egui::Sense::hover());
                let progress = progress.clamp(0.0, 1.0);
                ui.painter().rect_filled(track, 2.0 * scale, RULE);
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(
                        track.min,
                        egui::vec2(track.width() * progress, track.height()),
                    ),
                    2.0 * scale,
                    egui::Color32::from_rgb(190, 150, 76),
                );
                ui.painter().text(
                    track.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{:.0}%", (progress * 100.0).floor()),
                    egui::FontId::proportional(11.0 * scale),
                    INK,
                );
                let (cancel_rect, cancel) = ui.allocate_exact_size(
                    egui::Vec2::splat(22.0 * scale),
                    if can_cancel {
                        egui::Sense::click()
                    } else {
                        egui::Sense::hover()
                    },
                );
                let cancel_fill =
                    if can_cancel && (cancel.is_pointer_button_down_on() || cancel.clicked()) {
                        ui.visuals().widgets.active.bg_fill
                    } else if can_cancel && cancel.hovered() {
                        ui.visuals().widgets.hovered.bg_fill
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                ui.painter().rect_filled(cancel_rect, 3.0 * scale, cancel_fill);
                ui.painter().rect_stroke(
                    cancel_rect,
                    3.0 * scale,
                    egui::Stroke::new(scale, RULE),
                    egui::StrokeKind::Inside,
                );
                ui.painter().text(
                    cancel_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "\u{00d7}",
                    egui::FontId::proportional(16.0 * scale),
                    if can_cancel {
                        label_ink
                    } else {
                        RULE
                    },
                );
                cancel.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        can_cancel,
                        "Cancel active work",
                    )
                });
                let cancel = cancel.on_hover_text("Cancel active work. Costs are not refunded.");
                if can_cancel && cancel.clicked() {
                    action = Some(WorkQueueAction::CancelActive);
                }
            });
        }
        if !queued.is_empty() {
            ui.add_space(6.0 * scale);
            ui.horizontal(|ui| {
                label(ui, "Queue");
                egui::ScrollArea::horizontal()
                    .id_salt("waiting")
                    .max_width(ui.available_width())
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for (index, art, tooltip) in queued {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::Vec2::splat(icon_size),
                                    if can_cancel {
                                        egui::Sense::click()
                                    } else {
                                        egui::Sense::hover()
                                    },
                                );
                                paint_icon(ui, *art, rect);
                                response.widget_info(|| {
                                    egui::WidgetInfo::labeled(
                                        egui::WidgetType::Button,
                                        can_cancel,
                                        tooltip,
                                    )
                                });
                                if can_cancel && response.secondary_clicked() {
                                    action = Some(WorkQueueAction::CancelQueued(*index));
                                }
                                response.on_hover_text(if can_cancel {
                                    format!("{tooltip}\nRight-click to remove and refund its cost.")
                                } else {
                                    tooltip.clone()
                                });
                            }
                        });
                    });
            });
        }
    });
    action
}

/// Paint within an already allocated badge or ledger cell without advancing layout.
pub(in crate::app) fn paint_icon(ui: &egui::Ui, kind: Icon, rect: egui::Rect) {
    if matches!(kind, Icon::PoliticalDistance | Icon::Vassalize | Icon::Integrate) {
        paint_diplomacy_symbol(ui, kind, rect);
        return;
    }
    if matches!(kind, Icon::MilitaryRank(_)) {
        paint_military_symbol(ui, kind, rect);
        return;
    }
    paint_raster_icon(ui, kind, rect, egui::Color32::WHITE);
}

/// Paint raster art with a tint without allocating another layout item.
pub(super) fn paint_raster_icon(ui: &egui::Ui, kind: Icon, rect: egui::Rect, tint: egui::Color32) {
    ui.painter().image(texture(ui.ctx(), kind), rect, icon_uv(kind), tint);
}

/// Unit textures may be atlases; select the same cell for cards and drag previews.
pub(super) fn icon_uv(kind: Icon) -> egui::Rect {
    if matches!(kind, Icon::Unit(unit) if !matches!(unit, UnitType::Catapult | UnitType::WarChariots))
    {
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(0.25, 0.25))
    } else {
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0))
    }
}

/// Small Roman route, allegiance and annexation symbols stay sharp at every UI scale.
fn paint_diplomacy_symbol(ui: &egui::Ui, kind: Icon, rect: egui::Rect) {
    let painter = ui.painter_at(rect);
    let at = |x: f32, y: f32| rect.min + egui::vec2(x * rect.width(), y * rect.height());
    let gold = egui::Color32::from_rgb(199, 159, 82);
    let ink = province_panel::INK;
    let stroke = egui::Stroke::new((rect.width() / 12.0).max(1.0), gold);
    match kind {
        Icon::PoliticalDistance => {
            painter.line_segment([at(0.2, 0.72), at(0.8, 0.28)], stroke);
            for (x, y) in [(0.2, 0.72), (0.5, 0.5), (0.8, 0.28)] {
                painter.circle_filled(at(x, y), rect.width() * 0.13, ink);
                painter.circle_stroke(at(x, y), rect.width() * 0.13, stroke);
            }
        },
        Icon::Vassalize => {
            // Two interlocking rings represent allegiance without direct ownership.
            for (x, y) in [(0.35, 0.6), (0.65, 0.4)] {
                painter.circle_stroke(
                    at(x, y),
                    rect.width() * 0.24,
                    egui::Stroke::new(stroke.width * 1.8, ink),
                );
                painter.circle_stroke(at(x, y), rect.width() * 0.24, stroke);
            }
        },
        Icon::Integrate => {
            // A civic temple represents direct provincial rule.
            painter.add(egui::Shape::convex_polygon(
                vec![at(0.1, 0.35), at(0.5, 0.1), at(0.9, 0.35)],
                gold,
                egui::Stroke::NONE,
            ));
            for x in [0.22, 0.5, 0.78] {
                painter.line_segment([at(x, 0.43), at(x, 0.76)], stroke);
            }
            painter.line_segment([at(0.1, 0.86), at(0.9, 0.86)], stroke);
        },
        _ => unreachable!(),
    }
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

/// Province terrain, settlement and activity banners have distinct cached illustrations.
#[derive(Clone, Copy)]
pub(in crate::app) enum ProvinceLandscape {
    Terrain(crate::game::economy::Terrain),
    City,
    Village,
    Military,
    Army,
    Diplomacy,
    Trade,
}

/// Draw cached landscape artwork with the same cropped framing across province tabs.
pub(in crate::app) fn portrait(
    ui: &mut egui::Ui,
    landscape: ProvinceLandscape,
    height: f32,
) -> egui::Rect {
    use crate::game::economy::Terrain;
    let (name, bytes): (&str, &[u8]) = match landscape {
        ProvinceLandscape::City => {
            ("city", include_bytes!("../../assets/images/cities/city-panel-banner.png"))
        },
        ProvinceLandscape::Village => {
            ("village", include_bytes!("../../assets/images/cities/village-panel-banner.png"))
        },
        ProvinceLandscape::Military => {
            ("military", include_bytes!("../../assets/images/cities/military-panel-banner.png"))
        },
        ProvinceLandscape::Army => {
            ("army", include_bytes!("../../assets/images/cities/army-panel-banner.png"))
        },
        ProvinceLandscape::Diplomacy => {
            ("diplomacy", include_bytes!("../../assets/images/cities/diplomacy-panel-banner.png"))
        },
        ProvinceLandscape::Trade => {
            ("trade", include_bytes!("../../assets/images/cities/trade-panel-banner.png"))
        },
        ProvinceLandscape::Terrain(terrain) => match terrain {
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
        },
    };
    let key = egui::Id::new(("campaign-landscape", name));
    let image = ui.ctx().data(|d| d.get_temp::<egui::TextureHandle>(key)).unwrap_or_else(|| {
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
    rect
}

/// Keep landscape labels over the image; the city badge opens its building inspector.
pub(in crate::app) fn landscape_badges(
    ui: &mut egui::Ui,
    landscape: egui::Rect,
    terrain: crate::game::economy::Terrain,
    city: Option<&str>,
    scale: f32,
) -> bool {
    use crate::game::economy::Terrain;
    let label = match terrain {
        Terrain::Desert => "Desert",
        Terrain::Farmland => "Farmland",
        Terrain::Forest => "Forest",
        Terrain::Hills => "Hills",
        Terrain::Mountains => "Mountains",
        Terrain::Marsh => "Marsh",
        Terrain::Plains => "Plains",
    };
    let painter = ui.painter().with_clip_rect(landscape);
    let margin = 8.0 * scale;
    let height = 27.0 * scale;
    let bottom = landscape.bottom() - margin;
    let font = egui::FontId::proportional(14.0 * scale);
    let text = painter.layout_no_wrap(label.to_owned(), font, egui::Color32::WHITE);
    let badge = egui::Rect::from_min_max(
        egui::pos2(landscape.left() + margin, bottom - height),
        egui::pos2(landscape.left() + margin + text.size().x + 20.0 * scale, bottom),
    );
    painter.rect_filled(badge, 3.0 * scale, egui::Color32::from_black_alpha(180));
    painter.galley(badge.center() - text.size() * 0.5, text, egui::Color32::WHITE);
    if let Some(city) = city {
        let width = (province_panel::navigation_badge_width(&painter, city, scale) * scale)
            .min((landscape.right() - badge.right() - 2.0 * margin).max(1.0));
        let rect = egui::Rect::from_min_max(
            egui::pos2(landscape.right() - margin - width, bottom - height),
            egui::pos2(landscape.right() - margin, bottom),
        );
        return province_panel::navigation_badge(
            ui,
            &painter,
            rect,
            city,
            province_panel::NavigationIcon::City,
            scale,
            "landscape-city",
        );
    }
    false
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

/// Crisp transparent heraldic symbols for ranks.
fn paint_military_symbol(ui: &egui::Ui, kind: Icon, rect: egui::Rect) {
    let painter = ui.painter().with_clip_rect(ui.clip_rect().intersect(rect));
    let size = rect.width().min(rect.height());
    let origin = rect.center() - egui::vec2(size, size) * 0.5;
    let point = |x: f32, y: f32| origin + egui::vec2(x, y) * size;
    let gold = egui::Color32::from_rgb(164, 116, 43);
    let ink = egui::Color32::from_rgb(75, 57, 40);
    let red = egui::Color32::from_rgb(146, 45, 35);
    let stroke = egui::Stroke::new((size * 0.045).max(1.), ink);
    let line = |a: (f32, f32), b: (f32, f32)| {
        painter.line_segment([point(a.0, a.1), point(b.0, b.1)], stroke);
    };
    let polygon = |coords: &[(f32, f32)], fill| {
        painter.add(egui::Shape::convex_polygon(
            coords.iter().map(|&(x, y)| point(x, y)).collect(),
            fill,
            stroke,
        ));
    };
    if let Icon::MilitaryRank(rank) = kind {
        let level = rank as usize;
        // Shield and spear, with promotion bars, laurel and imperial wings.
        polygon(&[(0.26, 0.22), (0.74, 0.22), (0.7, 0.7), (0.5, 0.88), (0.3, 0.7)], red);
        line((0.5, 0.77), (0.5, 0.12));
        polygon(&[(0.43, 0.14), (0.5, 0.03), (0.57, 0.14)], gold);
        for bar in 0..=level {
            let y = 0.34 + bar as f32 * 0.11;
            painter.line_segment(
                [point(0.36, y), point(0.64, y)],
                egui::Stroke::new(size * 0.055, gold),
            );
        }
        if level >= 2 {
            for side in [-1., 1.] {
                for leaf in 0..5 {
                    let y = 0.36 + leaf as f32 * 0.09;
                    painter.circle_filled(
                        point(0.5 + side * (0.34 - (leaf as f32 - 2.).abs() * 0.015), y),
                        size * 0.045,
                        gold,
                    );
                }
            }
        }
        if level == 3 {
            for side in [-1., 1.] {
                polygon(
                    &[
                        (0.5 + side * 0.12, 0.25),
                        (0.5 + side * 0.46, 0.08),
                        (0.5 + side * 0.39, 0.28),
                        (0.5 + side * 0.18, 0.39),
                    ],
                    gold,
                );
            }
        }
    }
}
