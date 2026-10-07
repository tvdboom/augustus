use super::*;

#[test]
fn every_database_operation_announces_failures_without_repeating_background_errors() {
    for operation in [
        Operation::Create,
        Operation::Join,
        Operation::Open,
        Operation::LoadStarted,
        Operation::Recover,
        Operation::Profile,
        Operation::Start,
        Operation::Resume,
        Operation::List,
        Operation::Save,
        Operation::Sync,
        Operation::Leave,
    ] {
        let mut world = World::new();
        world.init_resource::<Time>();
        let (sender, receiver) = mpsc::channel();
        sender
            .send(Response {
                session: None,
                result: Err("Anonymous sign-ins are disabled".into()),
            })
            .unwrap();
        world.insert_resource(OnlineClient {
            pending: Some(Pending {
                kind: operation,
                payload: json!({}),
                receiver: Mutex::new(receiver),
                sent: None,
            }),
            ..Default::default()
        });
        update(&mut world);
        let mut client = world.resource_mut::<OnlineClient>();
        assert_eq!(client.take_errors().collect::<Vec<_>>(), ["Anonymous sign-ins are disabled"]);
        client.report_error("Anonymous sign-ins are disabled");
        assert_eq!(client.take_errors().count(), 0, "Automatic retries do not flood notifications");
        if operation == Operation::Save {
            client.save();
        } else {
            client.request(operation, json!({}));
        }
        client.report_error("Anonymous sign-ins are disabled");
        assert_eq!(client.take_errors().count(), 1, "An explicit retry announces its failure");
        assert!(client.error.is_some(), "Displaying an error preserves the online failure state");
    }
}

#[test]
fn database_configuration_errors_and_state_conflicts_use_the_same_notifications() {
    let mut world = World::new();
    world.init_resource::<Time>();
    let mut client = OnlineClient {
        transport: Err("Could not initialize the online connection.".into()),
        ..Default::default()
    };
    client.request(Operation::List, json!({}));
    world.insert_resource(client);
    update(&mut world);
    assert_eq!(
        world.resource_mut::<OnlineClient>().take_errors().collect::<Vec<_>>(),
        ["Could not initialize the online connection."]
    );
    let (sender, receiver) = mpsc::channel();
    sender
        .send(Response {
            session: None,
            result: Err("State conflict".into()),
        })
        .unwrap();
    world.resource_mut::<OnlineClient>().pending = Some(Pending {
        kind: Operation::Save,
        payload: json!({}),
        receiver: Mutex::new(receiver),
        sent: None,
    });
    update(&mut world);
    assert_eq!(
        world.resource_mut::<OnlineClient>().take_errors().collect::<Vec<_>>(),
        ["Another player changed the game. Please retry your last action after syncing."]
    );
}

fn fixture(count: usize) -> Snapshot {
    let mut ownership = ProvinceOwnership::default();
    ownership.start_game(&PLAYER_COLORS[..count]);
    let mut campaign = campaign::Campaign::default();
    campaign.start(&ownership, count);
    Snapshot {
        version: 1,
        campaign,
        clock: GameClock::default(),
        paused: false,
    }
}

fn battle_fixture() -> Snapshot {
    use crate::game::military::{ForceOwner, MilitaryTerrain, UnitType};
    let mut snapshot = fixture(2);
    let campaign = &mut snapshot.campaign;
    let province = campaign.economy.provinces.iter().position(|p| p.owner == Some(0)).unwrap();
    let owners = [ForceOwner::Player(0), ForceOwner::Player(1)];
    campaign.wars[0][1] = true;
    campaign.wars[1][0] = true;
    for owner in owners {
        campaign.military.seed_unit(province, owner, UnitType::HeavyInfantry).unwrap();
    }
    campaign
        .military
        .start_battle(
            province,
            &owners[..1],
            &owners[1..],
            Some(0),
            None,
            MilitaryTerrain::Plains,
            0,
            177,
        )
        .unwrap();
    snapshot
}

