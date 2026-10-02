//! Icon-led economy and construction tables for the parchment province panel.

use super::spectator::InspectionHover;
use crate::game::economy::*;
use crate::game::military::MilitaryWorld;
use bevy_egui::egui;

use super::campaign_widgets::{paint_icon, work_queue, Icon, WorkQueueAction};

const CLASS_NAMES: [&str; 4] = ["Nobles", "Citizens", "Plebeians", "Slaves"];
const CLASS_ICONS: [Icon; 4] = [Icon::Nobles, Icon::Citizens, Icon::Plebeians, Icon::Slaves];
const RESOURCE_NAMES: [&str; 3] = ["Food", "Metal", "Stone"];
const RESOURCE_ICONS: [Icon; 3] = [Icon::Food, Icon::Metal, Icon::Stone];
const MUTED: egui::Color32 = egui::Color32::from_rgb(112, 91, 71);
pub(in crate::app) const CIVIC_POWER: &str = "Civic Power";

/// Use the full province layout as the scale reference, including for NPCs.
/// Their shorter overview then keeps the same badge and population row sizes.
pub(in crate::app) fn overview_height(_world: &EconomyWorld, _province: usize) -> f32 {
    let population = 48.0 + 10.0 + 28.0 + 4.0 * 34.0;
    let civic_power = 12.0 + 28.0 + 2.0 * 34.0;
    let resources = 12.0 + 28.0 + 3.0 * 34.0;
    population + civic_power + resources
}

/// Paint a value within its column, reducing type only for unusually large numbers.
pub(super) fn ledger_text(
    ui: &egui::Ui,
    rect: egui::Rect,
    value: &str,
    size: f32,
    color: egui::Color32,
    align: egui::Align,
) {
    let mut font_size = size;
    let text = loop {
        let text = ui.painter().layout_no_wrap(
            value.to_owned(),
            egui::FontId::proportional(font_size),
            color,
        );
        if text.size().x <= rect.width() || font_size <= size * 0.5 {
            break text;
        }
        font_size *= 0.9;
    };
    let x = if align == egui::Align::Min {
        rect.left()
    } else {
        rect.center().x - text.size().x * 0.5
    };
    ui.painter().galley(egui::pos2(x, rect.center().y - text.size().y * 0.5), text, color);
}

/// Equal-width badges stay on a single row.
pub(super) fn overview_badge(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    symbol: Icon,
    caption: &str,
    value: &str,
    tip: &str,
    value_color: egui::Color32,
    scale: f32,
) {
    use super::province_panel::{RULE, TABLE_STRIPE};
    ui.painter().rect_filled(rect, 4.0 * scale, TABLE_STRIPE);
    ui.painter().rect_stroke(
        rect,
        4.0 * scale,
        egui::Stroke::new(0.7 * scale, RULE),
        egui::StrokeKind::Inside,
    );
    let compact = rect.width() < 122.0 * scale;
    let icon_offset = if compact {
        17.0
    } else {
        22.0
    };
    let icon_size = if compact {
        24.0
    } else {
        30.0
    };
    let text_offset = if compact {
        32.0
    } else {
        41.0
    };
    let image = egui::Rect::from_center_size(
        egui::pos2(rect.left() + icon_offset * scale, rect.center().y),
        egui::vec2(icon_size, icon_size) * scale,
    );
    paint_icon(ui, symbol, image);
    let text = egui::Rect::from_min_max(
        egui::pos2(rect.left() + text_offset * scale, rect.top()),
        rect.max - egui::vec2(5.0 * scale, 0.0),
    );
    ledger_text(
        ui,
        egui::Rect::from_min_max(
            text.min + egui::vec2(0.0, 3.0 * scale),
            egui::pos2(text.right(), text.top() + 23.0 * scale),
        ),
        caption,
        (if compact {
            11.5
        } else {
            13.0
        }) * scale,
        MUTED,
        egui::Align::Center,
    );
    ledger_text(
        ui,
        egui::Rect::from_min_max(egui::pos2(text.left(), text.top() + 18.0 * scale), text.max),
        value,
        16.0 * scale,
        value_color,
        egui::Align::Center,
    );
    if !tip.is_empty() {
        ui.interact(rect, ui.id().with(caption), egui::Sense::hover()).inspection_hover_text(tip);
    }
}

/// Fixed full-width rows align portraits, values and icon headers in both ledgers.
pub(in crate::app) fn ledger_row(
    ui: &mut egui::Ui,
    columns: usize,
    height: f32,
    stripe: bool,
    render: impl FnOnce(&mut egui::Ui, &[egui::Rect]),
) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), height), egui::Sense::hover());
    if stripe {
        ui.painter().rect_filled(rect, 3.0, super::province_panel::TABLE_STRIPE);
    }
    let name_width = rect.width() * 0.38;
    let value_width = (rect.width() - name_width) / columns.saturating_sub(1).max(1) as f32;
    let mut cells = vec![egui::Rect::from_min_max(
        rect.min,
        egui::pos2(rect.left() + name_width, rect.bottom()),
    )];
    for index in 1..columns {
        let left = rect.left() + name_width + (index - 1) as f32 * value_width;
        cells.push(egui::Rect::from_min_max(
            egui::pos2(left, rect.top()),
            egui::pos2(left + value_width, rect.bottom()),
        ));
    }
    render(ui, &cells);
}

pub(in crate::app) fn ledger_header(
    ui: &mut egui::Ui,
    title: &str,
    headers: &[(Icon, &str)],
    scale: f32,
) {
    ledger_row(ui, headers.len() + 1, 28.0 * scale, false, |ui, cells| {
        ui.painter().text(
            cells[0].left_center() + egui::vec2(7.0 * scale, 0.0),
            egui::Align2::LEFT_CENTER,
            title,
            egui::FontId::proportional(14.0 * scale),
            MUTED,
        );
        for (cell, &(symbol, tip)) in cells[1..].iter().zip(headers) {
            let rect = egui::Rect::from_center_size(cell.center(), egui::vec2(22.0, 22.0) * scale);
            paint_icon(ui, symbol, rect);
            ui.interact(rect, ui.id().with((title, tip)), egui::Sense::hover())
                .inspection_hover_ui(|ui| {
                    ui.add(egui::Label::new(tip).wrap_mode(egui::TextWrapMode::Extend));
                });
        }
    });
}

pub(in crate::app) fn ledger_name(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    symbol: Icon,
    name: &str,
    scale: f32,
) {
    // The noble portrait reads larger than the other class portraits at equal bounds.
    let icon_size = if symbol == Icon::Nobles {
        24.0
    } else {
        28.0
    };
    let image = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 20.0 * scale, rect.center().y),
        egui::vec2(icon_size, icon_size) * scale,
    );
    paint_icon(ui, symbol, image);
    ui.interact(image, ui.id().with(name), egui::Sense::hover()).inspection_hover_text(name);
    let text = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 39.0 * scale, rect.top()),
        rect.max - egui::vec2(4.0 * scale, 0.0),
    );
    ledger_text(ui, text, name, 16.0 * scale, super::province_panel::INK, egui::Align::Min);
}

