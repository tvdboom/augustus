use super::*;
use crate::game::economy::{EconomicProvince, EconomyWorld, Terrain};
use crate::game::politics::PoliticalPlayer;

#[test]
fn nationwide_event_is_instant_and_each_type_has_its_own_24_month_cooldown() {
    let mut campaign = Campaign {
        active: true,
        ..Default::default()
    };
    let mut provinces: Vec<_> = (0..3)
        .map(|_| {
            EconomicProvince::new(
                "Province",
                20.0,
                Terrain::Plains,
                false,
                [1.0; 3],
                [10.0, 70.0, 115.0, 85.0],
                2,
            )
        })
        .collect();
    provinces[0].owner = Some(0);
    provinces[1].owner = Some(0);
    provinces[2].owner = Some(1);
    campaign.economy = EconomyWorld::new(2, provinces, vec![vec![]; 3]);
    campaign.actors = vec![PoliticalPlayer::default(); 2];
    campaign.economy.players[0].coin = 500.0;
    campaign.hold_event(0, CivicEvent::Theater).unwrap();
    assert_eq!(campaign.economy.players[0].coin, 400.0);
    assert_eq!(campaign.actors[0].coin, 400.0);
    assert_eq!(campaign.economy.provinces[0].happiness[1], 60.0);
    assert_eq!(campaign.economy.provinces[1].happiness[1], 60.0);
    assert_eq!(campaign.economy.provinces[2].happiness[1], 50.0);
    assert_eq!(
        campaign.hold_event(0, CivicEvent::Theater),
        Err(EventError::Cooldown {
            ready_month: 24
        })
    );
    assert!(campaign.hold_event(0, CivicEvent::Games).is_ok());
    campaign.economy.month = 23;
    assert!(matches!(
        campaign.event_availability(0, CivicEvent::Theater),
        Err(EventError::Cooldown { .. })
    ));
    campaign.economy.month = 24;
    assert!(campaign.hold_event(0, CivicEvent::Theater).is_ok());
}

#[test]
fn unaffordable_event_does_not_charge_or_start_cooldown() {
    let mut campaign = Campaign {
        active: true,
        ..Default::default()
    };
    let mut province =
        EconomicProvince::new("Home", 20.0, Terrain::Plains, false, [1.0; 3], [10.0; 4], 1);
    province.owner = Some(0);
    campaign.economy = EconomyWorld::new(1, vec![province], vec![vec![]]);
    campaign.actors = vec![PoliticalPlayer::default()];
    campaign.economy.players[0].coin = 0.0;
    assert_eq!(campaign.hold_event(0, CivicEvent::Theater), Err(EventError::CannotAfford));
    assert_eq!(campaign.economy.players[0].coin, 0.0);
    assert!(campaign.event_used.is_empty());
}

#[test]
fn patronage_converts_influence_to_money_without_a_monthly_modifier() {
    let mut campaign = Campaign {
        active: true,
        ..Default::default()
    };
    let mut province = EconomicProvince::new(
        "Home",
        20.0,
        Terrain::Plains,
        false,
        [1.0; 3],
        [70.0, 140.0, 230.0, 170.0],
        1,
    );
    province.owner = Some(0);
    campaign.economy = EconomyWorld::new(1, vec![province], vec![vec![]]);
    campaign.actors = vec![PoliticalPlayer::default()];
    campaign.economy.players[0].coin = 0.0;
    campaign.economy.players[0].influence = 15.0;

    campaign.hold_event(0, CivicEvent::Patronage).unwrap();

    assert_eq!(campaign.economy.players[0].coin, 90.0);
    assert_eq!(campaign.economy.players[0].influence, 0.0);
    assert_eq!(campaign.actors[0].coin, 90.0);
    assert_eq!(campaign.economy.provinces[0].happiness, [50.0; 4]);

    campaign.economy.provinces[0].population[0] = 140.0;
    campaign.economy.players[0].influence = 30.0;
    campaign.economy.month = 24;
    let quote = campaign.event_quote(0, CivicEvent::Patronage);
    assert_eq!(quote.cost.influence, 30.0);
    assert_eq!(quote.coin_reward, 180.0);
    campaign.hold_event(0, CivicEvent::Patronage).unwrap();
    assert_eq!(campaign.economy.players[0].coin, 270.0);
    assert_eq!(campaign.economy.players[0].influence, 0.0);
}

#[test]
fn every_event_cost_scales_with_its_owned_beneficiaries() {
    let mut campaign = Campaign::default();
    let mut home = EconomicProvince::new(
        "Home",
        20.0,
        Terrain::Plains,
        false,
        [1.0; 3],
        [70.0, 140.0, 230.0, 170.0],
        2,
    );
    home.owner = Some(0);
    let mut foreign = home.clone();
    foreign.owner = Some(1);
    foreign.population = [1_000.0; 4];
    campaign.economy = EconomyWorld::new(2, vec![home, foreign], vec![vec![]; 2]);
    for (event, coin, food, influence) in [
        (CivicEvent::Theater, 100.0, 0.0, 0.0),
        (CivicEvent::Feast, 120.0, 30.0, 0.0),
        (CivicEvent::Games, 160.0, 0.0, 0.0),
        (CivicEvent::GrainDole, 70.0, 70.0, 0.0),
        (CivicEvent::Patronage, 0.0, 0.0, 15.0),
    ] {
        let quote = campaign.event_quote(0, event);
        assert_eq!(
            (quote.cost.coin, quote.cost.food, quote.cost.influence),
            (coin, food, influence)
        );
    }
    campaign.economy.provinces[0].population = [140.0, 280.0, 460.0, 340.0];
    for (event, coin, food, influence) in [
        (CivicEvent::Theater, 200.0, 0.0, 0.0),
        (CivicEvent::Feast, 240.0, 60.0, 0.0),
        (CivicEvent::Games, 320.0, 0.0, 0.0),
        (CivicEvent::GrainDole, 140.0, 140.0, 0.0),
        (CivicEvent::Patronage, 0.0, 0.0, 30.0),
    ] {
        let quote = campaign.event_quote(0, event);
        assert_eq!(
            (quote.cost.coin, quote.cost.food, quote.cost.influence),
            (coin, food, influence)
        );
    }
}
