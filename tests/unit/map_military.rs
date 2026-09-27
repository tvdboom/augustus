use super::*;

#[test]
fn troop_clusters_leave_landmark_and_other_force_markers_clear() {
    let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
    let desired = egui::pos2(400.0, 300.0);
    let landmarks = [egui::Rect::from_center_size(desired, egui::vec2(110.0, 85.0))];
    let first = clear_anchor(desired, 50.0, viewport, &landmarks, &[]);
    let first_bounds = egui::Rect::from_center_size(first, egui::vec2(105.0, 55.0));
    assert!(!landmarks[0].intersects(first_bounds));
    let second = clear_anchor(desired, 50.0, viewport, &landmarks, &[first_bounds]);
    let second_bounds = egui::Rect::from_center_size(second, egui::vec2(105.0, 55.0));
    assert!(!first_bounds.intersects(second_bounds));
    assert!(!landmarks[0].intersects(second_bounds));
}

#[test]
fn every_unit_has_a_complete_transparent_four_by_four_sheet() {
    for kind in UnitType::ALL {
        let decoded = image::load_from_memory(SHEETS[kind as usize])
            .unwrap_or_else(|error| panic!("{}: {error}", kind.name()))
            .to_rgba8();
        assert!(decoded.width() >= 512 && decoded.height() >= 512, "{} resolution", kind.name());
        assert!(
            decoded.pixels().any(|p| p.0[3] < 255),
            "{} must have transparent background",
            kind.name()
        );
        // Image generation may add one or two boundary pixels. Rendering
        // normalizes the source to a 768px sheet before sampling exact UVs.
        let decoded =
            image::imageops::resize(&decoded, 768, 768, image::imageops::FilterType::Lanczos3);
        let tile_width = decoded.width() / 4;
        let tile_height = decoded.height() / 4;
        for row in 0..4 {
            for column in 0..4 {
                let visible = (row * tile_height..(row + 1) * tile_height).any(|y| {
                    (column * tile_width..(column + 1) * tile_width)
                        .any(|x| decoded.get_pixel(x, y).0[3] > 127)
                });
                assert!(visible, "{} animation row {row} frame {column} is empty", kind.name());
            }
        }
    }
}
