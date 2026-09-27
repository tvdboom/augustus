use super::*;

#[test]
fn pointer_can_move_from_population_icon_to_class_and_province_detail() {
    let context = egui::Context::default();
    let pixel = egui::ColorImage::from_rgba_unmultiplied([1, 1], &[255, 255, 255, 255]);
    let icon = context.load_texture("test-pop", pixel.clone(), egui::TextureOptions::LINEAR);
    let class_icons = std::array::from_fn(|index| {
        context.load_texture(
            format!("test-class-{index}"),
            pixel.clone(),
            egui::TextureOptions::LINEAR,
        )
    });
    let mut ownership = ProvinceOwnership::default();
    ownership.start_game(&[egui::Color32::RED]);
    let total = ownership.population_for(0).into_iter().sum();
    let population = HudResource {
        amount: total,
        monthly_delta: 1.0,
    };
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 900.0));
    let x = hud_resource_positions()[5] + 40.0;
    let mut open = false;
    let mut class = None;
    for (pointer, expected_open, expected_class) in [
        (egui::pos2(x, 24.0), true, None),
        (egui::pos2(x, 45.0 + HEADER_HEIGHT + SUMMARY_HEIGHT + CLASS_HEIGHT * 2.5), true, Some(2)),
        (
            egui::pos2(
                x,
                45.0 + HEADER_HEIGHT
                    + SUMMARY_HEIGHT
                    + CLASS_HEIGHT * 4.0
                    + DETAIL_HEADER_HEIGHT
                    + 10.0,
            ),
            true,
            Some(2),
        ),
        (egui::pos2(1200.0, 500.0), false, None),
    ] {
        context.begin_pass(egui::RawInput {
            screen_rect: Some(screen),
            events: vec![egui::Event::PointerMoved(pointer)],
            ..Default::default()
        });
        show(
            &context,
            screen,
            1.0,
            1200.0,
            0,
            &ownership,
            population,
            &icon,
            &class_icons,
            &mut open,
            &mut class,
        );
        assert_eq!((open, class), (expected_open, expected_class));
        let mut output = context.end_pass();
        output.textures_delta.clear();
    }
}
