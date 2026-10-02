use super::*;

#[test]
fn spectator_status_is_only_text_at_the_bottom_right() {
    for size in [egui::vec2(1600.0, 900.0), egui::vec2(1024.0, 768.0)] {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ui| status(ui.ctx()),
        );
        let texts: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text),
                _ => None,
            })
            .collect();
        assert_eq!(texts.len(), 2, "Only the status and its shadow are painted");
        for text in texts {
            assert_eq!(text.galley.job.text, "Spectator");
            let bounds = egui::Rect::from_min_size(text.pos, text.galley.size());
            assert!(screen.contains_rect(bounds));
            assert!(bounds.left() > screen.center().x);
            assert!(bounds.top() > screen.bottom() - 50.0);
        }
        assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(_))));
        output.textures_delta.clear();
    }
}
