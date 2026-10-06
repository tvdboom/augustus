//! Compact current-state battlefield inspection using the shared cohort artwork.
use super::super::province_panel::{INK, PAPER, RULE, TABLE_STRIPE};
use super::*;

const PANEL_ID: &str = "campaign-battle-inspection";
const PANEL_WIDTH: f32 = 900.;
const PANEL_HEIGHT: f32 = 328.;

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

/// The strongest starting contingent supplies its coalition's banner color.
fn side_owner(side: &BattleSide) -> ForceOwner {
    side.initial_manpower
        .iter()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map_or(ForceOwner::Local(0), |(&owner, _)| owner)
}

fn panel_rect(screen: egui::Rect) -> (egui::Rect, f32) {
    let scale = super::super::viewport_ui_scale(screen.size())
        .min((screen.width() - 24.).max(1.) / PANEL_WIDTH)
        .min((screen.height() - 24.).max(1.) / PANEL_HEIGHT);
    let size = egui::vec2(PANEL_WIDTH, PANEL_HEIGHT) * scale;
    let bottom = screen.bottom() - (screen.height() * 0.01).max(8. * scale);
    (
        egui::Rect::from_min_size(
            egui::pos2(screen.center().x - size.x / 2., bottom - size.y),
            size,
        ),
        scale,
    )
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
    let (province, terrain, attackers, defenders, round) = if let Some(b) = battle {
        (b.province, b.terrain, &b.attackers, &b.defenders, b.rounds.last())
    } else if let Some(b) = outcome {
        (b.province, b.terrain, &b.attackers, &b.defenders, b.round_history.last())
    } else {
        dismiss(ctx);
        return None;
    };
    let economic = economy.provinces.get(province)?;
    let finished = outcome.is_some() || battle.is_some_and(|battle| battle.result.is_some());
    let fortification = battle
        .map(|battle| battle.fortification_level)
        .or_else(|| outcome.map(|outcome| outcome.fortification_level))
        .unwrap_or(0);
    // Public strength remains visible to observers; locked plans and deployments do not.
    let inspection = super::super::spectator::read_only(ctx);
    let reveal = inspection || participant(attackers, player) || participant(defenders, player);
    let colors = [
        army_owner_color(ctx, side_owner(attackers)),
        army_owner_color(ctx, side_owner(defenders)),
    ];
    let (rect, scale) = panel_rect(ctx.content_rect());
    let mut open = true;
    let mut action = None;
    let response = egui::Area::new(egui::Id::new(PANEL_ID).with("window"))
        .order(egui::Order::Foreground)
        .fixed_pos(rect.min)
        .movable(false)
        .sense(egui::Sense::hover())
        .show(ctx, |ui| {
            *ui.style_mut() = super::super::campaign_widgets::map_style(scale);
            let panel = ui.allocate_exact_size(rect.size(), egui::Sense::hover()).0;
            ui.painter().rect_filled(panel, 6. * scale, PAPER);
            let header =
                egui::Rect::from_min_size(panel.min, egui::vec2(panel.width(), 42. * scale));
            battle_header(
                ui,
                header,
                &format!("Battle of {}", economic.name),
                colors,
                scale,
                &mut open,
            );
            defense_icons(ui, header, terrain, fortification, attackers, world, scale);
            let field = egui::Rect::from_min_size(
                panel.min + egui::vec2(12., 92.) * scale,
                egui::vec2(panel.width() - 24. * scale, 154. * scale),
            );
            battlefield_background(ui, field, terrain, scale);
            for (index, side) in [attackers, defenders].into_iter().enumerate() {
                let card = egui::Rect::from_min_size(
                    panel.min
                        + egui::vec2(
                            10.,
                            if index == 0 {
                                46.
                            } else {
                                248.
                            },
                        ) * scale,
                    egui::vec2(
                        panel.width() - 20. * scale,
                        if index == 0 {
                            42.
                        } else {
                            50.
                        } * scale,
                    ),
                );
                let mut side_ui = ui.new_child(
                    egui::UiBuilder::new().id_salt(("battle-side", index)).max_rect(card),
                );
                side_card(&mut side_ui, card, side, index, round, world, economy, reveal, scale);
                if let Some(battle) =
                    battle.filter(|battle| battle.result.is_none() && participant(side, player))
                {
                    let allowed = !inspection
                        && battle.months >= world.config.minimum_retreat_months
                        && !side.units.iter().any(|unit| battle.trapped.contains(&unit.owner));
                    let retreat =
                        side_dice_rect(card, scale).translate(egui::vec2(-34. * scale, 0.));
                    if retreat_icon(&mut side_ui, retreat, allowed, scale) {
                        action = Some((
                            province,
                            MilitaryUiAction::Retreat {
                                battle: id,
                                attacker: index == 0,
                            },
                        ));
                    }
                }
                let rows = egui::Rect::from_min_size(
                    field.min + egui::vec2(0., 3. + index as f32 * 76.) * scale,
                    egui::vec2(field.width(), 72. * scale),
                );
                if reveal {
                    battlefield_rows(
                        &side_ui,
                        rows,
                        side,
                        id,
                        index == 0,
                        finished,
                        &world.config,
                        scale,
                    );
                } else {
                    public_composition(&side_ui, rows, side, finished, scale);
                }
            }
            ui.painter().rect_stroke(
                panel,
                6. * scale,
                egui::Stroke::new(1.5 * scale, RULE),
                egui::StrokeKind::Inside,
            );
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

fn battle_header(
    ui: &egui::Ui,
    rect: egui::Rect,
    title: &str,
    colors: [egui::Color32; 2],
    scale: f32,
    open: &mut bool,
) {
    let mut font = egui::FontId::proportional(20. * scale);
    let available = rect.width() - 196. * scale;
    let measured = ui.painter().layout_no_wrap(title.to_owned(), font.clone(), INK).size().x;
    font.size *= (available / measured.max(1.)).min(1.);
    for (index, color) in colors.into_iter().enumerate() {
        let half = egui::Rect::from_min_max(
            egui::pos2(rect.left() + index as f32 * rect.width() / 2., rect.top()),
            egui::pos2(rect.left() + (index + 1) as f32 * rect.width() / 2., rect.bottom()),
        );
        ui.painter().rect_filled(
            half,
            egui::CornerRadius {
                nw: if index == 0 {
                    (5. * scale) as u8
                } else {
                    0
                },
                ne: if index == 1 {
                    (5. * scale) as u8
                } else {
                    0
                },
                sw: 0,
                se: 0,
            },
            color,
        );
        let (_, ink) = super::super::province_panel::header_close_colors(color, false, false);
        ui.painter().with_clip_rect(half).text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            title,
            font.clone(),
            ink,
        );
    }
    paint_icon(
        ui,
        Icon::Attack,
        egui::Rect::from_min_size(
            rect.min + egui::vec2(9., 7.) * scale,
            egui::vec2(28., 28.) * scale,
        ),
    );
    let close = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 21. * scale, rect.center().y),
        egui::vec2(28., 28.) * scale,
    );
    let response = ui.interact(close, egui::Id::new((PANEL_ID, "close")), egui::Sense::click());
    let (fill, ink) = super::super::province_panel::header_close_colors(
        colors[1],
        response.hovered(),
        response.is_pointer_button_down_on() || response.clicked(),
    );
    ui.painter().circle(close.center(), 12. * scale, fill, egui::Stroke::new(1.3 * scale, ink));
    for sign in [-1., 1.] {
        ui.painter().line_segment(
            [
                close.center() + egui::vec2(-4.5, sign * -4.5) * scale,
                close.center() + egui::vec2(4.5, sign * 4.5) * scale,
            ],
            egui::Stroke::new(1.7 * scale, ink),
        );
    }
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Close battle panel")
    });
    if response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        *open = false;
    }
}

