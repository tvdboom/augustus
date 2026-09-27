use super::*;

/// Wealthy actors for focused political invariants.
fn actors() -> Vec<PoliticalPlayer> {
    vec![
        PoliticalPlayer {
            rank: PoliticalRank::Aedile,
            influence: 1000.0,
            coin: 1000.0,
            consul_until: None
        };
        3
    ]
}

#[test]
fn sealed_tie_requires_additional_month_and_spends_losing_bids() {
    let mut state = SenateState::new(1);
    let mut players = actors();
    let config = SenateConfig::default();
    state.submit_bid(0, Ballot::Praetor, 100.0, &mut players, false, &config).unwrap();
    state.submit_bid(1, Ballot::Praetor, 100.0, &mut players, false, &config).unwrap();
    assert!(state.nominations.is_empty());
    for _ in 0..12 {
        state.advance_month(&mut players, &[], &config);
    }
    assert!(state.campaign.is_none());
    assert_eq!(state.sudden_death.len(), 2);
    assert!(state.submit_bid(2, Ballot::Praetor, 200.0, &mut players, false, &config).is_err());
    state.submit_bid(1, Ballot::Praetor, 1.0, &mut players, false, &config).unwrap();
    state.advance_month(&mut players, &[], &config);
    assert_eq!(state.campaign.unwrap().candidate, 1);
    assert_eq!(players[0].influence, 900.0);
    assert_eq!(players[1].influence, 899.0);
}

#[test]
fn censor_is_required_and_elected_through_shared_ballot() {
    let state = SenateState::new(1);
    let mut players = actors();
    let config = SenateConfig::default();
    players[0].rank = PoliticalRank::Praetor;
    assert!(state.eligibility(0, Ballot::Censor, &players, false, &config).is_ok());
    assert!(state.eligibility(0, Ballot::Consul, &players, false, &config).is_err());
    players[0].rank = PoliticalRank::Censor;
    assert!(state.eligibility(0, Ballot::Consul, &players, false, &config).is_ok());
    assert_eq!(config.rank_income(PoliticalRank::Censor), 3.0);
}

#[test]
fn projection_and_recorded_votes_always_have_one_hundred_seats() {
    let mut state = SenateState::new(4);
    let mut players = actors();
    let config = SenateConfig::default();
    state.submit_bid(0, Ballot::Praetor, 100.0, &mut players, false, &config).unwrap();
    for _ in 0..12 {
        state.advance_month(&mut players, &[], &config);
    }
    let projection = state.projection(&players, &[], &config);
    assert_eq!(
        projection
            .iter()
            .map(|p| p.yes as usize + p.no as usize + p.undecided as usize)
            .sum::<usize>(),
        100
    );
    for _ in 0..12 {
        state.advance_month(&mut players, &[], &config);
    }
    let result = state.last_result.unwrap();
    assert_eq!(result.votes.len(), 100);
    assert_eq!(result.yes + result.no, 100);
    let mut offset = 0;
    for bloc in projection {
        assert!(result.votes[offset..offset + bloc.yes as usize].iter().all(|v| *v));
        offset += bloc.yes as usize;
        assert!(result.votes[offset..offset + bloc.no as usize].iter().all(|v| !v));
        offset += bloc.no as usize + bloc.undecided as usize;
    }
}

#[test]
fn no_third_consul_and_proconsul_cannot_seek_augustus() {
    let state = SenateState::new(1);
    let mut players = actors();
    let config = SenateConfig::default();
    players[0].rank = PoliticalRank::Censor;
    for player in &mut players[1..] {
        player.rank = PoliticalRank::Consul;
        player.consul_until = Some(48);
    }
    assert_eq!(
        state.eligibility(0, Ballot::Consul, &players, false, &config),
        Err(PoliticalError::NoConsulSeat)
    );
    players[0].rank = PoliticalRank::Proconsul;
    assert!(state.eligibility(0, Ballot::Augustus, &players, false, &config).is_err());
    players[1].consul_until = Some(24);
    assert!(state.eligibility(0, Ballot::Consul, &players, false, &config).is_ok());
}

