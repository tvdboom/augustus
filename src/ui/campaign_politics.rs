//! Political career cards, faction approval badges and an interactive Senate chamber.
use super::campaign_widgets::{paint_purchase_background, Icon};
use super::province_panel::{PAPER, RULE};
use crate::game::politics::espionage::{EspionageConfig, EspionageState};
use crate::game::politics::senate::{
    Bloc, PoliticalProfile, SenateConfig, SenateState, SenatorAction,
};
use crate::game::politics::{Currency, PoliticalPlayer, PoliticalRank};
use bevy_egui::egui;

const INK: egui::Color32 = egui::Color32::from_rgb(57, 43, 37);
const UNDECIDED: egui::Color32 = egui::Color32::from_rgb(145, 143, 134);
const POSITIVE: egui::Color32 = egui::Color32::from_rgb(32, 116, 58);
const NEGATIVE: egui::Color32 = egui::Color32::from_rgb(166, 44, 34);

#[cfg(test)]
#[path = "../../tests/unit/senate_ui.rs"]
mod tests;

/// Two sections: paid political promotions, then public approval and personal actions.
pub(in crate::app) fn show(
    ui: &mut egui::Ui,
    senate: &mut SenateState,
    players: &mut [PoliticalPlayer],
    _profiles: &[PoliticalProfile],
    config: &SenateConfig,
    player: usize,
    espionage: &mut EspionageState,
    espionage_config: &EspionageConfig,
    player_colors: &[egui::Color32],
) -> Option<String> {
    players.get(player)?;
    let scale = super::viewport_ui_scale(ui.ctx().content_rect().size());
    let mut message = None;
    ui.spacing_mut().item_spacing.y = 3.0 * scale;
    super::campaign_widgets::portrait(
        ui,
        super::campaign_widgets::ProvinceLandscape::RomeSenate,
        76.0 * scale,
    );
    super::policy_widgets::section_with_height(ui, scale, "POLITICAL RANK", 28.0);
    rank_ladder(ui, senate, players, config, player, scale, &mut message);
    ui.add_space(6.0 * scale);
    super::policy_widgets::section_with_height(ui, scale, "SENATE", 28.0);
    if let Some(winner) = senate.winner {
        ui.heading(format!("Player {} is Augustus · victory", winner + 1));
    }
    faction_badges(ui, senate, config, player, scale);
    if let Some((id, anchor)) = draw_chamber(ui, senate, config, player, player_colors) {
        let selection_key = ui.id().with(("senator-selection", player));
        let popup_id = selection_key.with("actions");
        let popup = egui::Popup::from_response(&anchor).id(popup_id).width(650.0 * scale);
        let hovering = ui.ctx().pointer_hover_pos().is_some_and(|point| {
            anchor.rect.expand(4.0 * scale).contains(point)
                || popup
                    .get_popup_rect()
                    .is_some_and(|rect| rect.expand(4.0 * scale).contains(point))
        });
        if hovering && !ui.ctx().input(|input| input.key_pressed(egui::Key::Escape)) {
            let actions = senator_popover(&anchor, popup_id, 650.0 * scale, scale, true, |ui| {
                senator_actions(
                    ui,
                    senate,
                    players,
                    config,
                    player,
                    id,
                    espionage,
                    espionage_config,
                    player_colors,
                    scale,
                    &mut message,
                );
            });
            // The rank roadmap is Foreground and the decorative map standard is
            // Debug. Keep the interactive menu above both, regardless of paint order.
            ui.ctx().set_sublayer(
                egui::LayerId::new(egui::Order::Debug, egui::Id::new("augustus_map_standard_flag")),
                actions.response.layer_id,
            );
        } else {
            ui.ctx().data_mut(|d| d.remove::<usize>(selection_key));
        }
    }
    ui.add_space(6.0 * scale);
    senate_legend(ui, senate, players.len(), player_colors, scale);
    message
}

/// Keep egui's screen-aware popup placement while painting above the map HUD.
fn senator_popover<R>(
    anchor: &egui::Response,
    id: egui::Id,
    width: f32,
    scale: f32,
    interactable: bool,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let popup = egui::Popup::from_response(anchor).id(id).width(width);
    let (pivot, position) =
        popup.get_best_align().pivot_pos(&popup.get_anchor_rect().unwrap_or(anchor.rect), 0.0);
    egui::Area::new(id)
        .order(egui::Order::Debug)
        .pivot(pivot)
        .fixed_pos(position)
        .movable(false)
        .interactable(interactable)
        .sense(if interactable {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        })
        .default_width(width)
        .sizing_pass(anchor.ctx.read_response(id).is_none())
        .show(&anchor.ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(PAPER)
                .stroke(egui::Stroke::new(scale, RULE))
                .corner_radius(8.0 * scale)
                .inner_margin(egui::Margin::same((12.0 * scale) as i8))
                .show(ui, contents)
                .inner
        })
}

