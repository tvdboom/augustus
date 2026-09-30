//! Cross-system regressions for military recruitment, upkeep, and occupation order.

use super::super::campaign_notifications::{NoticeAction, NoticeKind};
use super::*;

/// Construct a small deterministic three-province campaign without renderer assets.
fn campaign() -> Campaign {
    campaign_with_players(2)
}

#[test]
fn an_empty_deficit_treasury_flees_spies_and_breaks_unpaid_armies() {
    use crate::game::politics::espionage::SpyAssignment;

    let mut c = campaign();
    c.economy.provinces[0].population = [2.0, 0.0, 0.0, 0.0];
    c.economy.players[0].coin = 0.0;
    c.actors[0].coin = 0.0;
    c.actors[0].influence = 100.0;
    c.espionage_config.detection_range = [1.0; 2];
    let distance = c.distance(0, 1);
    c.espionage
        .deploy_assignment(
            0,
            1,
            &mut c.actors,
            &c.politics,
            &c.espionage_config,
            SpyAssignment::GainControl,
            distance,
        )
        .unwrap();
    let id = c.military.seed_unit(0, ForceOwner::Player(0), UnitType::HeavyInfantry).unwrap();
    c.military.provinces[0].forces.get_mut(&ForceOwner::Player(0)).unwrap()[0].morale = 5.0;

    c.advance_month();
    assert!(c.espionage.missions.is_empty(), "all spies flee immediately");
    assert!(c.politics[1].relation(0) < 50.0, "flight still applies detection effects");
    assert_eq!(c.economy.provinces[0].insolvency_unhappiness, 3.0);
    assert!(c.economy.provinces[0].happiness[0] < 50.0);
    let morale = c.military.provinces[0].forces[&ForceOwner::Player(0)][0].morale;
    assert_eq!(morale, 3.0, "insolvency costs exactly two morale despite recovery");

    c.advance_month();
    assert_eq!(c.economy.provinces[0].insolvency_unhappiness, 6.0);
    assert_eq!(c.military.provinces[0].forces[&ForceOwner::Player(0)][0].morale, 1.0);

    c.advance_month();
    assert_eq!(c.economy.provinces[0].insolvency_unhappiness, 9.0);
    assert!(c.military.all_units().all(|unit| unit.id != id), "zero morale disbands automatically");
    assert!(
        c.economy.provinces[0].population[2] >= 10.0,
        "survivors return to their class at home"
    );
    assert!(c.notifications.history_for(0).any(|notice| notice.kind == NoticeKind::ArmyDisbanded));
}

#[test]
fn zero_morale_disbands_stationary_marching_and_fighting_cohorts() {
    let mut c = campaign();
    let owner = ForceOwner::Player(0);
    let stationary = c.military.seed_unit(0, owner, UnitType::LightInfantry).unwrap();
    let marching = c.military.seed_unit(0, owner, UnitType::Archers).unwrap();
    c.military
        .order_movement(0, 1, owner, &[marching], None, &c.graph, |_, _| MilitaryAccess::Peaceful)
        .unwrap();
    let fighting = c.military.seed_unit(2, owner, UnitType::HeavyInfantry).unwrap();
    c.military.seed_unit(2, ForceOwner::Player(1), UnitType::LightInfantry).unwrap();
    c.military
        .start_battle(
            2,
            &[owner],
            &[ForceOwner::Player(1)],
            None,
            None,
            MilitaryTerrain::Plains,
            0,
            7,
        )
        .unwrap();
    c.military.reduce_morale(owner, 100.0, None);

    let disbanded = c.military.disband_zero_morale();
    let ids: Vec<_> = disbanded.iter().map(|(_, unit)| unit.id).collect();
    assert!(ids.contains(&stationary));
    assert!(ids.contains(&marching));
    assert!(ids.contains(&fighting));
    assert!(c.military.movements.is_empty());
    assert!(c.military.all_units().all(|unit| unit.owner != owner));
    assert!(c.military.all_units().any(|unit| unit.owner == ForceOwner::Player(1)));
}

#[test]
fn insolvency_requires_both_a_deficit_and_an_empty_treasury() {
    let mut c = campaign();
    c.economy.provinces[0].policies.civic_spending = crate::game::economy::CivicSpending::Frugal;
    c.economy.provinces[0].population[0] = 0.0;
    let report = crate::game::economy::MonthlyReport {
        province_reports: vec![Default::default(); c.economy.provinces.len()],
        ..Default::default()
    };
    c.economy.players[0].coin = 0.0;
    c.apply_insolvency(&report, &Default::default());
    assert_eq!(c.economy.provinces[0].insolvency_unhappiness, 0.0);

    let unit = c.military.seed_unit(0, ForceOwner::Player(0), UnitType::LightInfantry).unwrap();
    c.economy.players[0].coin = 1.0;
    c.apply_insolvency(&report, &Default::default());
    assert_eq!(c.economy.provinces[0].insolvency_unhappiness, 0.0);
    assert_eq!(c.military.provinces[0].forces[&ForceOwner::Player(0)][0].id, unit);
    assert_eq!(
        c.military.provinces[0].forces[&ForceOwner::Player(0)][0].morale,
        c.military.config.base_morale
    );
}

