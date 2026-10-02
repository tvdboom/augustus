//! Invariant and balance regression coverage for irreversible military changes.

use super::*;

#[test]
fn d6_rounds_are_seeded_shared_and_record_actual_simultaneous_losses() {
    let config = MilitaryConfig::default();
    let make = || {
        Battle::new(
            41,
            0,
            None,
            None,
            MilitaryTerrain::Plains,
            0,
            side(vec![unit(1, ForceOwner::Player(0), UnitType::HeavyInfantry)], 4, &config),
            side(vec![unit(2, ForceOwner::Player(1), UnitType::HeavyInfantry)], 4, &config),
            177,
        )
    };
    let mut a = make();
    let mut b = make();
    a.advance_month(&config);
    for _ in 0..config.rounds_per_month {
        b.advance_timed_round(&config);
    }
    assert_eq!(a.months, b.months);
    assert_eq!(a.rounds.len(), config.rounds_per_month);
    for (left, right) in a.rounds.iter().zip(&b.rounds) {
        assert_eq!(left.dice, right.dice);
        assert!(left.dice.iter().all(|roll| (1..=6).contains(roll)));
        assert_eq!(left.dice_multipliers, left.dice.map(|roll| dice_multiplier(roll, &config)));
        assert_eq!(left.casualties, right.casualties);
    }
    let attacker_lost: u64 = a.rounds.iter().map(|r| r.casualties[0]).sum();
    let defender_lost: u64 = a.rounds.iter().map(|r| r.casualties[1]).sum();
    assert_eq!(attacker_lost, 1_000 - a.attackers.units.iter().map(Unit::people).sum::<u64>());
    assert_eq!(defender_lost, 1_000 - a.defenders.units.iter().map(Unit::people).sum::<u64>());
    assert_eq!(dice_multiplier(1, &config), config.random_range[0]);
    assert_eq!(dice_multiplier(6, &config), config.random_range[1]);
}

#[test]
fn timed_combat_deadline_matches_monthly_resolution() {
    let config = MilitaryConfig {
        base_manpower_damage: 0.,
        base_morale_damage: 0.,
        ..Default::default()
    };
    let mut battle = Battle::new(
        1,
        1,
        None,
        Some(0),
        MilitaryTerrain::Plains,
        0,
        side(vec![unit(1, ForceOwner::Player(0), UnitType::HeavyInfantry)], 4, &config),
        side(vec![unit(2, ForceOwner::Local(1), UnitType::HeavyInfantry)], 4, &config),
        10,
    );
    let rounds = config.maximum_battle_months * config.rounds_per_month;
    for tick in 1..=rounds {
        battle.advance_timed_round(&config);
        assert_eq!(battle.round, tick);
        assert_eq!(battle.months, tick / config.rounds_per_month);
        assert_eq!(battle.result, (tick == rounds).then_some(BattleResult::DefenderVictory));
    }
}
use std::collections::{BTreeMap, BTreeSet};

fn graph() -> Vec<MilitaryProvince> {
    (0..3)
        .map(|i| MilitaryProvince {
            terrain: MilitaryTerrain::Plains,
            area: 100.,
            road_level: 0,
            neighbors: match i {
                0 => vec![1],
                1 => vec![0, 2],
                _ => vec![1],
            },
        })
        .collect()
}
fn unit(id: u64, owner: ForceOwner, kind: UnitType) -> Unit {
    let config = MilitaryConfig::default();
    let manpower = config.unit(kind).manpower;
    Unit {
        id,
        owner,
        unit_type: kind,
        current_manpower: manpower,
        max_manpower: manpower,
        training: 10.,
        morale: 50.,
    }
}
fn side(units: Vec<Unit>, width: usize, config: &MilitaryConfig) -> BattleSide {
    let plans = units.iter().map(|u| (u.owner, BattlePlan::default())).collect();
    let ranks = units.iter().map(|u| (u.owner, MilitaryRank::Centurion)).collect();
    BattleSide::new(units, plans, ranks, width, config)
}

#[test]
fn population_forces_scale_with_residents_and_preserve_partial_cohorts() {
    for (population, cohorts, food) in
        [(12.0, 2, 1.8), (120.0, 12, 18.0), (300_000.0, 30_000, 45_000.0)]
    {
        let mut world = MilitaryWorld::new(1);
        let owner = ForceOwner::Local(0);
        assert_eq!(
            world.seed_population_force(0, owner, UnitType::LightInfantry, population).unwrap(),
            cohorts
        );
        assert_eq!(world.total_manpower(owner), population);
        assert_eq!(world.peak_manpower[&owner], population);
        assert!((world.food_demand(owner) - food).abs() < 1e-8);
        assert!(world.provinces[0].forces[&owner].iter().all(|unit| {
            unit.max_manpower == 10.0
                && unit.current_manpower <= unit.max_manpower
                && unit.people() > 0
        }));
    }
}

#[test]
fn cohorts_display_whole_people_and_combat_removes_whole_people() {
    let config = MilitaryConfig::default();
    assert_eq!(config.unit(UnitType::HeavyInfantry).cohort_people(), 1_000);
    assert_eq!(config.unit(UnitType::WarElephants).cohort_people(), 1_000);
    let infantry = unit(1, ForceOwner::Player(0), UnitType::HeavyInfantry);
    let elephants = unit(2, ForceOwner::Player(1), UnitType::WarElephants);
    assert_eq!((infantry.people(), elephants.people()), (1_000, 1_000));
    let mut battle = Battle::new(
        1,
        0,
        None,
        None,
        MilitaryTerrain::Plains,
        0,
        side(vec![infantry], 4, &config),
        side(vec![elephants], 4, &config),
        17,
    );
    battle.advance_round(&config);
    for cohort in battle.attackers.units.iter().chain(&battle.defenders.units) {
        let people = cohort.current_manpower * PEOPLE_PER_POPULATION;
        assert!((people - people.round()).abs() < 1e-9);
        assert!(cohort.people() < cohort.max_people());
    }
}

#[test]
fn cohort_wages_reflect_type_and_surviving_strength() {
    let config = MilitaryConfig::default();
    let mut infantry = unit(1, ForceOwner::Player(0), UnitType::HeavyInfantry);
    let elephants = unit(2, ForceOwner::Player(0), UnitType::WarElephants);
    assert_eq!(infantry.coin_demand(&config), 2.0);
    assert_eq!(elephants.coin_demand(&config), 5.0);
    infantry.current_manpower *= 0.5;
    assert_eq!(infantry.coin_demand(&config), 1.0);
}

#[test]
fn army_strength_never_returns_negative_zero() {
    let config = MilitaryConfig::default();
    let mut cohort = unit(1, ForceOwner::Player(0), UnitType::HeavyInfantry);
    cohort.current_manpower = -0.0;
    assert_eq!(format!("{:.0}", cohort.effective_strength(&config)), "0");
    cohort.current_manpower = -1.0;
    assert_eq!(cohort.effective_strength(&config), 0.0);
}

#[test]
fn merging_packs_same_type_and_weights_morale_by_people() {
    let mut world = MilitaryWorld::new(1);
    let owner = ForceOwner::Player(0);
    for _ in 0..3 {
        world.seed_unit(0, owner, UnitType::HeavyInfantry).unwrap();
    }
    let units = world.provinces[0].forces.get_mut(&owner).unwrap();
    for (unit, (people, morale)) in units.iter_mut().zip([(800, 80.), (400, 60.), (200, 20.)]) {
        unit.current_manpower = people as f64 / PEOPLE_PER_POPULATION;
        unit.morale = morale;
    }
    assert_eq!(world.merge_army(0, owner).unwrap(), 1);
    let units = &world.provinces[0].forces[&owner];
    assert_eq!(units.iter().map(Unit::people).collect::<Vec<_>>(), vec![1_000, 400]);
    assert!((units[0].morale - 68.).abs() < 1e-9);
    assert_eq!(units[1].morale, 60.);
    assert_eq!(units.iter().map(Unit::people).sum::<u64>(), 1_400);
}

