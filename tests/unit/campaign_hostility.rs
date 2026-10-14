//! Hostile-neighbour attacks through the authoritative campaign clock.

use super::*;

fn hostile_campaign() -> Campaign {
    let mut c = campaign();
    c.politics[1].relations[0] = 19.9;
    c.military.seed_unit(1, ForceOwner::Local(1), UnitType::LightInfantry).unwrap();
    c.military.config.npc_invasion_monthly_chance = 1.;
    c.military.config.base_manpower_damage = 0.;
    c.military.config.base_morale_damage = 0.;
    c
}

fn arrive(c: &mut Campaign) {
    let months = c.military.movements[0].required_progress.ceil() as usize;
    for _ in 0..months {
        c.advance_live_month();
    }
    assert!(c.military.movements.is_empty());
}

#[test]
fn border_attacks_roll_only_at_month_boundaries_and_keep_real_travel_records() {
    let mut c = hostile_campaign();
    let unit = c.military.all_units().next().unwrap().id;
    c.graph[0].area = 900.;
    for _ in 0..5 {
        c.reconcile_provinces();
        c.advance_live_combat();
    }
    assert!(c.military.movements.is_empty());
    assert!(!c.npc_wars[0][1]);
    c.advance_live_month();
    assert_eq!(c.economy.month, 1);
    assert_eq!(c.military.movements.len(), 1);
    let movement = &c.military.movements[0];
    assert_eq!(movement.owner, ForceOwner::Local(1));
    assert_eq!(movement.origin, 1);
    assert_eq!(movement.route, [0]);
    assert_eq!(movement.units[0].id, unit);
    assert_eq!(movement.attack_target, Some(ForceOwner::Player(0)));
    assert_eq!(movement.progress, 0.);
    assert!(movement.required_progress > 1.);
    assert!(c.npc_wars[0][1]);
    assert!(!c.npc_wars[1][1]);
    assert!(c.military.battles.is_empty());
    assert!(c.notifications.history_for(0).any(|notice| {
        notice.kind == NoticeKind::InvasionBegins
            && notice.province == Some(0)
            && notice.body.contains("Test 1")
    }));
    c.advance_live_month();
    assert_eq!(c.military.movements.len(), 1, "No duplicate attack while marching");
    assert_eq!(c.military.all_units().count(), 1, "An invasion never creates extra cohorts");
}

#[test]
fn exactly_twenty_relation_is_safe_but_very_hostile_neighbours_can_attack() {
    for (relation, attacks) in [(40., false), (20., false), (19.9, true), (0., true)] {
        let mut c = hostile_campaign();
        c.politics[1].relations[0] = relation;
        c.advance_live_month();
        assert_eq!(!c.military.movements.is_empty(), attacks, "Relation {relation}");
        assert_eq!(c.npc_wars[0][1], attacks);
    }
}

#[test]
fn a_monthly_attack_is_probabilistic_and_rechecks_repaired_relations() {
    let chance = MilitaryConfig::default().npc_invasion_monthly_chance;
    assert_eq!(chance, 0.10);
    for succeeds in [false, true] {
        let seed =
            (0..1000).find(|&seed| (PoliticalRng::new(seed).unit() < chance) == succeeds).unwrap();
        let mut c = hostile_campaign();
        c.diplomacy_rng = PoliticalRng::new(seed);
        c.military.config.npc_invasion_monthly_chance = chance;
        c.advance_live_month();
        assert_eq!(!c.military.movements.is_empty(), succeeds);
        if !succeeds {
            c.politics[1].relations[0] = 20.;
            c.military.config.npc_invasion_monthly_chance = 1.;
            c.advance_live_month();
            assert!(c.military.movements.is_empty());
            assert!(!c.npc_wars[0][1]);
        }
    }
}