#[test]
fn host_resolves_combat_and_guests_install_the_committed_round_without_rerolling() {
    use bevy::ecs::system::RunSystemOnce;
    let before = serde_json::to_value(battle_fixture()).unwrap();
    let mut host = reconnect_world();
    let mut client = OnlineClient::default();
    let mut record: GameRecord = serde_json::from_value(reconnect_record()).unwrap();
    record.player = 0;
    record.state = Some(before.clone());
    enter(&mut host, &mut client, record.clone()).unwrap();
    host.insert_resource(client);
    host.insert_resource(State::new(AppState::Map));
    host.resource_mut::<Time>().advance_by(std::time::Duration::from_secs_f32(0.75));
    host.run_system_once(advance_game_time).unwrap();
    let after = snapshot(&host).unwrap();
    assert_eq!(after["campaign"]["military"]["battles"][0]["round"], 1);
    assert!(
        host.resource::<OnlineClient>().save_requested,
        "Publish the completed round immediately"
    );

    let mut guest = reconnect_world();
    let mut client = OnlineClient::default();
    record.player = 1;
    enter(&mut guest, &mut client, record).unwrap();
    guest.insert_resource(client);
    guest.insert_resource(State::new(AppState::Map));
    guest.resource_mut::<Time>().advance_by(std::time::Duration::from_secs(3));
    guest.run_system_once(advance_game_time).unwrap();
    assert_eq!(snapshot(&guest).unwrap(), before, "Guests never resolve their own dice or damage");
    let replay: Vec<_> = patch::changes(&before, &after)
        .into_iter()
        .map(|mut change| {
            change.before = None;
            change
        })
        .collect();
    guest.resource_scope(|world, mut client: Mut<OnlineClient>| {
        reconcile(
            world,
            &mut client,
            serde_json::from_value(json!({
                "revision":2, "status":"active", "roster_token":"roster", "members":null,
                "events":[{"revision":2,"changes":replay}], "state":null
            }))
            .unwrap(),
        )
        .unwrap();
    });
    assert_eq!(
        snapshot(&guest).unwrap(),
        after,
        "Replay preserves every authoritative combat field"
    );
    guest.run_system_once(advance_game_time).unwrap();
    assert_eq!(snapshot(&guest).unwrap(), after, "Viewing a committed round cannot reroll it");

    let mut resumed: Snapshot = serde_json::from_value(after).unwrap();
    let mut original: Snapshot = serde_json::from_value(snapshot(&host).unwrap()).unwrap();
    while original.campaign.military.history.is_empty() {
        original.campaign.advance_live_combat();
        resumed.campaign.advance_live_combat();
        assert_eq!(
            serde_json::to_value(&resumed).unwrap(),
            serde_json::to_value(&original).unwrap()
        );
    }
    assert!(!original.campaign.military.history[0].round_history.is_empty());
}

#[test]
fn snapshots_restore_every_rule_and_continue_the_same_seeded_simulation() {
    for count in 1..=4 {
        let mut original = fixture(count);
        original.campaign.province_access.insert((0, 0, count - 1), true);
        for _ in 0..4 {
            original.campaign.advance_month();
        }
        let json = serde_json::to_string(&original).unwrap();
        let mut restored: Snapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(
            serde_json::to_value(&restored).unwrap(),
            serde_json::to_value(&original).unwrap()
        );
        for _ in 0..6 {
            original.campaign.advance_month();
            restored.campaign.advance_month();
            assert_eq!(
                serde_json::to_value(&restored).unwrap(),
                serde_json::to_value(&original).unwrap()
            );
        }
    }
}

#[test]
fn new_snapshots_install_with_the_protected_rome_province() {
    for count in 1..=4 {
        let snapshot = fixture(count);
        let mut world = World::new();
        world.init_resource::<ProvinceOwnership>();
        world.init_resource::<GamePaused>();
        install(&mut world, serde_json::to_value(&snapshot).unwrap()).unwrap();
        assert_eq!(
            world.resource::<campaign::Campaign>().economy.provinces.last().unwrap().name,
            "Rome"
        );
    }
}

#[test]
fn cloud_campaign_map_stays_visible_under_menus_and_results() {
    use bevy::ecs::system::RunSystemOnce;
    let mut world = World::new();
    world.insert_resource(ActiveGame::Online);
    world.init_resource::<LoadingSequence>();
    for state in [AppState::GameMenu, AppState::GameSettings, AppState::EndGame] {
        world.insert_resource(State::new(state));
        assert!(world.run_system_once(map_visible).unwrap());
    }
}

