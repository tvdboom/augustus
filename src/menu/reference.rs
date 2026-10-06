//! Reference menu geometry and copy from the local Stellarion repository.
//! Only the palette and online model boundary differ.
use super::*;
use crate::multiplayer::model::GameSummary;
use chrono::Local as ChronoLocal;

const MAX_DISPLAY_NAME_CHARS: usize = 16;

pub(in crate::app) fn valid_player_name(value: &str) -> bool {
    (1..=MAX_DISPLAY_NAME_CHARS).contains(&value.trim().chars().count())
}

fn format_saved_timestamp(timestamp: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(timestamp)
        .map(|saved_at| saved_at.with_timezone(&ChronoLocal).format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|_| "Time unavailable".to_string())
}

pub(in crate::app) fn reference_menu_error_title(state: AppState) -> &'static str {
    match state {
        AppState::JoinGame => "Unable to join game",
        AppState::RecoverPlayer => "Unable to recover game",
        AppState::Lobby => "Unable to continue game",
        _ => "Unable to complete action",
    }
}

pub(in crate::app) fn reference_menu_error_width(
    viewport: egui::Vec2,
    content_width: f32,
    state: AppState,
) -> f32 {
    let occupied_width = if state == AppState::MainMenu {
        308.0_f32.min(content_width)
    } else {
        content_width
    };
    let content_right = viewport.x * 0.5 + occupied_width * 0.5;
    let clear_side_width = (viewport.x - 36.0 - content_right).max(0.0);
    let preferred = (viewport.x * 0.38).clamp(260.0, 400.0).min((viewport.x - 48.0).max(220.0));
    if clear_side_width >= 220.0 {
        preferred.min(clear_side_width)
    } else {
        preferred
    }
}

pub(in crate::app) fn reference_menu_error_panel_width(
    ui: &mut egui::Ui,
    title: &str,
    error: &str,
    width: f32,
) -> egui::Response {
    let frame = egui::Frame::new()
        .fill(egui::Color32::from_rgba_unmultiplied(67, 23, 32, 232))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(240, 112, 123, 150)))
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(16, 12));
    frame
        .show(ui, |ui| {
            ui.set_width((width - frame.total_margin().sum().x).max(1.0));
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                ui.label(
                    egui::RichText::new(title)
                        .size(17.0)
                        .strong()
                        .color(egui::Color32::from_rgb(255, 171, 179)),
                );
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(error)
                            .size(16.0)
                            .color(egui::Color32::from_rgb(255, 230, 232)),
                    )
                    .wrap(),
                );
            });
        })
        .response
}

pub(in crate::app) fn reference_resume_action_buttons(
    ui: &mut egui::Ui,
    busy: bool,
    refreshing: bool,
) -> (bool, bool, bool) {
    let (size, text_size, _) = reference_menu_button_metrics(ui);
    let gap = 12.0;
    let width = size.x.min(((ui.available_width() - gap * 2.0) / 3.0).max(1.0));
    let labels = ["Back", "Refresh", "Recover Game"];
    let measured_width = labels
        .iter()
        .map(|label| {
            ui.painter()
                .layout_no_wrap((*label).to_string(), egui::FontId::proportional(text_size), CREAM)
                .size()
                .x
        })
        .fold(0.0_f32, f32::max);
    let text_size = text_size * ((width - 24.0) / measured_width).clamp(0.1, 1.0);
    let clicked = ui
        .allocate_ui_with_layout(
            egui::vec2(width * 3.0 + gap * 2.0, size.y),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.spacing_mut().item_spacing.x = gap;
                ui.spacing_mut().item_spacing.y = gap;
                let button_size = egui::vec2(width, size.y);
                let back = reference_menu_action_button(
                    ui,
                    labels[0],
                    true,
                    false,
                    false,
                    button_size,
                    text_size,
                );
                let refresh = ui.scope(|ui| {
                    reference_menu_action_button(
                        ui,
                        labels[1],
                        !busy && !refreshing,
                        false,
                        false,
                        button_size,
                        text_size,
                    )
                });
                if refreshing {
                    // Paint into the existing padding so progress never changes the layout.
                    let label_width = ui
                        .painter()
                        .layout_no_wrap(
                            labels[1].to_string(),
                            egui::FontId::proportional(text_size),
                            CREAM,
                        )
                        .size()
                        .x;
                    let padding = (button_size.x - label_width) * 0.5;
                    let spinner_size = (padding - 8.0).clamp(4.0, 16.0);
                    let spinner_rect = egui::Rect::from_center_size(
                        egui::pos2(
                            refresh.response.rect.left() + padding * 0.5,
                            refresh.response.rect.center().y,
                        ),
                        egui::Vec2::splat(spinner_size),
                    );
                    egui::Spinner::new().paint_at(ui, spinner_rect);
                }
                let recover = reference_menu_action_button(
                    ui,
                    labels[2],
                    !busy,
                    false,
                    false,
                    button_size,
                    text_size,
                );
                (back, refresh.inner, recover)
            },
        )
        .inner;
    clicked
}

