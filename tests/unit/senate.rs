use super::*;

#[test]
fn battle_results_immediately_change_only_military_confidence() {
    let mut senate = SenateState::new(19);
    senate.record_battle_result(0, true);
    let military: f64 =
        senate.senators.iter().filter(|s| s.bloc == Bloc::Military).map(|s| s.confidence[0]).sum();
    assert_eq!(military, 3.);
    assert!(senate
        .senators
        .iter()
        .filter(|s| s.bloc != Bloc::Military)
        .all(|s| s.confidence[0] == 0.));
    senate.record_battle_result(0, false);
    let military: f64 =
        senate.senators.iter().filter(|s| s.bloc == Bloc::Military).map(|s| s.confidence[0]).sum();
    assert_eq!(military, 1.);
}
use crate::game::politics::espionage::{
    EspionageState, Scandal, ScandalKind, ScandalTarget, Severity,
};

fn actors(count: usize) -> Vec<PoliticalPlayer> {
    vec![
        PoliticalPlayer {
            coin: 10_000.0,
            influence: 5000.0,
            ..Default::default()
        };
        count
    ]
}
fn pledge(state: &mut SenateState, player: usize, count: usize) {
    state.ensure_players(player + 1);
    for s in state.senators.iter_mut().take(count) {
        s.confidence.fill(0.0);
        s.confidence[player] = state.seat_confidence;
    }
    state.review_allegiances();
}
fn points(state: &SenateState, player: usize, bloc: Bloc) -> f64 {
    state
        .senators
        .iter()
        .filter(|s| s.bloc == bloc)
        .map(|s| s.confidence.get(player).copied().unwrap_or(0.0))
        .sum()
}
fn evidence(target: usize, kind: ScandalKind, severity: Severity) -> EspionageState {
    let mut e = EspionageState::new(4);
    e.scandals.push(Scandal {
        id: 1,
        source_id: 1,
        holder: 0,
        target: ScandalTarget::Player(target),
        province: None,
        kind,
        severity,
        acquired: 0,
        expires: 24,
        reserved_for_motion: false,
    });
    e
}
fn safe_config() -> SenateConfig {
    SenateConfig {
        action_risks: [0.0; 9],
        ..Default::default()
    }
}

