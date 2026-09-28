use super::*;

#[test]
fn scouting_excludes_neutral_guests_and_includes_engaged_hostile_cohorts() {
    let mut world = MilitaryWorld::new(2);
    let enemy = ForceOwner::Player(1);
    let local = ForceOwner::Local(1);
    let hostile_id = world.seed_unit(1, enemy, UnitType::HeavyInfantry).unwrap();
    world.seed_unit(1, ForceOwner::Player(2), UnitType::LightCavalry).unwrap();
    world.seed_unit(1, local, UnitType::LightInfantry).unwrap();
    world
        .start_battle(1, &[enemy], &[local], None, Some(0), MilitaryTerrain::Plains, 0, 9)
        .unwrap();
    let known = known_hostile_units(&world, 1, ForceOwner::Player(0), |_, target| target == enemy);
    assert_eq!(known.len(), 1);
    assert_eq!(known[0].id, hostile_id);
    assert_eq!(known[0].unit_type, UnitType::HeavyInfantry);
}

#[test]
fn current_army_includes_engaged_cohorts_and_uses_the_locked_plan() {
    let mut world = MilitaryWorld::new(2);
    let owner = ForceOwner::Player(0);
    let enemy = ForceOwner::Local(0);
    let first = world.seed_unit(0, owner, UnitType::HeavyInfantry).unwrap();
    world.seed_unit(0, owner, UnitType::LightCavalry).unwrap();
    world.seed_unit(0, enemy, UnitType::LightInfantry).unwrap();
    let moving = world.seed_unit(1, owner, UnitType::Archers).unwrap();
    let plan = BattlePlan {
        tactic: CombatTactic::Envelopment,
        flank_size: 2,
        ..Default::default()
    };
    world.set_plan(0, owner, plan).unwrap();
    world.start_battle(0, &[owner], &[enemy], None, None, MilitaryTerrain::Plains, 0, 10).unwrap();
    // A stale province default must never replace the battle's captured plan.
    world.provinces[0].plans.insert(owner, BattlePlan::default());
    let units = province_force_units(&world, 0, owner);
    assert_eq!(units.len(), 2);
    assert!(units.iter().any(|unit| unit.id == first));
    assert!(!units.iter().any(|unit| unit.id == moving || unit.owner != owner));
    assert_eq!(province_force_plan(&world, 0, owner), plan);
}

#[test]
fn overview_separates_owners_and_marching_forces_without_losing_battle_units() {
    let mut world = MilitaryWorld::new(3);
    let own = ForceOwner::Player(0);
    let guest = ForceOwner::Player(1);
    let first = world.seed_unit(0, own, UnitType::HeavyInfantry).unwrap();
    let second = world.seed_unit(0, own, UnitType::Archers).unwrap();
    world.seed_unit(0, guest, UnitType::LightCavalry).unwrap();
    world.seed_unit(0, ForceOwner::Local(0), UnitType::LightInfantry).unwrap();
    let dead = world.seed_unit(0, ForceOwner::Player(2), UnitType::Catapult).unwrap();
    world.provinces[0].forces.get_mut(&ForceOwner::Player(2)).unwrap()[0].current_manpower = 0.;
    world.seed_unit(2, guest, UnitType::HeavyInfantry).unwrap();
    let plan = BattlePlan {
        tactic: CombatTactic::Envelopment,
        ..Default::default()
    };
    world.set_plan(0, own, plan).unwrap();
    {
        let units = world.provinces[0].forces.get_mut(&own).unwrap();
        units[0].current_manpower = 10.;
        units[0].morale = 20.;
        units[0].training = 40.;
        units[1].current_manpower = 30.;
        units[1].morale = 100.;
        units[1].training = 80.;
    }
    world.start_battle(0, &[own], &[guest], None, None, MilitaryTerrain::Plains, 0, 10).unwrap();
    world.provinces[0].plans.insert(own, BattlePlan::default());
    let graph: Vec<_> = (0..3)
        .map(|id| MilitaryProvince {
            terrain: MilitaryTerrain::Plains,
            area: 100.,
            road_level: 0,
            neighbors: (0..3).filter(|other| *other != id).collect(),
        })
        .collect();
    let marching = world.seed_unit(1, own, UnitType::LightCavalry).unwrap();
    let marching_again = world.seed_unit(1, own, UnitType::Archers).unwrap();
    let stationary = world.seed_unit(1, own, UnitType::LightInfantry).unwrap();
    for id in [marching, marching_again] {
        world
            .order_movement(1, 2, own, &[id], Some(plan), &graph, |_, _| MilitaryAccess::Peaceful)
            .unwrap();
    }
    let rows = army_overview_rows(&world, 0);
    assert_eq!(rows.len(), 6, "Three owners in Italia, plus a stationary and two marching armies");
    assert_eq!(rows.iter().filter(|row| row.owner == own).count(), 4);
    let army = &rows[0];
    assert_eq!(army.owner, own);
    assert_eq!(army.province, 0);
    assert!(army.in_battle);
    assert_eq!(army.tactic, Some(CombatTactic::Envelopment));
    assert_eq!(army.averages(), (80., 70.));
    assert!(rows.iter().filter(|row| row.owner != own).all(|row| row.tactic.is_none()));
    assert!(!rows.iter().any(|row| row.province == 2));
    let own_ids: Vec<_> = rows
        .iter()
        .filter(|row| row.owner == own)
        .flat_map(|row| row.units.iter().map(|unit| unit.id))
        .collect();
    for id in [first, second, marching, marching_again, stationary] {
        assert_eq!(own_ids.iter().filter(|actual| **actual == id).count(), 1);
    }
    assert!(!rows.iter().flat_map(|row| &row.units).any(|unit| unit.id == dead));
    for row in rows.iter().filter(|row| row.movement.is_some()) {
        assert_eq!(row.province, 1);
        assert_eq!(row.destination, Some(2));
        assert_eq!(row.tactic, Some(CombatTactic::Envelopment));
    }
}

