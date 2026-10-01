use super::*;

fn fixture() -> (MilitaryWorld, EconomyWorld, Vec<MilitaryProvince>) {
    use crate::game::economy::{EconomicProvince, Terrain};
    let mut provinces: Vec<_> = (0..3)
        .map(|id| {
            EconomicProvince::new(
                format!("Province {id}"),
                60.,
                Terrain::Plains,
                false,
                [10.; 3],
                [10., 40., 80., 10.],
                1,
            )
        })
        .collect();
    provinces[0].owner = Some(0);
    provinces[1].owner = Some(0);
    let economy = EconomyWorld::new(1, provinces, vec![vec![1, 2], vec![0], vec![0]]);
    let mut world = MilitaryWorld::new(3);
    for kind in [UnitType::HeavyInfantry, UnitType::HeavyInfantry, UnitType::LightCavalry] {
        world.seed_unit(0, ForceOwner::Player(0), kind).unwrap();
    }
    let graph = (0..3)
        .map(|id| MilitaryProvince {
            terrain: MilitaryTerrain::Plains,
            area: 60.,
            road_level: 0,
            neighbors: if id == 0 {
                vec![1, 2]
            } else {
                vec![0]
            },
        })
        .collect();
    (world, economy, graph)
}

fn render(
    ctx: &egui::Context,
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    graph: &[MilitaryProvince],
    time: f64,
    events: Vec<egui::Event>,
) -> (egui::FullOutput, Option<(usize, MilitaryUiAction)>) {
    let mut action = None;
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600., 900.))),
            time: Some(time),
            events,
            ..Default::default()
        },
        |_| {
            action = draw_orders_menu(
                ctx,
                world,
                economy,
                graph,
                0,
                |_, _| MilitaryAccess::Peaceful,
                |_| true,
                "",
            );
        },
    );
    output.textures_delta.clear();
    (output, action)
}

fn label(output: &egui::FullOutput, prefix: &str) -> egui::Pos2 {
    output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text.starts_with(prefix) => {
                Some(text.pos + text.galley.size() * 0.5)
            },
            _ => None,
        })
        .unwrap_or_else(|| panic!("Missing {prefix}"))
}

fn click(
    ctx: &egui::Context,
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    graph: &[MilitaryProvince],
    position: egui::Pos2,
) -> Option<(usize, MilitaryUiAction)> {
    let events = |pressed| {
        vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    };
    render(ctx, world, economy, graph, 0.1, events(true));
    render(ctx, world, economy, graph, 0.2, events(false)).1
}

#[test]
fn cohort_picker_half_sends_a_detachment_and_preserves_plan() {
    let (mut world, economy, graph) = fixture();
    let ctx = egui::Context::default();
    open_army_panel(&ctx, 0, 0, None);
    let plan = BattlePlan {
        tactic: CombatTactic::ShockAction,
        ..Default::default()
    };
    world.provinces[0].plans.insert(ForceOwner::Player(0), plan);
    let army = selected_army(&ctx).unwrap();
    ctx.data_mut(|data| {
        data.insert_temp(
            egui::Id::new(MENU_ID),
            ProvinceOrders {
                army,
                destination: 1,
                position: egui::pos2(300., 250.),
                units: world.provinces[0].forces[&ForceOwner::Player(0)]
                    .iter()
                    .map(|u| u.id)
                    .collect(),
            },
        )
    });
    render(&ctx, &world, &economy, &graph, 0., vec![]);
    let output = render(&ctx, &world, &economy, &graph, 0., vec![]).0;
    click(&ctx, &world, &economy, &graph, label(&output, "Half"));
    let menu: ProvinceOrders = ctx.data(|data| data.get_temp(egui::Id::new(MENU_ID))).unwrap();
    assert_eq!(menu.units.len(), 2);
    let output = render(&ctx, &world, &economy, &graph, 0.3, vec![]).0;
    let action = click(&ctx, &world, &economy, &graph, label(&output, "Move   ·"));
    let Some((
        0,
        MilitaryUiAction::Move {
            units,
            route,
            plan: captured,
            ..
        },
    )) = action
    else {
        panic!("Move should issue selected cohorts")
    };
    assert_eq!(units.len(), 2);
    assert_eq!(route, vec![1]);
    assert_eq!(captured, plan);
}

#[test]
fn menu_stays_inside_screen_at_bottom_right_and_can_cancel() {
    let (world, economy, graph) = fixture();
    let ctx = egui::Context::default();
    open_army_panel(&ctx, 0, 0, None);
    let army = selected_army(&ctx).unwrap();
    ctx.data_mut(|data| {
        data.insert_temp(
            egui::Id::new(MENU_ID),
            ProvinceOrders {
                army,
                destination: 2,
                position: egui::pos2(1595., 895.),
                units: BTreeSet::new(),
            },
        )
    });
    render(&ctx, &world, &economy, &graph, 0., vec![]);
    let output = render(&ctx, &world, &economy, &graph, 0., vec![]).0;
    let attack = label(&output, "Attack");
    let pressure = label(&output, "Pressure");
    assert!(attack.x < 1600. && pressure.y < 900.);
    assert!(
        click(&ctx, &world, &economy, &graph, attack).is_none(),
        "empty detachment cannot attack"
    );
    assert!(dismiss(&ctx));
    assert!(!dismiss(&ctx));
    assert!(selected_army(&ctx).is_some(), "cancelling orders retains the selected army");
}

#[test]
fn half_does_not_send_all_singleton_unit_types() {
    let (mut world, economy, graph) = fixture();
    world.provinces[0].forces.get_mut(&ForceOwner::Player(0)).unwrap().remove(0);
    let ctx = egui::Context::default();
    open_army_panel(&ctx, 0, 0, None);
    let army = selected_army(&ctx).unwrap();
    ctx.data_mut(|data| {
        data.insert_temp(
            egui::Id::new(MENU_ID),
            ProvinceOrders {
                army,
                destination: 1,
                position: egui::pos2(300., 250.),
                units: BTreeSet::new(),
            },
        )
    });
    render(&ctx, &world, &economy, &graph, 0., vec![]);
    let output = render(&ctx, &world, &economy, &graph, 0., vec![]).0;
    click(&ctx, &world, &economy, &graph, label(&output, "Half"));
    let menu: ProvinceOrders = ctx.data(|data| data.get_temp(egui::Id::new(MENU_ID))).unwrap();
    assert_eq!(menu.units.len(), 1, "one cohort must remain behind");
}
