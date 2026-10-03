use super::*;
use crate::game::economy::{EconomicProvince, Terrain};
use crate::game::politics::espionage::ScandalKind;
use crate::game::politics::PoliticalPlayer;

use crate::egui_capture as capture;
use egui::epaint::text::{FontInsert, FontPriority, InsertFontFamily};

fn layout_context() -> egui::Context {
    let ctx = egui::Context::default();
    ctx.add_font(FontInsert::new(
        "firasans",
        egui::FontData::from_static(include_bytes!("../../assets/fonts/FiraSans-Bold.ttf")),
        vec![InsertFontFamily {
            family: egui::FontFamily::Proportional,
            priority: FontPriority::Highest,
        }],
    ));
    ctx
}

fn fixture() -> Campaign {
    let mut campaign = Campaign {
        actors: vec![PoliticalPlayer::default(); 3],
        ..Default::default()
    };
    campaign.economy.provinces.push(EconomicProvince::new(
        "Achaia",
        60.0,
        Terrain::Farmland,
        true,
        [1.0; 3],
        [25.0; 4],
        3,
    ));
    campaign.senate.month = 10;
    let evidence = |id, holder, target, severity, acquired, expires, reserved_for_motion| Scandal {
        id,
        holder,
        target,
        severity,
        acquired,
        expires,
        reserved_for_motion,
        kind: match severity {
            Severity::Major => ScandalKind::Famine,
            Severity::Medium => ScandalKind::Espionage,
            Severity::Minor => ScandalKind::LowFood,
        },
        source_id: id,
        province: Some(0),
    };
    campaign.espionage.scandals = vec![
        evidence(1, 0, ScandalTarget::Player(1), Severity::Minor, 9, 24, false),
        evidence(2, 0, ScandalTarget::Player(1), Severity::Major, 1, 24, false),
        evidence(3, 0, ScandalTarget::Province(0), Severity::Medium, 9, 24, false),
        evidence(4, 0, ScandalTarget::Player(2), Severity::Major, 2, 24, false),
        evidence(5, 1, ScandalTarget::Player(2), Severity::Major, 8, 24, false),
        evidence(6, 0, ScandalTarget::Player(1), Severity::Major, 8, 10, false),
        evidence(7, 0, ScandalTarget::Player(1), Severity::Major, 8, 24, true),
    ];
    campaign
}

#[test]
fn scandals_sort_by_severity_then_recency_and_only_show_available_owned_evidence() {
    let campaign = fixture();
    let ids: Vec<_> = filtered_evidence(&campaign, 0, &ScandalFilters::default())
        .iter()
        .map(|scandal| scandal.id)
        .collect();
    assert_eq!(ids, [7, 4, 2, 3, 1]);
    let ids: Vec<_> = filtered_evidence(&campaign, 1, &ScandalFilters::default())
        .iter()
        .map(|scandal| scandal.id)
        .collect();
    assert_eq!(ids, [5], "Other players' evidence stays private");
}

#[test]
fn scandal_player_and_severity_filters_intersect_and_npc_evidence_stays_separate() {
    let campaign = fixture();
    let mut filters = ScandalFilters {
        target: TargetFilter::Player(1),
        ..Default::default()
    };
    let ids = |filters: &ScandalFilters| {
        filtered_evidence(&campaign, 0, filters)
            .iter()
            .map(|scandal| scandal.id)
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&filters), [7, 2, 1]);
    filters.severity[2] = false;
    assert_eq!(ids(&filters), [1]);
    filters.target = TargetFilter::Provinces;
    assert_eq!(ids(&filters), [3]);
    filters.severity.fill(false);
    assert!(ids(&filters).is_empty());
}

fn render(
    ctx: &egui::Context,
    campaign: &Campaign,
    filters: &mut ScandalFilters,
    width: f32,
    scale: f32,
    time: f64,
    events: Vec<egui::Event>,
) -> (egui::FullOutput, Option<u64>, egui::Rect) {
    let mut selected = None;
    let mut bounds = egui::Rect::NOTHING;
    let output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width + 16.0, 700.0),
            )),
            time: Some(time),
            events,
            ..Default::default()
        },
        |ui| {
            *ui.style_mut() = super::super::campaign_widgets::map_style(scale);
            ui.set_width(width);
            selected = overview(ui, campaign, 0, &super::super::PLAYER_COLORS[..3], filters, scale)
                .selected;
            bounds = ui.min_rect();
        },
    );
    (output, selected, bounds)
}

fn click(pos: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        },
    ]
}

