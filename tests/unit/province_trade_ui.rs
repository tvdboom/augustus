//! Province trade uses the shared composer and filters routes to its live partner.

use super::*;
use crate::game::military::{MilitaryProvince, MilitaryTerrain};
use crate::game::politics::diplomacy::ProvincePolitics;
use crate::game::politics::{PoliticalPlayer, PoliticalRank};

#[test]
fn human_trade_previews_do_not_probe_foreign_wallets() {
    let mut campaign = fixture();
    let agreement = TradeAgreement::new(
        TradeParty::Player(0),
        TradeParty::Player(1),
        TradeBundle {
            coin: 10.0,
            ..Default::default()
        },
        TradeBundle {
            resources: [20.0, 0.0, 0.0],
            ..Default::default()
        },
        TradeFrequency::OneTime,
    );
    campaign.economy.players[1].resources = [0.0; 3];
    campaign.economy.players[1].coin = 0.0;
    let poor = quote(&campaign.economy, &agreement, &campaign.inputs()).unwrap();
    campaign.economy.players[1].resources = [1_000_000.0; 3];
    campaign.economy.players[1].coin = 1_000_000.0;
    let rich = quote(&campaign.economy, &agreement, &campaign.inputs()).unwrap();
    assert_eq!(poor.a_receives, rich.a_receives);
    assert_eq!(poor.efficiency, rich.efficiency);
    assert_eq!(poor.fulfillment, rich.fulfillment);
}

#[test]
fn active_route_cards_align_send_and_receive_columns_without_route_metadata() {
    for scale in [0.5, 1.0] {
        for viewer_is_sender in [true, false] {
            let ctx = form_context();
            let mut campaign = fixture();
            let mut trade = TradeAgreement::new(
                if viewer_is_sender {
                    TradeParty::Player(0)
                } else {
                    TradeParty::Npc(2)
                },
                if viewer_is_sender {
                    TradeParty::Npc(2)
                } else {
                    TradeParty::Player(0)
                },
                TradeBundle {
                    resources: [11.0, 13.0, 17.0],
                    coin: 19.0,
                    ..Default::default()
                },
                TradeBundle {
                    resources: [23.0, 29.0, 31.0],
                    coin: 37.0,
                    ..Default::default()
                },
                TradeFrequency::Monthly,
            );
            trade.id = 42;
            trade.status = TradeStatus::Active;
            let path = campaign
                .economy
                .trade_route(trade.party_a, trade.party_b, &campaign.inputs())
                .unwrap();
            let efficiency = campaign.economy.route_efficiency(&path);
            let (sent, received) = if viewer_is_sender {
                (trade.a_gives, trade.b_gives.scaled(efficiency))
            } else {
                (trade.b_gives, trade.a_gives.scaled(efficiency))
            };
            campaign.economy.trades.push(trade);
            let mut capture = crate::egui_capture::Capture::default();
            let mut render_card = |time, events| {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1000.0, 900.0),
                        )),
                        time: Some(time),
                        events,
                        ..Default::default()
                    },
                    |root| {
                        root.scope_builder(
                            egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(546.0, 900.0) * scale,
                            )),
                            |ui| {
                                *ui.style_mut() = super::super::campaign_widgets::map_style(scale);
                                ui.set_width(546.0 * scale);
                                routes(ui, &mut campaign, 0, scale, None);
                            },
                        );
                    },
                );
                capture.frame(
                    &ctx,
                    &output,
                    &format!("active-route-card-{scale:.2}-{viewer_is_sender}"),
                );
                output.textures_delta.clear();
                output
            };
            render_card(0.0, vec![]);
            let output = render_card(0.1, vec![]);
            let text = labels(&output);
            for removed in ["#42", "Active", "monthly", "arrives", "→"] {
                assert!(!text.contains(removed), "Unexpected {removed}: {text}");
            }
            let send_heading = text_position(&output, "You send");
            let receive_heading = text_position(&output, "You receive");
            assert!(send_heading.x < receive_heading.x);
            assert!((send_heading.y - receive_heading.y).abs() < 0.1);
            let mut previous_y = send_heading.y;
            for (sent, received) in sent
                .resources
                .into_iter()
                .chain([sent.coin])
                .zip(received.resources.into_iter().chain([received.coin]))
            {
                let sent = text_position(&output, &format_statistic(sent));
                let received = text_position(&output, &format_statistic(received));
                assert!(sent.x < received.x);
                assert!((sent.y - received.y).abs() < 0.1);
                assert!(sent.y > previous_y, "Resources must precede coin");
                previous_y = sent.y;
            }
            let name_position = text_position(&output, "Achaia");
            render_card(0.2, vec![egui::Event::PointerMoved(name_position)]);
            let hovered = render_card(1.0, vec![egui::Event::PointerMoved(name_position)]);
            assert_eq!(labels(&hovered).matches("Achaia").count(), 1);
        }
    }
}

#[test]
fn province_trade_aligns_whole_amount_badges_without_metadata_or_amount_tooltips() {
    for (width, scale) in [(546.0, 1.0), (380.0, 1.0), (380.0, 0.85)] {
        let ctx = form_context();
        let mut campaign = fixture();
        let mut trade = TradeAgreement::new(
            TradeParty::Player(0),
            TradeParty::Npc(2),
            TradeBundle {
                resources: [7.0, 5.0, 0.0],
                coin: 11.0,
                ..Default::default()
            },
            TradeBundle {
                resources: [15.0, 0.0, 0.0],
                coin: 23.0,
                ..Default::default()
            },
            TradeFrequency::Monthly,
        );
        trade.id = 1;
        trade.status = TradeStatus::Active;
        campaign.economy.trades.push(trade);
        let mut capture = crate::egui_capture::Capture::default();
        let mut render_network = |time, events| {
            let (mut output, used) = form_frame(&ctx, &mut campaign, width, scale, time, events);
            capture.frame(&ctx, &output, &format!("compact-trade-network-{width:.0}-{scale:.2}"));
            output.textures_delta.clear();
            assert!(used.width() <= width + 1.0);
            (output, used)
        };
        render_network(0.0, vec![]);
        let (output, _) = render_network(0.1, vec![]);
        let send = text_position(&output, "You send");
        let receive = text_position(&output, "You receive");
        assert!(send.x < receive.x);
        assert!((send.y - receive.y).abs() < 0.1);
        let text = labels(&output);
        for removed in ["#1", "Status", "Delivery", "100%", "Active"] {
            assert!(!text.contains(removed), "Unexpected {removed}: {text}");
        }
        assert!(!text.contains(".0"), "Trade summary amounts use whole numbers");
        assert!((text_position(&output, "7").y - text_position(&output, "15").y).abs() < 0.1);
        assert!((text_position(&output, "11").y - text_position(&output, "23").y).abs() < 0.1);
        for amount in ["7", "5", "11", "15", "23"] {
            let position = text_position(&output, amount);
            let badge = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Rect(rect)
                        if rect.fill == TABLE_STRIPE
                            && rect.rect.contains(position)
                            && (rect.rect.height() - 36.0 * scale).abs() < 0.1 =>
                    {
                        Some(rect.rect)
                    },
                    _ => None,
                })
                .expect("Amount must sit in a compact resource badge");
            assert!(badge.width() <= 153.0 * scale && badge.width() >= 145.0 * scale);
            assert!(position.x - badge.left() < 56.0 * scale, "Keep the number close to its icon");
        }
        for (index, amount) in ["7", "5", "15"].into_iter().enumerate() {
            let pointer = text_position(&output, amount);
            let time = 1.0 + index as f64 * 2.0;
            render_network(time, vec![egui::Event::PointerMoved(pointer)]);
            let (hovered, _) = render_network(time + 1.0, vec![]);
            let text = labels(&hovered);
            assert!(!text
                .lines()
                .any(|line| ["Food", "Metal", "Status", "Delivery"].contains(&line)));
        }
    }
}

