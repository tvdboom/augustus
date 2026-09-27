use bevy_egui::egui;

#[test]
fn land_portraits_use_original_resolution() {
    for (name, bytes) in super::TERRAIN_IMAGES.iter().take(9) {
        let image = image::load_from_memory(bytes).expect("terrain portrait must be valid");
        assert_eq!(image.width(), 735, "{name}");
        assert_eq!(image.height(), 92, "{name}");
    }
}

#[test]
fn city_banner_respects_egui_texture_limits() {
    let banner = include_bytes!("../../assets/images/cities/city-panel-banner.png");
    for limit in [2048, 1024] {
        let context = egui::Context::default();
        context.begin_pass(egui::RawInput {
            max_texture_side: Some(limit),
            ..Default::default()
        });
        let texture = super::load_image(&context, ("city", banner), "test-banner");
        assert!(texture.size()[0] <= limit);
        assert!(texture.size()[1] <= limit);
        assert_eq!(texture.size()[0], limit);
        let mut output = context.end_pass();
        output.textures_delta.clear();
    }
}