#[test]
fn new_match_has_one_hundred_neutral_stable_seats_and_zero_confidence() {
    let s = SenateState::new(1);
    assert_eq!(s.senators.len(), 100);
    for (id, seat) in s.senators.iter().enumerate() {
        assert_eq!(seat.id, id);
        assert_eq!(seat.allegiance, None);
        assert_eq!(seat.bloc, Bloc::ALL[id / 20]);
        assert!(seat.confidence.is_empty());
    }
}
#[test]
fn first_rank_requires_ten_and_requirements_scale_without_lowering_majority() {
    let c = SenateConfig::default();
    let ranks = [
        PoliticalRank::Quaestor,
        PoliticalRank::Aedile,
        PoliticalRank::Praetor,
        PoliticalRank::Censor,
        PoliticalRank::Consul,
    ];
    assert_eq!(ranks.map(|r| c.requirements(r, 2).unwrap().senators), [10, 20, 30, 40, 60]);
    assert_eq!(ranks.map(|r| c.requirements(r, 4).unwrap().senators), [7, 14, 21, 28, 51]);
    assert_eq!(c.requirements(PoliticalRank::Consul, 8).unwrap().senators, 51);
    assert_eq!(
        ranks.map(|r| c.requirements(r, 2).unwrap().influence),
        [500.0, 1000.0, 1500.0, 2000.0, 3000.0]
    );
}
#[test]
fn half_point_trade_route_accumulates_and_claims_one_seat_after_twenty_months() {
    let c = SenateConfig::default();
    let mut s = SenateState::new(4);
    let mut p = actors(1);
    let profiles = [PoliticalProfile {
        active_trade_routes: 1.0,
        ..Default::default()
    }];
    for _ in 0..19 {
        s.advance_month(&mut p, &profiles, &c);
    }
    assert_eq!(points(&s, 0, Bloc::Merchants), 9.5);
    assert_eq!(s.support(0), 0);
    s.advance_month(&mut p, &profiles, &c);
    assert_eq!(s.bloc_support(0, Bloc::Merchants), 1);
    s.advance_month(&mut p, &[PoliticalProfile::default()], &c);
    assert_eq!(points(&s, 0, Bloc::Merchants), 10.0);
    assert_eq!(s.bloc_support(0, Bloc::Merchants), 1);
}
#[test]
fn immediate_battle_results_initialize_players_and_refresh_approval_without_erasing_rivals() {
    let mut s = SenateState::new(1);
    for _ in 0..4 {
        s.record_battle_result(1, true);
    }
    assert_eq!(s.bloc_support(1, Bloc::Military), 1);
    s.record_battle_result(0, true);
    assert_eq!(points(&s, 1, Bloc::Military), 12.0);
    assert_eq!(points(&s, 0, Bloc::Military), 3.0);
    s.record_battle_result(1, false);
    assert_eq!(s.bloc_support(1, Bloc::Military), 1);
    s.record_battle_result(1, false);
    assert_eq!(s.bloc_support(1, Bloc::Military), 0);
    s.winner = Some(0);
    s.record_battle_result(0, true);
    assert_eq!(points(&s, 0, Bloc::Military), 3.0);
}
#[test]
fn wars_reduce_merchants_monthly_but_not_unrelated_military_confidence() {
    let c = SenateConfig::default();
    let mut s = SenateState::new(1);
    let mut p = actors(1);
    s.ensure_players(1);
    s.gain_confidence(0, Bloc::Merchants, 20.0);
    s.gain_confidence(0, Bloc::Military, 10.0);
    s.review_allegiances();
    s.advance_month(
        &mut p,
        &[PoliticalProfile {
            active_wars: 1.0,
            ..Default::default()
        }],
        &c,
    );
    assert_eq!(points(&s, 0, Bloc::Merchants), 19.5);
    assert_eq!(s.bloc_support(0, Bloc::Merchants), 1);
    assert_eq!(points(&s, 0, Bloc::Military), 10.0);
}
#[test]
fn neutral_seats_are_claimed_before_rival_confidence_and_capacity_is_shared() {
    let mut s = SenateState::new(1);
    s.ensure_players(2);
    s.gain_confidence(0, Bloc::Military, 100.0);
    s.gain_confidence(1, Bloc::Military, 100.0);
    s.review_allegiances();
    assert_eq!(s.bloc_support(0, Bloc::Military), 10);
    assert_eq!(s.bloc_support(1, Bloc::Military), 10);
    s.gain_confidence(0, Bloc::Military, 10.0);
    s.review_allegiances();
    assert_eq!(s.bloc_support(0, Bloc::Military), 11);
    assert_eq!(s.bloc_support(1, Bloc::Military), 9);
    assert_eq!(points(&s, 0, Bloc::Military) + points(&s, 1, Bloc::Military), 200.0);
    for seat in &s.senators {
        assert!(seat.confidence.iter().sum::<f64>() <= 10.0 + 1e-9);
    }
    let mut mixed = SenateState::new(1);
    mixed.ensure_players(2);
    mixed.add_to_seat(80, 1, 10.0);
    mixed.add_to_seat(80, 0, 1.0);
    mixed.gain_confidence(0, Bloc::Military, 5.0);
    assert_eq!(
        mixed.senators[80].confidence[1], 9.0,
        "Ordinary gains must use remaining neutral confidence before stealing from a rival"
    );
}
#[test]
fn equal_players_build_separate_support_and_the_seed_is_reproducible() {
    let play = || {
        let c = SenateConfig::default();
        let mut s = SenateState::new(31);
        let mut p = actors(2);
        let profile = PoliticalProfile {
            active_trade_routes: 1.0,
            ..Default::default()
        };
        for _ in 0..40 {
            s.advance_month(&mut p, &[profile.clone(), profile.clone()], &c);
        }
        assert_eq!(s.bloc_support(0, Bloc::Merchants), 2);
        assert_eq!(s.bloc_support(1, Bloc::Merchants), 2);
        s.senators.iter().map(|s| s.allegiance).collect::<Vec<_>>()
    };
    assert_eq!(play(), play());
}
#[test]
fn confidence_losses_release_support_immediately_and_do_not_reapply() {
    let c = SenateConfig::default();
    let mut s = SenateState::new(8);
    let mut p = actors(2);
    s.ensure_players(2);
    for b in Bloc::ALL {
        s.gain_confidence(1, b, 100.0);
    }
    s.review_allegiances();
    let mut e = evidence(1, ScandalKind::Famine, Severity::Major);
    s.expose_scandal(0, 1, &p, &mut e, &c).unwrap();
    assert!(e.scandals.is_empty());
    assert_eq!(s.bloc_support(1, Bloc::Populares), 4);
    assert_eq!(s.bloc_support(1, Bloc::Provincials), 4);
    assert_eq!(s.bloc_support(1, Bloc::Merchants), 10);
    for _ in 0..12 {
        s.advance_month(&mut p, &[], &c);
    }
    assert_eq!(s.bloc_support(1, Bloc::Populares), 4);
    assert!(s.accusations.is_empty());
}
#[test]
fn three_severities_increase_immediate_losses_and_affect_at_most_two_factions() {
    for kind in [
        ScandalKind::Famine,
        ScandalKind::HarshLabor,
        ScandalKind::PoliticalBribery,
        ScandalKind::SenatorMurder,
        ScandalKind::MilitaryIncompetence,
    ] {
        let amounts = [Severity::Minor, Severity::Medium, Severity::Major].map(|severity| {
            let losses = kind.senate_losses(severity);
            assert!((1..=2).contains(&losses.iter().filter(|v| **v > 0.0).count()));
            losses.iter().sum::<f64>()
        });
        assert!(amounts[0] < amounts[1] && amounts[1] < amounts[2]);
    }
}
#[test]
fn invalid_or_used_evidence_never_mutates_or_consumes_other_scandals() {
    let c = SenateConfig::default();
    let mut s = SenateState::new(8);
    let p = actors(2);
    let mut e = evidence(0, ScandalKind::Famine, Severity::Major);
    assert_eq!(s.expose_scandal(0, 1, &p, &mut e, &c), Err(PoliticalError::Ineligible));
    e.scandals[0].target = ScandalTarget::Player(1);
    e.scandals[0].expires = 0;
    assert_eq!(s.expose_scandal(0, 1, &p, &mut e, &c), Err(PoliticalError::ScandalRequired));
    assert_eq!(e.scandals.len(), 1);
    assert!(s.accusations.is_empty());
}
#[test]
fn trade_breach_releases_one_merchant_seat_immediately() {
    let mut s = SenateState::new(1);
    s.ensure_players(1);
    s.gain_confidence(0, Bloc::Merchants, 30.0);
    s.review_allegiances();
    s.record_trade_breach(0);
    assert_eq!(s.bloc_support(0, Bloc::Merchants), 2);
}
#[test]
fn personal_actions_cost_funds_and_cannot_repeat_or_roll_when_invalid() {
    let c = safe_config();
    let mut s = SenateState::new(1);
    let mut p = actors(2);
    p[0].coin = 39.0;
    assert_eq!(
        s.act_on_senator(0, 0, SenatorAction::Gift, &mut p, &c).unwrap_err(),
        PoliticalError::InsufficientFunds
    );
    assert_eq!(p[0].coin, 39.0);
    assert!(s.senators[0].confidence.is_empty());
    s.act_on_senator(0, 0, SenatorAction::Petition, &mut p, &c).unwrap();
    assert_eq!(p[0].influence, 4990.0);
    assert_eq!(s.senators[0].confidence[0], 3.0);
    assert_eq!(
        s.act_on_senator(1, 0, SenatorAction::Bribe, &mut p, &c).unwrap_err(),
        PoliticalError::AlreadyUsed
    );
    assert_eq!(p[1].coin, 10_000.0);
}
#[test]
fn patronage_and_threats_apply_monthly_for_exactly_six_months_and_cap_at_one_seat() {
    let c = safe_config();
    let mut s = SenateState::new(1);
    let mut p = actors(1);
    s.act_on_senator(0, 0, SenatorAction::Patronage, &mut p, &c).unwrap();
    s.act_on_senator(0, 80, SenatorAction::Threaten, &mut p, &c).unwrap();
    assert_eq!(s.support(0), 0);
    for _ in 0..6 {
        s.advance_month(&mut p, &[], &c);
    }
    assert_eq!(s.senators[80].confidence[0], 10.0);
    // Coercion also reduces Aristocrat confidence during the arrangement.
    assert!(s.senators[0].confidence[0] < 6.0);
    assert!(s.senators[0].arrangement.is_none());
    assert!(s.senators[80].arrangement.is_none());
    let before = s.senators[80].confidence[0];
    s.advance_month(&mut p, &[], &c);
    assert_eq!(s.senators[80].confidence[0], before);
}
#[test]
fn murder_resets_all_players_and_replacement_does_not_inherit_arrangements() {
    let c = safe_config();
    let mut s = SenateState::new(1);
    let mut p = actors(2);
    s.ensure_players(2);
    s.add_to_seat(0, 1, 10.0);
    s.add_to_seat(0, 0, 3.0);
    s.review_allegiances();
    let outcome = s.act_on_senator(0, 0, SenatorAction::Assassinate, &mut p, &c).unwrap();
    assert!(!outcome.caught);
    assert_eq!(outcome.misconduct, Some((ScandalKind::SenatorMurder, Severity::Major)));
    assert_eq!(s.senators[0].confidence, vec![0.0, 0.0]);
    assert_eq!(s.senators[0].allegiance, None);
    assert_eq!(s.senators[0].generation, 1);
    assert_eq!(s.senators[0].id, 0);
    s.advance_month(&mut p, &[], &c);
    assert_eq!(s.senators[0].allegiance, None);
}
#[test]
fn detected_bribe_has_larger_immediate_losses_than_its_gain_and_is_seeded() {
    let c = SenateConfig {
        action_risks: [1.0; 9],
        ..Default::default()
    };
    let mut s = SenateState::new(1);
    let mut p = actors(1);
    s.ensure_players(1);
    s.gain_confidence(0, Bloc::Aristocrats, 100.0);
    s.review_allegiances();
    let outcome = s.act_on_senator(0, 10, SenatorAction::Bribe, &mut p, &c).unwrap();
    assert!(outcome.caught);
    assert!(points(&s, 0, Bloc::Aristocrats) < 100.0);
    assert_eq!(s.accusations.len(), 1);
    assert_eq!(p[0].coin, 9920.0);
    let run = || {
        let mut s = SenateState::new(91);
        let mut p = actors(1);
        SenatorAction::ALL.map(|a| {
            s.act_on_senator(0, a as usize, a, &mut p, &SenateConfig::default())
                .ok()
                .map(|o| o.caught)
        })
    };
    assert_eq!(run(), run());
}
#[test]
fn banquet_and_discredit_are_targeted_and_do_not_create_free_currency() {
    let c = safe_config();
    let mut s = SenateState::new(1);
    let mut p = actors(2);
    s.act_on_senator(0, 0, SenatorAction::Banquet, &mut p, &c).unwrap();
    assert_eq!(s.senators[0].confidence[0], 5.0);
    assert_eq!(points(&s, 0, Bloc::Populares), 5.0);
    s.add_to_seat(1, 1, 10.0);
    s.review_allegiances();
    s.act_on_senator(0, 1, SenatorAction::Discredit, &mut p, &c).unwrap();
    assert_eq!(s.senators[1].confidence[1], 5.0);
    assert_eq!(s.senators[1].allegiance, None);
    assert_eq!(p[0].coin, 9930.0);
    assert_eq!(p[0].influence, 4965.0);
}
#[test]
fn outreach_is_nonstacking_recurring_and_runs_for_exactly_six_months() {
    let c = SenateConfig::default();
    let mut s = SenateState::new(5);
    let mut p = actors(1);
    s.court(0, Bloc::Military, &mut p, &c).unwrap();
    assert_eq!(s.court(0, Bloc::Military, &mut p, &c), Err(PoliticalError::AlreadyUsed));
    for _ in 0..6 {
        s.advance_month(&mut p, &[], &c);
    }
    assert_eq!(points(&s, 0, Bloc::Military), 6.0);
    s.advance_month(&mut p, &[], &c);
    assert_eq!(points(&s, 0, Bloc::Military), 6.0);
}

