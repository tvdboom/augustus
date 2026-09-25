//! Map HUD systems and province panels.

use super::*;

pub(super) fn draw_map_hud(
    mut contexts: EguiContexts,
    mut standard_textures: Local<Option<[Option<egui::TextureHandle>; 6]>>,
    mut menu_icon_textures: Local<Option<[Option<egui::TextureHandle>; 4]>>,
    mut panels: MapPanelParams,
    mut rank_textures: Local<Option<[Option<egui::TextureHandle>; 6]>>,
    mut scepter_texture: Local<Option<egui::TextureHandle>>,
    state: Res<State<AppState>>,
    game: Res<ActiveGame>,
    mut practice: ResMut<LocalPractice>,
    lobby: Res<LobbyPreview>,
    mut paused: ResMut<GamePaused>,
    mut clock: ResMut<GameClock>,
    mut next: ResMut<NextState<AppState>>,
    mut sound: ResMut<MenuAudio>,
    audio: Res<Audio>,
    assets: Res<AssetServer>,
) {
    panels.close_click.0 = false;
    let Ok(context) = contexts.ctx_mut() else {
        return;
    };
    let scale = viewport_ui_scale(context.content_rect().size());
    let mut color_index = if *game == ActiveGame::LobbyPreview {
        lobby.color_index.min(PLAYER_STANDARDS.len() - 1)
    } else {
        practice.color_index.min(PLAYER_STANDARDS.len() - 1)
    };
    if *game == ActiveGame::LocalPractice && !practice.players.is_empty() {
        practice.active_player = practice.active_player.min(practice.players.len() - 1);
        color_index = practice.players[practice.active_player].color_index;
    }
    let standards = standard_textures.get_or_insert_with(|| std::array::from_fn(|_| None));
    let standard =
        standards[color_index].get_or_insert_with(|| load_player_standard(context, color_index));
    let mut speed_button_state = [(false, false); 2];
    if matches!(*state.get(), AppState::Map | AppState::EmptyScreen) {
        draw_map_menu_hitboxes(context, scale);
        if map_resource_strip_visible(context.content_rect(), scale) {
            let minus_available = clock.speed_step > MIN_SPEED_STEP;
            let minus = map_date_hitbox(context, scale, "minus", 24.0, 24.0, minus_available);
            speed_button_state[0] = (
                minus_available && minus.hovered(),
                minus_available && minus.is_pointer_button_down_on(),
            );
            if minus.clicked() {
                paused.0 = false;
                let previous = clock.speed_step;
                clock.change_speed(-1);
                if clock.speed_step != previous {
                    play_click(&sound, &audio, &assets);
                }
            }
            if map_date_hitbox(context, scale, "date", 51.0, 108.0, true).clicked() {
                paused.0 = !paused.0;
                play_click(&sound, &audio, &assets);
            }
            let plus_available = clock.speed_step < MAX_SPEED_STEP;
            let plus = map_date_hitbox(context, scale, "plus", 162.0, 24.0, plus_available);
            speed_button_state[1] = (
                plus_available && plus.hovered(),
                plus_available && plus.is_pointer_button_down_on(),
            );
            if plus.clicked() {
                paused.0 = false;
                let previous = clock.speed_step;
                clock.change_speed(1);
                if clock.speed_step != previous {
                    play_click(&sound, &audio, &assets);
                }
            }
        }
    }
    paint_map_edge_frame(context, scale, standard, &clock, paused.0, speed_button_state);
    let icons = menu_icon_textures.get_or_insert_with(|| std::array::from_fn(|_| None));
    let governance_clicked = draw_map_left_menu(
        context,
        scale,
        icons,
        matches!(*state.get(), AppState::Map | AppState::EmptyScreen),
    );
    if governance_clicked && *state.get() == AppState::Map {
        panels.governance.0 = !panels.governance.0;
        panels.province.0 = None;
        play_click(&sound, &audio, &assets);
    }
    let responses = egui::Area::new(egui::Id::new("augustus_map_hud"))
        .anchor(
            egui::Align2::RIGHT_TOP,
            egui::vec2(
                -12.0 * scale,
                context.content_rect().height() * MAP_CORNER_CONTROLS_TOP_FRACTION,
            ),
        )
        .order(egui::Order::Foreground)
        .show(context, |ui| {
            ui.horizontal(|ui| {
                let settings = settings_button(ui, scale);
                let audio_button = audio_mode_button(ui, sound.mode, scale);
                (settings, audio_button)
            })
            .inner
        })
        .inner;
    if responses.0.clicked() {
        next.set(settings_button_destination(*state.get(), *game));
        play_click(&sound, &audio, &assets);
    }
    if responses.1.clicked() {
        sound.toggle_mute();
        play_click(&sound, &audio, &assets);
    }
    volume_popover(&responses.1, &mut sound);
    if *game == ActiveGame::LocalPractice && !practice.players.is_empty() {
        let textures = rank_textures.get_or_insert_with(|| std::array::from_fn(|_| None));
        let scepter = scepter_texture.get_or_insert_with(|| load_rank_scepter(context));
        let active_rank = practice.players[practice.active_player].rank.min(RANKS.len() - 1);
        draw_rank_ladder(context, scale, active_rank, textures, scepter);
        draw_practice_players(context, scale, &mut practice, textures, &sound, &audio, &assets);
    }
}

