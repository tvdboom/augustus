use super::*;
use crate::game::military::BattlePlan;

#[path = "egui_capture.rs"]
mod revolt_capture;

#[test]
fn revolt_infantry_remains_visible_at_wide_zoom_and_under_landmarks() {
    let context = egui::Context::default();
    let province = atlas().provinces.iter().position(|p| p.name == "Africa Proconsularis").unwrap();
    let mut world = MilitaryWorld::new(atlas().provinces.len());
    let owner = ForceOwner::Local(province);
    world.seed_population_force(province, owner, UnitType::LightInfantry, 120.0).unwrap();
    world.provinces[province].slave_rebellion = true;
    let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200., 800.));
    let mut capture = revolt_capture::Capture::default();
    for zoom in [MIN_ZOOM, 2.4, 4.0] {
        let projection = Projection {
            origin: viewport.center(),
            scale: 18. * zoom,
            center: atlas().provinces[province].visual_center,
        };
        context.begin_pass(egui::RawInput {
            screen_rect: Some(viewport),
            ..Default::default()
        });
        let painter = context.layer_painter(egui::LayerId::background());
        let markers = paint(
            &painter,
            &world,
            &ProvinceOwnership::default(),
            &projection,
            zoom,
            0.,
            viewport,
            &[viewport],
            &[],
            &[],
            &mut Anchors::default(),
        );
        assert!(markers.len() >= 2, "rebel infantry and its banner disappeared at zoom {zoom}");
        let hits = context
            .data(|data| data.get_temp::<Vec<ArmyHit>>(egui::Id::new("map-army-hit-targets")))
            .unwrap();
        assert!(hits.iter().all(|hit| hit.province == province && hit.owner == owner));
        let cache = context
            .data(|data| data.get_temp::<Textures>(egui::Id::new("military-sprite-sheet-cache")))
            .unwrap();
        assert!(cache.idle[UnitType::LightInfantry as usize].is_some());
        let mut output = context.end_pass();
        capture.frame(&context, &output, &format!("revolt-infantry-{}", (zoom * 10.) as u32));
        output.textures_delta.clear();
        assert!(output.shapes.iter().any(|shape|
            matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == "Revolt")));
        assert!(output.shapes.iter().any(|shape|
            matches!(&shape.shape, egui::Shape::Rect(rect) if rect.fill == egui::Color32::from_rgb(176, 45, 35))));
    }
}

#[test]
fn a_revolt_battle_shows_both_armies_and_combat_art_at_wide_zoom() {
    let context = egui::Context::default();
    let province = atlas().provinces.iter().position(|p| p.name == "Africa Proconsularis").unwrap();
    let mut world = MilitaryWorld::new(atlas().provinces.len());
    let rebel = ForceOwner::Local(province);
    let player = ForceOwner::Player(0);
    world.seed_unit(province, rebel, UnitType::LightInfantry).unwrap();
    world.seed_unit(province, player, UnitType::HeavyInfantry).unwrap();
    world.provinces[province].slave_rebellion = true;
    world
        .start_battle(
            province,
            &[rebel],
            &[player],
            Some(0),
            None,
            crate::game::military::MilitaryTerrain::Farmland,
            0,
            1,
        )
        .unwrap();
    let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200., 800.));
    let projection = Projection {
        origin: viewport.center(),
        scale: 18. * MIN_ZOOM,
        center: atlas().provinces[province].visual_center,
    };
    context.begin_pass(egui::RawInput {
        screen_rect: Some(viewport),
        ..Default::default()
    });
    let painter = context.layer_painter(egui::LayerId::background());
    let markers = paint(
        &painter,
        &world,
        &ProvinceOwnership::default(),
        &projection,
        MIN_ZOOM,
        0.,
        viewport,
        &[viewport],
        &[],
        &[],
        &mut Anchors::default(),
    );
    assert_eq!(markers.len(), 4, "both armies and their banners must appear immediately");
    let hits = context
        .data(|data| data.get_temp::<Vec<ArmyHit>>(egui::Id::new("map-army-hit-targets")))
        .unwrap();
    assert!(hits.iter().any(|hit| hit.owner == rebel));
    assert!(hits.iter().any(|hit| hit.owner == player));
    let banners: Vec<_> = hits.iter().filter(|hit| hit.rect.height() == 12.).collect();
    assert_eq!(banners.len(), 2);
    assert!(!banners[0].rect.intersects(banners[1].rect), "opposing owner labels overlap");
    let cache = context
        .data(|data| data.get_temp::<Textures>(egui::Id::new("military-sprite-sheet-cache")))
        .unwrap();
    for kind in [UnitType::LightInfantry, UnitType::HeavyInfantry] {
        assert!(cache.combat[kind as usize].is_some());
    }
    let mut output = context.end_pass();
    revolt_capture::Capture::default().frame(&context, &output, "revolt-combat");
    output.textures_delta.clear();
}

