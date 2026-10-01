//! Tests for application state and UI integration.

use super::*;

#[test]
fn practice_boost_updates_campaign_balances_and_every_owned_province() {
    let mut ownership = ProvinceOwnership::default();
    ownership.start_game(&[egui::Color32::RED, egui::Color32::BLUE]);
    let mut campaign = campaign::Campaign::default();
    campaign.start(&ownership, 2);
    let mut resources = HudResources::default();
    resources.start_players(2, &ownership);
    let before_wallet = campaign.economy.players[0].clone();
    let before_populations: Vec<_> = campaign
        .economy
        .provinces
        .iter()
        .map(|province| {
            (province.owner, province.population, province.production(&campaign.economy.config).1)
        })
        .collect();

    apply_practice_boost(0, &mut campaign, &mut resources, &mut ownership);

    let wallet = &campaign.economy.players[0];
    for index in 0..3 {
        assert_eq!(wallet.resources[index], before_wallet.resources[index] + 5_000.0);
        assert_eq!(wallet.practice_storage_bonus[index], 5_000.0);
    }
    assert_eq!(wallet.coin, before_wallet.coin + 5_000.0);
    assert_eq!(wallet.influence, before_wallet.influence + 1_000.0);
    assert_eq!(campaign.actors[0].coin, wallet.coin);
    assert_eq!(campaign.actors[0].influence, wallet.influence);
    let owner = crate::game::military::ForceOwner::Player(0);
    assert_eq!(campaign.military.peak_manpower[&owner], 600.0);
    assert_eq!(campaign.military.victories[&owner], 6);
    assert_eq!(campaign.military.rank(owner), crate::game::military::MilitaryRank::Centurion);
    let boosted_resources = wallet.resources;
    assert_eq!(campaign.economy.players[1].practice_storage_bonus, [0.0; 3]);
    for (province, (owner, population, output)) in
        campaign.economy.provinces.iter().zip(before_populations)
    {
        assert_eq!(
            province.population,
            population.map(|count| count
                * if owner == Some(0) {
                    10.0
                } else {
                    1.0
                })
        );
        let factor = if owner == Some(0) {
            10.0
        } else {
            1.0
        };
        for (actual, before) in
            province.production(&campaign.economy.config).1.into_iter().zip(output)
        {
            assert!((actual - before * factor).abs() < actual.max(1.0) * 1e-12);
        }
    }
    campaign.economy.recalculate_storage();
    campaign.economy.players[0].clamp_storage();
    assert_eq!(campaign.economy.players[0].resources, boosted_resources);
}

#[test]
fn practice_boost_updates_preview_balances_and_owned_population() {
    let mut ownership = ProvinceOwnership::default();
    ownership.start_game(&[egui::Color32::RED, egui::Color32::BLUE]);
    let mut resources = HudResources::default();
    resources.start_players(2, &ownership);
    let before = resources.players[0];
    let before_population = ownership.population_for(0);
    let other_population = ownership.population_for(1);

    apply_practice_boost(0, &mut campaign::Campaign::default(), &mut resources, &mut ownership);

    for index in 0..4 {
        assert_eq!(resources.players[0][index].amount, before[index].amount + 5_000.0);
    }
    assert_eq!(resources.players[0][4].amount, before[4].amount + 1_000.0);
    assert_eq!(ownership.population_for(0), before_population.map(|count| count * 10.0));
    assert_eq!(ownership.population_for(1), other_population);
    assert_eq!(resources.players[0][5].amount, ownership.total_population_for(0));
}

#[test]
fn selected_local_house_color_reaches_players_and_province_headers() {
    let mut practice = LocalPractice {
        color_index: 4,
        player_count: 4,
        ..Default::default()
    };
    let mut ownership = ProvinceOwnership::default();
    practice.start_new_game(&mut ownership);
    assert_eq!(practice.players.iter().map(|p| p.color_index).collect::<Vec<_>>(), [4, 0, 1, 2]);
    for (id, province) in ownership.campaign_seeds().iter().enumerate() {
        if let Some(owner) = province.owner {
            assert_eq!(
                ownership.province_overview(id).unwrap().owner_color,
                Some(PLAYER_COLORS[practice.players[owner].color_index])
            );
        }
    }
}
use bevy::ecs::system::{IntoSystem, System};