pub(in crate::app) fn reference_resume_game_card(
    ui: &mut egui::Ui,
    game: &GameSummary,
    enabled: bool,
) -> bool {
    let secondary = if enabled {
        MUTED_TEXT
    } else {
        egui::Color32::from_rgb(112, 124, 137)
    };
    let content_width = (ui.available_width() - 55.0).max(1.0);
    let player_color = PLAYER_COLORS[game.player_color.min(PLAYER_COLORS.len() - 1)];
    let metadata = format!(
        "{}   ·   Turn {}   ·   {} player{}   ·",
        format_saved_timestamp(&game.saved_at),
        game.turn,
        game.player_count,
        if game.player_count == 1 {
            ""
        } else {
            "s"
        }
    );
    let metadata_font_size = 13.0;
    let measured_text_width = ui
        .painter()
        .layout_no_wrap(
            format!("{metadata} {}", game.display_name),
            egui::FontId::proportional(metadata_font_size),
            secondary,
        )
        .size()
        .x;
    let player_marker_width = 22.0;
    let details_font = egui::FontId::proportional(
        (metadata_font_size * (content_width - player_marker_width).max(1.0) / measured_text_width)
            .clamp(9.0, metadata_font_size),
    );
    let metadata = ui.painter().layout_no_wrap(metadata, details_font.clone(), secondary);
    let name_offset = metadata.size().x + player_marker_width;
    let mut name_job = egui::text::LayoutJob::simple_singleline(
        game.display_name.clone(),
        details_font,
        secondary,
    );
    name_job.wrap.max_width = (content_width - name_offset).max(1.0);
    name_job.wrap.max_rows = 1;
    name_job.wrap.break_anywhere = true;
    let name = ui.painter().layout_job(name_job);
    let size = egui::vec2(ui.available_width(), 72.0);
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    let hovered = enabled && response.hovered();
    let pressed = enabled && response.is_pointer_button_down_on();
    let fill = if pressed {
        egui::Color32::from_rgba_unmultiplied(91, 48, 39, 248)
    } else if hovered {
        egui::Color32::from_rgba_unmultiplied(72, 38, 32, 246)
    } else {
        egui::Color32::from_rgba_unmultiplied(31, 20, 18, 238)
    };
    let border = if hovered {
        egui::Color32::from_rgba_unmultiplied(220, 170, 105, 190)
    } else {
        egui::Color32::from_rgba_unmultiplied(190, 143, 94, 88)
    };
    let foreground = if enabled {
        CREAM
    } else {
        egui::Color32::from_rgb(142, 153, 166)
    };

    ui.painter().rect(
        rect,
        egui::CornerRadius::same(7),
        fill,
        egui::Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );
    ui.painter().rect_filled(
        egui::Rect::from_min_max(
            rect.min + egui::vec2(0.0, 10.0),
            egui::pos2(rect.left() + 3.0, rect.bottom() - 10.0),
        ),
        2.0,
        if enabled {
            egui::Color32::from_rgb(195, 145, 87)
        } else {
            egui::Color32::from_rgb(74, 101, 116)
        },
    );

    let left = rect.left() + 17.0;
    ui.painter().text(
        egui::pos2(left, rect.top() + 10.0),
        egui::Align2::LEFT_TOP,
        game.code.as_str(),
        egui::FontId::proportional(19.0),
        foreground,
    );
    let details_top = rect.top() + 39.0;
    ui.painter().galley(egui::pos2(left, details_top), metadata.clone(), secondary);
    let player_dot =
        egui::pos2(left + metadata.size().x + 11.0, details_top + metadata.size().y * 0.5);
    ui.painter().circle_filled(
        player_dot,
        4.0,
        if enabled {
            player_color
        } else {
            player_color.gamma_multiply(0.5)
        },
    );
    ui.painter().galley(egui::pos2(left + name_offset, details_top), name, secondary);

    let (status, status_color) = reference_resume_status_style(&game.status, enabled);
    let status_right = rect.right() - 38.0;
    let status_rect = ui.painter().text(
        egui::pos2(status_right, rect.top() + 13.0),
        egui::Align2::RIGHT_TOP,
        status,
        egui::FontId::proportional(13.0),
        status_color,
    );
    ui.painter().circle_filled(
        egui::pos2(status_rect.left() - 10.0, status_rect.center().y),
        3.5,
        status_color,
    );

    let arrow_center = egui::pos2(rect.right() - 18.0, rect.center().y);
    let arrow_color = if hovered {
        CREAM
    } else {
        secondary
    };
    ui.painter().line_segment(
        [arrow_center + egui::vec2(-2.0, -5.0), arrow_center + egui::vec2(3.0, 0.0)],
        egui::Stroke::new(1.8, arrow_color),
    );
    ui.painter().line_segment(
        [arrow_center + egui::vec2(3.0, 0.0), arrow_center + egui::vec2(-2.0, 5.0)],
        egui::Stroke::new(1.8, arrow_color),
    );

    if enabled {
        let clicked = response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
        if clicked {
            reference_mark_menu_click(ui);
        }
        clicked
    } else {
        false
    }
}