fn fixture() -> Campaign {
    let mut provinces: Vec<_> = ["Italia", "Africa", "Achaia", "Cappadocia"]
        .into_iter()
        .map(|name| {
            EconomicProvince::new(
                name,
                60.0,
                Terrain::Farmland,
                true,
                [1.4, 0.8, 0.6],
                [8.0, 22.0, 33.0, 37.0],
                2,
            )
        })
        .collect();
    provinces[0].owner = Some(0);
    provinces[1].owner = Some(1);
    let graph = vec![vec![1, 2, 3], vec![0], vec![0], vec![0]];
    let mut campaign = Campaign {
        economy: EconomyWorld::new(2, provinces, graph.clone()),
        ..Default::default()
    };
    for wallet in &mut campaign.economy.players {
        wallet.coin = 5000.0;
        wallet.resources = [2000.0; 3];
    }
    campaign.actors = vec![
        PoliticalPlayer {
            coin: 5000.0,
            influence: 1000.0,
            rank: PoliticalRank::Aedile,
            ..Default::default()
        };
        2
    ];
    campaign.politics = vec![
        ProvincePolitics::owned(2, 0),
        ProvincePolitics::owned(2, 1),
        ProvincePolitics::independent(2),
        ProvincePolitics::independent(2),
    ];
    campaign.graph = graph
        .into_iter()
        .map(|neighbors| MilitaryProvince {
            terrain: MilitaryTerrain::Plains,
            area: 100.0,
            road_level: 0,
            neighbors,
        })
        .collect();
    campaign.wars = vec![vec![false; 2]; 2];
    campaign.npc_wars = vec![vec![false; 4]; 2];
    campaign.economy.refresh_npc_markets(&campaign.inputs());
    campaign
}

fn render(
    ctx: &egui::Context,
    campaign: &mut Campaign,
    province: usize,
    time: f64,
    events: Vec<egui::Event>,
) -> (egui::FullOutput, Option<String>) {
    let mut message = None;
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000.0, 900.0),
            )),
            time: Some(time),
            events,
            ..Default::default()
        },
        |root| {
            egui::CentralPanel::default().show(root, |ui| {
                *ui.style_mut() = super::super::campaign_widgets::map_style(1.0);
                ui.set_width(546.0);
                message = show_province(ui, campaign, province, 0, 1.0);
            });
        },
    );
    output.textures_delta.clear();
    (output, message)
}

fn text_position(output: &egui::FullOutput, label: &str) -> egui::Pos2 {
    output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == label => {
                Some(text.galley.rect.translate(text.pos.to_vec2()).center())
            },
            _ => None,
        })
        .unwrap_or_else(|| panic!("Missing {label}"))
}

fn labels(output: &egui::FullOutput) -> String {
    output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn form_context() -> egui::Context {
    let ctx = egui::Context::default();
    ctx.add_font(egui::epaint::text::FontInsert::new(
        "firasans",
        egui::FontData::from_static(include_bytes!("../../assets/fonts/FiraSans-Bold.ttf")),
        vec![egui::epaint::text::InsertFontFamily {
            family: egui::FontFamily::Proportional,
            priority: egui::epaint::text::FontPriority::Highest,
        }],
    ));
    ctx
}

fn form_frame(
    ctx: &egui::Context,
    campaign: &mut Campaign,
    width: f32,
    scale: f32,
    time: f64,
    events: Vec<egui::Event>,
) -> (egui::FullOutput, egui::Rect) {
    let mut used = egui::Rect::NOTHING;
    let output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, 900.0),
            )),
            time: Some(time),
            events,
            ..Default::default()
        },
        |root| {
            root.scope_builder(
                egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 900.0),
                )),
                |ui| {
                    *ui.style_mut() = super::super::campaign_widgets::map_style(scale);
                    ui.set_width(width);
                    show_province(ui, campaign, 2, 0, scale);
                    used = ui.min_rect();
                },
            );
        },
    );
    (output, used)
}

fn market_frame(
    ctx: &egui::Context,
    campaign: &mut Campaign,
    view: &mut TradeView,
    width: f32,
    scale: f32,
    time: f64,
    events: Vec<egui::Event>,
) -> (egui::FullOutput, egui::Rect, Option<String>) {
    let mut used = egui::Rect::NOTHING;
    let mut message = None;
    let output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, 600.0),
            )),
            time: Some(time),
            events,
            ..Default::default()
        },
        |root| {
            root.scope_builder(
                egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 600.0),
                )),
                |ui| {
                    *ui.style_mut() = super::super::campaign_widgets::map_style(scale);
                    ui.set_width(width);
                    message = market(ui, campaign, 0, scale, view);
                    used = ui.min_rect();
                },
            );
        },
    );
    (output, used, message)
}

#[test]
fn open_market_table_shows_three_quantities_storage_and_visible_prices_without_reserve_badges() {
    for (width, scale) in [(546.0, 1.0), (380.0, 1.0), (380.0, 0.85)] {
        let ctx = form_context();
        let mut campaign = fixture();
        campaign.economy.players[0].resources = [100.4, 1499.5, 300.25];
        let mut view = TradeView {
            quantities: [20.0, 30.0, 40.0],
            ..Default::default()
        };
        let prices =
            campaign.economy.price_open_market_order(0, MarketSide::Buy, view.quantities).unwrap();
        let mut capture = crate::egui_capture::Capture::default();
        for frame in 0..2 {
            let (mut output, used, _) = market_frame(
                &ctx,
                &mut campaign,
                &mut view,
                width,
                scale,
                frame as f64 * 0.1,
                vec![],
            );
            capture.frame(&ctx, &output, &format!("open-market-order-{width:.0}-{scale:.2}"));
            output.textures_delta.clear();
            assert!(used.width() <= width + 1.0, "Market exceeds width: {used:?}");
            assert!(used.height() <= 400.0 * scale, "Market must fit without scrolling: {used:?}");
            let text = labels(&output);
            for quantity in ["20", "30", "40"] {
                assert!(text.lines().any(|line| line == quantity), "Missing quantity: {text}");
            }
            for quote in prices.resources.into_iter().flatten() {
                assert!(text.lines().any(|line| line == format_statistic(quote.coin)));
            }
            assert!(text.contains(&format_statistic(prices.coin)));
            assert!(text.contains("Stock / capacity"));
            for resource in 0..3 {
                let wallet = &campaign.economy.players[0];
                assert!(text.contains(&format!(
                    "{} / {}",
                    format_statistic(wallet.resources[resource]),
                    format_statistic(wallet.storage[resource])
                )));
            }
            assert!(text.contains("Total cost"));
            for removed in ["Your reserves", "Treasury", "Exchange preview", "Per unit"] {
                assert!(!text.contains(removed), "Removed badge returned: {text}");
            }
        }
    }
}

