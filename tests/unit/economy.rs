//! Behavioral checks focus on conservation, failure atomicity, and monthly ordering.

use super::*;

/// A small deterministic map is sufficient to test the shared graph/economy contracts.
fn world(provinces: usize, players: usize) -> EconomyWorld {
    let states = (0..provinces)
        .map(|id| {
            EconomicProvince::new(
                format!("Province {id}"),
                100.0,
                Terrain::Plains,
                false,
                [2.0, 1.0, 4.0],
                [5.0, 10.0, 20.0, 15.0],
                players,
            )
        })
        .collect();
    let adjacency = (0..provinces)
        .map(|id| {
            let mut neighbors = Vec::new();
            if id > 0 {
                neighbors.push(id - 1);
            }
            if id + 1 < provinces {
                neighbors.push(id + 1);
            }
            neighbors
        })
        .collect();
    let mut world = EconomyWorld::new(players, states, adjacency);
    world.provinces[0].owner = Some(0);
    world
}

/// Exact conservation comparisons tolerate only ordinary floating-point rounding.
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}

#[test]
fn happiness_report_and_player_hud_average_use_actual_class_changes() {
    let mut world = world(3, 2);
    world.provinces[1].owner = Some(0);
    world.provinces[2].owner = Some(1);
    world.provinces[0].population = [2.0; 4];
    world.provinces[1].population = [6.0; 4];
    world.provinces[2].population = [100.0; 4];
    world.provinces[0].policies.food = FoodPolicy::High;
    world.provinces[1].policies.food = FoodPolicy::Low;
    world.config.birth_rates = [0.0; 4];
    world.config.death_rates = [0.0; 4];
    world.config.class_change_rates = [0.0; 3];
    world.config.migration_rates = [0.0; 4];
    world.players[0].resources[0] = 1000.0;
    let report = world.advance_month(&MonthlyInputs::default());
    assert_eq!(report.province_reports[0].happiness_delta, [10.0; 4]);
    assert_eq!(report.province_reports[1].happiness_delta, [-10.0; 4]);
    for class in 0..4 {
        let (happiness, delta) = world.player_happiness(0, class);
        close(happiness, 45.0);
        close(delta, -5.0);
    }
    assert_eq!(world.player_happiness(2, 0), (50.0, 0.0));
}

#[test]
fn immediate_quote_and_acceptance_share_bilateral_affordability() {
    let mut world = world(2, 2);
    world.provinces[1].owner = Some(1);
    let trade = TradeAgreement::new(
        TradeParty::Player(0),
        TradeParty::Player(1),
        TradeBundle {
            coin: 20.0,
            ..Default::default()
        },
        TradeBundle {
            resources: [0.0, 10.0, 0.0],
            ..Default::default()
        },
        TradeFrequency::OneTime,
    );
    let inputs = MonthlyInputs::default();
    world.players[1].resources[1] = 0.0;
    assert!(world.quote_trade(&trade, &inputs).is_ok());
    assert!(world.quote_trade_execution(&trade, &inputs).is_err());
    let (id, _) = world.propose_trade(trade, &inputs).unwrap();
    let before = world.players[0].coin;
    assert!(world.accept_trade(id, 1, &inputs).is_err());
    assert_eq!(world.players[0].coin, before);
    assert_eq!(world.trades[0].status, TradeStatus::Proposed);
    world.players[1].resources[1] = 10.0;
    assert!(world.quote_trade_execution(&world.trades[0], &inputs).is_ok());
    world.accept_trade(id, 1, &inputs).unwrap();
    assert_eq!(world.players[0].coin, before - 20.0);
    assert_eq!(world.trades[0].status, TradeStatus::Completed);
}

#[test]
fn workers_are_allocated_once_and_absent_resources_receive_no_labor() {
    let mut world = world(1, 1);
    let p = &mut world.provinces[0];
    p.potential = [1.0, 0.0, 1.0];
    let (labor, output) = p.production(&world.config);
    close(labor.iter().sum(), 20.0 + 15.0 * 1.5);
    assert_eq!(labor[1], 0.0);
    assert_eq!(output[1], 0.0);
    p.policies.focus = ResourceFocus::Food;
    let (focused, _) = p.production(&world.config);
    close(focused[0] / focused[2], 3.0);
}