#[test]
fn highest_rank_recovers_five_percent_manpower_and_five_morale() {
    let mut world = MilitaryWorld::new(1);
    let owner = ForceOwner::Player(0);
    world.seed_unit(0, owner, UnitType::WarElephants).unwrap();
    world.ranks.insert(owner, MilitaryRank::Imperator);
    let unit = &mut world.provinces[0].forces.get_mut(&owner).unwrap()[0];
    unit.current_manpower = 2.;
    unit.morale = 50.;
    world.apply_supply(owner, 1.);
    let unit = &world.provinces[0].forces[&owner][0];
    assert_eq!(unit.people(), 250);
    assert_eq!(unit.morale, 55.);
    assert_eq!(unit.food_demand(&world.config), 1.5);
    assert_eq!(unit.coin_demand(&world.config), 1.25);
}

#[test]
fn morale_below_twenty_routes_only_when_escape_is_possible() {
    let config = MilitaryConfig {
        base_manpower_damage: 0.,
        base_morale_damage: 0.,
        ..Default::default()
    };
    let a = ForceOwner::Player(0);
    let d = ForceOwner::Player(1);
    let mut attacker = unit(1, a, UnitType::HeavyInfantry);
    attacker.morale = 19.;
    let mut defender = unit(2, d, UnitType::HeavyInfantry);
    defender.morale = 20.;
    let mut battle = Battle::new(
        1,
        0,
        None,
        None,
        MilitaryTerrain::Plains,
        0,
        side(vec![attacker.clone()], 4, &config),
        side(vec![defender.clone()], 4, &config),
        1,
    );
    battle.advance_round(&config);
    assert_eq!(battle.result, Some(BattleResult::DefenderVictory));
    assert!(battle.attackers.routed.contains(&attacker.id));
    assert_eq!(battle.attackers.units[0].people(), 1_000);
    assert!(!battle.defenders.routed.contains(&defender.id));

    let mut trapped = Battle::new(
        2,
        0,
        None,
        None,
        MilitaryTerrain::Plains,
        0,
        side(vec![attacker], 4, &config),
        side(vec![defender], 4, &config),
        1,
    );
    trapped.trapped.insert(a);
    trapped.advance_round(&config);
    assert_eq!(trapped.result, None);
    assert!(trapped.attackers.routed.is_empty());
}

#[test]
fn depleted_cohort_deals_damage_in_proportion_to_its_people() {
    let mut config = MilitaryConfig {
        random_range: [1., 1.],
        ..Default::default()
    };
    config.units[UnitType::LightInfantry as usize].offense = 0.;
    let damage = |people: u64| {
        let mut attacker = unit(1, ForceOwner::Player(0), UnitType::HeavyInfantry);
        attacker.current_manpower = people as f64 / PEOPLE_PER_POPULATION;
        let defender = unit(2, ForceOwner::Player(1), UnitType::LightInfantry);
        let mut battle = Battle::new(
            1,
            0,
            None,
            None,
            MilitaryTerrain::Plains,
            0,
            side(vec![attacker], 4, &config),
            side(vec![defender], 4, &config),
            5,
        );
        battle.advance_round(&config);
        (1_000 - battle.defenders.units[0].people(), 50. - battle.defenders.units[0].morale)
    };
    let full = damage(1_000);
    let fifth = damage(200);
    assert!((fifth.0 as f64 - full.0 as f64 * 0.2).abs() <= 1.);
    assert!((fifth.1 - full.1 * 0.2).abs() < 1e-9);
}

#[test]
fn friendly_npc_passage_allows_routes_and_waypoints_but_not_stationing() {
    let mut world = MilitaryWorld::new(3);
    let owner = ForceOwner::Player(0);
    let id = world.seed_unit(0, owner, UnitType::LightInfantry).unwrap();
    let units = &world.provinces[0].forces[&owner];
    let access = |_, province| {
        if province == 1 {
            MilitaryAccess::Transit
        } else {
            MilitaryAccess::Peaceful
        }
    };
    assert_eq!(
        fastest_route(&graph(), 0, 1, owner, units, access, &world.config),
        Err(MilitaryError::NoLegalRoute)
    );
    assert_eq!(
        route_via(&graph(), 0, 2, &[1], owner, units, access, &world.config).unwrap(),
        vec![1, 2]
    );
    assert!(world.order_movement_route(0, owner, &[id], None, &[1], &graph(), access).is_err());
    assert_eq!(world.provinces[0].forces[&owner].len(), 1);
    world.order_movement(0, 2, owner, &[id], None, &graph(), access).unwrap();
    for _ in 0..10 {
        world.advance_movement(&graph(), access);
        assert!(world.provinces[1].forces.get(&owner).is_none_or(Vec::is_empty));
    }
    assert!(world.movements.is_empty());
    assert_eq!(world.provinces[2].forces[&owner][0].id, id);
}

#[test]
fn stationing_permission_is_rechecked_before_arrival() {
    let mut world = MilitaryWorld::new(3);
    let owner = ForceOwner::Player(0);
    let id = world.seed_unit(0, owner, UnitType::LightInfantry).unwrap();
    world
        .order_movement(0, 1, owner, &[id], None, &graph(), |_, _| MilitaryAccess::Peaceful)
        .unwrap();
    world.advance_movement(&graph(), |_, province| {
        if province == 1 {
            MilitaryAccess::Transit
        } else {
            MilitaryAccess::Peaceful
        }
    });
    assert!(world.movements.is_empty());
    assert_eq!(world.provinces[0].forces[&owner][0].id, id);
    assert!(world.provinces[1].forces.get(&owner).is_none_or(Vec::is_empty));
}

#[test]
fn interrupted_passage_withdraws_without_stationing_in_friendly_province() {
    let mut world = MilitaryWorld::new(3);
    let owner = ForceOwner::Player(0);
    let id = world.seed_unit(0, owner, UnitType::LightInfantry).unwrap();
    let access = |_, province| {
        if province == 1 {
            MilitaryAccess::Transit
        } else {
            MilitaryAccess::Peaceful
        }
    };
    world.order_movement(0, 2, owner, &[id], None, &graph(), access).unwrap();
    for _ in 0..10 {
        if world.movements[0].origin == 1 {
            break;
        }
        world.advance_movement(&graph(), access);
    }
    assert_eq!(world.movements[0].origin, 1);
    for _ in 0..10 {
        world.advance_movement(&graph(), |_, province| match province {
            0 => MilitaryAccess::Peaceful,
            1 => MilitaryAccess::Transit,
            _ => MilitaryAccess::Blocked,
        });
        assert!(world.provinces[1].forces.get(&owner).is_none_or(Vec::is_empty));
    }
    assert!(world.movements.is_empty());
    assert_eq!(world.provinces[0].forces[&owner][0].id, id);
}

