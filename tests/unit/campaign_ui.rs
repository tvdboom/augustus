use super::*;

#[test]
fn unhappy_population_notice_opens_its_province_overview() {
    use super::super::campaign_notifications::{NoticeKind, NoticeSeverity};

    let mut campaign = Campaign::default();
    campaign.notifications.province_notice(
        0,
        1,
        2,
        NoticeSeverity::Warning,
        NoticeKind::PopulationUnhappy(0),
        "Population unhappy",
        "Nobles are unhappy in the province.",
    );
    let notice = campaign.notifications.history_for(0).next().unwrap().clone();
    let mut view = CampaignUi::default();
    let mut detail = ProvincePanelOpen::default();
    let mut map = MapView::default();

    open_notification(&mut view, &notice, &campaign, &mut detail, &mut map);

    assert_eq!(view.open, Some(CampaignTab::Province));
    assert_eq!(view.province, Some(1));
    assert_eq!(view.section, 0);
    assert_eq!(detail.0, Some(MapDetail::Province(1)));
}

#[test]
fn food_shortage_notice_opens_its_province_overview() {
    use super::super::campaign_notifications::{NoticeKind, NoticeSeverity};

    let mut campaign = Campaign::default();
    campaign.notifications.province_notice(
        0,
        1,
        2,
        NoticeSeverity::Warning,
        NoticeKind::FoodShortage,
        "Food shortage",
        "Food requests were only partly supplied.",
    );
    let notice = campaign.notifications.history_for(0).next().unwrap().clone();
    let mut view = CampaignUi::default();
    let mut detail = ProvincePanelOpen::default();
    let mut map = MapView::default();

    open_notification(&mut view, &notice, &campaign, &mut detail, &mut map);

    assert_eq!(view.open, Some(CampaignTab::Province));
    assert_eq!(view.province, Some(1));
    assert_eq!(view.section, 0);
    assert_eq!(detail.0, Some(MapDetail::Province(1)));
}

#[test]
fn politics_directory_reports_player_control_and_capital_exception() {
    let mut province = ProvincePolitics::independent(2);
    assert_eq!(politics_reading(&province, 0, 50.0), ("Independent", 0.0, Some(50.0)));
    province.state = PoliticalState::Independent {
        local: 70.0,
        shares: vec![25.0, 5.0],
    };
    province.change_relation(0, 20.0);
    assert_eq!(politics_reading(&province, 0, 50.0), ("Independent", 25.0, Some(70.0)));
    province.state = PoliticalState::Vassal {
        overlord: 1,
        control: 80.0,
        tribute: Tribute::Normal,
    };
    assert_eq!(politics_reading(&province, 0, 50.0), ("Foreign vassal", 0.0, Some(70.0)));
    assert_eq!(politics_reading(&province, 1, 50.0), ("Your vassal", 80.0, Some(50.0)));
    province.state = PoliticalState::Owned {
        owner: 0,
    };
    assert_eq!(politics_reading(&province, 0, 72.0), ("Your province", 100.0, Some(72.0)));
    province.state = PoliticalState::Rome;
    assert_eq!(politics_reading(&province, 0, 50.0), ("Capital", 0.0, None));
}

#[test]
fn acquisition_actions_use_actual_thresholds_and_preserve_both_choices_at_full_control() {
    let mut politics = ProvincePolitics::independent(2);
    for (control, expected) in [(50.0, 0), (50.25, 1), (75.0, 1), (100.0, 2)] {
        politics.state = PoliticalState::Independent {
            local: 100.0 - control,
            shares: vec![control, 0.0],
        };
        assert_eq!(acquisition_actions(&politics, 0).len(), expected);
    }
    politics.state = PoliticalState::Vassal {
        overlord: 0,
        control: 100.0,
        tribute: Tribute::Normal,
    };
    assert_eq!(acquisition_actions(&politics, 0).len(), 1);
    assert!(acquisition_actions(&politics, 1).is_empty());
    politics.state = PoliticalState::Owned {
        owner: 1,
    };
    assert!(acquisition_actions(&politics, 0).is_empty());
}

