//! Cross-system assertions covering the live atlas, trade evidence, and Senate inputs.

use super::*;
use crate::game::politics::espionage::TradeLeverage;
use crate::game::politics::PoliticalRank;

/// Start the same local campaign as the application, with two distinct map colors.
fn atlas_campaign() -> Campaign {
    let mut ownership = ProvinceOwnership::default();
    ownership.start_game(&[egui::Color32::RED, egui::Color32::BLUE]);
    let mut campaign = Campaign::default();
    campaign.start(&ownership, 2);
    campaign
}

#[test]
fn starting_provinces_feed_themselves_and_support_an_idle_opening_year() {
    let mut starts = std::collections::BTreeSet::new();
    for player_count in 1..=4 {
        for sample in 0..32 {
            let mut ownership = ProvinceOwnership::default();
            ownership.start_game(&vec![egui::Color32::RED; player_count]);
            let mut campaign = Campaign::default();
            campaign.start(&ownership, player_count);
            let mut discovered = false;
            for player in 0..player_count {
                let province = campaign
                    .economy
                    .provinces
                    .iter()
                    .position(|p| p.owner == Some(player))
                    .unwrap();
                let p = &campaign.economy.provinces[province];
                let production = p.production(&campaign.economy.config).1;
                let food_need = p.food_request(&campaign.economy.config)
                    + campaign.military.food_demand(ForceOwner::Player(player));
                let coin_income = p.tax_income(&campaign.economy.config);
                let influence_income =
                    p.population[0] * campaign.economy.config.influence_per_noble;
                assert!(
                    p.total_population() <= p.capacity(&campaign.economy.config) * 0.6,
                    "{} starts too close to capacity: {:.1} residents for {:.1} capacity",
                    p.name,
                    p.total_population(),
                    p.capacity(&campaign.economy.config)
                );
                assert!(
                    production[0] >= food_need,
                    "{} opens with a food deficit: {production:?} versus {food_need}",
                    p.name
                );
                assert!(
                    (campaign.projected_food_delta(player, &campaign.inputs())
                        - (production[0] - food_need))
                        .abs()
                        < 1e-8,
                    "{} must report its worker-based Food surplus accurately",
                    p.name
                );
                assert!(coin_income > 0.0, "{} has no coin income", p.name);
                assert!(influence_income > 0.0, "{} has no influence income", p.name);
                if starts.insert(p.name.clone()) {
                    discovered = true;
                }
            }
            if !discovered && sample > 0 {
                continue;
            }
            for month in 0..12 {
                campaign.advance_month();
                for player in 0..player_count {
                    let owned = campaign
                        .economy
                        .provinces
                        .iter()
                        .position(|p| p.owner == Some(player))
                        .unwrap();
                    let province = &campaign.economy.provinces[owned];
                    assert!(
                        campaign.economy.last_report.food_supply_ratio[player] >= 0.999,
                        "{} has a food shortage in month {} of its opening year: ratio {:.3}, stock {:.1}, output {:.1}, need {:.1}, pop {:?}, cap {:.1}, happiness {:?}, owner {:?}, rebellion {}, players {} sample {}",
                        province.name, month + 1,
                        campaign.economy.last_report.food_supply_ratio[player],
                        campaign.economy.players[player].resources[0],
                        province.production(&campaign.economy.config).1[0],
                        province.food_request(&campaign.economy.config),
                        province.population, province.capacity(&campaign.economy.config), province.happiness, province.owner,
                        campaign.military.provinces[owned].slave_rebellion,
                        player_count, sample
                    );
                    assert!(campaign.economy.players[player].coin > 0.0);
                    assert!(campaign.economy.players[player].influence > 0.0);
                }
            }
        }
    }
    assert_eq!(starts.len(), 7, "cover every urban starting province");
}

