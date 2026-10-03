//! Persistent discovered scandals, ordered by severity with local target filters.

use super::campaign::Campaign;
use super::campaign_widgets::{
    directory_owner_marker_size, map_style, paint_directory_row, paint_icon, portrait, Icon,
    ProvinceLandscape, DIRECTORY_ROW_HEIGHT,
};
use super::policy_widgets;
use super::province_panel::{INK, NEUTRAL};
use crate::game::politics::espionage::{Scandal, ScandalTarget, ScandalUse, Severity};
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum ScandalCommand {
    Province {
        id: u64,
        province: usize,
        usage: ScandalUse,
    },
    Senate {
        id: u64,
    },
}

#[derive(Default)]
pub(super) struct ScandalResponse {
    pub selected: Option<u64>,
    pub command: Option<ScandalCommand>,
}

pub(super) fn apply_command(
    campaign: &mut Campaign,
    player: usize,
    command: ScandalCommand,
) -> String {
    let (id, province, usage) = match command {
        ScandalCommand::Province {
            id,
            province,
            usage,
        } => (id, province, usage),
        ScandalCommand::Senate {
            id,
        } => {
            return campaign.expose_scandal(player, id).map_or_else(
                |error| error.to_string(),
                |target| format!("Scandal exposed in the Senate against Player {}.", target + 1),
            )
        },
    };
    match campaign.use_scandal(player, id, province, usage) {
        Ok(gain) => {
            let province = &campaign.economy.provinces[province].name;
            match usage {
                ScandalUse::Control => format!(
                    "Scandal used: +{} Control in {province}.",
                    policy_widgets::compact_decimal(gain)
                ),
                ScandalUse::Relation => format!(
                    "Scandal used: +{} Relation in {province}.",
                    policy_widgets::compact_decimal(gain)
                ),
                ScandalUse::Trade => format!(
                    "Scandal used: favorable trade with {province} for {} months.",
                    campaign.espionage_config.favorable_trade_months
                ),
            }
        },
        Err(error) => error.to_string(),
    }
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
            && scandal.is_current(campaign.economy.month.max(campaign.senate.month))
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

/// Match the notification banner's dark badges while keeping both filters fixed.
fn filters_banner(
    ui: &mut egui::Ui,
    campaign: &Campaign,
    filters: &mut ScandalFilters,
    scale: f32,
) {
    let banner = portrait(ui, ProvinceLandscape::Scandals, 76.0 * scale);
    let margin = 7.0 * scale;
    let rect = egui::Rect::from_min_max(
        egui::pos2(banner.left() + margin, banner.bottom() - margin - 32.0 * scale),
        banner.right_bottom() - egui::Vec2::splat(margin),
    );
    let mut overlay = ui.new_child(
        egui::UiBuilder::new()
            .id_salt("scandal-filters")
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    overlay.set_clip_rect(ui.clip_rect().intersect(banner));
    overlay.scope(|ui| {
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
            widget.weak_bg_fill = egui::Color32::TRANSPARENT;
        }
        widgets.hovered.bg_fill = egui::Color32::from_white_alpha(50);
        widgets.active.bg_fill = egui::Color32::from_white_alpha(70);
        widgets.hovered.weak_bg_fill = egui::Color32::from_white_alpha(30);
        widgets.active.weak_bg_fill = egui::Color32::from_white_alpha(50);
        widgets.open = widgets.hovered;
        let badge = egui::Frame::new()
            .fill(egui::Color32::from_black_alpha(180))
            .corner_radius(3.0 * scale)
            .inner_margin(egui::Margin::symmetric(
                (6.0 * scale).round() as i8,
                (3.0 * scale).round() as i8,
            ));
        badge.show(ui, |ui| {
            egui::ComboBox::from_id_salt("scandal-target")
                .selected_text(filters.target.label())
                .width(140.0 * scale)
                .popup_style(egui::style::StyleModifier::new(move |style| {
                    *style = map_style(scale)
                }))
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
                });
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            badge.show(ui, |ui| {
                for (severity, label) in
                    [(Severity::Minor, "I"), (Severity::Medium, "II"), (Severity::Major, "III")]
                        .into_iter()
                        .rev()
                {
                    ui.checkbox(&mut filters.severity[severity_index(severity)], label)
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                }
            });
        });
    });
}