#[test]
fn slave_revolt_removes_residents_and_attacks_the_owners_garrison() {
    let mut c = campaign();
    c.economy.provinces[0].population[3] = 12.0;
    c.military.seed_unit(0, ForceOwner::Player(0), UnitType::HeavyInfantry).unwrap();
    c.start_slave_revolt(0, 0, 3);
    assert_eq!(c.economy.provinces[0].population[3], 0.0);
    assert!(c.npc_wars[0][0]);
    assert!(c.military.provinces[0].slave_rebellion);
    assert_eq!(c.military.battles.len(), 1);
    let battle = &c.military.battles[0];
    assert_eq!(battle.attackers.units.len(), 3);
    assert!(battle.attackers.units.iter().all(|unit| {
        unit.owner == ForceOwner::Local(0) && unit.unit_type == UnitType::LightInfantry
    }));
    assert!(battle.defenders.plans.contains_key(&ForceOwner::Player(0)));
    assert!(c.notifications.history_for(0).any(|notice| notice.kind == NoticeKind::SlaveRevolt));
}

#[test]
fn desperate_slaves_revolt_with_configured_certain_chance() {
    let mut c = campaign();
    c.economy.config.slave_revolt_chance = [1.0; 2];
    c.economy.provinces[0].happiness[3] = 0.0;
    c.economy.provinces[0].population[3] = 4.0 * crate::map::POPULATION_SCALE;
    c.resolve_slave_revolts();
    assert_eq!(c.economy.provinces[0].population[3], 0.0);
    let army = &c.military.provinces[0].forces[&ForceOwner::Local(0)];
    assert!((1..=4).contains(&army.len()));
    assert!(army.iter().all(|unit| unit.unit_type == UnitType::LightInfantry));
    assert!(c.notifications.history_for(0).any(|notice| notice.kind == NoticeKind::SlaveRevolt));
}

#[test]
fn province_hosts_pay_for_every_stationed_and_fighting_army_once() {
    let mut c = campaign_with_players(3);
    let host = ForceOwner::Player(0);
    let guest = ForceOwner::Player(1);
    let enemy = ForceOwner::Player(2);
    c.military.seed_unit(0, host, UnitType::HeavyInfantry).unwrap();
    c.military.seed_unit(0, guest, UnitType::LightInfantry).unwrap();
    c.military.seed_unit(0, enemy, UnitType::Archers).unwrap();
    let marching = c.military.seed_unit(0, guest, UnitType::HeavyInfantry).unwrap();
    c.military
        .order_movement(0, 1, guest, &[marching], None, &c.graph, |_, _| MilitaryAccess::Peaceful)
        .unwrap();
    c.military
        .start_battle(0, &[host], &[enemy], None, None, MilitaryTerrain::Plains, 0, 7)
        .unwrap();
    // The rule also charges a foreign host for our soldiers on their territory.
    c.military.seed_unit(2, host, UnitType::LightCavalry).unwrap();
    c.military.seed_unit(1, host, UnitType::LightInfantry).unwrap();
    c.military.seed_unit(1, ForceOwner::Local(1), UnitType::LightInfantry).unwrap();
    let food = |kind| c.military.config.unit(kind).food_per_month;
    let inputs = c.inputs();
    assert_eq!(
        inputs.army_food[0],
        food(UnitType::HeavyInfantry)
            + food(UnitType::LightInfantry) * 2.
            + food(UnitType::Archers)
    );
    assert_eq!(inputs.army_food[1], food(UnitType::HeavyInfantry) + food(UnitType::LightCavalry));
    assert_eq!(inputs.army_food[2], 0.);
    assert_eq!(inputs.npc_army_food[1], food(UnitType::LightInfantry));
    let total: f64 = c.military.all_units().map(|unit| unit.food_demand(&c.military.config)).sum();
    assert!(
        (inputs.army_food.iter().chain(&inputs.npc_army_food).sum::<f64>() - total).abs() < 1e-8
    );
}

#[test]
fn food_card_and_hud_projection_include_hosted_and_remote_armies() {
    let mut c = campaign();
    let host = ForceOwner::Player(0);
    let guest = ForceOwner::Player(1);
    c.military.seed_unit(0, guest, UnitType::HeavyInfantry).unwrap();
    c.military.seed_unit(1, host, UnitType::LightInfantry).unwrap();

    let sources = c.food_breakdown(0);
    let province = sources.iter().find(|(name, _, _, _)| *name == "Test 0").unwrap();
    let civilian = c.economy.provinces[0].food_request(&c.economy.config);
    let hosted = c.military.config.unit(UnitType::HeavyInfantry).food_per_month;
    let remote = c.military.config.unit(UnitType::LightInfantry).food_per_month;
    assert_eq!((province.2, province.3), (civilian, hosted));
    assert_eq!(
        sources.iter().find(|(name, _, _, _)| *name == "Armies elsewhere").unwrap(),
        &("Armies elsewhere", 0.0, 0.0, remote)
    );
    assert_eq!(
        sources.iter().map(|(_, _, civilian, military)| civilian + military).sum::<f64>(),
        civilian + c.inputs().army_food[0]
    );
    let projected = c.projected_food_delta(0, &c.inputs());
    assert_eq!(projected, province.1 - civilian - hosted - remote);
    assert_ne!(projected, 0.0);
}

