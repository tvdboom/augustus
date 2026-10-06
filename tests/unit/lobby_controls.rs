use super::*;

#[test]
fn color_palette_does_not_dim_during_requests() {
    let context = egui::Context::default();
    let mut selected = 2;
    for busy in [false, true, false] {
        let mut output = context.run_ui(egui::RawInput::default(), |ui| {
            assert!(!lobby_color_picker(ui, &mut selected, &[1], busy));
        });
        let fills: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Circle(circle) if circle.radius == 11.0 => Some(circle.fill),
                _ => None,
            })
            .collect();
        let mut expected = PLAYER_COLORS.to_vec();
        expected[1] = expected[1].gamma_multiply(0.32);
        assert_eq!(fills, expected, "Polling must preserve every available house color");
        assert_eq!(selected, 2);
        output.textures_delta.clear();
    }
}
