//! Scrollable campaign panels using the existing parchment, banner colors, and button treatment.

use super::campaign::Campaign;
use super::campaign_notifications::{NoticeAction, NoticeSeverity};
use super::campaign_widgets::{icon, stat, Icon};
use super::*;
use crate::game::economy::ConstructionProject;
use crate::game::military::*;
use crate::game::politics::diplomacy::*;
use crate::game::politics::{Currency, PoliticalError, PoliticalPlayer};

/// Global banner destinations and the contextual province inspector.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CampaignTab {
    Governance,
    Military,
    Trade,
    Senate,
    Province,
}

/// View state is separate from game rules and never changes simulation outcomes.
#[derive(Resource, Default)]
pub(crate) struct CampaignUi {
    pub open: Option<CampaignTab>,
    pub province: Option<usize>,
    section: usize,
    last_detail: Option<MapDetail>,
    notice: String,
    highlight_scandal: Option<u64>,
}

impl CampaignUi {
    /// Open the context containing acquired evidence instead of a generic overview.
    pub(crate) fn open_evidence(&mut self, province: Option<usize>) {
        if let Some(province) = province {
            self.province = Some(province);
            self.section = 4;
            self.open = Some(CampaignTab::Province);
            self.last_detail = Some(MapDetail::Province(province));
        } else {
            self.open = Some(CampaignTab::Senate);
        }
    }
}

/// Match the hand-painted map's parchment surfaces locally without changing menu styling.
fn panel_style(ui: &mut egui::Ui, scale: f32) {
    *ui.style_mut() = campaign_widgets::map_style(scale);
}

/// The same frame is shared by native rendering and whole-panel layout checks.
fn panel_frame(scale: f32) -> egui::Frame {
    egui::Frame::new()
        .fill(province_panel::PAPER)
        .stroke(egui::Stroke::new(1.2, province_panel::RULE))
        .corner_radius(6.0)
        .inner_margin(12.0 * scale)
}

/// Header content participates in normal layout so wrapped titles and tabs are
/// accounted for before the body receives its remaining vertical space.
fn panel_header(
    ui: &mut egui::Ui,
    title: &str,
    scale: f32,
    provinces: &[crate::game::economy::EconomicProvince],
    player: usize,
    view: &mut CampaignUi,
) -> bool {
    let tab = view.open.unwrap_or(CampaignTab::Province);
    let province = view.province.unwrap_or(0).min(provinces.len() - 1);
    let mut closed = false;
    let color = ui
        .ctx()
        .data(|d| d.get_temp::<egui::Color32>(egui::Id::new("campaign-owner-color")))
        .unwrap_or(PLAYER_COLORS[0]);
    let brightness =
        f32::from(color.r()) * 0.299 + f32::from(color.g()) * 0.587 + f32::from(color.b()) * 0.114;
    let header_ink = if brightness > 150.0 {
        province_panel::INK
    } else {
        egui::Color32::from_rgb(255, 238, 210)
    };
    egui::Frame::new().fill(color).inner_margin(6.0 * scale).corner_radius(3.0).show(ui, |ui| {
        ui.horizontal(|ui| {
            icon(ui, Icon::Eagle, 32.0 * scale);
            let available = (ui.available_width() - 40.0 * scale).max(1.0);
            ui.add_sized(
                [available, 32.0 * scale],
                egui::Label::new(egui::RichText::new(title).size(20.0 * scale).color(header_ink))
                    .truncate(),
            )
            .on_hover_text(title);
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(28.0, 28.0) * scale, egui::Sense::click());
            let painter = ui.painter();
            painter.circle_filled(
                rect.center(),
                12.0 * scale,
                if response.hovered() {
                    egui::Color32::from_white_alpha(35)
                } else {
                    egui::Color32::TRANSPARENT
                },
            );
            painter.circle_stroke(
                rect.center(),
                12.0 * scale,
                egui::Stroke::new(1.3 * scale, header_ink),
            );
            for sign in [-1.0, 1.0] {
                painter.line_segment(
                    [
                        rect.center() + egui::vec2(-4.5, sign * -4.5) * scale,
                        rect.center() + egui::vec2(4.5, sign * 4.5) * scale,
                    ],
                    egui::Stroke::new(1.7 * scale, header_ink),
                );
            }
            closed = response
                .on_hover_text("Close this panel · Escape")
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked();
        });
    });
    if tab != CampaignTab::Senate {
        egui::ComboBox::from_id_salt("campaign_province_selector")
            .selected_text(&provinces[province].name)
            .width((ui.available_width() - 28.0 * scale).max(1.0))
            .show_ui(ui, |ui| {
                for (id, province) in provinces.iter().enumerate() {
                    ui.selectable_value(
                        &mut view.province,
                        Some(id),
                        format!(
                            "{}{}",
                            province.name,
                            if province.owner == Some(player) {
                                " · yours"
                            } else {
                                ""
                            }
                        ),
                    );
                }
            });
        if tab == CampaignTab::Province {
            let tabs = [
                (Icon::Province, "Overview"),
                (Icon::Population, "Policies"),
                (Icon::Building(crate::game::economy::BuildingType::Forum), "Buildings"),
                (Icon::Attack, "Military"),
                (Icon::Control, "Diplomacy"),
                (Icon::Trade, "Trade"),
            ];
            let width = (ui.available_width() - 5.0 * ui.spacing().item_spacing.x) / 6.0;
            ui.horizontal(|ui| {
                for (index, (symbol, label)) in tabs.into_iter().enumerate() {
                    let (rect, response) = ui
                        .allocate_exact_size(egui::vec2(width, 47.0 * scale), egui::Sense::click());
                    let selected = view.section == index;
                    if selected || response.hovered() {
                        ui.painter().rect_filled(
                            rect,
                            3.0,
                            if selected {
                                egui::Color32::from_rgb(219, 200, 168)
                            } else {
                                province_panel::TABLE_STRIPE
                            },
                        );
                    }
                    ui.scope_builder(
                        egui::UiBuilder::new()
                            .max_rect(rect)
                            .layout(egui::Layout::top_down(egui::Align::Center)),
                        |ui| {
                            icon(ui, symbol, 26.0 * scale);
                            ui.label(egui::RichText::new(label).size(11.0 * scale));
                        },
                    );
                    if selected {
                        ui.painter().hline(
                            rect.x_range(),
                            rect.bottom(),
                            egui::Stroke::new(2.0 * scale, color),
                        );
                    }
                    if response
                        .on_hover_text(label)
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        view.section = index;
                    }
                }
            });
        }
        ui.separator();
        if (tab == CampaignTab::Province && matches!(view.section, 0 | 2))
            || tab == CampaignTab::Governance
        {
            campaign_widgets::portrait(
                ui,
                provinces[province].terrain,
                view.section == 2,
                76.0 * scale,
            );
        }
    }
    closed
}

