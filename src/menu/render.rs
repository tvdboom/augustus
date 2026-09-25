//! Menu layer, loading overlay, and shared presentation.

use super::*;

pub(super) fn draw_menu(
    mut contexts: EguiContexts,
    mut clipboard: ResMut<EguiClipboard>,
    mut style_initialized: Local<bool>,
    state: Res<State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
    mut draft: ResMut<MenuDraft>,
    mut lobby: ResMut<LobbyPreview>,
    mut loading: ResMut<LoadingSequence>,
    mut game: ResMut<ActiveGame>,
    mut practice_setup_params: PracticeSetupParams,
    mut map_view: ResMut<MapView>,
    wallpapers: Res<LoadingWallpapers>,
    #[cfg(target_arch = "wasm32")] discovery: Res<LoadingWallpaperDiscovery>,
    mut menu_audio: ResMut<MenuAudio>,
    audio: Res<Audio>,
    assets: Res<AssetServer>,
) {
    let Ok(context) = contexts.ctx_mut() else {
        return;
    };
    apply_pending_menu_paste(context, &mut clipboard);
    if !*style_initialized {
        context.set_global_style(augustus_ui_style());
        context.add_font(FontInsert::new(
            "firasans",
            egui::FontData::from_static(include_bytes!("../../assets/fonts/FiraSans-Bold.ttf")),
            vec![InsertFontFamily {
                family: egui::FontFamily::Proportional,
                priority: FontPriority::Highest,
            }],
        ));
        *style_initialized = true;
    }

    let viewport = context.content_rect();
    let scale = viewport_ui_scale(viewport.size());
    let menu_size = viewport.size() / scale;
    let current = *state.get();
    if current == AppState::Loading {
        context.request_repaint();
        if loading.map_reveal_progress.is_some() {
            return;
        }
        let item = if loading.destination == AppState::Map && !map_view.is_loaded() {
            let item = map_view.load_progress().item;
            map_view.load_next(context);
            item
        } else if loading.destination == AppState::EmptyScreen {
            "Local game preview".to_string()
        } else {
            "Map ready".to_string()
        };
        let map_progress = map_view.load_progress();
        let wallpapers_ready = !wallpapers.0.is_empty() && {
            #[cfg(target_arch = "wasm32")]
            {
                discovery.complete
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                true
            }
        };
        let fraction = if loading.destination == AppState::Map {
            (map_progress.completed as f32 + f32::from(wallpapers_ready))
                / (map_progress.total + 1) as f32
        } else {
            f32::from(wallpapers_ready)
        };
        let label = if !wallpapers_ready
            && (loading.destination != AppState::Map || map_view.is_loaded())
        {
            "Loading wallpapers"
        } else {
            &item
        };
        draw_loading_progress(context, viewport.min, menu_size, scale, fraction, label);
        return;
    }
    let preferred_width = if current == AppState::ResumeGame {
        RESUME_MENU_WIDTH
    } else {
        MENU_CONTENT_WIDTH
    };
    let content_width = preferred_width.min((menu_size.x - 32.0).max(240.0));
    let is_main = current == AppState::MainMenu;
    let is_game_overlay = matches!(current, AppState::GameMenu | AppState::GameSettings);
    let (pivot, content_y) = if is_main {
        (egui::Align2::CENTER_TOP, main_menu_top(context, menu_size))
    } else {
        let footer_space = if current == AppState::CreateGame {
            48.0
        } else {
            96.0
        };
        (egui::Align2::CENTER_CENTER, (menu_size.y - footer_space).max(0.0) * 0.5)
    };
    let content_id = egui::Id::new(("augustus_menu_content", current));
    set_menu_layer_scale(context, content_id, egui::Order::Middle, viewport.min, scale);
    egui::Area::new(content_id)
        .pivot(pivot)
        .fixed_pos(egui::pos2(menu_size.x * 0.5, content_y))
        .constrain(false)
        .order(egui::Order::Middle)
        .show(context, |ui| {
            ui.set_clip_rect(logical_content_rect(ui));
            apply_menu_style(ui);
            ui.set_width(content_width);
            ui.vertical_centered(|ui| match current {
                AppState::MainMenu => main_menu(ui, &mut next, &menu_audio, &audio, &assets),
                AppState::CreateGame => {
                    create_game(ui, &mut draft, &mut lobby, &mut next, &menu_audio, &audio, &assets)
                },
                AppState::PracticeSetup => practice_setup(
                    ui,
                    &mut practice_setup_params.practice,
                    &mut practice_setup_params.ownership,
                    &mut next,
                    &mut loading,
                    &mut game,
                    &menu_audio,
                    &audio,
                    &assets,
                ),
                AppState::JoinGame => {
                    join_game(ui, &mut draft, &mut lobby, &mut next, &menu_audio, &audio, &assets)
                },
                AppState::ResumeGame => resume_game(ui, &mut next, &menu_audio, &audio, &assets),
                AppState::Lobby => show_lobby(
                    ui,
                    &mut lobby,
                    &mut next,
                    &mut loading,
                    &mut game,
                    &menu_audio,
                    &audio,
                    &assets,
                ),
                AppState::Settings => {
                    settings_screen(ui, &mut menu_audio, &mut next, &audio, &assets)
                },
                AppState::GameMenu => game_menu(ui, *game, &mut next, &menu_audio, &audio, &assets),
                AppState::GameSettings => {
                    game_settings_screen(ui, &mut menu_audio, &mut next, &audio, &assets)
                },
                AppState::Loading => {},
                AppState::Map | AppState::EmptyScreen => {},
            });
        });

    if is_main {
        let title = main_menu_title(context, menu_size);
        let title_id = egui::Id::new("augustus_main_title");
        set_menu_layer_scale(context, title_id, egui::Order::Middle, viewport.min, scale);
        egui::Area::new(title_id)
            .pivot(egui::Align2::CENTER_CENTER)
            .fixed_pos(egui::pos2(menu_size.x * 0.5, menu_size.y * 0.17))
            .constrain(false)
            .interactable(false)
            .order(egui::Order::Middle)
            .show(context, |ui| {
                ui.set_clip_rect(logical_content_rect(ui));
                let (rect, _) = ui.allocate_exact_size(title.size(), egui::Sense::hover());
                ui.painter().galley(rect.min, title, CREAM);
            });
    }

    if is_game_overlay {
        return;
    }

    let footer_id = egui::Id::new("augustus_menu_footer");
    set_menu_layer_scale(context, footer_id, egui::Order::Foreground, viewport.min, scale);
    egui::Area::new(footer_id)
        .pivot(egui::Align2::RIGHT_BOTTOM)
        .fixed_pos(egui::pos2(menu_size.x - 24.0, menu_size.y - 18.0))
        .constrain(false)
        .order(egui::Order::Foreground)
        .show(context, |ui| {
            ui.set_min_width(220.0);
            ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
                connection_status_badge(ui);
                ui.label(egui::RichText::new("Created by Mavs").weak());
            });
        });
}

