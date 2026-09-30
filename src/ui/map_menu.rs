//! Map menu controls and click regions.

use super::*;

pub(in crate::app) const MAP_MENU_ICONS: [(&str, &[u8]); 4] = [
    ("Overview", include_bytes!("../../assets/images/ui/map-menu/laws.png")),
    ("Military", include_bytes!("../../assets/images/ui/map-menu/military.png")),
    ("Trade", include_bytes!("../../assets/images/ui/map-menu/trade.png")),
    ("Senate", include_bytes!("../../assets/images/ui/map-menu/politics.png")),
];

pub(in crate::app) fn map_menu_icon(
    ctx: &egui::Context,
    textures: &mut [Option<egui::TextureHandle>; 4],
    index: usize,
) -> egui::TextureHandle {
    textures[index].get_or_insert_with(|| map_menu_icon_texture(ctx, index)).clone()
}

pub(in crate::app) fn map_menu_icon_texture(
    ctx: &egui::Context,
    index: usize,
) -> egui::TextureHandle {
    let key = egui::Id::new(("augustus-map-menu-icon", index));
    if let Some(texture) = ctx.data(|data| data.get_temp::<egui::TextureHandle>(key)) {
        return texture;
    }
    let texture = {
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
            image::imageops::crop_imm(&image, min.0, min.1, max.0 - min.0, max.1 - min.1).to_image()
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
    };
    ctx.data_mut(|data| data.insert_temp(key, texture.clone()));
    texture
}

pub(in crate::app) fn draw_map_left_menu(
    ctx: &egui::Context,
    scale: f32,
    textures: &mut [Option<egui::TextureHandle>; 4],
    interactive: bool,
) -> Option<usize> {
    let screen = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("augustus_map_edge_frame"),
    ));
    let standard_height = map_standard_height(screen, scale);
    if standard_height - MAP_STANDARD_FLAG_HEIGHT < 260.0 {
        return None;
    }
    // Centers of the four cloth panels, measured between the thin seams in
    // player-standard-*.png. The edge frame maps source y 20..1684 to the rail.
    const PANEL_CENTERS: [f32; 4] = [615.0, 827.0, 1041.0, 1255.0];
    // The visible cloth is centered at x=29 after the standard's transparent
    // edge is mapped to the rail; the 64-pixel hitbox extends farther right.
    const CLOTH_CENTER_X: f32 = 29.0;
    let mut governance_clicked = None;
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
        if clicked {
            governance_clicked = Some(index);
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

pub(in crate::app) const MAP_STANDARD_WIDTH: f32 = 146.0;
pub(in crate::app) const MAP_STANDARD_RAIL_IMAGE_WIDTH: f32 = 120.0;
pub(in crate::app) const MAP_STANDARD_RAIL_WIDTH: f32 = 64.0;
pub(in crate::app) const MAP_STANDARD_FLAG_HEIGHT: f32 = 245.0;
pub(in crate::app) const MAP_RESOURCE_STRIP_LEFT: f32 = MAP_STANDARD_WIDTH - 2.0;
pub(in crate::app) const MAP_RESOURCE_STRIP_HEIGHT: f32 = 48.0;
pub(in crate::app) const MAP_DATE_SECTION_WIDTH: f32 = 230.0;

pub(in crate::app) fn map_standard_height(screen: egui::Rect, scale: f32) -> f32 {
    (screen.height() / scale - 35.0).clamp(300.0, 842.0)
}

pub(in crate::app) fn map_resource_strip_right(screen: egui::Rect, scale: f32) -> f32 {
    let content_right = MAP_RESOURCE_STRIP_LEFT
        + HUD_FIRST_GROUP_WIDTH
        + HUD_SECOND_GROUP_WIDTH
        + HUD_THIRD_GROUP_WIDTH
        + MAP_DATE_SECTION_WIDTH;
    (screen.width() / scale - 170.0).clamp(310.0, content_right)
}

pub(in crate::app) fn map_resource_strip_visible(screen: egui::Rect, scale: f32) -> bool {
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

/// Follow the flag's opaque face and lower hem, excluding the folded cloth below it.
fn map_flag_contains(screen: egui::Rect, scale: f32, pointer: egui::Pos2) -> bool {
    let local = (pointer - screen.min) / scale;
    if !(0.0..MAP_STANDARD_WIDTH).contains(&local.x) || local.y < 0.0 {
        return false;
    }
    // Invert the top segment of the mesh in paint_map_edge_frame.
    let x = 7.0 + local.x / MAP_STANDARD_WIDTH * 213.0;
    let y = 20.0 + local.y / map_standard_height(screen, scale) * 1664.0;
    const HEM: [(f32, f32); 10] = [
        (7.0, 277.0),
        (20.0, 278.0),
        (40.0, 280.0),
        (70.0, 282.0),
        (100.0, 283.0),
        (140.0, 282.0),
        (160.0, 277.0),
        (180.0, 269.0),
        (190.0, 261.0),
        (220.0, 244.0),
    ];
    let edge = HEM.windows(2).find(|edge| x < edge[1].0).unwrap();
    let bottom = edge[0].1 + (edge[1].1 - edge[0].1) * (x - edge[0].0) / (edge[1].0 - edge[0].0);
    if y >= bottom {
        return false;
    }
    static FLAG: std::sync::OnceLock<image::RgbaImage> = std::sync::OnceLock::new();
    let flag = FLAG.get_or_init(|| {
        image::load_from_memory(PLAYER_STANDARD_PNG)
            .expect("player standard PNG must be valid")
            .to_rgba8()
    });
    flag.get_pixel(x as u32, y as u32)[3] > 8
}

pub(in crate::app) fn draw_map_menu_hitboxes(ctx: &egui::Context, scale: f32) -> bool {
    let screen = ctx.content_rect();
    let rail_height = map_standard_height(screen, scale) - MAP_STANDARD_FLAG_HEIGHT;
    let strip_right = map_resource_strip_right(screen, scale);
    let date_left = strip_right - MAP_DATE_SECTION_WIDTH;
    let mut banner_clicked = false;
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
                if id == "augustus_standard_flag_hitbox" {
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Main province")
                    });
                    let on_flag = ui.input(|input| {
                        input
                            .pointer
                            .interact_pos()
                            .is_some_and(|pointer| map_flag_contains(screen, scale, pointer))
                    });
                    let response = response.on_hover_cursor(if on_flag {
                        egui::CursorIcon::PointingHand
                    } else {
                        egui::CursorIcon::Default
                    });
                    let press_id = response.id.with("pressed_on_flag");
                    // egui clears press_origin on release, so retain it while held.
                    if response.is_pointer_button_down_on() {
                        let started_on_flag = ui.input(|input| {
                            input
                                .pointer
                                .press_origin()
                                .is_some_and(|pointer| map_flag_contains(screen, scale, pointer))
                        });
                        ui.ctx().data_mut(|data| data.insert_temp(press_id, started_on_flag));
                    }
                    banner_clicked = response.clicked()
                        && on_flag
                        && ui.ctx().data(|data| data.get_temp::<bool>(press_id).unwrap_or(false));
                } else {
                    response.on_hover_cursor(egui::CursorIcon::Default);
                }
            });
    }
    if map_resource_strip_visible(screen, scale) {
        egui::Area::new(egui::Id::new("augustus_resource_strip_hitbox"))
            .fixed_pos(screen.min + egui::vec2(MAP_RESOURCE_STRIP_LEFT, 0.0) * scale)
            // Block map drags underneath the resource controls without stealing their hover.
            .order(egui::Order::Middle)
            .movable(false)
            .show(ctx, |ui| {
                let (_, response) = ui.allocate_exact_size(
                    egui::vec2(date_left - MAP_RESOURCE_STRIP_LEFT, MAP_RESOURCE_STRIP_HEIGHT)
                        * scale,
                    egui::Sense::click_and_drag(),
                );
                response.on_hover_cursor(egui::CursorIcon::Default);
            });
    }
    banner_clicked
}

