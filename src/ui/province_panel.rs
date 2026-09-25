//! Province facts card, occupying the same map-side position as governance.

use bevy_egui::egui;

use super::{format_hud_number, HUD_RESOURCE_ICONS, POP_CLASS_ICONS};
use crate::map::{paint_marker_icon, MarkerIcon, ProvinceOverview};

pub(super) const WIDTH: f32 = 500.0;
pub(super) const HEIGHT: f32 = 468.0;
pub(super) const INK: egui::Color32 = egui::Color32::from_rgb(57, 43, 37);
const DISABLED_INK: egui::Color32 = egui::Color32::from_rgb(151, 142, 129);
const MUTED: egui::Color32 = egui::Color32::from_rgb(112, 91, 71);
pub(super) const RULE: egui::Color32 = egui::Color32::from_rgb(191, 171, 143);
pub(super) const PAPER: egui::Color32 = egui::Color32::from_rgb(238, 233, 219);
pub(super) const TABLE_STRIPE: egui::Color32 = egui::Color32::from_rgb(244, 239, 225);
pub(super) const NEUTRAL: egui::Color32 = egui::Color32::from_rgb(205, 208, 207);

const TERRAIN_IMAGES: [(&str, &[u8]); 12] = [
    ("desert", include_bytes!("../../assets/images/map/terrain/desert.png")),
    ("farmland", include_bytes!("../../assets/images/map/terrain/farmland.png")),
    ("forest", include_bytes!("../../assets/images/map/terrain/forest.png")),
    ("hills", include_bytes!("../../assets/images/map/terrain/hills.png")),
    ("impassable", include_bytes!("../../assets/images/map/terrain/impassable.png")),
    ("jungle", include_bytes!("../../assets/images/map/terrain/jungle.png")),
    ("marsh", include_bytes!("../../assets/images/map/terrain/marsh.png")),
    ("mountain", include_bytes!("../../assets/images/map/terrain/mountain.png")),
    ("plains", include_bytes!("../../assets/images/map/terrain/plains.png")),
    ("coastal", include_bytes!("../../assets/images/map/terrain/coastal.png")),
    ("ocean", include_bytes!("../../assets/images/map/terrain/ocean.png")),
    ("river", include_bytes!("../../assets/images/map/terrain/river.png")),
];

const DIPLOMACY_ICONS: [(&str, &[u8]); 4] = [
    ("spy", include_bytes!("../../assets/images/icons/spy.png")),
    ("trade", include_bytes!("../../assets/images/icons/trade.png")),
    ("court-nobles", include_bytes!("../../assets/images/icons/court-nobles.png")),
    ("attack", include_bytes!("../../assets/images/icons/attack.png")),
];

pub(super) fn load_terrain_images(context: &egui::Context) -> [egui::TextureHandle; 12] {
    std::array::from_fn(|index| load_image(context, TERRAIN_IMAGES[index], "province-terrain"))
}

pub(super) fn load_population_icons(context: &egui::Context) -> [egui::TextureHandle; 4] {
    std::array::from_fn(|index| {
        load_panel_icon(
            context,
            ("population", POP_CLASS_ICONS[index]),
            &format!("province-pop-{index}"),
        )
    })
}

pub(super) fn load_resource_icons(context: &egui::Context) -> [egui::TextureHandle; 3] {
    std::array::from_fn(|index| {
        load_panel_icon(
            context,
            ("resource", HUD_RESOURCE_ICONS[index]),
            &format!("province-resource-{index}"),
        )
    })
}

pub(super) fn load_diplomacy_icons(context: &egui::Context) -> [egui::TextureHandle; 4] {
    std::array::from_fn(|index| {
        load_panel_icon(context, DIPLOMACY_ICONS[index], &format!("province-action-{index}"))
    })
}

