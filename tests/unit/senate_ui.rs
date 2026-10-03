//! Check the real chamber paint output and section hover behavior.

use super::*;

use crate::egui_capture as capture;

fn render(
    ctx: &egui::Context,
    senate: &SenateState,
    width: f32,
    time: f64,
    pointer: Option<egui::Pos2>,
) -> (egui::FullOutput, egui::Rect) {
    let mut bounds = egui::Rect::NOTHING;
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000.0, 800.0),
            )),
            time: Some(time),
            events: pointer.map_or_else(Vec::new, |p| vec![egui::Event::PointerMoved(p)]),
            ..Default::default()
        },
        |root| {
            root.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(egui::Rect::from_min_size(
                        egui::pos2(20.0, 20.0),
                        egui::vec2(width, 600.0),
                    ))
                    .layout(egui::Layout::top_down(egui::Align::Min)),
                |ui| {
                    ui.set_width(width);
                    bounds = egui::Rect::from_min_size(
                        ui.next_widget_position(),
                        egui::vec2(width.min(590.0), width.min(590.0) * 0.56),
                    );
                    draw_chamber(
                        ui,
                        senate,
                        &SenateConfig::default(),
                        0,
                        &[egui::Color32::RED, egui::Color32::BLUE, egui::Color32::GREEN],
                    );
                },
            );
        },
    );
    output.textures_delta.clear();
    (output, bounds)
}

#[test]
fn chamber_shows_one_hundred_separate_larger_seats_in_the_actual_player_colors() {
    let mut senate = SenateState::new(1);
    for (id, senator) in senate.senators.iter_mut().enumerate() {
        senator.allegiance = if id % 4 == 3 {
            None
        } else {
            Some(id % 4)
        };
    }
    for width in [380.0, 680.0] {
        let ctx = egui::Context::default();
        let (output, bounds) = render(&ctx, &senate, width, 0.0, None);
        let seats: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Circle(circle) if circle.fill != egui::Color32::TRANSPARENT => {
                    Some(circle)
                },
                _ => None,
            })
            .collect();
        assert_eq!(seats.len(), 100);
        for color in [UNDECIDED, egui::Color32::RED, egui::Color32::BLUE, egui::Color32::GREEN] {
            assert_eq!(seats.iter().filter(|s| s.fill == color).count(), 25);
        }
        for (index, seat) in seats.iter().enumerate() {
            assert!(seat.radius > 5.0, "Seats must exceed the former five-pixel radius");
            assert!(bounds.contains_rect(egui::Rect::from_center_size(
                seat.center,
                egui::Vec2::splat(seat.radius * 2.0)
            )));
            for other in &seats[index + 1..] {
                assert!(
                    seat.center.distance(other.center) > seat.radius + other.radius,
                    "Distinct senators must remain visibly separated"
                );
            }
        }
    }
}

#[test]
fn faction_names_follow_the_rim_without_covering_seats() {
    let senate = SenateState::new(1);
    for width in [380.0, 590.0] {
        let ctx = egui::Context::default();
        let (output, bounds) = render(&ctx, &senate, width, 0.0, None);
        let seats: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Circle(circle) if circle.fill != egui::Color32::TRANSPARENT => {
                    Some(circle)
                },
                _ => None,
            })
            .collect();
        let labels: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text)
                    if Bloc::ALL.iter().any(|bloc| text.galley.job.text == bloc.label()) =>
                {
                    Some(text)
                },
                _ => None,
            })
            .collect();
        assert_eq!(labels.len(), Bloc::ALL.len());
        for (index, label) in labels.iter().enumerate() {
            let expected_angle = (index as f32 - 2.0) * std::f32::consts::PI / 5.0;
            assert!((label.angle - expected_angle).abs() < 0.001);
            assert!(bounds.contains_rect(label.visual_bounding_rect()));
            let label_center = label.visual_bounding_rect().center();
            let tangent = egui::vec2(label.angle.cos(), label.angle.sin());
            let normal = egui::vec2(-tangent.y, tangent.x);
            for seat in &seats {
                let offset = seat.center - label_center;
                let dx = (offset.dot(tangent).abs() - label.galley.rect.width() / 2.0).max(0.0);
                let dy = (offset.dot(normal).abs() - label.galley.rect.height() / 2.0).max(0.0);
                assert!(dx * dx + dy * dy >= seat.radius * seat.radius);
            }
        }
    }
}

