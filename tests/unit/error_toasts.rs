use super::*;

#[test]
fn database_retries_refresh_a_badge_and_error_storms_stay_bounded() {
    let mut errors = ErrorNotifications::default();
    errors.push("anonymous sign-ins are disabled".into());
    errors.advance(4.0);
    errors.push("anonymous sign-ins are disabled".into());
    assert_eq!(errors.0.len(), 1);
    assert_eq!(errors.0[0].text, "Anonymous sign-ins are disabled");
    errors.advance(4.0);
    assert_eq!(errors.0.len(), 1, "A failed retry gets a fresh display lifetime");
    errors.advance(1.0);
    assert!(errors.0.is_empty());
    for index in 0..20 {
        errors.push(format!("Failure {index}"));
    }
    assert_eq!(errors.0.len(), MAX_ERRORS);
    assert_eq!(errors.0.front().unwrap().text, "Failure 14");
}

#[test]
fn database_badges_match_the_reference_and_wrap_inside_the_right_edge() {
    let messages = [
        "Anonymous sign-ins are disabled",
        "The database is not set up yet. Run supabase/schema.sql in the Supabase SQL Editor.",
        "Game is full.",
    ];
    for (name, size) in
        [("desktop", egui::vec2(1600.0, 900.0)), ("compact", egui::vec2(360.0, 640.0))]
    {
        let context = egui::Context::default();
        context.set_global_style(augustus_ui_style());
        context.global_style_mut(|style| style.animation_time = 0.0);
        let mut errors = ErrorNotifications::default();
        for message in messages {
            errors.push(message.into());
        }
        let mut capture = crate::egui_capture::Capture::default();
        for frame in 0..3 {
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                    time: Some(frame as f64 * 0.1),
                    ..Default::default()
                },
                |_| {
                    show(&context, &errors);
                    assert!(campaign_toast_top(&context, 1.0) > 76.0);
                },
            );
            capture.frame(&context, &output, &format!("database-errors-{name}"));
            output.textures_delta.clear();
            if frame < 2 {
                continue;
            }
            let badges: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Rect(rect) if rect.fill == error_fill() => {
                        assert!(
                            shape.clip_rect.contains_rect(rect.rect),
                            "A scaled badge must not be clipped: {name}"
                        );
                        Some(rect)
                    },
                    _ => None,
                })
                .collect();
            assert_eq!(badges.len(), messages.len());
            let scale = (viewport_ui_scale(size) * 1.1).clamp(0.8, 1.35);
            for badge in &badges {
                assert!(badge.rect.left() >= 0.0, "Wrapped badges fit: {name} {:?}", badge.rect);
                assert!((size.x - badge.rect.right() - 12.0 * scale).abs() < 1.0);
                assert_eq!(badge.stroke.color, ERROR_ACCENT);
            }
            for pair in badges.windows(2) {
                assert!(pair[0].rect.bottom() < pair[1].rect.top());
            }
            for message in messages {
                assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
                    egui::Shape::Text(text) if text.galley.job.text == message
                        && text.galley.job.sections[0].format.color == ERROR_ACCENT)));
            }
        }
        let mut output = context.run_ui(egui::RawInput::default(), |_| {
            assert_eq!(
                campaign_toast_top(&context, 1.0),
                76.0,
                "Expired stack measurements do not leave a gap"
            );
        });
        output.textures_delta.clear();
    }
}
