//! Shared illustrated widgets reuse the existing province/city artwork and typography.

use super::spectator::InspectionHover;

use super::province_panel;
use crate::game::{
    economy::BuildingType, military::UnitType, politics::diplomacy::distance_multiplier,
};
use bevy_egui::egui;

/// Set map popup typography before any map UI; restore the original menu style on exit.
pub(in crate::app) fn configure_style(
    mut contexts: bevy_egui::EguiContexts,
    state: bevy::prelude::Res<bevy::prelude::State<super::AppState>>,
    clock: bevy::prelude::Res<super::GameClock>,
    terminal: Option<bevy::prelude::Res<super::TerminalPresentation>>,
    mut previous: bevy::prelude::Local<Option<(bool, u32)>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    super::spectator::set_read_only(ctx, terminal.is_some_and(|terminal| terminal.spectating));
    // Keep every progress display advancing, even when all inspectors are closed.
    ctx.data_mut(|data| {
        data.insert_temp(
            egui::Id::new("campaign-construction-month-fraction"),
            (clock.month_progress / super::SECONDS_PER_MONTH).clamp(0., 1.),
        )
    });
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

/// Both careers explain eligibility and rewards with the same readable hierarchy.
pub(super) fn rank_tooltip(
    ui: &mut egui::Ui,
    scale: f32,
    requirements: &[(bool, String)],
    bonuses: &[String],
) {
    let size = 14.0 * scale;
    ui.set_max_width(340.0 * scale);
    ui.label(egui::RichText::new("Requirements").strong().size(size));
    if requirements.is_empty() {
        ui.label(egui::RichText::new("• None").size(size));
    }
    for (met, text) in requirements {
        let color = if *met {
            egui::Color32::from_rgb(32, 116, 58)
        } else {
            egui::Color32::from_rgb(166, 44, 34)
        };
        ui.add(
            egui::Label::new(egui::RichText::new(format!("• {text}")).size(size).color(color))
                .wrap(),
        );
    }
    if !bonuses.is_empty() {
        ui.separator();
        ui.label(egui::RichText::new("Bonuses").strong().size(size));
        for bonus in bonuses {
            ui.add(egui::Label::new(egui::RichText::new(format!("• {bonus}")).size(size)).wrap());
        }
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
    SpyFlee,
    SpyBuildControl,
    SpyImproveRelations,
    SpyUncoverScandals,
    ScandalControl,
    ScandalRelation,
    SpyUndermineOpponents,
    SpySupportRevolt,
    SpyDiscreditRivals,
    BribeNobles,
    SenatorPetition,
    SenatorGift,
    SenatorPatronage,
    SenatorBribe,
    SenatorThreaten,
    SenatorMurder,
    SenatorBanquet,
    SenatorDiscredit,
    SenatorLobby,
    InsultPlayer,
    Trade,
    Control,
    Relation,
    Diplomacy,
    PoliticalDistance,
    Vassalize,
    Integrate,
    Policies,
    Events,
    Construction,
    Recruitment,
    Attack,
    Offense,
    Defense,
    Speed,
    Maneuver,
    Province,
    Morale,
    Terrain,
    MilitaryPower,
    Cohorts,
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
        Icon::PoliticalDistance
        | Icon::Vassalize
        | Icon::Integrate
        | Icon::SpyFlee
        | Icon::ScandalControl
        | Icon::ScandalRelation => {
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
        Icon::SpySupportRevolt => prepared!("spy-support-revolt"),
        Icon::SpyDiscreditRivals => prepared!("spy-discredit-rivals"),
        Icon::BribeNobles => prepared!("bribe-nobles"),
        Icon::SenatorPetition => prepared!("senator-petition"),
        Icon::SenatorGift => prepared!("senator-gift"),
        Icon::SenatorPatronage => prepared!("senator-patronage"),
        Icon::SenatorBribe => prepared!("senator-bribe"),
        Icon::SenatorThreaten => prepared!("senator-threaten"),
        Icon::SenatorMurder => prepared!("senator-murder"),
        Icon::SenatorBanquet => prepared!("senator-banquet"),
        Icon::SenatorDiscredit => prepared!("senator-discredit"),
        Icon::SenatorLobby => prepared!("senator-lobby"),
        Icon::InsultPlayer => prepared!("insult-player"),
        Icon::Trade => prepared!("trade"),
        Icon::Control => prepared!("control"),
        Icon::Relation => prepared!("relation"),
        Icon::Diplomacy => prepared!("diplomacy"),
        Icon::Policies => prepared!("policies"),
        Icon::Events => include_bytes!(concat!(env!("OUT_DIR"), "/panel-icons/events.png")),
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
        Icon::Duration => prepared!("recruitment-time"),
        Icon::Province => prepared!("province"),
        Icon::Morale => prepared!("morale"),
        Icon::Terrain => prepared!("terrain"),
        Icon::MilitaryPower => prepared!("military-power"),
        Icon::MilitaryRank(crate::game::military::MilitaryRank::Centurion) => {
            prepared!("rank-centurion")
        },
        Icon::MilitaryRank(crate::game::military::MilitaryRank::MilitaryTribune) => {
            prepared!("rank-military-tribune")
        },
        Icon::MilitaryRank(crate::game::military::MilitaryRank::Legate) => {
            prepared!("rank-legate")
        },
        Icon::MilitaryRank(crate::game::military::MilitaryRank::Imperator) => {
            prepared!("rank-imperator")
        },
        Icon::Cohorts => prepared!("cohorts"),
        Icon::Eagle => prepared!("spqr-eagle-gold"),
        Icon::Building(building) => match building {
            BuildingType::Granary => building_art!("granary-rural"),
            BuildingType::Warehouse => building_art!("granary"),
            BuildingType::Aqueduct => building_art!("aqueduct"),
            BuildingType::Baths => building_art!("baths"),
            BuildingType::Road => building_art!("road"),
            BuildingType::CityHall => building_art!("city-hall"),
            BuildingType::Academy => building_art!("academy"),
            BuildingType::CityWalls => building_art!("walls"),
            BuildingType::Forum => building_art!("forum"),
            BuildingType::Temple => building_art!("great-temple"),
            BuildingType::Arena => building_art!("grand-theater"),
            BuildingType::UrbanMarket => building_art!("marketplace"),
        },
        Icon::Wonder(id) => crate::map::wonder_image(id).unwrap_or(prepared!("great-temple")),
        Icon::Unit(_) => {
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
    let enabled = enabled && ui.is_enabled();
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

/// Show why a purchase is unavailable in the same red hover-card badge.
pub(in crate::app) fn unavailable_reason(ui: &mut egui::Ui, reason: &str, scale: f32) {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(248, 223, 217))
        .stroke(egui::Stroke::new(scale, egui::Color32::from_rgb(174, 50, 38)))
        .corner_radius(5.0 * scale)
        .inner_margin(egui::Margin::symmetric(9, 7))
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(reason)
                        .strong()
                        .size(14.0 * scale)
                        .color(egui::Color32::from_rgb(145, 35, 26)),
                )
                .wrap(),
            );
        });
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
    wrap_waiting: bool,
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
            // Keep both icon rows aligned to the same pixel grid.
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
                response.inspection_hover_text(tooltip);
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
                let cancel =
                    cancel.inspection_hover_text("Cancel active work. Costs are not refunded.");
                if can_cancel && cancel.clicked() {
                    action = Some(WorkQueueAction::CancelActive);
                }
            });
        }
        if !queued.is_empty() {
            ui.add_space(6.0 * scale);
            ui.horizontal(|ui| {
                label(ui, "Queue");
                let draw_waiting = |ui: &mut egui::Ui, action: &mut Option<WorkQueueAction>| {
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
                            egui::WidgetInfo::labeled(egui::WidgetType::Button, can_cancel, tooltip)
                        });
                        if can_cancel && response.secondary_clicked() {
                            *action = Some(WorkQueueAction::CancelQueued(*index));
                        }
                        response.inspection_hover_text(if can_cancel {
                            format!("{tooltip}\nRight-click to remove and refund its cost.")
                        } else {
                            tooltip.clone()
                        });
                    }
                };
                if wrap_waiting {
                    ui.horizontal_wrapped(|ui| draw_waiting(ui, &mut action));
                } else {
                    egui::ScrollArea::horizontal()
                        .id_salt("waiting")
                        .max_width(ui.available_width())
                        .show(ui, |ui| {
                            ui.horizontal(|ui| draw_waiting(ui, &mut action));
                        });
                }
            });
        }
    });
    action
}

