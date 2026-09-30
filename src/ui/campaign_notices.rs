//! Shared cards and destinations for live campaign notices and province history.

use super::campaign::Campaign;
use super::campaign_notifications::{CampaignNotice, NoticeKind, NoticeSeverity};
use super::campaign_widgets::{icon, paint_icon, Icon};
use super::province_panel::{INK, RULE, TABLE_STRIPE};
use bevy_egui::egui;

const FILTER_CATEGORIES: [(&str, Icon); 5] = [
    ("Resources", Icon::Coin),
    ("Spies", Icon::Spy),
    ("Military", Icon::Attack),
    ("Buildings", Icon::Construction),
    ("Politics", Icon::Diplomacy),
];

/// Notification filters are local presentation state; each view starts with all types visible.
#[derive(Clone, Copy)]
pub(in crate::app) struct NoticeFilters {
    enabled: [bool; 5],
}

impl Default for NoticeFilters {
    fn default() -> Self {
        Self {
            enabled: [true; 5],
        }
    }
}

fn category(kind: NoticeKind) -> usize {
    use NoticeKind::*;
    match kind {
        FoodShortage | TreasuryExhausted | TradeInterrupted | PopulationUnhappy(_) => 0,
        SpyDetected | SpyWithdrawn | ScandalDiscovered | ForeignUnrest => 1,
        RecruitmentCompleted
        | ArmyDisbanded
        | UnitsDestroyed
        | ForeignArrival
        | InvasionBegins
        | BattleResolved
        | NpcDefeated
        | OccupationEstablished
        | MilitaryAccessGranted
        | MilitaryAccessRevoked
        | MilitaryMovementStopped
        | GarrisonWeakened
        | MilitaryRankIncreased
        | SlaveRevolt => 2,
        BuildingCompleted | WonderStarted | WonderCompleted => 3,
        SenateOfficeAppointed
        | ConsulTermExpired
        | ConsulRemoved
        | AugustusVictory
        | PlayerDefeated
        | ControlFifty
        | ControlFull
        | OwnedControlThreatened
        | VassalWeakened
        | HostileRelation
        | VeryHostileRelation
        | VassalRelationDecay => 4,
    }
}

fn filtered_history<'a>(
    campaign: &'a Campaign,
    player: usize,
    filters: &'a NoticeFilters,
) -> impl Iterator<Item = &'a CampaignNotice> {
    campaign
        .notifications
        .history_for(player)
        .filter(move |notice| filters.enabled[category(notice.kind)])
}

pub(super) fn filters_row(ui: &mut egui::Ui, filters: &mut NoticeFilters, scale: f32) {
    ui.add_space(4.0 * scale);
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 3.0 * scale;
        for (index, &(name, artwork)) in FILTER_CATEGORIES.iter().enumerate().rev() {
            icon(ui, artwork, 18.0 * scale).on_hover_text(name);
            ui.checkbox(&mut filters.enabled[index], "").on_hover_text(name);
            if index > 0 {
                ui.add_space(17.0 * scale);
            }
        }
    });
    ui.separator();
}

pub(super) fn allows(filters: &NoticeFilters, notice: &CampaignNotice) -> bool {
    filters.enabled[category(notice.kind)]
}

