use super::*;

#[path = "egui_capture.rs"]
mod revolt_capture;

#[test]
fn slave_revolt_toasts_are_critical_illustrated_and_navigate_to_the_province() {
    let mut campaign = super::super::campaign::Campaign::default();
    campaign.notifications.province_notice(
        0,
        3,
        7,
        NoticeSeverity::Warning,
        NoticeKind::SlaveRevolt,
        "Slave revolt in Africa Proconsularis",
        "All 30000 slaves have risen and formed 1000 light infantry cohorts. The rebel light infantry is fighting your army.",
    );
    let notice = campaign.notifications.drain_for(0).pop().unwrap();
    let context = egui::Context::default();
    let mut output = context.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(420., 180.))),
            ..Default::default()
        },
        |ui| {
            super::super::campaign_notices::card(ui, &notice, 1.0);
        },
    );
    revolt_capture::Capture::default().frame(&context, &output, "slave-revolt-alert");
    output.textures_delta.clear();
    assert!(output
        .shapes
        .iter()
        .any(|shape| matches!(&shape.shape, egui::Shape::LineSegment { stroke, .. }
            if stroke.color == egui::Color32::from_rgb(176, 45, 35))));
    assert_eq!(
        super::super::campaign_notices::notice_symbol(&notice),
        super::super::campaign_widgets::Icon::Slaves
    );
    assert_eq!(super::super::campaign_notices::province_section(notice.kind), 3);
    let toast = Toast::from_notice(notice);
    assert_eq!(toast.level, ToastLevel::Error);
    assert_eq!(toast.action, Some(ToastAction::OpenProvince(3)));
    assert!(toast.seconds_left > TOAST_SECONDS);
    assert!(toast.text.contains("Africa Proconsularis"));
    let mut queue = ToastQueue::default();
    queue.push(toast);
    assert_eq!(queue.1, [false, false, true]);
}

#[test]
fn a_slave_revolt_alert_survives_a_batch_of_routine_notices() {
    let mut campaign = super::super::campaign::Campaign::default();
    campaign.notifications.province_notice(
        0,
        3,
        7,
        NoticeSeverity::Warning,
        NoticeKind::SlaveRevolt,
        "Slave revolt in Africa Proconsularis",
        "A hostile rebel army now stands in the province.",
    );
    let mut queue = ToastQueue::default();
    queue.push(Toast::from_notice(campaign.notifications.drain_for(0).pop().unwrap()));
    for _ in 0..MAX_TOASTS * 2 {
        queue.push(Toast::info("Construction completed"));
        queue.push(Toast::warning("Food stores low"));
    }
    assert_eq!(queue.0.len(), MAX_TOASTS);
    assert_eq!(queue.0[0].level, ToastLevel::Error);
    assert_eq!(queue.0[0].action, Some(ToastAction::OpenProvince(3)));
}

fn resource(amount: f64, monthly_delta: f64) -> HudResource {
    HudResource {
        amount,
        monthly_delta,
    }
}

#[test]
fn happiness_warnings_use_strict_class_thresholds() {
    let mut happiness = [resource(50.0, 0.0); 4];
    let resources = [resource(100.0, 1.0); 7];
    happiness[0].amount = 40.0;
    happiness[1].amount = 30.0;
    happiness[2].amount = 20.0;
    happiness[3].amount = 10.0;
    assert_eq!(warning_conditions(resources, happiness), [false; 6]);
    for class in 0..4 {
        happiness[class].amount -= 0.1;
    }
    assert_eq!(warning_conditions(resources, happiness), [true, true, true, true, false, false]);
}

#[test]
fn shortages_use_three_month_forecast_and_require_nonpositive_net() {
    let happiness = [resource(50.0, 0.0); 4];
    let mut resources = [resource(100.0, 1.0); 7];
    resources[0] = resource(30.0, -10.0);
    resources[3] = resource(24.0, 0.0);
    assert_eq!(warning_conditions(resources, happiness)[4..], [true, true]);
    resources[0].amount = 30.1;
    resources[3].monthly_delta = 1.0;
    assert_eq!(warning_conditions(resources, happiness)[4..], [false, false]);
}