pub(super) fn load_panel_icon(
    context: &egui::Context,
    asset: (&str, &[u8]),
    prefix: &str,
) -> egui::TextureHandle {
    let mut rgba =
        image::load_from_memory(asset.1).expect("province icon PNG must be valid").to_rgba8();
    // Egui uses bilinear sampling without mipmaps. Upload a filtered icon close
    // to its on-screen size instead of minifying the 1254px source in one pass.
    for pixel in rgba.pixels_mut() {
        let alpha = u16::from(pixel[3]);
        for channel in &mut pixel.0[..3] {
            *channel = ((u16::from(*channel) * alpha + 127) / 255) as u8;
        }
    }
    let mut small = image::imageops::resize(&rgba, 64, 64, image::imageops::FilterType::Lanczos3);
    for pixel in small.pixels_mut() {
        let alpha = u16::from(pixel[3]);
        for channel in &mut pixel.0[..3] {
            *channel = (u16::from(*channel) * 255 + alpha / 2)
                .checked_div(alpha)
                .unwrap_or_default()
                .min(255) as u8;
        }
    }
    context.load_texture(
        format!("{prefix}-{}", asset.0),
        egui::ColorImage::from_rgba_unmultiplied([64, 64], small.as_raw()),
        egui::TextureOptions::LINEAR,
    )
}

pub(super) fn load_image(
    context: &egui::Context,
    asset: (&str, &[u8]),
    prefix: &str,
) -> egui::TextureHandle {
    let rgba =
        image::load_from_memory(asset.1).expect("province panel PNG must be valid").to_rgba8();
    context.load_texture(
        format!("{prefix}-{}", asset.0),
        egui::ColorImage::from_rgba_unmultiplied(
            [rgba.width() as usize, rgba.height() as usize],
            rgba.as_raw(),
        ),
        egui::TextureOptions::LINEAR,
    )
}

pub(super) fn load_spqr_emblem(context: &egui::Context) -> egui::TextureHandle {
    load_panel_icon(
        context,
        ("spqr-eagle-gold", include_bytes!("../../assets/images/ui/spqr-eagle-gold.png")),
        "province-owner",
    )
}

