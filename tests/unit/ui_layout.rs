//! Headless layout checks run real campaign panel bodies at desktop/compact widths.

use super::super::campaign_notifications::NoticeSeverity;
use super::*;
use crate::game::economy::{EconomicProvince, EconomyWorld, Terrain};
use crate::game::politics::espionage::{Scandal, ScandalKind, ScandalTarget, Severity};
use crate::game::politics::senate::PoliticalProfile;
use crate::game::politics::PoliticalPlayer;
use crate::game::politics::PoliticalRank;

/// Deterministic representative states cover ownership, independence, vassals and troops.
fn fixture() -> Campaign {
    let names = ["Italia", "Africa Proconsularis", "Achaia", "Cappadocia", "Sardinia et Corsica"];
    let mut provinces: Vec<_> = names
        .iter()
        .map(|name| {
            EconomicProvince::new(
                *name,
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
    provinces[0].wonder_sites = vec![0];
    provinces[1].owner = Some(1);
    provinces[3].overlord = Some(0);
    provinces[4].overlord = Some(1);
    let graph = vec![vec![1, 2], vec![0, 3], vec![0, 3, 4], vec![1, 2, 4], vec![2, 3]];
    let mut campaign = Campaign::default();
    campaign.active = true;
    campaign.economy = EconomyWorld::new(2, provinces, graph.clone());
    for wallet in &mut campaign.economy.players {
        wallet.coin = 5000.0;
        wallet.influence = 1000.0;
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
        ProvincePolitics::independent(2),
    ];
    campaign.politics[2].state = PoliticalState::Independent {
        local: 15.0,
        shares: vec![55.0, 30.0],
    };
    campaign.politics[3].state = PoliticalState::Vassal {
        overlord: 0,
        control: 72.0,
        tribute: Tribute::High,
    };
    campaign.politics[4].state = PoliticalState::Vassal {
        overlord: 1,
        control: 40.0,
        tribute: Tribute::Normal,
    };
    campaign.military = MilitaryWorld::new(5);
    campaign.graph = graph
        .into_iter()
        .map(|neighbors| MilitaryProvince {
            terrain: MilitaryTerrain::Plains,
            area: 100.0,
            road_level: 1,
            neighbors,
        })
        .collect();
    for unit_type in UnitType::ALL {
        campaign.military.seed_unit(0, ForceOwner::Player(0), unit_type).unwrap();
    }
    campaign.military.seed_unit(1, ForceOwner::Player(1), UnitType::HeavyInfantry).unwrap();
    campaign.wars = vec![vec![false; 2]; 2];
    campaign.npc_wars = vec![vec![false; 5]; 2];
    campaign.invitations = vec![vec![true; 2]; 2];
    campaign.profiles = vec![PoliticalProfile::default(); 2];
    campaign.recent_victories = vec![0.0; 2];
    campaign.actors[1].rank = PoliticalRank::Consul;
    campaign.actors[1].consul_until = Some(48);
    campaign.espionage.scandals = vec![
        Scandal {
            id: 1,
            holder: 0,
            target: ScandalTarget::Province(2),
            kind: ScandalKind::MilitaryIncompetence,
            severity: Severity::Major,
            province: Some(2),
            source_id: 10,
            acquired: 0,
            expires: 24,
            reserved_for_motion: false,
        },
        Scandal {
            id: 2,
            holder: 0,
            target: ScandalTarget::Player(1),
            kind: ScandalKind::FriendlyAttack,
            severity: Severity::Medium,
            province: Some(1),
            source_id: 11,
            acquired: 0,
            expires: 24,
            reserved_for_motion: false,
        },
    ];
    campaign.notifications.province_notice(0, 1, 0, NoticeSeverity::Warning, super::super::campaign_notifications::NoticeKind::HostileRelation, "Relations with Africa Proconsularis have become hostile", "A foreign political campaign reduced our relationship. Inspect the provincial diplomacy panel for its full breakdown.");
    campaign.economy.refresh_npc_markets(&campaign.inputs());
    campaign
}

/// Exercise egui's actual layout, including expanded sections, without a desktop window.
fn layout_context() -> egui::Context {
    let ctx = egui::Context::default();
    ctx.set_global_style(augustus_ui_style());
    ctx.add_font(FontInsert::new(
        "firasans",
        egui::FontData::from_static(include_bytes!("../../assets/fonts/FiraSans-Bold.ttf")),
        vec![InsertFontFamily {
            family: egui::FontFamily::Proportional,
            priority: FontPriority::Highest,
        }],
    ));
    ctx.memory_mut(|memory| memory.set_everything_is_visible(true));
    ctx
}

#[test]
fn spy_overview_lists_only_provinces_open_to_player_networks() {
    let ctx = layout_context();
    let mut campaign = fixture();
    let mut search = String::new();
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 800.0),
            )),
            ..Default::default()
        },
        |ui| {
            politics_spies(ui, &mut campaign, 0, &[], &mut search, 1.0);
        },
    );
    let labels: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
            _ => None,
        })
        .collect();
    for name in ["Africa Proconsularis", "Achaia", "Sardinia et Corsica"] {
        assert!(labels.contains(&name), "Missing eligible target: {name}");
    }
    for name in ["Italia", "Cappadocia"] {
        assert!(!labels.contains(&name), "Ineligible target listed: {name}");
    }
    output.textures_delta.clear();
}

#[test]
fn province_and_spy_directories_share_row_spacing_and_name_gaps_with_contained_meters() {
    for (width, scale) in [(546.0, 1.0), (380.0, 1.0), (320.0, 0.85)] {
        let mut campaign = fixture();
        let mut measurements = Vec::new();
        for spies in [false, true] {
            let ctx = layout_context();
            let mut search = String::new();
            let mut bounds = egui::Rect::NOTHING;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(800.0, 1400.0),
                    )),
                    ..Default::default()
                },
                |root| {
                    root.scope_builder(
                        egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 1400.0),
                        )),
                        |ui| {
                            panel_style(ui, scale);
                            if spies {
                                politics_spies(ui, &mut campaign, 0, &[], &mut search, scale);
                            } else {
                                politics_provinces(ui, &campaign, 0, &[], &mut search, scale);
                            }
                            bounds = ui.min_rect();
                        },
                    );
                },
            );
            assert!(bounds.width() <= width + 1.5, "Directory overflow at {width}px: {bounds:?}");
            let text = |name: &str| {
                output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) if text.galley.job.text == name => Some(text.pos),
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("Missing {name}"))
            };
            let achaia = text("Achaia");
            let africa = text("Africa Proconsularis");
            let rows: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Rect(rect)
                        if (rect.rect.height() - 50.0 * scale).abs() < 0.1
                            && rect.rect.width() > width * 0.8 =>
                    {
                        Some(rect.rect)
                    },
                    _ => None,
                })
                .collect();
            let row = *rows
                .iter()
                .find(|row| row.contains(achaia))
                .expect("Province name stays inside its row");
            measurements.push((achaia.x - row.left(), africa.y - achaia.y));
            if !spies {
                assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if matches!(text.galley.job.text.as_str(), "CONTROL" | "RELATION"))));
                let meters: Vec<_> = output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Rect(rect)
                            if (rect.rect.height() - 24.0 * scale).abs() < 0.1
                                && rect.rect.width() > 50.0 * scale =>
                        {
                            Some(rect.rect)
                        },
                        _ => None,
                    })
                    .collect();
                assert!(!meters.is_empty());
                for meter in meters {
                    assert!(
                        rows.iter().any(|row| row.contains_rect(meter)),
                        "Meter escaped its row: {meter:?}"
                    );
                }
            }
            output.textures_delta.clear();
        }
        assert!(
            (measurements[0].0 - measurements[1].0).abs() < 0.1,
            "Province names need the same gap after their owner marker"
        );
        assert!(
            (measurements[0].1 - measurements[1].1).abs() < 0.1,
            "Province rows need the same vertical spacing as spies"
        );
    }
}

fn render_spy_overview(
    ctx: &egui::Context,
    campaign: &mut Campaign,
    time: f64,
    events: Vec<egui::Event>,
) -> (egui::FullOutput, Option<usize>, Option<String>) {
    let mut search = "Achaia".to_owned();
    let mut result = (None, None);
    let output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(550.0, 700.0),
            )),
            time: Some(time),
            events,
            ..Default::default()
        },
        |ui| {
            ui.set_width(550.0);
            result = politics_spies(ui, campaign, 0, &[], &mut search, 1.0);
        },
    );
    (output, result.0, result.1)
}

fn overview_spy_row(output: &egui::FullOutput) -> egui::Rect {
    output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect) if (rect.rect.height() - 50.0).abs() < 0.1 => Some(rect.rect),
            _ => None,
        })
        .expect("Achaia overview row")
}

#[test]
fn spy_overview_icon_actions_deploy_recall_and_flee_without_opening_province() {
    let ctx = layout_context();
    ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
    let mut campaign = fixture();
    let (mut output, _, _) = render_spy_overview(&ctx, &mut campaign, 0.0, vec![]);
    assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
        egui::Shape::Text(text) if text.galley.job.text.contains("Open diplomacy to establish a network"))));
    let row = overview_spy_row(&output);
    let discredit = egui::pos2(row.right() - 21.0, row.center().y);
    output.textures_delta.clear();
    let mut result = None;
    for (time, pressed) in [(0.1, true), (0.2, false)] {
        let (next, target, message) = render_spy_overview(
            &ctx,
            &mut campaign,
            time,
            vec![
                egui::Event::PointerMoved(discredit),
                egui::Event::PointerButton {
                    pos: discredit,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        assert_eq!(target, None, "Icon button must not open province");
        result = message.or(result);
        output = next;
        output.textures_delta.clear();
    }
    assert_eq!(result.as_deref(), Some("Spy deployed: Discredit rivals."));
    assert_eq!(campaign.espionage.missions[0].assignment, SpyAssignment::DiscreditRivals);

    let (next, _, _) = render_spy_overview(&ctx, &mut campaign, 0.3, vec![]);
    output = next;
    assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
        egui::Shape::Text(text) if text.galley.job.text.contains("months active"))));
    spy_text_position(&output, "Discredit rivals");
    let row = overview_spy_row(&output);
    let recall = egui::pos2(row.right() - 52.0, row.center().y);
    output.textures_delta.clear();
    for (time, pressed) in [(0.4, true), (0.5, false)] {
        let (next, target, _) = render_spy_overview(
            &ctx,
            &mut campaign,
            time,
            vec![
                egui::Event::PointerMoved(recall),
                egui::Event::PointerButton {
                    pos: recall,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        assert_eq!(target, None);
        output = next;
        output.textures_delta.clear();
    }
    assert_eq!(campaign.espionage.missions[0].recall_month, Some(6));
    for (month, label) in [(0, "6 months"), (5, "1 month")] {
        campaign.economy.month = month;
        let (next, _, _) = render_spy_overview(&ctx, &mut campaign, 0.55, vec![]);
        output = next;
        let countdown = spy_text_position(&output, label);
        let row = overview_spy_row(&output);
        assert!(row.contains(countdown), "Recall countdown must stay inside its spy row");
        output.textures_delta.clear();
    }
    let row = overview_spy_row(&output);
    let flee = egui::pos2(row.right() - 21.0, row.center().y);
    for (time, pressed) in [(0.6, true), (0.7, false)] {
        let (next, target, _) = render_spy_overview(
            &ctx,
            &mut campaign,
            time,
            vec![
                egui::Event::PointerMoved(flee),
                egui::Event::PointerButton {
                    pos: flee,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        assert_eq!(target, None);
        output = next;
        output.textures_delta.clear();
    }
    assert!(campaign.espionage.missions.is_empty());
}

fn render_spy_network(
    ctx: &egui::Context,
    campaign: &mut Campaign,
    province: usize,
    width: f32,
    time: f64,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000.0, 720.0),
            )),
            time: Some(time),
            events,
            ..Default::default()
        },
        |root| {
            root.scope_builder(
                egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                    egui::pos2(20.0, 20.0),
                    egui::vec2(width, 650.0),
                )),
                |ui| {
                    panel_style(ui, 1.0);
                    spy_network(ui, campaign, province, 0, 1.0);
                },
            );
        },
    )
}

fn spy_text_position(output: &egui::FullOutput, label: &str) -> egui::Pos2 {
    output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == label => {
                Some(text.pos + egui::vec2(4.0, 4.0))
            },
            _ => None,
        })
        .unwrap_or_else(|| panic!("Missing spy label: {label}"))
}

#[test]
fn spy_buttons_deploy_disable_explain_and_recall_without_switching() {
    for width in [380.0, 570.0] {
        let ctx = layout_context();
        ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
        let mut campaign = fixture();
        let mut output = render_spy_network(&ctx, &mut campaign, 2, width, 0.0, vec![]);
        for label in [
            "SPY NETWORK",
            "Build control",
            "Improve relations",
            "Uncover scandals",
            "Undermine foes",
            "Support revolt",
            "Discredit rivals",
            "10",
            "5",
            "15",
            "20",
            "5/mo",
        ] {
            spy_text_position(&output, label);
        }
        assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Text(text) if text.galley.job.text == "No spy deployed."
                || text.galley.job.text == "SPY MISSIONS")));
        let cards: Vec<_> = SPY_ACTIONS
            .iter()
            .map(|(_, name, _)| {
                let label = spy_text_position(&output, name);
                output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Rect(rect)
                            if rect.fill == province_panel::TABLE_STRIPE
                                && rect.rect.contains(label) =>
                        {
                            Some(rect.rect)
                        },
                        _ => None,
                    })
                    .expect("Mission title is inside its button")
            })
            .collect();
        assert_eq!(cards[0].top(), cards[1].top());
        let columns = if width >= 390.0 {
            3
        } else {
            2
        };
        assert_eq!(cards[columns].left(), cards[0].left());
        assert_eq!(cards[columns].top(), cards[columns + 1].top());
        assert!(cards[0].right() < cards[1].left());
        assert!(cards[0].bottom() < cards[columns].top());
        for ((card, (_, name, _)), price) in
            cards.iter().zip(SPY_ACTIONS.iter()).zip(["10", "5", "15", "20", "20", "20"])
        {
            let title = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text)
                        if text.galley.job.text == *name
                            && card.contains(text.pos + egui::vec2(4.0, 4.0)) =>
                    {
                        Some(text.galley.rect.translate(text.pos.to_vec2()))
                    },
                    _ => None,
                })
                .unwrap();
            assert!(card.contains_rect(title), "{name} fits on one line inside its mission button");
            for label in [price, "5/mo"] {
                let cost = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(text)
                            if text.galley.job.text == label
                                && card.contains_rect(
                                    text.galley.rect.translate(text.pos.to_vec2()),
                                ) =>
                        {
                            Some(text.galley.rect.translate(text.pos.to_vec2()))
                        },
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("{label} must fit inside its mission button"));
                assert!(cost.top() > title.bottom(), "Costs sit below {name}");
            }
        }
        let deploy = spy_text_position(&output, "Build control");
        output.textures_delta.clear();
        for (time, pressed) in [(0.1, true), (0.2, false)] {
            output = render_spy_network(
                &ctx,
                &mut campaign,
                2,
                width,
                time,
                vec![
                    egui::Event::PointerMoved(deploy),
                    egui::Event::PointerButton {
                        pos: deploy,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
            output.textures_delta.clear();
        }
        assert_eq!(campaign.espionage.missions.len(), 1);
        assert_eq!(campaign.actors[0].influence, 990.0);
        output = render_spy_network(&ctx, &mut campaign, 2, width, 0.3, vec![]);
        for label in ["Build control", "Sestertii spent", "Control added", "Detection"] {
            spy_text_position(&output, label);
        }
        assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Text(text) if text.galley.job.text == "Improve relations")));
        assert_eq!(campaign.espionage.missions[0].assignment, SpyAssignment::GainControl);
        let recall = spy_text_position(&output, "Recall");
        output.textures_delta.clear();
        for (time, pressed) in [(1.9, true), (2.0, false)] {
            output = render_spy_network(
                &ctx,
                &mut campaign,
                2,
                width,
                time,
                vec![
                    egui::Event::PointerMoved(recall),
                    egui::Event::PointerButton {
                        pos: recall,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
            output.textures_delta.clear();
        }
        assert_eq!(campaign.espionage.missions.len(), 1);
        assert_eq!(campaign.espionage.missions[0].recall_month, Some(6));
        assert_eq!(campaign.actors[0].influence, 990.0, "Recall does not refund deployment");
        for month in 0..6 {
            campaign.economy.month = month;
            output =
                render_spy_network(&ctx, &mut campaign, 2, width, 2.1 + f64::from(month), vec![]);
            let remaining = 6 - month;
            let label = format!(
                "{remaining} {}",
                if remaining == 1 {
                    "month"
                } else {
                    "months"
                }
            );
            let countdown = spy_text_position(&output, &label);
            let flee = spy_text_position(&output, "Flee");
            assert!((countdown.y - flee.y).abs() < 5.0);
            assert!(countdown.x < flee.x);
            assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
                egui::Shape::Text(text) if text.galley.job.text == "Recall"
                    || text.galley.job.text.contains("Mission and upkeep continue"))));
            output.textures_delta.clear();
        }
    }
}

