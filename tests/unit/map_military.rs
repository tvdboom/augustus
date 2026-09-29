use super::*;

#[test]
fn map_shows_present_front_rear_and_flank_types_even_below_share_threshold() {
    let mut world = MilitaryWorld::new(1);
    let owner = ForceOwner::Player(0);
    for _ in 0..8 {
        world.seed_unit(0, owner, UnitType::LightInfantry).unwrap();
    }
    for kind in [UnitType::HeavyInfantry, UnitType::Archers, UnitType::WarCamels] {
        world.seed_unit(0, owner, kind).unwrap();
    }
    let units = &world.provinces[0].forces[&owner];
    let plan = BattlePlan {
        primary_unit_type: UnitType::HeavyInfantry,
        secondary_unit_type: UnitType::Archers,
        flank_unit_type: UnitType::WarCamels,
        ..BattlePlan::default()
    };
    assert_eq!(
        map_representatives(units, plan, &world.config),
        vec![UnitType::HeavyInfantry, UnitType::Archers, UnitType::WarCamels]
    );
}

#[test]
fn map_falls_back_to_surviving_types_without_repeating_a_sprite() {
    let mut world = MilitaryWorld::new(1);
    let owner = ForceOwner::Player(0);
    for kind in [UnitType::HeavyInfantry, UnitType::Archers, UnitType::HeavyCavalry] {
        world.seed_unit(0, owner, kind).unwrap();
    }
    let units = &world.provinces[0].forces[&owner];
    assert_eq!(
        map_representatives(units, BattlePlan::default(), &world.config),
        vec![UnitType::HeavyInfantry, UnitType::Archers, UnitType::HeavyCavalry]
    );
    assert_eq!(
        map_representatives(&units[..1], BattlePlan::default(), &world.config),
        vec![UnitType::HeavyInfantry]
    );
    let mut depleted = units.to_vec();
    depleted[2].current_manpower = 0.;
    assert_eq!(
        map_representatives(&depleted, BattlePlan::default(), &world.config),
        vec![UnitType::HeavyInfantry, UnitType::Archers]
    );
}

#[test]
fn painted_troops_retain_owner_and_province_for_army_navigation() {
    let context = egui::Context::default();
    let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900., 700.));
    let province =
        atlas().provinces.iter().position(|province| province.name == "Samnium").unwrap();
    let mut world = MilitaryWorld::new(atlas().provinces.len());
    let owner = ForceOwner::Player(0);
    world.seed_unit(province, owner, UnitType::HeavyInfantry).unwrap();
    let projection = Projection {
        origin: viewport.center(),
        scale: 100.,
        center: atlas().provinces[province].visual_center,
    };
    let mut anchors = Anchors::default();
    context.begin_pass(egui::RawInput::default());
    let painter = context.layer_painter(egui::LayerId::background());
    let markers = paint(
        &painter,
        &world,
        &ProvinceOwnership::default(),
        &projection,
        4.,
        0.,
        viewport,
        &[],
        &[],
        &[],
        &mut anchors,
    );
    let hits = context
        .data(|data| data.get_temp::<Vec<ArmyHit>>(egui::Id::new("map-army-hit-targets")))
        .unwrap();
    assert!(!hits.is_empty());
    assert_eq!(hits.len(), markers.len());
    assert!(hits
        .iter()
        .all(|hit| hit.province == province && hit.owner == owner && hit.movement.is_none()));
    context.data_mut(|data| data.insert_temp(egui::Id::new("map-army-click"), hits[0]));
    assert_eq!(take_army_click(&context).unwrap().owner, owner);
    assert!(take_army_click(&context).is_none(), "Each click is consumed only once");
    let mut output = context.end_pass();
    output.textures_delta.clear();
}

