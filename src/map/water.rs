//! Geographic water currents shared by the open sea and inland water.
//!
//! The source images each contain four repeats per axis. Sampling them through
//! a continuous, gently warped world-space field breaks up those repeats without
//! moving the texture when the camera pans or introducing seams between tiles.

use bevy_egui::egui;

use super::view::LONGITUDE_SCALE;

const GRID_DEGREES: f32 = 0.75;
const MAX_GRID_VERTICES: usize = 8_192;

#[derive(Clone, Copy, Debug)]
pub(super) enum WaterLayer {
    Swell,
    CrossCurrent,
    Ripples,
}

pub(super) const WATER_LAYERS: [WaterLayer; 3] =
    [WaterLayer::Swell, WaterLayer::CrossCurrent, WaterLayer::Ripples];

#[derive(Clone, Copy)]
struct Current {
    // Geographic distances use the same longitude correction as the map.
    wavelength: [f32; 2],
    rotation: [f32; 2], // cosine, sine
    drift: [f32; 2],
    phase: f32,
    opacity: f32,
}

impl WaterLayer {
    fn current(self) -> Current {
        match self {
            Self::Swell => Current {
                wavelength: [17.73, 12.91],
                rotation: [0.961_055_46, 0.276_355_65],
                drift: [0.028, 0.007],
                phase: 0.0,
                opacity: 70.0,
            },
            Self::CrossCurrent => Current {
                wavelength: [25.87, 19.13],
                rotation: [0.808_027_5, -0.589_144_77],
                drift: [-0.013, 0.020],
                phase: 2.37,
                opacity: 40.0,
            },
            Self::Ripples => Current {
                wavelength: [4.93, 3.67],
                rotation: [0.444_661_53, 0.895_698_67],
                drift: [0.043, -0.017],
                phase: 5.19,
                opacity: 104.0,
            },
        }
    }
}

/// Unwrapped UVs require a texture loaded with `TextureWrapMode::Repeat`.
/// Both positions and phases depend on geography and time, never camera motion.
pub(super) fn water_vertex(
    point: [f32; 2],
    time: f32,
    zoom: f32,
    layer: WaterLayer,
) -> (egui::Pos2, egui::Color32) {
    let current = layer.current();
    let x = point[0] * LONGITUDE_SCALE;
    let y = point[1] - 35.0;
    let phase = current.phase;

    // Two scales bend the image gradually into local currents. Their slopes
    // remain small enough to avoid folds, while their different directions and
    // periods prevent the four-by-four source pattern from forming a lattice.
    let bend_x = (x * 0.23 + y * 0.31 + phase + time * 0.012).sin() * 1.30
        + (x * 0.61 - y * 0.39 - phase * 0.7 - time * 0.017).sin() * 0.47;
    let bend_y = (-x * 0.33 + y * 0.21 - phase - time * 0.010).sin() * 0.90
        + (x * 0.42 + y * 0.49 + phase * 1.3 + time * 0.020).cos() * 0.36;
    let warped_x = x + bend_x + time * current.drift[0];
    let warped_y = y + bend_y + time * current.drift[1];
    let [cos, sin] = current.rotation;
    let uv = egui::pos2(
        (warped_x * cos - warped_y * sin) / current.wavelength[0] + phase,
        -(warped_x * sin + warped_y * cos) / current.wavelength[1] + phase * 0.37,
    );

    // Large quiet patches interrupt the detail rather than covering every part
    // of the sea equally. Their very slow evolution avoids a global pulse.
    let broad = (x * 0.18 - y * 0.24 + phase + time * 0.008).sin();
    let local = (x * 0.43 + y * 0.29 - phase * 1.7 - time * 0.013).sin();
    let patch = smoothstep(0.52 + broad * 0.29 + local * 0.19);
    let close = smoothstep((zoom - 1.2) / 1.8);
    let visibility = match layer {
        WaterLayer::Swell => 1.0 - close * 0.15,
        WaterLayer::CrossCurrent => 1.0,
        WaterLayer::Ripples => close,
    };
    let alpha = current.opacity * visibility * (0.20 + patch * 0.80);
    (uv, egui::Color32::from_white_alpha(alpha.round() as u8))
}