pub(super) fn overview(
    ui: &mut egui::Ui,
    campaign: &Campaign,
    player: usize,
    colors: &[egui::Color32],
    filters: &mut ScandalFilters,
    scale: f32,
) -> ScandalResponse {
    filters_banner(ui, campaign, filters, scale);
    egui::ScrollArea::vertical()
        .id_salt(("campaign_scandals", player))
        .max_height(ui.available_height())
        .min_scrolled_height(0.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 3.0 * scale;
            headers(ui, scale);
            let scandals = filtered_evidence(campaign, player, filters);
            if scandals.is_empty() {
                ui.add_space(8.0 * scale);
                ui.label(if available(campaign, player).next().is_some() {
                    "No scandals match these filters."
                } else {
                    "No discovered scandals. Spy networks can uncover scandals."
                });
            }
            let mut result = ScandalResponse::default();
            for (index, scandal) in scandals.into_iter().enumerate() {
                ui.push_id(scandal.id, |ui| {
                    let response = row(ui, campaign, scandal, colors, index, scale);
                    result.selected = response.selected.or(result.selected);
                    result.command = response.command.or(result.command);
                });
            }
            result
        })
        .inner
}

const METADATA_INK: egui::Color32 = egui::Color32::from_rgb(108, 105, 96);

fn expanded_actions(width: f32, scale: f32) -> bool {
    width >= 440.0 * scale
}

fn columns(width: f32, scale: f32) -> [f32; 4] {
    let left = 57.0 * scale;
    let actions = width
        - (7.0
            + if expanded_actions(width, scale) {
                114.0
            } else {
                36.0
            })
            * scale;
    let body = (actions - left - 8.0 * scale).max(1.0);
    let validity = (body * 0.30).max(90.0 * scale);
    let severity = (body * 0.22).max(76.0 * scale);
    [left, left + (body - validity - severity).max(1.0), left + body - validity, actions]
}