#[test]
fn opening_recruitment_construction_and_market_trade_remain_affordable() {
    use crate::game::economy::{CivicSpending, MarketSide};
    for _ in 0..8 {
        let mut ownership = ProvinceOwnership::default();
        ownership.start_game(&[egui::Color32::RED]);
        let mut campaign = Campaign::default();
        campaign.start(&ownership, 1);
        campaign.economy.config.slave_revolt_chance = [0.0; 2];
        let province = campaign.economy.provinces.iter().position(|p| p.owner == Some(0)).unwrap();
        let plebeians = campaign.economy.provinces[province].population[2];
        campaign
            .military
            .recruit(
                province,
                0,
                UnitType::HeavyInfantry,
                true,
                &[],
                &mut campaign.economy.provinces[province].population,
                &mut campaign.economy.players[0].resources[1],
            )
            .unwrap();
        assert_eq!(
            campaign.economy.provinces[province].population[2],
            plebeians - crate::map::POPULATION_SCALE
        );
        campaign.economy.start_building(0, province, BuildingType::Road).unwrap();
        campaign.economy.provinces[province].policies.civic_spending = CivicSpending::Normal;
        campaign.economy.exchange_open_market(0, 0, MarketSide::Sell, 100.0).unwrap();
        campaign.economy.exchange_open_market(0, 1, MarketSide::Buy, 20.0).unwrap();
        for _ in 0..12 {
            campaign.advance_month();
            assert!(
                campaign.economy.last_report.food_supply_ratio[0] >= 0.999,
                "{} ran short after a modest opening",
                campaign.economy.provinces[province].name
            );
            assert!(campaign.economy.players[0].coin > 0.0);
        }
        assert!(campaign.military.all_units().any(|unit| {
            unit.owner == ForceOwner::Player(0) && unit.unit_type == UnitType::HeavyInfantry
        }));
        assert_eq!(campaign.economy.provinces[province].level(BuildingType::Road), 1);
    }
}

#[test]
fn surviving_spy_control_resolves_in_the_same_campaign_month() {
    use crate::game::politics::espionage::SpyAssignment;
    let mut campaign = atlas_campaign();
    let target = (0..campaign.politics.len())
        .find(|&id| {
            matches!(campaign.politics[id].state, PoliticalState::Independent { .. })
                && campaign.distance(0, id).is_some()
        })
        .unwrap();
    campaign.actors[0].coin = 1000.0;
    campaign.actors[0].influence = 1000.0;
    campaign.push_wallets();
    campaign.espionage_config.detection_range = [0.0; 2];
    let distance = campaign.distance(0, target);
    campaign
        .espionage
        .deploy_assignment(
            0,
            target,
            &mut campaign.actors,
            &campaign.politics,
            &campaign.espionage_config,
            SpyAssignment::GainControl,
            distance,
        )
        .unwrap();
    campaign.push_wallets();
    assert_eq!(campaign.politics[target].independent_control(0), 0.0);
    campaign.advance_month();
    let rolled = campaign.espionage.missions[0].totals.control_contributed;
    assert!((0.0..=3.0).contains(&rolled));
    assert_eq!(campaign.politics[target].independent_control(0), rolled);
    assert_eq!(campaign.politics[target].queued_control_pressure(0), 0.0);
}

#[test]
fn rome_starts_protected_with_fifty_supplied_cohorts_and_rejects_provincial_actions() {
    let mut c = atlas_campaign();
    let rome = c.economy.provinces.iter().position(|p| p.name == "Latium").unwrap();
    assert_eq!(c.politics[rome].state, PoliticalState::Rome);
    assert_eq!(c.economy.provinces[rome].owner, None);
    assert_eq!(c.access_snapshot()[0][rome], MilitaryAccess::Blocked);
    assert_eq!(c.military.all_units().filter(|u| u.owner == ForceOwner::Local(rome)).count(), 50);
    assert_eq!(c.inputs().npc_army_food[rome], 0.0);
    let influence = c.actors[0].influence;
    assert!(c.espionage.deploy(0, rome, &mut c.actors, &c.politics, &c.espionage_config).is_err());
    assert_eq!(c.actors[0].influence, influence);
    assert!(c.politics[rome]
        .buy_control(
            0,
            &mut c.actors[0],
            crate::game::politics::Currency::Influence,
            5.0,
            Some(1),
            &c.diplomacy_config
        )
        .is_err());
    assert!(c.politics[rome]
        .improve_relation(
            0,
            &mut c.actors[0],
            crate::game::politics::Currency::Influence,
            5.0,
            Some(1),
            &c.diplomacy_config
        )
        .is_err());
    assert!(c.politics[rome].vassalize(0).is_err());
    assert!(c.politics[rome].take_ownership(0).is_err());
    let offer = TradeAgreement::new(
        TradeParty::Player(0),
        TradeParty::Npc(rome),
        TradeBundle {
            coin: 100.0,
            ..Default::default()
        },
        TradeBundle {
            resources: [1.0, 0.0, 0.0],
            ..Default::default()
        },
        TradeFrequency::OneTime,
    );
    assert!(c.economy.quote_trade(&offer, &c.inputs()).unwrap_err().contains("Rome"));
    let manpower: f64 = c
        .military
        .all_units()
        .filter(|u| u.owner == ForceOwner::Local(rome))
        .map(|u| u.current_manpower)
        .sum();
    for _ in 0..3 {
        c.advance_month();
    }
    assert_eq!(c.politics[rome].state, PoliticalState::Rome);
    assert_eq!(c.politics[rome].independent_control(0), 0.0);
    assert_eq!(
        c.military
            .all_units()
            .filter(|u| u.owner == ForceOwner::Local(rome))
            .map(|u| u.current_manpower)
            .sum::<f64>(),
        manpower
    );
}