#[test]
fn scandal_rows_show_targets_expiry_and_fit_compact_widths() {
    for (width, scale) in [(546.0, 1.0), (380.0, 1.0), (320.0, 0.85)] {
        let ctx = layout_context();
        let campaign = fixture();
        let (mut output, _, bounds) =
            render(&ctx, &campaign, &mut ScandalFilters::default(), width, scale, 0.0, vec![]);
        let mut capture = capture::Capture::default();
        capture.frame(&ctx, &output, &format!("scandals-{}", width as u32));
        assert!(bounds.width() <= width + 1.0, "Scandals overflow at {width}px: {bounds:?}");
        let text = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        for player in [1, 2] {
            assert!(
                output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(rect)
                if rect.fill == super::super::PLAYER_COLORS[player]
                    && (rect.rect.width() - 6.0 * scale).abs() < 0.01
                    && (rect.rect.height() - 34.0 * scale).abs() < 0.01)),
                "Player markers must match the province and spy directories"
            );
        }
        for label in [
            "Against Player 2",
            "Against Player 3",
            "Against Achaia",
            "14 months remaining",
            "Found in Achaia",
            "III · grave",
            "Reserved until vote",
            "SCANDAL",
            "SEVERITY",
            "VALIDITY",
        ] {
            assert!(text.contains(label), "Missing {label}");
        }
        assert!(!text.contains("Severity ") && !text.contains("DISCOVERED SCANDALS"));
        let rows: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Rect(row)
                    if (row.rect.height() - 50.0 * scale).abs() < 0.1
                        && row.rect.width() > width * 0.9 =>
                {
                    Some(row.rect)
                },
                _ => None,
            })
            .collect();
        assert!(!rows.is_empty(), "Scandals must use the active-network row height");
        for shape in &output.shapes {
            if let egui::Shape::Text(text) = &shape.shape {
                if text.galley.job.text.starts_with("Against ")
                    || text.galley.job.text.contains(" · grave")
                    || text.galley.job.text.ends_with("remaining")
                    || text.galley.job.text == "Reserved until vote"
                {
                    let bounds = text.galley.rect.translate(text.pos.to_vec2());
                    assert!(
                        rows.iter().any(|row| row.contains_rect(bounds)),
                        "Row label escapes its row: {bounds:?}"
                    );
                }
            }
        }
        let label_rect = |label: &str| {
            output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.job.text == label => {
                        Some(text.galley.rect.translate(text.pos.to_vec2()))
                    },
                    _ => None,
                })
                .unwrap_or_else(|| panic!("Missing filter {label}"))
        };
        let target = label_rect("All targets");
        let first = label_rect("I");
        let second = label_rect("II");
        let third = label_rect("III");
        let banner = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Mesh(mesh) if mesh.vertices.len() == 4 => Some(mesh.calc_bounds()),
                _ => None,
            })
            .expect("Scandal banner must be painted");
        for filter in [target, first, second, third] {
            assert!(banner.contains_rect(filter), "Filters must stay inside the banner");
        }
        assert!(
            target.left() < first.left()
                && first.left() < second.left()
                && second.left() < third.left()
        );
        assert!(
            (banner.right() - third.right() - 13.0 * scale).abs() < 3.0 * scale,
            "Severity filters must align inside the right banner badge"
        );
        assert!(second.left() - first.right() > 25.0 * scale);
        assert!(third.left() - second.right() > 25.0 * scale);
        assert!(
            !output.shapes.iter().any(|shape| matches!(
                &shape.shape,
                egui::Shape::LineSegment { points, .. }
                    if points[0].y == points[1].y
                        && points[0].y >= banner.bottom()
                        && (points[0].x - points[1].x).abs() > width * 0.9
            )),
            "No separator beneath the banner"
        );
        assert!((first.center().y - target.center().y).abs() < scale, "Filters must share a row");
        let dropdown = target.center();
        output.textures_delta.clear();
        let mut filters = ScandalFilters::default();
        for (time, pressed) in [(0.1, true), (0.2, false)] {
            let (mut opened, _, _) =
                render(&ctx, &campaign, &mut filters, width, scale, time, click(dropdown, pressed));
            capture.frame(&ctx, &opened, &format!("scandals-dropdown-{}", width as u32));
            if !pressed {
                let frame = opened
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Rect(rect)
                            if rect.rect.contains(dropdown)
                                && rect.rect.height() < 30.0 * scale =>
                        {
                            Some(rect)
                        },
                        _ => None,
                    })
                    .expect("Open dropdown retains its button frame");
                assert!(
                    frame.fill.a() < 100,
                    "Open dropdown must retain the dark banner badge's translucent highlight"
                );
            }
            opened.textures_delta.clear();
        }
        let (mut opened, _, _) = render(&ctx, &campaign, &mut filters, width, scale, 0.3, vec![]);
        capture.frame(&ctx, &opened, &format!("scandals-dropdown-{}", width as u32));
        opened.textures_delta.clear();
        // Let the popup's normal fade-in finish before checking its final colors.
        let (mut opened, _, _) = render(&ctx, &campaign, &mut filters, width, scale, 0.6, vec![]);
        capture.frame(&ctx, &opened, &format!("scandals-dropdown-{}", width as u32));
        let mut options = egui::Rect::NOTHING;
        for label in ["Player 1", "Player 2", "Player 3", "Independent provinces"] {
            let text = opened
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.job.text == label => Some(text),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("Missing dropdown option {label}"));
            options = options.union(text.galley.rect.translate(text.pos.to_vec2()));
            assert!(
                text.galley.job.sections[0].format.color.r() < 100,
                "Popup options must use readable parchment ink"
            );
        }
        let popup = opened
            .shapes
            .iter()
            .flat_map(|shape| match &shape.shape {
                egui::Shape::Vec(shapes) => shapes.as_slice(),
                shape => std::slice::from_ref(shape),
            })
            .filter_map(|shape| match shape {
                egui::Shape::Rect(rect)
                    if rect.stroke.width > 0.0
                        && rect.rect.contains_rect(options)
                        && rect.rect.width() < width * 0.8
                        && rect.rect.top() >= target.bottom() =>
                {
                    Some(rect)
                },
                _ => None,
            })
            .min_by(|a, b| a.rect.area().total_cmp(&b.rect.area()))
            .expect("Target dropdown must paint its popup frame");
        assert!(
            popup.fill.r() > 180 && popup.fill.a() == 255,
            "Dropdown options must have an opaque parchment background: {:?}",
            popup.fill
        );
        opened.textures_delta.clear();
    }
}

