//! National route ledger, agreement composer, and immediate open market.

use super::campaign::Campaign;
use super::campaign_widgets::{
    icon, network_action_button, paint_icon, paint_purchase_background, portrait, sestertius_unit,
    space_after_portrait, territorial_distance_badge, Icon, ProvinceLandscape,
};
use super::province_panel::{INK, PAPER, RULE, TABLE_STRIPE};
use super::{campaign_economy, policy_widgets};
use crate::game::economy::*;
use bevy_egui::egui;

const SYMBOLS: [Icon; 5] = [Icon::Food, Icon::Metal, Icon::Stone, Icon::Coin, Icon::Influence];
const NAMES: [&str; 5] = ["Food", "Metal", "Stone", "Sestertius", "Influence"];
const MUTED: egui::Color32 = egui::Color32::from_rgb(112, 91, 71);

#[cfg(test)]
#[path = "../../tests/unit/province_trade_ui.rs"]
mod province_tests;

#[derive(Clone)]
struct TradeView {
    page: usize,
    partner: Option<TradeParty>,
    monthly: bool,
    give: [f64; 5],
    receive: [f64; 5],
    market_side: MarketSide,
    resource: usize,
    quantity: f64,
}

impl Default for TradeView {
    fn default() -> Self {
        Self {
            page: 0,
            partner: None,
            monthly: true,
            give: [0.0; 5],
            receive: [0.0; 5],
            market_side: MarketSide::Buy,
            resource: 0,
            quantity: 20.0,
        }
    }
}

fn party_name(world: &EconomyWorld, party: TradeParty) -> String {
    match party {
        TradeParty::Player(id) => format!("Player {}", id + 1),
        TradeParty::Npc(id) => world.provinces[id].name.clone(),
    }
}

fn bundle(values: [f64; 5]) -> TradeBundle {
    TradeBundle {
        resources: [values[0], values[1], values[2]],
        coin: values[3],
        influence: values[4],
    }
}

fn frame<R>(ui: &mut egui::Ui, scale: f32, render: impl FnOnce(&mut egui::Ui) -> R) -> R {
    inset_frame(ui, scale, 10.0, render)
}

fn compact_frame<R>(ui: &mut egui::Ui, scale: f32, render: impl FnOnce(&mut egui::Ui) -> R) -> R {
    inset_frame(ui, scale, 4.0, render)
}

fn inset_frame<R>(
    ui: &mut egui::Ui,
    scale: f32,
    inset: f32,
    render: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let margin = (inset * scale).round() as i8;
    let width = ui.available_width() - 2.0 * f32::from(margin) - 1.6 * scale;
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(247, 243, 232))
        .stroke(egui::Stroke::new(0.8 * scale, RULE))
        .corner_radius(2.0 * scale)
        .inner_margin(egui::Margin::same(margin))
        .show(ui, |ui| {
            ui.set_width(width.max(1.0));
            ui.spacing_mut().item_spacing.y = 5.0 * scale;
            render(ui)
        })
        .inner
}

fn section(ui: &mut egui::Ui, title: &str, scale: f32) {
    policy_widgets::section(ui, scale, title);
}

fn accent(ui: &egui::Ui) -> egui::Color32 {
    ui.ctx()
        .data(|data| data.get_temp::<egui::Color32>(egui::Id::new("campaign-owner-color")))
        .unwrap_or(egui::Color32::from_rgb(155, 77, 52))
}

fn card_title(ui: &mut egui::Ui, title: &str, scale: f32) {
    ui.label(egui::RichText::new(title).size(14.5 * scale).color(INK));
}

fn route_title(ui: &mut egui::Ui, name: &str, width: f32, scale: f32) {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, 30.0 * scale), egui::Sense::hover());
    campaign_economy::ledger_text(ui, rect, name, 14.5 * scale, INK, egui::Align::Min);
    response.on_hover_text(name);
}

/// Use the same radio chips as Governance and province policies.
fn choices<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    title: &str,
    selected: &mut T,
    options: &[(T, &str, &str)],
    scale: f32,
) {
    let gap = 6.0 * scale;
    let (row, _) = ui
        .allocate_exact_size(egui::vec2(ui.available_width(), 29.0 * scale), egui::Sense::hover());
    let width = (row.width() - gap * (options.len() - 1) as f32) / options.len() as f32;
    for (index, &(candidate, label, tip)) in options.iter().enumerate() {
        let chip = egui::Rect::from_min_size(
            row.min + egui::vec2(index as f32 * (width + gap), 0.0),
            egui::vec2(width, row.height()),
        );
        let (_, response) = policy_widgets::choice(
            ui,
            chip,
            ui.id().with((title, index)),
            title,
            label,
            selected,
            candidate,
            accent(ui),
            scale,
        );
        response.on_hover_text(tip);
    }
}