#[test]
fn open_market_max_buttons_fill_each_resource_and_complete_the_order() {
    for (width, scale) in [(546.0, 1.0), (380.0, 1.0), (380.0, 0.85)] {
        for side in [MarketSide::Buy, MarketSide::Sell] {
            let ctx = form_context();
            let mut campaign = fixture();
            let storage = campaign.economy.players[0].storage;
            campaign.economy.players[0].coin = 1_000_000.0;
            campaign.economy.players[0].resources = if side == MarketSide::Buy {
                [storage[0] - 0.25, storage[1], 100.4]
            } else {
                [0.25, 0.0, 100.4]
            };
            let before = serde_json::to_value(&campaign.economy).unwrap();
            let maximums = if side == MarketSide::Buy {
                [0.25, 0.0, storage[2] - 100.4]
            } else {
                [0.25, 0.0, 100.4]
            };
            let mut view = TradeView {
                market_side: side,
                ..Default::default()
            };
            let mut expected = [0.0; 3];
            let mut capture = crate::egui_capture::Capture::default();
            let mut time = 0.0;
            for resource in 0..3 {
                let (mut output, used, _) =
                    market_frame(&ctx, &mut campaign, &mut view, width, scale, time, vec![]);
                capture.frame(
                    &ctx,
                    &output,
                    &format!("open-market-max-{side:?}-{width:.0}-{scale:.2}"),
                );
                output.textures_delta.clear();
                assert!(used.width() <= width + 1.0);
                let mut buttons = Vec::new();
                for shape in &output.shapes {
                    if let egui::Shape::Rect(rect) = &shape.shape {
                        if (rect.rect.width() - 36.0 * scale).abs() < 1.0
                            && (18.0 * scale..=30.0 * scale).contains(&rect.rect.height())
                        {
                            buttons.push(rect.rect);
                        }
                    }
                }
                buttons.sort_by(|a, b| a.center().y.total_cmp(&b.center().y));
                buttons.dedup_by(|a, b| a.center().distance(b.center()) < 2.0);
                assert_eq!(buttons.len(), 3, "Every resource needs its own Max button");
                assert!(buttons[resource].left() >= 40.0 * scale, "Max overlaps the resource icon");
                let position = buttons[resource].center();
                for pressed in [true, false] {
                    time += 0.1;
                    let (mut output, _, _) = market_frame(
                        &ctx,
                        &mut campaign,
                        &mut view,
                        width,
                        scale,
                        time,
                        vec![
                            egui::Event::PointerMoved(position),
                            egui::Event::PointerButton {
                                pos: position,
                                button: egui::PointerButton::Primary,
                                pressed,
                                modifiers: egui::Modifiers::NONE,
                            },
                        ],
                    );
                    capture.frame(
                        &ctx,
                        &output,
                        &format!("open-market-max-{side:?}-{width:.0}-{scale:.2}"),
                    );
                    output.textures_delta.clear();
                }
                expected[resource] = maximums[resource];
                assert_eq!(view.quantities, expected, "Max must only fill its own resource");
                assert_eq!(serde_json::to_value(&campaign.economy).unwrap(), before);
                time += 0.1;
            }
            let (mut output, _, _) =
                market_frame(&ctx, &mut campaign, &mut view, width, scale, time, vec![]);
            capture.frame(
                &ctx,
                &output,
                &format!("open-market-max-{side:?}-{width:.0}-{scale:.2}"),
            );
            output.textures_delta.clear();
            let position = text_position(
                &output,
                if side == MarketSide::Buy {
                    "Buy now"
                } else {
                    "Sell now"
                },
            );
            let mut message = None;
            for pressed in [true, false] {
                time += 0.1;
                let (mut output, _, action) = market_frame(
                    &ctx,
                    &mut campaign,
                    &mut view,
                    width,
                    scale,
                    time,
                    vec![
                        egui::Event::PointerMoved(position),
                        egui::Event::PointerButton {
                            pos: position,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
                output.textures_delta.clear();
                message = action;
            }
            assert!(message.is_some(), "The filled order must be executable");
            assert_eq!(
                campaign.economy.players[0].resources,
                if side == MarketSide::Buy {
                    storage
                } else {
                    [0.0; 3]
                }
            );
            assert_eq!(view.quantities, [0.0; 3]);
        }
    }
}

#[test]
fn open_market_clamps_each_quantity_to_remaining_capacity_and_buys_other_resources_together() {
    let ctx = form_context();
    let mut campaign = fixture();
    let storage = campaign.economy.players[0].storage;
    campaign.economy.players[0].resources = [storage[0] - 0.25, 100.0, storage[2]];
    let mut view = TradeView {
        quantities: [20.0, 30.0, 40.0],
        ..Default::default()
    };
    let (mut output, _, _) = market_frame(&ctx, &mut campaign, &mut view, 380.0, 1.0, 0.0, vec![]);
    output.textures_delta.clear();
    assert_eq!(view.quantities, [0.25, 30.0, 0.0]);
    let position = text_position(&output, "Buy now");
    let mut message = None;
    for (time, pressed) in [(0.1, true), (0.2, false)] {
        let (mut output, _, action) = market_frame(
            &ctx,
            &mut campaign,
            &mut view,
            380.0,
            1.0,
            time,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        output.textures_delta.clear();
        message = action;
    }
    let message = message.unwrap();
    assert!(message.contains("Bought"));
    let notice = campaign.notifications.history_for(0).next().unwrap();
    assert_eq!(notice.body, message);
    assert_eq!(notice.kind, super::super::campaign_notifications::NoticeKind::TradeUpdated);
    assert_eq!(notice.action, super::super::campaign_notifications::NoticeAction::OpenMarket);
    assert_eq!(campaign.notifications.history_for(1).count(), 0);
    assert_eq!(campaign.economy.players[0].resources, [storage[0], 130.0, storage[2]]);
    assert_eq!(view.quantities, [0.0; 3]);
}

#[test]
fn trade_history_navigation_opens_routes_or_the_market_and_preserves_drafts() {
    use super::super::campaign_notifications::{
        CampaignNotice, NoticeAction, NoticeKind, NoticeSeverity,
    };
    let ctx = form_context();
    let key = egui::Id::new(("national-trade-view", 0_usize));
    ctx.data_mut(|data| {
        data.insert_temp(
            key,
            TradeView {
                give: [17.0; 5],
                quantities: [23.0; 3],
                ..Default::default()
            },
        )
    });
    let campaign = fixture();
    for (action, page) in [(NoticeAction::OpenMarket, 1), (NoticeAction::OpenTrade(42), 0)] {
        let notice = CampaignNotice {
            id: 1,
            recipient: 0,
            severity: NoticeSeverity::Info,
            title: "Trade update".into(),
            body: "Completed exchange".into(),
            kind: NoticeKind::TradeUpdated,
            province: None,
            building: None,
            wonder: None,
            scandal: None,
            month: 0,
            action,
        };
        open_notice(&ctx, &notice);
        let view = ctx.data(|data| data.get_temp::<TradeView>(key)).unwrap();
        assert_eq!(view.page, page);
        assert_eq!(view.give, [17.0; 5]);
        assert_eq!(view.quantities, [23.0; 3]);
        let mut panel = super::super::campaign_panel::CampaignUi::default();
        let mut detail = super::super::ProvincePanelOpen(None);
        super::super::campaign_panel::open_notification(
            &mut panel,
            &notice,
            &campaign,
            &mut detail,
            &mut crate::map::MapView::default(),
        );
        assert_eq!(panel.open, Some(super::super::campaign_panel::CampaignTab::Trade));
        assert!(detail.0.is_none());
    }
}

#[test]
fn open_market_blocking_reason_appears_only_on_disabled_button_hover_and_cost_stays_visible() {
    let ctx = form_context();
    let mut campaign = fixture();
    campaign.economy.players[0].resources = [100.0; 3];
    campaign.economy.players[0].coin = 1.0;
    let mut view = TradeView {
        quantities: [20.0, 30.0, 40.0],
        ..Default::default()
    };
    let prices =
        campaign.economy.price_open_market_order(0, MarketSide::Buy, view.quantities).unwrap();
    let (mut output, _, _) = market_frame(&ctx, &mut campaign, &mut view, 380.0, 1.0, 0.0, vec![]);
    output.textures_delta.clear();
    let reason = "Not enough sestertii for this purchase";
    assert!(!labels(&output).contains(reason));
    assert!(labels(&output).contains(&format_statistic(prices.coin)));
    let position = text_position(&output, "Buy now");
    let before = campaign.economy.players[0].balances();
    for time in [0.1, 1.0, 2.0] {
        let (mut frame, _, _) = market_frame(
            &ctx,
            &mut campaign,
            &mut view,
            380.0,
            1.0,
            time,
            if time == 0.1 {
                vec![egui::Event::PointerMoved(position)]
            } else {
                vec![]
            },
        );
        frame.textures_delta.clear();
        output = frame;
    }
    assert!(
        labels(&output).contains(reason),
        "Hover must explain the blocked order: {}",
        labels(&output)
    );
    assert_eq!(campaign.economy.players[0].balances(), before);
}

#[test]
fn open_market_selectors_and_enabled_buy_button_have_no_tooltips() {
    let ctx = form_context();
    let mut campaign = fixture();
    campaign.economy.players[0].resources = [100.0; 3];
    let mut view = TradeView {
        quantities: [20.0, 30.0, 40.0],
        ..Default::default()
    };
    let (mut output, _, _) = market_frame(&ctx, &mut campaign, &mut view, 380.0, 1.0, 0.0, vec![]);
    output.textures_delta.clear();
    let text = labels(&output);
    for (index, label) in ["Buy", "Sell", "Buy now"].into_iter().enumerate() {
        let position = text_position(&output, label);
        for offset in [0.1, 1.0, 2.0] {
            let (mut output, _, _) = market_frame(
                &ctx,
                &mut campaign,
                &mut view,
                380.0,
                1.0,
                index as f64 * 3.0 + offset,
                if offset == 0.1 {
                    vec![egui::Event::PointerMoved(position)]
                } else {
                    vec![]
                },
            );
            output.textures_delta.clear();
            assert_eq!(labels(&output), text, "{label} needs no hover tooltip");
        }
    }
}

#[test]
fn partner_market_shows_whole_unit_limits_for_the_selected_frequency() {
    for frequency in [TradeFrequency::Monthly, TradeFrequency::OneTime] {
        let ctx = form_context();
        let mut campaign = fixture();
        for (index, capacity) in [19.27, 73.05, 16.99].into_iter().enumerate() {
            campaign.economy.provinces[2].market.resources[index].export_capacity = capacity;
        }
        campaign.economy.provinces[2].market.resources[0].local_unit_value = 0.85;
        let key = egui::Id::new(("province-trade-view", 0_usize, 2_usize));
        ctx.data_mut(|data| {
            data.insert_temp(
                key,
                TradeView {
                    monthly: frequency == TradeFrequency::Monthly,
                    give: [0.0, 0.0, 0.0, 10.0, 0.0],
                    receive: [2.0, 0.0, 0.0, 0.0, 0.0],
                    ..Default::default()
                },
            )
        });
        let mut capture = crate::egui_capture::Capture::default();
        for frame in 0..2 {
            let (mut output, used) =
                form_frame(&ctx, &mut campaign, 380.0, 1.0, frame as f64 * 0.1, vec![]);
            capture.frame(&ctx, &output, &format!("province-trade-limits-{frequency:?}"));
            output.textures_delta.clear();
            assert!(used.width() <= 381.0);
            assert!(used.height() <= 600.0);
            let text = labels(&output);
            let expected = if frequency == TradeFrequency::Monthly {
                ["19", "73", "16"]
            } else {
                ["57", "219", "50"]
            };
            for amount in expected {
                assert!(text.lines().any(|line| line == amount), "Missing {amount}: {text}");
            }
            for decimal in ["19.27", "73.05", "16.99", "57.81", "219.15", "50.97"] {
                assert!(
                    !text.contains(decimal),
                    "Resource quantities must use whole units: {text}"
                );
            }
            assert!(text.contains("0.85"), "Unit prices must retain their precision");
        }
    }
}

#[test]
fn province_max_buttons_fill_only_the_selected_receipt_for_the_current_frequency() {
    for frequency in [TradeFrequency::Monthly, TradeFrequency::OneTime] {
        for influence in [false, true] {
            let ctx = form_context();
            let mut campaign = fixture();
            campaign.economy.config.trade.allow_influence = influence;
            let market = &mut campaign.economy.provinces[2].market;
            for (resource, capacity) in [19.27, 73.05, 0.25].into_iter().enumerate() {
                market.resources[resource].export_capacity = capacity;
            }
            market.resources[0].exported = 5.0;
            market.coin_treasury = 101.9;
            market.trade_budget = 25.9;
            market.coin_spent = 3.5;
            market.influence_treasury = 3.7;
            let before = serde_json::to_value(&campaign.economy).unwrap();
            let key = egui::Id::new(("province-trade-view", 0_usize, 2_usize));
            let give = [0.0, 0.0, 0.0, 1000.0, 0.0];
            let mut expected = [
                1.0,
                2.0,
                3.0,
                4.0,
                if influence {
                    1.0
                } else {
                    0.0
                },
            ];
            ctx.data_mut(|data| {
                data.insert_temp(
                    key,
                    TradeView {
                        monthly: frequency == TradeFrequency::Monthly,
                        give,
                        receive: expected,
                        ..Default::default()
                    },
                )
            });
            let maximums = if frequency == TradeFrequency::Monthly {
                [14.0, 73.0, 0.0, 22.0, 3.0]
            } else {
                [52.0, 219.0, 0.0, 50.0, 3.0]
            };
            let mut capture = crate::egui_capture::Capture::default();
            let mut time = 0.0;
            for resource in 0..if influence {
                5
            } else {
                4
            } {
                let (mut output, _) = form_frame(&ctx, &mut campaign, 380.0, 1.0, time, vec![]);
                capture.frame(
                    &ctx,
                    &output,
                    &format!("province-trade-max-{frequency:?}-{influence}"),
                );
                output.textures_delta.clear();
                fn maximum_buttons(shape: &egui::Shape, buttons: &mut Vec<egui::Pos2>) {
                    match shape {
                        egui::Shape::Rect(rect)
                            if (28.0..=42.0).contains(&rect.rect.width())
                                && (18.0..=30.0).contains(&rect.rect.height()) =>
                        {
                            buttons.push(rect.rect.center());
                        },
                        egui::Shape::Vec(shapes) => {
                            for shape in shapes {
                                maximum_buttons(shape, buttons);
                            }
                        },
                        _ => {},
                    }
                }
                let mut buttons = Vec::new();
                for shape in &output.shapes {
                    maximum_buttons(&shape.shape, &mut buttons);
                }
                buttons.sort_by(|a, b| a.y.total_cmp(&b.y));
                buttons.dedup_by(|a, b| a.distance(*b) < 2.0);
                assert_eq!(
                    buttons.len(),
                    if influence {
                        5
                    } else {
                        4
                    }
                );
                let position = buttons[resource];
                for pressed in [true, false] {
                    time += 0.1;
                    let (mut output, _) = form_frame(
                        &ctx,
                        &mut campaign,
                        380.0,
                        1.0,
                        time,
                        vec![
                            egui::Event::PointerMoved(position),
                            egui::Event::PointerButton {
                                pos: position,
                                button: egui::PointerButton::Primary,
                                pressed,
                                modifiers: egui::Modifiers::NONE,
                            },
                        ],
                    );
                    capture.frame(
                        &ctx,
                        &output,
                        &format!("province-trade-max-{frequency:?}-{influence}"),
                    );
                    output.textures_delta.clear();
                }
                time += 0.1;
                if maximums[resource] > 0.0 {
                    expected[resource] = maximums[resource];
                }
                let view = ctx.data(|data| data.get_temp::<TradeView>(key).unwrap());
                assert_eq!(view.receive, expected);
                assert_eq!(view.give, give, "Max must only fill the selected receipt");
            }
            assert_eq!(serde_json::to_value(&campaign.economy).unwrap(), before);
        }
    }
}

#[test]
fn maximum_receipts_follow_live_supply_and_frequency_while_manual_amounts_stay_fixed() {
    let ctx = form_context();
    let mut campaign = fixture();
    campaign.economy.config.trade.allow_influence = true;
    let market = &mut campaign.economy.provinces[2].market;
    for (resource, capacity) in [8.9, 12.7, 20.9].into_iter().enumerate() {
        market.resources[resource].export_capacity = capacity;
    }
    market.coin_treasury = 100.5;
    market.trade_budget = 40.9;
    market.influence_treasury = 5.7;
    let key = egui::Id::new(("province-trade-view", 0_usize, 2_usize));
    ctx.data_mut(|data| {
        data.insert_temp(
            key,
            TradeView {
                give: [0.0, 0.0, 0.0, 1000.0, 0.0],
                receive: [8.0, 6.0, 20.0, 40.0, 5.0],
                ..Default::default()
            },
        )
    });
    let mut capture = crate::egui_capture::Capture::default();
    let mut frame = |campaign: &mut Campaign, time| {
        let before = serde_json::to_value(&campaign.economy).unwrap();
        let (mut output, _) = form_frame(&ctx, campaign, 380.0, 1.0, time, vec![]);
        capture.frame(&ctx, &output, "live-maximum-trade");
        output.textures_delta.clear();
        assert_eq!(serde_json::to_value(&campaign.economy).unwrap(), before);
        ctx.data(|data| data.get_temp::<TradeView>(key).unwrap().receive)
    };
    assert_eq!(frame(&mut campaign, 0.0), [8.0, 6.0, 20.0, 40.0, 5.0]);
    let market = &mut campaign.economy.provinces[2].market;
    market.resources[0].export_capacity = 3.2;
    market.resources[1].export_capacity = 9.9;
    market.resources[2].export_capacity = 0.0;
    market.coin_treasury = 11.9;
    market.influence_treasury = 0.0;
    assert_eq!(frame(&mut campaign, 0.1), [3.0, 6.0, 0.0, 11.0, 0.0]);
    let market = &mut campaign.economy.provinces[2].market;
    market.resources[0].export_capacity = 20.8;
    market.resources[1].export_capacity = 20.7;
    market.resources[2].export_capacity = 6.2;
    market.coin_treasury = 80.0;
    market.influence_treasury = 4.9;
    assert_eq!(frame(&mut campaign, 0.2), [20.0, 6.0, 6.0, 40.0, 4.0]);
    ctx.data_mut(|data| {
        let mut view = data.get_temp::<TradeView>(key).unwrap();
        view.monthly = false;
        data.insert_temp(key, view);
    });
    assert_eq!(frame(&mut campaign, 0.3), [62.0, 6.0, 18.0, 40.0, 4.0]);
}

#[test]
fn manually_editing_a_maximum_receipt_stops_live_tracking() {
    let ctx = form_context();
    let mut campaign = fixture();
    campaign.economy.provinces[2].market.resources[0].export_capacity = 8.9;
    let key = egui::Id::new(("province-trade-view", 0_usize, 2_usize));
    ctx.data_mut(|data| {
        data.insert_temp(
            key,
            TradeView {
                give: [0.0, 0.0, 0.0, 1000.0, 0.0],
                receive: [8.0, 0.0, 0.0, 0.0, 0.0],
                ..Default::default()
            },
        )
    });
    let frame = |campaign: &mut Campaign, time, events| {
        let (mut output, _) = form_frame(&ctx, campaign, 380.0, 1.0, time, events);
        output.textures_delta.clear();
        output
    };
    frame(&mut campaign, 0.0, vec![]);
    let output = frame(&mut campaign, 0.1, vec![]);
    let position = output
        .shapes
        .iter()
        .rev()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == "8" => {
                Some(text.galley.rect.translate(text.pos.to_vec2()).center())
            },
            _ => None,
        })
        .expect("Maximum receipt field");
    for (time, pressed) in [(0.2, true), (0.3, false)] {
        frame(
            &mut campaign,
            time,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    frame(&mut campaign, 0.4, vec![egui::Event::Text("4".into())]);
    assert_eq!(ctx.data(|data| data.get_temp::<TradeView>(key).unwrap().receive[0]), 4.0);
    campaign.economy.provinces[2].market.resources[0].export_capacity = 12.9;
    let (mut output, _) = form_frame(&ctx, &mut campaign, 380.0, 1.0, 0.5, vec![]);
    output.textures_delta.clear();
    assert_eq!(ctx.data(|data| data.get_temp::<TradeView>(key).unwrap().receive[0]), 4.0);
}

#[test]
fn province_trade_action_shows_whole_unit_red_reasons_below_the_button_only_when_blocked() {
    let ctx = form_context();
    let mut campaign = fixture();
    let market = &mut campaign.economy.provinces[2].market;
    market.resources[2].import_demand = 5.0;
    market.resources[2].internal_need = 2.0;
    let key = egui::Id::new(("province-trade-view", 0_usize, 2_usize));
    let red = super::super::resource_hud::hud_delta_color(-1.0);
    let mut capture = crate::egui_capture::Capture::default();
    for (case, (stone, payment)) in
        [(8.0, 1.0), (30.0, 1.0), (0.1, 1.0), (7.0, 1.0), (30.0, 40.0)].into_iter().enumerate()
    {
        ctx.data_mut(|data| {
            data.insert_temp(
                key,
                TradeView {
                    give: [0.0, 0.0, stone, 0.0, 0.0],
                    receive: [0.0, 0.0, 0.0, payment, 0.0],
                    ..Default::default()
                },
            )
        });
        let (mut output, _) =
            form_frame(&ctx, &mut campaign, 380.0, 1.0, case as f64 * 0.1, vec![]);
        capture.frame(&ctx, &output, &format!("province-trade-action-{stone:.0}"));
        output.textures_delta.clear();
        let text = labels(&output);
        assert!(!text.contains("To you") && !text.contains("Delivery preview"));
        assert!(!text.contains("7.5 Stone"));
        let button = text_position(&output, "Open monthly route");
        let reason = output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text.contains("offered value") => Some(text),
            _ => None,
        });
        assert!(!text.contains("can accept at most"));
        if stone < 1.0 || payment > 1.0 {
            let reason = reason.expect("An underpriced offer must explain how to improve it");
            if payment > 1.0 {
                assert!(reason.galley.job.text.contains("25%"));
            } else {
                assert!(reason.galley.job.text.contains("Increase what you send"));
                assert!(
                    !reason.galley.job.text.contains("0.1")
                        && !reason.galley.job.text.contains("1.10")
                );
            }
            assert!(reason.pos.y > button.y, "The reason must appear below the action");
            assert!(reason.galley.job.sections.iter().all(|section| section.format.color == red));
        } else {
            assert!(reason.is_none(), "A valid offer must not show a rejection reason");
        }
    }
}

#[test]
fn province_composer_blocks_repeat_single_time_trades_and_shows_remaining_cooldown() {
    let ctx = form_context();
    let mut campaign = fixture();
    campaign
        .propose_national_trade(TradeAgreement::new(
            TradeParty::Player(0),
            TradeParty::Npc(2),
            TradeBundle {
                coin: 10.0,
                ..Default::default()
            },
            TradeBundle {
                resources: [2.0, 0.0, 0.0],
                ..Default::default()
            },
            TradeFrequency::OneTime,
        ))
        .unwrap();
    let key = egui::Id::new(("province-trade-view", 0_usize, 2_usize));
    ctx.data_mut(|data| {
        data.insert_temp(
            key,
            TradeView {
                monthly: false,
                give: [0.0, 0.0, 0.0, 10.0, 0.0],
                receive: [2.0, 0.0, 0.0, 0.0, 0.0],
                ..Default::default()
            },
        )
    });
    render(&ctx, &mut campaign, 2, 0.0, vec![]);
    let (output, _) = render(&ctx, &mut campaign, 2, 0.1, vec![]);
    assert!(labels(&output).contains("Single-time trade available in 3 months"));
    assert!(click_action(&ctx, &mut campaign, 2, "Exchange now", 0.2).is_none());
    assert_eq!(campaign.economy.trades.len(), 1);
    ctx.data_mut(|data| {
        let mut view = data.get_temp::<TradeView>(key).unwrap();
        view.monthly = true;
        view.receive = [0.0, 1.0, 0.0, 0.0, 0.0];
        data.insert_temp(key, view);
    });
    render(&ctx, &mut campaign, 2, 0.45, vec![]);
    render(&ctx, &mut campaign, 2, 0.46, vec![]);
    assert!(click_action(&ctx, &mut campaign, 2, "Open monthly route", 0.5).is_some());
    assert_eq!(campaign.economy.trades.len(), 2);
    assert_eq!(campaign.economy.trades[1].frequency, TradeFrequency::Monthly);
}

#[test]
fn provincial_trade_body_fits_short_panels_and_executes_the_visible_action_only_once() {
    for (width, height) in [(546.0, 480.0), (380.0, 400.0), (380.0, 320.0)] {
        for offered_stone in [0.0, 22.6, 60.0] {
            let ctx = form_context();
            let mut campaign = fixture();
            let key = egui::Id::new(("province-trade-view", 0_usize, 2_usize));
            ctx.data_mut(|data| {
                data.insert_temp(
                    key,
                    TradeView {
                        monthly: false,
                        give: [0.0, 0.0, offered_stone, 10.0, 0.0],
                        receive: [2.0, 0.0, 0.0, 0.0, 0.0],
                        ..Default::default()
                    },
                )
            });
            let mut capture = crate::egui_capture::Capture::default();
            let mut frame = |campaign: &mut Campaign, time, events| {
                let mut used = egui::Rect::NOTHING;
                let mut message = None;
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, height),
                        )),
                        time: Some(time),
                        events,
                        ..Default::default()
                    },
                    |root| {
                        root.scope_builder(
                            egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(width, height),
                            )),
                            |ui| {
                                *ui.style_mut() = super::super::campaign_widgets::map_style(1.0);
                                ui.set_width(width);
                                message = super::super::campaign_panel::province_trade_body(
                                    ui, height, campaign, 2, 0, 1.0,
                                );
                                used = ui.min_rect();
                            },
                        );
                    },
                );
                capture.frame(
                    &ctx,
                    &output,
                    &format!("province-trade-fit-{width:.0}-{height:.0}-{offered_stone:.1}"),
                );
                output.textures_delta.clear();
                assert!(used.right() <= width + 1.0, "Trade content exceeds width: {used:?}");
                assert!(
                    used.bottom() <= height + 1.0,
                    "Trade content still requires scrolling: {used:?}"
                );
                (output, message)
            };
            frame(&mut campaign, 0.0, vec![]);
            let (output, _) = frame(&mut campaign, 0.1, vec![]);
            let text = labels(&output);
            for label in ["TRADE NETWORK", "Partner market", "Exchange terms", "Exchange now"] {
                assert!(text.contains(label), "Missing {label}: {text}");
            }
            assert!(!text.contains("can accept at most"));
            let position = text_position(&output, "Exchange now");
            for (time, pressed) in [(0.2, true), (0.3, false)] {
                frame(
                    &mut campaign,
                    time,
                    vec![
                        egui::Event::PointerMoved(position),
                        egui::Event::PointerButton {
                            pos: position,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
            assert_eq!(campaign.economy.trades.len(), 1);
        }
    }
}

#[test]
fn province_trade_form_fits_without_scrolling_and_uses_icon_resource_rows() {
    for (width, scale) in [(546.0, 1.0), (380.0, 1.0), (380.0, 0.85)] {
        for valid in [false, true] {
            let ctx = form_context();
            let mut campaign = fixture();
            if valid {
                ctx.data_mut(|data| {
                    data.insert_temp(
                        egui::Id::new(("province-trade-view", 0_usize, 2_usize)),
                        TradeView {
                            give: [0.0, 0.0, 0.0, 10.0, 0.0],
                            receive: [2.0, 0.0, 0.0, 0.0, 0.0],
                            ..Default::default()
                        },
                    );
                });
            }
            let mut capture = crate::egui_capture::Capture::default();
            for frame in 0..2 {
                let (mut output, used) =
                    form_frame(&ctx, &mut campaign, width, scale, frame as f64 * 0.1, vec![]);
                capture.frame(
                    &ctx,
                    &output,
                    &format!("province-trade-form-{width:.0}-{scale:.2}-{valid}"),
                );
                output.textures_delta.clear();
                assert!(used.width() <= width + 1.0, "Form exceeds panel width: {used:?}");
                assert!(used.height() <= 600.0 * scale, "Form requires scrolling: {used:?}");
                let text = labels(&output);
                for unwanted in [
                    "Trading partner",
                    "Achaia",
                    "Resource",
                    "Food",
                    "Metal",
                    "Stone",
                    "Sestertius",
                ] {
                    assert!(
                        !text.lines().any(|line| line == unwanted),
                        "Repeated label: {unwanted}"
                    );
                }
                assert!(text.contains("Monthly"));
                assert!(text.contains("Single-time"));
                assert!(text.contains("Can send"));
                assert!(text.contains("Wants"));
            }
        }
    }
}

#[test]
fn province_trade_amounts_remain_compact_and_readable_while_typing() {
    let ctx = form_context();
    let mut campaign = fixture();
    let key = egui::Id::new(("province-trade-view", 0_usize, 2_usize));
    ctx.data_mut(|data| {
        data.insert_temp(
            key,
            TradeView {
                give: [20.0, 0.0, 0.0, 0.0, 0.0],
                ..Default::default()
            },
        )
    });
    let mut capture = crate::egui_capture::Capture::default();
    let (mut output, _) = form_frame(&ctx, &mut campaign, 380.0, 1.0, 0.0, vec![]);
    capture.frame(&ctx, &output, "province-trade-amount-idle");
    output.textures_delta.clear();
    let position = text_position(&output, "20");
    for (time, pressed) in [(0.1, true), (0.2, false)] {
        let (mut output, _) = form_frame(
            &ctx,
            &mut campaign,
            380.0,
            1.0,
            time,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        capture.frame(&ctx, &output, "province-trade-amount-click");
        output.textures_delta.clear();
    }
    assert!(ctx.memory(|memory| memory.focused().is_some()), "Click must enter text editing");
    let (mut output, _) = form_frame(&ctx, &mut campaign, 380.0, 1.0, 0.3, vec![]);
    capture.frame(&ctx, &output, "province-trade-amount-focused");
    output.textures_delta.clear();
    let field = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect) if rect.rect.contains(position) && rect.fill == PAPER => {
                Some(rect.rect)
            },
            _ => None,
        })
        .expect("Focused field must keep a light paper background");
    assert!(field.width() <= 84.0, "Focused amount expanded: {field:?}");
    assert!(field.width() >= 80.0, "Amount field is too narrow: {field:?}");
    let (mut output, _) =
        form_frame(&ctx, &mut campaign, 380.0, 1.0, 0.4, vec![egui::Event::Text("37".into())]);
    capture.frame(&ctx, &output, "province-trade-amount-typed");
    output.textures_delta.clear();
    assert_eq!(ctx.data(|data| data.get_temp::<TradeView>(key).unwrap().give[0]), 37.0);
    assert!(
        output.shapes.iter().any(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == "37" =>
                text.galley.job.sections.iter().all(|section| section.format.color == INK),
            _ => false,
        }),
        "Typed amount must retain dark readable ink"
    );
}