#[test]
fn assembling_armies_preserves_each_cohorts_training_and_morale() {
    let mut world = MilitaryWorld::new(3);
    let own = ForceOwner::Player(0);
    let guest = ForceOwner::Player(1);
    let first = world.seed_unit(0, own, UnitType::HeavyInfantry).unwrap();
    let arriving = world.seed_unit(1, own, UnitType::HeavyInfantry).unwrap();
    world.seed_unit(0, own, UnitType::Archers).unwrap();
    world.seed_unit(0, guest, UnitType::HeavyInfantry).unwrap();
    for unit in world.provinces[0].forces.get_mut(&own).unwrap() {
        unit.morale = 40.;
        unit.training = if unit.unit_type == UnitType::HeavyInfantry {
            10.
        } else {
            80.
        };
    }
    let incoming = &mut world.provinces[1].forces.get_mut(&own).unwrap()[0];
    incoming.training = 20.;
    incoming.morale = 80.;
    let before = world.all_units().map(|unit| unit.current_manpower).sum::<f64>();
    world
        .order_movement(1, 0, own, &[arriving], None, &graph(), |_, _| MilitaryAccess::Peaceful)
        .unwrap();
    for _ in 0..20 {
        world.advance_movement(&graph(), |_, _| MilitaryAccess::Peaceful);
    }
    let army = &world.provinces[0].forces[&own];
    assert_eq!(army.len(), 3);
    assert!(army.iter().any(|unit| unit.id == first));
    assert!(army.iter().any(|unit| unit.id == arriving));
    assert_eq!(army.iter().find(|unit| unit.id == first).unwrap().training, 10.);
    assert_eq!(army.iter().find(|unit| unit.id == arriving).unwrap().training, 20.);
    assert_eq!(army.iter().find(|unit| unit.unit_type == UnitType::Archers).unwrap().training, 80.);
    assert_eq!(army.iter().find(|unit| unit.id == arriving).unwrap().morale, 80.);
    assert_eq!(army.iter().find(|unit| unit.id == first).unwrap().morale, 40.);
    assert_eq!(world.provinces[0].forces[&guest][0].training, world.config.starting_training);
    assert_eq!(world.all_units().map(|unit| unit.current_manpower).sum::<f64>(), before);
}

#[test]
fn disband_whole_army_is_owner_scoped_and_returns_only_survivors() {
    let mut world = MilitaryWorld::new(1);
    let own = ForceOwner::Player(0);
    let guest = ForceOwner::Player(1);
    world.seed_unit(0, own, UnitType::HeavyInfantry).unwrap();
    world.seed_unit(0, own, UnitType::LightCavalry).unwrap();
    world.seed_unit(0, guest, UnitType::Archers).unwrap();
    let mut expected = [0.; 4];
    for unit in world.provinces[0].forces.get_mut(&own).unwrap() {
        unit.current_manpower *= 0.5;
        let definition = world.config.unit(unit.unit_type);
        expected[definition.manpower_class] += definition.population_cost * unit.manpower_ratio();
    }
    let mut population = [0.; 4];
    assert_eq!(
        world.disband_army(0, 0, false, &mut population),
        Err(MilitaryError::NotDirectlyOwned)
    );
    assert_eq!(population, [0.; 4]);
    assert_eq!(world.provinces[0].forces[&own].len(), 2);
    let mut engaged = world.clone();
    engaged.start_battle(0, &[own], &[guest], None, None, MilitaryTerrain::Plains, 0, 5).unwrap();
    assert_eq!(engaged.disband_army(0, 0, true, &mut population), Err(MilitaryError::InBattle));
    assert_eq!(population, [0.; 4]);
    assert_eq!(engaged.battles[0].attackers.units.len(), 2);
    world.disband_army(0, 0, true, &mut population).unwrap();
    assert_eq!(population, expected);
    assert!(world.provinces[0].forces[&own].is_empty());
    assert_eq!(world.provinces[0].forces[&guest].len(), 1);
}

#[test]
fn battle_losses_affect_each_cohorts_own_morale() {
    let config = MilitaryConfig::default();
    let owner = ForceOwner::Player(0);
    let enemy = ForceOwner::Player(1);
    let mut battle = Battle::new(
        1,
        0,
        Some(1),
        Some(1),
        MilitaryTerrain::Plains,
        0,
        side(
            vec![
                unit(1, owner, UnitType::HeavyInfantry),
                unit(2, owner, UnitType::HeavyInfantry),
                unit(3, owner, UnitType::LightCavalry),
            ],
            4,
            &config,
        ),
        side(
            vec![unit(4, enemy, UnitType::HeavyInfantry), unit(5, enemy, UnitType::LightInfantry)],
            4,
            &config,
        ),
        19,
    );
    battle.advance_round(&config);
    assert!(battle.attackers.units.iter().any(|unit| unit.morale < 50.));
    assert!(battle.attackers.units.iter().any(|unit| unit.morale == 50.));
}

#[test]
fn recruitment_queue_caps_waiting_orders_at_thirteen_without_charging_rejected_orders() {
    let mut world = MilitaryWorld::new(2);
    let mut population = [1_000.; 4];
    let mut metal = 10_000.;
    for _ in 0..=ProvinceMilitaryState::MAX_RECRUITMENT_QUEUE {
        world
            .recruit(0, 0, UnitType::LightInfantry, true, &[], &mut population, &mut metal)
            .unwrap();
    }
    assert!(world.provinces[0].recruitment_queue_full());
    assert!(world.provinces[0].recruitment.is_some());
    assert_eq!(world.provinces[0].recruitment_queue.len(), 13);
    let before = (population, metal, world.provinces[0].draft_penalties);
    assert_eq!(
        world.recruit(0, 0, UnitType::Archers, true, &[], &mut population, &mut metal),
        Err(MilitaryError::RecruitmentBusy)
    );
    assert_eq!((population, metal, world.provinces[0].draft_penalties), before);
    assert_eq!(world.provinces[0].recruitment_queue.len(), 13);
    world.recruit(1, 0, UnitType::LightInfantry, true, &[], &mut population, &mut metal).unwrap();
    world.cancel_queued_recruitment(0, 0, 0, true, &mut population, &mut metal).unwrap();
    assert!(!world.provinces[0].recruitment_queue_full());
    world.recruit(0, 0, UnitType::Archers, true, &[], &mut population, &mut metal).unwrap();
    assert!(world.provinces[0].recruitment_queue_full());
    world.advance_recruitment_with_speed(|_| Some(0), |_| 100.);
    assert!(!world.provinces[0].recruitment_queue_full());
    world.recruit(0, 0, UnitType::LightInfantry, true, &[], &mut population, &mut metal).unwrap();
    assert!(world.provinces[0].recruitment_queue_full());
}

#[test]
fn recruitment_is_atomic_and_uses_real_population_once() {
    let mut world = MilitaryWorld::new(2);
    let mut pops = [5., 10., 20., 10.];
    let mut metal = 100.;
    assert_eq!(
        world.recruit(0, 0, UnitType::HorseArchers, true, &[], &mut pops, &mut metal),
        Err(MilitaryError::MissingRecruitmentTag)
    );
    assert_eq!(pops, [5., 10., 20., 10.]);
    assert_eq!(metal, 100.);
    world.recruit(0, 0, UnitType::HeavyInfantry, true, &[], &mut pops, &mut metal).unwrap();
    assert_eq!(pops, [5., 10., 10., 10.]);
    assert_eq!(metal, 60.);
    world.recruit(0, 0, UnitType::LightInfantry, true, &[], &mut pops, &mut metal).unwrap();
    assert_eq!(pops[2], 0.);
    assert_eq!(metal, 48.);
    assert_eq!(world.provinces[0].recruitment_queue.len(), 1);
    assert!(world.provinces[0].draft_penalties[2] > 0.);
    assert_eq!(world.food_demand(ForceOwner::Player(0)), 0.);
    for _ in 0..3 {
        world.advance_recruitment(|_| Some(0));
    }
    assert_eq!(world.all_units().count(), 1);
    assert_eq!(metal, 48.);
    assert_eq!(world.provinces[0].recruitment.as_ref().unwrap().unit_type, UnitType::LightInfantry);
    assert_eq!(world.provinces[0].recruitment.as_ref().unwrap().progress, 0.);
    for _ in 0..2 {
        world.advance_recruitment(|_| Some(0));
    }
    assert_eq!(world.all_units().count(), 2);
    assert!(world.all_units().all(|unit| unit.max_people() == 1_000));
    assert!(world.provinces[0].recruitment.is_none());
}