#[allow(clippy::too_many_arguments)]
fn side_card(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    side: &BattleSide,
    index: usize,
    round: Option<&BattleRound>,
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    reveal: bool,
    scale: f32,
) {
    let inset = rect.shrink2(egui::vec2(8., 2.) * scale);
    let info_width = inset.width() - 166. * scale;
    let names = side
        .initial_manpower
        .keys()
        .map(|&owner| army_name(owner, economy, world))
        .collect::<Vec<_>>()
        .join(" + ");
    let identity = egui::Rect::from_min_size(
        inset.min
            + egui::vec2(
                0.,
                if index == 0 {
                    0.
                } else {
                    28.
                },
            ) * scale,
        egui::vec2(info_width, 20. * scale),
    );
    let mut name_font = egui::FontId::proportional(16. * scale);
    let measured = ui.painter().layout_no_wrap(names.clone(), name_font.clone(), INK).size().x;
    name_font.size *= (info_width / measured.max(1.)).clamp(0.75, 1.);
    let mut name_job = egui::text::LayoutJob::default();
    for (i, &owner) in side.initial_manpower.keys().enumerate() {
        if i > 0 {
            name_job.append(
                " + ",
                0.,
                egui::TextFormat {
                    font_id: name_font.clone(),
                    color: INK,
                    ..Default::default()
                },
            );
        }
        name_job.append(
            &army_name(owner, economy, world),
            0.,
            egui::TextFormat {
                font_id: name_font.clone(),
                color: army_owner_color(ui.ctx(), owner),
                ..Default::default()
            },
        );
    }
    let mut name_ui = ui.new_child(egui::UiBuilder::new().max_rect(identity));
    name_ui.add(egui::Label::new(name_job).truncate().show_tooltip_when_elided(false));
    let dice = side_dice_rect(rect, scale);
    dice_face(
        ui,
        dice,
        round.map(|r| r.dice[index]),
        round.map(|r| r.dice_multipliers[index]),
        scale,
    );
    if reveal {
        let owner = side_owner(side);
        if let Some(plan) = side.plans.get(&owner) {
            let tactic = dice.translate(egui::vec2(34. * scale, 0.));
            paint_tactic(ui, plan.tactic, tactic);
            let multiplier =
                round.and_then(|r| r.tactics[index].get(&owner)).copied().unwrap_or(1.);
            let modifier = ((multiplier - 1.) * 100.).round() as i32;
            let color = if modifier > 0 {
                egui::Color32::from_rgb(32, 116, 58)
            } else if modifier < 0 {
                egui::Color32::from_rgb(166, 44, 34)
            } else {
                INK
            };
            let damage = egui::Rect::from_min_max(
                egui::pos2(tactic.right() + 8. * scale, tactic.top()),
                egui::pos2(inset.right(), tactic.bottom()),
            );
            ui.painter().text(
                damage.left_center(),
                egui::Align2::LEFT_CENTER,
                if modifier == 0 {
                    "0%".to_owned()
                } else {
                    format!("{modifier:+}%")
                },
                egui::FontId::proportional(14. * scale),
                color,
            );
            ui.interact(tactic.union(damage), ui.id().with("tactic"), egui::Sense::hover())
                .on_hover_text(plan.tactic.name());
        }
    }
    let initial: f64 = side.initial_manpower.values().sum();
    let people = |manpower: f64| {
        format_person_count((manpower.max(0.) * PEOPLE_PER_POPULATION).round() as u64)
    };
    let mut stats = vec![
        (Icon::Population, people(side.manpower()), "Surviving soldiers"),
        (Icon::Morale, format!("{:.0}%", army_morale(&side.units)), "Current army morale"),
        (
            Icon::Cohorts,
            side.units.iter().filter(|unit| unit.current_manpower > 0.).count().to_string(),
            "Surviving cohorts",
        ),
        (Icon::Cancel, people(initial - side.manpower()), "Soldiers lost in this battle"),
    ];
    if reveal {
        stats.extend([
            (Icon::Cohorts, side.formation.reserves.len().to_string(), "Cohorts in reserve"),
            (Icon::SpyFlee, side.routed.len().to_string(), "Cohorts routed from this battle"),
        ]);
    }
    for (i, (kind, value, tooltip)) in stats.into_iter().enumerate() {
        let fact = egui::Rect::from_min_size(
            inset.min
                + egui::vec2(
                    i as f32 * 80.,
                    if index == 0 {
                        20.
                    } else {
                        0.
                    },
                ) * scale,
            egui::vec2(80., 22.) * scale,
        );
        compact_stat(ui, fact, kind, &value, tooltip, scale);
    }
}