#[test]
fn happiness_suppresses_births_without_directly_increasing_natural_deaths() {
    close(birth_modifier(0.0), 0.0);
    close(birth_modifier(25.0), 0.5);
    close(birth_modifier(100.0), 1.5);
    let world = world(1, 1);
    let mut neutral = world.provinces[0].clone();
    let mut unhappy = neutral.clone();
    unhappy.happiness_modifiers = [-50.0; 4];
    let mut a = ProvinceMonth {
        food_supply_ratio: 1.0,
        ..Default::default()
    };
    let mut b = a.clone();
    neutral.advance_demographics(&mut a, &world.config);
    unhappy.advance_demographics(&mut b, &world.config);
    assert_eq!(a.normal_deaths, b.normal_deaths);
    assert!(a.births.iter().all(|births| *births > 0.0));
    assert_eq!(b.births, [0.0; 4]);
}

#[test]
fn proportional_food_supply_includes_armies_and_all_owned_provinces() {
    let mut world = world(2, 1);
    for province in &mut world.provinces {
        province.owner = Some(0);
        province.potential = [0.0; 3];
    }
    world.players[0].resources[0] = 60.0;
    let report = world.advance_month(&MonthlyInputs {
        army_food: vec![20.0],
        ..Default::default()
    });
    close(report.food_supply_ratio[0], 0.5); // 50+50 civilian and 20 military.
    for province in report.province_reports {
        close(province.food_supply_ratio, 0.5);
        close(province.famine_deaths.iter().sum(), 50.0 * 0.5 * world.config.max_famine_death_rate);
    }
    close(world.players[0].resources[0], 0.0);
}

#[test]
fn happiness_buildings_cannot_override_soft_population_equilibrium() {
    let world = world(1, 1);
    let mut config = world.config.clone();
    config.class_change_rates = [0.0; 3];
    let mut province = world.provinces[0].clone();
    province.has_city = true;
    province.buildings[BuildingType::Temple as usize] = 50;
    province.policies.food = FoodPolicy::High;
    province.policies.slave_labor = SlaveLabor::Light;
    let capacity = province.capacity(&config);
    province.population = [capacity * 1.5 / 4.0; 4];
    close(province.crowding_birth_modifier(&config), 1.0);
    province.population = [capacity * 3.0 / 4.0; 4];
    close(province.crowding_birth_modifier(&config), 0.5);
    province.population = [capacity * 8.0 / 4.0; 4];
    let mut month = ProvinceMonth {
        food_supply_ratio: 1.0,
        ..Default::default()
    };
    province.advance_demographics(&mut month, &config);
    assert_eq!(province.happiness, [100.0; 4]);
    assert_eq!(month.famine_deaths, [0.0; 4]);
    assert!(month.births.iter().zip(month.normal_deaths).all(|(births, deaths)| *births < deaths));
    // Decline is gradual, never a hard clamp to carrying capacity.
    assert!(province.total_population() > capacity * 7.9);
    for _ in 0..6000 {
        province.advance_demographics(&mut month, &config);
    }
    assert!(province.total_population().is_finite());
    assert!(province.total_population() > capacity * 1.5);
    assert!(province.total_population() < capacity * 6.0);
}

#[test]
fn closed_migration_still_conserves_population_and_slaves_never_migrate() {
    let mut world = world(2, 1);
    world.provinces[1].owner = Some(0);
    world.provinces[0].capacity_area = 0.1;
    world.provinces[0].policies.migration = MigrationPolicy::Closed;
    world.config.birth_rates = [0.0; 4];
    world.config.death_rates = [0.0; 4];
    world.config.class_change_rates = [0.0; 3];
    let before: f64 = world.provinces.iter().map(EconomicProvince::total_population).sum();
    let report = world.advance_month(&MonthlyInputs::default());
    let after: f64 = world.provinces.iter().map(EconomicProvince::total_population).sum();
    close(before, after);
    assert!(report.province_reports[0].migration[2] < 0.0);
    assert_eq!(report.province_reports[0].migration[3], 0.0);
    for class in 0..4 {
        close(report.province_reports.iter().map(|p| p.migration[class]).sum(), 0.0);
    }
}