pub(super) fn draw_governance_panel(
    mut contexts: EguiContexts,
    state: Res<State<AppState>>,
    game: Res<ActiveGame>,
    practice: Res<LocalPractice>,
    mut open: ResMut<GovernancePanelOpen>,
    mut panel_close_click: ResMut<MapPanelCloseClick>,
    mut ownership: ResMut<ProvinceOwnership>,
    mut resources: ResMut<HudResources>,
    mut icon: Local<Option<egui::TextureHandle>>,
    mut effect_icons: Local<Option<[egui::TextureHandle; governance_panel::EFFECT_ICON_COUNT]>>,
    mut close_deadline: Local<Option<f64>>,
    sound: Res<MenuAudio>,
    audio: Res<Audio>,
    assets: Res<AssetServer>,
) {
    if *state.get() != AppState::Map || *game != ActiveGame::LocalPractice || !open.0 {
        *close_deadline = None;
        return;
    }
    let Ok(context) = contexts.ctx_mut() else {
        return;
    };
    let now = context.input(|input| input.time);
    if close_deadline.is_some_and(|deadline| now >= deadline) {
        *close_deadline = None;
        open.0 = false;
        panel_close_click.0 = true;
        return;
    }
    let icon = icon.get_or_insert_with(|| {
        let image = image::load_from_memory(MAP_MENU_ICONS[0].1)
            .expect("governance icon PNG must be valid")
            .to_rgba8();
        context.load_texture(
            "augustus_governance_header_icon",
            egui::ColorImage::from_rgba_unmultiplied(
                [image.width() as usize, image.height() as usize],
                image.as_raw(),
            ),
            egui::TextureOptions::LINEAR,
        )
    });
    let player = practice.active_player;
    let mut governance = ownership.governance_for(player);
    let scale = viewport_ui_scale(context.content_rect().size());
    let color_index = practice.players[player].color_index;
    let banner_color = PLAYER_COLORS[color_index];
    let effect_icons =
        effect_icons.get_or_insert_with(|| governance_panel::load_effect_icons(context));
    let (changed, close_clicked) = governance_panel::show(
        context,
        scale,
        icon,
        effect_icons,
        banner_color,
        &mut governance,
        close_deadline.is_some(),
    );
    if close_clicked && close_deadline.is_none() {
        *close_deadline = Some(now + 0.18);
        panel_close_click.0 = true;
        play_click(&sound, &audio, &assets);
        context.request_repaint_after(std::time::Duration::from_millis(180));
    }
    if changed {
        ownership.set_governance_for(player, governance);
        resources.refresh_player_rates(player, &ownership);
        play_click(&sound, &audio, &assets);
    }
}