#[test]
fn spy_network_shows_cumulative_results_detection_and_adjacent_exit_choices() {
    for (assignment, province, result, value) in [
        (SpyAssignment::GainControl, 2, "Control added", "3"),
        (SpyAssignment::ImproveRelations, 2, "Relation gained", "2.5"),
        (SpyAssignment::DiscoverScandals, 1, "Scandals found", "2"),
        (SpyAssignment::UndermineOpponents, 1, "Happiness lost", "4.5"),
        (SpyAssignment::DiscreditRivals, 2, "Rival losses", "1 C · 2 R"),
    ] {
        let ctx = layout_context();
        ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
        let mut campaign = fixture();
        let distance = campaign.distance(0, province);
        campaign
            .espionage
            .deploy_assignment(
                0,
                province,
                &mut campaign.actors,
                &campaign.politics,
                &campaign.espionage_config,
                assignment,
                distance,
            )
            .unwrap();
        let totals = &mut campaign.espionage.missions[0].totals;
        totals.coin_spent = 15.0;
        totals.control_contributed = 3.0;
        totals.relation_gained = 2.5;
        totals.scandals_revealed = 2;
        totals.happiness_reduced = 4.5;
        totals.rival_control_reduced = 1.0;
        totals.rival_relation_reduced = 2.0;
        campaign.economy.provinces[province].happiness[0] = 0.0;
        let mut output = render_spy_network(&ctx, &mut campaign, province, 380.0, 0.0, vec![]);
        spy_text_position(&output, result);
        spy_text_position(&output, value);
        spy_text_position(&output, "Sestertii spent");
        spy_text_position(&output, "Detection");
        spy_text_position(&output, "2%");
        campaign.economy.provinces[province].happiness[0] = 100.0;
        let recall = spy_text_position(&output, "Recall");
        let flee = spy_text_position(&output, "Flee");
        assert!((recall.y - flee.y).abs() < 5.0);
        assert!(recall.x < flee.x);
        output.textures_delta.clear();
        output = render_spy_network(
            &ctx,
            &mut campaign,
            province,
            380.0,
            0.1,
            vec![egui::Event::PointerMoved(recall)],
        );
        output.textures_delta.clear();
        output = render_spy_network(&ctx, &mut campaign, province, 380.0, 1.1, vec![]);
        output.textures_delta.clear();
        output = render_spy_network(&ctx, &mut campaign, province, 380.0, 1.2, vec![]);
        spy_text_position(&output, "10%");
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Text(text) if text.galley.job.text.contains("Return after six months"))));
        let primitives = ctx.tessellate(output.shapes.clone(), output.pixels_per_point);
        assert!(primitives.iter().any(|primitive| matches!(&primitive.primitive,
            egui::epaint::Primitive::Mesh(mesh) if mesh.texture_id == campaign_widgets::texture(&ctx, Icon::Cancel))));
        output.textures_delta.clear();
    }
}

#[test]
fn spy_cards_show_the_same_whole_distance_prices_as_the_rules() {
    let ctx = layout_context();
    let mut campaign = fixture();
    campaign.economy.adjacency = vec![vec![1], vec![0, 2], vec![1], vec![], vec![]];
    assert_eq!(campaign.distance(0, 2), Some(2));
    let mut output = render_spy_network(&ctx, &mut campaign, 2, 380.0, 0.0, vec![]);
    for price in ["13", "6/mo", "6", "19", "25"] {
        spy_text_position(&output, price);
    }
    assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
        egui::Shape::Text(text) if text.galley.job.text.contains(".0"))));
    output.textures_delta.clear();
}

#[test]
fn foreign_provinces_show_control_relation_and_unrest_spy_options() {
    for province in [1, 2, 4] {
        let ctx = layout_context();
        ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
        let mut campaign = fixture();
        let mut output = render_spy_network(&ctx, &mut campaign, province, 380.0, 0.0, vec![]);
        let labels: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect();
        assert!(labels.contains(&"Undermine foes"));
        assert!(labels.contains(&"Uncover scandals"));
        for label in ["Build control", "Improve relations"] {
            assert!(labels.contains(&label));
        }
        assert!(!labels.iter().any(|label| label.contains(".00")), "Spy costs use whole numbers");
        if province == 1 {
            let position = spy_text_position(&output, "Undermine foes");
            output.textures_delta.clear();
            output = render_spy_network(
                &ctx,
                &mut campaign,
                province,
                380.0,
                0.1,
                vec![egui::Event::PointerMoved(position)],
            );
            output.textures_delta.clear();
            output = render_spy_network(&ctx, &mut campaign, province, 380.0, 1.1, vec![]);
            output.textures_delta.clear();
            output = render_spy_network(&ctx, &mut campaign, province, 380.0, 1.2, vec![]);
            assert!(
                output.shapes.iter().any(|shape| match &shape.shape {
                    egui::Shape::Text(text) =>
                        text.galley.job.text == "Reduce 0–3 happiness for one random Noble, Citizen or Plebeian class each month.",
                    _ => false,
                }),
                "Undermining explains whose Happiness may fall"
            );
        }
        output.textures_delta.clear();
    }
}

#[test]
fn unavailable_spy_mission_explains_its_restriction_below_the_effect() {
    let ctx = layout_context();
    ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
    let mut campaign = fixture();
    let mut output = render_spy_network(&ctx, &mut campaign, 1, 380.0, 0.0, vec![]);
    let card = spy_text_position(&output, "Discredit rivals");
    output.textures_delta.clear();
    output = render_spy_network(
        &ctx,
        &mut campaign,
        1,
        380.0,
        0.1,
        vec![egui::Event::PointerMoved(card)],
    );
    output.textures_delta.clear();
    output = render_spy_network(&ctx, &mut campaign, 1, 380.0, 1.1, vec![]);
    output.textures_delta.clear();
    output = render_spy_network(&ctx, &mut campaign, 1, 380.0, 1.2, vec![]);
    let effect = "Reduce a random rivals' Control or Relation by 0–3 each month.";
    let reason = "Discredit rivals is available only in independent provinces.";
    let effect_y = spy_text_position(&output, effect).y;
    let reason_y = spy_text_position(&output, reason).y;
    assert!(reason_y > effect_y + 15.0, "The reason follows a blank line");
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
        egui::Shape::Text(text) if text.galley.job.text == reason
            && text.galley.job.sections[0].format.color == egui::Color32::from_rgb(170, 45, 35))));
    output.textures_delta.clear();
}

#[test]
fn recall_and_flee_buttons_have_matching_dimensions() {
    let ctx = layout_context();
    for remaining in [None, Some(6), Some(1)] {
        let mut sizes = None;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000.0, 720.0),
                )),
                ..Default::default()
            },
            |ui| {
                panel_style(ui, 1.0);
                ui.horizontal(|ui| {
                    let recall = spy_recall_button(ui, 1.0, remaining);
                    let flee = spy_flee_button(ui, 1.0);
                    assert_eq!(recall.enabled(), remaining.is_none());
                    sizes = Some((recall.rect.size(), flee.rect.size()));
                });
            },
        );
        let (recall, flee) = sizes.unwrap();
        output.textures_delta.clear();
        assert_eq!(recall, flee);
        assert_eq!(recall.x, 98.0);
        assert!(recall.y >= 30.0);
    }
}

#[test]
fn coin_flow_shows_trade_deals_spy_upkeep_and_negative_expenses() {
    let ctx = layout_context();
    ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
    let mut campaign = fixture();
    campaign
        .espionage
        .deploy(0, 2, &mut campaign.actors, &campaign.politics, &campaign.espionage_config)
        .unwrap();
    let icon = ctx.load_texture(
        "coin-flow-test",
        egui::ColorImage::from_rgba_unmultiplied([1, 1], &[255; 4]),
        egui::TextureOptions::LINEAR,
    );
    let mut open = false;
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 900.0));
    let pointer = egui::pos2(hud_resource_positions()[3] + 15.0, 15.0);
    for deployed in [true, false] {
        if !deployed {
            campaign.espionage.withdraw(0, 2);
        }
        for frame in 0..2 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    time: Some(
                        frame as f64
                            + if deployed {
                                0.0
                            } else {
                                2.0
                            },
                    ),
                    events: vec![egui::Event::PointerMoved(pointer)],
                    ..Default::default()
                },
                |_| {
                    super::super::coin_panel::show(
                        &ctx,
                        screen,
                        1.0,
                        1500.0,
                        0,
                        &ProvinceOwnership::default(),
                        Some(&campaign),
                        &icon,
                        &mut open,
                    );
                },
            );
            if frame == 1 {
                let rows: Vec<_> = output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) => Some(text),
                        _ => None,
                    })
                    .collect();
                let row_value = |label: &str| {
                    let heading = rows
                        .iter()
                        .find(|text| text.galley.job.text == label)
                        .unwrap_or_else(|| panic!("Missing coin row: {label}"));
                    rows.iter()
                        .find(|text| {
                            text.pos.x > heading.pos.x
                                && ((text.pos.y + text.galley.size().y * 0.5)
                                    - (heading.pos.y + heading.galley.size().y * 0.5))
                                    .abs()
                                    < 1.0
                        })
                        .unwrap_or_else(|| panic!("Missing amount for coin row {label}"))
                        .galley
                        .job
                        .text
                        .as_str()
                };
                assert_eq!(
                    row_value("Spy upkeep"),
                    if deployed {
                        "-5"
                    } else {
                        "0"
                    }
                );
                assert_eq!(row_value("Trade deals"), "0");
                assert_eq!(row_value("Recruitment effort"), "0");
                assert!(row_value("Noble wages").starts_with('-'));
                assert!(row_value("Army wages").starts_with('-'));
                assert!(row_value("TOTAL OUTFLOW").starts_with('-'));
                assert!(!rows
                    .iter()
                    .any(|text| matches!(text.galley.job.text.as_str(), "Deals" | "-0" | "+0")));
            }
            output.textures_delta.clear();
        }
    }
}

fn hover_card_fixture(own_all: bool) -> (Campaign, ProvinceOwnership) {
    let mut ownership = ProvinceOwnership::default();
    ownership.start_game(&[PLAYER_COLORS[0]]);
    if own_all {
        for (id, seed) in ownership.campaign_seeds().into_iter().enumerate() {
            ownership.sync_campaign_province(id, Some(0), seed.population, seed.potential, 1.0);
        }
    }
    let mut campaign = Campaign::default();
    campaign.start(&ownership, 1);
    (campaign, ownership)
}

fn hover_card_textures(
    ctx: &egui::Context,
) -> ([egui::TextureHandle; 7], [egui::TextureHandle; 4]) {
    let load = |name| {
        ctx.load_texture(
            name,
            egui::ColorImage::from_rgba_unmultiplied([1, 1], &[255, 255, 255, 255]),
            egui::TextureOptions::LINEAR,
        )
    };
    (
        std::array::from_fn(|index| load(format!("hover-resource-{index}"))),
        std::array::from_fn(|index| load(format!("hover-class-{index}"))),
    )
}

#[allow(clippy::too_many_arguments)]
fn render_hover_cards(
    ctx: &egui::Context,
    campaign: &Campaign,
    ownership: &ProvinceOwnership,
    icons: &[egui::TextureHandle; 7],
    class_icons: &[egui::TextureHandle; 4],
    hover: &mut super::super::resource_hud::HudHoverState,
    screen: egui::Rect,
    scale: f32,
    time: f64,
    pointer: egui::Pos2,
) -> egui::FullOutput {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(screen),
            time: Some(time),
            events: vec![egui::Event::PointerMoved(pointer)],
            ..Default::default()
        },
        |_| {
            super::super::map_menu::draw_map_menu_hitboxes(ctx, scale);
            super::super::resource_hud::show_hud_hover_cards(
                ctx,
                screen,
                scale,
                super::super::map_menu::map_resource_strip_right(screen, scale)
                    - MAP_DATE_SECTION_WIDTH,
                0,
                ownership,
                Some(campaign),
                HudResource {
                    amount: ownership.total_population_for(0),
                    monthly_delta: 0.0,
                },
                icons,
                class_icons,
                hover,
            );
        },
    );
    output.textures_delta.clear();
    output
}

fn hover_card_rect(output: &egui::FullOutput) -> Option<egui::Rect> {
    output.shapes.iter().find_map(|shape| match &shape.shape {
        egui::Shape::Rect(rect) if rect.fill == egui::Color32::from_rgb(236, 231, 216) => {
            Some(rect.rect)
        },
        _ => None,
    })
}

#[test]
fn resource_hover_cards_open_over_the_drag_guard_and_stay_open_on_the_card() {
    let (campaign, ownership) = hover_card_fixture(false);
    let widths = [350.0, 265.0, 265.0, 430.0, 430.0, 350.0];
    for scale in [1.0, 0.85] {
        let ctx = layout_context();
        ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
        let (icons, class_icons) = hover_card_textures(&ctx);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 900.0));
        let mut hover = super::super::resource_hud::HudHoverState::default();
        let mut frame = 0;
        let mut render = |position| {
            frame += 1;
            let output = render_hover_cards(
                &ctx,
                &campaign,
                &ownership,
                &icons,
                &class_icons,
                &mut hover,
                screen,
                scale,
                frame as f64 * 0.1,
                position,
            );
            let active = hover.resource.or_else(|| {
                if hover.coin {
                    Some(3)
                } else if hover.influence {
                    Some(4)
                } else if hover.population {
                    Some(5)
                } else {
                    None
                }
            });
            (active, hover_card_rect(&output))
        };
        for (index, x) in hud_resource_positions().into_iter().enumerate() {
            let icon = egui::pos2(x + 35.0, 22.0) * scale;
            render(icon);
            let (active, rect) = render(icon);
            assert_eq!(active, Some(index));
            let rect = rect.expect("Hover must paint the original illustrated card");
            assert!((rect.width() - widths[index] * scale).abs() <= 1.0);
            assert!((rect.top() - 45.0 * scale).abs() <= 1.0);
            assert!(screen.contains_rect(rect));
            assert_eq!(
                render(rect.center()).0,
                Some(index),
                "Card must remain usable for scrolling"
            );
            assert_eq!(render(egui::pos2(1550.0, 850.0)).0, None);
        }
    }
}

