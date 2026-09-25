//! City details and the first set of building illustrations.

use bevy_egui::egui;

use super::province_panel::{
    self, navigation_badge, navigation_badge_width, NavigationIcon, HEIGHT, INK, NEUTRAL, PAPER,
    RULE, TABLE_STRIPE, WIDTH,
};
use crate::map::ProvinceOverview;

const CITY_BANNER: (&str, &[u8]) =
    ("roman-city", include_bytes!("../../assets/images/cities/city-panel-banner.png"));
const PROVINCE_ICON: (&str, &[u8]) =
    ("province", include_bytes!("../../assets/images/icons/province.png"));
const BUILDINGS: [(&str, &str, &[u8]); 8] = [
    ("Aqueduct", "aqueduct", include_bytes!("../../assets/images/buildings/aqueduct.png")),
    ("Granary", "granary", include_bytes!("../../assets/images/buildings/granary.png")),
    ("Forum", "forum", include_bytes!("../../assets/images/buildings/forum.png")),
    ("Marketplace", "marketplace", include_bytes!("../../assets/images/buildings/marketplace.png")),
    ("Foundry", "foundry", include_bytes!("../../assets/images/buildings/foundry.png")),
    ("Academy", "academy", include_bytes!("../../assets/images/buildings/academy.png")),
    (
        "Great Temple",
        "great-temple",
        include_bytes!("../../assets/images/buildings/great-temple.png"),
    ),
    (
        "Grand Theater",
        "grand-theater",
        include_bytes!("../../assets/images/buildings/grand-theater.png"),
    ),
];

pub(super) fn load_banner(context: &egui::Context) -> egui::TextureHandle {
    province_panel::load_image(context, CITY_BANNER, "city-banner")
}

pub(super) fn load_province_icon(context: &egui::Context) -> egui::TextureHandle {
    province_panel::load_panel_icon(context, PROVINCE_ICON, "city-navigation")
}

pub(super) fn load_buildings(context: &egui::Context) -> [egui::TextureHandle; 8] {
    std::array::from_fn(|index| {
        province_panel::load_panel_icon(
            context,
            (BUILDINGS[index].1, BUILDINGS[index].2),
            "city-building",
        )
    })
}

pub(super) fn show(
    context: &egui::Context,
    scale: f32,
    province: &ProvinceOverview,
    banner: &egui::TextureHandle,
    buildings: &[egui::TextureHandle; 8],
    province_icon: &egui::TextureHandle,
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
    let mut province_clicked = false;
    egui::Area::new(egui::Id::new("augustus_city_panel"))
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
                province.city_name.unwrap_or(province.name),
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
            let pressed = closing || response.is_pointer_button_down_on() || response.clicked();
            let close_ink = if pressed {
                egui::Color32::from_rgb(255, 239, 209)
            } else {
                header_ink
            };
            painter.circle_filled(
                close.center(),
                14.0 * draw_scale,
                if pressed {
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
                    (if pressed {
                        2.2
                    } else {
                        1.4
                    }) * draw_scale,
                    close_ink,
                ),
            );
            for direction in [-1.0, 1.0] {
                painter.line_segment(
                    [
                        close.center() + egui::vec2(-5.0, -5.0 * direction) * draw_scale,
                        close.center() + egui::vec2(5.0, 5.0 * direction) * draw_scale,
                    ],
                    egui::Stroke::new(2.2 * draw_scale, close_ink),
                );
            }
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Close city overview")
            });

            let landscape = box_at(12.0, 61.0, 476.0, 95.0);
            painter.image(
                banner.id(),
                landscape,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.22), egui::pos2(1.0, 0.78)),
                egui::Color32::WHITE,
            );
            painter.rect_stroke(
                landscape,
                2.0 * draw_scale,
                egui::Stroke::new(draw_scale, RULE),
                egui::StrokeKind::Inside,
            );
            let badge_width = navigation_badge_width(&painter, province.name, draw_scale);
            province_clicked = navigation_badge(
                ui,
                &painter,
                box_at(478.0 - badge_width, 120.0, badge_width, 27.0),
                province.name,
                NavigationIcon::Province(province_icon),
                draw_scale,
                "province",
            );

            painter.text(
                at(17.0, 184.0),
                egui::Align2::LEFT_CENTER,
                "BUILDINGS",
                egui::FontId::proportional(16.0 * draw_scale),
                INK,
            );
            for (index, (name, _, _)) in BUILDINGS.iter().enumerate() {
                let column = (index % 2) as f32;
                let row = (index / 2) as f32;
                let x = 17.0 + column * 236.0;
                let y = 207.0 + row * 59.0;
                if (index / 2) % 2 == 0 {
                    painter.rect_filled(box_at(x, y, 226.0, 57.0), 0.0, TABLE_STRIPE);
                }
                painter.image(
                    buildings[index].id(),
                    box_at(x + 4.0, y + 4.0, 49.0, 49.0),
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                painter.text(
                    at(x + 61.0, y + 28.5),
                    egui::Align2::LEFT_CENTER,
                    *name,
                    egui::FontId::proportional(15.0 * draw_scale),
                    INK,
                );
            }
        });
    (closed, province_clicked)
}