pub(in crate::app) fn ledger_value(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    value: &str,
    tip: &str,
    color: egui::Color32,
    scale: f32,
) {
    ledger_text(
        ui,
        rect.shrink2(egui::vec2(4.0 * scale, 0.0)),
        value,
        16.0 * scale,
        color,
        egui::Align::Center,
    );
    ui.interact(
        rect,
        ui.id().with((rect.min.x.to_bits(), rect.min.y.to_bits())),
        egui::Sense::hover(),
    )
    .inspection_hover_text(tip);
}

/// Explain a provincial contribution with illustrated bullets instead of formulas.
fn ledger_breakdown_value(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    value: &str,
    title: &str,
    entries: &[(Icon, String)],
    color: egui::Color32,
    scale: f32,
) {
    ledger_text(
        ui,
        rect.shrink2(egui::vec2(4.0 * scale, 0.0)),
        value,
        16.0 * scale,
        color,
        egui::Align::Center,
    );
    ui.interact(
        rect,
        ui.id().with((rect.min.x.to_bits(), rect.min.y.to_bits())),
        egui::Sense::hover(),
    )
    .inspection_hover_ui(|ui| {
        ui.strong(title);
        for (symbol, text) in entries {
            ui.horizontal(|ui| {
                ui.label("•");
                super::campaign_widgets::icon(ui, *symbol, 22.0 * scale);
                ui.label(text);
            });
        }
    });
}

/// Colour the displayed contribution, keeping rounded zero neutral.
fn signed_contribution(value: f64, decimals: usize) -> (String, egui::Color32) {
    let displayed = if decimals == 0 {
        value.round()
    } else {
        let factor = 10_f64.powi(decimals as i32);
        (value * factor).round() / factor
    };
    if displayed == 0.0 {
        ("0".to_owned(), egui::Color32::BLACK)
    } else {
        (format!("{displayed:+.decimals$}"), super::resource_hud::hud_delta_color(displayed))
    }
}

fn rounded_breakdown_total(entries: &[(&str, f64)]) -> f64 {
    entries.iter().map(|(_, amount)| (amount * 1000.0).round() / 1000.0).sum()
}

#[cfg(test)]
#[test]
fn population_growth_total_matches_the_displayed_contributions() {
    let all_zero = [("Births", 0.0002), ("Natural deaths", -0.0004)];
    assert!(all_zero.iter().all(|(_, amount)| signed_contribution(*amount, 3).0 == "0"));
    assert_eq!(rounded_breakdown_total(&all_zero), 0.0);

    let mixed = [("Births", 0.105), ("Natural deaths", -0.06), ("Migration", -0.006)];
    assert!((rounded_breakdown_total(&mixed) - 0.039).abs() < 1e-9);
    assert_eq!(signed_contribution(-0.006, 3).0, "-0.006");
}

fn food_demand_amount(amount: f64) -> (String, egui::Color32) {
    (
        super::resource_hud::format_food_demand(amount),
        super::resource_hud::hud_delta_color(-amount.abs().floor()),
    )
}

fn tooltip_numeric_bullet(
    ui: &mut egui::Ui,
    label: &str,
    value: &str,
    color: egui::Color32,
    indent: f32,
) {
    ui.scope(|ui| {
        // Text-only bullet rows should not inherit the minimum button height.
        ui.spacing_mut().interact_size.y = 0.0;
        ui.horizontal(|ui| {
            if indent > 0.0 {
                ui.add_space(indent);
            }
            ui.label(egui::RichText::new(format!("• {label}:")).color(egui::Color32::BLACK));
            ui.label(egui::RichText::new(value).color(color));
        });
    });
}

fn ledger_signed_breakdown_value(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    value: &str,
    entries: &[(&str, f64)],
    decimals: usize,
    color: egui::Color32,
    scale: f32,
) {
    ledger_text(
        ui,
        rect.shrink2(egui::vec2(4.0 * scale, 0.0)),
        value,
        16.0 * scale,
        color,
        egui::Align::Center,
    );
    ui.interact(
        rect,
        ui.id().with((rect.min.x.to_bits(), rect.min.y.to_bits())),
        egui::Sense::hover(),
    )
    .inspection_hover_ui(|ui| {
        for &(label, amount) in entries {
            let (value, color) = signed_contribution(amount, decimals);
            tooltip_numeric_bullet(ui, label, &value, color, 0.0);
        }
    });
}

/// Population and resource ledgers fill the inspector without a scroll container.
pub(in crate::app) fn overview(
    ui: &mut egui::Ui,
    world: &EconomyWorld,
    military: &MilitaryWorld,
    province: usize,
    scale: f32,
) {
    overview_for_player(
        ui,
        world,
        military,
        province,
        world.provinces.get(province).and_then(|p| p.owner),
        scale,
    );
}