#[test]
fn battle_troops_remain_clickable_with_foreign_deployment_hidden() {
    let context = egui::Context::default();
    let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900., 700.));
    let province =
        atlas().provinces.iter().position(|province| province.name == "Samnium").unwrap();
    let mut world = MilitaryWorld::new(atlas().provinces.len());
    let own = ForceOwner::Player(0);
    let enemy = ForceOwner::Player(1);
    world.seed_unit(province, own, UnitType::HeavyInfantry).unwrap();
    world.seed_unit(province, enemy, UnitType::HeavyInfantry).unwrap();
    world
        .start_battle(
            province,
            &[own],
            &[enemy],
            None,
            None,
            crate::game::military::MilitaryTerrain::Plains,
            0,
            5,
        )
        .unwrap();
    world.battles[0].defenders.formation = Default::default();
    let projection = Projection {
        origin: viewport.center(),
        scale: 100.,
        center: atlas().provinces[province].visual_center,
    };
    let mut anchors = Anchors::default();
    context.begin_pass(egui::RawInput::default());
    let painter = context.layer_painter(egui::LayerId::background());
    paint(
        &painter,
        &world,
        &ProvinceOwnership::default(),
        &projection,
        4.,
        0.,
        viewport,
        &[],
        &[],
        &[],
        &mut anchors,
    );
    let hits = context
        .data(|data| data.get_temp::<Vec<ArmyHit>>(egui::Id::new("map-army-hit-targets")))
        .unwrap();
    assert!(hits.iter().any(|hit| hit.owner == enemy));
    assert!(hits.iter().any(|hit| hit.owner == own));
    let mut output = context.end_pass();
    output.textures_delta.clear();
}
use image::GenericImageView;

#[test]
fn first_placement_does_not_depend_on_viewport_clipping() {
    let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
    let province = atlas().provinces.iter().find(|p| p.name == "Samnium").unwrap();
    let projection = Projection {
        origin: viewport.center(),
        scale: 100.,
        center: province.visual_center,
    };
    let desired = projection.point(province.visual_center);
    let place =
        |viewport| province_anchor(desired, 40., viewport, &[], &[], province, &projection, 1);
    let first = place(viewport).expect("army fits inside Samnium");
    let clipped = egui::Rect::from_min_max(first, viewport.max);
    assert_eq!(place(clipped), Some(first));
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

#[test]
fn stationary_armies_stay_inside_their_province_when_avoiding_cities() {
    let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200., 800.));
    for name in ["Latium", "Apulia", "Samnium"] {
        let province = atlas().provinces.iter().find(|p| p.name == name).unwrap();
        for zoom in [4., 6., 8.] {
            let projection = Projection {
                origin: viewport.center(),
                scale: 18. * zoom,
                center: province.visual_center,
            };
            let landmarks: Vec<_> =
                layout_cities(&projection, viewport, zoom).iter().map(|city| city.bounds).collect();
            let size = troop_size(zoom);
            let point = province_anchor(
                projection.point(province.visual_center) + cluster_offset(0, 1, size),
                size,
                viewport,
                &landmarks,
                &[],
                province,
                &projection,
                if name == "Latium" {
                    1
                } else {
                    2
                },
            );
            if let Some(point) = point {
                let bounds = cluster_bounds(
                    point,
                    size,
                    if name == "Latium" {
                        1
                    } else {
                        2
                    },
                );
                assert!(province.contains(projection.inverse(point)), "{name} center zoom {zoom}");
                let ground = ground_footprint(
                    point,
                    size,
                    if name == "Latium" {
                        1
                    } else {
                        2
                    },
                );
                assert!(footprint_in_province(ground, province, &projection), "{name} zoom {zoom}");
                assert!(!landmarks.iter().any(|landmark| landmark.intersects(bounds)));
            } else {
                assert!(zoom < 8., "{name} army should fit at close zoom");
            }
        }
    }
}

#[test]
fn zoomed_out_troops_emit_no_badges_or_sprites() {
    let context = egui::Context::default();
    let mut world = MilitaryWorld::new(atlas().provinces.len());
    let province = atlas().provinces.iter().position(|p| p.name == "Latium").unwrap();
    world.seed_unit(province, ForceOwner::Local(province), UnitType::HeavyInfantry).unwrap();
    let threshold = world.config.sprite_zoom_threshold as f32;
    let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200., 800.));
    let projection = Projection {
        origin: viewport.center(),
        scale: 14.,
        center: [14., 41.],
    };
    for zoom in [MIN_ZOOM, threshold - 0.01, threshold] {
        assert_eq!(troop_alpha(zoom, threshold), 0);
        context.begin_pass(egui::RawInput::default());
        let painter = context.layer_painter(egui::LayerId::background());
        assert!(paint(
            &painter,
            &world,
            &ProvinceOwnership::default(),
            &projection,
            zoom,
            0.,
            viewport,
            &[],
            &[],
            &[],
            &mut Anchors::default(),
        )
        .is_empty());
        let mut output = context.end_pass();
        assert!(output.shapes.is_empty());
        output.textures_delta.clear();
    }
}