#[test]
fn menu_and_hud_system_parameters_do_not_alias_online_resources() {
    use bevy::ecs::system::{IntoSystem, System};
    let mut world = World::new();
    IntoSystem::into_system(handle_escape).initialize(&mut world);
    IntoSystem::into_system(draw_map_hud).initialize(&mut world);
    IntoSystem::into_system(draw_menu).initialize(&mut world);
    IntoSystem::into_system(spectator::draw_end_game).initialize(&mut world);
}

#[test]
fn entering_a_saved_game_pins_the_caller_and_keeps_one_snapshot_copy() {
    let mut world = World::new();
    world.init_resource::<LocalPractice>();
    world.init_resource::<ProvinceOwnership>();
    world.init_resource::<GamePaused>();
    world.init_resource::<LoadingSequence>();
    world.init_resource::<NextState<AppState>>();
    let mut client = OnlineClient::default();
    let record = GameRecord {
        id: "g".into(),
        code: "AAAA-BBBB".into(),
        status: "active".into(),
        single_player: false,
        revision: 1,
        state: Some(serde_json::to_value(fixture(2)).unwrap()),
        members: vec![
            Member {
                player: 0,
                display_name: "Host".into(),
                color: 0,
                connected: true,
            },
            Member {
                player: 1,
                display_name: "Guest".into(),
                color: 2,
                connected: true,
            },
        ],
        player: 1,
        recovery_code: "A".repeat(32),
        roster_token: "t".into(),
    };
    enter(&mut world, &mut client, record.clone()).unwrap();
    assert_eq!(world.resource::<LocalPractice>().active_player, 1);
    assert_eq!(world.resource::<LocalPractice>().players[1].name, "Guest");
    assert!(client.base.is_some());
    assert!(client.record.as_ref().unwrap().state.is_none());
    let mut invalid = record;
    invalid.members[1].player = 3;
    assert!(enter(&mut world, &mut client, invalid).is_err());
}

fn reconnect_world() -> World {
    let mut world = World::new();
    world.init_resource::<Time>();
    world.init_resource::<LocalPractice>();
    world.init_resource::<ProvinceOwnership>();
    world.init_resource::<GamePaused>();
    world.init_resource::<LoadingSequence>();
    world.init_resource::<LobbyPreview>();
    world.init_resource::<NextState<AppState>>();
    world.init_resource::<toasts::ToastQueue>();
    world.insert_resource(State::new(AppState::Lobby));
    world
}

fn reconnect_record() -> Value {
    json!({"id":"fixture", "code":"4T2H6K", "status":"active", "single_player":false,
        "revision":1, "state":fixture(2), "resume_generation":7,
        "members":[{"player":0,"display_name":"Host","color":0,"connected":true},
            {"player":1,"display_name":"Guest","color":1,"connected":true}],
        "player":1, "recovery_code":"4T2H-6K9P-W3RX-Y7CZ", "roster_token":"roster"})
}

#[test]
fn foreground_actions_queue_behind_polling_without_a_second_http_request() {
    let mut world = reconnect_world();
    let (_sender, receiver) = mpsc::channel();
    let mut record: GameRecord = serde_json::from_value(reconnect_record()).unwrap();
    record.status = "lobby".into();
    record.player = 0;
    record.revision = 0;
    record.state = None;
    record.members.truncate(1);
    let mut client = OnlineClient {
        record: Some(record),
        pending: Some(Pending {
            kind: Operation::Sync,
            payload: json!({}),
            receiver: Mutex::new(receiver),
            sent: None,
        }),
        ..Default::default()
    };
    assert!(client.busy());
    assert!(!client.foreground_busy());
    assert!(lobby_primary_enabled(&client), "A solo host can start while presence is polling");
    client.start().unwrap();
    assert!(client.foreground_busy());
    assert!(!lobby_primary_enabled(&client));
    let (kind, payload) = client.queue.front().unwrap();
    assert!(*kind == Operation::Start);
    assert_eq!(payload["p_state"]["campaign"]["actors"].as_array().unwrap().len(), 1);
    assert_eq!(payload["p_roster_token"], "roster");
    client.request(Operation::Profile, json!({"p_name":"Host","p_color":2}));
    assert_eq!(client.queue.len(), 1, "Do not overlap foreground writes");
    world.insert_resource(client);
    update(&mut world);
    let client = world.resource::<OnlineClient>();
    assert!(client.pending.as_ref().unwrap().kind == Operation::Sync);
    assert_eq!(client.queue.len(), 1, "Wait for the outstanding HTTP response before dispatch");
}