/// Local facts are public; food fulfillment and national balances belong to the owner.
pub(in crate::app) fn overview_for_player(
    ui: &mut egui::Ui,
    world: &EconomyWorld,
    military: &MilitaryWorld,
    province: usize,
    player: Option<usize>,
    scale: f32,
) {
    let Some(p) = world.provinces.get(province) else {
        return;
    };
    let capacity = p.capacity(&world.config);
    let last = world.last_report.province_reports.get(province);
    let ink = super::province_panel::INK;
    ui.spacing_mut().item_spacing.y = 0.0;
    let (badges, _) = ui
        .allocate_exact_size(egui::vec2(ui.available_width(), 48.0 * scale), egui::Sense::hover());
    let gap = 6.0 * scale;
    let show_food = p.owner.is_some() && p.owner == player;
    let badge_width = (badges.width() - 2.0 * gap) / 3.0;
    let badge = |index: usize| {
        egui::Rect::from_min_size(
            badges.min + egui::vec2(index as f32 * (badge_width + gap), 0.0),
            egui::vec2(badge_width, badges.height()),
        )
    };
    overview_badge(
        ui,
        badge(0),
        Icon::Population,
        "Population",
        &format!(
            "{} / {}",
            super::resource_hud::format_population(p.total_population()),
            super::resource_hud::format_population(capacity)
        ),
        "",
        if p.total_population() > capacity {
            super::resource_hud::hud_delta_color(-1.0)
        } else {
            ink
        },
        scale,
    );
    ui.interact(badge(0), ui.id().with("population-capacity"), egui::Sense::hover())
        .inspection_hover_ui(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0 * scale;
            ui.strong("Population capacity");
            tooltip_numeric_bullet(
                ui,
                "Area",
                &format!("{:.1}", p.capacity_area),
                egui::Color32::BLACK,
                0.0,
            );
            let terrain = world.config.terrain_capacity[p.terrain as usize];
            let terrain_displayed = (terrain * 10.0).round() / 10.0;
            let terrain_color = if terrain_displayed == 1.0 {
                egui::Color32::BLACK
            } else {
                super::resource_hud::hud_delta_color(terrain_displayed - 1.0)
            };
            tooltip_numeric_bullet(
                ui,
                "Terrain modifier",
                &format!("{terrain:.1}"),
                terrain_color,
                0.0,
            );
            for (label, amount) in [
                (
                    "City",
                    if p.has_city {
                        world.config.city_capacity
                    } else {
                        0.0
                    },
                ),
                ("Buildings", p.building_effects(&world.config).capacity),
            ] {
                let (value, color) = signed_contribution(amount, 1);
                tooltip_numeric_bullet(ui, label, &value, color, 0.0);
            }
        });
    if show_food {
        let civilian_food = p.food_request(&world.config);
        let military_food: f64 = military
            .provinces
            .get(province)
            .into_iter()
            .flat_map(|state| state.forces.values().flatten())
            .chain(
                military.battles.iter().filter(|battle| battle.province == province).flat_map(
                    |battle| battle.attackers.units.iter().chain(&battle.defenders.units),
                ),
            )
            .map(|unit| unit.food_demand(&military.config))
            .sum();
        let food_request = civilian_food + military_food;
        let food_policy = world.config.food_policy[p.policies.food as usize].consumption;
        let (food_value, food_color) = food_demand_amount(food_request);
        overview_badge(
            ui,
            badge(1),
            Icon::Food,
            "Food demand",
            &format!("{food_value}/mo"),
            "",
            food_color,
            scale,
        );
        ui.interact(badge(1), ui.id().with("food-demand-breakdown"), egui::Sense::hover()).inspection_hover_ui(
        |ui| {
            ui.spacing_mut().item_spacing.y = 2.0 * scale;
            ui.strong("Food demand per month");
            ui.label("Food consumed each month, not a food deficit. Your national food stock supplies this demand.");
            let (value, color) = food_demand_amount(civilian_food);
            tooltip_numeric_bullet(ui, "Civilians", &value, color, 0.0);
            for (class, name) in CLASS_NAMES.iter().enumerate() {
                let amount = p.population[class] * world.config.food_per_class[class] * food_policy;
                let (value, color) = food_demand_amount(amount);
                tooltip_numeric_bullet(ui, name, &value, color, 14.0 * scale);
            }
            let (value, color) = food_demand_amount(military_food);
            tooltip_numeric_bullet(ui, "Military", &value, color, 0.0);
        },
    );
        let supply_color = last.map_or(ink, |month| {
            super::resource_hud::hud_delta_color(if month.food_supply_ratio < 1.0 {
                -1.0
            } else {
                1.0
            })
        });
        overview_badge(ui, badge(2), Icon::Food, "Food supplied", &last.map_or_else(|| "—".to_owned(), |last| format!("{:.0}%", (last.food_supply_ratio * 100.0).floor())),
            "Last month's food fulfillment\n\n100% means all food demand was covered by the owner's national food stock. Shortages proportionally lower happiness and births and can potentially cause famines.", supply_color, scale);
    }
    ui.add_space(10.0 * scale);
    ledger_header(
        ui,
        "Population",
        &[
            (Icon::Amount, "Total population"),
            (Icon::Delta, "Monthly growth"),
            (Icon::Happiness, "Happiness"),
            (Icon::Delta, "Monthly happiness modifiers"),
        ],
        scale,
    );
    for class in 0..4 {
        ledger_row(ui, 5, 34.0 * scale, class % 2 == 0, |ui, cells| {
            ledger_name(ui, cells[0], CLASS_ICONS[class], CLASS_NAMES[class], scale);
            ledger_text(
                ui,
                cells[1].shrink2(egui::vec2(4.0 * scale, 0.0)),
                &super::resource_hud::format_population(p.population[class]),
                16.0 * scale,
                ink,
                egui::Align::Center,
            );
            if let Some(month) = last {
                let mut entries = vec![
                    ("Births", month.births[class]),
                    ("Natural deaths", -month.normal_deaths[class]),
                    ("Famine deaths", -month.famine_deaths[class]),
                    ("Migration", month.migration[class]),
                ];
                if class == 3 && month.slave_revolt_loss > 0.0 {
                    entries.push(("Slave revolt", -month.slave_revolt_loss));
                }
                let change = rounded_breakdown_total(&entries);
                let displayed_change = change.floor();
                let change_color = if displayed_change == 0.0 {
                    egui::Color32::BLACK
                } else {
                    super::resource_hud::hud_delta_color(displayed_change)
                };
                ledger_signed_breakdown_value(
                    ui,
                    cells[2],
                    &super::resource_hud::format_population_delta(change),
                    &entries,
                    3,
                    change_color,
                    scale,
                );
            } else {
                ledger_value(ui, cells[2], "—", "No month has resolved yet.", MUTED, scale);
            }
            let food = world.config.food_policy[p.policies.food as usize];
            let slave = world.config.slave_policy[p.policies.slave_labor as usize];
            let mut entries = vec![
                ("Food policy", food.happiness),
                ("Buildings", p.building_effects(&world.config).happiness[class]),
                (
                    "Manumission policy",
                    world.config.manumission_happiness[p.policies.manumission as usize][class],
                ),
                ("Events / unrest", p.happiness_modifiers[class] + p.temporary_happiness[class]),
            ];
            if class == 1 || class == 2 {
                entries.push(("Recruitment", p.recruitment_happiness));
            }
            if class == 3 {
                entries.push(("Slave labor", slave.happiness));
            } else {
                entries.push((
                    "Migration policy",
                    world.config.migration_happiness[p.policies.migration as usize],
                ));
                entries.push(("Civic spending", p.civic_happiness));
            }
            entries.push(("Overcrowding", -last.map_or(0.0, |month| month.overcrowding_penalty)));
            entries.push(("Food shortage", -last.map_or(0.0, |month| month.shortage_penalty)));
            // Show only modifiers large enough to appear at the tooltip's precision.
            entries.retain(|(_, amount)| (amount * 10.0).round() != 0.0);
            let modifier: f64 =
                entries.iter().map(|(_, amount)| (amount * 10.0).round() / 10.0).sum();
            let (modifier_value, modifier_color) = signed_contribution(modifier, 1);
            let affected =
                ["Noble Influence", "Citizen taxes", "Plebeian resources", "Slave resources"]
                    [class];
            let threshold = UNHAPPINESS_THRESHOLDS[class];
            let mut consequence = format!(
                "Below {threshold:.0} happiness, {affected} fall with happiness. Current output: {:.0}%. Accumulated overcrowding loss: {:.1}; food shortage loss: {:.1}. These recover after conditions improve.",
                happiness_output_multiplier(class, p.happiness[class], &world.config) * 100.0,
                p.overcrowding_unhappiness,
                p.shortage_unhappiness,
            );
            if class == 3 {
                consequence.push_str(" At 5 or lower, Slaves can revolt.");
            }
            ledger_value(
                ui,
                cells[3],
                &format!("{:.0}", p.happiness[class]),
                &consequence,
                ink,
                scale,
            );
            if entries.is_empty() {
                ledger_text(
                    ui,
                    cells[4].shrink2(egui::vec2(4.0 * scale, 0.0)),
                    &modifier_value,
                    16.0 * scale,
                    modifier_color,
                    egui::Align::Center,
                );
            } else {
                ledger_signed_breakdown_value(
                    ui,
                    cells[4],
                    &modifier_value,
                    &entries,
                    1,
                    modifier_color,
                    scale,
                );
            }
        });
    }
    if p.owner.is_some() {
        let (_, production) = p.production(&world.config);
        let workers = p.production_workers(&world.config);
        let effects = p.building_effects(&world.config);
        let owner = p.owner.and_then(|id| world.players.get(id));
        let own_wallet = p.owner.is_some() && p.owner == player;
        let balance = |amount: fn(&PlayerEconomy) -> f64| {
            owner.filter(|_| own_wallet).map_or_else(
                || {
                    if owner.is_some() {
                        "?"
                    } else {
                        "—"
                    }
                    .into()
                },
                |owner| format!("{:.0}", amount(owner)),
            )
        };
        {
            ui.add_space(12.0 * scale);
            ledger_header(
                ui,
                CIVIC_POWER,
                &[(Icon::Amount, "Total amount"), (Icon::Delta, "Monthly income")],
                scale,
            );
            let noble = p.population[0] * world.config.influence_per_noble;
            let buildings = p.building_effects(&world.config).influence;
            let wonder = p
                .completed_wonder
                .and_then(|id| world.config.wonders.iter().find(|d| d.wonder_id == id))
                .map_or(0.0, |d| d.monthly_influence);
            let displayed_noble = noble.round();
            let displayed_buildings = buildings.round();
            let displayed_wonder = wonder.round();
            let displayed_income = displayed_noble + displayed_buildings + displayed_wonder;
            ledger_row(ui, 3, 34.0 * scale, true, |ui, cells| {
                ledger_name(ui, cells[0], Icon::Influence, "Influence", scale);
                ledger_text(
                    ui,
                    cells[1].shrink2(egui::vec2(4.0 * scale, 0.0)),
                    &balance(|owner| owner.influence),
                    16.0 * scale,
                    ink,
                    egui::Align::Center,
                );
                let mut entries = vec![(
                    Icon::Nobles,
                    format!(
                        "{} Nobles: {} Influence",
                        super::resource_hud::format_population(p.population[0]),
                        signed_contribution(displayed_noble, 0).0,
                    ),
                )];
                if buildings != 0.0 {
                    entries.push((
                        Icon::Construction,
                        format!(
                            "Buildings: {} Influence",
                            signed_contribution(displayed_buildings, 0).0
                        ),
                    ));
                }
                if let Some(id) = p.completed_wonder.filter(|_| wonder != 0.0) {
                    entries.push((
                        Icon::Wonder(id),
                        format!(
                            "Completed wonder: {} Influence",
                            signed_contribution(displayed_wonder, 0).0
                        ),
                    ));
                }
                ledger_breakdown_value(
                    ui,
                    cells[2],
                    &signed_contribution(displayed_income, 0).0,
                    "Influence per month",
                    &entries,
                    super::resource_hud::hud_delta_color(displayed_income),
                    scale,
                );
            });
            ledger_row(ui, 3, 34.0 * scale, false, |ui, cells| {
                ledger_name(ui, cells[0], Icon::Coin, "Sestertius", scale);
                ledger_text(
                    ui,
                    cells[1].shrink2(egui::vec2(4.0 * scale, 0.0)),
                    &balance(|owner| owner.coin),
                    16.0 * scale,
                    ink,
                    egui::Align::Center,
                );
                let mut entries: Vec<_> = (1..=2)
                    .filter_map(|class| {
                        let income = p.population[class] * world.config.tax_rates[class];
                        (income != 0.0).then(|| {
                            (
                                CLASS_ICONS[class],
                                format!(
                                    "{} {}: +{income:.1}",
                                    super::resource_hud::format_population(p.population[class]),
                                    CLASS_NAMES[class],
                                ),
                            )
                        })
                    })
                    .collect();
                if effects.tax != 0.0 {
                    entries.push((
                        Icon::Construction,
                        format!("Buildings: {:+.0}% taxes", effects.tax * 100.0),
                    ));
                }
                ledger_breakdown_value(
                    ui,
                    cells[2],
                    &format!("{:+.0}", p.tax_income(&world.config).floor()),
                    "Sestertii per month",
                    &entries,
                    super::resource_hud::hud_delta_color(p.tax_income(&world.config)),
                    scale,
                );
            });
        }
        ui.add_space(12.0 * scale);
        ledger_header(
            ui,
            "Resources",
            &[(Icon::BaseAmount, "Base amount"), (Icon::Delta, "Monthly production")],
            scale,
        );
        for resource in 0..3 {
            ledger_row(ui, 3, 34.0 * scale, resource % 2 == 0, |ui, cells| {
                ledger_name(
                    ui,
                    cells[0],
                    RESOURCE_ICONS[resource],
                    RESOURCE_NAMES[resource],
                    scale,
                );
                ledger_value(
                    ui,
                    cells[1],
                    &p.potential[resource].to_string(),
                    "This province's natural resource potential.",
                    ink,
                    scale,
                );
                let mut entries: Vec<_> = [2, 3]
                    .into_iter()
                    .map(|class| {
                        (
                            CLASS_ICONS[class],
                            format!(
                                "{} {} working",
                                super::resource_hud::format_population(workers[resource][class]),
                                CLASS_NAMES[class],
                            ),
                        )
                    })
                    .collect();
                if effects.production[resource] != 0.0 {
                    entries.push((
                        Icon::Construction,
                        format!(
                            "Buildings: {:+.0}% production",
                            effects.production[resource] * 100.0,
                        ),
                    ));
                }
                ledger_breakdown_value(
                    ui,
                    cells[2],
                    &format!("{:+.0}", production[resource].floor()),
                    &format!("{} production per month", RESOURCE_NAMES[resource]),
                    &entries,
                    super::resource_hud::hud_delta_color(production[resource]),
                    scale,
                );
            });
        }
    }
}