fn click_action(
    ctx: &egui::Context,
    campaign: &mut Campaign,
    province: usize,
    label: &str,
    time: f64,
) -> Option<String> {
    let (output, _) = render(ctx, campaign, province, time, vec![]);
    let position = text_position(&output, label);
    let mut message = None;
    for (offset, pressed) in [(0.1, true), (0.2, false)] {
        message = render(
            ctx,
            campaign,
            province,
            time + offset,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        )
        .1;
    }
    message
}

#[test]
fn province_trade_network_shows_the_form_only_without_a_live_agreement() {
    for status in [
        TradeStatus::Active,
        TradeStatus::Suspended,
        TradeStatus::Proposed,
        TradeStatus::Completed,
        TradeStatus::Cancelled,
    ] {
        let ctx = egui::Context::default();
        let mut campaign = fixture();
        let mut trade = TradeAgreement::new(
            TradeParty::Player(0),
            TradeParty::Player(1),
            TradeBundle {
                coin: 10.0,
                ..Default::default()
            },
            TradeBundle {
                resources: [2.0, 0.0, 0.0],
                ..Default::default()
            },
            TradeFrequency::Monthly,
        );
        trade.status = status;
        campaign.economy.trades.push(trade);
        render(&ctx, &mut campaign, 1, 0.0, vec![]);
        let (output, _) = render(&ctx, &mut campaign, 1, 0.1, vec![]);
        let text = labels(&output);
        assert_eq!(text.matches("TRADE NETWORK").count(), 1);
        let form = matches!(status, TradeStatus::Completed | TradeStatus::Cancelled);
        assert!(!text.contains("Trading partner"), "{status:?}: {text}");
        assert_eq!(text.contains("Exchange terms"), form);
        assert!(
            !text.lines().any(|line| line == "Max"),
            "Human trade partners keep their funds private"
        );
        assert!(!text.contains("No open routes"));
        assert!(!text.contains("NEW AGREEMENT"));
        assert_eq!(
            text.contains("Abort"),
            matches!(status, TradeStatus::Active | TradeStatus::Suspended)
        );
    }
}

