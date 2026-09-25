//! Rank ladder and local player list.

use super::*;

pub(super) const RANKS: [&str; 6] =
    ["Quaestor", "Aedile", "Praetor", "Censor", "Consul", "Augustus"];
pub(super) const RANK_LADDER_TOP_FRACTION: f32 = 0.13;
// Centers of the six crossbars in rank-scepter-v2.png, from top to bottom.
pub(super) const RANK_SLOT_CENTERS: [f32; 6] = [0.222, 0.330, 0.439, 0.548, 0.657, 0.766];
// Each promotion illuminates the next crossbar and the shaft below it.
pub(super) const RANK_LIGHT_START: [f32; 6] = [0.711, 0.602, 0.493, 0.384, 0.276, 0.0];

pub(super) fn draw_rank_ladder(
    ctx: &egui::Context,
    scale: f32,
    active_rank: usize,
    textures: &mut [Option<egui::TextureHandle>; 6],
    scepter: &egui::TextureHandle,
) {
    let top_offset = ctx.content_rect().height() * RANK_LADDER_TOP_FRACTION;
    egui::Area::new(egui::Id::new("augustus_rank_ladder"))
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-6.0 * scale, top_offset))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let (ladder_rect, _) =
                ui.allocate_exact_size(egui::vec2(204.0, 266.0) * scale, egui::Sense::hover());
            let painter = ui.painter();
            let staff_x = ladder_rect.right() - 34.0 * scale;
            let image_rect = egui::Rect::from_center_size(
                egui::pos2(staff_x, ladder_rect.center().y),
                egui::vec2(54.0, 266.0) * scale,
            );
            let scepter_uv = egui::Rect::from_min_max(egui::pos2(0.35, 0.0), egui::pos2(0.65, 1.0));
            painter.image(
                scepter.id(),
                image_rect,
                scepter_uv,
                egui::Color32::from_rgb(115, 103, 88),
            );
            let lit_from = RANK_LIGHT_START[active_rank];
            painter.image(
                scepter.id(),
                egui::Rect::from_min_max(
                    egui::pos2(
                        image_rect.left(),
                        image_rect.top() + image_rect.height() * lit_from,
                    ),
                    image_rect.max,
                ),
                egui::Rect::from_min_max(egui::pos2(0.35, lit_from), egui::pos2(0.65, 1.0)),
                egui::Color32::WHITE,
            );

            for (visual_index, slot_center) in
                RANK_SLOT_CENTERS.iter().enumerate().take(RANKS.len())
            {
                let rank_index = RANKS.len() - 1 - visual_index;
                let section_y = image_rect.top() + image_rect.height() * slot_center;
                let icon_center = egui::pos2(staff_x - 34.0 * scale, section_y);
                let icon_rect =
                    egui::Rect::from_center_size(icon_center, egui::vec2(24.0, 24.0) * scale);
                let response = ui.interact(
                    icon_rect.expand(2.0 * scale),
                    egui::Id::new(("augustus_rank", rank_index)),
                    egui::Sense::hover(),
                );
                let hovered = response.hovered();
                let texture = rank_texture(ctx, textures, rank_index);
                painter.image(
                    texture.id(),
                    icon_rect,
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                    if rank_index <= active_rank {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_rgb(130, 120, 105)
                    },
                );

                if hovered {
                    let text_pos = egui::pos2(staff_x - 53.0 * scale, section_y);
                    painter.text(
                        text_pos + egui::vec2(1.0, 1.0) * scale,
                        egui::Align2::RIGHT_CENTER,
                        RANKS[rank_index],
                        egui::FontId::proportional(12.0 * scale),
                        egui::Color32::from_rgba_unmultiplied(20, 12, 10, 230),
                    );
                    painter.text(
                        text_pos,
                        egui::Align2::RIGHT_CENTER,
                        RANKS[rank_index],
                        egui::FontId::proportional(12.0 * scale),
                        egui::Color32::from_rgb(213, 180, 137),
                    );
                }
            }
        });
}

pub(super) fn load_rank_scepter(ctx: &egui::Context) -> egui::TextureHandle {
    let image = image::load_from_memory(RANK_SCEPTER)
        .expect("rank scepter PNG must be valid")
        .resize_exact(512, 768, image::imageops::FilterType::Lanczos3)
        .to_rgba8();
    ctx.load_texture(
        "rank-scepter-v2",
        egui::ColorImage::from_rgba_unmultiplied(
            [image.width() as usize, image.height() as usize],
            image.as_raw(),
        ),
        egui::TextureOptions::LINEAR,
    )
}