#[test]
fn food_province_hover_opens_civilian_and_military_breakdown() {
    let (campaign, ownership) = hover_card_fixture(false);
    let ctx = layout_context();
    ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
    let (icons, class_icons) = hover_card_textures(&ctx);
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 900.0));
    let mut hover = super::super::resource_hud::HudHoverState::default();
    let icon = egui::pos2(hud_resource_positions()[0] + 35.0, 22.0);
    render_hover_cards(
        &ctx,
        &campaign,
        &ownership,
        &icons,
        &class_icons,
        &mut hover,
        screen,
        1.0,
        0.0,
        icon,
    );
    let base = render_hover_cards(
        &ctx,
        &campaign,
        &ownership,
        &icons,
        &class_icons,
        &mut hover,
        screen,
        1.0,
        0.1,
        icon,
    );
    let rect = hover_card_rect(&base).unwrap();
    let row = egui::pos2(rect.left() + 60.0, rect.top() + 78.0 + 29.0 + 15.0);
    render_hover_cards(
        &ctx,
        &campaign,
        &ownership,
        &icons,
        &class_icons,
        &mut hover,
        screen,
        1.0,
        0.2,
        row,
    );
    let detail = render_hover_cards(
        &ctx,
        &campaign,
        &ownership,
        &icons,
        &class_icons,
        &mut hover,
        screen,
        1.0,
        0.3,
        row,
    );
    assert_eq!(hover.resource, Some(0));
    assert_eq!(hover.food_row, Some(0));
    assert_eq!(hover_card_rect(&detail).unwrap().height(), rect.height() + 91.0);
    for label in ["FOOD BREAKDOWN", "Civilians", "Military"] {
        assert!(
            detail.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Text(text) if text.galley.job.text == label)),
            "Missing food detail label {label}"
        );
    }
}

#[test]
fn original_governance_card_retains_its_banner_buttons_badges_and_close() {
    let ctx = layout_context();
    ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
    let (icons, _) = hover_card_textures(&ctx);
    let mut governance = Governance::default();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 900.0));
    let mut time = 0.0;
    let mut render = |events| {
        time += 0.1;
        let mut changed = false;
        let mut closed = false;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen),
                time: Some(time),
                events,
                ..Default::default()
            },
            |_| {
                (changed, closed) = super::super::governance_panel::show(
                    &ctx,
                    1.0,
                    &icons[0],
                    &icons,
                    PLAYER_COLORS[0],
                    &mut governance,
                    false,
                );
            },
        );
        output.textures_delta.clear();
        (output, changed, closed, governance)
    };
    render(vec![]);
    let (output, _, _, _) = render(vec![]);
    let panel = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect) if rect.fill == egui::Color32::from_rgb(238, 233, 219) => {
                Some(rect.rect)
            },
            _ => None,
        })
        .expect("The original parchment governance card must be painted");
    assert_eq!(panel, map_corner_panel_rect(screen, 1.0, egui::vec2(PANEL_WIDTH, PANEL_HEIGHT)));
    for label in [
        "Governance",
        "LABOR & AGRARIAN",
        "ECONOMY",
        "Food Rations",
        "Slave Labor",
        "Noble Wages",
        "Army Wages",
    ] {
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == label)), "Missing original governance label {label}");
    }
    assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == "Noble Taxes")));
    let high = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == "High" => {
                Some(egui::Rect::from_min_size(text.pos, text.galley.size()).center())
            },
            _ => None,
        })
        .min_by(|a, b| a.y.total_cmp(&b.y))
        .expect("Food rations must retain their High button");
    for pressed in [true, false] {
        let (_, changed, _, selected) = render(vec![
            egui::Event::PointerMoved(high),
            egui::Event::PointerButton {
                pos: high,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        if !pressed {
            assert!(changed);
            assert_eq!(selected.food_rations, crate::map::EdictLevel::High);
        }
    }
    let close = panel.right_top() + egui::vec2(-25.0, 25.0);
    for pressed in [true, false] {
        let (_, _, closed, _) = render(vec![
            egui::Event::PointerMoved(close),
            egui::Event::PointerButton {
                pos: close,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        if !pressed {
            assert!(closed, "Original governance close button must remain clickable");
        }
    }
}

#[test]
fn overview_tabs_keep_governance_and_open_the_moved_directories() {
    let ctx = layout_context();
    ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
    let (icons, _) = hover_card_textures(&ctx);
    let mut governance = Governance::default();
    let mut selected = 0;
    let mut time = 0.0;
    let mut render = |events| {
        time += 0.1;
        let mut shown = None;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1600.0, 900.0),
                )),
                time: Some(time),
                events,
                ..Default::default()
            },
            |_| {
                super::super::governance_panel::show_overview(
                    &ctx,
                    1.0,
                    &icons[0],
                    &icons,
                    PLAYER_COLORS[0],
                    &mut governance,
                    false,
                    &mut selected,
                    |ui, section| {
                        shown = Some(section);
                        ui.label(format!("Directory {section}"));
                        None
                    },
                );
            },
        );
        output.textures_delta.clear();
        (output, shown, selected)
    };
    render(vec![]);
    let (output, shown, active) = render(vec![]);
    assert_eq!(active, 0);
    assert_eq!(shown, None);
    let texts: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) => Some(text),
            _ => None,
        })
        .collect();
    for label in [
        "Overview",
        "Policies",
        "Events",
        "Provinces",
        "Spies",
        "Scandals",
        "Notifications",
        "Food Rations",
    ] {
        assert!(texts.iter().any(|text| text.galley.job.text == label), "Missing {label}");
    }
    let tab_x =
        |label: &str| texts.iter().find(|text| text.galley.job.text == label).unwrap().pos.x;
    assert!(tab_x("Spies") < tab_x("Scandals"));
    assert!(tab_x("Scandals") < tab_x("Notifications"));
    let spies = texts.iter().find(|text| text.galley.job.text == "Spies").unwrap();
    let position = spies.galley.rect.translate(spies.pos.to_vec2()).center();
    for pressed in [true, false] {
        render(vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
    }
    let (output, shown, active) = render(vec![]);
    assert_eq!(active, 3);
    assert_eq!(shown, Some(3));
    assert!(output.shapes.iter().any(|shape| matches!(
        &shape.shape,
        egui::Shape::Text(text) if text.galley.job.text == "Directory 3"
    )));
    let scandals = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == "Scandals" => {
                Some(text.galley.rect.translate(text.pos.to_vec2()).center())
            },
            _ => None,
        })
        .unwrap();
    for pressed in [true, false] {
        render(vec![
            egui::Event::PointerMoved(scandals),
            egui::Event::PointerButton {
                pos: scandals,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
    }
    let (_, shown, active) = render(vec![]);
    assert_eq!(active, 4);
    assert_eq!(shown, Some(4));
}

#[test]
fn global_panel_headers_use_menu_art_and_senate_has_no_directory_tabs() {
    let ctx = layout_context();
    let campaign = fixture();
    let (icons, _) = hover_card_textures(&ctx);
    for (index, (tab, title)) in [
        (CampaignTab::Military, "Military"),
        (CampaignTab::Trade, "Trade"),
        (CampaignTab::Senate, "Rome"),
    ]
    .into_iter()
    .enumerate()
    {
        let mut view = CampaignUi {
            open: Some(tab),
            ..Default::default()
        };
        let mut render = |time, events| {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1200.0, 900.0),
                    )),
                    time: Some(time + index as f64 * 5.0),
                    events,
                    ..Default::default()
                },
                |_| {
                    egui::Area::new(egui::Id::new(("global-header-test", title)))
                        .fixed_pos(egui::pos2(30.0, 30.0))
                        .show(&ctx, |ui| {
                            panel_style(ui, 1.0);
                            panel_frame(1.0).show(ui, |ui| {
                                ui.set_width(PANEL_WIDTH);
                                panel_header(
                                    ui,
                                    title,
                                    1.0,
                                    &campaign.economy.provinces,
                                    0,
                                    &mut view,
                                    Some(&icons[0]),
                                );
                            });
                        });
                },
            )
        };
        let mut first = render(0.0, vec![]);
        first.textures_delta.clear();
        let mut output = render(0.1, vec![]);
        output.textures_delta.clear();
        assert!(
            output.shapes.iter().any(|shape| matches!(
                &shape.shape,
                egui::Shape::Rect(rect) if rect.fill_texture_id() == icons[0].id()
            )),
            "{title} header is missing its menu art"
        );
        let texts: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect();
        assert!(texts.contains(&title));
        if tab == CampaignTab::Senate {
            assert!(!texts.contains(&"Provinces"));
            assert!(!texts.contains(&"Spies"));
        }
        let heading = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == title => {
                    Some(text.pos + text.galley.rect.center().to_vec2())
                },
                _ => None,
            })
            .unwrap();
        for frame in 1..=4 {
            let mut hover = render(
                frame as f64,
                if frame == 1 {
                    vec![egui::Event::PointerMoved(heading)]
                } else {
                    vec![]
                },
            );
            hover.textures_delta.clear();
            assert_eq!(
                hover
                    .shapes
                    .iter()
                    .filter(|shape| matches!(&shape.shape,
                egui::Shape::Text(text) if text.galley.job.text == title))
                    .count(),
                1,
                "{title} header must not create a duplicate hover tooltip"
            );
        }
    }
}

fn run_province_header(
    ctx: &egui::Context,
    campaign: &Campaign,
    view: &mut CampaignUi,
    time: f64,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 900.0),
            )),
            time: Some(time),
            events,
            ..Default::default()
        },
        |_| {
            egui::Area::new(egui::Id::new("header-interaction-test"))
                .fixed_pos(egui::pos2(30.0, 30.0))
                .order(egui::Order::Foreground)
                .movable(false)
                .show(ctx, |ui| {
                    panel_style(ui, 1.0);
                    panel_frame(1.0).show(ui, |ui| {
                        ui.set_width(570.0);
                        panel_header(ui, "Italia", 1.0, &campaign.economy.provinces, 0, view, None);
                    });
                });
        },
    );
    output.textures_delta.clear();
    output
}

#[test]
fn province_banner_is_flush_and_its_title_search_selects_by_typing() {
    let ctx = layout_context();
    ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
    let campaign = fixture();
    let mut view = CampaignUi {
        open: Some(CampaignTab::Province),
        province: Some(0),
        ..Default::default()
    };
    run_province_header(&ctx, &campaign, &mut view, 0.0, vec![]);
    let output = run_province_header(&ctx, &campaign, &mut view, 0.1, vec![]);
    assert!(!output.shapes.iter().any(
        |shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == "Trade")
    ));
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == "Notifications")));
    let banner = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect) if rect.fill == PLAYER_COLORS[0] => Some(rect.rect),
            _ => None,
        })
        .expect("Owner banner must be painted");
    assert_eq!(banner.left_top(), egui::pos2(30.0, 30.0));
    assert!((banner.width() - 570.0).abs() <= 1.0);
    let title = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text)
                if text.galley.job.text == "Italia" && banner.contains(text.pos) =>
            {
                Some(text.clone())
            },
            _ => None,
        })
        .unwrap();
    let position = title.pos + title.galley.size() * 0.5;
    for (time, pressed) in [(0.2, true), (0.3, false)] {
        run_province_header(
            &ctx,
            &campaign,
            &mut view,
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
    assert!(view.province_selector_open, "Clicking the province name must open the dropdown");
    let editing = run_province_header(&ctx, &campaign, &mut view, 0.4, vec![]);
    let placeholder = editing
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text)
                if text.galley.job.text == "Italia" && banner.contains(text.pos) =>
            {
                Some(text)
            },
            _ => None,
        })
        .expect("The current province remains the title placeholder");
    assert!(
        title.pos.distance(placeholder.pos) < 1.0,
        "Editing must preserve the title position: {:?} -> {:?}",
        title.pos,
        placeholder.pos
    );
    assert!((title.galley.size() - placeholder.galley.size()).length() < 1.0);
    assert_eq!(
        title.galley.job.sections[0].format.font_id,
        placeholder.galley.job.sections[0].format.font_id
    );
    assert_eq!(
        title.galley.job.sections[0].format.color.gamma_multiply(0.5),
        placeholder.galley.job.sections[0].format.color,
        "The current name must recede while the search field is editable"
    );
    assert!(editing.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == "Cappadocia" && text.pos.y >= banner.bottom())), "Other provinces appear below the title");
    // Clicking an already open search raises its parent panel in the foreground layer.
    for (time, pressed) in [(0.42, true), (0.44, false)] {
        run_province_header(
            &ctx,
            &campaign,
            &mut view,
            time,
            vec![egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
    let filtered = run_province_header(
        &ctx,
        &campaign,
        &mut view,
        0.5,
        vec![egui::Event::Text("cAPpa".into())],
    );
    assert_eq!(view.province_search, "cAPpa");
    let result = filtered
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == "Cappadocia" => {
                Some(text.pos + text.galley.size() * 0.5)
            },
            _ => None,
        })
        .expect("The filtered province must be painted");
    assert_ne!(
        ctx.layer_id_at(result),
        Some(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("header-interaction-test"))),
        "The dropdown must remain above the foreground province panel after typing"
    );
    run_province_header(
        &ctx,
        &campaign,
        &mut view,
        0.6,
        vec![egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert_eq!(view.province, Some(3));
    assert!(!view.province_selector_open);
    assert!(!ctx.egui_wants_keyboard_input());
}

#[test]
fn province_search_list_aligns_with_the_field_and_accepts_clicks_past_the_name() {
    let ctx = layout_context();
    ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
    let campaign = fixture();
    let mut view = CampaignUi {
        open: Some(CampaignTab::Province),
        province: Some(0),
        province_selector_open: true,
        province_search: "Achaia".into(),
        ..Default::default()
    };
    run_province_header(&ctx, &campaign, &mut view, 0.0, vec![]);
    let output = run_province_header(&ctx, &campaign, &mut view, 0.1, vec![]);
    let field = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect) if rect.fill == egui::Color32::from_black_alpha(18) => {
                Some(rect.rect)
            },
            _ => None,
        })
        .expect("The editable title must have its own background");
    // Popup frames group their shadow and parchment rectangle in a shape vector.
    fn paper_below(shape: &egui::Shape, bottom: f32) -> Option<egui::Rect> {
        match shape {
            egui::Shape::Rect(rect)
                if rect.fill == province_panel::PAPER && rect.rect.top() >= bottom =>
            {
                Some(rect.rect)
            },
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| paper_below(shape, bottom)),
            _ => None,
        }
    }
    let dropdown = output
        .shapes
        .iter()
        .find_map(|shape| paper_below(&shape.shape, field.bottom()))
        .expect("The province list must appear below the search field");
    assert!((dropdown.left() - field.left()).abs() < 1.0);
    assert!((dropdown.top() - field.bottom()).abs() < 1.0);
    assert!((dropdown.width() - field.width()).abs() < 1.0);
    let row = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text)
                if text.galley.job.text == "Achaia" && dropdown.contains(text.pos) =>
            {
                Some(egui::Rect::from_min_size(text.pos, text.galley.size()))
            },
            _ => None,
        })
        .expect("The filtered province must be the dropdown's only result");
    assert!(!output.shapes.iter().any(|shape| matches!(
        &shape.shape, egui::Shape::Text(text) if text.galley.job.text == "Type in the province title to search"
    )));
    let position = egui::pos2(dropdown.right() - 12.0, row.center().y);
    assert!(position.x > row.right() + 100.0, "Exercise the empty part of the row");
    for (time, pressed) in [(0.2, true), (0.3, false)] {
        run_province_header(
            &ctx,
            &campaign,
            &mut view,
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
    assert_eq!(view.province, Some(2));
    assert!(!view.province_selector_open);
    assert!(view.province_search.is_empty());
}

#[test]
fn province_close_button_changes_color_on_hover_and_press() {
    let mut feedback = Vec::new();
    for color in PLAYER_COLORS.into_iter().chain([province_panel::NEUTRAL]) {
        let ctx = layout_context();
        ctx.data_mut(|data| data.insert_temp(egui::Id::new("campaign-owner-color"), color));
        ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
        let campaign = fixture();
        let mut view = CampaignUi {
            open: Some(CampaignTab::Province),
            province: Some(0),
            ..Default::default()
        };
        run_province_header(&ctx, &campaign, &mut view, 0.0, vec![]);
        let normal = run_province_header(&ctx, &campaign, &mut view, 0.1, vec![]);
        let close_circle = |output: &egui::FullOutput| {
            output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Circle(circle)
                        if (circle.radius - 12.0).abs() < 0.1
                            && circle.center.x > 550.0
                            && circle.center.y < 75.0 =>
                    {
                        Some(circle.clone())
                    },
                    _ => None,
                })
                .expect("Close button circle")
        };
        let normal = close_circle(&normal);
        let hover = run_province_header(
            &ctx,
            &campaign,
            &mut view,
            0.2,
            vec![egui::Event::PointerMoved(normal.center)],
        );
        let hover = close_circle(&hover);
        let pressed = run_province_header(
            &ctx,
            &campaign,
            &mut view,
            0.3,
            vec![egui::Event::PointerButton {
                pos: normal.center,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        let pressed = close_circle(&pressed);
        assert_ne!(normal.fill, hover.fill);
        assert_ne!(hover.fill, pressed.fill);
        assert_eq!(normal.stroke.color, hover.stroke.color, "Hover must retain contrasting ink");
        assert_eq!(hover.stroke.color, pressed.stroke.color, "Press must retain contrasting ink");
        feedback.push((hover.fill, pressed.fill));
    }
    assert!(
        feedback.windows(2).all(|colors| colors[0] != colors[1]),
        "Close feedback must change with the owner banner"
    );
}

#[test]
fn province_tabs_offer_policies_only_to_the_direct_owner_and_trade_to_everyone_else() {
    let campaign = fixture();
    for province in 0..5 {
        let ctx = layout_context();
        ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
        let mut view = CampaignUi {
            open: Some(CampaignTab::Province),
            province: Some(province),
            section: 1,
            ..Default::default()
        };
        run_province_header(&ctx, &campaign, &mut view, 0.0, vec![]);
        let output = run_province_header(&ctx, &campaign, &mut view, 0.1, vec![]);
        let labels: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect();
        let owned = campaign.economy.provinces[province].owner == Some(0);
        assert_eq!(labels.contains(&"Policies"), owned);
        assert_eq!(labels.contains(&"Trade"), !owned);
    }
}

#[test]
fn top_bar_draws_civic_power_before_resources_with_abbreviated_population() {
    let ctx = layout_context();
    let (icons, _) = hover_card_textures(&ctx);
    let mut resources = [HudResource {
        amount: 0.0,
        monthly_delta: 0.0,
    }; 7];
    for (index, value) in [110.0, 220.0, 330.0, 440.0, 550.0, 1660.9].into_iter().enumerate() {
        resources[index].amount = value;
    }
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 900.0),
            )),
            ..Default::default()
        },
        |root| {
            super::super::resource_hud::paint_hud_resources(
                root.painter(),
                egui::Pos2::ZERO,
                1.0,
                1200.0,
                &resources,
                &icons,
            );
        },
    );
    output.textures_delta.clear();
    let left = |label: &str| {
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == label => Some(text.pos.x),
                _ => None,
            })
            .unwrap_or_else(|| panic!("Missing top-bar value {label}"))
    };
    let positions = ["550", "440", "110", "220", "330", "1.7k"].map(left);
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn top_bar_population_growth_and_icon_use_whole_numbers_and_fixed_size() {
    use super::super::resource_hud::format_population_delta;

    assert_eq!(format_population_delta(-41.299), "-42");
    assert_eq!(format_population_delta(5.9), "+5");
    assert_eq!(format_population_delta(0.9), "0");

    let ctx = layout_context();
    let (icons, _) = hover_card_textures(&ctx);
    let mut resources = [HudResource {
        amount: 0.0,
        monthly_delta: 0.0,
    }; 7];
    let mut icon_bounds = |amount, delta| {
        resources[5] = HudResource {
            amount,
            monthly_delta: delta,
        };
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1600.0, 900.0),
                )),
                ..Default::default()
            },
            |root| {
                super::super::resource_hud::paint_hud_resources(
                    root.painter(),
                    egui::Pos2::ZERO,
                    1.0,
                    1200.0,
                    &resources,
                    &icons,
                );
            },
        );
        output.textures_delta.clear();
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Mesh(mesh) if mesh.texture_id == icons[5].id() => {
                    Some(mesh.calc_bounds().size())
                },
                _ => None,
            })
            .expect("population icon should be painted")
    };
    let small = icon_bounds(916.0, -41.299);
    let large = icon_bounds(123_456.9, -12_345.678);
    assert_eq!(small, egui::vec2(26.0, 26.0));
    assert_eq!(large, small);
}