#[test]
fn guests_eat_from_the_hosts_food_and_share_its_shortage_without_shifting_wages() {
    let mut c = campaign();
    let host = ForceOwner::Player(0);
    let guest = ForceOwner::Player(1);
    let stationed = c.military.seed_unit(0, guest, UnitType::HeavyInfantry).unwrap();
    let marching = c.military.seed_unit(0, guest, UnitType::LightInfantry).unwrap();
    let owned_unit = c.military.seed_unit(2, guest, UnitType::HeavyInfantry).unwrap();
    c.military
        .order_movement(0, 1, guest, &[marching], None, &c.graph, |_, _| MilitaryAccess::Peaceful)
        .unwrap();
    c.military.config.passive_training = 0.;
    for units in c.military.provinces.iter_mut().flat_map(|p| p.forces.values_mut()) {
        for unit in units {
            unit.morale = 50.;
            unit.training = 0.;
        }
    }
    for unit in &mut c.military.movements[0].units {
        unit.morale = 50.;
        unit.training = 0.;
    }
    for province in &mut c.economy.provinces {
        province.potential = [0.; 3];
    }
    let inputs = c.inputs();
    let host_demand = c.economy.provinces[0].food_request(&c.economy.config) + inputs.army_food[0];
    let guest_demand = c.economy.provinces[2].food_request(&c.economy.config) + inputs.army_food[1];
    c.economy.players[0].resources[0] = host_demand * 0.5;
    c.economy.players[1].resources[0] = 1000.;
    c.economy.players[0].coin = 100.;
    c.economy.players[1].coin = 100.;
    let report = c.economy.advance_month(&inputs);
    assert!((report.food_supply_ratio[0] - 0.5).abs() < 1e-8);
    assert_eq!(report.food_supply_ratio[1], 1.);
    assert!((c.economy.players[1].resources[0] - (1000. - guest_demand)).abs() < 1e-8);
    let host_coin = c.economy.players[0].coin;
    let guest_coin = c.economy.players[1].coin;
    let wages = c.army_wages(1);
    for (player, supply) in report.food_supply_ratio.iter().enumerate() {
        c.pay_army_wages(player, *supply);
    }
    let units: Vec<_> = c.military.all_units().collect();
    let morale = |id| units.iter().find(|unit| unit.id == id).unwrap().morale;
    assert_eq!(morale(stationed), 50. - c.military.config.shortage_morale_penalty * 0.5);
    assert_eq!(morale(owned_unit), 51.);
    assert_eq!(morale(marching), 51.);
    assert_eq!(c.economy.players[0].coin, host_coin);
    assert!((c.economy.players[1].coin - (guest_coin - wages)).abs() < 1e-8);
    assert_eq!(c.military.food_demand(host), 0.);
}

#[test]
fn province_access_is_local_and_notifies_the_guest_immediately_on_each_change() {
    let mut c = campaign();
    c.politics[1] = ProvincePolitics::owned(2, 0);
    c.economy.provinces[1].owner = Some(0);
    for granted in [true, false, true] {
        c.set_province_access(0, 0, 1, granted).unwrap();
        assert_eq!(
            c.access_snapshot()[1][0],
            if granted {
                MilitaryAccess::Peaceful
            } else {
                MilitaryAccess::Blocked
            }
        );
        assert_eq!(c.access_snapshot()[1][1], MilitaryAccess::Blocked);
    }
    let notices = c.notifications.drain_for(1);
    assert_eq!(notices.len(), 3);
    assert_eq!(notices[0].title, "Military access granted to Test 0");
    assert_eq!(notices[1].title, "Military access revoked to Test 0");
    assert_eq!(notices[2].kind, NoticeKind::MilitaryAccessGranted);
    assert!(c.notifications.history_for(0).next().is_none());
    c.set_province_access(0, 0, 1, true).unwrap();
    assert!(c.notifications.drain_for(1).is_empty());
}

#[test]
fn revoking_province_access_marches_all_guests_to_an_owned_province() {
    let mut c = campaign_with_players(3);
    c.set_province_access(0, 0, 1, true).unwrap();
    let guest = ForceOwner::Player(1);
    let other_guest = ForceOwner::Player(2);
    let host = ForceOwner::Player(0);
    for kind in [UnitType::HeavyInfantry, UnitType::LightInfantry] {
        c.military.seed_unit(0, guest, kind).unwrap();
    }
    c.military.seed_unit(0, host, UnitType::HeavyInfantry).unwrap();
    c.military.seed_unit(0, other_guest, UnitType::HeavyInfantry).unwrap();
    let ids: Vec<_> = c.military.provinces[0].forces[&guest].iter().map(|unit| unit.id).collect();
    c.notifications.drain_for(1);
    c.set_province_access(0, 0, 1, false).unwrap();
    assert_eq!(c.access_snapshot()[1][0], MilitaryAccess::Blocked);
    assert!(c.military.provinces[0].forces[&guest].is_empty());
    assert!(!c.military.provinces[2].forces.contains_key(&guest));
    assert_eq!(c.military.provinces[0].forces[&host].len(), 1);
    assert_eq!(c.military.provinces[0].forces[&other_guest].len(), 1);
    let order = &c.military.movements[0];
    assert_eq!(order.route, vec![1, 2]);
    assert!(order.withdrawing);
    assert_eq!(order.progress, 0.0);
    let notice = c.notifications.drain_for(1).pop().unwrap();
    assert_eq!(notice.kind, NoticeKind::MilitaryAccessRevoked);
    assert_eq!(notice.title, "Military access revoked to Test 0");
    assert!(notice.body.contains("Units marching to Test 2."));
    // Even closed transit provinces cannot strand a fixed peaceful withdrawal order.
    for _ in 0..100 {
        let events = c.military.advance_movement(&c.graph, |_, _| MilitaryAccess::Blocked);
        assert!(events.iter().all(|event| !matches!(
            event,
            MilitaryEvent::Arrived {
                invasion: true,
                ..
            }
        )));
        if c.military.movements.is_empty() {
            break;
        }
    }
    assert!(c.military.movements.is_empty());
    let arrived: Vec<_> =
        c.military.provinces[2].forces[&guest].iter().map(|unit| unit.id).collect();
    assert_eq!(arrived, ids);
    assert_eq!(c.military.provinces[2].occupation, None);
}