fn senator_actions(
    ui: &mut egui::Ui,
    senate: &mut SenateState,
    players: &mut [PoliticalPlayer],
    config: &SenateConfig,
    player: usize,
    id: usize,
    espionage: &mut EspionageState,
    espionage_config: &EspionageConfig,
    colors: &[egui::Color32],
    scale: f32,
    message: &mut Option<String>,
) {
    let seat = &senate.senators[id];
    ui.horizontal(|ui| {
        let (rect, _) =
            ui.allocate_exact_size(egui::Vec2::splat(44.0 * scale), egui::Sense::hover());
        paint_faction_icon(ui.painter(), seat.bloc, rect.shrink(4.0 * scale));
        ui.vertical(|ui| {
            ui.heading(format!("Senator {}", id + 1));
            ui.label(seat.bloc.label());
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (color, label) = seat.allegiance.map_or((UNDECIDED, "Neutral".to_owned()), |p| {
                (player_color(colors, p), player_name(p))
            });
            color_key(ui, color, &label, scale);
        });
    });
    ui.add_space(5.0 * scale);
    ui.separator();
    ui.add_space(5.0 * scale);
    let gap = 8.0 * scale;
    let columns = if ui.available_width() >= 560.0 * scale {
        3
    } else {
        2
    };
    let size = egui::vec2(
        (ui.available_width() - (columns - 1) as f32 * gap) / columns as f32,
        72.0 * scale,
    );
    let rows = SenatorAction::ALL.len().div_ceil(columns);
    let (grid, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), rows as f32 * (size.y + gap) - gap),
        egui::Sense::hover(),
    );
    for (index, action) in SenatorAction::ALL.into_iter().enumerate() {
        let active = senate.senators[id].arrangement(player).is_some_and(|a| a.action == action);
        let quote = senate.senator_action_quote(player, id, action, players, config);
        let currency = if action.currency() == Currency::Coin {
            Icon::Coin
        } else {
            Icon::Influence
        };
        let mut costs = vec![(currency, format!("{:.0}", config.action_costs[action as usize]))];
        if let Some(upkeep) = action.upkeep(config) {
            costs.push((currency, format!("{upkeep:.0}/mo")));
        }
        let (title, icon) = action_presentation(action);
        let rect = egui::Rect::from_min_size(
            grid.min
                + egui::vec2(
                    (index % columns) as f32 * (size.x + gap),
                    (index / columns) as f32 * (size.y + gap),
                ),
            size,
        );
        let response = super::campaign_panel::diplomacy_action_card_with_title_inset(
            ui,
            rect,
            ui.id().with(("senator-action", id, action as usize)),
            title,
            icon,
            &costs,
            quote.is_ok(),
            scale,
            if active {
                27.0
            } else {
                0.0
            },
        );
        let cancel = active
            .then(|| senator_cancel_button(ui, rect, id, action, senate.winner.is_none(), scale));
        let clicked = response.clicked();
        if !cancel.as_ref().is_some_and(|cancel| cancel.hovered())
            && egui::Tooltip::should_show_tooltip(&response, false)
        {
            let tooltip = senator_popover(
                &response,
                response.id.with("description"),
                ui.spacing().tooltip_width,
                scale,
                false,
                |ui| {
                    ui.label(action.description());
                    let risk = config.action_risks[action as usize];
                    if risk > 0.0 {
                        ui.colored_label(
                            NEGATIVE,
                            format!("Chance of exposure: {:.0}%", risk * 100.0),
                        );
                    }
                    if let Err(error) = &quote {
                        ui.add_space(5.0 * scale);
                        let reason = if *error
                            == crate::game::politics::PoliticalError::InsufficientFunds
                        {
                            match action.currency() {
                                Currency::Coin => {
                                    "You do not have enough sestertii for this action.".to_owned()
                                },
                                Currency::Influence => {
                                    "You do not have enough Influence for this action.".to_owned()
                                },
                            }
                        } else {
                            error.to_string()
                        };
                        super::campaign_widgets::unavailable_reason(ui, &reason, scale);
                    }
                },
            );
            // Action descriptions must also clear the menu's topmost layer.
            ui.ctx().move_to_top(tooltip.response.layer_id);
        }
        if let Some(cancel) = cancel {
            if egui::Tooltip::should_show_tooltip(&cancel, false) {
                let tooltip = senator_popover(
                    &cancel,
                    cancel.id.with("description"),
                    ui.spacing().tooltip_width,
                    scale,
                    false,
                    |ui| {
                        ui.label("Cancel action");
                        ui.label(match action {
                            SenatorAction::Lobby => "You are already lobbying this senator.",
                            SenatorAction::Bribe => "You are already bribing this senator.",
                            _ => "You have an ongoing action with this senator.",
                        });
                    },
                );
                ui.ctx().move_to_top(tooltip.response.layer_id);
            }
            if cancel.clicked() {
                *message = Some(match senate.cancel_senator_action(player, id) {
                    Ok(()) => format!(
                        "{} cancelled. Earned confidence will gradually fade.",
                        action.label()
                    ),
                    Err(error) => error.to_string(),
                });
            }
        }
        if clicked {
            *message = Some(match senate.act_on_senator(player, id, action, players, config) {
                Ok(outcome) => {
                    if let Some((kind, severity)) = outcome.misconduct {
                        espionage.record_action(
                            player,
                            None,
                            kind,
                            severity,
                            senate.month,
                            espionage_config,
                        );
                    }
                    if outcome.caught {
                        format!(
                            "{} exposed! Faction confidence fell · {}.",
                            action.label(),
                            outcome.misconduct.unwrap().1.label()
                        )
                    } else if action == SenatorAction::Assassinate {
                        format!("Senator {} replaced by a neutral successor.", id + 1)
                    } else {
                        let result = match action {
                            SenatorAction::Petition => "petition sent",
                            SenatorAction::Gift => "gift sent",
                            SenatorAction::Patronage => "patronage started",
                            SenatorAction::Bribe => "bribery started",
                            SenatorAction::Threaten => "coercion started",
                            SenatorAction::Assassinate => "replacement installed",
                            SenatorAction::Banquet => "banquet held",
                            SenatorAction::Discredit => "discrediting started",
                            SenatorAction::Lobby => "lobbying started",
                        };
                        format!("Senator {}: {result}.", id + 1)
                    }
                },
                Err(error) => error.to_string(),
            });
        }
    }
}

