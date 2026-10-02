use super::*;

use crate::egui_capture as capture;

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
fn an_owned_occupied_province_offers_an_attack_to_liberate_it() {
    let (mut world, mut economy, graph) = fixture();
    world.seed_unit(1, ForceOwner::Player(1), UnitType::HeavyInfantry).unwrap();
    world.provinces[1].occupation = Some(ForceOwner::Player(1));
    economy.provinces[1].occupied = true;
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
                units: world.provinces[0].forces[&ForceOwner::Player(0)]
                    .iter()
                    .map(|unit| unit.id)
                    .collect(),
            },
        );
    });
    render(&ctx, &world, &economy, &graph, 0., vec![]);
    let (output, _) = render(&ctx, &world, &economy, &graph, 0.05, vec![]);
    let attack = label(&output, "Attack");
    let action = click(&ctx, &world, &economy, &graph, attack);
    assert!(matches!(
        action,
        Some((
            0,
            MilitaryUiAction::Attack {
                destination: 1,
                ..
            }
        ))
    ));
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
    let action = click(&ctx, &world, &economy, &graph, label(&output, "Move"));
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

#[test]
fn rome_orders_offer_a_direct_attack_only_from_the_three_border_provinces() {
    let (world, mut economy, graph) = fixture();
    economy.provinces[2].name = "Rome".into();
    let ctx = egui::Context::default();
    open_army_panel(&ctx, 0, 0, None);
    let army = selected_army(&ctx).unwrap();
    let menu = ProvinceOrders {
        army,
        destination: 2,
        position: egui::Pos2::ZERO,
        units: BTreeSet::new(),
    };
    let units = &world.provinces[0].forces[&ForceOwner::Player(0)];
    let access = |_, _| MilitaryAccess::Peaceful;
    assert!(order_route(&world, &economy, &graph, &menu, units, ArmyOrderKind::Attack, &access)
        .unwrap_err()
        .contains("Etruria, Latium, or Samnium"));
    for name in ROME_APPROACHES {
        economy.provinces[0].name = name.into();
        assert_eq!(
            order_route(&world, &economy, &graph, &menu, units, ArmyOrderKind::Attack, &access)
                .unwrap(),
            vec![2]
        );
        assert!(order_route(&world, &economy, &graph, &menu, units, ArmyOrderKind::Move, &access)
            .is_err());
        assert!(order_route(
            &world,
            &economy,
            &graph,
            &menu,
            units,
            ArmyOrderKind::Pressure,
            &access
        )
        .is_err());
    }
}

#[test]
fn destination_picker_hides_army_details_and_preserves_selection_on_cancel() {
    let (world, economy, graph) = fixture();
    let ctx = egui::Context::default();
    open_army_panel(&ctx, 0, 0, None);
    let army = selected_army(&ctx).unwrap();
    assert!(army_details_open(&ctx));
    let mut destination = crate::map::take_province_order_click(&ctx).unwrap_or_default();
    destination.province = 2;
    destination.position = egui::pos2(300., 250.);
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("map-province-order-click"), destination));
    assert!(prepare_orders(&ctx, &world, &economy, 0, None));
    assert!(is_open(&ctx));
    assert!(!prepare_orders(&ctx, &world, &economy, 0, None), "Consume each right-click only once");
    assert!(!army_details_open(&ctx));
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600., 900.))),
            ..Default::default()
        },
        |_| {
            assert!(super::super::draw_army_panel(
                &ctx,
                &world,
                &economy,
                &graph,
                0,
                |_| true,
                |_| None,
                |_, _| MilitaryAccess::Peaceful,
                |a, b| a != b
            )
            .is_none());
        },
    );
    output.textures_delta.clear();
    assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
        egui::Shape::Text(text) if text.galley.job.text == "Deployment" || text.galley.job.text == "Details")), "Only the destination picker should cover the map");
    assert_eq!(
        ctx.data(|data| data
            .get_temp::<(usize, Option<usize>)>(egui::Id::new("map-army-order-selection"))),
        Some((0, Some(2)))
    );
    assert!(dismiss(&ctx));
    assert_eq!(selected_army(&ctx), Some(army));
    assert!(!army_details_open(&ctx), "Cancelling leaves the map clear and retains the origin");
    assert_eq!(
        ctx.data(|data| data
            .get_temp::<(usize, Option<usize>)>(egui::Id::new("map-army-order-selection"))),
        Some((0, None))
    );
    for frame in 0..3 {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1600., 900.),
                )),
                time: Some(frame as f64 * 0.1),
                ..Default::default()
            },
            |_| {
                super::super::draw_army_panel(
                    &ctx,
                    &world,
                    &economy,
                    &graph,
                    0,
                    |_| true,
                    |_| None,
                    |_, _| MilitaryAccess::Peaceful,
                    |a, b| a != b,
                );
            },
        );
        output.textures_delta.clear();
        assert!(output.shapes.is_empty(), "No selection card should appear after cancellation");
    }
    toggle_map_army_panel(&ctx, 0, 0, None);
    assert_eq!(selected_army(&ctx), Some(army));
    assert!(army_details_open(&ctx), "Clicking the retained army reopens its full panel");
}