#[derive(Default)]
pub(super) struct DetailTextures {
    terrain: Option<[egui::TextureHandle; 12]>,
    population: Option<[egui::TextureHandle; 4]>,
    resources: Option<[egui::TextureHandle; 3]>,
    diplomacy: Option<[egui::TextureHandle; 4]>,
    city_banner: Option<egui::TextureHandle>,
    buildings: Option<[egui::TextureHandle; 8]>,
    province_icon: Option<egui::TextureHandle>,
}

pub(super) fn draw_province_panel(
    mut contexts: EguiContexts,
    state: Res<State<AppState>>,
    game: Res<ActiveGame>,
    mut open: ResMut<ProvincePanelOpen>,
    mut panel_close_click: ResMut<MapPanelCloseClick>,
    ownership: Res<ProvinceOwnership>,
    practice: Res<LocalPractice>,
    mut textures: Local<DetailTextures>,
    mut emblem: Local<Option<egui::TextureHandle>>,
    mut close_deadline: Local<Option<(MapDetail, f64)>>,
    sound: Res<MenuAudio>,
    audio: Res<Audio>,
    assets: Res<AssetServer>,
) {
    if *state.get() != AppState::Map || *game != ActiveGame::LocalPractice {
        *close_deadline = None;
        return;
    }
    let Some(selected) = open.0 else {
        *close_deadline = None;
        return;
    };
    let index = match selected {
        MapDetail::Province(index) | MapDetail::City(index) => index,
    };
    if close_deadline.is_some_and(|(closing_detail, _)| closing_detail != selected) {
        *close_deadline = None;
    }
    let Some(province) = ownership.province_overview(index) else {
        open.0 = None;
        return;
    };
    let Ok(context) = contexts.ctx_mut() else {
        return;
    };
    let now = context.input(|input| input.time);
    if close_deadline.is_some_and(|(_, deadline)| now >= deadline) {
        *close_deadline = None;
        open.0 = None;
        panel_close_click.0 = true;
        return;
    }
    let emblem = province
        .owner
        .map(|_| emblem.get_or_insert_with(|| province_panel::load_spqr_emblem(context)));
    let scale = viewport_ui_scale(context.content_rect().size());
    let DetailTextures {
        terrain,
        population,
        resources,
        diplomacy,
        city_banner,
        buildings,
        province_icon,
    } = &mut *textures;
    let (close_clicked, related_clicked) = match selected {
        MapDetail::Province(_) => province_panel::show(
            context,
            scale,
            &province,
            terrain.get_or_insert_with(|| province_panel::load_terrain_images(context)),
            population.get_or_insert_with(|| province_panel::load_population_icons(context)),
            resources.get_or_insert_with(|| province_panel::load_resource_icons(context)),
            diplomacy.get_or_insert_with(|| province_panel::load_diplomacy_icons(context)),
            practice.active_player,
            ownership.can_trade_with(index, practice.active_player),
            emblem.as_deref(),
            close_deadline.is_some(),
        ),
        MapDetail::City(_) => city_panel::show(
            context,
            scale,
            &province,
            city_banner.get_or_insert_with(|| city_panel::load_banner(context)),
            buildings.get_or_insert_with(|| city_panel::load_buildings(context)),
            province_icon.get_or_insert_with(|| city_panel::load_province_icon(context)),
            emblem.as_deref(),
            close_deadline.is_some(),
        ),
    };
    if related_clicked {
        open.0 = Some(match selected {
            MapDetail::Province(index) => MapDetail::City(index),
            MapDetail::City(index) => MapDetail::Province(index),
        });
        *close_deadline = None;
        panel_close_click.0 = true;
        play_click(&sound, &audio, &assets);
        return;
    }
    if close_clicked && close_deadline.is_none() {
        *close_deadline = Some((selected, now + 0.18));
        panel_close_click.0 = true;
        play_click(&sound, &audio, &assets);
        context.request_repaint_after(std::time::Duration::from_millis(180));
    }
}