#[test]
fn provincial_income_tooltips_show_illustrated_contributors_without_formulas() {
    use crate::game::economy::BuildingType;

    for (name, title, expected) in [
        ("Food", "Food production per month", vec!["16 Plebeians working", "18 Slaves working"]),
        (
            "Metal",
            "Metal production per month",
            vec!["9 Plebeians working", "10 Slaves working", "Buildings: +15% production"],
        ),
        ("Stone", "Stone production per month", vec!["7 Plebeians working", "7 Slaves working"]),
        (
            "Influence",
            "Influence per month",
            vec![
                "8 Nobles: +2 Influence",
                "Buildings: +1 Influence",
                "Completed wonder: +3 Influence",
            ],
        ),
        (
            "Sestertius",
            "Sestertii per month",
            vec!["22 Citizens: +11.0", "33 Plebeians: +6.6", "Buildings: +10% taxes"],
        ),
    ] {
        let ctx = layout_context();
        ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
        let mut campaign = fixture();
        for building in [BuildingType::CityHall, BuildingType::UrbanMarket, BuildingType::Forum] {
            campaign.economy.provinces[0].buildings[building as usize] = 1;
        }
        if name == "Influence" {
            campaign.economy.provinces[0].completed_wonder = Some(0);
            campaign
                .economy
                .config
                .wonders
                .iter_mut()
                .find(|wonder| wonder.wonder_id == 0)
                .unwrap()
                .monthly_influence = 3.0;
        }
        let render = |time, pointer: Option<egui::Pos2>| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    time: Some(time),
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1200.0, 900.0),
                    )),
                    events: pointer.into_iter().map(egui::Event::PointerMoved).collect(),
                    ..Default::default()
                },
                |root| {
                    egui::CentralPanel::default().show(root, |ui| {
                        ui.set_width(546.0);
                        panel_style(ui, 1.0);
                        campaign_economy::overview(
                            ui,
                            &campaign.economy,
                            &campaign.military,
                            0,
                            1.0,
                        );
                    });
                },
            );
            output.textures_delta.clear();
            output
        };
        let output = render(0.0, None);
        let plain_labels: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.clone()),
                _ => None,
            })
            .collect();
        let row_y = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == name => Some(text.pos.y),
                _ => None,
            })
            .unwrap();
        if name == "Influence" {
            assert!(output.shapes.iter().any(|shape| match &shape.shape {
                egui::Shape::Text(text) => {
                    (text.pos.y - row_y).abs() < 1.0 && text.galley.job.text == "+6"
                },
                _ => false,
            }));
        }
        let amount_pointer = output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::Shape::Text(text)
                if (text.pos.y - row_y).abs() < 1.0
                    && text.galley.job.text != name
                    && !text.galley.job.text.starts_with('+') =>
            {
                Some(egui::Rect::from_min_size(text.pos, text.galley.size()).center())
            },
            _ => None,
        });
        let pointer = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if (text.pos.y - row_y).abs() < 1.0 => {
                    Some(egui::Rect::from_min_size(text.pos, text.galley.size()).center())
                },
                _ => None,
            })
            .max_by(|a, b| a.x.total_cmp(&b.x))
            .unwrap();
        render(0.1, Some(pointer));
        render(1.0, None);
        let output = render(1.1, None);
        let labels: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect();
        assert!(labels.contains(&title), "Missing {title} at {pointer:?}: {labels:?}");
        if name == "Sestertius" {
            assert!(!labels.iter().any(|text| text.to_lowercase().contains("coin")));
            assert!(!labels.iter().any(|text| text.contains("Nobles:")));
        }
        assert_eq!(labels.iter().filter(|text| **text == "•").count(), expected.len());
        for entry in expected {
            assert!(labels.contains(&entry), "Missing {entry}: {labels:?}");
            let text = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.job.text == entry => Some(text),
                    _ => None,
                })
                .unwrap();
            let text_rect = egui::Rect::from_min_size(text.pos, text.galley.size());
            assert!(
                output.shapes.iter().any(|shape| match &shape.shape {
                    egui::Shape::Mesh(mesh) => {
                        let icon = mesh.calc_bounds();
                        (icon.height() - 22.0).abs() < 1.0
                            && icon.top() < text_rect.bottom()
                            && icon.bottom() > text_rect.top()
                            && icon.right() < text.pos.x
                            && text.pos.x - icon.right() < 22.0
                    },
                    _ => false,
                }),
                "Missing class or building icon beside {entry}"
            );
        }
        for removed in
            [" × ", "potential", "scale", "Labor is allocated", "Per Noble", "Rank and vassal"]
        {
            assert!(
                !labels.iter().any(|text| text.contains(removed)),
                "Formula remains: {labels:?}"
            );
        }
        if matches!(name, "Influence" | "Sestertius") {
            render(2.0, Some(amount_pointer.unwrap()));
            render(3.0, None);
            let output = render(3.1, None);
            let labels: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) => Some(text.galley.job.text.clone()),
                    _ => None,
                })
                .collect();
            assert_eq!(labels, plain_labels, "{name} amount must have no tooltip");
        }
    }
}

#[test]
fn food_demand_and_capacity_tooltips_use_consistent_signs_colours_and_one_decimal() {
    use crate::game::economy::BuildingType;

    for (terrain, military, city) in [(0.7, false, false), (1.0, true, true), (1.2, false, true)] {
        let ctx = layout_context();
        ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
        let mut campaign = fixture();
        let p = &mut campaign.economy.provinces[0];
        p.has_city = city;
        p.buildings[BuildingType::Baths as usize] = u32::from(city);
        if !military {
            campaign.military.provinces[0].forces.clear();
            p.population[3] = 0.0;
        }
        campaign.economy.config.terrain_capacity[p.terrain as usize] = terrain;
        let p = &campaign.economy.provinces[0];
        let civilian_food = p.food_request(&campaign.economy.config);
        let military_food = campaign.military.food_demand(ForceOwner::Player(0));
        let text_shapes = |output: &egui::FullOutput| {
            output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) => Some((text.pos, text.galley.clone())),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let render = |time, pointer: Option<egui::Pos2>| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    time: Some(time),
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1200.0, 900.0),
                    )),
                    events: pointer.into_iter().map(egui::Event::PointerMoved).collect(),
                    ..Default::default()
                },
                |root| {
                    egui::CentralPanel::default().show(root, |ui| {
                        ui.set_width(546.0);
                        panel_style(ui, 1.0);
                        campaign_economy::overview(
                            ui,
                            &campaign.economy,
                            &campaign.military,
                            0,
                            1.0,
                        );
                    });
                },
            );
            output.textures_delta.clear();
            output
        };
        let check_bullet = |output: &egui::FullOutput,
                            label: &str,
                            value: &str,
                            color: egui::Color32| {
            let text = text_shapes(output);
            let label =
                text.iter().find(|(_, galley)| galley.job.text == format!("• {label}:")).unwrap();
            let right = label.0.x + label.1.size().x;
            let number = text
                .iter()
                .find(|(pos, galley)| {
                    galley.job.text == value
                        && (pos.y - label.0.y).abs() < 1.0
                        && pos.x > right
                        && pos.x - right < 20.0
                })
                .unwrap();
            assert_eq!(number.1.job.sections[0].format.color, color);
        };
        let baseline = render(0.0, None);
        let text = text_shapes(&baseline);
        let demand = text
            .iter()
            .find(|(_, galley)| {
                galley.job.text == format!("−{:.0}/mo", (civilian_food + military_food).floor())
            })
            .unwrap();
        assert_eq!(
            demand.1.job.sections[0].format.color,
            super::super::resource_hud::hud_delta_color(-1.0)
        );
        let population = text.iter().find(|(_, galley)| galley.job.text == "Population").unwrap();
        let population_pointer =
            egui::Rect::from_min_size(population.0, population.1.size()).center();
        render(0.1, Some(egui::Rect::from_min_size(demand.0, demand.1.size()).center()));
        render(1.0, None);
        let output = render(1.1, None);
        let food_value = |amount: f64| {
            if amount.abs().floor() == 0.0 {
                "0".to_owned()
            } else {
                format!("−{:.0}", amount.abs().floor())
            }
        };
        check_bullet(
            &output,
            "Civilians",
            &food_value(civilian_food),
            super::super::resource_hud::hud_delta_color(-1.0),
        );
        check_bullet(
            &output,
            "Military",
            &food_value(military_food),
            if military {
                super::super::resource_hud::hud_delta_color(-1.0)
            } else {
                super::super::resource_hud::hud_delta_color(0.0)
            },
        );
        for (class, name) in ["Nobles", "Citizens", "Plebeians", "Slaves"].into_iter().enumerate() {
            let amount = p.population[class]
                * campaign.economy.config.food_per_class[class]
                * campaign.economy.config.food_policy[p.policies.food as usize].consumption;
            check_bullet(
                &output,
                name,
                &food_value(amount),
                if amount.abs().floor() == 0.0 {
                    super::super::resource_hud::hud_delta_color(0.0)
                } else {
                    super::super::resource_hud::hud_delta_color(-1.0)
                },
            );
        }
        render(2.0, Some(population_pointer));
        render(3.0, None);
        let output = render(3.1, None);
        check_bullet(&output, "Area", &format!("{:.1}", p.capacity_area), egui::Color32::BLACK);
        check_bullet(
            &output,
            "Terrain modifier",
            &format!("{terrain:.1}"),
            if terrain == 1.0 {
                egui::Color32::BLACK
            } else {
                super::super::resource_hud::hud_delta_color(terrain - 1.0)
            },
        );
        for (label, amount) in [
            (
                "City",
                if city {
                    campaign.economy.config.city_capacity
                } else {
                    0.0
                },
            ),
            ("Buildings", p.building_effects(&campaign.economy.config).capacity),
        ] {
            check_bullet(
                &output,
                label,
                &if amount == 0.0 {
                    "0".into()
                } else {
                    format!("+{amount:.1}")
                },
                if amount == 0.0 {
                    egui::Color32::BLACK
                } else {
                    super::super::resource_hud::hud_delta_color(1.0)
                },
            );
        }
    }
}

