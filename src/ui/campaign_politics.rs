//! Parchment-styled Senate controls and a precisely one-hundred-seat chamber.

use super::campaign_widgets::{icon, sestertius_unit, stat, Icon};

use crate::game::politics::espionage::{
    EspionageConfig, EspionageState, ScandalKind, ScandalTarget, Severity,
};
use crate::game::politics::senate::{
    Bloc, PoliticalProfile, SenateConfig, SenateState, SupportReason,
};
use crate::game::politics::{PoliticalPlayer, PoliticalRank};
use bevy_egui::egui;

const INK: egui::Color32 = egui::Color32::from_rgb(57, 43, 37);
const UNDECIDED: egui::Color32 = egui::Color32::from_rgb(145, 143, 134);

#[cfg(test)]
#[path = "../../tests/unit/senate_ui.rs"]
mod tests;

/// Show persistent loyalties, office requirements and faction-specific political tools.
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
        ui.strong(actor.rank.label()).on_hover_text("Quaestor → Aedile → Praetor → Censor → Consul → Augustus. Each promotion costs Influence and requires loyal senators. Only one promotion per month.");
        stat(ui, Icon::Influence, &format!("+{:.0}/mo", config.rank_income(actor.rank)), "Monthly Influence from your current office.");
        if let Some(until) = actor.consul_until {
            ui.small(format!("{} months left", until.saturating_sub(senate.month)))
                .on_hover_text("Consuls serve 24 months, then become Proconsuls. Every departing Consul waits 12 months before seeking office again.");
        } else if actor.consul_again_at > senate.month {
            ui.small(format!("Return in {} months", actor.consul_again_at - senate.month));
        }
    });
    ui.horizontal_wrapped(|ui| {
        stat(ui, Icon::Coin, &format!("{:.0}", actor.coin), "Available sestertii.");
        stat(ui, Icon::Influence, &format!("{:.0}", actor.influence), "Available Influence.");
        stat(ui, Icon::Nobles, &format!("{} senators", senate.support(player)), "Senators in your color support you. Their loyalties are compared against every player's performance each month.");
    });
    ui.horizontal_wrapped(|ui| {
        ui.strong("Consuls");
        for (id, actor) in
            players.iter().enumerate().filter(|(_, p)| p.rank == PoliticalRank::Consul)
        {
            ui.colored_label(
                player_color(player_colors, id),
                format!(
                    "P{} · {} mo",
                    id + 1,
                    actor.consul_until.unwrap_or(senate.month).saturating_sub(senate.month)
                ),
            );
        }
        let seats = players.iter().filter(|p| p.rank == PoliticalRank::Consul).count();
        if seats < 2 {
            ui.small(format!("{} vacant", 2 - seats));
        }
    });
    let retention = config.retention_support(players.len());
    ui.small(format!("Consuls need {retention} loyal senators to retain office."))
        .on_hover_text(format!("Below {retention} supporters for {} consecutive months forces resignation. An exposed scandal forces resignation at the next monthly review if support is below {retention}. Every departure imposes a {}-month return cooldown.", config.loss_grace_months, config.consul_cooldown));
    if let Some(winner) = senate.winner {
        ui.heading(format!("Player {} is Augustus · victory", winner + 1));
    } else if let Some(requirement) = config.requirements(players[player].rank, players.len()) {
        ui.horizontal_wrapped(|ui| {
            let eligibility = senate.promotion_eligibility(player, players, config);
            let explanation = eligibility.as_ref().err().map_or_else(
                || "Appoint immediately. Influence is spent once; your senators remain loyal until their next monthly review.".to_owned(),
                ToString::to_string,
            );
            if ui.add_enabled(eligibility.is_ok(), egui::Button::new(format!("Become {}", requirement.rank.label())))
                .on_hover_text(&explanation).on_disabled_hover_text(&explanation).clicked() {
                message = Some(match senate.promote(player, players, config) {
                    Ok(_) => format!("Appointed {}.", requirement.rank.label()), Err(error) => error.to_string(),
                });
            }
            stat(ui, Icon::Influence, &format!("{:.0}", requirement.influence), "Influence required for this promotion.");
            ui.label(format!("{}/{} senators", senate.support(player), requirement.senators));
        });
    }
    ui.separator();
    ui.horizontal_wrapped(|ui| {
        color_key(
            ui,
            UNDECIDED,
            &format!(
                "Neutral {}",
                senate.senators.iter().filter(|s| s.allegiance.is_none()).count()
            ),
        );
        for id in 0..players.len() {
            color_key(
                ui,
                player_color(player_colors, id),
                &format!(
                    "P{}{} · {}",
                    id + 1,
                    if id == player {
                        " (you)"
                    } else {
                        ""
                    },
                    senate.support(id)
                ),
            );
        }
    });
    draw_chamber(ui, senate, players, profiles, config, player, player_colors);
    ui.small("Five faction sections · hover a section or senator to inspect preferences.");
    for bloc in Bloc::ALL {
        let reasons = senate.reasons(
            player,
            bloc,
            &players[player],
            &profiles.get(player).cloned().unwrap_or_default(),
            config,
        );
        ui.horizontal_wrapped(|ui| {
            icon(ui, bloc_identity(bloc), 20.0);
            ui.strong(format!("{} · {}/{}", bloc.label(), senate.bloc_support(player, bloc), config.bloc_sizes[bloc.index()]))
                .on_hover_ui(|ui| { show_reasons(ui, bloc, &reasons); });
            let mut preview = senate.clone();
            let mut wallets = players.to_vec();
            let court = preview.court(player, bloc, &mut wallets, config);
            let court_tip = court.as_ref().err().map_or_else(
                || format!("{:.0} Influence: +{:.0} attraction points fading over {} months. Does not stack. Loyal senators choose at the next monthly review.", config.court_cost, config.court_bonus, config.court_months),
                ToString::to_string,
            );
            if ui.add_enabled(court.is_ok(), egui::Button::new("Court")).on_hover_text(&court_tip).on_disabled_hover_text(&court_tip).clicked() {
                message = Some(match senate.court(player, bloc, players, config) { Ok(()) => format!("Outreach to {} begins; loyalties update next month.", bloc.label()), Err(e) => e.to_string() });
            }
            let quote = senate.bribe_quote(player, bloc, players, config);
            let affordable = quote.as_ref().is_ok_and(|cost| players[player].coin >= *cost);
            let bribe_tip = quote.as_ref().map_or_else(ToString::to_string, |cost|
                format!("{cost:.0} {}: buy one senator's loyalty immediately for {} months. At most {} active bribes. Cost rises by 20 sestertii per active bribe. One bribe per faction per month. Creates discoverable corruption evidence; exposure cancels all your bribes.", sestertius_unit(*cost), config.bribe_months, config.bribe_cap));
            if ui.add_enabled(affordable, egui::Button::new("Bribe"))
                .on_hover_text(&bribe_tip).on_disabled_hover_text(if affordable || quote.is_err() { bribe_tip.clone() } else { format!("{bribe_tip}\nInsufficient sestertii.") }).clicked() {
                message = Some(match senate.bribe(player, bloc, players, config) {
                    Ok(id) => {
                        espionage.record_action(player, None, ScandalKind::PoliticalBribery, Severity::Medium, senate.month, espionage_config);
                        format!("Senator {} supports you for {} months. Bribery is discoverable.", id + 1, config.bribe_months)
                    }, Err(e) => e.to_string(),
                });
            }
        });
    }
    ui.separator();
    ui.collapsing("Office requirements", |ui| {
        for rank in [PoliticalRank::Quaestor, PoliticalRank::Aedile, PoliticalRank::Praetor, PoliticalRank::Censor, PoliticalRank::Consul] {
            let r = config.requirements(rank, players.len()).unwrap();
            ui.label(format!("{} · {:.0} Influence · {} senators", r.rank.label(), r.influence, r.senators));
        }
        ui.small("Requirements scale with player count. Augustus always needs an absolute majority and an active Consul seat.");
    });
    ui.collapsing("Evidence and accusations", |ui| {
        let evidence: Vec<_> = espionage.scandals.iter().filter(|s| s.holder == player && s.expires > senate.month && !s.reserved_for_motion).cloned().collect();
        if evidence.is_empty() { ui.small("Spy networks uncover misconduct from actual foreign actions."); }
        for scandal in evidence {
            ui.horizontal_wrapped(|ui| {
                let target = match scandal.target { ScandalTarget::Player(id) => format!("P{}", id + 1), ScandalTarget::Province(id) => format!("Province {}", id + 1) };
                ui.label(format!("{target} · {}", scandal.kind.label())).on_hover_text(format!("{:?} · {} months until evidence expires.", scandal.severity, scandal.expires.saturating_sub(senate.month)));
                if matches!(scandal.target, ScandalTarget::Player(id) if id != player)
                    && ui.button("Expose").on_hover_text("Consumes this evidence. Applies a faction-specific attraction penalty fading over 12 months and cancels the target's bribes. Senators may switch to you, another player or neutral at the next monthly review. An incumbent with insufficient support is forcibly demoted.").clicked() {
                        message = Some(match senate.expose_scandal(player, scandal.id, players, espionage, config) { Ok(target) => format!("Exposed Player {}'s {}. Senate reviews loyalties next month.", target + 1, scandal.kind.label()), Err(e) => e.to_string() });
                }
            });
        }
        for accusation in &senate.accusations {
            ui.small(format!("P{} under scrutiny · {} months", accusation.target + 1, accusation.until.saturating_sub(senate.month)));
        }
    });
    message
}
fn player_color(colors: &[egui::Color32], player: usize) -> egui::Color32 {
    colors.get(player).copied().unwrap_or(egui::Color32::from_rgb(162, 115, 56))
}
fn bloc_identity(bloc: Bloc) -> Icon {
    match bloc {
        Bloc::Aristocrats => Icon::Nobles,
        Bloc::Merchants => Icon::Trade,
        Bloc::Provincials => Icon::Province,
        Bloc::Populares => Icon::Plebeians,
        Bloc::Military => Icon::MilitaryPower,
    }
}
fn color_key(ui: &mut egui::Ui, color: egui::Color32, label: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(13.0, 13.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 5.5, color);
        ui.label(label);
    });
}
fn show_reasons(ui: &mut egui::Ui, bloc: Bloc, reasons: &[SupportReason]) {
    ui.strong(bloc.label());
    ui.label(bloc.preferences());
    for reason in reasons {
        ui.label(format!("{}: {:+.1} attraction points", reason.label, reason.points));
    }
    ui.small("Each senator weighs these factors differently. A player must clear their personal threshold; an incumbent retains loyalty until a rival leads by more than 2 points. Equal best scores stay neutral.");
}

