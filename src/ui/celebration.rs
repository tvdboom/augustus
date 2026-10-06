//! A short, non-interactive confetti burst for successful civic events.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use super::{ActiveGame, AppState, TerminalPresentation};

const DURATION: f64 = 2.0;
const PIECES: u32 = 108;
const COLORS: [egui::Color32; 6] = [
    egui::Color32::from_rgb(195, 60, 46),
    egui::Color32::from_rgb(222, 166, 58),
    egui::Color32::from_rgb(234, 213, 151),
    egui::Color32::from_rgb(71, 123, 104),
    egui::Color32::from_rgb(76, 111, 154),
    egui::Color32::from_rgb(241, 231, 205),
];

#[derive(Resource, Default)]
pub(in crate::app) struct EventCelebration {
    started_at: Option<f64>,
}

impl EventCelebration {
    pub(in crate::app) fn start(&mut self, now: f64) {
        self.started_at = Some(now);
    }
}

fn random(index: u32, salt: u32) -> f32 {
    let mut value = index.wrapping_mul(0x9e37_79b9) ^ salt.wrapping_mul(0x85eb_ca6b);
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846c_a68b);
    value ^= value >> 16;
    value as f32 / u32::MAX as f32
}

pub(in crate::app) fn draw(
    mut contexts: EguiContexts,
    state: Res<State<AppState>>,
    game: Res<ActiveGame>,
    terminal: Res<TerminalPresentation>,
    mut celebration: ResMut<EventCelebration>,
) {
    if *state.get() != AppState::Map || !game.is_campaign() || terminal.spectating {
        celebration.started_at = None;
        return;
    }
    let Some(started_at) = celebration.started_at else {
        return;
    };
    let Ok(context) = contexts.ctx_mut() else {
        return;
    };
    let elapsed = (context.input(|input| input.time) - started_at).max(0.0);
    if elapsed >= DURATION {
        celebration.started_at = None;
        return;
    }

    let viewport = context.content_rect();
    let scale = super::viewport_ui_scale(viewport.size());
    let seconds = elapsed as f32;
    let fade = ((DURATION as f32 - seconds) / 0.55).clamp(0.0, 1.0);
    let painter = context.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("civic-event-confetti"),
    ));
    for index in 0..PIECES {
        let origin = egui::pos2(
            viewport.left() + viewport.width() * [0.17, 0.5, 0.83][(index % 3) as usize],
            viewport.top() + viewport.height() * 0.68,
        );
        let vx = (random(index, 1) - 0.5) * 590.0 * scale;
        let vy = (-290.0 - random(index, 2) * 280.0) * scale;
        let position = origin
            + egui::vec2(vx * seconds, vy * seconds + 0.5 * 435.0 * scale * seconds * seconds);
        if !viewport.expand(12.0 * scale).contains(position) {
            continue;
        }
        let angle =
            random(index, 3) * std::f32::consts::TAU + seconds * (3.0 + random(index, 4) * 9.0);
        let direction = egui::vec2(angle.cos(), angle.sin());
        let cross = egui::vec2(-direction.y, direction.x);
        let half_width = (2.0 + random(index, 5) * 1.8) * scale;
        let half_height = (3.0 + random(index, 6) * 3.0) * scale;
        let color = COLORS[(index as usize) % COLORS.len()].gamma_multiply(fade);
        painter.add(egui::Shape::convex_polygon(
            vec![
                position - direction * half_width - cross * half_height,
                position + direction * half_width - cross * half_height,
                position + direction * half_width + cross * half_height,
                position - direction * half_width + cross * half_height,
            ],
            color,
            egui::Stroke::NONE,
        ));
    }
    context.request_repaint_after(std::time::Duration::from_millis(16));
}