#[test]
fn lobby_start_waits_for_connections_and_host_authority() {
    let mut record: GameRecord = serde_json::from_value(reconnect_record()).unwrap();
    record.status = "lobby".into();
    record.player = 0;
    record.state = None;
    let mut client = OnlineClient {
        record: Some(record),
        ..Default::default()
    };
    assert!(lobby_primary_enabled(&client));
    client.record.as_mut().unwrap().members[1].connected = false;
    assert!(!lobby_primary_enabled(&client));
    client.record.as_mut().unwrap().members[1].connected = true;
    client.record.as_mut().unwrap().player = 1;
    assert!(!lobby_primary_enabled(&client));
}

fn complete_response(world: &mut World, kind: Operation, value: Value) {
    let (sender, receiver) = mpsc::channel();
    sender
        .send(Response {
            session: None,
            result: Ok(value),
        })
        .unwrap();
    {
        let mut client = world.resource_mut::<OnlineClient>();
        client.queue.clear();
        client.pending = Some(Pending {
            kind,
            payload: json!({}),
            receiver: Mutex::new(receiver),
            sent: None,
        });
    }
    update(world);
}

#[test]
fn saved_games_wait_for_the_host_and_guests_follow_a_successful_resume() {
    for kind in [Operation::Open, Operation::Recover] {
        let mut world = reconnect_world();
        world.init_resource::<OnlineClient>();
        complete_response(&mut world, kind, reconnect_record());
        assert!(matches!(
            world.resource::<NextState<AppState>>(),
            NextState::Pending(AppState::Lobby)
        ));
        assert!(world.resource::<OnlineClient>().reconnecting());
        assert_eq!(world.resource::<LobbyPreview>().code, "4T2H6K");
        world
            .resource_mut::<OnlineClient>()
            .report_error("The player roster changed. Please retry.");
        complete_response(
            &mut world,
            Operation::Sync,
            json!({"revision":1,"resume_generation":7,
            "status":"active","roster_token":"roster","members":null,"events":[],"state":null}),
        );
        assert!(
            world.resource::<OnlineClient>().error().is_some(),
            "Presence updates retain the menu error panel"
        );
        world.resource_scope(|world, mut client: Mut<OnlineClient>| {
            client.record.as_mut().unwrap().player = 0;
            assert!(!client.runs_time(), "The reconnect lobby must not advance the campaign");
            client.record.as_mut().unwrap().player = 1;
            let sync = |revision, generation| {
                serde_json::from_value(json!({
                    "revision":revision,"resume_generation":generation,"status":"active",
                    "roster_token":"roster","members":null,"events":[],"state":null
                }))
                .unwrap()
            };
            reconcile(world, &mut client, sync(1, 7)).unwrap();
            assert!(client.reconnecting(), "Old resume markers cannot release a new lobby");
            assert!(reconcile(world, &mut client, sync(2, 8)).is_err());
            assert_eq!(
                client.resume_generation, 7,
                "A failed delta must not consume the resume marker"
            );
            reconcile(world, &mut client, sync(1, 8)).unwrap();
            assert!(!client.reconnecting());
        });
        assert!(matches!(
            world.resource::<NextState<AppState>>(),
            NextState::Pending(AppState::Loading)
        ));
        assert_eq!(world.resource::<LoadingSequence>().destination, AppState::Map);
    }
}

#[test]
fn guests_enter_newly_started_games_without_a_second_resume_lobby() {
    let mut world = reconnect_world();
    let mut client = OnlineClient::default();
    let mut record: GameRecord = serde_json::from_value(reconnect_record()).unwrap();
    record.status = "lobby".into();
    record.state = None;
    record.revision = 0;
    enter(&mut world, &mut client, record).unwrap();
    let sync =
        serde_json::from_value(json!({"revision":1,"status":"active","roster_token":"roster",
        "members":null,"events":[],"state":null}))
        .unwrap();
    reconcile(&mut world, &mut client, sync).unwrap();
    assert!(matches!(client.queue.front(), Some((Operation::LoadStarted, _))));
    world.insert_resource(client);
    complete_response(&mut world, Operation::LoadStarted, reconnect_record());
    assert!(!world.resource::<OnlineClient>().reconnecting());
    assert!(matches!(
        world.resource::<NextState<AppState>>(),
        NextState::Pending(AppState::Loading)
    ));
}