fn side_dice_rect(rect: egui::Rect, scale: f32) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(rect.right() - 132. * scale, rect.center().y - 14. * scale),
        egui::vec2(28., 28.) * scale,
    )
}

fn retreat_icon(ui: &mut egui::Ui, rect: egui::Rect, enabled: bool, scale: f32) -> bool {
    let response = ui
        .add_enabled_ui(enabled, |ui| {
            let response = ui.interact(rect, ui.id().with("retreat"), egui::Sense::click());
            if response.hovered() || response.has_focus() {
                let visuals = ui.style().interact(&response);
                ui.painter().rect_filled(rect, 4. * scale, visuals.weak_bg_fill);
            }
            paint_icon(ui, Icon::SpyFlee, rect.shrink(4. * scale));
            response
        })
        .inner;
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, "Retreat"));
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text("Retreat")
        .on_disabled_hover_text("Retreat\nRequires a completed combat month and an adjacent province permitting stationing.")
        .clicked()
}

fn compact_stat(
    ui: &egui::Ui,
    rect: egui::Rect,
    kind: Icon,
    value: &str,
    tooltip: &str,
    scale: f32,
) {
    let art = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 11. * scale, rect.center().y),
        egui::vec2(22., 22.) * scale,
    );
    paint_icon(ui, kind, art);
    ui.painter().text(
        egui::pos2(art.right() + 3. * scale, rect.center().y),
        egui::Align2::LEFT_CENTER,
        value,
        egui::FontId::proportional(13. * scale),
        INK,
    );
    ui.interact(rect, ui.id().with(tooltip), egui::Sense::hover())
        .on_hover_text(format!("{tooltip}: {value}"));
}

