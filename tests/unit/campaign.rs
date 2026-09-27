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
fn every_live_wonder_can_begin_in_its_atlas_province() {
    let mut campaign = atlas_campaign();
    let definitions = campaign.economy.config.wonders.clone();
    assert_eq!(definitions.len(), 10);
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
        ("Asia", "Achaia"),
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
    assert_eq!(c.profiles[1].trade_volume, 0.);
    c.economy.month = 4;
    c.economy.trades[0].status = TradeStatus::Suspended;
    c.refresh_profiles();
    assert_eq!(c.profiles[0].trade_volume, 0.);
    assert_eq!(c.profiles[0].trade_reliability, 0.);
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
    assert!((c.economy.players[0].influence - before - domestic - 3.).abs() < 1e-8);
    assert_eq!(c.actors[0].influence, c.economy.players[0].influence);
    assert!((c.economy.last_report.player_delta[0][4] - domestic - 3.).abs() < 1e-8);
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
