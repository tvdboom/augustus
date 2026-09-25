//! Menu buttons and shared audio controls.

use super::*;

pub(super) fn audio_choice_row(
    ui: &mut egui::Ui,
    sound: &mut MenuAudio,
    audio: &Audio,
    assets: &AssetServer,
) {
    form_option_card(
        ui,
        "Audio",
        "Muted turns off all audio. Effects plays sound effects only. Music plays both music and sound effects.",
        |ui| {
            let gap = 8.0;
            let row_width = ui.available_width();
            let width = ((row_width - gap * 2.0) / 3.0).max(1.0);
            ui.allocate_ui_with_layout(
                egui::vec2(row_width, MENU_CONTROL_HEIGHT),
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for (mode, label) in [
                        (AudioMode::Mute, "Muted"),
                        (AudioMode::Effects, "Effects"),
                        (AudioMode::Music, "Music"),
                    ] {
                        let selected = sound.mode == mode;
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(width, MENU_CONTROL_HEIGHT),
                            egui::Sense::click(),
                        );
                        let fill = if response.hovered() {
                            egui::Color32::from_rgb(136, 65, 46)
                        } else if selected {
                            egui::Color32::from_rgb(107, 49, 38)
                        } else {
                            egui::Color32::from_rgba_unmultiplied(37, 23, 21, 228)
                        };
                        let border = if selected {
                            egui::Color32::from_rgba_unmultiplied(220, 170, 105, 145)
                        } else {
                            egui::Color32::from_rgba_unmultiplied(190, 145, 97, 92)
                        };
                        ui.painter().rect(
                            rect,
                            6.0,
                            fill,
                            egui::Stroke::new(1.0, border),
                            egui::StrokeKind::Inside,
                        );
                        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
                        response.widget_info(|| {
                            egui::WidgetInfo::selected(
                                egui::WidgetType::Button,
                                true,
                                selected,
                                label,
                            )
                        });
                        paint_audio_choice_icon(ui, rect, label);
                        if response.clicked() {
                            play_click(sound, audio, assets);
                            sound.set_mode(mode);
                        }
                    }
                },
            );
        },
    );
}

pub(super) fn paint_audio_choice_icon(ui: &egui::Ui, rect: egui::Rect, label: &str) {
    let painter = ui.painter();
    let color = ui.visuals().strong_text_color();
    let measured = painter.layout_no_wrap(
        label.to_owned(),
        egui::FontId::proportional(MENU_CONTROL_TEXT_SIZE),
        color,
    );
    let font_size = MENU_CONTROL_TEXT_SIZE
        * ((rect.width() - 12.0 - 23.0).max(1.0) / measured.size().x).min(1.0);
    let label_galley =
        painter.layout_no_wrap(label.to_owned(), egui::FontId::proportional(font_size), color);
    let icon_width = 16.0;
    let gap = 7.0;
    let group_width = icon_width + gap + label_galley.size().x;
    let group_left = rect.center().x - group_width * 0.5;
    let center = egui::pos2(group_left + icon_width * 0.5, rect.center().y);
    let stroke = egui::Stroke::new(1.6, color);

    if label == "Music" {
        painter.line_segment(
            [center + egui::vec2(2.0, -6.0), center + egui::vec2(2.0, 3.5)],
            egui::Stroke::new(2.0, color),
        );
        painter.line_segment(
            [center + egui::vec2(2.0, -6.0), center + egui::vec2(7.0, -4.0)],
            egui::Stroke::new(2.0, color),
        );
        painter.circle_filled(center + egui::vec2(-1.0, 4.5), 3.0, color);
    } else {
        painter.add(egui::Shape::convex_polygon(
            vec![
                center + egui::vec2(-6.0, -2.5),
                center + egui::vec2(-3.0, -2.5),
                center + egui::vec2(1.0, -6.0),
                center + egui::vec2(1.0, 6.0),
                center + egui::vec2(-3.0, 2.5),
                center + egui::vec2(-6.0, 2.5),
            ],
            color,
            egui::Stroke::NONE,
        ));
        if label == "Muted" {
            painter.line_segment(
                [center + egui::vec2(4.0, -3.5), center + egui::vec2(10.0, 3.5)],
                stroke,
            );
            painter.line_segment(
                [center + egui::vec2(10.0, -3.5), center + egui::vec2(4.0, 3.5)],
                stroke,
            );
        } else {
            painter.line_segment(
                [center + egui::vec2(4.0, -3.5), center + egui::vec2(6.5, -1.5)],
                stroke,
            );
            painter.line_segment(
                [center + egui::vec2(6.5, -1.5), center + egui::vec2(6.5, 1.5)],
                stroke,
            );
            painter.line_segment(
                [center + egui::vec2(6.5, 1.5), center + egui::vec2(4.0, 3.5)],
                stroke,
            );
        }
    }

    painter.galley(
        egui::pos2(group_left + icon_width + gap, rect.center().y - label_galley.size().y * 0.5),
        label_galley,
        color,
    );
}

pub(super) fn menu_button_metrics(ui: &egui::Ui) -> egui::Vec2 {
    egui::vec2(MENU_ACTION_WIDTH.min(ui.available_width()), MENU_ACTION_HEIGHT)
}

