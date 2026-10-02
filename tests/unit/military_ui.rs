use super::*;

use crate::egui_capture as capture;

#[test]
fn spectator_army_panels_reveal_foreign_and_local_forces_without_actions() {
    for owner in [ForceOwner::Player(1), ForceOwner::Local(0)] {
        let (mut world, economy, graph) = military_fixture();
        world.seed_unit(0, owner, UnitType::Archers).unwrap();
        let units = world.provinces[0].forces.get_mut(&owner).unwrap();
        units[0].training = 93.0;
        let mut damaged = units[0].clone();
        damaged.id += 1000;
        damaged.current_manpower = 4.0;
        units[0].current_manpower = 4.0;
        units.push(damaged);
        let ctx = egui::Context::default();
        super::super::spectator::set_read_only(&ctx, true);
        toggle_inspection_army_panel(&ctx, 0, 1, owner, None);
        let mut capture = capture::Capture::default();
        let mut render = |time, events| {
            let mut action = None;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1600.0, 1000.0),
                    )),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ui| {
                    action = draw_army_panel(
                        ui.ctx(),
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
            if time <= 0.1 {
                capture.frame(&ctx, &output, &format!("spectator-army-{owner:?}"));
            }
            output.textures_delta.clear();
            assert!(action.is_none(), "Spectators cannot issue army commands");
            output
        };
        render(0.0, vec![]);
        let output = render(0.1, vec![]);
        text_position(&output, "Italia");
        text_position(&output, "Deployment");
        assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Text(text) if text.galley.job.text == "Cohort capabilities")));
        for (index, label) in ["Merge", "Disband"].into_iter().enumerate() {
            let position = text_position(&output, label);
            render(0.2 + index as f64, click_events(position, true));
            render(0.3 + index as f64, click_events(position, false));
        }
        assert_eq!(selected_army(&ctx).unwrap().owner, owner);
        // Disabled deployment/tactic controls still expose information on hover.
        let position = text_position(&output, "2 cohorts");
        render(3.0, vec![egui::Event::PointerMoved(position)]);
        render(4.0, vec![]);
        let output = render(4.1, vec![]);
        text_position(&output, "Cohort capabilities");
        text_position(&output, "Training");
        text_position(&output, "93%");
    }
}

#[test]
fn spectator_inspects_the_selected_foreign_marching_army() {
    let (mut world, economy, graph) = military_fixture();
    let owner = ForceOwner::Player(1);
    world.seed_unit(0, owner, UnitType::Archers).unwrap();
    let mut units = world.provinces[0].forces.remove(&owner).unwrap();
    units[0].training = 94.0;
    world.movements.push(MovementOrder {
        id: 42,
        owner,
        units,
        origin: 0,
        route: vec![0],
        progress: 0.5,
        required_progress: 2.0,
        plan: BattlePlan::default(),
        withdrawing: false,
    });
    let ctx = egui::Context::default();
    super::super::spectator::set_read_only(&ctx, true);
    toggle_inspection_army_panel(&ctx, 0, 1, owner, Some(42));
    let mut pointer = None;
    for frame in 0..5 {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1600.0, 1000.0),
                )),
                time: Some(frame as f64),
                events: if frame == 2 {
                    pointer
                        .map(|position| vec![egui::Event::PointerMoved(position)])
                        .unwrap_or_default()
                } else {
                    vec![]
                },
                ..Default::default()
            },
            |ui| {
                assert!(draw_army_panel(
                    ui.ctx(),
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
        if frame == 1 {
            text_position(&output, "Marching to Italia");
            text_position(&output, "Deployment");
            pointer = Some(text_position(&output, "1 cohort"));
        }
        if frame == 4 {
            text_position(&output, "94%");
        }
    }
}

#[test]
fn spectator_recruitment_is_inspectable_but_cannot_recruit() {
    let (world, economy, graph) = military_fixture();
    let ctx = egui::Context::default();
    super::super::spectator::set_read_only(&ctx, true);
    render_military(&ctx, &world, &economy, &graph, 0.0, vec![]);
    let (output, _) = render_military(&ctx, &world, &economy, &graph, 0.1, vec![]);
    let position = text_position(&output, "Recruit units");
    render_military(&ctx, &world, &economy, &graph, 0.2, click_events(position, true));
    let (output, action) =
        render_military(&ctx, &world, &economy, &graph, 0.3, click_events(position, false));
    assert!(action.is_none());
    let position = text_position(&output, "Light Infantry");
    render_military(&ctx, &world, &economy, &graph, 0.4, click_events(position, true));
    let (_, action) =
        render_military(&ctx, &world, &economy, &graph, 0.5, click_events(position, false));
    assert!(action.is_none());
}

#[test]
fn army_badges_show_unsigned_zero_strength_and_morale() {
    let (mut world, economy, graph) = military_fixture();
    let unit = &mut world.provinces[0].forces.get_mut(&ForceOwner::Player(0)).unwrap()[0];
    unit.current_manpower = -0.0;
    unit.morale = -0.0;
    let ctx = egui::Context::default();
    let (output, _) = render_army(&ctx, &world, &economy, &graph, 0.0, vec![]);
    text_position(&output, "0");
    text_position(&output, "0%");
    let negatives: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text)
                if text.galley.job.text == "-0" || text.galley.job.text == "-0%" =>
            {
                Some((text.galley.job.text.clone(), text.pos))
            },
            _ => None,
        })
        .collect();
    assert!(negatives.is_empty(), "{negatives:?}");

    let unit = &mut world.provinces[0].forces.get_mut(&ForceOwner::Player(0)).unwrap()[0];
    unit.current_manpower = 1.0;
    unit.morale = -1.0;
    assert_eq!(format!("{:.0}%", army_morale(std::slice::from_ref(unit))), "0%");
}

#[test]
fn cohort_strength_uses_whole_people_instead_of_fractional_population() {
    let mut world = MilitaryWorld::new(1);
    world.seed_unit(0, ForceOwner::Player(0), UnitType::HeavyInfantry).unwrap();
    world.seed_unit(0, ForceOwner::Player(0), UnitType::WarElephants).unwrap();
    let units = &mut world.provinces[0].forces.get_mut(&ForceOwner::Player(0)).unwrap();
    assert_eq!(cohort_manpower_text(&units[0]), "1000/1000");
    assert_eq!(cohort_manpower_text(&units[1]), "1000/1000");
    units[1].current_manpower = 1.89;
    assert_eq!(cohort_manpower_text(&units[1]), "189/1000");
}

#[test]
fn army_unit_hover_keeps_description_and_current_condition_beside_the_image() {
    let mut world = MilitaryWorld::new(1);
    world.seed_unit(0, ForceOwner::Player(0), UnitType::LightInfantry).unwrap();
    world.seed_unit(0, ForceOwner::Player(0), UnitType::LightInfantry).unwrap();
    world.seed_unit(0, ForceOwner::Player(0), UnitType::Archers).unwrap();
    let units = world.provinces[0].forces.get_mut(&ForceOwner::Player(0)).unwrap();
    units[0].current_manpower = 8.85;
    let infantry: Vec<_> =
        units.iter().filter(|unit| unit.unit_type == UnitType::LightInfantry).collect();
    let archers: Vec<_> = units.iter().filter(|unit| unit.unit_type == UnitType::Archers).collect();
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600., 1000.))),
            ..Default::default()
        },
        |ui| {
            army_unit_type_hover(ui, UnitType::LightInfantry, &infantry, &world.config, 1.);
            army_unit_type_hover(ui, UnitType::Archers, &archers, &world.config, 1.);
        },
    );
    output.textures_delta.clear();
    for label in [
        "Statistics",
        "Offense",
        "Defense",
        "Speed",
        "Maneuver",
        "Training",
        "Morale",
        "Cohort capabilities",
        "Tactic capabilities",
    ] {
        text_position(&output, label);
    }
    let description = text_position(&output, unit_description(UnitType::LightInfantry));
    let title = text_position(&output, "Light Infantry");
    let stats = text_position(&output, "Statistics");
    let capabilities = text_position(&output, "Cohort capabilities");
    let tactics = text_position(&output, "Tactic capabilities");
    assert!(title.y < description.y && description.y < stats.y && stats.y < capabilities.y);
    assert!(capabilities.y < tactics.y);
    assert!(
        stats.x > 120. && capabilities.x > 120. && tactics.x > 120.,
        "tables should stay beside the art"
    );
    let label_left = |label| {
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == label => Some(text.pos.x),
                _ => None,
            })
            .unwrap_or_else(|| panic!("Missing military control: {label}"))
    };
    let label_x = label_left("Offense");
    for label in
        ["Defense", "Speed", "Maneuver", "Training", "Morale", "Heavy Infantry", "War Elephants"]
    {
        assert!(
            (label_left(label) - label_x).abs() < 1.,
            "{label} should start at the left edge of its label column"
        );
    }
    for removed in [
        "Soldiers",
        "Cohorts",
        "New cohort",
        "Food",
        "Wages",
        "Individual cohorts: 885 / 1,000 · 1,000 / 1,000",
    ] {
        assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Text(text) if text.galley.job.text == removed)));
    }
}

#[test]
fn unit_hover_tactic_table_shows_one_configured_percentage_per_tactic() {
    let mut config = MilitaryConfig::default();
    config.tactic_fit[UnitType::LightInfantry as usize][CombatTactic::ShockAction as usize] = 0.37;
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600., 1000.))),
            ..Default::default()
        },
        |ui| recruitment_hover(ui, UnitType::LightInfantry, &config, 1., None, 1.),
    );
    output.textures_delta.clear();
    let section = text_position(&output, "Tactic capabilities");
    let shock = text_position(&output, "Shock Action");
    let fit = text_position(&output, "37%");
    assert!(shock.y > section.y);
    assert!((shock.y - fit.y).abs() < 2.);
    for tactic in CombatTactic::ALL {
        assert!(text_position(&output, tactic.name()).y > section.y);
    }
    for removed in ["Fit", "Bonus", "Manpower", "Casualties", "×1.10"] {
        assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Text(text) if text.galley.job.text == removed)));
    }
}

