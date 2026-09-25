//! Main menu, game setup, lobby, and settings screens.

use super::*;

pub(super) fn main_menu(
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
                    ("Local Practice", AppState::PracticeSetup),
                    ("Settings", AppState::Settings),
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

pub(super) fn create_game(
    ui: &mut egui::Ui,
    draft: &mut MenuDraft,
    lobby: &mut LobbyPreview,
    next: &mut NextState<AppState>,
    sound: &MenuAudio,
    audio: &Audio,
    assets: &AssetServer,
) {
    let enter_pressed = menu_submit_pressed(ui);
    menu_form(ui, "augustus_create_form", "Create Game", |ui| {
        editable_form_card(
            ui,
            "Player name",
            &mut draft.display_name,
            "Enter player name",
            "Choose the name other players will see in this game.",
            Some(32),
        );
    });
    let (back, create) = menu_button_pair(
        ui,
        "Back",
        "Create Game",
        !draft.display_name.trim().is_empty(),
        sound,
        audio,
        assets,
    );
    if back {
        next.set(AppState::MainMenu);
    } else if !draft.display_name.trim().is_empty() && (create || enter_pressed) {
        lobby.reset(draft.display_name.trim(), generate_game_code());
        lobby.color_index = random_range(0..PLAYER_COLORS.len());
        next.set(AppState::Lobby);
    }
}

pub(super) fn practice_setup(
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

pub(super) fn join_game(
    ui: &mut egui::Ui,
    draft: &mut MenuDraft,
    lobby: &mut LobbyPreview,
    next: &mut NextState<AppState>,
    sound: &MenuAudio,
    audio: &Audio,
    assets: &AssetServer,
) {
    let enter_pressed = menu_submit_pressed(ui);
    menu_form(ui, "augustus_join_form", "Join Game", |ui| {
        editable_form_card(
            ui,
            "Player name",
            &mut draft.display_name,
            "Enter player name",
            "Choose the name other players will see in this game.",
            Some(32),
        );
        editable_form_card(
            ui,
            "Game code",
            &mut draft.join_code,
            "Enter game code",
            "Enter the game code shared by the host to join their lobby.",
            None,
        );
    });
    let can_join = !draft.display_name.trim().is_empty() && !draft.join_code.trim().is_empty();
    let (back, join) = menu_button_pair(ui, "Back", "Join", can_join, sound, audio, assets);
    if back {
        next.set(AppState::MainMenu);
    } else if can_join && (join || enter_pressed) {
        lobby.reset(draft.display_name.trim(), draft.join_code.trim().to_ascii_uppercase());
        next.set(AppState::Lobby);
    }
}

pub(super) fn resume_game(
    ui: &mut egui::Ui,
    next: &mut NextState<AppState>,
    sound: &MenuAudio,
    audio: &Audio,
    assets: &AssetServer,
) {
    menu_form(ui, "augustus_resume_form", "Resume Game", |ui| {
        form_option_card(
            ui,
            "Saved games",
            "Campaigns available to resume will appear here.",
            |ui| {
                ui.label(egui::RichText::new("No Augustus campaigns are saved yet.").size(16.0));
            },
        );
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

pub(super) fn lobby_code_card(ui: &mut egui::Ui, code: &str) -> bool {
    let frame = card_frame();
    let inner_width = (ui.available_width() - frame.total_margin().sum().x).max(1.0);
    frame
        .show(ui, |ui| {
            ui.set_min_width(inner_width);
            ui.set_max_width(inner_width);
            ui.spacing_mut().item_spacing.y = 0.0;
            let copied = code_card_heading(
                ui,
                "Game code",
                "Share this code to invite players to the lobby.",
                Some(code),
            );
            ui.add_space(FORM_CARD_GAP);
            let mut read_only_code = code;
            ui.add_sized(
                egui::vec2(inner_width, MENU_CONTROL_HEIGHT),
                egui::TextEdit::singleline(&mut read_only_code)
                    .id_salt("lobby_game_code")
                    .horizontal_align(egui::Align::Center)
                    .vertical_align(egui::Align::Center)
                    .font(egui::FontId::monospace(27.0))
                    .text_color(CREAM)
                    .desired_width(inner_width)
                    .margin(egui::vec2(0.0, 4.0))
                    .frame(egui::Frame::NONE),
            );
            copied
        })
        .inner
}

pub(super) fn show_lobby(
    ui: &mut egui::Ui,
    lobby: &mut LobbyPreview,
    next: &mut NextState<AppState>,
    loading: &mut LoadingSequence,
    game: &mut ActiveGame,
    sound: &MenuAudio,
    audio: &Audio,
    assets: &AssetServer,
) {
    menu_form(ui, "augustus_lobby_form", "Game Lobby", |ui| {
        if lobby_code_card(ui, &lobby.code) {
            play_click(sound, audio, assets);
        }
        form_option_card(
            ui,
            "Player color",
            "Choose the color used to identify your house on the map.",
            |ui| {
                ui.vertical_centered(|ui| {
                    let gap = 6.0;
                    let row_width =
                        (PLAYER_COLORS.len() as f32 * (34.0 + gap) - gap).min(ui.available_width());
                    ui.allocate_ui_with_layout(
                        egui::vec2(row_width, MENU_CONTROL_HEIGHT),
                        egui::Layout::left_to_right(egui::Align::Center).with_main_wrap(true),
                        |ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            for (index, color) in PLAYER_COLORS.iter().enumerate() {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::vec2(34.0, 34.0),
                                    egui::Sense::click(),
                                );
                                ui.painter().circle_filled(rect.center(), 11.0, *color);
                                if index == lobby.color_index || response.hovered() {
                                    ui.painter().circle_stroke(
                                        rect.center(),
                                        14.0,
                                        egui::Stroke::new(1.5, CREAM),
                                    );
                                }
                                if response
                                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    lobby.color_index = index;
                                    play_click(sound, audio, assets);
                                }
                            }
                        },
                    );
                });
            },
        );
        editable_form_card(
            ui,
            "Player name",
            &mut lobby.display_name,
            "Enter player name",
            "Choose the name other players will see in this game.",
            Some(32),
        );
        lobby_players_card(ui, lobby);
    });
    let (leave, start) = menu_button_pair(
        ui,
        "Leave Lobby",
        "Start Game",
        !lobby.display_name.trim().is_empty(),
        sound,
        audio,
        assets,
    );
    if leave {
        next.set(AppState::MainMenu);
    } else if start {
        *game = ActiveGame::LobbyPreview;
        loading.begin(AppState::EmptyScreen, next);
    }
}

pub(super) fn lobby_players_card(ui: &mut egui::Ui, lobby: &LobbyPreview) {
    let frame = card_frame();
    let inner_width = (ui.available_width() - frame.total_margin().sum().x).max(1.0);
    frame.show(ui, |ui| {
        ui.set_min_width(inner_width);
        ui.set_max_width(inner_width);
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.horizontal(|ui| {
                ui.set_min_height(24.0);
                ui.label(egui::RichText::new("PLAYERS").size(16.0).strong().color(GOLD));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("1 player").size(13.0).color(MUTED_TEXT));
                });
            });
            ui.add_space(FORM_CARD_GAP);
            ui.label(
                egui::RichText::new("Waiting for at least one more player…")
                    .size(14.0)
                    .color(MUTED_TEXT),
            );
            ui.add_space(FORM_CARD_GAP);

            let row_margin_x = 10.0;
            let row_inner_width = inner_width - row_margin_x * 2.0;
            let color_width = 26.0;
            let id_width = 38.0;
            let role_width = 56.0;
            let gap = 8.0;
            let name_width =
                (row_inner_width - color_width - id_width - role_width - gap * 3.0).max(0.0);
            egui::Frame::new()
                .fill(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 14))
                .inner_margin(egui::Margin::symmetric(row_margin_x as i8, 4))
                .show(ui, |ui| {
                    ui.set_min_width(row_inner_width);
                    ui.set_max_width(row_inner_width);
                    ui.spacing_mut().item_spacing.x = gap;
                    ui.horizontal(|ui| {
                        let color = PLAYER_COLORS[lobby.color_index];
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
                                egui::RichText::new("#01").size(14.0).monospace().color(GOLD),
                            ),
                        );
                        ui.allocate_ui_with_layout(
                            egui::vec2(name_width, 22.0),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                ui.set_min_size(egui::vec2(name_width, 22.0));
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&lobby.display_name)
                                            .size(16.0)
                                            .strong(),
                                    )
                                    .halign(egui::Align::LEFT)
                                    .truncate(),
                                );
                            },
                        );
                        ui.allocate_ui_with_layout(
                            egui::vec2(role_width, 22.0),
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label(
                                    egui::RichText::new("HOST")
                                        .size(12.0)
                                        .strong()
                                        .color(MUTED_TEXT),
                                );
                            },
                        );
                    });
                });
        });
    });
}

