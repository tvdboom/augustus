//! Main menu, game setup, lobby, and settings screens.

use super::*;

pub(in crate::app) fn main_menu(
    ui: &mut egui::Ui,
    next: &mut NextState<AppState>,
    sound: &MenuAudio,
    audio: &Audio,
    assets: &AssetServer,
) {
    let available_height =
        (logical_content_rect(ui).bottom() - ui.next_widget_position().y - 96.0).max(80.0);
    ui.set_max_height(available_height);
    let size = menu_button_metrics(ui);
    let count = MAIN_MENU_ACTION_COUNT as f32;
    let gap = ((available_height - count * MENU_ACTION_HEIGHT) / (count - 1.0))
        .clamp(8.0, FORM_CARD_GAP)
        .floor();
    egui::ScrollArea::vertical()
        .id_salt("augustus_main_actions")
        .auto_shrink([false, true])
        .max_height(available_height)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = gap;
            ui.vertical_centered(|ui| {
                for (label, destination) in [
                    ("New Game", AppState::CreateGame),
                    ("Join Game", AppState::JoinGame),
                    ("Resume Game", AppState::ResumeGame),
                ] {
                    if menu_action_button(
                        ui,
                        label,
                        true,
                        false,
                        size,
                        MENU_ACTION_TEXT_SIZE,
                        sound,
                        audio,
                        assets,
                    ) {
                        next.set(destination);
                    }
                }
                #[cfg(debug_assertions)]
                if menu_action_button(
                    ui,
                    "Local Practice",
                    true,
                    false,
                    size,
                    MENU_ACTION_TEXT_SIZE,
                    sound,
                    audio,
                    assets,
                ) {
                    next.set(AppState::PracticeSetup);
                }
                if menu_action_button(
                    ui,
                    "Settings",
                    true,
                    false,
                    size,
                    MENU_ACTION_TEXT_SIZE,
                    sound,
                    audio,
                    assets,
                ) {
                    next.set(AppState::Settings);
                }
                #[cfg(not(target_arch = "wasm32"))]
                if menu_action_button(
                    ui,
                    "Quit",
                    true,
                    false,
                    size,
                    MENU_ACTION_TEXT_SIZE,
                    sound,
                    audio,
                    assets,
                ) {
                    std::process::exit(0);
                }
            });
        });
}

pub(in crate::app) fn create_game(
    ui: &mut egui::Ui,
    draft: &mut MenuDraft,
    online: &mut online::OnlineClient,
    next: &mut NextState<AppState>,
    _sound: &MenuAudio,
    _audio: &Audio,
    _assets: &AssetServer,
) {
    let enter_pressed = menu_submit_pressed(ui);
    menu_form(ui, "augustus_create_form", "Create Game", |ui| {
        ui.add_enabled_ui(!online.busy(), |ui| {
            reference_player_name_field(ui, &mut draft.display_name)
        });
    });
    let can_create = !online.busy() && valid_player_name(&draft.display_name);
    let (back, create) =
        reference_menu_button_pair(ui, "Back", true, "Create Game", can_create, false);
    if back {
        next.set(AppState::MainMenu);
    } else if can_create && (create || enter_pressed) {
        online.request(
            online::Operation::Create,
            serde_json::json!({"p_name":draft.display_name.trim(),"p_color":0}),
        );
    }
}