#[test]
fn province_orders_allow_the_players_stationed_units_in_any_province() {
    for owner in [Some(0), Some(1), None] {
        for has_units in [false, true] {
            let (mut world, mut economy, graph) = fixture();
            economy.provinces[1].owner = owner;
            world.seed_unit(1, ForceOwner::Local(1), UnitType::HeavyInfantry).unwrap();
            world.seed_unit(1, ForceOwner::Player(1), UnitType::LightInfantry).unwrap();
            let own_unit = has_units.then(|| {
                world.seed_unit(1, ForceOwner::Player(0), UnitType::LightInfantry).unwrap()
            });
            let ctx = egui::Context::default();
            open_army_panel(&ctx, 0, 0, None);
            let previous_army = selected_army(&ctx);
            let destination = crate::map::take_province_order_click(&ctx).unwrap_or_default();
            ctx.data_mut(|data| {
                data.insert_temp(egui::Id::new("map-province-order-click"), destination);
            });
            let allowed = has_units;
            assert_eq!(prepare_orders(&ctx, &world, &economy, 0, Some(1)), allowed);
            assert_eq!(is_open(&ctx), allowed);
            assert!(crate::map::take_province_order_click(&ctx).is_none());
            if allowed {
                let menu: ProvinceOrders =
                    ctx.data(|data| data.get_temp(egui::Id::new(MENU_ID))).unwrap();
                assert_eq!(
                    menu.army.province, 1,
                    "Use the inspected province, not the previous army"
                );
                assert_eq!(menu.destination, 0);
                assert_eq!(menu.units, own_unit.into_iter().collect());
                assert!(!army_details_open(&ctx));
                render(&ctx, &world, &economy, &graph, 0., vec![]);
                let (output, action) = render(&ctx, &world, &economy, &graph, 0.1, vec![]);
                assert!(action.is_none());
                label(&output, "Send cohorts");
            } else {
                assert_eq!(selected_army(&ctx), previous_army, "Ignored clicks keep the selection");
                assert!(army_details_open(&ctx), "Ignored clicks leave the current panel alone");
                let (output, action) = render(&ctx, &world, &economy, &graph, 0., vec![]);
                assert!(
                    output.shapes.is_empty(),
                    "An invalid origin must not paint an orders panel"
                );
                assert!(action.is_none());
            }
        }
    }
}

#[test]
fn retained_army_selection_can_order_foreign_stationed_troops_but_not_an_empty_force() {
    for owner in [Some(0), Some(1), None] {
        for has_units in [false, true] {
            let (mut world, mut economy, _) = fixture();
            economy.provinces[0].owner = owner;
            if !has_units {
                world.provinces[0].forces.insert(ForceOwner::Player(0), Vec::new());
            }
            let ctx = egui::Context::default();
            open_army_panel(&ctx, 0, 0, None);
            let mut destination = crate::map::take_province_order_click(&ctx).unwrap_or_default();
            destination.province = 2;
            ctx.data_mut(|data| {
                data.insert_temp(egui::Id::new("map-province-order-click"), destination);
            });
            let allowed = has_units;
            assert_eq!(prepare_orders(&ctx, &world, &economy, 0, None), allowed);
            assert_eq!(is_open(&ctx), allowed);
        }
    }
}

