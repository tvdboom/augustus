//! Exercise the actual menu system, including scaled footer and navigation.
use super::*;
use crate::multiplayer::model::{GameRecord, Member};

#[derive(Resource, Default)]
struct MenuCapture(crate::egui_capture::Capture);

fn menu_app(state: AppState) -> (App, egui::Context) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .insert_resource(State::new(state))
        .init_resource::<NextState<AppState>>()
        .init_resource::<MenuDraft>()
        .init_resource::<LobbyPreview>()
        .init_resource::<online::OnlineClient>()
        .init_resource::<LoadingSequence>()
        .init_resource::<ActiveGame>()
        .init_resource::<LocalPractice>()
        .init_resource::<ProvinceOwnership>()
        .init_resource::<MapView>()
        .insert_resource(LoadingWallpapers(vec![]))
        .insert_resource(MenuAudio {
            volume: 0.0,
            mode: AudioMode::Mute,
            restored_mode: AudioMode::Effects,
            music: default(),
        })
        .init_resource::<Audio>()
        .init_resource::<bevy_egui::EguiUserTextures>()
        .init_resource::<EguiClipboard>()
        .init_resource::<MenuCapture>()
        .add_systems(Update, draw_menu);
    let entity = app
        .world_mut()
        .spawn((bevy_egui::EguiContext::default(), bevy_egui::PrimaryEguiContext))
        .id();
    let context =
        app.world_mut().get_mut::<bevy_egui::EguiContext>(entity).unwrap().get_mut().clone();
    context.global_style_mut(|style| style.animation_time = 0.0);
    (app, context)
}

fn lobby_fixture(app: &mut App, caller: usize, count: usize) {
    app.world_mut().resource_mut::<LobbyPreview>().reset("Mavs", "4T2H6K".into());
    app.world_mut().resource_mut::<online::OnlineClient>().record = Some(GameRecord {
        id: "fixture".into(),
        code: "4T2H6K".into(),
        status: "lobby".into(),
        single_player: false,
        revision: 0,
        state: None,
        members: (0..count)
            .map(|player| Member {
                player,
                display_name: if player == 0 {
                    "Mavs".into()
                } else {
                    "Guest".into()
                },
                color: player,
                connected: true,
            })
            .collect(),
        player: caller,
        recovery_code: "4T2H6K9PW3RX".into(),
        roster_token: "roster".into(),
    });
}

fn frame(
    app: &mut App,
    context: &egui::Context,
    size: egui::Vec2,
    time: f64,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    context.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
            time: Some(time),
            events,
            ..Default::default()
        },
        |_| app.update(),
    )
}

fn settled(
    app: &mut App,
    context: &egui::Context,
    size: egui::Vec2,
    name: &str,
) -> egui::FullOutput {
    let start = context.input(|input| input.time);
    for index in 0..3 {
        let mut output = frame(app, context, size, start + (index + 1) as f64 * 0.1, vec![]);
        app.world_mut().resource_mut::<MenuCapture>().0.frame(context, &output, name);
        output.textures_delta.clear();
        if index == 2 {
            return output;
        }
    }
    unreachable!()
}

fn text<'a>(output: &'a egui::FullOutput, label: &str) -> &'a egui::epaint::TextShape {
    output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == label => Some(text),
            _ => None,
        })
        .unwrap_or_else(|| panic!("Missing menu text: {label}"))
}

fn has_text(output: &egui::FullOutput, label: &str) -> bool {
    output.shapes.iter().any(
        |shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == label),
    )
}