/// Province-local policies share the nationwide Governance card treatment.
pub(in crate::app) fn policies(
    ui: &mut egui::Ui,
    world: &mut EconomyWorld,
    province: usize,
    player: usize,
) {
    super::campaign_policies::province(ui, world, province, player);
}

const BUILDING_CELL_SIZE: egui::Vec2 = egui::vec2(108.0, 120.0);
const BUILDING_COLUMNS: usize = 4;

/// Remember when a project first occupies the active slot within this month.
fn record_construction_preview_start(ctx: &egui::Context, province: usize, month: u32) {
    ctx.data_mut(|data| {
        let fraction = data
            .get_temp::<f32>(egui::Id::new("campaign-construction-month-fraction"))
            .unwrap_or(0.0);
        data.insert_temp(
            egui::Id::new(("construction-preview-start", province)),
            (month, fraction),
        );
    });
}

fn construction_preview_fraction(ctx: &egui::Context, province: usize, month: u32) -> f32 {
    ctx.data(|data| {
        let fraction = data
            .get_temp::<f32>(egui::Id::new("campaign-construction-month-fraction"))
            .unwrap_or(0.0);
        let start = data
            .get_temp::<(u32, f32)>(egui::Id::new(("construction-preview-start", province)))
            .filter(|(start_month, _)| *start_month == month)
            .map_or(0.0, |(_, start_fraction)| start_fraction);
        (fraction - start).max(0.0)
    })
}

#[cfg(test)]
#[test]
fn construction_preview_starts_at_zero_and_stays_still_while_paused() {
    let ctx = egui::Context::default();
    let clock_fraction = egui::Id::new("campaign-construction-month-fraction");
    ctx.data_mut(|data| data.insert_temp(clock_fraction, 0.4_f32));
    record_construction_preview_start(&ctx, 2, 7);

    assert_eq!(construction_preview_fraction(&ctx, 2, 7), 0.0);
    assert_eq!(construction_preview_fraction(&ctx, 2, 7), 0.0);

    ctx.data_mut(|data| data.insert_temp(clock_fraction, 0.5_f32));
    assert!((construction_preview_fraction(&ctx, 2, 7) - 0.1).abs() < 1e-6);

    ctx.data_mut(|data| data.insert_temp(clock_fraction, 0.05_f32));
    assert!((construction_preview_fraction(&ctx, 2, 8) - 0.05).abs() < 1e-6);
}

