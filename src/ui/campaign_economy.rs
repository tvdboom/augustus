//! Icon-led economy and construction tables for the parchment province panel.

use crate::game::economy::*;
use bevy_egui::egui;

use super::campaign_widgets::{paint_icon, Icon};

const CLASS_NAMES: [&str; 4] = ["Nobles", "Citizens", "Plebeians", "Slaves"];
const CLASS_ICONS: [Icon; 4] = [Icon::Nobles, Icon::Citizens, Icon::Plebeians, Icon::Slaves];
const RESOURCE_NAMES: [&str; 3] = ["Food", "Metal", "Stone"];
const RESOURCE_ICONS: [Icon; 3] = [Icon::Food, Icon::Metal, Icon::Stone];
const MUTED: egui::Color32 = egui::Color32::from_rgb(112, 91, 71);
pub(in crate::app) const CIVIC_POWER: &str = "Civic Power";

/// Budget the fixed rows before rendering, including owned wallets and construction.
pub(in crate::app) fn overview_height(world: &EconomyWorld, province: usize) -> f32 {
    let p = &world.provinces[province];
    let civic_power = if p.owner.and_then(|id| world.players.get(id)).is_some() {
        12.0 + 28.0 + 2.0 * 34.0
    } else {
        0.0
    };
    48.0 + 10.0
        + 28.0
        + 4.0 * 34.0
        + 12.0
        + 28.0
        + 3.0 * 34.0
        + civic_power
        + if p.construction.is_some() {
            50.0
        } else {
            0.0
        }
}

/// Paint a value within its column, reducing type only for unusually large numbers.
fn ledger_text(
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

/// Equal-width badges use all available space and never wrap onto a second row.
fn overview_badge(
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
    let image = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 18.0 * scale, rect.center().y),
        egui::vec2(25.0, 25.0) * scale,
    );
    paint_icon(ui, symbol, image);
    let text = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 34.0 * scale, rect.top()),
        rect.max - egui::vec2(5.0 * scale, 0.0),
    );
    ledger_text(
        ui,
        egui::Rect::from_min_max(text.min, egui::pos2(text.right(), text.top() + 23.0 * scale)),
        caption,
        11.5 * scale,
        MUTED,
        egui::Align::Center,
    );
    ledger_text(
        ui,
        egui::Rect::from_min_max(egui::pos2(text.left(), text.top() + 22.0 * scale), text.max),
        value,
        16.0 * scale,
        value_color,
        egui::Align::Center,
    );
    ui.interact(rect, ui.id().with(caption), egui::Sense::hover()).on_hover_text(tip);
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
            ui.interact(rect, ui.id().with(tip), egui::Sense::hover()).on_hover_text(tip);
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
    let image = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 20.0 * scale, rect.center().y),
        egui::vec2(28.0, 28.0) * scale,
    );
    paint_icon(ui, symbol, image);
    ui.interact(image, ui.id().with(name), egui::Sense::hover()).on_hover_text(name);
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
    .on_hover_text(tip);
}

