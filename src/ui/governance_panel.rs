//! Parchment governance card opened from the map's laws icon.

use crate::map::{EdictLevel, Governance};
use bevy_egui::egui;

const WIDTH: f32 = 500.0;
const HEIGHT: f32 = 570.0;
const HEADER_HEIGHT: f32 = 61.0;
const ROW_HEIGHT: f32 = 95.0;
const SECTION_HEIGHT: f32 = 34.0;

const INK: egui::Color32 = egui::Color32::from_rgb(57, 43, 37);
const RULE: egui::Color32 = egui::Color32::from_rgb(191, 171, 143);

const FOOD: usize = 0;
const HAPPINESS: usize = 1;
const SLAVES: usize = 2;
const SESTERTIUS: usize = 3;
const POPULATION: usize = 4;
const FOOD_TINTABLE: usize = 5;
const MORALE: usize = 6;
pub(super) const EFFECT_ICON_COUNT: usize = 7;
const EFFECT_ICON_ASSETS: [(&str, &[u8]); EFFECT_ICON_COUNT] = [
    ("food", include_bytes!("../../assets/images/icons/food.png")),
    ("happiness", include_bytes!("../../assets/images/icons/happiness.png")),
    ("slaves", include_bytes!("../../assets/images/icons/slaves.png")),
    ("sestertius", include_bytes!("../../assets/images/icons/sestertius.png")),
    ("population", include_bytes!("../../assets/images/icons/manpower.png")),
    ("food_tintable", include_bytes!("../../assets/images/icons/food.png")),
    ("morale", include_bytes!("../../assets/images/icons/morale.png")),
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

pub(super) fn load_effect_icons(
    context: &egui::Context,
) -> [egui::TextureHandle; EFFECT_ICON_COUNT] {
    std::array::from_fn(|index| {
        let (name, bytes) = EFFECT_ICON_ASSETS[index];
        let mut image = image::load_from_memory(bytes)
            .expect("governance effect icon must be valid")
            .resize_exact(64, 64, image::imageops::FilterType::Lanczos3)
            .to_rgba8();
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

pub(super) fn show(
    context: &egui::Context,
    scale: f32,
    icon: &egui::TextureHandle,
    effect_icons: &[egui::TextureHandle; EFFECT_ICON_COUNT],
    banner_color: egui::Color32,
    governance: &mut Governance,
    closing: bool,
) -> (bool, bool) {
    let screen = context.content_rect();
    let left = screen.left() + 60.0 * scale;
    let width = (WIDTH * scale).min((screen.right() - left - 12.0 * scale).max(1.0));
    let height = (HEIGHT * scale).min((screen.height() - 78.0 * scale).max(1.0));
    let rect = super::map_corner_panel_rect(screen, scale, egui::vec2(width, height));
    let mut changed = false;
    let mut close_clicked = false;
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
                    panel.min + egui::vec2(13.0, 8.0) * scale,
                    egui::vec2(34.0, 34.0) * scale,
                ),
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
            painter.text(
                panel.min + egui::vec2(56.0, 25.0) * scale,
                egui::Align2::LEFT_CENTER,
                "Governance",
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
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Close governance")
            });
            if close_response.clicked() {
                close_clicked = true;
            }
            painter.line_segment(
                [
                    panel.min + egui::vec2(13.0, HEADER_HEIGHT - 4.0) * scale,
                    panel.right_top() + egui::vec2(-13.0, HEADER_HEIGHT - 4.0) * scale,
                ],
                egui::Stroke::new(scale, RULE),
            );

            let body_rect = egui::Rect::from_min_max(
                panel.min + egui::vec2(12.0, HEADER_HEIGHT) * scale,
                panel.max - egui::vec2(12.0, 12.0) * scale,
            );
            let mut body =
                ui.new_child(egui::UiBuilder::new().id_salt("edicts").max_rect(body_rect));
            body.spacing_mut().item_spacing.y = 5.0 * scale;
            body.spacing_mut().scroll.bar_width = 5.0 * scale;
            egui::ScrollArea::vertical()
                .max_height(body_rect.height())
                .auto_shrink([false, false])
                .show(&mut body, |list| {
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
                                "Food: 0.8 per person",
                                "All classes: -1 happiness",
                                "Population growth: -0.5%",
                            ],
                            &[
                                "Food: 1 per person",
                                "All classes: +0 happiness",
                                "Population growth: +0%",
                            ],
                            &[
                                "Food: 1.2 per person",
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
                        "noble_taxes",
                        "Noble Taxes",
                        &mut governance.noble_taxes,
                        effect_icons,
                        header_color,
                        [
                            &["Noble tax: 0.5 sestertius per noble", "Nobles: +1 happiness"],
                            &["Noble tax: 1 sestertius per noble", "Nobles: +0 happiness"],
                            &["Noble tax: 1.5 sestertii per noble", "Nobles: -1 happiness"],
                        ],
                        [
                            [badge(SESTERTIUS, "0.5"), badge(HAPPINESS, "+1"), None, None],
                            [badge(SESTERTIUS, "1.0"), badge(HAPPINESS, "0"), None, None],
                            [badge(SESTERTIUS, "1.5"), badge(HAPPINESS, "-1"), None, None],
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
    (changed, close_clicked)
}

fn section(ui: &mut egui::Ui, scale: f32, title: &str) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), SECTION_HEIGHT * scale),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(
        rect.shrink2(egui::vec2(0.0, 3.0 * scale)),
        1.0 * scale,
        egui::Color32::from_rgb(73, 69, 61),
    );
    painter.text(
        rect.left_center() + egui::vec2(10.0, 0.0) * scale,
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(13.5 * scale),
        egui::Color32::from_rgb(249, 238, 215),
    );
}

fn hover_effects(response: egui::Response, scale: f32, effects: &[&str]) -> egui::Response {
    if effects.is_empty() {
        return response;
    }
    response.on_hover_ui(|ui| {
        ui.set_max_width(310.0 * scale);
        ui.spacing_mut().item_spacing.y = 3.0 * scale;
        for effect in effects {
            ui.add(
                egui::Label::new(egui::RichText::new(format!("• {effect}")).size(12.5 * scale))
                    .wrap(),
            );
        }
    })
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
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), ROW_HEIGHT * scale),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0 * scale, egui::Color32::from_rgb(247, 243, 232));
    painter.rect_stroke(
        rect,
        2.0 * scale,
        egui::Stroke::new(0.8 * scale, RULE),
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.left_top() + egui::vec2(10.0, 17.0) * scale,
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(14.5 * scale),
        INK,
    );
    let chosen = match *selected {
        EdictLevel::Low => 0,
        EdictLevel::Medium => 1,
        EdictLevel::High => 2,
    };
    let visible_badges: Vec<_> = badges[chosen].into_iter().flatten().collect();
    let badge_left = rect.right() - 103.0 * scale;
    let badge_stack_height = (visible_badges.len() as f32 * 19.0
        + visible_badges.len().saturating_sub(1) as f32 * 3.0)
        * scale;
    let badge_top = rect.bottom() - 7.0 * scale - badge_stack_height;
    for (index, badge) in visible_badges.into_iter().enumerate() {
        let badge_rect = egui::Rect::from_min_size(
            egui::pos2(badge_left, badge_top + index as f32 * 22.0 * scale),
            egui::vec2(93.0, 19.0) * scale,
        );
        painter.rect_filled(badge_rect, 3.0 * scale, egui::Color32::from_rgb(235, 226, 208));
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
        let chip = egui::Rect::from_min_size(
            rect.left_bottom() + egui::vec2(10.0 + index as f32 * 100.0, -38.0) * scale,
            egui::vec2(94.0, 29.0) * scale,
        );
        let response = ui
            .interact(chip, ui.id().with((id, index)), egui::Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let response = hover_effects(response, scale, descriptions[index]);
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::RadioButton,
                *selected == level,
                format!("{title}: {}", labels[index]),
            )
        });
        if response.clicked() && *selected != level {
            *selected = level;
            changed = true;
        }
        let active = *selected == level;
        let pressed = response.is_pointer_button_down_on();
        painter.rect_filled(
            chip,
            3.0 * scale,
            if pressed {
                egui::Color32::from_rgb(204, 177, 145)
            } else if active {
                egui::Color32::from_rgb(227, 208, 181)
            } else if response.hovered() {
                egui::Color32::from_rgb(239, 229, 208)
            } else {
                egui::Color32::from_rgb(249, 246, 237)
            },
        );
        painter.rect_stroke(
            chip,
            3.0 * scale,
            egui::Stroke::new(
                (if pressed {
                    1.6
                } else {
                    1.0
                }) * scale,
                if active || pressed {
                    accent
                } else {
                    RULE
                },
            ),
            egui::StrokeKind::Inside,
        );
        let circle = chip.left_center() + egui::vec2(15.0 * scale, 0.0);
        painter.circle_stroke(circle, 6.5 * scale, egui::Stroke::new(1.4 * scale, accent));
        if active {
            painter.circle_filled(circle, 3.1 * scale, accent);
        }
        painter.text(
            circle + egui::vec2(12.0 * scale, 0.0),
            egui::Align2::LEFT_CENTER,
            labels[index],
            egui::FontId::proportional(12.5 * scale),
            INK,
        );
    }
    changed
}