fn render_overview(
    ctx: &egui::Context,
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    width: f32,
    scale: f32,
    time: f64,
    events: Vec<egui::Event>,
) -> (egui::FullOutput, Option<ProvinceId>) {
    let mut selected = None;
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 650.))),
            time: Some(time),
            events,
            ..Default::default()
        },
        |ui| {
            *ui.style_mut() = super::super::campaign_widgets::map_style(scale);
            selected =
                overview(ui, world, economy, 0, &[egui::Color32::RED, egui::Color32::BLUE], scale);
        },
    );
    output.textures_delta.clear();
    (output, selected)
}

#[test]
fn overview_renders_compact_composition_and_foreign_row_opens_the_province() {
    let (mut world, mut economy, _) = military_fixture();
    economy.provinces[0].name = "Africa Proconsularis et Numidia".into();
    for kind in UnitType::ALL {
        world.seed_unit(0, ForceOwner::Player(0), kind).unwrap();
    }
    world.seed_unit(0, ForceOwner::Player(1), UnitType::Archers).unwrap();
    for (width, scale) in [(796., 1.), (380., 1.), (380., 0.85)] {
        let ctx = egui::Context::default();
        let (output, _) = render_overview(&ctx, &world, &economy, width, scale, 0., vec![]);
        for label in [
            "Province / army",
            "Units",
            "Tactic",
            "Morale",
            "Avg. training",
            "You",
            "Player 2",
            "?",
        ] {
            text_position(&output, label);
        }
        for forbidden in ["Recruit units", "Available cohorts", "ARMY ORDERS", "+"] {
            assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == forbidden)));
        }
        for clipped in &output.shapes {
            if let egui::Shape::Text(text) = &clipped.shape {
                let visible = egui::Rect::from_min_size(text.pos, text.galley.size())
                    .intersect(clipped.clip_rect);
                assert!(
                    visible.right() <= width + 0.5,
                    "Text escaped overview: {}",
                    text.galley.job.text
                );
            }
        }
        let row = text_position(&output, "Player 2");
        render_overview(&ctx, &world, &economy, width, scale, 0.1, click_events(row, true));
        let (_, selected) =
            render_overview(&ctx, &world, &economy, width, scale, 0.2, click_events(row, false));
        assert_eq!(selected, Some(0), "Every owner's row opens the same province");
    }
}

#[test]
fn overview_shows_an_empty_state_and_opening_an_army_leaves_the_recruitment_tab() {
    let (mut world, economy, graph) = military_fixture();
    let ctx = egui::Context::default();
    ctx.data_mut(|data| {
        data.insert_temp(
            egui::Id::new(("province-military-tab", 0_usize, 0_usize)),
            MilitaryTab::Recruitment,
        )
    });
    open_army_tab(&ctx, 0, 0);
    let (output, _) = render_military(&ctx, &world, &economy, &graph, 0., vec![]);
    assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == "Available cohorts")));
    world.provinces[0].forces.clear();
    let (output, selected) = render_overview(&ctx, &world, &economy, 500., 1., 0.1, vec![]);
    text_position(&output, "You have no armies.");
    assert!(selected.is_none());
}