#[test]
fn population_tooltips_show_signed_coloured_causes_and_capacity_on_the_badge_total() {
    use crate::game::economy::BuildingType;

    for (class, name) in ["Nobles", "Citizens", "Plebeians", "Slaves"].into_iter().enumerate() {
        let ctx = layout_context();
        ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
        let mut campaign = fixture();
        campaign.advance_month();
        let p = &mut campaign.economy.provinces[0];
        p.capacity_area = 1000.0;
        p.buildings[BuildingType::Temple as usize] = 1;
        p.happiness_modifiers[class] = -3.0;
        p.temporary_happiness[class] = 0.0;
        p.civic_happiness = 4.0;
        p.recruitment_happiness = -2.0;
        let report = &mut campaign.economy.last_report.province_reports[0];
        report.births[class] = 0.105;
        report.normal_deaths[class] = 0.06;
        report.famine_deaths[class] = 0.0;
        report.migration[class] = -0.006;
        report.food_supply_ratio = 1.0;
        let render = |time, pointer: Option<egui::Pos2>| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    time: Some(time),
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1200.0, 900.0),
                    )),
                    events: pointer.into_iter().map(egui::Event::PointerMoved).collect(),
                    ..Default::default()
                },
                |root| {
                    egui::CentralPanel::default().show(root, |ui| {
                        ui.set_width(546.0);
                        panel_style(ui, 1.0);
                        campaign_economy::overview(
                            ui,
                            &campaign.economy,
                            &campaign.military,
                            0,
                            1.0,
                        );
                    });
                },
            );
            output.textures_delta.clear();
            output
        };
        let baseline = render(0.0, None);
        let text_shapes = |output: &egui::FullOutput| {
            output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) => Some((text.pos, text.galley.clone())),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let text = text_shapes(&baseline);
        let row_y = text.iter().find(|(_, galley)| galley.job.text == name).unwrap().0.y;
        let mut columns: Vec<_> = text
            .iter()
            .filter(|(pos, _)| (pos.y - row_y).abs() < 1.0)
            .map(|(pos, galley)| egui::Rect::from_min_size(*pos, galley.size()).center())
            .collect();
        columns.sort_by(|a, b| a.x.total_cmp(&b.x));
        assert_eq!(columns.len(), 5);
        let growth = &text
            .iter()
            .filter(|(pos, _)| (pos.y - row_y).abs() < 1.0)
            .min_by(|left, right| {
                let left_distance = (left.0.x + left.1.size().x / 2.0 - columns[2].x).abs();
                let right_distance = (right.0.x + right.1.size().x / 2.0 - columns[2].x).abs();
                left_distance.total_cmp(&right_distance)
            })
            .unwrap()
            .1;
        assert_eq!(growth.job.text, "0");
        assert_eq!(growth.job.sections[0].format.color, egui::Color32::BLACK);
        let modifier = &text
            .iter()
            .filter(|(pos, _)| (pos.y - row_y).abs() < 1.0)
            .max_by(|left, right| left.0.x.total_cmp(&right.0.x))
            .unwrap()
            .1;
        let modifier_sign = if modifier.job.text.starts_with('+') {
            1.0
        } else if modifier.job.text.starts_with('-') {
            -1.0
        } else {
            assert_eq!(modifier.job.text, "0");
            0.0
        };
        assert_eq!(
            modifier.job.sections[0].format.color,
            if modifier_sign == 0.0 {
                egui::Color32::BLACK
            } else {
                super::super::resource_hud::hud_delta_color(modifier_sign)
            },
            "Happiness modifier should use the sign's color"
        );
        let check_bullet = |output: &egui::FullOutput,
                            title: &str,
                            value: &str,
                            color: egui::Color32| {
            let text = text_shapes(output);
            let label =
                text.iter().find(|(_, galley)| galley.job.text == format!("• {title}:")).unwrap();
            let right = label.0.x + label.1.size().x;
            let amount = text
                .iter()
                .find(|(pos, galley)| {
                    galley.job.text == value
                        && (pos.y - label.0.y).abs() < 1.0
                        && pos.x > right
                        && pos.x - right < 20.0
                })
                .unwrap();
            assert_eq!(
                amount.1.job.sections[0].format.color, color,
                "Wrong colour for {title}: {value}"
            );
        };
        render(0.1, Some(columns[2]));
        render(1.0, None);
        let growth = render(1.1, None);
        check_bullet(&growth, "Births", "+0.105", super::super::resource_hud::hud_delta_color(1.0));
        check_bullet(
            &growth,
            "Natural deaths",
            "-0.060",
            super::super::resource_hud::hud_delta_color(-1.0),
        );
        check_bullet(&growth, "Famine deaths", "0", egui::Color32::BLACK);
        check_bullet(
            &growth,
            "Migration",
            "-0.006",
            super::super::resource_hud::hud_delta_color(-1.0),
        );
        render(2.0, Some(columns[3]));
        render(3.0, None);
        let happiness = render(3.1, None);
        assert!(text_shapes(&happiness).iter().any(|(_, galley)| {
            galley.job.text.contains("happiness") && galley.job.text.contains("Current output")
        }));
        assert!(!text_shapes(&happiness)
            .iter()
            .any(|(_, galley)| galley.job.text.starts_with('•')));
        render(3.2, Some(columns[4]));
        render(4.2, None);
        let modifiers = render(4.3, None);
        check_bullet(
            &modifiers,
            "Buildings",
            "+2.0",
            super::super::resource_hud::hud_delta_color(1.0),
        );
        check_bullet(
            &modifiers,
            "Events / unrest",
            "-3.0",
            super::super::resource_hud::hud_delta_color(-1.0),
        );
        assert!(!text_shapes(&modifiers).iter().any(|(_, galley)| {
            ["Base happiness", "Food shortage", "Food policy"]
                .iter()
                .any(|removed| galley.job.text.contains(removed))
        }));
        if class != 3 {
            check_bullet(
                &modifiers,
                "Civic spending",
                "+4.0",
                super::super::resource_hud::hud_delta_color(1.0),
            );
        }
        if class == 1 || class == 2 {
            check_bullet(
                &modifiers,
                "Recruitment",
                "-2.0",
                super::super::resource_hud::hud_delta_color(-1.0),
            );
        }
        for output in [&growth, &happiness, &modifiers] {
            assert!(!text_shapes(output).iter().any(|(_, galley)| {
                ["Class promotions", "Space factor", "Birth factors", "never directly"]
                    .iter()
                    .any(|removed| galley.job.text.contains(removed))
            }));
        }
        let baseline_labels: Vec<_> =
            text.iter().map(|(_, galley)| galley.job.text.clone()).collect();
        let total = text.iter().find(|(_, galley)| galley.job.text.contains(" / ")).unwrap();
        for (time, pointer) in
            [(5.0, columns[1]), (7.0, egui::Rect::from_min_size(total.0, total.1.size()).center())]
        {
            render(time, Some(pointer));
            render(time + 1.0, None);
            let output = render(time + 1.1, None);
            let labels: Vec<_> =
                text_shapes(&output).iter().map(|(_, galley)| galley.job.text.clone()).collect();
            if time == 7.0 {
                assert!(labels.iter().any(|label| label == "Population capacity"));
            } else {
                assert_eq!(
                    labels, baseline_labels,
                    "Ledger population counts must not show tooltips"
                );
            }
        }
    }
}

#[test]
fn overview_groups_and_badges_keep_floor_counts_alignment_and_warning_colors() {
    for warning in [false, true] {
        let ctx = layout_context();
        ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
        let mut campaign = fixture();
        campaign.advance_month();
        campaign.economy.provinces[0].population = [11.9, 27.9, 53.9, 31.9];
        campaign.economy.provinces[0].capacity_area = if warning {
            0.0
        } else {
            1000.0
        };
        if warning {
            campaign.economy.provinces[0].has_city = false;
        }
        campaign.economy.last_report.province_reports[0].food_supply_ratio = if warning {
            0.996
        } else {
            1.0
        };
        let capacity = campaign.economy.provinces[0].capacity(&campaign.economy.config);
        let mut output = ctx.run_ui(egui::RawInput::default(), |root| {
            egui::CentralPanel::default().show(root, |ui| {
                ui.set_width(546.0);
                panel_style(ui, 1.0);
                campaign_economy::overview(ui, &campaign.economy, &campaign.military, 0, 1.0);
            });
        });
        output.textures_delta.clear();
        let texts: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text),
                _ => None,
            })
            .collect();
        let text = |label: &str| {
            *texts
                .iter()
                .find(|text| text.galley.job.text == label)
                .unwrap_or_else(|| panic!("Missing {label} in overview"))
        };
        let total = text(&format!("125 / {:.0}", capacity.floor()));
        assert!(text("Population").pos.y < total.pos.y);
        let food_demand = campaign.economy.provinces[0].food_request(&campaign.economy.config)
            + campaign.military.food_demand(ForceOwner::Player(0));
        text(&format!("−{:.0}/mo", food_demand.floor()));
        let supplied = text(if warning {
            "99%"
        } else {
            "100%"
        });
        assert!(text("Food supplied").pos.y < supplied.pos.y);
        for (value, normal_color) in [
            (total, province_panel::INK),
            (supplied, super::super::resource_hud::hud_delta_color(1.0)),
        ] {
            assert_eq!(
                value.galley.job.sections[0].format.color,
                if warning {
                    super::super::resource_hud::hud_delta_color(-1.0)
                } else {
                    normal_color
                }
            );
        }
        assert!(text("Civic Power").pos.y < text("Influence").pos.y);
        assert!(text("Influence").pos.y < text("Sestertius").pos.y);
        assert!(text("Sestertius").pos.y < text("Resources").pos.y);
        assert!(text("Resources").pos.y < text("Food").pos.y);
        for (name, count) in
            [("Nobles", "11"), ("Citizens", "27"), ("Plebeians", "53"), ("Slaves", "31")]
        {
            assert!(texts.iter().any(|value| value.galley.job.text == count
                && (value.pos.y - text(name).pos.y).abs() < 1.0));
            assert!(
                (text(name).pos.x - text("Food").pos.x).abs() < 1.0,
                "Names must share a left edge after their icons"
            );
        }
    }
}

#[test]
fn all_hud_ledgers_fit_with_a_full_atlas_of_owned_provinces() {
    let (campaign, ownership) = hover_card_fixture(true);
    for scale in [1.0, 0.85] {
        let ctx = layout_context();
        ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
        let (icons, class_icons) = hover_card_textures(&ctx);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 800.0));
        let mut hover = super::super::resource_hud::HudHoverState::default();
        for (index, x) in hud_resource_positions().into_iter().enumerate() {
            let date_left = super::super::map_menu::map_resource_strip_right(screen, scale)
                - MAP_DATE_SECTION_WIDTH;
            if x + HUD_RESOURCE_WIDTH > date_left - HUD_RESOURCE_GROUP_PADDING {
                continue;
            }
            let pointer = egui::pos2(x + 35.0, 22.0) * scale;
            for frame in 0..2 {
                let output = render_hover_cards(
                    &ctx,
                    &campaign,
                    &ownership,
                    &icons,
                    &class_icons,
                    &mut hover,
                    screen,
                    scale,
                    (index * 2 + frame) as f64 * 0.1,
                    pointer,
                );
                if frame > 0 {
                    let rect = hover_card_rect(&output).expect("Original HUD card must be painted");
                    assert!(
                        screen.contains_rect(rect),
                        "HUD card {index} exceeds the viewport: {rect:?}"
                    );
                }
            }
        }
    }
}

/// Measure actual panel bodies without substituting implementation-shaped mock widgets.
fn assert_body_fits(label: &str, width: f32, scale: f32, mut render: impl FnMut(&mut egui::Ui)) {
    let ctx = layout_context();
    let mut used_width: f32 = 0.0;
    let mut used_height: f32 = 0.0;
    // A second frame resolves font/layout cache and CollapsingHeader animation state.
    for frame in 0..2 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1600.0, 12_000.0),
            )),
            time: Some(frame as f64),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |root| {
            root.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 12_000.0),
                    ))
                    .layout(egui::Layout::top_down(egui::Align::Min)),
                |ui| {
                    ui.set_max_width(width);
                    panel_style(ui, scale);
                    let start = ui.next_widget_position().x;
                    render(ui);
                    used_width = used_width.max(ui.min_rect().right() - start);
                    used_height = used_height.max(ui.min_rect().height());
                },
            );
        });
        // Headless layout intentionally has no GPU; acknowledge atlas/icon deltas.
        output.textures_delta.clear();
        assert!(!output.shapes.is_empty(), "{label}: empty output would not verify a real panel");
    }
    assert!(used_height > 25.0, "{label}: the actual content was not laid out");
    assert!(
        used_width <= width + 1.5,
        "{label} overflow: used {used_width:.1}px inside {width:.1}px at scale {scale:.2}"
    );
}

#[test]
fn complete_panel_shell_bounds_long_headers_and_status_with_scrolled_content() {
    for (width, height, scale) in [(570.0, 720.0, 1.0), (404.0, 540.0, 0.85), (404.0, 400.0, 1.0)] {
        for tab in [CampaignTab::Province, CampaignTab::Senate, CampaignTab::Trade] {
            for status in [String::new(), "Political action completed. The affected province's relationship and control have changed; review the detailed modifiers and the next monthly resolution before committing more resources. ".repeat(20)] {
                let ctx = layout_context();
                let mut campaign = fixture();
                let mut view = CampaignUi { open: Some(tab), province: Some(4), notice: status.clone(), ..Default::default() };
                let rect = egui::Rect::from_min_size(egui::pos2(30.0, 30.0), egui::vec2(width, height));
                for frame in 0..2 {
                    let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 900.0))), time: Some(frame as f64), ..Default::default() };
                    let mut outer = rect;
                    let mut output = ctx.run_ui(input, |root| {
                        root.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                            panel_style(ui, scale);
                            outer = panel_frame(scale).show(ui, |ui| {
                                ui.set_width(width);
                                ui.set_min_height(height);
                                panel_header(ui, "Sardinia et Corsica · provincial administration", scale, &campaign.economy.provinces, 0, &mut view, None);
                                scroll_body(ui, rect.bottom() - 12.0 * scale - 1.2, "shell_test_body", |ui| {
                                    if tab == CampaignTab::Province {
                                        diplomacy(ui, &mut campaign, 4, 0, &[PLAYER_COLORS[0], PLAYER_COLORS[1]], &mut view);
                                    } else if tab == CampaignTab::Trade {
                                        campaign_trade::show(ui, &mut campaign, 0, scale);
                                    } else {
                                        campaign_politics::show(ui, &mut campaign.senate, &mut campaign.actors, &campaign.profiles, &campaign.senate_config, 0, &mut campaign.espionage, &campaign.espionage_config, &[PLAYER_COLORS[0], PLAYER_COLORS[1]]);
                                    }
                                });
                            }).response.rect;
                        });
                    });
                    output.textures_delta.clear();
                    if !status.is_empty() {
                        assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == status)), "Action text must not appear in the panel footer");
                    }
                    assert!(outer.right() <= rect.right() + 1.0, "{tab:?}: header/frame horizontal overflow at {width}px: {outer:?}");
                    assert!(outer.bottom() <= rect.bottom() + 1.0, "{tab:?}: frame/footer vertical overflow at {width}×{height}px: {outer:?}");
                }
            }
        }
    }
}

#[test]
fn diplomacy_legal_states_fit_desktop_and_compact_panels() {
    for (width, scale) in [(500.0, 1.0), (380.0, 0.85), (380.0, 1.0)] {
        for province in 0..5 {
            let mut campaign = fixture();
            assert_body_fits(&format!("Diplomacy province {province}"), width, scale, |ui| {
                diplomacy(
                    ui,
                    &mut campaign,
                    province,
                    0,
                    &[PLAYER_COLORS[0], PLAYER_COLORS[1]],
                    &mut CampaignUi::default(),
                );
            });
        }
    }
}

