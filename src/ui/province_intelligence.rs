//! Read-only provincial facts: exact administrative information or dated spy reports.

use super::campaign_widgets::Icon;
use crate::app::campaign::Campaign;
use crate::game::economy::ConstructionProject;
use bevy_egui::egui;

const CLASSES: [(&str, Icon); 4] = [
    ("Nobles", Icon::Nobles),
    ("Citizens", Icon::Citizens),
    ("Plebeians", Icon::Plebeians),
    ("Slaves", Icon::Slaves),
];
const RESOURCES: [(&str, Icon); 3] =
    [("Food", Icon::Food), ("Metal", Icon::Metal), ("Stone", Icon::Stone)];

/// Make historical values unmistakable even when the network has been lost this month.
pub(in crate::app) fn report_label(
    campaign: &Campaign,
    province: usize,
    player: usize,
    month: u32,
) -> String {
    if campaign.administers_province(player, province) {
        "Current administrative report".into()
    } else {
        let active =
            campaign.espionage.missions.iter().any(|m| m.owner == player && m.province == province);
        format!(
            "Observed in month {month} · {}",
            if active {
                "monthly spy report"
            } else {
                "last report; network inactive"
            }
        )
    }
}

/// Fixed row heights keep dated reports within the same inspector budget as domestic ledgers.
pub(in crate::app) fn overview_height(campaign: &Campaign, province: usize, player: usize) -> f32 {
    let p = &campaign.economy.provinces[province];
    let admin = campaign.administers_province(player, province);
    let report = campaign.intelligence.get(&(player, province));
    let economic = admin || report.is_some_and(|r| r.economy.is_some());
    let operations = report.and_then(|r| r.operations.as_ref());
    let project = if admin {
        p.construction.as_ref()
    } else {
        operations.and_then(|r| r.construction.as_ref())
    };
    28.0 + if admin || report.is_some() {
        20.0
    } else {
        0.0
    } + 4.0 * 34.0
        + 12.0
        + 28.0
        + if economic {
            20.0
        } else {
            0.0
        }
        + 3.0 * 34.0
        + if economic {
            6.0 * 22.0
                + if admin {
                    24.0
                } else {
                    0.0
                }
        } else {
            0.0
        }
        + if admin || operations.is_some() {
            12.0 + 28.0
                + 20.0
                + if project.is_some() {
                    42.0
                } else {
                    22.0
                }
        } else if matches!(p.construction, Some(ConstructionProject::Wonder(_))) {
            12.0 + 28.0 + 22.0
        } else {
            0.0
        }
}

fn line(ui: &mut egui::Ui, text: &str, height: f32, scale: f32) {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height * scale),
        egui::Sense::hover(),
    );
    let mut font_size = 12.0 * scale;
    let galley = loop {
        let galley = ui.painter().layout_no_wrap(
            text.into(),
            egui::FontId::proportional(font_size),
            super::province_panel::INK,
        );
        if galley.size().x <= rect.width() || font_size <= 6.0 * scale {
            break galley;
        }
        font_size *= 0.9;
    };
    ui.painter().galley(
        egui::pos2(rect.left(), rect.center().y - galley.size().y * 0.5),
        galley,
        super::province_panel::INK,
    );
    response.on_hover_text(text);
}