pub(super) fn menu_button_pair(
    ui: &mut egui::Ui,
    left_label: &str,
    right_label: &str,
    right_enabled: bool,
    sound: &MenuAudio,
    audio: &Audio,
    assets: &AssetServer,
) -> (bool, bool) {
    let width = MENU_ACTION_WIDTH.min(((ui.available_width() - FORM_CARD_GAP) * 0.5).max(1.0));
    let button_size = egui::vec2(width, MENU_ACTION_HEIGHT);
    ui.allocate_ui_with_layout(
        egui::vec2(width * 2.0 + FORM_CARD_GAP, MENU_ACTION_HEIGHT),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = FORM_CARD_GAP;
            let left = menu_action_button(
                ui,
                left_label,
                true,
                false,
                button_size,
                MENU_ACTION_TEXT_SIZE,
                sound,
                audio,
                assets,
            );
            let right = menu_action_button(
                ui,
                right_label,
                right_enabled,
                true,
                button_size,
                MENU_ACTION_TEXT_SIZE,
                sound,
                audio,
                assets,
            );
            (left, right)
        },
    )
    .inner
}

pub(super) fn menu_action_button(
    ui: &mut egui::Ui,
    label: &str,
    enabled: bool,
    primary: bool,
    size: egui::Vec2,
    text_size: f32,
    sound: &MenuAudio,
    audio: &Audio,
    assets: &AssetServer,
) -> bool {
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    let hovered = enabled && response.hovered();
    let pressed = enabled && response.is_pointer_button_down_on();
    let fill = match (primary, pressed, hovered, enabled) {
        (_, _, _, false) => egui::Color32::from_rgba_unmultiplied(39, 25, 23, 225),
        (true, true, _, _) => egui::Color32::from_rgb(122, 57, 42),
        (true, _, true, _) => egui::Color32::from_rgb(136, 65, 46),
        (true, _, _, _) => egui::Color32::from_rgb(107, 49, 38),
        (false, true, _, _) => egui::Color32::from_rgba_unmultiplied(91, 48, 39, 242),
        (false, _, true, _) => egui::Color32::from_rgba_unmultiplied(72, 38, 32, 238),
        (false, _, _, _) => egui::Color32::from_rgba_unmultiplied(37, 23, 21, 228),
    };
    let border = if primary && enabled {
        egui::Color32::from_rgba_unmultiplied(220, 170, 105, 145)
    } else {
        egui::Color32::from_rgba_unmultiplied(190, 145, 97, 92)
    };
    ui.painter().rect(
        rect,
        egui::CornerRadius::same(6),
        fill,
        egui::Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );
    let measured =
        ui.painter().layout_no_wrap(label.to_owned(), egui::FontId::proportional(text_size), CREAM);
    let label_size = text_size * ((size.x - 24.0).max(1.0) / measured.size().x).min(1.0);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(label_size),
        if enabled {
            CREAM
        } else {
            MUTED_TEXT.gamma_multiply(0.6)
        },
    );
    if enabled {
        let clicked = response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
        if clicked {
            play_click(sound, audio, assets);
        }
        clicked
    } else {
        false
    }
}

pub(super) fn apply_menu_style(ui: &mut egui::Ui) {
    let visuals = &mut ui.style_mut().visuals;
    visuals.button_frame = true;
    visuals.disabled_alpha = 0.65;
    visuals.extreme_bg_color = egui::Color32::from_rgb(38, 25, 23);
    let states = [
        (&mut visuals.widgets.inactive, egui::Color32::from_rgba_unmultiplied(37, 23, 21, 228)),
        (&mut visuals.widgets.hovered, egui::Color32::from_rgba_unmultiplied(72, 38, 32, 238)),
        (&mut visuals.widgets.active, egui::Color32::from_rgb(122, 57, 42)),
        (&mut visuals.widgets.open, egui::Color32::from_rgb(136, 65, 46)),
        (
            &mut visuals.widgets.noninteractive,
            egui::Color32::from_rgba_unmultiplied(39, 25, 23, 225),
        ),
    ];
    for (state, color) in states {
        state.bg_fill = color;
        state.weak_bg_fill = color;
        state.bg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(190, 145, 97, 92));
        state.corner_radius = egui::CornerRadius::same(6);
        state.expansion = 0.0;
    }
}

pub(super) fn play_click(sound: &MenuAudio, audio: &Audio, assets: &AssetServer) {
    if sound.mode == AudioMode::Mute || sound.volume <= 0.001 {
        return;
    }
    let decibels = -8.0 + 20.0 * sound.volume.clamp(0.001, 1.0).log10();
    audio.play(assets.load("audio/ui-click.ogg")).with_volume(decibels);
}

pub(super) fn draw_audio_controls(
    mut contexts: EguiContexts,
    mut sound: ResMut<MenuAudio>,
    audio: Res<Audio>,
    assets: Res<AssetServer>,
) {
    let Ok(context) = contexts.ctx_mut() else {
        return;
    };
    let scale = viewport_ui_scale(context.content_rect().size());
    let button = egui::Area::new(egui::Id::new("augustus_audio_controls"))
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-20.0, 20.0) * scale)
        .order(egui::Order::Foreground)
        .show(context, |ui| audio_mode_button(ui, sound.mode, scale))
        .inner;
    if button.clicked() {
        sound.toggle_mute();
        play_click(&sound, &audio, &assets);
    }
    volume_popover(&button, &mut sound);
}