#[test]
fn unguarded_rome_requires_hostility_and_presence_then_wins_immediately() {
    let mut c = atlas_campaign();
    let rome = c.economy.provinces.iter().position(|p| p.name == "Latium").unwrap();
    c.military.provinces[rome].forces.clear();
    c.military.seed_unit(rome, ForceOwner::Player(0), UnitType::HeavyInfantry).unwrap();
    c.begin_encounter(rome, ForceOwner::Player(0), None);
    assert_eq!(c.senate.winner, None);
    c.declare_hostility(0, rome);
    assert_eq!(c.senate.winner, Some(0));
    assert_eq!(c.actors[0].rank, PoliticalRank::Augustus);
    assert_eq!(c.economy.provinces[rome].owner, Some(0));
    let month = c.economy.month;
    c.advance_month();
    assert_eq!(c.economy.month, month);
}

#[test]
fn construction_only_emits_a_private_actionable_notice_when_completed() {
    use super::super::campaign_notifications::{NoticeAction, NoticeKind, NoticeSeverity};
    let mut campaign = atlas_campaign();
    let province = campaign.economy.provinces.iter().position(|p| p.owner == Some(0)).unwrap();
    campaign.economy.players[0].resources = [10_000.0; 3];
    campaign.economy.start_building(0, province, BuildingType::Granary).unwrap();
    assert!(campaign.notifications.drain_for(0).is_empty());
    assert_eq!(campaign.notifications.history_for(0).count(), 0);
    assert!(campaign.notifications.drain_for(1).is_empty());
    for _ in 0..4 {
        let report = campaign.economy.advance_month(&campaign.inputs());
        campaign.record_economic_events(&report);
    }
    let finished: Vec<_> = campaign
        .notifications
        .drain_for(0)
        .into_iter()
        .filter(|notice| notice.kind == NoticeKind::BuildingCompleted)
        .collect();
    assert_eq!(finished.len(), 1, "Construction must announce completion exactly once");
    assert_eq!(
        campaign
            .notifications
            .history_for(0)
            .filter(|n| n.kind == NoticeKind::BuildingCompleted)
            .count(),
        1,
        "The completion must remain in notification history"
    );
    assert_eq!(finished[0].severity, NoticeSeverity::Info);
    assert_eq!(finished[0].action, NoticeAction::OpenProvince(province));
    assert_eq!(finished[0].title, "Granary completed");
    assert_eq!(finished[0].building, Some(BuildingType::Granary));
    assert_eq!(
        finished[0].body,
        format!("Level 1 construction finished in {}.", campaign.economy.provinces[province].name)
    );
    assert_eq!(campaign.economy.provinces[province].level(BuildingType::Granary), 1);
    assert!(campaign.notifications.drain_for(1).is_empty());
}

#[test]
fn cancelling_and_restarting_in_the_same_month_does_not_create_notices() {
    let mut campaign = atlas_campaign();
    let province = campaign.economy.provinces.iter().position(|p| p.owner == Some(0)).unwrap();
    campaign.economy.players[0].resources = [10_000.0; 3];
    for building in [BuildingType::Granary, BuildingType::Warehouse] {
        campaign.economy.start_building(0, province, building).unwrap();
        assert!(campaign.notifications.drain_for(0).is_empty());
        campaign.economy.cancel_construction(0, province).unwrap();
    }
    assert_eq!(campaign.notifications.history_for(0).count(), 0);
}

