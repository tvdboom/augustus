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
            if (delta.x - delta.x.round()).abs() > 0.08 || (delta.y - delta.y.round()).abs() > 0.08
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
