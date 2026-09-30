//! Terminal overlay and a read-only, fully informed map inspector.

use super::*;
use crate::game::economy::BuildingType;
use crate::game::military::{ForceOwner, Unit};

fn owner_name(owner: ForceOwner) -> String {
    match owner {
        ForceOwner::Player(player) => format!("Player {}", player + 1),
        ForceOwner::Local(province) => format!("Local defenders of province {}", province + 1),
    }
}

fn cohort_line(unit: &Unit) -> String {
    format!(
        "{} · {} soldiers · morale {:.0} · training {:.0}",
        unit.unit_type.name(),
        unit.people(),
        unit.morale,
        unit.training,
    )
}

/// Fade the final result over the loaded map, matching the reference game's two choices.
pub(in crate::app) fn draw_end_game(
    mut contexts: EguiContexts,
    time: Res<Time>,
    mut terminal: ResMut<TerminalPresentation>,
    mut next: ResMut<NextState<AppState>>,
    sound: Res<MenuAudio>,
    audio: Res<Audio>,
    assets: Res<AssetServer>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let Some(outcome) = terminal.outcome else {
        return;
    };
    terminal.fade_elapsed += time.delta_secs();
    let opacity = terminal.opacity();
    if opacity < 1.0 {
        ctx.request_repaint();
    }
    let viewport = ctx.content_rect();
    egui::Area::new(egui::Id::new("augustus_end_game_backdrop"))
        .fixed_pos(viewport.min)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(viewport.size(), egui::Sense::click());
            ui.painter().rect_filled(
                rect,
                0.0,
                egui::Color32::from_black_alpha((opacity * 170.0) as u8),
            );
        });
    let scale = viewport_ui_scale(viewport.size());
    let content_width = MENU_CONTENT_WIDTH.min((viewport.width() / scale - 32.0).max(240.0));
    let id = egui::Id::new("augustus_end_game_result");
    set_menu_layer_scale(ctx, id, egui::Order::Foreground, viewport.min, scale);
    egui::Area::new(id)
        .pivot(egui::Align2::CENTER_CENTER)
        .fixed_pos(egui::pos2(viewport.width() / scale * 0.5, viewport.height() / scale * 0.5))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            apply_menu_style(ui);
            ui.set_opacity(opacity);
            ui.set_width(content_width);
            ui.vertical_centered(|ui| {
                let heading = match outcome {
                    TerminalOutcome::Victory => "You won",
                    TerminalOutcome::Defeat => "You lost",
                };
                ui.heading(egui::RichText::new(heading).size(MENU_TITLE_TEXT_SIZE));
                ui.add_space(28.0);
                let size = menu_button_metrics(ui);
                if menu_action_button(
                    ui,
                    "Spectate",
                    true,
                    false,
                    size,
                    MENU_ACTION_TEXT_SIZE,
                    &sound,
                    &audio,
                    &assets,
                ) {
                    terminal.spectating = true;
                    next.set(AppState::Map);
                }
                ui.add_space(FORM_CARD_GAP);
                if menu_action_button(
                    ui,
                    "Return to Main Menu",
                    true,
                    false,
                    size,
                    MENU_ACTION_TEXT_SIZE,
                    &sound,
                    &audio,
                    &assets,
                ) {
                    next.set(AppState::MainMenu);
                }
            });
        });
}