#[test]
fn army_overview_covers_players_and_uses_shared_badges_and_section_headers() {
    let (mut world, economy, graph) = military_fixture();
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
        let scale = super::super::viewport_ui_scale(size);
        let players = egui::Rect::from_min_max(
            egui::pos2(size.x - 180. * scale, size.y - 210. * scale),
            egui::pos2(size.x, size.y) - egui::vec2(8., 8.) * scale,
        );
        let color = egui::Color32::from_rgb(48, 102, 174);
        ctx.data_mut(|data| {
            data.insert_temp(egui::Id::new("practice-players-panel-rect"), players);
            data.insert_temp(egui::Id::new("campaign-active-player-color"), color);
        });
        open_army_panel(&ctx, 0, 0, None);
        let mut output = egui::FullOutput::default();
        for frame in 0..3 {
            output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                    time: Some(frame as f64 * 0.1),
                    ..Default::default()
                },
                |ui| {
                    show(
                        ui,
                        &world,
                        &economy,
                        &graph,
                        0,
                        0,
                        |_| true,
                        |_| None,
                        |_, _| MilitaryAccess::Peaceful,
                        |a, b| a != b,
                    );
                    draw_army_panel(
                        ui.ctx(),
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
        }
        let panel = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Rect(rect)
                    if rect.fill == super::super::province_panel::PAPER
                        && rect.rect.height() > 100. =>
                {
                    Some(rect.rect)
                },
                _ => None,
            })
            .expect("Parchment army panel");
        let title = egui::pos2(panel.center().x, panel.top() + 21. * scale);
        assert!(
            panel.top() >= 0. && panel.contains_rect(players),
            "Panel must fit on screen and cover players at {size:?}: {panel:?}"
        );
        assert!((panel.right() - (size.x - 8. * scale)).abs() < 3.);
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(rect) if rect.fill == color && rect.rect.contains(title))));
        let badges =
            ["Army strength", "Cohorts", "Terrain", "Food demand", "Army morale"].map(|label| {
                output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(text)
                            if text.galley.job.text == label && panel.contains(text.pos) =>
                        {
                            Some(text.pos + text.galley.size() * 0.5)
                        },
                        _ => None,
                    })
                    .expect("Overview badge")
            });
        for pair in badges.windows(2) {
            assert!(pair[0].x < pair[1].x && (pair[0].y - pair[1].y).abs() < 2.);
        }
        let cohort_count = world.provinces[0].forces[&ForceOwner::Player(0)].len().to_string();
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Text(text)
                if text.galley.job.text == cohort_count
                    && (text.visual_bounding_rect().center().x - badges[1].x).abs() < 10. * scale
                    && (0. ..30. * scale).contains(&(text.visual_bounding_rect().center().y - badges[1].y))
        )));
        let deployment = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == "Deployment" => {
                    Some(text.visual_bounding_rect().center())
                },
                _ => None,
            })
            .expect("Deployment heading");
        assert!(
            deployment.x < panel.center().x,
            "Deployment heading uses the shared left-aligned dark section at {size:?}"
        );
        assert!(
            output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(rect)
            if rect.fill == egui::Color32::from_rgb(73, 69, 61) && rect.rect.contains(deployment)))
        );
        let controls: Vec<_> = ["Tactics", "Frontline", "Rear line", "Flanks", "Flank size"]
            .into_iter()
            .map(|label| text_position(&output, label))
            .collect();
        assert!(
            controls
                .windows(2)
                .all(|pair| { pair[0].x < pair[1].x && (pair[0].y - pair[1].y).abs() < 2. }),
            "Five deployment controls must stay in one row at {size:?}"
        );
        assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Text(text) if text.galley.job.text == "Tactic" && panel.contains(text.pos))));
        assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text)
            if panel.contains(text.pos) && (text.galley.job.text == "Unit types"
                || text.galley.job.text.contains("combat width")
                || text.galley.job.text.starts_with("Drag a unit")))));
        for forbidden in ["Your army", "Your Army", "ARMY ORDERS", "Balanced · neutral", "Manpower"]
        {
            assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == forbidden && panel.contains(text.pos))));
        }
    }
}

#[test]
fn army_banner_shows_disband_and_merge_without_orders() {
    let (mut world, economy, graph) = military_fixture();
    for kind in UnitType::ALL {
        world.seed_unit(0, ForceOwner::Player(0), kind).unwrap();
    }
    let ctx = egui::Context::default();
    let (output, _) = render_army(&ctx, &world, &economy, &graph, 0., vec![]);
    for label in ["Deployment", "Disband", "Merge"] {
        text_position(&output, label);
    }
    assert!(text_position(&output, "Merge").x < text_position(&output, "Disband").x);
    assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
        egui::Shape::Text(text) if matches!(text.galley.job.text.as_str(), "Orders" | "Army orders"))));
}

#[test]
fn army_actions_use_banner_buttons_and_all_unit_types_stay_in_one_row() {
    let (mut world, economy, graph) = military_fixture();
    for kind in UnitType::ALL {
        world.seed_unit(0, ForceOwner::Player(0), kind).unwrap();
    }
    world
        .set_plan(
            0,
            ForceOwner::Player(0),
            BattlePlan {
                flank_size: 5,
                ..Default::default()
            },
        )
        .unwrap();
    let ctx = egui::Context::default();
    let (output, _) = render_army(&ctx, &world, &economy, &graph, 0., vec![]);
    for label in ["Left flank · 5", "Frontline · 6", "Right flank · 5"] {
        text_position(&output, label);
    }
    let cards: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect)
                if (rect.rect.height() - 86.).abs() < 0.1
                    && rect.fill == super::super::province_panel::TABLE_STRIPE =>
            {
                Some(rect.rect)
            },
            _ => None,
        })
        .collect();
    // Count the lower unit-type row.
    let unit_row_top = cards.iter().map(|rect| rect.top()).fold(f32::NEG_INFINITY, f32::max);
    let cards: Vec<_> =
        cards.into_iter().filter(|rect| (rect.top() - unit_row_top).abs() < 0.1).collect();
    assert_eq!(cards.len(), UnitType::ALL.len());
    assert!(cards.windows(2).all(
        |pair| pair[0].right() < pair[1].left() && (pair[0].top() - pair[1].top()).abs() < 0.1
    ));
    let mut rows: Vec<_> = deployment_cells(&output).iter().map(|rect| rect.top() as i32).collect();
    rows.sort_unstable();
    rows.dedup();
    assert_eq!(rows.len(), 2, "Support must remain in the two-row deployment preview");
    let first_row =
        deployment_cells(&output).into_iter().filter(|rect| rect.top() as i32 == rows[0]).count();
    assert_eq!(first_row, 16);
    let disband = text_position(&output, "Disband");
    render_army(&ctx, &world, &economy, &graph, 0.1, click_events(disband, true));
    let (_, action) =
        render_army(&ctx, &world, &economy, &graph, 0.2, click_events(disband, false));
    assert!(matches!(action, Some(MilitaryUiAction::DisbandArmy)));
    let units = world.provinces[0].forces.get_mut(&ForceOwner::Player(0)).unwrap();
    let heavy = units.iter_mut().find(|unit| unit.unit_type == UnitType::HeavyInfantry).unwrap();
    heavy.current_manpower = 5.;
    let second = units.iter_mut().find(|unit| unit.unit_type == UnitType::LightInfantry).unwrap();
    second.unit_type = UnitType::HeavyInfantry;
    second.current_manpower = 5.;
    let (output, _) = render_army(&ctx, &world, &economy, &graph, 0.3, vec![]);
    let merge = text_position(&output, "Merge");
    render_army(&ctx, &world, &economy, &graph, 0.4, click_events(merge, true));
    let (_, action) = render_army(&ctx, &world, &economy, &graph, 0.5, click_events(merge, false));
    assert!(matches!(action, Some(MilitaryUiAction::MergeArmy)));
}
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
    assert_eq!(rows.len(), 7, "All stationed owners plus a stationary and two marching armies");
    assert_eq!(rows.iter().filter(|row| row.owner == own).count(), 4);
    let army = &rows[0];
    assert_eq!(army.owner, own);
    assert_eq!(army.province, 0);
    assert_eq!(army.tactic, Some(CombatTactic::Envelopment));
    assert_eq!(army.averages(), (80., 70.));
    assert!(rows.iter().filter(|row| row.owner != own).all(|row| row.tactic.is_none()));
    assert!(rows.iter().any(|row| row.province == 2 && row.owner == guest));
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
    let mut promotion = None;
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 650.))),
            time: Some(time),
            events,
            ..Default::default()
        },
        |ui| {
            *ui.style_mut() = super::super::campaign_widgets::map_style(scale);
            selected = overview(
                ui,
                world,
                economy,
                0,
                economy.players[0].influence,
                &[egui::Color32::RED, egui::Color32::BLUE],
                scale,
                &mut promotion,
            );
        },
    );
    if time == 0. {
        capture::Capture::default().frame(
            ctx,
            &output,
            &format!("army-overview-{width}-{scale}-{}-forces", army_overview_rows(world, 0).len()),
        );
    }
    output.textures_delta.clear();
    (output, selected)
}

#[test]
fn rank_hover_lists_requirements_and_readable_monthly_bonuses() {
    let ctx = egui::Context::default();
    let mut world = MilitaryWorld::new(1);
    world.peak_manpower.insert(ForceOwner::Player(0), -0.0);
    let draw = |time, events| {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800., 600.),
                )),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ui| rank_ladder(ui, &world, 0, 180., 1., &mut None),
        );
        output.textures_delta.clear();
        output
    };
    let initial = draw(0., vec![]);
    let position = text_position(&initial, "Tribune");
    let mut found = false;
    for frame in 1..=4 {
        let output = draw(
            frame as f64,
            if frame == 1 {
                vec![egui::Event::PointerMoved(position)]
            } else {
                vec![]
            },
        );
        let labels: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect();
        if !labels.contains(&"Requirements") {
            continue;
        }
        found = true;
        for expected in [
            "Bonuses",
            "• Previous rank: Centurion",
            "• Army manpower: 0/200",
            "• Battles won: 0/2",
            "• Influence cost: 180/500",
            "• Influence: +5 per month",
        ] {
            assert!(labels.contains(&expected), "Missing {expected}: {labels:?}");
        }
        assert_eq!(labels.iter().filter(|label| **label == "Tribune").count(), 1);
        for removed in [
            "-0",
            " / ",
            "Peak combined",
            "garrison",
            "confidence improves",
            "recovery",
            "Combat morale",
            "Senate military faction",
            "One promotion",
            "Support requirements",
        ] {
            assert!(!labels.iter().any(|label| label.contains(removed)));
        }
        let last_requirement = text_position(&output, "• Influence cost: 180/500");
        let bonuses = text_position(&output, "Bonuses");
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::LineSegment { points, .. } if points[0].y > last_requirement.y
                && points[0].y < bonuses.y && (points[1].x - points[0].x).abs() > 50.)));
        for shape in &output.shapes {
            if let egui::Shape::Text(text) = &shape.shape {
                if text.galley.job.text.starts_with('•')
                    || ["Requirements", "Bonuses"].contains(&text.galley.job.text.as_str())
                {
                    assert!(text.galley.job.sections.iter().all(|section| section
                        .format
                        .font_id
                        .size
                        == 14.));
                }
            }
        }
    }
    assert!(found, "The rank card must show its tooltip on hover");
}