#[test]
fn overview_scrolls_to_all_armies_while_keeping_column_headers_visible() {
    let (_, mut economy, _) = military_fixture();
    let province = economy.provinces[0].clone();
    economy.provinces = (0..12)
        .map(|id| {
            let mut province = province.clone();
            province.name = format!("Province {id:02}");
            province
        })
        .collect();
    let mut world = MilitaryWorld::new(12);
    for id in 0..12 {
        world.seed_unit(id, ForceOwner::Player(0), UnitType::LightInfantry).unwrap();
    }
    let ctx = egui::Context::default();
    render_overview(&ctx, &world, &economy, 796., 1., 0., vec![]);
    render_overview(
        &ctx,
        &world,
        &economy,
        796.,
        1.,
        0.1,
        vec![
            egui::Event::PointerMoved(egui::pos2(400., 400.)),
            egui::Event::MouseWheel {
                phase: egui::TouchPhase::Move,
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0., -2000.),
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    let mut output = render_overview(&ctx, &world, &economy, 796., 1., 0.2, vec![]).0;
    for frame in 3..15 {
        output = render_overview(&ctx, &world, &economy, 796., 1., frame as f64 * 0.1, vec![]).0;
    }
    text_position(&output, "Province / army");
    let last_army = text_position(&output, "Province 11");
    assert!(last_army.y < 650.);
    render_overview(&ctx, &world, &economy, 796., 1., 1.5, click_events(last_army, true));
    let (_, selected) =
        render_overview(&ctx, &world, &economy, 796., 1., 1.6, click_events(last_army, false));
    assert_eq!(selected, Some(11));
}

fn military_fixture() -> (MilitaryWorld, EconomyWorld, Vec<MilitaryProvince>) {
    use crate::game::economy::{EconomicProvince, Terrain};
    let mut province = EconomicProvince::new(
        "Italia",
        60.,
        Terrain::Plains,
        false,
        [10.; 3],
        [10., 40., 80., 10.],
        1,
    );
    province.owner = Some(0);
    let mut economy = EconomyWorld::new(1, vec![province], vec![vec![]]);
    economy.players[0].resources[1] = 500.;
    let mut world = MilitaryWorld::new(1);
    world.seed_unit(0, ForceOwner::Player(0), UnitType::HeavyInfantry).unwrap();
    (
        world,
        economy,
        vec![MilitaryProvince {
            terrain: MilitaryTerrain::Plains,
            area: 60.,
            road_level: 0,
            neighbors: vec![],
        }],
    )
}

fn render_military(
    ctx: &egui::Context,
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    graph: &[MilitaryProvince],
    time: f64,
    events: Vec<egui::Event>,
) -> (egui::FullOutput, Option<MilitaryUiAction>) {
    let mut action = None;
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(500., 1400.))),
            time: Some(time),
            events,
            ..Default::default()
        },
        |ui| {
            *ui.style_mut() = super::super::campaign_widgets::map_style(1.);
            action = show(
                ui,
                world,
                economy,
                graph,
                0,
                0,
                |_| true,
                |_| None,
                |_, _| MilitaryAccess::Peaceful,
                |a, b| a != b,
            );
        },
    );
    // These headless interaction checks have no GPU to apply texture updates.
    output.textures_delta.clear();
    (output, action)
}

fn text_position(output: &egui::FullOutput, label: &str) -> egui::Pos2 {
    output
        .shapes
        .iter()
        .find_map(|shape| {
            if let egui::Shape::Text(text) = &shape.shape {
                (text.galley.job.text == label).then_some(text.pos + text.galley.size() * 0.5)
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("Missing military control: {label}"))
}

fn click_events(position: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(position),
        egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        },
    ]
}

#[test]
fn recruitment_tab_opens_the_roster_and_dispatches_a_paid_recruitment_intent() {
    let (world, economy, graph) = military_fixture();
    let ctx = egui::Context::default();
    let (output, action) = render_military(&ctx, &world, &economy, &graph, 0., vec![]);
    assert!(action.is_none());
    assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == "Available cohorts")));
    let tab = text_position(&output, "Recruit units");
    render_military(&ctx, &world, &economy, &graph, 0.1, click_events(tab, true));
    let (output, _) =
        render_military(&ctx, &world, &economy, &graph, 0.2, click_events(tab, false));
    text_position(&output, "Available cohorts");
    let recruit = text_position(&output, "+");
    render_military(&ctx, &world, &economy, &graph, 0.3, click_events(recruit, true));
    let (_, action) =
        render_military(&ctx, &world, &economy, &graph, 0.4, click_events(recruit, false));
    assert!(matches!(action, Some(MilitaryUiAction::Recruit(UnitType::LightInfantry))));
    assert_eq!(
        economy.players[0].resources[1], 500.,
        "The command bridge owns payment, not rendering"
    );
    assert!(world.provinces[0].recruitment.is_none());
}