#[test]
fn migrants_do_not_disappear_when_every_destination_is_hostile() {
    let mut world = world(2, 2);
    world.provinces[1].owner = Some(1);
    world.provinces[0].happiness = [0.0; 4];
    let mut reports = vec![ProvinceMonth::default(); 2];
    let before = world.provinces[0].population;
    world.migrate(
        &MonthlyInputs {
            player_hostility: vec![vec![false, true], vec![true, false]],
            ..Default::default()
        },
        &mut reports,
    );
    assert_eq!(world.provinces[0].population, before);
    assert_eq!(reports[0].migration, [0.0; 4]);
}

#[test]
fn class_changes_preserve_population_without_cascading_same_month() {
    let world = world(1, 1);
    let mut config = world.config.clone();
    config.birth_rates = [0.0; 4];
    config.death_rates = [0.0; 4];
    config.class_change_rates = [1.0; 3];
    let mut p = world.provinces[0].clone();
    p.has_city = true;
    p.population = [0.0, 0.0, 0.0, 10.0];
    p.advance_demographics(
        &mut ProvinceMonth {
            food_supply_ratio: 1.0,
            ..Default::default()
        },
        &config,
    );
    assert_eq!(p.population, [0.0, 0.0, 10.0, 0.0]);
}

#[test]
fn construction_checks_are_atomic_and_share_one_slot_with_wonders() {
    let mut world = world(1, 1);
    let before = world.players[0].balances();
    assert!(world.start_building(0, 0, BuildingType::Forum).is_err());
    assert_eq!(world.players[0].balances(), before);
    world.start_building(0, 0, BuildingType::Granary).unwrap();
    let paid = world.players[0].balances();
    assert!(world.start_building(0, 0, BuildingType::Mine).is_err());
    assert_eq!(world.players[0].balances(), paid);
    world.cancel_construction(0, 0).unwrap();
    assert_eq!(world.players[0].balances(), paid);
    let definition = BuildingDefinition::for_type(BuildingType::Granary);
    close(definition.quote(3).stone, 410.0);
    assert!(definition.quote(30).stone > definition.quote(29).stone);
}

#[test]
fn construction_completion_expands_global_storage_and_loss_removes_capacity() {
    let mut world = world(2, 1);
    world.start_building(0, 0, BuildingType::Granary).unwrap();
    for _ in 0..3 {
        world.advance_month(&MonthlyInputs::default());
    }
    assert_eq!(world.provinces[0].level(BuildingType::Granary), 1);
    assert_eq!(world.players[0].storage[0], world.config.base_storage[0] + 600.0);
    world.provinces[0].change_owner(None, None);
    world.players[0].resources[0] = world.config.base_storage[0] + 500.0;
    world.advance_month(&MonthlyInputs::default());
    close(world.players[0].resources[0], world.config.base_storage[0]);
}

#[test]
fn wonder_assigned_slaves_remain_fed_but_do_not_produce_and_reward_only_once() {
    let mut world = world(1, 1);
    world.provinces[0].wonder_sites = vec![0];
    world.players[0].resources = [2000.0, 1000.0, 3000.0];
    world.config.wonders[0].required_progress = 1.0;
    world.config.influence_per_noble = 0.0;
    let baseline = world.provinces[0].production(&world.config).1;
    world.start_wonder(0, 0, 0).unwrap();
    world.assign_wonder_slaves(0, 0, 15.0).unwrap();
    let first = world.advance_month(&MonthlyInputs::default());
    close(first.province_reports[0].food_requested, 50.0);
    assert!(first.province_reports[0].production[0] < baseline[0]);
    assert_eq!(world.provinces[0].completed_wonder, Some(0));
    close(world.players[0].influence, 40.0 + 250.0 + 3.0);
    world.advance_month(&MonthlyInputs::default());
    close(world.players[0].influence, 40.0 + 250.0 + 6.0);
    assert!(world.start_wonder(0, 0, 0).is_err());
}

