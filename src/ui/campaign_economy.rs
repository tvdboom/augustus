//! Icon-led economy, construction, and trade tables for the parchment campaign panel.

use crate::game::economy::*;
use bevy_egui::egui;

use super::campaign_widgets::{icon, stat, Icon};

const CLASS_NAMES: [&str; 4] = ["Nobles", "Citizens", "Plebeians", "Slaves"];
const CLASS_ICONS: [Icon; 4] = [Icon::Nobles, Icon::Citizens, Icon::Plebeians, Icon::Slaves];
const RESOURCE_NAMES: [&str; 5] = ["Food", "Metal", "Stone", "Coin", "Influence"];
const RESOURCE_ICONS: [Icon; 5] =
    [Icon::Food, Icon::Metal, Icon::Stone, Icon::Coin, Icon::Influence];
const MUTED: egui::Color32 = egui::Color32::from_rgb(112, 91, 71);

/// Local trade draft is kept per selected province/player, never in simulation state.
#[derive(Clone)]
struct TradeDraft {
    give: [f64; 5],
    receive: [f64; 5],
    monthly: bool,
    message: String,
}

impl Default for TradeDraft {
    /// Empty amounts require an explicit player proposal.
    fn default() -> Self {
        Self {
            give: [0.0; 5],
            receive: [0.0; 5],
            monthly: true,
            message: String::new(),
        }
    }
}