#[test]
fn revoked_access_chooses_the_closest_currently_owned_province() {
    let mut c = campaign();
    c.politics[1] = ProvincePolitics::owned(2, 1);
    c.economy.provinces[1].owner = Some(1);
    c.set_province_access(0, 0, 1, true).unwrap();
    c.military.seed_unit(0, ForceOwner::Player(1), UnitType::HeavyInfantry).unwrap();
    c.set_province_access(0, 0, 1, false).unwrap();
    assert_eq!(c.military.movements[0].route, vec![1]);
    assert!(c.notifications.drain_for(1).last().unwrap().body.contains("Test 1"));
}

#[test]
fn losing_the_last_owned_province_defeats_the_player_immediately() {
    let mut c = campaign();
    c.wars[0][1] = true;
    c.wars[1][0] = true;
    c.military.seed_unit(0, ForceOwner::Player(1), UnitType::HeavyInfantry).unwrap();
    c.politics[2].capture_owned(0).unwrap();
    c.reconcile_provinces();
    assert_eq!(c.economy.provinces[2].owner, Some(0));
    assert!(c.defeated[1]);
    assert!(!c.defeated[0]);
    assert!(c.notifications.history_for(1).any(|notice| notice.kind == NoticeKind::PlayerDefeated));
    assert!(c.military.provinces[0].forces.contains_key(&ForceOwner::Player(1)));
    c.reconcile_provinces();
    assert_eq!(
        c.notifications
            .history_for(1)
            .filter(|notice| notice.kind == NoticeKind::PlayerDefeated)
            .count(),
        1
    );
}

#[test]
fn losing_one_of_multiple_owned_provinces_does_not_defeat_the_player() {
    let mut c = campaign();
    c.politics[1] = ProvincePolitics::owned(2, 1);
    c.economy.provinces[1].owner = Some(1);
    c.wars[0][1] = true;
    c.wars[1][0] = true;
    c.politics[2].capture_owned(0).unwrap();
    c.reconcile_provinces();
    assert!(!c.defeated[1]);
    assert_eq!(c.economy.provinces[1].owner, Some(1));
}

#[test]
fn access_revocation_without_units_has_no_marching_claim_and_failure_preserves_access() {
    let mut c = campaign();
    c.set_province_access(0, 0, 1, true).unwrap();
    c.set_province_access(0, 0, 1, false).unwrap();
    let notice = c.notifications.drain_for(1).pop().unwrap();
    assert!(!notice.body.contains("marching"));
    assert!(c.military.movements.is_empty());
    c.set_province_access(0, 0, 1, true).unwrap();
    c.military.seed_unit(0, ForceOwner::Player(1), UnitType::HeavyInfantry).unwrap();
    c.graph[1].neighbors.clear();
    c.notifications.drain_for(1);
    assert_eq!(c.set_province_access(0, 0, 1, false), Err(MilitaryError::NoLegalRoute));
    assert!(c.province_access_granted(0, 0, 1));
    assert_eq!(c.military.provinces[0].forces[&ForceOwner::Player(1)].len(), 1);
    assert!(c.notifications.drain_for(1).is_empty());
}

#[test]
fn war_clears_province_access_and_prevents_new_grants() {
    let mut c = campaign();
    c.set_province_access(0, 0, 1, true).unwrap();
    c.set_province_access(2, 1, 0, true).unwrap();
    c.declare_hostility(0, 2);
    assert!(!c.province_access_granted(0, 0, 1));
    assert!(!c.province_access_granted(2, 1, 0));
    assert!(c.set_province_access(0, 0, 1, true).is_err());
    assert_eq!(c.access_snapshot()[1][0], MilitaryAccess::Invasion);
}

#[test]
fn defeating_romes_defenders_awards_immediate_victory_without_political_control() {
    let mut c = campaign();
    c.economy.provinces[1].name = "Latium".into();
    c.politics[1] = ProvincePolitics::rome(2);
    c.npc_wars[0][1] = true;
    for _ in 0..8 {
        c.military.seed_unit(1, ForceOwner::Player(0), UnitType::HeavyInfantry).unwrap();
    }
    c.military.seed_unit(1, ForceOwner::Local(1), UnitType::LightInfantry).unwrap();
    c.military.config.base_morale_damage = 100.0;
    c.begin_encounter(1, ForceOwner::Player(0), Some(0));
    assert_eq!(c.senate.winner, None);
    c.advance_month();
    assert_eq!(c.senate.winner, Some(0));
    assert_eq!(c.economy.provinces[1].owner, Some(0));
    assert_eq!(c.actors[0].rank, crate::game::politics::PoliticalRank::Augustus);
    for recipient in 0..2 {
        assert!(c
            .notifications
            .history_for(recipient)
            .any(|n| n.kind == NoticeKind::AugustusVictory));
    }
    let month = c.economy.month;
    c.advance_month();
    assert_eq!(c.economy.month, month);
}

