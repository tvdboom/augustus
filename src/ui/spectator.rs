//! Terminal overlay and the shared inspectors' read-only spectator presentation.

use super::*;

const READ_ONLY_ID: &str = "augustus-spectator-read-only";

pub(super) fn set_read_only(ctx: &egui::Context, read_only: bool) {
    ctx.data_mut(|data| data.insert_temp(egui::Id::new(READ_ONLY_ID), read_only));
}

pub(super) fn read_only(ctx: &egui::Context) -> bool {
    ctx.data(|data| data.get_temp::<bool>(egui::Id::new(READ_ONLY_ID)).unwrap_or(false))
}

/// Inspect from the province owner's perspective without changing the active player.
pub(super) fn inspection_player(
    campaign: &campaign::Campaign,
    province: Option<usize>,
    fallback: usize,
) -> usize {
    province
        .and_then(|id| campaign.economy.provinces.get(id))
        .and_then(|province| province.owner.or(province.overlord))
        .unwrap_or(fallback)
}

/// Independent provinces expose the same complete sections as owned provinces.
pub(super) fn domestic_view(ctx: &egui::Context, owned: bool) -> bool {
    owned || read_only(ctx)
}

/// Disabled commands retain their information tooltips during inspection.
pub(super) trait InspectionHover {
    fn inspection_hover_ui(self, contents: impl FnOnce(&mut egui::Ui)) -> Self;
    fn inspection_hover_text(self, text: impl Into<egui::WidgetText>) -> Self;
}

impl InspectionHover for egui::Response {
    fn inspection_hover_ui(self, contents: impl FnOnce(&mut egui::Ui)) -> Self {
        if read_only(&self.ctx) && !self.enabled() {
            self.on_disabled_hover_ui(contents)
        } else {
            self.on_hover_ui(contents)
        }
    }

    fn inspection_hover_text(self, text: impl Into<egui::WidgetText>) -> Self {
        self.inspection_hover_ui(|ui| {
            ui.set_max_width(ui.spacing().tooltip_width);
            ui.add(egui::Label::new(text));
        })
    }
}

/// Paint only the reference-style status text, without adding a clickable surface.
fn status(ctx: &egui::Context) {
    let screen = ctx.content_rect();
    let scale = viewport_ui_scale(screen.size());
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("augustus_spectator_status"),
    ));
    let position = screen.right_bottom() - egui::vec2(16.0, 12.0) * scale;
    let font = egui::FontId::proportional(32.0 * scale);
    painter.text(
        position + egui::vec2(1.0, 1.0) * scale,
        egui::Align2::RIGHT_BOTTOM,
        "Spectator",
        font.clone(),
        egui::Color32::from_black_alpha(180),
    );
    painter.text(
        position,
        egui::Align2::RIGHT_BOTTOM,
        "Spectator",
        font,
        egui::Color32::from_rgb(255, 238, 210),
    );
}

/// Fade the final result over the loaded map, matching the reference game's two choices.
pub(in crate::app) fn draw_end_game(
    mut contexts: EguiContexts,
    time: Res<Time>,
    mut terminal: ResMut<TerminalPresentation>,
    mut next: ResMut<NextState<AppState>>,
    sound: Res<MenuAudio>,
    audio: Res<Audio>,
    assets: Res<AssetServer>,
    mut online: Option<ResMut<online::OnlineClient>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let Some(outcome) = terminal.outcome else {
        return;
    };
    terminal.fade_elapsed += time.delta_secs();
    let opacity = terminal.opacity();
    if opacity < 1.0 {
        ctx.request_repaint();
    }
    let viewport = ctx.content_rect();
    egui::Area::new(egui::Id::new("augustus_end_game_backdrop"))
        .fixed_pos(viewport.min)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(viewport.size(), egui::Sense::click());
            ui.painter().rect_filled(
                rect,
                0.0,
                egui::Color32::from_black_alpha((opacity * 170.0) as u8),
            );
        });
    let scale = viewport_ui_scale(viewport.size());
    let content_width = MENU_CONTENT_WIDTH.min((viewport.width() / scale - 32.0).max(240.0));
    let id = egui::Id::new("augustus_end_game_result");
    set_menu_layer_scale(ctx, id, egui::Order::Foreground, viewport.min, scale);
    egui::Area::new(id)
        .pivot(egui::Align2::CENTER_CENTER)
        .fixed_pos(egui::pos2(viewport.width() / scale * 0.5, viewport.height() / scale * 0.5))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            apply_menu_style(ui);
            ui.set_opacity(opacity);
            ui.set_width(content_width);
            ui.vertical_centered(|ui| {
                let heading = match outcome {
                    TerminalOutcome::Victory => "You won",
                    TerminalOutcome::Defeat => "You lost",
                };
                ui.heading(egui::RichText::new(heading).size(MENU_TITLE_TEXT_SIZE));
                ui.add_space(28.0);
                let size = menu_button_metrics(ui);
                if menu_action_button(
                    ui,
                    "Spectate",
                    true,
                    false,
                    size,
                    MENU_ACTION_TEXT_SIZE,
                    &sound,
                    &audio,
                    &assets,
                ) {
                    terminal.spectating = true;
                    next.set(AppState::Map);
                }
                ui.add_space(FORM_CARD_GAP);
                if menu_action_button(
                    ui,
                    "Return to Main Menu",
                    true,
                    false,
                    size,
                    MENU_ACTION_TEXT_SIZE,
                    &sound,
                    &audio,
                    &assets,
                ) {
                    if let Some(client) = online.as_mut().filter(|c| c.record.is_some()) {
                        client.leave();
                    } else {
                        next.set(AppState::MainMenu);
                    }
                }
            });
        });
}

/// Province and army inspection is handled by the regular campaign panels.
pub(in crate::app) fn draw_spectator(
    mut contexts: EguiContexts,
    terminal: Res<TerminalPresentation>,
    campaign: Res<campaign::Campaign>,
) {
    if terminal.spectating && campaign.active {
        if let Ok(ctx) = contexts.ctx_mut() {
            status(ctx);
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/spectator_ui.rs"]
mod tests;