#[test]
fn lobbying_spends_only_influence_and_stops_monthly_payments_when_full() {
    let c = safe_config();
    let mut s = SenateState::new(1);
    let mut p = actors(1);
    s.act_on_senator(0, 0, SenatorAction::Lobby, &mut p, &c).unwrap();
    assert_eq!(p[0].influence, 4980.0);
    assert_eq!(p[0].coin, 10_000.0);
    assert_eq!(points(&s, 0, Bloc::Aristocrats), 0.0);
    assert_eq!(s.outreach_upkeep(0, &c), 4.0);
    assert_eq!(s.spent_on(0, SenatorAction::Lobby), 20.0);
    for _ in 0..10 {
        s.pay_outreach(&mut p, &c);
        let paid = p[0].influence;
        s.pay_outreach(&mut p, &c);
        assert_eq!(p[0].influence, paid, "Do not pay twice in one month");
        s.advance_month(&mut p, &[], &c);
    }
    assert_eq!(p[0].influence, 4940.0);
    assert_eq!(p[0].coin, 10_000.0);
    assert_eq!(s.senators[0].allegiance, Some(0));
    assert!(s.senators[0].arrangement.is_none());
    assert_eq!(s.outreach_upkeep(0, &c), 0.0);
    s.pay_outreach(&mut p, &c);
    assert_eq!(p[0].influence, 4940.0);
}