#[test]
fn romes_local_defenders_fight_at_zero_morale_past_the_normal_deadline() {
    let mut c = campaign();
    c.economy.provinces[1].name = "Latium".into();
    c.politics[1] = ProvincePolitics::rome(2);
    c.npc_wars[0][1] = true;
    for _ in 0..8 {
        c.military.seed_unit(1, ForceOwner::Player(0), UnitType::HeavyInfantry).unwrap();
    }
    c.military.seed_unit(1, ForceOwner::Local(1), UnitType::LightInfantry).unwrap();
    c.military.config.units[UnitType::LightInfantry as usize].offense = 0.;
    c.military.config.base_manpower_damage = 0.;
    c.military.config.base_morale_damage = 100.;
    c.begin_encounter(1, ForceOwner::Player(0), Some(0));

    for _ in 0..=c.military.config.maximum_battle_months {
        c.military.battles[0].advance_month(&c.military.config);
    }
    let battle = &c.military.battles[0];
    assert_eq!(battle.result, None);
    assert_eq!(battle.defenders.units[0].morale, 1.);
    assert!(battle.defenders.routed.is_empty());
    let mut routed_attacker = battle.clone();
    for unit in &mut routed_attacker.attackers.units {
        unit.morale = 0.;
    }
    routed_attacker.advance_round(&c.military.config);
    assert_eq!(routed_attacker.result, Some(BattleResult::DefenderVictory));

    c.military.config.base_manpower_damage = 10.;
    c.military.battles[0].advance_round(&c.military.config);
    assert_eq!(c.military.battles[0].result, Some(BattleResult::AttackerVictory));
}

#[test]
fn defeating_a_visiting_rival_cannot_win_while_romes_local_defenders_remain() {
    let mut c = campaign();
    c.economy.provinces[1].name = "Latium".into();
    c.politics[1] = ProvincePolitics::rome(2);
    c.wars[0][1] = true;
    c.wars[1][0] = true;
    let attacker = ForceOwner::Player(0);
    let rival = ForceOwner::Player(1);
    for _ in 0..8 {
        c.military.seed_unit(1, attacker, UnitType::HeavyInfantry).unwrap();
    }
    c.military.seed_unit(1, rival, UnitType::LightInfantry).unwrap();
    c.military.seed_unit(1, ForceOwner::Local(1), UnitType::HeavyInfantry).unwrap();
    c.military.config.base_morale_damage = 100.0;
    c.military
        .start_battle(1, &[attacker], &[rival], None, Some(0), MilitaryTerrain::Plains, 0, 1)
        .unwrap();
    c.advance_month();
    assert!(c.military.battles.is_empty());
    assert_eq!(c.military.history[0].result, BattleResult::AttackerVictory);
    assert_eq!(c.senate.winner, None);
    assert_eq!(c.politics[1].state, PoliticalState::Rome);
    assert_eq!(c.economy.provinces[1].owner, None);
}

#[test]
fn recruitment_effort_funds_active_projects_proportionally_and_resets_when_cancelled() {
    let mut c = campaign();
    c.economy.provinces[2].owner = Some(0);
    for id in [0, 2] {
        c.economy.provinces[id].policies.recruitment = RecruitmentEffort::High;
        c.military
            .recruit(
                id,
                0,
                UnitType::LightInfantry,
                true,
                &[],
                &mut c.economy.provinces[id].population,
                &mut c.economy.players[0].resources[1],
            )
            .unwrap();
    }
    c.economy.provinces[1].policies.recruitment = RecruitmentEffort::High;
    c.economy.players[0].coin = 0.1;
    assert_eq!(c.recruitment_effort_cost(0), 20.0);
    let speeds = c.pay_recruitment_effort();
    assert!(c.economy.players[0].coin.abs() < 1e-8);
    for id in [0, 2] {
        assert!((speeds[id] - 1.25 / 200.0).abs() < 1e-8);
        assert!((c.economy.provinces[id].recruitment_happiness + 1.0 / 200.0).abs() < 1e-8);
    }
    assert_eq!(c.economy.provinces[1].recruitment_happiness, 0.0);
    let unfunded = c.pay_recruitment_effort();
    for id in [0, 2] {
        assert_eq!(unfunded[id], 0.0);
        assert_eq!(c.economy.provinces[id].recruitment_happiness, 0.0);
    }
    c.military.cancel_recruitment(0, ForceOwner::Player(0)).unwrap();
    c.economy.provinces[2].change_owner(Some(1), None);
    c.economy.players[0].coin = 10.0;
    c.pay_recruitment_effort();
    assert_eq!(c.recruitment_effort_cost(0), 0.0);
    assert_eq!(c.economy.players[0].coin, 10.0);
    assert!(c.economy.provinces.iter().all(|p| p.recruitment_happiness == 0.0));
}