/// Preview the work accruing this month; completion still belongs to the monthly tick.
fn construction_completion(
    ui: &egui::Ui,
    project: &ConstructionProject,
    config: &EconomyConfig,
    pace: ConstructionPace,
    province: usize,
    month: u32,
) -> f32 {
    let fraction = construction_preview_fraction(ui.ctx(), province, month);
    let (progress, required) = project.progress();
    ((progress + project.speed(config, pace) * f64::from(fraction)) / required.max(0.001))
        .clamp(0.0, 0.9999) as f32
}

#[derive(Clone, Copy)]
enum BuildingCellAction {
    Build,
}

struct BuildingHover {
    level: Option<u32>,
    reason: Option<&'static str>,
    description: &'static str,
    effects: String,
}

fn building_description(building: BuildingType) -> &'static str {
    use BuildingType::*;
    match building {
        Granary => "A sheltered storehouse keeps the harvest dry and ready for lean seasons.",
        Warehouse => {
            "Sturdy storerooms hold metal, stone and supplies for the province's workshops."
        },
        Road => {
            "A paved road connects settlements, carrying merchants, messengers and marching armies."
        },
        Aqueduct => "Stone channels bring fresh water from distant springs to growing settlements.",
        Forum => {
            "A public square brings together civic business, debate and the bustle of daily life."
        },
        Baths => "Public baths offer a place to wash, unwind and meet neighbours.",
        UrbanMarket => "Market stalls make the province a more attractive place to settle.",
        Temple => {
            "A sanctuary welcomes worshippers, offerings and the province's religious festivals."
        },
        Arena => "An arena gathers crowds for spectacles, contests and public celebrations.",
        CityWalls => "Strong walls and guarded gates shelter the city from approaching enemies.",
        Academy => "A place of study where scholars share ideas and teach the next generation.",
        CityHall => "Civic officials manage records, levies and the province's tax collection.",
    }
}

fn wonder_description(wonder: usize) -> &'static str {
    match wonder {
        0 => "A towering stone pyramid stands as an enduring monument to royal ambition.",
        1 => "A circle of great standing stones creates a solemn gathering place beneath the open sky.",
        2 => "A grand sanctuary honours Zeus with stately columns and a richly adorned hall.",
        3 => "A royal palace brings together grand halls, courtyards and the splendour of the Argead court.",
        4 => "A colossal bronze figure watches over Rhodes, celebrating the island's pride.",
        5 => "Ranks of stone arches carry fresh water across the landscape to Segovia.",
        6 => "A monumental bridge of arches carries an aqueduct high above the river.",
        _ => "A landmark built to inspire generations and celebrate the province's achievements.",
    }
}

fn wonder_effects_text(definition: &WonderDefinition) -> String {
    format!(
        "+{} Influence\n+{} Influence/month",
        effect_number(definition.completion_influence),
        effect_number(definition.monthly_influence)
    )
}

fn effect_number(value: f64) -> String {
    format!("{value:.2}").trim_end_matches('0').trim_end_matches('.').to_owned()
}

fn building_cost_order<'a>(
    definitions: impl Iterator<Item = &'a BuildingDefinition>,
    province: &EconomicProvince,
) -> Vec<&'a BuildingDefinition> {
    let mut ordered: Vec<_> = definitions.collect();
    ordered.sort_by(|a, b| {
        let a_quote = a.quote(province.planned_building_level(a.building));
        let b_quote = b.quote(province.planned_building_level(b.building));
        (a_quote.stone + a_quote.metal)
            .total_cmp(&(b_quote.stone + b_quote.metal))
            .then_with(|| a_quote.stone.total_cmp(&b_quote.stone))
            .then_with(|| (a.building as usize).cmp(&(b.building as usize)))
    });
    ordered
}

fn resource_shortage(
    world: &EconomyWorld,
    player: usize,
    stone: f64,
    metal: f64,
) -> Option<&'static str> {
    let resources = world.players.get(player).map(|wallet| wallet.resources).unwrap_or([0.0; 3]);
    match (resources[2] < stone, resources[1] < metal) {
        (true, true) => Some("Not enough Stone and Metal."),
        (true, false) => Some("Not enough Stone."),
        (false, true) => Some("Not enough Metal."),
        _ => None,
    }
}

/// Keep four columns, shrinking the contents only when the viewport is narrow.
fn building_grid_layout(available_width: f32, scale: f32) -> (usize, f32) {
    let row_width =
        BUILDING_COLUMNS as f32 * BUILDING_CELL_SIZE.x + (BUILDING_COLUMNS - 1) as f32 * 8.0;
    (BUILDING_COLUMNS, scale.min(available_width / row_width))
}

fn building_section_header(ui: &mut egui::Ui, title: &str, scale: f32) {
    super::policy_widgets::section(ui, scale, title);
}

fn building_grid(
    ui: &mut egui::Ui,
    empty: Option<&str>,
    count: usize,
    columns: usize,
    scale: f32,
) -> Vec<egui::Rect> {
    if count == 0 {
        if let Some(empty) = empty {
            ui.label(egui::RichText::new(empty).small().color(MUTED));
        }
        ui.add_space(12.0 * scale);
        return Vec::new();
    }
    let rows = count.div_ceil(columns);
    let gap = 8.0 * scale;
    let size = egui::vec2(
        (ui.available_width() - (columns - 1) as f32 * gap) / columns as f32,
        BUILDING_CELL_SIZE.y * scale,
    );
    let (grid, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), rows as f32 * (size.y + gap) - gap),
        egui::Sense::hover(),
    );
    ui.add_space(2.0 * scale);
    (0..count)
        .map(|index| {
            egui::Rect::from_min_size(
                grid.min
                    + egui::vec2(
                        (index % columns) as f32 * (size.x + gap),
                        (index / columns) as f32 * (size.y + gap),
                    ),
                size,
            )
        })
        .collect()
}