pub(in crate::app) fn practice_setup(
    ui: &mut egui::Ui,
    practice: &mut LocalPractice,
    ownership: &mut ProvinceOwnership,
    next: &mut NextState<AppState>,
    loading: &mut LoadingSequence,
    game: &mut ActiveGame,
    sound: &MenuAudio,
    audio: &Audio,
    assets: &AssetServer,
) {
    let enter_pressed = menu_submit_pressed(ui);
    menu_form(ui, "augustus_practice_setup", "Local Practice", |ui| {
        form_option_card(
            ui,
            "Local players",
            "Choose how many local players take part in this practice game.",
            |ui| {
                let gap = 8.0;
                let row_width = ui.available_width();
                let button_width = ((row_width - gap * 3.0) / 4.0).max(1.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(row_width, MENU_CONTROL_HEIGHT),
                    egui::Layout::left_to_right(egui::Align::Min),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for player_count in 1..=4 {
                            let selected = practice.player_count == player_count;
                            let (rect, response) = ui.allocate_exact_size(
                                egui::vec2(button_width, MENU_CONTROL_HEIGHT),
                                egui::Sense::click(),
                            );
                            ui.painter().rect(
                                rect,
                                6.0,
                                if response.hovered() {
                                    egui::Color32::from_rgb(72, 38, 32)
                                } else if selected {
                                    egui::Color32::from_rgb(107, 49, 38)
                                } else {
                                    egui::Color32::from_rgba_unmultiplied(37, 23, 21, 228)
                                },
                                egui::Stroke::new(
                                    1.0,
                                    if selected {
                                        egui::Color32::from_rgba_unmultiplied(220, 170, 105, 145)
                                    } else {
                                        egui::Color32::from_rgba_unmultiplied(190, 145, 97, 92)
                                    },
                                ),
                                egui::StrokeKind::Inside,
                            );
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                player_count.to_string(),
                                egui::FontId::proportional(MENU_CONTROL_TEXT_SIZE),
                                CREAM,
                            );
                            if response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                practice.player_count = player_count;
                                play_click(sound, audio, assets);
                            }
                        }
                    },
                );
            },
        );
        form_option_card(
            ui,
            "Player color",
            "Choose Player 1's house color. Other local players use the next distinct colors.",
            |ui| {
                if player_color_picker(ui, &mut practice.color_index) {
                    play_click(sound, audio, assets);
                }
            },
        );
    });
    let (back, start) = menu_button_pair(ui, "Back", "Start Practice", true, sound, audio, assets);
    if back {
        next.set(AppState::MainMenu);
    } else if start || enter_pressed {
        practice.start_new_game(ownership);
        *game = ActiveGame::LocalPractice;
        loading.begin(AppState::Map, next);
    }
}

pub(in crate::app) fn join_game(
    ui: &mut egui::Ui,
    draft: &mut MenuDraft,
    online: &mut online::OnlineClient,
    next: &mut NextState<AppState>,
    _sound: &MenuAudio,
    _audio: &Audio,
    _assets: &AssetServer,
) {
    let enter_pressed = menu_submit_pressed(ui);
    menu_form(ui, "augustus_join_form", "Join Game", |ui| {
        ui.add_enabled_ui(!online.busy(), |ui| {
            reference_player_name_field(ui, &mut draft.display_name);
            reference_join_game_code_field(ui, &mut draft.join_code);
        });
    });
    let can_join = !online.busy()
        && valid_player_name(&draft.display_name)
        && crate::multiplayer::lobby::valid_game_code(&draft.join_code);
    let (back, join) = reference_menu_button_pair(ui, "Back", true, "Join", can_join, false);
    if back {
        next.set(AppState::MainMenu);
    } else if can_join && (join || enter_pressed) {
        online.request(online::Operation::Join,serde_json::json!({"p_name":draft.display_name.trim(),"p_code":draft.join_code.trim().to_ascii_uppercase()}));
    }
}

pub(in crate::app) fn resume_game(
    ui: &mut egui::Ui,
    draft: &mut MenuDraft,
    online: &mut online::OnlineClient,
    next: &mut NextState<AppState>,
    _sound: &MenuAudio,
    _audio: &Audio,
    _assets: &AssetServer,
) {
    menu_form(ui, "augustus_resume_content", "Resume Game", |ui| {
        let mut games = online.games.iter().filter(|game| game.status != "lobby").peekable();
        if games.peek().is_none() {
            reference_resume_empty_state(ui);
            reference_resume_recovery_toast(ui);
        } else {
            let mut selected = None;
            for game in games {
                if reference_resume_game_card(ui, game, !online.busy()) {
                    selected = Some(game.id.clone());
                }
            }
            if let Some(id) = selected {
                online.request(online::Operation::Open, serde_json::json!({"p_game_id":id}));
            }
        }
    });
    let (back, refresh, recover) =
        reference_resume_action_buttons(ui, online.busy(), online.list_pending());
    if back {
        next.set(AppState::MainMenu);
    }
    if refresh {
        online.request(online::Operation::List, serde_json::json!({}));
    }
    if recover {
        next.set(AppState::RecoverPlayer);
    }
    let _ = draft;
}

pub(in crate::app) fn recover_game(
    ui: &mut egui::Ui,
    draft: &mut MenuDraft,
    online: &mut online::OnlineClient,
    next: &mut NextState<AppState>,
) {
    let enter_pressed = menu_submit_pressed(ui);
    menu_form(ui, "augustus_recovery_form", "Recover Game", |ui| {
        ui.add_enabled_ui(!online.busy(), |ui| {
            reference_recovery_code_field(
                ui,
                "Game code",
                &mut draft.join_code,
                false,
                "Enter game code",
            );
            reference_recovery_code_field(
                ui,
                "Recovery code",
                &mut draft.recovery_code,
                true,
                "Enter recovery code",
            );
        });
    });
    let can_recover = !online.busy()
        && crate::multiplayer::lobby::valid_game_code(&draft.join_code)
        && !draft.recovery_code.trim().is_empty();
    let (back, recover) = reference_menu_button_pair(
        ui,
        "Back",
        !online.busy(),
        "Recover Game",
        can_recover,
        online.busy(),
    );
    if back {
        next.set(AppState::ResumeGame);
    } else if can_recover && (recover || enter_pressed) {
        online.request(online::Operation::Recover, serde_json::json!({"p_code":draft.join_code.trim(),"p_recovery_code":draft.recovery_code.trim()}));
    }
}