#[test]
fn permanent_scandals_show_their_validity_without_an_artificial_countdown() {
    let ctx = layout_context();
    let mut campaign = fixture();
    campaign.espionage.scandals[3].expires = u32::MAX;
    let (mut output, _, _) =
        render(&ctx, &campaign, &mut ScandalFilters::default(), 546.0, 1.0, 0.0, vec![]);
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == "Permanent")));
    assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text.contains("429496"))));
    output.textures_delta.clear();
}

#[test]
fn scandal_card_click_returns_its_evidence_and_severity_checkbox_filters_the_list() {
    let ctx = layout_context();
    let campaign = fixture();
    let mut filters = ScandalFilters::default();
    let (mut output, _, _) = render(&ctx, &campaign, &mut filters, 546.0, 1.0, 0.0, vec![]);
    let position = |label: &str| {
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
    };
    let card = position(ScandalKind::Famine.label());
    let checkbox = position("III");
    output.textures_delta.clear();
    let mut selected = None;
    for (time, pressed) in [(0.1, true), (0.2, false)] {
        let (mut output, clicked, _) =
            render(&ctx, &campaign, &mut filters, 546.0, 1.0, time, click(card, pressed));
        selected = clicked.or(selected);
        output.textures_delta.clear();
    }
    assert_eq!(selected, Some(7));
    for (time, pressed) in [(0.3, true), (0.4, false)] {
        let (mut output, _, _) =
            render(&ctx, &campaign, &mut filters, 546.0, 1.0, time, click(checkbox, pressed));
        output.textures_delta.clear();
    }
    assert!(!filters.severity[2]);
    assert_eq!(
        filtered_evidence(&campaign, 0, &filters)
            .iter()
            .map(|scandal| scandal.id)
            .collect::<Vec<_>>(),
        [3, 1]
    );
}

fn action_fixture(target: ScandalTarget) -> Campaign {
    use crate::game::economy::EconomyWorld;
    use crate::game::military::{MilitaryProvince, MilitaryTerrain};
    use crate::game::politics::diplomacy::ProvincePolitics;
    let mut campaign = fixture();
    campaign.espionage.scandals.truncate(1);
    let scandal = &mut campaign.espionage.scandals[0];
    scandal.target = target;
    scandal.kind = ScandalKind::SecretPayments;
    scandal.expires = u32::MAX;
    campaign.politics = vec![if matches!(target, ScandalTarget::Player(_)) {
        campaign.economy.provinces[0].owner = Some(1);
        ProvincePolitics::owned(3, 1)
    } else {
        ProvincePolitics::independent(3)
    }];
    campaign.economy = EconomyWorld::new(3, campaign.economy.provinces.clone(), vec![vec![]]);
    campaign.graph = vec![MilitaryProvince {
        terrain: MilitaryTerrain::Plains,
        area: 60.0,
        road_level: 0,
        neighbors: vec![],
    }];
    campaign
}

