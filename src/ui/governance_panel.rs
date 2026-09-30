//! Parchment governance card opened from the map's laws icon.

use super::campaign_widgets::{icon as tab_icon, Icon};
use super::policy_widgets::{self, section};
use crate::map::{EdictLevel, Governance};
use bevy_egui::egui;

const HEADER_HEIGHT: f32 = 61.0;

const INK: egui::Color32 = egui::Color32::from_rgb(57, 43, 37);
const RULE: egui::Color32 = egui::Color32::from_rgb(191, 171, 143);

const FOOD: usize = 0;
const HAPPINESS: usize = 1;
const SLAVES: usize = 2;
const SESTERTIUS: usize = 3;
const POPULATION: usize = 4;
const FOOD_TINTABLE: usize = 5;
const MORALE: usize = 6;
pub(in crate::app) const EFFECT_ICON_COUNT: usize = 7;
const EFFECT_ICON_ASSETS: [(&str, &[u8]); EFFECT_ICON_COUNT] = [
    ("food", include_bytes!(concat!(env!("OUT_DIR"), "/panel-icons/food.png"))),
    ("happiness", include_bytes!(concat!(env!("OUT_DIR"), "/panel-icons/happiness.png"))),
    ("slaves", include_bytes!(concat!(env!("OUT_DIR"), "/panel-icons/slaves.png"))),
    ("sestertius", include_bytes!(concat!(env!("OUT_DIR"), "/panel-icons/coin.png"))),
    ("population", include_bytes!(concat!(env!("OUT_DIR"), "/panel-icons/population.png"))),
    ("food_tintable", include_bytes!(concat!(env!("OUT_DIR"), "/panel-icons/food.png"))),
    ("morale", include_bytes!(concat!(env!("OUT_DIR"), "/panel-icons/morale.png"))),
];

#[derive(Clone, Copy)]
struct EffectBadge {
    icon: usize,
    tint: egui::Color32,
    value: &'static str,
}

const fn badge(icon: usize, value: &'static str) -> Option<EffectBadge> {
    Some(EffectBadge {
        icon,
        tint: egui::Color32::WHITE,
        value,
    })
}

const fn tinted_badge(
    icon: usize,
    value: &'static str,
    tint: egui::Color32,
) -> Option<EffectBadge> {
    Some(EffectBadge {
        icon,
        tint,
        value,
    })
}

pub(in crate::app) fn load_effect_icons(
    context: &egui::Context,
) -> [egui::TextureHandle; EFFECT_ICON_COUNT] {
    std::array::from_fn(|index| {
        let (name, bytes) = EFFECT_ICON_ASSETS[index];
        let mut image = image::load_from_memory(bytes)
            .expect("prepared governance effect icon must be valid")
            .to_rgba8();
        debug_assert_eq!((image.width(), image.height()), (64, 64));
        if index == FOOD_TINTABLE {
            for pixel in image.pixels_mut() {
                let gray = (0.2126 * f32::from(pixel[0])
                    + 0.7152 * f32::from(pixel[1])
                    + 0.0722 * f32::from(pixel[2])) as u8;
                pixel.0[..3].fill(gray);
            }
        }
        context.load_texture(
            format!("augustus_governance_effect_{name}"),
            egui::ColorImage::from_rgba_unmultiplied(
                [image.width() as usize, image.height() as usize],
                image.as_raw(),
            ),
            egui::TextureOptions::LINEAR,
        )
    })
}

#[cfg(test)]
pub(in crate::app) fn show(
    context: &egui::Context,
    scale: f32,
    icon: &egui::TextureHandle,
    effect_icons: &[egui::TextureHandle; EFFECT_ICON_COUNT],
    banner_color: egui::Color32,
    governance: &mut Governance,
    closing: bool,
) -> (bool, bool) {
    let (changed, closed, _) = show_impl(
        context,
        scale,
        icon,
        effect_icons,
        banner_color,
        governance,
        closing,
        None,
        |_, _| None,
    );
    (changed, closed)
}

