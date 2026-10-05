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

#[test]
fn practice_support_prefers_neutral_seats_and_keeps_existing_support() {
    let mut senate = SenateState::new(1);
    pledge(&mut senate, 0, 20);
    senate.grant_practice_support(1, 10);
    assert_eq!(senate.support(0), 20);
    assert_eq!(senate.support(1), 10);
    senate.grant_practice_support(1, 5);
    assert_eq!(senate.support(1), 10);
    senate.review_allegiances();
    assert_eq!(senate.support(1), 10);
}

#[test]
fn practice_support_transfers_full_confidence_when_all_seats_are_taken() {
    let mut senate = SenateState::new(1);
    pledge(&mut senate, 0, 100);
    senate.grant_practice_support(1, 60);
    assert_eq!(senate.support(0), 40);
    assert_eq!(senate.support(1), 60);
    for seat in &senate.senators {
        assert!(seat.confidence.iter().sum::<f64>() <= senate.seat_confidence + 1e-9);
    }
    senate.review_allegiances();
    assert_eq!(senate.support(1), 60);
    assert_eq!(senate.senators.len(), 100);
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
fn zero_income_and_a_home_province_never_grant_passive_support() {
    let config = SenateConfig::default();
    let mut senate = SenateState::new(1);
    let players = actors(1);
    let profile = PoliticalProfile {
        nobles: 1000.0,
        controlled_provinces: 1.0,
        ..Default::default()
    };
    for _ in 0..600 {
        senate.advance_month(&players, std::slice::from_ref(&profile), &config);
    }
    for bloc in Bloc::ALL {
        assert_eq!(points(&senate, 0, bloc), 0.0, "{bloc:?}");
        assert_eq!(senate.bloc_support(0, bloc), 0);
    }
}

#[test]
fn merchant_income_contributes_from_the_first_coin_and_keeps_growing() {
    let actor = PoliticalPlayer::default();
    let rate = |coin_income| {
        structural_reasons(
            Bloc::Merchants,
            &PoliticalProfile {
                coin_income,
                ..Default::default()
            },
            &actor,
        )
        .iter()
        .map(|reason| reason.points)
        .sum::<f64>()
    };
    assert!(rate(-10.0) < 0.0);
    assert_eq!(rate(0.0), 0.0);
    let mut previous = 0.0;
    for income in [0.01, 1.0, 10.0, 29.99, 30.0, 30.01, 60.0, 600.0, 6000.0, 60_000.0] {
        let current = rate(income);
        assert!(current > previous, "Income {income} must increase support");
        previous = current;
    }
    assert!(rate(6000.0) > 4.0, "Large profits must not saturate below ordinary penalties");
}

#[test]
fn high_income_earns_merchant_seats_despite_shortages_and_war() {
    let config = SenateConfig::default();
    let mut senate = SenateState::new(1);
    let players = actors(1);
    let profile = PoliticalProfile {
        coin_income: 6000.0,
        resource_security: 0.0,
        active_wars: 1.0,
        ..Default::default()
    };
    for _ in 0..4 {
        senate.advance_month(&players, std::slice::from_ref(&profile), &config);
    }
    assert!(senate.bloc_support(0, Bloc::Merchants) > 0);
}

#[test]
fn each_faction_earns_support_from_its_actual_positive_effects() {
    let baseline = PoliticalProfile {
        controlled_provinces: 1.0,
        ..Default::default()
    };
    let mut cases = vec![];
    macro_rules! effect {
        ($bloc:ident, $field:ident, $value:expr) => {
            cases.push((
                Bloc::$bloc,
                stringify!($field),
                PoliticalProfile {
                    $field: $value,
                    ..baseline.clone()
                },
                PoliticalRank::Quaestor,
            ));
        };
    }
    effect!(Aristocrats, noble_happiness, 60.0);
    effect!(Aristocrats, political_buildings, 1.0);
    effect!(Aristocrats, wonders, 1.0);
    cases.push((Bloc::Aristocrats, "higher office", baseline.clone(), PoliticalRank::Aedile));
    effect!(Merchants, coin_income, 60.0);
    effect!(Merchants, active_trade_routes, 1.0);
    effect!(Merchants, markets, 1.0);
    effect!(Provincials, vassal_count, 1.0);
    effect!(Provincials, province_relation, 70.0);
    effect!(Provincials, provincial_trade, 40.0);
    effect!(Populares, citizen_happiness, 60.0);
    effect!(Populares, plebeian_happiness, 60.0);
    effect!(Populares, food_policy, 1.0);
    effect!(Populares, food_reserve_months, 6.0);
    effect!(Military, military_strength, 100.0);
    effect!(Military, military_rank, 1.0);
    effect!(Military, recent_victories, 1.0);
    effect!(Military, controlled_provinces, 2.0);

    let config = SenateConfig::default();
    for (bloc, effect, profile, rank) in cases {
        let mut senate = SenateState::new(1);
        let mut players = actors(1);
        players[0].rank = rank;
        for _ in 0..120 {
            senate.advance_month(&players, std::slice::from_ref(&profile), &config);
        }
        assert!(senate.bloc_support(0, bloc) > 0, "{bloc:?}: {effect}");
        for other in Bloc::ALL.into_iter().filter(|other| *other != bloc) {
            assert_eq!(points(&senate, 0, other), 0.0, "{other:?}: {effect}");
        }
    }
}

#[test]
fn larger_noble_populations_only_amplify_happy_nobles() {
    let actor = PoliticalPlayer::default();
    let rate = |nobles, noble_happiness| {
        structural_reasons(
            Bloc::Aristocrats,
            &PoliticalProfile {
                nobles,
                noble_happiness,
                ..Default::default()
            },
            &actor,
        )
        .iter()
        .map(|reason| reason.points)
        .sum::<f64>()
    };
    assert_eq!(rate(100.0, 50.0), 0.0);
    assert_eq!(rate(1000.0, 50.0), 0.0);
    assert!(rate(1000.0, 60.0) > rate(100.0, 60.0));
    assert!(rate(1000.0, 40.0) < 0.0);
}

#[test]
fn generous_food_rewards_actual_supply_and_requires_a_governed_population() {
    let actor = PoliticalPlayer::default();
    let policy = |controlled_provinces, food_security| {
        structural_reasons(
            Bloc::Populares,
            &PoliticalProfile {
                controlled_provinces,
                food_policy: 1.0,
                food_security,
                ..Default::default()
            },
            &actor,
        )
        .into_iter()
        .find(|reason| reason.label == "Generous or restricted food policy")
        .unwrap()
        .points
    };
    assert_eq!(policy(0.0, 1.0), 0.0);
    assert_eq!(policy(1.0, 0.0), 0.0);
    assert_eq!(policy(1.0, 0.5), policy(1.0, 1.0) * 0.5);
    assert!(policy(1.0, 1.0) > 0.0);
}

#[test]
fn food_reserves_build_populares_support_at_normal_rations() {
    let actor = PoliticalPlayer::default();
    let reserve = |controlled_provinces, food_security, food_reserve_months| {
        structural_reasons(
            Bloc::Populares,
            &PoliticalProfile {
                controlled_provinces,
                food_security,
                food_reserve_months,
                ..Default::default()
            },
            &actor,
        )
        .into_iter()
        .find(|reason| reason.label == "Ample food reserves")
        .unwrap()
        .points
    };
    assert_eq!(reserve(0.0, 1.0, 6.0), 0.0);
    assert_eq!(reserve(1.0, 1.0, 0.0), 0.0);
    assert_eq!(reserve(1.0, 0.0, 6.0), 0.0);
    assert!(reserve(1.0, 1.0, 3.0) > 0.0);
    assert!(reserve(1.0, 1.0, 6.0) > reserve(1.0, 1.0, 3.0));
    assert_eq!(reserve(1.0, 0.5, 6.0), 0.5 * reserve(1.0, 1.0, 6.0));
}

#[test]
fn each_faction_loses_confidence_from_its_negative_conditions() {
    let baseline = PoliticalProfile {
        controlled_provinces: 1.0,
        ..Default::default()
    };
    let mut cases = vec![];
    macro_rules! effect {
        ($bloc:ident, $field:ident, $value:expr) => {
            cases.push((
                Bloc::$bloc,
                stringify!($field),
                PoliticalProfile {
                    $field: $value,
                    ..baseline.clone()
                },
            ));
        };
    }
    effect!(Aristocrats, noble_happiness, 40.0);
    effect!(Merchants, coin_income, -10.0);
    effect!(Merchants, trade_reliability, 0.0);
    effect!(Merchants, resource_security, 0.0);
    effect!(Merchants, active_wars, 1.0);
    effect!(Provincials, province_relation, 30.0);
    effect!(Provincials, high_tribute, 1.0);
    effect!(Provincials, active_wars, 1.0);
    effect!(Populares, citizen_happiness, 40.0);
    effect!(Populares, plebeian_happiness, 40.0);
    effect!(Populares, food_policy, -1.0);
    effect!(Populares, food_security, 0.5);
    effect!(Populares, famine, 1.0);
    effect!(Populares, tax_pressure, 1.0);
    effect!(Populares, harsh_policies, 1.0);
    effect!(Military, recent_victories, -1.0);

    let config = SenateConfig::default();
    for (bloc, effect, profile) in cases {
        let mut senate = SenateState::new(1);
        let players = actors(1);
        senate.ensure_players(1);
        senate.gain_confidence(0, bloc, 10.0);
        senate.review_allegiances();
        assert_eq!(senate.bloc_support(0, bloc), 1);
        senate.advance_month(&players, &[profile], &config);
        assert!(points(&senate, 0, bloc) < 10.0, "{bloc:?}: {effect}");
        assert_eq!(senate.bloc_support(0, bloc), 0, "{bloc:?}: {effect}");
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
    let p = actors(1);
    let profiles = [PoliticalProfile {
        active_trade_routes: 1.0,
        ..Default::default()
    }];
    for _ in 0..19 {
        s.advance_month(&p, &profiles, &c);
    }
    assert_eq!(points(&s, 0, Bloc::Merchants), 9.5);
    assert_eq!(s.support(0), 0);
    s.advance_month(&p, &profiles, &c);
    assert_eq!(s.bloc_support(0, Bloc::Merchants), 1);
    s.advance_month(&p, &[PoliticalProfile::default()], &c);
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
    let p = actors(1);
    s.ensure_players(1);
    s.gain_confidence(0, Bloc::Merchants, 20.0);
    s.gain_confidence(0, Bloc::Military, 10.0);
    s.review_allegiances();
    s.advance_month(
        &p,
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
        let p = actors(2);
        let profile = PoliticalProfile {
            active_trade_routes: 1.0,
            ..Default::default()
        };
        for _ in 0..40 {
            s.advance_month(&p, &[profile.clone(), profile.clone()], &c);
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
    let p = actors(2);
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
        s.advance_month(&p, &[], &c);
    }
    assert_eq!(s.bloc_support(1, Bloc::Populares), 4);
    assert!(s.accusations.is_empty());
}
#[test]
fn exposed_population_unhappiness_hurts_populares_without_hurting_provincials() {
    for kind in [ScandalKind::UnhappyCitizens, ScandalKind::UnhappyPlebeians] {
        let config = SenateConfig::default();
        let mut senate = SenateState::new(8);
        let players = actors(2);
        pledge(&mut senate, 1, 100);
        let mut espionage = evidence(1, kind, Severity::Medium);
        senate.expose_scandal(0, 1, &players, &mut espionage, &config).unwrap();
        assert_eq!(senate.bloc_support(1, Bloc::Provincials), 20);
        assert_eq!(senate.bloc_support(1, Bloc::Populares), 17);
        assert_eq!(kind.bloc_penalties(Severity::Medium)[Bloc::Provincials.index()], 0.0);
    }
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
fn temporary_actions_charge_once_and_allow_same_month_competition() {
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
    s.act_on_senator(0, 0, SenatorAction::Petition, &mut p, &c).unwrap();
    s.act_on_senator(1, 0, SenatorAction::Gift, &mut p, &c).unwrap();
    assert_eq!(s.month, 0);
    assert_eq!(s.senators[0].confidence, vec![0.0, 0.0]);
    assert_eq!(p[0].influence, 4980.0);
    assert_eq!(p[1].coin, 9960.0);
    s.advance_month(&p, &[], &c);
    assert_eq!(s.senators[0].confidence, vec![2.0, 1.25]);
    assert_eq!(s.senators[0].allegiance, None);
}
#[test]
fn ongoing_actions_are_exclusive_per_player_and_invalid_clicks_do_not_charge_or_roll() {
    let c = safe_config();
    for first in SenatorAction::ALL.into_iter().filter(|a| a.is_ongoing()) {
        let mut s = SenateState::new(17);
        let mut p = actors(2);
        s.act_on_senator(0, 0, first, &mut p, &c).unwrap();
        let balances: Vec<_> = p.iter().map(|actor| (actor.coin, actor.influence)).collect();
        let confidence = s.senators[0].confidence.clone();
        let mut expected_rng = s.rng.clone();
        for repeat in SenatorAction::ALL.into_iter().filter(|a| a.is_ongoing()) {
            assert_eq!(
                s.act_on_senator(0, 0, repeat, &mut p, &c).unwrap_err(),
                PoliticalError::SenatorArrangementActive {
                    action: first,
                    months_remaining: first.duration(),
                },
                "{first:?} must block another ongoing {repeat:?} by the same player"
            );
        }
        assert_eq!(
            p.iter().map(|actor| (actor.coin, actor.influence)).collect::<Vec<_>>(),
            balances
        );
        assert_eq!(s.senators[0].confidence, confidence);
        assert_eq!(s.rng.unit(), expected_rng.unit(), "Rejected clicks must not reroll exposure");
        s.act_on_senator(1, 0, SenatorAction::Lobby, &mut p, &c).unwrap();
        s.act_on_senator(0, 0, SenatorAction::Gift, &mut p, &c).unwrap();
        s.act_on_senator(1, 0, SenatorAction::Petition, &mut p, &c).unwrap();
        assert_eq!(s.senators[0].arrangement(0).unwrap().action, first);
        assert_eq!(s.senators[0].arrangement(1).unwrap().action, SenatorAction::Lobby);
        s.cancel_senator_action(0, 0).unwrap();
        assert!(s.senators[0].arrangement(1).is_some());
        assert!(s.senator_action_quote(0, 0, SenatorAction::Lobby, &p, &c).is_ok());
    }
}

#[test]
fn unavailable_senator_actions_explain_the_actual_allegiance_and_relationship() {
    let c = safe_config();
    let mut s = SenateState::new(1);
    let mut p = actors(2);
    let neutral = s.senator_action_quote(0, 0, SenatorAction::Discredit, &p, &c).unwrap_err();
    assert_eq!(neutral, PoliticalError::SenatorIsNeutral);
    assert!(neutral.to_string().contains("neutral"));
    assert!(neutral.to_string().contains("supports another player"));
    pledge(&mut s, 0, 1);
    let own_patron = s.senator_action_quote(0, 0, SenatorAction::Discredit, &p, &c).unwrap_err();
    assert_eq!(own_patron, PoliticalError::RivalPatronRequired);
    assert!(own_patron.to_string().contains("supports you"));
    assert!(s.senator_action_quote(1, 0, SenatorAction::Discredit, &p, &c).is_ok());
    assert!(s.senator_action_quote(0, 0, SenatorAction::Gift, &p, &c).is_ok());
    assert!(s.senator_action_quote(0, 0, SenatorAction::Lobby, &p, &c).is_ok());
    for active in SenatorAction::ALL.into_iter().filter(|a| a.is_ongoing()) {
        let mut s = SenateState::new(1);
        s.act_on_senator(0, 0, active, &mut p, &c).unwrap();
        s.advance_month(&p, &[], &c);
        for replacement in SenatorAction::ALL.into_iter().filter(|a| a.is_ongoing()) {
            assert!(s.senator_action_quote(1, 0, replacement, &p, &c).is_ok());
            let error = s.senator_action_quote(0, 0, replacement, &p, &c).unwrap_err();
            assert_eq!(
                error,
                PoliticalError::SenatorArrangementActive {
                    action: active,
                    months_remaining: s.senators[0].arrangement(0).unwrap().until - s.month,
                }
            );
            if active.upkeep(&c).is_some() {
                assert!(error.to_string().contains("cross"));
            } else {
                assert!(error.to_string().contains("5 months remaining"));
            }
        }
    }
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
        s.advance_month(&p, &[], &c);
    }
    assert_eq!(s.senators[80].confidence[0], 9.5);
    // Coercion also reduces Aristocrat confidence during the arrangement.
    assert!(s.senators[0].confidence[0] < 6.0);
    assert!(s.senators[0].arrangement(0).is_none());
    assert!(s.senators[80].arrangement(0).is_none());
    let before = s.senators[80].confidence[0];
    s.advance_month(&p, &[], &c);
    assert_eq!(s.senators[80].confidence[0], before - c.personal_confidence_decay);
}
#[test]
fn murder_resets_all_players_and_replacement_does_not_inherit_arrangements() {
    let c = safe_config();
    let mut s = SenateState::new(1);
    let mut p = actors(2);
    s.ensure_players(2);
    s.act_on_senator(0, 0, SenatorAction::Lobby, &mut p, &c).unwrap();
    s.act_on_senator(1, 0, SenatorAction::Bribe, &mut p, &c).unwrap();
    s.act_on_senator(1, 0, SenatorAction::Gift, &mut p, &c).unwrap();
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
    assert!(s.senators[0].arrangements.is_empty());
    assert!(s.senators[0].effects.is_empty());
    s.advance_month(&p, &[], &c);
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
fn banquet_and_discredit_apply_temporary_effects_only_to_the_selected_senator() {
    let c = safe_config();
    let mut s = SenateState::new(1);
    let mut p = actors(2);
    s.act_on_senator(0, 0, SenatorAction::Banquet, &mut p, &c).unwrap();
    assert_eq!(s.senators[0].confidence[0], 0.0);
    assert_eq!(points(&s, 0, Bloc::Populares), 0.0);
    assert_eq!(s.support(0), 0);
    s.add_to_seat(1, 1, 10.0);
    s.review_allegiances();
    s.act_on_senator(0, 1, SenatorAction::Discredit, &mut p, &c).unwrap();
    assert_eq!(s.senators[1].confidence[1], 10.0);
    s.advance_month(&p, &[], &c);
    assert_eq!(s.senators[0].confidence[0], 1.5);
    assert_eq!(s.senators[1].confidence[1], 9.0);
    assert_eq!(s.senators[1].allegiance, None);
    assert_eq!(points(&s, 0, Bloc::Populares), 0.0);
    for _ in 0..3 {
        s.advance_month(&p, &[], &c);
    }
    assert_eq!(s.senators[0].confidence[0], 4.5);
    assert_eq!(s.senators[1].confidence[1], 7.0);
    assert!(s.senators[0].effects.is_empty());
    for _ in 0..9 {
        s.advance_month(&p, &[], &c);
    }
    assert_eq!(s.senators[0].confidence[0], 0.0);
    assert_eq!(s.senators[1].confidence[1], 7.0);
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
        s.advance_month(&p, &[], &c);
    }
    assert_eq!(points(&s, 0, Bloc::Military), 6.0);
    s.advance_month(&p, &[], &c);
    assert_eq!(points(&s, 0, Bloc::Military), 6.0);
}

#[test]
fn lobbying_builds_and_maintains_support_until_cancelled_then_confidence_fades() {
    let c = safe_config();
    let mut s = SenateState::new(1);
    let mut p = actors(1);
    s.act_on_senator(0, 0, SenatorAction::Lobby, &mut p, &c).unwrap();
    assert_eq!(p[0].influence, 4980.0);
    assert_eq!(p[0].coin, 10_000.0);
    assert_eq!(points(&s, 0, Bloc::Aristocrats), 0.0);
    assert_eq!(s.outreach_upkeep(0, &c), 4.0);
    assert_eq!(s.spent_on(0, SenatorAction::Lobby), 20.0);
    for _ in 0..20 {
        s.pay_outreach(&mut p, &c);
        let paid = p[0].influence;
        s.pay_outreach(&mut p, &c);
        assert_eq!(p[0].influence, paid, "Do not pay twice in one month");
        s.advance_month(&p, &[], &c);
    }
    assert_eq!(p[0].influence, 4900.0);
    assert_eq!(p[0].coin, 10_000.0);
    assert_eq!(s.senators[0].allegiance, Some(0));
    assert!(s.senators[0].arrangement(0).is_some());
    assert_eq!(s.outreach_upkeep(0, &c), 4.0);
    s.cancel_senator_action(0, 0).unwrap();
    assert_eq!(s.outreach_upkeep(0, &c), 0.0);
    s.pay_outreach(&mut p, &c);
    assert_eq!(p[0].influence, 4900.0);
    s.advance_month(&p, &[], &c);
    assert_eq!(s.senators[0].allegiance, None);
    for _ in 0..19 {
        s.advance_month(&p, &[], &c);
    }
    assert_eq!(s.senators[0].confidence[0], 0.0);
}

#[test]
fn unpaid_lobbying_gives_no_favor_and_cancellation_removes_outflow() {
    let c = safe_config();
    let mut s = SenateState::new(1);
    let mut p = actors(1);
    p[0].influence = 23.0;
    s.act_on_senator(0, 0, SenatorAction::Lobby, &mut p, &c).unwrap();
    s.pay_outreach(&mut p, &c);
    s.advance_month(&p, &[], &c);
    assert_eq!(p[0].influence, 3.0);
    assert_eq!(points(&s, 0, Bloc::Aristocrats), 0.0);
    p[0].influence = 4.0;
    s.pay_outreach(&mut p, &c);
    s.advance_month(&p, &[], &c);
    assert_eq!(p[0].influence, 0.0);
    assert_eq!(points(&s, 0, Bloc::Aristocrats), 1.0);
    assert_eq!(s.cancel_senator_action(1, 0), Err(PoliticalError::Ineligible));
    s.cancel_senator_action(0, 0).unwrap();
    assert_eq!(s.outreach_upkeep(0, &c), 0.0);
    p[0].influence = 100.0;
    s.pay_outreach(&mut p, &c);
    s.advance_month(&p, &[], &c);
    assert_eq!(p[0].influence, 100.0);
    assert_eq!(points(&s, 0, Bloc::Aristocrats), 0.5);
}

#[test]
fn recurring_bribes_build_faster_than_lobbying_and_unpaid_months_only_decay() {
    let c = safe_config();
    let mut s = SenateState::new(1);
    let mut p = actors(2);
    s.act_on_senator(0, 0, SenatorAction::Bribe, &mut p, &c).unwrap();
    s.act_on_senator(1, 0, SenatorAction::Lobby, &mut p, &c).unwrap();
    assert_eq!(s.senators[0].confidence, vec![0.0, 0.0]);
    assert_eq!(s.action_upkeep(0, SenatorAction::Bribe, &c), 8.0);
    let payments = s.pay_outreach(&mut p, &c);
    assert_eq!(payments.len(), 1);
    assert!(!payments[0].caught);
    assert_eq!(p[0].coin, 9912.0);
    assert_eq!(p[1].influence, 4976.0);
    let mut expected_rng = s.rng.clone();
    assert!(s.pay_outreach(&mut p, &c).is_empty());
    assert_eq!(p[0].coin, 9912.0);
    assert_eq!(s.rng.unit(), expected_rng.unit());
    s.advance_month(&p, &[], &c);
    assert_eq!(s.senators[0].confidence, vec![2.0, 1.0]);
    p[0].coin = 7.0;
    s.pay_outreach(&mut p, &c);
    s.advance_month(&p, &[], &c);
    assert_eq!(p[0].coin, 7.0);
    assert_eq!(s.senators[0].confidence, vec![1.5, 1.5]);
    p[0].coin = 1000.0;
    for _ in 0..10 {
        s.pay_outreach(&mut p, &c);
        s.advance_month(&p, &[], &c);
        let seat = &s.senators[0];
        assert!(seat.confidence.iter().sum::<f64>() <= c.seat_confidence + 1e-9);
        for player in 0..2 {
            assert!(seat.personal_confidence[player] <= seat.confidence[player] + 1e-9);
        }
    }
    s.cancel_senator_action(0, 0).unwrap();
    assert!(s.senators[0].arrangement(1).is_some());
    assert_eq!(s.action_upkeep(0, SenatorAction::Bribe, &c), 0.0);
}

#[test]
fn recurring_bribe_exposure_ends_only_the_payers_action_and_returns_misconduct() {
    let mut c = safe_config();
    let mut s = SenateState::new(1);
    let mut p = actors(2);
    s.act_on_senator(0, 0, SenatorAction::Bribe, &mut p, &c).unwrap();
    s.act_on_senator(1, 0, SenatorAction::Lobby, &mut p, &c).unwrap();
    c.action_risks[SenatorAction::Bribe as usize] = 1.0;
    let payments = s.pay_outreach(&mut p, &c);
    assert_eq!(payments.len(), 1);
    assert!(payments[0].caught);
    assert_eq!(payments[0].player, 0);
    assert_eq!(payments[0].senator, 0);
    assert!(s.senators[0].arrangement(0).is_none());
    assert!(s.senators[0].arrangement(1).is_some());
    assert_eq!(s.accusations[0].target, 0);
    s.advance_month(&p, &[], &c);
    assert_eq!(s.senators[0].confidence, vec![0.0, 1.0]);
}

#[test]
fn fading_personal_confidence_does_not_remove_structural_confidence() {
    let c = safe_config();
    let mut s = SenateState::new(1);
    let mut p = actors(1);
    s.ensure_players(1);
    s.add_to_seat(0, 0, 5.0);
    s.act_on_senator(0, 0, SenatorAction::Gift, &mut p, &c).unwrap();
    for _ in 0..12 {
        s.advance_month(&p, &[], &c);
    }
    assert_eq!(s.senators[0].personal_confidence[0], 0.0);
    assert_eq!(s.senators[0].confidence[0], 5.0);
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
fn consul_promotion_is_available_when_other_players_are_already_consuls() {
    let c = SenateConfig::default();
    let mut s = SenateState::new(7);
    let mut p = actors(3);
    p[0].rank = PoliticalRank::Censor;
    for actor in &mut p[1..] {
        actor.rank = PoliticalRank::Consul;
    }
    pledge(&mut s, 0, 40);
    assert!(matches!(
        s.promote(0, &mut p, &c).unwrap(),
        SenateEvent::RankAdvanced(0, PoliticalRank::Consul)
    ));
    assert!(p.iter().all(|actor| actor.rank == PoliticalRank::Consul));
    assert_eq!(p[0].influence, 3000.0);
    assert_eq!(s.support(0), 40);
    assert_eq!(c.requirements(p[0].rank, p.len()).unwrap().rank, PoliticalRank::Augustus);
}
#[test]
fn consuls_keep_their_rank_through_support_loss_scandals_and_monthly_reviews() {
    let c = SenateConfig::default();
    let mut s = SenateState::new(1);
    let mut p = actors(2);
    p[0].rank = PoliticalRank::Consul;
    p[1].rank = PoliticalRank::Consul;
    pledge(&mut s, 1, 100);
    let mut e = evidence(1, ScandalKind::Famine, Severity::Major);
    s.expose_scandal(0, 1, &p, &mut e, &c).unwrap();
    assert!(s.support(1) < 100, "exposure still reduces Senate support");
    for _ in 0..36 {
        s.advance_month(&p, &[], &c);
        assert!(p.iter().all(|actor| actor.rank == PoliticalRank::Consul));
    }
    assert_eq!(s.month, 36);
    assert_eq!(s.winner, None);
    assert_eq!(
        s.promotion_eligibility(0, &p, &c).unwrap_err(),
        PoliticalError::InsufficientSupport
    );
}
#[test]
fn consul_advances_directly_to_augustus_and_victory_stops_all_actions_and_ticks() {
    let c = SenateConfig::default();
    let mut s = SenateState::new(1);
    let mut p = actors(2);
    p[0].rank = PoliticalRank::Censor;
    pledge(&mut s, 0, 40);
    s.promote(0, &mut p, &c).unwrap();
    assert_eq!(p[0].rank, PoliticalRank::Consul);
    assert_eq!(s.promote(0, &mut p, &c).unwrap_err(), PoliticalError::AlreadyUsed);
    s.month += 1;
    assert_eq!(s.promote(0, &mut p, &c).unwrap_err(), PoliticalError::InsufficientSupport);
    pledge(&mut s, 0, 60);
    p[0].influence = 2999.0;
    assert_eq!(s.promote(0, &mut p, &c).unwrap_err(), PoliticalError::InsufficientFunds);
    p[0].influence = 3000.0;
    assert!(matches!(s.promote(0, &mut p, &c).unwrap(), SenateEvent::Victory(0)));
    assert_eq!(s.winner, Some(0));
    assert_eq!(p[0].rank, PoliticalRank::Augustus);
    assert_eq!(p[0].influence, 0.0);
    assert_eq!(s.support(0), 60);
    s.advance_month(&p, &[], &c);
    assert_eq!(s.month, 1);
    assert_eq!(
        s.act_on_senator(0, 99, SenatorAction::Assassinate, &mut p, &c).unwrap_err(),
        PoliticalError::CampaignFinished
    );
}
#[test]
fn arbitrary_months_keep_confidence_finite_bounded_and_support_exclusive() {
    let c = SenateConfig::default();
    let mut s = SenateState::new(11);
    let p = actors(8);
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
        s.advance_month(&p, &profiles, &c);
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