/// Icon tabs match the province inspector's selection, hover and pressed states.
fn icon_tabs(
    ui: &mut egui::Ui,
    selected: &mut usize,
    options: &[(Icon, &str)],
    id: &str,
    scale: f32,
) {
    let gap = 6.0 * scale;
    let (row, _) = ui
        .allocate_exact_size(egui::vec2(ui.available_width(), 47.0 * scale), egui::Sense::hover());
    let width = (row.width() - gap * (options.len() - 1) as f32) / options.len() as f32;
    for (index, &(symbol, label)) in options.iter().enumerate() {
        let rect = egui::Rect::from_min_size(
            row.min + egui::vec2(index as f32 * (width + gap), 0.0),
            egui::vec2(width, row.height()),
        );
        let response = ui.interact(rect, ui.id().with((id, index)), egui::Sense::click());
        if response.clicked() {
            *selected = index;
        }
        let active = *selected == index;
        let pressed = response.is_pointer_button_down_on();
        if active || response.hovered() || pressed {
            let fill = if pressed {
                egui::Color32::from_rgb(199, 163, 111)
            } else if active && response.hovered() {
                egui::Color32::from_rgb(211, 185, 145)
            } else if active {
                egui::Color32::from_rgb(219, 200, 168)
            } else {
                egui::Color32::from_rgb(231, 213, 181)
            };
            ui.painter().rect_filled(rect, 3.0 * scale, fill);
            ui.painter().rect_stroke(
                rect,
                3.0 * scale,
                egui::Stroke::new(
                    scale,
                    if active {
                        accent(ui)
                    } else {
                        RULE
                    },
                ),
                egui::StrokeKind::Inside,
            );
        }
        paint_icon(
            ui,
            symbol,
            egui::Rect::from_center_size(
                rect.center_top() + egui::vec2(0.0, 15.0 * scale),
                egui::Vec2::splat(26.0 * scale),
            ),
        );
        campaign_economy::ledger_text(
            ui,
            egui::Rect::from_min_max(
                rect.min + egui::vec2(3.0 * scale, 29.0 * scale),
                rect.max - egui::vec2(3.0 * scale, 0.0),
            ),
            label,
            11.0 * scale,
            INK,
            egui::Align::Center,
        );
        if active {
            ui.painter().hline(
                rect.x_range(),
                rect.bottom(),
                egui::Stroke::new(3.0 * scale, accent(ui)),
            );
        }
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::SelectableLabel,
                ui.is_enabled(),
                active,
                label,
            )
        });
        response.on_hover_text(label).on_hover_cursor(egui::CursorIcon::PointingHand);
    }
}

fn badges(ui: &mut egui::Ui, values: &[(Icon, &str, String, String)], scale: f32) {
    let gap = 6.0 * scale;
    let (row, _) = ui
        .allocate_exact_size(egui::vec2(ui.available_width(), 44.0 * scale), egui::Sense::hover());
    let width = (row.width() - gap * (values.len() - 1) as f32) / values.len() as f32;
    for (index, (symbol, caption, value, tip)) in values.iter().enumerate() {
        let rect = egui::Rect::from_min_size(
            row.min + egui::vec2(index as f32 * (width + gap), 0.0),
            egui::vec2(width, row.height()),
        );
        campaign_economy::overview_badge(ui, rect, *symbol, caption, value, tip, INK, scale);
    }
}

fn primary_action(
    ui: &mut egui::Ui,
    symbol: Icon,
    title: &str,
    tip: &str,
    enabled: bool,
    scale: f32,
) -> bool {
    ui.add_enabled_ui(enabled, |ui| {
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), 36.0 * scale),
            egui::Sense::click(),
        );
        paint_purchase_background(
            ui,
            rect,
            enabled,
            response.hovered(),
            response.is_pointer_button_down_on(),
            3.0 * scale,
            scale,
        );
        ui.painter().rect_stroke(
            rect,
            3.0 * scale,
            egui::Stroke::new(scale, RULE),
            egui::StrokeKind::Inside,
        );
        paint_icon(
            ui,
            symbol,
            egui::Rect::from_center_size(
                rect.left_center() + egui::vec2(20.0 * scale, 0.0),
                egui::Vec2::splat(25.0 * scale),
            ),
        );
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            title,
            egui::FontId::proportional(14.0 * scale),
            if enabled {
                INK
            } else {
                MUTED
            },
        );
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, title));
        response
            .on_hover_text(tip)
            .on_disabled_hover_text(tip)
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
    })
    .inner
}

fn action(
    ui: &mut egui::Ui,
    symbol: Icon,
    label: &str,
    tip: &str,
    enabled: bool,
    scale: f32,
) -> bool {
    ui.add_enabled_ui(enabled, |ui| {
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(30.0, 30.0) * scale, egui::Sense::click());
        ui.painter().rect_filled(
            rect,
            3.0 * scale,
            if response.is_pointer_button_down_on() {
                ui.visuals().widgets.active.bg_fill
            } else if response.hovered() {
                TABLE_STRIPE
            } else {
                PAPER
            },
        );
        ui.painter().rect_stroke(
            rect,
            3.0 * scale,
            egui::Stroke::new(scale, RULE),
            egui::StrokeKind::Inside,
        );
        paint_icon(ui, symbol, rect.shrink(3.0 * scale));
        if !enabled {
            ui.painter().rect_filled(rect, 3.0 * scale, PAPER.gamma_multiply(0.55));
        }
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
        response
            .on_hover_text(tip)
            .on_disabled_hover_text(tip)
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
    })
    .inner
}

fn amount_text(ui: &egui::Ui, rect: egui::Rect, text: &str, scale: f32) {
    let mut size = 11.0 * scale;
    let galley = loop {
        let galley =
            ui.painter().layout_no_wrap(text.to_owned(), egui::FontId::proportional(size), INK);
        if galley.size().x <= rect.width() || size <= 6.0 * scale {
            break galley;
        }
        size *= 0.9;
    };
    ui.painter().galley(rect.center() - galley.size() * 0.5, galley, INK);
}

