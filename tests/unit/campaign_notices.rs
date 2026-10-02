use super::*;

#[test]
fn overview_filters_history_by_type_and_recipient() {
    let mut campaign = Campaign::default();
    for (player, kind, title) in [
        (0, NoticeKind::FoodShortage, "Food"),
        (0, NoticeKind::TreasuryExhausted, "Treasury"),
        (0, NoticeKind::TradeInterrupted, "Trade"),
        (0, NoticeKind::BuildingCompleted, "Building"),
        (0, NoticeKind::BattleResolved, "Battle"),
        (0, NoticeKind::SpyDetected, "Spy"),
        (0, NoticeKind::SpyWithdrawn, "Withdrawn spy"),
        (0, NoticeKind::ScandalDiscovered, "Scandal"),
        (0, NoticeKind::ForeignUnrest, "Unrest"),
        (1, NoticeKind::BuildingCompleted, "Other player"),
    ] {
        campaign.notifications.province_notice(
            player,
            if title == "Battle" {
                1
            } else {
                0
            },
            0,
            NoticeSeverity::Info,
            kind,
            title,
            "Details",
        );
    }
    campaign.notifications.push(CampaignNotice {
        id: 0,
        recipient: 0,
        severity: NoticeSeverity::Info,
        title: "Senate".into(),
        body: "Details".into(),
        kind: NoticeKind::SenateOfficeAppointed,
        province: None,
        building: None,
        wonder: None,
        scandal: None,
        month: 1,
        action: super::super::campaign_notifications::NoticeAction::OpenSenate,
    });
    let mut filters = NoticeFilters::default();
    let titles = |filters: &NoticeFilters| {
        filtered_history(&campaign, 0, filters)
            .map(|notice| notice.title.clone())
            .collect::<Vec<_>>()
    };
    let all = [
        "Senate",
        "Unrest",
        "Scandal",
        "Withdrawn spy",
        "Spy",
        "Battle",
        "Building",
        "Trade",
        "Treasury",
        "Food",
    ];
    assert_eq!(titles(&filters), all);
    filters.enabled[2] = false;
    assert_eq!(
        titles(&filters),
        [
            "Senate",
            "Unrest",
            "Scandal",
            "Withdrawn spy",
            "Spy",
            "Building",
            "Trade",
            "Treasury",
            "Food",
        ]
    );
    for (index, expected) in [
        vec!["Trade", "Treasury", "Food"],
        vec!["Building"],
        vec!["Battle"],
        vec!["Senate", "Unrest", "Scandal", "Withdrawn spy", "Spy"],
    ]
    .into_iter()
    .enumerate()
    {
        filters.enabled.fill(false);
        filters.enabled[index] = true;
        assert_eq!(titles(&filters), expected);
    }
    filters.enabled.fill(false);
    assert!(titles(&filters).is_empty());
    filters.enabled.fill(true);
    assert_eq!(titles(&filters), all);
}
