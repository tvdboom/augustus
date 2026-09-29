//! National route ledger, agreement composer, and immediate open market.

use super::campaign::Campaign;
use super::campaign_widgets::{
    icon, paint_icon, portrait, sestertius_unit, Icon, ProvinceLandscape,
};
use super::province_panel::{INK, PAPER, RULE, TABLE_STRIPE};
use crate::game::economy::*;
use bevy_egui::egui;

const SYMBOLS: [Icon; 5] = [Icon::Food, Icon::Metal, Icon::Stone, Icon::Coin, Icon::Influence];
const NAMES: [&str; 5] = ["Food", "Metal", "Stone", "Sestertius", "Influence"];

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
    let margin = (9.0 * scale).round() as i8;
    let width = ui.available_width() - 2.0 * f32::from(margin) - 2.0 * scale;
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(247, 243, 232))
        .stroke(egui::Stroke::new(scale, RULE))
        .corner_radius(4.0 * scale)
        .inner_margin(egui::Margin::same(margin))
        .show(ui, |ui| {
            ui.set_width(width.max(1.0));
            render(ui)
        })
        .inner
}

fn section(ui: &mut egui::Ui, title: &str, scale: f32) {
    let (rect, _) = ui
        .allocate_exact_size(egui::vec2(ui.available_width(), 27.0 * scale), egui::Sense::hover());
    ui.painter().rect_filled(rect, 2.0 * scale, egui::Color32::from_rgb(73, 69, 61));
    ui.painter().text(
        rect.left_center() + egui::vec2(10.0 * scale, 0.0),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(11.0 * scale),
        PAPER,
    );
    ui.add_space(7.0 * scale);
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
            if response.hovered() {
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
        egui::FontId::proportional(11.0 * scale),
        INK,
    );
    let start = rect.left() + 55.0 * scale;
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
    if selected.name == "Latium" {
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
            portrait(ui, ProvinceLandscape::Trade, 76.0 * scale);
            ui.add_space(8.0 * scale);
            let route_message = routes(ui, campaign, player, scale, Some(partner), true);
            ui.add_space(8.0 * scale);
            let agreement_message =
                composer(ui, campaign, player, scale, &mut view, Some(partner), "NEW AGREEMENT");
            agreement_message.or(route_message)
        })
        .inner;
    ui.ctx().data_mut(|data| data.insert_temp(key, view));
    result
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
        &["Trade routes", "New agreement"]
    } else {
        &["Trade routes", "New agreement", "Open market"]
    };
    let width = (ui.available_width() - (pages.len() - 1) as f32 * ui.spacing().item_spacing.x)
        / pages.len() as f32;
    ui.horizontal(|ui| {
        for (index, &title) in pages.iter().enumerate() {
            if ui
                .add_sized(
                    [width, 30.0 * scale],
                    egui::Button::new(egui::RichText::new(title).size(12.0 * scale)).fill(
                        if view.page == index {
                            egui::Color32::from_rgb(227, 208, 181)
                        } else {
                            TABLE_STRIPE
                        },
                    ),
                )
                .clicked()
            {
                view.page = index;
            }
        }
    });
    ui.add_space(7.0 * scale);
    match view.page {
        1 => composer(ui, campaign, player, scale, view, partner, "NEW TRADE AGREEMENT"),
        2 if partner.is_none() => market(ui, campaign, player, scale, view),
        _ => routes(ui, campaign, player, scale, partner, false),
    }
}

