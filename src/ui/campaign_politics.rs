//! Parchment-styled Senate controls and a precisely one-hundred-seat chamber.

use super::campaign_widgets::{icon, stat, Icon};
use crate::game::politics::diplomacy::{PoliticalState, ProvincePolitics};
use crate::game::politics::espionage::{
    EspionageConfig, EspionageState, ScandalKind, ScandalTarget, Severity,
};
use crate::game::politics::senate::{
    Ballot, Bloc, BlocProjection, PoliticalProfile, SenateConfig, SenateState,
};
use crate::game::politics::{PoliticalPlayer, PoliticalRank};
use bevy_egui::egui;

const INK: egui::Color32 = egui::Color32::from_rgb(57, 43, 37);
const UNDECIDED: egui::Color32 = egui::Color32::from_rgb(145, 143, 134);
const OPPOSITION: egui::Color32 = egui::Color32::from_rgb(120, 53, 48);

/// Show only the local player's evidence about the selected NPC government.
/// Both uses consume evidence; one-time trade itself never grants political control.
pub(in crate::app) fn npc_evidence(
    ui: &mut egui::Ui,
    espionage: &mut EspionageState,
    politics: &mut [ProvincePolitics],
    player: usize,
    province: usize,
    month: u32,
    config: &EspionageConfig,
) -> Option<String> {
    let state = politics.get(province)?.state.clone();
    if matches!(state, PoliticalState::Owned { .. }) {
        return None;
    }
    let mut message = None;
    let evidence: Vec<_> = espionage
        .scandals
        .iter()
        .filter(|s| {
            s.holder == player
                && s.target == ScandalTarget::Province(province)
                && s.expires > month
                && !s.reserved_for_motion
        })
        .cloned()
        .collect();
    if evidence.is_empty() {
        return None;
    }
    ui.collapsing(format!("Evidence · {}", evidence.len()), |ui| {
    for scandal in evidence {
        ui.horizontal_wrapped(|ui| {
            icon(ui, Icon::Spy, 20.0);
            ui.strong(scandal.kind.label()).on_hover_text(format!("{:?} evidence · expires in {} months. Either blackmail action consumes this evidence.", scandal.severity, scandal.expires.saturating_sub(month)));
        });
        ui.horizontal_wrapped(|ui| {
            icon(ui, Icon::Control, 20.0);
            if ui.add_enabled(matches!(state, PoliticalState::Independent { .. }), egui::Button::new(format!("+{:.1}", scandal.severity.control_gain())))
                .on_hover_text("Consume the evidence. Adds political pressure to the next simultaneous monthly resolution and lowers relation by 5. Existing rival pressure may oppose the gain.").clicked() {
                message = Some(match espionage.blackmail_control(player, scandal.id, month, politics) { Ok(()) => "Scandal consumed. Blackmail pressure will resolve next month; relation decreased by 5.".into(), Err(error) => error.to_string() });
            }
            icon(ui, Icon::Trade, 20.0);
            if ui.button("Trade terms").on_hover_text(format!("Consume this evidence. For {} months, this NPC requires {:.0}% of its usual offered value from you. Other players receive no benefit.", config.favorable_trade_months, config.favorable_trade_ratio * 100.0)).clicked() {
                message = Some(match espionage.blackmail_trade(player, scandal.id, month, config) { Ok(()) => format!("Favorable trade terms secured for {} months.", config.favorable_trade_months), Err(error) => error.to_string() });
            }
        });
    }
    });
    message
}

