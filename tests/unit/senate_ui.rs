//! Check the real chamber paint output and section hover behavior.

use super::*;

fn render(
    ctx: &egui::Context,
    senate: &SenateState,
    width: f32,
    time: f64,
    pointer: Option<egui::Pos2>,
) -> (egui::FullOutput, egui::Rect) {
    let players = vec![PoliticalPlayer::default(); 3];
    let profiles = vec![PoliticalProfile::default(); 3];
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
                        egui::vec2(width, width * 0.56),
                    );
                    draw_chamber(
                        ui,
                        senate,
                        &players,
                        &profiles,
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
fn hovering_between_senators_explains_the_faction_and_its_live_influences() {
    let senate = SenateState::new(1);
    let ctx = egui::Context::default();
    let (_, bounds) = render(&ctx, &senate, 680.0, 0.0, None);
    let pointer = egui::pos2(bounds.center().x, bounds.bottom() - 6.0 - 680.0 * 0.47 * 0.76);
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
        tooltip |= texts.contains(&Bloc::Provincials.preferences())
            && texts.iter().any(|s| s.contains("Free-pop happiness:"))
            && texts.iter().any(|s| s.contains("Exposed scandals (fades):"));
    }
    assert!(tooltip, "Section gaps must show the actual faction tooltip without requiring a precise circle hover");
}