/// Split the actual space left by the header between scrollable content and a
/// bounded, wrapped status message. Newly produced status appears next frame so
/// an action cannot grow the panel after its body budget has already been used.
fn scroll_body_with_status<R>(
    ui: &mut egui::Ui,
    inner_bottom: f32,
    scale: f32,
    status: &str,
    id: impl std::hash::Hash + std::fmt::Debug,
    render: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let remaining = (inner_bottom - ui.next_widget_position().y).max(0.0);
    let gap = ui.spacing().item_spacing.y;
    let footer = if status.is_empty() {
        0.0
    } else {
        (72.0 * scale).min(remaining * 0.25)
    };
    let body = (remaining
        - footer
        - if footer > 0.0 {
            gap
        } else {
            0.0
        })
    .max(0.0);
    let id = egui::Id::new(id);
    let response = egui::ScrollArea::vertical()
        .id_salt(id)
        .max_height(body)
        .min_scrolled_height(0.0)
        .auto_shrink([false, false])
        .show(ui, render);
    if footer > 0.0 {
        egui::ScrollArea::vertical()
            .id_salt(id.with("status"))
            .max_height(footer)
            .min_scrolled_height(0.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add(egui::Label::new(status).wrap());
            });
    }
    response.inner
}

/// Render one bounded panel; section bodies use normal layout and an internal scroll area.
pub(in crate::app) fn draw(
    mut contexts: EguiContexts,
    state: Res<State<AppState>>,
    mut campaign: ResMut<Campaign>,
    mut view: ResMut<CampaignUi>,
    mut detail: ResMut<ProvincePanelOpen>,
    mut close_click: ResMut<MapPanelCloseClick>,
    practice: Res<LocalPractice>,
    mut map_view: ResMut<MapView>,
    sound: Res<MenuAudio>,
    audio: Res<Audio>,
    assets: Res<AssetServer>,
) {
    if !campaign.active || *state.get() != AppState::Map {
        return;
    }
    if detail.0 != view.last_detail {
        view.last_detail = detail.0;
        if let Some(selected) = detail.0 {
            view.province = Some(match selected {
                MapDetail::Province(id) | MapDetail::City(id) => id,
            });
            view.section = if matches!(selected, MapDetail::City(_)) {
                2
            } else {
                0
            };
            view.open = Some(CampaignTab::Province);
        }
    }
    let Some(tab) = view.open else {
        return;
    };
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let player = practice.active_player;
    if player >= campaign.actors.len() {
        return;
    }
    let scale = viewport_ui_scale(ctx.content_rect().size());
    let screen = ctx.content_rect();
    let width = (if tab == CampaignTab::Senate {
        730.0
    } else {
        570.0
    }) * scale;
    let width = width.min((screen.width() - 80.0 * scale).max(120.0));
    let height = (720.0 * scale).min((screen.height() - 90.0 * scale).max(140.0));
    let rect = map_corner_panel_rect(screen, scale, egui::vec2(width, height));
    if view.province.is_none() {
        view.province =
            campaign.economy.provinces.iter().position(|p| p.owner == Some(player)).or(Some(0));
    }
    let province = view.province.unwrap_or(0).min(campaign.politics.len() - 1);
    campaign.pull_wallets();
    campaign.refresh_profiles();
    let header = match tab {
        CampaignTab::Senate => "Rome · Senate".to_owned(),
        CampaignTab::Governance => "Province Governance".to_owned(),
        CampaignTab::Military => "Military".to_owned(),
        CampaignTab::Trade => "Trade & Routes".to_owned(),
        CampaignTab::Province => campaign.economy.provinces[province].name.clone(),
    };
    let mut closed = false;
    let mut action = None;
    let mut navigation = None;
    let old_notice = view.notice.clone();
    let owner_color = campaign.economy.provinces[province]
        .owner
        .and_then(|owner| practice.players.get(owner))
        .map(|owner| PLAYER_COLORS[owner.color_index])
        .unwrap_or(province_panel::NEUTRAL);
    ctx.data_mut(|d| {
        d.insert_temp(
            egui::Id::new("campaign-owner-color"),
            if tab == CampaignTab::Senate {
                PLAYER_COLORS[practice.players[player].color_index]
            } else {
                owner_color
            },
        )
    });
    egui::Area::new(egui::Id::new("augustus_campaign_panel"))
        .fixed_pos(rect.min)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            panel_style(ui, scale);
            panel_frame(scale).show(ui, |ui| {
                ui.set_width(width - 24.0 * scale - 2.4);
                ui.set_min_height(height - 24.0 * scale - 2.4);
                closed = panel_header(
                    ui,
                    &header,
                    scale,
                    &campaign.economy.provinces,
                    player,
                    &mut view,
                );
                scroll_body_with_status(
                    ui,
                    rect.bottom() - 12.0 * scale - 1.2,
                    scale,
                    &old_notice,
                    ("campaign_body", tab as usize, province, view.section),
                    |ui| {
                        if tab == CampaignTab::Senate {
                            if let Some(id) = view.highlight_scandal {
                                ui.group(|ui| {
                                    if let Some(scandal) = campaign
                                        .espionage
                                        .scandals
                                        .iter()
                                        .find(|s| s.id == id && s.holder == player)
                                    {
                                        ui.strong(format!(
                                            "Selected evidence · {}",
                                            scandal.kind.label()
                                        ));
                                        ui.label(format!(
                                            "{:?} · {:?} · {} months remaining",
                                            scandal.target,
                                            scandal.severity,
                                            scandal.expires.saturating_sub(campaign.senate.month)
                                        ));
                                    } else {
                                        ui.label("This evidence has been consumed or has expired.");
                                    }
                                    if ui.small_button("Dismiss selection").clicked() {
                                        view.highlight_scandal = None;
                                    }
                                });
                            }
                            let colors: Vec<_> = practice
                                .players
                                .iter()
                                .map(|p| PLAYER_COLORS[p.color_index])
                                .collect();
                            let Campaign {
                                senate,
                                actors,
                                profiles,
                                senate_config,
                                espionage,
                                espionage_config,
                                ..
                            } = &mut *campaign;
                            if let Some(message) = campaign_politics::show(
                                ui,
                                senate,
                                actors,
                                profiles,
                                senate_config,
                                player,
                                espionage,
                                espionage_config,
                                &colors,
                            ) {
                                view.notice = message;
                            }
                        } else {
                            let section = match tab {
                                CampaignTab::Governance => 1,
                                CampaignTab::Military => 3,
                                CampaignTab::Trade => 5,
                                _ => view.section,
                            };
                            match section {
                                0 => campaign_economy::overview(ui, &campaign.economy, province),
                                1 => campaign_economy::policies(
                                    ui,
                                    &mut campaign.economy,
                                    province,
                                    player,
                                ),
                                2 => {
                                    let before = wonder_under_construction(&campaign, province);
                                    if let Some(message) = campaign_economy::buildings(
                                        ui,
                                        &mut campaign.economy,
                                        province,
                                        player,
                                    ) {
                                        view.notice = message;
                                    }
                                    let after = wonder_under_construction(&campaign, province);
                                    if before != after {
                                        if let Some(wonder) = after {
                                            campaign
                                                .notify_wonder_started(player, province, wonder);
                                        }
                                    }
                                },
                                3 => {
                                    let access = campaign.access_snapshot();
                                    action = campaign_military::show(
                                        ui,
                                        &campaign.military,
                                        &campaign.economy,
                                        &campaign.graph,
                                        province,
                                        player,
                                        |owner, id| match owner {
                                            ForceOwner::Player(p) => access[p][id],
                                            ForceOwner::Local(home) => {
                                                if home == id {
                                                    MilitaryAccess::Peaceful
                                                } else {
                                                    MilitaryAccess::Blocked
                                                }
                                            },
                                        },
                                        |a, b| campaign.forces_hostile(a, b),
                                    );
                                },
                                4 => {
                                    if let Some(message) =
                                        diplomacy(ui, &mut campaign, province, player)
                                    {
                                        view.notice = message;
                                    }
                                },
                                _ => {
                                    let inputs = campaign.inputs();
                                    if let Some(effect) = campaign_economy::trade(
                                        ui,
                                        &mut campaign.economy,
                                        &inputs,
                                        province,
                                        player,
                                    ) {
                                        let config = campaign.diplomacy_config.clone();
                                        campaign.politics[effect.province].apply_trade(
                                            effect.player,
                                            effect.relation_gain,
                                            effect.control_gain,
                                            &config,
                                        );
                                    }
                                },
                            }
                        }
                        if let Some(target) = recent_notifications(ui, &campaign, player) {
                            navigation = Some(target);
                        }
                    },
                );
            });
        });
    // Political and economic controls have separate wallet boundaries.
    if tab == CampaignTab::Senate || (tab == CampaignTab::Province && view.section == 4) {
        campaign.push_wallets();
    }
    if let Some(action) = action {
        view.notice = apply_military_action(&mut campaign, province, player, action);
    }
    if let Some((navigation, province)) = navigation {
        match navigation {
            NoticeAction::OpenSenate => {
                view.open = Some(CampaignTab::Senate);
                view.section = 0;
                detail.0 = None;
            },
            NoticeAction::OpenProvince(id) => {
                view.province = Some(id);
                view.section = 0;
                view.open = Some(CampaignTab::Province);
                detail.0 = Some(MapDetail::Province(id));
                view.last_detail = detail.0;
                map_view.focus_province(id);
            },
            NoticeAction::FocusWonder(id) => {
                map_view.focus_wonder(id);
                if let Some(province) = province {
                    view.province = Some(province);
                    view.section = 2;
                    view.open = Some(CampaignTab::Province);
                    detail.0 = Some(MapDetail::Province(province));
                    view.last_detail = detail.0;
                }
            },
            NoticeAction::OpenScandal {
                scandal,
                province,
            } => {
                let npc = campaign
                    .espionage
                    .scandals
                    .iter()
                    .find(|evidence| evidence.id == scandal)
                    .is_some_and(|evidence| {
                        matches!(
                            evidence.target,
                            crate::game::politics::espionage::ScandalTarget::Province(_)
                        )
                    });
                view.open_evidence(if npc {
                    province
                } else {
                    None
                });
                if npc {
                    if let Some(province) = province {
                        detail.0 = Some(MapDetail::Province(province));
                    }
                } else {
                    view.highlight_scandal = Some(scandal);
                }
                if let Some(province) = province {
                    map_view.focus_province(province);
                }
            },
        }
    }
    if closed {
        view.open = None;
        detail.0 = None;
        view.last_detail = None;
        close_click.0 = true;
        play_click(&sound, &audio, &assets);
    } else if old_notice != view.notice {
        play_click(&sound, &audio, &assets);
    }
}