#[test]
fn province_trade_stop_gives_notice_then_stop_now_reopens_the_form_and_applies_penalties() {
    let ctx = egui::Context::default();
    let mut campaign = fixture();
    campaign.economy.players[0].influence = 1000.0;
    let id = campaign
        .propose_national_trade(TradeAgreement::new(
            TradeParty::Player(0),
            TradeParty::Npc(2),
            TradeBundle {
                coin: 10.0,
                ..Default::default()
            },
            TradeBundle {
                resources: [2.0, 0.0, 0.0],
                ..Default::default()
            },
            TradeFrequency::Monthly,
        ))
        .unwrap();
    let relation = campaign.politics[2].relation(0);
    render(&ctx, &mut campaign, 2, 0.0, vec![]);
    assert!(click_action(&ctx, &mut campaign, 2, "Close", 0.1).is_some());
    let trade = campaign.economy.trades.iter().find(|trade| trade.id == id).unwrap();
    assert_eq!(trade.status, TradeStatus::Active);
    assert_eq!(trade.cancellation_month, Some(6));
    assert_eq!(campaign.politics[2].relation(0), relation);
    assert_eq!(campaign.actors[0].influence, 1000.0);
    let (output, _) = render(&ctx, &mut campaign, 2, 0.4, vec![]);
    let text = labels(&output);
    assert!(text.contains("6 months"));
    assert!(!text.contains("Trading partner"));
    assert!(click_action(&ctx, &mut campaign, 2, "Abort", 0.5).is_some());
    assert_eq!(campaign.economy.trades[0].status, TradeStatus::Cancelled);
    assert_eq!(
        campaign.politics[2].relation(0),
        relation - campaign.economy.config.trade.cancellation_relation_penalty
    );
    assert_eq!(campaign.actors[0].influence, 990.0);
    let (output, _) = render(&ctx, &mut campaign, 2, 0.8, vec![]);
    assert!(labels(&output).contains("Exchange terms"));
}