#[test]
fn loading_reveal_system_initializes_without_conflicting_access() {
    let mut system = IntoSystem::into_system(draw_loading_reveal);
    system.initialize(&mut World::new());
}

#[test]
fn map_reveal_starts_only_after_loading_and_for_the_map() {
    let mut loading = LoadingSequence::default();
    assert!(!revealing_map(AppState::Loading, &loading));
    loading.map_reveal_progress = Some(0.0);
    assert!(revealing_map(AppState::Loading, &loading));
    assert!(!revealing_map(AppState::Map, &loading));
    loading.destination = AppState::EmptyScreen;
    assert!(!revealing_map(AppState::Loading, &loading));
}

#[test]
fn hud_floors_fractional_values_and_uses_the_least_happy_class_trend() {
    assert_eq!(format_hud_number(999.9), "999");
    assert_eq!(format_hud_number(1_000.0), "1.0k");
    assert_eq!(format_hud_number(1_983.0), "2.0k");
    assert_eq!(format_hud_number(5_240.9), "5.2k");
    assert_eq!(format_hud_number(708_211.0), "708.2k");
    assert_eq!(format_hud_number(999_949.0), "999.9k");
    assert_eq!(format_hud_number(999_950.0), "1.0M");
    assert_eq!(format_hud_number(1_000_000.0), "1.0M");
    assert_eq!(format_hud_number(1_250_000.0), "1.3M");
    assert_eq!(format_hud_number(0.9), "0");
    assert_eq!(format_hud_number(-0.0), "0");
    assert_eq!(format_hud_delta(0.0), "0");
    assert_eq!(format_hud_delta(-0.0), "0");
    assert_eq!(format_hud_delta(0.9), "0");
    assert_eq!(format_hud_delta(-0.1), "-1");
    assert_eq!(format_hud_delta(1_550.0), "+1.6k");
    assert_eq!(format_hud_delta(-1_550.0), "-1.6k");
    let mut fractional_resources = HudResources::default();
    fractional_resources.players[0][3].monthly_delta = 0.5;
    fractional_resources.advance(2, &mut ProvinceOwnership::default(), false);
    assert_eq!(fractional_resources.for_player(0)[3].amount, 202.0);
    let mut resources = HudResources::default();
    resources.happiness[0][0] = HudResource {
        amount: 58.0,
        monthly_delta: 2.0,
    };
    resources.happiness[0][1] = HudResource {
        amount: 42.0,
        monthly_delta: -1.0,
    };
    assert_eq!(resources.for_player(0)[6].amount, 42.0);
    assert_eq!(resources.for_player(0)[6].monthly_delta, -1.0);
    let mut ownership = ProvinceOwnership::default();
    ownership.start_game(&[egui::Color32::RED]);
    resources.advance(1, &mut ownership, true);
    assert_eq!(resources.for_player(0)[6].amount, 41.0);
    assert_eq!(resources.for_player(0)[6].monthly_delta, -1.0);
}

#[test]
fn governance_happiness_drift_updates_without_stacking_old_edicts() {
    use crate::map::EdictLevel;

    let mut ownership = ProvinceOwnership::default();
    ownership.start_game(&[egui::Color32::RED]);
    let mut resources = HudResources::default();
    resources.start_players(1, &ownership);

    let mut edicts = Governance {
        food_rations: EdictLevel::High,
        slave_labor: EdictLevel::Low,
        ..Default::default()
    };
    ownership.set_governance_for(0, edicts);
    resources.refresh_player_rates(0, &ownership);
    let happiness = resources.happiness_for(0);
    assert_eq!(happiness.map(|class| class.monthly_delta), [1.0, 1.0, 1.0, 3.0]);
    resources.advance(1, &mut ownership, true);
    assert_eq!(resources.happiness_for(0).map(|class| class.amount), [51.0, 51.0, 51.0, 53.0]);

    edicts.food_rations = EdictLevel::Low;
    edicts.slave_labor = EdictLevel::High;
    ownership.set_governance_for(0, edicts);
    resources.refresh_player_rates(0, &ownership);
    assert_eq!(
        resources.happiness_for(0).map(|class| class.monthly_delta),
        [-1.0, -1.0, -1.0, -3.0]
    );
    resources.advance(1, &mut ownership, true);
    assert_eq!(resources.happiness_for(0).map(|class| class.amount), [50.0, 50.0, 50.0, 50.0]);
}

