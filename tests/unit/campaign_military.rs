//! Cross-system regressions for military recruitment, upkeep, and occupation order.

use super::super::campaign_notifications::{NoticeAction, NoticeKind};
use super::*;

/// Construct a small deterministic three-province campaign without renderer assets.
fn campaign() -> Campaign {
    campaign_with_players(2)
}

/// The three-player variant exercises neutral territory and incompatible coalitions.
fn campaign_with_players(players: usize) -> Campaign {
    let mut campaign = Campaign::default();
    let provinces = (0..3)
        .map(|id| {
            let mut p = EconomicProvince::new(
                format!("Test {id}"),
                100.,
                Terrain::Plains,
                false,
                [1., 1., 1.],
                [2., 10., 30., 20.],
                players,
            );
            p.owner = if id == 0 {
                Some(0)
            } else if id == 2 {
                Some(1)
            } else {
                None
            };
            p
        })
        .collect();
    campaign.economy = EconomyWorld::new(players, provinces, vec![vec![1], vec![0, 2], vec![1]]);
    campaign.politics = vec![
        ProvincePolitics::owned(players, 0),
        ProvincePolitics::independent(players),
        ProvincePolitics::owned(players, 1),
    ];
    campaign.actors = vec![PoliticalPlayer::default(); players];
    campaign.profiles = vec![PoliticalProfile::default(); players];
    campaign.wars = vec![vec![false; players]; players];
    campaign.npc_wars = vec![vec![false; 3]; players];
    campaign.invitations = vec![vec![false; players]; players];
    campaign.recent_victories = vec![0.; players];
    campaign.military = MilitaryWorld::new(3);
    campaign.graph = (0..3)
        .map(|id| MilitaryProvince {
            terrain: MilitaryTerrain::Plains,
            area: 100.,
            road_level: 0,
            neighbors: campaign.economy.adjacency[id].clone(),
        })
        .collect();
    for player in &mut campaign.economy.players {
        player.resources = [1000., 100., 100.];
    }
    campaign.active = true;
    campaign
}

#[test]
fn completing_recruits_receive_this_months_food_shortage_and_draft_penalty() {
    let mut c = campaign();
    c.economy.provinces[0].potential = [0.; 3];
    c.economy.players[0].resources[0] = 0.;
    c.military.config.units[UnitType::LightInfantry as usize].recruitment_months = 1.;
    c.military
        .recruit(
            0,
            0,
            UnitType::LightInfantry,
            true,
            &[],
            &mut c.economy.provinces[0].population,
            &mut c.economy.players[0].resources[1],
        )
        .unwrap();
    c.advance_month();
    let unit = &c.military.provinces[0].forces[&ForceOwner::Player(0)][0];
    assert_eq!(unit.morale, 25., "new cohorts must share the same month's food shortage");
    assert_eq!(unit.current_manpower, 10., "food shortage must not instantly kill soldiers");
    assert!(c.economy.provinces[0].happiness_modifiers[2] < 0., "drafting must lower happiness");
}

#[test]
fn new_occupation_waits_one_month_then_uses_surviving_strength() {
    let mut c = campaign();
    let owner = ForceOwner::Player(0);
    c.npc_wars[0][1] = true;
    let id = c.military.seed_unit(0, owner, UnitType::HeavyInfantry).unwrap();
    let access = c.access_snapshot();
    c.military.order_movement(0, 1, owner, &[id], None, &c.graph, |_, p| access[0][p]).unwrap();
    c.advance_month();
    assert_eq!(c.military.provinces[1].occupation, Some(owner));
    assert_eq!(
        c.politics[1].independent_control(0),
        0.,
        "arrival cannot generate immediate military Control"
    );
    c.advance_month();
    assert!(c.politics[1].independent_control(0) > 0.);
}