/// Each angular section contains only its own faction; larger circles show actual
/// persistent allegiances. Background sections remain hoverable between the seats.
fn draw_chamber(
    ui: &mut egui::Ui,
    senate: &SenateState,
    players: &[PoliticalPlayer],
    profiles: &[PoliticalProfile],
    config: &SenateConfig,
    player: usize,
    colors: &[egui::Color32],
) {
    let width = ui.available_width().min(680.0);
    let height = width * 0.56;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let center = egui::pos2(rect.center().x, rect.bottom() - 6.0);
    let outer = width * 0.47;
    let radius = (width / 64.0).clamp(3.5, 10.5);
    let pointer = ui.input(|i| i.pointer.hover_pos());
    let hover_angle = pointer.and_then(|p| {
        let d = p - center;
        (d.y <= 0.0 && d.length() <= outer + radius * 2.0 && d.length() > outer * 0.30)
            .then(|| (-d.y).atan2(-d.x))
    });
    let mut start = 0.0;
    let mut hovered_bloc = None;
    let mut hovered_senator = None;
    for bloc in Bloc::ALL {
        let arc = std::f32::consts::PI * config.bloc_sizes[bloc.index()] as f32 / 100.0;
        let hovered = hover_angle.is_some_and(|a| a >= start && a < start + arc);
        if hovered {
            hovered_bloc = Some(bloc);
        }
        let mut points = vec![center];
        for step in 0..=12 {
            let a = start + arc * step as f32 / 12.0;
            points.push(
                center + egui::vec2(-a.cos() * (outer + radius), -a.sin() * (outer + radius)),
            );
        }
        ui.painter().add(egui::Shape::convex_polygon(
            points,
            INK.gamma_multiply(if hovered {
                0.12
            } else {
                0.045
            }),
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
            if senator.bribe.is_some() {
                ui.painter().circle_stroke(
                    point,
                    radius + 1.8,
                    egui::Stroke::new(1.3, egui::Color32::from_rgb(184, 135, 52)),
                );
            }
            if pointer.is_some_and(|p| p.distance(point) < radius + 2.0) {
                hovered_senator = Some(*senator);
            }
        }
        let angle = start + arc * 0.5;
        let label = match bloc {
            Bloc::Aristocrats => "Nobles",
            Bloc::Merchants => "Trade",
            Bloc::Provincials => "Provinces",
            Bloc::Populares => "Plebs",
            Bloc::Military => "Military",
        };
        let mut label_point =
            center + egui::vec2(-angle.cos(), -angle.sin()) * (outer + radius * 2.9);
        label_point.x = label_point.x.clamp(rect.left() + 32.0, rect.right() - 32.0);
        ui.painter().text(
            label_point,
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional((width / 46.0).clamp(9.0, 14.0)),
            INK,
        );
        start += arc;
    }
    ui.painter().text(
        center - egui::vec2(0.0, 12.0),
        egui::Align2::CENTER_BOTTOM,
        "SENATUS",
        egui::FontId::proportional((width / 36.0).clamp(10.0, 17.0)),
        INK,
    );
    if let Some(bloc) = hovered_bloc {
        response.on_hover_ui(|ui| {
            if let Some(s) = hovered_senator {
                ui.strong(format!(
                    "Senator {} · {}",
                    s.id + 1,
                    s.allegiance.map_or("Neutral".to_owned(), |p| format!("Player {}", p + 1))
                ));
                if let Some(bribe) = s.bribe {
                    ui.label(format!(
                        "Bribed · {} months remaining",
                        bribe.until.saturating_sub(senate.month)
                    ));
                }
            }
            let reasons = senate.reasons(
                player,
                bloc,
                &players[player],
                &profiles.get(player).cloned().unwrap_or_default(),
                config,
            );
            show_reasons(ui, bloc, &reasons);
            ui.label(format!(
                "Your supporters: {} of {}",
                senate.bloc_support(player, bloc),
                config.bloc_sizes[bloc.index()]
            ));
        });
    }
}