#[test]
fn map_shows_strongest_types_from_each_category_independent_of_deployment() {
    let mut world = MilitaryWorld::new(1);
    let owner = ForceOwner::Player(0);
    for _ in 0..8 {
        world.seed_unit(0, owner, UnitType::LightInfantry).unwrap();
    }
    for kind in [
        UnitType::HeavyInfantry,
        UnitType::Archers,
        UnitType::LightCavalry,
        UnitType::HeavyCavalry,
        UnitType::WarCamels,
        UnitType::WarElephants,
    ] {
        world.seed_unit(0, owner, kind).unwrap();
    }
    let units = &world.provinces[0].forces[&owner];
    let plan = BattlePlan {
        primary_unit_type: UnitType::HeavyInfantry,
        secondary_unit_type: UnitType::Archers,
        flank_unit_type: UnitType::WarCamels,
        ..BattlePlan::default()
    };
    let shown = map_representatives(units);
    assert_eq!(
        shown.iter().copied().collect::<std::collections::BTreeSet<_>>(),
        [
            UnitType::HeavyInfantry,
            UnitType::Archers,
            UnitType::HeavyCavalry,
            UnitType::WarElephants,
        ]
        .into()
    );
    assert_eq!(shown, map_representatives(units));
    // The old map order followed these private plan fields. Changing them must
    // leave both the selected types and their stable order untouched.
    world.provinces[0].plans.insert(owner, plan);
    assert_eq!(shown, map_representatives(&world.provinces[0].forces[&owner]));
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
        map_representatives(units).into_iter().collect::<std::collections::BTreeSet<_>>(),
        [UnitType::HeavyInfantry, UnitType::Archers, UnitType::HeavyCavalry].into()
    );
    assert_eq!(map_representatives(&units[..1]), vec![UnitType::HeavyInfantry]);
    let mut depleted = units.to_vec();
    depleted[2].current_manpower = 0.;
    assert_eq!(
        map_representatives(&depleted).into_iter().collect::<std::collections::BTreeSet<_>>(),
        [UnitType::HeavyInfantry, UnitType::Archers].into()
    );
}