fn dice_face(
    ui: &egui::Ui,
    rect: egui::Rect,
    roll: Option<u8>,
    multiplier: Option<f64>,
    scale: f32,
) {
    ui.painter().rect_filled(rect, 4. * scale, egui::Color32::from_rgb(244, 224, 185));
    ui.painter().rect_stroke(
        rect,
        4. * scale,
        egui::Stroke::new(scale, RULE),
        egui::StrokeKind::Inside,
    );
    let pips: &[(f32, f32)] = match roll {
        Some(1) => &[(0.5, 0.5)],
        Some(2) => &[(0.25, 0.25), (0.75, 0.75)],
        Some(3) => &[(0.25, 0.25), (0.5, 0.5), (0.75, 0.75)],
        Some(4) => &[(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)],
        Some(5) => &[(0.25, 0.25), (0.75, 0.25), (0.5, 0.5), (0.25, 0.75), (0.75, 0.75)],
        Some(6) => {
            &[(0.25, 0.25), (0.75, 0.25), (0.25, 0.5), (0.75, 0.5), (0.25, 0.75), (0.75, 0.75)]
        },
        _ => &[],
    };
    for &(x, y) in pips {
        ui.painter().circle_filled(rect.min + rect.size() * egui::vec2(x, y), 2. * scale, INK);
    }
    if roll.is_none() {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "–",
            egui::FontId::proportional(16. * scale),
            INK,
        );
    }
    let tooltip = roll.map_or_else(
        || "Awaiting the first combat roll".to_owned(),
        |roll| {
            format!("Current die: {roll}\n{:+.0}% damage", (multiplier.unwrap_or(1.) - 1.) * 100.)
        },
    );
    ui.interact(rect, ui.id().with("dice"), egui::Sense::hover()).on_hover_text(tooltip);
}

fn terrain_portrait(ctx: &egui::Context, terrain: MilitaryTerrain) -> egui::TextureHandle {
    use super::super::campaign_widgets::{prepare_portrait, ProvinceLandscape};
    use crate::game::economy::Terrain;
    let terrain = match terrain {
        MilitaryTerrain::Farmland => Terrain::Farmland,
        MilitaryTerrain::Plains => Terrain::Plains,
        MilitaryTerrain::Forest => Terrain::Forest,
        MilitaryTerrain::Hills => Terrain::Hills,
        MilitaryTerrain::Mountains => Terrain::Mountains,
        MilitaryTerrain::Desert => Terrain::Desert,
        MilitaryTerrain::Marsh => Terrain::Marsh,
    };
    prepare_portrait(ctx, ProvinceLandscape::Terrain(terrain))
}

fn battlefield_background(ui: &egui::Ui, rect: egui::Rect, terrain: MilitaryTerrain, scale: f32) {
    let image = terrain_portrait(ui.ctx(), terrain);
    let aspect = image.size()[0] as f32 / image.size()[1] as f32;
    let target = rect.width() / rect.height();
    let visible = egui::vec2((target / aspect).min(1.), (aspect / target).min(1.));
    let uv = egui::Rect::from_center_size(egui::pos2(0.5, 0.5), visible);
    ui.painter().image(image.id(), rect, uv, egui::Color32::WHITE);
    ui.painter().rect_stroke(rect, 0., egui::Stroke::new(scale, RULE), egui::StrokeKind::Inside);
}