fn bundle_line(ui: &mut egui::Ui, label: &str, value: TradeBundle, scale: f32) {
    let values =
        [value.resources[0], value.resources[1], value.resources[2], value.coin, value.influence];
    let shown: Vec<_> = values.into_iter().enumerate().filter(|(_, v)| *v > 0.0).collect();
    let (rect, _) = ui
        .allocate_exact_size(egui::vec2(ui.available_width(), 28.0 * scale), egui::Sense::hover());
    ui.painter().text(
        rect.left_center(),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(12.0 * scale),
        MUTED,
    );
    let start = rect.left() + 65.0 * scale;
    if shown.is_empty() {
        ui.painter().text(
            egui::pos2(start, rect.center().y),
            egui::Align2::LEFT_CENTER,
            "—",
            egui::FontId::proportional(12.0 * scale),
            MUTED,
        );
    }
    let width = (rect.right() - start) / shown.len().max(1) as f32;
    for (index, (resource, amount)) in shown.iter().enumerate() {
        let cell = egui::Rect::from_min_size(
            egui::pos2(start + index as f32 * width, rect.top()),
            egui::vec2(width - 4.0 * scale, rect.height()),
        );
        ui.painter().rect_filled(cell, 3.0 * scale, TABLE_STRIPE);
        let image = egui::Rect::from_center_size(
            cell.left_center() + egui::vec2(12.0 * scale, 0.0),
            egui::vec2(20.0, 20.0) * scale,
        );
        paint_icon(ui, SYMBOLS[*resource], image);
        let text = egui::Rect::from_min_max(cell.min + egui::vec2(25.0 * scale, 0.0), cell.max);
        amount_text(ui, text, &format!("{amount:.1}"), scale);
        ui.interact(cell, ui.id().with((label, resource)), egui::Sense::hover())
            .on_hover_text(NAMES[*resource]);
    }
}

fn quote(
    world: &EconomyWorld,
    agreement: &TradeAgreement,
    inputs: &MonthlyInputs,
) -> Result<TradeQuote, String> {
    // A human proposal must not act as a read-only oracle for the other player's wallet.
    // Both parties' actual funds are validated when an accepted exchange executes.
    if agreement.frequency == TradeFrequency::OneTime
        && matches!(agreement.party_b, TradeParty::Npc(_))
    {
        world.quote_trade_execution(agreement, inputs)
    } else {
        world.quote_trade(agreement, inputs)
    }
}

/// This panel is scoped to the viewing nation, independently of selected map provinces.
pub(in crate::app) fn show(
    ui: &mut egui::Ui,
    campaign: &mut Campaign,
    player: usize,
    scale: f32,
) -> Option<String> {
    let key = egui::Id::new(("national-trade-view", player));
    let portrait = portrait(ui, ProvinceLandscape::Trade, 76.0 * scale);
    space_after_portrait(ui, portrait, scale, 0.0);
    let mut view = ui.ctx().data_mut(|data| data.get_temp::<TradeView>(key)).unwrap_or_default();
    let result = page(ui, campaign, player, scale, &mut view, None);
    ui.ctx().data_mut(|data| data.insert_temp(key, view));
    result
}

/// Reuse national agreement rules with the selected province's current owner.
pub(in crate::app) fn show_province(
    ui: &mut egui::Ui,
    campaign: &mut Campaign,
    province: usize,
    player: usize,
    scale: f32,
) -> Option<String> {
    let selected = campaign.economy.provinces.get(province)?;
    if selected.name == "Rome" {
        return None;
    }
    if selected.owner == Some(player) {
        return None;
    }
    let partner = selected.owner.map_or(TradeParty::Npc(province), TradeParty::Player);
    let key = egui::Id::new(("province-trade-view", player, province));
    let mut view = ui.ctx().data_mut(|data| data.get_temp::<TradeView>(key)).unwrap_or_default();
    view.partner = Some(partner);
    let result = ui
        .push_id(key, |ui| {
            let portrait = portrait(ui, ProvinceLandscape::Trade, 76.0 * scale);
            territorial_distance_badge(
                ui,
                portrait,
                campaign.distance(player, province),
                scale,
                "Territorial distance multiplies the trade costs.",
            );
            space_after_portrait(ui, portrait, scale, 3.0);
            province_network(ui, campaign, player, scale, &mut view, partner)
        })
        .inner;
    ui.ctx().data_mut(|data| data.insert_temp(key, view));
    result
}

/// A live route or pending offer replaces the province's new-agreement form.
fn province_network(
    ui: &mut egui::Ui,
    campaign: &mut Campaign,
    player: usize,
    scale: f32,
    view: &mut TradeView,
    partner: TradeParty,
) -> Option<String> {
    section(ui, "TRADE NETWORK", scale);
    let trades: Vec<_> = campaign
        .economy
        .trades
        .iter()
        .filter(|trade| {
            (trade.party_a == TradeParty::Player(player)
                || trade.party_b == TradeParty::Player(player))
                && (trade.party_a == partner || trade.party_b == partner)
                && matches!(
                    trade.status,
                    TradeStatus::Active | TradeStatus::Suspended | TradeStatus::Proposed
                )
        })
        .cloned()
        .collect();
    if trades.is_empty() {
        return composer(ui, campaign, player, scale, view, Some(partner), None);
    }
    let mut message = None;
    for trade in trades {
        let result = ui.push_id(trade.id, |ui| network_trade(ui, campaign, player, scale, &trade));
        message = result.inner.or(message);
        ui.add_space(7.0 * scale);
    }
    message
}