/// Show all notices received by this player, most recent first, using province history cards.
pub(in crate::app) fn overview(
    ui: &mut egui::Ui,
    campaign: &Campaign,
    player: usize,
    filters: &mut NoticeFilters,
    scale: f32,
) -> Option<CampaignNotice> {
    filters_row(ui, filters, scale);
    ui.spacing_mut().item_spacing.y = 8.0 * scale;
    let mut navigation = None;
    let mut any = false;
    for notice in filtered_history(campaign, player, filters) {
        any = true;
        ui.push_id(notice.id, |ui| {
            if card(ui, notice, scale).clicked() {
                navigation = Some(notice.clone());
            }
        });
    }
    if !any {
        ui.add_space(8.0 * scale);
        ui.horizontal(|ui| {
            ui.add_space(12.0 * scale);
            ui.label("No notifications match these filters.");
        });
    }
    navigation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overview_filters_history_by_type_and_recipient() {
        let mut campaign = Campaign::default();
        for (player, kind, title) in [
            (0, NoticeKind::FoodShortage, "Food"),
            (0, NoticeKind::BattleResolved, "Battle"),
            (0, NoticeKind::SpyDetected, "Spy"),
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
        assert_eq!(titles(&filters), ["Senate", "Spy", "Battle", "Food"]);
        filters.enabled[2] = false;
        assert_eq!(titles(&filters), ["Senate", "Spy", "Food"]);
        filters.enabled.fill(false);
        assert!(titles(&filters).is_empty());
        filters.enabled.fill(true);
        assert_eq!(titles(&filters), ["Senate", "Spy", "Battle", "Food"]);
    }
}

pub(in crate::app) fn symbol(kind: NoticeKind) -> Icon {
    use NoticeKind::*;
    match kind {
        FoodShortage => Icon::Food,
        TreasuryExhausted => Icon::Coin,
        PopulationUnhappy(_) => Icon::Happiness,
        BuildingCompleted | WonderStarted | WonderCompleted => Icon::Construction,
        TradeInterrupted => Icon::Trade,
        RecruitmentCompleted
        | UnitsDestroyed
        | ArmyDisbanded
        | ForeignArrival
        | InvasionBegins
        | BattleResolved
        | NpcDefeated
        | OccupationEstablished
        | MilitaryAccessGranted
        | MilitaryAccessRevoked
        | MilitaryMovementStopped
        | GarrisonWeakened
        | SlaveRevolt
        | MilitaryRankIncreased => Icon::Attack,
        SenateOfficeAppointed
        | ConsulRemoved
        | ConsulTermExpired
        | AugustusVictory
        | PlayerDefeated => Icon::Eagle,
        SpyDetected | SpyWithdrawn | ScandalDiscovered => Icon::Spy,
        ForeignUnrest => Icon::Happiness,
        ControlFifty
        | ControlFull
        | OwnedControlThreatened
        | VassalWeakened
        | HostileRelation
        | VeryHostileRelation
        | VassalRelationDecay => Icon::Diplomacy,
    }
}

pub(in crate::app) fn province_section(kind: NoticeKind) -> usize {
    if matches!(
        kind,
        NoticeKind::FoodShortage
            | NoticeKind::OwnedControlThreatened
            | NoticeKind::PopulationUnhappy(_)
    ) {
        return 0;
    }
    match symbol(kind) {
        Icon::Food => 1,
        Icon::Construction => 2,
        Icon::Attack => 3,
        Icon::Trade => 5,
        _ => 4,
    }
}

/// Retain the event's original artwork even after its project has finished.
pub(in crate::app) fn notice_symbol(notice: &CampaignNotice) -> Icon {
    if let Some(building) = notice.building {
        Icon::Building(building)
    } else if let Some(wonder) = notice.wonder {
        Icon::Wonder(wonder)
    } else {
        symbol(notice.kind)
    }
}

/// The campaign begins in January 60 AD, matching the map clock.
fn date(month: u32) -> String {
    let months =
        ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    format!("{} {} AD", months[(month % 12) as usize], 60 + month / 12)
}

/// The entire card is actionable; body text wraps independently of the date and title.
pub(in crate::app) fn card(
    ui: &mut egui::Ui,
    notice: &CampaignNotice,
    scale: f32,
) -> egui::Response {
    message_card(
        ui,
        Message {
            icon: notice_symbol(notice),
            title: &notice.title,
            body: &notice.body,
            month: notice.month,
            warning: notice.severity == NoticeSeverity::Warning,
            actionable: true,
        },
        scale,
    )
}

/// Common presentation also covers HUD warnings without a domain history entry.
pub(in crate::app) struct Message<'a> {
    pub icon: Icon,
    pub title: &'a str,
    pub body: &'a str,
    pub month: u32,
    pub warning: bool,
    pub actionable: bool,
}

pub(in crate::app) fn message_card(
    ui: &mut egui::Ui,
    message: Message<'_>,
    scale: f32,
) -> egui::Response {
    let width = ui.available_width();
    let padding = 10.0 * scale;
    let date_width = 82.0 * scale;
    let title_width = (width - 52.0 * scale - date_width - 2.0 * padding).max(1.0);
    let title = ui.painter().layout(
        message.title.to_owned(),
        egui::FontId::proportional(14.0 * scale),
        INK,
        title_width,
    );
    let body = ui.painter().layout(
        message.body.to_owned(),
        egui::FontId::proportional(12.0 * scale),
        INK,
        (width - 2.0 * padding).max(1.0),
    );
    let header_height = title.size().y.max(30.0 * scale);
    let height = 2.0 * padding + header_height + 6.0 * scale + body.size().y;
    let sense = if message.actionable {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), sense);
    let accent = if message.warning {
        egui::Color32::from_rgb(176, 117, 49)
    } else {
        RULE
    };
    ui.painter().rect_filled(
        rect,
        4.0 * scale,
        if response.hovered() {
            TABLE_STRIPE
        } else {
            egui::Color32::from_rgb(247, 243, 232)
        },
    );
    ui.painter().rect_stroke(
        rect,
        4.0 * scale,
        egui::Stroke::new(
            scale,
            if response.hovered() {
                accent
            } else {
                RULE
            },
        ),
        egui::StrokeKind::Inside,
    );
    let icon_rect = egui::Rect::from_min_size(
        rect.min + egui::vec2(padding, padding),
        egui::vec2(30.0, 30.0) * scale,
    );
    paint_icon(ui, message.icon, icon_rect);
    ui.painter().galley(
        rect.min + egui::vec2(48.0 * scale, padding + (header_height - title.size().y) * 0.5),
        title,
        INK,
    );
    ui.painter().text(
        rect.right_top() + egui::vec2(-padding, padding + 4.0 * scale),
        egui::Align2::RIGHT_TOP,
        date(message.month),
        egui::FontId::proportional(10.0 * scale),
        egui::Color32::from_rgb(112, 91, 71),
    );
    ui.painter().galley(
        rect.min + egui::vec2(padding, padding + header_height + 6.0 * scale),
        body,
        INK,
    );
    // A thin inner accent marks severity without competing with the type icon.
    ui.painter().line_segment(
        [
            rect.left_top() + egui::vec2(1.5 * scale, 6.0 * scale),
            rect.left_bottom() + egui::vec2(1.5 * scale, -6.0 * scale),
        ],
        egui::Stroke::new(2.0 * scale, accent),
    );
    if message.actionable {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    }
}