#[test]
fn orders_survive_an_ownership_change_and_close_when_the_stationed_units_leave() {
    for loses_units in [false, true] {
        let (mut world, mut economy, graph) = fixture();
        let ctx = egui::Context::default();
        open_army_panel(&ctx, 0, 0, None);
        let mut destination = crate::map::take_province_order_click(&ctx).unwrap_or_default();
        destination.province = 2;
        ctx.data_mut(|data| {
            data.insert_temp(egui::Id::new("map-province-order-click"), destination);
        });
        assert!(prepare_orders(&ctx, &world, &economy, 0, None));
        if loses_units {
            world.provinces[0].forces.remove(&ForceOwner::Player(0));
        } else {
            economy.provinces[0].owner = Some(1);
        }
        let (output, action) = render(&ctx, &world, &economy, &graph, 0., vec![]);
        assert_eq!(output.shapes.is_empty(), loses_units);
        assert!(action.is_none());
        assert_eq!(is_open(&ctx), !loses_units);
    }
}

#[test]
fn a_foreign_stationed_army_can_be_sent_home_without_ordering_the_hosts_units() {
    let (mut world, economy, graph) = fixture();
    let own = ForceOwner::Player(0);
    let unit = world.seed_unit(2, own, UnitType::LightInfantry).unwrap();
    let local = ForceOwner::Local(2);
    let host = world.seed_unit(2, local, UnitType::HeavyInfantry).unwrap();
    let ctx = egui::Context::default();
    open_army_panel(&ctx, 2, 0, None);
    let mut destination = crate::map::take_province_order_click(&ctx).unwrap_or_default();
    destination.province = 0;
    destination.position = egui::pos2(300., 250.);
    ctx.data_mut(|data| {
        data.insert_temp(egui::Id::new("map-province-order-click"), destination);
    });
    assert!(prepare_orders(&ctx, &world, &economy, 0, None));
    render(&ctx, &world, &economy, &graph, 0., vec![]);
    let output = render(&ctx, &world, &economy, &graph, 0.05, vec![]).0;
    let Some((
        province,
        MilitaryUiAction::Move {
            units,
            route,
            plan,
            ..
        },
    )) = click(&ctx, &world, &economy, &graph, label(&output, "Move"))
    else {
        panic!("A foreign stationed army must be able to move home");
    };
    assert_eq!(province, 2);
    assert_eq!(units, vec![unit]);
    assert_eq!(route, vec![0]);
    world
        .order_movement_route(province, own, &units, Some(plan), &route, &graph, |_, _| {
            MilitaryAccess::Peaceful
        })
        .unwrap();
    assert_eq!(world.movements[0].units[0].id, unit);
    assert_eq!(world.provinces[2].forces[&local][0].id, host);
}

#[test]
fn armies_in_battle_or_on_the_march_cannot_open_stationed_orders() {
    let (mut world, economy, _) = fixture();
    let ctx = egui::Context::default();
    let own = ForceOwner::Player(0);
    world.seed_unit(0, ForceOwner::Local(0), UnitType::LightInfantry).unwrap();
    world
        .start_battle(
            0,
            &[ForceOwner::Local(0)],
            &[own],
            Some(0),
            None,
            MilitaryTerrain::Plains,
            0,
            1,
        )
        .unwrap();
    world.seed_unit(0, own, UnitType::LightInfantry).unwrap();
    open_army_panel(&ctx, 0, 0, None);
    let mut destination = crate::map::take_province_order_click(&ctx).unwrap_or_default();
    destination.province = 1;
    destination.position = egui::pos2(300., 250.);
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("map-province-order-click"), destination));
    assert!(!prepare_orders(&ctx, &world, &economy, 0, None));
    world.battles.clear();
    open_army_panel(&ctx, 0, 0, Some(99));
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("map-province-order-click"), destination));
    assert!(!prepare_orders(&ctx, &world, &economy, 0, None));
}