#[test]
fn local_players_receive_only_their_provinces_monthly_resources() {
    let mut practice = LocalPractice {
        player_count: 2,
        ..Default::default()
    };
    let mut ownership = ProvinceOwnership::default();
    practice.start_new_game(&mut ownership);
    let mut resources = HudResources::default();
    resources.start_players(practice.players.len(), &ownership);
    for player in 0..practice.players.len() {
        let production = ownership.net_production_for(player);
        for (resource, amount) in production.into_iter().enumerate() {
            assert_eq!(resources.for_player(player)[resource].monthly_delta, amount);
        }
        assert_eq!(resources.for_player(player)[3].monthly_delta, ownership.coin_delta_for(player));
        assert_eq!(
            resources.for_player(player)[4].monthly_delta,
            ownership.influence_delta_for(player)
        );
    }
    let starting: Vec<_> = (0..practice.players.len())
        .map(|player| {
            (
                ownership.net_production_for(player),
                ownership.coin_delta_for(player),
                ownership.influence_delta_for(player),
                ownership.population_for(player).into_iter().sum::<f64>(),
                ownership.population_change_for(player, 20.0, 0),
            )
        })
        .collect();
    resources.advance(1, &mut ownership, true);
    for (player, (production, coin_delta, influence_delta, population, change)) in
        starting.into_iter().enumerate()
    {
        for (resource, amount) in production.into_iter().enumerate() {
            let opening = if resource == 0 {
                20.0
            } else {
                0.0
            };
            assert_eq!(resources.for_player(player)[resource].amount, (opening + amount).max(0.0));
            assert_eq!(
                resources.for_player(player)[resource].monthly_delta,
                ownership.net_production_for(player)[resource]
            );
        }
        assert_eq!(resources.for_player(player)[3].amount, 201.0 + coin_delta);
        assert_eq!(resources.for_player(player)[3].monthly_delta, ownership.coin_delta_for(player));
        assert_eq!(resources.for_player(player)[4].amount, 40.0 + influence_delta);
        assert_eq!(
            resources.for_player(player)[4].monthly_delta,
            ownership.influence_delta_for(player)
        );
        assert!((resources.for_player(player)[5].amount - (population + change)).abs() < 1e-9);
        assert!(
            (resources.for_player(player)[5].amount
                - ownership.population_for(player).into_iter().sum::<f64>())
            .abs()
                < 1e-9
        );
    }
    let after_one_month: Vec<_> =
        (0..practice.players.len()).map(|player| resources.for_player(player)).collect();
    resources.advance(2, &mut ownership, true);
    for (player, previous) in after_one_month.into_iter().enumerate() {
        for resource in 1..3 {
            assert!(resources.for_player(player)[resource].amount >= previous[resource].amount);
        }
    }
}

#[test]
fn map_hud_hit_region_covers_visible_frame() {
    const { assert!(MAP_STANDARD_WIDTH >= MAP_STANDARD_RAIL_IMAGE_WIDTH * 1.1) };
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 900.0));
    assert!(map_hud_contains(screen, egui::pos2(110.0, 40.0)));
    assert!(map_hud_contains(screen, egui::pos2(140.0, 100.0)));
    assert!(map_hud_contains(screen, egui::pos2(40.0, 600.0)));
    assert!(map_hud_contains(screen, egui::pos2(500.0, 20.0)));
    assert!(!map_hud_contains(screen, egui::pos2(100.0, 600.0)));
    assert!(!map_hud_contains(screen, egui::pos2(500.0, 100.0)));
    assert!(!map_hud_contains(screen, egui::pos2(40.0, 870.0)));
}

