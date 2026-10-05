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
        assert!(!text.contains("No open routes"));
        assert!(!text.contains("NEW AGREEMENT"));
        assert_eq!(
            text.contains("Stop now"),
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
    assert!(click_action(&ctx, &mut campaign, 2, "Stop", 0.1).is_some());
    let trade = campaign.economy.trades.iter().find(|trade| trade.id == id).unwrap();
    assert_eq!(trade.status, TradeStatus::Active);
    assert_eq!(trade.cancellation_month, Some(6));
    assert_eq!(campaign.politics[2].relation(0), relation);
    assert_eq!(campaign.actors[0].influence, 1000.0);
    let (output, _) = render(&ctx, &mut campaign, 2, 0.4, vec![]);
    let text = labels(&output);
    assert!(text.contains("6 months"));
    assert!(!text.contains("Trading partner"));
    assert!(click_action(&ctx, &mut campaign, 2, "Stop now", 0.5).is_some());
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
        assert!(labels(&output).contains("Pending"));
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
                coin: 10.0,
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
        assert!(labels.contains("Stop now"));
        assert!(labels.contains("Stop"));
        assert!(
            labels.contains(if owned {
                "#33"
            } else {
                "#11"
            }),
            "Missing selected route: {labels}"
        );
        assert!(!labels.contains("#22"), "Unrelated province must be excluded");
        assert!(!labels.contains(if owned {
            "#11"
        } else {
            "#33"
        }));
        assert!(!labels.contains("Open market"));
    }
}
