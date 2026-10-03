//! Shared cards and destinations for live campaign notices and province history.

use super::campaign::Campaign;
#[cfg(test)]
use super::campaign_notifications::NoticeSeverity;
use super::campaign_notifications::{CampaignNotice, NoticeKind};
use super::campaign_widgets::{
    paint_directory_row, paint_icon, portrait, texture, Icon, ProvinceLandscape,
};
use super::province_panel::{INK, NEUTRAL};
use crate::game::politics::espionage::ScandalTarget;
use bevy_egui::egui;

const FILTER_CATEGORIES: [(&str, Icon); 5] = [
    ("Resources", Icon::Food),
    ("Buildings", Icon::Construction),
    ("Military", Icon::Attack),
    ("Diplomacy", Icon::Diplomacy),
    ("Scandals", Icon::SpyUncoverScandals),
];

/// Notification filters are local presentation state; each view starts with all types visible.
#[derive(Clone, Copy)]
pub(in crate::app) struct NoticeFilters {
    enabled: [bool; FILTER_CATEGORIES.len()],
}

impl Default for NoticeFilters {
    fn default() -> Self {
        Self {
            enabled: [true; FILTER_CATEGORIES.len()],
        }
    }
}

fn category(kind: NoticeKind) -> usize {
    use NoticeKind::*;
    match kind {
        FoodShortage | TreasuryExhausted | TradeInterrupted | PopulationUnhappy(_) => 0,
        BuildingCompleted | WonderStarted | WonderCompleted => 1,
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
        | MilitaryPromotionAvailable
        | SlaveRevolt => 2,
        SpyDetected
        | SpyWithdrawn
        | SenatorBriberyExposed
        | ForeignUnrest
        | SenateOfficeAppointed
        | PoliticalPromotionAvailable
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
        | VassalRelationDecay => 3,
        ScandalDiscovered | ScandalExpired => 4,
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

/// Overlay filters on the banner with the political-distance badge's dark treatment.
fn filters_banner(ui: &mut egui::Ui, filters: &mut NoticeFilters, scale: f32) {
    let banner = portrait(ui, ProvinceLandscape::Notifications, 76.0 * scale);
    let margin = 7.0 * scale;
    let rect = egui::Rect::from_min_max(
        egui::pos2(banner.left() + margin, banner.bottom() - margin - 32.0 * scale),
        banner.right_bottom() - egui::Vec2::splat(margin),
    );
    let mut overlay = ui.new_child(
        egui::UiBuilder::new()
            .id_salt("notification-filters")
            .max_rect(rect)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    overlay.set_clip_rect(ui.clip_rect().intersect(banner));
    egui::Frame::new()
        .fill(egui::Color32::from_black_alpha(180))
        .corner_radius(3.0 * scale)
        .inner_margin(egui::Margin::symmetric(
            (6.0 * scale).round() as i8,
            (3.0 * scale).round() as i8,
        ))
        .show(&mut overlay, |ui| {
            ui.spacing_mut().item_spacing.x = 12.0 * scale;
            ui.spacing_mut().icon_spacing = 3.0 * scale;
            ui.spacing_mut().icon_width = 14.0 * scale;
            ui.spacing_mut().icon_width_inner = 8.0 * scale;
            ui.spacing_mut().interact_size.y = 24.0 * scale;
            ui.visuals_mut().override_text_color = Some(egui::Color32::WHITE);
            let widgets = &mut ui.visuals_mut().widgets;
            for widget in [&mut widgets.inactive, &mut widgets.hovered, &mut widgets.active] {
                widget.fg_stroke = egui::Stroke::new(scale, egui::Color32::WHITE);
                widget.bg_stroke = egui::Stroke::new(scale, egui::Color32::from_white_alpha(180));
                widget.bg_fill = egui::Color32::from_white_alpha(20);
            }
            widgets.hovered.bg_fill = egui::Color32::from_white_alpha(50);
            widgets.active.bg_fill = egui::Color32::from_white_alpha(70);
            for (index, &(name, artwork)) in FILTER_CATEGORIES.iter().enumerate().rev() {
                let image =
                    egui::Image::new((texture(ui.ctx(), artwork), egui::vec2(24.0, 24.0) * scale))
                        .alt_text(name);
                let response = ui.add(egui::Checkbox::new(&mut filters.enabled[index], image));
                response.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::Checkbox,
                        ui.is_enabled(),
                        filters.enabled[index],
                        name,
                    )
                });
                let tooltip = match index {
                    0 => "Resources, population, and trade",
                    3 => "Diplomacy, spies, and Senate",
                    _ => name,
                };
                response.on_hover_text(tooltip).on_hover_cursor(egui::CursorIcon::PointingHand);
            }
        });
}

/// Keep the banner and its filters fixed while only the notice cards scroll.
pub(super) fn filtered_list<R>(
    ui: &mut egui::Ui,
    filters: &mut NoticeFilters,
    scale: f32,
    id: impl std::hash::Hash + std::fmt::Debug,
    render: impl FnOnce(&mut egui::Ui, &NoticeFilters) -> R,
) -> R {
    filters_banner(ui, filters, scale);
    egui::ScrollArea::vertical()
        .id_salt(id)
        .max_height(ui.available_height())
        .min_scrolled_height(0.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 3.0 * scale;
            render(ui, filters)
        })
        .inner
}

pub(super) fn allows(filters: &NoticeFilters, notice: &CampaignNotice) -> bool {
    filters.enabled[category(notice.kind)]
}

