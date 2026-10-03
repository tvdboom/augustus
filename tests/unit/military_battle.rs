use super::*;
use crate::game::economy::{EconomicProvince, Terrain};

use crate::egui_capture as capture;

fn fixture(enemy: ForceOwner) -> (MilitaryWorld, EconomyWorld, u64) {
    let mut world = MilitaryWorld::new(1);
    let own = ForceOwner::Player(0);
    for owner in [own, enemy] {
        for kind in [UnitType::HeavyInfantry, UnitType::Archers] {
            for _ in 0..6 {
                world.seed_unit(0, owner, kind).unwrap();
            }
        }
    }
    let id = world
        .start_battle(0, &[own], &[enemy], Some(1), None, MilitaryTerrain::Forest, 1, 71)
        .unwrap();
    let economy = EconomyWorld::new(
        3,
        vec![EconomicProvince::new(
            "Aquitania",
            50.,
            Terrain::Forest,
            false,
            [10.; 3],
            [10.; 4],
            3,
        )],
        vec![vec![]],
    );
    (world, economy, id)
}

fn context() -> egui::Context {
    let ctx = egui::Context::default();
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "augustus".into(),
        egui::FontData::from_static(include_bytes!("../../assets/fonts/FiraSans-Bold.ttf")).into(),
    );
    fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, "augustus".into());
    ctx.set_fonts(fonts);
    ctx.data_mut(|data| {
        data.insert_temp(
            egui::Id::new("campaign-player-colors"),
            vec![egui::Color32::from_rgb(39, 150, 109), egui::Color32::from_rgb(48, 132, 204)],
        )
    });
    ctx
}

#[allow(clippy::too_many_arguments)]
fn render(
    ctx: &egui::Context,
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    player: usize,
    size: egui::Vec2,
    time: f64,
    events: Vec<egui::Event>,
) -> (egui::FullOutput, Option<(usize, MilitaryUiAction)>) {
    let mut action = None;
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
            time: Some(time),
            events,
            ..Default::default()
        },
        |_| {
            action = draw_battle_panel(ctx, world, economy, player);
        },
    );
    if ctx
        .data(|data| data.get_temp::<bool>(egui::Id::new("battle-review-capture")))
        .unwrap_or(false)
    {
        let capture = ctx.data_mut(|data| {
            data.get_temp_mut_or_default::<std::sync::Arc<std::sync::Mutex<capture::Capture>>>(
                egui::Id::new("battle-review-textures"),
            )
            .clone()
        });
        let name = if world.battles.is_empty() && world.history[0].defenders.units.is_empty() {
            if player == 0 {
                "battle-defeated-defenders"
            } else {
                "battle-observer-result"
            }
        } else if world.battles.is_empty() {
            "battle-result"
        } else {
            "battle-current"
        };
        capture.lock().unwrap().frame(ctx, &output, name);
    }
    output.textures_delta.clear();
    (output, action)
}