#[test]
fn completed_recruits_join_their_locked_battle_side_immediately() {
    for attacker in [true, false] {
        let mut world = MilitaryWorld::new(2);
        let owner = ForceOwner::Player(0);
        let enemy = ForceOwner::Player(1);
        world.config.units[UnitType::LightInfantry as usize].recruitment_months = 1.;
        world.config.base_manpower_damage = 0.;
        world.config.base_morale_damage = 0.;
        let veteran = world.seed_unit(0, owner, UnitType::HeavyInfantry).unwrap();
        world.seed_unit(0, enemy, UnitType::HeavyInfantry).unwrap();
        let plan = BattlePlan {
            tactic: CombatTactic::Phalanx,
            ..Default::default()
        };
        world.set_plan(0, owner, plan).unwrap();
        let mut population = [100.; 4];
        let mut metal = 1000.;
        // One project predates the battle; the next is queued while fighting.
        world
            .recruit(0, 0, UnitType::LightInfantry, true, &[], &mut population, &mut metal)
            .unwrap();
        let (a, d) = if attacker {
            (owner, enemy)
        } else {
            (enemy, owner)
        };
        world.start_battle(0, &[a], &[d], Some(0), None, MilitaryTerrain::Plains, 0, 1).unwrap();
        world.battles[0].advance_round(&world.config);
        world
            .recruit(0, 0, UnitType::LightInfantry, true, &[], &mut population, &mut metal)
            .unwrap();
        world.provinces[0].plans.insert(owner, BattlePlan::default());
        for index in 0..2 {
            let events = world.advance_recruitment(|_| Some(0));
            let [MilitaryEvent::Recruited {
                unit: recruit,
                ..
            }] = events.as_slice()
            else {
                panic!("exactly one cohort completes each month");
            };
            let battle = &world.battles[0];
            let side = if attacker {
                &battle.attackers
            } else {
                &battle.defenders
            };
            assert_eq!(battle.round, 1, "recruitment cannot advance combat");
            assert_eq!(side.plans[&owner], plan);
            assert_eq!(side.units.len(), index + 2);
            assert!(side.units.iter().any(|unit| unit.id == veteran));
            assert!(side.formation.front.iter().flatten().any(|id| id == recruit));
            assert!(!side.participated.contains(recruit));
            assert_eq!(side.initial_manpower[&owner], (index + 2) as f64 * 10.);
            assert!(!world.provinces[0].forces.contains_key(&owner));
            assert_eq!(world.all_units().filter(|unit| unit.id == *recruit).count(), 1);
        }
        assert!(world.provinces[0].recruitment.is_none());
        assert!(world.provinces[0].recruitment_queue.is_empty());
        world.battles[0].advance_round(&world.config);
        let side = if attacker {
            &world.battles[0].attackers
        } else {
            &world.battles[0].defenders
        };
        assert!(side.units.iter().all(|unit| side.participated.contains(&unit.id)));
    }
}

#[test]
fn enemy_occupation_pauses_recruitment_and_rejects_drafts_without_charging() {
    let mut world = MilitaryWorld::new(1);
    let owner = ForceOwner::Player(0);
    let mut population = [100.; 4];
    let mut metal = 1000.;
    world.config.units[UnitType::LightInfantry as usize].recruitment_months = 1.;
    for _ in 0..2 {
        world
            .recruit(0, 0, UnitType::LightInfantry, true, &[], &mut population, &mut metal)
            .unwrap();
    }
    world.provinces[0].recruitment.as_mut().unwrap().progress = 0.5;
    world.provinces[0].occupation = Some(ForceOwner::Player(1));
    let paid = (population, metal);
    assert_eq!(
        world.recruit(0, 0, UnitType::LightInfantry, true, &[], &mut population, &mut metal),
        Err(MilitaryError::Occupied)
    );
    assert_eq!(world.cancel_recruitment(0, owner), Err(MilitaryError::Occupied));
    assert_eq!(
        world.cancel_queued_recruitment(0, 0, 0, true, &mut population, &mut metal),
        Err(MilitaryError::Occupied)
    );
    assert!(world.advance_recruitment_with_speed(|_| Some(0), |_| 100.).is_empty());
    assert_eq!(world.provinces[0].recruitment.as_ref().unwrap().progress, 0.5);
    assert_eq!(world.provinces[0].recruitment_queue.len(), 1);
    assert_eq!((population, metal), paid);
    world.provinces[0].occupation = None;
    assert_eq!(world.advance_recruitment(|_| Some(0)).len(), 1);
    assert_eq!(world.provinces[0].forces[&owner].len(), 1);
}

#[test]
fn one_population_levy_causes_persistent_and_stacking_draft_fatigue() {
    let mut world = MilitaryWorld::new(1);
    let mut population = [0.0, 200.0, 300.0, 0.0];
    let mut metal = 100.0;
    world.recruit(0, 0, UnitType::LightInfantry, true, &[], &mut population, &mut metal).unwrap();
    assert_eq!(population[2], 290.0);
    let first = world.provinces[0].draft_penalties[2];
    assert!(first >= 5.0);
    world.advance_recruitment(|_| Some(0));
    assert!(world.provinces[0].draft_penalties[2] >= first - 1.0 - 1e-9);
    world.recruit(0, 0, UnitType::LightInfantry, true, &[], &mut population, &mut metal).unwrap();
    assert_eq!(population[2], 280.0);
    assert!(world.provinces[0].draft_penalties[2] > first);
    for _ in 0..12 {
        world.advance_recruitment(|_| Some(0));
    }
    assert_eq!(world.provinces[0].draft_penalties[2], 0.0);
}

#[test]
fn waiting_recruitment_refunds_only_selected_cohort_and_its_original_payment() {
    let mut world = MilitaryWorld::new(1);
    let mut population = [5.0, 100.0, 100.0, 10.0];
    let mut metal = 1000.0;
    for kind in [UnitType::LightInfantry, UnitType::HeavyInfantry, UnitType::Archers] {
        world.recruit(0, 0, kind, true, &[], &mut population, &mut metal).unwrap();
    }
    let removed = world.provinces[0].recruitment_queue[0].clone();
    let paid_population = population;
    let paid_metal = metal;
    world.config.units[removed.unit_type as usize].metal_cost = 999.0;
    world.config.units[removed.unit_type as usize].manpower_class =
        (removed.manpower_class + 1) % 4;
    assert!(world.cancel_queued_recruitment(0, 1, 0, true, &mut population, &mut metal).is_err());
    assert!(world.cancel_queued_recruitment(0, 0, 0, false, &mut population, &mut metal).is_err());
    assert!(world.cancel_queued_recruitment(0, 0, 99, true, &mut population, &mut metal).is_err());
    assert_eq!(population, paid_population);
    assert_eq!(metal, paid_metal);
    assert_eq!(world.provinces[0].recruitment_queue.len(), 2);
    world.cancel_queued_recruitment(0, 0, 0, true, &mut population, &mut metal).unwrap();
    for class in 0..4 {
        assert_eq!(
            population[class],
            paid_population[class]
                + if class == removed.manpower_class {
                    removed.population_cost
                } else {
                    0.0
                }
        );
    }
    assert_eq!(metal, paid_metal + removed.paid_metal);
    assert_eq!(world.provinces[0].recruitment.as_ref().unwrap().unit_type, UnitType::LightInfantry);
    assert_eq!(world.provinces[0].recruitment_queue[0].unit_type, UnitType::Archers);
    let refunded_population = population;
    let refunded_metal = metal;
    world.cancel_recruitment(0, ForceOwner::Player(0)).unwrap();
    assert_eq!(world.provinces[0].recruitment.as_ref().unwrap().unit_type, UnitType::Archers);
    assert!(world.provinces[0].recruitment_queue.is_empty());
    assert_eq!(population, refunded_population);
    assert_eq!(metal, refunded_metal);
    for _ in 0..10 {
        world.advance_recruitment(|_| Some(0));
    }
    assert_eq!(world.all_units().count(), 1);
}