#[test]
fn nationwide_edicts_follow_ownership_and_leave_local_focus_and_migration_intact() {
    use crate::map::EdictLevel;
    let mut campaign = atlas_campaign();
    let owned = campaign.economy.provinces.iter().position(|p| p.owner == Some(0)).unwrap();
    let foreign = campaign.economy.provinces.iter().position(|p| p.owner == Some(1)).unwrap();
    campaign.economy.provinces[owned].policies.focus = ResourceFocus::Metal;
    campaign.economy.provinces[owned].policies.migration = MigrationPolicy::Closed;
    let foreign_policies = campaign.economy.provinces[foreign].policies;
    let tax_before = campaign.economy.provinces[owned].tax_income(&campaign.economy.config);
    campaign.set_governance(
        0,
        Governance {
            food_rations: EdictLevel::High,
            slave_labor: EdictLevel::High,
            army_wages: EdictLevel::High,
            ..Default::default()
        },
    );
    let p = &campaign.economy.provinces[owned];
    assert_eq!(p.policies.food, FoodPolicy::High);
    assert_eq!(p.policies.slave_labor, SlaveLabor::Harsh);
    assert_eq!(p.policies.focus, ResourceFocus::Metal);
    assert_eq!(p.policies.migration, MigrationPolicy::Closed);
    assert_eq!(p.tax_income(&campaign.economy.config), tax_before);
    assert_eq!(campaign.economy.provinces[foreign].policies, foreign_policies);
    campaign.politics[foreign] = ProvincePolitics::owned(2, 0);
    campaign.reconcile_provinces();
    assert_eq!(campaign.economy.provinces[foreign].policies.food, FoodPolicy::High);
    assert_eq!(campaign.economy.provinces[foreign].policies.slave_labor, SlaveLabor::Harsh);
    campaign.politics[owned] = ProvincePolitics::owned(2, 1);
    campaign.reconcile_provinces();
    assert_eq!(campaign.economy.provinces[owned].policies.food, FoodPolicy::Normal);
    assert_eq!(campaign.economy.provinces[owned].policies.slave_labor, SlaveLabor::Normal);
}

#[test]
fn nationwide_wages_charge_real_soldiers_and_change_peaceful_morale_targets() {
    use crate::map::EdictLevel;
    let mut campaign = atlas_campaign();
    let owned = campaign.economy.provinces.iter().position(|p| p.owner == Some(0)).unwrap();
    let owner = ForceOwner::Player(0);
    let unit = campaign.military.seed_unit(owned, owner, UnitType::HeavyInfantry).unwrap();
    campaign.set_governance(
        0,
        Governance {
            army_wages: EdictLevel::High,
            ..Default::default()
        },
    );
    campaign.economy.players[0].coin = 100.0;
    campaign.military.config.passive_training = 0.0;
    let stationed = campaign.military.provinces[owned].forces.get_mut(&owner).unwrap();
    let unit = stationed.iter_mut().find(|u| u.id == unit).unwrap();
    unit.morale = campaign.military.config.base_morale;
    unit.training = 0.0;
    let wage =
        campaign.military.config.unit(unit.unit_type).coin_per_month * unit.manpower_ratio() * 1.25;
    campaign.pay_army_wages(0, 1.0);
    assert!((campaign.economy.players[0].coin - (100.0 - wage)).abs() < 1e-8);
    assert_eq!(campaign.military.provinces[owned].forces[&owner][0].morale, 100.0);
    campaign.pay_army_wages(0, 1.0);
    assert_eq!(
        campaign.military.provinces[owned].forces[&owner][0].morale, 100.0,
        "Wage morale must not accumulate each month"
    );
    campaign.economy.players[0].coin = 0.0;
    campaign.pay_army_wages(0, 1.0);
    assert_eq!(campaign.economy.players[0].coin, 0.0);
    assert!(campaign.military.provinces[owned].forces[&owner][0].morale < 100.0);
}