/// Dispatch validated military commands; never grant free recruits or mutate locked battle plans.
fn apply_military_action(
    campaign: &mut Campaign,
    province: usize,
    player: usize,
    action: campaign_military::MilitaryUiAction,
) -> String {
    use campaign_military::MilitaryUiAction::*;
    let result: Result<(), String> = match action {
        Recruit(kind) => {
            let p = &mut campaign.economy.provinces[province];
            campaign
                .military
                .recruit(
                    province,
                    player,
                    kind,
                    p.owner == Some(player),
                    &recruitment_tags(&p.name),
                    &mut p.population,
                    &mut campaign.economy.players[player].resources[1],
                )
                .map_err(|e| e.to_string())
        },
        CancelRecruitment => campaign
            .military
            .cancel_recruitment(province, ForceOwner::Player(player))
            .map_err(|e| e.to_string()),
        Disband(id) => {
            let p = &mut campaign.economy.provinces[province];
            campaign
                .military
                .disband(province, player, id, p.owner == Some(player), &mut p.population)
                .map_err(|e| e.to_string())
        },
        SavePlan(plan) => campaign
            .military
            .set_plan(province, ForceOwner::Player(player), plan)
            .map_err(|e| e.to_string()),
        SaveMovementPlan {
            order,
            plan,
        } => campaign
            .military
            .set_movement_plan(order, ForceOwner::Player(player), plan)
            .map_err(|e| e.to_string()),
        Move {
            destination,
            units,
            route,
            plan,
        } => {
            if route.last().copied() != Some(destination) {
                return "The selected route no longer reaches the destination.".into();
            }
            let access = campaign.access_snapshot();
            campaign
                .military
                .order_movement_route(
                    province,
                    ForceOwner::Player(player),
                    &units,
                    Some(plan),
                    &route,
                    &campaign.graph,
                    |owner, id| match owner {
                        ForceOwner::Player(p) => access[p][id],
                        _ => MilitaryAccess::Blocked,
                    },
                )
                .map(|_| ())
                .map_err(|e| e.to_string())
        },
        Retreat {
            battle,
            attacker,
        } => {
            let config = campaign.military.config.clone();
            campaign
                .military
                .battles
                .iter_mut()
                .find(|b| b.id == battle)
                .ok_or_else(|| "Battle has ended.".to_owned())
                .and_then(|b| b.request_retreat(attacker, &config).map_err(|e| e.to_string()))
        },
    };
    result.map_or_else(|message| message, |_| "Order accepted.".to_owned())
}