pub(in crate::app) fn reference_resume_status_style(
    status: &str,
    enabled: bool,
) -> (&'static str, egui::Color32) {
    if !enabled {
        return (
            match status {
                "lobby" => "Waiting for players",
                "active" => "In progress",
                _ => "Finished",
            },
            egui::Color32::from_rgb(112, 124, 137),
        );
    }
    match status {
        "lobby" => ("Waiting for players", egui::Color32::from_rgb(243, 190, 92)),
        "active" => ("In progress", egui::Color32::from_rgb(102, 224, 170)),
        _ => ("Finished", egui::Color32::from_rgb(167, 184, 202)),
    }
}

pub(in crate::app) fn reference_resume_empty_state(ui: &mut egui::Ui) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 72.0), egui::Sense::hover());
    ui.painter().rect(
        rect,
        egui::CornerRadius::same(7),
        egui::Color32::from_rgba_unmultiplied(31, 20, 18, 218),
        egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(190, 143, 94, 70)),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "No games linked to this device",
        egui::FontId::proportional(15.0),
        MUTED_TEXT,
    );
}

pub(in crate::app) fn reference_resume_recovery_toast(ui: &mut egui::Ui) {
    let frame = card_frame();
    let width = (ui.available_width() - frame.total_margin().sum().x).max(1.0);
    frame.show(ui, |ui| {
        ui.set_width(width);
        ui.vertical_centered(|ui| {
            ui.add(
                egui::Label::new(
                    egui::RichText::new("Played in another window or on another device?")
                        .size(17.0)
                        .color(CREAM),
                )
                .wrap(),
            );
            ui.add(
                egui::Label::new(
                    egui::RichText::new(
                        "Choose Recover Game, then enter the game code and your own private recovery code.",
                    )
                    .size(17.0)
                    .color(CREAM),
                )
                .wrap(),
            );
            ui.add(
                egui::Label::new(
                    egui::RichText::new("Each player has a different recovery code.")
                        .size(16.0)
                        .color(MUTED_TEXT),
                )
                .wrap(),
            );
        });
    });
}