#[test]
fn ownership_changes_preserve_project_progress_but_reset_wonder_labor() {
    let mut world = world(1, 2);
    world.provinces[0].construction = Some(ConstructionProject::Wonder(WonderProject {
        wonder_id: 0,
        progress: 9.0,
        required_progress: 36.0,
        assigned_slaves: 15.0,
    }));
    world.provinces[0].change_owner(Some(1), None);
    let Some(ConstructionProject::Wonder(project)) = &world.provinces[0].construction else {
        panic!("Lost project")
    };
    close(project.progress, 9.0);
    close(project.assigned_slaves, 0.0);
    world.provinces[0].completed_wonder = Some(1);
    world.provinces[0].construction = None;
    world.config.influence_per_noble = 0.0;
    world.advance_month(&MonthlyInputs::default());
    close(world.players[0].influence, 40.0);
    close(world.players[1].influence, 43.0);
}

#[test]
fn routes_use_sea_edges_and_reject_hostile_transit() {
    let mut world = world(4, 2);
    world.provinces[3].owner = Some(1);
    let path = world
        .trade_route(TradeParty::Player(0), TradeParty::Player(1), &MonthlyInputs::default())
        .unwrap();
    assert_eq!(path, vec![0, 1, 2, 3]);
    close(world.route_efficiency(&path), 0.9);
    world.provinces[2].relation_by_player[0] = 0.0;
    assert!(world
        .trade_route(TradeParty::Player(0), TradeParty::Player(1), &MonthlyInputs::default())
        .is_err());
    world.adjacency[0].push(3);
    world.adjacency[3].push(0); // Authorized sea crossing.
    assert_eq!(
        world
            .trade_route(TradeParty::Player(0), TradeParty::Player(1), &MonthlyInputs::default())
            .unwrap(),
        vec![0, 3]
    );
}

#[test]
fn bilateral_trade_requires_consent_and_transfers_after_distance_loss_atomically() {
    let mut world = world(3, 2);
    world.provinces[2].owner = Some(1);
    let a = TradeBundle {
        resources: [100.0, 0.0, 0.0],
        ..Default::default()
    };
    let b = TradeBundle {
        resources: [0.0, 20.0, 0.0],
        ..Default::default()
    };
    let before_a = world.players[0].resources;
    let before_b = world.players[1].resources;
    let (id, _) = world
        .propose_trade(
            TradeAgreement::new(
                TradeParty::Player(0),
                TradeParty::Player(1),
                a,
                b,
                TradeFrequency::OneTime,
            ),
            &MonthlyInputs::default(),
        )
        .unwrap();
    assert_eq!(world.players[0].resources, before_a);
    assert!(world.accept_trade(id, 0, &MonthlyInputs::default()).is_err());
    world.accept_trade(id, 1, &MonthlyInputs::default()).unwrap();
    close(world.players[0].resources[0], before_a[0] - 100.0);
    close(world.players[1].resources[0], before_b[0] + 95.0);
    close(world.players[0].resources[1], before_a[1] + 19.0);
    close(world.players[1].resources[1], before_b[1] - 20.0);
    close(world.trades[0].last_delivered_value, 142.5);
    close(world.trades[0].last_fulfillment, 1.0);
    assert_eq!(world.trades[0].last_executed_month, Some(0));
    assert!(world.accept_trade(id, 1, &MonthlyInputs::default()).is_err());
}