/// Display sovereignty, exact domain-quoted prices, eligibility, spies and military access.
fn diplomacy(
    ui: &mut egui::Ui,
    campaign: &mut Campaign,
    province: usize,
    player: usize,
) -> Option<String> {
    let state = campaign.politics[province].state.clone();
    let relation = campaign.politics[province].relation(player);
    let distance = campaign.distance(player, province);
    let config = campaign.diplomacy_config.clone();
    ui.horizontal_wrapped(|ui| {
        stat(ui, Icon::Coin, &format!("{:.0}", campaign.actors[player].coin), "Available Coin.");
        stat(ui, Icon::Influence, &format!("{:.0}", campaign.actors[player].influence), "Available Influence.");
        stat(ui, Icon::Trade, &distance.map_or_else(|| "—".into(), |d| d.to_string()), &distance.map_or_else(|| "No usable political route.".into(), |d| format!("Political distance: {d} edges. Spending multiplier ×{:.2}. Includes usable sea crossings.", distance_multiplier(Some(d)).unwrap_or(1.0))));
    });
    if !matches!(state, PoliticalState::Owned { owner } if owner == player) {
        diplomatic_meter(ui, Icon::Happiness, "Relation", relation, "Friendship toward you. Political Control separately measures power; occupation can create high Control with low Relation.");
    }
    let mut result: Option<Result<(), PoliticalError>> = None;
    let mut message = None;
    match &state {
        PoliticalState::Independent {
            local,
            shares,
        } => {
            ui.strong("Independent").on_hover_text("51 Control and a unique lead permit vassalization. 100 Control permits ownership. Vassalization is optional; initial Vassal Control equals Independent Control minus 50.");
            diplomatic_meter(
                ui,
                Icon::Province,
                "Local",
                *local,
                "Control retained by the independent local government.",
            );
            for (id, share) in shares.iter().enumerate() {
                if *share > 0.0 || id == player {
                    diplomatic_meter(ui, Icon::Control, &format!("P{}{}", id + 1, if id == player { " · you" } else { "" }), *share, "Independent Control share. All players and Local sum to 100. Monthly political pressure resolves simultaneously.");
                }
            }
            let pending = campaign.politics[province].queued_control_pressure(player);
            if pending > 0.0 {
                stat(ui, Icon::Control, &format!("+{pending:.1} pending"), "Your paid control and blackmail pressure awaits simultaneous month-end resolution. Rival pressure can change the final gain; this has not already been added to your Control.");
            }
            let mut preview = campaign.politics[province].clone();
            let vassal_eligible = preview.vassalize(player).is_ok();
            let mut preview = campaign.politics[province].clone();
            let ownership_eligible = preview.take_ownership(player).is_ok();
            ui.horizontal_wrapped(|ui| {
                if ui.add_enabled(vassal_eligible, egui::Button::new("Vassalize"))
                    .on_hover_text("Create a vassal; retain relations and remove every independent control share.")
                    .on_disabled_hover_text("Requires at least 51 Independent Control and a unique lead over every rival.").clicked() {
                    result = Some(campaign.politics[province].vassalize(player));
                }
                if ui.add_enabled(ownership_eligible, egui::Button::new("Integrate"))
                    .on_hover_text(format!("Directly own this province. Current relation produces a {:+.1} happiness shift for every class.", relation - 50.0))
                    .on_disabled_hover_text("Requires 100 Independent Control, resolved at the monthly boundary.").clicked() {
                    result = Some(integrate_province(campaign, province, player));
                }
            });
            ui.horizontal_wrapped(|ui| {
                for currency in [Currency::Coin, Currency::Influence] {
                    let quote = political_quote(&campaign.politics[province], &campaign.actors[player], currency,
                        |p, actor| p.buy_control(player, actor, currency, 5.0, distance, &config));
                    if political_button(ui, "+5 Control", currency, quote,
                        "Adds pressure to the next simultaneous monthly resolution. Distance, relation, your existing position and rival entrenchment determine the price.") {
                        result = Some(campaign.politics[province].buy_control(player, &mut campaign.actors[player], currency, 5.0, distance, &config));
                    }
                }
            });
        },
        PoliticalState::Vassal {
            overlord,
            control,
            ..
        } => {
            let delta = &campaign.politics[province].last_control_change;
            ui.horizontal_wrapped(|ui| {
                ui.strong(format!("Vassal · P{}", overlord + 1));
                stat(ui, Icon::Control, &format!("{:+.1}/mo", delta.total), &format!("Last monthly change. Relation {:+.2}; garrison {:+.2}; paid support {:+.2}. At 0 the province becomes independent; at 100 integration is optional.", delta.relation, delta.military, delta.support));
            });
            diplomatic_meter(ui, Icon::Control, "Control", *control, "Vassal stability and integration progress. Relation below 50 causes decay; military garrisons and paid government support can offset it.");
            if *overlord == player {
                if let PoliticalState::Vassal {
                    tribute,
                    ..
                } = &mut campaign.politics[province].state
                {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Tribute:");
                        for (level, label) in [(Tribute::Low, "Low"), (Tribute::Normal, "Normal"), (Tribute::High, "High")] {
                            ui.selectable_value(tribute, level, label).on_hover_text(format!("Pays {:.0}% of available monthly Coin output. Relation changes {:+.1} per month.", level.income_fraction() * 100.0, level.relation_delta()));
                        }
                    });
                }
                if ui.add_enabled(*control >= 100.0 - 1e-7, egui::Button::new("Integrate"))
                    .on_hover_text(format!("End vassal tribute and vassal Influence. Relation produces a {:+.1} happiness shift for every population class.", relation - 50.0))
                    .on_disabled_hover_text("Requires 100 Vassal Control. Integration is always your choice.").clicked() {
                    result = Some(integrate_province(campaign, province, player));
                }
                ui.horizontal_wrapped(|ui| {
                    for currency in [Currency::Coin, Currency::Influence] {
                        let quote = political_quote(
                            &campaign.politics[province],
                            &campaign.actors[player],
                            currency,
                            |p, actor| {
                                p.support_government(
                                    player, actor, currency, 5.0, distance, &config,
                                )
                            },
                        );
                        if political_button(
                            ui,
                            "+5 Control",
                            currency,
                            quote,
                            "Support the government directly. This does not improve Relation.",
                        ) {
                            result = Some(campaign.politics[province].support_government(
                                player,
                                &mut campaign.actors[player],
                                currency,
                                5.0,
                                distance,
                                &config,
                            ));
                        }
                    }
                });
            }
        },
        PoliticalState::Owned {
            owner,
        } => {
            ui.horizontal_wrapped(|ui| {
                icon(ui, Icon::Province, 22.0);
                ui.strong(format!("Owned · P{}", owner + 1)).on_hover_text(
                    "Domestic happiness replaces foreign Control in directly owned provinces.",
                );
            });
            if let Some(text) = military_diplomacy(ui, campaign, province, player, *owner) {
                message = Some(text);
            }
        },
    }
    if !matches!(campaign.politics[province].state, PoliticalState::Owned { .. }) {
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            for currency in [Currency::Coin, Currency::Influence] {
                let quote = political_quote(&campaign.politics[province], &campaign.actors[player], currency,
                    |p, actor| p.improve_relation(player, actor, currency, 5.0, distance, &config));
                if political_button(ui, "+5 Relation", currency, quote, "Pay immediately. High existing friendship has diminishing returns; political distance also raises the price.") {
                    result = Some(campaign.politics[province].improve_relation(player, &mut campaign.actors[player], currency, 5.0, distance, &config));
                }
            }
        });
        ui.collapsing("Monthly support", |ui| {
        let route = distance_multiplier(distance);
        let support = &mut campaign.politics[province].support[player];
        for (enabled, currency, base, label) in [
            (&mut support.relation_coin, Currency::Coin, config.monthly_coin, "+1 Relation"),
            (
                &mut support.relation_influence,
                Currency::Influence,
                config.monthly_influence,
                "+1 Relation",
            ),
        ] {
            recurring_support(ui, enabled, currency, base, route, label);
        }
        if matches!(campaign.politics[province].state, PoliticalState::Vassal { overlord, .. } if overlord == player)
        {
            let support = &mut campaign.politics[province].support[player];
            recurring_support(
                ui,
                &mut support.control_coin,
                Currency::Coin,
                config.monthly_coin,
                route,
                "+1 Control",
            );
            recurring_support(
                ui,
                &mut support.control_influence,
                Currency::Influence,
                config.monthly_influence,
                route,
                "+1 Control",
            );
        }
        });
    }
    ui.separator();
    let foreign = !matches!(campaign.politics[province].state, PoliticalState::Owned { owner } if owner == player)
        && !matches!(campaign.politics[province].state, PoliticalState::Vassal { overlord, .. } if overlord == player);
    if foreign {
        let deployed = campaign
            .espionage
            .missions
            .iter()
            .any(|mission| mission.owner == player && mission.province == province);
        let spy_config = &campaign.espionage_config;
        let can_deploy = campaign.actors[player].influence >= spy_config.deployment_influence;
        let detection = crate::game::politics::espionage::detection_chance(
            campaign.economy.provinces[province].happiness[0],
            spy_config,
        );
        let age = campaign
            .espionage
            .missions
            .iter()
            .find(|mission| mission.owner == player && mission.province == province)
            .map_or(0, |mission| mission.months_active);
        ui.horizontal_wrapped(|ui| {
        icon(ui, Icon::Spy, 24.0).on_hover_text(format!("{} · active {age} months. Monthly detection risk {:.1}% from Noble happiness {:.1}. Detection resolves before discovery.", if deployed { "Network active" } else { "No network" }, detection * 100.0, campaign.economy.provinces[province].happiness[0]));
        let label = if deployed { "Withdraw" } else { "Deploy spy" };
        if ui.add_enabled(deployed || can_deploy, egui::Button::new(label))
            .on_hover_text(format!("Deployment: {:.1} Influence. Maintenance: {:.1} Coin/month. Detection is checked before scandal discovery. Happier Nobles make spies easier to detect. Unpaid or detected networks are removed.", spy_config.deployment_influence, spy_config.monthly_coin))
            .on_disabled_hover_text(format!("Requires {:.1} Influence to deploy; only one network per player and province is allowed.", spy_config.deployment_influence)).clicked() {
            if deployed { campaign.espionage.withdraw(player, province); message = Some("Spy network withdrawn.".into()); }
            else { result = Some(campaign.espionage.deploy(player, province, &mut campaign.actors, &campaign.politics, &campaign.espionage_config)); }
        }
        stat(ui, Icon::Coin, &format!("{:.0}/mo", spy_config.monthly_coin), "Monthly upkeep of a deployed spy network.");
        });
        if !matches!(campaign.politics[province].state, PoliticalState::Owned { .. }) {
            let hostile = campaign.npc_wars[player][province];
            let battling = campaign.military.province_in_battle(province);
            if hostile {
                if ui.add_enabled(!battling, egui::Button::new("End hostility"))
                    .on_hover_text("End invasion permission and military occupation. Peaceful access still requires friendly relations.")
                    .on_disabled_hover_text("Resolve or retreat from the battle before ending hostility.").clicked() {
                    campaign.npc_wars[player][province] = false;
                    if campaign.military.provinces[province].occupation == Some(ForceOwner::Player(player)) { campaign.military.provinces[province].occupation = None; }
                    message = Some("Hostility ended. Military occupation no longer creates Control.".into());
                }
            } else if ui.button("Declare hostility").on_hover_text("Permits invasion and starts a battle if hostile units coexist. NPC defenders must be defeated before occupation can generate Control.").clicked() {
                campaign.declare_hostility(player, province); message = Some("Hostility declared.".into());
            }
        }
    }
    let current_state = campaign.politics[province].state.clone();
    for rival in 0..campaign.actors.len() {
        if rival == player {
            continue;
        }
        let actions: &[Interference] = match current_state {
            PoliticalState::Independent {
                ..
            } => &[
                Interference::SmearCampaign,
                Interference::FundOpposition,
                Interference::UndermineRival,
            ],
            PoliticalState::Vassal {
                overlord,
                ..
            } if overlord == rival => &[
                Interference::SmearCampaign,
                Interference::PoliticalAgitation,
                Interference::FundDissidents,
            ],
            PoliticalState::Owned {
                owner,
            } if owner == rival => &[Interference::AgitatePopulation, Interference::FundUnrest],
            _ => &[],
        };
        if actions.is_empty() {
            continue;
        }
        egui::CollapsingHeader::new(format!("Rival · P{}", rival + 1))
            .id_salt((province, rival))
            .show(ui, |ui| {
                for &action in actions {
                    let (label, currency, explanation) = interference_details(action);
                    let quote = political_quote(
                        &campaign.politics[province],
                        &campaign.actors[player],
                        currency,
                        |p, actor| {
                            p.interfere(player, rival, actor, action, distance, &config).map(|_| ())
                        },
                    );
                    if political_button(ui, label, currency, quote, explanation) {
                        result = Some(
                            campaign.politics[province]
                                .interfere(
                                    player,
                                    rival,
                                    &mut campaign.actors[player],
                                    action,
                                    distance,
                                    &config,
                                )
                                .map(|changes| {
                                    campaign
                                        .notifications
                                        .record_foreign_happiness(player, rival, province, changes);
                                    for (class, change) in changes.into_iter().enumerate() {
                                        campaign.economy.provinces[province].temporary_happiness
                                            [class] += change;
                                    }
                                }),
                        );
                    }
                }
            });
    }
    if let Some(text) = campaign_politics::npc_evidence(
        ui,
        &mut campaign.espionage,
        &mut campaign.politics,
        player,
        province,
        campaign.economy.month,
        &campaign.espionage_config,
    ) {
        message = Some(text);
    }
    if matches!(campaign.politics[province].state, PoliticalState::Owned { .. }) {
        let evidence: Vec<_> = campaign
            .espionage
            .scandals
            .iter()
            .filter(|evidence| {
                evidence.holder == player
                    && evidence.province == Some(province)
                    && (evidence.expires > campaign.economy.month || evidence.reserved_for_motion)
            })
            .collect();
        if !evidence.is_empty() {
            ui.separator();
            ui.collapsing(format!("Evidence · {}", evidence.len()), |ui| {
            for scandal in evidence {
                ui.horizontal_wrapped(|ui| {
                    icon(ui, Icon::Spy, 18.0);
                    ui.label(scandal.kind.label()).on_hover_text(format!("{:?} · expires in {} months. Use player evidence in the Senate campaign or for No Confidence against an active Consul.", scandal.severity, scandal.expires.saturating_sub(campaign.economy.month)));
                });
            }
            });
        }
    }
    result
        .map(|value| {
            value.map_or_else(|error| error.to_string(), |_| "Political action accepted.".into())
        })
        .or(message)
}