#[test]
fn absent_categories_use_other_strong_types_and_large_troops_keep_their_feet_aligned() {
    let mut world = MilitaryWorld::new(1);
    let owner = ForceOwner::Player(0);
    for kind in [UnitType::LightInfantry, UnitType::Archers, UnitType::HeavyInfantry] {
        world.seed_unit(0, owner, kind).unwrap();
    }
    assert_eq!(
        map_representatives(&world.provinces[0].forces[&owner])
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>(),
        [UnitType::LightInfantry, UnitType::Archers, UnitType::HeavyInfantry].into()
    );
    let anchor = egui::pos2(100., 100.);
    let infantry = troop_rect(anchor, 30., 0, 1, UnitType::HeavyInfantry);
    let light_infantry = troop_rect(anchor, 30., 0, 1, UnitType::LightInfantry);
    let cavalry = troop_rect(anchor, 30., 0, 1, UnitType::HeavyCavalry);
    let camel = troop_rect(anchor, 30., 0, 1, UnitType::WarCamels);
    let elephant = troop_rect(anchor, 30., 0, 1, UnitType::WarElephants);
    assert_eq!(infantry.size(), light_infantry.size());
    assert!(infantry.width() < cavalry.width());
    assert!(cavalry.width() < camel.width() && camel.width() < elephant.width());
    let ground = |rect: egui::Rect| {
        rect.min.y + rect.height() * frames::BASELINE as f32 / frames::SIZE as f32
    };
    for rect in [infantry, light_infantry, cavalry, camel, elephant] {
        assert!((ground(rect) - (anchor.y + 15.)).abs() < 0.001);
    }
}

#[test]
fn several_cohorts_of_one_type_still_show_three_figures() {
    let mut world = MilitaryWorld::new(1);
    let owner = ForceOwner::Player(0);
    for _ in 0..3 {
        world.seed_unit(0, owner, UnitType::HeavyInfantry).unwrap();
    }
    assert_eq!(
        map_representatives(&world.provinces[0].forces[&owner]),
        vec![UnitType::HeavyInfantry; 3]
    );
}

#[test]
fn rome_starts_with_light_cavalry_and_shows_all_four_army_types() {
    let defenders = crate::game::military::initial_defenders("Latium");
    assert_eq!(defenders.len(), 50);
    assert_eq!(defenders.iter().filter(|&&kind| kind == UnitType::LightCavalry).count(), 6);
    let mut world = MilitaryWorld::new(1);
    for kind in defenders {
        world.seed_unit(0, ForceOwner::Local(0), kind).unwrap();
    }
    assert_eq!(
        map_representatives(&world.provinces[0].forces[&ForceOwner::Local(0)])
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>(),
        [
            UnitType::HeavyInfantry,
            UnitType::Archers,
            UnitType::HeavyCavalry,
            UnitType::LightCavalry,
        ]
        .into()
    );
}

#[test]
fn rome_keeps_four_full_size_figures_with_their_ground_anchors_in_latium() {
    let context = egui::Context::default();
    let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200., 800.));
    let province = atlas().provinces.iter().position(|p| p.name == "Latium").unwrap();
    let mut world = MilitaryWorld::new(atlas().provinces.len());
    for kind in crate::game::military::initial_defenders("Latium") {
        world.seed_unit(province, ForceOwner::Local(province), kind).unwrap();
    }
    let types =
        map_representatives(&world.provinces[province].forces[&ForceOwner::Local(province)]);
    for zoom in [3., 4., 6., 8.] {
        let projection = Projection {
            origin: viewport.center(),
            scale: 18. * zoom,
            center: atlas().provinces[province].visual_center,
        };
        let landmarks: Vec<_> =
            layout_cities(&projection, viewport, zoom).iter().map(|city| city.bounds).collect();
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
            &landmarks,
            &[],
            &[],
            &mut Anchors::default(),
        );
        assert_eq!(
            markers.len(),
            5,
            "Rome's four types and badge should be visible at zoom {zoom}"
        );
        for (&kind, sprite) in types.iter().zip(&markers[..4]) {
            let expected = troop_size(zoom) * troop_scale(kind) * 2.;
            assert!(
                (sprite.width() - expected).abs() < 0.001,
                "Rome must use the common unit size"
            );
            let feet = egui::pos2(
                sprite.center().x,
                sprite.min.y + sprite.height() * frames::BASELINE as f32 / frames::SIZE as f32,
            );
            assert!(
                atlas().provinces[province].contains(projection.inverse(feet)),
                "Rome's {kind:?} ground anchor leaves Latium at zoom {zoom}"
            );
        }
        assert!(atlas().provinces[province].contains(projection.inverse(markers[4].center())));
        let mut output = context.end_pass();
        output.textures_delta.clear();
    }
}