/// Match the active spy network's unboxed heading, action buttons and summary badges.
fn network_trade(
    ui: &mut egui::Ui,
    campaign: &mut Campaign,
    player: usize,
    scale: f32,
    trade: &TradeAgreement,
) -> Option<String> {
    let pending = trade.status == TradeStatus::Proposed;
    let frequency = if trade.frequency == TradeFrequency::Monthly {
        "Monthly"
    } else {
        "Single-time"
    };
    let title = format!(
        "{frequency} {}",
        if pending {
            "offer"
        } else {
            "trade"
        }
    );
    let mut message = None;
    ui.horizontal(|ui| {
        icon(ui, Icon::Trade, 20.0 * scale);
        let width = (ui.available_width() - 2.0 * (98.0 * scale + ui.spacing().item_spacing.x)).max(1.0);
        route_title(ui, &title, width, scale);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if pending {
                let invited = trade.party_b == TradeParty::Player(player);
                if network_action_button(ui, Icon::Cancel, if invited { "Decline" } else { "Withdraw" }, true, scale)
                    .on_hover_text("Decline or withdraw this pending offer without game penalties.")
                    .clicked()
                {
                    message = Some(campaign.end_trade(player, trade.id, false).unwrap_or_else(|error| error));
                }
                let preview = quote(&campaign.economy, trade, &campaign.inputs());
                let tip = preview.as_ref().err().map(String::as_str).unwrap_or(if invited {
                    "Accept this offer. One-time goods exchange immediately; monthly delivery starts next month."
                } else {
                    "Waiting for the other player's acceptance."
                });
                if network_action_button(ui, Icon::Confirm, "Accept", invited && preview.is_ok(), scale)
                    .on_hover_text(tip)
                    .on_disabled_hover_text(tip)
                    .clicked()
                {
                    message = Some(campaign.accept_national_trade(player, trade.id).unwrap_or_else(|error| error));
                }
            } else {
                let tip = format!("Stop now. Relation with this partner −{:.0}, Influence −up to {:.0}, and Merchant Senate confidence drops immediately.", campaign.economy.config.trade.cancellation_relation_penalty, campaign.economy.config.trade.cancellation_influence_penalty);
                if network_action_button(ui, Icon::Cancel, "Stop now", true, scale)
                    .on_hover_text(tip)
                    .clicked()
                {
                    message = Some(campaign.end_trade(player, trade.id, false).unwrap_or_else(|error| error));
                }
                let (label, tip) = trade.cancellation_month.map_or_else(
                    || ("Stop".to_owned(), format!("Give {} months' notice. Trading continues during notice, then ends without penalties.", campaign.economy.config.trade.cancellation_notice_months)),
                    |due| {
                        let months = due.saturating_sub(campaign.economy.month);
                        (format!("{months} {}", if months == 1 { "month" } else { "months" }), format!("Notice already given. This trade ends after {months} more months without penalties."))
                    },
                );
                if network_action_button(ui, Icon::Notice, &label, trade.cancellation_month.is_none(), scale)
                    .on_hover_text(&tip)
                    .on_disabled_hover_text(tip)
                    .clicked()
                {
                    message = Some(campaign.end_trade(player, trade.id, true).unwrap_or_else(|error| error));
                }
            }
        });
    });
    ui.label(
        egui::RichText::new(format!("#{} · {frequency}", trade.id)).size(12.0 * scale).color(MUTED),
    );
    let path = campaign.economy.trade_route(trade.party_a, trade.party_b, &campaign.inputs());
    let efficiency = path.as_ref().map_or(0.0, |route| campaign.economy.route_efficiency(route));
    let route_tip = path
        .map(|route| {
            route
                .iter()
                .map(|&id| campaign.economy.provinces[id].name.as_str())
                .collect::<Vec<_>>()
                .join(" → ")
        })
        .unwrap_or_else(|reason| format!("Route blocked: {reason}"));
    let status = if pending {
        "Pending"
    } else if trade.status == TradeStatus::Suspended {
        "Suspended"
    } else {
        "Active"
    };
    let status_tip = trade.last_failure.as_deref().unwrap_or(if pending {
        "This offer awaits the other player's acceptance."
    } else {
        "Both parties send the displayed amounts each month. Route losses reduce receipts."
    });
    let gap = 6.0 * scale;
    let (row, _) = ui
        .allocate_exact_size(egui::vec2(ui.available_width(), 48.0 * scale), egui::Sense::hover());
    let width = (row.width() - gap) * 0.5;
    for (index, (symbol, caption, value, tip)) in [
        (Icon::Duration, "Status", status.to_owned(), status_tip),
        (Icon::Trade, "Delivery", format!("{:.0}%", efficiency * 100.0), route_tip.as_str()),
    ]
    .into_iter()
    .enumerate()
    {
        let rect = egui::Rect::from_min_size(
            row.min + egui::vec2(index as f32 * (width + gap), 0.0),
            egui::vec2(width, row.height()),
        );
        campaign_economy::overview_badge(
            ui,
            rect,
            symbol,
            caption,
            &value,
            tip,
            if trade.status == TradeStatus::Suspended {
                super::resource_hud::hud_delta_color(-1.0)
            } else {
                INK
            },
            scale,
        );
    }
    let (outgoing, incoming) = if trade.party_a == TradeParty::Player(player) {
        (trade.a_gives, trade.b_gives)
    } else {
        (trade.b_gives, trade.a_gives)
    };
    bundle_line(ui, "Send", outgoing, scale);
    bundle_line(
        ui,
        "Receive",
        incoming.scaled(if pending {
            1.0
        } else {
            efficiency
        }),
        scale,
    );
    message
}

#[cfg(test)]
pub(in crate::app) fn select_page(ctx: &egui::Context, player: usize, page: usize) {
    let key = egui::Id::new(("national-trade-view", player));
    ctx.data_mut(|data| {
        let mut view = data.get_temp::<TradeView>(key).unwrap_or_default();
        view.page = page;
        data.insert_temp(key, view);
    });
}

/// Opening Trade from the menu or an alert starts at the route overview and keeps draft amounts.
pub(in crate::app) fn open_routes(ctx: &egui::Context, player: usize) {
    let key = egui::Id::new(("national-trade-view", player));
    ctx.data_mut(|data| {
        let mut view = data.get_temp::<TradeView>(key).unwrap_or_default();
        view.page = 0;
        data.insert_temp(key, view);
    });
}

