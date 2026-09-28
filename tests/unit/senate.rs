use super::*;
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
    for s in state.senators.iter_mut().take(count) {
        s.allegiance = Some(player);
    }
}
fn evidence(target: usize, kind: ScandalKind) -> EspionageState {
    let mut e = EspionageState::new(4);
    e.scandals.push(Scandal {
        id: 1,
        source_id: 1,
        holder: 0,
        target: ScandalTarget::Player(target),
        province: None,
        kind,
        severity: Severity::Major,
        acquired: 0,
        expires: 24,
        reserved_for_motion: false,
    });
    e
}
fn excellent() -> PoliticalProfile {
    PoliticalProfile {
        nobles: 500.0,
        noble_happiness: 95.0,
        citizen_happiness: 95.0,
        plebeian_happiness: 95.0,
        coin_income: 500.0,
        trade_volume: 500.0,
        political_buildings: 10.0,
        markets: 10.0,
        wonders: 3.0,
        provincial_happiness: 95.0,
        provincial_trade: 200.0,
        food_policy: 1.0,
        military_strength: 10_000.0,
        military_rank: 3.0,
        recent_victories: 4.0,
        ..Default::default()
    }
}
fn terrible() -> PoliticalProfile {
    PoliticalProfile {
        noble_happiness: 0.0,
        citizen_happiness: 0.0,
        plebeian_happiness: 0.0,
        provincial_happiness: 0.0,
        food_security: 0.0,
        resource_security: 0.0,
        trade_reliability: 0.0,
        famine: 1.0,
        recent_victories: -4.0,
        ..Default::default()
    }
}
#[test]
fn new_match_has_one_hundred_neutral_stable_senators() {
    let s = SenateState::new(1);
    assert_eq!(s.senators.len(), 100);
    for (id, senator) in s.senators.iter().enumerate() {
        assert_eq!(senator.id, id);
        assert_eq!(senator.allegiance, None);
        assert_eq!(senator.bloc, Bloc::ALL[id / 20]);
    }
}
#[test]
fn costs_and_support_scale_without_lowering_victory_below_majority() {
    let c = SenateConfig::default();
    let ranks = [
        PoliticalRank::Quaestor,
        PoliticalRank::Aedile,
        PoliticalRank::Praetor,
        PoliticalRank::Censor,
        PoliticalRank::Consul,
    ];
    assert_eq!(ranks.map(|rank| c.requirements(rank, 2).unwrap().senators), [5, 12, 22, 40, 60]);
    assert_eq!(ranks.map(|rank| c.requirements(rank, 4).unwrap().senators), [4, 8, 16, 28, 51]);
    assert_eq!(
        ranks.map(|rank| c.requirements(rank, 2).unwrap().influence),
        [100.0, 180.0, 280.0, 400.0, 800.0]
    );
    assert_eq!(c.requirements(PoliticalRank::Consul, 8).unwrap().senators, 51);
}
#[test]
fn promotion_is_immediate_atomic_and_never_consumes_senators() {
    let c = SenateConfig::default();
    let mut s = SenateState::new(1);
    let mut p = actors(2);
    let before = p[0].influence;
    assert_eq!(s.promote(0, &mut p, &c).unwrap_err(), PoliticalError::InsufficientSupport);
    assert_eq!(p[0].influence, before);
    pledge(&mut s, 0, 5);
    s.promote(0, &mut p, &c).unwrap();
    assert_eq!(p[0].rank, PoliticalRank::Aedile);
    assert_eq!(p[0].influence, before - 100.0);
    assert_eq!(s.support(0), 5);
    pledge(&mut s, 0, 12);
    assert_eq!(s.promote(0, &mut p, &c).unwrap_err(), PoliticalError::AlreadyUsed);
    s.month += 1;
    p[0].influence = 179.0;
    assert_eq!(s.promote(0, &mut p, &c).unwrap_err(), PoliticalError::InsufficientFunds);
    assert_eq!(p[0].influence, 179.0);
    p[0].influence = 180.0;
    s.promote(0, &mut p, &c).unwrap();
    assert_eq!(p[0].rank, PoliticalRank::Praetor);
}
#[test]
fn all_loyalties_recalculate_monthly_and_can_switch_or_return_to_gray() {
    let c = SenateConfig::default();
    let mut s = SenateState::new(4);
    let mut p = actors(2);
    s.advance_month(&mut p, &[excellent(), terrible()], &c);
    assert!(s.support(0) > 80);
    let first = s.support(0);
    s.advance_month(&mut p, &[terrible(), excellent()], &c);
    assert!(s.support(1) > 80);
    assert!(s.support(0) < first);
    s.advance_month(&mut p, &[terrible(), terrible()], &c);
    assert_eq!(s.senators.iter().filter(|s| s.allegiance.is_none()).count(), 100);
}
#[test]
fn identical_players_do_not_gain_support_by_player_order() {
    let c = SenateConfig::default();
    let mut s = SenateState::new(4);
    let mut p = actors(2);
    s.advance_month(&mut p, &[excellent(), excellent()], &c);
    assert_eq!(s.support(0), 0);
    assert_eq!(s.support(1), 0);
}
#[test]
fn faction_preferences_differ_and_keep_seeded_individuality() {
    let c = SenateConfig::default();
    let mut s = SenateState::new(21);
    let mut p = actors(2);
    let welfare = PoliticalProfile {
        citizen_happiness: 100.0,
        plebeian_happiness: 100.0,
        provincial_happiness: 100.0,
        ..Default::default()
    };
    let soldiers = PoliticalProfile {
        military_rank: 3.0,
        military_strength: 10000.0,
        recent_victories: 4.0,
        ..Default::default()
    };
    s.advance_month(&mut p, &[welfare, soldiers], &c);
    assert_eq!(s.bloc_support(0, Bloc::Populares), 20);
    assert_eq!(s.bloc_support(1, Bloc::Military), 20);
    assert_ne!(s.senators[0].preferences, s.senators[1].preferences);
}
#[test]
fn same_seed_and_actions_produce_identical_monthly_allegiances() {
    let c = SenateConfig::default();
    let play = || {
        let mut s = SenateState::new(31);
        let mut p = actors(2);
        s.court(0, Bloc::Aristocrats, &mut p, &c).unwrap();
        for _ in 0..12 {
            s.advance_month(&mut p, &[excellent(), terrible()], &c);
        }
        s.senators.iter().map(|s| s.allegiance).collect::<Vec<_>>()
    };
    assert_eq!(play(), play());
}
#[test]
fn outreach_is_nonstacking_fades_and_respects_monthly_action_limits() {
    let c = SenateConfig::default();
    let mut s = SenateState::new(5);
    let mut p = actors(2);
    s.court(0, Bloc::Military, &mut p, &c).unwrap();
    assert_eq!(s.court(0, Bloc::Military, &mut p, &c), Err(PoliticalError::AlreadyUsed));
    let bonus = |s: &SenateState, p: &[PoliticalPlayer]| {
        s.reasons(0, Bloc::Military, &p[0], &PoliticalProfile::default(), &c)
            .iter()
            .find(|r| r.label == "Faction outreach (fades)")
            .unwrap()
            .points
    };
    assert_eq!(bonus(&s, &p), 8.0);
    s.advance_month(&mut p, &[], &c);
    assert!(bonus(&s, &p) < 8.0);
    s.court(0, Bloc::Military, &mut p, &c).unwrap();
    assert_eq!(bonus(&s, &p), 8.0);
    for _ in 0..6 {
        s.advance_month(&mut p, &[], &c);
    }
    assert_eq!(bonus(&s, &p), 0.0);
}
#[test]
fn bribe_is_temporary_cannot_be_overwritten_and_price_increases() {
    let c = SenateConfig::default();
    let mut s = SenateState::new(3);
    let mut p = actors(2);
    let id = s.bribe(0, Bloc::Military, &mut p, &c).unwrap();
    assert_eq!(s.support(0), 1);
    assert_eq!(p[0].coin, 9920.0);
    assert_eq!(s.bribe(0, Bloc::Military, &mut p, &c), Err(PoliticalError::AlreadyUsed));
    assert_eq!(s.bribe_quote(0, Bloc::Populares, &p, &c), Ok(100.0));
    s.bribe(1, Bloc::Military, &mut p, &c).unwrap();
    assert_eq!(s.senators[id].allegiance, Some(0));
    for _ in 0..5 {
        s.advance_month(&mut p, &[terrible(), excellent()], &c);
    }
    assert_eq!(s.senators[id].allegiance, Some(0));
    s.advance_month(&mut p, &[terrible(), excellent()], &c);
    assert!(s.senators[id].bribe.is_none());
    assert_eq!(s.senators[id].allegiance, Some(1));
}
#[test]
fn bribery_cap_cannot_be_bypassed_by_switching_factions() {
    let c = SenateConfig::default();
    let mut s = SenateState::new(3);
    let mut p = actors(2);
    for _ in 0..2 {
        for bloc in Bloc::ALL {
            s.bribe(0, bloc, &mut p, &c).unwrap();
        }
        s.advance_month(&mut p, &[], &c);
    }
    assert_eq!(s.active_bribes(0), 10);
    assert_eq!(s.bribe_quote(0, Bloc::Military, &p, &c), Err(PoliticalError::Ineligible));
}
#[test]
fn scandal_consumes_real_evidence_cancels_bribes_and_applies_relevant_penalties() {
    let c = SenateConfig::default();
    let mut s = SenateState::new(8);
    let mut p = actors(2);
    let id = s.bribe(1, Bloc::Aristocrats, &mut p, &c).unwrap();
    let mut e = evidence(1, ScandalKind::PoliticalBribery);
    s.expose_scandal(0, 1, &p, &mut e, &c).unwrap();
    assert!(e.scandals.is_empty());
    assert!(s.senators[id].bribe.is_none());
    assert!(s.accusations[0].penalties[0] > s.accusations[0].penalties[4]);
    assert_eq!(s.expose_scandal(0, 1, &p, &mut e, &c), Err(PoliticalError::ScandalRequired));
    for _ in 0..12 {
        s.advance_month(&mut p, &[], &c);
    }
    assert!(s.accusations.is_empty());
}
#[test]
fn invalid_scandals_cannot_mutate_the_chamber_or_consume_evidence() {
    let c = SenateConfig::default();
    let mut s = SenateState::new(8);
    let p = actors(2);
    let mut e = evidence(0, ScandalKind::Famine);
    assert_eq!(s.expose_scandal(0, 1, &p, &mut e, &c), Err(PoliticalError::Ineligible));
    e.scandals[0].target = ScandalTarget::Player(1);
    e.scandals[0].expires = 0;
    assert_eq!(s.expose_scandal(0, 1, &p, &mut e, &c), Err(PoliticalError::ScandalRequired));
    assert_eq!(e.scandals.len(), 1);
    assert!(s.accusations.is_empty());
}
#[test]
fn consul_slots_term_and_one_year_return_cooldown_are_authoritative() {
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
    pledge(&mut s, 1, 40);
    assert_eq!(s.promote(1, &mut p, &c).unwrap_err(), PoliticalError::ConsulCooldown);
    s.month = 13;
    s.promote(1, &mut p, &c).unwrap();
    assert_eq!(p[1].rank, PoliticalRank::Consul);
    assert_eq!(p[1].consul_until, Some(37));
}
#[test]
fn large_support_loss_has_grace_and_recovery_resets_it() {
    let c = SenateConfig::default();
    let mut s = SenateState::new(1);
    let mut p = actors(2);
    p[0].rank = PoliticalRank::Consul;
    p[0].consul_until = Some(24);
    for _ in 0..2 {
        assert!(s.advance_month(&mut p, &[terrible(), excellent()], &c).is_empty());
    }
    s.advance_month(&mut p, &[excellent(), terrible()], &c);
    assert_eq!(p[0].rank, PoliticalRank::Consul);
    for _ in 0..2 {
        s.advance_month(&mut p, &[terrible(), excellent()], &c);
    }
    assert_eq!(p[0].rank, PoliticalRank::Consul);
    let events = s.advance_month(&mut p, &[terrible(), excellent()], &c);
    assert!(events.iter().any(|e| matches!(e, SenateEvent::ConsulRemoved(0))));
    assert_eq!(p[0].rank, PoliticalRank::Proconsul);
    assert_eq!(p[0].consul_again_at, s.month + 12);
}
#[test]
fn scandal_forces_early_demotion_when_support_collapses() {
    let c = SenateConfig::default();
    let mut s = SenateState::new(1);
    let mut p = actors(2);
    p[1].rank = PoliticalRank::Consul;
    p[1].consul_until = Some(24);
    let mut e = evidence(1, ScandalKind::Famine);
    s.expose_scandal(0, 1, &p, &mut e, &c).unwrap();
    let events = s.advance_month(&mut p, &[excellent(), terrible()], &c);
    assert!(events.iter().any(|e| matches!(e, SenateEvent::ConsulRemoved(1))));
    assert_eq!(p[1].rank, PoliticalRank::Proconsul);
}
#[test]
fn proconsul_cannot_win_until_reappointed_and_victory_stops_ticks() {
    let c = SenateConfig::default();
    let mut s = SenateState::new(1);
    let mut p = actors(2);
    p[0].rank = PoliticalRank::Proconsul;
    pledge(&mut s, 0, 100);
    assert_eq!(s.promotion_eligibility(0, &p, &c).unwrap().rank, PoliticalRank::Consul);
    s.promote(0, &mut p, &c).unwrap();
    s.month += 1;
    s.promote(0, &mut p, &c).unwrap();
    assert_eq!(s.winner, Some(0));
    assert_eq!(p[0].rank, PoliticalRank::Augustus);
    assert!(s.advance_month(&mut p, &[], &c).is_empty());
    assert_eq!(s.month, 1);
}
