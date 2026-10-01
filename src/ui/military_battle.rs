//! Live battlefield inspection using the same cohort artwork as deployment.
use super::*;

const PANEL_ID: &str = "campaign-battle-inspection";

pub(in crate::app) fn selected_battle(ctx: &egui::Context) -> Option<(u64, usize)> {
    ctx.data(|data| data.get_temp(egui::Id::new(PANEL_ID)))
}

pub(in crate::app) fn open_battle_panel(ctx: &egui::Context, battle: u64, player: usize) {
    set_selected_army(ctx, None);
    ctx.data_mut(|data| data.insert_temp(egui::Id::new(PANEL_ID), (battle, player)));
}

pub(super) fn dismiss(ctx: &egui::Context) -> bool {
    let open = selected_battle(ctx).is_some();
    ctx.data_mut(|data| data.remove::<(u64, usize)>(egui::Id::new(PANEL_ID)));
    open
}

fn participant(side: &BattleSide, player: usize) -> bool {
    side.initial_manpower.contains_key(&ForceOwner::Player(player))
}

pub(in crate::app) fn draw_battle_panel(
    ctx: &egui::Context,
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    player: usize,
) -> Option<(usize, MilitaryUiAction)> {
    let (id, viewer) = selected_battle(ctx)?;
    if viewer != player {
        dismiss(ctx);
        return None;
    }
    let battle = world.battles.iter().find(|b| b.id == id);
    let outcome = world.history.iter().find(|b| b.id == id);
    let (province, terrain, attackers, defenders, rounds, months, result) = if let Some(b) = battle
    {
        (b.province, b.terrain, &b.attackers, &b.defenders, &b.rounds, b.months, b.result)
    } else if let Some(b) = outcome {
        (
            b.province,
            b.terrain,
            &b.attackers,
            &b.defenders,
            &b.round_history,
            b.months,
            Some(b.result),
        )
    } else {
        dismiss(ctx);
        return None;
    };
    // An observer sees public strength; combat plans are revealed to participants only.
    let inspect_deployment = participant(attackers, player) || participant(defenders, player);
    let economic = economy.provinces.get(province)?;
    let scale = super::super::viewport_ui_scale(ctx.content_rect().size());
    let width = (980. * scale).min(ctx.content_rect().width() - 24. * scale);
    let max_height = (ctx.content_rect().height() - 140. * scale).max(180. * scale);
    let mut open = true;
    let mut action = None;
    let response = egui::Area::new(egui::Id::new(PANEL_ID).with("window"))
        .order(egui::Order::Foreground).anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .movable(false).show(ctx, |ui| {
        *ui.style_mut() = super::super::campaign_widgets::map_style(scale);
        egui::Frame::new().fill(super::super::province_panel::PAPER)
            .stroke(egui::Stroke::new(2. * scale, super::super::province_panel::RULE))
            .corner_radius(6.).inner_margin(12. * scale).show(ui, |ui| {
            ui.set_width(width - 24. * scale);
            ui.set_height((700. * scale).min(ctx.content_rect().height() - 40. * scale));
            ui.horizontal(|ui| {
                icon(ui, Icon::Attack, 28. * scale);
                ui.heading(format!("Battle of {}", economic.name));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { if ui.button("×").clicked() { open = false; } });
            });
            let status = match result {
                Some(BattleResult::AttackerVictory) => "Attacker victory",
                Some(BattleResult::DefenderVictory) => "Defenders hold the province",
                Some(BattleResult::MutualRout) => "Both armies routed",
                None => "Battle in progress",
            };
            ui.label(format!("{status} · {terrain:?} · {} completed months · {} rounds", months, rounds.len()));
            egui::ScrollArea::vertical().id_salt(("battle-content", id)).max_height(max_height).show(ui, |ui| {
                let landscape = match terrain {
                    MilitaryTerrain::Farmland => crate::game::economy::Terrain::Farmland,
                    MilitaryTerrain::Plains => crate::game::economy::Terrain::Plains,
                    MilitaryTerrain::Forest => crate::game::economy::Terrain::Forest,
                    MilitaryTerrain::Hills => crate::game::economy::Terrain::Hills,
                    MilitaryTerrain::Mountains => crate::game::economy::Terrain::Mountains,
                    MilitaryTerrain::Desert => crate::game::economy::Terrain::Desert,
                    MilitaryTerrain::Marsh => crate::game::economy::Terrain::Marsh,
                };
                let scene = ui.allocate_exact_size(egui::vec2(ui.available_width(), 358. * scale), egui::Sense::hover()).0;
                let image = super::super::campaign_widgets::prepare_portrait(ctx, super::super::campaign_widgets::ProvinceLandscape::Terrain(landscape));
                let aspect = image.size()[0] as f32 / image.size()[1] as f32;
                let target = scene.width() / scene.height();
                let visible = if aspect > target { egui::vec2(target / aspect, 1.) } else { egui::vec2(1., aspect / target) };
                let uv = egui::Rect::from_center_size(egui::pos2(0.5, 0.5), visible);
                ui.painter().image(image.id(), scene, uv, egui::Color32::WHITE);
                ui.painter().rect_filled(scene, 0., egui::Color32::from_black_alpha(130));
                let center = scene.center().y;
                let pulse = if result.is_some() { 0.3 } else { 0.5 + 0.3 * (ctx.input(|i| i.time) as f32 * 4.).sin() };
                ui.painter().line_segment([egui::pos2(scene.left() + 16., center), egui::pos2(scene.right() - 16., center)], egui::Stroke::new(2., egui::Color32::from_rgba_unmultiplied(221, 167, 87, (pulse * 255.) as u8)));
                for (index, side) in [attackers, defenders].into_iter().enumerate() {
                    let half = egui::Rect::from_min_size(scene.min + egui::vec2(10. * scale, (index as f32 * 179. + 6.) * scale), egui::vec2(scene.width() - 20. * scale, 166. * scale));
                    let mut side_ui = ui.new_child(egui::UiBuilder::new().max_rect(half));
                    side_ui.visuals_mut().override_text_color = Some(egui::Color32::from_rgb(255, 239, 207));
                    side_ui.spacing_mut().item_spacing.y = 3. * scale;
                    side_header(&mut side_ui, side, index, rounds.last(), world, economy, inspect_deployment, scale);
                    if inspect_deployment { battlefield_rows(&mut side_ui, side, id, index == 0, scale); }
                    else { side_ui.label("Select a battle involving your army to inspect deployment and tactics."); unit_composition(&mut side_ui, &side.units); }
                }
                ui.add_space(8. * scale);
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!("Defender terrain: +{:.0}% defense", (world.config.terrain_defense[terrain as usize] - 1.) * 100.));
                    if let Some(battle) = battle {
                        ui.label(format!("Fortifications: level {}", battle.fortification_level));
                        let siege: f64 = attackers.formation.front.iter().chain(&attackers.formation.support).flatten()
                            .filter_map(|id| attackers.units.iter().find(|u| u.id == *id))
                            .map(|u| world.config.unit(u.unit_type).siege_power * u.manpower_ratio()).sum();
                        let fort = (f64::from(battle.fortification_level) * world.config.fort_defense_per_level).min(world.config.fort_defense_cap)
                            * (1. - (siege * world.config.siege_suppression_per_point).min(world.config.siege_suppression_cap));
                        ui.label(format!("After siege suppression: +{:.0}% defense", fort * 100.));
                    }
                });
                if let Some(battle) = battle {
                    for (attacker, side) in [(true, attackers), (false, defenders)] {
                        if participant(side, player) {
                            let allowed = months >= world.config.minimum_retreat_months
                                && !side.units.iter().any(|unit| battle.trapped.contains(&unit.owner));
                            if ui.add_enabled(allowed, egui::Button::new("Retreat"))
                                .on_disabled_hover_text("Requires a completed combat month and an adjacent province permitting stationing.").clicked() {
                                action = Some((province, MilitaryUiAction::Retreat { battle: id, attacker }));
                            }
                        }
                    }
                }
                ui.separator();
                ui.strong("ROUND HISTORY");
                ui.small(format!("{} rounds per month · both sides strike simultaneously · dice are rolled once per side each round", world.config.rounds_per_month));
                if rounds.is_empty() { ui.label("Awaiting the first combat month."); }
                egui::Grid::new(("battle-rounds", id)).num_columns(5).striped(true).spacing(egui::vec2(18. * scale, 5. * scale)).show(ui, |ui| {
                    for label in ["Round", "Attacker die / bonus", "Attacker losses", "Defender die / bonus", "Defender losses"] { ui.strong(label); } ui.end_row();
                    for round in rounds.iter().rev() {
                        ui.label(round.number.to_string());
                        ui.label(format!("{} / {:+.0}%", round.dice[0], (round.dice_multipliers[0] - 1.) * 100.));
                        ui.label(format_person_count(round.casualties[0]));
                        ui.label(format!("{} / {:+.0}%", round.dice[1], (round.dice_multipliers[1] - 1.) * 100.));
                        ui.label(format_person_count(round.casualties[1])); ui.end_row();
                    }
                });
            });
        });
    });
    ctx.move_to_top(response.response.layer_id);
    if !open {
        dismiss(ctx);
    }
    if battle.is_some() {
        ctx.request_repaint_after(std::time::Duration::from_millis(33));
    }
    action
}