#[cfg(test)]
#[path = "../../tests/unit/ui_layout.rs"]
mod layout_tests;

/// Apply integration's already-computed relation shift once while preserving other causes.
fn integrate_province(
    campaign: &mut Campaign,
    province: usize,
    player: usize,
) -> Result<(), PoliticalError> {
    let shift = campaign.politics[province].take_ownership(player)?;
    for happiness in &mut campaign.economy.provinces[province].temporary_happiness {
        *happiness += shift;
    }
    Ok(())
}

/// A validated price and the original actor's ability to pay it.
#[derive(Debug, Clone, Copy)]
struct PoliticalQuote {
    cost: f64,
    affordable: bool,
}

/// A compact illustrated meter puts the rule explanation on the entire row's hover.
fn diplomatic_meter(ui: &mut egui::Ui, symbol: Icon, label: &str, value: f64, explanation: &str) {
    ui.horizontal(|ui| {
        icon(ui, symbol, 21.0);
        ui.add_sized([61.0, 20.0], egui::Label::new(label));
        ui.add(
            egui::ProgressBar::new((value / 100.0).clamp(0.0, 1.0) as f32)
                .desired_width(ui.available_width().max(1.0))
                .text(format!("{value:.1}")),
        );
    })
    .response
    .on_hover_text(explanation);
}