#[allow(clippy::too_many_arguments)]
pub(in crate::app) fn show_overview(
    context: &egui::Context,
    scale: f32,
    icon: &egui::TextureHandle,
    effect_icons: &[egui::TextureHandle; EFFECT_ICON_COUNT],
    banner_color: egui::Color32,
    governance: &mut Governance,
    closing: bool,
    selected: &mut usize,
    content: impl FnMut(&mut egui::Ui, usize) -> Option<usize>,
) -> (bool, bool, Option<usize>) {
    show_impl(
        context,
        scale,
        icon,
        effect_icons,
        banner_color,
        governance,
        closing,
        Some(selected),
        content,
    )
}

#[allow(clippy::too_many_arguments)]
fn show_impl(
    context: &egui::Context,
    scale: f32,
    icon: &egui::TextureHandle,
    effect_icons: &[egui::TextureHandle; EFFECT_ICON_COUNT],
    banner_color: egui::Color32,
    governance: &mut Governance,
    closing: bool,
    mut selected: Option<&mut usize>,
    mut content: impl FnMut(&mut egui::Ui, usize) -> Option<usize>,
) -> (bool, bool, Option<usize>) {
    let screen = context.content_rect();
    let width = (super::campaign_panel::PANEL_WIDTH * scale)
        .min((screen.width() - 80.0 * scale).max(120.0));
    let height = (super::campaign_panel::PANEL_HEIGHT * scale)
        .min((screen.height() - 90.0 * scale).max(140.0));
    let rect = super::map_corner_panel_rect(screen, scale, egui::vec2(width, height));
    let mut changed = false;
    let mut close_clicked = false;
    let mut target = None;
    let header_color = banner_color;
    egui::Area::new(egui::Id::new("augustus_governance_panel"))
        .fixed_pos(rect.min)
        .order(egui::Order::Foreground)
        .show(context, |ui| {
            let (panel, _) = ui.allocate_exact_size(rect.size(), egui::Sense::hover());
            let painter = ui.painter_at(panel);
            painter.rect_filled(
                panel.translate(egui::vec2(4.0, 5.0) * scale),
                6.0 * scale,
                egui::Color32::from_black_alpha(80),
            );
            painter.rect_filled(panel, 6.0 * scale, egui::Color32::from_rgb(238, 233, 219));
            painter.rect_stroke(
                panel,
                6.0 * scale,
                egui::Stroke::new(1.2 * scale, header_color),
                egui::StrokeKind::Inside,
            );
            painter.rect_filled(
                egui::Rect::from_min_size(panel.min, egui::vec2(panel.width(), 51.0 * scale)),
                5.0 * scale,
                header_color,
            );
            painter.rect_filled(
                egui::Rect::from_min_size(
                    panel.min + egui::vec2(0.0, 44.0) * scale,
                    egui::vec2(panel.width(), 7.0 * scale),
                ),
                0.0,
                header_color,
            );
            painter.line_segment(
                [
                    panel.min + egui::vec2(12.0, 46.0) * scale,
                    panel.right_top() + egui::vec2(-12.0, 46.0) * scale,
                ],
                egui::Stroke::new(scale, egui::Color32::from_rgb(194, 148, 103)),
            );
            painter.image(
                icon.id(),
                egui::Rect::from_min_size(
                    panel.min + egui::vec2(8.0, 8.0) * scale,
                    egui::vec2(34.0, 34.0) * scale,
                ),
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
            painter.text(
                panel.center_top() + egui::vec2(0.0, 25.0 * scale),
                egui::Align2::CENTER_CENTER,
                if selected.is_some() {
                    "Overview"
                } else {
                    "Governance"
                },
                egui::FontId::proportional(25.0 * scale),
                egui::Color32::from_rgb(255, 238, 198),
            );
            let close = egui::Rect::from_center_size(
                panel.right_top() + egui::vec2(-25.0, 25.0) * scale,
                egui::vec2(32.0, 32.0) * scale,
            );
            let close_response = ui
                .interact(close, ui.id().with("close"), egui::Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            let close_pressed =
                closing || close_response.is_pointer_button_down_on() || close_response.clicked();
            let close_fill = if close_pressed {
                egui::Color32::from_rgb(112, 48, 43)
            } else if close_response.hovered() {
                egui::Color32::from_rgba_unmultiplied(255, 237, 205, 55)
            } else {
                egui::Color32::TRANSPARENT
            };
            painter.circle_filled(close.center(), 14.0 * scale, close_fill);
            painter.circle_stroke(
                close.center(),
                13.0 * scale,
                egui::Stroke::new(
                    (if close_pressed {
                        2.2
                    } else if close_response.hovered() {
                        2.0
                    } else {
                        1.3
                    }) * scale,
                    if close_pressed {
                        egui::Color32::from_rgb(255, 239, 209)
                    } else {
                        egui::Color32::from_rgb(247, 224, 190)
                    },
                ),
            );
            let cross_center = close.center();
            let cross_arm = 5.0 * scale;
            let cross_stroke = egui::Stroke::new(
                2.2 * scale,
                if close_pressed {
                    egui::Color32::from_rgb(255, 219, 166)
                } else {
                    egui::Color32::from_rgb(255, 240, 216)
                },
            );
            painter.line_segment(
                [
                    cross_center + egui::vec2(-cross_arm, -cross_arm),
                    cross_center + egui::vec2(cross_arm, cross_arm),
                ],
                cross_stroke,
            );
            painter.line_segment(
                [
                    cross_center + egui::vec2(-cross_arm, cross_arm),
                    cross_center + egui::vec2(cross_arm, -cross_arm),
                ],
                cross_stroke,
            );
            close_response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Close overview")
            });
            if close_response.clicked() {
                close_clicked = true;
            }
            let body_rect = egui::Rect::from_min_max(
                panel.min + egui::vec2(12.0, HEADER_HEIGHT) * scale,
                panel.max - egui::vec2(12.0, 12.0) * scale,
            );
            let mut body =
                ui.new_child(egui::UiBuilder::new().id_salt("edicts").max_rect(body_rect));
            body.spacing_mut().item_spacing.y = 5.0 * scale;
            body.spacing_mut().scroll.bar_width = 5.0 * scale;
            if let Some(selected) = selected.as_deref_mut() {
                let tabs = [
                    (Icon::Policies, "Governance"),
                    (Icon::Events, "Events"),
                    (Icon::Province, "Provinces"),
                    (Icon::Spy, "Spies"),
                    (Icon::Notifications, "Notifications"),
                ];
                let gap = body.spacing().item_spacing.x;
                let width = ((body.available_width() - (tabs.len() - 1) as f32 * gap)
                    / tabs.len() as f32)
                    .max(1.0);
                body.horizontal(|ui| {
                    for (index, (symbol, label)) in tabs.into_iter().enumerate() {
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(width, 47.0 * scale),
                            egui::Sense::click(),
                        );
                        let active = *selected == index;
                        let pressed = response.is_pointer_button_down_on();
                        if active || response.hovered() || pressed {
                            let fill = if pressed {
                                egui::Color32::from_rgb(199, 163, 111)
                            } else if active && response.hovered() {
                                egui::Color32::from_rgb(211, 185, 145)
                            } else if active {
                                egui::Color32::from_rgb(219, 200, 168)
                            } else {
                                egui::Color32::from_rgb(231, 213, 181)
                            };
                            ui.painter().rect_filled(rect, 3.0 * scale, fill);
                            ui.painter().rect_stroke(
                                rect,
                                3.0 * scale,
                                egui::Stroke::new(
                                    scale,
                                    if active {
                                        header_color
                                    } else {
                                        RULE
                                    },
                                ),
                                egui::StrokeKind::Inside,
                            );
                        }
                        ui.scope_builder(
                            egui::UiBuilder::new()
                                .max_rect(rect)
                                .layout(egui::Layout::top_down(egui::Align::Center)),
                            |ui| {
                                tab_icon(ui, symbol, 26.0 * scale);
                                ui.add_sized(
                                    [width, 16.0 * scale],
                                    egui::Label::new(egui::RichText::new(label).size(11.0 * scale))
                                        .truncate()
                                        .show_tooltip_when_elided(false),
                                );
                            },
                        );
                        if active {
                            ui.painter().hline(
                                rect.x_range(),
                                rect.bottom(),
                                egui::Stroke::new(3.0 * scale, header_color),
                            );
                        }
                        if response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            *selected = index;
                        }
                    }
                });
                body.separator();
            }
            let current_section = selected.as_deref().copied().unwrap_or(0);
            egui::ScrollArea::vertical()
                .id_salt(("overview_body", current_section))
                .max_height(body.available_height())
                .auto_shrink([false, false])
                .show(&mut body, |list| {
                    if current_section != 0 {
                        if current_section != 4 {
                            super::campaign_widgets::portrait(
                                list,
                                if current_section == 1 {
                                    super::campaign_widgets::ProvinceLandscape::RomeEvents
                                } else if current_section == 2 {
                                    super::campaign_widgets::ProvinceLandscape::RomeProvinces
                                } else {
                                    super::campaign_widgets::ProvinceLandscape::RomeSpies
                                },
                                76.0 * scale,
                            );
                        }
                        target = content(list, current_section);
                        return;
                    }
                    super::campaign_widgets::portrait(
                        list,
                        super::campaign_widgets::ProvinceLandscape::Policies,
                        76.0 * scale,
                    );
                    list.add_space(8.0 * scale);
                    section(list, scale, "LABOR & AGRARIAN");
                    changed |= edict_row(
                        list,
                        scale,
                        "food_rations",
                        "Food Rations",
                        &mut governance.food_rations,
                        effect_icons,
                        header_color,
                        [
                            &[
                                "Food: 0.08 per resident",
                                "All classes: -1 happiness",
                                "Population growth: -0.5%",
                            ],
                            &[
                                "Food: 0.10 per resident",
                                "All classes: +0 happiness",
                                "Population growth: +0%",
                            ],
                            &[
                                "Food: 0.12 per resident",
                                "All classes: +1 happiness",
                                "Population growth: +0.5%",
                            ],
                        ],
                        [
                            [
                                tinted_badge(
                                    FOOD_TINTABLE,
                                    "0.8",
                                    egui::Color32::from_rgb(95, 195, 115),
                                ),
                                badge(HAPPINESS, "-1"),
                                badge(POPULATION, "-0.5%"),
                                None,
                            ],
                            [
                                badge(FOOD, "1.0"),
                                badge(HAPPINESS, "0"),
                                badge(POPULATION, "0%"),
                                None,
                            ],
                            [
                                tinted_badge(
                                    FOOD_TINTABLE,
                                    "1.2",
                                    egui::Color32::from_rgb(235, 85, 70),
                                ),
                                badge(HAPPINESS, "+1"),
                                badge(POPULATION, "+0.5%"),
                                None,
                            ],
                        ],
                    );
                    changed |= edict_row(
                        list,
                        scale,
                        "slave_labor",
                        "Slave Labor",
                        &mut governance.slave_labor,
                        effect_icons,
                        header_color,
                        [
                            &[
                                "Slave production: -50%",
                                "Slaves: +2 happiness",
                                "Slave population growth: +0%",
                            ],
                            &[
                                "Slave production: +0%",
                                "Slaves: +0 happiness",
                                "Slave population growth: +0%",
                            ],
                            &[
                                "Slave production: +50%",
                                "Slaves: -2 happiness",
                                "Slave population growth: -0.5%",
                            ],
                        ],
                        [
                            [
                                badge(SLAVES, "-50%"),
                                badge(HAPPINESS, "+2"),
                                badge(POPULATION, "0%"),
                                None,
                            ],
                            [
                                badge(SLAVES, "0%"),
                                badge(HAPPINESS, "0"),
                                badge(POPULATION, "0%"),
                                None,
                            ],
                            [
                                badge(SLAVES, "+50%"),
                                badge(HAPPINESS, "-2"),
                                badge(POPULATION, "-0.5%"),
                                None,
                            ],
                        ],
                    );
                    section(list, scale, "ECONOMY");
                    changed |= edict_row(
                        list,
                        scale,
                        "noble_wages",
                        "Noble Wages",
                        &mut governance.noble_wages,
                        effect_icons,
                        header_color,
                        [
                            &["Noble wage costs: -25%", "Nobles: -2 happiness"],
                            &["Noble wage costs: +0%", "Nobles: +0 happiness"],
                            &["Noble wage costs: +25%", "Nobles: +1 happiness"],
                        ],
                        [
                            [badge(SESTERTIUS, "-25%"), badge(HAPPINESS, "-2"), None, None],
                            [badge(SESTERTIUS, "0%"), badge(HAPPINESS, "0"), None, None],
                            [badge(SESTERTIUS, "+25%"), badge(HAPPINESS, "+1"), None, None],
                        ],
                    );
                    changed |= edict_row(
                        list,
                        scale,
                        "army_wages",
                        "Army Wages",
                        &mut governance.army_wages,
                        effect_icons,
                        header_color,
                        [
                            &["Army wage costs: -25%", "Army morale: -2"],
                            &["Army wage costs: +0%", "Army morale: +0"],
                            &["Army wage costs: +25%", "Army morale: +1"],
                        ],
                        [
                            [badge(SESTERTIUS, "-25%"), badge(MORALE, "-2"), None, None],
                            [badge(SESTERTIUS, "0%"), badge(MORALE, "0"), None, None],
                            [badge(SESTERTIUS, "+25%"), badge(MORALE, "+1"), None, None],
                        ],
                    );
                });
        });
    (changed, close_clicked, target)
}

