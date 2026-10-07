//! Replay real campaign commands through the same contract used by all four seats.
use super::*;
use crate::app::campaign_events::CivicEvent;
use crate::game::economy::*;
use crate::game::military::*;
use crate::game::politics::espionage::SpyAssignment;
use crate::game::politics::senate::SenatorAction;
use crate::map::EdictLevel;

fn interaction_trail() -> Value {
    let mut state = fixture(4);
    let homes: Vec<_> = (0..4)
        .map(|player| {
            state.campaign.economy.provinces.iter().position(|p| p.owner == Some(player)).unwrap()
        })
        .collect();
    let npc = state
        .campaign
        .economy
        .provinces
        .iter()
        .position(|p| p.owner.is_none() && p.name != "Rome")
        .unwrap();
    // Fixed, adjacent test provinces make costs and routes independent of random starts.
    for a in [homes[0], homes[1], npc] {
        for b in [homes[0], homes[1], npc] {
            if a != b && !state.campaign.graph[a].neighbors.contains(&b) {
                state.campaign.graph[a].neighbors.push(b);
                state.campaign.economy.adjacency[a].push(b);
            }
        }
    }
    for wallet in &mut state.campaign.economy.players {
        wallet.coin = 100_000.;
        wallet.influence = 100_000.;
        wallet.resources = [10_000.; 3];
        wallet.storage = [10_000.; 3];
    }
    state.campaign.pull_wallets();
    for _ in 0..3 {
        state
            .campaign
            .military
            .seed_unit(homes[1], ForceOwner::Player(1), UnitType::HeavyInfantry)
            .unwrap();
    }
    let troops =
        state.campaign.military.provinces[homes[1]].forces.get_mut(&ForceOwner::Player(1)).unwrap();
    troops[0].current_manpower = 2.;
    troops[1].current_manpower = 3.;
    let seats = state.campaign.senate.senators.len();
    state.campaign.senate.grant_practice_support(0, seats);
    state.campaign.reconcile_provinces();
    state.campaign.notify_rank_opportunities();
    let before = serde_json::to_value(&state).unwrap();
    let mut steps = Vec::new();
    let mut step = |name: &str, player: usize, action: &mut dyn FnMut(&mut campaign::Campaign)| {
        let old = serde_json::to_value(&state).unwrap();
        action(&mut state.campaign);
        state.campaign.reconcile_provinces();
        state.campaign.notify_rank_opportunities();
        let after = serde_json::to_value(&state).unwrap();
        let changes = patch::changes(&old, &after);
        assert!(!changes.is_empty(), "{name} must exercise a real state change");
        steps.push(json!({"name":name,"player":player,"changes":changes,"after":after}));
    };
    step("national edicts", 1, &mut |c| {
        let mut governance = c.governance_for(1);
        governance.food_rations = EdictLevel::High;
        governance.slave_labor = EdictLevel::High;
        governance.army_wages = EdictLevel::High;
        c.set_governance(1, governance);
    });
    step("province policies", 1, &mut |c| {
        let p = &mut c.economy.provinces[homes[1]];
        p.policies.focus = ResourceFocus::Metal;
        p.policies.migration = MigrationPolicy::Closed;
        p.policies.construction = ConstructionPace::Urgent;
    });
    step("construction", 1, &mut |c| {
        c.economy.start_building(1, homes[1], BuildingType::Road).unwrap();
    });
    step("queued construction", 1, &mut |c| {
        c.economy.start_building(1, homes[1], BuildingType::Road).unwrap();
    });
    step("cancel queued construction", 1, &mut |c| {
        c.economy.cancel_queued_construction(1, homes[1], 0).unwrap();
    });
    step("cancel construction", 1, &mut |c| {
        c.economy.cancel_construction(1, homes[1]).unwrap();
    });
    step("open market sell", 1, &mut |c| {
        c.economy.exchange_open_market(1, 1, MarketSide::Sell, 10.).unwrap();
        c.pull_wallets();
    });
    step("open market buy", 1, &mut |c| {
        c.economy.exchange_open_market(1, 1, MarketSide::Buy, 5.).unwrap();
        c.pull_wallets();
    });
    for (name, action) in [
        ("recruit", campaign_military::MilitaryUiAction::Recruit(UnitType::LightInfantry)),
        ("queued recruit", campaign_military::MilitaryUiAction::Recruit(UnitType::LightInfantry)),
        ("cancel queued recruit", campaign_military::MilitaryUiAction::CancelQueuedRecruitment(0)),
        ("cancel recruit", campaign_military::MilitaryUiAction::CancelRecruitment),
        (
            "battle plan",
            campaign_military::MilitaryUiAction::SavePlan(BattlePlan {
                tactic: CombatTactic::Phalanx,
                ..Default::default()
            }),
        ),
        ("merge army", campaign_military::MilitaryUiAction::MergeArmy),
    ] {
        let mut action = Some(action);
        step(name, 1, &mut |c| {
            assert_eq!(
                campaign_panel::apply_military_action(c, homes[1], 1, action.take().unwrap()),
                ""
            );
        });
    }
    step("grant province access", 0, &mut |c| {
        c.set_province_access(homes[0], 0, 1, true).unwrap();
    });
    step("revoke province access", 0, &mut |c| {
        c.set_province_access(homes[0], 0, 1, false).unwrap();
    });
    for event in CivicEvent::ALL {
        step(&format!("event {event:?}"), 1, &mut |c| {
            c.hold_event(1, event).unwrap();
        });
    }
    step("noble bribe", 1, &mut |c| {
        c.bribe_nobles(1, npc).unwrap();
    });
    step("diplomatic insult", 1, &mut |c| {
        c.send_insult(1, npc).unwrap();
    });
    for assignment in [
        SpyAssignment::GainControl,
        SpyAssignment::ImproveRelations,
        SpyAssignment::DiscoverScandals,
        SpyAssignment::UndermineOpponents,
        SpyAssignment::SupportRevolt,
        SpyAssignment::DiscreditRivals,
    ] {
        step(&format!("spy {assignment:?}"), 1, &mut |c| {
            let distance = c.distance(1, npc);
            c.espionage
                .deploy_assignment(
                    1,
                    npc,
                    &mut c.actors,
                    &c.politics,
                    &c.espionage_config,
                    assignment,
                    distance,
                )
                .unwrap();
            c.push_wallets();
        });
        step("spy recall", 1, &mut |c| {
            c.espionage.recall(1, npc, c.economy.month, &c.espionage_config).unwrap();
        });
        step("spy flee", 1, &mut |c| {
            c.flee_spy(1, npc).unwrap();
        });
    }
    for (seat, action) in SenatorAction::ALL.into_iter().enumerate() {
        let mut ongoing = false;
        step(&format!("senator {action:?}"), 1, &mut |c| {
            c.senate.act_on_senator(1, seat, action, &mut c.actors, &c.senate_config).unwrap();
            ongoing = c.senate.senators[seat].arrangement(1).is_some();
            c.push_wallets();
        });
        if ongoing {
            step("cancel senator arrangement", 1, &mut |c| {
                c.senate.cancel_senator_action(1, seat).unwrap();
            });
        }
    }
    let mut trade = 0;
    step("propose trade", 1, &mut |c| {
        trade = c
            .propose_national_trade(TradeAgreement::new(
                TradeParty::Player(1),
                TradeParty::Player(0),
                TradeBundle {
                    coin: 5.,
                    ..Default::default()
                },
                TradeBundle {
                    coin: 5.,
                    ..Default::default()
                },
                TradeFrequency::Monthly,
            ))
            .unwrap();
    });
    step("accept trade", 0, &mut |c| {
        c.accept_national_trade(0, trade).unwrap();
    });
    step("trade cancellation notice", 1, &mut |c| {
        c.end_trade(1, trade, true).unwrap();
    });
    step("trade cancellation", 1, &mut |c| {
        c.end_trade(1, trade, false).unwrap();
    });
    step("declare war", 1, &mut |c| {
        c.declare_hostility(1, homes[0]);
    });
    step("peace", 1, &mut |c| {
        c.wars[0][1] = false;
        c.wars[1][0] = false;
    });
    step("allow allied movement", 0, &mut |c| {
        c.set_province_access(homes[0], 0, 1, true).unwrap();
    });
    step("move army", 1, &mut |c| {
        let units: Vec<_> = c.military.provinces[homes[1]].forces[&ForceOwner::Player(1)]
            .iter()
            .map(|u| u.id)
            .collect();
        c.order_army(
            homes[1],
            1,
            homes[0],
            &units,
            &[homes[0]],
            BattlePlan::default(),
            ArmyOrderKind::Move,
        )
        .unwrap();
    });
    step("change movement plan", 1, &mut |c| {
        let order = c.military.movements.last().unwrap().id;
        c.military
            .set_movement_plan(
                order,
                ForceOwner::Player(1),
                BattlePlan {
                    tactic: CombatTactic::Envelopment,
                    ..Default::default()
                },
            )
            .unwrap();
    });
    step("monthly simulation", 0, &mut |c| {
        c.advance_live_month();
    });
    json!({"before":before,"steps":steps})
}