pub(in crate::app) fn map_date_hitbox(
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

pub(in crate::app) fn load_player_standard(
    ctx: &egui::Context,
    color_index: usize,
) -> egui::TextureHandle {
    let name = format!("player-standard-{}", PLAYER_COLOR_NAMES[color_index]);
    let mut rgba = player_standard_image(color_index);
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

/// Tint the cloth to the shared palette while retaining goldwork, folds and alpha.
pub(in crate::app) fn player_standard_image(color_index: usize) -> image::RgbaImage {
    let mut rgba = image::load_from_memory(PLAYER_STANDARD_PNG)
        .expect("player standard PNG must be valid")
        .to_rgba8();
    let color = PLAYER_COLORS[color_index];
    let cloth_red = f32::from(rgba.get_pixel(80, 700)[0]);
    for pixel in rgba.pixels_mut() {
        let [red, green, blue, alpha] = pixel.0;
        // The red cloth has a much stronger red channel than the gold ornament.
        if alpha == 0 || f32::from(red) <= f32::from(green) * 1.6 || red <= blue {
            continue;
        }
        for (channel, target) in pixel.0[..3].iter_mut().zip([color.r(), color.g(), color.b()]) {
            let target = f32::from(target);
            let shade = f32::from(red);
            *channel = if shade <= cloth_red {
                target * shade / cloth_red
            } else {
                target + (255.0 - target) * (shade - cloth_red) / (255.0 - cloth_red)
            }
            .round() as u8;
        }
    }
    rgba
}