#[test]
fn chamber_has_matching_horizontal_edges_under_aristocrats_and_military() {
    let ctx = egui::Context::default();
    let (output, bounds) = render(&ctx, &SenateState::new(1), 590.0, 0.0, None);
    let edges: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::LineSegment {
                points,
                ..
            } if (points[0].y - (bounds.bottom() - 6.0)).abs() < 0.01
                && (points[0].y - points[1].y).abs() < 0.01 =>
            {
                Some(*points)
            },
            _ => None,
        })
        .collect();
    assert_eq!(edges.len(), 2);
    assert!((edges[0][0].distance(edges[0][1]) - edges[1][0].distance(edges[1][1])).abs() < 0.01);
    assert!(edges[0].iter().all(|p| p.x < bounds.center().x));
    assert!(edges[1].iter().all(|p| p.x > bounds.center().x));
}

#[test]
fn chamber_hover_never_opens_faction_tooltips() {
    let senate = SenateState::new(1);
    let ctx = egui::Context::default();
    let (_, bounds) = render(&ctx, &senate, 680.0, 0.0, None);
    let pointer =
        egui::pos2(bounds.center().x, bounds.bottom() - 6.0 - bounds.width() * 0.47 * 0.76);
    let mut tooltip = false;
    for frame in 1..=4 {
        let (output, _) =
            render(&ctx, &senate, 680.0, frame as f64, (frame == 1).then_some(pointer));
        let texts: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect();
        tooltip |= texts.contains(&"Positive effects") || texts.contains(&"Negative effects");
    }
    assert!(!tooltip, "Only faction badges may open faction tooltips");
}

fn panel_frame(
    ctx: &egui::Context,
    senate: &mut SenateState,
    players: &mut [PoliticalPlayer],
    espionage: &mut EspionageState,
    time: f64,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let scale = super::super::viewport_ui_scale(egui::vec2(1200.0, 1000.0));
    ctx.set_global_style(super::super::campaign_widgets::map_style(scale));
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 1000.0),
            )),
            time: Some(time),
            events,
            ..Default::default()
        },
        |root| {
            root.painter().rect_filled(
                egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(680.0, 900.0)),
                8.0,
                PAPER,
            );
            root.scope_builder(
                egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                    egui::pos2(20.0, 20.0),
                    egui::vec2(680.0, 900.0),
                )),
                |ui| {
                    ui.set_width(680.0);
                    let profiles = vec![PoliticalProfile::default(); players.len()];
                    show(
                        ui,
                        senate,
                        players,
                        &profiles,
                        &SenateConfig {
                            action_risks: [0.0; 9],
                            ..Default::default()
                        },
                        0,
                        espionage,
                        &EspionageConfig::default(),
                        &[egui::Color32::RED, egui::Color32::BLUE],
                    );
                },
            );
            // Paint the real decorative standard after the Senate, reproducing
            // the banner overlap even when the UI systems run in this order.
            let key = egui::Id::new("senate-test-standard");
            let standard =
                ctx.data(|d| d.get_temp::<egui::TextureHandle>(key)).unwrap_or_else(|| {
                    let texture = super::super::map_menu::load_player_standard(ctx, 0);
                    ctx.data_mut(|d| d.insert_temp(key, texture.clone()));
                    texture
                });
            super::super::map_edge::paint_map_edge_frame(
                ctx,
                1.0,
                &standard,
                &crate::app::GameClock::default(),
                false,
                [(false, false); 2],
            );
            let roadmap_key = egui::Id::new("senate-test-roadmap");
            let scepter =
                ctx.data(|d| d.get_temp::<egui::TextureHandle>(roadmap_key)).unwrap_or_else(|| {
                    let texture = super::super::rank_hud::load_rank_scepter(ctx);
                    ctx.data_mut(|d| d.insert_temp(roadmap_key, texture.clone()));
                    texture
                });
            let mut ranks = ctx
                .data(|d| d.get_temp::<[Option<egui::TextureHandle>; 6]>(roadmap_key.with("ranks")))
                .unwrap_or_else(|| std::array::from_fn(|_| None));
            super::super::rank_hud::draw_rank_ladder(ctx, scale, 0, &mut ranks, &scepter);
            ctx.data_mut(|d| d.insert_temp(roadmap_key.with("ranks"), ranks));
        },
    );
    if std::env::var_os("AUGUSTUS_UI_CAPTURE").is_some() {
        let key = egui::Id::new("senate-software-capture");
        let capture = ctx
            .data(|d| d.get_temp::<std::sync::Arc<std::sync::Mutex<capture::Capture>>>(key))
            .unwrap_or_else(|| {
                let capture =
                    std::sync::Arc::new(std::sync::Mutex::new(capture::Capture::default()));
                ctx.data_mut(|d| d.insert_temp(key, capture.clone()));
                capture
            });
        let text = texts(&output);
        let name = if text.contains(&"Positive effects") {
            let count_index =
                text.iter().position(|label| label.ends_with("senators support you")).unwrap();
            format!("senate-{}-tooltip", text[count_index - 1].to_lowercase())
        } else if text.contains(&"Senator 1") {
            if let Some(action) =
                SenatorAction::ALL.iter().position(|action| text.contains(&action.description()))
            {
                format!("senate-action-{action}-tooltip")
            } else if senate.spent_on(0, SenatorAction::Bribe) > 0.0 {
                "senate-actions-used".to_owned()
            } else {
                "senate-actions".to_owned()
            }
        } else {
            "senate-panel".to_owned()
        };
        capture.lock().unwrap().frame(ctx, &output, &name);
    }
    output.textures_delta.clear();
    output
}