#[test]
fn lobby_matches_reference_cards_copy_and_host_guest_actions() {
    for (name, size) in
        [("desktop", egui::vec2(1600.0, 900.0)), ("compact", egui::vec2(360.0, 640.0))]
    {
        for (caller, count) in [(0, 1), (0, 2), (1, 2)] {
            let (mut app, context) = menu_app(AppState::Lobby);
            lobby_fixture(&mut app, caller, count);
            let output =
                settled(&mut app, &context, size, &format!("menu-lobby-{name}-{caller}-{count}"));
            let labels = ["GAME CODE", "RECOVERY CODE", "PLAYER COLOR", "PLAYERS"];
            for pair in labels.windows(2) {
                assert!(text(&output, pair[0]).pos.y < text(&output, pair[1]).pos.y);
            }
            assert!(!has_text(&output, "PRIVATE RECOVERY CODE"));
            assert!(!has_text(&output, "PLAYER NAME"));
            assert!(!has_text(&output, "Update player card"));
            assert!(!has_text(&output, "The host starts when everyone is connected."));
            text(&output, "4T2H6K");
            let code = text(&output, "4T2H6K9PW3RX");
            assert_eq!(code.galley.rows.len(), 1);
            assert!(code.galley.size().x < 414.0 * viewport_ui_scale(size));
            assert!(code.pos.x >= 0.0 && code.pos.x + code.galley.size().x <= size.x);
            assert_eq!(has_text(&output, "Start Game"), caller == 0);
            assert_eq!(
                lobby_primary_enabled(app.world().resource::<online::OnlineClient>()),
                caller == 0
            );
            let waiting = if caller != 0 {
                "Waiting for the host to start..."
            } else if count == 1 {
                "Ready to start solo — more players may still join."
            } else {
                "Ready to start — more players may still join."
            };
            text(&output, waiting);
            text(
                &output,
                if count == 1 {
                    "1 player"
                } else {
                    "2 players"
                },
            );
            if caller != 0 {
                let leave = text(&output, "Leave Lobby");
                assert!((leave.pos.x + leave.galley.size().x * 0.5 - size.x * 0.5).abs() < 1.0);
            }
        }
    }
}

#[test]
fn menu_errors_share_the_footer_right_edge_at_desktop_and_compact_sizes() {
    for (name, size) in
        [("desktop", egui::vec2(1600.0, 900.0)), ("compact", egui::vec2(360.0, 640.0))]
    {
        let (mut app, context) = menu_app(AppState::JoinGame);
        app.world_mut()
            .resource_mut::<online::OnlineClient>()
            .report_error("Anonymous sign-ins are disabled");
        let output = settled(&mut app, &context, size, &format!("menu-error-{name}"));
        let error = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Rect(rect)
                    if rect.fill == egui::Color32::from_rgba_unmultiplied(67, 23, 32, 232) =>
                {
                    assert!(shape.clip_rect.contains_rect(rect.rect));
                    Some(rect.rect)
                },
                _ => None,
            })
            .unwrap();
        let badge = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Rect(rect)
                    if rect.fill == egui::Color32::from_rgba_unmultiplied(31, 20, 18, 210) =>
                {
                    Some(rect.rect)
                },
                _ => None,
            })
            .unwrap();
        let scale = viewport_ui_scale(size);
        assert!((error.right() - badge.right()).abs() < 1.0);
        assert!((badge.top() - error.bottom() - 10.0 * scale).abs() < 1.0);
        assert!((size.x - error.right() - 24.0 * scale).abs() < 1.0);
        assert!(error.left() >= 0.0);
        text(&output, "Unable to join game");
        text(&output, "Anonymous sign-ins are disabled");
    }
}

