//! Decorative map edge frame.

use super::*;

pub(super) fn paint_map_edge_frame(
    ctx: &egui::Context,
    scale: f32,
    standard: &egui::TextureHandle,
    clock: &GameClock,
    paused: bool,
    speed_button_state: [(bool, bool); 2],
) {
    let screen = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("augustus_map_edge_frame"),
    ));
    let p = |x: f32, y: f32| screen.min + egui::vec2(x, y) * scale;

    // Resource groups share the parchment strip with the date controls.
    let strip_right = map_resource_strip_right(screen, scale);
    let date_left = strip_right - MAP_DATE_SECTION_WIDTH;
    if map_resource_strip_visible(screen, scale) {
        painter.add(egui::Shape::convex_polygon(
            vec![
                p(MAP_RESOURCE_STRIP_LEFT, 0.0),
                p(strip_right, 0.0),
                p(strip_right - 26.0, MAP_RESOURCE_STRIP_HEIGHT),
                p(MAP_RESOURCE_STRIP_LEFT, MAP_RESOURCE_STRIP_HEIGHT),
            ],
            egui::Color32::from_rgb(236, 231, 216),
            egui::Stroke::new(scale, egui::Color32::from_rgb(144, 133, 110)),
        ));
        painter.line_segment(
            [p(MAP_RESOURCE_STRIP_LEFT + 1.0, 44.0), p(strip_right - 29.0, 44.0)],
            egui::Stroke::new(scale, egui::Color32::from_rgb(184, 164, 129)),
        );
        painter.line_segment(
            [p(date_left, 4.0), p(date_left, 43.0)],
            egui::Stroke::new(scale, egui::Color32::from_rgb(192, 183, 164)),
        );
        let date_center = date_left + 105.0;
        let date_rect = painter.text(
            p(date_center, 17.0),
            egui::Align2::CENTER_CENTER,
            clock.date_label(),
            egui::FontId::proportional(18.0 * scale),
            egui::Color32::from_rgb(35, 35, 32),
        );
        for (index, (offset, symbol, available)) in [
            (36.0, "−", clock.speed_step > MIN_SPEED_STEP),
            (174.0, "+", clock.speed_step < MAX_SPEED_STEP),
        ]
        .into_iter()
        .enumerate()
        {
            let (hovered, pressed) = if available {
                speed_button_state[index]
            } else {
                (false, false)
            };
            let center = p(date_left + offset, 24.0);
            let fill = if pressed {
                egui::Color32::from_rgb(177, 150, 111)
            } else if hovered {
                egui::Color32::from_rgb(210, 187, 147)
            } else {
                egui::Color32::from_rgb(227, 216, 196)
            };
            painter.rect_filled(
                egui::Rect::from_center_size(center, egui::vec2(27.0, 27.0) * scale),
                3.0 * scale,
                fill,
            );
            painter.rect_stroke(
                egui::Rect::from_center_size(center, egui::vec2(27.0, 27.0) * scale),
                3.0 * scale,
                egui::Stroke::new(scale, egui::Color32::from_rgb(156, 139, 110)),
                egui::StrokeKind::Inside,
            );
            painter.text(
                center,
                egui::Align2::CENTER_CENTER,
                symbol,
                egui::FontId::proportional(21.0 * scale),
                if available {
                    egui::Color32::from_rgb(35, 35, 32)
                } else {
                    egui::Color32::from_rgb(156, 139, 110)
                },
            );
        }
        let active_bars = if paused {
            0
        } else {
            (clock.speed_step + 3).clamp(0, 5) as usize
        };
        let bar_gap = 2.0 * scale;
        let bar_width = ((date_rect.width() - bar_gap * 4.0) / 5.0).max(scale);
        for index in 0..5 {
            let rect = egui::Rect::from_min_size(
                egui::pos2(
                    date_rect.left() + index as f32 * (bar_width + bar_gap),
                    p(date_center, 39.0).y - 2.0 * scale,
                ),
                egui::vec2(bar_width, 4.0 * scale),
            );
            painter.rect_filled(
                rect,
                0.5 * scale,
                if index < active_bars {
                    egui::Color32::from_rgb(142, 103, 63)
                } else {
                    egui::Color32::from_rgb(202, 192, 173)
                },
            );
        }
    }

    // Each player color has its own transparent image of the entire standard
    // and rail. Menu glyphs are painted over the cloth afterward.
    let height = map_standard_height(screen, scale);
    let mut mesh = egui::Mesh::with_texture(standard.id());
    let rows = [
        (20.0, MAP_STANDARD_WIDTH),
        (300.0, MAP_STANDARD_WIDTH),
        (440.0, MAP_STANDARD_RAIL_IMAGE_WIDTH),
        (1684.0, MAP_STANDARD_RAIL_IMAGE_WIDTH),
    ];
    for (source_y, width) in rows {
        let y = (source_y - 20.0) / (1684.0 - 20.0) * height;
        for (x, u) in [(0.0, 7.0 / 220.0), (width, 1.0)] {
            mesh.vertices.push(egui::epaint::Vertex {
                pos: p(x, y),
                uv: egui::pos2(u, source_y / 1684.0),
                color: egui::Color32::WHITE,
            });
        }
    }
    for row in 0..3 {
        let top = row * 2;
        mesh.indices.extend_from_slice(&[top, top + 1, top + 2, top + 1, top + 3, top + 2]);
    }
    painter.add(egui::Shape::Mesh(mesh.into()));
}