fn page(
    ui: &mut egui::Ui,
    campaign: &mut Campaign,
    player: usize,
    scale: f32,
    view: &mut TradeView,
    partner: Option<TradeParty>,
) -> Option<String> {
    let pages: &[_] = if partner.is_some() {
        &[(Icon::Trade, "Trade routes"), (Icon::Confirm, "New agreement")]
    } else {
        &[
            (Icon::Trade, "Trade routes"),
            (Icon::Confirm, "New agreement"),
            (Icon::Building(BuildingType::UrbanMarket), "Open market"),
        ]
    };
    icon_tabs(ui, &mut view.page, pages, "trade-pages", scale);
    ui.add_space(8.0 * scale);
    match view.page {
        1 => composer(ui, campaign, player, scale, view, partner, Some("NEW TRADE AGREEMENT")),
        2 if partner.is_none() => market(ui, campaign, player, scale, view),
        _ => routes(ui, campaign, player, scale, partner),
    }
}

fn routes(
    ui: &mut egui::Ui,
    campaign: &mut Campaign,
    player: usize,
    scale: f32,
    partner: Option<TradeParty>,
) -> Option<String> {
    let trades: Vec<_> = campaign
        .economy
        .trades
        .iter()
        .filter(|t| {
            (t.party_a == TradeParty::Player(player) || t.party_b == TradeParty::Player(player))
                && partner.is_none_or(|partner| t.party_a == partner || t.party_b == partner)
        })
        .cloned()
        .collect();
    let active: Vec<_> = trades
        .iter()
        .filter(|t| matches!(t.status, TradeStatus::Active | TradeStatus::Suspended))
        .collect();
    let heading = format!("OPEN ROUTES · {}", active.len());
    section(ui, &heading, scale);
    let mut result = None;
    if active.is_empty() {
        frame(ui, scale, |ui| {
            ui.horizontal(|ui| {
                icon(ui, Icon::Trade, 28.0 * scale);
                ui.vertical(|ui| {
                    card_title(ui, "No open routes", scale);
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new("Choose New agreement to start trading.")
                                .size(12.0 * scale)
                                .color(MUTED),
                        )
                        .wrap(),
                    );
                });
            });
        });
    }
    for trade in active {
        ui.push_id(trade.id, |ui| {
            let outgoing = if trade.party_a == TradeParty::Player(player) { trade.a_gives } else { trade.b_gives };
            let incoming = if trade.party_a == TradeParty::Player(player) { trade.b_gives } else { trade.a_gives };
            let partner = if trade.party_a == TradeParty::Player(player) { trade.party_b } else { trade.party_a };
            frame(ui, scale, |ui| {
                ui.horizontal(|ui| {
                    icon(ui, Icon::Trade, 25.0 * scale);
                    let label_width = (ui.available_width() - 2.0 * (30.0 * scale + ui.spacing().item_spacing.x)).max(1.0);
                    route_title(ui, &party_name(&campaign.economy, partner), label_width, scale);
                    let tip = format!("Stop now. Relation with this partner −{:.0}, Influence −up to {:.0}, and Merchant Senate confidence drops immediately.", campaign.economy.config.trade.cancellation_relation_penalty, campaign.economy.config.trade.cancellation_influence_penalty);
                    if action(ui, Icon::Cancel, "Cancel immediately", &tip, true, scale) { result = Some(campaign.end_trade(player, trade.id, false).unwrap_or_else(|e| e)); }
                    let tip = if let Some(due) = trade.cancellation_month { format!("Notice already given. This route ends after {} more months without a relation penalty.", due.saturating_sub(campaign.economy.month)) }
                        else { "Give six months' notice. Trading continues during notice, then ends without penalties.".into() };
                    if action(ui, Icon::Notice, "Give six months notice", &tip, trade.cancellation_month.is_none(), scale) { result = Some(campaign.end_trade(player, trade.id, true).unwrap_or_else(|e| e)); }
                });
                let status = if let Some(due) = trade.cancellation_month { format!("#{} · {:?} · ends in {} months", trade.id, trade.status, due.saturating_sub(campaign.economy.month)) }
                    else { format!("#{} · {:?} · monthly", trade.id, trade.status) };
                let status_color = if trade.status == TradeStatus::Suspended { super::resource_hud::hud_delta_color(-1.0) } else { MUTED };
                ui.label(egui::RichText::new(status).size(12.0 * scale).color(status_color)).on_hover_text(trade.last_failure.as_deref().unwrap_or("Both parties send the displayed amounts each month. Route losses reduce receipts."));
                let inputs = campaign.inputs();
                let path = campaign.economy.trade_route(trade.party_a, trade.party_b, &inputs);
                let efficiency = path.as_ref().map_or(0.0, |route| campaign.economy.route_efficiency(route));
                bundle_line(ui, "Send", outgoing, scale);
                bundle_line(ui, "Receive", incoming.scaled(efficiency), scale);
                let text = path.map(|route| format!("{:.0}% arrives · {}", efficiency * 100.0, route.iter().map(|&id| campaign.economy.provinces[id].name.as_str()).collect::<Vec<_>>().join(" → "))).unwrap_or_else(|reason| format!("Route blocked: {reason}"));
                ui.add(egui::Label::new(egui::RichText::new(&text).small()).truncate()).on_hover_text(text);
            });
            ui.add_space(7.0 * scale);
        });
    }
    let pending: Vec<_> = trades.iter().filter(|t| t.status == TradeStatus::Proposed).collect();
    if !pending.is_empty() {
        section(ui, "PENDING OFFERS", scale);
    }
    for trade in pending {
        ui.push_id(("pending", trade.id), |ui| frame(ui, scale, |ui| {
            ui.horizontal(|ui| {
                let partner = if trade.party_a == TradeParty::Player(player) { trade.party_b } else { trade.party_a };
                icon(ui, Icon::Trade, 25.0 * scale);
                let available = (ui.available_width() - 2.0 * (30.0 * scale + ui.spacing().item_spacing.x)).max(1.0);
                route_title(ui, &party_name(&campaign.economy, partner), available, scale);
                let invited = trade.party_b == TradeParty::Player(player);
                let quote = quote(&campaign.economy, trade, &campaign.inputs());
                let tip = quote.as_ref().err().map(String::as_str).unwrap_or(if invited { "Accept this offer. One-time goods exchange immediately; monthly delivery starts next month." } else { "Waiting for the other player's acceptance." });
                if action(ui, Icon::Confirm, "Accept offer", tip, invited && quote.is_ok(), scale) { result = Some(campaign.accept_national_trade(player, trade.id).unwrap_or_else(|e| e)); }
                if action(ui, Icon::Cancel, "Decline or withdraw offer", "Decline or withdraw this pending offer without game penalties.", true, scale) { result = Some(campaign.end_trade(player, trade.id, false).unwrap_or_else(|e| e)); }
            });
            ui.label(egui::RichText::new(format!("Offer #{} · {} · {}", trade.id,
                if trade.frequency == TradeFrequency::Monthly { "Monthly" } else { "Single-time" },
                if trade.party_b == TradeParty::Player(player) { "Awaiting your response" } else { "Awaiting acceptance" },
            )).size(12.0 * scale).color(MUTED));
            let (outgoing, incoming) = if trade.party_a == TradeParty::Player(player) { (trade.a_gives, trade.b_gives) } else { (trade.b_gives, trade.a_gives) };
            bundle_line(ui, "Send", outgoing, scale);
            bundle_line(ui, "Receive", incoming, scale);
        }));
        ui.add_space(7.0 * scale);
    }
    result
}