pub(super) fn show(
    context: &egui::Context,
    scale: f32,
    province: &ProvinceOverview,
    terrain_images: &[egui::TextureHandle; 12],
    population_icons: &[egui::TextureHandle; 4],
    resource_icons: &[egui::TextureHandle; 3],
    diplomacy_icons: &[egui::TextureHandle; 4],
    active_player: usize,
    can_trade: bool,
    emblem: Option<&egui::TextureHandle>,
    closing: bool,
) -> (bool, bool) {
    let screen = context.content_rect();
    let left = screen.left() + 60.0 * scale;
    let width = (WIDTH * scale).min((screen.right() - left - 12.0 * scale).max(1.0));
    let available_height = (screen.height() - 78.0 * scale).max(1.0);
    let height = (HEIGHT * scale).min(available_height);
    let draw_scale = scale.min(width / WIDTH).min(height / HEIGHT);
    let rect = super::map_corner_panel_rect(screen, scale, egui::vec2(width, height));
    let header = province.owner_color.unwrap_or(NEUTRAL);
    let bright = f32::from(header.r()) * 0.299
        + f32::from(header.g()) * 0.587
        + f32::from(header.b()) * 0.114
        > 150.0;
    let header_ink = if bright {
        INK
    } else {
        egui::Color32::from_rgb(255, 238, 210)
    };
    let mut closed = false;
    let mut city_clicked = false;
    egui::Area::new(egui::Id::new("augustus_province_panel"))
        .fixed_pos(rect.min)
        .order(egui::Order::Foreground)
        .show(context, |ui| {
            let (panel, _) = ui.allocate_exact_size(rect.size(), egui::Sense::hover());
            let painter = ui.painter_at(panel);
            let at = |x: f32, y: f32| panel.min + egui::vec2(x, y) * draw_scale;
            let box_at = |x: f32, y: f32, w: f32, h: f32| {
                egui::Rect::from_min_size(at(x, y), egui::vec2(w, h) * draw_scale)
            };
            painter.rect_filled(
                panel.translate(egui::vec2(4.0, 5.0) * draw_scale),
                6.0 * draw_scale,
                egui::Color32::from_black_alpha(80),
            );
            painter.rect_filled(panel, 6.0 * draw_scale, PAPER);
            painter.rect_stroke(
                panel,
                6.0 * draw_scale,
                egui::Stroke::new(1.2 * draw_scale, header),
                egui::StrokeKind::Inside,
            );
            painter.rect_filled(box_at(0.0, 0.0, WIDTH, 51.0), 5.0 * draw_scale, header);
            painter.rect_filled(box_at(0.0, 44.0, WIDTH, 7.0), 0.0, header);
            painter.line_segment(
                [at(12.0, 46.0), at(488.0, 46.0)],
                egui::Stroke::new(draw_scale, egui::Color32::from_rgb(194, 148, 103)),
            );
            if let Some(emblem) = emblem {
                painter.image(
                    emblem.id(),
                    box_at(13.0, 5.5, 40.0, 40.0),
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            painter.text(
                at(
                    if emblem.is_some() {
                        61.0
                    } else {
                        18.0
                    },
                    25.0,
                ),
                egui::Align2::LEFT_CENTER,
                province.name,
                egui::FontId::proportional(25.0 * draw_scale),
                header_ink,
            );
            let close = box_at(459.0, 9.0, 32.0, 32.0);
            let response = ui
                .interact(close, ui.id().with("close"), egui::Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if response.clicked() {
                closed = true;
            }
            let close_pressed =
                closing || response.is_pointer_button_down_on() || response.clicked();
            let close_ink = if close_pressed {
                egui::Color32::from_rgb(255, 239, 209)
            } else {
                header_ink
            };
            painter.circle_filled(
                close.center(),
                14.0 * draw_scale,
                if close_pressed {
                    egui::Color32::from_rgb(112, 48, 43)
                } else if response.hovered() {
                    egui::Color32::from_white_alpha(55)
                } else {
                    egui::Color32::TRANSPARENT
                },
            );
            painter.circle_stroke(
                close.center(),
                13.0 * draw_scale,
                egui::Stroke::new(
                    (if close_pressed {
                        2.2
                    } else {
                        1.4
                    }) * draw_scale,
                    close_ink,
                ),
            );
            let cross_center = close.center();
            for direction in [-1.0, 1.0] {
                painter.line_segment(
                    [
                        cross_center + egui::vec2(-5.0, -5.0 * direction) * draw_scale,
                        cross_center + egui::vec2(5.0, 5.0 * direction) * draw_scale,
                    ],
                    egui::Stroke::new(2.2 * draw_scale, close_ink),
                );
            }
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Close province overview")
            });

            let landscape = box_at(12.0, 61.0, 476.0, 95.0);
            painter.image(
                terrain_images[province.terrain.image_index()].id(),
                landscape,
                // The original land portraits are 735×92; this center crop
                // fills the banner at close to native resolution.
                egui::Rect::from_min_max(egui::pos2(0.187, 0.0), egui::pos2(0.813, 1.0)),
                egui::Color32::WHITE,
            );
            painter.rect_stroke(
                landscape,
                2.0 * draw_scale,
                egui::Stroke::new(draw_scale, RULE),
                egui::StrokeKind::Inside,
            );
            if let Some(city_name) = province.city_name {
                let width = navigation_badge_width(&painter, city_name, draw_scale);
                city_clicked = navigation_badge(
                    ui,
                    &painter,
                    box_at(21.0, 120.0, width, 27.0),
                    city_name,
                    NavigationIcon::City,
                    draw_scale,
                    "city",
                );
            }

            section(&painter, at(17.0, 184.0), "POPULATION", draw_scale);
            painter.text(
                at(241.0, 184.0),
                egui::Align2::RIGHT_CENTER,
                format!("{} total", format_hud_number(province.population.iter().sum())),
                egui::FontId::proportional(13.0 * draw_scale),
                MUTED,
            );
            const CLASSES: [&str; 4] = ["Nobles", "Citizens", "Plebeians", "Slaves"];
            for index in 0..4 {
                let row = index as f32;
                if index % 2 == 0 {
                    painter.rect_filled(
                        box_at(17.0, 203.0 + row * 31.0, 226.0, 31.0),
                        0.0,
                        TABLE_STRIPE,
                    );
                }
                painter.image(
                    population_icons[index].id(),
                    box_at(21.0, 205.0 + row * 31.0, 25.0, 25.0),
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                painter.text(
                    at(53.0, 217.5 + row * 31.0),
                    egui::Align2::LEFT_CENTER,
                    CLASSES[index],
                    egui::FontId::proportional(14.0 * draw_scale),
                    INK,
                );
                painter.text(
                    at(235.0, 217.5 + row * 31.0),
                    egui::Align2::RIGHT_CENTER,
                    format_hud_number(province.population[index]),
                    egui::FontId::proportional(15.0 * draw_scale),
                    INK,
                );
            }

            section(&painter, at(253.0, 184.0), "RESOURCES", draw_scale);
            painter.text(
                at(409.0, 202.0),
                egui::Align2::RIGHT_CENTER,
                "Base",
                egui::FontId::proportional(11.0 * draw_scale),
                MUTED,
            );
            painter.text(
                at(477.0, 202.0),
                egui::Align2::RIGHT_CENTER,
                "Prod.",
                egui::FontId::proportional(11.0 * draw_scale),
                MUTED,
            );
            const RESOURCES: [&str; 3] = ["Food", "Metal", "Stone"];
            for index in 0..3 {
                let row = index as f32;
                if index % 2 == 0 {
                    painter.rect_filled(
                        box_at(253.0, 210.0 + row * 38.0, 226.0, 38.0),
                        0.0,
                        TABLE_STRIPE,
                    );
                }
                painter.image(
                    resource_icons[index].id(),
                    box_at(257.0, 214.0 + row * 38.0, 30.0, 30.0),
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                painter.text(
                    at(291.0, 229.0 + row * 38.0),
                    egui::Align2::LEFT_CENTER,
                    RESOURCES[index],
                    egui::FontId::proportional(14.0 * draw_scale),
                    INK,
                );
                painter.text(
                    at(408.0, 229.0 + row * 38.0),
                    egui::Align2::RIGHT_CENTER,
                    province.base_resources[index].to_string(),
                    egui::FontId::proportional(14.0 * draw_scale),
                    INK,
                );
                painter.text(
                    at(472.0, 229.0 + row * 38.0),
                    egui::Align2::RIGHT_CENTER,
                    format_hud_number(province.production[index]),
                    egui::FontId::proportional(14.0 * draw_scale),
                    INK,
                );
            }

            painter.line_segment(
                [at(17.0, 335.0), at(479.0, 335.0)],
                egui::Stroke::new(draw_scale, RULE),
            );
            section(&painter, at(17.0, 350.0), "DIPLOMACY", draw_scale);
            let foreign = province.owner != Some(active_player);
            let actions = [
                (0, "Spy", foreign),
                (1, "Trade", foreign && can_trade),
                (2, "Court Nobles", foreign),
                (3, "Attack", foreign),
            ];
            for (slot, (icon, title, enabled)) in actions.into_iter().enumerate() {
                let column = (slot % 2) as f32;
                let row = (slot / 2) as f32;
                let button = box_at(17.0 + column * 236.0, 367.0 + row * 46.0, 226.0, 39.0);
                let response = ui.interact(
                    button,
                    ui.id().with(("diplomacy", icon)),
                    if enabled {
                        egui::Sense::click()
                    } else {
                        egui::Sense::hover()
                    },
                );
                let response = if enabled {
                    response.on_hover_cursor(egui::CursorIcon::PointingHand)
                } else {
                    response
                };
                let fill = if !enabled {
                    egui::Color32::from_rgb(229, 225, 216)
                } else if response.is_pointer_button_down_on() {
                    egui::Color32::from_rgb(220, 204, 177)
                } else if response.hovered() {
                    egui::Color32::from_rgb(255, 250, 239)
                } else {
                    egui::Color32::from_rgb(248, 244, 234)
                };
                painter.rect_filled(button, 4.0 * draw_scale, fill);
                painter.rect_stroke(
                    button,
                    4.0 * draw_scale,
                    egui::Stroke::new(
                        draw_scale,
                        if enabled {
                            RULE
                        } else {
                            egui::Color32::from_rgba_unmultiplied(191, 171, 143, 100)
                        },
                    ),
                    egui::StrokeKind::Inside,
                );
                painter.image(
                    diplomacy_icons[icon].id(),
                    box_at(23.0 + column * 236.0, 372.0 + row * 46.0, 29.0, 29.0),
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                    if enabled {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_white_alpha(105)
                    },
                );
                painter.text(
                    at(61.0 + column * 236.0, 386.5 + row * 46.0),
                    egui::Align2::LEFT_CENTER,
                    title,
                    egui::FontId::proportional(16.0 * draw_scale),
                    if enabled {
                        INK
                    } else {
                        DISABLED_INK
                    },
                );
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, title)
                });
            }
        });
    (closed, city_clicked)
}

