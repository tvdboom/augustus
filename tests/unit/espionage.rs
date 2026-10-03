use super::*;

#[test]
fn happiness_missions_roll_zero_to_three_and_keep_their_target_classes() {
    let config = EspionageConfig {
        detection_range: [0.0; 2],
        npc_generation_chance: 0.0,
        ..Default::default()
    };
    for assignment in [SpyAssignment::UndermineOpponents, SpyAssignment::SupportRevolt] {
        let mut state = EspionageState::new(29);
        let mut players = vec![
            PoliticalPlayer {
                coin: 500.0,
                influence: 100.0,
                ..Default::default()
            };
            2
        ];
        let mut politics = vec![ProvincePolitics::owned(2, 1)];
        let province = SpyProvince {
            owner: Some(1),
            noble_happiness: 50.0,
            conditions: vec![],
        };
        state
            .deploy_assignment(0, 0, &mut players, &politics, &config, assignment, Some(1))
            .unwrap();
        let mut positive = 0;
        for month in 1..=24 {
            let events = state.advance_month(
                month,
                &mut players,
                std::slice::from_ref(&province),
                &mut politics,
                &config,
            );
            for event in events {
                if let EspionageEvent::PopulationUndermined(_, _, class, points) = event {
                    assert!((1.0..=3.0).contains(&points));
                    assert!(if assignment == SpyAssignment::SupportRevolt {
                        class == 3
                    } else {
                        class < 3
                    });
                    positive += 1;
                }
            }
        }
        assert!(positive > 0 && positive < 24, "the 0–3 roll includes idle months");
    }
}

#[test]
fn a_rival_network_can_expose_noble_bribery_as_player_evidence() {
    let mut state = EspionageState::new(41);
    let config = EspionageConfig::default();
    let politics = vec![ProvincePolitics::owned(3, 0)];
    let mut players = vec![
        PoliticalPlayer {
            coin: 100.0,
            influence: 100.0,
            ..Default::default()
        };
        3
    ];
    state
        .deploy_assignment(
            2,
            0,
            &mut players,
            &politics,
            &config,
            SpyAssignment::DiscoverScandals,
            Some(1),
        )
        .unwrap();
    let mut exposed = 0;
    for month in 0..100 {
        if let Some((observer, id)) = state.observe_noble_bribe(1, 0, month, &config) {
            exposed += 1;
            assert_eq!(observer, 2);
            let scandal = state.scandals.iter().find(|scandal| scandal.id == id).unwrap();
            assert_eq!(scandal.target, ScandalTarget::Player(1));
            assert_eq!(scandal.kind, ScandalKind::NobleBribery);
        }
    }
    assert!((15..=45).contains(&exposed));
}

#[test]
fn a_spy_in_the_bribers_land_cannot_expose_a_bribe_elsewhere() {
    let mut state = EspionageState::new(41);
    let config = EspionageConfig::default();
    let politics = vec![ProvincePolitics::owned(3, 0), ProvincePolitics::owned(3, 1)];
    let mut players = vec![
        PoliticalPlayer {
            coin: 100.0,
            influence: 100.0,
            ..Default::default()
        };
        3
    ];
    state
        .deploy_assignment(
            2,
            1,
            &mut players,
            &politics,
            &config,
            SpyAssignment::DiscoverScandals,
            Some(1),
        )
        .unwrap();
    for month in 0..100 {
        assert_eq!(state.observe_noble_bribe(1, 0, month, &config), None);
    }
    assert!(state.scandals.is_empty());
}