#[test]
fn disband_returns_only_survivors_without_refunding_equipment() {
    let mut world = MilitaryWorld::new(1);
    let id = world.seed_unit(0, ForceOwner::Player(0), UnitType::HeavyInfantry).unwrap();
    world.provinces[0].forces.get_mut(&ForceOwner::Player(0)).unwrap()[0].current_manpower = 3.4;
    let mut pops = [0.; 4];
    assert_eq!(world.disband(0, 0, id, false, &mut pops), Err(MilitaryError::NotDirectlyOwned));
    world.disband(0, 0, id, true, &mut pops).unwrap();
    assert!((pops[2] - 3.4).abs() < 1e-9);
    assert_eq!((pops[0], pops[1], pops[3]), (0., 0., 0.));
    assert_eq!(world.all_units().count(), 0);
}

#[test]
fn supply_restores_manpower_by_rank_and_shortages_reduce_morale() {
    let mut world = MilitaryWorld::new(1);
    let owner = ForceOwner::Player(0);
    world.seed_unit(0, owner, UnitType::HeavyInfantry).unwrap();
    world.provinces[0].forces.get_mut(&owner).unwrap()[0].current_manpower = 4.;
    world.apply_supply(owner, 0.);
    assert_eq!(world.provinces[0].forces[&owner][0].morale, 75.);
    world.apply_supply(owner, 1.);
    assert_eq!(world.provinces[0].forces[&owner][0].people(), 410);
    assert_eq!(world.provinces[0].forces[&owner][0].morale, 76.);
    for _ in 0..100 {
        world.apply_supply(owner, 1.);
    }
    let troop = &world.provinces[0].forces[&owner][0];
    assert_eq!(troop.current_manpower, 10.);
    assert_eq!(troop.morale, 100.);
    assert_eq!(troop.training, 100.);
    assert!((world.food_demand(owner) - 2.5).abs() < 1e-10);
}

#[test]
fn recruitment_ownership_change_cancels_and_npc_defenders_do_not_regenerate() {
    let mut world = MilitaryWorld::new(1);
    let mut pop = [0., 10., 20., 0.];
    let mut metal = 100.;
    world.recruit(0, 0, UnitType::LightInfantry, true, &[], &mut pop, &mut metal).unwrap();
    world.advance_recruitment(|_| Some(1));
    assert!(world.provinces[0].recruitment.is_none());
    assert_eq!(pop[2], 10.);
    assert_eq!(metal, 88.);
    world.seed_local_defenders(0, "Achaia").unwrap();
    assert_eq!(world.all_units().count(), 4);
    world.provinces[0].forces.clear();
    for _ in 0..12 {
        world.advance_recruitment(|_| None);
    }
    assert_eq!(world.all_units().count(), 0);
}

#[test]
fn deterministic_formation_obeys_primary_secondary_and_narrow_flanks() {
    let owner = ForceOwner::Player(0);
    let config = MilitaryConfig::default();
    let mut units: Vec<_> = (0..8)
        .map(|i| {
            unit(
                i,
                owner,
                if i < 3 {
                    UnitType::LightCavalry
                } else {
                    UnitType::HeavyInfantry
                },
            )
        })
        .collect();
    units[4].training = 80.;
    units[5].current_manpower = 2.;
    let plan = BattlePlan {
        flank_size: 2,
        ..Default::default()
    };
    let plans = BTreeMap::from([(owner, plan)]);
    let formation = deploy_formation(&units, &plans, 6, &BTreeSet::new(), &config);
    assert_eq!(formation.flank_size, 2);
    assert_eq!(formation.front[0], Some(0));
    assert_eq!(formation.front[1], Some(1));
    assert_eq!(formation.front[4], Some(2));
    assert!(formation.front.iter().all(Option::is_some));
    units.reverse();
    assert_eq!(formation, deploy_formation(&units, &plans, 6, &BTreeSet::new(), &config));
    let all: Vec<_> = formation
        .front
        .iter()
        .chain(&formation.support)
        .flatten()
        .copied()
        .chain(formation.reserves.iter().copied())
        .collect();
    assert_eq!(all.len(), 8);
    assert_eq!(all.iter().copied().collect::<BTreeSet<_>>().len(), 8);
}

#[test]
fn rear_line_preference_keeps_the_only_matching_cohort_out_of_the_flanks() {
    let owner = ForceOwner::Player(0);
    let config = MilitaryConfig::default();
    let mut units: Vec<_> = (0..16).map(|id| unit(id, owner, UnitType::HeavyInfantry)).collect();
    units.push(unit(16, owner, UnitType::HeavyCavalry));
    let plans = BTreeMap::from([(
        owner,
        BattlePlan {
            secondary_unit_type: UnitType::HeavyCavalry,
            flank_size: 5,
            ..Default::default()
        },
    )]);
    let formation = deploy_formation(&units, &plans, 16, &BTreeSet::new(), &config);
    assert_eq!(formation.front.iter().filter(|id| id.is_some()).count(), 16);
    assert!(!formation.front.contains(&Some(16)));
    assert_eq!(formation.reserves, vec![16]);

    units.reverse();
    assert_eq!(formation, deploy_formation(&units, &plans, 16, &BTreeSet::new(), &config));
    units.retain(|unit| unit.id != 0);
    let shortage = deploy_formation(&units, &plans, 16, &BTreeSet::new(), &config);
    assert!(shortage.front.contains(&Some(16)));
    assert!(shortage.reserves.is_empty());
}

#[test]
fn flanks_fit_sixteen_slots_with_five_on_each_side() {
    let config = MilitaryConfig::default();
    let plan = BattlePlan {
        flank_size: 5,
        ..Default::default()
    };
    for (width, expected) in [(16, 5), (14, 4), (12, 4), (10, 3), (8, 2)] {
        assert_eq!(plan.effective_flank_size(width), expected);
        assert!(width - 2 * expected >= expected);
    }
    assert_eq!(config.combat_widths, [16, 16, 10, 12, 8, 14, 8]);
    for choice in [1, 2, 3, 4, 5] {
        assert_eq!(
            BattlePlan {
                flank_size: choice,
                ..plan
            }
            .normalized_flank_size(),
            choice
        );
    }
    assert_eq!(
        BattlePlan {
            flank_size: 10,
            ..plan
        }
        .normalized_flank_size(),
        5
    );
}

#[test]
fn secondary_center_cohorts_replace_primary_before_unused_primary_reserves() {
    let owner = ForceOwner::Player(0);
    let config = MilitaryConfig::default();
    let mut units = Vec::new();
    units.extend((0..2).map(|id| unit(id, owner, UnitType::LightCavalry)));
    units.extend((2..8).map(|id| unit(id, owner, UnitType::HeavyInfantry)));
    units.extend((8..10).map(|id| unit(id, owner, UnitType::LightInfantry)));
    let plans = BTreeMap::from([(owner, BattlePlan::default())]);
    let mut formation = deploy_formation(&units, &plans, 6, &BTreeSet::new(), &config);
    assert_eq!(formation.front[2], Some(3));
    assert!(formation.reserves.contains(&6));
    assert!(formation.reserves.contains(&8));

    let routed = BTreeSet::from([formation.front[2].unwrap()]);
    refill_formation(&mut formation, &units, &plans, &routed, &config);
    assert_eq!(formation.front[2], Some(8));
    assert!(formation.reserves.contains(&6));
}

