use super::*;
use crate::game::economy::{EconomicProvince, EconomyWorld, Terrain};
use crate::game::politics::diplomacy::ProvincePolitics;
use crate::game::politics::espionage::SpyMission;
use crate::game::politics::PoliticalPlayer;

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
    campaign.economy = EconomyWorld::new(2, vec![province], vec![vec![]]);
    campaign.actors = vec![
        PoliticalPlayer {
            coin: 100.0,
            influence: 100.0,
            ..Default::default()
        };
        2
    ];
    campaign.politics = vec![ProvincePolitics::owned(2, 1)];
    campaign.espionage_config.detection_range = [1.0, 1.0];
    campaign.espionage.missions.push(SpyMission {
        owner: 0,
        province: 0,
        months_active: 0,
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
