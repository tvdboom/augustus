//! Available evidence, ordered by severity with local target and severity filters.

use super::campaign::Campaign;
use super::campaign_notices::{message_card, Message};
use super::campaign_widgets::Icon;
use crate::game::politics::espionage::{Scandal, ScandalTarget, Severity};
use bevy_egui::egui;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum TargetFilter {
    #[default]
    All,
    Player(usize),
    Provinces,
}

impl TargetFilter {
    fn label(self) -> String {
        match self {
            Self::All => "All targets".into(),
            Self::Player(player) => format!("Player {}", player + 1),
            Self::Provinces => "Independent provinces".into(),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct ScandalFilters {
    severity: [bool; 3],
    target: TargetFilter,
}

impl Default for ScandalFilters {
    fn default() -> Self {
        Self {
            severity: [true; 3],
            target: TargetFilter::All,
        }
    }
}

fn severity_index(severity: Severity) -> usize {
    match severity {
        Severity::Minor => 0,
        Severity::Medium => 1,
        Severity::Major => 2,
    }
}

fn available(campaign: &Campaign, player: usize) -> impl Iterator<Item = &Scandal> {
    campaign.espionage.scandals.iter().filter(move |scandal| {
        scandal.holder == player
            && scandal.expires > campaign.senate.month
            && !scandal.reserved_for_motion
    })
}

fn filtered_evidence<'a>(
    campaign: &'a Campaign,
    player: usize,
    filters: &ScandalFilters,
) -> Vec<&'a Scandal> {
    let mut scandals: Vec<_> = available(campaign, player)
        .filter(|scandal| {
            filters.severity[severity_index(scandal.severity)]
                && match filters.target {
                    TargetFilter::All => true,
                    TargetFilter::Player(player) => scandal.target == ScandalTarget::Player(player),
                    TargetFilter::Provinces => matches!(scandal.target, ScandalTarget::Province(_)),
                }
        })
        .collect();
    scandals.sort_by_key(|scandal| {
        (
            std::cmp::Reverse(severity_index(scandal.severity)),
            std::cmp::Reverse(scandal.acquired),
            scandal.id,
        )
    });
    scandals
}

fn filters_row(ui: &mut egui::Ui, campaign: &Campaign, filters: &mut ScandalFilters, scale: f32) {
    ui.add_space(4.0 * scale);
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), ui.spacing().interact_size.y),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = 8.0 * scale;
            ui.spacing_mut().icon_spacing = 3.0 * scale;
            // Keep both the pressed and open dropdown fills on parchment.
            ui.visuals_mut().widgets.open = ui.visuals().widgets.hovered;
            ui.visuals_mut().widgets.active.weak_bg_fill =
                ui.visuals().widgets.hovered.weak_bg_fill;
            egui::ComboBox::from_id_salt("scandal-target")
                .selected_text(filters.target.label())
                .width(140.0 * scale)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut filters.target, TargetFilter::All, "All targets");
                    for player in 0..campaign.actors.len() {
                        ui.selectable_value(
                            &mut filters.target,
                            TargetFilter::Player(player),
                            format!("Player {}", player + 1),
                        );
                    }
                    ui.selectable_value(
                        &mut filters.target,
                        TargetFilter::Provinces,
                        "Independent provinces",
                    );
                })
                .response
                .on_hover_text(
                    "Filter by the player or independent government the evidence is against.",
                );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                for (severity, label) in
                    [(Severity::Minor, "I"), (Severity::Medium, "II"), (Severity::Major, "III")]
                        .into_iter()
                        .rev()
                {
                    ui.checkbox(&mut filters.severity[severity_index(severity)], label)
                        .on_hover_text(severity.label())
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                }
            });
        },
    );
    ui.separator();
}

pub(super) fn overview(
    ui: &mut egui::Ui,
    campaign: &Campaign,
    player: usize,
    filters: &mut ScandalFilters,
    scale: f32,
) -> Option<u64> {
    filters_row(ui, campaign, filters, scale);
    egui::ScrollArea::vertical()
        .id_salt(("campaign_scandals", player))
        .max_height(ui.available_height())
        .min_scrolled_height(0.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0 * scale;
            let scandals = filtered_evidence(campaign, player, filters);
            if scandals.is_empty() {
                ui.add_space(8.0 * scale);
                ui.label(if available(campaign, player).next().is_some() {
                    "No scandals match these filters."
                } else {
                    "No scandals available. Spy networks can uncover evidence."
                });
            }
            let mut selected = None;
            for scandal in scandals {
                let target = match scandal.target {
                    ScandalTarget::Player(player) => format!("Player {}", player + 1),
                    ScandalTarget::Province(province) => format!(
                        "{} · independent government",
                        campaign.economy.provinces[province].name,
                    ),
                };
                let months = scandal.expires.saturating_sub(campaign.senate.month);
                let mut body = format!(
                    "Against {target}\n{} · {months} {} remaining",
                    scandal.severity.label(),
                    if months == 1 {
                        "month"
                    } else {
                        "months"
                    },
                );
                if let Some(province) = scandal.province {
                    body.push_str(&format!(
                        "\nFound in {}",
                        campaign.economy.provinces[province].name
                    ));
                }
                ui.push_id(scandal.id, |ui| {
                    if message_card(
                        ui,
                        Message {
                            icon: Icon::SpyUncoverScandals,
                            title: scandal.kind.label(),
                            body: &body,
                            month: scandal.acquired,
                            warning: scandal.severity == Severity::Major,
                            critical: false,
                            actionable: true,
                        },
                        scale,
                    )
                    .on_hover_text(match scandal.target {
                        ScandalTarget::Player(_) => "Open this evidence in the Senate.",
                        ScandalTarget::Province(_) => {
                            "Open this province's diplomacy and evidence."
                        },
                    })
                    .clicked()
                    {
                        selected = Some(scandal.id);
                    }
                });
            }
            selected
        })
        .inner
}

#[cfg(test)]
#[path = "../../tests/unit/scandals_ui.rs"]
mod tests;