fn composer(
    ui: &mut egui::Ui,
    campaign: &mut Campaign,
    player: usize,
    scale: f32,
    view: &mut TradeView,
    fixed_partner: Option<TradeParty>,
    heading: Option<&str>,
) -> Option<String> {
    if let Some(heading) = heading {
        section(ui, heading, scale);
    }
    let mut parties: Vec<_> = (0..campaign.economy.players.len())
        .filter(|&id| id != player)
        .map(TradeParty::Player)
        .collect();
    let mut npc: Vec<_> = campaign
        .economy
        .provinces
        .iter()
        .enumerate()
        .filter(|(_, p)| p.owner.is_none() && p.name != "Rome")
        .map(|(id, _)| TradeParty::Npc(id))
        .collect();
    npc.sort_by_key(|&party| party_name(&campaign.economy, party));
    parties.extend(npc);
    if let Some(partner) = fixed_partner {
        view.partner = Some(partner);
    } else if view.partner.is_none_or(|p| !parties.contains(&p)) {
        view.partner = parties.first().copied();
    }
    let Some(mut partner) = view.partner else {
        ui.label("No other trade partners available.");
        return None;
    };
    let mut result = None;
    if fixed_partner.is_none() {
        card_title(ui, "Trading partner", scale);
        egui::ComboBox::from_id_salt("national-trade-partner")
            .selected_text(party_name(&campaign.economy, partner))
            .width((ui.available_width() - 28.0 * scale).max(1.0))
            .show_ui(ui, |ui| {
                for candidate in parties {
                    ui.selectable_value(
                        &mut partner,
                        candidate,
                        party_name(&campaign.economy, candidate),
                    );
                }
            });
        ui.add_space(4.0 * scale);
    }
    view.partner = Some(partner);
    let once_tip = format!("Exchange once. NPC partners require {:.0}% more value than monthly routes. Player offers use agreed terms.", (campaign.economy.config.trade.one_time_margin - 1.0) * 100.0);
    choices(
        ui,
        "Frequency",
        &mut view.monthly,
        &[
            (
                true,
                "Monthly",
                "Both parties exchange these amounts every month, starting next month.",
            ),
            (false, "Single-time", &once_tip),
        ],
        scale,
    );
    ui.add_space(6.0 * scale);
    if let TradeParty::Npc(id) = partner {
        partner_market(ui, campaign, id, player, scale);
        ui.add_space(6.0 * scale);
    }
    compact_frame(ui, scale, |ui| {
        card_title(ui, "Exchange terms", scale);
        ui.spacing_mut().item_spacing.y = 0.0;
        table_headings(
            ui,
            &[
                ("You send", "Amounts you commit before transport losses. Zero means none."),
                ("You receive", "Amounts the partner sends before transport losses. See Arrives below for what reaches you."),
            ],
            scale,
        );
        let columns = if campaign.economy.config.trade.allow_influence {
            5
        } else {
            4
        };
        for (resource, name) in NAMES.iter().enumerate().take(columns) {
            ui.push_id(("terms", resource), |ui| {
                trade_row(ui, 3, 28.0 * scale, resource % 2 == 0, scale, |ui, cells| {
                    resource_icon(ui, cells[0], resource, scale);
                    for (cell, (side, value)) in cells[1..].iter().zip([
                        ("Send", &mut view.give[resource]),
                        ("Receive", &mut view.receive[resource]),
                    ]) {
                        ui.scope_builder(
                            egui::UiBuilder::new()
                                .max_rect(cell.shrink2(egui::vec2(4.0, 2.0) * scale))
                                .layout(egui::Layout::top_down(egui::Align::Center)),
                            |ui| {
                                ui.spacing_mut().interact_size = egui::vec2(82.0, 24.0) * scale;
                                ui.add_sized(
                                    [82.0 * scale, 24.0 * scale],
                                    egui::DragValue::new(value)
                                        .range(0.0..=1_000_000.0)
                                        .speed(1.0)
                                        .max_decimals(1),
                                )
                                .on_hover_text(format!(
                                    "{side} {}. Drag to adjust or click to type an amount.",
                                    name
                                ));
                            },
                        );
                    }
                });
            });
        }
    });
    let agreement = TradeAgreement::new(
        TradeParty::Player(player),
        partner,
        bundle(view.give),
        bundle(view.receive),
        if view.monthly {
            TradeFrequency::Monthly
        } else {
            TradeFrequency::OneTime
        },
    );
    let quote = quote(&campaign.economy, &agreement, &campaign.inputs());
    let available = quote.as_ref().is_ok_and(|q| q.fulfillment + 1e-9 >= 1.0);
    ui.add_space(6.0 * scale);
    compact_frame(ui, scale, |ui| {
        ui.horizontal(|ui| {
            card_title(ui, "Delivery preview", scale);
            if let Ok(q) = &quote {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(format!(
                        "{:.0}% delivery · {:.0}% capacity",
                        q.efficiency * 100.0,
                        q.fulfillment * 100.0,
                    )).size(11.0 * scale).color(MUTED))
                        .on_hover_text("Delivery is the share that survives transport. Capacity is the share this partner can supply; a new agreement requires 100%.");
                });
            }
        });
        match &quote {
            Ok(q) => {
                bundle_line(ui, "Arrives", q.a_receives.scaled(q.fulfillment), scale);
                if !available {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(
                                "Reduce the amounts to fit this partner's market capacity.",
                            )
                            .size(12.0 * scale)
                            .color(super::resource_hud::hud_delta_color(-1.0)),
                        )
                        .wrap(),
                    );
                }
            },
            Err(reason) => {
                ui.add(
                    egui::Label::new(egui::RichText::new(reason).size(12.0 * scale).color(MUTED))
                        .wrap(),
                );
            },
        }
        let title = if matches!(partner, TradeParty::Player(_)) {
            "Send offer"
        } else if view.monthly {
            "Open monthly route"
        } else {
            "Exchange now"
        };
        let tip = quote.as_ref().err().map(String::as_str).unwrap_or(if !available {
            "Reduce the amounts until this agreement has 100% fulfillment."
        } else if matches!(partner, TradeParty::Player(_)) {
            "Send these terms to the other player for acceptance."
        } else if view.monthly {
            "Open this route. The first delivery occurs next month."
        } else {
            "Exchange the displayed amounts immediately."
        });
        if primary_action(ui, Icon::Trade, title, tip, available, scale) {
            result = Some(match campaign.propose_national_trade(agreement) {
                Ok(id) => {
                    view.page = 0;
                    format!(
                        "Agreement #{id} {}.",
                        if matches!(partner, TradeParty::Player(_)) {
                            "sent for acceptance"
                        } else if view.monthly {
                            "opened"
                        } else {
                            "completed"
                        }
                    )
                },
                Err(reason) => reason,
            });
        }
    });
    result
}