fn defense_icons(
    ui: &egui::Ui,
    header: egui::Rect,
    terrain: MilitaryTerrain,
    fortification: u32,
    attackers: &BattleSide,
    world: &MilitaryWorld,
    scale: f32,
) {
    let terrain_rect = egui::Rect::from_center_size(
        egui::pos2(header.right() - 57. * scale, header.center().y),
        egui::vec2(30., 24.) * scale,
    );
    paint_icon(
        ui,
        Icon::Terrain,
        egui::Rect::from_center_size(terrain_rect.center(), egui::Vec2::splat(24. * scale)),
    );
    ui.interact(terrain_rect, ui.id().with("battle-terrain"), egui::Sense::hover()).on_hover_ui(
        |ui| {
            ui.label(
                egui::RichText::new(format!(
                    "{terrain:?} · Width: {} cohorts",
                    world.config.combat_widths[terrain as usize]
                ))
                .size(14. * scale)
                .color(INK),
            );
        },
    );
    if fortification > 0 {
        let fort_rect = egui::Rect::from_center_size(
            terrain_rect.center() - egui::vec2(32. * scale, 0.),
            egui::vec2(24., 24.) * scale,
        );
        paint_icon(ui, Icon::Building(BuildingType::CityWalls), fort_rect);
        let siege: f64 = attackers
            .formation
            .front
            .iter()
            .chain(&attackers.formation.support)
            .flatten()
            .filter_map(|id| attackers.units.iter().find(|u| u.id == *id))
            .map(|u| world.config.unit(u.unit_type).siege_power * u.manpower_ratio())
            .sum();
        let fort = (f64::from(fortification) * world.config.fort_defense_per_level)
            .min(world.config.fort_defense_cap)
            * (1.
                - (siege * world.config.siege_suppression_per_point)
                    .min(world.config.siege_suppression_cap));
        ui.interact(fort_rect, ui.id().with("battle-fort"), egui::Sense::hover()).on_hover_text(
            format!(
                "Fortifications: level {}\n{:+.0}% defense after siege suppression",
                fortification,
                fort * 100.
            ),
        );
    }
}

fn public_composition(
    ui: &egui::Ui,
    rect: egui::Rect,
    side: &BattleSide,
    finished: bool,
    scale: f32,
) {
    let units = if finished {
        &side.initial_units
    } else {
        &side.units
    };
    let mut slot = 0;
    for kind in UnitType::ALL {
        let count = units
            .iter()
            .filter(|unit| unit.unit_type == kind && unit.current_manpower > 0.)
            .count();
        if count == 0 {
            continue;
        }
        let tile = egui::Rect::from_min_size(
            rect.min
                + egui::vec2(
                    (slot % 6) as f32 * rect.width() / 6.,
                    (slot / 6) as f32 * 29. * scale,
                ),
            egui::vec2(rect.width() / 6., 28. * scale),
        );
        let survivors = side
            .units
            .iter()
            .filter(|unit| unit.unit_type == kind && unit.current_manpower > 0.)
            .count();
        super::super::campaign_widgets::paint_raster_icon(
            ui,
            Icon::Unit(kind),
            tile.shrink2(egui::vec2(5., 1.) * scale),
            egui::Color32::from_white_alpha(if survivors == 0 {
                90
            } else {
                255
            }),
        );
        ui.interact(tile, ui.id().with(("composition", kind)), egui::Sense::hover()).on_hover_text(
            format!(
                "{} · {}\n{survivors} surviving cohorts",
                kind.name(),
                cohort_count_label(count)
            ),
        );
        slot += 1;
    }
}