fn senator_cancel_button(
    ui: &mut egui::Ui,
    card: egui::Rect,
    senator: usize,
    action: SenatorAction,
    enabled: bool,
    scale: f32,
) -> egui::Response {
    let rect = egui::Rect::from_min_size(
        egui::pos2(card.right() - 27.0 * scale, card.top() + 5.0 * scale),
        egui::Vec2::splat(22.0 * scale),
    );
    let response = ui.interact(
        rect,
        ui.id().with(("cancel-senator-action", senator, action as usize)),
        if enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    let fill = if enabled && response.is_pointer_button_down_on() {
        ui.visuals().widgets.active.bg_fill
    } else if enabled && response.hovered() {
        ui.visuals().widgets.hovered.bg_fill
    } else {
        PAPER
    };
    ui.painter().rect_filled(rect, 3.0 * scale, fill);
    ui.painter().rect_stroke(
        rect,
        3.0 * scale,
        egui::Stroke::new(scale, RULE),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "×",
        egui::FontId::proportional(18.0 * scale),
        if enabled {
            INK
        } else {
            RULE
        },
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, "Cancel action")
    });
    if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    }
}

fn action_presentation(action: SenatorAction) -> (&'static str, Icon) {
    match action {
        SenatorAction::Petition => ("Petition", Icon::SenatorPetition),
        SenatorAction::Gift => ("Send a gift", Icon::SenatorGift),
        SenatorAction::Patronage => ("Patronage", Icon::SenatorPatronage),
        SenatorAction::Bribe => ("Bribe", Icon::SenatorBribe),
        SenatorAction::Threaten => ("Threaten", Icon::SenatorThreaten),
        SenatorAction::Assassinate => ("Murder", Icon::SenatorMurder),
        SenatorAction::Banquet => ("Public banquet", Icon::SenatorBanquet),
        SenatorAction::Discredit => ("Discredit patron", Icon::SenatorDiscredit),
        SenatorAction::Lobby => ("Lobby senator", Icon::SenatorLobby),
    }
}

fn player_name(player: usize) -> String {
    format!("Player {}", player + 1)
}