fn render_actions(
    ctx: &egui::Context,
    campaign: &Campaign,
    width: f32,
    time: f64,
    events: Vec<egui::Event>,
) -> (egui::FullOutput, ScandalResponse) {
    let mut result = ScandalResponse::default();
    let output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width + 16.0, 500.0),
            )),
            time: Some(time),
            events,
            ..Default::default()
        },
        |ui| {
            *ui.style_mut() = super::super::campaign_widgets::map_style(1.0);
            ui.set_width(width);
            result = overview(
                ui,
                campaign,
                0,
                &super::super::PLAYER_COLORS[..3],
                &mut ScandalFilters::default(),
                1.0,
            );
        },
    );
    (output, result)
}

#[test]
fn scandal_action_buttons_spend_evidence_without_opening_the_row_for_both_targets() {
    for target in [ScandalTarget::Province(0), ScandalTarget::Player(1)] {
        for usage in [ScandalUse::Control, ScandalUse::Relation] {
            let ctx = layout_context();
            let mut campaign = action_fixture(target);
            let (mut output, _) = render_actions(&ctx, &campaign, 546.0, 0.0, vec![]);
            let mut capture = capture::Capture::default();
            capture.frame(
                &ctx,
                &output,
                if matches!(target, ScandalTarget::Player(_)) {
                    "scandal-player-actions"
                } else {
                    "scandal-npc-actions"
                },
            );
            let buttons: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.job.text == "+5" => Some(text.pos),
                    _ => None,
                })
                .collect();
            assert_eq!(buttons.len(), 2);
            let pos = buttons[usize::from(usage == ScandalUse::Relation)] - egui::vec2(0.0, 10.0);
            output.textures_delta.clear();
            let mut command = None;
            for (time, pressed) in [(0.1, true), (0.2, false)] {
                let (mut output, result) =
                    render_actions(&ctx, &campaign, 546.0, time, click(pos, pressed));
                assert_eq!(result.selected, None, "Using evidence must not also open its row");
                command = result.command.or(command);
                output.textures_delta.clear();
            }
            let command = command.expect("The action button must dispatch a scandal use");
            assert_eq!(
                command,
                ScandalCommand::Province {
                    id: 1,
                    province: 0,
                    usage
                }
            );
            let message = apply_command(&mut campaign, 0, command);
            assert!(message.contains("+5"));
            assert!(campaign.espionage.scandals.is_empty());
            assert_eq!(
                if usage == ScandalUse::Control {
                    campaign.politics[0].control(0)
                } else {
                    campaign.politics[0].relation(0) - 50.0
                },
                5.0
            );
        }
    }
}

#[test]
fn compact_scandal_use_menu_lists_exact_benefits_and_only_players_get_senate_use() {
    for target in [ScandalTarget::Province(0), ScandalTarget::Player(1)] {
        let ctx = layout_context();
        let campaign = action_fixture(target);
        let (mut output, _) = render_actions(&ctx, &campaign, 380.0, 0.0, vec![]);
        let mut capture = capture::Capture::default();
        capture.frame(&ctx, &output, "scandal-compact-actions");
        let pos = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == "Use" => {
                    Some(text.pos - egui::vec2(0.0, 10.0))
                },
                _ => None,
            })
            .unwrap();
        output.textures_delta.clear();
        for (time, pressed) in [(0.1, true), (0.2, false)] {
            let (mut output, result) =
                render_actions(&ctx, &campaign, 380.0, time, click(pos, pressed));
            assert!(result.selected.is_none() && result.command.is_none());
            capture.frame(&ctx, &output, "scandal-compact-actions");
            output.textures_delta.clear();
        }
        let (mut output, _) = render_actions(&ctx, &campaign, 380.0, 0.6, vec![]);
        capture.frame(&ctx, &output, "scandal-compact-menu");
        let labels: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect();
        assert!(labels.iter().any(|label| label.contains("gain 5 Control")));
        assert!(labels.iter().any(|label| label.contains("gain 5 Relation")));
        assert_eq!(
            labels.contains(&"Expose in Senate"),
            matches!(target, ScandalTarget::Player(_))
        );
        assert_eq!(
            labels.iter().any(|label| label.contains("better trade terms")),
            matches!(target, ScandalTarget::Province(_))
        );
        output.textures_delta.clear();
    }
}