#[test]
fn high_recruitment_charges_ten_per_active_project_regardless_of_cohort_size() {
    let mut c = campaign();
    c.economy.provinces[2].owner = Some(0);
    for (province, kind) in [(0, UnitType::LightInfantry), (2, UnitType::Archers)] {
        c.economy.provinces[province].policies.recruitment = RecruitmentEffort::High;
        c.military
            .recruit(
                province,
                0,
                kind,
                true,
                &[],
                &mut c.economy.provinces[province].population,
                &mut c.economy.players[0].resources[1],
            )
            .unwrap();
    }
    c.military.provinces[2].recruitment.as_mut().unwrap().population_cost *= 2.0;
    assert_eq!(c.recruitment_effort_cost(0), 20.0);
    c.military.cancel_recruitment(2, ForceOwner::Player(0)).unwrap();
    assert_eq!(c.recruitment_effort_cost(0), 10.0);
}

#[test]
fn recruitment_effort_changes_monthly_progress_and_applies_only_for_active_months() {
    for (effort, speed, cost, happiness) in [
        (RecruitmentEffort::Low, 0.75, 0.0, 1.0),
        (RecruitmentEffort::Normal, 1.0, 0.0, 0.0),
        (RecruitmentEffort::High, 1.25, 10.0, -1.0),
    ] {
        let mut c = campaign();
        c.economy.provinces[0].policies.recruitment = effort;
        c.military.config.units[UnitType::LightInfantry as usize].recruitment_months = 1.25;
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
        assert_eq!(c.economy.config.recruitment_speed[effort as usize], speed);
        assert_eq!(c.economy.config.recruitment_coin_per_project[effort as usize], cost);
        assert_eq!(c.recruitment_effort_cost(0), cost);
        let coin_before = c.economy.players[0].coin;
        c.advance_month();
        assert_eq!(c.economy.provinces[0].recruitment_happiness, happiness);
        let during = c.economy.provinces[0].calculate_happiness(&c.economy.config);
        let mut neutral = c.economy.provinces[0].clone();
        neutral.recruitment_happiness = 0.0;
        let baseline = neutral.calculate_happiness(&c.economy.config);
        for class in 0..4 {
            let effect = if class == 1 || class == 2 {
                happiness
            } else {
                0.0
            };
            assert_eq!(during[class] - baseline[class], effect);
        }
        assert_eq!(
            c.economy.last_report.player_delta[0][3],
            c.economy.players[0].coin - coin_before
        );
        if effort == RecruitmentEffort::High {
            assert!(c.military.provinces[0].recruitment.is_none());
            assert_eq!(c.military.all_units().count(), 1);
            c.advance_month();
            assert_eq!(c.economy.provinces[0].recruitment_happiness, 0.0);
            assert_eq!(c.recruitment_effort_cost(0), 0.0);
        } else {
            assert_eq!(c.military.provinces[0].recruitment.as_ref().unwrap().progress, speed);
        }
    }
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
    campaign.defeated = vec![false; players];
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
fn completed_roads_reach_the_military_graph_and_reduce_army_travel_time() {
    let mut c = campaign();
    c.economy.players[0].resources = [10_000.0; 3];
    let travel = |c: &Campaign| {
        edge_travel_months(
            &c.graph[0],
            &c.graph[1],
            100.0,
            c.military.config.reference_speed,
            &c.military.config,
        )
    };
    let before = travel(&c);
    c.economy.start_building(0, 0, BuildingType::Road).unwrap();
    c.reconcile_provinces();
    assert_eq!(c.graph[0].road_level, 0, "Unfinished roads must not grant movement bonuses");
    for _ in 0..3 {
        c.economy.advance_month(&MonthlyInputs::default());
    }
    c.reconcile_provinces();
    assert_eq!(c.graph[0].road_level, 1);
    assert!(travel(&c) < before, "Completed roads must speed army movement across their province");
}

#[test]
fn unopposed_occupation_is_retained_in_the_invaders_province_history() {
    let mut c = campaign();
    c.wars[0][1] = true;
    c.military.seed_unit(2, ForceOwner::Player(0), UnitType::HeavyInfantry).unwrap();
    c.begin_encounter(2, ForceOwner::Player(0), None);
    assert!(matches!(
        c.politics[2].state,
        PoliticalState::Owned {
            owner: 1
        }
    ));
    assert_eq!(c.military.provinces[2].occupation, Some(ForceOwner::Player(0)));
    let notices: Vec<_> = c.notifications.history_for(0).collect();
    assert_eq!(notices.len(), 1);
    assert_eq!(notices[0].province, Some(2));
    assert_eq!(notices[0].title, "Occupation established");
    assert_eq!(notices[0].kind, NoticeKind::OccupationEstablished);
    assert_eq!(notices[0].action, NoticeAction::OpenProvince(2));
    assert!(c.notifications.history_for(1).next().is_none());
}

#[test]
fn completing_recruits_receive_this_months_food_shortage_and_draft_penalty() {
    let mut c = campaign();
    c.economy.config.slave_revolt_chance = [0.0; 2];
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
    let unit = c
        .military
        .all_units()
        .find(|unit| unit.owner == ForceOwner::Player(0))
        .expect("the recruit remains in its province or its uprising battle");
    assert_eq!(unit.morale, 75., "new cohorts must share the same month's food shortage");
    assert_eq!(unit.current_manpower, 10., "food shortage must not instantly kill soldiers");
    assert!(c.economy.provinces[0].happiness_modifiers[2] < 0., "drafting must lower happiness");
}

#[test]
fn recruits_in_a_peaceful_owned_province_start_full_even_beside_damaged_veterans() {
    let mut c = campaign();
    c.economy.config.slave_revolt_chance = [0.0; 2];
    c.economy.provinces[0].population[1] = 40.0;
    c.military.config.units[UnitType::LightCavalry as usize].recruitment_months = 1.0;
    let owner = ForceOwner::Player(0);
    let veteran = c.military.seed_unit(0, owner, UnitType::LightCavalry).unwrap();
    c.military.provinces[0].forces.get_mut(&owner).unwrap()[0].current_manpower = 3.97;

    for _ in 0..2 {
        c.military
            .recruit(
                0,
                0,
                UnitType::LightCavalry,
                true,
                &[],
                &mut c.economy.provinces[0].population,
                &mut c.economy.players[0].resources[1],
            )
            .unwrap();
        c.advance_month();
        assert!(c.military.battles.is_empty());
    }

    let units = &c.military.provinces[0].forces[&owner];
    assert_eq!(units.len(), 3);
    assert!(units.iter().filter(|unit| unit.id != veteran).all(|unit| unit.people() == 1_000));
    assert!(units.iter().find(|unit| unit.id == veteran).unwrap().people() < 1_000);
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
    assert_eq!(c.military.battles[0].attackers.plans[&attacker].tactic, CombatTactic::ShockAction);
    assert!(!c.military.provinces[1].forces.contains_key(&attacker));
}

#[test]
fn npc_access_separates_friendly_passage_from_very_friendly_stationing() {
    let mut c = campaign();
    for (relation, expected) in [
        (59., MilitaryAccess::Blocked),
        (60., MilitaryAccess::Transit),
        (79., MilitaryAccess::Transit),
        (80., MilitaryAccess::Peaceful),
        (100., MilitaryAccess::Peaceful),
    ] {
        c.politics[1].relations[0] = relation;
        assert_eq!(c.access_snapshot()[0][1], expected);
    }
    c.military.config.npc_access_relation = 65.;
    c.military.config.npc_stationing_relation = 85.;
    c.politics[1].relations[0] = 64.;
    assert_eq!(c.access_snapshot()[0][1], MilitaryAccess::Blocked);
    c.politics[1].relations[0] = 65.;
    assert_eq!(c.access_snapshot()[0][1], MilitaryAccess::Transit);
    c.politics[1].relations[0] = 85.;
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
fn attacks_above_ten_relation_create_senate_scandals_for_npcs_and_players() {
    for province in [1, 2] {
        let mut justified = campaign();
        justified
            .military
            .seed_unit(province, ForceOwner::Player(0), UnitType::HeavyInfantry)
            .unwrap();
        justified.politics[province].relations[0] = 10.0;
        justified.declare_hostility(0, province);
        assert!(justified.senate.accusations.is_empty());

        let mut unjustified = campaign();
        unjustified
            .military
            .seed_unit(province, ForceOwner::Player(0), UnitType::HeavyInfantry)
            .unwrap();
        unjustified.politics[province].relations[0] = 11.0;
        unjustified.declare_hostility(0, province);
        assert!(unjustified
            .senate
            .accusations
            .iter()
            .any(|event| event.target == 0 && event.penalties[1] > 0.0));
    }
}

#[test]
fn owner_gets_private_overview_warning_when_control_falls_below_ninety() {
    let mut c = campaign();
    c.initialize_notification_snapshot();
    c.politics[2].queue_control_gain(0, 9.0).unwrap();
    c.politics[2].resolve_month(&[0.0; 2], None, 0.0, &c.diplomacy_config);
    let first = c.notification_snapshot();
    c.record_monthly_notifications(first);
    assert!(!c
        .notifications
        .history_for(1)
        .any(|notice| notice.kind == NoticeKind::OwnedControlThreatened));

    c.politics[2].queue_control_gain(0, 2.0).unwrap();
    c.politics[2].resolve_month(&[0.0; 2], None, 0.0, &c.diplomacy_config);
    let before = c.notification_snapshot();
    c.record_monthly_notifications(before);
    let warning = c
        .notifications
        .history_for(1)
        .find(|notice| notice.kind == NoticeKind::OwnedControlThreatened)
        .unwrap();
    assert_eq!(warning.action, NoticeAction::OpenProvince(2));
    assert!(!warning.body.contains("Player 1"));
    assert!(!c
        .notifications
        .history_for(0)
        .any(|notice| notice.kind == NoticeKind::OwnedControlThreatened));
}

#[test]
fn domestic_relation_tracks_only_that_provinces_weighted_happiness() {
    let mut c = campaign();
    c.economy.provinces[0].population = [10.0, 30.0, 0.0, 0.0];
    c.economy.provinces[0].happiness = [50.0, 90.0, 0.0, 0.0];
    c.economy.provinces[2].population = [10.0, 0.0, 0.0, 0.0];
    c.economy.provinces[2].happiness = [0.0; 4];
    c.sync_owned_relations();
    assert_eq!(c.politics[0].relation(0), 80.0);
    assert_eq!(c.politics[0].relation(1), 50.0);
}

#[test]
fn military_victory_occupies_owned_province_until_control_is_earned() {
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
            owner: 1
        }
    ));
    assert_eq!(c.economy.provinces[2].owner, Some(1));
    assert_eq!(c.military.provinces[2].occupation, Some(ForceOwner::Player(0)));
    let control = c.politics[2].control(0);
    c.advance_month();
    assert!(c.politics[2].control(0) > control);
    assert!(c.politics[2].control(0) <= 5.0);
}