#[test]
fn orderly_recall_waits_six_months_and_flee_checks_once() {
    let mut state = EspionageState::new(17);
    let mut config = EspionageConfig {
        detection_range: [0.0; 2],
        npc_generation_chance: 0.0,
        ..Default::default()
    };
    let mut players = vec![PoliticalPlayer {
        coin: 100.0,
        influence: 100.0,
        ..Default::default()
    }];
    let mut politics = vec![ProvincePolitics::independent(1)];
    let target = SpyProvince {
        owner: None,
        noble_happiness: 50.0,
        conditions: vec![],
    };
    state
        .deploy_assignment(
            0,
            0,
            &mut players,
            &politics,
            &config,
            SpyAssignment::DiscoverScandals,
            Some(1),
        )
        .unwrap();
    assert_eq!(state.recall(0, 0, 0, &config), Ok(6));
    assert_eq!(state.recall(0, 0, 0, &config), Err(PoliticalError::AlreadyUsed));
    for month in 1..6 {
        state.advance_month(
            month,
            &mut players,
            std::slice::from_ref(&target),
            &mut politics,
            &config,
        );
        assert_eq!(state.missions.len(), 1);
    }
    let events =
        state.advance_month(6, &mut players, std::slice::from_ref(&target), &mut politics, &config);
    assert!(events.iter().any(|event| matches!(event, EspionageEvent::Recalled(0, 0))));
    assert!(state.missions.is_empty());
    assert_eq!(players[0].coin, 70.0);
    assert_eq!(politics[0].relation(0), 50.0);

    state
        .deploy_assignment(
            0,
            0,
            &mut players,
            &politics,
            &config,
            SpyAssignment::DiscoverScandals,
            Some(1),
        )
        .unwrap();
    config.detection_range = [1.0; 2];
    let events = state.flee(0, 0, &target, &mut politics, 6, &config).unwrap();
    assert!(events.iter().any(|event| matches!(event, EspionageEvent::Detected(0, 0))));
    assert!(state.missions.is_empty());
    assert_eq!(politics[0].relation(0), 40.0);
    assert_eq!(players[0].coin, 70.0, "Flee does not charge another month of upkeep");
}

#[test]
fn detected_flee_gives_the_target_player_espionage_evidence() {
    let mut state = EspionageState::new(19);
    let config = EspionageConfig {
        detection_range: [1.0; 2],
        ..Default::default()
    };
    let mut players = vec![
        PoliticalPlayer {
            influence: 100.0,
            ..Default::default()
        };
        2
    ];
    let mut politics = vec![ProvincePolitics::owned(2, 1)];
    state
        .deploy_assignment(
            0,
            0,
            &mut players,
            &politics,
            &config,
            SpyAssignment::DiscoverScandals,
            Some(1),
        )
        .unwrap();
    let target = SpyProvince {
        owner: Some(1),
        noble_happiness: 50.0,
        conditions: vec![],
    };
    let events = state.flee(0, 0, &target, &mut politics, 0, &config).unwrap();
    assert!(events.iter().any(|event| matches!(event, EspionageEvent::Detected(0, 0))));
    assert!(events.iter().any(|event| matches!(event, EspionageEvent::EvidenceDiscovered(1, _))));
    assert_eq!(state.scandals.len(), 1);
    assert_eq!(state.scandals[0].holder, 1);
    assert!(state.missions.is_empty());
}

#[test]
fn mission_prices_are_distinct_and_failed_deployment_does_not_charge() {
    let config = EspionageConfig::default();
    let politics = vec![ProvincePolitics::independent(1)];
    for (assignment, base_cost) in [
        (SpyAssignment::GainControl, 10.0),
        (SpyAssignment::ImproveRelations, 5.0),
        (SpyAssignment::DiscoverScandals, 15.0),
        (SpyAssignment::UndermineOpponents, 20.0),
        (SpyAssignment::SupportRevolt, 20.0),
        (SpyAssignment::DiscreditRivals, 20.0),
    ] {
        for distance in [0, 12] {
            let cost = config.deployment_cost_at(assignment, Some(distance)).unwrap();
            let mut state = EspionageState::new(1);
            let mut players = vec![PoliticalPlayer {
                influence: cost - 1.0,
                ..Default::default()
            }];
            assert!(state
                .deploy_assignment(
                    0,
                    0,
                    &mut players,
                    &politics,
                    &config,
                    assignment,
                    Some(distance),
                )
                .is_err());
            assert_eq!(players[0].influence, cost - 1.0);
            assert!(state.missions.is_empty());
            players[0].influence = cost;
            state
                .deploy_assignment(
                    0,
                    0,
                    &mut players,
                    &politics,
                    &config,
                    assignment,
                    Some(distance),
                )
                .unwrap();
            assert_eq!(players[0].influence, 0.0);
            assert_eq!(state.missions[0].assignment, assignment);
            assert_eq!(state.missions[0].totals.influence_spent, cost);
            assert_eq!(config.deployment_cost(assignment), base_cost);
        }
    }
}