#[test]
fn rank_cards_only_accept_the_next_fully_earned_promotion() {
    let ctx = egui::Context::default();
    let mut world = MilitaryWorld::new(1);
    let owner = ForceOwner::Player(0);
    let draw = |time, events, world: &MilitaryWorld| {
        let mut promotion = None;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(600., 180.),
                )),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ui| rank_ladder(ui, world, 0, 600., 1., &mut promotion),
        );
        output.textures_delta.clear();
        (output, promotion)
    };
    let (output, _) = draw(0., vec![], &world);
    let tribune = text_position(&output, "Tribune");
    draw(0.1, click_events(tribune, true), &world);
    assert_eq!(draw(0.2, click_events(tribune, false), &world).1, None);

    world.peak_manpower.insert(owner, 600.);
    world.victories.insert(owner, 6);
    // Register the newly clickable card before egui hit-tests the next input frame.
    draw(0.25, vec![], &world);
    draw(0.3, click_events(tribune, true), &world);
    assert_eq!(
        draw(0.4, click_events(tribune, false), &world).1,
        Some(MilitaryRank::MilitaryTribune)
    );
    let (output, _) = draw(0.5, vec![], &world);
    let imperator = text_position(&output, "Imperator");
    draw(0.6, click_events(imperator, true), &world);
    assert_eq!(draw(0.7, click_events(imperator, false), &world).1, None);
}
#[test]
fn overview_combines_armies_with_directory_styling_and_foreign_row_opens_the_province() {
    let (mut world, mut economy, _) = military_fixture();
    economy.provinces[0].name = "Africa Proconsularis et Numidia".into();
    for kind in UnitType::ALL {
        world.seed_unit(0, ForceOwner::Player(0), kind).unwrap();
    }
    world.seed_unit(0, ForceOwner::Player(1), UnitType::Archers).unwrap();
    for (width, scale) in [(796., 1.), (380., 1.), (380., 0.85)] {
        let ctx = egui::Context::default();
        let (output, _) = render_overview(&ctx, &world, &economy, width, scale, 0., vec![]);
        for label in ["MILITARY RANK", "Province", "Units", "Tactic", "Morale", "Training", "?"] {
            text_position(&output, label);
        }
        for forbidden in [
            "ARMIES",
            "FRIENDLY ARMIES",
            "Player 2",
            "Recruit units",
            "Available cohorts",
            "ARMY ORDERS",
            "+",
            "Stationed",
            "You",
            "Click an army to open its province.",
            "1 armies · 9 cohorts",
        ] {
            assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == forbidden)));
        }
        for label in ["Province", "Units", "Tactic", "Morale", "Training"] {
            assert_eq!(
                output
                    .shapes
                    .iter()
                    .filter(|shape| matches!(&shape.shape,
                    egui::Shape::Text(text) if text.galley.job.text == label))
                    .count(),
                1,
                "Owned and friendly armies must share one table header"
            );
        }
        assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Text(text) if CombatTactic::ALL.iter().any(|tactic| tactic.name() == text.galley.job.text))));
        let markers: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Rect(rect)
                    if [egui::Color32::RED, egui::Color32::BLUE].contains(&rect.fill) =>
                {
                    Some(rect)
                },
                _ => None,
            })
            .collect();
        assert_eq!(markers.len(), 2);
        assert_eq!(markers[0].fill, egui::Color32::RED);
        assert_eq!(markers[1].fill, egui::Color32::BLUE);
        let names: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == economy.provinces[0].name => {
                    Some(text)
                },
                _ => None,
            })
            .collect();
        assert_eq!(names.len(), 2, "Both armies must display the same province name");
        for (marker, name) in markers.iter().zip(&names) {
            let row = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Rect(rect)
                        if [
                            super::super::province_panel::PAPER,
                            super::super::province_panel::TABLE_STRIPE,
                        ]
                        .contains(&rect.fill)
                            && rect.rect.contains(marker.rect.center())
                            && (rect.rect.left() + 7. * scale - marker.rect.left()).abs() < 0.1 =>
                    {
                        Some(rect.rect)
                    },
                    _ => None,
                })
                .expect("Each owner marker must belong to an army row");
            assert!((marker.rect.width() - 6. * scale).abs() < 0.1);
            assert!((marker.rect.height() / row.height() - 0.68).abs() < 0.001);
            assert!((marker.rect.center().y - row.center().y).abs() < 0.1);
            assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
                egui::Shape::Rect(border) if border.rect == row
                    && border.stroke.color == super::super::province_panel::RULE
                    && (border.stroke.width - scale).abs() < 0.1
                    && border.corner_radius == egui::CornerRadius::from(3. * scale))));
            assert!((name.pos.x - marker.rect.right() - 8. * scale).abs() < 0.1);
            assert!(name
                .galley
                .job
                .sections
                .iter()
                .all(|section| section.format.font_id.size == 14. * scale));
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
        if width > 500. {
            let row = markers[1].rect.center();
            render_overview(&ctx, &world, &economy, width, scale, 0.1, click_events(row, true));
            let (_, selected) = render_overview(
                &ctx,
                &world,
                &economy,
                width,
                scale,
                0.2,
                click_events(row, false),
            );
            assert_eq!(selected, Some(0), "A visiting army opens its province");
            assert!(selected_army(&ctx).is_none());
        }
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
    assert!(selected_army(&ctx).is_none(), "Opening the military tab does not open an army");
    let (output, _) = render_military(&ctx, &world, &economy, &graph, 0., vec![]);
    assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == "Available cohorts")));
    world.provinces[0].forces.clear();
    let (output, selected) = render_overview(&ctx, &world, &economy, 500., 1., 0.1, vec![]);
    text_position(&output, "You have no armies.");
    assert!(selected.is_none());
}
#[test]
fn overview_shows_visiting_armies_without_an_owned_army_in_the_province() {
    let (mut world, economy, _) = military_fixture();
    world.provinces[0].forces.clear();
    world.seed_unit(0, ForceOwner::Player(1), UnitType::Archers).unwrap();
    let ctx = egui::Context::default();
    let (output, _) = render_overview(&ctx, &world, &economy, 796., 1., 0., vec![]);
    for removed in ["You have no armies.", "FRIENDLY ARMIES"] {
        assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Text(text) if text.galley.job.text == removed)));
    }
    let row = text_position(&output, &economy.provinces[0].name);
    render_overview(&ctx, &world, &economy, 796., 1., 0.1, click_events(row, true));
    let (_, selected) =
        render_overview(&ctx, &world, &economy, 796., 1., 0.2, click_events(row, false));
    assert_eq!(selected, Some(0));
}
#[test]
fn overview_scrolls_to_all_armies_and_opens_the_selected_army() {
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
    text_position(&output, "Province");
    let last_army = text_position(&output, "Province 11");
    assert!(last_army.y < 650.);
    render_overview(&ctx, &world, &economy, 796., 1., 1.5, click_events(last_army, true));
    let (_, selected) =
        render_overview(&ctx, &world, &economy, 796., 1., 1.6, click_events(last_army, false));
    assert_eq!(selected, None);
    assert_eq!(selected_army_province(&ctx), Some(11));
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
            if let Some((_, army_action)) = draw_army_panel(
                ui.ctx(),
                world,
                economy,
                graph,
                0,
                |_| true,
                |_| None,
                |_, _| MilitaryAccess::Peaceful,
                |a, b| a != b,
            ) {
                action = Some(army_action);
            }
        },
    );
    // These headless interaction checks have no GPU to apply texture updates.
    output.textures_delta.clear();
    (output, action)
}
fn render_army(
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
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600., 1000.),
            )),
            time: Some(time),
            events,
            ..Default::default()
        },
        |ui| {
            *ui.style_mut() = super::super::campaign_widgets::map_style(1.);
            ui.set_width(600.);
            action = stationary_army_panel(
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
fn stationed_rows_hide_details_and_only_owned_armies_open_a_panel() {
    let (mut world, economy, graph) = military_fixture();
    let enemy = ForceOwner::Player(1);
    world.seed_unit(0, enemy, UnitType::LightCavalry).unwrap();
    world.provinces[0].forces.get_mut(&enemy).unwrap()[0].morale = 81.;
    let ctx = egui::Context::default();
    let colors = vec![egui::Color32::from_rgb(32, 88, 142), egui::Color32::from_rgb(145, 49, 30)];
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("campaign-player-colors"), colors.clone()));
    render_military(&ctx, &world, &economy, &graph, 0., vec![]);
    let (output, _) = render_military(&ctx, &world, &economy, &graph, 0., vec![]);
    let military_icon = super::super::campaign_widgets::texture(&ctx, Icon::Attack);
    for label in ["Army strength", "Cohorts", "Army morale", "Food demand", "81%"] {
        text_position(&output, label);
    }
    let strength_badge = text_position(&output, "Army strength");
    let cohort_badge = text_position(&output, "Cohorts");
    let food_badge = text_position(&output, "Food demand");
    let morale_badge = text_position(&output, "Army morale");
    assert!((strength_badge.y - cohort_badge.y).abs() < 1.);
    assert!((cohort_badge.y - food_badge.y).abs() < 1.);
    assert!((food_badge.y - morale_badge.y).abs() < 1.);
    assert!(
        strength_badge.x < cohort_badge.x
            && cohort_badge.x < food_badge.x
            && food_badge.x < morale_badge.x
    );
    for (owner, title) in [(ForceOwner::Player(0), "Player 1"), (enemy, "Player 2")] {
        let title_pos = text_position(&output, title);
        let card = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Rect(rect)
                    if rect.fill == super::super::province_panel::TABLE_STRIPE
                        && rect.rect.contains(title_pos) =>
                {
                    Some(rect.rect)
                },
                _ => None,
            })
            .expect("Army card");
        let mut badges: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Rect(rect)
                    if rect.fill == egui::Color32::from_rgb(235, 226, 208)
                        && card.contains_rect(rect.rect) =>
                {
                    Some(rect.rect)
                },
                _ => None,
            })
            .collect();
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Mesh(mesh) if mesh.texture_id == military_icon
                && card.contains_rect(mesh.calc_bounds()))));
        badges.sort_by(|a, b| a.top().total_cmp(&b.top()));
        assert_eq!(badges.len(), 3);
        let units = &world.provinces[0].forces[&owner];
        let strength: f64 = units.iter().map(|unit| unit.effective_strength(&world.config)).sum();
        let label = format!("{strength:.0}");
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Text(text) if text.galley.job.text == label
                && badges[0].contains(text.visual_bounding_rect().center()))));
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Mesh(mesh) if badges[0].contains(mesh.calc_bounds().center()))));
    }
    for value in ["−5/mo", "−2/mo", "−3/mo", "100%"] {
        text_position(&output, value);
    }
    for shape in &output.shapes {
        if let egui::Shape::Text(text) = &shape.shape {
            if text.galley.job.text.starts_with('−') && text.galley.job.text.ends_with("/mo") {
                assert_eq!(
                    text.galley.job.sections[0].format.color,
                    super::super::resource_hud::hud_delta_color(-1.)
                );
            }
        }
    }
    for forbidden in [
        "Deployment",
        "Unit types",
        "ARMY ORDERS",
        "Frontline",
        "Manpower",
        "Average morale",
        "Stationed",
        "Your army",
    ] {
        assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == forbidden)));
    }
    let foreign = text_position(&output, "Player 2");
    for (index, name) in ["Player 1", "Player 2"].into_iter().enumerate() {
        let title = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == name => Some(text),
                _ => None,
            })
            .unwrap();
        assert_eq!(title.galley.job.sections[0].format.color, colors[index]);
    }
    render_military(&ctx, &world, &economy, &graph, 0.1, click_events(foreign, true));
    let (_, action) =
        render_military(&ctx, &world, &economy, &graph, 0.2, click_events(foreign, false));
    assert!(action.is_none());
    assert!(selected_army(&ctx).is_none());
    let (output, _) = render_military(&ctx, &world, &economy, &graph, 0.3, vec![]);
    let own = text_position(&output, "Player 1");
    render_military(&ctx, &world, &economy, &graph, 0.4, click_events(own, true));
    render_military(&ctx, &world, &economy, &graph, 0.5, click_events(own, false));
    let (output, _) = render_military(&ctx, &world, &economy, &graph, 0.6, vec![]);
    text_position(&output, "Deployment");
    assert!(matches!(
        selected_army(&ctx).map(|selected| selected.panel),
        Some(ArmyPanel::Stationary)
    ));
    render_military(&ctx, &world, &economy, &graph, 0.7, click_events(own, true));
    render_military(&ctx, &world, &economy, &graph, 0.8, click_events(own, false));
    assert!(selected_army(&ctx).is_none(), "A second click on the same army closes it");
}

