//! National route ledger, agreement composer, and immediate open market.

use super::campaign::Campaign;
use super::campaign_widgets::{
    directory_row_fill, icon, network_action_button, paint_icon, paint_purchase_background,
    portrait, sestertius_unit, space_after_portrait, territorial_distance_badge, Icon,
    ProvinceLandscape,
};
use super::province_panel::{INK, PAPER, RULE, TABLE_STRIPE};
use super::resource_hud::format_statistic;
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
    receive_at_maximum: [bool; 5],
    receive_maximum_partner: Option<TradeParty>,
    market_side: MarketSide,
    quantities: [f64; 3],
}

impl Default for TradeView {
    fn default() -> Self {
        Self {
            page: 0,
            partner: None,
            monthly: true,
            give: [0.0; 5],
            receive: [0.0; 5],
            receive_at_maximum: [false; 5],
            receive_maximum_partner: None,
            market_side: MarketSide::Buy,
            quantities: [0.0; 3],
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

/// Keep trade controls in the player's color, including inside a foreign province.
fn accent(ui: &egui::Ui) -> egui::Color32 {
    ui.ctx()
        .data(|data| data.get_temp::<egui::Color32>(egui::Id::new("campaign-active-player-color")))
        .unwrap_or(egui::Color32::from_rgb(155, 77, 52))
}

fn card_title(ui: &mut egui::Ui, title: &str, scale: f32) {
    ui.label(egui::RichText::new(title).size(14.5 * scale).color(INK));
}

fn route_title(ui: &mut egui::Ui, name: &str, width: f32, scale: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 30.0 * scale), egui::Sense::hover());
    campaign_economy::ledger_text(ui, rect, name, 14.5 * scale, INK, egui::Align::Min);
}

fn abort_trade_tip(campaign: &Campaign) -> String {
    format!(
        "Abort Trade\n• Relation with this partner: −{:.0}\n• Influence: −{:.0}\n• Merchant Senate confidence decreases.",
        campaign.economy.config.trade.cancellation_relation_penalty,
        campaign.economy.config.trade.cancellation_influence_penalty,
    )
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
        if !tip.is_empty() {
            response.on_hover_text(tip);
        }
    }
}

/// Icon tabs match the province inspector's selection, hover and pressed states.
fn icon_tabs(
    ui: &mut egui::Ui,
    selected: &mut usize,
    options: &[(Icon, &str)],
    id: &str,
    label_tooltips: bool,
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
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        if label_tooltips {
            response.on_hover_text(label);
        }
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
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        if tip.is_empty() {
            response.clicked()
        } else {
            response.on_hover_text(tip).on_disabled_hover_text(tip).clicked()
        }
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
    let response = ui
        .add_enabled_ui(enabled, |ui| {
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
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label)
            });
            response
                .on_hover_text(tip)
                .on_disabled_hover_text(tip)
                .on_hover_cursor(egui::CursorIcon::PointingHand)
        })
        .inner;
    block_disabled_action_click(ui, &response, tip);
    response.clicked()
}

