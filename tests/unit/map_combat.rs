use super::*;

use crate::egui_capture as capture;

fn battle_world(province: usize) -> MilitaryWorld {
    let mut world = MilitaryWorld::new(atlas().provinces.len());
    for owner in [ForceOwner::Player(0), ForceOwner::Local(province)] {
        for kind in [UnitType::LightInfantry, UnitType::Archers, UnitType::HeavyInfantry] {
            world.seed_unit(province, owner, kind).unwrap();
        }
    }
    world
        .start_battle(
            province,
            &[ForceOwner::Player(0)],
            &[ForceOwner::Local(province)],
            None,
            None,
            crate::game::military::MilitaryTerrain::Plains,
            0,
            5,
        )
        .unwrap();
    world
}

#[test]
fn opposing_ranks_face_inward_and_fire_across_the_battlefield() {
    let province =
        atlas().provinces.iter().position(|province| province.name == "Cilicia").unwrap();
    let world = battle_world(province);
    let battle = &world.battles[0];
    let attackers = fighters(&battle.attackers, 1.);
    let defenders = fighters(&battle.defenders, -1.);
    for side in [&attackers, &defenders] {
        for actor in side.iter() {
            assert!(actor.offset.x * actor.facing < 0., "unit faces away from its opponent");
        }
        let front = side.iter().find(|actor| actor.kind == UnitType::HeavyInfantry).unwrap();
        let archer = side.iter().find(|actor| actor.kind == UnitType::Archers).unwrap();
        assert!(
            archer.offset.x.abs() > front.offset.x.abs(),
            "archer belongs behind the close fighters"
        );
    }
    for (start, end) in [
        (egui::pos2(100., 100.), egui::pos2(240., 120.)),
        (egui::pos2(240., 120.), egui::pos2(100., 100.)),
    ] {
        let (launch, direction) = projectile(start, end, 0., 30.);
        let (impact, _) = projectile(start, end, 1., 30.);
        assert_eq!(launch, start);
        assert_eq!(impact, end);
        assert!(direction.x * (end.x - start.x) > 0.);
        assert!(projectile(start, end, 0.5, 30.).0.y < start.lerp(end, 0.5).y);
    }
}

#[test]
fn battle_animation_faces_both_ways_and_has_projectiles_without_a_label() {
    let context = egui::Context::default();
    let province =
        atlas().provinces.iter().position(|province| province.name == "Cilicia").unwrap();
    let world = battle_world(province);
    let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000., 700.));
    let projection = Projection {
        origin: viewport.center(),
        scale: 100.,
        center: atlas().provinces[province].visual_center,
    };
    let mut anchors = Anchors::default();
    let mut capture = capture::Capture::default();
    let mut missiles = false;
    for (index, clock) in [0., 0.3, 0.6, 0.9, 1.2, 1.5, 1.8, 2.1, 2.4].into_iter().enumerate() {
        context.begin_pass(egui::RawInput {
            screen_rect: Some(viewport),
            ..Default::default()
        });
        let painter = context.layer_painter(egui::LayerId::background());
        // Paint the real military pass, including its camera-audio publication.
        super::super::paint(
            &painter,
            None,
            &world,
            &ProvinceOwnership::default(),
            &projection,
            5.,
            clock,
            viewport,
            &[],
            &[],
            &[],
            &mut anchors,
        );
        let hits = context
            .data(|data| data.get_temp::<Vec<ArmyHit>>(egui::Id::new("map-army-hit-targets")))
            .unwrap();
        assert_eq!(hits.len(), 8, "six fighters and two owner banners remain clickable");
        let field = &anchors.battles[&world.battles[0].id];
        assert!(camera_gain(5., 2.3, projection.point(field.center), viewport) > 0.);
        assert_eq!(audible_battles(&context).len(), 1);
        let mut output = context.end_pass();
        let orientations: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| {
                let egui::Shape::Mesh(mesh) = &shape.shape else {
                    return None;
                };
                (mesh.vertices.len() == 4 && mesh.vertices[0].uv.x != mesh.vertices[1].uv.x)
                    .then_some(mesh.vertices[1].uv.x - mesh.vertices[0].uv.x)
            })
            .collect();
        assert!(orientations.iter().any(|width| *width > 0.), "attackers must face right");
        assert!(orientations.iter().any(|width| *width < 0.), "defenders must face left");
        assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Text(text) if text.galley.job.text == "BATTLE")));
        missiles |= output.shapes.iter().any(|shape| {
            matches!(&shape.shape,
            egui::Shape::Path(path) if path.points.len() == 3)
        });
        capture.frame(&context, &output, &format!("battle-motion-{index}"));
        output.textures_delta.clear();
    }
    assert!(missiles, "spears and arrows must visibly travel between the armies");
}