/// Render the political panel inside the caller's existing styled scroll frame.
/// Returned text is a confirmed result or a specific failure suitable for a toast.
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
    let actor = players.get(player)?;
    let mut message = None;
    ui.horizontal_wrapped(|ui| {
        icon(ui, Icon::Nobles, 24.0);
        ui.strong(actor.rank.label()).on_hover_text("Quaestor → Aedile → Praetor → Censor → Consul → Augustus. A former Consul becomes Proconsul and must win reelection before seeking Augustus.");
        stat(ui, Icon::Influence, &format!("+{:.0}/mo", config.rank_income(actor.rank)), "Monthly Influence supplied by your current office.");
        if let Some(until) = actor.consul_until {
            ui.small(format!("{} mo", until.saturating_sub(senate.month))).on_hover_text("Time remaining in your 48-month Consul term.");
        }
    });
    let consuls: Vec<_> = players
        .iter()
        .enumerate()
        .filter(|(_, p)| p.rank == PoliticalRank::Consul)
        .map(|(id, _)| format!("Player {}", id + 1))
        .collect();
    ui.horizontal_wrapped(|ui| {
        stat(ui, Icon::Coin, &format!("{:.0}", actor.coin), "Available Coin.");
        stat(
            ui,
            Icon::Influence,
            &format!("{:.0}", actor.influence),
            "Available Influence. Sealed additional bids have already been debited.",
        );
        stat(
            ui,
            Icon::Nobles,
            &format!("{}/2", consuls.len()),
            &format!(
                "Occupied Consul seats: {}",
                if consuls.is_empty() {
                    "none".to_owned()
                } else {
                    consuls.join(", ")
                }
            ),
        );
    });
    if players[player].rank == PoliticalRank::Quaestor {
        let enabled = players[player].influence >= config.aedile_cost;
        if ui.add_enabled(enabled, egui::Button::new("Become Aedile"))
            .on_hover_text(format!("Costs {:.0} Influence. Aedile is the one purchased office. Every later promotion, including Censor, requires a successful Senate vote.", config.aedile_cost)).on_disabled_hover_text(format!("Requires {:.0} Influence.", config.aedile_cost)).clicked() {
            message = Some(match senate.buy_aedile(&mut players[player], config) { Ok(()) => "Promoted to Aedile.".into(), Err(error) => error.to_string() });
        }
    }
    ui.separator();
    if let Some(winner) = senate.winner {
        ui.heading(format!("Player {} has been elected Augustus", winner + 1));
    } else if senate.campaign.is_none() {
        show_nomination(ui, senate, players, config, player, espionage, &mut message);
    } else {
        show_campaign(
            ui,
            senate,
            players,
            profiles,
            config,
            player,
            espionage,
            espionage_config,
            player_colors,
            &mut message,
        );
    }
    if senate.campaign.is_none() {
        let preview = senate.preview_projection(
            player,
            senate.chosen_ballot(player).unwrap_or_else(|| next_ballot(players[player].rank)),
            players,
            profiles,
            config,
        );
        if senate.last_result.is_none() {
            draw_chamber(ui, &[(UNDECIDED, "Your nomination outlook"); 100], &preview, config);
        }
        if let Some(result) = &senate.last_result {
            ui.separator();
            ui.strong(format!("LAST VOTE · {}", result.ballot.label()));
            let yes_color = player_color(player_colors, result.candidate);
            let no_color = match result.ballot {
                Ballot::NoConfidence {
                    target,
                    ..
                } => player_color(player_colors, target),
                _ => OPPOSITION,
            };
            let animation_id = ui.id().with(("senate_reveal", result.month));
            let now = ui.input(|i| i.time);
            let started =
                ui.ctx().data_mut(|data| *data.get_temp_mut_or_insert_with(animation_id, || now));
            let revealed = ((now - started) / 0.04).max(0.0) as usize;
            let mut undecided_index = 0;
            let seats: Vec<_> = result
                .votes
                .iter()
                .zip(&result.was_undecided)
                .map(|(yes, was_undecided)| {
                    let show = !*was_undecided || {
                        undecided_index += 1;
                        undecided_index <= revealed
                    };
                    (
                        if show {
                            if *yes {
                                yes_color
                            } else {
                                no_color
                            }
                        } else {
                            UNDECIDED
                        },
                        if !show {
                            "Undecided"
                        } else if *yes {
                            "YES"
                        } else {
                            "NO"
                        },
                    )
                })
                .collect();
            let shown_yes = seats.iter().filter(|(_, label)| *label == "YES").count();
            let shown_no = seats.iter().filter(|(_, label)| *label == "NO").count();
            ui.label(format!(
                "YES {shown_yes} · NO {shown_no} · UNDECIDED {}",
                100 - shown_yes - shown_no
            ));
            if revealed >= undecided_index {
                ui.label(if result.passed {
                    "Motion passed."
                } else {
                    "Motion failed. A new Nomination Year is open."
                });
            }
            draw_chamber(ui, &seats, &[], config);
            if revealed < undecided_index {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(40));
            }
        }
        ui.collapsing("Your bloc outlook", |ui| {
            for bloc in &preview {
                let (symbol, label) = bloc_identity(bloc.bloc);
                ui.horizontal(|ui| {
                    icon(ui, symbol, 20.0);
                    ui.label(label);
                    ui.small(format!("{:.0}%", bloc.support_probability * 100.0));
                }).response.on_hover_ui(|ui| {
                    ui.small("Structural preview before campaigning; no votes are committed by this forecast.");
                    show_reasons(ui, bloc);
                });
            }
        });
    }
    ui.separator();
    ui.collapsing("Evidence", |ui| {
        let evidence: Vec<_> = espionage.scandals.iter().filter(|s| s.holder == player && (s.expires > senate.month || s.reserved_for_motion)).collect();
        if evidence.is_empty() { ui.small("No evidence").on_hover_text("Spy networks uncover evidence from actual foreign policies and actions."); }
        for scandal in evidence {
            let target = match scandal.target { ScandalTarget::Player(id) => format!("Player {}", id + 1), ScandalTarget::Province(id) => format!("Province {}", id + 1) };
            ui.horizontal_wrapped(|ui| {
                icon(ui, Icon::Spy, 18.0);
                ui.label(format!("{} · {}", target, scandal.kind.label()))
                    .on_hover_text(format!("{:?} · {}. Evidence remains after its underlying condition ends. Use it for an accusation, removal motion or NPC blackmail.", scandal.severity, if scandal.reserved_for_motion { "reserved for removal motion".into() } else { format!("expires in {} months", scandal.expires.saturating_sub(senate.month)) }));
            });
        }
    });
    message
}