#[test]
fn full_control_diplomacy_shows_spy_and_political_actions() {
    for (width, scale) in [(500.0, 1.0), (380.0, 1.0), (380.0, 0.85)] {
        let mut campaign = fixture();
        campaign.politics[2].state = PoliticalState::Independent {
            local: 0.0,
            shares: vec![100.0, 0.0],
        };
        let mut view = CampaignUi::default();
        assert_body_fits("Full control diplomacy", width, scale, |ui| {
            diplomacy(ui, &mut campaign, 2, 0, &[PLAYER_COLORS[0], PLAYER_COLORS[1]], &mut view);
        });
        let ctx = layout_context();
        ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 1200.0),
                )),
                ..Default::default()
            },
            |ui| {
                panel_style(ui, scale);
                diplomacy(
                    ui,
                    &mut campaign,
                    2,
                    0,
                    &[PLAYER_COLORS[0], PLAYER_COLORS[1]],
                    &mut view,
                );
            },
        );
        let labels: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect();
        output.textures_delta.clear();
        for expected in [
            "Control",
            "Relation",
            "Vassalize",
            "Integrate",
            "Political distance ×1",
            "Build control",
            "Improve relations",
            "Uncover scandals",
            "Undermine foes",
            "Support revolt",
            "Discredit rivals",
            "POLITICAL MANEUVERS",
        ] {
            assert!(labels.contains(&expected), "Missing {expected:?} at {width}px: {labels:?}");
        }
        assert!(labels.contains(&"Bribe nobles"));
        assert!(labels.contains(&"Send insult"));
        let text_rect = |label| {
            output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.job.text == label => {
                        Some(text.galley.rect.translate(text.pos.to_vec2()))
                    },
                    _ => None,
                })
                .unwrap()
        };
        let spy_bottom = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Rect(card)
                    if card.fill == province_panel::TABLE_STRIPE
                        && card.rect.contains(text_rect("Discredit rivals").center()) =>
                {
                    Some(card.rect.bottom())
                },
                _ => None,
            })
            .unwrap();
        let section_gap = text_rect("POLITICAL MANEUVERS").top() - spy_bottom;
        assert!(section_gap <= 30.0 * scale, "Political section gap: {section_gap}");
        let bribe_cost = format!("{:.0}", campaign.noble_bribe_quote(0, 2).unwrap());
        for (title, cost) in [("Bribe nobles", bribe_cost.as_str()), ("Send insult", "Free")] {
            assert!(text_rect(cost).top() > text_rect(title).bottom());
        }
        for removed in [
            "+5 Control",
            "+5 Relation",
            "Monthly support",
            "Declare hostility",
            "Smear",
            "Opposition",
            "Undermine",
        ] {
            assert!(!labels.contains(&removed));
        }
        output.textures_delta.clear();
    }
}

#[test]
fn insult_cards_accept_npcs_and_show_the_target_effect_and_yearly_cooldown() {
    for (width, scale) in [(500.0, 1.0), (380.0, 0.85)] {
        for province in [1, 2, 3, 4] {
            let ctx = layout_context();
            ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
            let mut campaign = fixture();
            let mut view = CampaignUi::default();
            let mut pointer = None;
            let mut message = None;
            let description = if province == 1 {
                "Their provinces lose 10 Relation toward you. Once per year."
            } else {
                "This province loses 10 Relation toward you. Once per year."
            };
            for (frame, time) in
                [0.0, 0.1, 1.1, 1.2, 1.3, 1.4, 1.5, 2.5, 2.6].into_iter().enumerate()
            {
                let events = match frame {
                    1 => vec![egui::Event::PointerMoved(pointer.unwrap())],
                    4 | 5 => vec![egui::Event::PointerButton {
                        pos: pointer.unwrap(),
                        button: egui::PointerButton::Primary,
                        pressed: frame == 4,
                        modifiers: egui::Modifiers::NONE,
                    }],
                    6 => vec![egui::Event::PointerMoved(pointer.unwrap() + egui::vec2(1.0, 1.0))],
                    _ => vec![],
                };
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1000.0, 1000.0),
                        )),
                        time: Some(time),
                        events,
                        ..Default::default()
                    },
                    |root| {
                        root.scope_builder(
                            egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                                egui::pos2(20.0, 20.0),
                                egui::vec2(width, 900.0),
                            )),
                            |ui| {
                                panel_style(ui, scale);
                                message = diplomacy(
                                    ui,
                                    &mut campaign,
                                    province,
                                    0,
                                    &PLAYER_COLORS[..2],
                                    &mut view,
                                )
                                .or(message.take());
                            },
                        );
                    },
                );
                if frame == 0 {
                    pointer = Some(spy_text_position(&output, "Send insult"));
                }
                let labels: Vec<_> = output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                        _ => None,
                    })
                    .collect();
                if frame == 3 || frame == 8 {
                    assert!(labels.contains(&description), "Missing insult effect at frame {frame}, province {province}, width {width}: {labels:?}");
                    assert!(!labels.iter().any(|label| label.contains("requires")));
                }
                if frame == 8 {
                    assert!(
                        labels.contains(&"Try again in 12 months."),
                        "Missing cooldown: {labels:?}"
                    );
                }
                output.textures_delta.clear();
            }
            let expected = if province == 1 {
                "Player 2 insulted: −10 Relation in their provinces.".to_owned()
            } else {
                format!("{} insulted: −10 Relation.", campaign.economy.provinces[province].name)
            };
            assert_eq!(message, Some(expected), "Insult card must accept province {province}");
            assert_eq!(campaign.politics[province].relation(0), 40.0);
            assert_eq!(campaign.diplomacy_used.get(&(0, true)), Some(&0));
        }
    }
}

#[test]
fn own_diplomacy_shows_unboxed_player_rows_and_immediate_access_notices() {
    let ctx = layout_context();
    ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
    let mut campaign = fixture();
    campaign.actors.resize(4, PoliticalPlayer::default());
    campaign.wars = vec![vec![false; 4]; 4];
    campaign.invitations = vec![vec![false; 4]; 4];
    let colors = [PLAYER_COLORS[2], PLAYER_COLORS[3], PLAYER_COLORS[4], PLAYER_COLORS[0]];
    let mut pointer = None;
    for frame in 0..5 {
        let events = match frame {
            2 => vec![
                egui::Event::PointerMoved(pointer.unwrap()),
                egui::Event::PointerButton {
                    pos: pointer.unwrap(),
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            3 => vec![egui::Event::PointerButton {
                pos: pointer.unwrap(),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            _ => vec![],
        };
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(570.0, 720.0),
                )),
                time: Some(if frame == 4 {
                    2.0
                } else {
                    frame as f64 * 0.1
                }),
                events,
                ..Default::default()
            },
            |ui| {
                panel_style(ui, 1.0);
                diplomacy(ui, &mut campaign, 0, 0, &colors, &mut CampaignUi::default());
            },
        );
        output.textures_delta.clear();
        let texts: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text),
                _ => None,
            })
            .collect();
        assert_eq!(
            texts.len(),
            8,
            "The heading, description, players and access states should appear"
        );
        assert!(texts.iter().any(|text| text.galley.job.text == "Military Access"));
        assert!(texts.iter().any(|text| {
            text.galley.job.text.contains("foreign units march to the closest own province")
        }));
        let description = texts
            .iter()
            .find(|text| {
                text.galley.job.text.contains("foreign units march to the closest own province")
            })
            .unwrap();
        if frame == 2 || frame == 3 {
            assert!(
                output.shapes.iter().any(|shape| matches!(
                    &shape.shape,
                    egui::Shape::Rect(rect)
                        if rect.fill == egui::Color32::from_rgb(222, 202, 174)
                )),
                "Clicking an access row should briefly change its color"
            );
        }
        let mut previous_row = description.pos.y;
        for player in 1..4 {
            let text = texts
                .iter()
                .find(|text| text.galley.job.text == format!("Player {}", player + 1))
                .unwrap();
            assert_eq!(text.galley.job.sections[0].format.color, colors[player]);
            assert!(
                text.pos.x >= description.pos.x && text.pos.y > previous_row,
                "Player names should form a readable vertical list"
            );
            previous_row = text.pos.y;
            if player == 1 {
                pointer = Some(text.galley.rect.translate(text.pos.to_vec2()).center());
            }
        }
        assert!(texts.iter().any(|text| text.galley.job.text
            == if frame >= 3 {
                "Granted"
            } else {
                "Closed"
            }));
        assert!(
            !output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Rect(rect) if rect.fill == egui::Color32::from_rgb(247, 243, 232)
                && texts.iter().any(|text| text.galley.job.text.starts_with("Player ")
                    && rect.rect.contains_rect(text.galley.rect.translate(text.pos.to_vec2()))))),
            "Military access rows should not have individual pale cards"
        );
    }
    assert!(campaign.province_access_granted(0, 0, 1));
    let notices = campaign.notifications.drain_for(1);
    assert_eq!(notices.len(), 1);
    assert_eq!(notices[0].title, "Military access granted to Italia");
}

#[test]
fn senate_neutral_loyalty_and_consul_states_fit_desktop_and_compact_panels() {
    for (width, scale) in [(500.0, 1.0), (380.0, 0.85), (380.0, 1.0)] {
        for phase in 0..3 {
            let mut campaign = fixture();
            if phase > 0 {
                for (id, senator) in campaign.senate.senators.iter_mut().enumerate() {
                    senator.allegiance = if id % 3 == 0 {
                        None
                    } else {
                        Some(id % 2)
                    };
                }
            }
            if phase == 2 {
                campaign.actors[0].rank = PoliticalRank::Consul;
                campaign.actors[0].consul_until = Some(24);
            }
            assert_body_fits(&format!("Senate phase {phase}"), width, scale, |ui| {
                campaign_politics::show(
                    ui,
                    &mut campaign.senate,
                    &mut campaign.actors,
                    &campaign.profiles,
                    &campaign.senate_config,
                    0,
                    &mut campaign.espionage,
                    &campaign.espionage_config,
                    &[PLAYER_COLORS[0], PLAYER_COLORS[1]],
                );
            });
        }
    }
}

#[test]
fn rome_city_and_province_selection_open_senate_and_other_selections_stay_contextual() {
    let mut c = fixture();
    c.economy.provinces[2].name = "Latium".into();
    c.politics[2] = ProvincePolitics::rome(2);
    let mut view = CampaignUi::default();
    for selected in [MapDetail::Province(2), MapDetail::City(2)] {
        view.select_map_detail(selected, &c);
        assert_eq!(view.open, Some(CampaignTab::Senate));
        assert_eq!(view.province, Some(2));
        assert_eq!(view.last_detail, Some(selected));
    }
    view.select_map_detail(MapDetail::Province(0), &c);
    assert_eq!(view.open, Some(CampaignTab::Province));
    assert_eq!(view.section, 0);
    view.select_map_detail(MapDetail::City(0), &c);
    assert_eq!(view.section, 0, "City selection must retain the remembered province tab");
}

#[test]
fn province_selection_remembers_each_tab_across_map_changes_and_reopening() {
    let mut campaign = fixture();
    campaign.economy.provinces[2].name = "Latium".into();
    for section in 0..6 {
        let mut view = CampaignUi::default();
        view.open_province_section(0, section);
        for selected in [MapDetail::Province(1), MapDetail::City(3), MapDetail::Province(2)] {
            view.select_map_detail(selected, &campaign);
            assert_eq!(view.section, section);
        }
        assert_eq!(view.open, Some(CampaignTab::Senate));
        view.open = None;
        view.select_map_detail(MapDetail::Province(4), &campaign);
        assert_eq!(view.open, Some(CampaignTab::Province));
        assert_eq!(view.section, section);
    }
    let mut view = CampaignUi::default();
    view.select_map_detail(MapDetail::City(0), &campaign);
    assert_eq!(view.section, 2, "The first city selection still opens Buildings");
}

#[test]
fn economy_overview_policies_buildings_and_trade_fit_compact_panels() {
    for (width, scale) in [(500.0, 1.0), (380.0, 0.85), (380.0, 1.0)] {
        for section in 0..4 {
            let mut campaign = fixture();
            assert_body_fits(
                &format!("Economy section {section}"),
                width,
                scale,
                |ui| match section {
                    0 => {
                        province_overview(ui, &campaign, 0, 0, scale);
                    },
                    1 => campaign_economy::policies(ui, &mut campaign.economy, 0, 0),
                    2 => {
                        campaign_economy::buildings(ui, &mut campaign.economy, 0, 0, scale);
                    },
                    _ => {
                        campaign_trade::show(ui, &mut campaign, 0, scale);
                    },
                },
            );
        }
    }
}

#[test]
fn national_trade_routes_composer_and_market_fit_compact_widths() {
    use crate::game::economy::{TradeAgreement, TradeBundle, TradeFrequency, TradeParty};
    for (width, scale) in [(546.0, 1.0), (380.0, 0.85), (380.0, 1.0)] {
        for page in 0..3 {
            let mut campaign = fixture();
            let agreement = |party| {
                TradeAgreement::new(
                    TradeParty::Player(0),
                    party,
                    TradeBundle {
                        coin: 5.0,
                        ..Default::default()
                    },
                    TradeBundle {
                        resources: [0.0, 1.0, 0.0],
                        ..Default::default()
                    },
                    TradeFrequency::Monthly,
                )
            };
            let id = campaign.propose_national_trade(agreement(TradeParty::Npc(2))).unwrap();
            campaign.end_trade(0, id, true).unwrap();
            campaign.propose_national_trade(agreement(TradeParty::Npc(3))).unwrap();
            let id = campaign.propose_national_trade(agreement(TradeParty::Player(1))).unwrap();
            campaign.accept_national_trade(1, id).unwrap();
            campaign.propose_national_trade(agreement(TradeParty::Player(1))).unwrap();
            assert_body_fits(&format!("National trade page {page}"), width, scale, |ui| {
                campaign_trade::select_page(ui.ctx(), 0, page);
                campaign_trade::show(ui, &mut campaign, 0, scale);
            });
        }
    }
}

#[test]
fn province_trade_composer_fits_and_targets_the_selected_owner_or_npc() {
    for (width, scale) in [(546.0, 1.0), (380.0, 0.85), (380.0, 1.0)] {
        for province in 1..5 {
            let mut campaign = fixture();
            assert_body_fits(&format!("Province trade {province}"), width, scale, |ui| {
                campaign_trade::show_province(ui, &mut campaign, province, 0, scale);
            });
        }
    }
}

#[test]
fn national_trade_cancellation_updates_authoritative_relation_and_keeps_control() {
    use crate::game::economy::{TradeAgreement, TradeBundle, TradeFrequency, TradeParty};
    let mut campaign = fixture();
    let relation = campaign.politics[2].relation(0);
    let control = campaign.politics[2].independent_control(0);
    let id = campaign
        .propose_national_trade(TradeAgreement::new(
            TradeParty::Player(0),
            TradeParty::Npc(2),
            TradeBundle {
                coin: 5.0,
                ..Default::default()
            },
            TradeBundle {
                resources: [0.0, 1.0, 0.0],
                ..Default::default()
            },
            TradeFrequency::Monthly,
        ))
        .unwrap();
    campaign.end_trade(0, id, true).unwrap();
    assert_eq!(campaign.politics[2].relation(0), relation);
    assert_eq!(campaign.actors[0].influence, 1000.0);
    campaign.end_trade(0, id, false).unwrap();
    assert_eq!(campaign.actors[0].influence, 990.0);
    assert_eq!(campaign.economy.players[0].influence, 990.0);
    assert_eq!(campaign.senate.accusations.len(), 1);
    assert_eq!(
        campaign.politics[2].relation(0),
        relation - campaign.economy.config.trade.cancellation_relation_penalty
    );
    assert_eq!(
        campaign.economy.provinces[2].relation_by_player[0],
        campaign.politics[2].relation(0)
    );
    assert_eq!(campaign.politics[2].independent_control(0), control);
}

#[test]
fn player_trade_notice_is_penalty_free_and_immediate_stop_hurts_partner_relations() {
    use crate::game::economy::{
        TradeAgreement, TradeBundle, TradeFrequency, TradeParty, TradeStatus,
    };
    let mut campaign = fixture();
    let agreement = || {
        TradeAgreement::new(
            TradeParty::Player(0),
            TradeParty::Player(1),
            TradeBundle {
                coin: 5.0,
                ..Default::default()
            },
            TradeBundle {
                resources: [0.0, 1.0, 0.0],
                ..Default::default()
            },
            TradeFrequency::Monthly,
        )
    };
    let id = campaign.propose_national_trade(agreement()).unwrap();
    campaign.accept_national_trade(1, id).unwrap();
    let relation = campaign.politics[1].relation(0);
    campaign.end_trade(0, id, true).unwrap();
    assert_eq!(
        campaign.economy.trades.iter().find(|trade| trade.id == id).unwrap().cancellation_month,
        Some(6)
    );
    assert_eq!(campaign.politics[1].relation(0), relation);
    assert_eq!(campaign.actors[0].influence, 1000.0);
    campaign.end_trade(0, id, false).unwrap();
    assert_eq!(
        campaign.economy.trades.iter().find(|trade| trade.id == id).unwrap().status,
        TradeStatus::Cancelled
    );
    assert_eq!(campaign.politics[1].relation(0), relation - 10.0);
    assert_eq!(campaign.actors[0].influence, 990.0);
}