#[test]
fn arrival_joins_existing_owner_battle_without_resetting_locked_plan() {
    let mut c = campaign();
    let attacker = ForceOwner::Player(0);
    let defender = ForceOwner::Player(1);
    c.wars[0][1] = true;
    c.wars[1][0] = true;
    c.military.seed_unit(1, attacker, UnitType::HeavyInfantry).unwrap();
    c.military.seed_unit(1, defender, UnitType::HeavyInfantry).unwrap();
    c.military
        .start_battle(1, &[attacker], &[defender], None, Some(0), MilitaryTerrain::Plains, 0, 1)
        .unwrap();
    c.military.seed_unit(1, attacker, UnitType::LightCavalry).unwrap();
    c.military.provinces[1].plans.insert(
        attacker,
        BattlePlan {
            tactic: CombatTactic::ShockAction,
            ..Default::default()
        },
    );
    c.begin_encounter(1, attacker, Some(0));
    assert_eq!(c.military.battles[0].attackers.units.len(), 2);
    assert_eq!(c.military.battles[0].attackers.plans[&attacker].tactic, CombatTactic::Balanced);
    assert!(!c.military.provinces[1].forces.contains_key(&attacker));
}

#[test]
fn npc_access_uses_the_configured_relation_threshold() {
    let mut c = campaign();
    c.politics[1].relations[0] = 69.;
    assert_eq!(c.access_snapshot()[0][1], MilitaryAccess::Blocked);
    c.politics[1].relations[0] = 70.;
    assert_eq!(c.access_snapshot()[0][1], MilitaryAccess::Peaceful);
}

#[test]
fn declaring_war_revokes_both_directions_of_military_invitation() {
    let mut c = campaign();
    c.invitations[1][0] = true;
    c.invitations[0][1] = true;
    assert_eq!(c.access_snapshot()[0][2], MilitaryAccess::Peaceful);
    c.declare_hostility(0, 2);
    assert!(!c.invitations[1][0] && !c.invitations[0][1]);
    assert!(c.wars[0][1] && c.wars[1][0]);
    assert_eq!(c.access_snapshot()[0][2], MilitaryAccess::Invasion);
}

#[test]
fn military_conquest_transfers_immediately_and_clears_previous_control_programs() {
    let mut c = campaign();
    let mut legacy = ProvincePolitics::independent(2);
    legacy.queue_control_gain(1, 45.).unwrap();
    legacy.support[1] = MonthlySupport {
        relation_coin: true,
        relation_influence: true,
        control_coin: true,
        control_influence: true,
    };
    legacy.state = PoliticalState::Owned {
        owner: 1,
    };
    c.politics[2] = legacy;
    for _ in 0..8 {
        c.military.seed_unit(2, ForceOwner::Player(0), UnitType::HeavyInfantry).unwrap();
    }
    c.military.seed_unit(2, ForceOwner::Player(1), UnitType::LightInfantry).unwrap();
    c.military.config.base_morale_damage = 100.;
    c.declare_hostility(0, 2);
    c.advance_month();
    assert!(matches!(
        c.politics[2].state,
        PoliticalState::Owned {
            owner: 0
        }
    ));
    assert_eq!(c.economy.provinces[2].owner, Some(0));
    assert!(c.politics[2].support.iter().all(|support| !support.relation_coin
        && !support.relation_influence
        && !support.control_coin
        && !support.control_influence));
    assert!(
        c.military.provinces[2].occupation.is_none(),
        "owned conquest is not independent Control"
    );
    // Re-entering independence must not resurrect stale queued political pressure.
    c.politics[2].state = PoliticalState::Independent {
        local: 100.,
        shares: vec![0.; 2],
    };
    c.politics[2].advance_month(
        &mut c.actors,
        &[Some(1), Some(1)],
        &[0., 0.],
        None,
        &c.diplomacy_config,
    );
    assert_eq!(c.politics[2].independent_control(1), 0.);
}