#[test]
fn scandal_rows_and_filters_never_show_hover_tooltips() {
    let ctx = layout_context();
    let campaign = action_fixture(ScandalTarget::Province(0));
    let (mut output, _) = render_actions(&ctx, &campaign, 546.0, 0.0, vec![]);
    let baseline: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) => Some(text.galley.job.text.clone()),
            _ => None,
        })
        .collect();
    let mut positions: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) => {
                Some(text.galley.rect.translate(text.pos.to_vec2()).center())
            },
            _ => None,
        })
        .collect();
    let row = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Rect(row)
                if (row.rect.height() - 50.0).abs() < 0.01 && row.rect.width() > 500.0 =>
            {
                Some(row.rect)
            },
            _ => None,
        })
        .unwrap();
    positions.push(egui::pos2(row.left() + 10.0, row.center().y));
    output.textures_delta.clear();
    for (index, pos) in positions.into_iter().enumerate() {
        for time in [1.0 + index as f64 * 3.0, 3.0 + index as f64 * 3.0] {
            let (mut output, _) =
                render_actions(&ctx, &campaign, 546.0, time, vec![egui::Event::PointerMoved(pos)]);
            let labels: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) => Some(text.galley.job.text.clone()),
                    _ => None,
                })
                .collect();
            assert_eq!(labels, baseline, "Hovering scandal widgets must not add tooltip text");
            output.textures_delta.clear();
        }
    }
}

#[test]
fn player_scandals_without_an_origin_require_a_province_choice_before_spending() {
    use crate::game::politics::diplomacy::ProvincePolitics;
    let ctx = layout_context();
    let mut campaign = action_fixture(ScandalTarget::Player(1));
    campaign.espionage.scandals[0].province = None;
    let mut province = campaign.economy.provinces[0].clone();
    province.name = "Narbonensis".into();
    campaign.economy.provinces.push(province);
    campaign.politics.push(ProvincePolitics::owned(3, 1));
    campaign.graph.push(campaign.graph[0].clone());
    let (mut output, _) = render_actions(&ctx, &campaign, 546.0, 0.0, vec![]);
    let pos = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == "+5" => {
                Some(text.pos - egui::vec2(0.0, 10.0))
            },
            _ => None,
        })
        .unwrap();
    output.textures_delta.clear();
    for (time, pressed) in [(0.1, true), (0.2, false)] {
        let (mut output, result) =
            render_actions(&ctx, &campaign, 546.0, time, click(pos, pressed));
        assert!(result.command.is_none() && result.selected.is_none());
        output.textures_delta.clear();
    }
    let (mut output, _) = render_actions(&ctx, &campaign, 546.0, 0.6, vec![]);
    let choice = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text)
                if text.galley.job.text == "Narbonensis · Use scandal: gain 5 Control" =>
            {
                Some(text.pos + egui::vec2(5.0, 5.0))
            },
            _ => None,
        })
        .expect("Global player evidence must list the rival's eligible provinces");
    output.textures_delta.clear();
    let mut command = None;
    for (time, pressed) in [(0.7, true), (0.8, false)] {
        let (mut output, result) =
            render_actions(&ctx, &campaign, 546.0, time, click(choice, pressed));
        assert!(result.selected.is_none());
        command = result.command.or(command);
        output.textures_delta.clear();
    }
    let command = command.expect("Selecting a province must dispatch its benefit");
    assert_eq!(
        command,
        ScandalCommand::Province {
            id: 1,
            province: 1,
            usage: ScandalUse::Control
        }
    );
    apply_command(&mut campaign, 0, command);
    assert_eq!(campaign.politics[0].control(0), 0.0);
    assert_eq!(campaign.politics[1].control(0), 5.0);
    assert!(campaign.espionage.scandals.is_empty());
}

#[test]
fn senate_row_button_exposes_the_player_scandal_directly() {
    let ctx = layout_context();
    let mut campaign = action_fixture(ScandalTarget::Player(1));
    let (mut output, _) = render_actions(&ctx, &campaign, 546.0, 0.0, vec![]);
    let pos = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == "Senate" => {
                Some(text.pos - egui::vec2(0.0, 10.0))
            },
            _ => None,
        })
        .unwrap();
    output.textures_delta.clear();
    let mut command = None;
    for (time, pressed) in [(0.1, true), (0.2, false)] {
        let (mut output, result) =
            render_actions(&ctx, &campaign, 546.0, time, click(pos, pressed));
        assert!(result.selected.is_none());
        command = result.command.or(command);
        output.textures_delta.clear();
    }
    let command = command.expect("The Senate button must spend the selected scandal");
    assert_eq!(
        command,
        ScandalCommand::Senate {
            id: 1
        }
    );
    assert!(apply_command(&mut campaign, 0, command).contains("Player 2"));
    assert!(campaign.espionage.scandals.is_empty());
}