/// Run the authoritative operation on cloned state so UI prices and legality cannot
/// diverge from the actual action. No pending pressure or wallet change escapes the preview.
fn political_quote(
    province: &ProvincePolitics,
    actor: &PoliticalPlayer,
    currency: Currency,
    operation: impl FnOnce(&mut ProvincePolitics, &mut PoliticalPlayer) -> Result<(), PoliticalError>,
) -> Result<PoliticalQuote, PoliticalError> {
    let mut preview = province.clone();
    let mut wallet = actor.clone();
    // The panel quotes five-point actions. This finite budget safely exceeds every
    // configured price while retaining sub-cent precision in the subtraction.
    let preview_budget = 1_000_000_000.0;
    wallet.coin = preview_budget;
    wallet.influence = preview_budget;
    operation(&mut preview, &mut wallet)?;
    let (remaining, available) = match currency {
        Currency::Coin => (wallet.coin, actor.coin),
        Currency::Influence => (wallet.influence, actor.influence),
    };
    let cost = (preview_budget - remaining).max(0.0);
    Ok(PoliticalQuote {
        cost,
        affordable: available + 0.000_001 >= cost,
    })
}

/// Render a priced action with consistent explanations when disabled or affordable.
fn political_button(
    ui: &mut egui::Ui,
    label: &str,
    currency: Currency,
    quote: Result<PoliticalQuote, PoliticalError>,
    explanation: &str,
) -> bool {
    let name = currency_label(currency);
    let (enabled, tooltip) = match quote {
        Ok(quote) => (
            quote.affordable,
            if quote.affordable {
                format!("{label} · {:.1} {name}\n{explanation}", quote.cost)
            } else {
                format!(
                    "Requires {:.1} {name}; insufficient available funds.\n{explanation}",
                    quote.cost
                )
            },
        ),
        Err(error) => (false, format!("{error}\n{explanation}")),
    };
    let mut clicked = false;
    ui.horizontal(|ui| {
        icon(ui, currency_icon(currency), 20.0).on_hover_text(name);
        clicked = ui
            .add_enabled(enabled, egui::Button::new(label))
            .on_hover_text(&tooltip)
            .on_disabled_hover_text(tooltip)
            .clicked();
    });
    clicked
}