pub(in crate::app) fn reference_menu_button_pair(
    ui: &mut egui::Ui,
    left_label: &str,
    left_enabled: bool,
    right_label: &str,
    right_enabled: bool,
    right_busy: bool,
) -> (bool, bool) {
    let (button_size, text_size, spacing) = reference_menu_button_pair_metrics(ui);
    let gap = 12.0;
    let (pair_size, pair_layout) = reference_menu_action_pair_layout(ui);
    let clicked = ui
        .allocate_ui_with_layout(pair_size, pair_layout, |ui| {
            ui.spacing_mut().item_spacing.x = gap;
            ui.spacing_mut().item_spacing.y = gap;
            let left_clicked = reference_menu_action_button(
                ui,
                left_label,
                left_enabled,
                false,
                false,
                button_size,
                text_size,
            );
            let right_clicked = reference_menu_action_button(
                ui,
                right_label,
                right_enabled && !right_busy,
                true,
                right_busy,
                button_size,
                text_size,
            );
            (left_clicked, right_clicked)
        })
        .inner;
    ui.add_space(spacing);
    clicked
}

pub(in crate::app) fn reference_menu_button_metrics(ui: &egui::Ui) -> (egui::Vec2, f32, f32) {
    (
        egui::vec2(MENU_ACTION_WIDTH.min(ui.available_width()), MENU_ACTION_HEIGHT),
        MENU_ACTION_TEXT_SIZE,
        0.0,
    )
}

pub(in crate::app) fn reference_menu_button_pair_metrics(ui: &egui::Ui) -> (egui::Vec2, f32, f32) {
    let (size, text_size, spacing) = reference_menu_button_metrics(ui);
    let width = size.x.min(((ui.available_width() - FORM_CARD_GAP) * 0.5).max(1.0));
    (egui::vec2(width, size.y), text_size, spacing)
}

pub(in crate::app) fn reference_menu_action_pair_layout(
    ui: &egui::Ui,
) -> (egui::Vec2, egui::Layout) {
    let (size, _, _) = reference_menu_button_pair_metrics(ui);
    (
        egui::vec2(size.x * 2.0 + FORM_CARD_GAP, size.y),
        egui::Layout::left_to_right(egui::Align::Center),
    )
}

pub(in crate::app) fn reference_menu_button_widget(
    ui: &mut egui::Ui,
    label: &str,
    enabled: bool,
    size: egui::Vec2,
    text_size: f32,
) -> bool {
    reference_menu_action_button(ui, label, enabled, false, false, size, text_size)
}

pub(in crate::app) fn reference_menu_action_button(
    ui: &mut egui::Ui,
    label: &str,
    enabled: bool,
    primary: bool,
    busy: bool,
    size: egui::Vec2,
    text_size: f32,
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
    // Long navigation labels must fit the same button width on smaller windows.
    let measured =
        ui.painter().layout_no_wrap(label.to_owned(), egui::FontId::proportional(text_size), CREAM);
    let padding = if busy {
        64.0
    } else {
        24.0
    };
    let text_size = text_size * ((size.x - padding).max(1.0) / measured.size().x).min(1.0);
    ui.painter().text(
        rect.center()
            + egui::vec2(
                if busy {
                    10.0
                } else {
                    0.0
                },
                0.0,
            ),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(text_size),
        if enabled {
            CREAM
        } else {
            egui::Color32::from_rgb(132, 145, 158)
        },
    );
    if busy {
        let spinner_size = 16.0;
        let spinner_rect = egui::Rect::from_center_size(
            egui::pos2(rect.left() + 22.0, rect.center().y),
            egui::vec2(spinner_size, spinner_size),
        );
        ui.put(spinner_rect, egui::Spinner::new().size(spinner_size));
    }
    if enabled {
        let clicked = response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
        if clicked {
            reference_mark_menu_click(ui);
        }
        clicked
    } else {
        false
    }
}