pub(super) fn paint_water(
    painter: &egui::Painter,
    rect: egui::Rect,
    project: impl Fn([f32; 2]) -> egui::Pos2,
    bounds: [f32; 4],
    waves: Option<&egui::TextureHandle>,
    ripples: Option<&egui::TextureHandle>,
    time: f32,
    zoom: f32,
) {
    let clip = rect.intersect(painter.clip_rect());
    if !clip.is_positive() {
        return;
    }
    let Some(grid) = WaterGrid::covering(bounds) else {
        return;
    };
    let painter = painter.with_clip_rect(clip);
    for layer in WATER_LAYERS {
        let Some(texture) = layer_texture(layer, waves, ripples, zoom) else {
            continue;
        };
        painter.add(egui::Shape::mesh(grid.mesh(texture.id(), &project, time, zoom, layer)));
    }
}

/// Paint convex shoreline strips with the same field as the open sea. Keeping
/// the supplied coastline vertices clips the texture to the lake exactly.
pub(super) fn paint_lake(
    painter: &egui::Painter,
    project: impl Fn([f32; 2]) -> egui::Pos2,
    strips: &[[[f32; 2]; 4]],
    waves: Option<&egui::TextureHandle>,
    ripples: Option<&egui::TextureHandle>,
    time: f32,
    zoom: f32,
) {
    for layer in WATER_LAYERS {
        let Some(texture) = layer_texture(layer, waves, ripples, zoom) else {
            continue;
        };
        let mut mesh = egui::Mesh::with_texture(texture.id());
        for strip in strips {
            let base = mesh.vertices.len() as u32;
            for &point in strip {
                let (uv, color) = water_vertex(point, time, zoom, layer);
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: project(point),
                    uv,
                    color,
                });
            }
            mesh.indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        if !mesh.indices.is_empty() {
            painter.add(egui::Shape::mesh(mesh));
        }
    }
}

fn layer_texture<'a>(
    layer: WaterLayer,
    waves: Option<&'a egui::TextureHandle>,
    ripples: Option<&'a egui::TextureHandle>,
    zoom: f32,
) -> Option<&'a egui::TextureHandle> {
    match layer {
        WaterLayer::Swell | WaterLayer::CrossCurrent => waves,
        WaterLayer::Ripples if zoom > 1.2 => ripples,
        WaterLayer::Ripples => None,
    }
}

struct WaterGrid {
    west: i64,
    south: i64,
    columns: usize,
    rows: usize,
    step: f32,
}

impl WaterGrid {
    fn covering([west, south, east, north]: [f32; 4]) -> Option<Self> {
        if ![west, south, east, north].iter().all(|value| value.is_finite())
            || east <= west
            || north <= south
        {
            return None;
        }
        let mut step = GRID_DEGREES;
        loop {
            // Reserve the maximum border padding for these geographic spans
            // before aligning to the world grid. Panning can add one row or
            // column, but must not change the tessellation of shared water.
            let max_columns = (((f64::from(east) - f64::from(west)) / f64::from(step)).ceil()
                as usize)
                .saturating_add(2);
            let max_rows = (((f64::from(north) - f64::from(south)) / f64::from(step)).ceil()
                as usize)
                .saturating_add(2);
            if max_columns.saturating_mul(max_rows) > MAX_GRID_VERTICES {
                step *= 2.0;
                continue;
            }
            let left = (west / step).floor() as i64;
            let bottom = (south / step).floor() as i64;
            let right = (east / step).ceil() as i64;
            let top = (north / step).ceil() as i64;
            let columns = right.saturating_sub(left).saturating_add(1) as usize;
            let rows = top.saturating_sub(bottom).saturating_add(1) as usize;
            return Some(Self {
                west: left,
                south: bottom,
                columns,
                rows,
                step,
            });
        }
    }

    fn mesh(
        &self,
        texture: egui::TextureId,
        project: &impl Fn([f32; 2]) -> egui::Pos2,
        time: f32,
        zoom: f32,
        layer: WaterLayer,
    ) -> egui::Mesh {
        let mut mesh = egui::Mesh::with_texture(texture);
        mesh.vertices.reserve(self.columns * self.rows);
        mesh.indices.reserve((self.columns - 1) * (self.rows - 1) * 6);
        for row in 0..self.rows {
            for column in 0..self.columns {
                let point = [
                    (self.west + column as i64) as f32 * self.step,
                    (self.south + row as i64) as f32 * self.step,
                ];
                let (uv, color) = water_vertex(point, time, zoom, layer);
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: project(point),
                    uv,
                    color,
                });
            }
        }
        for row in 0..self.rows - 1 {
            for column in 0..self.columns - 1 {
                let bottom_left = (row * self.columns + column) as u32;
                let top_left = bottom_left + self.columns as u32;
                mesh.indices.extend([
                    bottom_left,
                    bottom_left + 1,
                    top_left + 1,
                    bottom_left,
                    top_left + 1,
                    top_left,
                ]);
            }
        }
        mesh
    }
}