#[test]
fn player_banner_click_opens_each_players_founding_province() {
    let mut practice = LocalPractice::default();
    let mut ownership = ProvinceOwnership::default();
    practice.start_new_game(&mut ownership);
    let seeds = ownership.campaign_seeds();
    for player in [0, 1, 0] {
        practice.active_player = player;
        let province = practice.players[player].main_province.unwrap();
        assert_eq!(seeds[province].owner, Some(player));
        let context = egui::Context::default();
        let mut view = campaign_panel::CampaignUi::default();
        view.open_province_section((province + 1) % seeds.len(), 3);
        view.open = Some(campaign_panel::CampaignTab::Military);
        let mut detail = ProvincePanelOpen(None);
        let mut governance = GovernancePanelOpen(true);
        let mut map_view = MapView::default();
        let mut expected_map_view = MapView::default();
        expected_map_view.focus_province(province);
        let position = egui::pos2(110.0, 100.0);
        {
            let mut frame = |events| {
                context.begin_pass(egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1600.0, 900.0),
                    )),
                    events,
                    ..Default::default()
                });
                let clicked = draw_map_menu_hitboxes(&context, 1.0);
                if clicked {
                    assert!(open_main_province(
                        &practice,
                        &mut view,
                        &mut detail,
                        &mut governance,
                        &mut map_view,
                    ));
                }
                let mut output = context.end_pass();
                output.textures_delta.clear();
                clicked
            };
            frame(vec![]);
            frame(vec![]);
            for pressed in [true, false] {
                let clicked = frame(vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ]);
                assert_eq!(clicked, !pressed);
            }
        }
        assert_eq!(view.open, Some(campaign_panel::CampaignTab::Province));
        assert_eq!(view.province, Some(province));
        assert_eq!(detail.0, Some(MapDetail::Province(province)));
        assert!(!governance.0);
        assert!(map_view.pending_focus().0.is_some());
        assert_eq!(map_view.pending_focus(), expected_map_view.pending_focus());
        ownership.sync_campaign_province(province, None, [0.0; 4], [0.0; 3], 0.0);
        assert_eq!(practice.players[player].main_province, Some(province));
    }
}

#[test]
fn player_banner_excludes_folds_rail_and_transparent_corners_without_a_tooltip() {
    for (size, scale) in [(egui::vec2(1600.0, 900.0), 1.0), (egui::vec2(800.0, 600.0), 0.85)] {
        let screen = egui::Rect::from_min_size(egui::pos2(25.0, 15.0), size);
        for (source, expected) in [
            (egui::pos2(100.0, 150.0), true),  // Eagle.
            (egui::pos2(100.0, 230.0), true),  // SPQR lettering.
            (egui::pos2(175.0, 250.0), true),  // Flag cloth near its lower edge.
            (egui::pos2(50.0, 340.0), false),  // Curled cloth below the flag.
            (egui::pos2(50.0, 450.0), false),  // Menu rail.
            (egui::pos2(210.0, 270.0), false), // Empty lower-right corner.
            (egui::pos2(210.0, 400.0), false), // Lower-right part of the old square.
        ] {
            let context = egui::Context::default();
            let position = screen.min
                + egui::vec2(
                    (source.x - 7.0) / 213.0 * MAP_STANDARD_WIDTH,
                    (source.y - 20.0) / 1664.0 * map_standard_height(screen, scale),
                ) * scale;
            let mut time = 0.0;
            let mut frame = |events| {
                time += 0.1;
                context.begin_pass(egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(time),
                    events,
                    ..Default::default()
                });
                let clicked = draw_map_menu_hitboxes(&context, scale);
                let mut output = context.end_pass();
                output.textures_delta.clear();
                assert!(output.shapes.is_empty(), "The banner must not paint a tooltip");
                (clicked, output.platform_output.cursor_icon)
            };
            frame(vec![]);
            frame(vec![egui::Event::PointerMoved(position)]);
            for _ in 0..10 {
                frame(vec![]);
            }
            let (_, cursor) = frame(vec![egui::Event::PointerMoved(position)]);
            assert_eq!(
                cursor,
                if expected {
                    egui::CursorIcon::PointingHand
                } else {
                    egui::CursorIcon::Default
                },
                "Hover at {source:?} with scale {scale}"
            );
            for pressed in [true, false] {
                let (clicked, _) = frame(vec![egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                }]);
                assert_eq!(clicked, expected && !pressed, "Click at {source:?} with scale {scale}");
            }
        }
    }
}

