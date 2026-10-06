//! Shared right-side database failure badges, including menu and campaign requests.

use std::collections::VecDeque;

use super::*;

const DISPLAY_SECONDS: f32 = 5.0;
const MAX_ERRORS: usize = 6;
const ERROR_ACCENT: egui::Color32 = egui::Color32::from_rgb(255, 105, 120);

fn error_fill() -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(58, 25, 29, 240)
}

struct ErrorNotification {
    text: String,
    seconds_left: f32,
}

/// Database failures stay independent of player-scoped campaign notifications.
#[derive(Resource, Default)]
pub(in crate::app) struct ErrorNotifications(VecDeque<ErrorNotification>);

impl ErrorNotifications {
    fn push(&mut self, mut text: String) {
        if let Some(first) = text.get_mut(..1) {
            first.make_ascii_uppercase();
        }
        if let Some(existing) = self.0.iter_mut().find(|error| error.text == text) {
            existing.seconds_left = DISPLAY_SECONDS;
            return;
        }
        self.0.push_back(ErrorNotification {
            text,
            seconds_left: DISPLAY_SECONDS,
        });
        while self.0.len() > MAX_ERRORS {
            self.0.pop_front();
        }
    }

    fn advance(&mut self, seconds: f32) {
        for error in &mut self.0 {
            error.seconds_left -= seconds;
        }
        self.0.retain(|error| error.seconds_left > 0.0);
    }
}

pub(in crate::app) fn update(
    time: Res<Time>,
    mut client: ResMut<online::OnlineClient>,
    mut errors: ResMut<ErrorNotifications>,
    mut sounds: ResMut<toasts::ToastQueue>,
) {
    errors.advance(time.delta_secs());
    for error in client.take_errors() {
        errors.push(error);
        sounds.request_error_sound();
    }
}

pub(in crate::app) fn draw(
    mut contexts: EguiContexts,
    errors: Res<ErrorNotifications>,
    state: Res<State<AppState>>,
) {
    if errors.0.is_empty()
        || matches!(
            *state.get(),
            AppState::MainMenu
                | AppState::CreateGame
                | AppState::JoinGame
                | AppState::ResumeGame
                | AppState::RecoverPlayer
                | AppState::Lobby
                | AppState::Settings
                | AppState::PracticeSetup
        )
    {
        return;
    }
    let Ok(context) = contexts.ctx_mut() else {
        return;
    };
    show(context, &errors);
}

fn stack_id() -> egui::Id {
    egui::Id::new("augustus_error_stack_bottom")
}

/// Campaign notices follow errors without covering the same part of the map.
pub(in crate::app) fn campaign_toast_top(context: &egui::Context, scale: f32) -> f32 {
    let current_pass = context.cumulative_pass_nr();
    let viewport_top = context.content_rect().top();
    context.data(|data| {
        data.get_temp::<(u64, f32)>(stack_id())
            .filter(|(pass, _)| *pass == current_pass)
            .map_or(76.0 * scale, |(_, bottom)| {
                (76.0 * scale).max(bottom - viewport_top + 6.0 * scale)
            })
    })
}

fn show(context: &egui::Context, errors: &ErrorNotifications) {
    let viewport = context.content_rect();
    let scale = (viewport_ui_scale(viewport.size()) * 1.1).clamp(0.8, 1.35);
    let id = egui::Id::new("augustus_database_errors");
    let order = egui::Order::Tooltip;
    let transform = egui::emath::TSTransform::new(
        egui::vec2(viewport.right(), viewport.top()) * (1.0 - scale),
        scale,
    );
    context.set_transform_layer(egui::LayerId::new(order, id), transform);
    let response = egui::Area::new(id)
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-12.0, 70.0))
        .order(order)
        .constrain(false)
        .layout(egui::Layout::top_down(egui::Align::Max))
        .show(context, |ui| {
            ui.set_clip_rect(logical_content_rect(ui));
            ui.set_max_width(560.0_f32.min((viewport.width() / scale - 50.0).max(0.0)));
            ui.spacing_mut().item_spacing.y = 6.0;
            for error in &errors.0 {
                egui::Frame::new()
                    .fill(error_fill())
                    .stroke(egui::Stroke::new(1.0, ERROR_ACCENT))
                    .corner_radius(5.0)
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&error.text).small().color(ERROR_ACCENT),
                            )
                            .halign(egui::Align::Min)
                            .wrap(),
                        );
                    });
            }
        })
        .response;
    let pass = context.cumulative_pass_nr();
    context.data_mut(|data| {
        data.insert_temp(stack_id(), (pass, transform.mul_rect(response.rect).bottom()));
    });
}

#[cfg(test)]
#[path = "../../tests/unit/error_toasts.rs"]
mod tests;
