//! Shared Governance and province policy card styling.

use bevy_egui::egui;

pub(super) const ROW_HEIGHT: f32 = 95.0;
pub(super) const SECTION_HEIGHT: f32 = 34.0;
pub(super) const INK: egui::Color32 = egui::Color32::from_rgb(57, 43, 37);
const RULE: egui::Color32 = egui::Color32::from_rgb(191, 171, 143);

pub(super) fn compact_decimal(value: f64) -> String {
    let number = format!("{value:.1}").trim_end_matches(".0").to_owned();
    if number == "-0" {
        "0".into()
    } else {
        number
    }
}

pub(super) fn signed_decimal(value: f64) -> String {
    let number = compact_decimal(value);
    if value > 0.0 && number != "0" {
        format!("+{number}")
    } else {
        number
    }
}

pub(super) fn hover_effects(
    response: egui::Response,
    scale: f32,
    effects: &[&str],
) -> egui::Response {
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

pub(super) fn section(ui: &mut egui::Ui, scale: f32, title: &str) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), SECTION_HEIGHT * scale),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(
        rect.shrink2(egui::vec2(0.0, 3.0 * scale)),
        scale,
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

pub(super) fn card(ui: &mut egui::Ui, scale: f32, title: &str) -> egui::Rect {
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
    rect
}

pub(super) fn effect_rect(
    painter: &egui::Painter,
    rect: egui::Rect,
    scale: f32,
    count: usize,
    index: usize,
) -> egui::Rect {
    let stack_height = (count as f32 * 19.0 + count.saturating_sub(1) as f32 * 3.0) * scale;
    let badge = egui::Rect::from_min_size(
        egui::pos2(
            rect.right() - 103.0 * scale,
            rect.bottom() - 7.0 * scale - stack_height + index as f32 * 22.0 * scale,
        ),
        egui::vec2(93.0, 19.0) * scale,
    );
    painter.rect_filled(badge, 3.0 * scale, egui::Color32::from_rgb(235, 226, 208));
    badge
}

pub(super) fn choice_rect(rect: egui::Rect, scale: f32, index: usize) -> egui::Rect {
    egui::Rect::from_min_size(
        rect.left_bottom() + egui::vec2(10.0 + index as f32 * 100.0, -38.0) * scale,
        egui::vec2(94.0, 29.0) * scale,
    )
}

pub(super) fn choice<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    chip: egui::Rect,
    id: egui::Id,
    title: &str,
    label: &str,
    selected: &mut T,
    candidate: T,
    accent: egui::Color32,
    scale: f32,
) -> (bool, egui::Response) {
    let response =
        ui.interact(chip, id, egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::RadioButton,
            *selected == candidate,
            format!("{title}: {label}"),
        )
    });
    let changed = response.clicked() && *selected != candidate;
    if changed {
        *selected = candidate;
    }
    let active = *selected == candidate;
    let pressed = response.is_pointer_button_down_on();
    let painter = ui.painter_at(chip);
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
        label,
        egui::FontId::proportional(12.5 * scale),
        INK,
    );
    (changed, response)
}
