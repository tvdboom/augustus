//! Owned-population hover card and class-by-province drilldown.

use super::{
    format_hud_number, hud_resource_positions, HudResource, ProvinceOwnership,
    HUD_RESOURCE_GROUP_PADDING, HUD_RESOURCE_WIDTH, MAP_RESOURCE_STRIP_HEIGHT, POP_CLASS_NAMES,
};
use bevy_egui::egui;

const PANEL_WIDTH: f32 = 350.0;
const HEADER_HEIGHT: f32 = 78.0;
const SUMMARY_HEIGHT: f32 = 42.0;
const CLASS_HEIGHT: f32 = 38.0;
const DETAIL_HEADER_HEIGHT: f32 = 29.0;
const DETAIL_ROW_HEIGHT: f32 = 29.0;
const FOOTER_HEIGHT: f32 = 49.0;

pub(super) fn show(
    context: &egui::Context,
    screen: egui::Rect,
    scale: f32,
    date_left: f32,
    player: usize,
    ownership: &ProvinceOwnership,
    population: HudResource,
    icon: &egui::TextureHandle,
    class_icons: &[egui::TextureHandle; 4],
    open: &mut bool,
    open_class: &mut Option<usize>,
) {
    let x = hud_resource_positions()[5];
    if x + HUD_RESOURCE_WIDTH > date_left - HUD_RESOURCE_GROUP_PADDING {
        *open = false;
        *open_class = None;
        return;
    }
    let pointer = context.input(|input| input.pointer.hover_pos());
    let icon_hit = egui::Rect::from_min_size(
        screen.min + egui::vec2(x, 0.0) * scale,
        egui::vec2(HUD_RESOURCE_WIDTH, MAP_RESOURCE_STRIP_HEIGHT) * scale,
    );
    let top = screen.top() + 45.0 * scale;
    let left = (icon_hit.left())
        .max(screen.left() + 8.0 * scale)
        .min(screen.right() - PANEL_WIDTH * scale - 8.0 * scale);
    let base_height = HEADER_HEIGHT + SUMMARY_HEIGHT + CLASS_HEIGHT * 4.0 + FOOTER_HEIGHT;
    let previous_detail_height = detail_height(ownership, player, *open_class, screen, top, scale);
    let previous_rect = egui::Rect::from_min_size(
        egui::pos2(left, top),
        egui::vec2(PANEL_WIDTH, base_height + previous_detail_height) * scale,
    );
    let over_icon = pointer.is_some_and(|point| icon_hit.contains(point));
    let over_panel = *open && pointer.is_some_and(|point| previous_rect.contains(point));
    if !over_icon && !over_panel {
        *open = false;
        *open_class = None;
        return;
    }
    *open = true;

    let class_top = top + (HEADER_HEIGHT + SUMMARY_HEIGHT) * scale;
    let hovered_class = pointer.and_then(|point| {
        let row = (point.y - class_top) / (CLASS_HEIGHT * scale);
        (point.x >= left && point.x < left + PANEL_WIDTH * scale && (0.0..4.0).contains(&row))
            .then_some(row.floor() as usize)
    });
    if let Some(class) = hovered_class {
        *open_class = Some(class);
    } else if let Some(point) = pointer {
        let detail_top = class_top + CLASS_HEIGHT * 4.0 * scale;
        let in_detail = open_class.is_some()
            && point.x >= left
            && point.x < left + PANEL_WIDTH * scale
            && point.y >= detail_top
            && point.y < detail_top + previous_detail_height * scale;
        if !in_detail {
            *open_class = None;
        }
    }
    let detail_height = detail_height(ownership, player, *open_class, screen, top, scale);
    let rect = egui::Rect::from_min_size(
        egui::pos2(left, top),
        egui::vec2(PANEL_WIDTH, base_height + detail_height) * scale,
    );
    let counts = ownership.population_for(player);
    let mut sources =
        open_class.map(|class| ownership.population_sources(player, class)).unwrap_or_default();
    sources.sort_unstable_by(|a, b| a.0.cmp(b.0));
    paint(
        context,
        rect,
        scale,
        population,
        counts,
        icon,
        class_icons,
        *open_class,
        detail_height,
        &sources,
    );
}