pub(in crate::app) fn lobby_primary_enabled(online: &online::OnlineClient) -> bool {
    !online.foreground_busy()
        && online.record.as_ref().is_some_and(|r| {
            r.player == 0
                && if online.reconnecting() {
                    r.status == "active" && r.members.iter().all(|m| m.connected)
                } else {
                    r.status == "lobby"
                        && (1..=4).contains(&r.members.len())
                        && r.members.iter().all(|m| m.connected)
                }
        })
}

pub(in crate::app) fn lobby_code_card(ui: &mut egui::Ui, label: &str, code: &str, prominent: bool) {
    let frame = card_frame();
    let card_width = ui.available_width().min(MENU_CONTENT_WIDTH);
    let tooltip = if label.eq_ignore_ascii_case("Game code") {
        "Share this code to invite players before the game starts. It also identifies the saved game when recovering with your private recovery code."
    } else {
        "Keep this code private. Enter it with the game code to restore your player and host or player role. The host resumes once everyone reconnects; finished games open for viewing."
    };
    frame.show(ui, |ui| {
        let inner_width = (card_width - frame.total_margin().sum().x).max(1.0);
        ui.set_min_width(inner_width);
        ui.set_max_width(inner_width);
        ui.spacing_mut().item_spacing.y = 0.0;
        if code_card_heading(ui, label, tooltip, Some(code)) {
            reference_mark_menu_click(ui);
        }
        ui.add_space(FORM_CARD_GAP);
        let font_size = if prominent {
            27.0
        } else {
            16.0
        };
        let mut read_only_code = code;
        let response = ui.add_sized(
            egui::vec2(inner_width, MENU_CONTROL_HEIGHT),
            egui::TextEdit::singleline(&mut read_only_code)
                .id_salt(("lobby_code", label))
                .horizontal_align(egui::Align::Center)
                .vertical_align(egui::Align::Center)
                .font(egui::FontId::monospace(font_size))
                .text_color(CREAM)
                .desired_width(inner_width)
                .margin(egui::vec2(0.0, 4.0))
                .frame(egui::Frame::NONE),
        );
        let _ = response;
    });
}

pub(in crate::app) fn show_lobby(
    ui: &mut egui::Ui,
    lobby: &mut LobbyPreview,
    online: &mut online::OnlineClient,
    _sound: &MenuAudio,
    _audio: &Audio,
    _assets: &AssetServer,
) {
    let enter_pressed = menu_submit_pressed(ui);
    let Some(record) = online.record.as_ref() else {
        ui.label("No lobby is selected.");
        return;
    };
    let reconnecting = online.reconnecting();
    let is_host = record.player == 0;
    let title = if reconnecting {
        "Resume Game"
    } else {
        "Game Lobby"
    };
    menu_form(ui, "augustus_lobby_content", title, |ui| {
        lobby_code_card(ui, "Game code", &lobby.code, true);
        if let Some(record) = &online.record {
            lobby_code_card(ui, "Recovery code", &record.recovery_code, false);
        }
        if !reconnecting {
            form_option_card(ui, "Player color", "Choose the color used to identify your empire. Colors held by other players are unavailable.", |ui| {
                let record = online.record.as_ref().expect("selected lobby");
                let unavailable: Vec<_> = record.members.iter().filter(|m| m.player != record.player).map(|m| m.color).collect();
                let mut selected = lobby.color_index;
                if lobby_color_picker(ui, &mut selected, &unavailable, online.foreground_busy()) {
                    reference_mark_menu_click(ui);
                    online.request(online::Operation::Profile, serde_json::json!({"p_name":lobby.display_name.trim(),"p_color":selected}));
                }
            });
        }
        lobby_players_card(ui, online);
    });
    let can_continue = lobby_primary_enabled(online);
    let (leave, start) = if is_host {
        reference_menu_button_pair(
            ui,
            "Leave Lobby",
            true,
            if reconnecting {
                "Resume Game"
            } else {
                "Start Game"
            },
            can_continue,
            false,
        )
    } else {
        (reference_lobby_leave_button(ui, true), false)
    };
    if leave && !online.foreground_busy() {
        online.request(online::Operation::Leave, serde_json::json!({}));
    } else if can_continue && (start || enter_pressed) {
        if reconnecting {
            online.resume();
        } else if let Err(error) = online.start() {
            online.report_error(error);
        }
    }
}