#[test]
fn distance_adjusts_the_whole_price_charged_at_launch_and_each_month() {
    let config = EspionageConfig {
        detection_range: [0.0; 2],
        ..Default::default()
    };
    assert_eq!(config.deployment_cost_at(SpyAssignment::GainControl, Some(8)), Ok(20.0));
    assert_eq!(config.monthly_cost_at(Some(8)), Ok(10.0));
    assert_eq!(config.deployment_cost_at(SpyAssignment::GainControl, Some(2)), Ok(13.0));
    assert_eq!(config.monthly_cost_at(Some(4)), Ok(8.0));
    assert_eq!(config.deployment_cost_at(SpyAssignment::GainControl, None), Ok(30.0));
    let mut state = EspionageState::new(1);
    let mut players = vec![PoliticalPlayer {
        influence: 20.0,
        coin: 10.0,
        ..Default::default()
    }];
    let mut politics = vec![ProvincePolitics::independent(1)];
    state
        .deploy_assignment(
            0,
            0,
            &mut players,
            &politics,
            &config,
            SpyAssignment::GainControl,
            Some(8),
        )
        .unwrap();
    assert_eq!(players[0].influence, 0.0);
    assert_eq!(state.missions[0].totals.influence_spent, 20.0);
    state.advance_month(
        1,
        &mut players,
        &[SpyProvince {
            owner: None,
            noble_happiness: 50.0,
            conditions: vec![],
        }],
        &mut politics,
        &config,
    );
    assert_eq!(players[0].coin, 0.0);
    assert_eq!(state.missions[0].totals.coin_spent, 10.0);
}

#[test]
fn undermining_targets_only_positive_rival_stats_and_preserves_the_control_pool() {
    for (control, relation) in [(30.0, 40.0), (30.0, 0.0), (0.0, 40.0), (0.0, 0.0)] {
        let mut state = EspionageState::new(17);
        let config = EspionageConfig {
            detection_range: [0.0; 2],
            npc_generation_chance: 1.0,
            npc_discovery_chance: 1.0,
            ..Default::default()
        };
        let mut players = vec![
            PoliticalPlayer {
                coin: 100.0,
                influence: 100.0,
                ..Default::default()
            };
            3
        ];
        let mut target = ProvincePolitics::independent(3);
        target.state = PoliticalState::Independent {
            local: 80.0 - control,
            shares: vec![20.0, control, 0.0],
        };
        target.relations = vec![50.0, relation, 0.0];
        let mut politics = vec![target];
        state
            .deploy_assignment(
                0,
                0,
                &mut players,
                &politics,
                &config,
                SpyAssignment::DiscreditRivals,
                Some(12),
            )
            .unwrap();
        state.advance_month(
            1,
            &mut players,
            &[SpyProvince {
                owner: None,
                noble_happiness: 50.0,
                conditions: vec![],
            }],
            &mut politics,
            &config,
        );
        politics[0].resolve_month(
            &[0.0; 3],
            None,
            0.0,
            &super::super::diplomacy::DiplomacyConfig::default(),
        );
        let loss = control - politics[0].control(1) + relation - politics[0].relation(1);
        assert!((0.0..=3.0).contains(&loss));
        if control + relation == 0.0 {
            assert_eq!(loss, 0.0);
        }
        assert_eq!(politics[0].control(0), 20.0, "The spy cannot undermine its own player");
        assert_eq!(politics[0].relation(0), 50.0);
        assert_eq!(politics[0].relation(2), 0.0, "An absent rival is never targeted");
        assert!(state.scandals.is_empty(), "Undermining does not also uncover scandals");
        if let PoliticalState::Independent {
            local,
            shares,
        } = &politics[0].state
        {
            assert!((local + shares.iter().sum::<f64>() - 100.0).abs() < 1e-9);
        }
        assert_eq!(players[0].influence, 50.0);
        assert_eq!(players[0].coin, 87.0);
    }
}

