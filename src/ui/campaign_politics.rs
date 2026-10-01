//! Political career cards, faction approval badges and an interactive Senate chamber.
use super::campaign_widgets::{paint_purchase_background, Icon};
use super::province_panel::{PAPER, RULE};
use crate::game::politics::espionage::{EspionageConfig, EspionageState};
use crate::game::politics::senate::{
    Bloc, PoliticalProfile, SenateConfig, SenateState, SenatorAction, SupportReason,
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
    profiles: &[PoliticalProfile],
    config: &SenateConfig,
    player: usize,
    espionage: &mut EspionageState,
    espionage_config: &EspionageConfig,
    player_colors: &[egui::Color32],
) -> Option<String> {
    players.get(player)?;
    let scale = super::viewport_ui_scale(ui.ctx().content_rect().size());
    let mut message = None;
    super::policy_widgets::section(ui, scale, "POLITICAL RANK");
    rank_ladder(ui, senate, players, config, player, scale, &mut message);
    ui.add_space(10.0 * scale);
    super::policy_widgets::section(ui, scale, "SENATE");
    if let Some(winner) = senate.winner {
        ui.heading(format!("Player {} is Augustus · victory", winner + 1));
    }
    let profile = profiles.get(player).cloned().unwrap_or_default();
    faction_badges(ui, senate, players, &profile, config, player, scale);
    if let Some((id, anchor)) = draw_chamber(ui, senate, config, player, player_colors) {
        let selection_key = ui.id().with(("senator-selection", player));
        let mut open = true;
        egui::Popup::from_response(&anchor)
            .id(selection_key.with("actions"))
            .open_bool(&mut open)
            .close_behavior(if anchor.clicked() {
                egui::PopupCloseBehavior::IgnoreClicks
            } else {
                egui::PopupCloseBehavior::CloseOnClickOutside
            })
            .width(650.0 * scale)
            .frame(
                egui::Frame::popup(ui.style())
                    .fill(PAPER)
                    .stroke(egui::Stroke::new(scale, RULE))
                    .corner_radius(8.0 * scale)
                    .inner_margin(egui::Margin::same((12.0 * scale) as i8)),
            )
            .show(|ui| {
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
        if !open {
            ui.ctx().data_mut(|d| d.remove::<usize>(selection_key));
        }
    }
    ui.add_space(10.0 * scale);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 30.0 * scale;
        color_key(
            ui,
            UNDECIDED,
            &format!(
                "Neutral ({})",
                senate.senators.iter().filter(|s| s.allegiance.is_none()).count()
            ),
            scale,
        );
        for id in 0..players.len() {
            color_key(
                ui,
                player_color(player_colors, id),
                &format!("{} ({})", player_name(id), senate.support(id)),
                scale,
            );
        }
    });
    message
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
    let arrangement = seat.arrangement;
    if let Some(a) = arrangement {
        if a.action == SenatorAction::Lobby {
            ui.small(format!(
                "Lobbying for {} · {:.0} Influence/month",
                player_name(a.player),
                config.senator_outreach_upkeep
            ));
        } else {
            ui.small(format!(
                "{} · {} · {} months remaining",
                a.action.label(),
                player_name(a.player),
                a.until.saturating_sub(senate.month)
            ));
        }
    }
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
        let quote = senate.senator_action_quote(player, id, action, players, config);
        let currency = if action.currency() == Currency::Coin {
            Icon::Coin
        } else {
            Icon::Influence
        };
        let mut costs = vec![(currency, format!("{:.0}", config.action_costs[action as usize]))];
        if action == SenatorAction::Lobby {
            costs.push((Icon::Influence, format!("{:.0}/mo", config.senator_outreach_upkeep)));
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
        let response = super::campaign_panel::diplomacy_action_card(
            ui,
            rect,
            ui.id().with(("senator-action", id, action as usize)),
            title,
            icon,
            &costs,
            quote.is_ok(),
            scale,
        );
        let clicked = response.clicked();
        response.on_hover_ui(|ui| {
            ui.strong(action.label());
            ui.label(action.description());
            let unit = if action.currency() == Currency::Coin {
                "sestertii"
            } else {
                "Influence"
            };
            ui.small(format!("Upfront: {:.0} {unit}", config.action_costs[action as usize]));
            if action == SenatorAction::Lobby {
                ui.small(format!("Monthly: {:.0} Influence", config.senator_outreach_upkeep));
            }
            let risk = config.action_risks[action as usize];
            if risk > 0.0 {
                ui.colored_label(NEGATIVE, format!("Chance of exposure: {:.0}%", risk * 100.0));
            }
            if let Err(error) = &quote {
                ui.add_space(5.0 * scale);
                super::campaign_widgets::unavailable_reason(ui, &error.to_string(), scale);
            }
        });
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
                            "{} exposed! Your faction confidence drops immediately; {}.",
                            action.label(),
                            outcome.misconduct.unwrap().1.label()
                        )
                    } else if action == SenatorAction::Assassinate {
                        format!("Senator {} was replaced. This seat is neutral; every player's confidence was removed.", id + 1)
                    } else {
                        format!("{}: Senator {}. {}", action.label(), id + 1, action.description())
                    }
                },
                Err(error) => error.to_string(),
            });
        }
    }
    if arrangement.is_some_and(|a| a.player == player && a.action == SenatorAction::Lobby) {
        ui.add_space(gap);
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), 72.0 * scale),
            egui::Sense::hover(),
        );
        if super::campaign_panel::diplomacy_action_card(
            ui,
            rect,
            ui.id().with(("end-lobbying", id)),
            "End senator lobbying",
            Icon::Cancel,
            &[],
            true,
            scale,
        )
        .clicked()
        {
            *message = Some(match senate.end_outreach(player, id) {
                Ok(()) => "Senator lobbying ended. Monthly payments stop.".to_owned(),
                Err(error) => error.to_string(),
            });
        }
    }
}