fn text(output: &egui::FullOutput) -> Vec<String> {
    output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) => Some(text.galley.job.text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn live_battle_card_shows_current_state_and_stays_open_without_a_result_strip() {
    let (mut world, economy, id) = fixture(ForceOwner::Player(1));
    world.battles[0].defenders.plans.get_mut(&ForceOwner::Player(1)).unwrap().tactic =
        CombatTactic::Envelopment;
    let ctx = context();
    open_battle_panel(&ctx, id, 0);
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("battle-review-capture"), true));
    for time in [0.1, 0.4] {
        let (output, _) = render(&ctx, &world, &economy, 0, egui::vec2(1600., 900.), time, vec![]);
        if time == 0.1 {
            continue;
        }
        let labels = text(&output);
        for expected in ["Battle of Aquitania", "Player 1", "Player 2", "Retreat"] {
            assert!(labels.iter().any(|s| s == expected), "Missing {expected}: {labels:?}");
        }
    }
    world.battles[0].advance_month(&world.config);
    let (output, _) = render(&ctx, &world, &economy, 0, egui::vec2(1600., 900.), 0.8, vec![]);
    let labels = text(&output);
    for removed in [
        "ROUND HISTORY",
        "completed months",
        "rounds",
        "Battle in progress",
        "Defender terrain",
        "Shock Action",
        "% dice",
        "ATTACKERS",
        "DEFENDERS",
    ] {
        assert!(!labels.iter().any(|s| s.contains(removed)), "Unexpected clutter: {removed}");
    }
    world.battles[0].result = Some(BattleResult::DefenderVictory);
    world.history.push(world.battles.remove(0).into_outcome(&world.config).unwrap());
    let (output, _) = render(&ctx, &world, &economy, 0, egui::vec2(1600., 900.), 1.2, vec![]);
    assert!(!text(&output).iter().any(|s| s.contains("Defenders hold")));
    assert!(!text(&output).iter().any(|s| s.contains("Attackers take")));
    assert!(!text(&output).iter().any(|s| s == "Retreat"));
    assert!(selected_battle(&ctx).is_some(), "Armies remain inspectable after battle ends");
    assert!(dismiss(&ctx));
}

#[test]
fn battle_card_fits_bottom_center_and_matches_both_army_colors() {
    for size in [
        egui::vec2(1600., 900.),
        egui::vec2(960., 540.),
        egui::vec2(640., 360.),
        egui::vec2(360., 640.),
    ] {
        let (world, economy, id) = fixture(ForceOwner::Player(1));
        let ctx = context();
        open_battle_panel(&ctx, id, 0);
        render(&ctx, &world, &economy, 0, size, 0.1, vec![]);
        let (output, _) = render(&ctx, &world, &economy, 0, size, 0.4, vec![]);
        let (expected, scale) = panel_rect(ctx.content_rect());
        let panel = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Rect(r) if r.fill == PAPER => Some(r.rect),
                _ => None,
            })
            .unwrap();
        // egui snaps the area origin and dimensions to the current pixel grid.
        assert!((panel.center().x - size.x / 2.).abs() < 1.);
        assert!((panel.width() - PANEL_WIDTH * scale).abs() < 1.);
        assert!((panel.height() - PANEL_HEIGHT * scale).abs() < 1.);
        assert!((panel.min - expected.min).length() < 1.);
        assert!(panel.bottom() <= size.y && panel.bottom() > size.y * 0.96);
        let names = ["Player 1", "Player 2"].map(|role| {
            output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(t) if t.galley.job.text == role => Some(t.pos),
                    _ => None,
                })
                .unwrap()
        });
        assert!(names[0].x < panel.center().x && names[1].x < panel.center().x);
        assert!((names[0].x - names[1].x).abs() < 0.1);
        assert!(names[0].y < panel.top() + 92. * scale);
        assert!(
            names[1].y > panel.top() + 246. * scale,
            "Defender name mirrors the attacker below the battlefield"
        );
        for (index, owner) in [ForceOwner::Player(0), ForceOwner::Player(1)].into_iter().enumerate()
        {
            let banner = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Rect(r)
                        if r.fill == army_owner_color(&ctx, owner)
                            && (r.rect.height() - 42. * scale).abs() < 0.1 =>
                    {
                        Some(r.rect)
                    },
                    _ => None,
                })
                .unwrap();
            assert!((banner.width() - panel.width() / 2.).abs() < 0.1);
            assert!(
                (banner.left() - (panel.left() + index as f32 * panel.width() / 2.)).abs() < 0.1
            );
            let label = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(t)
                        if t.galley.job.text == format!("Player {}", index + 1) =>
                    {
                        Some(t)
                    },
                    _ => None,
                })
                .unwrap();
            assert_eq!(label.galley.job.sections[0].format.color, army_owner_color(&ctx, owner));
        }
        for shape in &output.shapes {
            if let egui::Shape::Text(t) = &shape.shape {
                let bounds = egui::Rect::from_min_size(t.pos, t.galley.size());
                assert!(
                    panel.expand(1.).contains_rect(bounds),
                    "Text escaped panel at {size:?}: {} {bounds:?}",
                    t.galley.job.text
                );
            }
        }
    }
}