#[test]
fn noble_wages_charge_owned_nobles_and_recompose_happiness() {
    use crate::map::EdictLevel;
    let mut campaign = atlas_campaign();
    let owned = campaign.economy.provinces.iter().position(|p| p.owner == Some(0)).unwrap();
    let foreign = campaign.economy.provinces.iter().position(|p| p.owner == Some(1)).unwrap();
    for province in &mut campaign.economy.provinces {
        if province.owner == Some(0) {
            province.population = [0.0; 4];
        }
    }
    campaign.economy.provinces[owned].population[0] = 100.0;
    let foreign_happiness = campaign.economy.provinces[foreign].happiness[0];
    for (level, cost, happiness) in [
        (EdictLevel::Low, 0.75, -2.0),
        (EdictLevel::Medium, 1.0, 0.0),
        (EdictLevel::High, 1.25, 1.0),
    ] {
        campaign.set_governance(
            0,
            Governance {
                noble_wages: level,
                ..Default::default()
            },
        );
        campaign.economy.players[0].coin = 100.0;
        assert!((campaign.noble_wages(0) - cost).abs() < 1e-9);
        campaign.pay_noble_wages(0);
        assert!((campaign.economy.players[0].coin - (100.0 - cost)).abs() < 1e-9);
        assert!((campaign.economy.provinces[owned].happiness[0] - (50.0 + happiness)).abs() < 1e-9);
        assert_eq!(campaign.economy.provinces[foreign].happiness[0], foreign_happiness);
    }
    campaign.economy.players[0].coin = 0.0;
    campaign.pay_noble_wages(0);
    assert_eq!(campaign.economy.provinces[owned].happiness[0], 51.0);
    let report = MonthlyReport {
        province_reports: vec![Default::default(); campaign.economy.provinces.len()],
        ..Default::default()
    };
    campaign.apply_insolvency(&report, &Default::default());
    assert_eq!(campaign.economy.provinces[owned].insolvency_unhappiness, 3.0);
    assert_eq!(campaign.economy.provinces[owned].happiness[0], 48.0);
    campaign.apply_insolvency(&report, &Default::default());
    assert_eq!(campaign.economy.provinces[owned].insolvency_unhappiness, 6.0);
    assert_eq!(campaign.economy.provinces[owned].happiness[0], 45.0);
}

#[test]
fn food_shortage_is_announced_once_until_recovery_for_each_player() {
    use super::super::campaign_notifications::NoticeKind;
    let mut campaign = atlas_campaign();
    for (month, supply) in [(1, vec![0.88, 1.0]), (2, vec![0.88, 0.8]), (3, vec![0.7, 0.8])] {
        let report = MonthlyReport {
            month,
            food_supply_ratio: supply,
            ..Default::default()
        };
        campaign.record_economic_events(&report);
    }
    for player in 0..2 {
        let notices = campaign.notifications.drain_for(player);
        assert_eq!(notices.iter().filter(|n| n.kind == NoticeKind::FoodShortage).count(), 1);
        assert_eq!(
            notices.iter().find(|n| n.kind == NoticeKind::FoodShortage).unwrap().body,
            format!(
                "Civilian and military food requests were supplied at {}%.",
                if player == 0 {
                    88
                } else {
                    80
                }
            )
        );
    }
    campaign.record_economic_events(&MonthlyReport {
        month: 4,
        food_supply_ratio: vec![0.9, 0.8],
        ..Default::default()
    });
    campaign.record_economic_events(&MonthlyReport {
        month: 5,
        food_supply_ratio: vec![0.88, 0.8],
        ..Default::default()
    });
    assert_eq!(campaign.notifications.drain_for(0).len(), 1);
    assert!(campaign.notifications.drain_for(1).is_empty());
}

#[test]
fn civilian_hud_totals_sum_owned_provinces_and_their_growth() {
    let mut campaign = atlas_campaign();
    for (id, province) in campaign.economy.provinces.iter_mut().enumerate() {
        province.owner = if id < 2 {
            Some(0)
        } else {
            Some(1)
        };
        province.population = if id < 2 {
            [10.0; 4]
        } else {
            [1000.0; 4]
        };
    }
    campaign.economy.last_report.province_reports =
        vec![ProvinceMonth::default(); campaign.economy.provinces.len()];
    campaign.economy.last_report.province_reports[0].population_delta = 2.5;
    campaign.economy.last_report.province_reports[1].population_delta = -1.25;
    campaign.economy.last_report.province_reports[2].population_delta = 100.0;
    let (total, capacity, growth) = campaign.economy.player_population(0);
    assert_eq!(total, 80.0);
    assert_eq!(growth, 1.25);
    assert_eq!(
        capacity,
        campaign.economy.provinces[..2]
            .iter()
            .map(|p| p.capacity(&campaign.economy.config))
            .sum::<f64>()
    );
    assert_eq!(campaign.economy.player_population(2), (0.0, 0.0, 0.0));
}