#[test]
fn orders_controls_and_footer_fit_compact_screens_with_every_unit_type() {
    let (mut world, mut economy, graph) = fixture();
    economy.provinces[0].name = "Gallia Cisalpina".into();
    economy.provinces[2].name = "Africa Proconsularis".into();
    for kind in UnitType::ALL {
        world.seed_unit(0, ForceOwner::Player(0), kind).unwrap();
    }
    for size in [egui::vec2(832., 740.), egui::vec2(1366., 768.), egui::vec2(1920., 1080.)] {
        let ctx = egui::Context::default();
        ctx.add_font(egui::epaint::text::FontInsert::new(
            "firasans",
            egui::FontData::from_static(include_bytes!("../../assets/fonts/FiraSans-Bold.ttf")),
            vec![egui::epaint::text::InsertFontFamily {
                family: egui::FontFamily::Proportional,
                priority: egui::epaint::text::FontPriority::Highest,
            }],
        ));
        open_army_panel(&ctx, 0, 0, None);
        let army = selected_army(&ctx).unwrap();
        ctx.data_mut(|data| {
            data.insert_temp(
                egui::Id::new(MENU_ID),
                ProvinceOrders {
                    army,
                    destination: 2,
                    position: egui::pos2(size.x - 5., size.y - 5.),
                    units: world.provinces[0].forces[&ForceOwner::Player(0)]
                        .iter()
                        .map(|u| u.id)
                        .collect(),
                },
            )
        });
        let mut capture = capture::Capture::default();
        let mut output = egui::FullOutput::default();
        for frame in 0..3 {
            output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                    time: Some(frame as f64 * 0.1),
                    ..Default::default()
                },
                |_| {
                    draw_orders_menu(
                        &ctx,
                        &world,
                        &economy,
                        &graph,
                        0,
                        |_, _| MilitaryAccess::Peaceful,
                        |_| true,
                        "",
                    );
                },
            );
            capture.frame(&ctx, &output, &format!("orders-{}", size.x as u32));
            output.textures_delta.clear();
        }
        let bounds = ctx.memory(|memory| memory.area_rect(egui::Id::new(MENU_ID))).unwrap();
        assert!(
            egui::Rect::from_min_size(egui::Pos2::ZERO, size).contains_rect(bounds),
            "Orders overflow {size:?}: {bounds:?}"
        );
        for text in
            ["Send cohorts", "All", "Half", "None", "Marching", "Staying", "Attack", "Pressure"]
        {
            assert!(
                bounds.contains(label(&output, text)),
                "{text} must remain visible at {size:?}"
            );
        }
        let footer = label(&output, "Marching");
        for shape in &output.shapes {
            if let egui::Shape::Text(text) = &shape.shape {
                if text.galley.job.text == "Attack" || text.galley.job.text == "Pressure" {
                    assert!(text.pos.y > footer.y, "Orders stay below the scrollable cohort list");
                }
            }
        }
    }
}

#[test]
fn five_type_army_keeps_the_individual_cohort_control_visible() {
    let (mut world, mut economy, graph) = fixture();
    world.provinces[0].forces.clear();
    for kind in [
        UnitType::LightInfantry,
        UnitType::HeavyInfantry,
        UnitType::HeavyInfantry,
        UnitType::HeavyInfantry,
        UnitType::Archers,
        UnitType::LightCavalry,
        UnitType::HeavyCavalry,
    ] {
        world.seed_unit(0, ForceOwner::Player(0), kind).unwrap();
    }
    economy.provinces[0].name = "Italia".into();
    economy.provinces[2].name = "Aquitania".into();
    let ctx = egui::Context::default();
    ctx.add_font(egui::epaint::text::FontInsert::new(
        "firasans",
        egui::FontData::from_static(include_bytes!("../../assets/fonts/FiraSans-Bold.ttf")),
        vec![egui::epaint::text::InsertFontFamily {
            family: egui::FontFamily::Proportional,
            priority: egui::epaint::text::FontPriority::Highest,
        }],
    ));
    open_army_panel(&ctx, 0, 0, None);
    let army = selected_army(&ctx).unwrap();
    ctx.data_mut(|data| {
        data.insert_temp(
            egui::Id::new(MENU_ID),
            ProvinceOrders {
                army,
                destination: 2,
                position: egui::pos2(120., 70.),
                units: world.provinces[0].forces[&ForceOwner::Player(0)]
                    .iter()
                    .map(|u| u.id)
                    .collect(),
            },
        )
    });
    let mut capture = capture::Capture::default();
    for frame in 0..3 {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000., 900.),
                )),
                time: Some(frame as f64 * 0.1),
                ..Default::default()
            },
            |_| {
                draw_orders_menu(
                    &ctx,
                    &world,
                    &economy,
                    &graph,
                    0,
                    |_, _| MilitaryAccess::Invasion,
                    |_| true,
                    "",
                );
            },
        );
        capture.frame(&ctx, &output, "orders-aquitania");
        output.textures_delta.clear();
        if frame == 2 {
            label(&output, "7 cohorts");
            label(&output, "7,000 soldiers");
            let header = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text)
                        if text.galley.job.text == "Choose individual cohorts" =>
                    {
                        Some((shape.clip_rect, text))
                    },
                    _ => None,
                })
                .unwrap();
            assert!(
                header
                    .0
                    .contains_rect(egui::Rect::from_min_size(header.1.pos, header.1.galley.size())),
                "Five troop types must not clip the individual cohort control"
            );
        }
    }
}