#[test]
fn left_menu_stays_clickable_after_the_rail_is_clicked() {
    let context = egui::Context::default();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 900.0));
    let mut textures = std::array::from_fn(|_| None);
    let mut frame = |events| {
        context.begin_pass(egui::RawInput {
            screen_rect: Some(screen),
            events,
            ..Default::default()
        });
        draw_map_menu_hitboxes(&context, 1.0);
        let clicked = draw_map_left_menu(&context, 1.0, &mut textures, true);
        let mut output = context.end_pass();
        output.textures_delta.clear();
        clicked
    };
    let pointer = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    let rail = egui::pos2(60.0, 350.0);
    let governance = egui::pos2(29.0, 301.0);
    frame(vec![]);
    frame(vec![egui::Event::PointerMoved(rail), pointer(rail, true)]);
    frame(vec![pointer(rail, false)]);
    frame(vec![egui::Event::PointerMoved(governance), pointer(governance, true)]);
    assert_eq!(frame(vec![pointer(governance, false)]), Some(0));
}

#[test]
fn player_standards_share_the_red_outline() {
    let red = image::load_from_memory(PLAYER_STANDARD_PNG)
        .expect("red player standard PNG must be valid")
        .to_rgba8();
    let red_alpha: Vec<_> = red.pixels().map(|pixel| pixel[3]).collect();
    for (index, name) in PLAYER_COLOR_NAMES.iter().enumerate() {
        let standard = player_standard_image(index);
        assert_eq!(standard.dimensions(), red.dimensions(), "{name}");
        let alpha: Vec<_> = standard.pixels().map(|pixel| pixel[3]).collect();
        assert_eq!(alpha, red_alpha, "{name}");
        // Fixed samples cover the eagle, laurel and lettering on the standard.
        for (x, y) in [(100, 90), (100, 130), (80, 40), (110, 235)] {
            assert_eq!(
                red.get_pixel(x, y),
                standard.get_pixel(x, y),
                "goldwork must stay unchanged for {name}"
            );
        }
    }
}

#[test]
fn player_colors_match_their_banner_cloth() {
    for (index, name) in PLAYER_COLOR_NAMES.iter().enumerate() {
        let standard = player_standard_image(index);
        let pixel = standard.get_pixel(80, 700);
        assert_eq!(
            PLAYER_COLORS[index],
            egui::Color32::from_rgb(pixel[0], pixel[1], pixel[2]),
            "{name}"
        );
    }
}

#[test]
fn all_player_standards_load_with_eguis_texture_limits() {
    for limit in [2048, 1024] {
        let context = egui::Context::default();
        context.begin_pass(egui::RawInput {
            max_texture_side: Some(limit),
            ..Default::default()
        });
        for color_index in 0..PLAYER_COLORS.len() {
            let texture = load_player_standard(&context, color_index);
            assert!(texture.size()[0] <= limit);
            assert!(texture.size()[1] <= limit);
        }
        let mut output = context.end_pass();
        output.textures_delta.clear();
    }
}

#[test]
fn escape_opens_and_closes_the_correct_game_menu() {
    let mut governance = GovernancePanelOpen(true);
    let mut province = ProvincePanelOpen::default();
    assert!(dismiss_map_panel_on_escape(AppState::Map, &mut governance, &mut province));
    assert!(!governance.0);
    assert!(!dismiss_map_panel_on_escape(AppState::Map, &mut governance, &mut province));
    province.0 = Some(MapDetail::City(0));
    assert!(dismiss_map_panel_on_escape(AppState::Map, &mut governance, &mut province));
    assert_eq!(province.0, None);
    for (game, screen) in [
        (ActiveGame::LocalPractice, AppState::Map),
        (ActiveGame::LobbyPreview, AppState::EmptyScreen),
    ] {
        assert_eq!(escape_destination(screen, game), Some(AppState::GameMenu));
        assert_eq!(escape_destination(AppState::GameMenu, game), Some(screen));
        assert_eq!(escape_destination(AppState::GameSettings, game), Some(AppState::GameMenu));
    }
}