#[test]
fn light_infantry_idle_keeps_a_consistent_height_and_ground_baseline() {
    let sheet =
        image::load_from_memory(IDLE_SHEETS[UnitType::LightInfantry as usize]).unwrap().to_rgba8();
    let mut heights = Vec::new();
    let mut baselines = Vec::new();
    for frame in 0..16 {
        let cell = image::imageops::crop_imm(&sheet, frame % 4 * 192, frame / 4 * 192, 192, 192);
        // Measure helmet to feet, excluding the spear to the soldier's left.
        let body_top =
            (0..192).find(|&y| (85..110).any(|x| cell.get_pixel(x, y)[3] > 127)).unwrap();
        let feet =
            (0..192).rev().find(|&y| (0..192).any(|x| cell.get_pixel(x, y)[3] > 127)).unwrap();
        heights.push(feet - body_top + 1);
        baselines.push(feet);
        assert!(
            (175..192).all(|y| (0..192).all(|x| cell.get_pixel(x, y)[3] == 0)),
            "frame {frame} has debris below the planted feet"
        );
    }
    assert!(baselines.iter().max().unwrap() - baselines.iter().min().unwrap() <= 1);
    assert!(heights.iter().max().unwrap() - heights.iter().min().unwrap() <= 2);
}

#[test]
fn every_unit_has_sixteen_distinct_visible_idle_frames() {
    for kind in UnitType::ALL {
        let image = image::load_from_memory(IDLE_SHEETS[kind as usize]).unwrap().to_rgba8();
        assert_eq!(image.dimensions(), (768, 768));
        let mut frames = Vec::new();
        for frame in 0..16 {
            let uv = idle_uv(frame);
            let tile = image::imageops::crop_imm(
                &image,
                (uv.min.x * 768.) as u32,
                (uv.min.y * 768.) as u32,
                192,
                192,
            )
            .to_image();
            assert!(tile.pixels().any(|p| p[3] > 127), "{} idle {frame} empty", kind.name());
            assert!(tile.pixels().any(|p| p[3] == 0), "{} idle background", kind.name());
            assert!(!frames.contains(tile.as_raw()), "{} idle {frame} duplicate", kind.name());
            frames.push(tile.into_raw());
        }
    }
}

#[test]
fn shown_units_keep_the_same_map_size_as_the_camera_zooms() {
    let context = egui::Context::default();
    let mut world = MilitaryWorld::new(atlas().provinces.len());
    let province = atlas().provinces.iter().position(|p| p.name == "Samnium").unwrap();
    world.seed_unit(province, ForceOwner::Local(province), UnitType::HeavyInfantry).unwrap();
    let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200., 800.));
    let mut previous: Option<f32> = None;
    let mut anchors = Anchors::default();
    for zoom in [3., 4., 6., 8.] {
        let projection = Projection {
            origin: viewport.center(),
            scale: 18. * zoom,
            center: atlas().provinces[province].visual_center,
        };
        context.begin_pass(egui::RawInput::default());
        let painter = context.layer_painter(egui::LayerId::background());
        let markers = paint(
            &painter,
            &world,
            &ProvinceOwnership::default(),
            &projection,
            zoom,
            0.,
            viewport,
            &[],
            &[],
            &[],
            &mut anchors,
        );
        assert_eq!(markers.len(), 2, "unit and badge must be shown together at zoom {zoom}");
        let map_width = markers[0].width() / projection.scale;
        if let Some(previous) = previous {
            assert!((map_width - previous).abs() < 0.001, "unit changed map size at zoom {zoom}");
        }
        previous = Some(map_width);
        let mut output = context.end_pass();
        output.textures_delta.clear();
    }
}