#[test]
fn army_window_renders_without_the_province_military_panel() {
    let (world, economy, graph) = military_fixture();
    let ctx = egui::Context::default();
    open_army_panel(&ctx, 0, 0, None);
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600., 1000.),
            )),
            ..Default::default()
        },
        |ui| {
            draw_army_panel(
                ui.ctx(),
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
    output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600., 1000.),
            )),
            time: Some(0.1),
            ..Default::default()
        },
        |ui| {
            draw_army_panel(
                ui.ctx(),
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
    text_position(&output, "Italia");
    text_position(&output, "Deployment");
}

#[test]
fn army_window_helmet_requests_its_province_military_tab() {
    let (world, economy, graph) = military_fixture();
    let ctx = egui::Context::default();
    open_army_panel(&ctx, 0, 0, None);
    let render = |time, events| {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1600., 1000.),
                )),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ui| {
                draw_army_panel(
                    ui.ctx(),
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
        output
    };
    render(0., vec![]);
    let output = render(0.1, vec![]);
    let scale = super::super::viewport_ui_scale(egui::vec2(1600., 1000.));
    let helmet = egui::pos2(
        1600. - (7. + 640. - 23.) * scale,
        text_position(&output, "Italia").y + 8. * scale,
    );
    render(0.2, click_events(helmet, true));
    assert_eq!(take_army_province_click(&ctx), None);
    render(0.3, click_events(helmet, false));
    assert_eq!(take_army_province_click(&ctx), Some(0));
    assert_eq!(take_army_province_click(&ctx), None);
}

#[test]
fn army_title_search_lists_owned_armies_and_switches_provinces() {
    let (mut world, mut economy, mut graph) = military_fixture();
    for name in ["Achaia", "Gaul"] {
        let mut province = economy.provinces[0].clone();
        province.name = name.into();
        economy.provinces.push(province);
        world.provinces.push(ProvinceMilitaryState::default());
        graph.push(graph[0].clone());
    }
    world.seed_unit(1, ForceOwner::Player(0), UnitType::Archers).unwrap();
    let ctx = egui::Context::default();
    open_army_panel(&ctx, 0, 0, None);
    let render = |time, events| {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1600., 1000.),
                )),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ui| {
                draw_army_panel(
                    ui.ctx(),
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
        output
    };
    render(0., vec![]);
    let title = text_position(&render(0.1, vec![]), "Italia");
    render(0.2, click_events(title, true));
    let output = render(0.3, click_events(title, false));
    let achaia = text_position(&output, "Achaia");
    assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
        egui::Shape::Text(text) if text.galley.job.text == "Gaul")));
    render(0.4, click_events(achaia, true));
    render(0.5, click_events(achaia, false));
    assert_eq!(selected_army_province(&ctx), Some(1));
    text_position(&render(0.6, vec![]), "Achaia");
}
#[test]
fn stationed_army_tooltips_are_specific_and_disband_does_not_open_the_army() {
    let (mut world, _, _) = military_fixture();
    world.seed_unit(0, ForceOwner::Player(0), UnitType::LightInfantry).unwrap();
    let units = &world.provinces[0].forces[&ForceOwner::Player(0)];
    let strength: f64 = units.iter().map(|unit| unit.effective_strength(&world.config)).sum();
    let strength_label = format!("{strength:.0}");
    for (target, tooltip) in [
        ("Army", None),
        (strength_label.as_str(), Some("Army strength")),
        ("100%", Some("Army morale")),
        ("−4/mo", Some("Food demand")),
        ("Light Infantry", Some("Light Infantry")),
        ("Heavy Infantry", Some("Heavy Infantry")),
        (
            "Disband",
            Some("Disband this army. Soldiers will return to their respective population classes."),
        ),
    ] {
        let ctx = egui::Context::default();
        let render = |time, events| {
            let mut action = (false, false);
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(500., 400.),
                    )),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ui| {
                    *ui.style_mut() = super::super::campaign_widgets::map_style(1.);
                    action = stationed_army_row(
                        ui,
                        units,
                        &world.config,
                        "Army".into(),
                        super::super::PLAYER_COLORS[0],
                        true,
                        true,
                        1.,
                    );
                },
            );
            output.textures_delta.clear();
            (output, action)
        };
        render(0., vec![]);
        let (output, _) = render(0.1, vec![]);
        let pointer = if matches!(target, "Light Infantry" | "Heavy Infantry") {
            let card = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Rect(rect)
                        if rect.fill == super::super::province_panel::TABLE_STRIPE =>
                    {
                        Some(rect.rect)
                    },
                    _ => None,
                })
                .unwrap();
            card.left_bottom()
                + egui::vec2(
                    if target == "Light Infantry" {
                        21.
                    } else {
                        110.
                    },
                    -20.,
                )
        } else {
            text_position(&output, target)
        };
        render(0.2, vec![egui::Event::PointerMoved(pointer)]);
        render(1.2, vec![]);
        let (hover, _) = render(1.3, vec![]);
        if let Some(tooltip) = tooltip {
            let position = text_position(&hover, tooltip);
            if matches!(target, "Light Infantry" | "Heavy Infantry") {
                assert!(
                    (position.x - pointer.x).abs() < 120.,
                    "Unit tooltip must stay beside its unit"
                );
            }
            for other in
                ["Army strength", "Army morale", "Food demand", "Light Infantry", "Heavy Infantry"]
            {
                if other != tooltip {
                    assert!(
                        !hover.shapes.iter().any(|shape| matches!(&shape.shape,
                        egui::Shape::Text(text) if text.galley.job.text == other)),
                        "Hovering {target} must not show {other}"
                    );
                }
            }
        }
        assert!(!hover.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text)
            if text.galley.job.text.contains("surviving soldiers") || text.galley.job.text.contains("Open your army") || text.galley.job.text.contains("Metal"))));
        if target == "Army" {
            assert_eq!(
                hover.shapes.len(),
                output.shapes.len(),
                "The army row must not create a tooltip"
            );
        }
        if target == "Disband" {
            render(1.4, click_events(pointer, true));
            let (_, action) = render(1.5, click_events(pointer, false));
            assert_eq!(action, (false, true));
        }
    }
}
#[test]
fn stationed_army_badges_and_units_fit_without_overlap() {
    let (mut world, _, _) = military_fixture();
    for kind in UnitType::ALL {
        world.seed_unit(0, ForceOwner::Player(0), kind).unwrap();
    }
    let units = &world.provinces[0].forces[&ForceOwner::Player(0)];
    for (width, scale) in [(380., 1.), (548., 1.), (380., 0.7)] {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            *ui.style_mut() = super::super::campaign_widgets::map_style(scale);
            ui.set_width(width);
            stationed_army_row(
                ui,
                units,
                &world.config,
                "Player 1".into(),
                super::super::PLAYER_COLORS[0],
                true,
                true,
                scale,
            );
        });
        output.textures_delta.clear();
        let rectangles: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Rect(rect) => Some(rect),
                _ => None,
            })
            .collect();
        let card = rectangles
            .iter()
            .find(|rect| rect.fill == super::super::province_panel::TABLE_STRIPE)
            .unwrap()
            .rect;
        let badges: Vec<_> = rectangles
            .iter()
            .filter(|rect| rect.fill == egui::Color32::from_rgb(235, 226, 208))
            .map(|rect| rect.rect)
            .collect();
        assert!(card.height() >= 117. * scale);
        assert_eq!(badges.len(), 3);
        assert!(badges.iter().all(|badge| card.contains_rect(*badge)));
        assert!(badges[0].bottom() < badges[1].top());
        assert!(badges[0].left() > card.center().x);
        let button = rectangles
            .iter()
            .filter(|rect| rect.rect.contains(text_position(&output, "Disband")))
            .min_by(|a, b| a.rect.area().total_cmp(&b.rect.area()))
            .unwrap()
            .rect;
        assert!((button.top() - card.top() - (card.right() - button.right())).abs() < 0.1);
        assert!(button.width() < 96. * scale);
        let unit_images: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Mesh(mesh)
                    if (mesh.calc_bounds().width() - 44. * scale).abs() < 0.1 =>
                {
                    Some(mesh.calc_bounds())
                },
                _ => None,
            })
            .collect();
        assert_eq!(unit_images.len(), UnitType::ALL.len());
        for image in unit_images {
            assert!(card.contains_rect(image));
            let cell = image.expand2(egui::vec2(24. * scale, 0.));
            assert!(badges.iter().all(|badge| !cell.intersects(*badge)));
        }
    }
}

#[test]
fn stationed_army_cohorts_enlarge_and_wrap_to_fit() {
    let mut world = MilitaryWorld::new(1);
    for kind in UnitType::ALL.into_iter().take(4) {
        world.seed_unit(0, ForceOwner::Player(0), kind).unwrap();
    }
    let units = &world.provinces[0].forces[&ForceOwner::Player(0)];
    for (width, icon_size, count_size, expected_rows) in [(548., 54., 20., 1), (380., 44., 17., 2)]
    {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            *ui.style_mut() = super::super::campaign_widgets::map_style(1.);
            ui.set_width(width);
            stationed_army_row(
                ui,
                units,
                &world.config,
                "Player 1's army".into(),
                super::super::PLAYER_COLORS[0],
                true,
                true,
                1.,
            );
        });
        output.textures_delta.clear();
        let mut tops: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Mesh(mesh) if (mesh.calc_bounds().width() - icon_size).abs() < 0.1 => {
                    Some(mesh.calc_bounds().top())
                },
                _ => None,
            })
            .collect();
        assert_eq!(tops.len(), 4);
        tops.sort_by(f32::total_cmp);
        tops.dedup_by(|a, b| (*a - *b).abs() < 0.1);
        assert_eq!(tops.len(), expected_rows);
        let count_labels: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == "1" => {
                    Some(text.galley.job.sections[0].format.font_id.size)
                },
                _ => None,
            })
            .collect();
        assert_eq!(count_labels.len(), 4);
        assert!(count_labels.iter().all(|size| (*size - count_size).abs() < 0.1));
    }
}