#[test]
fn the_same_unit_is_the_same_size_in_rome_and_samnium() {
    let context = egui::Context::default();
    let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200., 800.));
    for zoom in [3., 4., 6., 8.] {
        let mut sizes = Vec::new();
        for name in ["Latium", "Samnium"] {
            let province = atlas().provinces.iter().position(|p| p.name == name).unwrap();
            let mut world = MilitaryWorld::new(atlas().provinces.len());
            world
                .seed_unit(province, ForceOwner::Local(province), UnitType::HeavyInfantry)
                .unwrap();
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
                &mut Anchors::default(),
            );
            assert_eq!(markers.len(), 2, "{name}'s army must be visible at zoom {zoom}");
            sizes.push(markers[0].size());
            let mut output = context.end_pass();
            output.textures_delta.clear();
        }
        assert!(
            (sizes[0] - sizes[1]).length() < 0.001,
            "Province changed unit size at zoom {zoom}"
        );
    }
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
    let place = |viewport| {
        province_anchor(
            desired,
            40.,
            viewport,
            &[],
            &[],
            province,
            &projection,
            &[UnitType::HeavyInfantry],
        )
    };
    let first = place(viewport).expect("army fits inside Samnium");
    let clipped = egui::Rect::from_min_max(first, viewport.max);
    assert_eq!(place(clipped), Some(first));
}