#[test]
fn interleaved_save_receipts_preserve_actions_made_while_waiting() {
    for ack_revision in [2, 3] {
        let before = serde_json::to_value(fixture(2)).unwrap();
        let mut world = reconnect_world();
        let mut client = OnlineClient::default();
        let mut record: GameRecord = serde_json::from_value(reconnect_record()).unwrap();
        record.state = Some(before.clone());
        enter(&mut world, &mut client, record).unwrap();
        world.resource_mut::<campaign::Campaign>().economy.players[1].coin -= 1.;
        let sent = snapshot(&world).unwrap();
        let home = world
            .resource::<campaign::Campaign>()
            .economy
            .provinces
            .iter()
            .position(|p| p.owner == Some(1))
            .unwrap();
        world.resource_mut::<campaign::Campaign>().economy.provinces[home].policies.focus =
            ResourceFocus::Stone;
        let current = snapshot(&world).unwrap();
        let mut remote = sent.clone();
        remote["campaign"]["economy"]["players"][0]["coin"] = json!(1234.);
        let (sender, receiver) = mpsc::channel();
        sender
            .send(Response {
                session: None,
                result: Ok(json!({"revision":ack_revision})),
            })
            .unwrap();
        client.transport = Err("Offline test transport".into());
        client.pending = Some(Pending {
            kind: Operation::Save,
            payload: json!({}),
            receiver: Mutex::new(receiver),
            sent: Some(sent.clone()),
        });
        world.insert_resource(client);
        update(&mut world);
        assert!(
            snapshot(&world).unwrap() == current,
            "A save receipt must preserve newer local commands"
        );
        world.resource_scope(|world, mut client: Mut<OnlineClient>| {
            let events = if ack_revision == 2 {
                vec![json!({"revision":3,"changes":patch::changes(&sent,&remote)})]
            } else {
                let mut other = before.clone();
                other["campaign"]["economy"]["players"][0]["coin"] = json!(1234.);
                vec![
                    json!({"revision":2,"changes":patch::changes(&before,&other)}),
                    json!({"revision":3,"changes":patch::changes(&other,&remote)}),
                ]
            };
            reconcile(
                world,
                &mut client,
                serde_json::from_value(json!({
                    "revision":3,"status":"active","roster_token":"roster","members":null,
                    "events":events,"state":null
                }))
                .unwrap(),
            )
            .unwrap();
            assert!(client.base.as_ref() == Some(&remote));
        });
        let mut expected = remote;
        expected["campaign"]["economy"]["provinces"][home]["policies"]["focus"] = json!("Stone");
        assert!(
            snapshot(&world).unwrap() == expected,
            "Committed remote commands and pending local commands must both survive"
        );
    }
}