/// Paint within an already allocated badge or ledger cell without advancing layout.
pub(in crate::app) fn paint_icon(ui: &egui::Ui, kind: Icon, rect: egui::Rect) {
    if matches!(
        kind,
        Icon::PoliticalDistance
            | Icon::Vassalize
            | Icon::Integrate
            | Icon::SpyFlee
            | Icon::ScandalControl
            | Icon::ScandalRelation
    ) {
        paint_diplomacy_symbol(ui, kind, rect);
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
        // Stay one atlas texel inside the cell so linear filtering cannot
        // sample the next animation frame along the bottom or right edge.
        let edge = 0.25 - 1.0 / 768.0;
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(edge, edge))
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
        Icon::ScandalControl | Icon::ScandalRelation => {
            // A sealed evidence sheet with a distinct civic-power or settlement mark.
            let sheet = egui::Rect::from_min_max(at(0.08, 0.08), at(0.65, 0.86));
            painter.rect_filled(sheet, rect.width() * 0.05, egui::Color32::from_rgb(249, 238, 215));
            painter.rect_stroke(
                sheet,
                rect.width() * 0.05,
                egui::Stroke::new((rect.width() * 0.06).max(1.0), ink),
                egui::StrokeKind::Inside,
            );
            for y in [0.28, 0.42] {
                painter.line_segment(
                    [at(0.2, y), at(0.5, y)],
                    egui::Stroke::new(stroke.width * 0.6, ink),
                );
            }
            painter.circle_filled(
                at(0.34, 0.65),
                rect.width() * 0.11,
                egui::Color32::from_rgb(166, 83, 66),
            );
            if kind == Icon::ScandalControl {
                painter.add(egui::Shape::convex_polygon(
                    vec![at(0.55, 0.55), at(0.77, 0.35), at(0.98, 0.55)],
                    gold,
                    egui::Stroke::NONE,
                ));
                for x in [0.61, 0.77, 0.92] {
                    painter.line_segment([at(x, 0.62), at(x, 0.86)], stroke);
                }
                painter.line_segment([at(0.53, 0.93), at(0.99, 0.93)], stroke);
            } else {
                for x in [0.65, 0.85] {
                    painter.circle_stroke(at(x, 0.73), rect.width() * 0.16, stroke);
                }
            }
        },
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
        Icon::SpyFlee => {
            // A dark exit door and heavy outward arrow remain legible at button size.
            let bold = egui::Stroke::new((rect.width() * 0.11).max(1.5), ink);
            painter.line_segment([at(0.12, 0.13), at(0.12, 0.87)], bold);
            painter.line_segment([at(0.12, 0.13), at(0.55, 0.13)], bold);
            painter.line_segment([at(0.12, 0.87), at(0.55, 0.87)], bold);
            painter.line_segment([at(0.55, 0.13), at(0.55, 0.31)], bold);
            painter.line_segment([at(0.55, 0.69), at(0.55, 0.87)], bold);
            painter.line_segment([at(0.28, 0.5), at(0.88, 0.5)], bold);
            painter.line_segment([at(0.88, 0.5), at(0.69, 0.32)], bold);
            painter.line_segment([at(0.88, 0.5), at(0.69, 0.68)], bold);
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
    .inspection_hover_text(tooltip)
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
    Notifications,
    Scandals,
    Policies,
    RomeSenate,
    RomeEvents,
    RomeProvinces,
    RomeSpies,
}

/// Prepare a portrait independently of painting it, so map panels can warm their art.
pub(in crate::app) fn prepare_portrait(
    ctx: &egui::Context,
    landscape: ProvinceLandscape,
) -> egui::TextureHandle {
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
        ProvinceLandscape::Notifications => (
            "notifications",
            include_bytes!("../../assets/images/cities/notifications-panel-banner.png"),
        ),
        ProvinceLandscape::Scandals => {
            ("scandals", include_bytes!("../../assets/images/cities/scandals-panel-banner.png"))
        },
        ProvinceLandscape::Policies => {
            ("policies", include_bytes!(concat!(env!("OUT_DIR"), "/panel-banners/policies.png")))
        },
        ProvinceLandscape::RomeSenate => (
            "rome-senate",
            include_bytes!("../../assets/images/cities/rome-senate-panel-banner.png"),
        ),
        ProvinceLandscape::RomeEvents => {
            ("rome-events", include_bytes!("../../assets/images/events/events-banner.png"))
        },
        ProvinceLandscape::RomeProvinces => (
            "rome-provinces",
            include_bytes!("../../assets/images/cities/rome-provinces-panel-banner.png"),
        ),
        ProvinceLandscape::RomeSpies => {
            ("rome-spies", include_bytes!("../../assets/images/cities/rome-spies-panel-banner.png"))
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
    ctx.data(|d| d.get_temp::<egui::TextureHandle>(key)).unwrap_or_else(|| {
        let handle = if matches!(landscape, ProvinceLandscape::RomeEvents) {
            let image = image::load_from_memory(bytes)
                .expect("valid events banner")
                .resize(800, 800, image::imageops::FilterType::Lanczos3)
                .to_rgba8();
            ctx.load_texture(
                format!("campaign-portrait-{name}"),
                egui::ColorImage::from_rgba_unmultiplied(
                    [image.width() as usize, image.height() as usize],
                    image.as_raw(),
                ),
                egui::TextureOptions::LINEAR,
            )
        } else {
            province_panel::load_image(ctx, (name, bytes), "campaign-portrait")
        };
        ctx.data_mut(|d| d.insert_temp(key, handle.clone()));
        handle
    })
}

/// Draw cached landscape artwork, keeping the Rome figures inside its shallow banner crop.
pub(in crate::app) fn portrait(
    ui: &mut egui::Ui,
    landscape: ProvinceLandscape,
    height: f32,
) -> egui::Rect {
    let image = prepare_portrait(ui.ctx(), landscape);
    let rect =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), height), egui::Sense::hover()).0;
    let aspect = image.size()[0] as f32 / image.size()[1] as f32;
    let target = rect.width() / rect.height();
    let vertical_anchor = match landscape {
        ProvinceLandscape::RomeProvinces => 0.35,
        ProvinceLandscape::RomeSpies => 0.32,
        ProvinceLandscape::RomeSenate => 0.69,
        ProvinceLandscape::Scandals => 0.4,
        _ => 0.5,
    };
    let (x, y_min, y_max) = if aspect > target {
        ((1.0 - target / aspect) * 0.5, 0.0, 1.0)
    } else {
        let cropped = 1.0 - aspect / target;
        let y_min = cropped * vertical_anchor;
        (0.0, y_min, y_min + aspect / target)
    };
    ui.painter().image(
        image.id(),
        rect,
        egui::Rect::from_min_max(egui::pos2(x, y_min), egui::pos2(1.0 - x, y_max)),
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

/// Align the visible content below portraits, including inset section backgrounds.
pub(in crate::app) fn space_after_portrait(
    ui: &mut egui::Ui,
    portrait: egui::Rect,
    scale: f32,
    content_inset: f32,
) {
    let top = portrait.bottom() + (8.0 - content_inset) * scale;
    ui.add_space(top - ui.next_widget_position().y);
}

/// Network actions share centered icons, text, dimensions and hover states.
pub(in crate::app) fn network_action_button(
    ui: &mut egui::Ui,
    symbol: Icon,
    label: &str,
    enabled: bool,
    scale: f32,
) -> egui::Response {
    ui.add_enabled_ui(enabled, |ui| {
        ui.spacing_mut().button_padding.x = 4.0 * scale;
        ui.spacing_mut().icon_spacing = 3.0 * scale;
        ui.visuals_mut().widgets.inactive.bg_fill = province_panel::TABLE_STRIPE;
        ui.visuals_mut().widgets.inactive.weak_bg_fill = province_panel::TABLE_STRIPE;
        ui.visuals_mut().widgets.hovered.bg_fill = egui::Color32::from_rgb(224, 210, 181);
        ui.visuals_mut().widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(224, 210, 181);
        ui.visuals_mut().widgets.active.bg_fill = egui::Color32::from_rgb(207, 180, 137);
        ui.visuals_mut().widgets.active.weak_bg_fill = egui::Color32::from_rgb(207, 180, 137);
        let response = ui.add_sized([98.0 * scale, 30.0 * scale], egui::Button::new(""));
        let color = ui.style().interact(&response).text_color();
        let text = ui.painter().layout_no_wrap(
            label.to_owned(),
            egui::TextStyle::Button.resolve(ui.style()),
            color,
        );
        let icon_size = 20.0 * scale;
        let gap = 3.0 * scale;
        let left = response.rect.center().x - (icon_size + gap + text.size().x) * 0.5;
        paint_icon(
            ui,
            symbol,
            egui::Rect::from_min_size(
                egui::pos2(left, response.rect.center().y - icon_size * 0.5),
                egui::Vec2::splat(icon_size),
            ),
        );
        ui.painter().galley(
            egui::pos2(left + icon_size + gap, response.rect.center().y - text.size().y * 0.5),
            text,
            color,
        );
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
        response
    })
    .inner
}

/// Diplomacy and trade share the same territorial distance value and portrait treatment.
pub(in crate::app) fn territorial_distance_badge(
    ui: &mut egui::Ui,
    portrait: egui::Rect,
    distance: Option<usize>,
    scale: f32,
    hover_text: &str,
) {
    let multiplier = distance_multiplier(distance).ok();
    let value = multiplier.map_or_else(
        || "—".into(),
        |value| format!("×{}", format!("{value:.2}").trim_end_matches('0').trim_end_matches('.')),
    );
    let text = ui.painter().layout_no_wrap(
        format!("Distance modifier {value}"),
        egui::FontId::proportional(12.0 * scale),
        egui::Color32::WHITE,
    );
    let size = egui::vec2(text.size().x + 34.0 * scale, 27.0 * scale);
    let rect = egui::Rect::from_min_size(
        portrait.right_bottom() - size - egui::vec2(7.0, 7.0) * scale,
        size,
    );
    ui.painter().rect_filled(rect, 3.0 * scale, egui::Color32::from_black_alpha(180));
    paint_icon(
        ui,
        Icon::PoliticalDistance,
        egui::Rect::from_min_size(
            rect.min + egui::vec2(5.0, 4.0) * scale,
            egui::vec2(19.0, 19.0) * scale,
        ),
    );
    ui.painter().galley(
        rect.min + egui::vec2(28.0 * scale, (rect.height() - text.size().y) * 0.5),
        text,
        egui::Color32::WHITE,
    );
    ui.interact(rect, ui.id().with("territorial-distance-badge"), egui::Sense::hover())
        .inspection_hover_text(hover_text);
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

/// Inset player color shared by the province, spy, and army directories.
pub(super) const DIRECTORY_ROW_HEIGHT: f32 = 50.0;

pub(super) fn directory_owner_marker_size(row_height: f32, scale: f32) -> egui::Vec2 {
    egui::vec2(6.0 * scale, row_height * (34.0 / DIRECTORY_ROW_HEIGHT))
}

pub(super) fn directory_owner_marker(
    ui: &mut egui::Ui,
    color: egui::Color32,
    row_height: f32,
    scale: f32,
) -> egui::Rect {
    let (swatch, _) = ui
        .allocate_exact_size(directory_owner_marker_size(row_height, scale), egui::Sense::hover());
    ui.painter().rect_filled(swatch, 2.0 * scale, color);
    swatch
}

pub(super) fn paint_directory_row(
    ui: &egui::Ui,
    rect: egui::Rect,
    response: &egui::Response,
    row: usize,
    scale: f32,
) {
    let fill = if response.is_pointer_button_down_on() || response.clicked() {
        egui::Color32::from_rgb(199, 163, 111)
    } else if response.hovered() || response.has_focus() {
        egui::Color32::from_rgb(231, 213, 181)
    } else if row.is_multiple_of(2) {
        province_panel::TABLE_STRIPE
    } else {
        province_panel::PAPER
    };
    ui.painter().rect_filled(rect, 3.0 * scale, fill);
    ui.painter().rect_stroke(
        rect,
        3.0 * scale,
        egui::Stroke::new(scale, province_panel::RULE),
        egui::StrokeKind::Inside,
    );
}

/// Keep province names at the directory body size and truncate long names.
pub(super) fn directory_province_name(ui: &mut egui::Ui, name: &str, width: f32, scale: f32) {
    ui.allocate_ui_with_layout(
        egui::vec2(width, 34.0 * scale),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_size(egui::vec2(width, 34.0 * scale));
            ui.add(
                egui::Label::new(egui::RichText::new(name).strong())
                    .truncate()
                    .halign(egui::Align::LEFT),
            );
        },
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
    style.visuals.text_edit_bg_color = Some(province_panel::PAPER);
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
    style.visuals.widgets.active.weak_bg_fill = egui::Color32::from_rgb(226, 210, 180);
    style
}