/// One interaction covers the illustration, name, level and all empty space.
fn building_cell(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    symbol: Icon,
    name: &str,
    level: &str,
    stone: f64,
    metal: f64,
    enabled: bool,
    hover: &BuildingHover,
    scale: f32,
) -> Option<BuildingCellAction> {
    use super::province_panel::{INK, RULE};
    let enabled = enabled && ui.is_enabled();
    let response = ui.interact(
        rect,
        ui.id().with(("building-cell", symbol)),
        if enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    let fill = super::campaign_widgets::paint_purchase_background(
        ui,
        rect,
        enabled,
        response.hovered(),
        response.is_pointer_button_down_on(),
        5.0 * scale,
        scale,
    );
    ui.painter().rect_stroke(
        rect,
        5.0 * scale,
        egui::Stroke::new(scale, RULE),
        egui::StrokeKind::Inside,
    );
    let long_name = ui
        .painter()
        .layout_no_wrap(name.to_owned(), egui::FontId::proportional(14.0 * scale), INK)
        .size()
        .x
        > rect.width() - 10.0 * scale;
    let (name_top, name_height) = if long_name {
        (78.0, 36.0)
    } else {
        (78.0, 18.0)
    };
    let content_offset =
        egui::vec2(0.0, (120.0_f32 - name_top - name_height).clamp(0.0, 14.0) * scale);
    let image = egui::Rect::from_center_size(
        egui::pos2(rect.center().x, rect.top() + 41.0 * scale),
        egui::Vec2::splat(70.0 * scale),
    )
    .translate(content_offset);
    ui.scope(|ui| {
        if !enabled {
            ui.set_opacity(0.85);
        }
        paint_icon(ui, symbol, image);
    });
    let ink = if enabled {
        INK
    } else {
        super::campaign_widgets::UNAVAILABLE_PURCHASE_INK
    };
    let text_row = |top: f32, height: f32| {
        egui::Rect::from_min_size(
            egui::pos2(rect.left() + 5.0 * scale, rect.top() + top * scale),
            egui::vec2(rect.width() - 10.0 * scale, height * scale),
        )
    };
    let name_rect = text_row(name_top, name_height).translate(content_offset);
    let mut font_size = 14.0 * scale;
    let name_galley = loop {
        let mut job = egui::text::LayoutJob::simple(
            name.to_owned(),
            egui::FontId::proportional(font_size),
            ink,
            name_rect.width(),
        );
        job.halign = egui::Align::Center;
        let galley = ui.painter().layout_job(job);
        if galley.rows.len() <= 2
            && galley.size().x <= name_rect.width()
            && galley.size().y <= name_rect.height()
        {
            break galley;
        }
        font_size *= 0.93;
    };
    let name_pos = name_rect.center() - name_galley.rect.center().to_vec2();
    ui.painter().with_clip_rect(ui.clip_rect().intersect(name_rect)).galley(
        name_pos,
        name_galley,
        ink,
    );
    if !level.is_empty() && matches!(symbol, Icon::Building(_)) {
        let label = ui.painter().layout_no_wrap(
            level.to_owned(),
            egui::FontId::proportional(11.0 * scale),
            MUTED,
        );
        let position =
            egui::pos2(rect.right() - 6.0 * scale - label.size().x, rect.top() + 5.0 * scale);
        let badge = egui::Rect::from_min_size(position, label.size()).expand(2.0 * scale);
        ui.painter().rect_filled(badge, 2.0 * scale, fill);
        ui.painter().galley(position, label, MUTED);
    } else if !level.is_empty() {
        ledger_text(ui, text_row(96.0, 14.0), level, 11.0 * scale, MUTED, egui::Align::Center);
    }
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            enabled,
            format!("Build {name}, {level}, {stone:.0} Stone and {metal:.0} Metal"),
        )
    });
    let response = response.inspection_hover_ui(|ui| {
        let width = (440.0 * scale).min(ui.ctx().content_rect().width() - 24.0);
        ui.set_width(width.max(120.0));
        ui.horizontal_top(|ui| {
            super::campaign_widgets::icon(ui, symbol, 112.0 * scale);
            ui.add_space(8.0 * scale);
            let width = ui.available_width();
            ui.vertical(|ui| {
                ui.set_width(width);
                ui.add(
                    egui::Label::new(egui::RichText::new(name).strong().size(20.0 * scale)).wrap(),
                );
                ui.horizontal_wrapped(|ui| {
                    for (symbol, cost, name) in
                        [(Icon::Stone, stone, "Stone"), (Icon::Metal, metal, "Metal")]
                    {
                        if cost == 0.0 {
                            continue;
                        }
                        super::campaign_widgets::icon(ui, symbol, 28.0 * scale)
                            .inspection_hover_text(name);
                        let cost = if !cost.is_finite() {
                            "—".into()
                        } else if cost >= 100_000.0 {
                            format!("{cost:.1e}")
                        } else {
                            format!("{cost:.0}")
                        };
                        ui.label(egui::RichText::new(cost).strong().size(17.0 * scale));
                    }
                });
                ui.add_space(10.0 * scale);
                ui.add(
                    egui::Label::new(egui::RichText::new(hover.description).size(14.0 * scale))
                        .wrap(),
                );
                if let Some(reason) = hover.reason {
                    ui.add_space(10.0 * scale);
                    super::campaign_widgets::unavailable_reason(ui, reason, scale);
                }
                ui.add_space(6.0 * scale);
                ui.strong(
                    egui::RichText::new(if hover.level.is_some() {
                        "Each completed level:"
                    } else {
                        "When completed:"
                    })
                    .size(14.0 * scale),
                );
                for effect in hover.effects.lines() {
                    ui.horizontal_top(|ui| {
                        ui.label(egui::RichText::new("•").size(14.0 * scale));
                        ui.add(
                            egui::Label::new(egui::RichText::new(effect).size(14.0 * scale)).wrap(),
                        );
                    });
                }
            });
        });
    });
    let response = if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    };
    if enabled && response.clicked() {
        Some(BuildingCellAction::Build)
    } else {
        None
    }
}

/// Separate construction from the province image and building cards.
pub(in crate::app) fn construction_queue_view(
    ui: &mut egui::Ui,
    world: &EconomyWorld,
    province: usize,
    owned: bool,
    scale: f32,
) -> Option<WorkQueueAction> {
    let p = &world.provinces[province];
    if p.construction.is_none() && p.construction_queue.is_empty() {
        return None;
    }
    building_section_header(ui, "Construction", scale);
    let details = |project: &ConstructionProject| match project {
        ConstructionProject::Building(project) => (
            Icon::Building(project.building),
            format!("{} level {}", project.building.name(), project.target_level),
        ),
        ConstructionProject::Wonder(project) => (
            Icon::Wonder(project.wonder_id),
            crate::map::wonder_name(project.wonder_id).unwrap_or("Wonder").to_owned(),
        ),
    };
    let active = p.construction.as_ref().map(|project| {
        let (art, name) = details(project);
        (
            art,
            if p.occupied {
                let (progress, required) = project.progress();
                (progress / required.max(0.001)).clamp(0.0, 0.9999) as f32
            } else {
                construction_completion(
                    ui,
                    project,
                    &world.config,
                    p.policies.construction,
                    province,
                    world.month,
                )
            },
            format!(
                "{name}, {} months left",
                project.months_remaining(&world.config, p.policies.construction)
            ),
        )
    });
    let queued: Vec<_> = p
        .construction_queue
        .iter()
        .enumerate()
        .map(|(index, project)| {
            let (art, name) = details(project);
            (index, art, name)
        })
        .collect();
    work_queue(
        ui,
        egui::Id::new(("construction-queue", province)),
        if p.occupied {
            "Construction paused: enemy occupation"
        } else {
            "In progress"
        },
        active,
        &queued,
        owned && !p.occupied,
        false,
        scale,
    )
}