#[test]
fn bid_preview_enforces_sealed_motion_funds_and_tie_participation() {
    let mut state = SenateState::new(7);
    let mut players = actors();
    let config = SenateConfig::default();
    players[1].rank = PoliticalRank::Consul;
    players[1].consul_until = Some(48);
    let removal = Ballot::NoConfidence {
        target: 1,
        scandal_id: 9,
    };
    state.submit_bid(0, Ballot::Praetor, 100.0, &mut players, false, &config).unwrap();
    let balance = players[0].influence;
    assert_eq!(state.chosen_ballot(0), Some(Ballot::Praetor));
    assert_eq!(
        state.bid_eligibility(0, removal, 100.0, &players, true, &config),
        Err(PoliticalError::Ineligible)
    );
    assert_eq!(
        state.bid_eligibility(0, Ballot::Praetor, balance + 1.0, &players, false, &config),
        Err(PoliticalError::InsufficientFunds)
    );
    state.sudden_death.insert(2);
    assert_eq!(
        state.bid_eligibility(0, Ballot::Praetor, 1.0, &players, false, &config),
        Err(PoliticalError::Ineligible)
    );
    assert_eq!(players[0].influence, balance);
    assert_eq!(state.private_bid(0), 100.0);
}

#[test]
fn nomination_outlook_uses_real_profile_without_starting_a_campaign() {
    let state = SenateState::new(7);
    let players = actors();
    let config = SenateConfig::default();
    let weak = PoliticalProfile {
        noble_happiness: 10.0,
        ..Default::default()
    };
    let strong = PoliticalProfile {
        noble_happiness: 90.0,
        ..Default::default()
    };
    let a = state.preview_projection(0, Ballot::Praetor, &players, &[weak], &config);
    let b = state.preview_projection(0, Ballot::Praetor, &players, &[strong], &config);
    assert!(b[0].support_probability > a[0].support_probability);
    assert!(!b[0].reasons.is_empty());
    assert!(state.campaign.is_none());
    assert_eq!(state.month, 0);
    assert!(state.nominations.is_empty());
}

#[test]
fn seeded_results_are_reproducible() {
    let config = SenateConfig::default();
    let play = || {
        let mut state = SenateState::new(19);
        let mut players = actors();
        state.submit_bid(0, Ballot::Praetor, 100.0, &mut players, false, &config).unwrap();
        for _ in 0..24 {
            state.advance_month(&mut players, &[], &config);
        }
        state.last_result.unwrap().votes
    };
    assert_eq!(play(), play());
}

#[test]
fn augustus_ballot_resolves_before_same_month_consul_expiry() {
    let config = SenateConfig {
        probability_bounds: [1.0, 1.0],
        ..Default::default()
    };
    let mut state = SenateState::new(1);
    let mut players = actors();
    players[0].rank = PoliticalRank::Consul;
    players[0].consul_until = Some(24);
    state.submit_bid(0, Ballot::Augustus, 400.0, &mut players, false, &config).unwrap();
    for _ in 0..24 {
        state.advance_month(&mut players, &[], &config);
    }
    assert_eq!(state.winner, Some(0));
    assert_eq!(players[0].rank, PoliticalRank::Augustus);
}

#[test]
fn failed_augustus_becomes_proconsul_on_term_expiry() {
    let config = SenateConfig {
        probability_bounds: [0.0, 0.0],
        ..Default::default()
    };
    let mut state = SenateState::new(1);
    let mut players = actors();
    players[0].rank = PoliticalRank::Consul;
    players[0].consul_until = Some(24);
    state.submit_bid(0, Ballot::Augustus, 400.0, &mut players, false, &config).unwrap();
    for _ in 0..24 {
        state.advance_month(&mut players, &[], &config);
    }
    assert_eq!(state.winner, None);
    assert_eq!(players[0].rank, PoliticalRank::Proconsul);
}

#[test]
fn no_confidence_needs_real_evidence_and_removes_on_majority() {
    let config = SenateConfig {
        probability_bounds: [1.0, 1.0],
        ..Default::default()
    };
    let mut state = SenateState::new(1);
    let mut players = actors();
    players[1].rank = PoliticalRank::Consul;
    players[1].consul_until = Some(48);
    let ballot = Ballot::NoConfidence {
        target: 1,
        scandal_id: 7,
    };
    assert_eq!(
        state.submit_bid(0, ballot, 200.0, &mut players, false, &config),
        Err(PoliticalError::ScandalRequired)
    );
    state.submit_bid(0, ballot, 200.0, &mut players, true, &config).unwrap();
    let mut events = Vec::new();
    for _ in 0..24 {
        events = state.advance_month(&mut players, &[], &config);
    }
    assert_eq!(players[1].rank, PoliticalRank::Proconsul);
    assert!(events.iter().any(|e| matches!(e, SenateEvent::ConsumeScandal(7))));
}