#[test]
fn ineligible_neighbours_and_targets_do_not_launch_an_attack() {
    for case in [
        "no army",
        "no morale",
        "no manpower",
        "no border",
        "owned",
        "Rome",
        "rebels",
        "occupation",
        "defeated",
        "occupied target",
    ] {
        let mut c = hostile_campaign();
        match case {
            "no army" => c.military.provinces[1].forces.clear(),
            "no morale" => {
                c.military.provinces[1].forces.get_mut(&ForceOwner::Local(1)).unwrap()[0].morale =
                    0.
            },
            "no manpower" => {
                c.military.provinces[1].forces.get_mut(&ForceOwner::Local(1)).unwrap()[0]
                    .current_manpower = 0.
            },
            "no border" => c.graph[1].neighbors.retain(|&id| id != 0),
            "owned" => c.politics[1] = ProvincePolitics::owned(2, 1),
            "Rome" => c.politics[1] = ProvincePolitics::rome(2),
            "rebels" => c.military.provinces[1].slave_rebellion = true,
            "occupation" => c.military.provinces[1].occupation = Some(ForceOwner::Player(1)),
            "defeated" => c.defeated[0] = true,
            "occupied target" => c.military.provinces[0].occupation = Some(ForceOwner::Player(1)),
            _ => unreachable!(),
        }
        c.advance_npc_aggression();
        assert!(c.military.movements.is_empty(), "{case}");
        assert!(!c.npc_wars[0][1], "{case}");
    }
}

#[test]
fn hostile_visitors_are_fought_at_home_before_any_border_attack() {
    let mut c = hostile_campaign();
    c.military.seed_unit(1, ForceOwner::Player(0), UnitType::HeavyInfantry).unwrap();
    c.advance_live_month();
    assert!(c.military.movements.is_empty());
    assert_eq!(c.military.battles.len(), 1);
    let battle = &c.military.battles[0];
    assert_eq!(battle.province, 1);
    assert!(battle.attackers.plans.contains_key(&ForceOwner::Local(1)));
    assert!(battle.defenders.plans.contains_key(&ForceOwner::Player(0)));
    assert!(c.senate.accusations.is_empty());
}

#[test]
fn victorious_locals_can_attack_a_border_after_defeating_visitors_at_home() {
    let mut c = hostile_campaign();
    let local = ForceOwner::Local(1);
    c.military.seed_unit(1, ForceOwner::Player(0), UnitType::HeavyInfantry).unwrap();
    c.reconcile_provinces();
    c.military.battles[0].defenders.units.iter_mut().for_each(|unit| unit.current_manpower = 0.);
    c.advance_live_combat();
    assert!(c.military.battles.is_empty());
    assert!(c.military.provinces[1].forces.get(&local).is_some_and(|units| !units.is_empty()));
    c.advance_live_month();
    assert_eq!(c.military.movements.len(), 1);
    assert_eq!(c.military.movements[0].owner, local);
    assert_eq!(c.military.movements[0].route, [0]);
}

#[test]
fn a_hostile_vassal_can_attack_and_selects_one_weak_adjacent_owned_province() {
    let mut c = hostile_campaign();
    c.politics[1].state = PoliticalState::Vassal {
        overlord: 0,
        control: 100.,
        tribute: Tribute::Normal,
    };
    c.politics[2] = ProvincePolitics::owned(2, 0);
    c.economy.provinces[2].owner = Some(0);
    c.military.seed_unit(0, ForceOwner::Player(0), UnitType::HeavyInfantry).unwrap();
    c.advance_npc_aggression();
    assert_eq!(c.military.movements.len(), 1);
    assert_eq!(c.military.movements[0].route, [2]);
    assert!(c.npc_wars[0][1]);
    c.advance_npc_aggression();
    assert_eq!(c.military.movements.len(), 1);
}