fn action_presentation(action: SenatorAction) -> (&'static str, Icon) {
    match action {
        SenatorAction::Petition => ("Petition", Icon::Diplomacy),
        SenatorAction::Gift => ("Send a gift", Icon::Happiness),
        SenatorAction::Patronage => ("Patronage", Icon::Nobles),
        SenatorAction::Bribe => ("Bribe", Icon::BribeNobles),
        SenatorAction::Threaten => ("Threaten", Icon::SpyUndermineOpponents),
        SenatorAction::Assassinate => ("Murder", Icon::Attack),
        SenatorAction::Banquet => ("Public banquet", Icon::Food),
        SenatorAction::Discredit => ("Discredit patron", Icon::SpyDiscreditRivals),
        SenatorAction::Lobby => ("Lobby senator", Icon::SpyImproveRelations),
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
                egui::vec2(ui.available_width(), 78.0 * scale),
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
                    egui::pos2(rect.center().x, rect.top() + 27.0 * scale),
                    egui::Vec2::splat(38.0 * scale),
                ),
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::from_white_alpha(if emphasized {
                    255
                } else {
                    217
                }),
            );
            ui.painter().text(
                egui::pos2(rect.center().x, rect.top() + 57.0 * scale),
                egui::Align2::CENTER_CENTER,
                if rank == PoliticalRank::Consul && current == PoliticalRank::Proconsul {
                    "Proconsul"
                } else {
                    rank.label()
                },
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
            response.on_hover_ui(|ui| {
                let mut rows = Vec::new();
                if rank != PoliticalRank::Quaestor {
                    let previous = [
                        PoliticalRank::Quaestor,
                        PoliticalRank::Aedile,
                        PoliticalRank::Praetor,
                        PoliticalRank::Censor,
                        PoliticalRank::Consul,
                    ][index - 1];
                    let req = config.requirements(previous, players.len()).unwrap();
                    rows.extend([
                        (
                            index <= current.ladder_index() || next == Some(rank),
                            format!("Previous rank: {}", previous.label()),
                        ),
                        (
                            senate.support(player) >= req.senators,
                            format!(
                                "Approving senators: {}/{}",
                                senate.support(player),
                                req.senators
                            ),
                        ),
                        (
                            players[player].influence >= req.influence,
                            format!(
                                "Influence to pay: {}/{:.0}",
                                players[player].influence.round() as u64,
                                req.influence
                            ),
                        ),
                    ]);
                }
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
                super::campaign_widgets::rank_tooltip(
                    ui,
                    scale,
                    &rows,
                    &[format!("Influence: +{:.0} per month", config.rank_income(rank))],
                );
            });
        }
    });
    ui.ctx().data_mut(|d| d.insert_temp(key, textures));
}

