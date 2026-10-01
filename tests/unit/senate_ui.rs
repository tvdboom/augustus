//! Check the real chamber paint output and section hover behavior.

use super::*;

#[path = "egui_capture.rs"]
mod capture;

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
            "senate-faction-tooltip"
        } else if text.contains(&"Senator 1") {
            if senate.spent_on(0, SenatorAction::Bribe) > 0.0 {
                "senate-actions-used"
            } else {
                "senate-actions"
            }
        } else {
            "senate-panel"
        };
        capture.lock().unwrap().frame(ctx, &output, name);
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
            "• Approving senators: 0/10",
            "• Influence to pay: 0/500",
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
        let last_requirement = tooltip
            .iter()
            .find(|text| text.galley.job.text == "• Influence to pay: 0/500")
            .unwrap();
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
fn clicking_an_actual_seat_opens_actions_and_bribe_spends_the_real_wallet() {
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
            egui::Shape::Circle(c) if c.fill == UNDECIDED && c.radius > 6.0 => Some(c.center),
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
        vec![egui::Event::PointerMoved(point), pointer_event(point, true)],
    );
    panel_frame(
        &ctx,
        &mut senate,
        &mut players,
        &mut espionage,
        0.2,
        vec![pointer_event(point, false)],
    );
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
    assert!(!texts(&opened).iter().any(|s| s.contains("One personal action")));
    assert!(texts(&opened).contains(&"4/mo"));
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
        0.4,
        vec![egui::Event::PointerMoved(bribe), pointer_event(bribe, true)],
    );
    panel_frame(
        &ctx,
        &mut senate,
        &mut players,
        &mut espionage,
        0.5,
        vec![pointer_event(bribe, false)],
    );
    assert_eq!(players[0].coin, 920.0);
    assert_eq!(senate.senators[0].allegiance, Some(0));
    assert_eq!(senate.support(0), 1);
}

#[test]
fn badge_hover_has_positive_and_negative_rules_and_hides_exact_confidence() {
    let ctx = egui::Context::default();
    let mut senate = SenateState::new(1);
    let mut players = vec![PoliticalPlayer::default(); 2];
    let mut espionage = EspionageState::new(1);
    let initial = panel_frame(&ctx, &mut senate, &mut players, &mut espionage, 0.0, vec![]);
    let badge = initial
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text == "Merchants" && t.angle == 0.0 => {
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
            let standard = ctx
                .data(|d| d.get_temp::<egui::TextureHandle>(egui::Id::new("senate-test-standard")))
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
        assert!(!text
            .iter()
            .any(|s| s.contains("attraction points") || s.contains("confidence points")));
    }
    assert!(found, "The badge itself must expose both lists");
}