#[test]
fn distance_prices_increase_by_quarters_and_cap_at_three() {
    for (steps, expected) in [
        (0, 1.0),
        (1, 1.0),
        (2, 1.25),
        (4, 1.5),
        (6, 1.75),
        (8, 2.0),
        (10, 2.25),
        (12, 2.5),
        (14, 2.75),
        (16, 3.0),
        (100, 3.0),
    ] {
        assert_eq!(distance_multiplier(Some(steps)).unwrap(), expected);
    }
    assert_eq!(distance_multiplier(None).unwrap(), 3.0);
}

#[test]
fn map_army_clicks_toggle_owned_armies_and_open_foreign_military() {
    let ctx = egui::Context::default();
    let mut view = CampaignUi::default();
    view.open_province_section(1, 3);
    let mut detail = ProvincePanelOpen(Some(MapDetail::Province(1)));
    open_map_army(&ctx, &mut view, &mut detail, 0, ForceOwner::Player(0), None, 0);
    assert_eq!(view.open, Some(CampaignTab::Province));
    assert_eq!(view.province, Some(1));
    assert_eq!(detail.0, Some(MapDetail::Province(1)));
    assert_eq!(campaign_military::selected_army_province(&ctx), Some(0));
    open_map_army(&ctx, &mut view, &mut detail, 0, ForceOwner::Player(0), None, 0);
    assert_eq!(campaign_military::selected_army_province(&ctx), None);

    open_map_army(&ctx, &mut view, &mut detail, 0, ForceOwner::Player(0), Some(42), 0);
    assert_eq!(campaign_military::selected_army_province(&ctx), Some(0));
    open_map_army(&ctx, &mut view, &mut detail, 1, ForceOwner::Player(1), None, 0);
    assert_eq!(campaign_military::selected_army_province(&ctx), None);
    assert_eq!(view.open, Some(CampaignTab::Province));
    assert_eq!(view.province, Some(1));
    assert_eq!(view.section, 3);
    assert_eq!(view.last_detail, detail.0);
    assert_eq!(detail.0, Some(MapDetail::Province(1)));

    open_map_army(&ctx, &mut view, &mut detail, 2, ForceOwner::Local(2), None, 0);
    assert_eq!(view.province, Some(2));
    assert_eq!(view.section, 3);
    assert_eq!(detail.0, Some(MapDetail::Province(2)));
}

#[test]
fn army_overview_navigation_keeps_military_selected_when_map_detail_updates() {
    let ctx = egui::Context::default();
    let mut view = CampaignUi {
        open: Some(CampaignTab::Military),
        province: Some(1),
        notice: "Old province action".into(),
        ..Default::default()
    };
    let mut detail = ProvincePanelOpen::default();
    let mut map = MapView::default();
    open_military_province(&ctx, &mut view, &mut detail, &mut map, 0, 0);
    assert_eq!(view.open, Some(CampaignTab::Province));
    assert_eq!(view.province, Some(0));
    assert_eq!(view.section, 3);
    assert_eq!(detail.0, Some(MapDetail::Province(0)));
    assert_eq!(view.last_detail, detail.0, "The following frame must not reset to Overview");
    assert!(view.notice.is_empty());
}

#[test]
fn integration_adds_the_relation_shift_once_and_preserves_prior_unrest() {
    use crate::game::economy::{EconomicProvince, Terrain};
    let mut campaign = Campaign::default();
    campaign.politics = vec![ProvincePolitics::independent(1)];
    campaign.politics[0].state = PoliticalState::Vassal {
        overlord: 0,
        control: 100.0,
        tribute: Tribute::Normal,
    };
    campaign.politics[0].relations[0] = 80.0;
    let mut province =
        EconomicProvince::new("Test", 50.0, Terrain::Plains, false, [1.0; 3], [10.0; 4], 1);
    province.temporary_happiness = [-5.0; 4];
    campaign.economy.provinces.push(province);
    campaign.graph.push(crate::game::military::MilitaryProvince {
        terrain: crate::game::military::MilitaryTerrain::Plains,
        area: 50.0,
        road_level: 0,
        neighbors: vec![],
    });
    integrate_province(&mut campaign, 0, 0).unwrap();
    assert_eq!(campaign.economy.provinces[0].temporary_happiness, [25.0; 4]);
    assert!(integrate_province(&mut campaign, 0, 0).is_err());
    assert_eq!(campaign.economy.provinces[0].temporary_happiness, [25.0; 4]);
}