/// Population and resource ledgers fill the inspector without a scroll container.
pub(in crate::app) fn overview(
    ui: &mut egui::Ui,
    world: &EconomyWorld,
    province: usize,
    scale: f32,
) {
    let Some(p) = world.provinces.get(province) else {
        return;
    };
    let capacity = p.capacity(&world.config);
    let ratio = p.total_population() / capacity;
    let last = world.last_report.province_reports.get(province);
    let ink = super::province_panel::INK;
    ui.spacing_mut().item_spacing.y = 0.0;
    let (badges, _) = ui
        .allocate_exact_size(egui::vec2(ui.available_width(), 48.0 * scale), egui::Sense::hover());
    let gap = 6.0 * scale;
    let badge_width = (badges.width() - 2.0 * gap) / 3.0;
    let badge = |index: usize| {
        egui::Rect::from_min_size(
            badges.min + egui::vec2(index as f32 * (badge_width + gap), 0.0),
            egui::vec2(badge_width, badges.height()),
        )
    };
    overview_badge(ui, badge(0), Icon::Population, "Population", &format!("{} / {}", super::resource_hud::format_population(p.total_population()), super::resource_hud::format_population(capacity)),
            &format!("Population / comfortable capacity ({:.0}% occupied).\nArea {:.1} × scale {:.1} × terrain {:.2}; city +{:.0}; buildings +{:.0}.\nOvercrowding lowers happiness and births and encourages migration. Capacity is not a hard cap.",
                ratio * 100.0, p.capacity_area, world.config.area_to_capacity_scale,
                world.config.terrain_capacity[p.terrain as usize], if p.has_city {world.config.city_capacity} else {0.0},
                p.building_effects(&world.config).capacity), if p.total_population() > capacity { super::resource_hud::hud_delta_color(-1.0) } else { ink }, scale);
    overview_badge(ui, badge(1), Icon::Food, "Food demand", &format!("−{:.1}/mo", p.food_request(&world.config)),
            "Civilian Food request. All owned provinces and military share the same proportional supply. Construction-assigned slaves still eat.", ink, scale);
    overview_badge(ui, badge(2), Icon::Food, "Food supplied", &last.map_or_else(|| "—".to_owned(), |last| format!("{:.0}%", (last.food_supply_ratio * 100.0).floor())),
            "Last month's Food fulfillment. Partial shortages proportionally lower happiness and births and cause famine deaths. A dash means no month has resolved yet.", if last.is_some_and(|last| last.food_supply_ratio < 1.0) { super::resource_hud::hud_delta_color(-1.0) } else { ink }, scale);
    ui.add_space(10.0 * scale);
    ledger_header(ui, "Population", &[(Icon::Amount, "Amount: total population"), (Icon::Delta, "Monthly population change: births minus normal/famine deaths plus migration. Automatic class changes are excluded."), (Icon::Happiness, "Class happiness")], scale);
    for class in 0..4 {
        ledger_row(ui, 4, 34.0 * scale, class % 2 == 0, |ui, cells| {
            ledger_name(ui, cells[0], CLASS_ICONS[class], CLASS_NAMES[class], scale);
            ledger_value(
                ui,
                cells[1],
                &super::resource_hud::format_population(p.population[class]),
                "Total population in this class.",
                ink,
                scale,
            );
            if let Some(month) = last {
                let change =
                    month.births[class] - month.normal_deaths[class] - month.famine_deaths[class]
                        + month.migration[class];
                ledger_value(ui, cells[2], &super::resource_hud::format_population_delta(change), &format!(
                    "Births +{}\nNatural deaths −{}\nFamine deaths −{}\nMigration {}\nClass promotions/manumission separately conserve total population. Displayed counts round down.",
                    super::resource_hud::format_population(month.births[class]), super::resource_hud::format_population(month.normal_deaths[class]), super::resource_hud::format_population(month.famine_deaths[class]), super::resource_hud::format_population_delta(month.migration[class])), super::resource_hud::hud_delta_color(change), scale);
            } else {
                ledger_value(ui, cells[2], "—", "No month has resolved yet.", MUTED, scale);
            }
            let overcrowding = ((ratio - 1.0).max(0.0) * world.config.overcrowding_scale)
                .min(world.config.overcrowding_cap);
            let supply = last.map_or(1.0, |month| month.food_supply_ratio);
            let food = world.config.food_policy[p.policies.food as usize];
            let slave = world.config.slave_policy[p.policies.slave_labor as usize];
            ledger_value(ui, cells[3], &format!("{:.0}%", p.happiness[class]), &format!(
                "Neutral 50\nFood policy {:+.1}\nBuildings {:+.1}\nEvents / unrest {:+.1}\nSlave labor {:+.1}\nNoble taxes {:+.1}\nOvercrowding −{overcrowding:.1}\nFood shortage −{:.1}\nBirth factors: happiness {:.2}×, space {:.2}×, rations {:.2}×, supply {:.2}×.\nSpace factor is min(1, {:.2} / population-to-capacity ratio). Happiness and space never directly increase natural mortality.",
                food.happiness, p.building_effects(&world.config).happiness[class],
                p.happiness_modifiers[class] + p.temporary_happiness[class], if class == 3 {slave.happiness} else {0.0},
                if class == 0 { p.noble_tax_happiness } else { 0.0 },
                (1.0-supply)*world.config.shortage_happiness_penalty, birth_modifier(p.happiness[class]),
                p.crowding_birth_modifier(&world.config), food.births, supply, world.config.crowding_birth_threshold), ink, scale);
        });
    }
    let (labor, production) = p.production(&world.config);
    let owner = p.owner.and_then(|id| world.players.get(id));
    if let Some(owner) = owner {
        ui.add_space(12.0 * scale);
        ledger_header(ui, CIVIC_POWER, &[(Icon::Amount, "Amount: shared Influence and Coin. Neither has a storage cap."), (Icon::Delta, "Monthly provincial Influence income or taxes. Player-wide net changes also include trade, spending, rank and vassal income.")], scale);
        let noble = p.population[0] * world.config.influence_per_noble;
        let buildings = p.building_effects(&world.config).influence;
        let wonder = p
            .completed_wonder
            .and_then(|id| world.config.wonders.iter().find(|d| d.wonder_id == id))
            .map_or(0.0, |d| d.monthly_influence);
        ledger_row(ui, 3, 34.0 * scale, true, |ui, cells| {
            ledger_name(ui, cells[0], Icon::Influence, "Influence", scale);
            ledger_value(
                ui,
                cells[1],
                &format!("{:.0}", owner.influence),
                "Player Influence; no storage cap.",
                ink,
                scale,
            );
            ledger_value(ui, cells[2], &format!("+{:.2}",noble+buildings+wonder), &format!(
                "Nobles: {} × {:.2} = +{noble:.2}\nCity buildings +{buildings:.2}\nCompleted wonder +{wonder:.2}\nRank and vassal income are additional player sources.",
                super::resource_hud::format_population(p.population[0]),world.config.influence_per_noble), super::resource_hud::hud_delta_color(noble+buildings+wonder), scale);
        });
        ledger_row(ui, 3, 34.0 * scale, false, |ui, cells| {
            ledger_name(ui, cells[0], Icon::Coin, "Coin", scale);
            ledger_value(
                ui,
                cells[1],
                &format!("{:.0}", owner.coin),
                "Player Coin treasury; no storage cap.",
                ink,
                scale,
            );
            ledger_value(ui, cells[2], &format!("+{:.1}", p.tax_income(&world.config)), &format!(
                "Monthly provincial taxes. Per Noble {:.2}, Citizen {:.2}, Plebeian {:.2}; no direct slave tax.\nBuilding tax multiplier {:.2}×.",
                world.config.tax_rates[0] * p.noble_tax_multiplier, world.config.tax_rates[1], world.config.tax_rates[2],1.0+p.building_effects(&world.config).tax), super::resource_hud::hud_delta_color(p.tax_income(&world.config)), scale);
        });
    }
    ui.add_space(12.0 * scale);
    ledger_header(ui, "Resources", &[(Icon::Amount, "Amount: total shared stock / storage capacity."), (Icon::Delta, "Monthly provincial production. Player-wide net changes also include trade, consumption and spending.")], scale);
    for resource in 0..3 {
        ledger_row(ui, 3, 34.0 * scale, resource % 2 == 0, |ui, cells| {
            ledger_name(ui, cells[0], RESOURCE_ICONS[resource], RESOURCE_NAMES[resource], scale);
            ledger_value(ui, cells[2], &format!("+{:.1}", production[resource]), &format!(
                "Labor {} × potential {:.1} × scale {:.2} × building multiplier {:.2}.\nLabor is allocated only once across all three sectors; zero-potential sectors get none.",
                super::resource_hud::format_population(labor[resource]), p.potential[resource], world.config.production_scale[resource], 1.0+p.building_effects(&world.config).production[resource]), super::resource_hud::hud_delta_color(production[resource]), scale);
            if let Some(owner) = owner {
                ledger_value(ui, cells[1], &format!("{:.0} / {:.0}", owner.resources[resource], owner.storage[resource]),
                    "Shared player stockpile and maximum storage. Province storage buildings add global capacity. Excess is discarded after production, trade and consumption.", ink, scale);
            } else {
                ledger_value(ui, cells[1], "—", "NPC resources use monthly production, export capacity and import demand instead of physical stockpiles.", MUTED, scale);
            }
        });
    }
    if let Some(project) = &p.construction {
        ui.add_space(8.0 * scale);
        overview_project(ui, project, &world.config, p.policies.construction, scale);
    }
}