fn faction_badges(
    ui: &mut egui::Ui,
    senate: &SenateState,
    players: &[PoliticalPlayer],
    profile: &PoliticalProfile,
    config: &SenateConfig,
    player: usize,
    scale: f32,
) {
    ui.columns(5, |columns| {
        for (ui, bloc) in columns.iter_mut().zip(Bloc::ALL) {
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), 76.0 * scale),
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
                    egui::pos2(rect.center().x, rect.top() + 20.0 * scale),
                    egui::Vec2::splat(28.0 * scale),
                ),
            );
            ui.painter().text(
                egui::pos2(rect.center().x, rect.top() + 43.0 * scale),
                egui::Align2::CENTER_CENTER,
                bloc.label(),
                egui::FontId::proportional(12.0 * scale),
                INK,
            );
            ui.painter().text(
                egui::pos2(rect.center().x, rect.top() + 61.0 * scale),
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
                    &senate.reasons(player, bloc, &players[player], profile, config),
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
    reasons: &[SupportReason],
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
                            egui::Vec2::splat(38.0 * scale),
                            egui::Sense::hover(),
                        );
                        paint_faction_icon(ui.painter(), bloc, rect.shrink(3.0 * scale));
                        ui.vertical(|ui| {
                            ui.heading(bloc.label());
                            ui.label(format!("{supporters} / {members} senators approve"));
                        });
                    });
                    ui.add_space(5.0 * scale);
                    ui.separator();
                    ui.add_space(5.0 * scale);
                    show_reasons(ui, bloc, reasons, scale);
                });
        });
    // The decorative standard uses Debug too. A sublayer keeps the badge card
    // above the flag regardless of the order in which the HUD systems paint.
    ctx.set_sublayer(
        egui::LayerId::new(egui::Order::Debug, egui::Id::new("augustus_map_standard_flag")),
        tooltip.response.layer_id,
    );
}

/// Rules are always visible; live recurring effects are marked active without revealing points.
fn show_reasons(ui: &mut egui::Ui, bloc: Bloc, reasons: &[SupportReason], scale: f32) {
    let (positive, negative): (&[&str], &[&str]) = match bloc {
        Bloc::Aristocrats => (
            &[
                "Happy nobles",
                "Noble population",
                "Political office",
                "Forums and completed wonders",
            ],
            &["Unhappy nobles", "Coercion of senators", "Exposed corruption, smears or murder"],
        ),
        Bloc::Merchants => (
            &["Active fulfilled trade routes", "Profitable income", "Urban Markets"],
            &[
                "Active wars",
                "Unfulfilled trade commitments",
                "Resource shortages",
                "Loss-making economy",
                "Broken routes or exposed corruption",
            ],
        ),
        Bloc::Provincials => (
            &["Happy free populations", "Friendly, stable vassals", "Provincial trade"],
            &[
                "Unhappy populations or hostile vassals",
                "High tribute",
                "Active wars",
                "Exposed famine, abuse or treaty violations",
            ],
        ),
        Bloc::Populares => (
            &[
                "Happy citizens and plebeians",
                "Generous food policy",
                "Reliable food supply",
                "Public banquets",
            ],
            &[
                "Unhappy citizens and plebeians",
                "Restricted food supply or famine",
                "High taxes",
                "Harsh slave labor",
                "Coercion or exposed abuse",
            ],
        ),
        Bloc::Military => (
            &[
                "Effective army strength",
                "High military rank",
                "Recent victories",
                "Controlled provinces",
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
    let active: Vec<_> = reasons.iter().filter(|r| r.points.abs() > 1e-9).collect();
    if !active.is_empty() {
        ui.separator();
        ui.strong("Active monthly effects");
        for reason in active {
            ui.colored_label(
                if reason.points > 0.0 {
                    POSITIVE
                } else {
                    NEGATIVE
                },
                format!(
                    "{} {}",
                    if reason.points > 0.0 {
                        "+"
                    } else {
                        "−"
                    },
                    reason.label
                ),
            );
        }
    }
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
            if senator.arrangement.is_some() || selected == Some(senator.id) {
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
            if seat_response.clicked() {
                selected = Some(senator.id);
                ui.ctx().data_mut(|d| d.insert_temp(selection_key, senator.id));
            }
            if selected == Some(senator.id) {
                anchor = Some((senator.id, seat_response));
            }
        }
        let angle = start + arc * 0.5;
        // The outermost seats end just inside `outer`; the names follow the rim
        // with their baselines tangent to it, leaving the senators unobscured.
        let label_point = center + egui::vec2(-angle.cos(), -angle.sin()) * (outer + radius);
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