#[test]
fn every_live_wonder_can_begin_in_its_atlas_province() {
    let mut campaign = atlas_campaign();
    let definitions = campaign.economy.config.wonders.clone();
    assert_eq!(definitions.len(), 7);
    for definition in definitions {
        let id = campaign
            .economy
            .provinces
            .iter()
            .position(|p| p.wonder_sites.contains(&definition.wonder_id))
            .unwrap();
        campaign.economy.provinces[id].owner = Some(0);
        campaign.economy.players[0].resources = [0.0, definition.metal_cost, definition.stone_cost];
        campaign.economy.start_wonder(0, id, definition.wonder_id).unwrap();
        assert!(matches!(&campaign.economy.provinces[id].construction,
            Some(ConstructionProject::Wonder(project)) if project.wonder_id==definition.wonder_id));
        campaign.economy.cancel_construction(0, id).unwrap();
    }
}

#[test]
fn all_requested_sea_crossings_are_bidirectional_campaign_edges() {
    let c = atlas_campaign();
    for (a, b) in [
        ("Macedonia", "Apulia"),
        ("Asia", "Achaia"),
        ("Creta", "Achaia"),
        ("Creta", "Asia"),
        ("Cyprus", "Cilicia"),
        ("Cyprus", "Syria"),
        ("Sicilia", "Africa Proconsularis"),
        ("Africa Proconsularis", "Sardinia"),
        ("Sardinia", "Etruria"),
        ("Mauretania Tingitana", "Baetica"),
        ("Britannia", "Belgica"),
    ] {
        let id = |name| c.economy.provinces.iter().position(|p| p.name == name).unwrap();
        let (a, b) = (id(a), id(b));
        assert!(c.economy.adjacency[a].contains(&b));
        assert!(c.economy.adjacency[b].contains(&a));
        assert!(c.graph[a].neighbors.contains(&b));
        assert!(c.graph[b].neighbors.contains(&a));
    }
}

#[test]
fn nile_uses_floodplain_capacity_without_changing_military_terrain() {
    let c = atlas_campaign();
    let id = c.economy.provinces.iter().position(|p| p.name == "Aegyptus").unwrap();
    assert_eq!(c.economy.provinces[id].terrain, Terrain::Farmland);
    assert_eq!(c.graph[id].terrain, MilitaryTerrain::Desert);
    assert!(
        c.economy.provinces[id].capacity(&c.economy.config)
            > c.economy.provinces[id].total_population()
    );
}

#[test]
fn blackmail_trade_multiplier_is_private_and_expires_in_economy_projection() {
    let mut c = atlas_campaign();
    let id = c.economy.provinces.iter().position(|p| p.owner.is_none()).unwrap();
    c.espionage.favorable_trade.push(TradeLeverage {
        player: 0,
        province: id,
        expires: 12,
        ratio: 0.75,
    });
    c.reconcile_provinces();
    assert_eq!(c.economy.provinces[id].trade_ratio_by_player, [0.75, 1.0]);
    c.economy.month = 12;
    c.reconcile_provinces();
    assert_eq!(c.economy.provinces[id].trade_ratio_by_player, [1.0, 1.0]);
}

