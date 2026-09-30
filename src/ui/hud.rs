//! Map HUD systems and province panels.

use super::*;

#[derive(Default)]
pub(in crate::app) struct RankArtTextures {
    ranks: Option<[Option<egui::TextureHandle>; 6]>,
    scepter: Option<egui::TextureHandle>,
}

pub(in crate::app) fn draw_map_hud(
    mut contexts: EguiContexts,
    mut standard_textures: Local<Option<[Option<egui::TextureHandle>; PLAYER_COLORS.len()]>>,
    mut menu_icon_textures: Local<Option<[Option<egui::TextureHandle>; 4]>>,
    mut panels: MapPanelParams,
    mut map_view: ResMut<MapView>,
    mut rank_art: Local<RankArtTextures>,
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
    if panels.terminal.as_ref().is_some_and(|terminal| terminal.spectating) {
        return;
    }
    let Ok(context) = contexts.ctx_mut() else {
        return;
    };
    let scale = viewport_ui_scale(context.content_rect().size());
    let mut color_index = if *game == ActiveGame::LobbyPreview {
        lobby.color_index.min(PLAYER_COLORS.len() - 1)
    } else {
        practice.color_index.min(PLAYER_COLORS.len() - 1)
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
        if draw_map_menu_hitboxes(context, scale)
            && *state.get() == AppState::Map
            && open_main_province(
                &practice,
                &mut panels.campaign_ui,
                &mut panels.province,
                &mut panels.governance,
                &mut map_view,
            )
        {
            panels.close_click.0 = true;
            play_click(&sound, &audio, &assets);
        }
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
    if let Some(index) = governance_clicked.filter(|_| *state.get() == AppState::Map) {
        panels.campaign_ui.close_province_selector();
        if index == 0 {
            panels.governance.0 = !panels.governance.0;
            if panels.governance.0 {
                panels.campaign_ui.overview_section = 0;
            }
            panels.campaign_ui.open = None;
        } else {
            let tab = [
                campaign_panel::CampaignTab::Military,
                campaign_panel::CampaignTab::Trade,
                campaign_panel::CampaignTab::Senate,
            ][index - 1];
            panels.campaign_ui.open = if panels.campaign_ui.open == Some(tab) {
                None
            } else {
                Some(tab)
            };
            panels.governance.0 = false;
            if tab == campaign_panel::CampaignTab::Trade && panels.campaign_ui.open == Some(tab) {
                campaign_trade::open_routes(context, practice.active_player);
            }
        }
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
        let RankArtTextures {
            ranks: rank_textures,
            scepter: scepter_texture,
        } = &mut *rank_art;
        let textures = rank_textures.get_or_insert_with(|| std::array::from_fn(|_| None));
        let scepter = scepter_texture.get_or_insert_with(|| load_rank_scepter(context));
        let active_rank = practice.players[practice.active_player].rank.min(RANKS.len() - 1);
        draw_rank_ladder(context, scale, active_rank, textures, scepter);
        draw_practice_players(context, scale, &mut practice, textures, &sound, &audio, &assets);
    }
}

/// The player standard always returns to the founding province's overview.
pub(in crate::app) fn open_main_province(
    practice: &LocalPractice,
    view: &mut campaign_panel::CampaignUi,
    detail: &mut ProvincePanelOpen,
    governance: &mut GovernancePanelOpen,
    map_view: &mut MapView,
) -> bool {
    let Some(province) = practice.players.get(practice.active_player).and_then(|p| p.main_province)
    else {
        return false;
    };
    view.open_province_section(province, 0);
    detail.0 = Some(MapDetail::Province(province));
    governance.0 = false;
    map_view.focus_province(province);
    true
}

#[derive(Default)]
pub(in crate::app) struct GovernancePanelLocal {
    close_deadline: Option<f64>,
    overview_art_ready: bool,
}

#[derive(SystemParam)]
pub(in crate::app) struct EventFeedback<'w> {
    toasts: ResMut<'w, toasts::ToastQueue>,
    celebration: ResMut<'w, super::celebration::EventCelebration>,
    sound: Res<'w, MenuAudio>,
}