pub(super) fn draw_loading_reveal(
    mut contexts: EguiContexts,
    sequence: Res<LoadingSequence>,
    wallpapers: Res<LoadingWallpapers>,
    images: Res<Assets<Image>>,
) {
    let (Some(progress), Some(wallpaper)) =
        (sequence.map_reveal_progress, wallpapers.0.get(sequence.slide_index))
    else {
        return;
    };
    let Some(image) = images.get(wallpaper) else {
        return;
    };
    let texture = contexts.add_image(EguiTextureHandle::Weak(wallpaper.id()));
    let Ok(context) = contexts.ctx_mut() else {
        return;
    };
    let viewport = context.content_rect();
    let image_size = image.size_f32();
    let crop = cover_source_rect(image_size, Vec2::new(viewport.width(), viewport.height()));
    let uv = egui::Rect::from_min_max(
        egui::pos2(crop.min.x / image_size.x, crop.min.y / image_size.y),
        egui::pos2(crop.max.x / image_size.x, crop.max.y / image_size.y),
    );
    let alpha = ((1.0 - progress) * 255.0).round() as u8;
    // Egui paints the map and HUD above Bevy sprites, so the exiting wallpaper covers both.
    egui::Area::new(egui::Id::new("augustus_loading_map_reveal"))
        .fixed_pos(viewport.min)
        .constrain(false)
        .order(egui::Order::Tooltip)
        .show(context, |ui| {
            let (rect, _) = ui.allocate_exact_size(viewport.size(), egui::Sense::click_and_drag());
            ui.painter().image(texture, rect, uv, egui::Color32::from_white_alpha(alpha));
        });
}