/// Use the same illustrated resource identity in every priced political control.
fn currency_icon(currency: Currency) -> Icon {
    match currency {
        Currency::Coin => Icon::Coin,
        Currency::Influence => Icon::Influence,
    }
}

/// Common currency labels keep configuration-driven prices readable.
fn currency_label(currency: Currency) -> &'static str {
    match currency {
        Currency::Coin => "Coin",
        Currency::Influence => "Influence",
    }
}

/// Display a recurring program's actual routed monthly cost; enabled programs may
/// always be switched off, including after their political route becomes invalid.
fn recurring_support(
    ui: &mut egui::Ui,
    enabled: &mut bool,
    currency: Currency,
    base_cost: f64,
    route: Result<f64, PoliticalError>,
    effect: &str,
) {
    let (cost, valid) = match route {
        Ok(distance) => {
            (format!("{:.1} {}/month", base_cost * distance, currency_label(currency)), true)
        },
        Err(_) => ("No usable route".into(), false),
    };
    ui.horizontal(|ui| {
    icon(ui, currency_icon(currency), 20.0);
    ui.add_enabled(valid || *enabled, egui::Checkbox::new(enabled, effect))
        .on_hover_text(format!("{cost}. Pays once each month if the route is usable and sufficient funds are available. Otherwise skips the month without debt."))
        .on_disabled_hover_text("A usable political connection is required before enabling this program.");
    });
}