#[test]
fn battle_anchors_stay_inside_the_province_and_survive_camera_changes() {
    let province = atlas().provinces.iter().find(|province| province.name == "Cilicia").unwrap();
    let id = atlas().provinces.iter().position(|item| item.name == province.name).unwrap();
    let world = battle_world(id);
    let mut actors = fighters(&world.battles[0].attackers, 1.);
    actors.extend(fighters(&world.battles[0].defenders, -1.));
    let projection = Projection {
        origin: egui::pos2(600., 400.),
        scale: 100.,
        center: province.visual_center,
    };
    let field = field_anchor(
        &actors,
        province,
        projection.point(province.visual_center),
        &projection,
        40.,
        &[],
    )
    .unwrap();
    let original: Vec<_> = actors
        .iter()
        .map(|actor| projection.inverse(position(&field, actor, &projection)))
        .collect();
    for zoom in [2.4, 4., 8.] {
        let camera = Projection {
            origin: egui::pos2(500. + zoom * 10., 320.),
            scale: 20. * zoom,
            center: projection.center,
        };
        for (actor, original) in actors.iter().zip(&original) {
            let anchor = position(&field, actor, &camera);
            assert!(
                province.contains(camera.inverse(anchor + egui::vec2(0., troop_size(zoom) * 0.5)))
            );
            assert!(
                camera.point(*original).distance(anchor) < 0.001,
                "camera movement rearranged the opposing ranks"
            );
        }
    }
}

#[test]
fn battle_audio_fades_with_zoom_and_panning_and_tracks_attack_cycles() {
    let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000., 700.));
    let center = viewport.center();
    assert_eq!(camera_gain(2.3, 2.3, center, viewport), 0.);
    assert_eq!(camera_gain(8., 2.3, viewport.right_center() + egui::vec2(1., 0.), viewport), 0.);
    assert_eq!(camera_gain(8., 2.3, viewport.right_center(), viewport), 0.);
    assert!(camera_gain(3., 2.3, center, viewport) < camera_gain(4., 2.3, center, viewport));
    assert!(
        camera_gain(5., 2.3, center + egui::vec2(400., 0.), viewport)
            < camera_gain(5., 2.3, center, viewport)
    );
    let archer = Fighter {
        id: 0,
        owner: ForceOwner::Player(0),
        kind: UnitType::Archers,
        seed: 0,
        facing: 1.,
        offset: egui::Vec2::ZERO,
    };
    let release = Animation::Combat.seconds() * RELEASE;
    assert_eq!(strike(release - 0.01, &archer).cycle, -1);
    assert_eq!(strike(release + 0.01, &archer).cycle, 0);
    assert_eq!(strike(release + Animation::Combat.seconds() + 0.01, &archer).cycle, 1);
    assert_eq!(strike(release, &archer).cue, "battle-archers");
}

#[test]
fn light_infantry_releases_its_javelin_and_readies_another() {
    let sheet = image::load_from_memory(COMBAT_SHEETS[UnitType::LightInfantry as usize])
        .unwrap()
        .to_rgba8();
    let weapon_pixels = |frame: u32| {
        let x = frame % frames::COLUMNS * frames::SIZE;
        let y = frame / frames::COLUMNS * frames::SIZE;
        (0..40)
            .flat_map(|row| (0..frames::SIZE).map(move |column| (column, row)))
            .filter(|&(column, row)| sheet.get_pixel(x + column, y + row)[3] > 127)
            .count()
    };
    let ready = weapon_pixels(0);
    let released = weapon_pixels(32);
    let rearmed = weapon_pixels(47);
    assert!(released * 5 < ready * 3, "the held spear must leave the hand after the cast");
    assert!(rearmed * 10 > ready * 9, "the next javelin must be ready before the loop wraps");
}