pub(in crate::app) fn draw_governance_panel(
    mut contexts: EguiContexts,
    state: Res<State<AppState>>,
    game: Res<ActiveGame>,
    practice: Res<LocalPractice>,
    mut panels: MapPanelParams,
    mut ownership: ResMut<ProvinceOwnership>,
    mut resources: ResMut<HudResources>,
    mut icon: Local<Option<egui::TextureHandle>>,
    mut effect_icons: Local<Option<[egui::TextureHandle; governance_panel::EFFECT_ICON_COUNT]>>,
    mut panel_local: Local<GovernancePanelLocal>,
    audio: Res<Audio>,
    assets: Res<AssetServer>,
    mut campaign: ResMut<campaign::Campaign>,
    mut feedback: EventFeedback,
    mut map_view: ResMut<MapView>,
    terminal: Res<TerminalPresentation>,
) {
    if terminal.spectating || *state.get() != AppState::Map || *game != ActiveGame::LocalPractice {
        panel_local.close_deadline = None;
        return;
    }
    let Ok(context) = contexts.ctx_mut() else {
        return;
    };
    let icon = icon.get_or_insert_with(|| {
        let mut textures = std::array::from_fn(|_| None);
        map_menu_icon(context, &mut textures, 0)
    });
    let effect_icons =
        effect_icons.get_or_insert_with(|| governance_panel::load_effect_icons(context));
    if !panel_local.overview_art_ready {
        let _ = campaign_widgets::prepare_portrait(
            context,
            campaign_widgets::ProvinceLandscape::Policies,
        );
        for tab in [
            campaign_widgets::Icon::Policies,
            campaign_widgets::Icon::Events,
            campaign_widgets::Icon::Province,
            campaign_widgets::Icon::Spy,
            campaign_widgets::Icon::Notifications,
        ] {
            campaign_widgets::texture(context, tab);
        }
        panel_local.overview_art_ready = true;
    }
    if !panels.governance.0 {
        panel_local.close_deadline = None;
        return;
    }
    let now = context.input(|input| input.time);
    if panel_local.close_deadline.is_some_and(|deadline| now >= deadline) {
        panel_local.close_deadline = None;
        panels.governance.0 = false;
        panels.close_click.0 = true;
        return;
    }
    let player = practice.active_player;
    let mut governance = if campaign.active {
        campaign.governance_for(player)
    } else {
        ownership.governance_for(player)
    };
    let scale = viewport_ui_scale(context.content_rect().size());
    let color_index = practice.players[player].color_index;
    let banner_color = PLAYER_COLORS[color_index];
    let colors: Vec<_> = practice.players.iter().map(|p| PLAYER_COLORS[p.color_index]).collect();
    let view = &mut *panels.campaign_ui;
    let selected = &mut view.overview_section;
    let province_search = &mut view.politics_province_search;
    let spy_search = &mut view.politics_spy_search;
    let notice_filters = &mut view.notice_filters;
    let mut requested_event = None;
    let mut requested_notice = None;
    let mut spy_message = None;
    let (changed, close_clicked, target) = governance_panel::show_overview(
        context,
        scale,
        icon,
        effect_icons,
        banner_color,
        &mut governance,
        panel_local.close_deadline.is_some(),
        selected,
        |ui, section| {
            *ui.style_mut() = campaign_widgets::map_style(scale);
            match section {
                1 => {
                    requested_event = campaign_events::show(ui, &campaign, player, scale);
                    None
                },
                2 => campaign_panel::politics_provinces(
                    ui,
                    &campaign,
                    player,
                    &colors,
                    province_search,
                    scale,
                ),
                3 => {
                    let (target, message) = campaign_panel::politics_spies(
                        ui,
                        &mut campaign,
                        player,
                        &colors,
                        spy_search,
                        scale,
                    );
                    spy_message = message;
                    target
                },
                4 => {
                    requested_notice =
                        campaign_notices::overview(ui, &campaign, player, notice_filters, scale);
                    None
                },
                _ => None,
            }
        },
    );
    if let Some(message) = spy_message {
        feedback.toasts.push(toasts::Toast::info(message));
        play_click(&feedback.sound, &audio, &assets);
    }
    if let Some(event) = requested_event {
        if campaign.hold_event(player, event).is_ok() {
            let reward = campaign.event_quote(player, event).coin_reward;
            feedback.toasts.push(
                toasts::Toast::info(format!(
                    "{} — {}",
                    event.held_label(),
                    event.reward_label(reward)
                ))
                .with_title("Event held")
                .without_sound(),
            );
            feedback.celebration.start(now);
            if feedback.sound.mode != AudioMode::Mute && feedback.sound.volume > 0.001 {
                let decibels = -8.0 + 20.0 * feedback.sound.volume.clamp(0.001, 1.0).log10();
                audio.play(assets.load("audio/victory.ogg")).with_volume(decibels);
            }
        }
    }
    if let Some(notice) = requested_notice {
        if notice.kind == super::campaign_notifications::NoticeKind::TradeInterrupted {
            campaign_trade::open_routes(context, player);
        }
        campaign_panel::open_notification(
            view,
            &notice,
            &campaign,
            &mut panels.province,
            &mut map_view,
        );
        panels.governance.0 = false;
        panels.close_click.0 = true;
        play_click(&feedback.sound, &audio, &assets);
    }
    if let Some(target) = target {
        let section = if view.overview_section == 3 {
            4
        } else {
            0
        };
        view.open_province_section(target, section);
        panels.province.0 = Some(MapDetail::Province(target));
        map_view.focus_province(target);
        panels.governance.0 = false;
        panels.close_click.0 = true;
        play_click(&feedback.sound, &audio, &assets);
    }
    if close_clicked && panel_local.close_deadline.is_none() {
        panel_local.close_deadline = Some(now + 0.18);
        panels.close_click.0 = true;
        play_click(&feedback.sound, &audio, &assets);
        context.request_repaint_after(std::time::Duration::from_millis(180));
    }
    if changed {
        ownership.set_governance_for(player, governance);
        if campaign.active {
            campaign.set_governance(player, governance);
        } else {
            resources.refresh_player_rates(player, &ownership);
        }
        play_click(&feedback.sound, &audio, &assets);
    }
}

#[derive(Default)]
pub(in crate::app) struct DetailTextures {
    terrain: Option<[egui::TextureHandle; 12]>,
    population: Option<[egui::TextureHandle; 4]>,
    resources: Option<[egui::TextureHandle; 3]>,
    diplomacy: Option<[egui::TextureHandle; 4]>,
    city_banner: Option<egui::TextureHandle>,
    buildings: Option<[egui::TextureHandle; 8]>,
    province_icon: Option<egui::TextureHandle>,
}

pub(in crate::app) fn draw_province_panel(
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
    campaign: Res<campaign::Campaign>,
) {
    if campaign.active {
        return;
    }
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