fn smoothstep(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_field_stays_continuous_across_grid_and_texture_boundaries() {
        for layer in WATER_LAYERS {
            for time in [0.0, 127.0, 1_024.0, 3_600.0] {
                for longitude in [-15.0, 0.0, 15.0, 35.25, 49.5] {
                    let (before, _) = water_vertex([longitude - 0.0001, 35.25], time, 3.0, layer);
                    let (after, _) = water_vertex([longitude + 0.0001, 35.25], time, 3.0, layer);
                    assert!(before.is_finite() && after.is_finite());
                    assert!(before.distance(after) < 0.0002, "grid seam in {layer:?}");
                    let (next_frame, _) =
                        water_vertex([longitude, 35.25], time + 1.0 / 60.0, 3.0, layer);
                    assert!(after.distance(next_frame) < 0.001, "time jump in {layer:?}");
                }
            }
        }
    }

    #[test]
    fn previous_tile_periods_do_not_repeat_the_new_currents() {
        // Include the 4x4 source-image periods and the former whole-image tiles.
        // Differences are compared modulo a quarter UV, the actual source repeat.
        for shift in [[5.0, 0.0], [20.0, 0.0], [0.0, 3.83], [0.0, 15.32]] {
            let mut mismatches = 0;
            for layer in WATER_LAYERS {
                let (a, _) = water_vertex([12.75, 37.5], 43.0, 3.0, layer);
                let (b, _) = water_vertex([12.75 + shift[0], 37.5 + shift[1]], 43.0, 3.0, layer);
                let delta = (b - a) * 4.0;
                if (delta.x - delta.x.round()).abs() > 0.08
                    || (delta.y - delta.y.round()).abs() > 0.08
                {
                    mismatches += 1;
                }
            }
            assert!(mismatches >= 2, "old repeat {shift:?} remains visible");
        }
    }

    #[test]
    fn geographic_grid_is_camera_stable_and_bounds_its_geometry() {
        let bounds = [-18.0, 17.0, 52.0, 61.0];
        let overview = WaterGrid::covering(bounds).unwrap();
        assert_eq!(overview.step, GRID_DEGREES);
        let moved = WaterGrid::covering([-17.9, 17.1, 52.1, 61.1]).unwrap();
        assert_eq!(overview.step, moved.step);
        let wide = WaterGrid::covering([-180.0, -90.0, 180.0, 90.0]).unwrap();
        let close = WaterGrid::covering([10.0, 32.0, 25.0, 43.0]).unwrap();
        assert!(close.columns * close.rows < overview.columns * overview.rows);
        for grid in [overview, moved, wide, close] {
            let mesh = grid.mesh(
                egui::TextureId::Managed(0),
                &|point| egui::pos2(point[0], point[1]),
                20.0,
                3.0,
                WaterLayer::Swell,
            );
            assert!(mesh.vertices.len() <= MAX_GRID_VERTICES);
            assert!(mesh.is_valid());
            assert_eq!(mesh.indices.len(), (grid.columns - 1) * (grid.rows - 1) * 6);
        }
    }

    #[test]
    fn panning_near_the_vertex_budget_keeps_the_same_tessellation() {
        // The aligned view used to fit 8,170 vertices at 0.75 degrees while
        // the panned view needed 8,256, causing the whole sea field to jump.
        let aligned = WaterGrid::covering([0.0, 0.0, 70.5, 63.75]).unwrap();
        for offset in [0.1, 0.4, 0.75, 1.0, 7.4, -0.1, -7.4] {
            let panned = WaterGrid::covering([offset, 0.0, 70.5 + offset, 63.75]).unwrap();
            assert_eq!(aligned.step, panned.step);
            assert!(panned.columns * panned.rows <= MAX_GRID_VERTICES);
        }
    }
}