#[test]
fn defeated_online_host_keeps_simulating_and_spectating_preserves_shared_pause() {
    use bevy::ecs::system::RunSystemOnce;
    let mut world = reconnect_world();
    world.init_resource::<TerminalPresentation>();
    world.init_resource::<GovernancePanelOpen>();
    world.init_resource::<ProvincePanelOpen>();
    world.init_resource::<campaign_panel::CampaignUi>();
    let mut client = OnlineClient::default();
    let mut record: GameRecord = serde_json::from_value(reconnect_record()).unwrap();
    record.player = 0;
    record.state = Some(serde_json::to_value(fixture(2)).unwrap());
    enter(&mut world, &mut client, record).unwrap();
    world.insert_resource(client);
    world.resource_mut::<campaign::Campaign>().defeated[0] = true;
    world.resource_mut::<TerminalPresentation>().outcome = Some(TerminalOutcome::Defeat);
    world.insert_resource(State::new(AppState::EndGame));
    world.run_system_once(campaign::sync_campaign).unwrap();
    assert!(
        !world.resource::<GamePaused>().0,
        "An individual online defeat cannot pause other players"
    );
    let month = world.resource::<campaign::Campaign>().economy.month;
    world.resource_mut::<Time>().advance_by(std::time::Duration::from_secs(3));
    world.run_system_once(advance_game_time).unwrap();
    assert_eq!(world.resource::<campaign::Campaign>().economy.month, month + 1);
    world.resource_mut::<GamePaused>().0 = true;
    world.resource_mut::<TerminalPresentation>().spectating = true;
    world.run_system_once(prepare_spectator).unwrap();
    assert!(world.resource::<GamePaused>().0, "Spectate cannot overwrite the host's shared pause");
}

