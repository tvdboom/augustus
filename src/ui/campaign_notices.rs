//! Shared cards and destinations for live campaign notices and province history.

use super::campaign_notifications::{CampaignNotice, NoticeKind, NoticeSeverity};
use super::campaign_widgets::{paint_icon, Icon};
use super::province_panel::{INK, RULE, TABLE_STRIPE};
use bevy_egui::egui;

pub(in crate::app) fn symbol(kind: NoticeKind) -> Icon {
    use NoticeKind::*;
    match kind {
        FoodShortage => Icon::Food,
        BuildingCompleted | WonderStarted | WonderCompleted => Icon::Construction,
        TradeInterrupted => Icon::Trade,
        RecruitmentCompleted
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
        | SlaveRevolt
        | MilitaryRankIncreased => Icon::Attack,
        SenateOfficeAppointed | ConsulRemoved | ConsulTermExpired | AugustusVictory => Icon::Eagle,
        SpyDetected | SpyWithdrawn | ScandalDiscovered => Icon::Spy,
        ForeignUnrest => Icon::Happiness,
        ControlFifty | ControlFull | VassalWeakened | HostileRelation | VeryHostileRelation
        | VassalRelationDecay => Icon::Diplomacy,
    }
}

pub(in crate::app) fn province_section(kind: NoticeKind) -> usize {
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