pub(super) fn draw_loading_progress(
    context: &egui::Context,
    viewport_origin: egui::Pos2,
    menu_size: egui::Vec2,
    scale: f32,
    fraction: f32,
    item: &str,
) {
    let id = egui::Id::new("augustus_loading_progress");
    set_menu_layer_scale(context, id, egui::Order::Foreground, viewport_origin, scale);
    egui::Area::new(id)
        .pivot(egui::Align2::CENTER_BOTTOM)
        .fixed_pos(egui::pos2(menu_size.x * 0.5, menu_size.y - 34.0))
        .constrain(false)
        .interactable(false)
        .order(egui::Order::Foreground)
        .show(context, |ui| {
            let width = (menu_size.x - 48.0).clamp(1.0, 780.0);
            ui.set_width(width);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(item).size(14.0).color(CREAM));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!("{}%", (fraction * 100.0).round() as u32))
                            .size(14.0)
                            .color(MUTED_TEXT),
                    );
                });
            });
            ui.add_space(3.0);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 12.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 4.0, egui::Color32::from_rgb(72, 43, 37));
            if fraction > 0.0 {
                let fill = egui::Rect::from_min_size(
                    rect.min,
                    egui::vec2(rect.width() * fraction.clamp(0.0, 1.0), rect.height()),
                );
                ui.painter().rect_filled(fill, 4.0, GOLD);
            }
        });
}

pub(super) fn augustus_ui_style() -> egui::Style {
    let mut style = egui::Style {
        text_styles: [
            (egui::TextStyle::Small, egui::FontId::proportional(18.0)),
            (egui::TextStyle::Body, egui::FontId::proportional(23.0)),
            (egui::TextStyle::Button, egui::FontId::proportional(20.0)),
            (egui::TextStyle::Heading, egui::FontId::proportional(40.0)),
            (egui::TextStyle::Monospace, egui::FontId::monospace(30.0)),
        ]
        .into(),
        ..Default::default()
    };
    style.spacing.item_spacing = egui::Vec2::splat(18.0);
    style.spacing.window_margin = egui::Margin::same(12);
    style.spacing.menu_margin = egui::Margin::same(12);
    style.spacing.button_padding = egui::vec2(12.0, 10.0);
    style.spacing.indent = 18.0;
    style.spacing.interact_size = egui::vec2(40.0, 20.0);
    style.spacing.slider_width = 200.0;
    style.spacing.combo_width = 130.0;
    style.spacing.text_edit_width = 280.0;
    style.spacing.icon_width = 14.0;
    style.spacing.icon_width_inner = 8.0;
    style.spacing.icon_spacing = 6.0;
    style.spacing.tooltip_width = 600.0;
    style.spacing.scroll.bar_width = 14.0;
    style.spacing.scroll.handle_min_length = 12.0;
    style.spacing.scroll.bar_inner_margin = 4.0;
    style.spacing.scroll.bar_outer_margin = 0.0;
    style.interaction.show_tooltips_only_when_still = true;
    style.interaction.selectable_labels = false;

    let visuals = &mut style.visuals;
    visuals.dark_mode = true;
    visuals.override_text_color = Some(CREAM);
    visuals.selection.bg_fill = egui::Color32::from_rgb(136, 65, 46);
    visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(31, 20, 18));
    visuals.hyperlink_color = GOLD;
    visuals.panel_fill = egui::Color32::from_rgb(31, 20, 18);
    visuals.faint_bg_color = egui::Color32::from_rgb(72, 43, 37);
    visuals.extreme_bg_color = egui::Color32::from_rgb(72, 43, 37);
    visuals.code_bg_color = egui::Color32::from_rgb(72, 43, 37);
    visuals.warn_fg_color = egui::Color32::from_rgb(255, 209, 111);
    visuals.error_fg_color = egui::Color32::from_rgb(255, 128, 109);
    visuals.window_corner_radius = egui::CornerRadius::same(6);
    visuals.window_fill = egui::Color32::from_rgb(31, 20, 18);
    visuals.window_stroke = egui::Stroke::new(1.0, GOLD);
    visuals.menu_corner_radius = egui::CornerRadius::same(6);
    visuals.button_frame = true;
    visuals.collapsing_header_frame = true;
    visuals.indent_has_left_vline = true;
    visuals.striped = true;
    visuals.slider_trailing_fill = true;
    style.animation_time = 0.083_333_336;
    style.explanation_tooltips = false;
    style
}