/// Show all notices received by this player, most recent first, using province history cards.
pub(in crate::app) fn overview(
    ui: &mut egui::Ui,
    campaign: &Campaign,
    player: usize,
    colors: &[egui::Color32],
    filters: &mut NoticeFilters,
    scale: f32,
) -> Option<CampaignNotice> {
    filtered_list(ui, filters, scale, ("campaign_notices", player), |ui, filters| {
        let mut navigation = None;
        let mut any = false;
        for (row, notice) in filtered_history(campaign, player, filters).enumerate() {
            any = true;
            ui.push_id(notice.id, |ui| {
                if card(ui, notice, campaign, colors, row, scale).clicked() {
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
    })
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
        | MilitaryRankIncreased
        | MilitaryPromotionAvailable => Icon::Attack,
        SenateOfficeAppointed
        | PoliticalPromotionAvailable
        | ConsulRemoved
        | ConsulTermExpired
        | AugustusVictory
        | PlayerDefeated => Icon::Eagle,
        SpyDetected | SpyWithdrawn => Icon::Spy,
        ScandalDiscovered | ScandalExpired => Icon::SpyUncoverScandals,
        SenatorBriberyExposed => Icon::SenatorBribe,
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
    if notice.kind == NoticeKind::SlaveRevolt {
        Icon::Slaves
    } else if let Some(building) = notice.building {
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
    campaign: &Campaign,
    colors: &[egui::Color32],
    row: usize,
    scale: f32,
) -> egui::Response {
    let subject = notice
        .scandal
        .and_then(|id| {
            campaign.espionage.scandals.iter().find(|s| s.id == id && s.holder == notice.recipient)
        })
        .map(|s| s.target)
        .or_else(|| notice.scandal.and_then(|id| campaign.notifications.scandal_target(id)))
        .or_else(|| notice.province.map(ScandalTarget::Province))
        .unwrap_or(ScandalTarget::Player(notice.recipient));
    let owner = match subject {
        ScandalTarget::Player(player) => Some(player),
        ScandalTarget::Province(province) => {
            campaign.economy.provinces.get(province).and_then(|p| p.owner)
        },
    };
    let owner_label = owner
        .map_or_else(|| "Independent province".into(), |player| format!("Player {}", player + 1));
    let owner_color = owner.and_then(|player| colors.get(player).copied()).unwrap_or(NEUTRAL);
    message_card(
        ui,
        Message {
            icon: notice_symbol(notice),
            title: &notice.title,
            body: &notice.body,
            month: notice.month,
            owner: Some((owner_color, &owner_label)),
            row,
            critical: notice.kind == NoticeKind::SlaveRevolt,
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
    pub owner: Option<(egui::Color32, &'a str)>,
    pub row: usize,
    pub critical: bool,
    pub actionable: bool,
}

pub(in crate::app) fn message_card(
    ui: &mut egui::Ui,
    message: Message<'_>,
    scale: f32,
) -> egui::Response {
    let width = ui.available_width();
    let padding = 7.0 * scale;
    let content_left = if message.owner.is_some() {
        21.0
    } else {
        7.0
    } * scale;
    let date_width = 82.0 * scale;
    let text_left = content_left + 36.0 * scale;
    let title_width = (width - text_left - date_width - padding).max(1.0);
    let title_color = if message.critical {
        egui::Color32::from_rgb(176, 45, 35)
    } else {
        INK
    };
    let title = ui.painter().layout(
        message.title.to_owned(),
        egui::FontId::proportional(14.0 * scale),
        title_color,
        title_width,
    );
    let body = (!message.body.is_empty()).then(|| {
        ui.painter().layout(
            message.body.to_owned(),
            egui::FontId::proportional(12.0 * scale),
            INK,
            (width - text_left - padding).max(1.0),
        )
    });
    let header_height = title.size().y.max(30.0 * scale);
    let height = 2.0 * padding
        + header_height
        + body.as_ref().map_or(0.0, |body| 6.0 * scale + body.size().y);
    let sense = if message.actionable {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), sense);
    paint_directory_row(ui, rect, &response, message.row, scale);
    if let Some((color, label)) = message.owner {
        let marker = egui::Rect::from_center_size(
            egui::pos2(rect.left() + 10.0 * scale, rect.center().y),
            egui::vec2(6.0 * scale, 34.0 * scale),
        );
        ui.painter().rect_filled(marker, 2.0 * scale, color);
        ui.interact(marker, response.id.with("owner"), egui::Sense::hover()).on_hover_text(label);
    }
    let icon_rect = egui::Rect::from_min_size(
        rect.min + egui::vec2(content_left, padding),
        egui::vec2(30.0, 30.0) * scale,
    );
    paint_icon(ui, message.icon, icon_rect);
    ui.painter().galley(
        rect.min + egui::vec2(text_left, padding + (header_height - title.size().y) * 0.5),
        title,
        title_color,
    );
    ui.painter().text(
        rect.right_top() + egui::vec2(-padding, padding + 4.0 * scale),
        egui::Align2::RIGHT_TOP,
        date(message.month),
        egui::FontId::proportional(10.0 * scale),
        egui::Color32::from_rgb(112, 91, 71),
    );
    if let Some(body) = body {
        ui.painter().galley(
            rect.min + egui::vec2(text_left, padding + header_height + 6.0 * scale),
            body,
            INK,
        );
    }
    if message.actionable {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    }
}

#[cfg(test)]
#[path = "../../tests/unit/campaign_notices.rs"]
mod tests;