/// Present resolved public bids separately from the viewing player's private addition.
fn show_nomination(
    ui: &mut egui::Ui,
    senate: &mut SenateState,
    players: &mut [PoliticalPlayer],
    config: &SenateConfig,
    player: usize,
    espionage: &EspionageState,
    message: &mut Option<String>,
) {
    ui.horizontal_wrapped(|ui| {
        ui.strong(if senate.sudden_death.is_empty() { "Nomination" } else { "Sudden death" })
            .on_hover_text("All bids are spent. Additional bids stay sealed until month end. Tied leaders alone enter repeated sealed rounds until the tie breaks.");
        ui.small(format!("{} mo", if senate.sudden_death.is_empty() { config.nomination_months.saturating_sub(senate.nomination_elapsed) } else { 1 }));
    });
    if senate.nominations.is_empty() {
        ui.small("No bids revealed");
    }
    for nomination in senate.nominations.values() {
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("P{} · {}", nomination.player + 1, nomination.ballot.label()));
            stat(
                ui,
                Icon::Influence,
                &format!("{:.0}", nomination.committed),
                "Publicly revealed commitment. All bids remain spent, including losing bids.",
            );
        });
    }
    let pending = senate.private_bid(player);
    if pending > 0.0 {
        ui.horizontal_wrapped(|ui| {
            icon(ui, Icon::Spy, 20.0);
            ui.small(format!("Sealed +{pending:.0}"))
                .on_hover_text("Only you can see this additional Influence bid until month end.");
        });
    }
    let next = next_ballot(players[player].rank);
    let mut options = vec![(next, false)];
    options.extend(
        espionage
            .scandals
            .iter()
            .filter(|s| s.holder == player && s.expires > senate.month && !s.reserved_for_motion)
            .filter_map(|s| match s.target {
                ScandalTarget::Player(target)
                    if players.get(target).is_some_and(|p| p.rank == PoliticalRank::Consul) =>
                {
                    Some((
                        Ballot::NoConfidence {
                            target,
                            scandal_id: s.id,
                        },
                        true,
                    ))
                },
                _ => None,
            }),
    );
    if let Some(chosen) = senate.chosen_ballot(player) {
        let evidence = match chosen {
            Ballot::NoConfidence {
                target,
                scandal_id,
            } => espionage.holds_evidence(player, scandal_id, target, senate.month),
            _ => false,
        };
        // Further bids belong to the already selected motion, including before its first reveal.
        options = vec![(chosen, evidence)];
    }
    for (ballot, evidence) in options {
        let committed = senate.nominations.get(&player).map_or(0.0, |n| n.committed);
        let minimum = if committed + pending > 0.0 {
            1.0
        } else {
            config.minimum_bid(ballot)
        };
        let id = ui.id().with(("senate_bid", player, format!("{ballot:?}")));
        let mut amount =
            ui.ctx().data_mut(|data| *data.get_temp_mut_or_insert_with(id, || minimum));
        ui.horizontal_wrapped(|ui| {
            let label = match ballot { Ballot::NoConfidence { target, .. } => format!("Remove Player {}", target + 1), _ => ballot.label().to_owned() };
            ui.label(label);
            icon(ui, Icon::Influence, 20.0);
            ui.add(egui::DragValue::new(&mut amount).range(minimum..=1_000_000.0).speed(5.0)).on_hover_text("Additional Influence to commit permanently to this nomination.");
            let eligibility = senate.bid_eligibility(player, ballot, amount, players, evidence, config);
            let available = eligibility.is_ok();
            let response = ui.add_enabled(available, egui::Button::new("Seal bid"));
            let explanation = if !senate.sudden_death.is_empty() && !senate.sudden_death.contains(&player) { "Only the tied leaders may add bids during sudden death.".into() } else { eligibility.as_ref().err().map_or_else(|| format!("Minimum entry: {:.0} Influence. Escrow is permanent; all players' bids reveal together at month end.", config.minimum_bid(ballot)), ToString::to_string) };
            if response.on_hover_text(&explanation).on_disabled_hover_text(explanation).clicked() {
                *message = Some(match senate.submit_bid(player, ballot, amount, players, evidence, config) { Ok(()) => format!("Sealed {amount:.0} additional Influence for {}.", ballot.label()), Err(error) => error.to_string() });
            }
        });
        ui.ctx().data_mut(|data| data.insert_temp(id, amount));
    }
}