#[test]
fn hostile_npc_forces_defend_home_without_ever_issuing_offensive_orders() {
    let mut c = campaign();
    c.military.seed_local_defenders(1, "Achaia").unwrap();
    let original: Vec<_> =
        c.military.provinces[1].forces[&ForceOwner::Local(1)].iter().map(|u| u.id).collect();
    c.declare_hostility(0, 1);
    for _ in 0..24 {
        c.advance_month();
    }
    assert!(c.military.movements.is_empty());
    assert!(c.military.battles.is_empty());
    let defenders = &c.military.provinces[1].forces[&ForceOwner::Local(1)];
    assert_eq!(defenders.iter().map(|u| u.id).collect::<Vec<_>>(), original);
    for id in [0, 2] {
        assert!(!c.military.provinces[id].forces.contains_key(&ForceOwner::Local(1)));
    }
}

#[test]
fn visiting_enemies_cannot_capture_their_neutral_hosts_province() {
    let mut c = campaign_with_players(3);
    c.politics[1] = ProvincePolitics::owned(3, 2);
    c.economy.provinces[1].owner = Some(2);
    c.wars[0][1] = true;
    c.wars[1][0] = true;
    c.invitations[2][0] = true;
    c.invitations[2][1] = true;
    let attacker = ForceOwner::Player(0);
    let guest = ForceOwner::Player(1);
    let host = ForceOwner::Player(2);
    for _ in 0..8 {
        c.military.seed_unit(1, attacker, UnitType::HeavyInfantry).unwrap();
    }
    c.military.seed_unit(1, guest, UnitType::LightInfantry).unwrap();
    let host_unit = c.military.seed_unit(1, host, UnitType::HeavyInfantry).unwrap();
    c.military.config.base_morale_damage = 100.;
    c.begin_encounter(1, attacker, Some(0));
    c.advance_month();
    assert_eq!(c.military.history.last().unwrap().result, BattleResult::AttackerVictory);
    assert_eq!(c.economy.provinces[1].owner, Some(2), "a neutral host is not a conquest target");
    assert!(matches!(
        c.politics[1].state,
        PoliticalState::Owned {
            owner: 2
        }
    ));
    assert!(c.military.provinces[1].forces[&host].iter().any(|unit| unit.id == host_unit));
}

#[test]
fn mutually_hostile_defenders_wait_instead_of_forming_an_alliance() {
    let mut c = campaign_with_players(3);
    for a in 0..3 {
        for b in 0..3 {
            c.wars[a][b] = a != b;
        }
    }
    let attacker = ForceOwner::Player(0);
    let territorial = ForceOwner::Player(1);
    let third = ForceOwner::Player(2);
    for owner in [attacker, territorial, third] {
        c.military.seed_unit(2, owner, UnitType::HeavyInfantry).unwrap();
    }
    c.begin_encounter(2, attacker, Some(1));
    assert_eq!(
        c.military.battles[0].defenders.plans.keys().copied().collect::<Vec<_>>(),
        vec![territorial]
    );
    assert!(c.military.provinces[2].forces.contains_key(&third));
    c.begin_encounter(2, third, Some(1));
    assert!(!c.military.battles[0].attackers.plans.contains_key(&third));
    assert!(!c.military.battles[0].defenders.plans.contains_key(&third));
    assert!(c.military.provinces[2].forces.contains_key(&third));
}

#[test]
fn military_recruitment_notice_is_private_clickable_and_emitted_once() {
    let mut c = campaign();
    c.military.config.units[UnitType::LightInfantry as usize].recruitment_months = 1.;
    c.military
        .recruit(
            0,
            0,
            UnitType::LightInfantry,
            true,
            &[],
            &mut c.economy.provinces[0].population,
            &mut c.economy.players[0].resources[1],
        )
        .unwrap();
    c.advance_month();
    let notices = c.notifications.drain_for(0);
    let drafted: Vec<_> =
        notices.iter().filter(|notice| notice.kind == NoticeKind::RecruitmentCompleted).collect();
    assert_eq!(drafted.len(), 1);
    assert_eq!(drafted[0].action, NoticeAction::OpenProvince(0));
    assert!(!c
        .notifications
        .history_for(1)
        .any(|notice| notice.kind == NoticeKind::RecruitmentCompleted));
    c.advance_month();
    assert_eq!(
        c.notifications
            .history_for(0)
            .filter(|notice| notice.kind == NoticeKind::RecruitmentCompleted)
            .count(),
        1
    );
}