#[test]
fn senate_merchant_inputs_follow_delivered_trade_and_failure() {
    let mut c = atlas_campaign();
    c.economy.month = 3;
    let npc = c.economy.provinces.iter().position(|p| p.owner.is_none()).unwrap();
    let mut trade = TradeAgreement::new(
        TradeParty::Player(0),
        TradeParty::Npc(npc),
        TradeBundle::default(),
        TradeBundle::default(),
        TradeFrequency::Monthly,
    );
    trade.status = TradeStatus::Active;
    trade.last_executed_month = Some(3);
    trade.last_delivered_value = 42.;
    trade.last_fulfillment = 0.85;
    c.economy.trades.push(trade);
    c.refresh_profiles();
    assert_eq!(c.profiles[0].trade_volume, 42.);
    assert_eq!(c.profiles[0].provincial_trade, 42.);
    assert_eq!(c.profiles[0].trade_reliability, 0.85);
    assert_eq!(c.profiles[0].active_trade_routes, 0.0);
    assert_eq!(c.profiles[1].trade_volume, 0.);
    c.economy.trades[0].last_fulfillment = 1.0;
    c.refresh_profiles();
    assert_eq!(c.profiles[0].active_trade_routes, 1.0);
    c.economy.month = 4;
    c.economy.trades[0].status = TradeStatus::Suspended;
    c.refresh_profiles();
    assert_eq!(c.profiles[0].trade_volume, 0.);
    assert_eq!(c.profiles[0].trade_reliability, 0.);
    assert_eq!(c.profiles[0].active_trade_routes, 0.0);
}

#[test]
fn senator_outreach_is_charged_to_the_authoritative_wallet_and_monthly_report() {
    use crate::game::politics::senate::SenatorAction;
    let mut c = atlas_campaign();
    c.economy.players[0].coin = 1000.0;
    c.economy.players[0].influence = 1000.0;
    c.pull_wallets();
    c.senate.act_on_senator(0, 0, SenatorAction::Lobby, &mut c.actors, &c.senate_config).unwrap();
    c.push_wallets();
    let before = c.economy.players[0].influence;
    c.advance_month();
    assert_eq!(c.senate.senators[0].arrangement.unwrap().paid_at, Some(0));
    assert!(
        (c.economy.players[0].influence - before - c.economy.last_report.player_delta[0][4]).abs()
            < 1e-8
    );
    assert_eq!(c.actors[0].influence, c.economy.players[0].influence);
    assert_eq!(c.senate.outreach_upkeep(0, &c.senate_config), 4.0);
}

#[test]
fn censor_income_is_added_once_to_the_authoritative_wallet() {
    let mut c = atlas_campaign();
    c.actors[0].rank = PoliticalRank::Censor;
    let before = c.economy.players[0].influence;
    c.advance_month();
    let domestic: f64 = c
        .economy
        .provinces
        .iter()
        .zip(&c.economy.last_report.province_reports)
        .filter(|(p, _)| p.owner == Some(0))
        .map(|(_, r)| r.influence_income)
        .sum();
    assert!((c.economy.players[0].influence - before - domestic - 15.).abs() < 1e-8);
    assert_eq!(c.actors[0].influence, c.economy.players[0].influence);
    assert!((c.economy.last_report.player_delta[0][4] - domestic - 15.).abs() < 1e-8);
}

#[test]
fn recurring_support_cannot_spend_tax_income_before_it_arrives() {
    let mut c = atlas_campaign();
    let npc = c.economy.provinces.iter().position(|p| p.owner.is_none()).unwrap();
    c.economy.players[0].coin = 0.0;
    c.politics[npc].support[0].relation_coin = true;
    let relation = c.politics[npc].relation(0);
    c.advance_month();
    assert_eq!(c.politics[npc].relation(0), relation);
    assert!(c.economy.players[0].coin > 0.0, "tax arrives after support was skipped");
}

#[test]
fn ownership_transfer_recalculates_storage_and_discards_excess_immediately() {
    let mut c = atlas_campaign();
    let province = c.economy.provinces.iter().position(|p| p.owner == Some(0)).unwrap();
    c.economy.provinces[province].buildings[BuildingType::Granary as usize] = 1;
    c.economy.recalculate_storage();
    let old_capacity = c.economy.players[0].storage[0];
    let new_owner_capacity = c.economy.players[1].storage[0];
    c.economy.players[0].resources[0] = old_capacity;
    c.politics[province].capture_owned(1).unwrap();
    c.reconcile_provinces();
    assert_eq!(c.economy.players[0].storage[0], old_capacity - 600.0);
    assert_eq!(c.economy.players[0].resources[0], old_capacity - 600.0);
    assert_eq!(c.economy.players[1].storage[0], new_owner_capacity + 600.0);
    // A synchronization without a territorial change must not disrupt an in-flight
    // month's temporary overflow before trade and consumption have finished.
    c.economy.players[0].resources[0] += 8.0;
    c.reconcile_provinces();
    assert_eq!(c.economy.players[0].resources[0], old_capacity - 592.0);
}