fn side_header(
    ui: &mut egui::Ui,
    side: &BattleSide,
    index: usize,
    round: Option<&BattleRound>,
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    reveal: bool,
    scale: f32,
) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.strong(
                egui::RichText::new(if index == 0 {
                    "ATTACKERS"
                } else {
                    "DEFENDERS"
                })
                .color(egui::Color32::from_rgb(255, 239, 207)),
            );
            let initial: f64 = side.initial_manpower.values().sum();
            ui.label(format!(
                "{} soldiers · {} lost · {:.0}% morale",
                format_people(side.manpower()),
                format_people((initial - side.manpower()).max(0.)),
                army_morale(&side.units)
            ));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            dice_face(
                ui,
                round.map(|r| r.dice[index]),
                round.map(|r| r.dice_multipliers[index]),
                scale,
            );
            if reveal {
                for (&owner, plan) in &side.plans {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            let rect = ui
                                .allocate_exact_size(
                                    egui::vec2(22., 22.) * scale,
                                    egui::Sense::hover(),
                                )
                                .0;
                            paint_tactic(ui, plan.tactic, rect);
                            ui.label(plan.tactic.name());
                        });
                        let bonus =
                            round.and_then(|r| r.tactics[index].get(&owner)).copied().unwrap_or(1.);
                        ui.small(format!(
                            "{} · {:.0}% fit · {:+.0}% damage",
                            army_name(owner, economy, world),
                            side.tactic_fit(owner, &world.config) * 100.,
                            (bonus - 1.) * 100.
                        ));
                    });
                }
            }
        });
    });
}