#[test]
fn one_time_trade_discards_storage_overflow_but_monthly_trade_defers_the_clamp() {
    for frequency in [TradeFrequency::OneTime, TradeFrequency::Monthly] {
        let mut world = world(2, 2);
        world.provinces[1].owner = Some(1);
        world.players[0].resources[2] = world.players[0].storage[2] - 2.0;
        world.players[1].resources[1] = world.players[1].storage[1] - 2.0;
        let offer = TradeAgreement::new(
            TradeParty::Player(0),
            TradeParty::Player(1),
            TradeBundle {
                resources: [0.0, 10.0, 0.0],
                ..Default::default()
            },
            TradeBundle {
                resources: [0.0, 0.0, 10.0],
                ..Default::default()
            },
            frequency,
        );
        let (id, _) = world.propose_trade(offer, &MonthlyInputs::default()).unwrap();
        world.accept_trade(id, 1, &MonthlyInputs::default()).unwrap();
        if frequency == TradeFrequency::Monthly {
            world.advance_trade(&MonthlyInputs::default(), &mut Vec::new());
        }
        let temporary_overflow = if frequency == TradeFrequency::Monthly {
            8.0
        } else {
            0.0
        };
        close(world.players[0].resources[2], world.players[0].storage[2] + temporary_overflow);
        close(world.players[1].resources[1], world.players[1].storage[1] + temporary_overflow);
        close(world.trades[0].last_delivered_value, 40.0);
        close(world.players[0].resources[1], world.config.starting_stock[1] - 10.0);
        close(world.players[1].resources[2], world.config.starting_stock[2] - 10.0);
    }
}

#[test]
fn monthly_failure_suspends_then_cancels_without_partial_payment() {
    let mut world = world(2, 2);
    world.provinces[1].owner = Some(1);
    let give = TradeBundle {
        resources: [0.0, 10000.0, 0.0],
        ..Default::default()
    };
    let buy = TradeBundle {
        coin: 1.0,
        ..Default::default()
    };
    let (id, _) = world
        .propose_trade(
            TradeAgreement::new(
                TradeParty::Player(0),
                TradeParty::Player(1),
                give,
                buy,
                TradeFrequency::Monthly,
            ),
            &MonthlyInputs::default(),
        )
        .unwrap();
    world.accept_trade(id, 1, &MonthlyInputs::default()).unwrap();
    let before = world.players[1].coin;
    let mut events = Vec::new();
    for iteration in 0..3 {
        world.advance_trade(&MonthlyInputs::default(), &mut events);
        assert_eq!(
            world.trades[0].status,
            if iteration < 2 {
                TradeStatus::Suspended
            } else {
                TradeStatus::Cancelled
            }
        );
        close(world.players[1].coin, before);
    }
}

#[test]
fn npc_trade_reserves_supply_and_one_time_deals_never_generate_control() {
    let mut world = world(2, 1);
    world.refresh_npc_markets(&MonthlyInputs::default());
    let offer = TradeBundle {
        coin: 40.0,
        ..Default::default()
    };
    let request = TradeBundle {
        resources: [0.0, 0.0, 10.0],
        ..Default::default()
    };
    let proposal = TradeAgreement::new(
        TradeParty::Player(0),
        TradeParty::Npc(1),
        offer,
        request,
        TradeFrequency::OneTime,
    );
    let treasury = world.provinces[1].market.coin_treasury;
    let (_, effect) = world.propose_trade(proposal, &MonthlyInputs::default()).unwrap();
    let effect = effect.unwrap();
    close(effect.control_gain, 0.0);
    assert!(effect.relation_gain <= 0.1);
    close(world.provinces[1].market.resources[2].exported, 10.0);
    close(world.provinces[1].market.coin_treasury, treasury + 40.0);
}