#[test]
fn neutral_battle_armies_use_the_province_name_and_gray_banner() {
    let (world, economy, id) = fixture(ForceOwner::Local(0));
    let ctx = context();
    open_battle_panel(&ctx, id, 0);
    render(&ctx, &world, &economy, 0, egui::vec2(1600., 900.), 0.1, vec![]);
    let (output, _) = render(&ctx, &world, &economy, 0, egui::vec2(1600., 900.), 0.4, vec![]);
    assert!(text(&output).iter().any(|s| s == "Aquitania's army"));
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(r) if r.fill == army_owner_color(&ctx, ForceOwner::Local(0)) && r.rect.height() == 42.)));
}

fn pointer(pos: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        },
    ]
}

#[test]
fn current_combat_modifiers_are_available_on_icon_hover() {
    let (mut world, economy, id) = fixture(ForceOwner::Player(1));
    world.battles[0].defenders.plans.get_mut(&ForceOwner::Player(1)).unwrap().tactic =
        CombatTactic::Envelopment;
    world.battles[0].advance_month(&world.config);
    let ctx = context();
    open_battle_panel(&ctx, id, 0);
    let size = egui::vec2(1600., 900.);
    render(&ctx, &world, &economy, 0, size, 0.1, vec![]);
    let (output, _) = render(&ctx, &world, &economy, 0, size, 0.4, vec![]);
    let (panel, _) = panel_rect(ctx.content_rect());
    let round = world.battles[0].rounds.last().unwrap();
    for (index, color) in
        [egui::Color32::from_rgb(32, 116, 58), egui::Color32::from_rgb(166, 44, 34)]
            .into_iter()
            .enumerate()
    {
        let modifier =
            ((round.tactics[index][&ForceOwner::Player(index)] - 1.) * 100.).round() as i32;
        assert!(
            output.shapes.iter().any(|shape| matches!(&shape.shape,
                egui::Shape::Text(text) if text.galley.job.text == format!("{modifier:+}%")
                    && text.galley.job.sections[0].format.color == color
            )),
            "Actual tactic modifier must show its sign and color for side {index}"
        );
    }
    let attackers = &world.battles[0].attackers;
    let tactic = attackers.plans[&ForceOwner::Player(0)].tactic;
    let (slot, cohort) = attackers
        .formation
        .front
        .iter()
        .enumerate()
        .find_map(|(slot, id)| {
            id.and_then(|id| {
                attackers.units.iter().find(|unit| {
                    unit.id == id && unit.current_manpower > 0. && !attackers.routed.contains(&id)
                })
            })
            .map(|unit| (slot, unit))
        })
        .unwrap();
    let cells: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Rect(r) if r.fill == TABLE_STRIPE => Some(r.rect),
            _ => None,
        })
        .collect();
    let targets = [
        (
            egui::pos2(panel.right() - 128., panel.top() + 67.),
            format!("Current die: {}", round.dice[0]),
        ),
        (egui::pos2(panel.right() - 94., panel.top() + 67.), tactic.name().to_owned()),
        (
            egui::pos2(panel.right() - 128., panel.top() + 273.),
            format!("Current die: {}", round.dice[1]),
        ),
        (
            egui::pos2(panel.right() - 94., panel.top() + 273.),
            world.battles[0].defenders.plans[&ForceOwner::Player(1)].tactic.name().to_owned(),
        ),
        (egui::pos2(panel.right() - 57., panel.top() + 21.), "Forest · Width: 10 cohorts".into()),
        (egui::pos2(panel.right() - 89., panel.top() + 21.), "Fortifications: level 1".into()),
        (
            cells[attackers.formation.front.len() + slot].center(),
            format!(
                "• Manpower: {}\n• Morale: {:.0}%\n• Training: {:.0}%",
                format_person_count(cohort.people()),
                cohort.morale,
                cohort.training
            ),
        ),
    ];
    let mut time = 1.;
    for (pos, expected) in targets {
        let mut labels = Vec::new();
        // Tooltips need their delay and an extra pass to lay out the popup area.
        for step in 0..4 {
            let events = if step == 0 {
                vec![egui::Event::PointerMoved(pos)]
            } else {
                vec![]
            };
            let (output, _) = render(&ctx, &world, &economy, 0, size, time, events);
            labels = text(&output);
            time += 0.7;
        }
        assert!(
            labels.iter().any(|s| s.contains(&expected)),
            "Missing icon tooltip {expected}: {labels:?}"
        );
        assert!(!labels.iter().any(
            |label| label.contains("composition fit") || label.contains("starting deployment")
        ));
        if expected.starts_with("• Manpower") {
            assert!(labels.iter().any(|s| s == cohort.unit_type.name()));
            assert!(!labels.iter().any(|s| s.contains('#') || s.contains("Frontline")));
        }
    }
    for pos in
        [panel.min + egui::vec2(70., 58.), egui::pos2(panel.right() - 21., panel.top() + 21.)]
    {
        let mut labels = Vec::new();
        for step in 0..4 {
            let events = if step == 0 {
                vec![egui::Event::PointerMoved(pos)]
            } else {
                vec![]
            };
            let (output, _) = render(&ctx, &world, &economy, 0, size, time, events);
            labels = text(&output);
            time += 0.7;
        }
        assert_eq!(
            labels.iter().filter(|s| *s == "Player 1").count(),
            1,
            "Player names must not be repeated in a hover tooltip"
        );
        assert!(!labels.iter().any(|s| s == "Close battle panel"));
    }
}