/// Four equal columns fill each City, Countryside and Wonders section.
pub(in crate::app) fn readonly_buildings(
    ui: &mut egui::Ui,
    world: &EconomyWorld,
    province: usize,
    scale: f32,
) {
    let p = &world.provinces[province];
    let (columns, scale) = building_grid_layout(ui.available_width().max(1.0), scale);
    ui.spacing_mut().item_spacing.y = 0.0;
    for (title, empty, city) in [
        ("City", None, true),
        ("Countryside", Some("No countryside improvements available."), false),
    ] {
        let definitions = building_cost_order(
            world
                .config
                .buildings
                .iter()
                .filter(|d| d.requires_city == city && (!city || p.has_city)),
            p,
        );
        if definitions.is_empty() && empty.is_none() {
            continue;
        }
        building_section_header(ui, title, scale);
        let cells = building_grid(ui, empty, definitions.len(), columns, scale);
        for (definition, cell) in definitions.into_iter().zip(cells) {
            let level = p.level(definition.building);
            let quote = definition.quote(p.planned_building_level(definition.building));
            let started = matches!(&p.construction, Some(ConstructionProject::Building(project)) if project.building == definition.building);
            let hover = BuildingHover {
                level: Some(level),
                reason: Some(if started {
                    "Being constructed."
                } else {
                    "Province not owned."
                }),
                description: building_description(definition.building),
                effects: building_effects_text(definition),
            };
            building_cell(
                ui,
                cell,
                Icon::Building(definition.building),
                definition.building.name(),
                &format!("Level {level}"),
                quote.stone,
                quote.metal,
                false,
                &hover,
                scale,
            );
        }
    }
    let cells = if p.wonder_sites.is_empty() {
        Vec::new()
    } else {
        building_section_header(ui, "Wonders", scale);
        building_grid(ui, None, p.wonder_sites.len(), columns, scale)
    };
    for (&wonder, cell) in p.wonder_sites.iter().zip(cells) {
        let Some(definition) = world.config.wonders.iter().find(|d| d.wonder_id == wonder) else {
            continue;
        };
        let completed = p.completed_wonder == Some(wonder);
        let started = matches!(&p.construction, Some(ConstructionProject::Wonder(w)) if w.wonder_id == wonder);
        let hover = BuildingHover {
            level: None,
            reason: Some(if completed {
                "Already built."
            } else if started {
                "Being constructed."
            } else {
                "Province not owned."
            }),
            description: wonder_description(wonder),
            effects: wonder_effects_text(definition),
        };
        building_cell(
            ui,
            cell,
            Icon::Wonder(wonder),
            crate::map::wonder_name(wonder).unwrap_or("Wonder"),
            "",
            definition.stone_cost,
            definition.metal_cost,
            false,
            &hover,
            scale,
        );
    }
}

/// Four equal columns fill each City, Countryside and Wonders section.
pub(in crate::app) fn buildings(
    ui: &mut egui::Ui,
    world: &mut EconomyWorld,
    province: usize,
    player: usize,
    scale: f32,
) -> Option<String> {
    let p = world.provinces.get(province)?.clone();
    let owned = p.can_administer(player);
    let definitions: Vec<_> =
        world.config.buildings.iter().filter(|d| !d.requires_city || p.has_city).cloned().collect();
    let wonders: Vec<_> = p
        .wonder_sites
        .iter()
        .filter_map(|id| world.config.wonders.iter().find(|d| d.wonder_id == *id))
        .cloned()
        .collect();
    let (columns, scale) = building_grid_layout(ui.available_width().max(1.0), scale);
    ui.spacing_mut().item_spacing.y = 0.0;
    let mut message = None;
    let mut building_action = None;
    let mut wonder_action = None;
    for (title, empty, city) in [
        ("City", None, true),
        ("Countryside", Some("No countryside improvements available."), false),
    ] {
        let definitions =
            building_cost_order(definitions.iter().filter(|d| d.requires_city == city), &p);
        if definitions.is_empty() && empty.is_none() {
            continue;
        }
        building_section_header(ui, title, scale);
        let cells = building_grid(ui, empty, definitions.len(), columns, scale);
        for (definition, cell) in definitions.iter().zip(cells) {
            let level = p.level(definition.building);
            let quote = definition.quote(p.planned_building_level(definition.building));
            let affordable = world
                .players
                .get(player)
                .is_some_and(|w| w.resources[1] >= quote.metal && w.resources[2] >= quote.stone);
            let supported = level.checked_add(1).is_some()
                && quote.stone.is_finite()
                && quote.metal.is_finite()
                && quote.required_progress.is_finite();
            let active = matches!(&p.construction, Some(ConstructionProject::Building(project)) if project.building == definition.building);
            let reason = if p.occupied {
                Some("Enemy occupation blocks construction.")
            } else if !owned {
                Some("Province not owned.")
            } else if p.construction_queue_full() {
                Some("Construction queue is full (maximum 10 orders).")
            } else if !supported {
                Some("Further upgrades exceed the supported numeric range.")
            } else {
                resource_shortage(world, player, quote.stone, quote.metal)
            };
            let hover = BuildingHover {
                level: Some(level),
                reason,
                description: building_description(definition.building),
                effects: building_effects_text(definition),
            };
            let name = definition.building.name();
            let level_label = if active {
                format!("Level {level} → {}", level.saturating_add(1))
            } else {
                format!("Level {level}")
            };
            match building_cell(
                ui,
                cell,
                Icon::Building(definition.building),
                name,
                &level_label,
                quote.stone,
                quote.metal,
                owned && affordable && supported && !p.construction_queue_full(),
                &hover,
                scale,
            ) {
                Some(BuildingCellAction::Build) => building_action = Some(definition.building),
                None => {},
            }
        }
    }
    let cells = if wonders.is_empty() {
        Vec::new()
    } else {
        building_section_header(ui, "Wonders", scale);
        building_grid(ui, None, wonders.len(), columns, scale)
    };
    for (definition, cell) in wonders.iter().zip(cells) {
        let wonder = definition.wonder_id;
        let completed = p.completed_wonder == Some(wonder);
        let active = matches!(&p.construction, Some(ConstructionProject::Wonder(project)) if project.wonder_id == wonder);
        let queued = p
            .construction_queue
            .iter()
            .any(|project| matches!(project, ConstructionProject::Wonder(_)));
        let name = crate::map::wonder_name(wonder).unwrap_or("Wonder");
        let affordable = world.players.get(player).is_some_and(|w| {
            w.resources[1] >= definition.metal_cost && w.resources[2] >= definition.stone_cost
        });
        let reason = if completed {
            Some("Already built.")
        } else if active {
            Some("Being constructed.")
        } else if p.occupied {
            Some("Enemy occupation blocks construction.")
        } else if !owned {
            Some("Province not owned.")
        } else if p.completed_wonder.is_some() {
            Some("This province already has a completed wonder.")
        } else if queued {
            Some("A wonder is already queued.")
        } else if p.construction_queue_full() {
            Some("Construction queue is full (maximum 10 orders).")
        } else {
            resource_shortage(world, player, definition.stone_cost, definition.metal_cost)
        };
        let hover = BuildingHover {
            level: None,
            reason,
            description: wonder_description(wonder),
            effects: wonder_effects_text(definition),
        };
        match building_cell(
            ui,
            cell,
            Icon::Wonder(wonder),
            name,
            "",
            definition.stone_cost,
            definition.metal_cost,
            owned
                && p.completed_wonder.is_none()
                && !active
                && !queued
                && affordable
                && !p.construction_queue_full(),
            &hover,
            scale,
        ) {
            Some(BuildingCellAction::Build) => wonder_action = Some(wonder),
            None => {},
        }
    }
    if let Some(building) = building_action {
        match world.start_building(player, province, building) {
            Ok(()) => {
                if p.construction.is_none() {
                    record_construction_preview_start(ui.ctx(), province, world.month);
                }
                super::audio_controls::request_construction_sound(ui.ctx());
            },
            Err(error) => message = Some(error),
        }
    } else if let Some(wonder) = wonder_action {
        match world.start_wonder(player, province, wonder) {
            Ok(()) => {
                if p.construction.is_none() {
                    record_construction_preview_start(ui.ctx(), province, world.month);
                }
                super::audio_controls::request_construction_sound(ui.ctx());
            },
            Err(error) => message = Some(error),
        }
    } else if let Some(ConstructionProject::Wonder(wonder)) = &p.construction {
        message = wonder_labor_controls(ui, world, province, player, wonder, scale);
    }
    message
}