#[test]
fn escape_closes_army_window_before_opening_game_menu() {
    let mut keyboard = ButtonInput::<KeyCode>::default();
    keyboard.press(KeyCode::Escape);
    let mut view = campaign_panel::CampaignUi::default();
    view.open = Some(campaign_panel::CampaignTab::Military);
    let mut app = App::new();
    app.insert_resource(keyboard)
        .insert_resource(State::new(AppState::Map))
        .insert_resource(view)
        .insert_resource(ProvincePanelOpen(Some(MapDetail::Province(0))))
        .init_resource::<ActiveGame>()
        .init_resource::<NextState<AppState>>()
        .init_resource::<GovernancePanelOpen>()
        .init_resource::<MapPanelCloseClick>()
        .add_systems(Update, handle_escape);
    let entity = app
        .world_mut()
        .spawn((bevy_egui::EguiContext::default(), bevy_egui::PrimaryEguiContext))
        .id();
    campaign_military::open_army_panel(
        app.world_mut().get_mut::<bevy_egui::EguiContext>(entity).unwrap().get_mut(),
        0,
        0,
        None,
    );

    app.update();
    assert!(!campaign_military::dismiss_army_panel(
        app.world_mut().get_mut::<bevy_egui::EguiContext>(entity).unwrap().get_mut()
    ));
    assert_eq!(app.world().resource::<campaign_panel::CampaignUi>().open, None);
    assert_eq!(app.world().resource::<ProvincePanelOpen>().0, None);
    assert!(matches!(app.world().resource::<NextState<AppState>>(), NextState::Unchanged));

    let mut keyboard = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keyboard.clear();
    keyboard.release(KeyCode::Escape);
    keyboard.press(KeyCode::Escape);
    drop(keyboard);
    app.update();
    assert!(matches!(
        app.world().resource::<NextState<AppState>>(),
        NextState::Pending(AppState::GameMenu)
    ));
}

#[test]
fn settings_button_closes_settings_to_the_game() {
    for (game, screen) in [
        (ActiveGame::LocalPractice, AppState::Map),
        (ActiveGame::LobbyPreview, AppState::EmptyScreen),
    ] {
        assert_eq!(settings_button_destination(screen, game), AppState::GameSettings);
        assert_eq!(settings_button_destination(AppState::GameMenu, game), AppState::GameSettings);
        assert_eq!(settings_button_destination(AppState::GameSettings, game), screen);
    }
}

#[test]
fn game_clock_advances_months_at_selected_speed() {
    let mut clock = GameClock::default();
    assert_eq!(clock.date_label(), "Jan, 60 AD");
    clock.advance(1.5);
    assert_eq!(clock.date_label(), "Jan, 60 AD");
    clock.advance(1.5);
    assert_eq!(clock.date_label(), "Feb, 60 AD");

    clock.change_speed(1);
    clock.advance(1.5);
    assert_eq!(clock.date_label(), "Mar, 60 AD");
    clock.change_speed(-10);
    clock.advance(12.0);
    assert_eq!(clock.date_label(), "Apr, 60 AD");
    clock.change_speed(10);
    assert_eq!(clock.speed(), 4.0);
    clock.advance(0.75);
    assert_eq!(clock.date_label(), "May, 60 AD");
}

#[test]
fn combat_clock_emits_ordered_rounds_and_respects_speed() {
    let mut clock = GameClock::default();
    assert!(clock.advance_timeline(0.7, 4).is_empty());
    assert_eq!(clock.advance_timeline(0.05, 4), [false]);
    assert_eq!(clock.advance_timeline(2.25, 4), [false, false, true]);
    assert_eq!(clock.date_label(), "Feb, 60 AD");
    clock.change_speed(1);
    assert_eq!(clock.advance_timeline(0.375, 4), [false]);
    assert_eq!(
        clock.advance_timeline(3., 4),
        [false, false, true, false, false, false, true, false]
    );
    assert_eq!(clock.date_label(), "Apr, 60 AD");
    assert!(clock.advance_timeline(0., 4).is_empty());
    let mut arbitrary = GameClock::default();
    assert_eq!(arbitrary.advance_timeline(3., 7), [false, false, false, false, false, false, true]);
}

#[test]
fn game_clock_skips_year_zero() {
    let mut clock = GameClock {
        year: 0,
        month: 11,
        ..Default::default()
    };
    assert_eq!(clock.date_label(), "Dec, 1 BC");
    clock.advance(3.0);
    assert_eq!(clock.date_label(), "Jan, 1 AD");
}
