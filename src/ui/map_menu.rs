//! Map menu controls and click regions.

use super::*;

pub(super) const MAP_MENU_ICONS: [(&str, &[u8]); 4] = [
    ("Governance", include_bytes!("../../assets/images/ui/map-menu/laws.png")),
    ("Military", include_bytes!("../../assets/images/ui/map-menu/military.png")),
    ("Trade", include_bytes!("../../assets/images/ui/map-menu/trade.png")),
    ("Politics", include_bytes!("../../assets/images/ui/map-menu/politics.png")),
];

pub(super) fn map_menu_icon(
    ctx: &egui::Context,
    textures: &mut [Option<egui::TextureHandle>; 4],
    index: usize,
) -> egui::TextureHandle {
    textures[index]
        .get_or_insert_with(|| {
            let (name, bytes) = MAP_MENU_ICONS[index];
            let image =
                image::load_from_memory(bytes).expect("map menu icon PNG must be valid").to_rgba8();
            // The source art has uneven transparent margins. Crop those margins so
            // centering the texture also centers the visible glyph.
            let mut min = (image.width(), image.height());
            let mut max = (0, 0);
            for (x, y, pixel) in image.enumerate_pixels() {
                if pixel[3] > 8 {
                    min.0 = min.0.min(x);
                    min.1 = min.1.min(y);
                    max.0 = max.0.max(x + 1);
                    max.1 = max.1.max(y + 1);
                }
            }
            let image = if max.0 > min.0 && max.1 > min.1 {
                image::imageops::crop_imm(&image, min.0, min.1, max.0 - min.0, max.1 - min.1)
                    .to_image()
            } else {
                image
            };
            let image = image::DynamicImage::ImageRgba8(image)
                .resize(128, 128, image::imageops::FilterType::Lanczos3)
                .to_rgba8();
            ctx.load_texture(
                format!("augustus_map_menu_{name}"),
                egui::ColorImage::from_rgba_unmultiplied(
                    [image.width() as usize, image.height() as usize],
                    image.as_raw(),
                ),
                egui::TextureOptions::LINEAR,
            )
        })
        .clone()
}

pub(super) fn draw_map_left_menu(
    ctx: &egui::Context,
    scale: f32,
    textures: &mut [Option<egui::TextureHandle>; 4],
    interactive: bool,
) -> bool {
    let screen = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("augustus_map_edge_frame"),
    ));
    let standard_height = map_standard_height(screen, scale);
    if standard_height - MAP_STANDARD_FLAG_HEIGHT < 260.0 {
        return false;
    }
    // Centers of the four cloth panels, measured between the thin seams in
    // player-standard-*.png. The edge frame maps source y 20..1684 to the rail.
    const PANEL_CENTERS: [f32; 4] = [615.0, 827.0, 1041.0, 1255.0];
    // The visible cloth is centered at x=29 after the standard's transparent
    // edge is mapped to the rail; the 64-pixel hitbox extends farther right.
    const CLOTH_CENTER_X: f32 = 29.0;
    let mut governance_clicked = false;
    for (index, (name, _)) in MAP_MENU_ICONS.iter().enumerate() {
        let icon = map_menu_icon(ctx, textures, index);
        let center_y = (PANEL_CENTERS[index] - 20.0) / (1684.0 - 20.0) * standard_height;
        let top_left = screen.min + egui::vec2(CLOTH_CENTER_X - 28.0, center_y - 32.0) * scale;
        let (rect, pressed, hovered, clicked) = if interactive {
            egui::Area::new(egui::Id::new(("augustus_map_left_menu", name)))
                .fixed_pos(top_left)
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    let (rect, response) = ui
                        .allocate_exact_size(egui::vec2(56.0, 64.0) * scale, egui::Sense::click());
                    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, *name)
                    });
                    let now = ui.input(|input| input.time);
                    let flash_id = response.id.with("flash_until");
                    let flash_until = if response.clicked() {
                        let until = now + 0.16;
                        ui.ctx().data_mut(|data| data.insert_temp(flash_id, until));
                        until
                    } else {
                        ui.ctx().data(|data| data.get_temp::<f64>(flash_id).unwrap_or(0.0))
                    };
                    let pressed = response.is_pointer_button_down_on() || now < flash_until;
                    let hovered = response.hovered() || response.has_focus();
                    if hovered {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if pressed {
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
                    }
                    (rect, pressed, hovered, response.clicked())
                })
                .inner
        } else {
            (
                egui::Rect::from_min_size(top_left, egui::vec2(56.0, 64.0) * scale),
                false,
                false,
                false,
            )
        };
        if index == 0 && clicked {
            governance_clicked = true;
        }
        let icon_size = icon.size_vec2() / icon.size_vec2().max_elem() * 44.0 * scale;
        painter.image(
            icon.id(),
            egui::Rect::from_center_size(rect.center(), icon_size),
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            if pressed {
                egui::Color32::from_rgb(255, 239, 195)
            } else if hovered {
                egui::Color32::from_rgb(255, 221, 160)
            } else {
                egui::Color32::WHITE
            },
        );
    }
    governance_clicked
}

pub(super) const MAP_STANDARD_WIDTH: f32 = 146.0;
pub(super) const MAP_STANDARD_RAIL_IMAGE_WIDTH: f32 = 120.0;
pub(super) const MAP_STANDARD_RAIL_WIDTH: f32 = 64.0;
pub(super) const MAP_STANDARD_FLAG_HEIGHT: f32 = 245.0;
pub(super) const MAP_RESOURCE_STRIP_LEFT: f32 = MAP_STANDARD_WIDTH - 2.0;
pub(super) const MAP_RESOURCE_STRIP_HEIGHT: f32 = 48.0;
pub(super) const MAP_DATE_SECTION_WIDTH: f32 = 230.0;