pub(super) fn settings_screen(
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

pub(super) fn game_menu(
    ui: &mut egui::Ui,
    game: ActiveGame,
    next: &mut NextState<AppState>,
    sound: &MenuAudio,
    audio: &Audio,
    assets: &AssetServer,
) {
    ui.spacing_mut().item_spacing.y = FORM_CARD_GAP;
    let size = menu_button_metrics(ui);
    let continue_with_enter = ui.input(|input| input.key_pressed(egui::Key::Enter));
    if menu_action_button(
        ui,
        "Continue",
        true,
        false,
        size,
        MENU_ACTION_TEXT_SIZE,
        sound,
        audio,
        assets,
    ) || continue_with_enter
    {
        next.set(game.screen());
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
    ) && !continue_with_enter
    {
        next.set(AppState::GameSettings);
    }
    if game == ActiveGame::LobbyPreview {
        menu_action_button(
            ui,
            "Save Game",
            false,
            false,
            size,
            MENU_ACTION_TEXT_SIZE,
            sound,
            audio,
            assets,
        );
        ui.label(
            egui::RichText::new("Saving is not available in this preview yet.")
                .size(15.0)
                .color(MUTED_TEXT),
        );
    }
    if menu_action_button(
        ui,
        "Exit to Main Menu",
        true,
        false,
        size,
        MENU_ACTION_TEXT_SIZE,
        sound,
        audio,
        assets,
    ) && !continue_with_enter
    {
        next.set(AppState::MainMenu);
    }
}

/// Keeps the existing Augustus audio options available while a game is paused.
pub(super) fn game_settings_screen(
    ui: &mut egui::Ui,
    sound: &mut MenuAudio,
    next: &mut NextState<AppState>,
    audio: &Audio,
    assets: &AssetServer,
) {
    menu_form(ui, "augustus_game_settings", "Settings", |ui| {
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
        next.set(AppState::GameMenu);
    }
}