fn rank_ladder(
    ui: &mut egui::Ui,
    senate: &mut SenateState,
    players: &mut [PoliticalPlayer],
    config: &SenateConfig,
    player: usize,
    scale: f32,
    message: &mut Option<String>,
) {
    let current = players[player].rank;
    let next = config.requirements(current, players.len()).map(|r| r.rank);
    let eligibility = senate.promotion_eligibility(player, players, config);
    let key = egui::Id::new("senate-rank-textures");
    let mut textures = ui
        .ctx()
        .data(|d| d.get_temp::<[Option<egui::TextureHandle>; 6]>(key))
        .unwrap_or_else(|| std::array::from_fn(|_| None));
    ui.columns(6, |columns| {
        for (index, (ui, rank)) in columns
            .iter_mut()
            .zip([
                PoliticalRank::Quaestor,
                PoliticalRank::Aedile,
                PoliticalRank::Praetor,
                PoliticalRank::Censor,
                PoliticalRank::Consul,
                PoliticalRank::Augustus,
            ])
            .enumerate()
        {
            let available = next == Some(rank) && eligibility.is_ok();
            let emphasized = available || index == current.ladder_index();
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), 68.0 * scale),
                if available {
                    egui::Sense::click()
                } else {
                    egui::Sense::hover()
                },
            );
            let response = response.on_hover_cursor(if available {
                egui::CursorIcon::PointingHand
            } else {
                egui::CursorIcon::Default
            });
            if index == current.ladder_index() {
                ui.painter().rect_filled(rect, 3.0 * scale, egui::Color32::from_rgb(218, 198, 162));
            } else {
                paint_purchase_background(
                    ui,
                    rect,
                    available,
                    response.hovered(),
                    response.is_pointer_button_down_on(),
                    3.0 * scale,
                    scale,
                );
            }
            let texture = super::rank_hud::rank_texture(ui.ctx(), &mut textures, index);
            ui.painter().image(
                texture.id(),
                egui::Rect::from_center_size(
                    egui::pos2(rect.center().x, rect.top() + 23.0 * scale),
                    egui::Vec2::splat(34.0 * scale),
                ),
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::from_white_alpha(if emphasized {
                    255
                } else {
                    217
                }),
            );
            ui.painter().text(
                egui::pos2(rect.center().x, rect.top() + 51.0 * scale),
                egui::Align2::CENTER_CENTER,
                rank.label(),
                egui::FontId::proportional(12.0 * scale),
                if emphasized {
                    INK
                } else {
                    super::campaign_widgets::UNAVAILABLE_PURCHASE_INK
                },
            );
            if available && response.clicked() {
                *message = Some(match senate.promote(player, players, config) {
                    Ok(_) => format!("Appointed {}.", rank.label()),
                    Err(e) => e.to_string(),
                });
            }
            if rank == PoliticalRank::Quaestor {
                continue;
            }
            response.on_hover_ui(|ui| {
                let previous = [
                    PoliticalRank::Quaestor,
                    PoliticalRank::Aedile,
                    PoliticalRank::Praetor,
                    PoliticalRank::Censor,
                    PoliticalRank::Consul,
                ][index - 1];
                let req = config.requirements(previous, players.len()).unwrap();
                let mut rows = vec![
                    (
                        index <= current.ladder_index() || next == Some(rank),
                        format!("Previous rank: {}", previous.label()),
                    ),
                    (
                        senate.support(player) >= req.senators,
                        format!("Supporting senators: {}/{}", senate.support(player), req.senators),
                    ),
                    (
                        players[player].influence >= req.influence,
                        format!(
                            "Influence cost: {}/{:.0}",
                            players[player].influence.round() as u64,
                            req.influence
                        ),
                    ),
                ];
                if next == Some(rank) {
                    if let Err(error) = &eligibility {
                        if !matches!(
                            error,
                            crate::game::politics::PoliticalError::InsufficientSupport
                                | crate::game::politics::PoliticalError::InsufficientFunds
                        ) {
                            rows.push((false, error.to_string()));
                        }
                    }
                }
                let bonuses = if rank == PoliticalRank::Augustus {
                    Vec::new()
                } else {
                    vec![format!("Influence: +{:.0} per month", config.rank_income(rank))]
                };
                super::campaign_widgets::rank_tooltip(ui, scale, &rows, &bonuses);
            });
        }
    });
    ui.ctx().data_mut(|d| d.insert_temp(key, textures));
}