/// Keep the resource column icon-sized instead of reserving room for a name.
fn trade_row(
    ui: &mut egui::Ui,
    columns: usize,
    height: f32,
    stripe: bool,
    scale: f32,
    render: impl FnOnce(&mut egui::Ui, &[egui::Rect]),
) {
    let (row, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), height), egui::Sense::hover());
    if stripe {
        ui.painter().rect_filled(row, 3.0, TABLE_STRIPE);
    }
    let icon_width = (30.0 * scale).min(row.width());
    let value_width = (row.width() - icon_width) / (columns - 1) as f32;
    let mut cells = vec![egui::Rect::from_min_size(row.min, egui::vec2(icon_width, height))];
    for index in 1..columns {
        cells.push(egui::Rect::from_min_size(
            row.min + egui::vec2(icon_width + (index - 1) as f32 * value_width, 0.0),
            egui::vec2(value_width, height),
        ));
    }
    render(ui, &cells);
}

fn resource_icon(ui: &mut egui::Ui, cell: egui::Rect, resource: usize, scale: f32) {
    paint_icon(
        ui,
        SYMBOLS[resource],
        egui::Rect::from_center_size(cell.center(), egui::Vec2::splat(24.0 * scale)),
    );
    ui.interact(cell, ui.id().with(("trade-resource", resource)), egui::Sense::hover())
        .on_hover_text(NAMES[resource]);
}

fn table_headings(ui: &mut egui::Ui, headings: &[(&str, &str)], scale: f32) {
    trade_row(ui, headings.len() + 1, 23.0 * scale, false, scale, |ui, cells| {
        for (cell, (heading, tip)) in cells[1..].iter().zip(headings) {
            campaign_economy::ledger_text(
                ui,
                cell.shrink2(egui::vec2(7.0 * scale, 0.0)),
                heading,
                11.5 * scale,
                MUTED,
                egui::Align::Center,
            );
            ui.interact(*cell, ui.id().with(heading), egui::Sense::hover()).on_hover_text(*tip);
        }
    });
}

fn partner_market(
    ui: &mut egui::Ui,
    campaign: &Campaign,
    province: usize,
    player: usize,
    scale: f32,
) {
    let p = &campaign.economy.provinces[province];
    compact_frame(ui, scale, |ui| {
        ui.horizontal(|ui| {
            card_title(ui, "Partner market", scale);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!("{:.0}", p.relation_by_player[player]))
                        .size(12.0 * scale),
                )
                .on_hover_text("Your relation with this trade partner.");
                icon(ui, Icon::Relation, 20.0 * scale).on_hover_text("Relation");
            });
        });
        ui.spacing_mut().item_spacing.y = 0.0;
        table_headings(
            ui,
            &[
                ("Can send", "Units this province has left to export this month. These supply your You receive amounts."),
                ("Wants", "Units this province still wants to import this month. These are resources you can send."),
                ("Value", "Local coin value per unit, used to compare both sides of your offer. Scarce resources are worth more."),
            ],
            scale,
        );
        for resource in 0..3 {
            let m = p.market.resources[resource];
            trade_row(ui, 4, 26.0 * scale, resource % 2 == 0, scale, |ui, cells| {
                resource_icon(ui, cells[0], resource, scale);
                for (cell, (value, tip)) in cells[1..].iter().zip([
                    ((m.export_capacity - m.exported).max(0.0), "Available to export this month"),
                    ((m.import_demand - m.imported).max(0.0), "Remaining import demand this month"),
                    (m.local_unit_value, "Sestertius per unit"),
                ]) {
                    campaign_economy::ledger_text(
                        ui,
                        *cell,
                        &policy_widgets::coin_decimal(value),
                        12.0 * scale,
                        INK,
                        egui::Align::Center,
                    );
                    ui.interact(*cell, ui.id().with((resource, tip)), egui::Sense::hover())
                        .on_hover_text(tip);
                }
            });
        }
        if campaign.administers_province(player, province) {
            ui.add_space(5.0 * scale);
            ui.horizontal(|ui| {
                icon(ui, Icon::Coin, 20.0 * scale);
                ui.small(format!(
                    "Local treasury: {:.1} {}",
                    p.market.coin_treasury,
                    sestertius_unit(p.market.coin_treasury)
                ));
            });
        }
    });
}