fn section(painter: &egui::Painter, position: egui::Pos2, title: &str, scale: f32) {
    painter.text(
        position,
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(16.0 * scale),
        INK,
    );
}

pub(super) enum NavigationIcon<'a> {
    City,
    Province(&'a egui::TextureHandle),
}

pub(super) fn navigation_badge_width(painter: &egui::Painter, label: &str, scale: f32) -> f32 {
    let text_width = painter
        .layout_no_wrap(
            label.to_owned(),
            egui::FontId::proportional(15.0 * scale),
            egui::Color32::WHITE,
        )
        .size()
        .x
        / scale;
    (text_width + 35.0).clamp(105.0, 225.0)
}

pub(super) fn navigation_badge(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    rect: egui::Rect,
    label: &str,
    icon: NavigationIcon<'_>,
    scale: f32,
    id: &str,
) -> bool {
    let response = ui
        .interact(rect, ui.id().with(("navigation_badge", id)), egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let pressed = response.is_pointer_button_down_on();
    painter.rect_filled(
        rect,
        3.0 * scale,
        if pressed {
            egui::Color32::from_rgb(80, 47, 37)
        } else if response.hovered() {
            egui::Color32::from_rgb(115, 76, 55)
        } else {
            egui::Color32::from_black_alpha(180)
        },
    );
    if response.hovered() {
        painter.rect_stroke(
            rect,
            3.0 * scale,
            egui::Stroke::new(scale, egui::Color32::from_rgb(240, 206, 149)),
            egui::StrokeKind::Inside,
        );
    }
    let font = egui::FontId::proportional(15.0 * scale);
    let text_width =
        painter.layout_no_wrap(label.to_owned(), font.clone(), egui::Color32::WHITE).size().x;
    let icon_size = 19.0 * scale;
    let gap = 4.0 * scale;
    let icon_left = rect.center().x - (icon_size + gap + text_width) * 0.5;
    let icon_rect = egui::Rect::from_min_size(
        egui::pos2(icon_left, rect.center().y - icon_size * 0.5),
        egui::vec2(icon_size, icon_size),
    );
    match icon {
        NavigationIcon::City => {
            paint_marker_icon(painter, icon_rect, MarkerIcon::City, egui::Color32::WHITE)
        },
        NavigationIcon::Province(texture) => {
            painter.image(
                texture.id(),
                icon_rect,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        },
    }
    painter.text(
        egui::pos2(icon_left + icon_size + gap, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        font,
        egui::Color32::WHITE,
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    response.clicked()
}

#[cfg(test)]
mod tests {
    #[test]
    fn land_portraits_use_original_resolution() {
        for (name, bytes) in super::TERRAIN_IMAGES.iter().take(9) {
            let image = image::load_from_memory(bytes).expect("terrain portrait must be valid");
            assert_eq!(image.width(), 735, "{name}");
            assert_eq!(image.height(), 92, "{name}");
        }
    }
}