fn detail_height(
    ownership: &ProvinceOwnership,
    player: usize,
    class: Option<usize>,
    screen: egui::Rect,
    top: f32,
    scale: f32,
) -> f32 {
    let Some(class) = class else {
        return 0.0;
    };
    let rows = ownership.population_sources(player, class).len().max(1) as f32;
    let base_height = HEADER_HEIGHT + SUMMARY_HEIGHT + CLASS_HEIGHT * 4.0 + FOOTER_HEIGHT;
    let available = (screen.bottom() - top - 12.0 * scale) / scale - base_height;
    DETAIL_HEADER_HEIGHT
        + (rows * DETAIL_ROW_HEIGHT).min((available - DETAIL_HEADER_HEIGHT).max(DETAIL_ROW_HEIGHT))
}

#[allow(clippy::too_many_arguments)]
fn paint(
    context: &egui::Context,
    panel_rect: egui::Rect,
    scale: f32,
    population: HudResource,
    counts: [f64; 4],
    icon: &egui::TextureHandle,
    class_icons: &[egui::TextureHandle; 4],
    open_class: Option<usize>,
    detail_height: f32,
    sources: &[(&str, f64)],
) {
    let ink = egui::Color32::from_rgb(51, 46, 39);
    let muted = egui::Color32::from_rgb(109, 97, 79);
    let rule = egui::Color32::from_rgb(188, 177, 155);
    egui::Area::new(egui::Id::new("augustus_population_sources"))
        .fixed_pos(panel_rect.min)
        .order(egui::Order::Tooltip)
        .show(context, |ui| {
            let (rect, _) = ui.allocate_exact_size(panel_rect.size(), egui::Sense::hover());
            let painter = ui.painter_at(rect);
            let p = |x: f32, y: f32| rect.min + egui::vec2(x, y) * scale;
            let classes_top = HEADER_HEIGHT + SUMMARY_HEIGHT;
            let detail_top = classes_top + CLASS_HEIGHT * 4.0;
            let footer_top = detail_top + detail_height;

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
                "Population",
                egui::FontId::proportional(22.0 * scale),
                ink,
            );
            painter.text(
                p(84.0, 52.0),
                egui::Align2::LEFT_CENTER,
                "The people who work and defend your empire.",
                egui::FontId::proportional(11.5 * scale),
                muted,
            );
            painter.line_segment(
                [p(14.0, HEADER_HEIGHT - 2.0), p(PANEL_WIDTH - 14.0, HEADER_HEIGHT - 2.0)],
                egui::Stroke::new(scale, rule),
            );
            painter.text(
                p(18.0, HEADER_HEIGHT + 19.0),
                egui::Align2::LEFT_CENTER,
                "POP CLASS",
                egui::FontId::proportional(10.0 * scale),
                muted,
            );
            painter.text(
                p(PANEL_WIDTH - 20.0, HEADER_HEIGHT + 19.0),
                egui::Align2::RIGHT_CENTER,
                "TOTAL",
                egui::FontId::proportional(10.0 * scale),
                muted,
            );
            painter.line_segment(
                [p(14.0, classes_top), p(PANEL_WIDTH - 14.0, classes_top)],
                egui::Stroke::new(scale, rule),
            );
            for class in 0..4 {
                let y = classes_top + class as f32 * CLASS_HEIGHT;
                let row =
                    egui::Rect::from_min_max(p(13.0, y), p(PANEL_WIDTH - 13.0, y + CLASS_HEIGHT));
                if open_class == Some(class) {
                    painter.rect_filled(row, 3.0 * scale, egui::Color32::from_rgb(226, 212, 184));
                } else if class % 2 == 0 {
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
                    p(56.0, y + CLASS_HEIGHT / 2.0),
                    egui::Align2::LEFT_CENTER,
                    POP_CLASS_NAMES[class],
                    egui::FontId::proportional(13.0 * scale),
                    ink,
                );
                painter.text(
                    p(PANEL_WIDTH - 20.0, y + CLASS_HEIGHT / 2.0),
                    egui::Align2::RIGHT_CENTER,
                    format_hud_number(counts[class]),
                    egui::FontId::proportional(13.0 * scale),
                    ink,
                );
            }
            if let Some(class) = open_class {
                painter.rect_filled(
                    egui::Rect::from_min_max(
                        p(13.0, detail_top),
                        p(PANEL_WIDTH - 13.0, detail_top + DETAIL_HEADER_HEIGHT),
                    ),
                    0.0,
                    egui::Color32::from_rgb(222, 211, 188),
                );
                painter.text(
                    p(18.0, detail_top + 15.0),
                    egui::Align2::LEFT_CENTER,
                    format!("{} BY PROVINCE", POP_CLASS_NAMES[class].to_uppercase()),
                    egui::FontId::proportional(10.0 * scale),
                    ink,
                );
                let body_rect = egui::Rect::from_min_size(
                    p(13.0, detail_top + DETAIL_HEADER_HEIGHT),
                    egui::vec2(PANEL_WIDTH - 26.0, detail_height - DETAIL_HEADER_HEIGHT) * scale,
                );
                let mut body_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt(("population_province_list", class))
                        .max_rect(body_rect),
                );
                body_ui.spacing_mut().item_spacing.y = 0.0;
                body_ui.spacing_mut().scroll.bar_width = 5.0 * scale;
                egui::ScrollArea::vertical()
                    .max_height(body_rect.height())
                    .auto_shrink([false, false])
                    .show(&mut body_ui, |list| {
                        for (index, (name, count)) in sources.iter().enumerate() {
                            let (row, _) = list.allocate_exact_size(
                                egui::vec2(list.available_width(), DETAIL_ROW_HEIGHT * scale),
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
                                egui::FontId::proportional(12.0 * scale),
                                ink,
                            );
                            row_painter.text(
                                egui::pos2(rect.right() - 20.0 * scale, row.center().y),
                                egui::Align2::RIGHT_CENTER,
                                format_hud_number(*count),
                                egui::FontId::proportional(12.0 * scale),
                                ink,
                            );
                        }
                    });
            }
            let footer = egui::Rect::from_min_max(
                p(4.0, footer_top),
                p(PANEL_WIDTH - 4.0, footer_top + FOOTER_HEIGHT - 4.0),
            );
            painter.rect_filled(footer, 5.0 * scale, egui::Color32::from_rgb(222, 211, 188));
            painter.line_segment(
                [p(14.0, footer_top), p(PANEL_WIDTH - 14.0, footer_top)],
                egui::Stroke::new(scale, rule),
            );
            painter.text(
                p(18.0, footer_top + 24.0),
                egui::Align2::LEFT_CENTER,
                "TOTAL POPS",
                egui::FontId::proportional(11.0 * scale),
                ink,
            );
            painter.text(
                p(PANEL_WIDTH - 20.0, footer_top + 24.0),
                egui::Align2::RIGHT_CENTER,
                format_hud_number(population.amount),
                egui::FontId::proportional(17.0 * scale),
                ink,
            );
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_can_move_from_population_icon_to_class_and_province_detail() {
        let context = egui::Context::default();
        let pixel = egui::ColorImage::from_rgba_unmultiplied([1, 1], &[255, 255, 255, 255]);
        let icon = context.load_texture("test-pop", pixel.clone(), egui::TextureOptions::LINEAR);
        let class_icons = std::array::from_fn(|index| {
            context.load_texture(
                format!("test-class-{index}"),
                pixel.clone(),
                egui::TextureOptions::LINEAR,
            )
        });
        let mut ownership = ProvinceOwnership::default();
        ownership.start_game(&[egui::Color32::RED]);
        let total = ownership.population_for(0).into_iter().sum();
        let population = HudResource {
            amount: total,
            monthly_delta: 1.0,
        };
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 900.0));
        let x = hud_resource_positions()[5] + 40.0;
        let mut open = false;
        let mut class = None;
        for (pointer, expected_open, expected_class) in [
            (egui::pos2(x, 24.0), true, None),
            (
                egui::pos2(x, 45.0 + HEADER_HEIGHT + SUMMARY_HEIGHT + CLASS_HEIGHT * 2.5),
                true,
                Some(2),
            ),
            (
                egui::pos2(
                    x,
                    45.0 + HEADER_HEIGHT
                        + SUMMARY_HEIGHT
                        + CLASS_HEIGHT * 4.0
                        + DETAIL_HEADER_HEIGHT
                        + 10.0,
                ),
                true,
                Some(2),
            ),
            (egui::pos2(1200.0, 500.0), false, None),
        ] {
            context.begin_pass(egui::RawInput {
                screen_rect: Some(screen),
                events: vec![egui::Event::PointerMoved(pointer)],
                ..Default::default()
            });
            show(
                &context,
                screen,
                1.0,
                1200.0,
                0,
                &ownership,
                population,
                &icon,
                &class_icons,
                &mut open,
                &mut class,
            );
            assert_eq!((open, class), (expected_open, expected_class));
            let mut output = context.end_pass();
            output.textures_delta.clear();
        }
    }
}