pub(in crate::app) fn lobby_players_card(ui: &mut egui::Ui, online: &online::OnlineClient) {
    let Some(game) = online.record.as_ref() else {
        return;
    };
    let reconnecting = online.reconnecting();
    let is_host = game.player == 0;
    let card_width = ui.available_width().min(MENU_CONTENT_WIDTH);
    let frame = card_frame();
    frame.show(ui, |ui| {
        let inner_width = (card_width - frame.total_margin().sum().x).max(1.0);
        ui.set_min_width(inner_width);
        ui.set_max_width(inner_width);
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.horizontal(|ui| {
            ui.set_min_height(24.0);
            ui.label(egui::RichText::new("PLAYERS").size(16.0).strong().color(GOLD));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{} {}",
                        game.members.len(),
                        if game.members.len() == 1 {
                            "player"
                        } else {
                            "players"
                        }
                    ))
                    .size(13.0)
                    .color(MUTED_TEXT),
                );
            });
        });
        ui.add_space(FORM_CARD_GAP);
        if !reconnecting {
            ui.label(
                egui::RichText::new(if !is_host {
                    "Waiting for the host to start..."
                } else if game.members.iter().any(|m| !m.connected) {
                    "Waiting for all players to connect…"
                } else if game.members.len() == 1 {
                    "Ready to start solo — more players may still join."
                } else {
                    "Ready to start — more players may still join."
                })
                .size(14.0)
                .color(MUTED_TEXT),
            );
        }
        ui.add_space(FORM_CARD_GAP);

        ui.spacing_mut().item_spacing.y = 0.0;
        for (index, member) in game.members.iter().enumerate() {
            let row_margin_x = 10.0;
            let row_inner_width = inner_width - row_margin_x * 2.0;
            let color_width = 26.0;
            let id_width = 38.0;
            let role_width = 56.0;
            let status_width = if reconnecting {
                118.0
            } else {
                0.0
            };
            let gap = 8.0;
            let gap_count = 3.0 + u8::from(reconnecting) as f32;
            let name_width = (row_inner_width
                - color_width
                - id_width
                - status_width
                - role_width
                - gap * gap_count)
                .max(0.0);
            egui::Frame::new()
                .fill(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 14))
                .inner_margin(egui::Margin::symmetric(row_margin_x as i8, 4))
                .show(ui, |ui| {
                    ui.set_min_width(row_inner_width);
                    ui.set_max_width(row_inner_width);
                    ui.spacing_mut().item_spacing.x = gap;
                    ui.horizontal(|ui| {
                        let color = PLAYER_COLORS[member.color];
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(color_width, 26.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().circle_filled(rect.center(), 6.0, color);
                        ui.painter().circle_stroke(
                            rect.center(),
                            6.0,
                            egui::Stroke::new(1.0, egui::Color32::from_white_alpha(180)),
                        );
                        ui.add_sized(
                            [id_width, 22.0],
                            egui::Label::new(
                                egui::RichText::new(format!("#{:02}", member.player + 1))
                                    .size(14.0)
                                    .monospace()
                                    .color(GOLD),
                            ),
                        );
                        ui.allocate_ui_with_layout(
                            egui::vec2(name_width, 22.0),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                ui.set_min_size(egui::vec2(name_width, 22.0));
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&member.display_name)
                                            .size(16.0)
                                            .strong(),
                                    )
                                    .halign(egui::Align::LEFT)
                                    .truncate(),
                                );
                            },
                        );
                        if reconnecting {
                            let (status, status_color) = if member.connected {
                                ("Connected", egui::Color32::from_rgb(102, 224, 170))
                            } else {
                                ("Not connected", egui::Color32::from_rgb(243, 190, 92))
                            };
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(status_width, 22.0),
                                egui::Sense::hover(),
                            );
                            ui.painter().circle_filled(
                                egui::pos2(rect.left() + 3.5, rect.center().y),
                                3.5,
                                status_color,
                            );
                            ui.painter().text(
                                egui::pos2(rect.left() + 15.0, rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                status,
                                egui::FontId::proportional(13.0),
                                status_color,
                            );
                        }
                        ui.allocate_ui_with_layout(
                            egui::vec2(role_width, 22.0),
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label(
                                    egui::RichText::new(if member.player == 0 {
                                        "HOST"
                                    } else {
                                        "CLIENT"
                                    })
                                    .size(12.0)
                                    .strong()
                                    .color(MUTED_TEXT),
                                );
                            },
                        );
                    });
                });
            if index + 1 < game.members.len() {
                ui.add_space(3.0);
            }
        }
    });
}