#[test]
fn army_details_group_cohorts_and_show_condition_in_the_unit_hover() {
    let (mut world, economy, graph) = military_fixture();
    world.seed_unit(0, ForceOwner::Player(0), UnitType::HeavyInfantry).unwrap();
    world.provinces[0].forces.get_mut(&ForceOwner::Player(0)).unwrap()[0].current_manpower = 8.85;
    let ctx = egui::Context::default();
    let (output, _) = render_army(&ctx, &world, &economy, &graph, 0., vec![]);
    let card = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect) if (rect.rect.height() - 86.).abs() < 0.1 => Some(rect.rect),
            _ => None,
        })
        .max_by(|a, b| a.top().total_cmp(&b.top()))
        .expect("Unit type card below deployment");
    assert!(card.top() > deployment_cells(&output).last().unwrap().bottom());
    let manpower = text_position(&output, "1,885");
    let cohorts = text_position(&output, "2 cohorts");
    assert!(card.contains(manpower) && card.contains(cohorts));
    assert!(manpower.y < cohorts.y);
    assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == "Unit types")));
    render_army(
        &ctx,
        &world,
        &economy,
        &graph,
        0.1,
        vec![egui::Event::PointerMoved(card.center())],
    );
    render_army(&ctx, &world, &economy, &graph, 1., vec![]);
    let (hover, _) = render_army(&ctx, &world, &economy, &graph, 1.1, vec![]);
    let tooltip = hover
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(tooltip.contains(unit_description(UnitType::HeavyInfantry)), "{tooltip}");
    assert!(tooltip.contains("Statistics\nOffense"));
    assert!(tooltip.contains("Training\n10%\nMorale\n100%"));
    assert!(!tooltip.contains("1,885 / 2,000"));
    assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text.starts_with("M  "))));
}

fn deployment_cells(output: &egui::FullOutput) -> Vec<egui::Rect> {
    output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect)
                if rect.rect.width() <= 34.1
                    && (rect.rect.height() - rect.rect.width() - 6.).abs() < 0.1
                    && (rect.stroke.width - 0.8).abs() < 0.1 =>
            {
                Some(rect.rect)
            },
            _ => None,
        })
        .collect()
}

fn first_visible_rear_cell(output: &egui::FullOutput) -> egui::Rect {
    let cells: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect)
                if rect.rect.width() <= 34.1
                    && (rect.rect.height() - rect.rect.width() - 6.).abs() < 0.1
                    && (rect.stroke.width - 0.8).abs() < 0.1
                    && shape.clip_rect.contains(rect.rect.center()) =>
            {
                Some(rect.rect)
            },
            _ => None,
        })
        .collect();
    let rear_top = cells.iter().map(egui::Rect::top).fold(f32::NEG_INFINITY, f32::max);
    cells
        .into_iter()
        .filter(|rect| (rect.top() - rear_top).abs() < 0.1)
        .min_by(|a, b| (a.center().x - 300.).abs().total_cmp(&(b.center().x - 300.).abs()))
        .expect("Visible rear-line deployment slot")
}

#[test]
fn choosing_rear_line_selects_an_available_type_and_saves_the_plan() {
    let (mut world, economy, graph) = military_fixture();
    world.seed_unit(0, ForceOwner::Player(0), UnitType::LightInfantry).unwrap();
    world
        .set_plan(
            0,
            ForceOwner::Player(0),
            BattlePlan {
                secondary_unit_type: UnitType::HeavyInfantry,
                ..Default::default()
            },
        )
        .unwrap();
    let ctx = egui::Context::default();
    let (output, _) = render_army(&ctx, &world, &economy, &graph, 0., vec![]);
    let target = text_position(&output, "Rear line");
    render_army(&ctx, &world, &economy, &graph, 0.1, click_events(target, true));
    render_army(&ctx, &world, &economy, &graph, 0.2, click_events(target, false));
    let (menu, _) = render_army(&ctx, &world, &economy, &graph, 0.3, vec![]);
    let choice = menu
        .shapes
        .iter()
        .rev()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == "Light Infantry" => {
                Some(text.pos + text.galley.size() * 0.5)
            },
            _ => None,
        })
        .expect("Light Infantry preference in the open selector");
    assert!(!menu.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text)
        if text.galley.job.text == "War Elephants")));
    render_army(&ctx, &world, &economy, &graph, 0.4, click_events(choice, true));
    let (_, action) = render_army(&ctx, &world, &economy, &graph, 0.5, click_events(choice, false));
    assert!(matches!(
        action,
        Some(MilitaryUiAction::SavePlan(BattlePlan {
            secondary_unit_type: UnitType::LightInfantry,
            ..
        }))
    ));
}