#[test]
fn route_icon_buttons_give_notice_then_apply_the_immediate_cancellation_penalty() {
    use crate::game::economy::{
        TradeAgreement, TradeBundle, TradeFrequency, TradeParty, TradeStatus,
    };
    fn render(
        ctx: &egui::Context,
        campaign: &mut Campaign,
        time: f64,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
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
                        egui::pos2(20.0, 20.0),
                        egui::vec2(380.0, 600.0),
                    )),
                    |ui| {
                        panel_style(ui, 1.0);
                        campaign_trade::show(ui, campaign, 0, 1.0);
                    },
                );
            },
        );
        output.textures_delta.clear();
        output
    }
    let ctx = layout_context();
    ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
    let mut campaign = fixture();
    campaign
        .propose_national_trade(TradeAgreement::new(
            TradeParty::Player(0),
            TradeParty::Npc(2),
            TradeBundle {
                coin: 5.0,
                ..Default::default()
            },
            TradeBundle {
                resources: [0.0, 1.0, 0.0],
                ..Default::default()
            },
            TradeFrequency::Monthly,
        ))
        .unwrap();
    let output = render(&ctx, &mut campaign, 0.0, vec![]);
    let buttons: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect)
                if rect.fill == province_panel::PAPER
                    && rect.rect.size() == egui::vec2(30.0, 30.0) =>
            {
                Some(rect.rect.center())
            },
            _ => None,
        })
        .collect();
    assert_eq!(buttons.len(), 2, "Each open route must have two icon actions");
    let relation = campaign.politics[2].relation(0);
    for (action, position) in [(0, buttons[1]), (1, buttons[0])] {
        for pressed in [true, false] {
            render(
                &ctx,
                &mut campaign,
                0.1 + f64::from(action) * 0.2
                    + if pressed {
                        0.0
                    } else {
                        0.1
                    },
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
        if action == 0 {
            assert_eq!(campaign.economy.trades[0].cancellation_month, Some(6));
            assert_eq!(campaign.politics[2].relation(0), relation);
        } else {
            assert_eq!(campaign.economy.trades[0].status, TradeStatus::Cancelled);
            assert_eq!(campaign.politics[2].relation(0), relation - 10.0);
        }
    }
}

#[test]
fn province_overview_fits_each_political_state_at_compact_widths() {
    let campaign = fixture();
    for (width, scale) in [(500.0, 1.0), (380.0, 0.85), (380.0, 1.0)] {
        for province in 0..campaign.politics.len() {
            assert_body_fits("Province overview", width, scale, |ui| {
                province_overview(ui, &campaign, province, 0, scale);
            });
        }
    }
}

#[test]
fn complete_province_overview_fits_without_scrolling_and_aligns_badges_and_ledgers() {
    use crate::game::economy::{BuildingProject, BuildingType, ConstructionProject};
    for (width, height, scale) in [(570.0, 720.0, 1.0), (404.0, 540.0, 0.85), (404.0, 400.0, 1.0)] {
        for province in 0..5 {
            for construction in [false, true] {
                let ctx = layout_context();
                ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
                let mut campaign = fixture();
                campaign.advance_month();
                if construction {
                    campaign.economy.provinces[province].construction =
                        Some(ConstructionProject::Building(BuildingProject {
                            building: BuildingType::Granary,
                            target_level: 1,
                            progress: 1.0,
                            required_progress: 4.0,
                            paid_metal: 0.0,
                            paid_stone: 0.0,
                        }));
                }
                let mut view = CampaignUi {
                    open: Some(CampaignTab::Province),
                    province: Some(province),
                    ..Default::default()
                };
                let rect =
                    egui::Rect::from_min_size(egui::pos2(30.0, 30.0), egui::vec2(width, height));
                for frame in 0..2 {
                    let mut outer = rect;
                    let mut body = rect;
                    let mut output = ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(1200.0, 900.0),
                            )),
                            time: Some(frame as f64),
                            ..Default::default()
                        },
                        |root| {
                            root.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                                panel_style(ui, scale);
                                outer = panel_frame(scale)
                                    .show(ui, |ui| {
                                        ui.set_width(width);
                                        ui.set_min_height(height);
                                        panel_header(
                                            ui,
                                            &campaign.economy.provinces[province].name,
                                            scale,
                                            &campaign.economy.provinces,
                                            0,
                                            &mut view,
                                            None,
                                        );
                                        egui::Frame::new()
                                            .inner_margin(egui::Margin {
                                                left: (12.0 * scale).round() as i8,
                                                right: (12.0 * scale).round() as i8,
                                                top: 0,
                                                bottom: (12.0 * scale).round() as i8,
                                            })
                                            .show(ui, |ui| {
                                                province_overview_body(
                                                    ui,
                                                    rect.bottom() - 12.0 * scale,
                                                    &campaign,
                                                    province,
                                                    0,
                                                    scale,
                                                );
                                                body = ui.min_rect();
                                            });
                                    })
                                    .response
                                    .rect;
                            });
                        },
                    );
                    output.textures_delta.clear();
                    assert!(
                        outer.right() <= rect.right() + 1.0
                            && outer.bottom() <= rect.bottom() + 1.0,
                        "Overview exceeds panel {width}×{height}, province {province}, construction={construction}: {outer:?}"
                    );
                    let text_rect = |label: &str| {
                        output
                            .shapes
                            .iter()
                            .find_map(|shape| {
                                if let egui::Shape::Text(text) = &shape.shape {
                                    (text.galley.job.text == label && text.pos.y >= body.top())
                                        .then_some(egui::Rect::from_min_size(
                                            text.pos,
                                            text.galley.size(),
                                        ))
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_else(|| panic!("Missing overview text {label}"))
                    };
                    if campaign.economy.provinces[province].owner == Some(0) {
                        let demand = text_rect("Food demand");
                        let supplied = text_rect("Food supplied");
                        assert!(
                            (demand.center().y - supplied.center().y).abs() < 1.0
                                && demand.right() < supplied.left(),
                            "Summary badges must stay on one horizontal row"
                        );
                        let row = text_rect("Nobles");
                        let values: Vec<_> = output
                            .shapes
                            .iter()
                            .filter_map(|shape| {
                                if let egui::Shape::Text(text) = &shape.shape {
                                    let rect =
                                        egui::Rect::from_min_size(text.pos, text.galley.size());
                                    ((rect.center().y - row.center().y).abs() < 1.0)
                                        .then_some((rect.center().x, text.galley.job.text.as_str()))
                                } else {
                                    None
                                }
                            })
                            .collect();
                        assert_eq!(
                            values.len(),
                            5,
                            "Population row must contain a name, total, growth, happiness and happiness modifiers: {values:?}"
                        );
                        let mut values = values;
                        values.sort_by(|a, b| a.0.total_cmp(&b.0));
                        assert!(
                            values[2].1 == "0" || values[2].1.starts_with(['+', '-']),
                            "Monthly population growth must precede happiness: {values:?}"
                        );
                        assert!(
                            values[3].1.parse::<u32>().is_ok(),
                            "Happiness must be a number without a percent sign: {values:?}"
                        );
                        assert!(
                            values[4].1 == "0" || values[4].1.starts_with(['+', '-']),
                            "Happiness modifiers must be signed: {values:?}"
                        );
                        assert!(
                            values[4].0 > body.left() + body.width() * 0.85,
                            "Ledger must use the right side of the panel"
                        );
                        for (resource, name) in ["Food", "Metal", "Stone"].iter().enumerate() {
                            let row = text_rect(name);
                            let mut values: Vec<_> = output
                                .shapes
                                .iter()
                                .filter_map(|shape| {
                                    if let egui::Shape::Text(text) = &shape.shape {
                                        let rect =
                                            egui::Rect::from_min_size(text.pos, text.galley.size());
                                        ((rect.center().y - row.center().y).abs() < 1.0)
                                            .then_some((rect, text.galley.job.text.as_str()))
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            values.sort_by(|a, b| a.0.center().x.total_cmp(&b.0.center().x));
                            assert_eq!(
                                values.len(),
                                3,
                                "Resource row must contain name, base and monthly output: {values:?}"
                            );
                            assert_eq!(
                                values[1].1,
                                campaign.economy.provinces[province].potential[resource]
                                    .to_string()
                            );
                            assert!(
                                values[2].1 == "0" || values[2].1.starts_with('+'),
                                "Monthly production must be last: {values:?}"
                            );
                            for pair in values.windows(2) {
                                assert!(
                                    pair[0].0.right() < pair[1].0.left(),
                                    "Resource columns overlap: {values:?}"
                                );
                            }
                        }
                    } else {
                        text_rect("Population");
                        for caption in ["Food demand", "Food supplied"] {
                            assert!(
                                !output.shapes.iter().any(|shape| matches!(&shape.shape,
                                egui::Shape::Text(text) if text.galley.job.text == caption)),
                                "Foreign province must hide {caption}"
                            );
                        }
                        if campaign.economy.provinces[province].owner.is_some() {
                            text_rect("Civic Power");
                        }
                    }
                    for shape in &output.shapes {
                        if let egui::Shape::Text(text) = &shape.shape {
                            if text.pos.y >= body.top() {
                                let text_bounds =
                                    egui::Rect::from_min_size(text.pos, text.galley.size());
                                assert!(
                                    text_bounds.right() <= body.right() + 1.0
                                        && text_bounds.bottom() <= rect.bottom() - 10.0 * scale,
                                    "Overview text must remain visible: {} {text_bounds:?}",
                                    text.galley.job.text
                                );
                                assert_ne!(text.galley.job.text, "Recent notifications");
                                assert!(
                                    !text.galley.job.text.contains(" · Lv "),
                                    "Construction summary must stay off the overview"
                                );
                            }
                        }
                    }
                    output.textures_delta.clear();
                }
            }
        }
    }
}

#[test]
fn campaign_symbols_use_transparent_art_without_circle_frames() {
    let ctx = layout_context();
    let symbols = [
        Icon::Amount,
        Icon::BaseAmount,
        Icon::Delta,
        Icon::Change,
        Icon::Notifications,
        Icon::Control,
        Icon::Relation,
        Icon::Diplomacy,
        Icon::Policies,
        Icon::Construction,
        Icon::Cancel,
        Icon::Notice,
        Icon::Confirm,
        Icon::SpyBuildControl,
        Icon::SpyImproveRelations,
        Icon::SpyUncoverScandals,
        Icon::SpyUndermineOpponents,
    ];
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        ui.horizontal(|ui| {
            for symbol in symbols {
                icon(ui, symbol, 32.0);
            }
        });
    });
    assert!(
        !output.shapes.iter().any(|shape| matches!(shape.shape, egui::Shape::Circle(_))),
        "Campaign icon rendering must not add circle frames"
    );
    let images: Vec<_> = output
        .textures_delta
        .set
        .iter()
        .flat_map(|(_, deltas)| deltas.iter())
        .filter_map(|delta| {
            let egui::ImageData::Color(image) = &delta.image;
            (image.size == [64, 64]).then_some(image)
        })
        .collect();
    assert_eq!(images.len(), symbols.len(), "Each semantic icon must load its own prepared art");
    for image in images {
        for corner in [0, 63, 64 * 63, 64 * 64 - 1] {
            assert_eq!(image.pixels[corner].a(), 0, "Icon corners must remain transparent");
        }
        assert!(image.pixels.iter().any(|pixel| pixel.a() > 128), "Icon art must be visible");
    }
    output.textures_delta.clear();
}

#[test]
fn province_policy_choices_apply_only_for_the_direct_owner() {
    use crate::game::economy::ResourceFocus;
    for owner in [0, 1] {
        let ctx = layout_context();
        ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
        let mut campaign = fixture();
        campaign.economy.provinces[0].owner = Some(owner);
        let mut render = |time, events| {
            ctx.run_ui(
                egui::RawInput {
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |root| {
                    root.scope_builder(
                        egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                            egui::pos2(20.0, 20.0),
                            egui::vec2(546.0, 480.0),
                        )),
                        |ui| {
                            panel_style(ui, 1.0);
                            campaign_economy::policies(ui, &mut campaign.economy, 0, 0);
                        },
                    );
                },
            )
        };
        let mut output = render(0.0, vec![]);
        let position = output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape {
                    (text.galley.job.text == "Food").then_some(text.pos + egui::vec2(4.0, 4.0))
                } else {
                    None
                }
            })
            .expect("Food policy choice");
        output.textures_delta.clear();
        for (time, pressed) in [(0.1, true), (0.2, false)] {
            let mut output = render(
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
        }
        drop(render);
        assert_eq!(
            campaign.economy.provinces[0].policies.focus,
            if owner == 0 {
                ResourceFocus::Food
            } else {
                ResourceFocus::Balanced
            }
        );
    }
}

#[test]
fn province_policy_sections_and_new_choices_are_available_only_to_the_direct_owner() {
    use crate::game::economy::{
        CivicSpending, ConstructionPace, ManumissionPolicy, RecruitmentEffort, ResourceFocus,
    };
    for owner in [0, 1] {
        let ctx = layout_context();
        ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
        let mut campaign = fixture();
        campaign.economy.provinces[0].owner = Some(owner);
        campaign.economy.provinces[0].policies.focus = ResourceFocus::Food;
        let mut render = |time, events| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    time: Some(time),
                    events,
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1200.0, 1100.0),
                    )),
                    ..Default::default()
                },
                |root| {
                    root.scope_builder(
                        egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                            egui::pos2(20.0, 20.0),
                            egui::vec2(546.0, 900.0),
                        )),
                        |ui| {
                            panel_style(ui, 1.0);
                            campaign_economy::policies(ui, &mut campaign.economy, 0, 0);
                        },
                    );
                },
            );
            output.textures_delta.clear();
            output
        };
        let output = render(0.0, vec![]);
        let labels: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect();
        for label in [
            "ECONOMY & WORKS",
            "POPULATION & WELFARE",
            "MILITARY",
            "Resource Focus",
            "Construction Pace",
            "Migration Focus",
            "Civic Spending",
            "Manumission",
            "Recruitment Effort",
            "Balanced",
        ] {
            assert!(labels.contains(&label), "Missing local policy label: {label}");
        }
        assert!(!labels.contains(&"All"));
        assert!(!labels.contains(&"PROVINCE POLICIES"));
        for (index, label) in
            ["Balanced", "Urgent", "Generous", "Free", "High"].into_iter().enumerate()
        {
            let output = render(index as f64 + 0.1, vec![]);
            let position = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.job.text == label => {
                        Some(text.pos + egui::vec2(3.0, 3.0))
                    },
                    _ => None,
                })
                .unwrap_or_else(|| panic!("Missing choice: {label}"));
            for (offset, pressed) in [(0.2, true), (0.3, false)] {
                render(
                    index as f64 + offset,
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
        }
        drop(render);
        let policies = campaign.economy.provinces[0].policies;
        if owner == 0 {
            assert_eq!(policies.focus, ResourceFocus::Balanced);
            assert_eq!(policies.construction, ConstructionPace::Urgent);
            assert_eq!(policies.civic_spending, CivicSpending::Generous);
            assert_eq!(policies.manumission, ManumissionPolicy::Free);
            assert_eq!(policies.recruitment, RecruitmentEffort::High);
        } else {
            assert_eq!(policies.focus, ResourceFocus::Food);
            assert_eq!(policies.construction, ConstructionPace::Normal);
            assert_eq!(policies.civic_spending, CivicSpending::Normal);
            assert_eq!(policies.manumission, ManumissionPolicy::Normal);
            assert_eq!(policies.recruitment, RecruitmentEffort::Normal);
        }
    }
}