/// The directory section bar doubles as the headings for both scandal lists.
pub(super) fn headers(ui: &mut egui::Ui, scale: f32) {
    let rect = policy_widgets::section_background(ui, scale, policy_widgets::SECTION_HEIGHT);
    for (x, label) in
        columns(rect.width(), scale).into_iter().zip(["SCANDAL", "SEVERITY", "VALIDITY"])
    {
        ui.painter().text(
            egui::pos2(rect.left() + x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(13.5 * scale),
            egui::Color32::from_rgb(249, 238, 215),
        );
    }
}

/// Same inset player swatch and striped parchment rows as spies and provinces.
pub(super) fn row(
    ui: &mut egui::Ui,
    campaign: &Campaign,
    scandal: &Scandal,
    colors: &[egui::Color32],
    index: usize,
    scale: f32,
) -> ScandalResponse {
    let width = ui.available_width();
    let starts = columns(width, scale);
    let name_width = (starts[1] - starts[0] - 8.0 * scale).max(1.0);
    let (rect, response) = ui
        .allocate_exact_size(egui::vec2(width, DIRECTORY_ROW_HEIGHT * scale), egui::Sense::click());
    paint_directory_row(ui, rect, &response, index, scale);
    let target = match scandal.target {
        ScandalTarget::Player(player) => format!("Player {}", player + 1),
        ScandalTarget::Province(province) => campaign.economy.provinces[province].name.clone(),
    };
    let owner = match scandal.target {
        ScandalTarget::Player(player) => Some(player),
        ScandalTarget::Province(province) => campaign.economy.provinces[province].owner,
    };
    let color = owner.and_then(|player| colors.get(player).copied()).unwrap_or(NEUTRAL);
    let mut detail = format!("Against {target}");
    if let Some(province) = scandal.province {
        if scandal.target != ScandalTarget::Province(province) {
            detail.push_str(&format!(" · Found in {}", campaign.economy.provinces[province].name));
        }
    }
    let (level, description) = match scandal.severity {
        Severity::Minor => ("I", "minor"),
        Severity::Medium => ("II", "serious"),
        Severity::Major => ("III", "grave"),
    };
    let mut severity_job = egui::text::LayoutJob::default();
    severity_job.append(
        level,
        0.0,
        egui::TextFormat::simple(egui::FontId::proportional(12.0 * scale), INK),
    );
    severity_job.append(
        &format!(" · {description}"),
        0.0,
        egui::TextFormat::simple(egui::FontId::proportional(12.0 * scale), METADATA_INK),
    );
    severity_job.wrap.max_width = (starts[2] - starts[1] - 8.0 * scale).max(1.0);
    let severity = ui.painter().layout_job(severity_job);
    let validity = ui.painter().layout(
        scandal.validity_label(campaign.economy.month.max(campaign.senate.month)),
        egui::FontId::proportional(12.0 * scale),
        METADATA_INK,
        (starts[3] - starts[2] - 8.0 * scale).max(1.0),
    );
    let text = |label: String, size: f32, color| {
        let mut job = egui::text::LayoutJob::simple(
            label,
            egui::FontId::proportional(size * scale),
            color,
            name_width,
        );
        job.wrap.max_rows = 1;
        job.wrap.break_anywhere = true;
        ui.painter().layout_job(job)
    };
    let title = text(scandal.kind.label().into(), 14.0, INK);
    let detail = text(detail, 10.0, METADATA_INK);
    let marker = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 10.0 * scale, rect.center().y),
        directory_owner_marker_size(rect.height(), scale),
    );
    ui.painter().rect_filled(marker, 2.0 * scale, color);
    paint_icon(
        ui,
        Icon::SpyUncoverScandals,
        egui::Rect::from_center_size(
            egui::pos2(rect.left() + 35.0 * scale, rect.center().y),
            egui::Vec2::splat(28.0 * scale),
        ),
    );
    let title_pos = egui::pos2(
        rect.left() + starts[0],
        rect.center().y - (title.size().y + detail.size().y + 2.0 * scale) * 0.5,
    );
    ui.painter().galley(
        title_pos + egui::vec2(0.0, title.size().y + 2.0 * scale),
        detail,
        METADATA_INK,
    );
    ui.painter().galley(title_pos, title, INK);
    for (x, text) in [(starts[1], severity), (starts[2], validity)] {
        ui.painter().galley(
            egui::pos2(rect.left() + x, rect.center().y - text.size().y * 0.5),
            text,
            METADATA_INK,
        );
    }
    let actions_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + starts[3], rect.center().y - 17.0 * scale),
        egui::pos2(rect.right() - 7.0 * scale, rect.center().y + 17.0 * scale),
    );
    let mut result = row_actions(ui, campaign, scandal, actions_rect, scale);
    let over_actions =
        ui.input(|input| input.pointer.hover_pos().is_some_and(|pos| actions_rect.contains(pos)));
    if !over_actions {
        if response.clicked() {
            result.selected = Some(scandal.id);
        }
        response.on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    result
}

