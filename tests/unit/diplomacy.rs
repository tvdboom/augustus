use super::*;

#[test]
fn unequal_simultaneous_pressure_matches_spec() {
    let mut local = 10.0;
    let mut shares = [45.0, 45.0];
    resolve_control(&mut local, &mut shares, &[10.0, 5.0], &[0.0; 2]);
    assert!((shares[0] - 53.333333).abs() < 1e-5);
    assert!((shares[1] - 46.666667).abs() < 1e-5);
    assert_eq!(local, 0.0);
}

#[test]
fn equal_pressure_has_no_first_mover_advantage() {
    let mut local = 10.0;
    let mut shares = [45.0, 45.0];
    resolve_control(&mut local, &mut shares, &[10.0; 2], &[0.0; 2]);
    assert_eq!(shares, [50.0, 50.0]);
}

#[test]
fn vassal_conversion_consumes_fifty_and_keeps_sentiment() {
    let mut province = ProvincePolitics::independent(2);
    province.state = PoliticalState::Independent {
        local: 10.0,
        shares: vec![60.0, 30.0],
    };
    province.relations[0] = 20.0;
    province.vassalize(0).unwrap();
    assert!(matches!(
        province.state,
        PoliticalState::Vassal {
            control: 10.0,
            ..
        }
    ));
    assert_eq!(province.relation(0), 20.0);
    assert!(province.queue_control_gain(1, 50.0).is_err());
    assert!(province.take_ownership(0).is_err());
}

#[test]
fn hostile_vassal_releases_without_destroying_relations() {
    let mut province = ProvincePolitics::independent(1);
    province.state = PoliticalState::Vassal {
        overlord: 0,
        control: 1.0,
        tribute: Tribute::Normal,
    };
    province.relations[0] = 20.0;
    province.advance_month(
        &mut [PoliticalPlayer::default()],
        &[Some(1)],
        &[0.0],
        None,
        &DiplomacyConfig::default(),
    );
    assert!(matches!(
        province.state,
        PoliticalState::Independent {
            local: 100.0,
            ..
        }
    ));
    assert_eq!(province.relation(0), 20.0);
}

#[test]
fn owned_unrest_cannot_transfer_control() {
    let mut province = ProvincePolitics::owned(2, 0);
    let mut actor = PoliticalPlayer {
        influence: 100.0,
        ..Default::default()
    };
    let deltas = province
        .interfere(
            1,
            0,
            &mut actor,
            Interference::AgitatePopulation,
            Some(1),
            &DiplomacyConfig::default(),
        )
        .unwrap();
    assert_eq!(deltas, [0.0, -5.0, -5.0, 0.0]);
    assert!(province
        .buy_control(1, &mut actor, Currency::Influence, 5.0, Some(1), &DiplomacyConfig::default())
        .is_err());
    assert_eq!(
        province.state,
        PoliticalState::Owned {
            owner: 0
        }
    );
}

#[test]
fn multiplayer_control_stays_nonnegative_and_conserved() {
    for seed in 0..100 {
        let mut rng = super::super::PoliticalRng::new(seed);
        let mut local = 10.0;
        let mut shares = [30.0, 20.0, 25.0, 15.0];
        let gains = std::array::from_fn::<_, 4, _>(|_| rng.unit() * 300.0);
        let losses = std::array::from_fn::<_, 4, _>(|_| rng.unit() * 50.0);
        resolve_control(&mut local, &mut shares, &gains, &losses);
        assert!(shares.iter().all(|value| *value >= 0.0));
        assert!((local + shares.iter().sum::<f64>() - 100.0).abs() < 1e-7);
    }
}

#[test]
fn full_control_requires_explicit_ownership_and_converts_sentiment_once() {
    let mut province = ProvincePolitics::independent(1);
    province.relations[0] = 80.0;
    province.queue_control_gain(0, 100.0).unwrap();
    province.advance_month(
        &mut [PoliticalPlayer::default()],
        &[Some(1)],
        &[0.0],
        None,
        &DiplomacyConfig::default(),
    );
    assert!(matches!(province.state, PoliticalState::Independent { .. }));
    assert_eq!(province.take_ownership(0), Ok(30.0));
    assert!(province.take_ownership(0).is_err());
    assert!(province.vassal_income(&DiplomacyConfig::default()).is_none());
}

#[test]
fn recurring_support_is_paid_before_income_and_not_recharged_at_resolution() {
    let config = DiplomacyConfig::default();
    let mut province = ProvincePolitics::independent(1);
    province.state = PoliticalState::Vassal {
        overlord: 0,
        control: 40.0,
        tribute: Tribute::Normal,
    };
    province.support[0].control_coin = true;
    let mut players = [PoliticalPlayer {
        coin: 0.0,
        ..Default::default()
    }];

    let paid = province.spend_recurring_support(&mut players, &[Some(1)], &config);
    players[0].coin += 100.0; // Income later in the same month cannot fund the skipped program.
    province.resolve_month(&[0.0], None, paid, &config);
    assert!(matches!(
        province.state,
        PoliticalState::Vassal {
            control: 40.0,
            ..
        }
    ));
    assert_eq!(players[0].coin, 100.0);

    let paid = province.spend_recurring_support(&mut players, &[Some(1)], &config);
    assert_eq!(paid, 1.0);
    assert_eq!(players[0].coin, 90.0);
    province.resolve_month(&[0.0], None, paid, &config);
    assert!(matches!(
        province.state,
        PoliticalState::Vassal {
            control: 41.0,
            ..
        }
    ));
    assert_eq!(players[0].coin, 90.0);
}
