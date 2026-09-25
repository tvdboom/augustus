//! Tests for application state and UI integration.

use super::*;
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
    assert_eq!(format_hud_delta(0.0), "+0");
    assert_eq!(format_hud_delta(0.9), "+0");
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
        noble_taxes: EdictLevel::High,
        ..Default::default()
    };
    ownership.set_governance_for(0, edicts);
    resources.refresh_player_rates(0, &ownership);
    let happiness = resources.happiness_for(0);
    assert_eq!(happiness.map(|class| class.monthly_delta), [0.0, 1.0, 1.0, 3.0]);
    resources.advance(1, &mut ownership, true);
    assert_eq!(resources.happiness_for(0).map(|class| class.amount), [50.0, 51.0, 51.0, 53.0]);

    edicts.food_rations = EdictLevel::Low;
    edicts.slave_labor = EdictLevel::High;
    edicts.noble_taxes = EdictLevel::Low;
    ownership.set_governance_for(0, edicts);
    resources.refresh_player_rates(0, &ownership);
    assert_eq!(
        resources.happiness_for(0).map(|class| class.monthly_delta),
        [0.0, -1.0, -1.0, -3.0]
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
    assert!(frame(vec![pointer(governance, false)]));
}

#[test]
fn player_standards_share_the_red_outline() {
    let red = image::load_from_memory(PLAYER_STANDARDS[0].1)
        .expect("red player standard PNG must be valid")
        .to_rgba8();
    let red_alpha: Vec<_> = red.pixels().map(|pixel| pixel[3]).collect();
    for (name, png) in PLAYER_STANDARDS.iter().skip(1).copied() {
        let standard =
            image::load_from_memory(png).expect("player standard PNG must be valid").to_rgba8();
        assert_eq!(standard.dimensions(), red.dimensions(), "{name}");
        let alpha: Vec<_> = standard.pixels().map(|pixel| pixel[3]).collect();
        assert_eq!(alpha, red_alpha, "{name}");
    }
}

#[test]
fn player_colors_match_their_banner_cloth() {
    for (index, (name, png)) in PLAYER_STANDARDS.iter().enumerate() {
        let standard =
            image::load_from_memory(png).expect("player standard PNG must be valid").to_rgba8();
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
        for color_index in 0..PLAYER_STANDARDS.len() {
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
