use super::*;
use crate::game::economy::{EconomicProvince, EconomyWorld, Terrain};
use crate::game::politics::diplomacy::ProvincePolitics;
use crate::game::politics::espionage::SpyMission;
use crate::game::politics::PoliticalPlayer;

#[test]
fn projected_spy_upkeep_and_actual_charge_follow_the_current_route() {
    use crate::game::politics::espionage::SpyAssignment;
    let mut campaign = Campaign::default();
    let mut provinces: Vec<_> = ["Home", "Middle", "Target"]
        .into_iter()
        .map(|name| {
            EconomicProvince::new(
                name,
                40.0,
                Terrain::Farmland,
                true,
                [1.0; 3],
                [10.0, 20.0, 30.0, 40.0],
                1,
            )
        })
        .collect();
    provinces[0].owner = Some(0);
    campaign.economy = EconomyWorld::new(1, provinces, vec![vec![1], vec![0, 2], vec![1]]);
    campaign.actors = vec![PoliticalPlayer {
        coin: 100.0,
        influence: 100.0,
        ..Default::default()
    }];
    campaign.politics = vec![
        ProvincePolitics::owned(1, 0),
        ProvincePolitics::independent(1),
        ProvincePolitics::independent(1),
    ];
    campaign.wars = vec![vec![false]];
    campaign.espionage_config.detection_range = [0.0; 2];
    let distance = campaign.distance(0, 2);
    campaign
        .espionage
        .deploy_assignment(
            0,
            2,
            &mut campaign.actors,
            &campaign.politics,
            &campaign.espionage_config,
            SpyAssignment::GainControl,
            distance,
        )
        .unwrap();
    assert_eq!(campaign.actors[0].influence, 87.0);
    assert_eq!(campaign.spy_upkeep(0), 6.0);
    campaign.economy.adjacency[0].push(2);
    campaign.economy.adjacency[2].push(0);
    assert_eq!(campaign.spy_upkeep(0), 5.0);
    campaign.advance_espionage();
    assert_eq!(campaign.actors[0].coin, 95.0);
    assert_eq!(campaign.espionage.missions[0].totals.coin_spent, 5.0);
    assert_eq!(campaign.espionage.missions[0].distance, 1);
}