#[test]
fn invited_troops_gain_slow_bounded_control_without_occupation() {
    let mut c = campaign();
    c.set_province_access(2, 1, 0, true).unwrap();
    for _ in 0..8 {
        c.military.seed_unit(2, ForceOwner::Player(0), UnitType::HeavyInfantry).unwrap();
    }
    c.advance_month();
    assert_eq!(c.military.provinces[2].occupation, None);
    assert!(c.politics[2].control(0) > 0.0);
    assert!(c.politics[2].control(0) < 1.0);
    assert_eq!(c.economy.provinces[2].owner, Some(1));
}

#[test]
fn noble_bribes_and_insults_obey_yearly_limits_and_change_their_own_values() {
    let mut c = campaign();
    c.economy.players[0].coin = 2_000.0;
    let cost = c.noble_bribe_cost(0, 2).unwrap();
    let (gain, _) = c.bribe_nobles(0, 2).unwrap();
    assert!(gain <= 10);
    assert!((c.economy.players[0].coin - (2_000.0 - cost)).abs() < 1e-8);
    assert!((c.politics[2].control(0) - gain as f64).abs() < 1e-8);
    assert_eq!(c.bribe_nobles(0, 2), Err(PoliticalError::AlreadyUsed));
    assert_eq!(c.bribe_nobles(0, 1), Err(PoliticalError::AlreadyUsed));
    let senate_before = c.senate.support(0);
    let accusations_before = c.senate.accusations.len();
    assert_eq!(c.insult_player(0, 2), Ok(1));
    assert_eq!(c.politics[2].relation(0), 40.0);
    assert_eq!(c.insult_player(0, 2), Err(PoliticalError::AlreadyUsed));
    assert_eq!(c.senate.support(0), senate_before);
    assert_eq!(c.senate.accusations.len(), accusations_before);
    c.economy.month = 12;
    assert!(c.noble_bribe_cost(0, 2).is_ok());
    assert_eq!(c.insult_player(0, 2), Ok(1));
    assert_eq!(c.politics[2].relation(0), 30.0);
}