#[test]
fn every_unit_has_a_complete_transparent_four_by_four_icon_sheet() {
    for kind in UnitType::ALL {
        let decoded = image::load_from_memory(SHEETS[kind as usize])
            .unwrap_or_else(|error| panic!("{}: {error}", kind.name()))
            .to_rgba8();
        assert_eq!(decoded.dimensions(), (768, 768), "{} resolution", kind.name());
        assert!(
            decoded.pixels().any(|p| p.0[3] < 255),
            "{} must have transparent background",
            kind.name()
        );
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
                    &[UnitType::HeavyInfantry][..]
                } else {
                    &[UnitType::HeavyInfantry, UnitType::LightInfantry][..]
                },
            );
            if let Some(point) = point {
                let bounds = cluster_bounds(
                    point,
                    size,
                    if name == "Latium" {
                        &[UnitType::HeavyInfantry][..]
                    } else {
                        &[UnitType::HeavyInfantry, UnitType::LightInfantry][..]
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
                if name == "Latium" {
                    assert!(ground_anchors_in_province(point, size, 1, province, &projection));
                } else {
                    assert!(
                        footprint_in_province(ground, province, &projection),
                        "{name} zoom {zoom}"
                    );
                }
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
    for frame in 0..frames::COUNT as u32 {
        let cell = image::imageops::crop_imm(&sheet, frame % 8 * 192, frame / 8 * 192, 192, 192);
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
    assert!(heights.iter().max().unwrap() - heights.iter().min().unwrap() <= 12);
}

#[test]
fn infantry_idle_silhouettes_have_similar_map_dimensions() {
    let anchor = egui::Pos2::ZERO;
    let dimensions = |kind: UnitType, frame: u32| {
        let sheet = image::load_from_memory(IDLE_SHEETS[kind as usize]).unwrap().to_rgba8();
        let cell = image::imageops::crop_imm(&sheet, frame % 8 * 192, frame / 8 * 192, 192, 192);
        let (left, top, right, bottom) = cell.pixels().filter(|(_, _, pixel)| pixel[3] > 8).fold(
            (192, 192, 0, 0),
            |(left, top, right, bottom), (x, y, _)| {
                (left.min(x), top.min(y), right.max(x + 1), bottom.max(y + 1))
            },
        );
        let rect = troop_rect(anchor, 30., 0, 1, kind);
        (rect.width() * (right - left) as f32, rect.height() * (bottom - top) as f32)
    };
    for frame in 0..frames::COUNT as u32 {
        let archer = dimensions(UnitType::Archers, frame);
        for kind in [UnitType::LightInfantry, UnitType::HeavyInfantry] {
            let soldier = dimensions(kind, frame);
            // Shields and bows change the outer silhouette; bodies keep a
            // similar stature without widening the light infantry on screen.
            assert!((soldier.0 / archer.0 - 1.).abs() < 0.16, "{kind:?} frame {frame} width");
            assert!((soldier.1 / archer.1 - 1.).abs() < 0.1, "{kind:?} frame {frame} height");
        }
    }
}

#[test]
fn light_infantry_matches_archer_stature_without_counting_its_spear() {
    let infantry =
        image::load_from_memory(IDLE_SHEETS[UnitType::LightInfantry as usize]).unwrap().to_rgba8();
    let archer =
        image::load_from_memory(IDLE_SHEETS[UnitType::Archers as usize]).unwrap().to_rgba8();
    let stature = |sheet: &image::RgbaImage, frame: u32| {
        let left = frame % frames::COLUMNS * frames::SIZE;
        let top = frame / frames::COLUMNS * frames::SIZE;
        // Both grounded figures are centered at x=96. This head/torso band
        // excludes the taller spear on the infantry's left: total silhouette
        // height previously concealed a soldier about 10% shorter than an archer.
        let crown = (0..frames::BASELINE)
            .find(|&y| (85..110).any(|x| sheet.get_pixel(left + x, top + y)[3] > 127))
            .expect("centered soldier must have a visible head");
        frames::BASELINE - crown + 1
    };
    for frame in 0..frames::COUNT as u32 {
        let soldier_height = stature(&infantry, frame);
        let archer_height = stature(&archer, frame);
        let ratio = soldier_height as f32 / archer_height as f32;
        assert!(
            (0.94..=1.06).contains(&ratio),
            "frame {frame}: light infantry body is {soldier_height}px versus archer {archer_height}px"
        );
    }
}

#[test]
fn idle_motion_has_visible_body_travel_and_keeps_planted_feet() {
    for kind in UnitType::ALL {
        let sheet = image::load_from_memory(IDLE_SHEETS[kind as usize]).unwrap().to_rgba8();
        let mut centers = Vec::with_capacity(frames::COUNT);
        for frame in 0..frames::COUNT as u32 {
            let left = frame % frames::COLUMNS * frames::SIZE;
            let top = frame / frames::COLUMNS * frames::SIZE;
            let (mut mass, mut weighted_x, mut weighted_y) = (0_f64, 0_f64, 0_f64);
            // Follow the visible upper body, excluding grounded boots/wheels.
            // This measures actual silhouette travel rather than byte changes
            // caused by subpixel filtering or transparency alone.
            for y in 0..frames::BASELINE - 40 {
                for x in 0..frames::SIZE {
                    let alpha = f64::from(sheet.get_pixel(left + x, top + y)[3]);
                    mass += alpha;
                    weighted_x += f64::from(x) * alpha;
                    weighted_y += f64::from(y) * alpha;
                }
            }
            assert!(mass > 0., "{} idle {frame} has no upper body", kind.name());
            centers.push([weighted_x / mass, weighted_y / mass]);
            for y in frames::BASELINE..frames::SIZE {
                for x in 0..frames::SIZE {
                    assert_eq!(
                        sheet.get_pixel(left + x, top + y),
                        sheet.get_pixel(x, y),
                        "{} idle {frame} slides its planted feet at ({x}, {y})",
                        kind.name()
                    );
                }
            }
        }
        let excursion = centers
            .iter()
            .flat_map(|a| centers.iter().map(move |b| (a[0] - b[0]).hypot(a[1] - b[1])))
            .fold(0_f64, f64::max);
        // The previous 0.4–0.7px poses passed frame-uniqueness tests but looked
        // stationary after map scaling. Demand meaningful travel across a cycle.
        assert!(
            excursion >= 2.,
            "{} idle is visually static: only {excursion:.2}px of upper-body travel",
            kind.name()
        );
    }
}

#[test]
fn every_unit_motion_has_a_continuous_loop_and_transparent_gutters() {
    let distance = |a: &image::RgbaImage, b: &image::RgbaImage| -> f64 {
        a.pixels()
            .zip(b.pixels())
            .map(|(a, b)| {
                let alpha = (f64::from(a[3]) - f64::from(b[3])).abs();
                alpha
                    + (0..3)
                        .map(|channel| {
                            (f64::from(a[channel]) * f64::from(a[3]) / 255.
                                - f64::from(b[channel]) * f64::from(b[3]) / 255.)
                                .abs()
                        })
                        .sum::<f64>()
            })
            .sum::<f64>()
            / f64::from(frames::SIZE * frames::SIZE)
    };
    for (motion, sheets) in
        [("idle", &IDLE_SHEETS), ("movement", &MOVEMENT_SHEETS), ("combat", &COMBAT_SHEETS)]
    {
        for kind in UnitType::ALL {
            let sheet = image::load_from_memory(sheets[kind as usize]).unwrap().to_rgba8();
            assert_eq!(sheet.dimensions(), (frames::WIDTH, frames::HEIGHT));
            let tiles: Vec<_> = (0..frames::COUNT as u32)
                .map(|frame| {
                    let tile = image::imageops::crop_imm(
                        &sheet,
                        frame % frames::COLUMNS * frames::SIZE,
                        frame / frames::COLUMNS * frames::SIZE,
                        frames::SIZE,
                        frames::SIZE,
                    )
                    .to_image();
                    assert!(
                        tile.pixels().any(|p| p[3] > 127),
                        "{} {motion} {frame} empty",
                        kind.name()
                    );
                    assert!(
                        tile.enumerate_pixels().all(|(x, y, p)| {
                            (x >= 2 && y >= 2 && x < frames::SIZE - 2 && y < frames::SIZE - 2)
                                || p[3] == 0
                        }),
                        "{} {motion} {frame} touches the next cell",
                        kind.name()
                    );
                    tile
                })
                .collect();
            let deltas: Vec<_> = (0..frames::COUNT)
                .map(|frame| distance(&tiles[frame], &tiles[(frame + 1) % frames::COUNT]))
                .collect();
            let interior_max = deltas[..frames::COUNT - 1].iter().copied().fold(0., f64::max);
            let seam = deltas[frames::COUNT - 1];
            assert!(interior_max > 0.01, "{} {motion} must move", kind.name());
            assert!(
                seam <= interior_max * 1.25 + 0.01,
                "{} {motion} jumps at wrap: {seam} versus {interior_max}",
                kind.name()
            );
            // The two transitions on either side of frame zero must remain
            // comparable, so the seam does not hide a sudden stop or burst.
            let adjacent = deltas[0].max(deltas[frames::COUNT - 2]);
            assert!(
                seam <= adjacent * 1.5 + 0.08,
                "{} {motion} changes speed at wrap",
                kind.name()
            );
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
fn cycles_advance_uniformly_and_wrap_without_a_duplicate_endpoint() {
    for motion in [Animation::Idle, Animation::Movement, Animation::Combat] {
        let step = motion.seconds() / frames::COUNT as f32;
        for frame in 0..frames::COUNT {
            assert_eq!(animation_frame((frame as f32 + 0.5) * step, 0, motion), frame);
            let uv = animation_uv(frame);
            assert!(uv.min.x >= 0. && uv.min.y >= 0. && uv.max.x <= 1. && uv.max.y <= 1.);
        }
        assert_eq!(animation_frame(motion.seconds() - step * 0.1, 0, motion), frames::COUNT - 1);
        assert_eq!(animation_frame(motion.seconds() + step * 0.1, 0, motion), 0);
        assert_ne!(animation_frame(0., 37, motion), animation_frame(0., 74, motion));
    }
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