#[test]
fn recurring_trade_reduces_both_sides_equally_and_counts_actual_deliveries() {
    let mut world = world(2, 1);
    world.refresh_npc_markets(&MonthlyInputs::default());
    let capacity = world.provinces[1].market.resources[2].export_capacity;
    let request = TradeBundle {
        resources: [0.0, 0.0, capacity],
        ..Default::default()
    };
    let offer = TradeBundle {
        coin: 190.0,
        ..Default::default()
    };
    let proposal = TradeAgreement::new(
        TradeParty::Player(0),
        TradeParty::Npc(1),
        offer,
        request,
        TradeFrequency::Monthly,
    );
    world.propose_trade(proposal.clone(), &MonthlyInputs::default()).unwrap();
    // A later supply reduction is tolerated; a new oversized deal is never accepted.
    world.provinces[1].market.resources[2].export_capacity *= 0.85;
    let quote = world.quote_trade(&proposal, &MonthlyInputs::default()).unwrap();
    close(quote.fulfillment, 0.85);
    let before = world.players[0].coin;
    let effects = world.advance_trade(&MonthlyInputs::default(), &mut Vec::new());
    close(before - world.players[0].coin, 190.0 * 0.85);
    close(world.provinces[1].market.resources[2].exported, capacity * 0.85);
    close(world.trades[0].last_fulfillment, 0.85);
    assert_eq!(world.trades[0].last_executed_month, Some(1));
    assert!(world.trades[0].last_delivered_value > 0.0);
    assert_eq!(effects.len(), 1);
    assert!(effects[0].control_gain > 0.0 && effects[0].control_gain <= 1.0);
    world.provinces[1].market.resources[2].export_capacity = 0.0;
    assert!(world.advance_trade(&MonthlyInputs::default(), &mut Vec::new()).is_empty());
    close(world.trades[0].last_fulfillment, 0.0);
    close(world.trades[0].last_delivered_value, 0.0);
}

#[test]
fn nonfinite_and_disabled_influence_offers_are_rejected() {
    let mut world = world(2, 1);
    let a = TradeBundle {
        coin: f64::NAN,
        ..Default::default()
    };
    let b = TradeBundle {
        resources: [0.0, 0.0, 1.0],
        ..Default::default()
    };
    assert!(world
        .propose_trade(
            TradeAgreement::new(
                TradeParty::Player(0),
                TradeParty::Npc(1),
                a,
                b,
                TradeFrequency::Monthly
            ),
            &MonthlyInputs::default()
        )
        .is_err());
    assert!(!TradeBundle {
        influence: 1.0,
        ..Default::default()
    }
    .valid(&world.config.trade));
}

#[test]
fn famine_stabilization_and_storage_remain_finite_over_a_long_campaign() {
    let mut world = world(6, 1);
    world.provinces[0].capacity_area = 20.0;
    world.provinces[0].policies.focus = ResourceFocus::Food;
    for _ in 0..480 {
        world.advance_month(&MonthlyInputs::default());
        for p in &world.provinces {
            assert!(p.population.iter().all(|count| count.is_finite() && *count >= 0.0));
            assert!(p.happiness.iter().all(|happiness| (0.0..=100.0).contains(happiness)));
        }
        for resource in 0..3 {
            assert!((0.0..=world.players[0].storage[resource])
                .contains(&world.players[0].resources[resource]));
        }
    }
}