#[test]
fn all_campaign_interactions_converge_for_four_players() {
    use bevy::ecs::system::RunSystemOnce;
    let trail = interaction_trail();
    let mut clients = Vec::new();
    for player in 0..4 {
        let mut world = reconnect_world();
        world.init_resource::<TerminalPresentation>();
        let mut client = OnlineClient::default();
        let mut record: GameRecord = serde_json::from_value(reconnect_record()).unwrap();
        record.player = player;
        record.members = (0..4)
            .map(|player| Member {
                player,
                display_name: format!("Player {player}"),
                color: player,
                connected: true,
            })
            .collect();
        record.state = Some(trail["before"].clone());
        enter(&mut world, &mut client, record).unwrap();
        clients.push((world, client));
    }
    for (index, step) in trail["steps"].as_array().unwrap().iter().enumerate() {
        let changes: Vec<patch::Change> = serde_json::from_value(step["changes"].clone()).unwrap();
        let replay: Vec<_> = changes
            .into_iter()
            .map(|mut change| {
                change.before = None;
                change
            })
            .collect();
        for (player, (world, client)) in clients.iter_mut().enumerate() {
            reconcile(
                world,
                client,
                serde_json::from_value(json!({
                    "revision":index+2,"status":"active","roster_token":"roster","members":null,
                    "events":[{"revision":index+2,"changes":replay}],"state":null
                }))
                .unwrap(),
            )
            .unwrap();
            assert!(
                snapshot(world).unwrap() == step["after"],
                "{} differs for player {player}",
                step["name"]
            );
            world.run_system_once(campaign::sync_campaign).unwrap();
            assert!(
                snapshot(world).unwrap() == step["after"],
                "HUD projection changed {} for player {player}",
                step["name"]
            );
        }
    }
}

pub(super) fn write_sql_interactions(folder: &std::path::Path) {
    std::fs::write(
        folder.join("interactions.json"),
        serde_json::to_vec(&interaction_trail()).unwrap(),
    )
    .unwrap();
    let mut battle = battle_fixture();
    let before = serde_json::to_value(&battle).unwrap();
    battle.campaign.advance_live_combat();
    let after = serde_json::to_value(&battle).unwrap();
    std::fs::write(
        folder.join("battle.json"),
        serde_json::to_vec(&json!({
            "before":before,"after":after,"changes":patch::changes(&before,&after)
        }))
        .unwrap(),
    )
    .unwrap();
}