fn faction_badges(
    ui: &mut egui::Ui,
    senate: &SenateState,
    config: &SenateConfig,
    player: usize,
    scale: f32,
) {
    ui.columns(5, |columns| {
        for (ui, bloc) in columns.iter_mut().zip(Bloc::ALL) {
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), 64.0 * scale),
                egui::Sense::hover(),
            );
            ui.painter().rect_filled(
                rect,
                4.0 * scale,
                INK.gamma_multiply(if response.hovered() {
                    0.10
                } else {
                    0.045
                }),
            );
            paint_faction_icon(
                ui.painter(),
                bloc,
                egui::Rect::from_center_size(
                    egui::pos2(rect.center().x, rect.top() + 17.0 * scale),
                    egui::Vec2::splat(24.0 * scale),
                ),
            );
            ui.painter().text(
                egui::pos2(rect.center().x, rect.top() + 36.0 * scale),
                egui::Align2::CENTER_CENTER,
                bloc.label(),
                egui::FontId::proportional(12.0 * scale),
                INK,
            );
            ui.painter().text(
                egui::pos2(rect.center().x, rect.top() + 52.0 * scale),
                egui::Align2::CENTER_CENTER,
                format!(
                    "{} / {}",
                    senate.bloc_support(player, bloc),
                    config.bloc_sizes[bloc.index()]
                ),
                egui::FontId::proportional(16.0 * scale),
                INK,
            );
            if response.hovered() {
                faction_tooltip(
                    &response,
                    bloc,
                    senate.bloc_support(player, bloc),
                    config.bloc_sizes[bloc.index()] as usize,
                    scale,
                );
            }
        }
    });
}

/// Five purpose-built vector emblems: temple, coin, aqueduct, wheat, and crossed swords.
fn paint_faction_icon(p: &egui::Painter, bloc: Bloc, rect: egui::Rect) {
    let at = |x: f32, y: f32| rect.min + egui::vec2(x * rect.width(), y * rect.height());
    let stroke = egui::Stroke::new(rect.width() * 0.065, INK);
    let line = |a, b| p.line_segment([a, b], stroke);
    match bloc {
        Bloc::Aristocrats => {
            p.add(egui::Shape::closed_line(
                vec![at(0.08, 0.32), at(0.5, 0.08), at(0.92, 0.32)],
                stroke,
            ));
            for x in [0.22, 0.5, 0.78] {
                line(at(x, 0.4), at(x, 0.82));
            }
            line(at(0.08, 0.9), at(0.92, 0.9));
            line(at(0.08, 0.38), at(0.92, 0.38));
        },
        Bloc::Merchants => {
            p.circle_stroke(rect.center(), rect.width() * 0.4, stroke);
            p.circle_stroke(
                rect.center(),
                rect.width() * 0.29,
                egui::Stroke::new(rect.width() * 0.035, INK),
            );
            line(at(0.35, 0.34), at(0.63, 0.34));
            line(at(0.35, 0.5), at(0.63, 0.5));
            line(at(0.35, 0.66), at(0.63, 0.66));
            line(at(0.4, 0.34), at(0.4, 0.66));
            line(at(0.59, 0.34), at(0.59, 0.66));
        },
        Bloc::Provincials => {
            line(at(0.08, 0.26), at(0.92, 0.26));
            line(at(0.08, 0.85), at(0.92, 0.85));
            for x in [0.12, 0.4, 0.68] {
                p.add(egui::Shape::line(
                    vec![
                        at(x, 0.83),
                        at(x, 0.47),
                        at(x + 0.12, 0.35),
                        at(x + 0.24, 0.47),
                        at(x + 0.24, 0.83),
                    ],
                    stroke,
                ));
            }
        },
        Bloc::Populares => {
            line(at(0.5, 0.9), at(0.5, 0.12));
            for y in [0.28, 0.46, 0.64] {
                line(at(0.5, y + 0.1), at(0.22, y - 0.08));
                line(at(0.5, y + 0.1), at(0.78, y - 0.08));
            }
        },
        Bloc::Military => {
            line(at(0.17, 0.85), at(0.83, 0.15));
            line(at(0.83, 0.85), at(0.17, 0.15));
            line(at(0.13, 0.64), at(0.36, 0.85));
            line(at(0.64, 0.85), at(0.87, 0.64));
            line(at(0.72, 0.13), at(0.86, 0.13));
            line(at(0.86, 0.13), at(0.86, 0.28));
            line(at(0.14, 0.13), at(0.28, 0.13));
            line(at(0.14, 0.13), at(0.14, 0.28));
        },
    }
}

fn player_color(colors: &[egui::Color32], player: usize) -> egui::Color32 {
    colors.get(player).copied().unwrap_or(egui::Color32::from_rgb(162, 115, 56))
}

