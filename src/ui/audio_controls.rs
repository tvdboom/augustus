//! Circular audio and volume controls.

use super::*;

pub(super) fn settings_button_destination(state: AppState, game: ActiveGame) -> AppState {
    if state == AppState::GameSettings {
        game.screen()
    } else {
        AppState::GameSettings
    }
}

pub(super) fn settings_button(ui: &mut egui::Ui, scale: f32) -> egui::Response {
    let response = circular_audio_button(ui, scale);
    let center = response.rect.center();
    let stroke = egui::Stroke::new(1.8 * scale, CREAM);
    ui.painter().circle_stroke(center, 5.0 * scale, stroke);
    for step in 0..8 {
        let angle = std::f32::consts::TAU * step as f32 / 8.0;
        let direction = egui::vec2(angle.cos(), angle.sin());
        ui.painter().line_segment(
            [center + direction * 6.5 * scale, center + direction * 9.5 * scale],
            stroke,
        );
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Settings"));
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub(super) fn circular_audio_button(ui: &mut egui::Ui, scale: f32) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(32.0, 32.0) * scale, egui::Sense::click());
    let highlighted = response.hovered() || response.has_focus();
    ui.painter().circle(
        rect.center(),
        15.0 * scale,
        if highlighted {
            egui::Color32::from_rgb(73, 39, 31)
        } else {
            egui::Color32::from_rgb(36, 23, 22)
        },
        egui::Stroke::new(
            1.5 * scale,
            if highlighted {
                CREAM
            } else {
                GOLD
            },
        ),
    );
    response
}

pub(super) fn audio_mode_button(ui: &mut egui::Ui, mode: AudioMode, scale: f32) -> egui::Response {
    let response = circular_audio_button(ui, scale);
    let center = response.rect.center();
    let point = |x, y| center + egui::vec2(x, y) * scale;
    let stroke = egui::Stroke::new(1.8 * scale, CREAM);
    match mode {
        AudioMode::Mute | AudioMode::Effects => {
            ui.painter().add(egui::Shape::convex_polygon(
                vec![
                    point(-9.0, -3.0),
                    point(-6.0, -3.0),
                    point(-1.0, -7.0),
                    point(-1.0, 7.0),
                    point(-6.0, 3.0),
                    point(-9.0, 3.0),
                ],
                CREAM,
                egui::Stroke::NONE,
            ));
            if mode == AudioMode::Mute {
                ui.painter().line_segment([point(3.0, -3.0), point(9.0, 3.0)], stroke);
                ui.painter().line_segment([point(9.0, -3.0), point(3.0, 3.0)], stroke);
            } else {
                for radius in [5.0, 9.0] {
                    let points = (0..=12)
                        .map(|step| {
                            let angle = (step as f32 / 12.0 - 0.5) * 2.0;
                            point(-1.0 + radius * angle.cos(), radius * angle.sin())
                        })
                        .collect();
                    ui.painter().add(egui::Shape::line(points, stroke));
                }
            }
        },
        AudioMode::Music => {
            ui.painter().add(egui::Shape::convex_polygon(
                vec![point(-3.0, -6.0), point(7.0, -8.0), point(7.0, -4.5), point(-3.0, -2.5)],
                CREAM,
                egui::Stroke::NONE,
            ));
            ui.painter().line_segment([point(-3.0, -5.0), point(-3.0, 6.0)], stroke);
            ui.painter().line_segment([point(7.0, -7.0), point(7.0, 4.0)], stroke);
            ui.painter().circle_filled(point(-5.0, 6.0), 2.6 * scale, CREAM);
            ui.painter().circle_filled(point(5.0, 4.0), 2.6 * scale, CREAM);
        },
    }
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Audio mode"));
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub(super) fn volume_slider(ui: &mut egui::Ui, sound: &mut MenuAudio) -> egui::Response {
    let mut volume = if sound.mode == AudioMode::Mute {
        0.0
    } else {
        sound.volume
    };
    ui.scope(|ui| {
        let width = ui.available_width().clamp(80.0, 280.0);
        let style = ui.style_mut();
        style.spacing.slider_width = width;
        style.spacing.interact_size.y = 24.0;
        style.visuals.selection.bg_fill = egui::Color32::from_rgb(136, 65, 46);
        for state in [
            &mut style.visuals.widgets.inactive,
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
        ] {
            state.bg_fill = egui::Color32::from_rgb(40, 25, 22);
            state.fg_stroke = egui::Stroke::new(2.0, CREAM);
            state.corner_radius = egui::CornerRadius::same(6);
        }
        ui.label(egui::RichText::new(format!("Volume  {:.0}%", volume * 100.0)).size(18.0));
        let response = ui
            .add(egui::Slider::new(&mut volume, 0.0..=1.0).show_value(false).trailing_fill(true))
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        if response.changed() {
            sound.volume = volume;
            if sound.mode == AudioMode::Mute && volume > 0.0 {
                sound.mode = sound.restored_mode;
            }
        }
        response
    })
    .inner
}

pub(super) fn volume_popover(button: &egui::Response, sound: &mut MenuAudio) {
    let id = button.id.with("volume");
    let was_open = button.ctx.data(|data| data.get_temp::<bool>(id).unwrap_or(false));
    let popup = egui::Popup::from_response(button)
        .id(id)
        .kind(egui::PopupKind::Tooltip)
        .gap(0.0)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .width(200.0_f32.min((button.ctx.content_rect().width() - 40.0).max(80.0)));
    let hovering = popup.get_popup_rect().is_some_and(|rect| {
        button
            .ctx
            .pointer_hover_pos()
            .is_some_and(|pos| rect.union(button.rect).expand(4.0).contains(pos))
    });
    let dragging =
        button.ctx.data(|data| data.get_temp::<bool>(id.with("dragging")).unwrap_or(false))
            && button.ctx.input(|input| input.pointer.primary_down());
    let mut open = button.hovered() || (was_open && (hovering || dragging));
    let frame = egui::Frame::new()
        .fill(egui::Color32::from_rgba_unmultiplied(31, 20, 18, 245))
        .stroke(egui::Stroke::new(1.0, GOLD))
        .corner_radius(6.0)
        .inner_margin(10);
    let _ = popup.frame(frame).open_bool(&mut open).show(|ui| {
        let response = volume_slider(ui, sound);
        ui.ctx().data_mut(|data| data.insert_temp(id.with("dragging"), response.dragged()));
    });
    button.ctx.data_mut(|data| data.insert_temp(id, open));
}