#[test]
fn owned_province_undermining_reduces_one_random_class_each_tick_and_keeps_a_temporary_penalty() {
    use crate::game::politics::espionage::SpyAssignment;
    let mut campaign = Campaign::default();
    let mut foreign = EconomicProvince::new(
        "Achaia",
        40.0,
        Terrain::Farmland,
        true,
        [1.0; 3],
        [10.0, 20.0, 30.0, 40.0],
        2,
    );
    foreign.owner = Some(1);
    foreign.happiness = [50.0; 4];
    foreign.temporary_happiness = [0.0; 4];
    let mut home = foreign.clone();
    home.owner = Some(0);
    campaign.economy = EconomyWorld::new(2, vec![foreign, home], vec![vec![1], vec![0]]);
    campaign.actors = vec![
        PoliticalPlayer {
            coin: 100.0,
            influence: 100.0,
            ..Default::default()
        };
        2
    ];
    campaign.politics = vec![ProvincePolitics::owned(2, 1), ProvincePolitics::owned(2, 0)];
    campaign.wars = vec![vec![false; 2]; 2];
    campaign.espionage_config.detection_range = [0.0; 2];
    // NPC success rolls must not prevent player-owned population losses.
    campaign.espionage_config.undermine_chance = 0.0;
    let distance = campaign.distance(0, 0);
    campaign
        .espionage
        .deploy_assignment(
            0,
            0,
            &mut campaign.actors,
            &campaign.politics,
            &campaign.espionage_config,
            SpyAssignment::UndermineOpponents,
            distance,
        )
        .unwrap();
    campaign.advance_espionage();
    let first_tick = campaign.economy.provinces[0].happiness;
    assert_eq!(campaign.espionage.missions[0].totals.happiness_reduced, 1.0);
    assert_eq!(first_tick.iter().filter(|&&value| value == 49.0).count(), 1);
    assert_eq!(first_tick.iter().filter(|&&value| value == 50.0).count(), 3);
    assert_eq!(
        campaign.economy.provinces[0].temporary_happiness,
        first_tick.map(|value| value - 50.0)
    );
    assert_eq!(campaign.economy.provinces[1].happiness, [50.0; 4], "Home population is unaffected");
    assert_eq!(campaign.politics[0].relation(0), 50.0);
    assert_eq!(
        campaign.politics[0].relation(1),
        50.0,
        "Owned provinces suffer happiness loss instead of Relation loss"
    );
    assert_eq!(campaign.actors[0].coin, 95.0);
    assert_eq!(campaign.actors[0].influence, 80.0);
    assert_eq!(campaign.spy_upkeep(0), 5.0);
    assert_eq!(campaign.spy_upkeep(1), 0.0, "Spy upkeep is private to its paying player");
    campaign.advance_espionage();
    assert_eq!(
        campaign.economy.provinces[0].happiness, first_tick,
        "Repeated ticks do not reduce Happiness twice"
    );
    for month in 1..=12 {
        let before = campaign.economy.provinces[0].happiness;
        campaign.economy.month = month;
        campaign.advance_espionage();
        let target = &campaign.economy.provinces[0];
        let changes =
            std::array::from_fn::<_, 4, _>(|class| before[class] - target.happiness[class]);
        assert_eq!(changes.iter().filter(|&&loss| loss == 1.0).count(), 1);
        assert_eq!(changes.iter().filter(|&&loss| loss == 0.0).count(), 3);
        assert_eq!(target.temporary_happiness, target.happiness.map(|value| value - 50.0));
    }
    assert!(
        campaign.economy.provinces[0].happiness.iter().filter(|&&value| value < 50.0).count() > 1,
        "The selected class varies across ticks"
    );
    assert_eq!(campaign.espionage.missions[0].totals.happiness_reduced, 13.0);
    assert_eq!(campaign.espionage.missions[0].totals.coin_spent, 65.0);
    campaign.economy.provinces[0].happiness = [0.5; 4];
    campaign.economy.provinces[0].temporary_happiness = [0.0; 4];
    campaign.economy.month += 1;
    campaign.advance_espionage();
    assert_eq!(
        campaign.economy.provinces[0].happiness.iter().filter(|&&value| value == 0.0).count(),
        1
    );
    assert_eq!(
        campaign.economy.provinces[0]
            .temporary_happiness
            .iter()
            .filter(|&&value| value == -0.5)
            .count(),
        1
    );
    campaign.economy.provinces[0].happiness = [0.0; 4];
    campaign.economy.month += 1;
    campaign.advance_espionage();
    assert_eq!(
        campaign.economy.provinces[0].happiness, [0.0; 4],
        "Happiness cannot fall below zero"
    );
    assert_eq!(
        campaign.espionage.missions[0].totals.happiness_reduced, 13.5,
        "Count only actual losses at the floor"
    );
    campaign.espionage.withdraw(0, 0);
    assert_eq!(campaign.spy_upkeep(0), 0.0, "Recall removes upkeep from projected outflow");
}

#[test]
fn detected_player_spy_evidence_opens_senate_and_retains_provincial_origin() {
    let mut campaign = Campaign::default();
    let mut province = EconomicProvince::new(
        "Achaia",
        40.0,
        Terrain::Farmland,
        true,
        [1.0; 3],
        [10.0, 20.0, 30.0, 40.0],
        2,
    );
    province.owner = Some(1);
    let mut home = province.clone();
    home.owner = Some(0);
    campaign.economy = EconomyWorld::new(2, vec![province, home], vec![vec![1], vec![0]]);
    campaign.actors = vec![
        PoliticalPlayer {
            coin: 100.0,
            influence: 100.0,
            ..Default::default()
        };
        2
    ];
    campaign.politics = vec![ProvincePolitics::owned(2, 1), ProvincePolitics::owned(2, 0)];
    campaign.wars = vec![vec![false; 2]; 2];
    campaign.espionage_config.detection_range = [1.0, 1.0];
    campaign.espionage.missions.push(SpyMission {
        owner: 0,
        province: 0,
        months_active: 0,
        assignment: crate::game::politics::espionage::SpyAssignment::DiscoverScandals,
        reachable: true,
        distance: 1,
        totals: Default::default(),
    });
    campaign.advance_espionage();

    let notice = campaign
        .notifications
        .history_for(1)
        .find(|notice| notice.kind == NoticeKind::ScandalDiscovered)
        .expect("victim should receive discovered player evidence");
    assert_eq!(notice.province, Some(0));
    assert!(matches!(
        notice.action,
        NoticeAction::OpenScandal {
            province: None,
            ..
        }
    ));
    assert_eq!(campaign.espionage.scandals[0].target, ScandalTarget::Player(0));
}