#[test]
fn reserves_replace_routed_troops_and_never_return_them() {
    let owner = ForceOwner::Player(0);
    let config = MilitaryConfig::default();
    let units: Vec<_> = (0..10).map(|i| unit(i, owner, UnitType::HeavyInfantry)).collect();
    let plans = BTreeMap::from([(owner, BattlePlan::default())]);
    let mut formation = deploy_formation(&units, &plans, 4, &BTreeSet::new(), &config);
    let lost = formation.front[1].unwrap();
    refill_formation(&mut formation, &units, &plans, &BTreeSet::from([lost]), &config);
    assert!(!formation.front.contains(&Some(lost)));
    assert!(formation.front.iter().all(Option::is_some));
}

#[test]
fn tactic_fit_uses_depleted_actual_units_and_each_tactic_has_two_counters() {
    let config = MilitaryConfig::default();
    let owner = ForceOwner::Player(0);
    let mut units =
        [unit(1, owner, UnitType::HeavyInfantry), unit(2, owner, UnitType::LightCavalry)];
    units[1].current_manpower *= 0.01;
    let fit = tactic_effectiveness(units.iter(), CombatTactic::Envelopment, &config);
    assert!(fit < 0.21);
    for tactic in CombatTactic::ALL {
        let wins =
            CombatTactic::ALL.into_iter().filter(|other| tactic.counters(*other, &config)).count();
        let losses =
            CombatTactic::ALL.into_iter().filter(|other| other.counters(tactic, &config)).count();
        assert_eq!((wins, losses), (2, 2));
        assert!(!tactic.counters(tactic, &config));
        for other in CombatTactic::ALL {
            assert!(!(tactic.counters(other, &config) && other.counters(tactic, &config)));
        }
    }
    assert!(CombatTactic::Bottleneck.counters(CombatTactic::ShockAction, &config));
    assert!(CombatTactic::Phalanx.counters(CombatTactic::ShockAction, &config));
}

#[test]
fn npc_tactics_use_each_owners_surviving_composition_and_keep_player_choices() {
    let config = MilitaryConfig::default();
    let npc = ForceOwner::Local(0);
    let player = ForceOwner::Player(0);
    let mut units: Vec<_> = (0..10)
        .map(|id| {
            let mut cohort = unit(id, npc, UnitType::HeavyInfantry);
            cohort.current_manpower *= 0.01;
            cohort
        })
        .collect();
    units.push(unit(10, npc, UnitType::LightCavalry));
    units.push(unit(11, player, UnitType::HeavyInfantry));
    let plan = BattlePlan {
        tactic: CombatTactic::Deception,
        ..Default::default()
    };
    let side =
        BattleSide::new(units, [(npc, plan), (player, plan)].into(), BTreeMap::new(), 10, &config);
    assert_eq!(side.plans[&npc].tactic, CombatTactic::Envelopment);
    assert_eq!(side.plans[&player], plan);
    let rebels = [unit(12, npc, UnitType::LightInfantry)];
    assert_eq!(best_composition_tactic(rebels.iter(), &config), CombatTactic::Skirmishing);
    assert_eq!(best_composition_tactic([].iter(), &config), CombatTactic::ShockAction);
}

#[test]
fn joining_npcs_choose_composition_fit_once_and_keep_the_tactic_locked() {
    let mut world = MilitaryWorld::new(1);
    let attacker = ForceOwner::Player(0);
    let defender = ForceOwner::Player(1);
    let npc = ForceOwner::Local(0);
    for owner in [attacker, defender] {
        world.seed_unit(0, owner, UnitType::HeavyInfantry).unwrap();
    }
    world
        .start_battle(0, &[attacker], &[defender], None, None, MilitaryTerrain::Plains, 0, 1)
        .unwrap();
    world.seed_unit(0, npc, UnitType::Archers).unwrap();
    world.join_battle(0, npc, true).unwrap();
    assert_eq!(world.battles[0].attackers.plans[&npc].tactic, CombatTactic::Skirmishing);
    for _ in 0..10 {
        world.seed_unit(0, npc, UnitType::LightCavalry).unwrap();
    }
    world.join_battle(0, npc, true).unwrap();
    assert_eq!(world.battles[0].attackers.plans[&npc].tactic, CombatTactic::Skirmishing);
}

#[test]
fn tactics_without_a_counter_have_no_casualty_multiplier() {
    let config = MilitaryConfig {
        random_range: [1., 1.],
        ..Default::default()
    };
    let fight = |tactic| {
        let attacker = ForceOwner::Player(0);
        let defender = ForceOwner::Player(1);
        let mut attackers = side(vec![unit(1, attacker, UnitType::HeavyInfantry)], 4, &config);
        attackers.plans.get_mut(&attacker).unwrap().tactic = tactic;
        let mut defenders = side(vec![unit(2, defender, UnitType::HeavyInfantry)], 4, &config);
        defenders.plans.get_mut(&defender).unwrap().tactic = tactic;
        let mut battle =
            Battle::new(1, 0, None, None, MilitaryTerrain::Plains, 0, attackers, defenders, 1);
        battle.advance_round(&config);
        [
            (battle.attackers.units[0].current_manpower, battle.attackers.units[0].morale),
            (battle.defenders.units[0].current_manpower, battle.defenders.units[0].morale),
        ]
    };
    let shock = fight(CombatTactic::ShockAction);
    let skirmishing = fight(CombatTactic::Skirmishing);
    let phalanx = fight(CombatTactic::Phalanx);
    for side in 0..2 {
        assert_eq!(shock[side], skirmishing[side]);
        assert_eq!(shock[side], phalanx[side]);
    }
}

#[test]
fn movement_uses_slowest_unit_area_roads_and_minimum_one_edge_per_tick() {
    let mut world = MilitaryWorld::new(3);
    let owner = ForceOwner::Player(0);
    let lc = world.seed_unit(0, owner, UnitType::LightCavalry).unwrap();
    let cat = world.seed_unit(0, owner, UnitType::Catapult).unwrap();
    let g = graph();
    assert_eq!(force_speed(&world.provinces[0].forces[&owner], &world.config), 1.5);
    let food = world.food_demand(owner);
    world
        .order_movement(0, 2, owner, &[lc, cat], None, &g, |_, _| MilitaryAccess::Peaceful)
        .unwrap();
    assert_eq!(world.food_demand(owner), food);
    assert_eq!(world.all_units().count(), 2);
    world.advance_movement(&g, |_, _| MilitaryAccess::Peaceful);
    assert_eq!(world.movements[0].origin, 0);
    world.advance_movement(&g, |_, _| MilitaryAccess::Peaceful);
    assert_eq!(world.movements[0].origin, 1);
    assert!(world.provinces[2].forces.is_empty());
    for _ in 0..2 {
        world.advance_movement(&g, |_, _| MilitaryAccess::Peaceful);
    }
    assert!(world.movements.is_empty());
    assert_eq!(world.provinces[2].forces[&owner].len(), 2);
    let mut large = g[0].clone();
    large.area = 400.;
    assert!(edge_travel_months(&large, &g[1], 100., 2.5, &world.config) > 1.);
    large.road_level = 10;
    assert!(edge_travel_months(&large, &g[1], 100., 2.5, &world.config) < 1.6);
}