/// Keep every player and the neutral count on one line, tightening gaps before shrinking text.
fn senate_legend(
    ui: &mut egui::Ui,
    senate: &SenateState,
    players: usize,
    colors: &[egui::Color32],
    scale: f32,
) {
    let mut entries = vec![(
        UNDECIDED,
        format!("Neutral ({})", senate.senators.iter().filter(|s| s.allegiance.is_none()).count()),
    )];
    entries.extend((0..players).map(|id| {
        (player_color(colors, id), format!("{} ({})", player_name(id), senate.support(id)))
    }));
    let available = ui.available_width().max(0.0);
    let count = entries.len() as f32;
    let gaps = count - 1.0;
    let mut dot_gap = 9.5 * scale;
    let mut entry_gap = 30.0 * scale;
    let layout = |factor: f32, dot_gap: f32, entry_gap: f32| {
        let galleys: Vec<_> = entries
            .iter()
            .map(|(_, label)| {
                ui.painter().layout_no_wrap(
                    label.clone(),
                    egui::FontId::proportional(14.0 * scale * factor),
                    INK,
                )
            })
            .collect();
        let width = galleys.iter().map(|g| g.rect.union(g.mesh_bounds).width()).sum::<f32>()
            + factor * (count * (11.0 * scale + dot_gap) + gaps * entry_gap);
        (galleys, width)
    };
    let (_, width) = layout(1.0, dot_gap, entry_gap);
    entry_gap = (entry_gap - (width - available).max(0.0) / gaps.max(1.0)).max(4.0 * scale);
    let (_, width) = layout(1.0, dot_gap, entry_gap);
    dot_gap = (dot_gap - (width - available).max(0.0) / count).max(2.0 * scale);
    let (mut galleys, width) = layout(1.0, dot_gap, entry_gap);
    let mut factor = 1.0;
    if width > available {
        let (mut low, mut high) = (0.0, 1.0);
        for _ in 0..12 {
            let candidate = (low + high) * 0.5;
            if layout(candidate, dot_gap, entry_gap).1 <= available {
                low = candidate;
            } else {
                high = candidate;
            }
        }
        factor = low;
        galleys = layout(factor, dot_gap, entry_gap).0;
    }
    let radius = 5.5 * scale * factor;
    let text_top = galleys.iter().map(|g| g.rect.union(g.mesh_bounds).top()).fold(0.0, f32::min);
    let text_bottom =
        galleys.iter().map(|g| g.rect.union(g.mesh_bounds).bottom()).fold(0.0, f32::max);
    let height = (text_bottom - text_top).max(16.0 * scale * factor);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(available, height), egui::Sense::hover());
    let mut x = rect.left();
    let text_y = rect.center().y - (text_top + text_bottom) * 0.5;
    for ((color, _), galley) in entries.iter().zip(galleys) {
        ui.painter().circle_filled(egui::pos2(x + radius, rect.center().y), radius, *color);
        x += radius * 2.0 + dot_gap * factor;
        let text_bounds = galley.rect.union(galley.mesh_bounds);
        ui.painter().galley(egui::pos2(x - text_bounds.left(), text_y), galley, INK);
        x += text_bounds.width() + entry_gap * factor;
    }
}

fn color_key(ui: &mut egui::Ui, color: egui::Color32, label: &str, scale: f32) {
    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        egui::FontId::proportional(14.0 * scale),
        INK,
    );
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(galley.size().x + 21.0 * scale, galley.size().y.max(16.0 * scale)),
        egui::Sense::hover(),
    );
    ui.painter().circle_filled(
        egui::pos2(rect.left() + 6.0 * scale, rect.center().y),
        5.5 * scale,
        color,
    );
    ui.painter().galley(
        egui::pos2(rect.left() + 21.0 * scale, rect.center().y - galley.size().y * 0.5),
        galley,
        INK,
    );
}