fn texts(output: &egui::FullOutput) -> Vec<&str> {
    output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) => Some(t.galley.job.text.as_str()),
            _ => None,
        })
        .collect()
}

fn actions_frame(
    ctx: &egui::Context,
    senate: &mut SenateState,
    players: &mut [PoliticalPlayer],
    player: usize,
    time: f64,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.set_global_style(super::super::campaign_widgets::map_style(1.0));
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 800.0),
            )),
            time: Some(time),
            events,
            ..Default::default()
        },
        |root| {
            root.scope_builder(
                egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                    egui::pos2(20.0, 20.0),
                    egui::vec2(650.0, 500.0),
                )),
                |ui| {
                    ui.set_width(650.0);
                    egui::Frame::popup(ui.style()).fill(PAPER).show(ui, |ui| {
                        senator_actions(
                            ui,
                            senate,
                            players,
                            &SenateConfig {
                                action_risks: [0.0; 9],
                                ..Default::default()
                            },
                            player,
                            0,
                            &mut EspionageState::new(1),
                            &EspionageConfig::default(),
                            &[egui::Color32::RED, egui::Color32::BLUE],
                            1.0,
                            &mut None,
                        );
                    });
                },
            );
        },
    );
    if std::env::var_os("AUGUSTUS_UI_CAPTURE").is_some() {
        let key = egui::Id::new("senator-actions-capture");
        let capture = ctx
            .data(|d| d.get_temp::<std::sync::Arc<std::sync::Mutex<capture::Capture>>>(key))
            .unwrap_or_else(|| {
                let capture =
                    std::sync::Arc::new(std::sync::Mutex::new(capture::Capture::default()));
                ctx.data_mut(|d| d.insert_temp(key, capture.clone()));
                capture
            });
        let text = texts(&output);
        let name = if text.contains(&"Cancel action") {
            "senator-cancel-tooltip"
        } else if text.contains(&"×") {
            "senator-active-action"
        } else {
            "senator-private-actions"
        };
        capture.lock().unwrap().frame(ctx, &output, name);
    }
    output.textures_delta.clear();
    output
}

#[test]
fn ongoing_actions_are_private_in_the_chamber_and_popup() {
    let mut senate = SenateState::new(1);
    let mut players = vec![
        PoliticalPlayer {
            coin: 1000.0,
            influence: 1000.0,
            ..Default::default()
        };
        2
    ];
    let config = SenateConfig {
        action_risks: [0.0; 9],
        ..Default::default()
    };
    senate.act_on_senator(1, 0, SenatorAction::Lobby, &mut players, &config).unwrap();
    let (chamber, _) = render(&egui::Context::default(), &senate, 590.0, 0.0, None);
    assert!(!chamber.shapes.iter().any(|shape| matches!(&shape.shape,
        egui::Shape::Circle(circle) if circle.stroke.color == egui::Color32::from_rgb(184, 135, 52))));
    let ctx = egui::Context::default();
    let neutral = actions_frame(&ctx, &mut SenateState::new(1), &mut players, 0, 0.0, vec![]);
    let rival = actions_frame(&ctx, &mut senate, &mut players, 0, 0.1, vec![]);
    assert_eq!(texts(&neutral), texts(&rival));
    assert!(!texts(&rival).contains(&"×"));
    assert!(senate.senator_action_quote(0, 0, SenatorAction::Lobby, &players, &config).is_ok());
    let owner = actions_frame(&ctx, &mut senate, &mut players, 1, 0.2, vec![]);
    assert_eq!(texts(&owner).iter().filter(|text| **text == "×").count(), 1);
    assert!(!texts(&owner)
        .iter()
        .any(|text| text.contains("Lobbying for") || text.contains("End senator lobbying")));
}