#[test]
fn circular_close_dismisses_the_battle_and_retreat_preserves_eligibility() {
    let (mut world, economy, id) = fixture(ForceOwner::Player(1));
    let ctx = context();
    open_battle_panel(&ctx, id, 0);
    let size = egui::vec2(1600., 900.);
    render(&ctx, &world, &economy, 0, size, 0.1, vec![]);
    let (output, _) = render(&ctx, &world, &economy, 0, size, 0.4, vec![]);
    let retreat = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(t) if t.galley.job.text == "Retreat" => {
                Some(t.pos + t.galley.size() / 2.)
            },
            _ => None,
        })
        .unwrap();
    render(&ctx, &world, &economy, 0, size, 0.6, pointer(retreat, true));
    let (_, action) = render(&ctx, &world, &economy, 0, size, 0.8, pointer(retreat, false));
    assert!(action.is_none(), "Retreat stays disabled before a completed combat month");
    world.battles[0].months = world.config.minimum_retreat_months;
    render(&ctx, &world, &economy, 0, size, 1., vec![]);
    render(&ctx, &world, &economy, 0, size, 1.2, pointer(retreat, true));
    let (_, action) = render(&ctx, &world, &economy, 0, size, 1.4, pointer(retreat, false));
    assert!(
        matches!(action, Some((0, MilitaryUiAction::Retreat { battle, attacker: true })) if battle == id)
    );
    let (panel, scale) = panel_rect(ctx.content_rect());
    let close = egui::pos2(panel.right() - 21. * scale, panel.top() + 21. * scale);
    render(&ctx, &world, &economy, 0, size, 1.6, pointer(close, true));
    render(&ctx, &world, &economy, 0, size, 1.8, pointer(close, false));
    assert!(selected_battle(&ctx).is_none());
}