#[test]
fn province_policy_scroll_reaches_military_without_shrinking_the_cards() {
    let ctx = layout_context();
    ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
    let mut campaign = fixture();
    campaign.economy.provinces[0].policies.recruitment =
        crate::game::economy::RecruitmentEffort::High;
    let rect = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(380.0, 240.0));
    let mut render = |time, events| {
        let mut output = ctx.run_ui(
            egui::RawInput {
                time: Some(time),
                events,
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 900.0),
                )),
                ..Default::default()
            },
            |root| {
                root.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                    panel_style(ui, 1.0);
                    campaign_economy::policies(ui, &mut campaign.economy, 0, 0);
                    assert!(ui.min_rect().bottom() <= rect.bottom() + 1.0);
                });
            },
        );
        output.textures_delta.clear();
        output
    };
    let visible = |output: &egui::FullOutput, label: &str| {
        output.shapes.iter().any(|s| match &s.shape {
            egui::Shape::Text(text) => {
                text.galley.job.text == label
                    && s.clip_rect
                        .intersects(egui::Rect::from_min_size(text.pos, text.galley.size()))
            },
            _ => false,
        })
    };
    let first = render(0.0, vec![]);
    assert!(!visible(&first, "Recruitment Effort"));
    render(
        0.1,
        vec![
            egui::Event::PointerMoved(rect.center()),
            egui::Event::MouseWheel {
                phase: egui::TouchPhase::Move,
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -1000.0),
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    let mut last = render(0.2, vec![]);
    for frame in 3..15 {
        last = render(frame as f64 * 0.1, vec![]);
    }
    assert!(visible(&last, "Recruitment Effort"), "Scrolling must expose the final policy");
    assert!(visible(&last, "10"), "High recruitment must display its monthly cost");
}

#[test]
fn province_policy_cards_fit_and_keep_nationwide_edicts_out_of_local_controls() {
    for (width, height, scale) in [(546.0, 480.0, 1.0), (380.0, 360.0, 0.85), (380.0, 240.0, 1.0)] {
        let ctx = layout_context();
        ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
        let mut campaign = fixture();
        let rect = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(width, height));
        let mut used = rect;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 900.0),
                )),
                ..Default::default()
            },
            |root| {
                root.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                    panel_style(ui, scale);
                    campaign_economy::policies(ui, &mut campaign.economy, 0, 0);
                    used = ui.min_rect();
                });
            },
        );
        output.textures_delta.clear();
        assert!(
            used.right() <= rect.right() + 1.0 && used.bottom() <= rect.bottom() + 1.0,
            "Province policy cards overflow {width}×{height}: {used:?}"
        );
        let labels: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect();
        for title in ["Food Rations", "Slave Labor", "Noble Taxes", "Noble Wages", "Army Wages"] {
            assert!(!labels.contains(&title), "Nationwide edict must stay in Governance: {title}");
        }
        for title in ["Resource Focus", "Migration Focus"] {
            assert!(labels.contains(&title), "Missing local policy: {title}");
        }
        assert!(!labels.contains(&"MONTHLY PREVIEW"), "Policies must not show a preview");
        for shape in &output.shapes {
            if let egui::Shape::Text(text) = &shape.shape {
                let bounds = egui::Rect::from_min_size(text.pos, text.galley.size());
                assert!(
                    bounds.left() >= rect.left() - 1.0 && bounds.right() <= rect.right() + 1.0,
                    "Policy text must fit: {} {bounds:?}",
                    text.galley.job.text
                );
            }
        }
    }
}

#[test]
fn province_notice_cards_filter_history_and_click_through_to_the_related_panel() {
    use super::super::campaign_notifications::NoticeKind;
    for (kind, section) in [
        (NoticeKind::BuildingCompleted, 2),
        (NoticeKind::RecruitmentCompleted, 3),
        (NoticeKind::HostileRelation, 4),
        (NoticeKind::TradeInterrupted, 5),
        (NoticeKind::FoodShortage, 1),
    ] {
        let ctx = layout_context();
        ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
        let mut campaign = fixture();
        campaign.notifications.province_notice(
            0,
            0,
            1,
            NoticeSeverity::Warning,
            kind,
            "Visible notice",
            "Details about this province's event.",
        );
        campaign.notifications.province_notice(
            1,
            0,
            1,
            NoticeSeverity::Info,
            kind,
            "Private notice",
            "Another player's private event.",
        );
        campaign.notifications.province_notice(
            0,
            1,
            1,
            NoticeSeverity::Info,
            kind,
            "Elsewhere notice",
            "An event from another province.",
        );
        let rect = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(380.0, 260.0));
        let mut selected = None;
        let mut filters = campaign_notices::NoticeFilters::default();
        let mut render = |time, events| {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1000.0, 700.0),
                    )),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |root| {
                    root.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                        panel_style(ui, 1.0);
                        if let Some(notice) =
                            province_notifications(ui, &campaign, 0, 0, &mut filters, 1.0)
                        {
                            selected = Some(notice);
                        }
                    });
                },
            )
        };
        let mut output = render(0.0, vec![]);
        output.textures_delta.clear();
        let labels: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape {
                    Some(text.galley.job.text.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert!(labels.contains(&"Visible notice") && labels.contains(&"Feb 60 AD"));
        assert!(!labels.contains(&"Private notice") && !labels.contains(&"Elsewhere notice"));
        let position = output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape {
                    (text.galley.job.text == "Visible notice")
                        .then_some(text.pos + egui::vec2(5.0, 5.0))
                } else {
                    None
                }
            })
            .unwrap();
        for (time, pressed) in [(0.1, true), (0.2, false)] {
            let mut output = render(
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
        }
        let notice = selected.expect("The entire notification card must be clickable");
        let mut view = CampaignUi {
            open: Some(CampaignTab::Province),
            province: Some(0),
            section: 5,
            ..Default::default()
        };
        let mut detail = ProvincePanelOpen(Some(MapDetail::Province(0)));
        let mut map = MapView::default();
        open_notification(&mut view, &notice, &campaign, &mut detail, &mut map);
        if kind == NoticeKind::FoodShortage {
            assert_eq!(view.open, Some(CampaignTab::Province));
            assert_eq!(view.section, 0);
            assert_eq!(detail.0, Some(MapDetail::Province(0)));
        } else if kind == NoticeKind::TradeInterrupted {
            assert_eq!(view.open, Some(CampaignTab::Trade));
            assert_eq!(detail.0, None);
        } else {
            assert_eq!(view.section, section);
            assert_eq!(view.last_detail, detail.0);
        }
    }
}

#[test]
fn overview_meters_follow_live_control_relation_and_ownership() {
    let ctx = layout_context();
    let mut politics = ProvincePolitics::independent(2);
    let states = [
        (
            PoliticalState::Independent {
                local: 100.0,
                shares: vec![0.0, 0.0],
            },
            0.0,
            vec!["0/100"],
        ),
        (
            PoliticalState::Independent {
                local: 15.0,
                shares: vec![55.0, 30.0],
            },
            24.5,
            vec!["55/100", "24/100"],
        ),
        (
            PoliticalState::Vassal {
                overlord: 0,
                control: 72.5,
                tribute: Tribute::Normal,
            },
            83.0,
            vec!["72/100", "83/100"],
        ),
        (
            PoliticalState::Vassal {
                overlord: 1,
                control: 40.0,
                tribute: Tribute::Normal,
            },
            61.0,
            vec!["40/100", "61/100"],
        ),
        (
            PoliticalState::Owned {
                owner: 0,
            },
            83.0,
            vec!["100/100"],
        ),
        (
            PoliticalState::Owned {
                owner: 1,
            },
            45.0,
            vec!["0/100", "45/100"],
        ),
    ];
    for (state, relation, expected) in states {
        politics.state = state;
        if let PoliticalState::Owned {
            owner,
        } = politics.state
        {
            politics.owned_shares.fill(0.0);
            politics.owned_shares[owner] = 100.0;
        }
        politics.relations[0] = relation;
        let mut output = ctx.run_ui(egui::RawInput::default(), |root| {
            egui::CentralPanel::default().show(root, |ui| {
                ui.set_width(500.0);
                panel_style(ui, 1.0);
                overview_politics(ui, &politics, 0, 1.0, relation);
            });
        });
        let labels: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape {
                    Some(text.galley.job.text.as_str())
                } else {
                    None
                }
            })
            .collect();
        for label in expected {
            assert!(labels.contains(&label), "Missing {label:?} in {labels:?}");
        }
        if matches!(
            politics.state,
            PoliticalState::Owned {
                owner: 0
            }
        ) {
            assert!(
                labels.iter().filter(|label| **label == "100/100").count() == 1
                    && labels.contains(&"83/100"),
                "Domestic relations must give way to happiness"
            );
        }
        assert!(!labels.contains(&"Your province") && !labels.contains(&"Domestic province"));
        output.textures_delta.clear();
    }
}

#[test]
fn terrain_city_badges_keep_their_sides_and_city_opens_buildings() {
    let ctx = layout_context();
    ctx.memory_mut(|memory| memory.set_everything_is_visible(false));
    let campaign = fixture();
    let mut view = CampaignUi {
        open: Some(CampaignTab::Province),
        province: Some(1),
        ..Default::default()
    };
    run_province_header(&ctx, &campaign, &mut view, 0.0, vec![]);
    let output = run_province_header(&ctx, &campaign, &mut view, 0.1, vec![]);
    let text_position = |label: &str| {
        output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape {
                    (text.galley.job.text == label).then_some(text.pos)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| panic!("Missing landscape badge {label}"))
    };
    let terrain = text_position("Farmland");
    let city = text_position("Carthage");
    assert!(terrain.x < city.x);
    assert!((terrain.y - city.y).abs() < 2.0);
    let position = city + egui::vec2(5.0, 5.0);
    for (time, pressed) in [(0.2, true), (0.3, false)] {
        run_province_header(
            &ctx,
            &campaign,
            &mut view,
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
    assert_eq!(view.section, 2, "City badge must open the buildings inspector");
}

#[test]
fn military_roster_and_active_battle_fit_desktop_and_compact_panels() {
    for (width, scale) in [(500.0, 1.0), (380.0, 0.85), (380.0, 1.0)] {
        for in_battle in [false, true] {
            let mut campaign = fixture();
            let province = if in_battle {
                1
            } else {
                0
            };
            if in_battle {
                campaign
                    .military
                    .seed_unit(1, ForceOwner::Player(0), UnitType::LightCavalry)
                    .unwrap();
                campaign.military.seed_unit(1, ForceOwner::Player(0), UnitType::Archers).unwrap();
                campaign
                    .military
                    .start_battle(
                        1,
                        &[ForceOwner::Player(0)],
                        &[ForceOwner::Player(1)],
                        Some(1),
                        Some(0),
                        MilitaryTerrain::Hills,
                        2,
                        9,
                    )
                    .unwrap();
            }
            assert_body_fits(&format!("Military battle={in_battle}"), width, scale, |ui| {
                campaign_military::show(
                    ui,
                    &campaign.military,
                    &campaign.economy,
                    &campaign.graph,
                    province,
                    0,
                    |_| true,
                    |_| None,
                    |_, _| MilitaryAccess::Peaceful,
                    |a, b| campaign.forces_hostile(a, b),
                );
            });
        }
    }
}

#[test]
fn military_panel_actions_drive_paid_drafting_movement_locked_battle_and_retreat() {
    use campaign_military::MilitaryUiAction as Action;
    let mut c = fixture();
    let owner = ForceOwner::Player(0);
    c.military.config.units[UnitType::LightInfantry as usize].recruitment_months = 1.;
    let population = c.economy.provinces[0].population[2];
    let metal = c.economy.players[0].resources[1];
    assert_eq!(apply_military_action(&mut c, 0, 0, Action::Recruit(UnitType::LightInfantry)), "");
    assert_eq!(c.economy.provinces[0].population[2], population - crate::map::POPULATION_SCALE);
    assert_eq!(c.economy.players[0].resources[1], metal - 12.);
    assert_eq!(apply_military_action(&mut c, 0, 0, Action::CancelRecruitment), "");
    assert_eq!(
        c.economy.provinces[0].population[2],
        population - crate::map::POPULATION_SCALE,
        "cancel must not refund drafted population"
    );
    assert_eq!(apply_military_action(&mut c, 0, 0, Action::Recruit(UnitType::LightInfantry)), "");
    c.advance_month();
    let recruit = c.military.provinces[0].forces[&owner].last().unwrap().clone();
    let civilians = c.economy.provinces[0].population[2];
    assert_eq!(apply_military_action(&mut c, 0, 0, Action::Disband(recruit.id)), "");
    assert_eq!(
        c.economy.provinces[0].population[2],
        civilians
            + c.military.config.unit(recruit.unit_type).population_cost * recruit.manpower_ratio()
    );
    let plan = BattlePlan {
        tactic: CombatTactic::Envelopment,
        ..Default::default()
    };
    assert_eq!(apply_military_action(&mut c, 0, 0, Action::SavePlan(plan)), "");
    let unit = c.military.provinces[0].forces[&owner]
        .iter()
        .find(|unit| unit.unit_type == UnitType::HeavyInfantry)
        .unwrap()
        .id;
    c.declare_hostility(0, 1);
    assert_eq!(
        apply_military_action(
            &mut c,
            0,
            0,
            Action::Attack {
                destination: 1,
                units: vec![unit],
                route: vec![1],
                plan
            }
        ),
        ""
    );
    assert!(c.military.provinces[0].forces[&owner].iter().all(|cohort| cohort.id != unit));
    let moving = c.military.movements[0].id;
    let override_plan = BattlePlan {
        tactic: CombatTactic::ShockAction,
        ..plan
    };
    assert_eq!(
        apply_military_action(
            &mut c,
            0,
            0,
            Action::SaveMovementPlan {
                order: moving,
                plan: override_plan
            }
        ),
        ""
    );
    assert_eq!(
        c.military.provinces[0].plans[&owner], plan,
        "moving override must preserve stationary default"
    );
    c.military.config.base_manpower_damage = 0.;
    c.military.config.base_morale_damage = 0.;
    c.advance_month();
    let battle = &c.military.battles[0];
    let battle_id = battle.id;
    assert_eq!(battle.attackers.plans[&owner], override_plan);
    assert_ne!(apply_military_action(&mut c, 1, 0, Action::SavePlan(plan)), "");
    assert_eq!(
        apply_military_action(
            &mut c,
            1,
            0,
            Action::Retreat {
                battle: battle_id,
                attacker: true
            }
        ),
        ""
    );
    c.advance_month();
    assert!(c.military.battles.is_empty());
    assert!(c.military.provinces[0].forces[&owner].iter().any(|cohort| cohort.id == unit));
    assert_eq!(c.economy.provinces[1].owner, Some(1));
}

#[test]
fn senate_appointment_notice_opens_the_chamber_for_each_local_player() {
    use super::super::campaign_notifications::{NoticeAction, NoticeKind};
    let mut c = fixture();
    let required = c.senate_config.requirements(c.actors[0].rank, c.actors.len()).unwrap().senators;
    for senator in c.senate.senators.iter_mut().take(required) {
        senator.allegiance = Some(0);
    }
    let event = c.senate.promote(0, &mut c.actors, &c.senate_config).unwrap();
    c.record_senate_event(&event);
    assert_eq!(c.actors[0].rank, PoliticalRank::Praetor);
    for player in 0..2 {
        let result = c
            .notifications
            .history_for(player)
            .find(|notice| notice.kind == NoticeKind::SenateOfficeAppointed)
            .unwrap();
        assert_eq!(result.action, NoticeAction::OpenSenate);
        assert_eq!(result.province, None);
    }
}
