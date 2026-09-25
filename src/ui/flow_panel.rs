//! Shared income and outflow card for coin and influence.

use super::{
    format_hud_number, hud_delta_color, hud_resource_positions, HUD_RESOURCE_GROUP_PADDING,
    HUD_RESOURCE_WIDTH, MAP_RESOURCE_STRIP_HEIGHT,
};
use bevy_egui::egui;

const PANEL_WIDTH: f32 = 430.0;
const HEADER_HEIGHT: f32 = 78.0;
const SECTION_HEIGHT: f32 = 30.0;
const ROW_HEIGHT: f32 = 29.0;

pub(super) struct FlowPanel<'a> {
    pub resource_index: usize,
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub income: &'a [(&'static str, f64)],
    pub outflow: &'a [(&'static str, f64)],
}

pub(super) fn show(
    context: &egui::Context,
    screen: egui::Rect,
    scale: f32,
    date_left: f32,
    icon: &egui::TextureHandle,
    panel: &FlowPanel<'_>,
    open: &mut bool,
) {
    let x = hud_resource_positions()[panel.resource_index];
    let pointer = context.input(|input| input.pointer.hover_pos());
    let hit = egui::Rect::from_min_size(
        screen.min + egui::vec2(x, 0.0) * scale,
        egui::vec2(HUD_RESOURCE_WIDTH, MAP_RESOURCE_STRIP_HEIGHT) * scale,
    );
    let hovered = x + HUD_RESOURCE_WIDTH <= date_left - HUD_RESOURCE_GROUP_PADDING
        && pointer.is_some_and(|point| hit.contains(point));
    if !hovered && !*open {
        return;
    }

    let top = screen.top() + 45.0 * scale;
    let desired_body =
        SECTION_HEIGHT * 2.0 + ROW_HEIGHT * (panel.income.len() + panel.outflow.len() + 2) as f32;
    let available_body = (screen.bottom() - top - 12.0 * scale) / scale - HEADER_HEIGHT;
    let body_height = desired_body.min(available_body.max(ROW_HEIGHT));
    let width = PANEL_WIDTH * scale;
    let left = (screen.left() + x * scale)
        .max(screen.left() + 8.0 * scale)
        .min(screen.right() - width - 8.0 * scale);
    let rect = egui::Rect::from_min_size(
        egui::pos2(left, top),
        egui::vec2(width, (HEADER_HEIGHT + body_height) * scale),
    );
    if hovered || pointer.is_some_and(|point| rect.contains(point)) {
        *open = true;
        paint(context, rect, scale, body_height, icon, panel);
    } else {
        *open = false;
    }
}

fn paint(
    context: &egui::Context,
    panel_rect: egui::Rect,
    scale: f32,
    body_height: f32,
    icon: &egui::TextureHandle,
    panel: &FlowPanel<'_>,
) {
    let ink = egui::Color32::from_rgb(51, 46, 39);
    let muted = egui::Color32::from_rgb(109, 97, 79);
    let rule = egui::Color32::from_rgb(188, 177, 155);

    egui::Area::new(egui::Id::new(panel.id))
        .fixed_pos(panel_rect.min)
        .order(egui::Order::Tooltip)
        .show(context, |ui| {
            let (rect, _) = ui.allocate_exact_size(panel_rect.size(), egui::Sense::hover());
            let painter = ui.painter_at(rect);
            let p = |x: f32, y: f32| rect.min + egui::vec2(x, y) * scale;

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
                [p(14.0, 5.0), p(PANEL_WIDTH - 14.0, 5.0)],
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
                panel.title,
                egui::FontId::proportional(22.0 * scale),
                ink,
            );
            painter.text(
                p(84.0, 52.0),
                egui::Align2::LEFT_CENTER,
                panel.description,
                egui::FontId::proportional(12.0 * scale),
                muted,
            );
            painter.line_segment(
                [p(14.0, HEADER_HEIGHT - 2.0), p(PANEL_WIDTH - 14.0, HEADER_HEIGHT - 2.0)],
                egui::Stroke::new(scale, rule),
            );

            let body_rect = egui::Rect::from_min_size(
                p(13.0, HEADER_HEIGHT),
                egui::vec2(PANEL_WIDTH - 26.0, body_height) * scale,
            );
            let mut body_ui =
                ui.new_child(egui::UiBuilder::new().id_salt("flow_list").max_rect(body_rect));
            body_ui.spacing_mut().item_spacing.y = 0.0;
            body_ui.spacing_mut().scroll.bar_width = 5.0 * scale;
            body_ui.spacing_mut().scroll.bar_inner_margin = 2.0 * scale;
            body_ui.spacing_mut().scroll.bar_outer_margin = 0.0;
            egui::ScrollArea::vertical()
                .max_height(body_height * scale)
                .auto_shrink([false, false])
                .show(&mut body_ui, |list| {
                    flow_section(list, scale, "INCOME", panel.income, true, ink, muted, rule);
                    flow_section(list, scale, "OUTFLOW", panel.outflow, false, ink, muted, rule);
                });
        });
}