pub(in crate::app) fn overview(
    ui: &mut egui::Ui,
    campaign: &Campaign,
    province: usize,
    player: usize,
    scale: f32,
) {
    use super::campaign_economy::{ledger_header, ledger_name, ledger_row, ledger_value};
    let p = &campaign.economy.provinces[province];
    let admin = campaign.administers_province(player, province);
    let report = campaign.intelligence.get(&(player, province));
    let ink = super::province_panel::INK;
    let demographic_month = report.map_or(campaign.economy.month, |r| r.demographics.month);
    ledger_header(
        ui,
        "Population",
        &[
            (Icon::Amount, "Amount: total population"),
            (Icon::Delta, "Monthly population change"),
            (Icon::Happiness, "Class happiness"),
        ],
        scale,
    );
    if admin || report.is_some() {
        line(ui, &report_label(campaign, province, player, demographic_month), 20.0, scale);
    }
    for (class, &(name, symbol)) in CLASSES.iter().enumerate() {
        ledger_row(ui, 4, 34.0 * scale, class % 2 == 0, |ui, cells| {
            ledger_name(ui, cells[0], symbol, name, scale);
            let values = if admin {
                Some((p.population[class], p.happiness[class]))
            } else {
                report.map(|r| (r.demographics.population[class], r.demographics.happiness[class]))
            };
            ledger_value(
                ui,
                cells[1],
                &values.map_or_else(
                    || "?".into(),
                    |(count, _)| super::resource_hud::format_population(count),
                ),
                "Class population",
                ink,
                scale,
            );
            ledger_value(
                ui,
                cells[2],
                &if admin {
                    campaign.economy.last_report.province_reports.get(province).map(|p| {
                        p.births[class] - p.normal_deaths[class] - p.famine_deaths[class]
                            + p.migration[class]
                    })
                } else {
                    report.and_then(|r| {
                        r.demographics.population_change.map(|changes| changes[class])
                    })
                }
                .map_or_else(|| "?".into(), super::resource_hud::format_population_delta),
                "Monthly population change",
                ink,
                scale,
            );
            ledger_value(
                ui,
                cells[3],
                &values.map_or_else(|| "?".into(), |(_, happy)| format!("{happy}%")),
                "Class happiness",
                ink,
                scale,
            );
        });
    }
    ui.add_space(12.0 * scale);
    let economic = report.and_then(|r| r.economy.as_ref());
    ledger_header(
        ui,
        "Resources",
        &[
            (Icon::Amount, "Amount: shared stock / storage capacity"),
            (Icon::Delta, "Monthly provincial production"),
        ],
        scale,
    );
    if admin || economic.is_some() {
        line(
            ui,
            &report_label(
                campaign,
                province,
                player,
                economic.map_or(campaign.economy.month, |r| r.month),
            ),
            20.0,
            scale,
        );
    }
    let production = if admin {
        Some(p.production(&campaign.economy.config).1)
    } else {
        economic.map(|r| r.production)
    };
    for (resource, &(name, symbol)) in RESOURCES.iter().enumerate() {
        ledger_row(ui, 3, 34.0 * scale, resource % 2 == 0, |ui, cells| {
            ledger_name(ui, cells[0], symbol, name, scale);
            ledger_value(
                ui,
                cells[1],
                "? / ?",
                &format!("{name} resource potential: {}", p.potential[resource]),
                ink,
                scale,
            );
            ledger_value(
                ui,
                cells[2],
                &production
                    .map_or_else(|| "?".into(), |values| format!("+{:.1}", values[resource])),
                "Monthly provincial production",
                ink,
                scale,
            );
        });
    }
    if admin || economic.is_some() {
        let food = if admin {
            p.food_request(&campaign.economy.config)
        } else {
            economic.unwrap().food_requested
        };
        line(ui, &format!("Civilian Food consumption: {food:.1}/mo"), 22.0, scale);
        let supply = if admin {
            campaign.economy.last_report.province_reports.get(province).map(|r| r.food_supply_ratio)
        } else {
            economic.unwrap().food_supply_ratio
        };
        line(
            ui,
            &supply.map_or_else(
                || "Food supplied: ?".into(),
                |s| format!("Food supplied: {:.1}%", s * 100.0),
            ),
            22.0,
            scale,
        );
        let policies = if admin {
            p.policies
        } else {
            economic.unwrap().policies
        };
        line(
            ui,
            &format!("Resource Focus: {:?} · Migration: {:?}", policies.focus, policies.migration),
            22.0,
            scale,
        );
        line(
            ui,
            &format!("Food Rations: {:?} · Slave Labor: {:?}", policies.food, policies.slave_labor),
            22.0,
            scale,
        );
        line(
            ui,
            &format!(
                "Construction: {:?} · Civic Spending: {:?}",
                policies.construction, policies.civic_spending
            ),
            22.0,
            scale,
        );
        line(
            ui,
            &format!(
                "Recruitment: {:?} · Manumission: {:?}",
                policies.recruitment, policies.manumission
            ),
            22.0,
            scale,
        );
        if admin {
            line(ui, &format!("Local Coin treasury: {:.1}", p.market.coin_treasury), 24.0, scale);
        }
    }
    let operations = report.and_then(|r| r.operations.as_ref());
    if admin || operations.is_some() {
        ui.add_space(12.0 * scale);
        ledger_header(ui, "Construction", &[], scale);
        line(
            ui,
            &report_label(
                campaign,
                province,
                player,
                operations.map_or(campaign.economy.month, |r| r.month),
            ),
            20.0,
            scale,
        );
        let project = if admin {
            p.construction.as_ref()
        } else {
            operations.and_then(|r| r.construction.as_ref())
        };
        if let Some(project) = project {
            let pace = if admin {
                p.policies.construction
            } else {
                operations.unwrap().construction_pace
            };
            super::campaign_economy::overview_project(
                ui,
                project,
                &campaign.economy.config,
                pace,
                scale,
            );
        } else {
            line(ui, "No construction in this report.", 22.0, scale);
        }
    } else if let Some(ConstructionProject::Wonder(wonder)) = &p.construction {
        ui.add_space(12.0 * scale);
        ledger_header(ui, "Construction", &[], scale);
        line(
            ui,
            &format!(
                "Public wonder project: {}",
                crate::map::wonder_name(wonder.wonder_id).unwrap_or("Wonder")
            ),
            22.0,
            scale,
        );
    }
}
/// Foreign construction cards never read live private progress or worker assignments.
pub(in crate::app) fn buildings(
    ui: &mut egui::Ui,
    campaign: &Campaign,
    province: usize,
    player: usize,
    scale: f32,
) {
    let p = &campaign.economy.provinces[province];
    let administrative = campaign.administers_province(player, province);
    let operations =
        campaign.intelligence.get(&(player, province)).and_then(|r| r.operations.as_ref());
    let project = if administrative {
        p.construction.as_ref()
    } else {
        operations.and_then(|r| r.construction.as_ref())
    };
    if administrative || operations.is_some() {
        ui.small(report_label(
            campaign,
            province,
            player,
            operations.map_or(campaign.economy.month, |r| r.month),
        ));
        if let Some(project) = project {
            let pace = if administrative {
                p.policies.construction
            } else {
                operations.unwrap().construction_pace
            };
            super::campaign_economy::overview_project(
                ui,
                project,
                &campaign.economy.config,
                pace,
                scale,
            );
        } else {
            ui.small("No construction in this report.");
        }
    }
    super::campaign_economy::readonly_buildings(ui, &campaign.economy, province, scale);
}