#[test]
fn selecting_a_tactic_saves_the_stationary_army_plan() {
    let (world, economy, graph) = military_fixture();
    let ctx = egui::Context::default();
    let (output, _) = render_military(&ctx, &world, &economy, &graph, 0., vec![]);
    let tactic = text_position(&output, "Balanced · neutral");
    render_military(&ctx, &world, &economy, &graph, 0.1, click_events(tactic, true));
    render_military(&ctx, &world, &economy, &graph, 0.2, click_events(tactic, false));
    let (output, _) = render_military(&ctx, &world, &economy, &graph, 0.3, vec![]);
    let shock = text_position(&output, "Shock Action");
    render_military(&ctx, &world, &economy, &graph, 0.4, click_events(shock, true));
    let (_, action) =
        render_military(&ctx, &world, &economy, &graph, 0.5, click_events(shock, false));
    assert!(matches!(
        action,
        Some(MilitaryUiAction::SavePlan(BattlePlan {
            tactic: CombatTactic::ShockAction,
            ..
        }))
    ));
}

#[test]
fn frontline_selector_changes_the_preference_without_creating_cohorts() {
    let (world, economy, graph) = military_fixture();
    let ctx = egui::Context::default();
    let (output, _) = render_military(&ctx, &world, &economy, &graph, 0., vec![]);
    let frontline = text_position(&output, "Frontline") + egui::vec2(0., 32.);
    render_military(&ctx, &world, &economy, &graph, 0.1, click_events(frontline, true));
    render_military(&ctx, &world, &economy, &graph, 0.2, click_events(frontline, false));
    let (output, _) = render_military(&ctx, &world, &economy, &graph, 0.3, vec![]);
    // The rear-line summary also says Light Infantry; choose the popup entry.
    let infantry = output
        .shapes
        .iter()
        .rev()
        .find_map(|shape| {
            if let egui::Shape::Text(text) = &shape.shape {
                (text.galley.job.text == "Light Infantry")
                    .then_some(text.pos + text.galley.size() * 0.5)
            } else {
                None
            }
        })
        .expect("Light Infantry preference in the open selector");
    render_military(&ctx, &world, &economy, &graph, 0.4, click_events(infantry, true));
    let (_, action) =
        render_military(&ctx, &world, &economy, &graph, 0.5, click_events(infantry, false));
    assert!(matches!(
        action,
        Some(MilitaryUiAction::SavePlan(BattlePlan {
            primary_unit_type: UnitType::LightInfantry,
            ..
        }))
    ));
    assert_eq!(world.provinces[0].forces[&ForceOwner::Player(0)].len(), 1);
}

#[test]
fn engaged_army_disables_tactic_changes_and_recruitment() {
    let (mut world, economy, graph) = military_fixture();
    let owner = ForceOwner::Player(0);
    let local = ForceOwner::Local(0);
    world.seed_unit(0, local, UnitType::LightInfantry).unwrap();
    world.start_battle(0, &[owner], &[local], None, None, MilitaryTerrain::Plains, 0, 10).unwrap();
    let ctx = egui::Context::default();
    let (output, _) = render_military(&ctx, &world, &economy, &graph, 0., vec![]);
    let tactic = text_position(&output, "Balanced · neutral");
    render_military(&ctx, &world, &economy, &graph, 0.1, click_events(tactic, true));
    let (output, action) =
        render_military(&ctx, &world, &economy, &graph, 0.2, click_events(tactic, false));
    assert!(action.is_none());
    assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text.starts_with("Shock Action"))));
    let tab = text_position(&output, "Recruit units");
    render_military(&ctx, &world, &economy, &graph, 0.3, click_events(tab, true));
    let (output, _) =
        render_military(&ctx, &world, &economy, &graph, 0.4, click_events(tab, false));
    let recruit = text_position(&output, "+");
    render_military(&ctx, &world, &economy, &graph, 0.5, click_events(recruit, true));
    let (_, action) =
        render_military(&ctx, &world, &economy, &graph, 0.6, click_events(recruit, false));
    assert!(action.is_none());
}
