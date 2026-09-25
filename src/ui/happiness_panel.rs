//! Class happiness for the active player's population.

use super::{
    format_hud_delta, format_hud_number, hud_resource_positions, HudResource,
    HUD_RESOURCE_GROUP_PADDING, HUD_RESOURCE_WIDTH, MAP_RESOURCE_STRIP_HEIGHT, POP_CLASS_NAMES,
};
use bevy_egui::egui;

const WIDTH: f32 = 350.0;
const HEADER_HEIGHT: f32 = 78.0;
const SUMMARY_HEIGHT: f32 = 42.0;
const ROW_HEIGHT: f32 = 38.0;

fn happiness_color(value: f64) -> egui::Color32 {
    if value < 50.0 {
        egui::Color32::from_rgb(170, 53, 45)
    } else if value > 50.0 {
        egui::Color32::from_rgb(42, 125, 63)
    } else {
        egui::Color32::from_rgb(51, 46, 39)
    }
}

pub(super) fn show(
    context: &egui::Context,
    screen: egui::Rect,
    scale: f32,
    date_left: f32,
    happiness: [HudResource; 4],
    icon: &egui::TextureHandle,
    class_icons: &[egui::TextureHandle; 4],
    open: &mut bool,
) {
    let x = hud_resource_positions()[6];
    if x + HUD_RESOURCE_WIDTH > date_left - HUD_RESOURCE_GROUP_PADDING {
        *open = false;
        return;
    }
    let hit = egui::Rect::from_min_size(
        screen.min + egui::vec2(x, 0.0) * scale,
        egui::vec2(HUD_RESOURCE_WIDTH, MAP_RESOURCE_STRIP_HEIGHT) * scale,
    );
    let top = screen.top() + 45.0 * scale;
    let left = (hit.left())
        .max(screen.left() + 8.0 * scale)
        .min(screen.right() - WIDTH * scale - 8.0 * scale);
    let height = HEADER_HEIGHT + SUMMARY_HEIGHT + ROW_HEIGHT * 4.0;
    let rect = egui::Rect::from_min_size(egui::pos2(left, top), egui::vec2(WIDTH, height) * scale);
    let pointer = context.input(|input| input.pointer.hover_pos());
    if !pointer.is_some_and(|point| hit.contains(point) || (*open && rect.contains(point))) {
        *open = false;
        return;
    }
    *open = true;
    paint(context, rect, scale, happiness, icon, class_icons);
}

fn paint(
    context: &egui::Context,
    panel_rect: egui::Rect,
    scale: f32,
    happiness: [HudResource; 4],
    icon: &egui::TextureHandle,
    class_icons: &[egui::TextureHandle; 4],
) {
    let ink = egui::Color32::from_rgb(51, 46, 39);
    let muted = egui::Color32::from_rgb(109, 97, 79);
    let rule = egui::Color32::from_rgb(188, 177, 155);
    egui::Area::new(egui::Id::new("augustus_happiness_panel"))
        .fixed_pos(panel_rect.min)
        .order(egui::Order::Tooltip)
        .show(context, |ui| {
            let (rect, _) = ui.allocate_exact_size(panel_rect.size(), egui::Sense::hover());
            let painter = ui.painter_at(rect);
            let p = |x: f32, y: f32| rect.min + egui::vec2(x, y) * scale;
            let rows_top = HEADER_HEIGHT + SUMMARY_HEIGHT;

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
                [p(14.0, 5.0), p(WIDTH - 14.0, 5.0)],
                egui::Stroke::new(2.0 * scale, egui::Color32::from_rgb(157, 95, 67)),
            );
            painter.circle_filled(
                p(45.0, 39.0),
                30.0 * scale,
                egui::Color32::from_rgb(224, 214, 194),
            );
            painter.circle_stroke(p(45.0, 39.0), 30.0 * scale, egui::Stroke::new(scale, rule));
            painter.image(
                icon.id(),
                egui::Rect::from_min_size(p(17.0, 11.0), egui::vec2(56.0, 56.0) * scale),
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
            painter.text(
                p(84.0, 29.0),
                egui::Align2::LEFT_CENTER,
                "Happiness",
                egui::FontId::proportional(22.0 * scale),
                ink,
            );
            painter.text(
                p(84.0, 52.0),
                egui::Align2::LEFT_CENTER,
                "The mood of each pop class in your empire.",
                egui::FontId::proportional(11.5 * scale),
                muted,
            );
            painter.line_segment(
                [p(14.0, HEADER_HEIGHT - 2.0), p(WIDTH - 14.0, HEADER_HEIGHT - 2.0)],
                egui::Stroke::new(scale, rule),
            );
            for (x, label) in [(18.0, "POP CLASS"), (WIDTH - 20.0, "HAPPINESS")].into_iter() {
                painter.text(
                    p(x, HEADER_HEIGHT + 19.0),
                    if x < WIDTH / 2.0 {
                        egui::Align2::LEFT_CENTER
                    } else {
                        egui::Align2::RIGHT_CENTER
                    },
                    label,
                    egui::FontId::proportional(10.0 * scale),
                    muted,
                );
            }
            painter.line_segment(
                [p(14.0, rows_top), p(WIDTH - 14.0, rows_top)],
                egui::Stroke::new(scale, rule),
            );
            for (class, value) in happiness.iter().enumerate() {
                let y = rows_top + class as f32 * ROW_HEIGHT;
                let row = egui::Rect::from_min_max(p(13.0, y), p(WIDTH - 13.0, y + ROW_HEIGHT));
                if class % 2 == 0 {
                    painter.rect_filled(row, 0.0, egui::Color32::from_rgb(244, 239, 225));
                }
                painter.line_segment(
                    [row.left_bottom(), row.right_bottom()],
                    egui::Stroke::new(0.6 * scale, rule),
                );
                painter.image(
                    class_icons[class].id(),
                    egui::Rect::from_min_size(p(19.0, y + 5.0), egui::vec2(28.0, 28.0) * scale),
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                painter.text(
                    p(56.0, y + ROW_HEIGHT / 2.0),
                    egui::Align2::LEFT_CENTER,
                    POP_CLASS_NAMES[class],
                    egui::FontId::proportional(13.0 * scale),
                    ink,
                );
                painter.text(
                    p(WIDTH - 20.0, y + ROW_HEIGHT / 2.0),
                    egui::Align2::RIGHT_CENTER,
                    format!(
                        "{} ({})",
                        format_hud_number(value.amount),
                        format_hud_delta(value.monthly_delta)
                    ),
                    egui::FontId::proportional(12.0 * scale),
                    happiness_color(value.amount),
                );
            }
        });
}
