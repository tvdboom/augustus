use super::*;
use crate::game::economy::{EconomicProvince, Terrain};
use crate::game::politics::espionage::ScandalKind;
use crate::game::politics::PoliticalPlayer;

fn fixture() -> Campaign {
    let mut campaign = Campaign::default();
    campaign.actors = vec![PoliticalPlayer::default(); 3];
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
    assert_eq!(ids, [4, 2, 3, 1]);
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
    assert_eq!(ids(&filters), [2, 1]);
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
            selected = overview(ui, campaign, 0, filters, scale);
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
fn scandal_cards_show_targets_expiry_and_fit_compact_widths() {
    for (width, scale) in [(546.0, 1.0), (380.0, 1.0), (320.0, 0.85)] {
        let ctx = egui::Context::default();
        let campaign = fixture();
        let (mut output, _, bounds) =
            render(&ctx, &campaign, &mut ScandalFilters::default(), width, scale, 0.0, vec![]);
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
        for label in [
            "Against Player 2",
            "Against Player 3",
            "Against Achaia",
            "14 months remaining",
            "Found in Achaia",
            "Severity III · grave",
        ] {
            assert!(text.contains(label), "Missing {label}");
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
        assert!(
            target.left() < first.left()
                && first.left() < second.left()
                && second.left() < third.left()
        );
        assert!(
            (bounds.right() - third.right()).abs() < 4.0 * scale,
            "Severity filters must align to the right edge"
        );
        assert!((first.center().y - target.center().y).abs() < scale, "Filters must share a row");
        let dropdown = target.center();
        output.textures_delta.clear();
        let mut filters = ScandalFilters::default();
        for (time, pressed) in [(0.1, true), (0.2, false)] {
            let (mut opened, _, _) =
                render(&ctx, &campaign, &mut filters, width, scale, time, click(dropdown, pressed));
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
                    frame.fill.r() > 180 && frame.fill.g() > 160,
                    "Open dropdown must use a light parchment fill"
                );
            }
            opened.textures_delta.clear();
        }
    }
}

#[test]
fn scandal_card_click_returns_its_evidence_and_severity_checkbox_filters_the_list() {
    let ctx = egui::Context::default();
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
    assert_eq!(selected, Some(4));
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