/// Population portraits and numeric ledgers keep the province readable at a glance.
pub(in crate::app) fn overview(ui: &mut egui::Ui, world: &EconomyWorld, province: usize) {
    let Some(p) = world.provinces.get(province) else {
        return;
    };
    let capacity = p.capacity(&world.config);
    let ratio = p.total_population() / capacity;
    let last = world.last_report.province_reports.get(province);
    ui.horizontal_wrapped(|ui| {
        stat(ui, Icon::Population, &format!("{:.1} / {capacity:.1}", p.total_population()),
            &format!("Population / comfortable capacity ({:.0}% occupied).\nArea {:.1} × scale {:.1} × terrain {:.2}; city +{:.0}; buildings +{:.0}.\nOvercrowding lowers happiness and births and encourages migration. Capacity is not a hard cap.",
                ratio * 100.0, p.capacity_area, world.config.area_to_capacity_scale,
                world.config.terrain_capacity[p.terrain as usize], if p.has_city {world.config.city_capacity} else {0.0},
                p.building_effects(&world.config).capacity));
        stat(ui, Icon::Food, &format!("−{:.1}/mo", p.food_request(&world.config)),
            "Civilian Food request. All owned provinces and military share the same proportional supply. Construction-assigned slaves still eat.");
        if let Some(last) = last {
            stat(ui, Icon::Food, &format!("{:.0}%", last.food_supply_ratio * 100.0),
                "Last month's Food fulfillment. Partial shortages proportionally lower happiness and births and cause famine deaths.");
        }
    });
    ui.add_space(4.0);
    egui::Grid::new(("campaign_population", province)).num_columns(5)
        .spacing([10.0, 4.0]).striped(true).show(ui, |ui| {
        ui.label(""); ui.label(""); ui.small("Pops");
        icon(ui, Icon::Happiness, 18.0).on_hover_text("Class happiness");
        ui.small("Δ/mo").on_hover_text("Births minus normal/famine deaths plus migration. Automatic class changes are excluded from this subtotal.");
        ui.end_row();
        for class in 0..4 {
            icon(ui, CLASS_ICONS[class], 30.0).on_hover_text(CLASS_NAMES[class]);
            ui.label(CLASS_NAMES[class]);
            ui.strong(format!("{:.1}", p.population[class]));
            let overcrowding = ((ratio - 1.0).max(0.0) * world.config.overcrowding_scale).min(world.config.overcrowding_cap);
            let supply = last.map_or(1.0, |month| month.food_supply_ratio);
            let food = world.config.food_policy[p.policies.food as usize];
            let slave = world.config.slave_policy[p.policies.slave_labor as usize];
            ui.label(format!("{:.0}%", p.happiness[class])).on_hover_text(format!(
                "Neutral 50\nFood policy {:+.1}\nBuildings {:+.1}\nEvents / unrest {:+.1}\nSlave labor {:+.1}\nOvercrowding −{overcrowding:.1}\nFood shortage −{:.1}\nBirth factors: happiness {:.2}×, space {:.2}×, rations {:.2}×, supply {:.2}×.\nSpace factor is min(1, {:.2} / population-to-capacity ratio). Happiness and space never directly increase natural mortality.",
                food.happiness, p.building_effects(&world.config).happiness[class],
                p.happiness_modifiers[class] + p.temporary_happiness[class], if class == 3 {slave.happiness} else {0.0},
                (1.0-supply)*world.config.shortage_happiness_penalty, birth_modifier(p.happiness[class]),
                p.crowding_birth_modifier(&world.config), food.births, supply, world.config.crowding_birth_threshold));
            if let Some(month) = last {
                let change = month.births[class]-month.normal_deaths[class]-month.famine_deaths[class]+month.migration[class];
                ui.label(format!("{change:+.2}")).on_hover_text(format!(
                    "Births +{:.3}\nNatural deaths −{:.3}\nFamine deaths −{:.3}\nMigration {:+.3}\nClass promotions/manumission separately conserve total population.",
                    month.births[class], month.normal_deaths[class], month.famine_deaths[class], month.migration[class]));
            } else { ui.label("—"); }
            ui.end_row();
        }
    });
    ui.add_space(8.0);
    let (labor, production) = p.production(&world.config);
    let owner = p.owner.and_then(|id| world.players.get(id));
    egui::Grid::new(("campaign_output", province)).num_columns(4)
        .spacing([10.0, 4.0]).striped(true).show(ui, |ui| {
        ui.label(""); ui.label(""); ui.small("/mo"); ui.small("Stock / cap"); ui.end_row();
        for resource in 0..3 {
            icon(ui, RESOURCE_ICONS[resource], 26.0).on_hover_text(RESOURCE_NAMES[resource]);
            ui.label(RESOURCE_NAMES[resource]);
            ui.strong(format!("+{:.1}", production[resource])).on_hover_text(format!(
                "Labor {:.2} × potential {:.1} × scale {:.2} × building multiplier {:.2}.\nLabor is allocated only once across all three sectors; zero-potential sectors get none.",
                labor[resource], p.potential[resource], world.config.production_scale[resource], 1.0+p.building_effects(&world.config).production[resource]));
            if let Some(owner) = owner {
                ui.label(format!("{:.0} / {:.0}", owner.resources[resource], owner.storage[resource]))
                    .on_hover_text("Shared player stockpile and maximum storage. Province storage buildings add global capacity. Excess is discarded after production, trade and consumption.");
            } else { ui.label("—").on_hover_text("NPC resources use monthly production, export capacity and import demand instead of physical stockpiles."); }
            ui.end_row();
        }
        if let Some(owner) = owner {
            icon(ui, Icon::Coin, 26.0); ui.label("Coin");
            ui.strong(format!("+{:.1}", p.tax_income(&world.config))).on_hover_text(format!(
                "Monthly provincial taxes. Per Noble {:.2}, Citizen {:.2}, Plebeian {:.2}; no direct slave tax.\nBuilding tax multiplier {:.2}×.",
                world.config.tax_rates[0], world.config.tax_rates[1], world.config.tax_rates[2],1.0+p.building_effects(&world.config).tax));
            ui.label(format!("{:.0}",owner.coin)).on_hover_text("Player Coin treasury; no storage cap."); ui.end_row();
            let noble = p.population[0]*world.config.influence_per_noble;
            let buildings = p.building_effects(&world.config).influence;
            let wonder = p.completed_wonder.and_then(|id|world.config.wonders.iter().find(|d|d.wonder_id==id)).map_or(0.0,|d|d.monthly_influence);
            icon(ui, Icon::Influence, 26.0); ui.label("Influence");
            ui.strong(format!("+{:.2}",noble+buildings+wonder)).on_hover_text(format!(
                "Nobles: {:.1} × {:.2} = +{noble:.2}\nCity buildings +{buildings:.2}\nCompleted wonder +{wonder:.2}\nRank and vassal income are additional player sources.",
                p.population[0],world.config.influence_per_noble));
            ui.label(format!("{:.0}",owner.influence)).on_hover_text("Player Influence; no storage cap."); ui.end_row();
        }
    });
    if let Some(project) = &p.construction {
        ui.separator();
        project_summary(ui, project, &world.config);
    }
}