#[test]
fn observers_see_public_composition_without_locked_deployment_or_retreat() {
    let (world, economy, id) = fixture(ForceOwner::Player(1));
    let ctx = context();
    open_battle_panel(&ctx, id, 2);
    render(&ctx, &world, &economy, 2, egui::vec2(1600., 900.), 0.1, vec![]);
    let (output, _) = render(&ctx, &world, &economy, 2, egui::vec2(1600., 900.), 0.4, vec![]);
    assert!(!text(&output).iter().any(|s| s == "Retreat"));
    assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(r) if r.rect.height() == 2. && [army_owner_color(&ctx, ForceOwner::Player(0)), army_owner_color(&ctx, ForceOwner::Player(1))].contains(&r.fill))));
    assert!(text(&output).iter().any(|s| s == "Player 1"));
    assert!(text(&output).iter().any(|s| s == "Player 2"));
}

#[test]
fn opposing_frontlines_keep_deployment_slots_and_show_only_living_units() {
    let (mut world, economy, id) = fixture(ForceOwner::Player(1));
    let battle = &mut world.battles[0];
    for (index, side) in [&mut battle.attackers, &mut battle.defenders].into_iter().enumerate() {
        side.formation.front = vec![None; 10];
        side.formation.support = vec![None; 10];
        side.formation.reserves.clear();
        side.formation.front[0] = Some(side.units[0].id);
        side.formation.front[7] = Some(side.units[1].id);
        side.formation.front[9] = Some(side.units[2].id);
        if index == 0 {
            side.routed.insert(side.units[2].id);
        } else {
            side.units[2].current_manpower = 0.;
        }
    }
    let ctx = context();
    open_battle_panel(&ctx, id, 0);
    render(&ctx, &world, &economy, 0, egui::vec2(1600., 900.), 0.1, vec![]);
    let (output, _) = render(&ctx, &world, &economy, 0, egui::vec2(1600., 900.), 0.4, vec![]);
    let bars = [0, 1].map(|player| {
        output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Rect(r)
                    if r.rect.height() == 2.
                        && r.fill == army_owner_color(&ctx, ForceOwner::Player(player)) =>
                {
                    Some(r.rect)
                },
                _ => None,
            })
            .collect::<Vec<_>>()
    });
    for row in &bars {
        assert_eq!(row.len(), 2, "Only living, non-routed cohorts have strength bars");
        assert!(
            (row[1].left() - row[0].left() - 230.).abs() < 0.1,
            "Empty slots must preserve the cohort's deployment column"
        );
    }
    assert!(bars[0][0].top() < bars[1][0].top());
    assert!(
        bars[1][0].top() - bars[0][0].top() < 60.,
        "The opposing frontlines should meet in the middle"
    );
    assert!((bars[0][0].left() - bars[1][0].left()).abs() < 0.1);
    let cells: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Rect(r) if r.fill == TABLE_STRIPE => Some(r.rect),
            _ => None,
        })
        .collect();
    assert_eq!(cells.len(), 40, "Both armies have a front and back row of terrain-width slots");
    let (panel, _) = panel_rect(ctx.content_rect());
    for row in cells.chunks(10) {
        assert!(((row[0].left() + row[9].right()) / 2. - panel.center().x).abs() < 0.1);
    }
    let before = cells;
    let battle = &mut world.battles[0];
    for side in [&mut battle.attackers, &mut battle.defenders] {
        side.units[0].current_manpower *= 0.1;
        side.units[1].current_manpower = 0.;
    }
    let (output, _) = render(&ctx, &world, &economy, 0, egui::vec2(1600., 900.), 1., vec![]);
    let after: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Rect(r) if r.fill == TABLE_STRIPE => Some(r.rect),
            _ => None,
        })
        .collect();
    assert_eq!(before, after, "Casualties cannot resize or move deployment cells");
}