/// Apply the banner's cancellation and reset the next project's progress preview.
pub(in crate::app) fn cancel_construction_order(
    ctx: &egui::Context,
    world: &mut EconomyWorld,
    province: usize,
    player: usize,
    action: WorkQueueAction,
) -> String {
    let result = match action {
        WorkQueueAction::CancelActive => {
            let result = world.cancel_construction(player, province);
            if result.is_ok() {
                record_construction_preview_start(ctx, province, world.month);
            }
            result
        },
        WorkQueueAction::CancelQueued(index) => {
            world.cancel_queued_construction(player, province, index)
        },
    };
    result.map(|()| "Construction cancelled.".into()).unwrap_or_else(|error| error)
}

/// Keep labor assignment beside the wonder grid after removing the duplicate project strip.
fn wonder_labor_controls(
    ui: &mut egui::Ui,
    world: &mut EconomyWorld,
    province: usize,
    player: usize,
    wonder: &WonderProject,
    scale: f32,
) -> Option<String> {
    let (rect, _) = ui
        .allocate_exact_size(egui::vec2(ui.available_width(), 36.0 * scale), egui::Sense::hover());
    let owned = world.provinces[province].can_administer(player);
    let mut message = None;
    let icon_rect = egui::Rect::from_min_size(
        rect.min + egui::vec2(0.0, 6.0) * scale,
        egui::vec2(24.0, 24.0) * scale,
    );
    paint_icon(ui, Icon::Slaves, icon_rect);
    let slider_rect = egui::Rect::from_min_max(
        rect.min + egui::vec2(32.0, 5.0) * scale,
        rect.max - egui::vec2(0.0, 5.0) * scale,
    );
    ui.scope_builder(egui::UiBuilder::new().max_rect(slider_rect), |ui| {
        if !owned {
            ui.disable();
        }
        ui.spacing_mut().interact_size.x = 40.0 * scale;
        ui.spacing_mut().slider_width = (slider_rect.width() - 90.0 * scale).max(0.0);
        let mut assigned = wonder.assigned_slaves;
        let response = ui
            .add(
                egui::Slider::new(&mut assigned, 0.0..=world.provinces[province].population[3])
                    .custom_formatter(|value, _| super::resource_hud::format_population(value)),
            )
            .inspection_hover_text(wonder_labor_tooltip(&world.config));
        if response.changed() {
            if let Err(error) = world.assign_wonder_slaves(player, province, assigned) {
                message = Some(error);
            }
        }
    });
    message
}

/// Compact descriptions are generated from the actual configured benefits.
fn building_effects_text(definition: &BuildingDefinition) -> String {
    let mut text = effects_text(&definition.effects);
    let senate_support = match definition.building {
        BuildingType::Forum => Some("Aristocrat support in the Senate"),
        BuildingType::UrbanMarket => Some("Merchant support in the Senate"),
        _ => None,
    };
    if let Some(support) = senate_support {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(support);
    }
    if definition.building == BuildingType::Road {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&format!(
            "+{}% army travel speed through this province",
            effect_number(
                crate::game::military::MilitaryConfig::default().road_speed_bonus * 100.0
            )
        ));
    }
    text
}

fn effects_text(effects: &BuildingEffects) -> String {
    let mut parts = Vec::new();
    for (index, name) in RESOURCE_NAMES.iter().take(3).enumerate() {
        if effects.storage[index] > 0.0 {
            parts.push(format!("+{:.0} {name} storage", effects.storage[index]));
        }
        if effects.production[index] > 0.0 {
            parts.push(format!("+{:.0}% {name} output", effects.production[index] * 100.0));
        }
    }
    if effects.capacity > 0.0 {
        parts.push(format!("+{:.0} population capacity", effects.capacity));
    }
    if effects.happiness.iter().any(|value| *value > 0.0) {
        if effects.happiness.windows(2).all(|pair| pair[0] == pair[1]) {
            parts.push(format!(
                "+{} happiness for every population class each month",
                effect_number(effects.happiness[0])
            ));
        } else if effects.happiness[..3].windows(2).all(|pair| pair[0] == pair[1])
            && effects.happiness[0] > 0.0
            && effects.happiness[3] == 0.0
        {
            parts.push(format!(
                "+{} happiness for Nobles, Citizens and Plebeians each month",
                effect_number(effects.happiness[0])
            ));
        } else {
            for (name, value) in CLASS_NAMES.iter().zip(effects.happiness) {
                if value != 0.0 {
                    parts.push(format!("+{} {name} happiness each month", effect_number(value)));
                }
            }
        }
    }
    if effects.influence > 0.0 {
        parts.push(format!("+{} Influence/month", effect_number(effects.influence)));
    }
    if effects.tax > 0.0 {
        parts.push(format!("+{:.0}% taxes", effects.tax * 100.0));
    }
    if effects.defense > 0.0 {
        parts.push(format!("+{:.0}% defense", effects.defense * 100.0));
    }
    if effects.migration > 0.0 {
        parts.push(format!(
            "+{}% attraction for incoming migrants",
            effect_number(effects.migration * 100.0)
        ));
    }
    parts.join("\n")
}

/// Slave assignment uses the configured thresholds shown beside the live slider.
fn wonder_labor_tooltip(config: &EconomyConfig) -> String {
    let tiers = config
        .wonder_slave_speeds
        .iter()
        .map(|(slaves, speed)| format!("{slaves:.0}+ Slaves: {}× work", effect_number(*speed)))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{tiers}\nAssignments below the next tier do not increase speed. Assigned Slaves keep consuming Food and stop ordinary resource production.")
}

#[cfg(test)]
mod effect_text_tests {
    use super::*;

    #[test]
    fn building_and_wonder_effects_show_compact_recurring_values() {
        assert_eq!(
            building_effects_text(&BuildingDefinition::for_type(BuildingType::Forum)),
            "+1 Influence/month\nAristocrat support in the Senate"
        );
        assert_eq!(
            building_effects_text(&BuildingDefinition::for_type(BuildingType::Baths)),
            "+150 population capacity\n+2 happiness for Nobles, Citizens and Plebeians each month"
        );
        assert_eq!(
            building_effects_text(&BuildingDefinition::for_type(BuildingType::Academy)),
            "+1 happiness for Nobles, Citizens and Plebeians each month\n+0.25 Influence/month"
        );
        assert_eq!(
            building_effects_text(&BuildingDefinition::for_type(BuildingType::UrbanMarket)),
            "+10% attraction for incoming migrants\nMerchant support in the Senate"
        );
        assert_eq!(
            building_effects_text(&BuildingDefinition::for_type(BuildingType::CityHall)),
            "+10% taxes"
        );
        assert_eq!(
            building_effects_text(&BuildingDefinition::for_type(BuildingType::Road)),
            "+20% army travel speed through this province"
        );
        assert_eq!(
            wonder_effects_text(&WonderDefinition::for_site(0)),
            "+250 Influence\n+3 Influence/month"
        );
    }
}