fn faction_tooltip(
    response: &egui::Response,
    bloc: Bloc,
    supporters: usize,
    members: usize,
    scale: f32,
) {
    let ctx = &response.ctx;
    let id = response.id.with("faction-tooltip");
    let tooltip = egui::Area::new(id)
        .order(egui::Order::Debug)
        .interactable(false)
        .pivot(egui::Align2::LEFT_BOTTOM)
        .fixed_pos(response.rect.left_top() - egui::vec2(0.0, 8.0 * scale))
        .constrain_to(ctx.content_rect().shrink(8.0 * scale))
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(PAPER)
                .stroke(egui::Stroke::new(scale, RULE))
                .corner_radius(9.0 * scale)
                .inner_margin(egui::Margin::same((12.0 * scale) as i8))
                .show(ui, |ui| {
                    ui.set_width(460.0 * scale);
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(
                            egui::Vec2::splat(56.0 * scale),
                            egui::Sense::hover(),
                        );
                        paint_faction_icon(ui.painter(), bloc, rect);
                        ui.vertical(|ui| {
                            ui.heading(bloc.label());
                            ui.label(format!("{supporters} / {members} senators support you"));
                        });
                    });
                    ui.add_space(5.0 * scale);
                    ui.separator();
                    ui.add_space(5.0 * scale);
                    show_effects(ui, bloc, scale);
                });
        });
    // The decorative standard uses Debug too. A sublayer keeps the badge card
    // above the flag regardless of the order in which the HUD systems paint.
    ctx.set_sublayer(
        egui::LayerId::new(egui::Order::Debug, egui::Id::new("augustus_map_standard_flag")),
        tooltip.response.layer_id,
    );
}

/// Explain what raises or lowers faction support in two effect lists.
fn show_effects(ui: &mut egui::Ui, bloc: Bloc, scale: f32) {
    let (positive, negative): (&[&str], &[&str]) = match bloc {
        Bloc::Aristocrats => (
            &[
                "Happy nobles",
                "Larger happy noble population",
                "Higher political rank",
                "Built forums",
                "Completed wonders",
            ],
            &[
                "Unhappy nobles",
                "Threatening senators",
                "Exposed bribery or corruption",
                "Exposed smears, murder or elite feuds",
            ],
        ),
        Bloc::Merchants => (
            &[
                "Fulfilled monthly trade routes",
                "Positive net monthly coin income",
                "Built markets",
            ],
            &[
                "Active wars",
                "Unfulfilled trade commitments",
                "Resource shortages",
                "Negative monthly coin income",
                "Exposed corruption, taxes or smuggling",
                "Exposed broken treaties or espionage",
            ],
        ),
        Bloc::Provincials => (
            &[
                "More vassal provinces",
                "Good relations with other provinces",
                "Delivered trade with provinces without cities",
            ],
            &[
                "Poor relations with other provinces",
                "High tribute",
                "Active wars",
                "Exposed famine, abuse or treaty violations",
                "Exposed hostile occupation or espionage",
            ],
        ),
        Bloc::Populares => (
            &[
                "Happy citizens and plebeians",
                "Ample food reserves",
                "Supplied generous food rations",
            ],
            &[
                "Unhappy citizens and plebeians",
                "Low rations, food shortages or famine",
                "High taxes or exposed smuggling",
                "Harsh slave labor",
                "Threatening senators or exposed smears",
                "Exposed abuse or hostile vassal rule",
            ],
        ),
        Bloc::Military => (
            &[
                "Effective army strength",
                "High military rank",
                "Recent victories",
                "Control beyond your first province",
            ],
            &["Recent defeats", "Exposed military incompetence or senator murder"],
        ),
    };
    let inner_width = (ui.available_width() - ui.spacing().item_spacing.x) * 0.5 - 16.0 * scale;
    let content_height = [positive, negative]
        .into_iter()
        .map(|rules| {
            ui.text_style_height(&egui::TextStyle::Body)
                + 4.0 * scale
                + rules
                    .iter()
                    .map(|rule| {
                        ui.painter()
                            .layout(
                                format!("• {rule}"),
                                egui::FontId::proportional(13.0 * scale),
                                INK,
                                inner_width,
                            )
                            .size()
                            .y
                            + ui.spacing().item_spacing.y
                    })
                    .sum::<f32>()
        })
        .fold(0.0_f32, f32::max);
    ui.columns(2, |columns| {
        for (column, (title, color, rules)) in columns.iter_mut().zip([
            ("Positive effects", POSITIVE, positive),
            ("Negative effects", NEGATIVE, negative),
        ]) {
            egui::Frame::new()
                .fill(color.gamma_multiply(0.06))
                .corner_radius(5.0 * scale)
                .inner_margin(egui::Margin::same((8.0 * scale) as i8))
                .show(column, |ui| {
                    ui.set_min_size(egui::vec2(inner_width, content_height));
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.colored_label(color, egui::RichText::new(title).strong());
                        ui.add_space(4.0 * scale);
                        for rule in rules {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(format!("• {rule}"))
                                        .size(13.0 * scale)
                                        .color(INK),
                                )
                                .wrap(),
                            );
                        }
                    });
                });
        }
    });
}