#[test]
fn undermining_is_intermittent_in_npc_provinces() {
    let mut state = EspionageState::new(17);
    let config = EspionageConfig {
        detection_range: [0.0; 2],
        npc_generation_chance: 0.0,
        ..Default::default()
    };
    let mut players = vec![
        PoliticalPlayer {
            coin: 1000.0,
            influence: 100.0,
            ..Default::default()
        };
        2
    ];
    let mut politics = vec![ProvincePolitics::independent(2)];
    state
        .deploy_assignment(
            0,
            0,
            &mut players,
            &politics,
            &config,
            SpyAssignment::UndermineOpponents,
            Some(1),
        )
        .unwrap();
    let snapshots = [SpyProvince {
        owner: None,
        noble_happiness: 50.0,
        conditions: vec![],
    }];
    let mut successes = 0;
    for month in 1..=48 {
        let events = state.advance_month(month, &mut players, &snapshots, &mut politics, &config);
        successes += usize::from(events.iter().any(|event| {
            matches!(event,
            EspionageEvent::PopulationUndermined(0, 0, class, points)
                if *class < 3 && (1.0..=3.0).contains(points))
        }));
    }
    assert!(
        successes > 0 && successes < 48,
        "Undermining succeeds sometimes, not every month: {successes}"
    );
}

#[test]
fn caught_or_unpaid_spies_cannot_undermine() {
    for (coin, detected, owner) in
        [(100.0, true, None), (0.0, false, None), (100.0, true, Some(1)), (0.0, false, Some(1))]
    {
        let config = EspionageConfig {
            detection_range: [if detected {
                1.0
            } else {
                0.0
            }; 2],
            ..Default::default()
        };
        let mut state = EspionageState::new(17);
        let mut players = vec![
            PoliticalPlayer {
                coin,
                influence: 100.0,
                ..Default::default()
            };
            2
        ];
        let mut politics = vec![owner.map_or_else(
            || ProvincePolitics::independent(2),
            |player| ProvincePolitics::owned(2, player),
        )];
        state
            .deploy_assignment(
                0,
                0,
                &mut players,
                &politics,
                &config,
                SpyAssignment::UndermineOpponents,
                Some(1),
            )
            .unwrap();
        let events = state.advance_month(
            1,
            &mut players,
            &[SpyProvince {
                owner,
                noble_happiness: 50.0,
                conditions: vec![],
            }],
            &mut politics,
            &config,
        );
        assert!(state.missions.is_empty());
        assert_eq!(politics[0].relation(1), 50.0);
        assert!(!events
            .iter()
            .any(|event| matches!(event, EspionageEvent::PopulationUndermined(..))));
    }
}