#[allow(clippy::too_many_arguments)]
fn flow_section(
    ui: &mut egui::Ui,
    scale: f32,
    heading: &str,
    entries: &[(&str, f64)],
    income: bool,
    ink: egui::Color32,
    muted: egui::Color32,
    rule: egui::Color32,
) {
    let (header, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), SECTION_HEIGHT * scale),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(header);
    painter.rect_filled(header, 0.0, egui::Color32::from_rgb(222, 211, 188));
    painter.line_segment(
        [header.left_bottom(), header.right_bottom()],
        egui::Stroke::new(scale, rule),
    );
    painter.text(
        header.left_center() + egui::vec2(5.0 * scale, 0.0),
        egui::Align2::LEFT_CENTER,
        heading,
        egui::FontId::proportional(11.5 * scale),
        ink,
    );

    for (index, &(label, value)) in entries.iter().enumerate() {
        let (row, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), ROW_HEIGHT * scale),
            egui::Sense::hover(),
        );
        let painter = ui.painter_at(row);
        if index % 2 == 0 {
            painter.rect_filled(row, 0.0, egui::Color32::from_rgb(244, 239, 225));
        }
        painter.line_segment(
            [row.left_bottom(), row.right_bottom()],
            egui::Stroke::new(0.6 * scale, rule),
        );
        painter.text(
            row.left_center() + egui::vec2(5.0 * scale, 0.0),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(13.0 * scale),
            ink,
        );
        painter.text(
            row.right_center() - egui::vec2(10.0 * scale, 0.0),
            egui::Align2::RIGHT_CENTER,
            format_hud_number(value),
            egui::FontId::proportional(13.0 * scale),
            flow_color(value, income, muted),
        );
    }

    let total: f64 = entries.iter().map(|(_, value)| value).sum();
    let (row, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), ROW_HEIGHT * scale),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(row);
    painter.rect_filled(row, 0.0, egui::Color32::from_rgb(230, 220, 198));
    painter.line_segment([row.left_bottom(), row.right_bottom()], egui::Stroke::new(scale, rule));
    painter.text(
        row.left_center() + egui::vec2(5.0 * scale, 0.0),
        egui::Align2::LEFT_CENTER,
        if income {
            "TOTAL INCOME"
        } else {
            "TOTAL OUTFLOW"
        },
        egui::FontId::proportional(11.5 * scale),
        ink,
    );
    painter.text(
        row.right_center() - egui::vec2(10.0 * scale, 0.0),
        egui::Align2::RIGHT_CENTER,
        format_hud_number(total),
        egui::FontId::proportional(13.0 * scale),
        flow_color(total, income, muted),
    );
}

fn flow_color(value: f64, income: bool, muted: egui::Color32) -> egui::Color32 {
    if value == 0.0 {
        muted
    } else {
        hud_delta_color(if income {
            value
        } else {
            -value
        })
    }
}
