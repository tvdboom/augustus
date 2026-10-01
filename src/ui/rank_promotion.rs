//! Borderless rank emblems assemble, catch the light, then dissolve above the map.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use super::{ActiveGame, AppState, LocalPractice, TerminalPresentation};
use crate::game::{military::MilitaryRank, politics::PoliticalRank};

const DURATION: f32 = 3.8;
const ASSEMBLED_AT: f32 = 1.05;
const FADE_AT: f32 = 3.05;
const GOLD: egui::Color32 = egui::Color32::from_rgb(255, 209, 116);
// Adjacent triangular sections cover the artwork exactly, with no gaps once joined.
const PERIMETER: [[f32; 2]; 8] = [
    [0.0, 0.0],
    [0.4, 0.0],
    [1.0, 0.0],
    [1.0, 0.36],
    [1.0, 1.0],
    [0.6, 1.0],
    [0.0, 1.0],
    [0.0, 0.64],
];

#[derive(Clone, Copy, Debug)]
pub(super) enum Rank {
    Military(MilitaryRank),
    Political(PoliticalRank),
}

impl Rank {
    fn label(self) -> &'static str {
        match self {
            Self::Military(rank) => rank.name(),
            Self::Political(rank) => rank.label(),
        }
    }

    fn category(self) -> &'static str {
        match self {
            Self::Military(_) => "MILITARY RANK",
            Self::Political(_) => "POLITICAL RANK",
        }
    }

    fn key(self) -> usize {
        match self {
            Self::Military(rank) => rank as usize,
            Self::Political(rank) => 4 + rank.ladder_index(),
        }
    }

    fn artwork(self) -> &'static [u8] {
        match self {
            Self::Military(MilitaryRank::Centurion) => {
                include_bytes!("../../assets/images/icons/rank-centurion.png")
            },
            Self::Military(MilitaryRank::MilitaryTribune) => {
                include_bytes!("../../assets/images/icons/rank-military-tribune.png")
            },
            Self::Military(MilitaryRank::Legate) => {
                include_bytes!("../../assets/images/icons/rank-legate.png")
            },
            Self::Military(MilitaryRank::Imperator) => {
                include_bytes!("../../assets/images/icons/rank-imperator.png")
            },
            Self::Political(rank) => super::RANK_ICONS[rank.ladder_index()],
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Promotion {
    rank: Rank,
    player: usize,
    started_at: f64,
}

impl Promotion {
    pub(super) fn new(rank: Rank, player: usize, started_at: f64) -> Self {
        Self {
            rank,
            player,
            started_at,
        }
    }
}

#[derive(Clone)]
struct EmblemTextures {
    color: egui::TextureHandle,
    silhouette: egui::TextureHandle,
}

fn textures(ctx: &egui::Context, rank: Rank) -> EmblemTextures {
    let key = egui::Id::new(("rank-promotion-art", rank.key()));
    if let Some(art) = ctx.data(|data| data.get_temp::<EmblemTextures>(key)) {
        return art;
    }
    let mut rgba = image::load_from_memory(rank.artwork()).expect("valid rank artwork").to_rgba8();
    let (mut left, mut top, mut right, mut bottom) = (rgba.width(), rgba.height(), 0, 0);
    for (x, y, pixel) in rgba.enumerate_pixels() {
        if pixel[3] > 8 {
            left = left.min(x);
            top = top.min(y);
            right = right.max(x + 1);
            bottom = bottom.max(y + 1);
        }
    }
    if right > left && bottom > top {
        let crop =
            image::imageops::crop_imm(&rgba, left, top, right - left, bottom - top).to_image();
        let side = crop.width().max(crop.height()) + 12;
        rgba = image::RgbaImage::new(side, side);
        image::imageops::overlay(
            &mut rgba,
            &crop,
            i64::from((side - crop.width()) / 2),
            i64::from((side - crop.height()) / 2),
        );
    }
    // Resize in premultiplied space to keep transparent gold edges clean.
    for pixel in rgba.pixels_mut() {
        for channel in 0..3 {
            pixel[channel] = (u16::from(pixel[channel]) * u16::from(pixel[3]) / 255) as u8;
        }
    }
    let mut rgba = image::imageops::resize(&rgba, 384, 384, image::imageops::FilterType::Lanczos3);
    for pixel in rgba.pixels_mut() {
        for channel in 0..3 {
            pixel[channel] = (u16::from(pixel[channel]) * 255)
                .checked_div(u16::from(pixel[3]))
                .unwrap_or_default()
                .min(255) as u8;
        }
    }
    let color = ctx.load_texture(
        format!("rank-promotion-color-{}", rank.key()),
        egui::ColorImage::from_rgba_unmultiplied([384, 384], rgba.as_raw()),
        egui::TextureOptions::LINEAR,
    );
    for pixel in rgba.pixels_mut() {
        pixel.0[..3].fill(255);
    }
    let silhouette = ctx.load_texture(
        format!("rank-promotion-light-{}", rank.key()),
        egui::ColorImage::from_rgba_unmultiplied([384, 384], rgba.as_raw()),
        egui::TextureOptions::LINEAR,
    );
    let art = EmblemTextures {
        color,
        silhouette,
    };
    ctx.data_mut(|data| data.insert_temp(key, art.clone()));
    art
}

fn smooth(value: f32) -> f32 {
    let t = value.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn rotate(vector: egui::Vec2, angle: f32) -> egui::Vec2 {
    let (sin, cos) = angle.sin_cos();
    egui::vec2(vector.x * cos - vector.y * sin, vector.x * sin + vector.y * cos)
}

/// A soft radial mesh, with transparent outer vertices instead of a containing shape.
fn glow(painter: &egui::Painter, center: egui::Pos2, radius: f32, color: egui::Color32) {
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(center, color);
    for i in 0..=48 {
        let angle = i as f32 / 48.0 * std::f32::consts::TAU;
        mesh.colored_vertex(
            center + egui::vec2(angle.cos(), angle.sin()) * radius,
            egui::Color32::TRANSPARENT,
        );
        if i > 0 {
            mesh.add_triangle(0, i, i + 1);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

fn sparkle(painter: &egui::Painter, center: egui::Pos2, radius: f32, alpha: f32) {
    glow(painter, center, radius * 2.0, GOLD.gamma_multiply(alpha * 0.12));
    let color = egui::Color32::from_rgb(255, 247, 215).gamma_multiply(alpha);
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(center, color);
    for i in 0..8 {
        let angle = i as f32 * std::f32::consts::FRAC_PI_4;
        let length = if i % 2 == 0 {
            radius
        } else {
            radius * 0.16
        };
        mesh.colored_vertex(center + egui::vec2(angle.cos(), angle.sin()) * length, color);
        mesh.add_triangle(0, i + 1, (i + 1) % 8 + 1);
    }
    painter.add(egui::Shape::mesh(mesh));
}

/// Clip the shine to both the image bounds and the emblem's alpha silhouette.
fn shine(
    painter: &egui::Painter,
    texture: egui::TextureId,
    rect: egui::Rect,
    time: f32,
    fade: f32,
) {
    let progress = ((time - ASSEMBLED_AT - 0.12) / 0.85).clamp(0.0, 1.0);
    if progress <= 0.0 || progress >= 1.0 {
        return;
    }
    let sweep = -0.25 + progress * 1.8;
    let mut mesh = egui::Mesh::with_texture(texture);
    for band in 0..16 {
        let offset = (band as f32 / 16.0 - 0.5) * 0.24;
        let next = ((band + 1) as f32 / 16.0 - 0.5) * 0.24;
        let mut polygon = vec![
            egui::pos2(0.0, 0.0),
            egui::pos2(1.0, 0.0),
            egui::pos2(1.0, 1.0),
            egui::pos2(0.0, 1.0),
        ];
        for (limit, sign) in [(sweep + offset, 1.0), (sweep + next, -1.0)] {
            let mut clipped = Vec::new();
            for i in 0..polygon.len() {
                let a = polygon[i];
                let b = polygon[(i + 1) % polygon.len()];
                let da = (a.x + 0.35 * a.y - limit) * sign;
                let db = (b.x + 0.35 * b.y - limit) * sign;
                if da >= 0.0 {
                    clipped.push(a);
                }
                if (da >= 0.0) != (db >= 0.0) {
                    clipped.push(a + (b - a) * (da / (da - db)));
                }
            }
            polygon = clipped;
        }
        let base = mesh.vertices.len() as u32;
        for uv in &polygon {
            let distance = (uv.x + 0.35 * uv.y - sweep) / 0.12;
            let strength = (1.0 - distance.abs()).max(0.0).powi(2) * fade * 0.85;
            // Additive white-gold light brightens the art without whitening a rectangular tile.
            mesh.vertices.push(egui::epaint::Vertex {
                pos: rect.min + uv.to_vec2() * rect.size(),
                uv: *uv,
                color: egui::Color32::from_rgba_premultiplied(
                    (255.0 * strength) as u8,
                    (241.0 * strength) as u8,
                    (204.0 * strength) as u8,
                    0,
                ),
            });
        }
        for i in 1..polygon.len().saturating_sub(1) {
            mesh.add_triangle(base, base + i as u32, base + i as u32 + 1);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

fn paint(ctx: &egui::Context, rank: Rank, time: f32, scale: f32) {
    if !(0.0..DURATION).contains(&time) {
        return;
    }
    let painter = ctx
        .layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("rank-promotion")));
    let center = ctx.content_rect().center();
    let fade = 1.0 - smooth((time - FADE_AT) / (DURATION - FADE_AT));
    let impact = (1.0 - ((time - ASSEMBLED_AT) / 0.32).abs()).max(0.0);
    let size = (224.0 + impact * 9.0 - smooth((time - FADE_AT) / 0.75) * 8.0) * scale;
    let rect = egui::Rect::from_center_size(center, egui::Vec2::splat(size));
    let art = textures(ctx, rank);
    glow(
        &painter,
        center,
        (158.0 + impact * 32.0) * scale,
        GOLD.gamma_multiply((0.11 + impact * 0.11) * smooth(time / 0.65) * fade),
    );

    // Staggered sections arrive from different directions and settle into one exact image.
    for i in 0..PERIMETER.len() {
        let uvs = [
            egui::pos2(0.5, 0.52),
            egui::pos2(PERIMETER[i][0], PERIMETER[i][1]),
            egui::pos2(PERIMETER[(i + 1) % 8][0], PERIMETER[(i + 1) % 8][1]),
        ];
        let pivot = (uvs[0].to_vec2() + uvs[1].to_vec2() + uvs[2].to_vec2()) / 3.0;
        let progress = smooth((time - (i * 3 % 8) as f32 * 0.028) / 0.82);
        let remaining = 1.0 - progress;
        let direction = (pivot - egui::vec2(0.5, 0.5)).normalized();
        let travel = direction * (remaining.powi(2) * 180.0 * scale);
        let angle = remaining
            * if i % 2 == 0 {
                0.65
            } else {
                -0.65
            };
        let mut mesh = egui::Mesh::with_texture(art.color.id());
        for uv in uvs {
            mesh.vertices.push(egui::epaint::Vertex {
                pos: rect.min
                    + pivot * size
                    + travel
                    + rotate((uv.to_vec2() - pivot) * size * (1.0 - remaining * 0.22), angle),
                uv,
                color: egui::Color32::WHITE.gamma_multiply(fade * smooth(progress / 0.16)),
            });
        }
        mesh.add_triangle(0, 1, 2);
        painter.add(egui::Shape::mesh(mesh));
    }

    if time >= ASSEMBLED_AT {
        painter.image(
            art.silhouette.id(),
            rect,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            GOLD.gamma_multiply(impact * 0.35 * fade),
        );
        shine(&painter, art.silhouette.id(), rect, time, fade);
    }

    for i in 0..28 {
        let angle = i as f32 * 2.399_963;
        let direction = egui::vec2(angle.cos(), angle.sin());
        let flight = ((time - ASSEMBLED_AT - (i % 4) as f32 * 0.035) / 0.9).clamp(0.0, 1.0);
        if flight > 0.0 && flight < 1.0 {
            let position =
                center + direction * (54.0 + flight * (100.0 + (i % 5) as f32 * 13.0)) * scale;
            let alpha = (1.0 - flight).powi(2) * fade;
            painter.line_segment(
                [position - direction * (7.0 + flight * 8.0) * scale, position],
                egui::Stroke::new(1.2 * scale, GOLD.gamma_multiply(alpha * 0.65)),
            );
            sparkle(&painter, position, (2.0 + (i % 3) as f32) * scale, alpha);
        }
    }
    for (i, offset) in [egui::vec2(-0.24, -0.29), egui::vec2(0.24, 0.12), egui::vec2(-0.08, 0.34)]
        .into_iter()
        .enumerate()
    {
        let glint = (1.0 - ((time - 1.48 - i as f32 * 0.28) / 0.24).abs()).max(0.0);
        if glint > 0.0 {
            sparkle(&painter, center + offset * size, (7.0 + 7.0 * glint) * scale, glint * fade);
        }
    }

    let text_alpha = smooth((time - 0.85) / 0.35) * fade;
    for (text, y, font_size, color) in [
        (rank.category(), 138.0, 12.0, GOLD),
        (rank.label(), 163.0, 28.0, egui::Color32::from_rgb(255, 245, 220)),
    ] {
        let pos = center + egui::vec2(0.0, y) * scale;
        for offset in [
            egui::vec2(-1.0, 0.0),
            egui::vec2(1.0, 0.0),
            egui::vec2(0.0, -1.0),
            egui::vec2(0.0, 2.0),
        ] {
            painter.text(
                pos + offset * scale,
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::proportional(font_size * scale),
                egui::Color32::BLACK.gamma_multiply(text_alpha * 0.8),
            );
        }
        painter.text(
            pos,
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(font_size * scale),
            color.gamma_multiply(text_alpha),
        );
    }
}

pub(in crate::app) fn draw(
    mut contexts: EguiContexts,
    state: Res<State<AppState>>,
    game: Res<ActiveGame>,
    practice: Res<LocalPractice>,
    terminal: Res<TerminalPresentation>,
    mut view: ResMut<super::campaign_panel::CampaignUi>,
) {
    if !matches!(*state.get(), AppState::Map | AppState::EndGame)
        || *game != ActiveGame::LocalPractice
        || terminal.spectating
    {
        view.promotion = None;
        return;
    }
    let Some(promotion) = view.promotion else {
        return;
    };
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let time = (ctx.input(|input| input.time) - promotion.started_at).max(0.0) as f32;
    if time >= DURATION || promotion.player != practice.active_player {
        view.promotion = None;
        return;
    }
    paint(ctx, promotion.rank, time, super::viewport_ui_scale(ctx.content_rect().size()));
    ctx.request_repaint_after(std::time::Duration::from_millis(16));
}

#[cfg(test)]
#[path = "../../tests/unit/rank_promotion.rs"]
mod tests;