/// Disabled controls consume clicks instead of activating the enclosing route card.
fn block_disabled_action_click(ui: &mut egui::Ui, response: &egui::Response, tip: &str) {
    if !response.enabled() {
        ui.interact(response.rect, response.id.with("disabled-action-click"), egui::Sense::CLICK)
            .on_hover_text(tip)
            .on_hover_cursor(egui::CursorIcon::Default);
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

/// Find the smallest whole amount in one offer row that makes the current NPC deal valid.
/// Ask the real quote path so scarcity, surplus valuation, distance and blackmail stay aligned.
fn required_offer_amount(
    campaign: &Campaign,
    player: usize,
    partner: TradeParty,
    give: [f64; 5],
    receive: [f64; 5],
    frequency: TradeFrequency,
    resource: usize,
) -> Result<f64, String> {
    if !matches!(partner, TradeParty::Npc(_)) {
        return Err("Automatic offer amounts require an AI trading partner.".into());
    }
    let inputs = campaign.inputs();
    let accepts = |amount: u32| -> Result<(), String> {
        let mut offered = give;
        offered[resource] = f64::from(amount);
        let agreement = TradeAgreement::new(
            TradeParty::Player(player),
            partner,
            bundle(offered),
            bundle(receive),
            frequency,
        );
        let quote = quote(&campaign.economy, &agreement, &inputs)?;
        if quote.fulfillment + 1e-9 < 1.0 {
            return Err(quote
                .limit_reason
                .unwrap_or_else(|| "This trade requires 100% supply.".into()));
        }
        Ok(())
    };
    if accepts(0).is_ok() {
        return Ok(0.0);
    }
    let mut high = if frequency == TradeFrequency::OneTime {
        let wallet = &campaign.economy.players[player];
        [
            wallet.resources[0],
            wallet.resources[1],
            wallet.resources[2],
            wallet.coin,
            wallet.influence,
        ][resource]
            .floor()
            .clamp(0.0, 1_000_000.0) as u32
    } else {
        1_000_000
    };
    accepts(high)?;
    let mut low = 1;
    while low < high {
        let middle = low + (high - low) / 2;
        if accepts(middle).is_ok() {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    Ok(f64::from(low))
}

fn exchange_columns(
    ui: &mut egui::Ui,
    outgoing: TradeBundle,
    incoming: TradeBundle,
    partner_name: &str,
    scale: f32,
) {
    let amounts = |bundle: TradeBundle| {
        [
            bundle.resources[0],
            bundle.resources[1],
            bundle.resources[2],
            bundle.coin,
            bundle.influence,
        ]
    };
    let sent = amounts(outgoing);
    let received = amounts(incoming);
    let resources = if outgoing.influence > 0.0 || incoming.influence > 0.0 {
        5
    } else {
        4
    };
    let partner_sends = format!("{partner_name} sends");
    ui.columns(2, |columns| {
        for (column, (label, values)) in
            columns.iter_mut().zip([("You send", sent), (partner_sends.as_str(), received)])
        {
            column.spacing_mut().item_spacing.y = 4.0 * scale;
            let (heading, _) = column.allocate_exact_size(
                egui::vec2(column.available_width(), 18.0 * scale),
                egui::Sense::hover(),
            );
            campaign_economy::ledger_text(
                column,
                heading,
                label,
                12.0 * scale,
                INK,
                egui::Align::Min,
            );
            let gap = 4.0 * scale;
            let width = (column.available_width() - gap) * 0.5;
            for first in (0..resources).step_by(2) {
                let (row, _) = column.allocate_exact_size(
                    egui::vec2(column.available_width(), 24.0 * scale),
                    egui::Sense::hover(),
                );
                for resource in first..(first + 2).min(resources) {
                    let amount = values[resource];
                    let cell = egui::Rect::from_min_size(
                        row.min + egui::vec2((resource - first) as f32 * (width + gap), 0.0),
                        egui::vec2(width, row.height()),
                    );
                    column.painter().rect_filled(cell, 3.0 * scale, TABLE_STRIPE);
                    column.painter().rect_stroke(
                        cell,
                        3.0 * scale,
                        egui::Stroke::new(0.65 * scale, RULE),
                        egui::StrokeKind::Inside,
                    );
                    super::campaign_widgets::paint_raster_icon(
                        column,
                        SYMBOLS[resource],
                        egui::Rect::from_center_size(
                            cell.left_center() + egui::vec2(13.0 * scale, 0.0),
                            egui::Vec2::splat(20.0 * scale),
                        ),
                        egui::Color32::from_white_alpha(if amount > 0.0 {
                            255
                        } else {
                            90
                        }),
                    );
                    campaign_economy::ledger_text(
                        column,
                        egui::Rect::from_min_max(
                            cell.min + egui::vec2(27.0 * scale, 0.0),
                            cell.max - egui::vec2(5.0 * scale, 0.0),
                        ),
                        &if amount > 0.0 {
                            format_statistic(amount)
                        } else {
                            "—".to_owned()
                        },
                        12.0 * scale,
                        if amount > 0.0 {
                            INK
                        } else {
                            MUTED
                        },
                        egui::Align::Min,
                    );
                }
            }
        }
    });
}

/// Right-to-left placement keeps Close on the left and Abort on the right.
fn trade_end_actions(
    ui: &mut egui::Ui,
    campaign: &mut Campaign,
    player: usize,
    trade: &TradeAgreement,
    scale: f32,
) -> Option<String> {
    let mut message = None;
    if network_action_button(ui, Icon::Cancel, "Abort", true, scale)
        .on_hover_text(abort_trade_tip(campaign))
        .clicked()
    {
        message = Some(campaign.end_trade(player, trade.id, false).unwrap_or_else(|error| error));
    }
    let (label, tip) = trade.cancellation_month.map_or_else(
        || ("Close".to_owned(), format!("Close Trade\nGive {} months' notice. Trading continues during notice, then ends without penalties.", campaign.economy.config.trade.cancellation_notice_months)),
        |due| {
            let months = due.saturating_sub(campaign.economy.month);
            (format!("{months} {}", if months == 1 { "month" } else { "months" }), format!("Close Trade\nNotice already given. This trade ends after {months} more months without penalties."))
        },
    );
    let close =
        network_action_button(ui, Icon::Notice, &label, trade.cancellation_month.is_none(), scale);
    block_disabled_action_click(ui, &close, &tip);
    if close.on_hover_text(&tip).on_disabled_hover_text(tip).clicked() {
        message = Some(campaign.end_trade(player, trade.id, true).unwrap_or_else(|error| error));
    }
    message
}

const TRADE_PROVINCE_CLICK_ID: &str = "campaign-trade-province-click";

pub(in crate::app) fn take_province_click(ctx: &egui::Context) -> Option<usize> {
    ctx.data_mut(|data| data.remove_temp::<usize>(egui::Id::new(TRADE_PROVINCE_CLICK_ID)))
}

fn party_province(world: &EconomyWorld, partner: TradeParty) -> Option<usize> {
    match partner {
        TradeParty::Npc(province) => world.provinces.get(province).map(|_| province),
        TradeParty::Player(player) => world
            .provinces
            .iter()
            .position(|province| province.owner == Some(player) && province.name != "Rome"),
    }
}

/// The entire route card opens its partner; child action buttons retain their own clicks.
fn route_card(
    ui: &mut egui::Ui,
    campaign: &mut Campaign,
    partner: TradeParty,
    scale: f32,
    render: impl FnOnce(&mut egui::Ui, &mut Campaign),
) {
    let province = party_province(&campaign.economy, partner);
    let margin = (10.0 * scale).round() as i8;
    let width = ui.available_width() - 2.0 * f32::from(margin) - 1.6 * scale;
    let card = ui.scope_builder(egui::UiBuilder::new().sense(egui::Sense::click()), |ui| {
        let mut prepared = egui::Frame::new()
            .fill(egui::Color32::from_rgb(247, 243, 232))
            .stroke(egui::Stroke::new(0.8 * scale, RULE))
            .corner_radius(2.0 * scale)
            .inner_margin(egui::Margin::same(margin))
            .begin(ui);
        prepared.content_ui.set_width(width.max(1.0));
        prepared.content_ui.spacing_mut().item_spacing.y = 5.0 * scale;
        render(&mut prepared.content_ui, campaign);
        prepared.allocate_space(ui);
        prepared
    });
    let mut prepared = card.inner;
    let response = card.response;
    if province.is_some()
        && (response.hovered()
            || response.has_focus()
            || response.is_pointer_button_down_on()
            || response.clicked())
    {
        prepared.frame.fill = directory_row_fill(&response, 0);
    }
    prepared.paint(ui);
    if let Some(province) = province {
        if response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            ui.ctx().data_mut(|data| {
                data.insert_temp(egui::Id::new(TRADE_PROVINCE_CLICK_ID), province);
            });
        }
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
    let mut view = ui.ctx().data_mut(|data| data.get_temp::<TradeView>(key)).unwrap_or_default();
    let result = page(ui, campaign, player, scale, &mut view);
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
                message = trade_end_actions(ui, campaign, player, trade, scale);
            }
        });
    });
    let path = campaign.economy.trade_route(trade.party_a, trade.party_b, &campaign.inputs());
    let efficiency = path.as_ref().map_or(0.0, |route| campaign.economy.route_efficiency(route));
    let (outgoing, incoming) = if trade.party_a == TradeParty::Player(player) {
        (trade.a_gives, trade.b_gives)
    } else {
        (trade.b_gives, trade.a_gives)
    };
    let partner = if trade.party_a == TradeParty::Player(player) {
        trade.party_b
    } else {
        trade.party_a
    };
    exchange_columns(
        ui,
        outgoing,
        incoming.scaled(if pending {
            1.0
        } else {
            efficiency
        }),
        &party_name(&campaign.economy, partner),
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

/// Trade alerts open the route overview while keeping draft amounts.
pub(in crate::app) fn open_routes(ctx: &egui::Context, player: usize) {
    let key = egui::Id::new(("national-trade-view", player));
    ctx.data_mut(|data| {
        let mut view = data.get_temp::<TradeView>(key).unwrap_or_default();
        view.page = 0;
        data.insert_temp(key, view);
    });
}

/// Receipts open the market; agreement updates open routes without clearing drafts.
pub(in crate::app) fn open_notice(
    ctx: &egui::Context,
    notice: &super::campaign_notifications::CampaignNotice,
) {
    open_routes(ctx, notice.recipient);
    if notice.action == super::campaign_notifications::NoticeAction::OpenMarket {
        let key = egui::Id::new(("national-trade-view", notice.recipient));
        ctx.data_mut(|data| {
            let mut view = data.get_temp::<TradeView>(key).unwrap_or_default();
            view.page = 1;
            data.insert_temp(key, view);
        });
    }
}

fn page(
    ui: &mut egui::Ui,
    campaign: &mut Campaign,
    player: usize,
    scale: f32,
    view: &mut TradeView,
) -> Option<String> {
    let pages =
        [(Icon::Trade, "Trade routes"), (Icon::Building(BuildingType::UrbanMarket), "Open market")];
    view.page = view.page.min(pages.len() - 1);
    icon_tabs(ui, &mut view.page, &pages, "trade-pages", false, scale);
    ui.separator();
    let landscape = if view.page == 1 {
        ProvinceLandscape::OpenMarket
    } else {
        ProvinceLandscape::Trade
    };
    let portrait = portrait(ui, landscape, 76.0 * scale);
    space_after_portrait(ui, portrait, scale, 0.0);
    match view.page {
        1 => market(ui, campaign, player, scale, view),
        _ => routes(ui, campaign, player, scale, None),
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
    section(ui, "OPEN ROUTES", scale);
    let mut result = None;
    if active.is_empty() {
        frame(ui, scale, |ui| {
            ui.horizontal(|ui| {
                icon(ui, Icon::Trade, 28.0 * scale);
                ui.vertical(|ui| {
                    card_title(ui, "No open routes", scale);
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new("Select a province to create a trade agreement.")
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
            let outgoing = if trade.party_a == TradeParty::Player(player) {
                trade.a_gives
            } else {
                trade.b_gives
            };
            let incoming = if trade.party_a == TradeParty::Player(player) {
                trade.b_gives
            } else {
                trade.a_gives
            };
            let partner = if trade.party_a == TradeParty::Player(player) {
                trade.party_b
            } else {
                trade.party_a
            };
            route_card(ui, campaign, partner, scale, |ui, campaign| {
                ui.horizontal(|ui| {
                    icon(ui, Icon::Trade, 25.0 * scale);
                    let label_width = (ui.available_width()
                        - 2.0 * (98.0 * scale + ui.spacing().item_spacing.x))
                        .max(1.0);
                    route_title(ui, &party_name(&campaign.economy, partner), label_width, scale);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        result =
                            trade_end_actions(ui, campaign, player, trade, scale).or(result.take());
                    });
                });
                let inputs = campaign.inputs();
                let path = campaign.economy.trade_route(trade.party_a, trade.party_b, &inputs);
                let efficiency =
                    path.as_ref().map_or(0.0, |route| campaign.economy.route_efficiency(route));
                exchange_columns(
                    ui,
                    outgoing,
                    incoming.scaled(efficiency),
                    &party_name(&campaign.economy, partner),
                    scale,
                );
            });
            ui.add_space(7.0 * scale);
        });
    }
    let pending: Vec<_> = trades.iter().filter(|t| t.status == TradeStatus::Proposed).collect();
    if !pending.is_empty() {
        section(ui, "PENDING OFFERS", scale);
    }
    for trade in pending {
        let partner = if trade.party_a == TradeParty::Player(player) {
            trade.party_b
        } else {
            trade.party_a
        };
        ui.push_id(("pending", trade.id), |ui| route_card(ui, campaign, partner, scale, |ui, campaign| {
            ui.horizontal(|ui| {
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
            exchange_columns(ui, outgoing, incoming, &party_name(&campaign.economy, partner), scale);
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
    let partner_name = party_name(&campaign.economy, partner);
    let partner_sends = format!("{partner_name} sends");
    let config = &campaign.economy.config.trade;
    let once_tip = format!("Exchange immediately, then wait {} months before another single-time exchange with this partner. NPC markets allow up to {}× monthly goods and require {:.0}% more value. Player offers use agreed amounts.", config.one_time_cooldown_months, config.one_time_export_multiplier, (config.one_time_margin - 1.0) * 100.0);
    choices(
        ui,
        "Frequency",
        &mut view.monthly,
        &[
            (
                true,
                "Monthly",
                "Smaller, sustainable amounts exchanged every month, starting next month.",
            ),
            (false, "Single-time", &once_tip),
        ],
        scale,
    );
    let frequency = if view.monthly {
        TradeFrequency::Monthly
    } else {
        TradeFrequency::OneTime
    };
    ui.add_space(6.0 * scale);
    if let TradeParty::Npc(id) = partner {
        partner_market(ui, campaign, id, player, frequency, scale);
        ui.add_space(6.0 * scale);
    }
    let maximum_receive = if let TradeParty::Npc(id) = partner {
        let market = &campaign.economy.provinces[id].market;
        Some(
            [
                market.resources[0].available_exports(config, frequency),
                market.resources[1].available_exports(config, frequency),
                market.resources[2].available_exports(config, frequency),
                market.available_coin(config, frequency),
                market.influence_treasury,
            ]
            .map(|amount| amount.floor().clamp(0.0, 1_000_000.0)),
        )
    } else {
        None
    };
    if view.receive_maximum_partner != Some(partner) {
        view.receive_at_maximum = [false; 5];
        view.receive_maximum_partner = Some(partner);
    }
    if let Some(maximums) = maximum_receive {
        for (resource, maximum) in maximums.into_iter().enumerate() {
            if view.receive_at_maximum[resource] {
                view.receive[resource] = maximum;
            } else if maximum > 0.0 && (view.receive[resource] - maximum).abs() < 1e-9 {
                view.receive_at_maximum[resource] = true;
            }
        }
    } else {
        view.receive_at_maximum = [false; 5];
    }
    let mut balance_resource = None;
    let can_balance_offer =
        maximum_receive.is_some() && view.receive.iter().any(|amount| *amount > 0.0);
    compact_frame(ui, scale, |ui| {
        card_title(ui, "Exchange terms", scale);
        ui.spacing_mut().item_spacing.y = 0.0;
        table_headings(ui, &[("You send", ""), (partner_sends.as_str(), "")], scale);
        let columns = if campaign.economy.config.trade.allow_influence {
            5
        } else {
            4
        };
        for resource in 0..columns {
            let at_maximum = &mut view.receive_at_maximum[resource];
            ui.push_id(("terms", resource), |ui| {
                trade_row(ui, 3, 28.0 * scale, resource % 2 == 0, scale, |ui, cells| {
                    resource_icon(ui, cells[0], resource, scale);
                    for (side, (cell, value)) in cells[1..]
                        .iter()
                        .zip([&mut view.give[resource], &mut view.receive[resource]])
                        .enumerate()
                    {
                        ui.scope_builder(
                            egui::UiBuilder::new()
                                .max_rect(cell.shrink2(egui::vec2(4.0, 2.0) * scale))
                                .layout(egui::Layout::right_to_left(egui::Align::Center)),
                            |ui| {
                                ui.spacing_mut().interact_size = egui::vec2(82.0, 24.0) * scale;
                                ui.spacing_mut().item_spacing.x = 4.0 * scale;
                                ui.style_mut().explanation_tooltips = false;
                                let amount = ui.add(
                                    egui::DragValue::new(value)
                                        .range(0.0..=1_000_000.0)
                                        .speed(1.0)
                                        .max_decimals(1),
                                );
                                if side == 0 && maximum_receive.is_some() {
                                    let maximum = maximum_icon_button(
                                        ui,
                                        &format!("Required {}", NAMES[resource]),
                                        can_balance_offer,
                                        false,
                                        scale,
                                    );
                                    if maximum_reset_clicked(ui, &maximum) {
                                        *value = 0.0;
                                        amount.surrender_focus();
                                    } else if maximum.clicked() {
                                        balance_resource = Some(resource);
                                        amount.surrender_focus();
                                    }
                                }
                                if let Some(maximum) = maximum_receive.filter(|_| side == 1) {
                                    let maximum = maximum[resource];
                                    if amount.changed() {
                                        *at_maximum =
                                            maximum > 0.0 && (*value - maximum).abs() < 1e-9;
                                    }
                                    let maximum_button = maximum_icon_button(
                                        ui,
                                        &format!("Maximum {}", NAMES[resource]),
                                        maximum > 0.0,
                                        *at_maximum,
                                        scale,
                                    );
                                    if maximum_reset_clicked(ui, &maximum_button) {
                                        *value = 0.0;
                                        *at_maximum = false;
                                        amount.surrender_focus();
                                    } else if maximum_button.clicked() {
                                        *value = maximum;
                                        *at_maximum = true;
                                        amount.surrender_focus();
                                    }
                                }
                            },
                        );
                    }
                });
            });
        }
        if let Some(resource) = balance_resource {
            match required_offer_amount(
                campaign,
                player,
                partner,
                view.give,
                view.receive,
                frequency,
                resource,
            ) {
                Ok(amount) => view.give[resource] = amount,
                Err(reason) => result = Some(reason),
            }
        }
        let values = if let TradeParty::Npc(id) = partner {
            campaign.economy.provinces[id].market.resources.map(|market| market.local_unit_value)
        } else {
            config.base_value
        };
        let totals = [view.give, view.receive]
            .map(|amounts| bundle(amounts).value(values, config.influence_value));
        let total_tip = if matches!(partner, TradeParty::Npc(_)) {
            "Total value at this province's listed coin prices, before transport losses and the discount on imports above Wants + Reserves. Relation and distance also affect acceptance."
        } else {
            "Total value at base coin prices, before transport losses. Both players choose and accept their own exchange terms."
        };
        trade_row(ui, 3, 27.0 * scale, true, scale, |ui, cells| {
            for (side, (cell, total)) in cells[1..].iter().zip(totals).enumerate() {
                campaign_economy::ledger_text(
                    ui,
                    cell.shrink2(egui::vec2(7.0 * scale, 0.0)),
                    &format!("Total: {} coins", policy_widgets::coin_decimal(total)),
                    12.0 * scale,
                    INK,
                    egui::Align::Max,
                );
                ui.interact(*cell, ui.id().with(("exchange-total", side)), egui::Sense::hover())
                    .on_hover_text(total_tip);
            }
        });
    });
    let agreement = TradeAgreement::new(
        TradeParty::Player(player),
        partner,
        bundle(view.give),
        bundle(view.receive),
        frequency,
    );
    let quote = quote(&campaign.economy, &agreement, &campaign.inputs());
    let available = quote.as_ref().is_ok_and(|q| q.fulfillment + 1e-9 >= 1.0);
    ui.add_space(6.0 * scale);
    compact_frame(ui, scale, |ui| {
        let title = if matches!(partner, TradeParty::Player(_)) {
            "Send offer"
        } else if view.monthly {
            "Open monthly route"
        } else {
            "Exchange now"
        };
        let tip = quote.as_ref().err().map(String::as_str).unwrap_or(if !available {
            quote
                .as_ref()
                .ok()
                .and_then(|q| q.limit_reason.as_deref())
                .unwrap_or("Reduce the amounts until this agreement has 100% fulfillment.")
        } else if matches!(partner, TradeParty::Player(_)) {
            "Send these terms to the other player for acceptance."
        } else if view.monthly {
            ""
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
        if !available {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(tip)
                        .size(12.0 * scale)
                        .color(super::resource_hud::hud_delta_color(-1.0)),
                )
                .wrap(),
            );
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

fn maximum_icon_button(
    ui: &mut egui::Ui,
    label: &str,
    enabled: bool,
    selected: bool,
    scale: f32,
) -> egui::Response {
    ui.add_enabled_ui(enabled, |ui| {
        let response =
            ui.add_sized(egui::vec2(36.0, 24.0) * scale, egui::Button::new("").selected(selected));
        paint_icon(
            ui,
            Icon::Maximum,
            egui::Rect::from_center_size(response.rect.center(), egui::Vec2::splat(18.0 * scale)),
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, response.enabled(), label)
        });
        response
    })
    .inner
}

/// An unavailable maximum can still clear a manually entered amount with a right click.
fn maximum_reset_clicked(ui: &mut egui::Ui, response: &egui::Response) -> bool {
    if response.enabled() {
        response.secondary_clicked()
    } else {
        ui.interact(response.rect, response.id.with("reset-maximum"), egui::Sense::CLICK)
            .secondary_clicked()
    }
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
                egui::Align::Max,
            );
            if !tip.is_empty() {
                ui.interact(*cell, ui.id().with(heading), egui::Sense::hover()).on_hover_text(*tip);
            }
        }
    });
}

fn partner_market(
    ui: &mut egui::Ui,
    campaign: &Campaign,
    province: usize,
    player: usize,
    frequency: TradeFrequency,
    scale: f32,
) {
    let p = &campaign.economy.provinces[province];
    let config = &campaign.economy.config.trade;
    let period = if frequency == TradeFrequency::Monthly {
        "each month"
    } else {
        "in a single-time exchange"
    };
    let export_tip = format!(
        "Whole units {} can send {period}, after this month's deliveries. These supply the {} sends amounts.",
        p.name, p.name,
    );
    let shortage_tip = format!(
        "Whole units of unmet internal need {period}, after local production and this month's delivered imports. Surplus-producing resources have no shortage.",
    );
    let reserve_tip = format!(
        "Additional imports {period} for replenishing reserves, separate from Wants. {} values units beyond Wants + Reserves at {:.0}% of the listed coin price, so surplus needs {:.0}× as many units for the same value.",
        p.name,
        config.surplus_import_value_share.clamp(0.01, 1.0) * 100.0,
        1.0 / config.surplus_import_value_share.clamp(0.01, 1.0),
    );
    compact_frame(ui, scale, |ui| {
        ui.horizontal(|ui| {
            card_title(ui, "Partner market", scale);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let value = ui.label(
                    egui::RichText::new(format!("{:.0}", p.relation_by_player[player]))
                        .size(12.0 * scale),
                );
                let symbol = icon(ui, Icon::Relation, 20.0 * scale);
                ui.interact(
                    value.rect.union(symbol.rect),
                    ui.id().with("partner-relation"),
                    egui::Sense::hover(),
                )
                .on_hover_text(format!("Your relation with {}.", p.name));
            });
        });
        ui.spacing_mut().item_spacing.y = 0.0;
        table_headings(
            ui,
            &[
                ("Can send", export_tip.as_str()),
                ("Wants", shortage_tip.as_str()),
                ("Reserves", reserve_tip.as_str()),
                ("Coins per unit", "Local coin value of one unit, used to compare both sides of your offer. Scarce resources are worth more."),
            ],
            scale,
        );
        for resource in 0..3 {
            let m = p.market.resources[resource];
            trade_row(ui, 5, 26.0 * scale, resource % 2 == 0, scale, |ui, cells| {
                resource_icon(ui, cells[0], resource, scale);
                for (cell, (value, tip)) in cells[1..].iter().zip([
                    (m.available_exports(config, frequency), Some(export_tip.as_str())),
                    (m.remaining_shortage(config, frequency), Some(shortage_tip.as_str())),
                    (m.reserve_demand(config, frequency), Some(reserve_tip.as_str())),
                    (m.local_unit_value, None),
                ]) {
                    let mut text_rect = cell.shrink2(egui::vec2(7.0 * scale, 0.0));
                    if tip.is_none() {
                        let coin_rect = egui::Rect::from_center_size(
                            text_rect.right_center() - egui::vec2(10.0 * scale, 0.0),
                            egui::Vec2::splat(20.0 * scale),
                        );
                        paint_icon(ui, Icon::Coin, coin_rect);
                        text_rect.max.x = coin_rect.left() - 3.0 * scale;
                    }
                    campaign_economy::ledger_text(
                        ui,
                        text_rect,
                        &if tip.is_some() {
                            format!("{:.0}", value.floor())
                        } else {
                            policy_widgets::coin_decimal(value)
                        },
                        12.0 * scale,
                        INK,
                        egui::Align::Max,
                    );
                    if let Some(tip) = tip {
                        ui.interact(*cell, ui.id().with((resource, tip)), egui::Sense::hover())
                            .on_hover_text(tip);
                    }
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
            &[(MarketSide::Buy, "Buy", ""), (MarketSide::Sell, "Sell", "")],
            scale,
        );
        ui.add_space(6.0 * scale);
        ui.spacing_mut().item_spacing.y = 0.0;
        table_headings(ui, &[
            ("Quantity", "Choose amounts independently for Food, Metal, and Stone."),
            ("Stock / capacity", "Current stock and storage capacity for each resource."),
            (if view.market_side == MarketSide::Buy { "Cost" } else { "Payment" }, "Price of this resource at the selected quantity, including this month's price impact."),
        ], scale);
        for resource in 0..3 {
            let wallet = &campaign.economy.players[player];
            let limit = if view.market_side == MarketSide::Buy {
                (wallet.storage[resource] - wallet.resources[resource]).max(0.0)
            } else {
                wallet.resources[resource].max(0.0)
            };
            if !view.quantities[resource].is_finite() {
                view.quantities[resource] = 0.0;
            }
            view.quantities[resource] = view.quantities[resource].clamp(0.0, limit);
            ui.push_id(("market-order", resource), |ui| {
                trade_row(ui, 4, 34.0 * scale, resource % 2 == 0, scale, |ui, cells| {
                    resource_icon(ui, cells[0], resource, scale);
                    ui.scope_builder(
                        egui::UiBuilder::new()
                            .max_rect(cells[1].shrink2(egui::vec2(4.0, 3.0) * scale))
                            .layout(egui::Layout::right_to_left(egui::Align::Center)),
                        |ui| {
                            let amount_width =
                                (ui.available_width() - 40.0 * scale).clamp(1.0, 82.0 * scale);
                            ui.spacing_mut().interact_size = egui::vec2(amount_width, 28.0 * scale);
                            ui.spacing_mut().item_spacing.x = 4.0 * scale;
                            ui.style_mut().explanation_tooltips = false;
                            let amount = ui.add_enabled(
                                limit > 0.0,
                                egui::DragValue::new(&mut view.quantities[resource])
                                    .range(0.0..=limit)
                                    .speed(1.0)
                                    .max_decimals(2),
                            );
                            let (tip, disabled_tip) = if view.market_side == MarketSide::Buy {
                                (
                                    format!(
                                        "Fill the remaining {} storage capacity ({limit:.2}).",
                                        NAMES[resource]
                                    ),
                                    format!(
                                        "There is no remaining {} storage capacity.",
                                        NAMES[resource]
                                    ),
                                )
                            } else {
                                (
                                    format!("Sell all available {} ({limit:.2}).", NAMES[resource]),
                                    format!("There is no {} stock to sell.", NAMES[resource]),
                                )
                            };
                            let maximum = maximum_icon_button(
                                ui,
                                &format!("Maximum {}", NAMES[resource]),
                                limit > 0.0,
                                limit > 0.0 && view.quantities[resource] == limit,
                                scale,
                            )
                            .on_hover_text(tip)
                            .on_disabled_hover_text(disabled_tip);
                            if maximum.secondary_clicked() {
                                view.quantities[resource] = 0.0;
                                amount.surrender_focus();
                            } else if maximum.clicked() {
                                view.quantities[resource] = limit;
                                amount.surrender_focus();
                            }
                        },
                    );
                    let stock = format!(
                        "{} / {}",
                        format_statistic(wallet.resources[resource]),
                        format_statistic(wallet.storage[resource])
                    );
                    campaign_economy::ledger_text(
                        ui,
                        cells[2].shrink2(egui::vec2(5.0 * scale, 0.0)),
                        &stock,
                        12.0 * scale,
                        INK,
                        egui::Align::Max,
                    );
                    let mut quantities = [0.0; 3];
                    quantities[resource] = view.quantities[resource];
                    let cost = campaign
                        .economy
                        .price_open_market_order(player, view.market_side, quantities)
                        .map_or(0.0, |quote| quote.coin);
                    let coin_rect = egui::Rect::from_center_size(
                        cells[3].right_center() - egui::vec2(12.0 * scale, 0.0),
                        egui::Vec2::splat(20.0 * scale),
                    );
                    paint_icon(ui, Icon::Coin, coin_rect);
                    let mut text_rect = cells[3].shrink2(egui::vec2(5.0 * scale, 0.0));
                    text_rect.max.x = coin_rect.left() - 3.0 * scale;
                    campaign_economy::ledger_text(
                        ui,
                        text_rect,
                        &format_statistic(cost),
                        12.0 * scale,
                        INK,
                        egui::Align::Max,
                    );
                });
            });
        }
    });
    ui.add_space(8.0 * scale);
    frame(ui, scale, |ui| {
        let prices =
            campaign.economy.price_open_market_order(player, view.market_side, view.quantities);
        let quote =
            campaign.economy.quote_open_market_order(player, view.market_side, view.quantities);
        ui.horizontal(|ui| {
            card_title(
                ui,
                if view.market_side == MarketSide::Buy {
                    "Total cost"
                } else {
                    "Total payment"
                },
                scale,
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                icon(ui, Icon::Coin, 22.0 * scale);
                ui.label(
                    egui::RichText::new(
                        prices.as_ref().map_or_else(
                            |_| "—".to_owned(),
                            |prices| format_statistic(prices.coin),
                        ),
                    )
                    .size(16.0 * scale)
                    .color(INK),
                );
            });
        });
        let title = if view.market_side == MarketSide::Buy {
            "Buy now"
        } else {
            "Sell now"
        };
        let tip = quote.as_ref().err().map(String::as_str).unwrap_or("");
        if primary_action(ui, Icon::Trade, title, tip, quote.is_ok(), scale) {
            result = Some(
                match campaign.economy.exchange_open_market_order(
                    player,
                    view.market_side,
                    view.quantities,
                ) {
                    Ok(quote) => {
                        campaign.pull_wallets();
                        let goods = quote
                            .resources
                            .iter()
                            .enumerate()
                            .filter_map(|(resource, quote)| {
                                quote.map(|quote| {
                                    format!(
                                        "{} {}",
                                        policy_widgets::compact_decimal(quote.quantity),
                                        NAMES[resource]
                                    )
                                })
                            })
                            .collect::<Vec<_>>()
                            .join(", ");
                        view.quantities = [0.0; 3];
                        let message = format!(
                            "{} {goods} for {} {}.",
                            if view.market_side == MarketSide::Buy {
                                "Bought"
                            } else {
                                "Sold"
                            },
                            format_statistic(quote.coin),
                            sestertius_unit(quote.coin)
                        );
                        campaign.notify_market_trade(player, view.market_side);
                        message
                    },
                    Err(reason) => reason,
                },
            );
        }
    });
    result
}