/// Segmented policy rows expose explanations only on hover.
pub(in crate::app) fn policies(
    ui: &mut egui::Ui,
    world: &mut EconomyWorld,
    province: usize,
    player: usize,
) {
    let explanations = policy_tooltips(&world.config);
    let Some(p) = world.provinces.get_mut(province) else {
        return;
    };
    let owned = p.owner == Some(player);
    ui.add_enabled_ui(owned, |ui| {
        policy_row(
            ui,
            0,
            Icon::Food,
            &mut p.policies.food,
            &[(FoodPolicy::Low, "Low"), (FoodPolicy::Normal, "Normal"), (FoodPolicy::High, "High")],
            &explanations[0],
        );
        policy_row(
            ui,
            1,
            Icon::Building(BuildingType::Farm),
            &mut p.policies.focus,
            &[
                (ResourceFocus::Balanced, "All"),
                (ResourceFocus::Food, "Food"),
                (ResourceFocus::Metal, "Metal"),
                (ResourceFocus::Stone, "Stone"),
            ],
            &explanations[1],
        );
        policy_row(
            ui,
            2,
            Icon::Slaves,
            &mut p.policies.slave_labor,
            &[
                (SlaveLabor::Light, "Light"),
                (SlaveLabor::Normal, "Normal"),
                (SlaveLabor::Harsh, "Harsh"),
            ],
            &explanations[2],
        );
        policy_row(
            ui,
            3,
            Icon::Population,
            &mut p.policies.migration,
            &[
                (MigrationPolicy::Encourage, "Open"),
                (MigrationPolicy::Normal, "Normal"),
                (MigrationPolicy::Discourage, "Limited"),
                (MigrationPolicy::Closed, "Closed"),
            ],
            &explanations[3],
        );
    })
    .response
    .on_hover_text(if owned {
        "Policies apply in the next monthly calculation."
    } else {
        "Only the direct owner can set these policies."
    });
    ui.add_space(8.0);
    let (labor, output) = p.production(&world.config);
    ui.horizontal_wrapped(|ui| {
        for i in 0..3 {
            stat(
                ui,
                RESOURCE_ICONS[i],
                &format!("+{:.1}", output[i]),
                "Monthly production preview with the selected policies.",
            );
        }
    });
    ui.horizontal_wrapped(|ui| {
        stat(
            ui,
            Icon::Population,
            &format!("{:.1}", labor.iter().sum::<f64>()),
            "Productive labor after the slave-labor policy and wonder assignment.",
        );
        stat(
            ui,
            Icon::Food,
            &format!("−{:.1}", p.food_request(&world.config)),
            "Monthly civilian Food request with selected rations.",
        );
    });
}