pub(super) fn draw_practice_players(
    ctx: &egui::Context,
    scale: f32,
    practice: &mut LocalPractice,
    textures: &mut [Option<egui::TextureHandle>; 6],
    sound: &MenuAudio,
    audio: &Audio,
    assets: &AssetServer,
) {
    let screen = ctx.content_rect();
    egui::Area::new(egui::Id::new("augustus_practice_players"))
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-8.0, -8.0) * scale)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_rgba_unmultiplied(31, 20, 18, 232))
                .stroke(egui::Stroke::new(
                    1.0 * scale,
                    egui::Color32::from_rgba_unmultiplied(190, 143, 94, 115),
                ))
                .corner_radius(6.0 * scale)
                .inner_margin(egui::Margin::symmetric((10.0 * scale) as i8, (8.0 * scale) as i8))
                .show(ui, |ui| {
                    ui.set_width((screen.width() / scale * 0.2).clamp(210.0, 270.0) * 0.6 * scale);
                    ui.label(
                        egui::RichText::new("PLAYERS").size(12.0 * scale).strong().color(GOLD),
                    );
                    ui.add_space(4.0 * scale);
                    for index in 0..practice.players.len() {
                        let selected = practice.active_player == index;
                        let player = &practice.players[index];
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 34.0 * scale),
                            egui::Sense::click(),
                        );
                        let hovered = response.hovered();
                        let pressed = response.is_pointer_button_down_on();
                        if selected || hovered || pressed {
                            ui.painter().rect_filled(
                                rect,
                                3.0 * scale,
                                if pressed {
                                    egui::Color32::from_rgba_unmultiplied(173, 113, 55, 150)
                                } else if selected {
                                    egui::Color32::from_rgba_unmultiplied(145, 91, 45, 105)
                                } else {
                                    egui::Color32::from_rgba_unmultiplied(117, 77, 47, 90)
                                },
                            );
                        }
                        if selected || hovered || pressed {
                            ui.painter().rect_stroke(
                                rect,
                                3.0 * scale,
                                egui::Stroke::new(
                                    (if hovered || pressed {
                                        1.25
                                    } else {
                                        1.0
                                    }) * scale,
                                    if selected || pressed {
                                        GOLD
                                    } else {
                                        egui::Color32::from_rgba_unmultiplied(220, 170, 105, 175)
                                    },
                                ),
                                egui::StrokeKind::Inside,
                            );
                        }
                        let color = PLAYER_COLORS[player.color_index.min(PLAYER_COLORS.len() - 1)];
                        ui.painter().circle_filled(
                            rect.min + egui::vec2(13.0, rect.height() * 0.5),
                            6.0 * scale,
                            color,
                        );
                        let texture = rank_texture(ctx, textures, player.rank.min(RANKS.len() - 1));
                        ui.painter().image(
                            texture.id(),
                            egui::Rect::from_min_size(
                                rect.min + egui::vec2(27.0, 4.0) * scale,
                                egui::vec2(25.0, 25.0) * scale,
                            ),
                            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                        ui.painter().text(
                            rect.min + egui::vec2(57.0, rect.height() * 0.5),
                            egui::Align2::LEFT_CENTER,
                            format!("Player {}", index + 1),
                            egui::FontId::proportional(12.0 * scale),
                            if selected || hovered {
                                CREAM
                            } else {
                                MUTED_TEXT
                            },
                        );
                        if response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            practice.active_player = index;
                            play_click(sound, audio, assets);
                        }
                    }
                });
        });
}

pub(super) fn rank_texture(
    ctx: &egui::Context,
    textures: &mut [Option<egui::TextureHandle>; 6],
    index: usize,
) -> egui::TextureHandle {
    textures[index]
        .get_or_insert_with(|| {
            let bytes = RANK_ICONS[index];
            let image = image::load_from_memory(bytes)
                .expect("rank icon PNG must be valid")
                .resize_exact(128, 128, image::imageops::FilterType::Lanczos3)
                .to_rgba8();
            ctx.load_texture(
                RANK_NAMES_ASSET[index],
                egui::ColorImage::from_rgba_unmultiplied(
                    [image.width() as usize, image.height() as usize],
                    image.as_raw(),
                ),
                egui::TextureOptions::LINEAR,
            )
        })
        .clone()
}