#[test]
fn unpaid_lobbying_gives_no_favor_and_cancellation_removes_outflow() {
    let c = safe_config();
    let mut s = SenateState::new(1);
    let mut p = actors(1);
    p[0].influence = 23.0;
    s.act_on_senator(0, 0, SenatorAction::Lobby, &mut p, &c).unwrap();
    s.pay_outreach(&mut p, &c);
    s.advance_month(&mut p, &[], &c);
    assert_eq!(p[0].influence, 3.0);
    assert_eq!(points(&s, 0, Bloc::Aristocrats), 0.0);
    p[0].influence = 4.0;
    s.pay_outreach(&mut p, &c);
    s.advance_month(&mut p, &[], &c);
    assert_eq!(p[0].influence, 0.0);
    assert_eq!(points(&s, 0, Bloc::Aristocrats), 1.0);
    assert_eq!(s.end_outreach(1, 0), Err(PoliticalError::Ineligible));
    s.end_outreach(0, 0).unwrap();
    assert_eq!(s.outreach_upkeep(0, &c), 0.0);
    p[0].influence = 100.0;
    s.pay_outreach(&mut p, &c);
    s.advance_month(&mut p, &[], &c);
    assert_eq!(p[0].influence, 100.0);
    assert_eq!(points(&s, 0, Bloc::Aristocrats), 1.0);
}
#[test]
fn promotions_are_atomic_pay_once_and_never_consume_supporters() {
    let c = SenateConfig::default();
    let mut s = SenateState::new(1);
    let mut p = actors(2);
    assert_eq!(s.promote(0, &mut p, &c).unwrap_err(), PoliticalError::InsufficientSupport);
    assert_eq!(p[0].influence, 5000.0);
    pledge(&mut s, 0, 10);
    s.promote(0, &mut p, &c).unwrap();
    assert_eq!(p[0].rank, PoliticalRank::Aedile);
    assert_eq!(s.support(0), 10);
    assert_eq!(p[0].influence, 4500.0);
    pledge(&mut s, 0, 20);
    assert_eq!(s.promote(0, &mut p, &c).unwrap_err(), PoliticalError::AlreadyUsed);
    s.month += 1;
    p[0].influence = 999.0;
    assert_eq!(s.promote(0, &mut p, &c).unwrap_err(), PoliticalError::InsufficientFunds);
    assert_eq!(p[0].influence, 999.0);
}
#[test]
fn consul_slots_term_cooldown_and_reappointment_gate_are_authoritative() {
    let c = SenateConfig::default();
    let mut s = SenateState::new(7);
    let mut p = actors(3);
    p[0].rank = PoliticalRank::Censor;
    for actor in &mut p[1..] {
        actor.rank = PoliticalRank::Consul;
        actor.consul_until = Some(1);
    }
    pledge(&mut s, 0, 40);
    assert_eq!(s.promote(0, &mut p, &c).unwrap_err(), PoliticalError::NoConsulSeat);
    s.advance_month(&mut p, &[], &c);
    assert_eq!(p[1].rank, PoliticalRank::Proconsul);
    assert_eq!(p[1].consul_again_at, 13);
    pledge(&mut s, 0, 40);
    s.promote(0, &mut p, &c).unwrap();
    assert_eq!(p[0].consul_until, Some(25));
    assert_eq!(s.promote(1, &mut p, &c).unwrap_err(), PoliticalError::ConsulCooldown);
    s.month = 13;
    pledge(&mut s, 1, 40);
    assert_eq!(s.promotion_eligibility(1, &p, &c).unwrap().rank, PoliticalRank::Consul);
    s.promote(1, &mut p, &c).unwrap();
    assert_eq!(p[1].consul_until, Some(37));
}
#[test]
fn lost_consul_confidence_has_grace_but_exposed_scandals_accelerate_removal() {
    let c = SenateConfig::default();
    let mut s = SenateState::new(1);
    let mut p = actors(2);
    p[0].rank = PoliticalRank::Consul;
    p[0].consul_until = Some(24);
    for _ in 0..2 {
        s.advance_month(&mut p, &[], &c);
        assert_eq!(p[0].rank, PoliticalRank::Consul);
    }
    let events = s.advance_month(&mut p, &[], &c);
    assert!(events.iter().any(|e| matches!(e, SenateEvent::ConsulRemoved(0))));
    p[1].rank = PoliticalRank::Consul;
    p[1].consul_until = Some(24);
    let mut e = evidence(1, ScandalKind::Famine, Severity::Major);
    s.expose_scandal(0, 1, &p, &mut e, &c).unwrap();
    assert!(s
        .advance_month(&mut p, &[], &c)
        .iter()
        .any(|e| matches!(e, SenateEvent::ConsulRemoved(1))));
}
#[test]
fn augustus_requires_active_consul_and_victory_stops_all_actions_and_ticks() {
    let c = SenateConfig::default();
    let mut s = SenateState::new(1);
    let mut p = actors(2);
    p[0].rank = PoliticalRank::Proconsul;
    pledge(&mut s, 0, 100);
    s.promote(0, &mut p, &c).unwrap();
    s.month += 1;
    s.promote(0, &mut p, &c).unwrap();
    assert_eq!(s.winner, Some(0));
    assert_eq!(p[0].rank, PoliticalRank::Augustus);
    assert!(s.advance_month(&mut p, &[], &c).is_empty());
    assert_eq!(s.month, 1);
    assert_eq!(
        s.act_on_senator(0, 99, SenatorAction::Assassinate, &mut p, &c).unwrap_err(),
        PoliticalError::Ineligible
    );
}
#[test]
fn arbitrary_months_keep_confidence_finite_bounded_and_support_exclusive() {
    let c = SenateConfig::default();
    let mut s = SenateState::new(11);
    let mut p = actors(8);
    for month in 0..240 {
        let profiles: Vec<_> = (0..8)
            .map(|player| PoliticalProfile {
                active_trade_routes: ((month + player) % 11) as f64,
                active_wars: ((month + player) % 3) as f64,
                military_rank: (player % 4) as f64,
                controlled_provinces: player as f64,
                citizen_happiness: ((month * 7 + player) % 101) as f64,
                ..Default::default()
            })
            .collect();
        s.advance_month(&mut p, &profiles, &c);
        for seat in &s.senators {
            assert!(seat
                .confidence
                .iter()
                .all(|v| v.is_finite() && *v >= 0.0 && *v <= 10.0 + 1e-9));
            assert!(seat.confidence.iter().sum::<f64>() <= 10.0 + 1e-9);
            if let Some(player) = seat.allegiance {
                assert!(seat.confidence[player] >= 10.0 - 1e-9);
            }
        }
        assert!((0..8).map(|player| s.support(player)).sum::<usize>() <= 100);
    }
}