#[test]
fn army_rows_use_player_names_and_local_forces_use_province_names_in_gray() {
    let (mut world, economy, graph) = military_fixture();
    world.seed_unit(0, ForceOwner::Local(0), UnitType::LightInfantry).unwrap();
    let ctx = egui::Context::default();
    render_military(&ctx, &world, &economy, &graph, 0., vec![]);
    let (output, _) = render_military(&ctx, &world, &economy, &graph, 0.1, vec![]);
    text_position(&output, "Player 1");
    let local = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == "Italia's army" => Some(text),
            _ => None,
        })
        .unwrap();
    assert_eq!(local.galley.job.sections[0].format.color, egui::Color32::from_rgb(118, 118, 111));
    assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text)
        if text.galley.job.text == "Local defenders")));
}
#[test]
fn deployment_slots_and_cohort_cards_are_read_only() {
    let (mut world, economy, graph) = military_fixture();
    world.seed_unit(0, ForceOwner::Player(0), UnitType::LightInfantry).unwrap();
    world
        .set_plan(
            0,
            ForceOwner::Player(0),
            BattlePlan {
                secondary_unit_type: UnitType::HeavyInfantry,
                ..Default::default()
            },
        )
        .unwrap();
    let ctx = egui::Context::default();
    let (output, _) = render_army(&ctx, &world, &economy, &graph, 0., vec![]);
    let roster = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect) if (rect.rect.height() - 86.).abs() < 0.1 => Some(rect.rect),
            _ => None,
        })
        .max_by(|a, b| a.top().total_cmp(&b.top()))
        .unwrap();
    text_position(&output, "1 cohort");
    assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
        egui::Shape::Text(text) if text.galley.job.text == "1 cohorts")));
    let source = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Mesh(mesh) if roster.contains_rect(mesh.calc_bounds()) => {
                Some(mesh.calc_bounds().center())
            },
            _ => None,
        })
        .expect("Light Infantry icon in the cohort summary");
    let target = first_visible_rear_cell(&output).center();
    for (index, position) in [source, target].into_iter().enumerate() {
        let time = index as f64 * 0.4;
        let (hover, _) = render_army(
            &ctx,
            &world,
            &economy,
            &graph,
            time + 0.1,
            vec![egui::Event::PointerMoved(position)],
        );
        assert_eq!(hover.platform_output.cursor_icon, egui::CursorIcon::Default);
        render_army(&ctx, &world, &economy, &graph, time + 0.2, click_events(position, true));
        let (_, action) =
            render_army(&ctx, &world, &economy, &graph, time + 0.3, click_events(position, false));
        assert!(action.is_none());
    }
    assert_eq!(world.provinces[0].forces[&ForceOwner::Player(0)].len(), 2);
}
#[test]
fn military_banner_badges_toggle_recruitment_and_keep_rank_read_only() {
    let (world, economy, graph) = military_fixture();
    let ctx = egui::Context::default();
    let (output, _) = render_military(&ctx, &world, &economy, &graph, 0., vec![]);
    let rank = text_position(&output, "Centurion");
    let recruit = text_position(&output, "Recruit units");
    let banner = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Mesh(mesh) => {
                let bounds = mesh.calc_bounds();
                (bounds.width() > bounds.height() * 3.
                    && bounds.contains(rank)
                    && bounds.contains(recruit))
                .then_some(bounds)
            },
            _ => None,
        })
        .expect("Rank and recruitment badges must overlay the camp image");
    assert!(rank.x < recruit.x);
    assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
        egui::Shape::Text(text) if text.galley.job.text.starts_with("Army ·"))));
    render_military(&ctx, &world, &economy, &graph, 0.1, click_events(rank, true));
    let (output, action) =
        render_military(&ctx, &world, &economy, &graph, 0.2, click_events(rank, false));
    assert!(action.is_none());
    assert!(banner.contains(text_position(&output, "Recruit units")));
    text_position(&output, "Army strength");
    let badge_fill = |output: &egui::FullOutput| {
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Rect(rect)
                    if rect.rect.contains(recruit) && rect.rect.width() < banner.width() =>
                {
                    Some(rect.fill)
                },
                _ => None,
            })
            .expect("Recruitment badge background")
    };
    let idle = badge_fill(&output);
    let (hovered, _) = render_military(
        &ctx,
        &world,
        &economy,
        &graph,
        0.3,
        vec![egui::Event::PointerMoved(recruit)],
    );
    assert_ne!(badge_fill(&hovered), idle);
    let (pressed, _) =
        render_military(&ctx, &world, &economy, &graph, 0.4, click_events(recruit, true));
    assert_ne!(badge_fill(&pressed), badge_fill(&hovered));
    let (output, _) =
        render_military(&ctx, &world, &economy, &graph, 0.5, click_events(recruit, false));
    text_position(&output, "RECRUITMENT");
    let stationed = text_position(&output, "Stationed units");
    render_military(&ctx, &world, &economy, &graph, 0.6, click_events(stationed, true));
    let (output, action) =
        render_military(&ctx, &world, &economy, &graph, 0.7, click_events(stationed, false));
    assert!(action.is_none());
    text_position(&output, "Army strength");
    text_position(&output, "Recruit units");
}
#[test]
fn province_banner_uses_territorial_rank_and_closes_recruitment_after_ownership_loss() {
    let (mut world, mut economy, graph) = military_fixture();
    let own = ForceOwner::Player(0);
    let enemy = ForceOwner::Player(1);
    world.ranks.insert(own, MilitaryRank::Imperator);
    world.ranks.insert(enemy, MilitaryRank::Legate);
    world.ranks.insert(ForceOwner::Local(0), MilitaryRank::Imperator);
    world.seed_unit(0, enemy, UnitType::Archers).unwrap();
    world.seed_unit(0, ForceOwner::Local(0), UnitType::LightInfantry).unwrap();
    for province_owner in [Some(0), None, Some(1)] {
        economy.provinces[0].owner = province_owner;
        for stationed in [true, false] {
            let mut shown_world = world.clone();
            if !stationed {
                shown_world.provinces[0].forces.clear();
            }
            let ctx = egui::Context::default();
            let tab_id = egui::Id::new(("province-military-tab", 0_usize, 0_usize));
            ctx.data_mut(|data| data.insert_temp(tab_id, MilitaryTab::Recruitment));
            let (output, action) =
                render_military(&ctx, &shown_world, &economy, &graph, 0., vec![]);
            assert!(action.is_none());
            let expected_rank = if province_owner == Some(1) {
                "Legate"
            } else {
                "Centurion"
            };
            text_position(&output, expected_rank);
            let owns_province = province_owner == Some(0);
            assert_eq!(
                ctx.data(|data| data.get_temp::<MilitaryTab>(tab_id))
                    == Some(MilitaryTab::Recruitment),
                owns_province,
            );
            for label in ["RECRUITMENT", "Stationed units"] {
                assert_eq!(
                    output.shapes.iter().any(|shape| matches!(&shape.shape,
                        egui::Shape::Text(text) if text.galley.job.text == label)),
                    owns_province,
                );
            }
            if !owns_province {
                assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
                    egui::Shape::Text(text) if text.galley.job.text == "Recruit units")));
            }
        }
    }
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
    text_position(&output, "RECRUITMENT");
    let recruit = text_position(&output, "Light Infantry");
    let hover_ctx = egui::Context::default();
    let scale = 1.;
    let mut hover = hover_ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(500., 1400.))),
            ..Default::default()
        },
        |ui| recruitment_hover(ui, UnitType::LightInfantry, &world.config, 1., None, scale),
    );
    hover.textures_delta.clear();
    let title = hover
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text)
                if text.galley.job.text == "Light Infantry"
                    && (text.galley.job.sections[0].format.font_id.size - 20. * scale).abs()
                        < 0.1 =>
            {
                Some(text.pos)
            },
            _ => None,
        })
        .expect("Recruitment hover must show the cohort name beside the image");
    let description = hover
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text)
                if text.galley.job.text == unit_description(UnitType::LightInfantry) =>
            {
                Some(text.pos)
            },
            _ => None,
        })
        .expect("Recruitment hover must describe the cohort");
    assert!((title.x - description.x).abs() < 1.);
    let statistics = text_position(&hover, "Statistics");
    assert!(statistics.y > description.y);
    assert!(text_position(&hover, "Cohort capabilities").y > statistics.y);
    assert!(
        text_position(&hover, "Tactic capabilities").y
            > text_position(&hover, "Cohort capabilities").y
    );
    for stat in ["Offense", "Defense", "Speed", "Maneuver"] {
        let position = text_position(&hover, stat);
        assert!(position.y > description.y);
        assert!(position.y > statistics.y);
        assert!(position.y < text_position(&hover, "Cohort capabilities").y);
    }
    assert!(text_position(&hover, "Heavy Infantry").y > statistics.y);
    assert!(hover.shapes.iter().any(|shape| match &shape.shape {
        egui::Shape::Mesh(mesh) => {
            let bounds = mesh.calc_bounds();
            (bounds.width() - 112. * scale).abs() < 1.
                && (bounds.height() - 112. * scale).abs() < 1.
                && bounds.right() + 12. * scale <= title.x
                && (bounds.top() - title.y).abs() < 3.
        },
        _ => false,
    }));
    for value in ["12", "1.5", "2"] {
        assert!(text_position(&hover, value).y < description.y);
    }
    render_military(&ctx, &world, &economy, &graph, 1.2, click_events(recruit, true));
    let (_, action) =
        render_military(&ctx, &world, &economy, &graph, 1.3, click_events(recruit, false));
    assert!(matches!(action, Some(MilitaryUiAction::Recruit(UnitType::LightInfantry))));
    assert_eq!(
        economy.players[0].resources[1], 500.,
        "The command bridge owns payment, not rendering"
    );
    assert!(world.provinces[0].recruitment.is_none());
}
#[test]
fn recruitment_hover_colors_only_matchup_percentages_and_omits_damage_and_footer_text() {
    let config = MilitaryConfig::default();
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600., 1000.))),
            ..Default::default()
        },
        |ui| {
            recruitment_hover(ui, UnitType::HeavyInfantry, &config, 1., None, 1.);
        },
    );
    output.textures_delta.clear();
    let mut positive = 0;
    let mut negative = 0;
    for shape in &output.shapes {
        let egui::Shape::Text(text) = &shape.shape else {
            continue;
        };
        let label = &text.galley.job.text;
        for removed in [
            "Morale damage",
            "Manpower damage",
            "Population and Metal",
            "upkeep starts",
            "full funding.",
            "Click to add",
        ] {
            assert!(!label.contains(removed), "Unwanted tooltip text: {label}");
        }
        if label.ends_with('%') && (label.starts_with('+') || label.starts_with('-')) {
            let color = text.galley.job.sections[0].format.color;
            if label.starts_with('+') {
                positive += 1;
                assert_eq!(color, egui::Color32::from_rgb(48, 112, 60));
            } else {
                negative += 1;
                assert_eq!(color, egui::Color32::from_rgb(170, 45, 35));
            }
        }
    }
    assert!(positive > 0 && negative > 0);
    assert!(!unit_definition_tip(UnitType::HeavyInfantry, &config).contains("damage"));
}
#[test]
fn recruitment_hover_places_a_warning_badge_between_description_and_statistics() {
    let config = MilitaryConfig::default();
    let ctx = egui::Context::default();
    let reason = "Requires the province's recruitment tradition";
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600., 1000.))),
            ..Default::default()
        },
        |ui| recruitment_hover(ui, UnitType::HorseArchers, &config, 1., Some(reason), 1.),
    );
    output.textures_delta.clear();
    let description = text_position(&output, unit_description(UnitType::HorseArchers));
    let warning = text_position(&output, reason);
    let statistics = text_position(&output, "Statistics");
    assert!(description.y < warning.y && warning.y < statistics.y);
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
        egui::Shape::Rect(rect) if rect.fill == egui::Color32::from_rgb(248, 223, 217)
    )));
}
#[test]
fn selecting_a_tactic_saves_the_stationary_army_plan() {
    let (world, economy, graph) = military_fixture();
    let ctx = egui::Context::default();
    let (output, _) = render_army(&ctx, &world, &economy, &graph, 0., vec![]);
    let tactic = text_position(&output, "Shock Action");
    render_army(&ctx, &world, &economy, &graph, 0.1, click_events(tactic, true));
    render_army(&ctx, &world, &economy, &graph, 0.2, click_events(tactic, false));
    let (output, _) = render_army(&ctx, &world, &economy, &graph, 0.3, vec![]);
    let phalanx = text_position(&output, "Phalanx");
    render_army(&ctx, &world, &economy, &graph, 0.4, click_events(phalanx, true));
    let (_, action) =
        render_army(&ctx, &world, &economy, &graph, 0.5, click_events(phalanx, false));
    assert!(matches!(
        action,
        Some(MilitaryUiAction::SavePlan(BattlePlan {
            tactic: CombatTactic::Phalanx,
            ..
        }))
    ));
}
#[test]
fn frontline_selector_changes_the_preference_without_creating_cohorts() {
    let (mut world, economy, graph) = military_fixture();
    world.seed_unit(0, ForceOwner::Player(0), UnitType::LightInfantry).unwrap();
    let ctx = egui::Context::default();
    let (output, _) = render_army(&ctx, &world, &economy, &graph, 0., vec![]);
    let frontline = text_position(&output, "Frontline") + egui::vec2(0., 32.);
    render_army(&ctx, &world, &economy, &graph, 0.1, click_events(frontline, true));
    render_army(&ctx, &world, &economy, &graph, 0.2, click_events(frontline, false));
    let (output, _) = render_army(&ctx, &world, &economy, &graph, 0.3, vec![]);
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
    render_army(&ctx, &world, &economy, &graph, 0.4, click_events(infantry, true));
    let (_, action) =
        render_army(&ctx, &world, &economy, &graph, 0.5, click_events(infantry, false));
    assert!(matches!(
        action,
        Some(MilitaryUiAction::SavePlan(BattlePlan {
            primary_unit_type: UnitType::LightInfantry,
            ..
        }))
    ));
    assert_eq!(world.provinces[0].forces[&ForceOwner::Player(0)].len(), 2);
}
#[test]
fn engaged_army_locks_tactics_but_allows_recruitment() {
    let (mut world, economy, graph) = military_fixture();
    let owner = ForceOwner::Player(0);
    let local = ForceOwner::Local(0);
    world.seed_unit(0, local, UnitType::LightInfantry).unwrap();
    world.start_battle(0, &[owner], &[local], None, None, MilitaryTerrain::Plains, 0, 10).unwrap();
    let ctx = egui::Context::default();
    let (output, _) = render_army(&ctx, &world, &economy, &graph, 0., vec![]);
    let tactic = text_position(&output, "Shock Action");
    render_army(&ctx, &world, &economy, &graph, 0.1, click_events(tactic, true));
    let (output, action) =
        render_army(&ctx, &world, &economy, &graph, 0.2, click_events(tactic, false));
    assert!(action.is_none());
    assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text.starts_with("Phalanx"))));
    let (output, _) = render_military(&ctx, &world, &economy, &graph, 0.25, vec![]);
    let tab = text_position(&output, "Recruit units");
    render_military(&ctx, &world, &economy, &graph, 0.3, click_events(tab, true));
    let (output, _) =
        render_military(&ctx, &world, &economy, &graph, 0.4, click_events(tab, false));
    let recruit = text_position(&output, "Light Infantry");
    render_military(&ctx, &world, &economy, &graph, 0.5, click_events(recruit, true));
    let (_, action) =
        render_military(&ctx, &world, &economy, &graph, 0.6, click_events(recruit, false));
    assert!(matches!(action, Some(MilitaryUiAction::Recruit(UnitType::LightInfantry))));
}
#[test]
fn foreign_army_and_battle_show_morale_but_hide_training_and_positions() {
    let (mut world, mut economy, graph) = military_fixture();
    let own = ForceOwner::Player(0);
    let enemy = ForceOwner::Player(1);
    economy.provinces[0].owner = Some(1);
    world.provinces[0].forces.clear();
    world.seed_unit(0, enemy, UnitType::Archers).unwrap();
    world.provinces[0].forces.get_mut(&enemy).unwrap()[0].training = 97.0;
    world.provinces[0].forces.get_mut(&enemy).unwrap()[0].morale = 81.0;
    let ctx = egui::Context::default();
    ctx.memory_mut(|memory| memory.set_everything_is_visible(true));
    let (output, action) = render_military(&ctx, &world, &economy, &graph, 0.0, vec![]);
    let texts = |output: &egui::FullOutput| {
        output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(t) => Some(t.galley.job.text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let labels = texts(&output);
    assert!(labels.iter().any(|s| s == "Player 2"));
    assert!(labels.iter().any(|s| s == "81%"));
    assert!(!labels.iter().any(|s| s.starts_with("T  ") || s.starts_with("M  ")));
    assert!(action.is_none());
    world.seed_unit(0, own, UnitType::HeavyInfantry).unwrap();
    world
        .set_plan(
            0,
            enemy,
            BattlePlan {
                tactic: CombatTactic::Deception,
                flank_size: 2,
                ..Default::default()
            },
        )
        .unwrap();
    world.start_battle(0, &[own], &[enemy], None, None, MilitaryTerrain::Plains, 0, 10).unwrap();
    let ctx = egui::Context::default();
    ctx.memory_mut(|memory| memory.set_everything_is_visible(true));
    let (first, _) = render_military(&ctx, &world, &economy, &graph, 1.0, vec![]);
    let before = texts(&first);
    assert!(!before
        .iter()
        .any(|s| s.contains("Player 2 · Deception") || s == "81" || s == "T  97" || s == "M  81"));
    let enemy_side = &mut world.battles[0].defenders;
    enemy_side.units[0].training = 73.0;
    enemy_side.units[0].morale = 29.0;
    enemy_side.plans.get_mut(&enemy).unwrap().tactic = CombatTactic::ShockAction;
    enemy_side.formation.front.reverse();
    enemy_side.formation.flank_size = 1;
    let ctx = egui::Context::default();
    ctx.memory_mut(|memory| memory.set_everything_is_visible(true));
    let (second, _) = render_military(&ctx, &world, &economy, &graph, 1.0, vec![]);
    assert!(texts(&second).iter().any(|s| s == "29%"));
    assert!(!texts(&second).iter().any(|s| s.contains("Shock Action") || s == "T  73"));
}
#[test]
fn scouting_estimates_use_baselines_instead_of_private_enemy_condition() {
    let (mut world, _, _) = military_fixture();
    let enemy = ForceOwner::Player(1);
    world.seed_unit(0, enemy, UnitType::Archers).unwrap();
    let unit = &mut world.provinces[0].forces.get_mut(&enemy).unwrap()[0];
    unit.training = 97.0;
    unit.morale = 81.0;
    let units = known_hostile_units(&world, 0, ForceOwner::Player(0), |a, b| a != b);
    assert_eq!(units[0].training, world.config.starting_training);
    assert_eq!(units[0].morale, 81.0);
    assert_eq!(world.provinces[0].forces[&enemy][0].training, 97.0);
}
#[test]
fn empty_owned_province_hides_formation_orders_but_shows_the_military_rank() {
    let (mut world, economy, graph) = military_fixture();
    world.provinces[0].forces.clear();
    let ctx = egui::Context::default();
    let (output, action) = render_military(&ctx, &world, &economy, &graph, 0., vec![]);
    assert!(action.is_none());
    text_position(&output, "Centurion");
    let empty = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == "No army stationed here." => {
                Some(text.pos)
            },
            _ => None,
        })
        .expect("Empty stationed army message");
    let banner = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Mesh(mesh) => {
                let bounds = mesh.calc_bounds();
                (bounds.width() > bounds.height() * 3.
                    && bounds.contains(text_position(&output, "Centurion")))
                .then_some(bounds)
            },
            _ => None,
        })
        .expect("Military camp banner");
    assert!(empty.y > banner.bottom());
    assert!(empty.x > banner.left() + 5.);
    for label in
        ["Frontline", "Rear line", "Flanks", "Deployment", "Recruit cohorts", "Available cohorts"]
    {
        assert!(!output.shapes.iter().any(
            |shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == label)
        ));
    }
}
#[test]
fn deployment_is_visible_without_expanding_and_owned_preferences_exclude_missing_units() {
    let (world, economy, graph) = military_fixture();
    let ctx = egui::Context::default();
    let (output, _) = render_army(&ctx, &world, &economy, &graph, 0., vec![]);
    text_position(&output, "Deployment");
    let frontline = text_position(&output, "Frontline") + egui::vec2(0., 32.);
    render_army(&ctx, &world, &economy, &graph, 0.1, click_events(frontline, true));
    render_army(&ctx, &world, &economy, &graph, 0.2, click_events(frontline, false));
    let (output, _) = render_army(&ctx, &world, &economy, &graph, 0.3, vec![]);
    assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == "Light Infantry")));
}
#[test]
fn recruitment_grid_overflows_with_orders_and_keeps_population_above_active_queue() {
    let (mut world, mut economy, graph) = military_fixture();
    for kind in [UnitType::LightInfantry, UnitType::Archers] {
        world
            .recruit(
                0,
                0,
                kind,
                true,
                &[],
                &mut economy.provinces[0].population,
                &mut economy.players[0].resources[1],
            )
            .unwrap();
    }
    let ctx = egui::Context::default();
    ctx.data_mut(|data| {
        data.insert_temp(
            egui::Id::new(("province-military-tab", 0usize, 0usize)),
            MilitaryTab::Recruitment,
        )
    });
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(570., 600.))),
            ..Default::default()
        },
        |ui| {
            *ui.style_mut() = super::super::campaign_widgets::map_style(1.);
            show(
                ui,
                &world,
                &economy,
                &graph,
                0,
                0,
                |_| true,
                |_| None,
                |_, _| MilitaryAccess::Peaceful,
                |a, b| a != b,
            );
        },
    );
    output.textures_delta.clear();
    let header = text_position(&output, "RECRUITMENT");
    let queue = text_position(&output, "Queue");
    let first = text_position(&output, "Light Infantry");
    let second = text_position(&output, "Heavy Infantry");
    let last = text_position(&output, "Catapult");
    assert!(header.y < queue.y && queue.y < first.y);
    assert!((first.y - second.y).abs() < 1.);
    assert!(second.x > first.x);
    assert!(last.y > 600., "Orders must make the roster scroll instead of shrinking it");
    for label in ["Available cohorts", "Current recruitment", "Order accepted.", "+", "Q"] {
        assert!(!output.shapes.iter().any(
            |shape| matches!(&shape.shape,egui::Shape::Text(text) if text.galley.job.text==label)
        ));
    }
}
#[test]
fn recruitment_queue_right_click_dispatches_only_the_selected_waiting_order() {
    let (mut world, mut economy, graph) = military_fixture();
    for kind in [UnitType::LightInfantry, UnitType::Archers, UnitType::HeavyInfantry] {
        world
            .recruit(
                0,
                0,
                kind,
                true,
                &[],
                &mut economy.provinces[0].population,
                &mut economy.players[0].resources[1],
            )
            .unwrap();
    }
    let ctx = egui::Context::default();
    ctx.data_mut(|data| {
        data.insert_temp(
            egui::Id::new(("province-military-tab", 0usize, 0usize)),
            MilitaryTab::Recruitment,
        )
    });
    render_military(&ctx, &world, &economy, &graph, 0.0, vec![]);
    let (output, _) = render_military(&ctx, &world, &economy, &graph, 0.1, vec![]);
    let queue_y = text_position(&output, "Queue").y;
    let icons: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Mesh(mesh) => {
                let bounds = mesh.calc_bounds();
                ((bounds.width()
                    - 28.0 * super::super::viewport_ui_scale(ctx.content_rect().size()))
                .abs()
                    < 0.1
                    && (bounds.center().y - queue_y).abs() < 5.0)
                    .then_some(bounds)
            },
            _ => None,
        })
        .collect();
    assert_eq!(icons.len(), 2);
    let position = icons[1].center();
    let secondary = |pressed| {
        vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    };
    render_military(&ctx, &world, &economy, &graph, 0.2, secondary(true));
    let (_, action) = render_military(&ctx, &world, &economy, &graph, 0.3, secondary(false));
    assert!(matches!(action, Some(MilitaryUiAction::CancelQueuedRecruitment(1))));
    let position = text_position(&output, "×");
    render_military(&ctx, &world, &economy, &graph, 0.4, click_events(position, true));
    let (_, action) =
        render_military(&ctx, &world, &economy, &graph, 0.5, click_events(position, false));
    assert!(matches!(action, Some(MilitaryUiAction::CancelRecruitment)));
}
#[test]
fn recruitment_queue_shows_all_thirteen_waiting_orders_without_scrolling() {
    let (mut world, mut economy, _) = military_fixture();
    economy.provinces[0].population[2] = 100_000.;
    economy.players[0].resources[1] = 100_000.;
    for _ in 0..=ProvinceMilitaryState::MAX_RECRUITMENT_QUEUE {
        world
            .recruit(
                0,
                0,
                UnitType::LightInfantry,
                true,
                &[],
                &mut economy.provinces[0].population,
                &mut economy.players[0].resources[1],
            )
            .unwrap();
    }
    let ctx = egui::Context::default();
    render_recruitment_roster(&ctx, &world, &economy, egui::vec2(380., 1000.), 0., vec![]);
    let output =
        render_recruitment_roster(&ctx, &world, &economy, egui::vec2(380., 1000.), 0.1, vec![]);
    let queue_y = text_position(&output, "Queue").y;
    let roster_y = text_position(&output, "Light Infantry").y;
    let icons: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Mesh(mesh) => {
                let bounds = mesh.calc_bounds();
                ((bounds.width() - 28.).abs() < 0.1
                    && bounds.center().y >= queue_y - 5.
                    && bounds.center().y < roster_y)
                    .then_some(bounds)
            },
            _ => None,
        })
        .collect();
    assert_eq!(icons.len(), ProvinceMilitaryState::MAX_RECRUITMENT_QUEUE);
    assert!(icons.iter().any(|icon| icon.center().y > queue_y + 28.));
}
fn render_recruitment_roster(
    ctx: &egui::Context,
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    size: egui::Vec2,
    time: f64,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let scale = 1.0;
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600., 1000.),
            )),
            time: Some(time),
            events,
            ..Default::default()
        },
        |ui| {
            *ui.style_mut() = super::super::campaign_widgets::map_style(scale);
            ui.set_width(size.x);
            ui.set_clip_rect(egui::Rect::from_min_size(ui.min_rect().min, size));
            recruitment_view(ui, world, economy, 0, 0, false);
        },
    );
    output.textures_delta.clear();
    output
}
fn recruitment_fill_width(output: &egui::FullOutput) -> f32 {
    output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect) if rect.fill == egui::Color32::from_rgb(190, 150, 76) => {
                Some(rect.rect.width())
            },
            _ => None,
        })
        .expect("The active cohort must have a progress fill")
}
#[test]
fn recruitment_cards_keep_their_empty_panel_size_and_scroll_only_after_orders() {
    let cards = |output: &egui::FullOutput| {
        output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Rect(rect) if rect.stroke.width == 0.7 => {
                    Some((rect.rect, shape.clip_rect))
                },
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let scroll_events = |position| {
        vec![
            egui::Event::PointerMoved(position),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0., -1000.),
                phase: egui::TouchPhase::Move,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    };
    for size in [egui::vec2(570., 600.), egui::vec2(548., 510.)] {
        let (mut world, mut economy, _) = military_fixture();
        let ctx = egui::Context::default();
        let empty = render_recruitment_roster(&ctx, &world, &economy, size, 0., vec![]);
        let baseline = cards(&empty);
        assert_eq!(baseline.len(), UnitType::ALL.len());
        assert!(
            baseline.iter().all(|(rect, clip)| clip.contains_rect(*rect)),
            "The empty roster must fit without scrolling at {size:?}: {baseline:?}"
        );
        let pointer = baseline[0].0.center();
        let empty_scrolled =
            render_recruitment_roster(&ctx, &world, &economy, size, 1., scroll_events(pointer));
        assert_eq!(cards(&empty_scrolled)[0].0, baseline[0].0, "The empty roster must not scroll");
        for (index, kind) in [UnitType::LightInfantry, UnitType::Archers].into_iter().enumerate() {
            world
                .recruit(
                    0,
                    0,
                    kind,
                    true,
                    &[],
                    &mut economy.provinces[0].population,
                    &mut economy.players[0].resources[1],
                )
                .unwrap();
            let time = 2. + index as f64 * 3.;
            let queued = render_recruitment_roster(&ctx, &world, &economy, size, time, vec![]);
            let queued_cards = cards(&queued);
            assert_eq!(queued_cards.len(), baseline.len());
            for ((card, _), (default, _)) in queued_cards.iter().zip(&baseline) {
                assert_eq!(card.size(), default.size(), "Orders must not shrink unit buttons");
            }
            assert!(
                queued_cards.last().unwrap().0.bottom() > queued_cards.last().unwrap().1.bottom(),
                "The queue must overflow instead of shrinking the cards"
            );
            render_recruitment_roster(
                &ctx,
                &world,
                &economy,
                size,
                time + 1.,
                scroll_events(egui::pos2(size.x - 25., size.y - 25.)),
            );
            let scrolled =
                render_recruitment_roster(&ctx, &world, &economy, size, time + 1.5, vec![]);
            let scrolled_cards = cards(&scrolled);
            assert!(
                scrolled_cards[0].0.top() < queued_cards[0].0.top(),
                "The overflowing roster must scroll: size={size:?}, order={index}, before={:?}, after={:?}",
                queued_cards[0],
                scrolled_cards[0]
            );
            let (last, clip) = scrolled_cards.last().unwrap();
            assert!(clip.contains_rect(*last), "Scrolling must reveal the last unit");
        }
        world
            .cancel_queued_recruitment(
                0,
                0,
                0,
                true,
                &mut economy.provinces[0].population,
                &mut economy.players[0].resources[1],
            )
            .unwrap();
        world.cancel_recruitment(0, ForceOwner::Player(0)).unwrap();
        let cancelled = render_recruitment_roster(&ctx, &world, &economy, size, 9., vec![]);
        assert_eq!(cards(&cancelled), baseline, "Cancelling must restore the fitted roster");
    }
}
#[test]
fn recruitment_bar_moves_with_the_game_clock_and_meets_monthly_work() {
    let (mut world, mut economy, _) = military_fixture();
    world
        .recruit(
            0,
            0,
            UnitType::LightInfantry,
            true,
            &[],
            &mut economy.provinces[0].population,
            &mut economy.players[0].resources[1],
        )
        .unwrap();
    let ctx = egui::Context::default();
    let size = egui::vec2(570., 600.);
    let mut previous_width = 0.;
    for (frame, fraction) in [0.25_f32, 0.5, 0.75, 0.999].into_iter().enumerate() {
        ctx.data_mut(|data| {
            data.insert_temp(egui::Id::new("campaign-construction-month-fraction"), fraction)
        });
        let output = render_recruitment_roster(&ctx, &world, &economy, size, frame as f64, vec![]);
        let width = recruitment_fill_width(&output);
        assert!(width > previous_width, "Recruitment must fill between monthly ticks");
        previous_width = width;
        assert_eq!(world.provinces[0].recruitment.as_ref().unwrap().progress, 0.);
    }
    let paused = render_recruitment_roster(&ctx, &world, &economy, size, 20., vec![]);
    assert_eq!(
        recruitment_fill_width(&paused),
        previous_width,
        "Wall time alone must not move a paused game-clock preview"
    );
    world.advance_recruitment(|_| Some(0));
    ctx.data_mut(|data| {
        data.insert_temp(egui::Id::new("campaign-construction-month-fraction"), 0.0_f32)
    });
    let next_month = render_recruitment_roster(&ctx, &world, &economy, size, 21., vec![]);
    assert!(
        (recruitment_fill_width(&next_month) - previous_width).abs() < 0.5,
        "The fill must meet committed work at the month boundary"
    );
    world.provinces[0].recruitment.as_mut().unwrap().progress = 0.;
    ctx.data_mut(|data| {
        data.insert_temp(egui::Id::new("campaign-construction-month-fraction"), 0.5_f32)
    });
    economy.config.recruitment_speed[economy.provinces[0].policies.recruitment as usize] = 2.;
    let faster_policy = render_recruitment_roster(&ctx, &world, &economy, size, 22., vec![]);
    assert!(
        (recruitment_fill_width(&faster_policy) - previous_width).abs() < 0.5,
        "Recruitment policy speed must scale work within the month"
    );
    economy.config.recruitment_speed[economy.provinces[0].policies.recruitment as usize] = 1.;
    let mut normal_clock = crate::app::GameClock::default();
    let mut fast_clock = crate::app::GameClock::default();
    fast_clock.change_speed(1);
    normal_clock.advance(0.6);
    fast_clock.advance(0.6);
    let mut widths = vec![];
    for clock in [normal_clock, fast_clock] {
        ctx.data_mut(|data| {
            data.insert_temp(
                egui::Id::new("campaign-construction-month-fraction"),
                clock.month_progress / crate::app::SECONDS_PER_MONTH,
            )
        });
        widths.push(recruitment_fill_width(&render_recruitment_roster(
            &ctx,
            &world,
            &economy,
            size,
            23.,
            vec![],
        )));
    }
    assert!(
        (widths[1] - 2. * widths[0]).abs() < 0.1,
        "Doubling game speed must double recruitment preview work over the same elapsed time"
    );
}
#[test]
fn recruitment_cards_keep_large_art_beside_titles_and_compact_costs() {
    let (world, economy, _) = military_fixture();
    for size in [egui::vec2(570., 600.), egui::vec2(380., 600.)] {
        let ctx = egui::Context::default();
        ctx.add_font(egui::epaint::text::FontInsert::new(
            "firasans",
            egui::FontData::from_static(include_bytes!("../../assets/fonts/FiraSans-Bold.ttf")),
            vec![egui::epaint::text::InsertFontFamily {
                family: egui::FontFamily::Proportional,
                priority: egui::epaint::text::FontPriority::Highest,
            }],
        ));
        let scale = 1.0;
        let output = render_recruitment_roster(&ctx, &world, &economy, size, 0., vec![]);
        let names = if size.x >= 500. {
            ["Light Infantry", "War Elephants"]
        } else {
            ["Light Infantry", "Heavy Infantry"]
        };
        for name in names {
            let title = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.job.text == name => Some(text),
                    _ => None,
                })
                .unwrap();
            let card = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Rect(rect)
                        if rect.stroke.width == 0.7 && rect.rect.contains(title.pos) =>
                    {
                        Some(rect.rect)
                    },
                    _ => None,
                })
                .unwrap();
            let image = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Mesh(mesh)
                        if card.contains_rect(mesh.calc_bounds())
                            && mesh.calc_bounds().right() < title.pos.x =>
                    {
                        Some(mesh.calc_bounds())
                    },
                    _ => None,
                })
                .unwrap();
            assert!(image.width() > 38. * scale, "Unit art must be larger than the old icon");
            let costs: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(text)
                        if card.contains(text.pos) && text.galley.job.text != name =>
                    {
                        Some(text)
                    },
                    _ => None,
                })
                .collect();
            assert_eq!(costs.len(), 5);
            assert!(costs[0].pos.x > title.pos.x && costs[0].pos.y > title.pos.y);
            for cost in &costs {
                assert!(
                    card.contains_rect(egui::Rect::from_min_size(cost.pos, cost.galley.size())),
                    "Cost values must fit within the recruitment card at {size:?}"
                );
            }
            for pair in costs.windows(2) {
                assert!((pair[0].pos.y - pair[1].pos.y).abs() < 1.);
                assert!(pair[0].pos.x + pair[0].galley.size().x < pair[1].pos.x);
            }
        }
        let target = text_position(&output, "Light Infantry");
        let hover = render_recruitment_roster(
            &ctx,
            &world,
            &economy,
            size,
            0.1,
            vec![egui::Event::PointerMoved(target)],
        );
        assert_eq!(hover.platform_output.cursor_icon, egui::CursorIcon::PointingHand);
    }
}
#[test]
fn unavailable_recruitment_cards_keep_the_default_cursor_and_do_not_recruit() {
    for (kind, shortage) in [
        (UnitType::HorseArchers, 0),
        (UnitType::WarCamels, 0),
        (UnitType::LightInfantry, 1),
        (UnitType::LightInfantry, 2),
        (UnitType::LightInfantry, 3),
        (UnitType::LightInfantry, 4),
    ] {
        let (mut world, mut economy, graph) = military_fixture();
        match shortage {
            1 => economy.players[0].resources[1] = 0.,
            2 => economy.provinces[0].population[2] = 0.,
            3 => {
                economy.provinces[0].population[2] = 100_000.;
                economy.players[0].resources[1] = 100_000.;
                for _ in 0..=ProvinceMilitaryState::MAX_RECRUITMENT_QUEUE {
                    world
                        .recruit(
                            0,
                            0,
                            kind,
                            true,
                            &[],
                            &mut economy.provinces[0].population,
                            &mut economy.players[0].resources[1],
                        )
                        .unwrap();
                }
            },
            4 => {
                world.seed_unit(0, ForceOwner::Player(1), UnitType::HeavyInfantry).unwrap();
                world.provinces[0].occupation = Some(ForceOwner::Player(1));
                economy.provinces[0].occupied = true;
            },
            _ => assert!(!recruitment_tags(&economy.provinces[0].name)
                .contains(&world.config.unit(kind).special_tag.unwrap())),
        }
        let ctx = egui::Context::default();
        super::super::campaign_widgets::configure_cursor(&ctx);
        ctx.data_mut(|data| {
            data.insert_temp(
                egui::Id::new(("province-military-tab", 0usize, 0usize)),
                MilitaryTab::Recruitment,
            )
        });
        render_military(&ctx, &world, &economy, &graph, 0., vec![]);
        let (output, _) = render_military(&ctx, &world, &economy, &graph, 0.1, vec![]);
        let target = text_position(&output, kind.name());
        let card = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Rect(rect)
                    if rect.stroke.width == 0.7 && rect.rect.contains(target) =>
                {
                    Some(rect.rect)
                },
                _ => None,
            })
            .unwrap();
        let images: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Mesh(mesh) if card.contains_rect(mesh.calc_bounds()) => Some(mesh),
                _ => None,
            })
            .collect();
        assert!(!images.is_empty());
        assert!(
            images.iter().all(|mesh| mesh.vertices.iter().all(|v| v.color.a() < 255)),
            "Unavailable unit art and cost icons must fade like unavailable buildings"
        );
        if shortage == 0 {
            let (available, _) = render_military(
                &ctx,
                &world,
                &economy,
                &graph,
                0.15,
                vec![egui::Event::PointerMoved(text_position(&output, "Light Infantry"))],
            );
            assert_eq!(available.platform_output.cursor_icon, egui::CursorIcon::PointingHand);
        }
        let (hover, _) = render_military(
            &ctx,
            &world,
            &economy,
            &graph,
            0.2,
            vec![egui::Event::PointerMoved(target)],
        );
        assert!(
            hover.shapes.iter().any(|shape| matches!(
                &shape.shape,
                egui::Shape::Rect(rect)
                    if rect.rect.contains(target)
                        && rect.fill == super::super::campaign_widgets::UNAVAILABLE_PURCHASE_FILL
            )),
            "{} must be visibly unavailable",
            kind.name()
        );
        assert_eq!(hover.platform_output.cursor_icon, egui::CursorIcon::Default);
        render_military(&ctx, &world, &economy, &graph, 0.3, click_events(target, true));
        let (_, action) =
            render_military(&ctx, &world, &economy, &graph, 0.4, click_events(target, false));
        assert!(action.is_none(), "Unavailable cards must not place an order");
    }
}