pub(super) fn map_standard_height(screen: egui::Rect, scale: f32) -> f32 {
    (screen.height() / scale - 35.0).clamp(300.0, 842.0)
}

pub(super) fn map_resource_strip_right(screen: egui::Rect, scale: f32) -> f32 {
    (screen.width() / scale - 170.0).max(310.0)
}

pub(super) fn map_resource_strip_visible(screen: egui::Rect, scale: f32) -> bool {
    map_resource_strip_right(screen, scale) > MAP_RESOURCE_STRIP_LEFT + MAP_DATE_SECTION_WIDTH
}

pub(crate) fn map_hud_contains(screen: egui::Rect, pointer: egui::Pos2) -> bool {
    if !screen.contains(pointer) {
        return false;
    }
    let scale = viewport_ui_scale(screen.size());
    let local = (pointer - screen.min) / scale;
    (local.x < MAP_STANDARD_WIDTH && local.y < MAP_STANDARD_FLAG_HEIGHT)
        || (local.x < MAP_STANDARD_RAIL_WIDTH && local.y < map_standard_height(screen, scale))
        || (map_resource_strip_visible(screen, scale)
            && (MAP_RESOURCE_STRIP_LEFT..map_resource_strip_right(screen, scale))
                .contains(&local.x)
            && local.y < MAP_RESOURCE_STRIP_HEIGHT)
}

pub(super) fn draw_map_menu_hitboxes(ctx: &egui::Context, scale: f32) {
    let screen = ctx.content_rect();
    let rail_height = map_standard_height(screen, scale) - MAP_STANDARD_FLAG_HEIGHT;
    let strip_right = map_resource_strip_right(screen, scale);
    let date_left = strip_right - MAP_DATE_SECTION_WIDTH;
    for (id, top, width, height) in [
        ("augustus_standard_flag_hitbox", 0.0, MAP_STANDARD_WIDTH, MAP_STANDARD_FLAG_HEIGHT),
        (
            "augustus_standard_rail_hitbox",
            MAP_STANDARD_FLAG_HEIGHT,
            MAP_STANDARD_RAIL_WIDTH,
            rail_height,
        ),
    ] {
        egui::Area::new(egui::Id::new(id))
            .fixed_pos(screen.min + egui::vec2(0.0, top) * scale)
            // These drag guards overlap the menu icons, so keep the icons on
            // the higher layer for reliable hit testing.
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                let (_, response) = ui.allocate_exact_size(
                    egui::vec2(width, height) * scale,
                    egui::Sense::click_and_drag(),
                );
                response.on_hover_cursor(egui::CursorIcon::Default);
            });
    }
    if map_resource_strip_visible(screen, scale) {
        egui::Area::new(egui::Id::new("augustus_resource_strip_hitbox"))
            .fixed_pos(screen.min + egui::vec2(MAP_RESOURCE_STRIP_LEFT, 0.0) * scale)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let (_, response) = ui.allocate_exact_size(
                    egui::vec2(date_left - MAP_RESOURCE_STRIP_LEFT, MAP_RESOURCE_STRIP_HEIGHT)
                        * scale,
                    egui::Sense::click_and_drag(),
                );
                response.on_hover_cursor(egui::CursorIcon::Default);
            });
    }
}

pub(super) fn map_date_hitbox(
    ctx: &egui::Context,
    scale: f32,
    id: &'static str,
    left: f32,
    width: f32,
    enabled: bool,
) -> egui::Response {
    let screen = ctx.content_rect();
    let date_left = map_resource_strip_right(screen, scale) - MAP_DATE_SECTION_WIDTH;
    egui::Area::new(egui::Id::new(("augustus_map_date", id)))
        .fixed_pos(screen.min + egui::vec2(date_left + left, 5.0) * scale)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let sense = if enabled {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            };
            let (_, response) = ui.allocate_exact_size(egui::vec2(width, 38.0) * scale, sense);
            response.on_hover_cursor(if enabled {
                egui::CursorIcon::PointingHand
            } else {
                egui::CursorIcon::Default
            })
        })
        .inner
}

pub(super) fn load_player_standard(ctx: &egui::Context, color_index: usize) -> egui::TextureHandle {
    let (name, png) = PLAYER_STANDARDS[color_index];
    let mut rgba =
        image::load_from_memory(png).expect("player standard PNG must be valid").to_rgba8();
    let max_side = ctx.input(|input| input.max_texture_side).max(1) as u32;
    if rgba.width() > max_side || rgba.height() > max_side {
        let fraction = (max_side as f32 / rgba.width().max(rgba.height()) as f32).min(1.0);
        let width = (rgba.width() as f32 * fraction).floor().max(1.0) as u32;
        let height = (rgba.height() as f32 * fraction).floor().max(1.0) as u32;
        rgba = image::imageops::resize(&rgba, width, height, image::imageops::FilterType::Lanczos3);
    }
    ctx.load_texture(
        name,
        egui::ColorImage::from_rgba_unmultiplied(
            [rgba.width() as usize, rgba.height() as usize],
            rgba.as_raw(),
        ),
        egui::TextureOptions::LINEAR,
    )
}
