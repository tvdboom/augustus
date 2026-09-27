//! Invariant and balance regression coverage for irreversible military changes.

use super::*;
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
    assert_eq!(
        world.recruit(0, 0, UnitType::LightInfantry, true, &[], &mut pops, &mut metal),
        Err(MilitaryError::RecruitmentBusy)
    );
    for _ in 0..3 {
        world.advance_recruitment(|_| Some(0));
    }
    assert_eq!(world.all_units().count(), 1);
    assert_eq!(metal, 60.);
    assert!(world.provinces[0].draft_penalties[2] > 0.);
}

#[test]
fn disband_returns_only_survivors_without_refunding_equipment() {
    let mut world = MilitaryWorld::new(1);
    let id = world.seed_unit(0, ForceOwner::Player(0), UnitType::HeavyInfantry).unwrap();
    world.provinces[0].forces.get_mut(&ForceOwner::Player(0)).unwrap()[0].current_manpower = 3.4;
    let mut pops = [0.; 4];
    assert_eq!(world.disband(0, 0, id, false, &mut pops), Err(MilitaryError::NotDirectlyOwned));
    world.disband(0, 0, id, true, &mut pops).unwrap();
    assert_eq!(pops, [0., 0., 3.4, 0.]);
    assert_eq!(world.all_units().count(), 0);
}

#[test]
fn supply_never_heals_manpower_and_shortages_reduce_morale() {
    let mut world = MilitaryWorld::new(1);
    let owner = ForceOwner::Player(0);
    world.seed_unit(0, owner, UnitType::HeavyInfantry).unwrap();
    world.provinces[0].forces.get_mut(&owner).unwrap()[0].current_manpower = 4.;
    world.apply_supply(owner, 0.);
    assert_eq!(world.provinces[0].forces[&owner][0].morale, 25.);
    for _ in 0..100 {
        world.apply_supply(owner, 1.);
    }
    let troop = &world.provinces[0].forces[&owner][0];
    assert_eq!(troop.current_manpower, 4.);
    assert_eq!(troop.training, 100.);
    assert!((world.food_demand(owner) - 3.2).abs() < 1e-10);
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
        flank_size: 3,
        ..Default::default()
    };
    let plans = BTreeMap::from([(owner, plan)]);
    let formation = deploy_formation(&units, &plans, 4, &BTreeSet::new(), &config);
    assert_eq!(formation.flank_size, 1);
    assert_eq!(formation.front, vec![Some(0), Some(4), Some(3), Some(1)]);
    units.reverse();
    assert_eq!(formation, deploy_formation(&units, &plans, 4, &BTreeSet::new(), &config));
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
fn tactic_fit_uses_depleted_actual_units_and_balanced_has_no_counter() {
    let config = MilitaryConfig::default();
    let owner = ForceOwner::Player(0);
    let mut units =
        vec![unit(1, owner, UnitType::HeavyInfantry), unit(2, owner, UnitType::LightCavalry)];
    units[1].current_manpower *= 0.01;
    let fit = tactic_effectiveness(units.iter(), CombatTactic::Envelopment, &config);
    assert!(fit < 0.21);
    for tactic in CombatTactic::ALL {
        assert!(!tactic.counters(CombatTactic::Balanced, &config));
        assert!(!CombatTactic::Balanced.counters(tactic, &config));
    }
    assert!(CombatTactic::Bottleneck.counters(CombatTactic::ShockAction, &config));
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
    let mut config = MilitaryConfig::default();
    config.random_range = [1., 1.];
    config.base_manpower_damage = 2.;
    let a = side(vec![unit(1, ForceOwner::Player(0), UnitType::HeavyInfantry)], 4, &config);
    let d = side(vec![unit(2, ForceOwner::Player(1), UnitType::HeavyInfantry)], 4, &config);
    let mut battle = Battle::new(1, 1, Some(1), Some(0), MilitaryTerrain::Plains, 0, a, d, 1);
    battle.advance_round(&config);
    assert_eq!(battle.result, Some(BattleResult::MutualRout));
    assert!(battle.attackers.units.is_empty());
    assert!(battle.defenders.units.is_empty());
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
    assert_eq!(
        world.occupation_control(1, a),
        0.,
        "direct conquest must not create independent Control"
    );
    assert!(!world.provinces[1].forces[&a].is_empty());
}

#[test]
fn every_roster_entry_has_complete_finite_configuration() {
    let c = MilitaryConfig::default();
    for kind in UnitType::ALL {
        let d = c.unit(kind);
        assert!(
            d.manpower > 0.
                && d.metal_cost > 0.
                && d.food_per_month > 0.
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
            flank_size: 3,
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
        let mut config = MilitaryConfig::default();
        config.random_range = [1., 1.];
        config.exposed_support_casualties = exposure;
        let attackers =
            side(vec![unit(1, ForceOwner::Player(0), UnitType::LightInfantry)], 4, &config);
        let defenders = side(vec![unit(2, ForceOwner::Player(1), UnitType::Ballista)], 4, &config);
        let mut battle =
            Battle::new(1, 0, None, None, MilitaryTerrain::Plains, 0, attackers, defenders, 1);
        battle.advance_round(&config);
        3. - battle.defenders.units[0].current_manpower
    };
    assert!((loss(1.5) / loss(1.) - 1.5).abs() < 1e-10);
}

#[test]
fn loss_events_include_trapped_survivors_without_double_counting() {
    let mut world = MilitaryWorld::new(3);
    let attacker = ForceOwner::Player(0);
    let defender = ForceOwner::Player(1);
    world.seed_unit(1, attacker, UnitType::HeavyInfantry).unwrap();
    world.seed_unit(1, defender, UnitType::HeavyInfantry).unwrap();
    world.config.maximum_battle_months = 1;
    world.config.base_manpower_damage = 0.;
    world.config.base_morale_damage = 0.;
    world
        .start_battle(1, &[attacker], &[defender], Some(1), Some(0), MilitaryTerrain::Plains, 0, 1)
        .unwrap();
    let events = world.advance_battles(&graph(), |_, _| MilitaryAccess::Blocked);
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
    assert_eq!(
        losses,
        vec![(attacker, 1)],
        "timeout survivors with no retreat are real destroyed cohorts"
    );
    assert_eq!(world.all_units().filter(|unit| unit.owner == attacker).count(), 0);
    assert_eq!(world.all_units().filter(|unit| unit.owner == defender).count(), 1);
    assert!(world.advance_battles(&graph(), |_, _| MilitaryAccess::Blocked).is_empty());
}