#[test]
fn spy_missions_charge_their_own_prices_and_require_recall_to_change_mission() {
    let mut state = EspionageState::new(17);
    let config = EspionageConfig {
        detection_range: [0.0; 2],
        npc_generation_chance: 1.0,
        npc_discovery_chance: 1.0,
        ..Default::default()
    };
    let mut players = vec![PoliticalPlayer {
        coin: 100.0,
        influence: 100.0,
        ..Default::default()
    }];
    let mut politics = vec![ProvincePolitics::independent(1)];
    let snapshots = [SpyProvince {
        owner: None,
        noble_happiness: 50.0,
        conditions: vec![],
    }];
    state
        .deploy_assignment(
            0,
            0,
            &mut players,
            &politics,
            &config,
            SpyAssignment::GainControl,
            Some(6),
        )
        .unwrap();
    assert_eq!(players[0].influence, 82.0);
    assert_eq!(state.missions[0].totals.influence_spent, 18.0);
    assert_eq!(state.missions[0].totals.coin_spent, 0.0);
    assert_eq!(politics[0].queued_control_pressure(0), 0.0);
    state.advance_month(1, &mut players, &snapshots, &mut politics, &config);
    assert_eq!(players[0].coin, 91.0);
    let control = politics[0].queued_control_pressure(0);
    assert!((0.0..=3.0).contains(&control));
    assert_eq!(state.missions[0].totals.control_contributed, control);
    assert_eq!(state.missions[0].totals.coin_spent, 9.0);
    assert_eq!(politics[0].relation(0), 50.0);
    assert!(state.scandals.is_empty());
    assert_eq!(
        state.deploy_assignment(
            0,
            0,
            &mut players,
            &politics,
            &config,
            SpyAssignment::ImproveRelations,
            Some(6)
        ),
        Err(PoliticalError::AlreadyUsed)
    );
    assert_eq!(state.missions[0].assignment, SpyAssignment::GainControl);
    assert_eq!(players[0].influence, 82.0);
    state.withdraw(0, 0);
    state
        .deploy_assignment(
            0,
            0,
            &mut players,
            &politics,
            &config,
            SpyAssignment::ImproveRelations,
            Some(6),
        )
        .unwrap();
    assert_eq!(politics[0].relation(0), 50.0, "Assignment has no instant benefit");
    assert_eq!(players[0].influence, 73.0, "Relations cost 9 Influence at this distance");
    assert_eq!(state.missions[0].totals.influence_spent, 9.0);
    assert_eq!(state.missions[0].totals.coin_spent, 0.0, "A fresh launch resets totals");
    assert_eq!(state.missions[0].totals.control_contributed, 0.0);
    state.advance_month(2, &mut players, &snapshots, &mut politics, &config);
    let relation = politics[0].relation(0);
    assert!((50.0..=53.0).contains(&relation));
    assert_eq!(politics[0].queued_control_pressure(0), control);
    assert!(state.scandals.is_empty());
    assert!(state.advance_month(2, &mut players, &snapshots, &mut politics, &config).is_empty());
    assert_eq!(politics[0].relation(0), relation, "Duplicate tick cannot deliver twice");
    assert_eq!(state.missions[0].totals.relation_gained, relation - 50.0);
    assert_eq!(state.missions[0].totals.coin_spent, 9.0, "Duplicate ticks do not inflate costs");
    state.withdraw(0, 0);
    state
        .deploy_assignment(
            0,
            0,
            &mut players,
            &politics,
            &config,
            SpyAssignment::DiscoverScandals,
            Some(6),
        )
        .unwrap();
    state.advance_month(3, &mut players, &snapshots, &mut politics, &config);
    assert!(!state.scandals.is_empty());
    assert_eq!(state.missions[0].totals.scandals_revealed, 1);
    state.scandals.clear();
    assert_eq!(
        state.missions[0].totals.scandals_revealed, 1,
        "Spending evidence preserves deployment history"
    );
    assert_eq!(politics[0].relation(0), relation);
    assert_eq!(
        players[0].influence, 47.0,
        "Control, relations and scandals cost 18, 9 and 26 here"
    );
    assert_eq!(players[0].coin, 73.0, "Every resolved mission costs 9 per month here");
    state.withdraw(0, 0);
    state.advance_month(4, &mut players, &snapshots, &mut politics, &config);
    assert_eq!(players[0].coin, 73.0, "Recall ends upkeep");
}

#[test]
fn spy_history_uses_paid_costs_and_clamped_relation_gains() {
    let mut state = EspionageState::new(17);
    let mut config = EspionageConfig {
        detection_range: [0.0; 2],
        npc_generation_chance: 0.0,
        ..Default::default()
    };
    let mut players = vec![PoliticalPlayer {
        coin: 100.0,
        influence: 100.0,
        ..Default::default()
    }];
    let mut politics = vec![ProvincePolitics::independent(1)];
    politics[0].relations[0] = 99.5;
    let provinces = [SpyProvince {
        owner: None,
        noble_happiness: 50.0,
        conditions: vec![],
    }];
    state
        .deploy_assignment(
            0,
            0,
            &mut players,
            &politics,
            &config,
            SpyAssignment::ImproveRelations,
            Some(1),
        )
        .unwrap();
    state.advance_month(1, &mut players, &provinces, &mut politics, &config);
    config.monthly_coin = 7.0;
    state.advance_month(2, &mut players, &provinces, &mut politics, &config);
    let totals = &state.missions[0].totals;
    assert_eq!(totals.influence_spent, 5.0);
    assert_eq!(totals.coin_spent, 12.0, "Use the cost actually paid in each month");
    assert!((0.0..=0.5).contains(&totals.relation_gained), "No gains beyond the relation ceiling");
    assert_eq!(totals.relation_gained, politics[0].relation(0) - 99.5);
}