/// Overview construction remains one bounded row even at compact panel sizes.
pub(in crate::app) fn overview_project(
    ui: &mut egui::Ui,
    project: &ConstructionProject,
    config: &EconomyConfig,
    pace: ConstructionPace,
    scale: f32,
) {
    let (progress, required) = project.progress();
    let name = match project {
        ConstructionProject::Building(p) => {
            format!("{} · Lv {}", p.building.name(), p.target_level)
        },
        ConstructionProject::Wonder(p) => {
            crate::map::wonder_name(p.wonder_id).unwrap_or("Wonder").to_owned()
        },
    };
    let (rect, response) = ui
        .allocate_exact_size(egui::vec2(ui.available_width(), 42.0 * scale), egui::Sense::hover());
    let completion = (progress / required.max(0.001)).clamp(0.0, 1.0) as f32;
    let track = egui::Rect::from_min_max(rect.min + egui::vec2(0.0, 31.0 * scale), rect.max);
    ui.painter().rect_filled(track, 2.0 * scale, super::province_panel::TABLE_STRIPE);
    ui.painter().rect_filled(
        egui::Rect::from_min_size(
            track.min,
            egui::vec2(track.width() * completion, track.height()),
        ),
        2.0 * scale,
        egui::Color32::from_rgb(190, 150, 76),
    );
    ledger_text(
        ui,
        egui::Rect::from_min_max(rect.min, egui::pos2(rect.right(), track.top())),
        &format!(
            "{name} · {:.0}% · {}mo",
            completion * 100.0,
            project.months_remaining(config, pace)
        ),
        12.0 * scale,
        MUTED,
        egui::Align::Center,
    );
    response.on_hover_text(format!("{progress:.1}/{required:.1} work; {:.2} work/month.\nConstruction pace: {pace:?}. Progress remains with the province after capture. Slave acceleration has diminishing returns.", project.speed(config, pace)));
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

/// Keep four columns, shrinking the contents only when the viewport is narrow.
fn building_grid_layout(available_width: f32, scale: f32) -> (usize, f32) {
    let row_width =
        BUILDING_COLUMNS as f32 * BUILDING_CELL_SIZE.x + (BUILDING_COLUMNS - 1) as f32 * 8.0;
    (BUILDING_COLUMNS, scale.min(available_width / row_width))
}

/// Each category starts its own grid; empty categories without a message are hidden.
fn building_section(
    ui: &mut egui::Ui,
    title: &str,
    empty: Option<&str>,
    count: usize,
    columns: usize,
    scale: f32,
) -> Vec<egui::Rect> {
    use super::province_panel::{INK, RULE};
    if count == 0 && empty.is_none() {
        return Vec::new();
    }
    let (header, _) = ui
        .allocate_exact_size(egui::vec2(ui.available_width(), 26.0 * scale), egui::Sense::hover());
    ui.painter().text(
        header.left_center(),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(15.0 * scale),
        INK,
    );
    ui.painter().hline(header.x_range(), header.bottom(), egui::Stroke::new(scale, RULE));
    ui.add_space(6.0 * scale);
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
    ui.add_space(6.0 * scale);
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
    active: bool,
    progress: Option<(f32, u32)>,
    tooltip: &str,
    scale: f32,
) -> bool {
    use super::province_panel::{INK, RULE, TABLE_STRIPE};
    let response = ui.interact(
        rect,
        ui.id().with(("building-cell", symbol)),
        if enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    let gold = egui::Color32::from_rgb(190, 150, 76);
    let fill = if enabled && response.is_pointer_button_down_on() {
        egui::Color32::from_rgb(207, 180, 137)
    } else if response.hovered() {
        egui::Color32::from_rgb(226, 210, 180)
    } else {
        TABLE_STRIPE
    };
    ui.painter().rect_filled(rect, 5.0 * scale, fill);
    ui.painter().rect_stroke(
        rect,
        5.0 * scale,
        egui::Stroke::new(
            if active {
                2.0 * scale
            } else {
                scale
            },
            if active {
                gold
            } else {
                RULE
            },
        ),
        egui::StrokeKind::Inside,
    );
    let image = egui::Rect::from_center_size(
        egui::pos2(rect.center().x, rect.top() + 41.0 * scale),
        egui::vec2(70.0, 70.0) * scale,
    );
    ui.scope(|ui| {
        if !enabled && !active {
            ui.set_opacity(0.45);
        }
        paint_icon(ui, symbol, image);
    });
    let ink = if enabled || active {
        INK
    } else {
        MUTED
    };
    let text_row = |top: f32, height: f32| {
        egui::Rect::from_min_size(
            egui::pos2(rect.left() + 5.0 * scale, rect.top() + top * scale),
            egui::vec2(rect.width() - 10.0 * scale, height * scale),
        )
    };
    ledger_text(ui, text_row(78.0, 18.0), name, 14.0 * scale, ink, egui::Align::Center);
    ledger_text(ui, text_row(96.0, 14.0), level, 11.0 * scale, MUTED, egui::Align::Center);
    if let Some((completion, months)) = progress {
        let track = text_row(108.0, 11.0);
        ui.painter().rect_filled(track, 2.0 * scale, RULE);
        ui.painter().rect_filled(
            egui::Rect::from_min_size(
                track.min,
                egui::vec2(track.width() * completion, track.height()),
            ),
            2.0 * scale,
            gold,
        );
        ledger_text(
            ui,
            track,
            &format!("{:.0}% · {months}mo", completion * 100.0),
            9.0 * scale,
            INK,
            egui::Align::Center,
        );
    }
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            enabled,
            format!("Build {name}, {level}, {stone:.0} Stone and {metal:.0} Metal"),
        )
    });
    let response = response.on_hover_ui(|ui| {
        let width = (340.0 * scale).min(ui.ctx().content_rect().width() - 24.0);
        ui.set_width(width.max(120.0));
        ui.horizontal_top(|ui| {
            super::campaign_widgets::icon(ui, symbol, 64.0 * scale);
            let width = ui.available_width();
            ui.vertical(|ui| {
                ui.set_width(width);
                ui.strong(egui::RichText::new(name).size(16.0 * scale));
                ui.horizontal_wrapped(|ui| {
                    for (symbol, cost, name) in
                        [(Icon::Stone, stone, "Stone"), (Icon::Metal, metal, "Metal")]
                    {
                        super::campaign_widgets::icon(ui, symbol, 17.0 * scale);
                        let cost = if !cost.is_finite() {
                            "—".into()
                        } else if cost >= 100_000.0 {
                            format!("{cost:.1e}")
                        } else {
                            format!("{cost:.0}")
                        };
                        ui.label(egui::RichText::new(format!("{cost} {name}")).size(12.0 * scale));
                    }
                });
                ui.add(egui::Label::new(egui::RichText::new(tooltip).size(12.0 * scale)).wrap());
            });
        });
    });
    let response = if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    };
    enabled && response.clicked()
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
        let definitions: Vec<_> = world
            .config
            .buildings
            .iter()
            .filter(|d| d.requires_city == city && (!city || p.has_city))
            .collect();
        let cells = building_section(ui, title, empty, definitions.len(), columns, scale);
        for (definition, cell) in definitions.into_iter().zip(cells) {
            let level = p.level(definition.building);
            building_cell(ui, cell, Icon::Building(definition.building), definition.building.name(),
                &format!("Level {level}"), 0.0, 0.0, false, level > 0, None,
                &format!("Public completed level: {level}.\nEach completed level:\n{}\nDirect ownership required.\nOnly the direct owner can build or upgrade.", building_effects_text(definition, &world.config)), scale);
        }
    }
    let cells = building_section(ui, "Wonders", None, p.wonder_sites.len(), columns, scale);
    for (&wonder, cell) in p.wonder_sites.iter().zip(cells) {
        let completed = p.completed_wonder == Some(wonder);
        let started = matches!(&p.construction, Some(ConstructionProject::Wonder(w)) if w.wonder_id == wonder);
        let status = if completed {
            "Completed"
        } else if started {
            "Under construction"
        } else {
            "Not built"
        };
        building_cell(
            ui,
            cell,
            Icon::Wonder(wonder),
            crate::map::wonder_name(wonder).unwrap_or("Wonder"),
            status,
            0.0,
            0.0,
            false,
            started || completed,
            None,
            "Wonder starts and completions are public.",
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
    let owned = p.owner == Some(player);
    let definitions: Vec<_> =
        world.config.buildings.iter().filter(|d| !d.requires_city || p.has_city).cloned().collect();
    let wonders: Vec<_> = p
        .wonder_sites
        .iter()
        .filter_map(|id| world.config.wonders.iter().find(|d| d.wonder_id == *id))
        .cloned()
        .collect();
    let header = match p.construction {
        Some(ConstructionProject::Wonder(_)) => 80.0,
        Some(ConstructionProject::Building(_)) => 40.0,
        None => 0.0,
    };
    let (columns, scale) = building_grid_layout(ui.available_width().max(1.0), scale);
    ui.spacing_mut().item_spacing.y = 0.0;
    let mut message = None;
    if let Some(project) = &p.construction {
        message = project_controls(ui, world, province, player, project, header * scale, scale);
    }
    // Cancellation releases the slot immediately; other cell actions are applied after painting.
    let p = &world.provinces[province];
    let progress = p.construction.as_ref().map(|project| {
        let (progress, required) = project.progress();
        (
            (progress / required.max(0.001)).clamp(0.0, 1.0) as f32,
            project.months_remaining(&world.config, p.policies.construction),
        )
    });
    let mut building_action = None;
    let mut wonder_action = None;
    for (title, empty, city) in [
        ("City", None, true),
        ("Countryside", Some("No countryside improvements available."), false),
    ] {
        let definitions: Vec<_> = definitions.iter().filter(|d| d.requires_city == city).collect();
        let cells = building_section(ui, title, empty, definitions.len(), columns, scale);
        for (definition, cell) in definitions.iter().zip(cells) {
            let level = p.level(definition.building);
            let quote = definition.quote(level);
            let affordable = world
                .players
                .get(player)
                .is_some_and(|w| w.resources[1] >= quote.metal && w.resources[2] >= quote.stone);
            let supported = level.checked_add(1).is_some()
                && quote.stone.is_finite()
                && quote.metal.is_finite()
                && quote.required_progress.is_finite();
            let active = matches!(&p.construction, Some(ConstructionProject::Building(project)) if project.building == definition.building);
            let reason = if !owned {
                "Direct ownership required."
            } else if p.construction.is_some() {
                "Construction slot occupied."
            } else if !supported {
                "Further upgrades exceed the supported numeric range."
            } else if !affordable {
                "Insufficient global Stone or Metal."
            } else {
                "Click anywhere in this cell to build the next level."
            };
            let tooltip = format!("Next level: {} · {:.0} months\n\nEach completed level:\n{}\n\nFull cost paid at start; one shared building/wonder slot.\nCosts ×{:.2} per upgrade.{}\n\n{reason}",
            level.saturating_add(1), quote.required_progress.ceil(), building_effects_text(definition, &world.config), definition.cost_growth,
            if definition.requires_city { "\nCity province required." } else { "" });
            let name = definition.building.name();
            let level_label = if active {
                format!("Level {level} → {}", level.saturating_add(1))
            } else {
                format!("Level {level}")
            };
            if building_cell(
                ui,
                cell,
                Icon::Building(definition.building),
                name,
                &level_label,
                quote.stone,
                quote.metal,
                owned && p.construction.is_none() && affordable && supported,
                active,
                if active {
                    progress
                } else {
                    None
                },
                &tooltip,
                scale,
            ) {
                building_action = Some(definition.building);
            }
        }
    }
    let cells = building_section(ui, "Wonders", None, wonders.len(), columns, scale);
    for (definition, cell) in wonders.iter().zip(cells) {
        let wonder = definition.wonder_id;
        let completed = p.completed_wonder == Some(wonder);
        let active = matches!(&p.construction, Some(ConstructionProject::Wonder(project)) if project.wonder_id == wonder);
        let name = crate::map::wonder_name(wonder).unwrap_or("Wonder");
        let affordable = world.players.get(player).is_some_and(|w| {
            w.resources[1] >= definition.metal_cost && w.resources[2] >= definition.stone_cost
        });
        let reason = if completed {
            "Completed."
        } else if !owned {
            "Direct ownership required."
        } else if p.completed_wonder.is_some() {
            "This province already has a completed wonder."
        } else if p.construction.is_some() {
            "Construction slot occupied."
        } else if !affordable {
            "Insufficient global Stone or Metal."
        } else {
            "Click anywhere in this cell to begin construction."
        };
        let tooltip = format!("Construction: {:.0} months\n\nCompletion: +{:.0} Influence\nAfter completion: +{:.1} Influence/month to the direct owner.\nOne completed wonder per province. Canonical site and direct ownership required.\nAssigned slaves speed work but stop resource production.\n{}\n\n{reason}", definition.required_progress.ceil(), definition.completion_influence, definition.monthly_influence, wonder_labor_tooltip(&world.config));
        let level = if completed {
            "Completed"
        } else if active {
            "Under construction"
        } else {
            "Not built"
        };
        if building_cell(
            ui,
            cell,
            Icon::Wonder(wonder),
            name,
            level,
            definition.stone_cost,
            definition.metal_cost,
            owned && p.completed_wonder.is_none() && p.construction.is_none() && affordable,
            active || completed,
            if active {
                progress
            } else {
                None
            },
            &tooltip,
            scale,
        ) {
            wonder_action = Some(wonder);
        }
    }
    if let Some(building) = building_action {
        message = Some(match world.start_building(player, province, building) {
            Ok(()) => format!("{} started.", building.name()),
            Err(error) => error,
        });
    } else if let Some(wonder) = wonder_action {
        message = Some(match world.start_wonder(player, province, wonder) {
            Ok(()) => "Wonder construction started.".into(),
            Err(error) => error,
        });
    }
    message
}

/// A compact strip keeps cancellation and wonder labor controls above the cards.
fn project_controls(
    ui: &mut egui::Ui,
    world: &mut EconomyWorld,
    province: usize,
    player: usize,
    project: &ConstructionProject,
    height: f32,
    scale: f32,
) -> Option<String> {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), height), egui::Sense::hover());
    let (symbol, name) = match project {
        ConstructionProject::Building(p) => (
            Icon::Building(p.building),
            format!("{} · Level {}", p.building.name(), p.target_level),
        ),
        ConstructionProject::Wonder(p) => (
            Icon::Wonder(p.wonder_id),
            crate::map::wonder_name(p.wonder_id).unwrap_or("Wonder").to_owned(),
        ),
    };
    paint_icon(ui, symbol, egui::Rect::from_min_size(rect.min, egui::vec2(32.0, 32.0) * scale));
    let title = egui::Rect::from_min_max(
        rect.min + egui::vec2(48.0, 0.0) * scale,
        egui::pos2(rect.right() - 72.0 * scale, rect.top() + 23.0 * scale),
    );
    ledger_text(ui, title, &name, 14.0 * scale, super::province_panel::INK, egui::Align::Min);
    let (progress, required) = project.progress();
    let completion = (progress / required.max(0.001)).clamp(0.0, 1.0) as f32;
    let pace = world.provinces[province].policies.construction;
    response.on_hover_text(format!("{name}\n{progress:.1}/{required:.1} work ({:.0}%).\n{} months remaining; {:.2} work/month.\nConstruction pace: {pace:?}. Progress remains with the province after capture.", completion * 100.0, project.months_remaining(&world.config, pace), project.speed(&world.config, pace)));
    let owned = world.provinces[province].owner == Some(player);
    let cancel = egui::Rect::from_min_size(
        egui::pos2(rect.right() - 66.0 * scale, rect.top() + 7.0 * scale),
        egui::vec2(66.0, 28.0) * scale,
    );
    let mut message = None;
    ui.scope(|ui| {
        *ui.style_mut() = super::campaign_widgets::map_style(scale);
        if ui.put(cancel, egui::Button::new("Cancel").sense(if owned { egui::Sense::click() } else { egui::Sense::hover() }))
            .on_hover_text("Cancel this project. The entire Stone/Metal payment is lost; no refund. Direct ownership required.")
            .clicked() && owned {
            message = Some(match world.cancel_construction(player, province) { Ok(()) => "Construction cancelled.".into(), Err(error) => error });
        }
        if let ConstructionProject::Wonder(wonder) = project {
            let icon_rect = egui::Rect::from_min_size(rect.min + egui::vec2(0.0, 48.0) * scale, egui::vec2(24.0, 24.0) * scale);
            paint_icon(ui, Icon::Slaves, icon_rect);
            let slider_rect = egui::Rect::from_min_max(rect.min + egui::vec2(32.0, 47.0) * scale, rect.max - egui::vec2(0.0, 7.0 * scale));
            ui.scope_builder(egui::UiBuilder::new().max_rect(slider_rect), |ui| {
                if !owned {
                    ui.disable();
                }
                ui.spacing_mut().interact_size.x = 40.0 * scale;
                ui.spacing_mut().slider_width = (slider_rect.width() - 90.0 * scale).max(0.0);
                let mut assigned = wonder.assigned_slaves;
                let response = ui.add(egui::Slider::new(&mut assigned, 0.0..=world.provinces[province].population[3])
                    .custom_formatter(|value, _| super::resource_hud::format_population(value)))
                    .on_hover_text(wonder_labor_tooltip(&world.config));
                if response.changed() {
                    if let Err(error) = world.assign_wonder_slaves(player, province, assigned) { message = Some(error); }
                }
            });
        }
    });
    message
}