/// Display bloc commitments, uncertainty and multiplayer support/opposition controls.
fn show_campaign(
    ui: &mut egui::Ui,
    senate: &mut SenateState,
    players: &mut [PoliticalPlayer],
    profiles: &[PoliticalProfile],
    config: &SenateConfig,
    player: usize,
    espionage: &mut EspionageState,
    espionage_config: &EspionageConfig,
    colors: &[egui::Color32],
    message: &mut Option<String>,
) {
    let campaign =
        senate.campaign.as_ref().expect("campaign panel requires active campaign").clone();
    ui.horizontal_wrapped(|ui| {
        ui.strong(format!("{} · P{}", campaign.ballot.label(), campaign.candidate + 1));
        ui.small(format!("{} mo", config.campaign_months.saturating_sub(campaign.elapsed)))
            .on_hover_text(
            "Months until the authoritative Senate vote. A motion requires at least 51 YES votes.",
        );
    });
    let projection = senate.projection(players, profiles, config);
    let yes: u16 = projection.iter().map(|p| u16::from(p.yes)).sum();
    let no: u16 = projection.iter().map(|p| u16::from(p.no)).sum();
    let undecided: u16 = projection.iter().map(|p| u16::from(p.undecided)).sum();
    let expected: f64 = projection
        .iter()
        .map(|p| f64::from(p.yes) + f64::from(p.undecided) * p.support_probability)
        .sum();
    ui.label(format!("{yes} YES · {no} NO · {undecided} undecided"))
        .on_hover_text(format!("Expected final YES: approximately {expected:.0}. Committed votes are fixed; only undecided seats roll at the final vote."));
    let yes_color = player_color(colors, campaign.candidate);
    let no_color = match campaign.ballot {
        Ballot::NoConfidence {
            target,
            ..
        } => player_color(colors, target),
        _ => OPPOSITION,
    };
    let mut seats = Vec::with_capacity(100);
    for bloc in &projection {
        seats.extend(std::iter::repeat_n((yes_color, "YES"), bloc.yes as usize));
        seats.extend(std::iter::repeat_n((no_color, "NO"), bloc.no as usize));
        seats.extend(std::iter::repeat_n((UNDECIDED, "Undecided"), bloc.undecided as usize));
    }
    draw_chamber(ui, &seats, &projection, config);
    ui.horizontal_wrapped(|ui| {
        ui.colored_label(
            yes_color,
            if matches!(campaign.ballot, Ballot::NoConfidence { .. }) {
                "● Remove Consul"
            } else {
                "● YES"
            },
        );
        ui.colored_label(
            no_color,
            if matches!(campaign.ballot, Ballot::NoConfidence { .. }) {
                "● Keep Consul"
            } else {
                "● NO"
            },
        );
        ui.colored_label(UNDECIDED, "● Undecided");
    });
    egui::Grid::new("senate_bloc_actions").spacing(egui::vec2(7.0, 6.0)).striped(true).show(ui, |ui| {
        for bloc in &projection {
            let (symbol, label) = bloc_identity(bloc.bloc);
            ui.horizontal(|ui| { icon(ui, symbol, 20.0); ui.label(label); })
                .response.on_hover_ui(|ui| show_reasons(ui, bloc));
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                ui.colored_label(yes_color, bloc.yes.to_string());
                ui.small("/");
                ui.colored_label(no_color, bloc.no.to_string());
                ui.small("/");
                ui.colored_label(UNDECIDED, bloc.undecided.to_string());
            }).response.on_hover_text("Committed YES / committed NO / undecided. Hover the bloc name for every influence on support.");
            let influence_cost = config.campaign_action_influence;
            let coin_cost = config.bribery_action_coin;
            let support_tip = format!("{} · {influence_cost:.0} Influence. Raises support in this bloc with diminishing returns.", if player == campaign.candidate { "Campaign" } else { "Endorse" });
            if ui.add_enabled(players[player].influence >= influence_cost, egui::Button::new("+"))
                .on_hover_text(&support_tip).on_disabled_hover_text(support_tip).clicked() {
                *message = Some(match senate.campaign_spend(player, bloc.bloc, true, influence_cost, players) { Ok(()) => format!("Supported the motion among {}.", bloc.bloc.label()), Err(error) => error.to_string() });
            }
            let may_oppose = player != campaign.candidate || matches!(campaign.ballot, Ballot::NoConfidence { .. });
            let oppose_tip = if may_oppose { format!("Oppose · {influence_cost:.0} Influence. Reduces support in this bloc.") } else { "You cannot oppose your own promotion.".into() };
            if ui.add_enabled(may_oppose && players[player].influence >= influence_cost, egui::Button::new("−"))
                .on_hover_text(&oppose_tip).on_disabled_hover_text(oppose_tip).clicked() {
                *message = Some(match senate.campaign_spend(player, bloc.bloc, false, influence_cost, players) { Ok(()) => format!("Opposed the motion among {}.", bloc.bloc.label()), Err(error) => error.to_string() });
            }
            let bribe_tip = format!("Bribe · {coin_cost:.0} Coin. Improves support and creates real Political Bribery evidence that enemy spies may uncover. Only the candidate may bribe.");
            if ui.add_enabled(player == campaign.candidate && players[player].coin >= coin_cost, egui::Button::new("Bribe"))
                .on_hover_text(&bribe_tip).on_disabled_hover_text(bribe_tip).clicked() {
                *message = Some(match senate.bribe(player, bloc.bloc, coin_cost, players) {
                    Ok(()) => { espionage.record_action(player, None, ScandalKind::PoliticalBribery, Severity::Medium, senate.month, espionage_config); "Bribery improved support and created a discoverable scandal.".into() },
                    Err(error) => error.to_string(),
                });
            }
            ui.end_row();
        }
    });
    let evidence_target = match campaign.ballot {
        Ballot::NoConfidence {
            target,
            ..
        } => target,
        _ => campaign.candidate,
    };
    let scandals: Vec<_> = espionage.scandals.iter().filter(|s| s.holder == player && s.target == ScandalTarget::Player(evidence_target) && s.expires > senate.month && !s.reserved_for_motion && !matches!(campaign.ballot, Ballot::NoConfidence { scandal_id, .. } if scandal_id == s.id)).cloned().collect();
    for scandal in scandals {
        if ui.button(format!("Expose {}", scandal.kind.label())).on_hover_text("Consumes this evidence. Each scandal affects the relevant blocs differently; severity scales its effect.").clicked() {
            let penalties = scandal.kind.bloc_penalties(scandal.severity);
            *message = Some(match senate.expose_scandal(evidence_target, penalties) {
                Ok(()) => match espionage.consume(player, scandal.id, senate.month) { Ok(_) => format!("Exposed {}.", scandal.kind.label()), Err(error) => error.to_string() },
                Err(error) => error.to_string(),
            });
        }
    }
}