#[test]
fn terrain_changes_combat_width_without_attack_or_defense_modifiers() {
    let config = MilitaryConfig::default();
    let terrains = [
        MilitaryTerrain::Farmland,
        MilitaryTerrain::Plains,
        MilitaryTerrain::Forest,
        MilitaryTerrain::Hills,
        MilitaryTerrain::Mountains,
        MilitaryTerrain::Desert,
        MilitaryTerrain::Marsh,
    ];
    for kind in UnitType::ALL {
        let fight = |terrain| {
            let mut battle = Battle::new(
                1,
                0,
                None,
                None,
                terrain,
                0,
                side(vec![unit(1, ForceOwner::Player(0), kind)], 8, &config),
                side(vec![unit(2, ForceOwner::Player(1), UnitType::HeavyInfantry)], 8, &config),
                71,
            );
            battle.advance_round(&config);
            (battle.attackers.units, battle.defenders.units, battle.rounds[0].casualties)
        };
        let baseline = fight(MilitaryTerrain::Plains);
        for terrain in terrains {
            assert_eq!(
                fight(terrain),
                baseline,
                "{kind:?} received a hidden modifier in {terrain:?}"
            );
        }
    }
    let attackers = vec![unit(1, ForceOwner::Player(0), UnitType::HeavyInfantry)];
    let defenders = vec![unit(2, ForceOwner::Player(1), UnitType::HeavyInfantry)];
    for terrain in terrains {
        let estimate = estimate_battle(
            &attackers,
            &defenders,
            BattlePlan::default(),
            terrain,
            0,
            MilitaryRank::Centurion,
            MilitaryRank::Centurion,
            &config,
        );
        assert_eq!(estimate.formation.front.len(), config.combat_widths[terrain as usize]);
        assert_eq!(estimate.assessment, "Even");
    }
}

#[test]
fn revoked_access_stops_movement_and_peaceful_presence_never_occupies() {
    let mut world = MilitaryWorld::new(3);
    let owner = ForceOwner::Player(0);
    let id = world.seed_unit(0, owner, UnitType::LightCavalry).unwrap();
    world
        .order_movement(0, 2, owner, &[id], None, &graph(), |_, _| MilitaryAccess::Peaceful)
        .unwrap();
    world.advance_movement(&graph(), |_, p| {
        if p == 2 {
            MilitaryAccess::Blocked
        } else {
            MilitaryAccess::Peaceful
        }
    });
    assert!(world.movements.is_empty());
    assert_eq!(world.provinces[1].forces[&owner].len(), 1);
    assert_eq!(world.occupation_control(1, owner), 0.);
}

#[test]
fn simultaneous_identical_combat_has_no_first_strike_advantage() {
    let config = MilitaryConfig {
        random_range: [1., 1.],
        base_manpower_damage: 2.,
        ..Default::default()
    };
    let a = side(vec![unit(1, ForceOwner::Player(0), UnitType::HeavyInfantry)], 4, &config);
    let d = side(vec![unit(2, ForceOwner::Player(1), UnitType::HeavyInfantry)], 4, &config);
    let mut battle = Battle::new(1, 1, Some(1), Some(0), MilitaryTerrain::Plains, 0, a, d, 1);
    battle.advance_round(&config);
    assert_eq!(battle.result, Some(BattleResult::MutualRout));
    assert!(battle.attackers.units.is_empty());
    assert!(battle.defenders.units.is_empty());
}

#[test]
fn morale_scales_attack_once_and_defense_resists_both_losses() {
    let losses = |morale, defense| {
        let mut config = MilitaryConfig {
            random_range: [1., 1.],
            ..Default::default()
        };
        config.units[UnitType::HeavyInfantry as usize].defense = defense;
        let mut attacker = unit(1, ForceOwner::Player(0), UnitType::LightInfantry);
        attacker.morale = morale;
        let victim = unit(2, ForceOwner::Player(1), UnitType::HeavyInfantry);
        let initial_manpower = victim.current_manpower;
        let initial_morale = victim.morale;
        let mut battle = Battle::new(
            1,
            0,
            None,
            None,
            MilitaryTerrain::Plains,
            0,
            side(vec![attacker], 4, &config),
            side(vec![victim], 4, &config),
            1,
        );
        battle.advance_round(&config);
        let survivor = &battle.defenders.units[0];
        (initial_manpower - survivor.current_manpower, initial_morale - survivor.morale)
    };
    let normal = losses(50., 1.);
    let confident = losses(100., 1.);
    let armored = losses(50., 2.);
    assert!(normal.0 > 0. && normal.1 > 0.);
    assert!((confident.0 - normal.0 * 1.25).abs() <= 0.02);
    assert!(
        (confident.1 / normal.1 - 1.25).abs() < 1e-10,
        "Morale must not be applied a second time to morale loss"
    );
    assert!((armored.0 - normal.0 * 0.5).abs() <= 0.02);
    assert!((armored.1 / normal.1 - 0.5).abs() < 1e-10);
}

#[test]
fn seeded_battles_repeat_and_respect_deadline_and_retreat_lock() {
    let config = MilitaryConfig::default();
    let a = side(vec![unit(1, ForceOwner::Player(0), UnitType::HeavyInfantry)], 4, &config);
    let d = side(vec![unit(2, ForceOwner::Player(1), UnitType::HeavyInfantry)], 4, &config);
    let mut battle = Battle::new(1, 1, Some(1), Some(0), MilitaryTerrain::Mountains, 2, a, d, 42);
    let mut replay = battle.clone();
    assert_eq!(battle.request_retreat(true, &config), Err(MilitaryError::RetreatTooEarly));
    for _ in 0..config.maximum_battle_months {
        battle.advance_month(&config);
        replay.advance_month(&config);
    }
    assert!(battle.result.is_some());
    assert_eq!(battle.result, replay.result);
    assert_eq!(battle.attackers.units, replay.attackers.units);
    assert_eq!(battle.defenders.units, replay.defenders.units);
}

#[test]
fn conquest_event_keeps_previous_owner_and_returns_survivors() {
    let mut world = MilitaryWorld::new(3);
    world.config.random_range = [1., 1.];
    let a = ForceOwner::Player(0);
    let d = ForceOwner::Player(1);
    for _ in 0..8 {
        world.seed_unit(1, a, UnitType::HeavyInfantry).unwrap();
    }
    world.seed_unit(1, d, UnitType::LightInfantry).unwrap();
    let before = world.food_demand(a);
    world.start_battle(1, &[a], &[d], Some(1), Some(0), MilitaryTerrain::Plains, 0, 1).unwrap();
    assert_eq!(world.food_demand(a), before);
    let mut events = vec![];
    for _ in 0..6 {
        events.extend(world.advance_battles(&graph(), |owner, p| {
            if (owner == a && p == 0) || (owner == d && p == 2) {
                MilitaryAccess::Peaceful
            } else {
                MilitaryAccess::Blocked
            }
        }));
    }
    assert!(events.iter().any(|e| matches!(
        e,
        MilitaryEvent::BattleEnded {
            previous_owner: Some(1),
            winner: Some(ForceOwner::Player(0)),
            ..
        }
    )));
    assert!(world.occupation_control(1, a) > 0.);
    assert!(!world.provinces[1].forces[&a].is_empty());
}

#[test]
fn every_roster_entry_has_complete_finite_configuration() {
    let c = MilitaryConfig::default();
    for kind in UnitType::ALL {
        let d = c.unit(kind);
        assert_eq!(d.cohort_people(), 1_000);
        assert!(
            d.manpower > 0.
                && d.metal_cost > 0.
                && d.food_per_month > 0.
                && d.coin_per_month > 0.
                && d.defense > 0.
                && d.movement_speed > 0.
        );
        assert!([1, 2].contains(&d.manpower_class));
        assert!(c.matchups[kind as usize].iter().all(|n| n.is_finite() && *n > 0.));
        assert!(c.tactic_fit[kind as usize].iter().all(|n| (0.0..=1.0).contains(n)));
    }
    assert!(c.matchups[UnitType::HeavyInfantry as usize][UnitType::LightCavalry as usize] > 1.);
    assert!(c.matchups[UnitType::LightCavalry as usize][UnitType::HeavyInfantry as usize] < 1.);
    assert_eq!(MilitaryRank::from_renown(900., &c), MilitaryRank::Imperator);
}