#[test]
fn npc_arrival_gives_the_player_walls_and_does_not_create_player_scandals() {
    use crate::game::politics::espionage::ScandalKind;

    let mut c = hostile_campaign();
    let player = ForceOwner::Player(0);
    let local = ForceOwner::Local(1);
    let unit = c.military.seed_unit(0, player, UnitType::HeavyInfantry).unwrap();
    c.economy.provinces[0].buildings[BuildingType::CityWalls as usize] = 3;
    c.advance_live_month();
    // Repairing relations does not undo an army already committed to an invasion.
    c.politics[1].relations[0] = 60.;
    arrive(&mut c);
    assert_eq!(c.military.battles.len(), 1);
    let battle = &c.military.battles[0];
    assert_eq!(battle.province, 0);
    assert_eq!(battle.attacker_origin, Some(1));
    assert!(battle.attackers.plans.contains_key(&local));
    assert_eq!(battle.defenders.units[0].id, unit);
    assert_eq!(battle.fortification_level, 3);
    assert_eq!(battle.round, 0);
    assert!(c.senate.accusations.is_empty());
    assert!(!c.espionage.opportunities.iter().any(|opportunity| {
        opportunity.player == 0 && opportunity.kind == ScandalKind::TreatyViolation
    }));
}

#[test]
fn unguarded_land_is_occupied_and_a_relief_army_can_restore_access() {
    let mut c = hostile_campaign();
    c.advance_live_month();
    arrive(&mut c);
    let local = ForceOwner::Local(1);
    assert_eq!(c.military.provinces[0].occupation, Some(local));
    assert_eq!(c.economy.provinces[0].owner, Some(0));
    assert!(c.economy.provinces[0].occupied);
    assert!(!c.economy.provinces[0].can_administer(0));
    assert!(c
        .notifications
        .history_for(0)
        .any(|notice| notice.kind == NoticeKind::OccupationEstablished));
    c.military.seed_unit(0, ForceOwner::Player(0), UnitType::HeavyInfantry).unwrap();
    c.begin_encounter(0, ForceOwner::Player(0), Some(1));
    assert!(c.military.battles[0].attackers.plans.contains_key(&ForceOwner::Player(0)));
    c.military.battles[0].defenders.units.iter_mut().for_each(|unit| unit.current_manpower = 0.);
    c.advance_live_combat();
    assert!(c.military.battles.is_empty());
    assert_eq!(c.military.provinces[0].occupation, None);
    assert!(c.economy.provinces[0].can_administer(0));
}

#[test]
fn a_victorious_npc_army_occupies_defended_land() {
    let mut c = hostile_campaign();
    c.military.seed_unit(0, ForceOwner::Player(0), UnitType::HeavyInfantry).unwrap();
    c.advance_live_month();
    arrive(&mut c);
    c.military.battles[0].defenders.units.iter_mut().for_each(|unit| unit.current_manpower = 0.);
    c.advance_live_combat();
    assert!(c.military.battles.is_empty());
    assert_eq!(c.military.history[0].result, BattleResult::AttackerVictory);
    assert_eq!(c.military.provinces[0].occupation, Some(ForceOwner::Local(1)));
    assert!(c.economy.provinces[0].occupied);
    assert!(c.notifications.history_for(0).any(|notice| notice.kind == NoticeKind::BattleResolved));
}

#[test]
fn a_defeated_npc_attacker_retreats_home_without_cloning_units() {
    let mut c = hostile_campaign();
    let local = ForceOwner::Local(1);
    let unit = c.military.all_units().next().unwrap().id;
    c.military.seed_unit(0, ForceOwner::Player(0), UnitType::HeavyInfantry).unwrap();
    c.advance_live_month();
    arrive(&mut c);
    for _ in 0..c.military.config.rounds_per_month {
        c.advance_live_combat();
    }
    c.military.battles[0].request_retreat(true, &c.military.config).unwrap();
    c.advance_live_combat();
    assert!(c.military.battles.is_empty());
    assert_eq!(c.military.history[0].result, BattleResult::DefenderVictory);
    assert_eq!(c.military.provinces[1].forces[&local][0].id, unit);
    assert_eq!(c.military.all_units().filter(|unit| unit.owner == local).count(), 1);
    assert_eq!(c.military.provinces[0].occupation, None);
    assert!(c.notifications.history_for(0).any(|notice| notice.kind == NoticeKind::NpcDefeated));
}

