use super::*;
use std::time::{Duration, Instant};

#[test]
fn bounded_label_search_preserves_exhaustive_placements() {
    compare_label_searches(&[MIN_ZOOM, 3.7, MAX_ZOOM]);
}

#[test]
#[ignore = "profiles every loading zoom level against the exhaustive search"]
fn profile_historical_province_loading() {
    let zooms: Vec<_> = (0..LABEL_ZOOM_LEVELS).map(label_zoom).collect();
    compare_label_searches(&zooms);
}

fn compare_label_searches(zooms: &[f32]) {
    let atlas = atlas();
    let ctx = egui::Context::default();
    ctx.begin_pass(egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280., 720.))),
        ..Default::default()
    });
    let painter = ctx.layer_painter(egui::LayerId::background());
    let fit = map_geometry(ctx.content_rect(), atlas).3;
    let mut exhaustive_time = Duration::ZERO;
    let mut bounded_time = Duration::ZERO;
    for &zoom in zooms {
        let projection = Projection {
            origin: egui::Pos2::ZERO,
            scale: fit * zoom,
            center: [0.; 2],
        };
        for province in &atlas.provinces {
            let started = Instant::now();
            let expected = exhaustive_label_candidates(&painter, province, &projection, zoom);
            exhaustive_time += started.elapsed();
            let started = Instant::now();
            let actual = label_candidates(&painter, province, &projection, zoom);
            bounded_time += started.elapsed();
            let placements = |candidates: &[LabelPlacement]| {
                candidates
                    .iter()
                    .map(|c| (c.center, c.angle, c.font_size, c.full_name))
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                placements(&actual),
                placements(&expected),
                "{} at zoom {zoom}",
                province.name
            );
        }
    }
    let mut output = ctx.end_pass();
    output.textures_delta.clear();
    eprintln!("Historical labels ({} zoom levels): exhaustive {exhaustive_time:?}, bounded {bounded_time:?}", zooms.len());
}

fn exhaustive_label_candidates(
    painter: &egui::Painter,
    province: &Province,
    projection: &Projection,
    zoom: f32,
) -> Vec<LabelPlacement> {
    let mut centers = Vec::new();
    if province.contains(province.visual_center) {
        centers.push(province.visual_center);
    }
    if province.contains(province.label) && province.label != province.visual_center {
        centers.push(province.label);
    }
    for row in 0..5 {
        for column in 0..5 {
            let center = [
                province.bounds[0]
                    + (province.bounds[2] - province.bounds[0]) * (column as f32 + 0.5) / 5.0,
                province.bounds[1]
                    + (province.bounds[3] - province.bounds[1]) * (row as f32 + 0.5) / 5.0,
            ];
            if province.contains(center) {
                centers.push(center);
            }
        }
    }
    centers.sort_by(|a, b| {
        let distance = |point: &[f32; 2]| {
            let dx = (point[0] - province.visual_center[0]) * LONGITUDE_SCALE;
            let dy = point[1] - province.visual_center[1];
            dx * dx + dy * dy
        };
        distance(a).total_cmp(&distance(b))
    });

    let angles = [
        0.0,
        0.35,
        -0.35,
        0.7,
        -0.7,
        1.05,
        -1.05,
        std::f32::consts::FRAC_PI_2,
        -std::f32::consts::FRAC_PI_2,
    ];
    let preferred_size = (10.5 * zoom.sqrt()).clamp(9.0, 18.0);
    let mut choices = Vec::new();
    let names = if province.name == province.short {
        vec![true]
    } else {
        vec![true, false]
    };
    for step in 0..12 {
        let font_size = preferred_size - step as f32 * 1.25;
        if font_size < 6.5 {
            break;
        }
        for &full_name in &names {
            let name = if full_name {
                &province.name
            } else {
                &province.short
            };
            let galley =
                painter.layout_no_wrap(name.clone(), egui::FontId::proportional(font_size), INK);
            let size = label_fit_size(galley.size());
            for &center in &centers {
                for &angle in &angles {
                    if label_fits_province(province, projection, center, size, angle) {
                        choices.push(LabelPlacement {
                            center,
                            angle,
                            font_size,
                            full_name,
                        });
                        break;
                    }
                }
            }
        }
    }
    // Prefer a centered, horizontal name even if it needs a modest reduction
    // (e.g. Umbria fits straight at 14.25px rather than rotated at 18px).
    // Keep substantially smaller options behind readable ones.
    let steps_per_band = if preferred_size >= 13.0 {
        4.0
    } else {
        2.0
    };
    let band = |candidate: &LabelPlacement| {
        ((preferred_size - candidate.font_size) / (1.25 * steps_per_band)).floor() as i32
    };
    let center_distance = |candidate: &LabelPlacement| {
        let dx = (candidate.center[0] - province.visual_center[0]) * LONGITUDE_SCALE;
        let dy = candidate.center[1] - province.visual_center[1];
        dx * dx + dy * dy
    };
    choices.sort_by(|a, b| {
        band(a)
            .cmp(&band(b))
            .then_with(|| center_distance(a).total_cmp(&center_distance(b)))
            .then_with(|| b.full_name.cmp(&a.full_name))
            .then_with(|| a.angle.abs().total_cmp(&b.angle.abs()))
            .then_with(|| b.font_size.total_cmp(&a.font_size))
    });
    choices.truncate(32);
    choices
}
