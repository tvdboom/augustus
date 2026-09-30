use super::*;

fn render(
    ctx: &egui::Context,
    pointer: egui::Pos2,
    cursor: Option<egui::CursorIcon>,
) -> egui::FullOutput {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 600.0),
            )),
            events: vec![egui::Event::PointerMoved(pointer)],
            ..Default::default()
        },
        |root| {
            egui::Area::new(egui::Id::new("cursor-test-panel"))
                .order(egui::Order::Foreground)
                .fixed_pos(egui::pos2(60.0, 60.0))
                .sense(egui::Sense::hover())
                .show(root.ctx(), |ui| {
                    let (panel, _) =
                        ui.allocate_exact_size(egui::vec2(500.0, 468.0), egui::Sense::hover());
                    let action = egui::Rect::from_min_size(
                        panel.min + egui::vec2(20.0, 20.0),
                        egui::vec2(80.0, 30.0),
                    );
                    let response =
                        ui.interact(action, ui.id().with("action"), egui::Sense::click());
                    if let Some(cursor) = cursor {
                        response.on_hover_cursor(cursor);
                    }
                    let disabled = action.translate(egui::vec2(120.0, 0.0));
                    ui.add_enabled_ui(false, |ui| {
                        ui.interact(disabled, ui.id().with("disabled"), egui::Sense::click());
                    });
                });
        },
    );
    output.textures_delta.clear();
    output
}

#[test]
fn cursor_only_points_on_explicit_foreground_actions() {
    for cursor in [None, Some(egui::CursorIcon::PointingHand)] {
        let ctx = egui::Context::default();
        configure_cursor(&ctx);
        let blank = egui::pos2(300.0, 300.0);
        // Warm the area layout before testing consecutive hover frames.
        render(&ctx, blank, cursor);
        assert_eq!(
            render(&ctx, blank, cursor).platform_output.cursor_icon,
            egui::CursorIcon::Default
        );
        for destination in [blank, egui::pos2(230.0, 95.0)] {
            assert_eq!(
                render(&ctx, egui::pos2(100.0, 95.0), cursor).platform_output.cursor_icon,
                egui::CursorIcon::PointingHand
            );
            assert_eq!(
                render(&ctx, destination, cursor).platform_output.cursor_icon,
                egui::CursorIcon::Default
            );
        }
    }
}

#[test]
fn current_text_and_drag_cursors_are_preserved_but_do_not_stick() {
    for cursor in [egui::CursorIcon::Text, egui::CursorIcon::Grabbing] {
        let ctx = egui::Context::default();
        configure_cursor(&ctx);
        let blank = egui::pos2(300.0, 300.0);
        render(&ctx, blank, Some(cursor));
        render(&ctx, blank, Some(cursor));
        assert_eq!(
            render(&ctx, egui::pos2(100.0, 95.0), Some(cursor)).platform_output.cursor_icon,
            cursor
        );
        assert_eq!(
            render(&ctx, blank, Some(cursor)).platform_output.cursor_icon,
            egui::CursorIcon::Default
        );
    }
}