#[test]
fn an_insult_cannot_be_sent_to_a_different_player_in_the_same_year() {
    let mut c = campaign_with_players(3);
    c.politics[1] = ProvincePolitics::owned(3, 2);
    c.economy.provinces[1].owner = Some(2);
    assert_eq!(c.insult_player(0, 2), Ok(1));
    assert_eq!(c.insult_player(0, 1), Err(PoliticalError::AlreadyUsed));
    assert_eq!(c.politics[1].relation(0), 50.0);
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
fn military_victory_surfaces_npc_defeat_and_occupation_without_automatic_promotion() {
    let mut c = campaign();
    c.npc_wars[0][1] = true;
    for _ in 0..8 {
        c.military.seed_unit(1, ForceOwner::Player(0), UnitType::HeavyInfantry).unwrap();
    }
    c.military.seed_unit(1, ForceOwner::Local(1), UnitType::LightInfantry).unwrap();
    c.military.config.base_morale_damage = 100.;
    c.begin_encounter(1, ForceOwner::Player(0), Some(0));
    c.advance_month();
    let kinds: Vec<_> = c.notifications.history_for(0).map(|notice| notice.kind).collect();
    for expected in [
        NoticeKind::InvasionBegins,
        NoticeKind::BattleResolved,
        NoticeKind::NpcDefeated,
        NoticeKind::OccupationEstablished,
    ] {
        assert!(kinds.contains(&expected), "missing reachable military notice: {expected:?}");
    }
    assert_eq!(c.military.rank(ForceOwner::Player(0)), MilitaryRank::Centurion);
    assert_eq!(c.military.victories[&ForceOwner::Player(0)], 1);
    assert!(!kinds.contains(&NoticeKind::MilitaryRankIncreased));
    assert!(!c
        .notifications
        .history_for(1)
        .any(|notice| notice.kind == NoticeKind::MilitaryRankIncreased));
}

#[test]
fn military_promotions_require_order_milestones_and_spend_influence_once() {
    let mut c = campaign();
    let owner = ForceOwner::Player(0);
    c.military.peak_manpower.insert(owner, 600.);
    c.military.victories.insert(owner, 6);
    c.economy.players[0].influence = 99.;
    assert!(c.promote_military(0, MilitaryRank::Legate).is_err());
    assert!(c.promote_military(0, MilitaryRank::MilitaryTribune).is_err());
    assert_eq!(c.military.rank(owner), MilitaryRank::Centurion);
    assert_eq!(c.economy.players[0].influence, 99.);

    c.economy.players[0].influence = 600.;
    c.promote_military(0, MilitaryRank::MilitaryTribune).unwrap();
    assert_eq!(c.economy.players[0].influence, 500.);
    assert!(c.promote_military(0, MilitaryRank::MilitaryTribune).is_err());
    c.promote_military(0, MilitaryRank::Legate).unwrap();
    c.promote_military(0, MilitaryRank::Imperator).unwrap();
    assert_eq!(c.military.rank(owner), MilitaryRank::Imperator);
    assert_eq!(c.economy.players[0].influence, 0.);
    assert_eq!(c.actors[0].influence, 0.);
    assert_eq!(
        c.notifications
            .history_for(0)
            .filter(|notice| notice.kind == NoticeKind::MilitaryRankIncreased)
            .count(),
        3
    );
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
    c.military.config.units[UnitType::LightInfantry as usize].offense = 0.;
    c.military.config.base_manpower_damage = 2.;
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