#[test]
fn ongoing_action_cross_cancels_only_your_action_without_spending_and_can_restart() {
    let config = SenateConfig {
        action_risks: [0.0; 9],
        ..Default::default()
    };
    for action in SenatorAction::ALL.into_iter().filter(|action| action.is_ongoing()) {
        let ctx = egui::Context::default();
        let mut senate = SenateState::new(1);
        let mut players = vec![
            PoliticalPlayer {
                coin: 1000.0,
                influence: 1000.0,
                ..Default::default()
            };
            2
        ];
        senate.act_on_senator(0, 0, action, &mut players, &config).unwrap();
        senate.act_on_senator(1, 0, SenatorAction::Lobby, &mut players, &config).unwrap();
        let opened = actions_frame(&ctx, &mut senate, &mut players, 0, 0.0, vec![]);
        let position = opened
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == "×" => {
                    Some(text.pos + text.galley.rect.center().to_vec2())
                },
                _ => None,
            })
            .expect("The ongoing action has a cross in its card");
        let mut hover = opened;
        for frame in 1..=4 {
            hover = actions_frame(
                &ctx,
                &mut senate,
                &mut players,
                0,
                frame as f64,
                (frame == 1).then_some(egui::Event::PointerMoved(position)).into_iter().collect(),
            );
        }
        assert!(texts(&hover).contains(&"Cancel action"));
        if action == SenatorAction::Lobby {
            assert!(texts(&hover).contains(&"You are already lobbying this senator."));
        }
        let balances: Vec<_> = players.iter().map(|p| (p.coin, p.influence)).collect();
        let click = |pressed| egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        actions_frame(&ctx, &mut senate, &mut players, 0, 5.0, vec![click(true)]);
        actions_frame(&ctx, &mut senate, &mut players, 0, 5.1, vec![click(false)]);
        assert!(senate.senators[0].arrangement(0).is_none());
        assert!(senate.senators[0].arrangement(1).is_some());
        assert_eq!(players.iter().map(|p| (p.coin, p.influence)).collect::<Vec<_>>(), balances);
        assert!(senate.senator_action_quote(0, 0, action, &players, &config).is_ok());
        assert_eq!(senate.month, 0);
    }
}

#[test]
fn rank_hover_lists_requirements_and_readable_monthly_bonuses() {
    let ctx = egui::Context::default();
    let mut senate = SenateState::new(1);
    let mut players = vec![PoliticalPlayer::default(); 2];
    let mut espionage = EspionageState::new(1);
    players[0].influence = -0.0;
    let initial = panel_frame(&ctx, &mut senate, &mut players, &mut espionage, 0., vec![]);
    let position = initial
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == "Aedile" => {
                Some(text.pos + text.galley.rect.center().to_vec2())
            },
            _ => None,
        })
        .unwrap();
    let mut found = false;
    for frame in 1..=4 {
        let output = panel_frame(
            &ctx,
            &mut senate,
            &mut players,
            &mut espionage,
            frame as f64,
            if frame == 1 {
                vec![egui::Event::PointerMoved(position)]
            } else {
                vec![]
            },
        );
        let labels = texts(&output);
        if !labels.contains(&"Requirements") {
            continue;
        }
        found = true;
        for expected in [
            "Bonuses",
            "• Previous rank: Quaestor",
            "• Supporting senators: 0/10",
            "• Influence cost: 0/500",
            "• Influence: +5 per month",
        ] {
            assert!(labels.contains(&expected), "Missing {expected}: {labels:?}");
        }
        assert_eq!(labels.iter().filter(|label| **label == "Aedile").count(), 1);
        assert!(!labels.iter().any(|label| [
            "Attract the required",
            "-0",
            "Senate aristocrats faction",
            "One promotion",
            "Support requirements"
        ]
        .iter()
        .any(|removed| label.contains(removed))));
        let tooltip: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text)
                    if text.galley.job.text.starts_with('•')
                        || ["Requirements", "Bonuses"].contains(&text.galley.job.text.as_str()) =>
                {
                    Some(text)
                },
                _ => None,
            })
            .collect();
        let size = 14. * super::super::viewport_ui_scale(ctx.content_rect().size());
        for text in &tooltip {
            assert!(text
                .galley
                .job
                .sections
                .iter()
                .all(|section| section.format.font_id.size == size));
            assert!(!text.galley.job.text.contains(" / "));
        }
        let last_requirement =
            tooltip.iter().find(|text| text.galley.job.text == "• Influence cost: 0/500").unwrap();
        let bonuses = tooltip.iter().find(|text| text.galley.job.text == "Bonuses").unwrap();
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::LineSegment { points, .. } if points[0].y > last_requirement.pos.y
                && points[0].y < bonuses.pos.y && (points[1].x - points[0].x).abs() > 50.)));
    }
    assert!(found, "The office card must show its tooltip on hover");
}