/// Use the user's actual assigned player color rather than inventing a faction palette.
fn player_color(colors: &[egui::Color32], player: usize) -> egui::Color32 {
    colors.get(player).copied().unwrap_or(egui::Color32::from_rgb(162, 115, 56))
}

/// Preserve the full rank ladder while showing only the player's next promotion.
fn next_ballot(rank: PoliticalRank) -> Ballot {
    match rank {
        PoliticalRank::Quaestor | PoliticalRank::Aedile => Ballot::Praetor,
        PoliticalRank::Praetor => Ballot::Censor,
        PoliticalRank::Censor | PoliticalRank::Proconsul => Ballot::Consul,
        PoliticalRank::Consul | PoliticalRank::Augustus => Ballot::Augustus,
    }
}

/// Compact bloc names retain their full constitutional identity in the reason tooltip.
fn bloc_identity(bloc: Bloc) -> (Icon, &'static str) {
    match bloc {
        Bloc::Aristocrats => (Icon::Nobles, "Nobles"),
        Bloc::Merchants => (Icon::Trade, "Trade"),
        Bloc::Provincials => (Icon::Province, "Provinces"),
        Bloc::Populares => (Icon::Plebeians, "Plebs"),
        Bloc::Military => (Icon::MilitaryPower, "Military"),
    }
}

