use super::*;

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
    assert_eq!(players[0].influence, 20.0);
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
fn detection_formula_matches_spec_and_scandals_have_distinct_bloc_effects() {
    let config = EspionageConfig::default();
    assert!((detection_chance(50.0, &config) - 0.06).abs() < 1e-9);
    let famine = ScandalKind::Famine.bloc_penalties(Severity::Medium);
    let bribery = ScandalKind::PoliticalBribery.bloc_penalties(Severity::Medium);
    assert!(famine[3] > famine[0]);
    assert!(bribery[0] > bribery[3]);
}

#[test]
fn motion_evidence_survives_expiration_but_cannot_be_spent_twice() {
    let mut state = EspionageState::new(4);
    let config = EspionageConfig::default();
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
fn foreign_attack_is_discoverable_in_perpetrator_holdings_with_origin_preserved() {
    let config = EspionageConfig {
        detection_range: [0.0, 0.0],
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