#[test]
fn province_pending_offer_can_be_accepted_or_withdrawn_without_a_duplicate_form() {
    for invited in [false, true] {
        let ctx = egui::Context::default();
        let mut campaign = fixture();
        let (sender, recipient) = if invited {
            (1, 0)
        } else {
            (0, 1)
        };
        let id = campaign
            .propose_national_trade(TradeAgreement::new(
                TradeParty::Player(sender),
                TradeParty::Player(recipient),
                TradeBundle {
                    coin: 10.0,
                    ..Default::default()
                },
                TradeBundle {
                    resources: [2.0, 0.0, 0.0],
                    ..Default::default()
                },
                TradeFrequency::Monthly,
            ))
            .unwrap();
        render(&ctx, &mut campaign, 1, 0.0, vec![]);
        let (output, _) = render(&ctx, &mut campaign, 1, 0.1, vec![]);
        assert!(labels(&output).contains("Monthly offer"));
        assert!(!labels(&output).contains("Status"));
        assert!(!labels(&output).contains("Exchange terms"));
        assert!(click_action(
            &ctx,
            &mut campaign,
            1,
            if invited {
                "Accept"
            } else {
                "Withdraw"
            },
            0.2
        )
        .is_some());
        assert_eq!(
            campaign.economy.trades.iter().find(|trade| trade.id == id).unwrap().status,
            if invited {
                TradeStatus::Active
            } else {
                TradeStatus::Cancelled
            }
        );
        let (output, _) = render(&ctx, &mut campaign, 1, 0.5, vec![]);
        assert_eq!(labels(&output).contains("Exchange terms"), !invited);
    }
}