/// Compact descriptions are generated from the actual configured benefits.
fn building_effects_text(definition: &BuildingDefinition, config: &EconomyConfig) -> String {
    let mut text = effects_text(&definition.effects);
    if definition.building == BuildingType::Road {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&format!(
            "Faster army travel across this province\n+{:.1}% route efficiency",
            config.trade.road_efficiency * 100.0
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
            parts
                .push(format!("+{:.0} happiness for every population class", effects.happiness[0]));
        } else {
            for (name, value) in CLASS_NAMES.iter().zip(effects.happiness) {
                if value != 0.0 {
                    parts.push(format!("{value:+.0} {name} happiness"));
                }
            }
        }
    }
    if effects.influence > 0.0 {
        parts.push(format!("+{:.2} Influence/month", effects.influence));
    }
    if effects.tax > 0.0 {
        parts.push(format!("+{:.0}% taxes", effects.tax * 100.0));
    }
    if effects.defense > 0.0 {
        parts.push(format!("+{:.0}% defense", effects.defense * 100.0));
    }
    if effects.migration > 0.0 {
        parts.push(format!("+{:.2} migration attraction", effects.migration));
    }
    if effects.trade > 0.0 {
        parts.push(format!("+{:.1}% route efficiency", effects.trade * 100.0));
    }
    parts.join("\n")
}

/// Slave assignment uses the configured thresholds shown beside the live slider.
fn wonder_labor_tooltip(config: &EconomyConfig) -> String {
    let tiers = config
        .wonder_slave_speeds
        .iter()
        .map(|(slaves, speed)| format!("{slaves:.0}+ Slaves: {speed:.2}× work"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{tiers}\nAssignments below the next tier do not increase speed. Assigned Slaves keep consuming Food and stop ordinary resource production.")
}
