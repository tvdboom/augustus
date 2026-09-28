use super::*;

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
fn political_preview_prices_distance_without_spending_or_queuing_pressure() {
    let province = ProvincePolitics::independent(1);
    let actor = PoliticalPlayer {
        coin: 100.0,
        ..Default::default()
    };
    let config = DiplomacyConfig::default();
    let quote = political_quote(&province, &actor, Currency::Coin, |p, a| {
        p.improve_relation(0, a, Currency::Coin, 5.0, Some(4), &config)
    })
    .unwrap();
    assert!((quote.cost - 150.0).abs() < 0.000_001);
    assert!(!quote.affordable);
    assert_eq!(actor.coin, 100.0);
    assert_eq!(province.relation(0), 50.0);
    assert!(political_quote(&province, &actor, Currency::Coin, |p, a| p.improve_relation(
        0,
        a,
        Currency::Coin,
        5.0,
        None,
        &config
    ))
    .is_err());
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
    integrate_province(&mut campaign, 0, 0).unwrap();
    assert_eq!(campaign.economy.provinces[0].temporary_happiness, [25.0; 4]);
    assert!(integrate_province(&mut campaign, 0, 0).is_err());
    assert_eq!(campaign.economy.provinces[0].temporary_happiness, [25.0; 4]);
}