fn battlefield_rows(
    ui: &egui::Ui,
    rect: egui::Rect,
    side: &BattleSide,
    battle: u64,
    attacker: bool,
    finished: bool,
    config: &MilitaryConfig,
    scale: f32,
) {
    // A completed battle retains its starting composition, including destroyed
    // cohorts. Live battles continue to display the authoritative deployment.
    let initial_formation;
    let (formation, units) = if finished {
        initial_formation = deploy_formation(
            &side.initial_units,
            &side.plans,
            side.formation.front.len(),
            &BTreeSet::new(),
            config,
        );
        (&initial_formation, &side.initial_units)
    } else {
        (&side.formation, &side.units)
    };
    let width = formation.front.len();
    if width == 0 {
        return;
    }
    let gap = 6. * scale;
    let spacing = 2. * scale;
    let wing = formation.flank_size.min(width / 3);
    let cell = ((rect.width() - width.saturating_sub(1) as f32 * spacing - 2. * gap)
        / width as f32)
        .min(30. * scale);
    let row_width = width as f32 * cell + width.saturating_sub(1) as f32 * spacing + 2. * gap;
    let mut rear = formation.support.clone();
    rear.resize(width, None);
    // Match the deployment preview's rear reserves while keeping support aligned
    // with the front slots that protect it during actual combat.
    let mut reserves: Vec<_> = formation
        .reserves
        .iter()
        .filter_map(|&id| {
            units.iter().find(|unit| {
                unit.id == id
                    && unit.current_manpower > 0.
                    && (finished || !side.routed.contains(&id))
                    && !unit.unit_type.is_support()
            })
        })
        .collect();
    reserves.sort_by_key(|unit| {
        let plan = side.plans.get(&unit.owner).copied().unwrap_or_default();
        (unit.unit_type != plan.secondary_unit_type, unit.id)
    });
    let mut reserves = reserves.into_iter();
    for slot in &mut rear[wing..width - wing] {
        if slot.is_none() {
            *slot = reserves.next().map(|unit| unit.id);
        }
    }
    let rows = if attacker {
        [&rear, &formation.front]
    } else {
        [&formation.front, &rear]
    };
    for (row_index, row) in rows.into_iter().enumerate() {
        for (slot, id) in row.iter().enumerate() {
            let flank_gaps = usize::from(slot >= wing) + usize::from(slot >= width - wing);
            let tile = egui::Rect::from_min_size(
                rect.min
                    + egui::vec2(
                        (rect.width() - row_width) / 2.
                            + slot as f32 * (cell + spacing)
                            + flank_gaps as f32 * gap,
                        row_index as f32 * (cell + 8. * scale),
                    ),
                egui::vec2(cell, cell + 4. * scale),
            );
            ui.painter().rect_filled(tile, 3. * scale, TABLE_STRIPE);
            ui.painter().rect_stroke(
                tile,
                3. * scale,
                egui::Stroke::new(0.8 * scale, RULE),
                egui::StrokeKind::Inside,
            );
            let initial = id.and_then(|id| units.iter().find(|unit| unit.id == id));
            let current = id.and_then(|id| {
                side.units.iter().find(|unit| {
                    unit.id == id
                        && unit.current_manpower > 0.
                        && (finished || !side.routed.contains(&id))
                })
            });
            let Some(unit) = current.or(initial.filter(|_| finished)) else {
                ui.painter().text(
                    tile.center(),
                    egui::Align2::CENTER_CENTER,
                    "·",
                    egui::FontId::proportional(20. * scale),
                    RULE,
                );
                continue;
            };
            let response =
                ui.interact(tile, ui.id().with(("cohort", row_index, slot)), egui::Sense::hover());
            let fallen = current.is_none();
            let routed = side.routed.contains(&unit.id);
            let strength = if fallen {
                0.
            } else {
                ui.ctx().animate_value_with_time(
                    egui::Id::new(("battle-strength", battle, unit.id)),
                    unit.manpower_ratio() as f32,
                    0.5,
                )
            };
            super::super::campaign_widgets::paint_raster_icon(
                ui,
                Icon::Unit(unit.unit_type),
                egui::Rect::from_min_size(tile.min, egui::vec2(cell, cell)).shrink(2. * scale),
                egui::Color32::from_white_alpha(if fallen || routed {
                    90
                } else {
                    (100. + 155. * strength) as u8
                }),
            );
            let track = egui::Rect::from_min_size(
                egui::pos2(tile.left() + scale, tile.bottom() - 3. * scale),
                egui::vec2(cell - 2. * scale, 2. * scale),
            );
            ui.painter().rect_filled(track, scale, RULE.gamma_multiply(0.45));
            let bar = egui::Rect::from_min_size(
                track.min,
                egui::vec2(track.width() * strength, track.height()),
            );
            ui.painter().rect_filled(bar, scale, army_owner_color(ui.ctx(), unit.owner));
            response.on_hover_ui(|ui| {
                ui.label(egui::RichText::new(unit.unit_type.name()).size(14. * scale).color(INK));
                ui.label(
                    egui::RichText::new(format!(
                        "• Manpower: {}\n• Morale: {:.0}%\n• Training: {:.0}%",
                        format_person_count(if fallen {
                            0
                        } else {
                            unit.people()
                        }),
                        if fallen {
                            0.
                        } else {
                            unit.morale
                        },
                        unit.training
                    ))
                    .size(14. * scale)
                    .color(INK),
                );
            });
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/military_battle.rs"]
mod tests;