pub(in crate::app) fn reference_mark_menu_click(ui: &egui::Ui) {
    ui.ctx().data_mut(|data| data.insert_temp(egui::Id::new("augustus_menu_click"), true));
}

pub(in crate::app) fn reference_lobby_leave_button(ui: &mut egui::Ui, enabled: bool) -> bool {
    let (size, text_size, spacing) = reference_menu_button_pair_metrics(ui);
    let clicked = ui
        .allocate_ui_with_layout(size, egui::Layout::left_to_right(egui::Align::Center), |ui| {
            reference_menu_button_widget(ui, "Leave Lobby", enabled, size, text_size)
        })
        .inner;
    ui.add_space(spacing);
    clicked
}

pub(in crate::app) fn reference_join_game_code_field(
    ui: &mut egui::Ui,
    value: &mut String,
) -> egui::Response {
    reference_input_card(
        ui,
        "Game code",
        value,
        false,
        "Enter game code",
        "Enter the game code shared by the host to join their lobby.",
        None,
    )
}

pub(in crate::app) fn reference_recovery_code_field(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut String,
    private: bool,
    hint: &str,
) -> egui::Response {
    let tooltip = if private {
        "Each player has one private recovery code per game. Enter your own code, not another player's. Recovery does not change the code. A connected player cannot be recovered in another window."
    } else {
        "Enter the shared code of the game you want to recover. Find it in your saved codes or ask another player in that game."
    };
    reference_input_card(ui, label, value, private, hint, tooltip, None)
}

pub(in crate::app) fn reference_player_name_field(ui: &mut egui::Ui, display_name: &mut String) {
    if display_name.chars().count() > MAX_DISPLAY_NAME_CHARS {
        *display_name = display_name.chars().take(MAX_DISPLAY_NAME_CHARS).collect();
    }
    reference_input_card(
        ui,
        "Player name",
        display_name,
        false,
        "Enter player name",
        "Choose the name other players will see in this game.",
        Some(MAX_DISPLAY_NAME_CHARS),
    );
}

pub(in crate::app) fn reference_input_card(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut String,
    private: bool,
    hint: &str,
    tooltip: &str,
    char_limit: Option<usize>,
) -> egui::Response {
    form_option_card(ui, label, tooltip, |ui| {
        let mut editor = egui::TextEdit::singleline(value)
            .id_salt(label)
            .horizontal_align(egui::Align::Center)
            .vertical_align(egui::Align::Center)
            .interactive(ui.is_enabled())
            .font(egui::FontId::proportional(MENU_CONTROL_TEXT_SIZE))
            .password(private)
            .hint_text(egui::RichText::new(hint).size(MENU_CONTROL_TEXT_SIZE))
            .margin(egui::vec2(12.0, 6.0));
        if let Some(limit) = char_limit {
            editor = editor.char_limit(limit);
        }
        let response = ui.add_sized(egui::vec2(ui.available_width(), MENU_CONTROL_HEIGHT), editor);
        response.context_menu(|ui| {
            if ui.button("Paste").clicked() {
                request_menu_paste(ui, response.id);
                ui.close();
            }
        });
    })
}

#[cfg(test)]
#[path = "../../tests/unit/menu_reference.rs"]
mod tests;