#[test]
fn invading_npc_troops_use_home_food_instead_of_the_defenders_stockpile() {
    let mut c = hostile_campaign();
    c.advance_live_month();
    assert!(c.inputs().npc_army_food[1] > 0.);
    assert_eq!(c.inputs().army_food[0], 0.);
    arrive(&mut c);
    assert!(c.inputs().npc_army_food[1] > 0.);
    assert_eq!(c.inputs().army_food[0], 0.);
    assert_eq!(c.food_breakdown(0).iter().map(|(_, _, _, food)| food).sum::<f64>(), 0.);
    c.military.seed_unit(0, ForceOwner::Player(0), UnitType::HeavyInfantry).unwrap();
    c.begin_encounter(0, ForceOwner::Player(0), None);
    let player_food = c.military.battles[0].attackers.units[0].food_demand(&c.military.config);
    assert_eq!(c.inputs().army_food[0], player_food);
    assert!(c.inputs().npc_army_food[1] > 0.);
}

#[test]
fn saving_preserves_random_attack_outcomes_and_older_saves_get_default_rules() {
    let mut original = hostile_campaign();
    original.military.config.npc_invasion_monthly_chance = 0.10;
    original.diplomacy_rng = PoliticalRng::new(7);
    let snapshot = serde_json::to_value(&original).unwrap();
    let mut restored: Campaign = serde_json::from_value(snapshot.clone()).unwrap();
    for _ in 0..8 {
        original.advance_live_month();
        restored.advance_live_month();
        assert_eq!(
            serde_json::to_value(&original).unwrap(),
            serde_json::to_value(&restored).unwrap()
        );
    }
    let mut legacy = snapshot;
    let config = legacy["military"]["config"].as_object_mut().unwrap();
    config.remove("npc_invasion_relation");
    config.remove("npc_invasion_monthly_chance");
    let legacy: Campaign = serde_json::from_value(legacy).unwrap();
    assert_eq!(legacy.military.config.npc_invasion_relation, 20.);
    assert_eq!(legacy.military.config.npc_invasion_monthly_chance, 0.10);
}

#[test]
fn npc_reinforcements_join_compatible_battles_and_wait_if_both_sides_are_hostile() {
    for hostile_to_both in [false, true] {
        let mut c = hostile_campaign();
        c.advance_npc_aggression();
        c.military.movements[0].required_progress = 1.;
        c.npc_wars[1][1] = hostile_to_both;
        c.wars[0][1] = true;
        c.wars[1][0] = true;
        let player = ForceOwner::Player(0);
        let rival = ForceOwner::Player(1);
        let local = ForceOwner::Local(1);
        c.military.seed_unit(0, player, UnitType::HeavyInfantry).unwrap();
        c.military.seed_unit(0, rival, UnitType::HeavyInfantry).unwrap();
        c.military
            .start_battle(0, &[rival], &[player], Some(0), Some(1), MilitaryTerrain::Plains, 0, 1)
            .unwrap();
        c.advance_live_month();
        assert_eq!(c.military.battles.len(), 1);
        let battle = &c.military.battles[0];
        assert_eq!(battle.attackers.plans.contains_key(&local), !hostile_to_both);
        assert!(!battle.defenders.plans.contains_key(&local));
        assert_eq!(battle.round, 0);
        let waiting =
            c.military.provinces[0].forces.get(&local).is_some_and(|units| !units.is_empty());
        assert_eq!(waiting, hostile_to_both);
        assert_eq!(c.military.all_units().count(), 3);
    }
}