#[allow(clippy::too_many_arguments)]
fn edict_row(
    ui: &mut egui::Ui,
    scale: f32,
    id: &str,
    title: &str,
    selected: &mut EdictLevel,
    icons: &[egui::TextureHandle; EFFECT_ICON_COUNT],
    accent: egui::Color32,
    descriptions: [&[&str]; 3],
    badges: [[Option<EffectBadge>; 4]; 3],
) -> bool {
    let rect = policy_widgets::card(ui, scale, title);
    let painter = ui.painter_at(rect);
    let chosen = match *selected {
        EdictLevel::Low => 0,
        EdictLevel::Medium => 1,
        EdictLevel::High => 2,
    };
    let visible_badges: Vec<_> = badges[chosen].into_iter().flatten().collect();
    let count = visible_badges.len();
    for (index, badge) in visible_badges.into_iter().enumerate() {
        let badge_rect = policy_widgets::effect_rect(&painter, rect, scale, count, index);
        painter.image(
            icons[badge.icon].id(),
            egui::Rect::from_min_size(
                badge_rect.min + egui::vec2(1.0, 1.0) * scale,
                egui::vec2(17.0, 17.0) * scale,
            ),
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            badge.tint,
        );
        let value_color = if badge.value == "0" || badge.value == "0%" {
            egui::Color32::BLACK
        } else if badge.icon == HAPPINESS || badge.icon == POPULATION || badge.icon == MORALE {
            if badge.value.starts_with('-') {
                egui::Color32::from_rgb(165, 51, 43)
            } else if badge.value.starts_with('+') {
                egui::Color32::from_rgb(44, 122, 63)
            } else {
                INK
            }
        } else {
            INK
        };
        painter.text(
            badge_rect.right_center() - egui::vec2(4.0 * scale, 0.0),
            egui::Align2::RIGHT_CENTER,
            badge.value,
            egui::FontId::proportional(11.5 * scale),
            value_color,
        );
    }
    let levels = [EdictLevel::Low, EdictLevel::Medium, EdictLevel::High];
    let labels = ["Low", "Medium", "High"];
    let mut changed = false;
    for (index, level) in levels.into_iter().enumerate() {
        let chip = policy_widgets::choice_rect(rect, scale, index);
        let (choice_changed, response) = policy_widgets::choice(
            ui,
            chip,
            ui.id().with((id, index)),
            title,
            labels[index],
            selected,
            level,
            accent,
            scale,
        );
        changed |= choice_changed;
        policy_widgets::hover_effects(response, scale, descriptions[index]);
    }
    changed
}