fn dice_face(ui: &mut egui::Ui, roll: Option<u8>, multiplier: Option<f64>, scale: f32) {
    ui.allocate_ui_with_layout(
        egui::vec2(75., 58.) * scale,
        egui::Layout::top_down(egui::Align::Center),
        |ui| {
            let rect = ui.allocate_exact_size(egui::vec2(36., 36.) * scale, egui::Sense::hover()).0;
            ui.painter().rect_filled(rect, 5., egui::Color32::from_rgb(244, 224, 185));
            let pips: &[(f32, f32)] = match roll {
                Some(1) => &[(0.5, 0.5)],
                Some(2) => &[(0.25, 0.25), (0.75, 0.75)],
                Some(3) => &[(0.25, 0.25), (0.5, 0.5), (0.75, 0.75)],
                Some(4) => &[(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)],
                Some(5) => &[(0.25, 0.25), (0.75, 0.25), (0.5, 0.5), (0.25, 0.75), (0.75, 0.75)],
                Some(6) => &[
                    (0.25, 0.25),
                    (0.75, 0.25),
                    (0.25, 0.5),
                    (0.75, 0.5),
                    (0.25, 0.75),
                    (0.75, 0.75),
                ],
                _ => &[],
            };
            for &(x, y) in pips {
                ui.painter().circle_filled(
                    rect.min + rect.size() * egui::vec2(x, y),
                    2.5 * scale,
                    egui::Color32::from_rgb(68, 38, 24),
                );
            }
            ui.small(
                multiplier
                    .map_or("Awaiting roll".into(), |m| format!("{:+.0}% dice", (m - 1.) * 100.)),
            );
        },
    );
}