#[test]
fn destroyed_defenders_remain_visible_in_the_completed_battle() {
    let (mut world, economy, id) = fixture(ForceOwner::Local(0));
    for unit in &mut world.battles[0].defenders.units {
        unit.current_manpower = 0.001;
    }
    for _ in 0..128 {
        world.battles[0].advance_round(&world.config);
        if world.battles[0].result.is_some() {
            break;
        }
    }
    assert_eq!(world.battles[0].result, Some(BattleResult::AttackerVictory));
    assert!(world.battles[0].defenders.units.is_empty());
    world.history.push(world.battles.remove(0).into_outcome(&world.config).unwrap());
    assert_eq!(world.history[0].defenders.initial_units.len(), 12);
    assert_eq!(world.history[0].fortification_level, 1);

    for viewer in [0, 2] {
        let ctx = context();
        open_battle_panel(&ctx, id, viewer);
        ctx.data_mut(|data| data.insert_temp(egui::Id::new("battle-review-capture"), true));
        render(&ctx, &world, &economy, viewer, egui::vec2(1600., 900.), 0.1, vec![]);
        let (output, _) =
            render(&ctx, &world, &economy, viewer, egui::vec2(1600., 900.), 0.4, vec![]);
        let (panel, _) = panel_rect(ctx.content_rect());
        let infantry = crate::map::military_unit_icon(&ctx, UnitType::HeavyInfantry);
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Mesh(mesh) if mesh.texture_id == infantry
                && mesh.vertices.iter().all(|v| v.pos.y > panel.top() + 168. && v.color.a() == 90)
        )), "Destroyed defender cohorts must remain visible to viewer {viewer}");
        assert!(text(&output).iter().any(|label| label == "Aquitania's army"));
        assert!(!text(&output).iter().any(|label| label == "Retreat"));
    }
}

#[test]
fn every_terrain_has_a_background_and_both_summaries_stay_outside_the_unit_rows() {
    for terrain in [
        MilitaryTerrain::Farmland,
        MilitaryTerrain::Plains,
        MilitaryTerrain::Forest,
        MilitaryTerrain::Hills,
        MilitaryTerrain::Mountains,
        MilitaryTerrain::Desert,
        MilitaryTerrain::Marsh,
    ] {
        let (mut world, economy, id) = fixture(ForceOwner::Player(1));
        let battle = &mut world.battles[0];
        battle.terrain = terrain;
        let width = world.config.combat_widths[terrain as usize];
        for side in [&mut battle.attackers, &mut battle.defenders] {
            side.formation =
                deploy_formation(&side.units, &side.plans, width, &BTreeSet::new(), &world.config);
        }
        let ctx = context();
        // Viewing as defender must not swap the two sides of the battlefield.
        open_battle_panel(&ctx, id, 1);
        render(&ctx, &world, &economy, 1, egui::vec2(1600., 900.), 0.1, vec![]);
        let (output, _) = render(&ctx, &world, &economy, 1, egui::vec2(1600., 900.), 0.4, vec![]);
        let (panel, _) = panel_rect(ctx.content_rect());
        let field =
            egui::Rect::from_min_size(panel.min + egui::vec2(12., 92.), egui::vec2(876., 154.));
        let texture = terrain_portrait(&ctx, terrain).id();
        assert!(
            output.shapes.iter().any(|shape| matches!(&shape.shape,
                egui::Shape::Mesh(mesh) if mesh.texture_id == texture && mesh.calc_bounds() == field
            )),
            "Missing {terrain:?} battlefield landscape"
        );
        for shape in &output.shapes {
            match &shape.shape {
                egui::Shape::Rect(rect) if rect.fill == TABLE_STRIPE => {
                    assert!(field.contains_rect(rect.rect))
                },
                egui::Shape::Text(text)
                    if matches!(
                        text.galley.job.text.as_str(),
                        "Player 1" | "Player 2" | "100%"
                    ) =>
                {
                    assert!(
                        !field.intersects(egui::Rect::from_min_size(text.pos, text.galley.size()))
                    );
                },
                _ => {},
            }
        }
    }
}