#[test]
fn province_composer_sends_to_its_current_owner_or_npc_and_keeps_national_drafts() {
    for province in [1, 2] {
        let ctx = egui::Context::default();
        let mut campaign = fixture();
        let key = egui::Id::new(("province-trade-view", 0_usize, province));
        let national = egui::Id::new(("national-trade-view", 0_usize));
        ctx.data_mut(|data| {
            data.insert_temp(
                key,
                TradeView {
                    page: 1,
                    monthly: false,
                    partner: Some(TradeParty::Npc(3)),
                    give: [0.0, 0.0, 0.0, 10.0, 0.0],
                    receive: [2.0, 0.0, 0.0, 0.0, 0.0],
                    ..Default::default()
                },
            );
            data.insert_temp(
                national,
                TradeView {
                    give: [17.0; 5],
                    ..Default::default()
                },
            );
        });
        render(&ctx, &mut campaign, province, 0.0, vec![]);
        let (output, _) = render(&ctx, &mut campaign, province, 0.1, vec![]);
        let label = if province == 1 {
            "Send offer"
        } else {
            "Exchange now"
        };
        let position = text_position(&output, label);
        let mut message = None;
        for (time, pressed) in [(0.2, true), (0.3, false)] {
            message = render(
                &ctx,
                &mut campaign,
                province,
                time,
                vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            )
            .1;
        }
        assert!(message.is_some(), "The quoted offer must be actionable");
        assert_eq!(campaign.economy.trades.len(), 1);
        let trade = &campaign.economy.trades[0];
        assert_eq!(
            trade.party_b,
            if province == 1 {
                TradeParty::Player(1)
            } else {
                TradeParty::Npc(2)
            }
        );
        assert_eq!(
            trade.status,
            if province == 1 {
                TradeStatus::Proposed
            } else {
                TradeStatus::Completed
            }
        );
        assert_eq!(ctx.data(|data| data.get_temp::<TradeView>(national).unwrap().give), [17.0; 5]);
    }
}

