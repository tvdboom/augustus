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
    let mut campaign = Campaign::default();
    campaign.economy = EconomyWorld::new(2, provinces, graph.clone());
    for wallet in &mut campaign.economy.players {
        wallet.coin = 5000.0;
        wallet.resources = [2000.0; 3];
    }
    campaign.actors = vec![
        PoliticalPlayer {
            coin: 5000.0,
            influence: 1000.0,
            rank: PoliticalRank::Aedile,
            consul_until: None,
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
fn province_routes_filter_both_sides_of_offers_and_follow_owner_changes() {
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