fn action_button(ui: &mut egui::Ui, icon: Icon, caption: &str, scale: f32) -> egui::Response {
    let response = ui.add_sized([36.0 * scale, 34.0 * scale], egui::Button::new(""));
    paint_icon(
        ui,
        icon,
        egui::Rect::from_center_size(
            response.rect.center() - egui::vec2(0.0, 5.0 * scale),
            egui::Vec2::splat(17.0 * scale),
        ),
    );
    ui.painter().text(
        response.rect.center() + egui::vec2(0.0, 10.0 * scale),
        egui::Align2::CENTER_CENTER,
        caption,
        egui::FontId::proportional(9.0 * scale),
        INK,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn action_targets(campaign: &Campaign, scandal: &Scandal, usage: ScandalUse) -> Vec<(usize, f64)> {
    let mut targets: Vec<_> = (0..campaign.politics.len())
        .filter(|&province| match scandal.target {
            ScandalTarget::Province(target) => province == target,
            ScandalTarget::Player(target) => matches!(campaign.politics[province].state,
                crate::game::politics::diplomacy::PoliticalState::Owned { owner } if owner == target)
                || matches!(campaign.politics[province].state,
                    crate::game::politics::diplomacy::PoliticalState::Vassal { overlord, .. } if overlord == target),
        })
        .filter_map(|province| {
            campaign
                .scandal_use_quote(scandal.holder, scandal.id, province, usage)
                .ok()
                .map(|gain| (province, gain))
        })
        .collect();
    targets.sort_by_key(|(province, _)| {
        (Some(*province) != scandal.province, &campaign.economy.provinces[*province].name)
    });
    targets
}

fn use_label(usage: ScandalUse, gain: f64) -> String {
    let amount = policy_widgets::compact_decimal(gain);
    match usage {
        ScandalUse::Control => format!("Use scandal: gain {amount} Control"),
        ScandalUse::Relation => format!("Use scandal: gain {amount} Relation"),
        ScandalUse::Trade => format!("Use scandal: {amount}% better trade terms"),
    }
}

fn target_options(
    ui: &mut egui::Ui,
    campaign: &Campaign,
    scandal: &Scandal,
    usage: ScandalUse,
    targets: &[(usize, f64)],
    result: &mut ScandalResponse,
) {
    for &(province, gain) in targets {
        let label =
            format!("{} · {}", campaign.economy.provinces[province].name, use_label(usage, gain));
        if ui.button(label).clicked() {
            result.command = Some(ScandalCommand::Province {
                id: scandal.id,
                province,
                usage,
            });
            ui.close();
        }
    }
}

fn row_actions(
    ui: &mut egui::Ui,
    campaign: &Campaign,
    scandal: &Scandal,
    rect: egui::Rect,
    scale: f32,
) -> ScandalResponse {
    let mut result = ScandalResponse::default();
    if scandal.reserved_for_motion {
        return result;
    }
    let uses: Vec<_> = [
        (ScandalUse::Control, Icon::ScandalControl),
        (ScandalUse::Relation, Icon::ScandalRelation),
        (ScandalUse::Trade, Icon::Trade),
    ]
    .into_iter()
    .filter_map(|(usage, icon)| {
        let targets = action_targets(campaign, scandal, usage);
        (!targets.is_empty()).then_some((usage, icon, targets))
    })
    .collect();
    let senate = matches!(scandal.target, ScandalTarget::Player(target)
        if target != scandal.holder && target < campaign.actors.len())
        && campaign.senate.winner.is_none()
        && !campaign.defeated.get(scandal.holder).copied().unwrap_or(false);
    if uses.is_empty() && !senate {
        return result;
    }
    ui.scope_builder(
        egui::UiBuilder::new()
            .id_salt(("scandal-actions", scandal.id))
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.spacing_mut().item_spacing.x = 3.0 * scale;
            if rect.width() > 40.0 * scale {
                for (usage, icon, targets) in &uses {
                    let default = targets
                        .iter()
                        .find(|(province, _)| Some(*province) == scandal.province)
                        .or_else(|| {
                            matches!(scandal.target, ScandalTarget::Province(_))
                                .then(|| &targets[0])
                        });
                    let caption = if *usage == ScandalUse::Trade {
                        "Trade".into()
                    } else {
                        format!(
                            "+{}",
                            policy_widgets::compact_decimal(
                                default.map_or(scandal.severity.control_gain(), |(_, gain)| *gain)
                            )
                        )
                    };
                    let response = action_button(ui, *icon, &caption, scale);
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Button,
                            true,
                            use_label(
                                *usage,
                                default.map_or(scandal.severity.control_gain(), |(_, gain)| *gain),
                            ),
                        )
                    });
                    if let Some(&(province, _)) = default {
                        if response.clicked() {
                            result.command = Some(ScandalCommand::Province {
                                id: scandal.id,
                                province,
                                usage: *usage,
                            });
                        }
                    } else {
                        egui::Popup::menu(&response).show(|ui| {
                            target_options(ui, campaign, scandal, *usage, targets, &mut result)
                        });
                    }
                }
                if senate && action_button(ui, Icon::SenatorDiscredit, "Senate", scale).clicked() {
                    result.command = Some(ScandalCommand::Senate {
                        id: scandal.id,
                    });
                }
            } else {
                let response = action_button(ui, Icon::SpyUncoverScandals, "Use", scale);
                egui::Popup::menu(&response).show(|ui| {
                    *ui.style_mut() = map_style(scale);
                    for (usage, _, targets) in &uses {
                        target_options(ui, campaign, scandal, *usage, targets, &mut result);
                    }
                    if senate && ui.button("Expose in Senate").clicked() {
                        result.command = Some(ScandalCommand::Senate {
                            id: scandal.id,
                        });
                        ui.close();
                    }
                });
            }
        },
    );
    result
}

#[cfg(test)]
#[path = "../../tests/unit/scandals_ui.rs"]
mod tests;