fn market(
    ui: &mut egui::Ui,
    campaign: &mut Campaign,
    player: usize,
    scale: f32,
    view: &mut TradeView,
) -> Option<String> {
    section(ui, "OPEN MARKET", scale);
    let mut result = None;
    frame(ui, scale, |ui| {
        card_title(ui, "Immediate exchange", scale);
        choices(
            ui,
            "Market side",
            &mut view.market_side,
            &[
                (MarketSide::Buy, "Buy", "Buy resources with Sestertius; delivery is immediate."),
                (MarketSide::Sell, "Sell", "Sell resources for Sestertius; payment is immediate."),
            ],
            scale,
        );
        ui.add_space(5.0 * scale);
        icon_tabs(
            ui,
            &mut view.resource,
            &[(Icon::Food, "Food"), (Icon::Metal, "Metal"), (Icon::Stone, "Stone")],
            "market-resource",
            scale,
        );
        ui.add_space(5.0 * scale);
        ui.horizontal(|ui| {
            icon(ui, SYMBOLS[view.resource], 25.0 * scale);
            ui.label("Quantity");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_sized(
                    [ui.available_width().min(160.0 * scale), 28.0 * scale],
                    egui::DragValue::new(&mut view.quantity)
                        .range(0.0..=1_000_000.0)
                        .speed(1.0)
                        .max_decimals(1),
                );
            });
        });
    });
    ui.add_space(8.0 * scale);
    frame(ui, scale, |ui| {
        card_title(ui, "Your reserves", scale);
        let wallet = &campaign.economy.players[player];
        badges(
            ui,
            &[
                (
                    SYMBOLS[view.resource],
                    "Stock / capacity",
                    format!(
                        "{} / {}",
                        policy_widgets::compact_decimal(wallet.resources[view.resource]),
                        policy_widgets::compact_decimal(wallet.storage[view.resource])
                    ),
                    "Current stock and storage capacity for the selected resource.".into(),
                ),
                (
                    Icon::Coin,
                    "Treasury",
                    policy_widgets::compact_decimal(wallet.coin),
                    format!(
                        "{:.1} {} available for trade.",
                        wallet.coin,
                        sestertius_unit(wallet.coin)
                    ),
                ),
            ],
            scale,
        );
    });
    ui.add_space(8.0 * scale);
    frame(ui, scale, |ui| {
        card_title(ui, "Exchange preview", scale);
        let quote = campaign.economy.quote_open_market(
            player,
            view.resource,
            view.market_side,
            view.quantity,
        );
        match &quote {
            Ok(q) => {
                badges(
                    ui,
                    &[
                        (
                            Icon::Coin,
                            "Per unit",
                            format!("{:.3}", q.unit_price),
                            format!("{} per unit.", sestertius_unit(q.unit_price)),
                        ),
                        (
                            Icon::Coin,
                            if view.market_side == MarketSide::Buy {
                                "Total cost"
                            } else {
                                "Total payment"
                            },
                            format!("{:.1}", q.coin),
                            format!("{} for the full exchange.", sestertius_unit(q.coin)),
                        ),
                    ],
                    scale,
                );
                let mut goods = TradeBundle::default();
                goods.resources[view.resource] = q.quantity;
                let payment = TradeBundle {
                    coin: q.coin,
                    ..Default::default()
                };
                let (sent, received) = if view.market_side == MarketSide::Buy {
                    (payment, goods)
                } else {
                    (goods, payment)
                };
                bundle_line(ui, "Send", sent, scale);
                bundle_line(ui, "Receive", received, scale);
                ui.label(egui::RichText::new(format!("{:.1} units already {} this month", q.prior_volume, if view.market_side == MarketSide::Buy { "bought" } else { "sold" })).size(12.0 * scale).color(MUTED))
                    .on_hover_text("Larger transactions get worse unit prices. Prices also account for your previous exchanges this month.");
            },
            Err(reason) => {
                ui.add(
                    egui::Label::new(egui::RichText::new(reason).size(12.0 * scale).color(MUTED))
                        .wrap(),
                );
            },
        }
        let title = if view.market_side == MarketSide::Buy {
            "Buy now"
        } else {
            "Sell now"
        };
        let tip = quote
            .as_ref()
            .err()
            .map(String::as_str)
            .unwrap_or("Complete this exchange immediately at the displayed price.");
        if primary_action(ui, Icon::Trade, title, tip, quote.is_ok(), scale) {
            result = Some(
                match campaign.economy.exchange_open_market(
                    player,
                    view.resource,
                    view.market_side,
                    view.quantity,
                ) {
                    Ok(q) => {
                        campaign.pull_wallets();
                        format!(
                            "{} {:.1} {} for {:.1} {}.",
                            if view.market_side == MarketSide::Buy {
                                "Bought"
                            } else {
                                "Sold"
                            },
                            q.quantity,
                            NAMES[view.resource],
                            q.coin,
                            sestertius_unit(q.coin)
                        )
                    },
                    Err(reason) => reason,
                },
            );
        }
    });
    result
}