/// Map each allowed hostile action to its actual payment resource and distinct effect.
fn interference_details(action: Interference) -> (&'static str, Currency, &'static str) {
    match action {
        Interference::SmearCampaign => ("Smear", Currency::Influence, "Lowers only this rival's Relation. Limited to one smear per actor, rival and province each month. Strong existing friendship resists the action."),
        Interference::FundOpposition => ("Opposition", Currency::Coin, "Reduces this rival's Independent Control at monthly resolution and returns it to Local. You gain no Control. Entrenchment and friendship increase the cost."),
        Interference::UndermineRival => ("Undermine", Currency::Influence, "Reduces this rival's Independent Control at monthly resolution and returns it to Local. You gain no Control."),
        Interference::PoliticalAgitation => ("Agitate", Currency::Influence, "Immediately reduces enemy Vassal Control. At zero the province becomes independent. It does not give you a control share."),
        Interference::FundDissidents => ("Dissidents", Currency::Coin, "Lowers Relation toward the overlord. Poorer relations increase future Vassal Control decay; this does not directly remove Control."),
        Interference::AgitatePopulation => ("Agitate", Currency::Influence, "Reduces Citizen and Plebeian happiness in the rival's owned province. Ownership and Control do not change. The action is attributed to you."),
        Interference::FundUnrest => ("Unrest", Currency::Coin, "Reduces Plebeian happiness in the rival's owned province. Ownership and Control do not change. The action is attributed to you."),
    }
}

/// Global player access and bilateral peace controls, surfaced from owned-province context.
fn military_diplomacy(
    ui: &mut egui::Ui,
    campaign: &mut Campaign,
    province: usize,
    player: usize,
    owner: usize,
) -> Option<String> {
    let mut message = None;
    if owner == player {
        ui.collapsing("Military access", |ui| {
            for guest in 0..campaign.actors.len() {
                if guest == player { continue; }
                let war = campaign.wars[player][guest];
                let mut invited = campaign.invitations[player][guest];
                if ui.add_enabled(!war, egui::Checkbox::new(&mut invited, format!("Invite Player {}", guest + 1)))
                    .on_hover_text("Permit peaceful military entry. Uncheck to revoke future entry; stationed units are not automatically expelled.")
                    .on_disabled_hover_text("Military invitations are unavailable between players at war.").changed() {
                    campaign.invitations[player][guest] = invited;
                    message = Some(format!("Military access for Player {} {}.", guest + 1, if invited { "granted" } else { "revoked" }));
                }
            }
        });
    } else if campaign.wars[player][owner] {
        stat(
            ui,
            Icon::Attack,
            &format!("War · P{}", owner + 1),
            "Military access invitations are suspended during war.",
        );
        let fighting = campaign.military.battles.iter().any(|battle| {
            let player = ForceOwner::Player(player);
            let owner = ForceOwner::Player(owner);
            (battle.attackers.plans.contains_key(&player)
                && battle.defenders.plans.contains_key(&owner))
                || (battle.attackers.plans.contains_key(&owner)
                    && battle.defenders.plans.contains_key(&player))
        });
        if ui.add_enabled(!fighting, egui::Button::new("Make peace"))
            .on_hover_text("End bilateral hostility. Existing ownership remains unchanged. Peaceful entry still requires the owner's invitation.")
            .on_disabled_hover_text("Resolve or retreat from your active battles against this player before making peace.").clicked() {
            campaign.wars[player][owner] = false; campaign.wars[owner][player] = false;
            message = Some(format!("Peace established with Player {}.", owner + 1));
        }
    } else {
        stat(ui, Icon::Trade, if campaign.invitations[owner][player] { "Access granted" } else { "Access closed" }, "Peaceful military entry requires the owner's invitation. Peaceful stationing grants no occupation Control.");
        if ui.button("Declare hostility").on_hover_text("Ends military invitations between both players. Entering hostile territory begins combat; victory captures an owned province immediately.").clicked() {
            campaign.invitations[player][owner] = false; campaign.invitations[owner][player] = false;
            campaign.declare_hostility(player, province);
            message = Some(format!("Hostility declared against Player {}.", owner + 1));
        }
    }
    message
}

/// Read construction identity before and after the building UI to emit its one start event.
fn wonder_under_construction(campaign: &Campaign, province: usize) -> Option<usize> {
    match &campaign.economy.provinces[province].construction {
        Some(ConstructionProject::Wonder(project)) => Some(project.wonder_id),
        _ => None,
    }
}

/// Show private recent history and return a navigation intention after the frame is drawn.
fn recent_notifications(
    ui: &mut egui::Ui,
    campaign: &Campaign,
    player: usize,
) -> Option<(NoticeAction, Option<usize>)> {
    let mut navigation = None;
    ui.separator();
    ui.collapsing("Recent notifications", |ui| {
        let history: Vec<_> = campaign.notifications.history_for(player).take(30).collect();
        if history.is_empty() {
            ui.label("No recent campaign notifications.");
        }
        for notice in history {
            ui.push_id(notice.id, |ui| {
                let severity = match notice.severity {
                    NoticeSeverity::Info => "Info",
                    NoticeSeverity::Warning => "Warning",
                };
                ui.horizontal_wrapped(|ui| {
                    ui.small(format!("Month {} · {severity}", notice.month));
                    if ui.link(&notice.title).on_hover_text(&notice.body).clicked() {
                        navigation = Some((notice.action, notice.province));
                    }
                });
                ui.label(&notice.body);
                ui.add_space(3.0);
            });
        }
    });
    navigation
}

#[cfg(test)]
#[path = "../../tests/unit/campaign_ui.rs"]
mod tests;
