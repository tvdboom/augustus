//! Shared cards and destinations for live campaign notices and province history.

use super::campaign::Campaign;
use super::campaign_notifications::{CampaignNotice, NoticeKind, NoticeSeverity};
use super::campaign_widgets::{paint_icon, portrait, texture, Icon, ProvinceLandscape};
use super::province_panel::{INK, RULE, TABLE_STRIPE};
use bevy_egui::egui;

const FILTER_CATEGORIES: [(&str, Icon); 4] = [
    ("Resources", Icon::Food),
    ("Buildings", Icon::Construction),
    ("Military", Icon::Attack),
    ("Diplomacy", Icon::Diplomacy),
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
        | ScandalDiscovered
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
            ui.spacing_mut().item_spacing.y = 8.0 * scale;
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
    filters: &mut NoticeFilters,
    scale: f32,
) -> Option<CampaignNotice> {
    filtered_list(ui, filters, scale, ("campaign_notices", player), |ui, filters| {
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
        SpyDetected | SpyWithdrawn | ScandalDiscovered => Icon::Spy,
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
    pub warning: bool,
    pub critical: bool,
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
    let body = (!message.body.is_empty()).then(|| {
        ui.painter().layout(
            message.body.to_owned(),
            egui::FontId::proportional(12.0 * scale),
            INK,
            (width - 2.0 * padding).max(1.0),
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
    let accent = if message.critical {
        egui::Color32::from_rgb(176, 45, 35)
    } else if message.warning {
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
    if let Some(body) = body {
        ui.painter().galley(
            rect.min + egui::vec2(padding, padding + header_height + 6.0 * scale),
            body,
            INK,
        );
    }
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

#[cfg(test)]
#[path = "../../tests/unit/campaign_notices.rs"]
mod tests;