pub(super) fn viewport_ui_scale(viewport: egui::Vec2) -> f32 {
    let relative =
        (viewport.x / WINDOW_WIDTH as f32).min(viewport.y / WINDOW_HEIGHT as f32).max(0.0);
    relative.sqrt().clamp(0.5, 1.25)
}

pub(super) fn map_corner_panel_rect(
    screen: egui::Rect,
    scale: f32,
    size: egui::Vec2,
) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(
            screen.left() + 60.0 * scale,
            screen.bottom() - screen.height() * MAP_CORNER_PANEL_BOTTOM_FRACTION - size.y,
        ),
        size,
    )
}

pub(super) fn set_menu_layer_scale(
    context: &egui::Context,
    id: egui::Id,
    order: egui::Order,
    fixed_point: egui::Pos2,
    scale: f32,
) {
    let translation = fixed_point.to_vec2() * (1.0 - scale);
    context.set_transform_layer(
        egui::LayerId::new(order, id),
        egui::emath::TSTransform::new(translation, scale),
    );
}

pub(super) fn logical_content_rect(ui: &egui::Ui) -> egui::Rect {
    let transform = ui
        .ctx()
        .layer_transform_to_global(ui.layer_id())
        .unwrap_or(egui::emath::TSTransform::IDENTITY);
    transform.inverse().mul_rect(ui.ctx().content_rect())
}

pub(super) fn main_menu_title(
    context: &egui::Context,
    viewport: egui::Vec2,
) -> std::sync::Arc<egui::Galley> {
    context.fonts_mut(|fonts| {
        let font_size = (viewport.y * 0.11).clamp(68.0, 104.0);
        let title =
            fonts.layout_no_wrap(TITLE.to_string(), egui::FontId::proportional(font_size), CREAM);
        let available_width = (viewport.x - 48.0).max(1.0);
        if title.size().x <= available_width {
            title
        } else {
            fonts.layout_no_wrap(
                TITLE.to_string(),
                egui::FontId::proportional(font_size * available_width / title.size().x),
                CREAM,
            )
        }
    })
}

pub(super) fn main_menu_top(context: &egui::Context, viewport: egui::Vec2) -> f32 {
    let title_height = main_menu_title(context, viewport).size().y;
    let title_bottom = viewport.y * 0.17 + title_height * 0.5;
    let count = MAIN_MENU_ACTION_COUNT as f32;
    let actions_height = MENU_ACTION_HEIGHT * count + FORM_CARD_GAP * (count - 1.0);
    (viewport.y * 0.365).min(viewport.y - 96.0 - actions_height).max(title_bottom + 24.0)
}

pub(super) fn connection_status_badge(ui: &mut egui::Ui) {
    let label = ui.painter().layout_no_wrap(
        "Offline · local preview".to_string(),
        egui::FontId::proportional(14.0),
        CREAM,
    );
    let size = egui::vec2(label.size().x + 46.0, label.size().y.max(10.0) + 10.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter().rect(
        rect,
        egui::CornerRadius::same(10),
        egui::Color32::from_rgba_unmultiplied(31, 20, 18, 210),
        egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(224, 190, 146, 48)),
        egui::StrokeKind::Inside,
    );
    let center_y = rect.center().y;
    ui.painter().circle_filled(
        egui::pos2(rect.left() + 14.0, center_y),
        4.0,
        egui::Color32::from_rgb(224, 116, 91),
    );
    ui.painter().galley(
        egui::pos2(rect.left() + 28.0, center_y - label.size().y * 0.5),
        label,
        CREAM,
    );
}
