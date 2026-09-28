use super::*;

#[test]
fn notices_are_private_deduplicated_and_kept_in_history() {
    let mut notices = CampaignNotifications::default();
    for body in ["First cause", "All causes"] {
        notices.province_notice(
            1,
            2,
            3,
            NoticeSeverity::Warning,
            NoticeKind::VassalWeakened,
            "Vassal",
            body,
        );
    }
    assert!(notices.drain_for(0).is_empty());
    let delivered = notices.drain_for(1);
    assert_eq!(delivered.len(), 1);
    assert_eq!(delivered[0].body, "All causes");
    assert_eq!(notices.history_for(1).count(), 1);
    assert!(notices.drain_for(1).is_empty());
}

#[test]
fn province_history_retains_notices_beyond_the_transient_queue_limit() {
    let mut notices = CampaignNotifications::default();
    for month in 1..=250 {
        notices.province_notice(
            0,
            2,
            month,
            NoticeSeverity::Info,
            NoticeKind::RecruitmentCompleted,
            "Recruitment complete",
            "A new cohort is ready.",
        );
    }
    assert_eq!(notices.history_for(0).count(), 250);
    assert_eq!(notices.history_for(1).count(), 0);
    assert_eq!(notices.history_for(0).last().unwrap().month, 1);
}
