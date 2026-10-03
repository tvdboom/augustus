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
        vec!["Senate", "Unrest", "Withdrawn spy", "Spy"],
        vec!["Scandal"],
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

#[test]
fn scandal_notification_rows_match_the_filter_icon_and_keep_player_colors_after_expiry() {
    for width in [546.0, 380.0, 320.0] {
        let ctx = egui::Context::default();
        let mut campaign = Campaign::default();
        campaign.notifications.remember_scandal_target(3, ScandalTarget::Player(1));
        for (kind, title, body, month) in [
            (
                NoticeKind::ScandalExpired,
                "Scandal expired · Corrupt Governor",
                "Severity III · grave · Valid for 120 months.",
                120,
            ),
            (
                NoticeKind::ScandalDiscovered,
                "Scandal discovered · Corrupt Governor",
                "Against Player 2 · Severity III · grave · 120 months remaining at discovery.",
                0,
            ),
        ] {
            campaign.notifications.push(CampaignNotice {
                id: 0,
                recipient: 0,
                severity: NoticeSeverity::Info,
                title: title.into(),
                body: body.into(),
                kind,
                province: None,
                building: None,
                wonder: None,
                scandal: Some(3),
                month,
                action: super::super::campaign_notifications::NoticeAction::OpenScandal {
                    scandal: 3,
                    province: None,
                },
            });
        }
        let mut bounds = egui::Rect::NOTHING;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width + 16.0, 700.0),
                )),
                ..Default::default()
            },
            |ui| {
                *ui.style_mut() = super::super::campaign_widgets::map_style(1.0);
                ui.set_width(width);
                overview(
                    ui,
                    &campaign,
                    0,
                    &super::super::PLAYER_COLORS,
                    &mut NoticeFilters::default(),
                    1.0,
                );
                bounds = ui.min_rect();
            },
        );
        crate::egui_capture::Capture::default().frame(
            &ctx,
            &output,
            &format!("scandal-notifications-{}", width as u32),
        );
        assert!(
            bounds.width() <= width + 1.0,
            "Notification rows overflow at {width}px: {bounds:?}"
        );
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(rect)
            if rect.fill == super::super::PLAYER_COLORS[1] && rect.rect.width() == 6.0 && rect.rect.height() == 34.0)));
        assert_eq!(
            notice_symbol(campaign.notifications.history_for(0).next().unwrap()),
            FILTER_CATEGORIES[4].1
        );
        output.textures_delta.clear();
    }
}