fn routes(
    ui: &mut egui::Ui,
    campaign: &mut Campaign,
    player: usize,
    scale: f32,
    partner: Option<TradeParty>,
    provincial: bool,
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
    let heading = if provincial {
        "OPEN ROUTES".to_owned()
    } else {
        format!("OPEN ROUTES · {}", active.len())
    };
    section(ui, &heading, scale);
    let mut result = None;
    if active.is_empty() {
        if provincial {
            ui.horizontal(|ui| {
                ui.add_space(6.0 * scale);
                ui.label("No open route with this province.");
            });
        } else {
            ui.label("No open routes. Create an agreement to start trading.");
        }
    }
    for trade in active {
        ui.push_id(trade.id, |ui| {
            let outgoing = if trade.party_a == TradeParty::Player(player) { trade.a_gives } else { trade.b_gives };
            let incoming = if trade.party_a == TradeParty::Player(player) { trade.b_gives } else { trade.a_gives };
            let partner = if trade.party_a == TradeParty::Player(player) { trade.party_b } else { trade.party_a };
            let npc = matches!(partner, TradeParty::Npc(_));
            frame(ui, scale, |ui| {
                ui.horizontal(|ui| {
                    icon(ui, Icon::Trade, 25.0 * scale);
                    let label_width = (ui.available_width() - 2.0 * (30.0 * scale + ui.spacing().item_spacing.x)).max(1.0);
                    ui.add_sized([label_width, 30.0 * scale], egui::Label::new(egui::RichText::new(party_name(&campaign.economy, partner)).strong()).truncate());
                    let tip = if npc { format!("Cancel immediately. Relation with this province falls by {:.0}; delivered goods are retained.", campaign.economy.config.trade.cancellation_relation_penalty) } else { "Cancel immediately. No relation, Control, or other game penalties; delivered goods are retained.".into() };
                    if action(ui, Icon::Cancel, "Cancel immediately", &tip, true, scale) { result = Some(campaign.end_trade(player, trade.id, false).unwrap_or_else(|e| e)); }
                    let tip = if let Some(due) = trade.cancellation_month { format!("Notice already given. This route ends after {} more months without a relation penalty.", due.saturating_sub(campaign.economy.month)) }
                        else if npc { "Give six months' notice. Trading continues during notice, then ends without a relation penalty.".into() }
                        else { "Player agreements can end immediately without any game penalty; notice is unnecessary.".into() };
                    if action(ui, Icon::Notice, "Give six months notice", &tip, npc && trade.cancellation_month.is_none(), scale) { result = Some(campaign.end_trade(player, trade.id, true).unwrap_or_else(|e| e)); }
                });
                let status = if let Some(due) = trade.cancellation_month { format!("#{} · {:?} · ends in {} months", trade.id, trade.status, due.saturating_sub(campaign.economy.month)) }
                    else { format!("#{} · {:?} · monthly", trade.id, trade.status) };
                ui.small(status).on_hover_text(trade.last_failure.as_deref().unwrap_or("Both parties send the displayed amounts each month. Route losses reduce receipts."));
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
    if !pending.is_empty() && !provincial {
        section(ui, "PENDING OFFERS", scale);
    }
    for trade in pending {
        ui.push_id(("pending", trade.id), |ui| frame(ui, scale, |ui| {
            ui.horizontal(|ui| {
                let partner = if trade.party_a == TradeParty::Player(player) { trade.party_b } else { trade.party_a };
                let available = (ui.available_width() - 2.0 * (30.0 * scale + ui.spacing().item_spacing.x)).max(1.0);
                ui.add_sized([available, 30.0 * scale], egui::Label::new(format!("{}#{} · {} · {:?}", if provincial { "Pending offer " } else { "" }, trade.id, party_name(&campaign.economy, partner), trade.frequency)).truncate());
                let invited = trade.party_b == TradeParty::Player(player);
                let quote = quote(&campaign.economy, trade, &campaign.inputs());
                let tip = quote.as_ref().err().map(String::as_str).unwrap_or(if invited { "Accept this offer. One-time goods exchange immediately; monthly delivery starts next month." } else { "Waiting for the other player's acceptance." });
                if action(ui, Icon::Confirm, "Accept offer", tip, invited && quote.is_ok(), scale) { result = Some(campaign.accept_national_trade(player, trade.id).unwrap_or_else(|e| e)); }
                if action(ui, Icon::Cancel, "Decline or withdraw offer", "Decline or withdraw this pending offer without game penalties.", true, scale) { result = Some(campaign.end_trade(player, trade.id, false).unwrap_or_else(|e| e)); }
            });
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
    heading: &str,
) -> Option<String> {
    section(ui, heading, scale);
    let mut parties: Vec<_> = (0..campaign.economy.players.len())
        .filter(|&id| id != player)
        .map(TradeParty::Player)
        .collect();
    let mut npc: Vec<_> = campaign
        .economy
        .provinces
        .iter()
        .enumerate()
        .filter(|(_, p)| p.owner.is_none() && p.name != "Latium")
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
    frame(ui, scale, |ui| {
        if fixed_partner.is_some() {
            ui.strong(party_name(&campaign.economy, partner));
        } else {
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
        }
        view.partner = Some(partner);
        ui.horizontal(|ui| {
            ui.selectable_value(&mut view.monthly, true, "Monthly");
            ui.selectable_value(&mut view.monthly, false, "Single-time");
        });
        ui.add(egui::Label::new(egui::RichText::new(format!("Single-time NPC exchanges require {:.0}% more value than monthly routes. Player offers use agreed terms.", (campaign.economy.config.trade.one_time_margin - 1.0) * 100.0)).small()).wrap());
        if let TradeParty::Npc(id) = partner {
            let p = &campaign.economy.provinces[id];
            ui.small(format!("Relation {:.0}", p.relation_by_player[player]));
            if campaign.administers_province(player, id) {
                ui.small(format!(
                    "Local treasury: {:.1} {}",
                    p.market.coin_treasury,
                    sestertius_unit(p.market.coin_treasury)
                ));
            }
            for (resource, symbol) in SYMBOLS.iter().copied().enumerate().take(3) {
                let m = p.market.resources[resource];
                ui.horizontal(|ui| {
                    icon(ui, symbol, 20.0 * scale);
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(format!(
                                "Offers {:.1} exports / seeks {:.1} imports · {:.2} {}/unit",
                                (m.export_capacity - m.exported).max(0.0),
                                (m.import_demand - m.imported).max(0.0),
                                m.local_unit_value,
                                sestertius_unit(m.local_unit_value)
                            ))
                            .small(),
                        )
                        .wrap(),
                    );
                });
            }
        }
        ui.add_space(5.0 * scale);
        let columns = if campaign.economy.config.trade.allow_influence {
            5
        } else {
            4
        };
        let (header, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), 25.0 * scale),
            egui::Sense::hover(),
        );
        let left = 43.0 * scale;
        let cell_width = (header.width() - left) / 2.0;
        ui.painter().text(
            header.min + egui::vec2(left + cell_width * 0.5, header.height() * 0.5),
            egui::Align2::CENTER_CENTER,
            "You send",
            egui::FontId::proportional(12.0 * scale),
            INK,
        );
        ui.painter().text(
            header.min + egui::vec2(left + cell_width * 1.5, header.height() * 0.5),
            egui::Align2::CENTER_CENTER,
            "You receive",
            egui::FontId::proportional(12.0 * scale),
            INK,
        );
        for resource in 0..columns {
            ui.horizontal(|ui| {
                icon(ui, SYMBOLS[resource], 24.0 * scale).on_hover_text(NAMES[resource]);
                ui.add_space((left - 24.0 * scale - ui.spacing().item_spacing.x).max(0.0));
                let input_width = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0;
                ui.add_sized(
                    [input_width, 26.0 * scale],
                    egui::DragValue::new(&mut view.give[resource])
                        .range(0.0..=1_000_000.0)
                        .speed(1.0),
                );
                ui.add_sized(
                    [input_width, 26.0 * scale],
                    egui::DragValue::new(&mut view.receive[resource])
                        .range(0.0..=1_000_000.0)
                        .speed(1.0),
                );
            });
        }
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
        ui.add_space(7.0 * scale);
        match &quote {
            Ok(q) => {
                ui.small(format!(
                    "{:.0}% arrives · {:.0}% fulfillment",
                    q.efficiency * 100.0,
                    q.fulfillment * 100.0
                ));
                bundle_line(ui, "Arrives", q.a_receives.scaled(q.fulfillment), scale);
            },
            Err(reason) => {
                ui.add(egui::Label::new(egui::RichText::new(reason).small()).wrap());
            },
        }
        let title = if matches!(partner, TradeParty::Player(_)) {
            "Send offer"
        } else if view.monthly {
            "Open monthly route"
        } else {
            "Exchange now"
        };
        if ui.add_enabled(available, egui::Button::new(title)).clicked() {
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
        ui.horizontal(|ui| {
            icon(ui, Icon::Building(BuildingType::UrbanMarket), 28.0 * scale);
            ui.strong("Immediate exchange");
        });
        ui.add(egui::Label::new("Larger transactions get worse unit prices. Several smaller trades are more efficient than one large lump sum.").wrap());
        ui.horizontal(|ui| {
            ui.selectable_value(&mut view.market_side, MarketSide::Buy, "Buy");
            ui.selectable_value(&mut view.market_side, MarketSide::Sell, "Sell");
        });
        egui::ComboBox::from_id_salt("open-market-resource")
            .selected_text(NAMES[view.resource])
            .width((ui.available_width() - 28.0 * scale).max(1.0))
            .show_ui(ui, |ui| {
                for (id, name) in NAMES[..3].iter().enumerate() {
                    ui.selectable_value(&mut view.resource, id, *name);
                }
            });
        ui.horizontal(|ui| {
            icon(ui, SYMBOLS[view.resource], 25.0 * scale);
            ui.label("Quantity");
            ui.add(egui::DragValue::new(&mut view.quantity).range(0.0..=1_000_000.0).speed(1.0));
        });
        let wallet = &campaign.economy.players[player];
        ui.small(format!(
            "Stock {:.1} / {:.1} · Treasury {:.1} {}",
            wallet.resources[view.resource],
            wallet.storage[view.resource],
            wallet.coin,
            sestertius_unit(wallet.coin)
        ));
        let quote = campaign.economy.quote_open_market(
            player,
            view.resource,
            view.market_side,
            view.quantity,
        );
        match &quote {
            Ok(q) => {
                ui.small(format!(
                    "{:.3} {} per unit · {:.1} {} total",
                    q.unit_price,
                    sestertius_unit(q.unit_price),
                    q.coin,
                    sestertius_unit(q.coin)
                ));
                ui.small(format!(
                    "Prices include {:.1} units already {} this month.",
                    q.prior_volume,
                    if view.market_side == MarketSide::Buy {
                        "bought"
                    } else {
                        "sold"
                    }
                ));
            },
            Err(reason) => {
                ui.add(egui::Label::new(egui::RichText::new(reason).small()).wrap());
            },
        }
        let title = if view.market_side == MarketSide::Buy {
            "Buy now"
        } else {
            "Sell now"
        };
        if ui.add_enabled(quote.is_ok(), egui::Button::new(title)).clicked() {
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