#[test]
fn senate_panel_has_six_rank_cards_and_five_count_badges_without_internal_scores() {
    let ctx = egui::Context::default();
    let mut senate = SenateState::new(1);
    let mut players = vec![PoliticalPlayer::default(); 2];
    let mut espionage = EspionageState::new(1);
    let output = panel_frame(&ctx, &mut senate, &mut players, &mut espionage, 0.0, vec![]);
    let text = texts(&output);
    for rank in ["Quaestor", "Aedile", "Praetor", "Censor", "Consul", "Augustus"] {
        assert!(text.contains(&rank));
    }
    assert_eq!(text.iter().filter(|s| **s == "0 / 20").count(), 5);
    assert!(text.contains(&"POLITICAL RANK"));
    assert!(text.contains(&"SENATE"));
    assert!(text.contains(&"Imperialists"));
    assert!(!text.contains(&"Military"));
    assert!(text.contains(&"Neutral (100)"));
    assert!(text.contains(&"Player 1 (0)"));
    assert!(text.contains(&"Player 2 (0)"));
    for removed in [
        "Click a senator",
        "Begin outreach",
        "Scandals and evidence",
        " (you)",
        "Political career",
        "Roman Senate",
        "Influence/month",
        "Term:",
        "Two Consul seats",
        "May return to Consul",
    ] {
        assert!(!text.iter().any(|s| s.contains(removed)));
    }
    assert!(!text
        .iter()
        .any(|s| s.contains("attraction points") || s.contains("confidence points")));
    let chamber_bottom = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Circle(c) if c.fill == UNDECIDED && c.radius > 6.0 => {
                Some(c.center.y + c.radius)
            },
            _ => None,
        })
        .fold(0.0_f32, f32::max);
    let legend: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t)
                if ["Neutral (100)", "Player 1 (0)", "Player 2 (0)"]
                    .contains(&t.galley.job.text.as_str()) =>
            {
                Some(t.visual_bounding_rect())
            },
            _ => None,
        })
        .collect();
    assert_eq!(legend.len(), 3);
    assert!(legend.iter().all(|r| r.top() > chamber_bottom));
    assert!(legend[1].left() - legend[0].right() > 25.0);
    assert!(legend[2].left() - legend[1].right() > 25.0);
}

#[test]
fn senate_legend_fits_two_to_eight_players_on_one_row_at_different_widths_and_scales() {
    let mut senate = SenateState::new(1);
    for players in 2..=8 {
        for (id, senator) in senate.senators.iter_mut().enumerate() {
            senator.allegiance = (id % (players + 1) != players).then_some(id % (players + 1));
        }
        for width in [380.0, 550.0, 680.0] {
            for scale in [0.75, 1.0, 1.5] {
                let ctx = egui::Context::default();
                ctx.set_global_style(super::super::campaign_widgets::map_style(scale));
                let mut legend_top = 0.0;
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1200.0, 1000.0),
                        )),
                        ..Default::default()
                    },
                    |root| {
                        root.painter().rect_filled(
                            egui::Rect::from_min_size(
                                egui::pos2(20.0, 20.0),
                                egui::vec2(width, width * 0.56 + 50.0),
                            ),
                            0.0,
                            PAPER,
                        );
                        root.scope_builder(
                            egui::UiBuilder::new()
                                .max_rect(egui::Rect::from_min_size(
                                    egui::pos2(20.0, 20.0),
                                    egui::vec2(width, 600.0),
                                ))
                                .layout(egui::Layout::top_down(egui::Align::Min)),
                            |ui| {
                                ui.set_width(width);
                                draw_chamber(
                                    ui,
                                    &senate,
                                    &SenateConfig::default(),
                                    0,
                                    &[egui::Color32::RED, egui::Color32::BLUE],
                                );
                                ui.add_space(10.0 * scale);
                                legend_top = ui.next_widget_position().y;
                                senate_legend(
                                    ui,
                                    &senate,
                                    players,
                                    &[egui::Color32::RED, egui::Color32::BLUE],
                                    scale,
                                );
                            },
                        );
                    },
                );
                if players == 4 && width == 550.0 && scale == 1.0 {
                    capture::Capture::default().frame(&ctx, &output, "senate-four-player-legend");
                }
                output.textures_delta.clear();
                let labels: Vec<_> = output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Text(text)
                            if text.galley.job.text.starts_with("Neutral (")
                                || text.galley.job.text.starts_with("Player ") =>
                        {
                            Some(text)
                        },
                        _ => None,
                    })
                    .collect();
                let dots: Vec<_> = output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Circle(circle) if circle.center.y > legend_top => Some(circle),
                        _ => None,
                    })
                    .collect();
                assert_eq!(labels.len(), players + 1);
                assert_eq!(dots.len(), players + 1);
                let bounds = egui::Rect::from_min_size(
                    egui::pos2(20.0, legend_top),
                    egui::vec2(width, 32.0 * scale),
                );
                let mut previous_right = bounds.left();
                let row_center = dots[0].center.y;
                for (label, dot) in labels.iter().zip(dots) {
                    let text_rect = label.visual_bounding_rect();
                    assert!(
                        bounds.contains_rect(text_rect),
                        "{players} players, {width}, {scale}: {} {text_rect:?} outside {bounds:?}",
                        label.galley.job.text
                    );
                    assert!(dot.center.x - dot.radius >= previous_right);
                    assert!(text_rect.left() > dot.center.x + dot.radius);
                    assert!((label.pos.y - labels[0].pos.y).abs() < 0.01);
                    assert!((dot.center.y - row_center).abs() < 0.01);
                    previous_right = text_rect.right();
                    if players == 4 && width == 550.0 && scale == 1.0 {
                        assert!(label
                            .galley
                            .job
                            .sections
                            .iter()
                            .all(|s| s.format.font_id.size == 14.0));
                    }
                }
            }
        }
    }
}