/// Explain every structural and political contributor in a hover tooltip.
fn show_reasons(ui: &mut egui::Ui, bloc: &BlocProjection) {
    ui.strong(bloc.bloc.label());
    ui.label(format!("Undecided YES probability: {:.0}%", bloc.support_probability * 100.0));
    ui.label(format!("Baseline support: {:.0}%", bloc.baseline_support * 100.0));
    for reason in &bloc.reasons {
        ui.label(format!("{}: {:+.1} percentage points", reason.label, reason.points));
    }
    ui.label("Support converts some seats to committed votes. Only undecided seats are rolled at the final vote.");
}

/// Stable five-row semicircle with precisely 100 circles and bloc-aware hit targets.
fn draw_chamber(
    ui: &mut egui::Ui,
    seats: &[(egui::Color32, &'static str)],
    projection: &[BlocProjection],
    config: &SenateConfig,
) {
    let width = ui.available_width().clamp(220.0, 680.0);
    let height = width * 0.48;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let center = egui::pos2(rect.center().x, rect.bottom() - 8.0);
    let outer = width * 0.47;
    let radius = (width / 110.0).clamp(2.0, 5.0);
    let mut seat_index = 0;
    for (row, count) in [14, 18, 20, 23, 25].into_iter().enumerate() {
        let distance = outer * (0.46 + row as f32 * 0.135);
        for position in 0..count {
            let angle = std::f32::consts::PI * (position as f32 + 0.5) / count as f32;
            let point = center + egui::vec2(-angle.cos() * distance, -angle.sin() * distance);
            let (color, label) = seats.get(seat_index).copied().unwrap_or((UNDECIDED, "Undecided"));
            ui.painter().circle_filled(point, radius, color);
            ui.painter().circle_stroke(
                point,
                radius,
                egui::Stroke::new(0.5, INK.gamma_multiply(0.55)),
            );
            let mut cumulative = 0_usize;
            let bloc_index = config
                .bloc_sizes
                .iter()
                .position(|size| {
                    cumulative += *size as usize;
                    seat_index < cumulative
                })
                .unwrap_or(4);
            ui.interact(
                egui::Rect::from_center_size(point, egui::vec2(radius * 2.7, radius * 2.7)),
                ui.id().with(("senator", seat_index)),
                egui::Sense::hover(),
            )
            .on_hover_ui(|ui| {
                ui.strong(format!("{} · {label}", Bloc::ALL[bloc_index].label()));
                if let Some(bloc) = projection.get(bloc_index) {
                    show_reasons(ui, bloc);
                }
            });
            seat_index += 1;
        }
    }
    ui.painter().text(
        center - egui::vec2(0.0, 10.0),
        egui::Align2::CENTER_BOTTOM,
        "SENATUS",
        egui::FontId::proportional((width / 35.0).clamp(10.0, 16.0)),
        INK,
    );
}