#[test]
fn sql_fixtures_and_delta_budget() {
    let folder = std::path::Path::new("target/sql-verification");
    std::fs::create_dir_all(folder).unwrap();
    interactions::write_sql_interactions(folder);
    for (name, count) in [("single", 1), ("multi", 2)] {
        let mut snapshot = fixture(count);
        let before = serde_json::to_value(&snapshot).unwrap();
        std::fs::write(
            folder.join(format!("{name}.json")),
            serde_json::to_string(&snapshot).unwrap(),
        )
        .unwrap();
        snapshot.campaign.advance_month();
        let after = serde_json::to_value(&snapshot).unwrap();
        let deltas = patch::changes(&before, &after);
        assert_eq!(patch::apply(&before, &deltas, true).unwrap(), after);
        let snapshot_bytes = serde_json::to_vec(&after).unwrap().len();
        let replay: Vec<_> = deltas
            .into_iter()
            .map(|mut d| {
                d.before = None;
                d
            })
            .collect();
        let delta_bytes = serde_json::to_vec(&replay).unwrap().len();
        assert!(snapshot_bytes < 2_097_152);
        assert!(delta_bytes < snapshot_bytes / 2, "delta {delta_bytes}, snapshot {snapshot_bytes}");
        println!("{name}: snapshot {snapshot_bytes} bytes, monthly replay {delta_bytes} bytes");
    }
}

#[path = "multiplayer_interactions.rs"]
mod interactions;

#[test]
fn only_connected_host_advances_time() {
    let mut client = OnlineClient {
        record: Some(GameRecord {
            id: "g".into(),
            code: "AAAA-BBBB".into(),
            status: "active".into(),
            single_player: false,
            revision: 1,
            state: None,
            members: vec![
                Member {
                    player: 0,
                    display_name: "Host".into(),
                    color: 0,
                    connected: true,
                },
                Member {
                    player: 1,
                    display_name: "Guest".into(),
                    color: 1,
                    connected: true,
                },
            ],
            player: 0,
            recovery_code: "A".repeat(32),
            roster_token: "t".into(),
        }),
        connected: true,
        ..Default::default()
    };
    assert!(client.runs_time());
    client.record.as_mut().unwrap().player = 1;
    assert!(!client.runs_time());
    client.record.as_mut().unwrap().player = 0;
    client.record.as_mut().unwrap().members[1].connected = false;
    assert!(!client.runs_time());
    client.base = Some(json!({"campaign":{"defeated":[false,true]}}));
    assert!(client.runs_time(), "eliminated players do not block the remaining campaign");
}

#[test]
fn loading_a_cloud_game_preserves_campaign_and_clock() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let mut snapshot = fixture(1);
    snapshot.campaign.advance_month();
    snapshot.clock.year = 82;
    snapshot.clock.month = 7;
    let expected = serde_json::to_value(&snapshot.campaign).unwrap();
    app.insert_resource(snapshot.campaign)
        .insert_resource(snapshot.clock)
        .insert_resource(ActiveGame::Online)
        .insert_resource(LocalPractice {
            players: vec![PracticePlayer {
                name: "Solo".into(),
                color_index: 0,
                rank: 0,
                main_province: None,
            }],
            ..Default::default()
        })
        .init_resource::<ProvinceOwnership>()
        .init_resource::<GamePaused>()
        .init_resource::<HudResources>()
        .init_resource::<GovernancePanelOpen>()
        .init_resource::<ProvincePanelOpen>()
        .init_resource::<toasts::ToastQueue>()
        .init_resource::<toasts::WarningWatch>()
        .init_resource::<campaign_panel::CampaignUi>()
        .init_resource::<TerminalPresentation>()
        .add_systems(Update, reset_game_time);
    app.update();
    assert_eq!(
        serde_json::to_value(app.world().resource::<campaign::Campaign>()).unwrap(),
        expected
    );
    assert_eq!(app.world().resource::<GameClock>().year, 82);
    assert_eq!(app.world().resource::<GameClock>().month, 7);
}