#[test]
fn undersized_forces_engage_despite_different_flank_preferences() {
    let config = MilitaryConfig::default();
    let a = ForceOwner::Player(0);
    let d = ForceOwner::Player(1);
    let plans_a = BTreeMap::from([(
        a,
        BattlePlan {
            flank_size: 1,
            ..Default::default()
        },
    )]);
    let plans_d = BTreeMap::from([(
        d,
        BattlePlan {
            flank_size: 5,
            ..Default::default()
        },
    )]);
    let attackers = BattleSide::new(
        vec![unit(1, a, UnitType::HeavyInfantry)],
        plans_a,
        BTreeMap::new(),
        8,
        &config,
    );
    let defenders = BattleSide::new(
        vec![unit(2, d, UnitType::HeavyInfantry)],
        plans_d,
        BTreeMap::new(),
        8,
        &config,
    );
    let mut battle =
        Battle::new(1, 0, None, None, MilitaryTerrain::Plains, 0, attackers, defenders, 1);
    battle.advance_round(&config);
    assert!(battle.attackers.units[0].current_manpower < 10.);
    assert!(battle.defenders.units[0].current_manpower < 10.);
}

#[test]
fn defeating_a_rival_does_not_occupy_neutral_local_defenders() {
    let mut world = MilitaryWorld::new(3);
    let a = ForceOwner::Player(0);
    let d = ForceOwner::Player(1);
    for _ in 0..8 {
        world.seed_unit(1, a, UnitType::HeavyInfantry).unwrap();
    }
    world.seed_unit(1, d, UnitType::LightInfantry).unwrap();
    world.seed_unit(1, ForceOwner::Local(1), UnitType::LightInfantry).unwrap();
    world.start_battle(1, &[a], &[d], None, Some(0), MilitaryTerrain::Plains, 0, 1).unwrap();
    for _ in 0..6 {
        world.advance_battles(&graph(), |_, _| MilitaryAccess::Peaceful);
    }
    assert!(world
        .history
        .last()
        .is_some_and(|outcome| outcome.result == BattleResult::AttackerVictory));
    assert!(world.provinces[1].occupation.is_none());
    assert!(!world.provinces[1].forces[&ForceOwner::Local(1)].is_empty());
}

#[test]
fn waypoint_routes_preserve_player_choice_and_validate_before_removing_troops() {
    let mut world = MilitaryWorld::new(4);
    let owner = ForceOwner::Player(0);
    let id = world.seed_unit(0, owner, UnitType::HeavyInfantry).unwrap();
    let graph: Vec<_> = [vec![1, 2], vec![0, 3], vec![0, 3], vec![1, 2]]
        .into_iter()
        .map(|neighbors| MilitaryProvince {
            terrain: MilitaryTerrain::Plains,
            area: 100.,
            road_level: 0,
            neighbors,
        })
        .collect();
    let troops = &world.provinces[0].forces[&owner];
    let route = route_via(
        &graph,
        0,
        3,
        &[2],
        owner,
        troops,
        |_, _| MilitaryAccess::Peaceful,
        &world.config,
    )
    .unwrap();
    assert_eq!(route, vec![2, 3]);
    assert_eq!(
        world.order_movement_route(0, owner, &[id], None, &[3], &graph, |_, _| {
            MilitaryAccess::Peaceful
        }),
        Err(MilitaryError::NoLegalRoute)
    );
    assert_eq!(world.provinces[0].forces[&owner].len(), 1);
    assert_eq!(
        world.order_movement_route(0, owner, &[id], None, &route, &graph, |_, p| if p == 2 {
            MilitaryAccess::Invasion
        } else {
            MilitaryAccess::Peaceful
        }),
        Err(MilitaryError::NoLegalRoute)
    );
    world
        .order_movement_route(0, owner, &[id], None, &route, &graph, |_, _| {
            MilitaryAccess::Peaceful
        })
        .unwrap();
    assert_eq!(world.movements[0].route, vec![2, 3]);
}

#[test]
fn support_forced_into_the_front_line_takes_exposure_casualties() {
    let loss = |exposure| {
        let config = MilitaryConfig {
            random_range: [1., 1.],
            exposed_support_casualties: exposure,
            ..Default::default()
        };
        let attackers =
            side(vec![unit(1, ForceOwner::Player(0), UnitType::LightInfantry)], 4, &config);
        let defenders = side(vec![unit(2, ForceOwner::Player(1), UnitType::Ballista)], 4, &config);
        let mut battle =
            Battle::new(1, 0, None, None, MilitaryTerrain::Plains, 0, attackers, defenders, 1);
        battle.advance_round(&config);
        10. - battle.defenders.units[0].current_manpower
    };
    assert!((loss(1.5) - loss(1.) * 1.5).abs() <= 0.02);
}

#[test]
fn trapped_armies_ignore_timeout_and_fight_to_destruction() {
    let mut world = MilitaryWorld::new(3);
    let attacker = ForceOwner::Player(0);
    let defender = ForceOwner::Player(1);
    world.seed_unit(1, attacker, UnitType::HeavyInfantry).unwrap();
    world.seed_unit(1, defender, UnitType::HeavyInfantry).unwrap();
    world.config.maximum_battle_months = 1;
    world.config.base_manpower_damage = 0.5;
    world.config.base_morale_damage = 100.;
    world
        .start_battle(1, &[attacker], &[defender], Some(1), Some(0), MilitaryTerrain::Plains, 0, 1)
        .unwrap();
    let mut events = vec![];
    for _ in 0..10 {
        events.extend(world.advance_battles(&graph(), |_, _| MilitaryAccess::Blocked));
        if world.battles.is_empty() {
            break;
        }
    }
    let losses: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            MilitaryEvent::UnitsDestroyed {
                owner,
                count,
                ..
            } => Some((*owner, *count)),
            _ => None,
        })
        .collect();
    assert!(world.battles.is_empty());
    assert!(losses.iter().map(|(_, count)| count).sum::<usize>() >= 1);
    assert!(world.advance_battles(&graph(), |_, _| MilitaryAccess::Blocked).is_empty());
}

#[test]
fn recruitment_queue_cancel_and_capture_never_refund_or_duplicate_cohorts() {
    let mut world = MilitaryWorld::new(1);
    let mut population = [0., 0., 40., 0.];
    let mut metal = 100.;
    for _ in 0..3 {
        world
            .recruit(0, 0, UnitType::LightInfantry, true, &[], &mut population, &mut metal)
            .unwrap();
    }
    assert_eq!(population[2], 10.);
    assert_eq!(metal, 64.);
    world.cancel_recruitment(0, ForceOwner::Player(0)).unwrap();
    assert_eq!(world.provinces[0].recruitment_queue.len(), 1);
    world.advance_recruitment_with_speed(|_| Some(0), |_| 0.);
    assert_eq!(world.provinces[0].recruitment.as_ref().unwrap().progress, 0.);
    world.advance_recruitment(|_| Some(1));
    assert!(world.provinces[0].recruitment.is_none());
    assert!(world.provinces[0].recruitment_queue.is_empty());
    assert_eq!(world.all_units().count(), 0);
    assert_eq!(population[2], 10.);
    assert_eq!(metal, 64.);
}