#[test]
fn detected_or_unpaid_spies_never_deliver_control_or_relation() {
    for assignment in [SpyAssignment::GainControl, SpyAssignment::ImproveRelations] {
        for caught in [true, false] {
            let mut state = EspionageState::new(1);
            let config = EspionageConfig {
                detection_range: [if caught {
                    1.0
                } else {
                    0.0
                }; 2],
                ..Default::default()
            };
            let mut players = vec![PoliticalPlayer {
                coin: if caught {
                    100.0
                } else {
                    0.0
                },
                influence: 100.0,
                ..Default::default()
            }];
            let mut politics = vec![ProvincePolitics::independent(1)];
            state
                .deploy_assignment(0, 0, &mut players, &politics, &config, assignment, Some(1))
                .unwrap();
            state.advance_month(
                1,
                &mut players,
                &[SpyProvince {
                    owner: None,
                    noble_happiness: 50.0,
                    conditions: vec![],
                }],
                &mut politics,
                &config,
            );
            assert!(state.missions.is_empty());
            assert_eq!(politics[0].queued_control_pressure(0), 0.0);
            assert_eq!(
                politics[0].relation(0),
                if caught {
                    40.0
                } else {
                    50.0
                }
            );
        }
    }
}

#[test]
fn disconnected_missions_charge_the_farthest_tier_and_foreign_owned_control_is_available() {
    let mut state = EspionageState::new(1);
    let config = EspionageConfig::default();
    let mut players = vec![PoliticalPlayer {
        influence: 100.0,
        ..Default::default()
    }];
    let mut politics = vec![ProvincePolitics::independent(2)];
    assert_eq!(
        state.deploy_assignment(
            0,
            0,
            &mut players,
            &politics,
            &config,
            SpyAssignment::GainControl,
            None
        ),
        Ok(())
    );
    assert_eq!(players[0].influence, 70.0);
    assert_eq!(state.missions[0].distance, 16);
    state.withdraw(0, 0);
    politics[0] = ProvincePolitics::owned(2, 1);
    assert_eq!(
        state.deploy_assignment(
            0,
            0,
            &mut players,
            &politics,
            &config,
            SpyAssignment::GainControl,
            Some(1)
        ),
        Ok(())
    );
    assert!(players[0].influence < 100.0);
    assert_eq!(state.missions[0].assignment, SpyAssignment::GainControl);
}

#[test]
fn duplicate_network_is_rejected_and_unpaid_spy_withdraws() {
    let mut state = EspionageState::new(1);
    let config = EspionageConfig::default();
    let mut players = vec![PoliticalPlayer {
        influence: 30.0,
        ..Default::default()
    }];
    let mut politics = vec![ProvincePolitics::independent(1)];
    state.deploy(0, 0, &mut players, &politics, &config).unwrap();
    assert_eq!(
        state.deploy(0, 0, &mut players, &politics, &config),
        Err(PoliticalError::AlreadyUsed)
    );
    let events = state.advance_month(
        1,
        &mut players,
        &[SpyProvince {
            owner: None,
            noble_happiness: 50.0,
            conditions: vec![],
        }],
        &mut politics,
        &config,
    );
    assert!(matches!(events[0], EspionageEvent::Withdrawn(0, 0)));
    assert!(state.missions.is_empty());
    assert_eq!(players[0].influence, 15.0);
    assert_eq!(players[0].coin, 0.0);
}

#[test]
fn ending_condition_stops_discovery_but_retains_evidence_and_reactivation_is_new() {
    let mut state = EspionageState::new(4);
    let config = EspionageConfig::default();
    let mut provinces = vec![SpyProvince {
        owner: Some(0),
        noble_happiness: 50.0,
        conditions: vec![(ScandalKind::LowFood, Severity::Minor)],
    }];
    state.sync_opportunities(&provinces, 1);
    let source = state.opportunities[0].source_id;
    state.grant(
        1,
        ScandalTarget::Player(0),
        ScandalKind::LowFood,
        Severity::Minor,
        Some(0),
        source,
        1,
        &config,
    );
    provinces[0].conditions.clear();
    state.sync_opportunities(&provinces, 2);
    assert!(state.opportunities.is_empty());
    assert_eq!(state.scandals.len(), 1);
    provinces[0].conditions.push((ScandalKind::LowFood, Severity::Minor));
    state.sync_opportunities(&provinces, 3);
    assert_ne!(state.opportunities[0].source_id, source);
}

