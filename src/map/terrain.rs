//! Decorative rivers. All positions are longitude/latitude pairs.

use std::sync::OnceLock;

use bevy_egui::egui;
use serde::Deserialize;

#[derive(Deserialize)]
struct River {
    major: bool,
    #[serde(default)]
    minor: bool,
    mouth_start: bool,
    mouth_end: bool,
    points: Vec<[f32; 2]>,
}

fn rivers() -> &'static [River] {
    static RIVERS: OnceLock<Vec<River>> = OnceLock::new();
    RIVERS.get_or_init(|| {
        serde_json::from_str(include_str!("../../assets/map/terrain-rivers.json"))
            .expect("generated terrain rivers must be valid")
    })
}

pub(super) fn paint_rivers(
    painter: &egui::Painter,
    project: impl Fn([f32; 2]) -> egui::Pos2,
    zoom: f32,
    sea_color: egui::Color32,
) {
    let clip = painter.clip_rect();
    let detail = ((zoom - 1.0) / 3.0).clamp(0.0, 1.0);
    for river in rivers() {
        let points: Vec<_> = river.points.iter().copied().map(&project).collect();
        if !visible(&points, clip, 5.0) {
            continue;
        }
        paint_river(painter, river, &points, detail, sea_color);
    }
}

fn paint_river(
    painter: &egui::Painter,
    river: &River,
    points: &[egui::Pos2],
    detail: f32,
    sea_color: egui::Color32,
) {
    if points.len() < 2 {
        return;
    }
    let width = if river.minor {
        0.95 + detail * 0.60
    } else if river.major {
        1.8 + detail * 0.85
    } else {
        1.15 + detail * 0.85
    };
    let casing = egui::Color32::from_rgba_unmultiplied(
        52,
        76,
        89,
        if river.minor {
            100
        } else {
            135
        },
    );
    let water = egui::Color32::from_rgba_unmultiplied(
        123,
        190,
        198,
        if river.minor {
            195
        } else {
            225
        },
    );
    let fade_length = 12.0 + detail * 18.0;
    let start = if river.mouth_start {
        mouth_cut(points, true, fade_length)
    } else {
        0
    };
    let end = if river.mouth_end {
        mouth_cut(points, false, fade_length)
    } else {
        points.len() - 1
    };
    let body = if start < end {
        &points[start..=end]
    } else {
        &[]
    };

    if body.len() >= 2 {
        painter.add(egui::Shape::line(body.to_vec(), egui::Stroke::new(width + 1.3, casing)));
    }
    if river.mouth_start {
        paint_mouth(
            painter,
            &points[..=start],
            true,
            width,
            casing,
            water,
            sea_color,
            fade_length,
            false,
        );
    }
    if river.mouth_end {
        paint_mouth(
            painter,
            &points[end..],
            false,
            width,
            casing,
            water,
            sea_color,
            fade_length,
            false,
        );
    }
    if body.len() >= 2 {
        painter.add(egui::Shape::line(body.to_vec(), egui::Stroke::new(width, water)));
        if detail > 0.5 && !river.minor {
            painter.add(egui::Shape::line(
                body.to_vec(),
                egui::Stroke::new(0.55, egui::Color32::from_rgba_unmultiplied(197, 227, 220, 65)),
            ));
        }
    }
    if river.mouth_start {
        paint_mouth(
            painter,
            &points[..=start],
            true,
            width,
            casing,
            water,
            sea_color,
            fade_length,
            true,
        );
    }
    if river.mouth_end {
        paint_mouth(
            painter,
            &points[end..],
            false,
            width,
            casing,
            water,
            sea_color,
            fade_length,
            true,
        );
    }
}

fn mouth_cut(points: &[egui::Pos2], from_start: bool, fade_length: f32) -> usize {
    let mut distance = 0.0;
    if from_start {
        for index in 1..points.len() {
            distance += points[index].distance(points[index - 1]);
            if distance >= fade_length {
                return index;
            }
        }
        points.len() - 1
    } else {
        for index in (0..points.len() - 1).rev() {
            distance += points[index].distance(points[index + 1]);
            if distance >= fade_length {
                return index;
            }
        }
        0
    }
}

fn paint_mouth(
    painter: &egui::Painter,
    points: &[egui::Pos2],
    from_start: bool,
    width: f32,
    casing: egui::Color32,
    water: egui::Color32,
    sea: egui::Color32,
    fade_length: f32,
    water_pass: bool,
) {
    let ordered: Vec<_> = if from_start {
        points.to_vec()
    } else {
        points.iter().rev().copied().collect()
    };
    let mut distance = 0.0;
    for (index, pair) in ordered.windows(2).enumerate() {
        let segment = pair[0].distance(pair[1]);
        let amount = if index == 0 {
            0.0
        } else {
            ((distance + segment * 0.5) / fade_length).clamp(0.0, 1.0)
        };
        let (stroke_width, color) = if water_pass {
            (width, mix_color(sea, water, amount))
        } else {
            (
                width + 1.3,
                egui::Color32::from_rgba_unmultiplied(
                    casing.r(),
                    casing.g(),
                    casing.b(),
                    (f32::from(casing.a()) * amount).round() as u8,
                ),
            )
        };
        painter.line_segment([pair[0], pair[1]], egui::Stroke::new(stroke_width, color));
        distance += segment;
    }
}

fn mix_color(a: egui::Color32, b: egui::Color32, amount: f32) -> egui::Color32 {
    let amount = amount.clamp(0.0, 1.0);
    let mix = |left: u8, right: u8| {
        (f32::from(left) + (f32::from(right) - f32::from(left)) * amount).round() as u8
    };
    egui::Color32::from_rgba_unmultiplied(
        mix(a.r(), b.r()),
        mix(a.g(), b.g()),
        mix(a.b(), b.b()),
        mix(a.a(), b.a()),
    )
}

fn visible(points: &[egui::Pos2], clip: egui::Rect, margin: f32) -> bool {
    points.windows(2).any(|pair| segment_visible(pair[0], pair[1], clip, margin))
}

fn segment_visible(a: egui::Pos2, b: egui::Pos2, clip: egui::Rect, margin: f32) -> bool {
    egui::Rect::from_two_pos(a, b).expand(margin).intersects(clip)
}
