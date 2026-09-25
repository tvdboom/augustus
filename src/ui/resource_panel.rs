//! Province-level production card for food, metal, and stone.

use super::{
    format_hud_number, hud_delta_color, hud_resource_positions, ProvinceOwnership,
    HUD_RESOURCE_GROUP_PADDING, HUD_RESOURCE_NAMES, HUD_RESOURCE_WIDTH, MAP_RESOURCE_STRIP_HEIGHT,
};
use bevy_egui::egui;

const PANEL_WIDTH: f32 = 265.0;
const FOOD_PANEL_WIDTH: f32 = 350.0;
const HEADER_HEIGHT: f32 = 78.0;
const COLUMN_HEIGHT: f32 = 29.0;
const ROW_HEIGHT: f32 = 31.0;
const FOOTER_HEIGHT: f32 = 31.0;
const RESOURCE_DESCRIPTIONS: [&str; 3] =
    ["Feeds your population.", "Used to buy military forces.", "Used to buy buildings."];

fn panel_width(resource: usize) -> f32 {
    if resource == 0 {
        FOOD_PANEL_WIDTH
    } else {
        PANEL_WIDTH
    }
}

pub(super) fn show(
    context: &egui::Context,
    screen: egui::Rect,
    scale: f32,
    date_left: f32,
    player: usize,
    ownership: &ProvinceOwnership,
    icons: &[egui::TextureHandle; 7],
    open_resource: &mut Option<usize>,
) {
    let pointer = context.input(|input| input.pointer.hover_pos());
    let positions = hud_resource_positions();
    let hovered = positions.into_iter().enumerate().take(3).find_map(|(index, x)| {
        let hit = egui::Rect::from_min_size(
            screen.min + egui::vec2(x, 0.0) * scale,
            egui::vec2(HUD_RESOURCE_WIDTH, MAP_RESOURCE_STRIP_HEIGHT) * scale,
        );
        (x + HUD_RESOURCE_WIDTH <= date_left - HUD_RESOURCE_GROUP_PADDING
            && pointer.is_some_and(|point| hit.contains(point)))
        .then_some(index)
    });
    let Some(resource) = hovered.or(*open_resource) else {
        return;
    };

    let mut sources = ownership.production_sources(player, resource);
    sources.sort_unstable_by(|a, b| a.0.cmp(b.0));
    let top = screen.top() + 45.0 * scale;
    let fixed_height = HEADER_HEIGHT + COLUMN_HEIGHT + FOOTER_HEIGHT;
    let available_body = (screen.bottom() - top - 12.0 * scale) / scale - fixed_height;
    let body_height =
        (sources.len().max(1) as f32 * ROW_HEIGHT).min(available_body.max(ROW_HEIGHT));
    let width = panel_width(resource) * scale;
    let left = (screen.left() + hud_resource_positions()[resource] * scale)
        .max(screen.left() + 8.0 * scale)
        .min(screen.right() - width - 8.0 * scale);
    let rect = egui::Rect::from_min_size(
        egui::pos2(left, top),
        egui::vec2(width, (fixed_height + body_height) * scale),
    );

    // Keeping the card open over its own surface lets players scroll a long
    // province list without requiring the pointer to remain on the HUD icon.
    if hovered == Some(resource) || pointer.is_some_and(|point| rect.contains(point)) {
        *open_resource = Some(resource);
        paint(context, rect, scale, body_height, resource, icons, &sources);
    } else {
        *open_resource = None;
    }
}