#[test]
fn hovering_a_seat_keeps_actions_above_the_hud_and_bribe_spends_the_real_wallet() {
    let ctx = egui::Context::default();
    let mut senate = SenateState::new(1);
    let mut players = vec![
        PoliticalPlayer {
            coin: 1000.0,
            influence: 1000.0,
            ..Default::default()
        };
        2
    ];
    let mut espionage = EspionageState::new(1);
    let initial = panel_frame(&ctx, &mut senate, &mut players, &mut espionage, 0.0, vec![]);
    let point = initial
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Circle(c) if c.fill == UNDECIDED && c.radius > 6.0 => {
                Some(c.center + egui::vec2(c.radius * 0.8, 0.0))
            },
            _ => None,
        })
        .expect("Actual senator circle");
    let pointer_event = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    panel_frame(
        &ctx,
        &mut senate,
        &mut players,
        &mut espionage,
        0.1,
        vec![egui::Event::PointerMoved(point)],
    );
    panel_frame(&ctx, &mut senate, &mut players, &mut espionage, 0.2, vec![]);
    let opened = panel_frame(&ctx, &mut senate, &mut players, &mut espionage, 0.3, vec![]);
    assert!(texts(&opened).contains(&"Senator 1"), "Popup contents: {:?}", texts(&opened));
    for label in [
        "Petition",
        "Send a gift",
        "Patronage",
        "Threaten",
        "Murder",
        "Public banquet",
        "Discredit patron",
        "Lobby senator",
    ] {
        assert!(texts(&opened).iter().any(|s| s.starts_with(label)), "Missing action {label}");
    }
    assert!(!texts(&opened).iter().any(|text| text.contains("One action per senator")));
    assert!(texts(&opened).contains(&"4/mo"));
    let header_index = opened
        .shapes
        .iter()
        .position(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.job.text == "Senator 1"))
        .unwrap();
    for key in ["senate-test-standard", "senate-test-roadmap"] {
        let texture = ctx.data(|d| d.get_temp::<egui::TextureHandle>(egui::Id::new(key))).unwrap();
        let hud_index = opened
            .shapes
            .iter()
            .rposition(|s| matches!(&s.shape, egui::Shape::Mesh(m) if m.texture_id == texture.id()))
            .expect("Real HUD artwork");
        assert!(header_index > hud_index, "Senator actions must paint above {key}");
    }
    let popup_header = opened
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text == "Senator 1" => Some(t.pos.y),
            _ => None,
        })
        .unwrap();
    let icon_size = 38.0 * super::super::viewport_ui_scale(ctx.content_rect().size());
    let action_icons: std::collections::HashSet<_> = opened
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Mesh(mesh)
                if (mesh.calc_bounds().width() - icon_size).abs() < 0.1
                    && mesh.calc_bounds().top() > popup_header + 30.0 =>
            {
                Some(mesh.texture_id)
            },
            _ => None,
        })
        .collect();
    assert_eq!(action_icons.len(), 9, "Every action needs a distinct illustrated icon");
    let mut time = 0.4;
    for action in SenatorAction::ALL {
        let title = action_presentation(action).0;
        let option = opened
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.job.text == title => {
                    Some(t.pos + t.galley.rect.center().to_vec2())
                },
                _ => None,
            })
            .unwrap();
        panel_frame(
            &ctx,
            &mut senate,
            &mut players,
            &mut espionage,
            time,
            vec![egui::Event::PointerMoved(option)],
        );
        time += 1.0;
        // The description's first frame measures its size before painting it.
        panel_frame(&ctx, &mut senate, &mut players, &mut espionage, time, vec![]);
        time += 0.1;
        let hover = panel_frame(&ctx, &mut senate, &mut players, &mut espionage, time, vec![]);
        assert!(texts(&hover).contains(&"Senator 1"), "Hovering {title} must keep actions open");
        let description_index = hover
            .shapes
            .iter()
            .position(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.job.text == action.description()))
            .expect("Action description remains visible above the menu");
        let last_option_index = hover
            .shapes
            .iter()
            .position(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.job.text == "Lobby senator"))
            .unwrap();
        assert!(description_index > last_option_index);
        let tooltip_text = texts(&hover);
        assert_eq!(
            tooltip_text.iter().filter(|text| **text == title).count(),
            1,
            "The action name belongs only on its card"
        );
        assert!(
            !tooltip_text
                .iter()
                .any(|text| text.starts_with("Upfront:") || text.starts_with("Monthly:")),
            "Prices belong only on the action cards"
        );
        if action == SenatorAction::Discredit {
            assert!(tooltip_text.iter().any(|text| text.contains("neutral and has no patron")));
            assert!(tooltip_text.iter().any(|text| text.contains("supports another player")));
        }
        assert_eq!(players[0].coin, 1000.0, "Hovering must not purchase an action");
        assert_eq!(players[0].influence, 1000.0);
        time += 0.1;
    }
    let bribe = opened
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text == "Bribe" => {
                Some(t.pos + t.galley.rect.center().to_vec2())
            },
            _ => None,
        })
        .expect("Bribe button");
    panel_frame(
        &ctx,
        &mut senate,
        &mut players,
        &mut espionage,
        time,
        vec![egui::Event::PointerMoved(bribe), pointer_event(bribe, true)],
    );
    panel_frame(
        &ctx,
        &mut senate,
        &mut players,
        &mut espionage,
        time + 0.1,
        vec![pointer_event(bribe, false)],
    );
    assert_eq!(players[0].coin, 920.0);
    assert_eq!(senate.senators[0].allegiance, None);
    assert_eq!(senate.support(0), 0);
    assert_eq!(senate.senators[0].arrangement(0).unwrap().action, SenatorAction::Bribe);
    panel_frame(
        &ctx,
        &mut senate,
        &mut players,
        &mut espionage,
        time + 0.2,
        vec![egui::Event::PointerMoved(bribe), pointer_event(bribe, true)],
    );
    panel_frame(
        &ctx,
        &mut senate,
        &mut players,
        &mut espionage,
        time + 0.3,
        vec![pointer_event(bribe, false)],
    );
    // egui suppresses a tooltip after a click until the pointer moves again.
    panel_frame(
        &ctx,
        &mut senate,
        &mut players,
        &mut espionage,
        time + 0.4,
        vec![egui::Event::PointerMoved(bribe + egui::vec2(12.0, 0.0))],
    );
    let unavailable =
        panel_frame(&ctx, &mut senate, &mut players, &mut espionage, time + 1.3, vec![]);
    let unavailable = if texts(&unavailable).iter().any(|text| text.contains("already bribing")) {
        unavailable
    } else {
        panel_frame(&ctx, &mut senate, &mut players, &mut espionage, time + 1.4, vec![])
    };
    assert!(texts(&unavailable).iter().any(|text| text.contains("already bribing")));
    assert!(!texts(&unavailable).iter().any(|text| text.contains("Wait until next game month")));
    assert_eq!(
        players[0].coin, 920.0,
        "Repeated clicks on the disabled card must not charge again"
    );
    time += 1.5;
    let closed = panel_frame(
        &ctx,
        &mut senate,
        &mut players,
        &mut espionage,
        time + 0.2,
        vec![egui::Event::PointerMoved(egui::pos2(800.0, 50.0))],
    );
    assert!(!texts(&closed).contains(&"Senator 1"), "Leaving the seat and menu closes actions");
    let next_seat = initial
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Circle(c) if c.fill == UNDECIDED && c.radius > 6.0 => Some(c.center),
            _ => None,
        })
        .nth(1)
        .unwrap();
    panel_frame(
        &ctx,
        &mut senate,
        &mut players,
        &mut espionage,
        time + 0.3,
        vec![egui::Event::PointerMoved(next_seat)],
    );
    let switched = panel_frame(&ctx, &mut senate, &mut players, &mut espionage, time + 0.4, vec![]);
    assert!(texts(&switched).contains(&"Senator 2"), "Hovering another circle opens its actions");
    let gone = panel_frame(
        &ctx,
        &mut senate,
        &mut players,
        &mut espionage,
        time + 0.5,
        vec![egui::Event::PointerGone],
    );
    assert!(!texts(&gone).contains(&"Senator 2"));
}