#[test]
fn an_npc_border_attack_suspends_trade_without_a_player_treaty_scandal() {
    use crate::game::politics::espionage::ScandalKind;

    let mut c = hostile_campaign();
    c.politics[1].relations[0] = 0.;
    let mut trade = TradeAgreement::new(
        TradeParty::Player(0),
        TradeParty::Npc(1),
        TradeBundle {
            coin: 1.,
            ..Default::default()
        },
        TradeBundle::default(),
        TradeFrequency::Monthly,
    );
    trade.id = 1;
    trade.accepted = [true; 2];
    trade.status = TradeStatus::Active;
    c.economy.trades.push(trade);
    c.advance_live_month();
    assert!(c.npc_wars[0][1]);
    assert_eq!(c.economy.trades[0].status, TradeStatus::Suspended);
    assert!(!c.espionage.opportunities.iter().any(|opportunity| {
        opportunity.player == 0 && opportunity.kind == ScandalKind::TreatyViolation
    }));
}

#[test]
fn unopposed_npc_occupation_uses_the_same_monthly_control_curve_as_player_occupation() {
    let mut c = hostile_campaign();
    let occupier = ForceOwner::Local(1);
    let unit = c.military.all_units().next().unwrap().id;
    c.advance_live_month();
    arrive(&mut c);
    assert_eq!(c.politics[0].control(0), 100., "Arrival alone does not apply political pressure");
    for _ in 0..3 {
        let before = c.politics[0].control(0);
        c.advance_live_month();
        let power = c.military.stationed_strength(0, occupier)
            * c.military.config.rank_control[c.military.rank(occupier) as usize];
        let mut comparison = ProvincePolitics::independent(2);
        comparison.resolve_month(&[power, 0.], Some(0), 0., &c.diplomacy_config);
        assert!((before - c.politics[0].control(0) - comparison.control(0)).abs() < 1e-7);
        assert!(c.politics[0].control(0) < before);
        assert_eq!(c.military.provinces[0].forces[&occupier][0].id, unit);
        assert_eq!(c.military.all_units().count(), 1);
    }
    let before = c.politics[0].control(0);
    c.reconcile_provinces();
    c.advance_live_combat();
    assert_eq!(
        c.politics[0].control(0),
        before,
        "Pressure is monthly, not per frame or combat round"
    );
}

#[test]
fn relief_troops_stop_occupation_pressure_on_arrival_and_liberation_does_not_refund_control() {
    let mut c = hostile_campaign();
    c.politics[2] = ProvincePolitics::owned(2, 0);
    c.economy.provinces[2].owner = Some(0);
    c.graph[0].neighbors.push(2);
    c.graph[2].neighbors.push(0);
    c.graph[2].area = 90_000.;
    c.advance_live_month();
    arrive(&mut c);
    let player = ForceOwner::Player(0);
    let unit = c.military.seed_unit(2, player, UnitType::HeavyInfantry).unwrap();
    c.order_army(2, 0, 0, &[unit], &[0], BattlePlan::default(), ArmyOrderKind::Attack).unwrap();
    assert!(c.military.movements[0].required_progress > 1.);
    let before = c.politics[0].control(0);
    c.advance_live_month();
    assert!(c.politics[0].control(0) < before, "Troops still marching do not protect the province");
    assert!(c.military.battles.is_empty());
    let before_arrival = c.politics[0].control(0);
    c.military.movements[0].required_progress = c.military.movements[0].progress + 1.;
    c.advance_live_month();
    assert_eq!(c.military.battles.len(), 1);
    assert_eq!(c.politics[0].control(0), before_arrival);
    assert!(c.economy.provinces[0].occupied, "Relief combat alone does not end the occupation");
    c.advance_live_month();
    assert_eq!(c.politics[0].control(0), before_arrival, "Fighting troops stop further pressure");
    c.military.battles[0].defenders.units.iter_mut().for_each(|unit| unit.current_manpower = 0.);
    c.advance_live_combat();
    assert_eq!(c.military.provinces[0].occupation, None);
    assert_eq!(c.politics[0].control(0), before_arrival, "Winning does not refund lost Control");
    assert!(c.economy.provinces[0].can_administer(0));
    c.advance_live_month();
    assert!(
        c.politics[0].control(0) > before_arrival,
        "The surviving garrison can restore Control normally"
    );
}