fn paint(
    context: &egui::Context,
    panel_rect: egui::Rect,
    scale: f32,
    body_height: f32,
    resource: usize,
    icons: &[egui::TextureHandle; 7],
    sources: &[(&str, f64, f64)],
) {
    let width = panel_width(resource);
    let ink = egui::Color32::from_rgb(51, 46, 39);
    let muted = egui::Color32::from_rgb(109, 97, 79);
    let rule = egui::Color32::from_rgb(188, 177, 155);
    let total_produced: f64 = sources.iter().map(|(_, produced, _)| produced).sum();
    let total_consumed: f64 = sources.iter().map(|(_, _, consumed)| consumed).sum();
    egui::Area::new(egui::Id::new("augustus_resource_sources"))
        .fixed_pos(panel_rect.min)
        .order(egui::Order::Tooltip)
        .show(context, |ui| {
            let (rect, _) = ui.allocate_exact_size(panel_rect.size(), egui::Sense::hover());
            let painter = ui.painter_at(rect);
            let p = |x: f32, y: f32| rect.min + egui::vec2(x, y) * scale;
            let footer_top = HEADER_HEIGHT + COLUMN_HEIGHT + body_height;

            // Match the top strip's pale stone palette and etched border.
            painter.rect_filled(
                rect.translate(egui::vec2(3.0, 4.0) * scale),
                9.0 * scale,
                egui::Color32::from_black_alpha(65),
            );
            painter.rect_filled(rect, 9.0 * scale, egui::Color32::from_rgb(236, 231, 216));
            painter.rect_stroke(
                rect,
                9.0 * scale,
                egui::Stroke::new(scale, egui::Color32::from_rgb(144, 133, 110)),
                egui::StrokeKind::Inside,
            );
            painter.rect_stroke(
                rect.shrink(3.0 * scale),
                7.0 * scale,
                egui::Stroke::new(scale, egui::Color32::from_rgb(248, 244, 231)),
                egui::StrokeKind::Inside,
            );
            painter.line_segment(
                [p(14.0, 5.0), p(width - 14.0, 5.0)],
                egui::Stroke::new(2.0 * scale, egui::Color32::from_rgb(157, 95, 67)),
            );
            painter.circle_filled(
                p(45.0, 39.0),
                30.0 * scale,
                egui::Color32::from_rgb(224, 214, 194),
            );
            painter.circle_stroke(p(45.0, 39.0), 30.0 * scale, egui::Stroke::new(scale, rule));
            painter.image(
                icons[resource].id(),
                egui::Rect::from_min_size(p(17.0, 11.0), egui::vec2(56.0, 56.0) * scale),
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
            painter.text(
                p(84.0, 29.0),
                egui::Align2::LEFT_CENTER,
                HUD_RESOURCE_NAMES[resource],
                egui::FontId::proportional(22.0 * scale),
                ink,
            );
            painter.text(
                p(84.0, 52.0),
                egui::Align2::LEFT_CENTER,
                RESOURCE_DESCRIPTIONS[resource],
                egui::FontId::proportional(12.0 * scale),
                muted,
            );
            painter.line_segment(
                [p(14.0, HEADER_HEIGHT - 2.0), p(width - 14.0, HEADER_HEIGHT - 2.0)],
                egui::Stroke::new(scale, rule),
            );
            let produced_x = width
                - if resource == 0 {
                    94.0
                } else {
                    19.0
                };
            for (label, x, align) in [
                ("PROVINCE", 18.0, egui::Align2::LEFT_CENTER),
                ("PRODUCED", produced_x, egui::Align2::RIGHT_CENTER),
            ] {
                painter.text(
                    p(x, HEADER_HEIGHT + 15.0),
                    align,
                    label,
                    egui::FontId::proportional(10.5 * scale),
                    muted,
                );
            }
            if resource == 0 {
                painter.text(
                    p(width - 19.0, HEADER_HEIGHT + 15.0),
                    egui::Align2::RIGHT_CENTER,
                    "CONSUMED",
                    egui::FontId::proportional(10.5 * scale),
                    muted,
                );
            }
            painter.line_segment(
                [
                    p(14.0, HEADER_HEIGHT + COLUMN_HEIGHT),
                    p(width - 14.0, HEADER_HEIGHT + COLUMN_HEIGHT),
                ],
                egui::Stroke::new(scale, rule),
            );

            let body_rect = egui::Rect::from_min_size(
                p(13.0, HEADER_HEIGHT + COLUMN_HEIGHT),
                egui::vec2(width - 26.0, body_height) * scale,
            );
            let mut body_ui = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt(("province_sources_list", resource))
                    .max_rect(body_rect),
            );
            body_ui.spacing_mut().item_spacing.y = 0.0;
            body_ui.spacing_mut().scroll.bar_width = 5.0 * scale;
            body_ui.spacing_mut().scroll.bar_inner_margin = 2.0 * scale;
            body_ui.spacing_mut().scroll.bar_outer_margin = 0.0;
            egui::ScrollArea::vertical()
                .max_height(body_height * scale)
                .auto_shrink([false, false])
                .show(&mut body_ui, |list| {
                    if sources.is_empty() {
                        let (row, _) = list.allocate_exact_size(
                            egui::vec2(list.available_width(), ROW_HEIGHT * scale),
                            egui::Sense::hover(),
                        );
                        list.painter().text(
                            row.left_center() + egui::vec2(5.0 * scale, 0.0),
                            egui::Align2::LEFT_CENTER,
                            "No owned provinces",
                            egui::FontId::proportional(13.0 * scale),
                            muted,
                        );
                    }
                    for (index, (name, produced, consumed)) in sources.iter().enumerate() {
                        let (row, _) = list.allocate_exact_size(
                            egui::vec2(list.available_width(), ROW_HEIGHT * scale),
                            egui::Sense::hover(),
                        );
                        let row_painter = list.painter_at(row);
                        if index % 2 == 0 {
                            row_painter.rect_filled(
                                row,
                                0.0,
                                egui::Color32::from_rgb(244, 239, 225),
                            );
                        }
                        row_painter.line_segment(
                            [row.left_bottom(), row.right_bottom()],
                            egui::Stroke::new(0.6 * scale, rule),
                        );
                        row_painter.text(
                            row.left_center() + egui::vec2(5.0 * scale, 0.0),
                            egui::Align2::LEFT_CENTER,
                            *name,
                            egui::FontId::proportional(13.0 * scale),
                            ink,
                        );
                        row_painter.text(
                            egui::pos2(rect.left() + produced_x * scale, row.center().y),
                            egui::Align2::RIGHT_CENTER,
                            format_hud_number(*produced),
                            egui::FontId::proportional(13.0 * scale),
                            hud_delta_color(*produced),
                        );
                        if resource == 0 {
                            row_painter.text(
                                egui::pos2(rect.right() - 19.0 * scale, row.center().y),
                                egui::Align2::RIGHT_CENTER,
                                format_hud_number(*consumed),
                                egui::FontId::proportional(13.0 * scale),
                                hud_delta_color(-*consumed),
                            );
                        }
                    }
                });

            let footer = egui::Rect::from_min_max(
                p(4.0, footer_top),
                p(width - 4.0, footer_top + FOOTER_HEIGHT - 4.0),
            );
            painter.rect_filled(footer, 5.0 * scale, egui::Color32::from_rgb(222, 211, 188));
            painter.line_segment(
                [p(14.0, footer_top), p(width - 14.0, footer_top)],
                egui::Stroke::new(scale, rule),
            );
            painter.text(
                p(18.0, footer_top + 15.0),
                egui::Align2::LEFT_CENTER,
                "TOTAL",
                egui::FontId::proportional(10.5 * scale),
                ink,
            );
            painter.text(
                p(produced_x, footer_top + 15.0),
                egui::Align2::RIGHT_CENTER,
                format_hud_number(total_produced),
                egui::FontId::proportional(12.0 * scale),
                hud_delta_color(total_produced),
            );
            if resource == 0 {
                painter.text(
                    p(width - 19.0, footer_top + 15.0),
                    egui::Align2::RIGHT_CENTER,
                    format_hud_number(total_consumed),
                    egui::FontId::proportional(12.0 * scale),
                    hud_delta_color(-total_consumed),
                );
            }
        });
}