/// Inspect authoritative province and army state without exposing command controls.
pub(in crate::app) fn draw_spectator(
    mut contexts: EguiContexts,
    terminal: Res<TerminalPresentation>,
    campaign: Res<campaign::Campaign>,
    mut detail: ResMut<ProvincePanelOpen>,
    mut next: ResMut<NextState<AppState>>,
    sound: Res<MenuAudio>,
    audio: Res<Audio>,
    assets: Res<AssetServer>,
) {
    if !terminal.spectating || !campaign.active {
        return;
    }
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    if let Some(hit) = crate::map::take_army_click(ctx) {
        detail.0 = Some(MapDetail::Province(hit.province));
    }
    let scale = viewport_ui_scale(ctx.content_rect().size());
    egui::Area::new(egui::Id::new("augustus_spectator_status"))
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(20.0, 16.0) * scale)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(province_panel::PAPER)
                .stroke(egui::Stroke::new(scale, province_panel::RULE))
                .corner_radius(5.0 * scale)
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.strong("Spectator Mode");
                        ui.separator();
                        if ui.button("Main Menu").clicked() {
                            play_click(&sound, &audio, &assets);
                            next.set(AppState::MainMenu);
                        }
                    });
                });
        });

    let Some(selected) = detail.0 else {
        return;
    };
    let province = match selected {
        MapDetail::Province(id) | MapDetail::City(id) => id,
    };
    let Some(p) = campaign.economy.provinces.get(province) else {
        detail.0 = None;
        return;
    };
    let mut open = true;
    egui::Window::new(format!("{} · Province {}", p.name, province + 1))
        .id(egui::Id::new("augustus_spectator_inspector"))
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_pos(ctx.content_rect().min + egui::vec2(30.0, 80.0) * scale)
        .default_width(440.0 * scale)
        .frame(
            egui::Frame::new()
                .fill(province_panel::PAPER)
                .stroke(egui::Stroke::new(scale, province_panel::RULE)),
        )
        .show(ctx, |ui| {
            ui.set_width(440.0 * scale);
            egui::ScrollArea::vertical()
                .max_height((ctx.content_rect().height() - 145.0 * scale).max(120.0))
                .show(ui, |ui| {
                    ui.label(match (p.owner, p.overlord) {
                        (Some(owner), _) => format!("Owned by Player {}", owner + 1),
                        (None, Some(overlord)) => format!("Vassal of Player {}", overlord + 1),
                        _ => "Independent".to_owned(),
                    });
                    ui.label(format!(
                        "Terrain: {:?} · Population: {:.0}",
                        p.terrain,
                        p.total_population()
                    ));
                    ui.separator();
                    ui.strong("Population and happiness");
                    for (class, count) in
                        ["Nobles", "Citizens", "Plebeians", "Slaves"].into_iter().zip(p.population)
                    {
                        let index = match class {
                            "Nobles" => 0,
                            "Citizens" => 1,
                            "Plebeians" => 2,
                            _ => 3,
                        };
                        ui.label(format!(
                            "{class}: {count:.0} · happiness {:.0}",
                            p.happiness[index]
                        ));
                    }
                    let production = p.production(&campaign.economy.config).1;
                    ui.separator();
                    ui.strong("Economy");
                    ui.label(format!(
                        "Monthly output · Food {:.0} · Metal {:.0} · Stone {:.0}",
                        production[0], production[1], production[2]
                    ));
                    ui.label(format!(
                        "Food demand {:.0} · Tax income {:.0}",
                        p.food_request(&campaign.economy.config),
                        p.tax_income(&campaign.economy.config)
                    ));
                    let buildings: Vec<_> = p
                        .buildings
                        .iter()
                        .enumerate()
                        .filter(|(_, level)| **level > 0)
                        .filter_map(|(id, level)| {
                            BuildingType::ALL
                                .get(id)
                                .map(|kind| format!("{} {}", kind.name(), level))
                        })
                        .collect();
                    ui.label(if buildings.is_empty() {
                        "Buildings: none".to_owned()
                    } else {
                        format!("Buildings: {}", buildings.join(", "))
                    });
                    if let Some(project) = &p.construction {
                        ui.label(format!("Construction: {:?}", project));
                    }
                    ui.separator();
                    ui.strong("Politics");
                    for player in 0..campaign.actors.len() {
                        ui.label(format!(
                            "Player {} · Control {:.0} · Relation {:.0}",
                            player + 1,
                            campaign.politics[province].control(player),
                            campaign.politics[province].relation(player)
                        ));
                    }
                    ui.separator();
                    ui.strong("Armies");
                    let military = &campaign.military.provinces[province];
                    if military.forces.values().all(Vec::is_empty)
                        && !campaign.military.movements.iter().any(|order| {
                            order.origin == province || order.destination() == Some(province)
                        })
                        && !campaign
                            .military
                            .battles
                            .iter()
                            .any(|battle| battle.province == province)
                    {
                        ui.label("No armies present or moving nearby.");
                    }
                    for (owner, units) in &military.forces {
                        if units.is_empty() {
                            continue;
                        }
                        ui.strong(owner_name(*owner));
                        for unit in units {
                            ui.label(cohort_line(unit));
                        }
                    }
                    for order in &campaign.military.movements {
                        if order.origin != province && order.destination() != Some(province) {
                            continue;
                        }
                        let destination = order
                            .route
                            .last()
                            .and_then(|id| campaign.economy.provinces.get(*id))
                            .map(|p| p.name.as_str())
                            .unwrap_or("unknown");
                        ui.strong(format!(
                            "{} marching toward {destination}",
                            owner_name(order.owner)
                        ));
                        for unit in &order.units {
                            ui.label(cohort_line(unit));
                        }
                    }
                    for battle in &campaign.military.battles {
                        if battle.province != province {
                            continue;
                        }
                        ui.strong("Battle in progress");
                        for unit in battle.attackers.units.iter().chain(&battle.defenders.units) {
                            ui.label(format!("{} · {}", owner_name(unit.owner), cohort_line(unit)));
                        }
                    }
                });
        });
    if !open {
        detail.0 = None;
    }
}