#[test]
fn badge_hover_has_positive_and_negative_rules_and_hides_exact_confidence() {
    for bloc in Bloc::ALL {
        let ctx = egui::Context::default();
        let mut senate = SenateState::new(1);
        let mut players = vec![PoliticalPlayer::default(); 2];
        players[0].rank = PoliticalRank::Aedile;
        let mut espionage = EspionageState::new(1);
        let initial = panel_frame(&ctx, &mut senate, &mut players, &mut espionage, 0.0, vec![]);
        let badge = initial
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.job.text == bloc.label() && t.angle == 0.0 => {
                    Some(t.pos + t.galley.rect.center().to_vec2())
                },
                _ => None,
            })
            .unwrap();
        let mut found = false;
        for frame in 1..=4 {
            let output = panel_frame(
                &ctx,
                &mut senate,
                &mut players,
                &mut espionage,
                frame as f64,
                if frame == 1 {
                    vec![egui::Event::PointerMoved(badge)]
                } else {
                    vec![]
                },
            );
            let text = texts(&output);
            found |= text.contains(&"Positive effects") && text.contains(&"Negative effects");
            if text.contains(&"Positive effects") {
                assert!(text.contains(&"0 / 20 senators support you"));
                if bloc == Bloc::Aristocrats {
                    assert!(text.contains(&"• Built forums"));
                    assert!(text.contains(&"• Larger happy noble population"));
                }
                if bloc == Bloc::Merchants {
                    assert!(text.contains(&"• Positive net monthly coin income"));
                    assert!(!text.iter().any(|label| label.contains("income above")));
                }
                if bloc == Bloc::Populares {
                    assert!(text.contains(&"• Ample food reserves"));
                    assert!(text.contains(&"• Supplied generous food rations"));
                    assert!(!text.contains(&"• Reliable food supply"));
                }
                if bloc == Bloc::Military {
                    assert!(text.contains(&"• Recent defeats"));
                    assert!(text.contains(&"• Control beyond your first province"));
                }
                if bloc == Bloc::Provincials {
                    assert!(text.contains(&"• More vassal provinces"));
                    assert!(text.contains(&"• Good relations with other provinces"));
                    assert!(text.contains(&"• Delivered trade with provinces without cities"));
                    assert!(!text.iter().any(|label| label.contains("happiness")));
                    assert!(!text.iter().any(|label| label.contains("nobles")));
                }
                let standard = ctx
                    .data(|d| {
                        d.get_temp::<egui::TextureHandle>(egui::Id::new("senate-test-standard"))
                    })
                    .unwrap();
                let flag_index = output
                .shapes
                .iter()
                .rposition(
                    |s| matches!(&s.shape, egui::Shape::Mesh(m) if m.texture_id == standard.id()),
                )
                .unwrap();
                let positive_index = output.shapes.iter().position(|s|
                matches!(&s.shape, egui::Shape::Text(t) if t.galley.job.text == "Positive effects")).unwrap();
                assert!(
                    positive_index > flag_index,
                    "Faction tooltip must paint above the actual banner"
                );
            }
            assert!(!text.iter().any(|s| s.contains("attraction points")
                || s.contains("confidence points")
                || s.contains("senators approve")
                || s.contains("Active monthly effects")
                || s.contains("Exposed scandals cause immediate losses")
                || s.contains("Current monthly support factors")
                || s.contains("These conditions build or weaken")
                || s.contains("No monthly changes")
                || s.contains("builds support")
                || s.contains("weakens support")));
        }
        assert!(found, "The badge itself must expose both lists");
    }
}