/// Each angular section contains only its own faction; larger circles show actual
/// persistent allegiances. Only individual seats are interactive in the chamber.
fn draw_chamber(
    ui: &mut egui::Ui,
    senate: &SenateState,
    config: &SenateConfig,
    player: usize,
    colors: &[egui::Color32],
) -> Option<(usize, egui::Response)> {
    let width = ui.available_width().min(590.0);
    let height = width * 0.56;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let center = egui::pos2(rect.center().x, rect.bottom() - 6.0);
    let outer = width * 0.47;
    let radius = (width / 75.0).clamp(3.0, 8.0);
    let mut start = 0.0;
    let selection_key = ui.id().with(("senator-selection", player));
    let mut selected = ui.ctx().data(|d| d.get_temp::<usize>(selection_key));
    let mut anchor = None;
    for bloc in Bloc::ALL {
        let arc = std::f32::consts::PI * config.bloc_sizes[bloc.index()] as f32 / 100.0;
        let mut points = vec![center];
        for step in 0..=12 {
            let a = start + arc * step as f32 / 12.0;
            points.push(
                center + egui::vec2(-a.cos() * (outer + radius), -a.sin() * (outer + radius)),
            );
        }
        ui.painter().add(egui::Shape::convex_polygon(
            points,
            INK.gamma_multiply(0.045),
            egui::Stroke::NONE,
        ));
        let divider =
            center + egui::vec2(-start.cos() * (outer + radius), -start.sin() * (outer + radius));
        ui.painter().line_segment(
            [center + (divider - center) * 0.32, divider],
            egui::Stroke::new(1.0, INK.gamma_multiply(0.25)),
        );
        let members: Vec<_> = senate.senators.iter().filter(|s| s.bloc == bloc).collect();
        for (i, senator) in members.iter().enumerate() {
            let row = i % 5;
            let column = i / 5;
            let count = members.len().saturating_sub(row).div_ceil(5).max(1);
            let angle = start + arc * (column as f32 + 0.5) / count as f32;
            let distance = outer * (0.44 + row as f32 * 0.13);
            let point = center + egui::vec2(-angle.cos() * distance, -angle.sin() * distance);
            let color = senator.allegiance.map_or(UNDECIDED, |id| player_color(colors, id));
            ui.painter().circle_filled(point, radius, color);
            ui.painter().circle_stroke(
                point,
                radius,
                egui::Stroke::new(0.8, INK.gamma_multiply(0.65)),
            );
            if senator.arrangement(player).is_some() || selected == Some(senator.id) {
                ui.painter().circle_stroke(
                    point,
                    radius + 1.8,
                    egui::Stroke::new(1.3, egui::Color32::from_rgb(184, 135, 52)),
                );
            }
            let seat_response = ui
                .interact(
                    egui::Rect::from_center_size(point, egui::Vec2::splat((radius + 2.0) * 2.0)),
                    ui.id().with(("senator-seat", senator.id)),
                    egui::Sense::click(),
                )
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if seat_response.hovered() || seat_response.clicked() {
                selected = Some(senator.id);
                ui.ctx().data_mut(|d| d.insert_temp(selection_key, senator.id));
            }
            if selected == Some(senator.id) {
                anchor = Some((senator.id, seat_response));
            }
        }
        let angle = start + arc * 0.5;
        // Keep the names tangent to the rim, with a little extra radial spacing
        // beyond the outermost seats.
        let label_point =
            center + egui::vec2(-angle.cos(), -angle.sin()) * (outer + radius + width * 0.01);
        let galley = ui.painter().layout_no_wrap(
            bloc.label().to_owned(),
            egui::FontId::proportional((width / 46.0).clamp(9.0, 14.0)),
            INK,
        );
        let label_origin = label_point - galley.rect.center().to_vec2();
        ui.painter().add(
            egui::epaint::TextShape::new(label_origin, galley, INK).with_angle_and_anchor(
                angle - std::f32::consts::FRAC_PI_2,
                egui::Align2::CENTER_CENTER,
            ),
        );
        start += arc;
    }
    let right_divider = center + egui::vec2(outer + radius, 0.0);
    ui.painter().line_segment(
        [center + (right_divider - center) * 0.32, right_divider],
        egui::Stroke::new(1.0, INK.gamma_multiply(0.25)),
    );
    ui.painter().text(
        center - egui::vec2(0.0, 12.0),
        egui::Align2::CENTER_BOTTOM,
        "SENATUS",
        egui::FontId::proportional((width / 36.0).clamp(10.0, 17.0)),
        INK,
    );
    anchor
}