#[test]
fn canonical_map_economies_remain_finite_for_300_months() {
    let mut ownership = crate::map::ProvinceOwnership::default();
    ownership.start_game(&[bevy_egui::egui::Color32::RED]);
    let seeds = ownership.campaign_seeds();
    assert_eq!(seeds.len(), 54);
    let provinces = seeds
        .iter()
        .enumerate()
        .map(|(index, seed)| {
            let terrain = match seed.terrain {
                0 => Terrain::Desert,
                1 => Terrain::Farmland,
                2 => Terrain::Forest,
                3 => Terrain::Hills,
                6 => Terrain::Marsh,
                7 => Terrain::Mountains,
                _ => Terrain::Plains,
            };
            // Remove randomized shares/legacy compensation from the balance sample only.
            // Geography, city tags, resource potential, and crossings are real atlas data.
            let total = normalized_capacity_area(seed.area)
                + if seed.city {
                    22.0
                } else {
                    0.0
                };
            let shares = if seed.city {
                [0.12, 0.23, 0.37, 0.28]
            } else {
                [0.1, 0.2, 0.4, 0.3]
            };
            let mut province = EconomicProvince::new(
                &seed.name,
                normalized_capacity_area(seed.area),
                terrain,
                seed.city,
                seed.potential,
                shares.map(|share| share * total),
                seeds.len(),
            );
            province.owner = Some(index);
            province
        })
        .collect();
    let mut world = EconomyWorld::new(
        seeds.len(),
        provinces,
        seeds.iter().map(|seed| seed.neighbors.clone()).collect(),
    );
    let mut rows = Vec::new();
    let mut net_food = 0.0;
    let military = crate::game::military::MilitaryConfig::default();
    let heavy_cost = military.unit(crate::game::military::UnitType::HeavyInfantry).metal_cost;
    for province in &mut world.provinces {
        let neutral = province.production(&world.config).1;
        let food = province.food_request(&world.config);
        net_food += neutral[0] - food;
        province.policies.focus = ResourceFocus::Food;
        let focused = province.production(&world.config).1;
        province.policies.focus = ResourceFocus::Balanced;
        rows.push((
            province.name.clone(),
            province.total_population(),
            province.capacity(&world.config),
            neutral[0] - food,
            focused[0] - food,
            neutral[1],
            neutral[2],
            heavy_cost / neutral[1].max(0.001),
        ));
    }
    for _ in 0..300 {
        world.advance_month(&MonthlyInputs::default());
        for province in &world.provinces {
            assert!(
                province.population.iter().all(|count| count.is_finite() && *count >= 0.0),
                "{}",
                province.name
            );
            assert!(province.happiness.iter().all(|happiness| (0.0..=100.0).contains(happiness)));
        }
        for player in &world.players {
            assert!(player.balances().into_iter().all(|value| value.is_finite() && value >= 0.0));
            for resource in 0..3 {
                assert!(player.resources[resource] <= player.storage[resource]);
            }
        }
    }
    eprintln!("BALANCE: world opening Food surplus {net_food:.1}/month at Balanced focus");
    for ((name, population, capacity, net, focused_net, metal, stone, heavy_months), province) in
        rows.into_iter().zip(&world.provinces)
    {
        eprintln!("BALANCE {name}: pop {population:.1}; cap {capacity:.1}; Food {net:+.1} (focus {focused_net:+.1}); Metal {metal:.1}; Stone {stone:.1}; HI {heavy_months:.1} months Metal; pop300 {:.1}", province.total_population());
    }
}

#[test]
fn npc_abstract_provisioning_preserves_food_deficits_in_trade_demand() {
    let mut world = world(2, 1);
    world.provinces[1].potential = [0.0, 2.0, 3.0];
    let report = world.advance_month(&MonthlyInputs::default());
    close(report.province_reports[1].food_supply_ratio, 1.0);
    assert_eq!(report.province_reports[1].famine_deaths, [0.0; 4]);
    assert!(world.provinces[1].market.resources[0].import_demand > 40.0);
    assert_eq!(world.provinces[1].market.resources[0].band, DemandBand::SevereShortage);
}

#[test]
fn splitting_one_time_deals_cannot_farm_recurring_relation_rewards() {
    let mut world = world(2, 1);
    world.players[0].coin = 10_000.0;
    let offer = TradeBundle {
        coin: 100.0,
        ..Default::default()
    };
    let request = TradeBundle {
        resources: [0.0, 0.0, 1.0],
        ..Default::default()
    };
    let proposal = TradeAgreement::new(
        TradeParty::Player(0),
        TradeParty::Npc(1),
        offer,
        request,
        TradeFrequency::OneTime,
    );
    let mut relation = 0.0;
    for _ in 0..20 {
        relation += world
            .propose_trade(proposal.clone(), &MonthlyInputs::default())
            .unwrap()
            .1
            .unwrap()
            .relation_gain;
    }
    close(relation, 0.1);
}

#[test]
fn vassals_are_transit_territory_not_teleport_origins_and_npc_wars_block_routes() {
    let mut world = world(3, 1);
    world.provinces[2].overlord = Some(0);
    let route = world
        .trade_route(TradeParty::Player(0), TradeParty::Npc(2), &MonthlyInputs::default())
        .unwrap();
    assert_eq!(route, vec![0, 1, 2]);
    close(world.route_efficiency(&route), 0.95);
    // Relation is still neutral: an explicit war alone must close the transit edge.
    let input = MonthlyInputs {
        npc_hostility: vec![vec![false, true, false]],
        ..Default::default()
    };
    assert!(world.trade_route(TradeParty::Player(0), TradeParty::Npc(2), &input).is_err());
}