#[test]
fn province_trade_network_follows_the_selected_partner_and_owner_changes() {
    let ctx = egui::Context::default();
    let mut campaign = fixture();
    for (id, party_a, party_b) in [
        (11, TradeParty::Player(0), TradeParty::Npc(2)),
        (22, TradeParty::Player(0), TradeParty::Npc(3)),
        (33, TradeParty::Player(1), TradeParty::Player(0)),
    ] {
        let mut trade = TradeAgreement::new(
            party_a,
            party_b,
            TradeBundle {
                coin: id as f64,
                ..Default::default()
            },
            TradeBundle {
                resources: [2.0, 0.0, 0.0],
                ..Default::default()
            },
            TradeFrequency::Monthly,
        );
        trade.id = id;
        trade.status = TradeStatus::Active;
        campaign.economy.trades.push(trade);
    }
    let key = egui::Id::new(("province-trade-view", 0_usize, 2_usize));
    ctx.data_mut(|data| data.insert_temp(key, TradeView::default()));
    for owned in [false, true] {
        campaign.economy.provinces[2].owner = owned.then_some(1);
        let (output, _) = render(
            &ctx,
            &mut campaign,
            2,
            if owned {
                0.2
            } else {
                0.0
            },
            vec![],
        );
        let labels = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(labels.matches("TRADE NETWORK").count(), 1);
        assert!(!labels.contains("OPEN ROUTES"));
        assert!(!labels.contains("NEW AGREEMENT"));
        assert!(!labels.contains("Trading partner"));
        assert!(!labels.contains("Exchange terms"));
        assert!(!labels.contains("Trading with"));
        assert!(!labels.contains("Trade routes"));
        assert!(!labels.contains("PENDING OFFERS"));
        assert!(labels.contains("Abort"));
        assert!(labels.contains("Close"));
        assert!(
            labels.contains(if owned {
                "33"
            } else {
                "11"
            }),
            "Missing selected route: {labels}"
        );
        assert!(!labels.lines().any(|label| label == "22"), "Unrelated province must be excluded");
        assert!(!labels.contains(if owned {
            "11"
        } else {
            "33"
        }));
        assert!(!labels.contains("Open market"));
    }
}
