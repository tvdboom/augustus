//! Small white map symbols shared with the province overview.

use bevy_egui::egui;

#[derive(Clone, Copy)]
pub(crate) enum MarkerIcon {
    City,
    Wonder,
}

const CITY_LINES: &[&[[f32; 2]]] = &[
    &[[0.11, 0.39], [0.50, 0.16], [0.89, 0.39]],
    &[[0.14, 0.44], [0.86, 0.44]],
    &[[0.19, 0.50], [0.19, 0.77]],
    &[[0.39, 0.50], [0.39, 0.77]],
    &[[0.61, 0.50], [0.61, 0.77]],
    &[[0.81, 0.50], [0.81, 0.77]],
    &[[0.12, 0.82], [0.88, 0.82]],
];

const WONDER_LINES: &[&[[f32; 2]]] = &[
    &[[0.10, 0.82], [0.50, 0.17], [0.90, 0.82], [0.10, 0.82]],
    &[[0.50, 0.17], [0.61, 0.82]],
    &[[0.32, 0.59], [0.75, 0.59]],
];

pub(crate) fn paint_marker_icon(
    painter: &egui::Painter,
    rect: egui::Rect,
    icon: MarkerIcon,
    color: egui::Color32,
) {
    if color.a() == 0 {
        return;
    }
    let size = rect.width().min(rect.height());
    let origin = rect.center() - egui::vec2(size, size) * 0.5;
    let lines = match icon {
        MarkerIcon::City => CITY_LINES,
        MarkerIcon::Wonder => WONDER_LINES,
    };
    let ink = egui::Stroke::new((size * 0.082).max(1.3), color);
    let casing = egui::Stroke::new(
        ink.width + (size * 0.095).max(1.4),
        egui::Color32::from_rgba_unmultiplied(27, 37, 43, (u16::from(color.a()) * 3 / 4) as u8),
    );
    let paths: Vec<Vec<_>> = lines
        .iter()
        .map(|line| line.iter().map(|[x, y]| origin + egui::vec2(x * size, y * size)).collect())
        .collect();
    for points in &paths {
        painter.add(egui::Shape::line(points.clone(), casing));
    }
    for points in paths {
        painter.add(egui::Shape::line(points, ink));
    }
}