/// Display values come from the same configuration consumed by monthly rules.
fn policy_tooltips(config: &EconomyConfig) -> [String; 4] {
    let food = ["Low", "Normal", "High"]
        .into_iter()
        .zip(config.food_policy)
        .map(|(name, m)| {
            format!(
                "{name}: {:.0}% Food, happiness {:+.0}, births ×{:.2}, deaths ×{:.2}.",
                m.consumption * 100.0,
                m.happiness,
                m.births,
                m.deaths
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let focus = ["All", "Food", "Metal", "Stone"]
        .into_iter()
        .zip(config.focus_weights)
        .map(|(name, weights)| {
            format!(
                "{name}: Food/Metal/Stone weights {:.1}/{:.1}/{:.1}.",
                weights[0], weights[1], weights[2]
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let labor = ["Light", "Normal", "Harsh"]
        .into_iter()
        .zip(config.slave_policy)
        .map(|(name, m)| {
            format!(
                "{name}: production ×{:.2}, happiness {:+.0}, deaths ×{:.2}.",
                m.productivity, m.happiness, m.deaths
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let migration = ["Open", "Normal", "Limited", "Closed"]
        .into_iter()
        .enumerate()
        .map(|(id, name)| {
            format!(
                "{name}: arrival attraction ×{:.2}, departures ×{:.2}.",
                config.migration_in[id], config.migration_out[id]
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    [format!("{food}\nGlobal shortages still apply."),
        format!("{focus}\nWeights multiply geographic potential. Total labor is unchanged; zero-potential sectors get none."),
        labor,format!("{migration}\nOnly unhappy non-slaves migrate automatically. Slaves never freely migrate; blocked routes cannot carry migrants.")]
}

/// One alternating parchment row; it always stays inside the available panel width.
fn striped_row(ui: &mut egui::Ui, index: usize, content: impl FnOnce(&mut egui::Ui)) {
    let width = ui.available_width();
    let fill = if index.is_multiple_of(2) {
        super::province_panel::TABLE_STRIPE
    } else {
        egui::Color32::TRANSPARENT
    };
    egui::Frame::new().fill(fill).inner_margin(4.0).show(ui, |ui| {
        ui.set_min_width((width - 8.0).max(1.0));
        content(ui);
    });
}

/// Exclusive compact choices share one illustrated policy heading.
fn policy_row<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    index: usize,
    illustration: Icon,
    value: &mut T,
    options: &[(T, &str)],
    explanation: &str,
) {
    striped_row(ui, index, |ui| {
        ui.horizontal(|ui| {
            icon(ui, illustration, 30.0).on_hover_text(explanation);
            ui.horizontal_wrapped(|ui| {
                for &(candidate, label) in options {
                    ui.selectable_value(value, candidate, label).on_hover_text(explanation);
                }
            });
        });
    });
}

/// Illustrated building rows retain all construction actions with hover-only rules.
pub(in crate::app) fn buildings(
    ui: &mut egui::Ui,
    world: &mut EconomyWorld,
    province: usize,
    player: usize,
) -> Option<String> {
    let p = world.provinces.get(province)?.clone();
    let owned = p.owner == Some(player);
    let mut message = None;
    if let Some(project) = &p.construction {
        project_summary(ui, project, &world.config);
        ui.horizontal(|ui|{
            if let ConstructionProject::Wonder(wonder)=project {
                icon(ui,Icon::Slaves,24.0).on_hover_text("Assigned builders remain residents, eat and undergo ordinary births/deaths; they stop ordinary resource production.");
                let mut assigned=wonder.assigned_slaves;
                let response=ui.add_enabled(owned,egui::Slider::new(&mut assigned,0.0..=p.population[3]).max_decimals(1))
                    .on_hover_text(wonder_labor_tooltip(&world.config));
                if response.changed(){if let Err(error)=world.assign_wonder_slaves(player,province,assigned){message=Some(error);}}
            }
            if ui.add_enabled(owned,egui::Button::new("Cancel")).on_hover_text("Cancel this project. The entire Stone/Metal payment is lost; no refund.").clicked(){
                message=Some(match world.cancel_construction(player,province){Ok(())=>"Construction cancelled.".into(),Err(error)=>error});
            }
        });
        ui.add_space(6.0);
    }
    for (index, definition) in world
        .config
        .buildings
        .clone()
        .into_iter()
        .filter(|d| !d.requires_city || p.has_city)
        .enumerate()
    {
        let level = p.level(definition.building);
        let quote = definition.quote(level);
        let affordable = world
            .players
            .get(player)
            .is_some_and(|w| w.resources[1] >= quote.metal && w.resources[2] >= quote.stone);
        let tooltip=format!("{}\n{}\nLevel {} → {}\nFull cost paid at start; one shared building/wonder slot.\nCosts ×{:.2} per upgrade. {:.1} work; {:.0} monthly ticks.{}{}",
            definition.building.name(),effects_text(&definition.effects),level,level.saturating_add(1),definition.cost_growth,quote.required_progress,quote.required_progress.ceil(),
            if definition.requires_city {"\nCity province required."}else{""},
            if definition.building==BuildingType::Road {format!("\nPer level: +{:.1}% route efficiency; military travel also uses Road levels.",world.config.trade.road_efficiency*100.0)}else{String::new()});
        ui.push_id(definition.building as usize, |ui| {
            striped_row(ui, index, |ui| {
                ui.horizontal(|ui| {
                    icon(ui, Icon::Building(definition.building), 42.0).on_hover_text(&tooltip);
                    let middle = (ui.available_width() - 64.0).max(100.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(middle, 42.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.horizontal(|ui| {
                                ui.strong(if definition.building == BuildingType::Farm {
                                    "Farm"
                                } else {
                                    definition.building.name()
                                })
                                .on_hover_text(&tooltip);
                                ui.label(
                                    egui::RichText::new(format!("Lv {level}")).small().color(MUTED),
                                );
                            });
                            ui.horizontal_wrapped(|ui| {
                                stat(
                                    ui,
                                    Icon::Stone,
                                    &format!("{:.0}", quote.stone),
                                    "Stone paid immediately.",
                                );
                                stat(
                                    ui,
                                    Icon::Metal,
                                    &format!("{:.0}", quote.metal),
                                    "Metal paid immediately.",
                                );
                                ui.small(format!("{:.0}mo", quote.required_progress.ceil()))
                                    .on_hover_text(&tooltip);
                            });
                        },
                    );
                    if ui
                        .add_enabled(
                            owned && p.construction.is_none() && affordable,
                            egui::Button::new("Build"),
                        )
                        .on_hover_text(&tooltip)
                        .on_disabled_hover_text(if !owned {
                            "Direct ownership required."
                        } else if p.construction.is_some() {
                            "Construction slot occupied."
                        } else {
                            "Insufficient global Stone or Metal."
                        })
                        .clicked()
                    {
                        message = Some(
                            match world.start_building(player, province, definition.building) {
                                Ok(()) => format!("{} started.", definition.building.name()),
                                Err(error) => error,
                            },
                        );
                    }
                });
            });
        });
    }
    if !p.wonder_sites.is_empty() {
        ui.add_space(6.0);
        for (index, &wonder) in p.wonder_sites.iter().enumerate() {
            let Some(d) = world.config.wonders.iter().find(|d| d.wonder_id == wonder).cloned()
            else {
                continue;
            };
            let completed = p.completed_wonder == Some(wonder);
            let name = crate::map::wonder_name(wonder).unwrap_or("Wonder");
            let tooltip=format!("{name}\nCompletion +{:.0} Influence, then +{:.1}/month to the current direct owner.\nOne completed wonder per province. Canonical site and direct ownership required. Assigned slaves speed work but stop resource production. No secondary bonuses.\n{}",
                d.completion_influence,d.monthly_influence,wonder_labor_tooltip(&world.config));
            let affordable = world
                .players
                .get(player)
                .is_some_and(|w| w.resources[1] >= d.metal_cost && w.resources[2] >= d.stone_cost);
            striped_row(ui, index, |ui| {
                ui.horizontal(|ui|{
                    icon(ui,Icon::Wonder(wonder),46.0).on_hover_text(&tooltip);
                    let middle=(ui.available_width()-64.0).max(100.0);
                    ui.allocate_ui_with_layout(egui::vec2(middle,46.0),egui::Layout::top_down(egui::Align::Min),|ui|{
                        ui.add(egui::Label::new(egui::RichText::new(name).strong()).truncate()).on_hover_text(&tooltip);
                        ui.horizontal_wrapped(|ui|{
                            if completed {stat(ui,Icon::Influence,&format!("+{:.1}/mo",d.monthly_influence),&tooltip);}
                            else {
                                stat(ui,Icon::Stone,&format!("{:.0}",d.stone_cost),"Full Stone cost paid at start.");
                                stat(ui,Icon::Metal,&format!("{:.0}",d.metal_cost),"Full Metal cost paid at start.");
                                ui.small(format!("{:.0}mo",d.required_progress)).on_hover_text("Base construction time; assigned slaves accelerate it.");
                            }
                        });
                    });
                    if completed {ui.label("✓").on_hover_text("Completed");}
                    else if ui.add_enabled(owned&&p.completed_wonder.is_none()&&p.construction.is_none()&&affordable,egui::Button::new("Build"))
                        .on_hover_text(&tooltip).on_disabled_hover_text("Requires direct ownership, this site, no completed wonder, an empty construction slot and sufficient Stone/Metal.").clicked(){
                        message=Some(match world.start_wonder(player,province,wonder){Ok(())=>"Wonder construction started.".into(),Err(error)=>error});
                    }
                });
            });
        }
    }
    message
}

/// Compact project thumbnail, authoritative progress and remaining months.
fn project_summary(ui: &mut egui::Ui, project: &ConstructionProject, config: &EconomyConfig) {
    let (illustration, name) = match project {
        ConstructionProject::Building(p) => {
            (Icon::Building(p.building), format!("{} · Lv {}", p.building.name(), p.target_level))
        },
        ConstructionProject::Wonder(p) => (
            Icon::Wonder(p.wonder_id),
            crate::map::wonder_name(p.wonder_id).unwrap_or("Wonder").to_owned(),
        ),
    };
    let (progress, required) = project.progress();
    ui.horizontal(|ui|{
        icon(ui,illustration,38.0);
        ui.vertical(|ui|{
            ui.label(egui::RichText::new(name).strong());
            let width=ui.available_width().min(360.0);
            ui.add(egui::ProgressBar::new((progress/required.max(0.001)).clamp(0.0,1.0)as f32)
                .desired_width(width).text(format!("{:.0}% · {}mo",100.0*progress/required.max(0.001),project.months_remaining(config))))
                .on_hover_text(format!("{progress:.1}/{required:.1} work; {:.2} work/month.\nProgress remains with the province after capture. Slave acceleration has diminishing returns.",project.speed(config)));
        });
    });
}

/// Compact descriptions are generated from the actual configured benefits.
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
            parts.push(format!("+{:.0} happiness", effects.happiness[0]));
        } else {
            parts.push(format!(
                "happiness +{:.0}/{:.0}/{:.0}/{:.0} (N/C/P/S)",
                effects.happiness[0],
                effects.happiness[1],
                effects.happiness[2],
                effects.happiness[3]
            ));
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
    parts.join(" · ")
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

/// Icon-led barter editor and compact agreement ledger retain explicit human consent.
pub(in crate::app) fn trade(
    ui: &mut egui::Ui,
    world: &mut EconomyWorld,
    inputs: &MonthlyInputs,
    province: usize,
    player: usize,
) -> Option<TradePoliticalEffect> {
    let p = world.provinces.get(province)?.clone();
    let key = egui::Id::new(("campaign_trade_draft", province, player));
    let mut draft = ui.ctx().data_mut(|data| data.get_temp::<TradeDraft>(key)).unwrap_or_default();
    let counterparty = p.owner.map(TradeParty::Player).unwrap_or(TradeParty::Npc(province));
    let mut effect = None;
    if p.owner.is_none() {
        egui::Grid::new(("npc_market",province)).num_columns(4).spacing([10.0,4.0]).striped(true).show(ui,|ui|{
            ui.label("");ui.small("Market");ui.small("Free / need").on_hover_text("Unreserved export capacity / remaining import demand.");icon(ui,Icon::Coin,18.0).on_hover_text("Local value per resource unit.");ui.end_row();
            for i in 0..3{
                let market=p.market.resources[i];
                icon(ui,RESOURCE_ICONS[i],26.0).on_hover_text(RESOURCE_NAMES[i]);
                ui.label(market.band.name()).on_hover_text(format!("Production {:.1}; internal need {:.1}. Scarcity changes value; Coin keeps nominal value.",market.production_potential,market.internal_need));
                ui.label(format!("{:.0} / {:.0}",(market.export_capacity-market.exported).max(0.0),(market.import_demand-market.imported).max(0.0)));
                ui.label(format!("{:.2}",market.local_unit_value));ui.end_row();
            }
        });
        ui.horizontal_wrapped(|ui|{
            stat(ui,Icon::Coin,&format!("{:.0}",p.market.coin_treasury),"NPC treasury. Multiple agreements draw on the same real Coin balance.");
            stat(ui,Icon::Trade,&format!("{:.0}/mo",(p.market.trade_budget-p.market.coin_spent).max(0.0)),"Remaining recurring trade budget: a limited share of monthly income and treasury. One-time deals permit a larger finite draw.");
        });
        ui.add_space(6.0);
    }
    if counterparty != TradeParty::Player(player) {
        ui.horizontal_wrapped(|ui|{
            icon(ui,Icon::Trade,26.0);
            ui.selectable_value(&mut draft.monthly,true,"Monthly").on_hover_text("Recurring trade rewards relation and can slowly raise independent Control up to 40. Cancel at any time.");
            ui.selectable_value(&mut draft.monthly,false,"One-time").on_hover_text("Immediate exchange after acceptance. NPC terms are less favorable; no Control gain.");
        });
        egui::Grid::new(("trade_amounts", province))
            .num_columns(3)
            .spacing([18.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label("");
                ui.small("Send");
                ui.small("Request");
                ui.end_row();
                for i in 0..5 {
                    if i == 4 && !world.config.trade.allow_influence {
                        continue;
                    }
                    icon(ui, RESOURCE_ICONS[i], 26.0).on_hover_text(RESOURCE_NAMES[i]);
                    ui.add(
                        egui::DragValue::new(&mut draft.give[i])
                            .speed(1.0)
                            .range(0.0..=1_000_000.0)
                            .max_decimals(1),
                    );
                    ui.add(
                        egui::DragValue::new(&mut draft.receive[i])
                            .speed(1.0)
                            .range(0.0..=1_000_000.0)
                            .max_decimals(1),
                    );
                    ui.end_row();
                }
            });
        let proposal = TradeAgreement::new(
            TradeParty::Player(player),
            counterparty,
            bundle(draft.give),
            bundle(draft.receive),
            if draft.monthly {
                TradeFrequency::Monthly
            } else {
                TradeFrequency::OneTime
            },
        );
        let quote = trade_quote(world, &proposal, inputs);
        match &quote {
            Ok(quote) => {
                let route = quote
                    .route
                    .iter()
                    .map(|&id| world.provinces[id].name.as_str())
                    .collect::<Vec<_>>()
                    .join(" → ");
                let terms=format!("{route}\n{} crossings; {:.0}% fulfillment.{}\nBoth directions lose the same percentage to transport.",
                    quote.route.len().saturating_sub(1),quote.fulfillment*100.0,
                    quote.required_value_ratio.map_or(String::new(),|v|format!("\nNPC value margin {v:.2}×: incoming after transport must cover gross outgoing value.")));
                ui.horizontal_wrapped(|ui| {
                    stat(ui, Icon::Trade, &format!("{:.0}%", quote.efficiency * 100.0), &terms);
                    ui.small(format!("{} stops", quote.route.len().saturating_sub(1)))
                        .on_hover_text(&terms);
                });
                bundle_icons(
                    ui,
                    "In",
                    quote.a_receives.scaled(quote.fulfillment),
                    "You receive these amounts after transport and fulfillment.",
                );
                bundle_icons(
                    ui,
                    "Out",
                    quote.b_receives.scaled(quote.fulfillment),
                    "The counterparty receives these amounts after transport and fulfillment.",
                );
            },
            Err(reason) => {
                ui.small("Unavailable").on_hover_text(reason);
            },
        }
        let available = quote.as_ref().is_ok_and(|q| q.fulfillment + 1e-9 >= 1.0);
        let title = if matches!(counterparty, TradeParty::Player(_)) {
            "Propose"
        } else if draft.monthly {
            "Open route"
        } else {
            "Exchange"
        };
        if ui
            .add_enabled(available, egui::Button::new(title))
            .on_disabled_hover_text(
                quote
                    .as_ref()
                    .err()
                    .map(String::as_str)
                    .unwrap_or("A new agreement must support the entire promised amount."),
            )
            .clicked()
        {
            match world.propose_trade(proposal, inputs) {
                Ok((id, political_effect)) => {
                    effect = political_effect;
                    draft.message = format!(
                        "#{id} · {}",
                        if matches!(counterparty, TradeParty::Player(_)) {
                            "Awaiting acceptance"
                        } else if draft.monthly {
                            "Route opened"
                        } else {
                            "Exchanged"
                        }
                    );
                },
                Err(reason) => draft.message = format!("Failed: {reason}"),
            }
        }
    } else {
        ui.horizontal(|ui|{
            icon(ui,Icon::Trade,26.0);
            ui.small("Shared stores").on_hover_text("Your provinces already share global resources. Select an independent, vassal or foreign-owned province to propose trade.");
        });
    }
    if !draft.message.is_empty() {
        let label = if draft.message.starts_with("Failed:") {
            "Action failed"
        } else {
            draft.message.as_str()
        };
        ui.label(egui::RichText::new(label).small().color(MUTED)).on_hover_text(&draft.message);
    }
    ui.add_space(8.0);
    let agreements: Vec<_> = world
        .trades
        .iter()
        .filter(|t| {
            t.party_a == TradeParty::Player(player) || t.party_b == TradeParty::Player(player)
        })
        .cloned()
        .collect();
    ui.horizontal(|ui| {
        icon(ui, Icon::Trade, 24.0);
        ui.small(format!("Agreements · {}", agreements.len()));
    });
    for (index, agreement) in agreements.into_iter().enumerate() {
        let other = if agreement.party_a == TradeParty::Player(player) {
            agreement.party_b
        } else {
            agreement.party_a
        };
        let name = match other {
            TradeParty::Player(id) => format!("Player {}", id + 1),
            TradeParty::Npc(id) => {
                world.provinces.get(id).map_or("Unknown", |p| p.name.as_str()).to_owned()
            },
        };
        let (sent, received) = if agreement.party_a == TradeParty::Player(player) {
            (agreement.a_gives, agreement.b_gives)
        } else {
            (agreement.b_gives, agreement.a_gives)
        };
        let current_quote = trade_quote(world, &agreement, inputs);
        let current_terms = match &current_quote {
            Ok(quote) => {
                let delivered = if agreement.party_a == TradeParty::Player(player) {
                    quote.a_receives
                } else {
                    quote.b_receives
                }
                .scaled(quote.fulfillment);
                let route = quote
                    .route
                    .iter()
                    .map(|&id| world.provinces[id].name.as_str())
                    .collect::<Vec<_>>()
                    .join(" → ");
                format!("\nCurrent route: {route}\nTransport {:.0}%; fulfillment {:.0}%.\nYou receive {} after transport.",quote.efficiency*100.0,quote.fulfillment*100.0,bundle_text(delivered))
            },
            Err(reason) => format!("\nCurrently unavailable: {reason}"),
        };
        let details=format!("Agreement #{} · {:?}\nSend {}\nReceive {} before transport.\nLast delivered value {:.1}; fulfillment {:.0}%.{}{}",
            agreement.id,agreement.frequency,bundle_text(sent),bundle_text(received),agreement.last_delivered_value,agreement.last_fulfillment*100.0,
            agreement.last_failure.as_ref().map_or(String::new(),|r|format!("\n{r}\n{} failed months; resumes automatically if possible, cancelled after {} failures.",agreement.failure_months,world.config.trade.failure_limit)),current_terms);
        ui.push_id(agreement.id, |ui| {
            striped_row(ui, index, |ui| {
                ui.horizontal(|ui| {
                    icon(ui, Icon::Trade, 24.0).on_hover_text(&details);
                    let label_width = (ui.available_width() - 126.0).max(80.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(label_width, 30.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.add(
                                egui::Label::new(egui::RichText::new(&name).strong()).truncate(),
                            )
                            .on_hover_text(&details);
                            ui.small(format!("{:?}", agreement.status)).on_hover_text(&details);
                        },
                    );
                    if agreement.status == TradeStatus::Proposed
                        && agreement.party_b == TradeParty::Player(player)
                        && ui
                            .add_enabled(current_quote.is_ok(), egui::Button::new("Accept").small())
                            .on_hover_text(&details)
                            .on_disabled_hover_text(&details)
                            .clicked()
                    {
                        draft.message = match world.accept_trade(agreement.id, player, inputs) {
                            Ok(()) => "Accepted".into(),
                            Err(r) => format!("Failed: {r}"),
                        };
                    }
                    if matches!(
                        agreement.status,
                        TradeStatus::Proposed | TradeStatus::Active | TradeStatus::Suspended
                    ) && ui
                        .small_button("Cancel")
                        .on_hover_text("Cancel this agreement. Existing deliveries are retained.")
                        .clicked()
                    {
                        draft.message = match world.cancel_trade(agreement.id, player) {
                            Ok(()) => "Cancelled".into(),
                            Err(r) => format!("Failed: {r}"),
                        };
                    }
                });
            });
        });
    }
    ui.ctx().data_mut(|data| data.insert_temp(key, draft));
    effect
}

/// Immediate exchanges expose the same affordability validation as execution.
fn trade_quote(
    world: &EconomyWorld,
    agreement: &TradeAgreement,
    inputs: &MonthlyInputs,
) -> Result<TradeQuote, String> {
    if agreement.frequency == TradeFrequency::OneTime {
        world.quote_trade_execution(agreement, inputs)
    } else {
        world.quote_trade(agreement, inputs)
    }
}

/// Only nonzero delivered goods appear; overflow wraps as compact icon/value pairs.
fn bundle_icons(ui: &mut egui::Ui, label: &str, goods: TradeBundle, explanation: &str) {
    let values =
        [goods.resources[0], goods.resources[1], goods.resources[2], goods.coin, goods.influence];
    ui.horizontal_wrapped(|ui| {
        ui.small(label).on_hover_text(explanation);
        for i in 0..5 {
            if values[i] > 0.0 {
                stat(ui, RESOURCE_ICONS[i], &format!("{:.1}", values[i]), explanation);
            }
        }
    });
}

/// Convert editor values to the simulation's explicit physical/currency separation.
fn bundle(values: [f64; 5]) -> TradeBundle {
    TradeBundle {
        resources: [values[0], values[1], values[2]],
        coin: values[3],
        influence: values[4],
    }
}

/// Omit zero goods while keeping transfer previews short enough for the panel.
fn bundle_text(bundle: TradeBundle) -> String {
    let values = [
        bundle.resources[0],
        bundle.resources[1],
        bundle.resources[2],
        bundle.coin,
        bundle.influence,
    ];
    let parts: Vec<_> = values
        .iter()
        .zip(RESOURCE_NAMES)
        .filter(|(value, _)| **value > 0.0)
        .map(|(value, name)| format!("{value:.1} {name}"))
        .collect();
    if parts.is_empty() {
        "nothing".into()
    } else {
        parts.join(", ")
    }
}