#[test]
fn zero_control_releases_npc_occupied_land_and_only_the_last_owned_loss_defeats_the_player() {
    for last_owned in [false, true] {
        let mut c = hostile_campaign();
        if !last_owned {
            c.politics[2] = ProvincePolitics::owned(2, 0);
            c.economy.provinces[2].owner = Some(0);
        }
        c.economy.provinces[0].buildings[BuildingType::Granary as usize] = 2;
        c.advance_live_month();
        arrive(&mut c);
        let occupier = ForceOwner::Local(1);
        let units = c.military.provinces[0].forces[&occupier].clone();
        c.politics[0].owned_shares = vec![0.1, 20.];
        c.advance_live_month();
        assert_eq!(c.politics[0].control(0), 0.);
        assert_eq!(c.politics[0].control(1), 20.);
        assert!(matches!(
            c.politics[0].state,
            PoliticalState::Independent {
                local: 80.0,
                ..
            }
        ));
        assert_eq!(c.economy.provinces[0].owner, None);
        assert_eq!(c.economy.provinces[0].overlord, None);
        assert_eq!(c.military.provinces[0].occupation, None);
        assert!(!c.economy.provinces[0].occupied);
        assert_eq!(c.economy.provinces[0].level(BuildingType::Granary), 2);
        assert_eq!(c.defeated[0], last_owned);
        assert_eq!(c.military.provinces[0].forces[&occupier][0].id, units[0].id);
        assert_eq!(c.military.provinces[0].forces[&occupier].len(), units.len());
        c.reconcile_provinces();
        c.advance_live_month();
        assert_eq!(
            c.politics[0].control(0),
            0.,
            "Independent land cannot revert to owned legacy Control"
        );
        assert_eq!(
            c.notifications
                .history_for(0)
                .filter(|notice| notice.title == "Test 0 became independent")
                .count(),
            1
        );
        assert_eq!(
            c.notifications
                .history_for(0)
                .filter(|notice| notice.kind == NoticeKind::PlayerDefeated)
                .count(),
            usize::from(last_owned)
        );
        c.politics[0].gain_control_now(1, 80.).unwrap();
        c.politics[0].take_ownership(1).unwrap();
        c.reconcile_provinces();
        assert!(
            c.economy.provinces[0].can_administer(1),
            "Independence clears the previous occupation's management block"
        );
    }
}

#[test]
fn empty_or_broken_npc_occupiers_do_not_drain_control() {
    for broken_morale in [false, true] {
        let mut c = hostile_campaign();
        c.advance_live_month();
        arrive(&mut c);
        let before = c.politics[0].control(0);
        let unit = &mut c.military.provinces[0].forces.get_mut(&ForceOwner::Local(1)).unwrap()[0];
        if broken_morale {
            unit.morale = 0.;
        } else {
            unit.current_manpower = 0.;
        }
        c.advance_live_month();
        assert_eq!(c.politics[0].control(0), before);
        assert_eq!(c.military.provinces[0].occupation, None);
    }
}

#[test]
fn saves_replay_occupation_control_loss_and_independence_identically() {
    let mut c = hostile_campaign();
    c.advance_live_month();
    arrive(&mut c);
    c.politics[0].owned_shares = vec![0.1, 20.];
    let mut restored: Campaign = serde_json::from_value(serde_json::to_value(&c).unwrap()).unwrap();
    for _ in 0..3 {
        c.advance_live_month();
        restored.advance_live_month();
        assert_eq!(serde_json::to_value(&c).unwrap(), serde_json::to_value(&restored).unwrap());
    }
    assert!(matches!(restored.politics[0].state, PoliticalState::Independent { .. }));
    assert!(restored.defeated[0]);
}