#[test]
fn military_victory_surfaces_npc_defeat_occupation_and_rank() {
    let mut c = campaign();
    c.npc_wars[0][1] = true;
    for _ in 0..8 {
        c.military.seed_unit(1, ForceOwner::Player(0), UnitType::HeavyInfantry).unwrap();
    }
    c.military.seed_unit(1, ForceOwner::Local(1), UnitType::LightInfantry).unwrap();
    c.military.config.base_morale_damage = 100.;
    c.military.config.rank_thresholds = [0., 0.1, 0.2, 0.3];
    c.begin_encounter(1, ForceOwner::Player(0), Some(0));
    c.advance_month();
    let kinds: Vec<_> = c.notifications.history_for(0).map(|notice| notice.kind).collect();
    for expected in [
        NoticeKind::InvasionBegins,
        NoticeKind::BattleResolved,
        NoticeKind::NpcDefeated,
        NoticeKind::OccupationEstablished,
        NoticeKind::MilitaryRankIncreased,
    ] {
        assert!(kinds.contains(&expected), "missing reachable military notice: {expected:?}");
    }
    assert!(!c
        .notifications
        .history_for(1)
        .any(|notice| notice.kind == NoticeKind::MilitaryRankIncreased));
}

#[test]
fn military_access_changes_and_departed_vassal_garrison_are_reported() {
    let mut c = campaign();
    c.politics[1].state = PoliticalState::Vassal {
        overlord: 0,
        control: 80.,
        tribute: Tribute::Normal,
    };
    c.reconcile_provinces();
    let owner = ForceOwner::Player(0);
    let ids: Vec<_> =
        (0..8).map(|_| c.military.seed_unit(1, owner, UnitType::HeavyInfantry).unwrap()).collect();
    c.initialize_notification_snapshot();
    c.invitations[1][0] = true;
    let access = c.access_snapshot();
    c.military
        .order_movement(1, 0, owner, &ids, None, &c.graph, |_, province| access[0][province])
        .unwrap();
    c.advance_month();
    assert!(c
        .notifications
        .history_for(0)
        .any(|notice| notice.kind == NoticeKind::GarrisonWeakened && notice.province == Some(1)));
    assert!(c.notifications.history_for(0).any(|notice| notice.kind
        == NoticeKind::MilitaryAccessGranted
        && notice.province == Some(2)));
    c.invitations[1][0] = false;
    c.advance_month();
    assert!(c.notifications.history_for(0).any(|notice| notice.kind
        == NoticeKind::MilitaryAccessRevoked
        && notice.province == Some(2)));
}

#[test]
fn military_foreign_arrival_and_trapped_defeat_reach_the_affected_players() {
    let mut c = campaign();
    let owner = ForceOwner::Player(0);
    c.invitations[1][0] = true;
    let unit = c.military.seed_unit(1, owner, UnitType::LightInfantry).unwrap();
    let access = c.access_snapshot();
    c.military
        .order_movement(1, 2, owner, &[unit], None, &c.graph, |_, province| access[0][province])
        .unwrap();
    c.advance_month();
    assert!(c
        .notifications
        .history_for(1)
        .any(|notice| notice.kind == NoticeKind::ForeignArrival && notice.province == Some(2)));
    c.military.seed_unit(2, ForceOwner::Player(1), UnitType::HeavyInfantry).unwrap();
    c.military.config.maximum_battle_months = 1;
    c.military.config.base_manpower_damage = 0.;
    c.military.config.base_morale_damage = 0.;
    c.declare_hostility(0, 2);
    c.advance_month();
    assert!(c.notifications.history_for(0).any(|notice| notice.kind == NoticeKind::UnitsDestroyed));
    assert!(c.notifications.history_for(0).any(|notice| notice.kind == NoticeKind::BattleResolved));
    assert!(!c
        .notifications
        .history_for(1)
        .any(|notice| notice.kind == NoticeKind::UnitsDestroyed));
}