#[test]
fn guaranteed_detection_creates_real_player_evidence_before_discovery() {
    let mut state = EspionageState::new(1);
    let config = EspionageConfig {
        detection_range: [1.0, 1.0],
        ..Default::default()
    };
    let mut players = vec![
        PoliticalPlayer {
            influence: 30.0,
            coin: 30.0,
            ..Default::default()
        };
        2
    ];
    let mut politics = vec![ProvincePolitics::owned(2, 1)];
    state.deploy(0, 0, &mut players, &politics, &config).unwrap();
    state.advance_month(
        1,
        &mut players,
        &[SpyProvince {
            owner: Some(1),
            noble_happiness: 50.0,
            conditions: vec![],
        }],
        &mut politics,
        &config,
    );
    assert!(state.missions.is_empty());
    assert_eq!(state.scandals.len(), 1);
    assert_eq!(state.scandals[0].holder, 1);
    assert_eq!(state.scandals[0].target, ScandalTarget::Player(0));
    assert_eq!(state.scandals[0].kind, ScandalKind::Espionage);
}

#[test]
fn detection_formula_uses_reduced_monthly_risk_and_scandals_have_distinct_bloc_effects() {
    let config = EspionageConfig::default();
    for (happiness, risk) in
        [(-10.0, 0.005), (0.0, 0.005), (50.0, 0.02), (100.0, 0.035), (110.0, 0.035)]
    {
        assert!((detection_chance(happiness, &config) - risk).abs() < 1e-9);
    }
    let famine = ScandalKind::Famine.bloc_penalties(Severity::Medium);
    let bribery = ScandalKind::PoliticalBribery.bloc_penalties(Severity::Medium);
    assert!(famine[3] > famine[0]);
    assert!(bribery[0] > bribery[3]);
}

#[test]
fn monthly_detection_outcomes_match_the_displayed_risk_across_assignments() {
    const SAMPLES: u64 = 20_000;
    for config in [
        EspionageConfig {
            detection_range: [0.02, 0.10],
            ..Default::default()
        },
        EspionageConfig {
            detection_range: [0.01, 0.07],
            ..Default::default()
        },
        EspionageConfig::default(),
    ] {
        let risk = detection_chance(50.0, &config);
        for assignment in [
            SpyAssignment::GainControl,
            SpyAssignment::ImproveRelations,
            SpyAssignment::DiscoverScandals,
            SpyAssignment::UndermineOpponents,
            SpyAssignment::SupportRevolt,
            SpyAssignment::DiscreditRivals,
        ] {
            let mut caught_by_month = [0; 3];
            for seed in 0..SAMPLES {
                let mut state = EspionageState::new(seed);
                let mut players = vec![PoliticalPlayer {
                    coin: 100.0,
                    influence: 100.0,
                    ..Default::default()
                }];
                let mut politics = vec![ProvincePolitics::independent(1)];
                let provinces = [SpyProvince {
                    owner: None,
                    noble_happiness: 50.0,
                    conditions: vec![],
                }];
                state
                    .deploy_assignment(0, 0, &mut players, &politics, &config, assignment, Some(1))
                    .unwrap();
                for month in 1..=3 {
                    state.advance_month(month, &mut players, &provinces, &mut politics, &config);
                    let coin = players[0].coin;
                    assert!(state
                        .advance_month(month, &mut players, &provinces, &mut politics, &config)
                        .is_empty());
                    assert_eq!(
                        players[0].coin, coin,
                        "duplicate ticks cannot charge or roll again"
                    );
                    if state.missions.is_empty() {
                        caught_by_month[(month - 1) as usize] += 1;
                    }
                }
            }
            for (index, caught) in caught_by_month.into_iter().enumerate() {
                let months = (index + 1) as i32;
                let expected = 1.0 - (1.0 - risk).powi(months);
                let observed = f64::from(caught) / SAMPLES as f64;
                assert!(
                    (observed - expected).abs() < 0.01,
                    "{assignment:?}, {risk:.4} monthly risk over {months} months: expected {expected}, observed {observed}"
                );
                if months == 3 {
                    eprintln!(
                        "{assignment:?}: monthly risk {risk:.4}, caught within 3 months {observed:.4} (expected {expected:.4})"
                    );
                }
            }
        }
    }
}

#[test]
fn fleeing_doubles_the_cumulative_recall_risk_and_caps_it() {
    let mut config = EspionageConfig {
        detection_range: [0.037; 2],
        ..Default::default()
    };
    let expected = 2.0 * (1.0 - 0.963_f64.powi(6));
    assert!((flee_detection_chance(50.0, &config) - expected).abs() < 1e-9);
    assert!((expected - 0.405).abs() < 0.001);
    config.detection_range = [0.0; 2];
    assert_eq!(flee_detection_chance(50.0, &config), 0.0);
    config.detection_range = [1.0; 2];
    assert_eq!(flee_detection_chance(50.0, &config), 1.0);
    let config = EspionageConfig::default();
    assert!(flee_detection_chance(100.0, &config) > flee_detection_chance(0.0, &config));
}