#[test]
fn visible_armies_keep_their_geographic_position_through_camera_and_landmark_changes() {
    let context = egui::Context::default();
    let mut world = MilitaryWorld::new(atlas().provinces.len());
    let province = atlas().provinces.iter().position(|p| p.name == "Samnium").unwrap();
    let owner = ForceOwner::Local(province);
    world.seed_unit(province, owner, UnitType::HeavyInfantry).unwrap();
    let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200., 800.));
    let mut anchors = Anchors::default();
    let mut expected = None;
    for (zoom, pan, obstruct) in [
        (3., egui::Vec2::ZERO, false),
        (6., egui::vec2(100., -30.), true),
        (8., egui::vec2(-70., 20.), true),
        (1., egui::Vec2::ZERO, false),
        (4., egui::vec2(25., 100.), true),
    ] {
        let projection = Projection {
            origin: viewport.center() + pan,
            scale: 18. * zoom,
            center: atlas().provinces[province].visual_center,
        };
        context.begin_pass(egui::RawInput::default());
        let painter = context.layer_painter(egui::LayerId::background());
        // Even a newly overlapping landmark cannot make an established army jump.
        let landmarks = if obstruct {
            vec![viewport]
        } else {
            vec![]
        };
        let markers = paint(
            &painter,
            &world,
            &ProvinceOwnership::default(),
            &projection,
            zoom,
            0.,
            viewport,
            &landmarks,
            &[],
            &[],
            &mut anchors,
        );
        if zoom > world.config.sprite_zoom_threshold as f32 {
            assert_eq!(markers.len(), 2);
            let position = projection.inverse(markers[0].center());
            if let Some(expected) = expected {
                let delta = projection.point(expected).distance(projection.point(position));
                assert!(delta < 0.001, "army jumped at zoom {zoom}");
            } else {
                expected = Some(position);
            }
        } else {
            assert!(markers.is_empty());
        }
        let mut output = context.end_pass();
        output.textures_delta.clear();
    }
    world.provinces[province].forces.clear();
    anchors.retain_for(&world);
    assert!(anchors.positions.is_empty(), "departed armies must release their anchor");
}

#[test]
fn relaxed_idle_cycle_has_pauses_and_independent_phases() {
    assert_eq!(idle_frame(0., 0), 0);
    assert_eq!(idle_frame(1.5, 0), 0);
    let frames: std::collections::BTreeSet<_> =
        (0..1000).map(|tick| idle_frame(tick as f32 * 0.01, 0)).collect();
    assert_eq!(frames.len(), 16);
    assert!(
        (0..100).any(|tick| idle_frame(tick as f32 * 0.1, 37) != idle_frame(tick as f32 * 0.1, 74)),
        "neighboring armies should not animate in lockstep"
    );
}

#[test]
fn new_armies_prefer_clear_labels_but_allow_overlap_when_no_clear_ground_remains() {
    let context = egui::Context::default();
    let mut world = MilitaryWorld::new(atlas().provinces.len());
    // Give the enlarged artwork enough ground to exercise the clear-label preference.
    let province = atlas().provinces.iter().position(|p| p.name == "Numidia").unwrap();
    let owner = ForceOwner::Local(province);
    world.seed_unit(province, owner, UnitType::HeavyInfantry).unwrap();
    let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200., 800.));
    let zoom = 6.;
    let projection = Projection {
        origin: viewport.center(),
        scale: 18. * zoom,
        center: atlas().provinces[province].visual_center,
    };
    let normal = viewport.center() + egui::vec2(0., -troop_size(zoom) * 1.25);
    let label = egui::Rect::from_center_size(normal, egui::vec2(100., 45.));
    for area in [label, viewport] {
        let mut areas = vec![None; atlas().provinces.len()];
        areas[province] = Some(area);
        context.begin_pass(egui::RawInput::default());
        let painter = context.layer_painter(egui::LayerId::background());
        let markers = paint(
            &painter,
            &world,
            &ProvinceOwnership::default(),
            &projection,
            zoom,
            0.,
            viewport,
            &[],
            &[],
            &areas,
            &mut Anchors::default(),
        );
        assert_eq!(markers.len(), 2, "a province name must never hide the army");
        if area == label {
            assert!(
                markers.iter().all(|bounds| !bounds.intersects(label)),
                "initial placement should keep the centered label clear when possible"
            );
        }
        let mut output = context.end_pass();
        output.textures_delta.clear();
    }
}