#[test]
fn warning_only_repeats_after_recovery() {
    let mut watch = WarningWatch::default();
    let mut toasts = ToastQueue::default();
    let mut campaign = super::super::campaign::Campaign::default();
    let mut resources = [resource(100.0, 1.0); 7];
    let happiness = [resource(50.0, 0.0); 4];
    resources[0] = resource(9.0, 0.0);
    watch.observe(0, resources, happiness, &mut campaign, &mut toasts);
    assert_eq!(toasts.0.len(), 2); // Welcome info and food warning.
    watch.observe(0, resources, happiness, &mut campaign, &mut toasts);
    assert_eq!(toasts.0.len(), 2);
    resources[0].amount = 20.0;
    watch.observe(0, resources, happiness, &mut campaign, &mut toasts);
    resources[0].amount = 9.0;
    watch.observe(0, resources, happiness, &mut campaign, &mut toasts);
    assert_eq!(toasts.0.len(), 3);
}

#[test]
fn unhappy_classes_record_the_source_province_for_navigation_and_history() {
    use crate::game::economy::{EconomicProvince, EconomyWorld, Terrain};

    let mut campaign = super::super::campaign::Campaign::default();
    let mut provinces: Vec<_> = ["Mild", "Unhappy", "Foreign"]
        .into_iter()
        .map(|name| {
            EconomicProvince::new(name, 40.0, Terrain::Farmland, true, [1.0; 3], [10.0; 4], 2)
        })
        .collect();
    provinces[0].owner = Some(0);
    provinces[0].happiness[0] = 38.0;
    provinces[0].happiness[1] = 28.0;
    provinces[1].owner = Some(0);
    provinces[1].happiness[0] = 20.0;
    provinces[1].happiness[1] = 25.0;
    provinces[2].owner = Some(1);
    provinces[2].happiness[0] = 1.0;
    campaign.economy = EconomyWorld::new(2, provinces, vec![vec![], vec![], vec![]]);

    let mut watch = WarningWatch::default();
    let mut toasts = ToastQueue::default();
    let resources = [resource(100.0, 1.0); 7];
    let happiness =
        std::array::from_fn(|class| resource(campaign.economy.player_happiness(0, class).0, 0.0));
    watch.observe(0, resources, happiness, &mut campaign, &mut toasts);

    let notices = campaign.notifications.history_for(0).collect::<Vec<_>>();
    assert_eq!(notices.len(), 2);
    assert!(notices.iter().all(|notice| {
        notice.province == Some(1)
            && notice.action == super::super::campaign_notifications::NoticeAction::OpenProvince(1)
            && notice.body.contains("Unhappy")
    }));
    assert!(notices.iter().any(|notice| notice.kind == NoticeKind::PopulationUnhappy(0)));
    assert!(notices.iter().any(|notice| notice.kind == NoticeKind::PopulationUnhappy(1)));
    assert_eq!(campaign.notifications.drain_for(0).len(), 2);
    watch.observe(0, resources, happiness, &mut campaign, &mut toasts);
    assert!(campaign.notifications.drain_for(0).is_empty());
}

#[test]
fn queued_toasts_request_one_sound_per_severity() {
    let mut toasts = ToastQueue::default();
    toasts.push(Toast::warning("First warning"));
    toasts.push(Toast::warning("Second warning"));
    toasts.push(Toast::info("Information"));
    toasts.push(Toast::error("Failure"));
    assert_eq!(toasts.1, [true, true, true]);
    toasts.clear();
    assert_eq!(toasts.1, [false; 3]);
}

#[test]
fn switching_viewers_keeps_newly_delivered_notices_through_the_warning_watch() {
    let mut watch = WarningWatch::default();
    let mut toasts = ToastQueue::default();
    let mut campaign = super::super::campaign::Campaign::default();
    let resources = [resource(100.0, 1.0); 7];
    let happiness = [resource(50.0, 0.0); 4];
    watch.observe(0, resources, happiness, &mut campaign, &mut toasts);
    toasts.set_player(1);
    toasts.push(Toast::info("Player 1 promoted to Tribune"));
    watch.observe(1, resources, happiness, &mut campaign, &mut toasts);
    assert_eq!(toasts.0.len(), 1);
    assert_eq!(toasts.0[0].text, "Player 1 promoted to Tribune");
}

#[test]
fn event_toast_has_its_own_title_without_requesting_the_message_chime() {
    let mut toasts = ToastQueue::default();
    toasts.push(Toast::info("Games held").with_title("Event held").without_sound());
    assert_eq!(toasts.1, [false; 3]);
    assert_eq!(toasts.0[0].message(1).title, "Event held");
}