#[test]
fn motion_evidence_survives_expiration_but_cannot_be_spent_twice() {
    let mut state = EspionageState::new(4);
    let config = EspionageConfig {
        evidence_lifetime: 24,
        ..Default::default()
    };
    let id = state.grant(
        0,
        ScandalTarget::Player(1),
        ScandalKind::Espionage,
        Severity::Medium,
        None,
        9,
        0,
        &config,
    );
    state.reserve_motion(0, id, 20).unwrap();
    assert_eq!(state.consume(0, id, 20).unwrap_err(), PoliticalError::ScandalRequired);
    state.advance_month(25, &mut [], &[], &mut [], &config);
    assert_eq!(state.scandals.len(), 1);
    state.consume_motion(id).unwrap();
    assert!(state.scandals.is_empty());
}

#[test]
fn newly_discovered_scandals_are_permanent_after_their_source_ends() {
    let mut state = EspionageState::new(4);
    let config = EspionageConfig::default();
    let id = state.grant(
        0,
        ScandalTarget::Player(1),
        ScandalKind::Espionage,
        Severity::Medium,
        None,
        9,
        5,
        &config,
    );
    assert_eq!(state.scandals[0].validity_label(5), "Permanent");
    state.advance_month(1200, &mut [], &[], &mut [], &config);
    assert!(state.holds_evidence(0, id, 1, 1200));
    state.consume(0, id, 1200).unwrap();
    assert!(state.scandals.is_empty(), "Deliberate use still spends the scandal");
}

#[test]
fn timed_scandals_expire_once_at_their_stated_deadline() {
    let mut state = EspionageState::new(4);
    let config = EspionageConfig {
        evidence_lifetime: 120,
        ..Default::default()
    };
    let id = state.grant(
        0,
        ScandalTarget::Player(1),
        ScandalKind::Espionage,
        Severity::Medium,
        None,
        9,
        5,
        &config,
    );
    assert_eq!(state.scandals[0].validity_label(124), "1 month remaining");
    assert!(state.advance_month(124, &mut [], &[], &mut [], &config).is_empty());
    let events = state.advance_month(125, &mut [], &[], &mut [], &config);
    assert!(
        matches!(events.as_slice(), [EspionageEvent::EvidenceExpired(scandal)] if scandal.id == id && scandal.holder == 0)
    );
    assert!(state.scandals.is_empty());
    assert!(state.advance_month(126, &mut [], &[], &mut [], &config).is_empty());
}

#[test]
fn foreign_attack_is_discoverable_in_perpetrator_holdings_with_origin_preserved() {
    let config = EspionageConfig {
        detection_range: [0.0, 0.0],
        evidence_lifetime: 24,
        ..Default::default()
    };
    let mut state = EspionageState::new(7);
    let mut players = vec![
        PoliticalPlayer {
            coin: 500.0,
            influence: 100.0,
            ..Default::default()
        };
        2
    ];
    let mut politics = vec![ProvincePolitics::owned(2, 0), ProvincePolitics::owned(2, 1)];
    let provinces = [
        SpyProvince {
            owner: Some(0),
            noble_happiness: 50.0,
            conditions: vec![],
        },
        SpyProvince {
            owner: Some(1),
            noble_happiness: 50.0,
            conditions: vec![],
        },
    ];
    state.deploy(1, 0, &mut players, &politics, &config).unwrap();
    state.record_action(0, Some(1), ScandalKind::FriendlyAttack, Severity::Major, 0, &config);
    for month in 1..config.evidence_lifetime {
        state.advance_month(month, &mut players, &provinces, &mut politics, &config);
        if !state.scandals.is_empty() {
            break;
        }
    }
    let evidence = state.scandals.first().expect("real foreign attack must be discoverable");
    assert_eq!(evidence.holder, 1);
    assert_eq!(evidence.target, ScandalTarget::Player(0));
    assert_eq!(evidence.province, Some(1));
    assert_eq!(evidence.kind, ScandalKind::FriendlyAttack);
}