pub(in crate::app) fn settings_screen(
    ui: &mut egui::Ui,
    sound: &mut MenuAudio,
    next: &mut NextState<AppState>,
    audio: &Audio,
    assets: &AssetServer,
) {
    menu_form(ui, "augustus_settings_form", "Settings", |ui| {
        audio_choice_row(ui, sound, audio, assets);
    });
    if menu_action_button(
        ui,
        "Back",
        true,
        false,
        menu_button_metrics(ui),
        MENU_ACTION_TEXT_SIZE,
        sound,
        audio,
        assets,
    ) {
        next.set(AppState::MainMenu);
    }
}

pub(in crate::app) fn game_menu(
    ui: &mut egui::Ui,
    game: ActiveGame,
    online: &mut online::OnlineClient,
    next: &mut NextState<AppState>,
    _sound: &MenuAudio,
    _audio: &Audio,
    _assets: &AssetServer,
) {
    ui.spacing_mut().item_spacing.y = FORM_CARD_GAP;
    let (size, text_size, _) = reference_menu_button_metrics(ui);
    if reference_menu_button_widget(ui, "Continue", true, size, text_size) {
        next.set(game.screen());
    }
    if game == ActiveGame::Online
        && online.can_save()
        && reference_menu_button_widget(ui, "Save Game", !online.busy(), size, text_size)
    {
        online.save();
    }
    if reference_menu_button_widget(ui, "Settings", true, size, text_size) {
        next.set(AppState::GameSettings);
    }
    if reference_menu_button_widget(ui, "Return to Main Menu", true, size, text_size) {
        if game == ActiveGame::Online {
            online.leave();
        } else {
            next.set(AppState::MainMenu);
        }
    }
}

/// Keeps the existing Augustus audio options available while a game is paused.
pub(in crate::app) fn game_settings_screen(
    ui: &mut egui::Ui,
    _sound: &mut MenuAudio,
    next: &mut NextState<AppState>,
    _audio: &Audio,
    _assets: &AssetServer,
) {
    ui.spacing_mut().item_spacing.y = FORM_CARD_GAP;
    menu_form(ui, "augustus_game_settings", "Settings", |ui| {
        audio_choice_row(ui, _sound, _audio, _assets);
    });
    let (size, text_size, _) = reference_menu_button_metrics(ui);
    if reference_menu_button_widget(ui, "Back", true, size, text_size) {
        next.set(AppState::GameMenu);
    }
}

/// One shared house-color control keeps local setup aligned with the existing lobby.
fn player_color_picker(ui: &mut egui::Ui, selected: &mut usize) -> bool {
    lobby_color_picker(ui, selected, &[], false)
}

fn lobby_color_picker(
    ui: &mut egui::Ui,
    selected: &mut usize,
    unavailable: &[usize],
    busy: bool,
) -> bool {
    let mut changed = false;
    ui.vertical_centered(|ui| {
        let row_width = (PLAYER_COLORS.len() as f32 * 40.0 - 6.0).min(ui.available_width());
        ui.allocate_ui_with_layout(
            egui::vec2(row_width, MENU_CONTROL_HEIGHT),
            egui::Layout::left_to_right(egui::Align::Center).with_main_wrap(true),
            |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for (index, color) in PLAYER_COLORS.iter().enumerate() {
                    let available = !unavailable.contains(&index);
                    let (rect, response) =
                        ui.allocate_exact_size(egui::vec2(34.0, 34.0), egui::Sense::click());
                    ui.painter().circle_filled(
                        rect.center(),
                        11.0,
                        if available {
                            *color
                        } else {
                            color.gamma_multiply(0.32)
                        },
                    );
                    if index == *selected || response.hovered() {
                        ui.painter().circle_stroke(
                            rect.center(),
                            14.0,
                            egui::Stroke::new(1.5, CREAM),
                        );
                    }
                    if !busy
                        && available
                        && index != *selected
                        && response
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text(format!("{} house color", PLAYER_COLOR_NAMES[index]))
                            .clicked()
                    {
                        *selected = index;
                        changed = true;
                    }
                }
            },
        );
    });
    changed
}

#[cfg(test)]
#[path = "../../tests/unit/lobby_controls.rs"]
mod lobby_tests;