#[test]
fn resume_actions_are_one_row_and_recovery_is_a_separate_password_page() {
    let size = egui::vec2(1600.0, 900.0);
    let (mut app, context) = menu_app(AppState::ResumeGame);
    let output = settled(&mut app, &context, size, "menu-resume-empty");
    text(&output, "No games linked to this device");
    let positions: Vec<_> =
        ["Back", "Refresh", "Recover Game"].iter().map(|label| text(&output, label).pos).collect();
    assert!(positions[0].x < positions[1].x && positions[1].x < positions[2].x);
    assert!(positions.windows(2).all(|pair| (pair[0].y - pair[1].y).abs() < 1.0));
    assert!(!has_text(&output, "RECOVERY CODE"));
    let recover = text(&output, "Recover Game");
    let position = recover.pos + recover.galley.size() * 0.5;
    let events = vec![
        egui::Event::PointerMoved(position),
        egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: default(),
        },
        egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: default(),
        },
    ];
    let time = context.input(|input| input.time) + 0.1;
    let mut clicked = frame(&mut app, &context, size, time, events);
    clicked.textures_delta.clear();
    assert!(matches!(
        app.world().resource::<NextState<AppState>>(),
        NextState::Pending(AppState::RecoverPlayer)
    ));
    app.world_mut().insert_resource(State::new(AppState::RecoverPlayer));
    app.world_mut().resource_mut::<MenuDraft>().recovery_code = "4T2H-6K9P-W3RX-Y7CZ".into();
    let output = settled(&mut app, &context, size, "menu-recover");
    text(&output, "GAME CODE");
    text(&output, "RECOVERY CODE");
    assert!(!has_text(&output, "4T2H-6K9P-W3RX-Y7CZ"), "Recovery input stays masked");
}

#[test]
fn shared_pages_and_game_overlay_keep_reference_navigation_and_code_positions() {
    let size = egui::vec2(1600.0, 900.0);
    let (mut app, context) = menu_app(AppState::MainMenu);
    let output = settled(&mut app, &context, size, "menu-main");
    let mut actions = vec!["New Game", "Join Game", "Resume Game"];
    if cfg!(debug_assertions) {
        actions.push("Local Practice");
    }
    actions.extend(["Settings", "Quit"]);
    assert!(actions
        .windows(2)
        .all(|pair| text(&output, pair[0]).pos.y < text(&output, pair[1]).pos.y));
    for (state, title, labels, name) in [
        (AppState::CreateGame, "Create Game", vec!["PLAYER NAME", "Back"], "menu-create"),
        (
            AppState::JoinGame,
            "Join Game",
            vec!["PLAYER NAME", "GAME CODE", "Back", "Join"],
            "menu-join",
        ),
        (
            AppState::Settings,
            "Settings",
            vec!["AUDIO", "Muted", "Effects", "Music", "Back"],
            "menu-settings",
        ),
        (
            AppState::GameSettings,
            "Settings",
            vec!["AUDIO", "Muted", "Effects", "Music", "Back"],
            "menu-game-settings",
        ),
    ] {
        app.world_mut().insert_resource(State::new(state));
        let output = settled(&mut app, &context, size, name);
        text(&output, title);
        for label in labels {
            text(&output, label);
        }
    }
    app.world_mut().insert_resource(State::new(AppState::ResumeGame));
    app.world_mut().resource_mut::<online::OnlineClient>().games = vec![GameSummary {
        id: "saved".into(),
        code: "4T2H6K".into(),
        status: "active".into(),
        saved_at: "2026-10-06T12:00:00Z".into(),
        display_name: "Mavs".into(),
        player_color: 0,
        turn: 12,
        player_count: 2,
    }];
    let output = settled(&mut app, &context, size, "menu-resume-saved");
    text(&output, "4T2H6K");
    assert!(!has_text(&output, "No games linked to this device"));
    lobby_fixture(&mut app, 0, 2);
    app.world_mut().resource_mut::<online::OnlineClient>().record.as_mut().unwrap().status =
        "active".into();
    app.world_mut().insert_resource(ActiveGame::Online);
    app.world_mut().insert_resource(State::new(AppState::GameMenu));
    for (name, size) in [("desktop", size), ("compact", egui::vec2(360.0, 640.0))] {
        let output = settled(&mut app, &context, size, &format!("menu-game-{name}"));
        for pair in ["Continue", "Save Game", "Settings", "Return to Main Menu"].windows(2) {
            assert!(text(&output, pair[0]).pos.y < text(&output, pair[1]).pos.y);
        }
        let code = text(&output, "GAME CODE");
        let recovery = text(&output, "RECOVERY CODE");
        assert!(code.pos.x < size.x * 0.25 && code.pos.y < recovery.pos.y);
        assert!(!has_text(&output, "Created by Mavs"));
        assert!(recovery.pos.y < size.y);
    }
}