fn battlefield_rows(ui: &mut egui::Ui, side: &BattleSide, battle: u64, attacker: bool, scale: f32) {
    let width = side.formation.front.len();
    let cell = ((ui.available_width() - width.saturating_sub(1) as f32 * 3. * scale)
        / width.max(1) as f32)
        .min(35. * scale);
    let rows = if attacker {
        [("Support", &side.formation.support), ("Frontline", &side.formation.front)]
    } else {
        [("Frontline", &side.formation.front), ("Support", &side.formation.support)]
    };
    for (label, row) in rows {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 3. * scale;
            let row_width = width as f32 * cell + width.saturating_sub(1) as f32 * 3. * scale;
            ui.add_space(((ui.available_width() - row_width) / 2.).max(0.));
            for (slot, id) in row.iter().enumerate() {
                let unit = id.and_then(|id| {
                    side.units.iter().find(|u| u.id == id && !side.routed.contains(&id))
                });
                let (rect, response) = ui
                    .allocate_exact_size(egui::vec2(cell, cell + 5. * scale), egui::Sense::hover());
                ui.painter().rect_filled(rect, 3., egui::Color32::from_black_alpha(95));
                let flank = slot < side.formation.flank_size
                    || slot >= width.saturating_sub(side.formation.flank_size);
                ui.painter().rect_stroke(
                    rect,
                    3.,
                    egui::Stroke::new(
                        0.8,
                        if flank {
                            egui::Color32::from_rgb(213, 157, 81)
                        } else {
                            egui::Color32::from_gray(180)
                        },
                    ),
                    egui::StrokeKind::Inside,
                );
                if let Some(unit) = unit {
                    let strength = ui.ctx().animate_value_with_time(
                        egui::Id::new(("battle-strength", battle, unit.id)),
                        unit.manpower_ratio() as f32,
                        0.5,
                    );
                    super::super::campaign_widgets::paint_raster_icon(
                        ui,
                        Icon::Unit(unit.unit_type),
                        rect.shrink2(egui::vec2(2., 3.)),
                        egui::Color32::from_white_alpha((100. + 155. * strength) as u8),
                    );
                    let bar = egui::Rect::from_min_size(
                        egui::pos2(rect.left() + 2., rect.bottom() - 4. * scale),
                        egui::vec2((cell - 4.) * strength, 3. * scale),
                    );
                    ui.painter().rect_filled(bar, 1., egui::Color32::from_rgb(149, 182, 106));
                    response.on_hover_text(format!(
                        "{} #{} · {}\n{} soldiers · {:.0}% morale",
                        unit.unit_type.name(),
                        unit.id,
                        if flank {
                            "Flank"
                        } else {
                            label
                        },
                        unit.people(),
                        unit.morale
                    ));
                }
            }
        });
    }
    ui.horizontal(|ui| {
        ui.small(format!(
            "Left flank · Frontline · Right flank    |    Reserve {} · Routed {}",
            side.formation.reserves.len(),
            side.routed.len()
        ));
    });
}

#[cfg(test)]
#[path = "../../tests/unit/military_battle.rs"]
mod tests;
