//! Menu wallpapers and loading transition.

use super::*;

pub(super) fn setup_background(mut commands: Commands, assets: Res<AssetServer>) {
    #[cfg(not(target_arch = "wasm32"))]
    let wallpaper_folder = LoadingWallpaperFolder(assets.load_folder("images/bg"));
    commands.spawn((
        Sprite::from_image(assets.load("images/bg/bg-menu.basisu.ktx2")),
        Transform::from_xyz(0.0, 0.0, -10.0),
        MenuBackground,
    ));
    for layer in 0..2 {
        commands.spawn((
            Sprite {
                image: if layer == 0 {
                    assets.load("images/bg/bg1.basisu.ktx2")
                } else {
                    Handle::default()
                },
                color: Color::srgba(
                    1.0,
                    1.0,
                    1.0,
                    if layer == 0 {
                        1.0
                    } else {
                        0.0
                    },
                ),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, -9.0 + layer as f32),
            Visibility::Hidden,
            LoadingBackground(layer),
        ));
    }
    commands.insert_resource(LoadingWallpapers(Vec::new()));
    #[cfg(not(target_arch = "wasm32"))]
    commands.insert_resource(wallpaper_folder);
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn populate_loading_wallpapers(
    folder: Res<LoadingWallpaperFolder>,
    folders: Res<Assets<LoadedFolder>>,
    mut wallpapers: ResMut<LoadingWallpapers>,
    mut sequence: ResMut<LoadingSequence>,
    mut backgrounds: Query<(&LoadingBackground, &mut Sprite)>,
) {
    if !wallpapers.0.is_empty() {
        return;
    }
    let Some(folder) = folders.get(&folder.0) else {
        return;
    };

    let mut images = folder
        .handles
        .iter()
        .filter_map(|handle| {
            let path = handle.path()?;
            let file_name = path.path().file_name()?.to_str()?;
            let number =
                file_name.strip_prefix("bg")?.strip_suffix(".basisu.ktx2")?.parse::<u8>().ok()?;
            (1..=12).contains(&number).then(|| (number, handle.clone().typed::<Image>()))
        })
        .collect::<Vec<_>>();
    images.sort_by_key(|(number, _)| *number);
    wallpapers.0 = images.into_iter().map(|(_, handle)| handle).collect();
    if wallpapers.0.is_empty() {
        warn!("No loading wallpapers found in images/bg");
        return;
    }

    shuffle_loading_wallpapers(&mut wallpapers.0);
    sequence.slide_index = 0;
    sequence.incoming_index = (sequence.slide_index + 1) % wallpapers.0.len();
    for (layer, mut sprite) in &mut backgrounds {
        sprite.image = wallpapers.0[sequence.slide_index].clone();
        sprite.color = Color::srgba(
            1.0,
            1.0,
            1.0,
            if layer.0 == 0 {
                1.0
            } else {
                0.0
            },
        );
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn discover_loading_wallpapers(
    assets: Res<AssetServer>,
    mut discovery: ResMut<LoadingWallpaperDiscovery>,
    mut wallpapers: ResMut<LoadingWallpapers>,
    mut sequence: ResMut<LoadingSequence>,
    mut backgrounds: Query<(&LoadingBackground, &mut Sprite)>,
) {
    if discovery.complete {
        return;
    }
    if discovery.next_index > 12 {
        discovery.complete = true;
        if wallpapers.0.is_empty() {
            warn!("No numbered loading wallpapers found in images/bg");
            return;
        }
        shuffle_loading_wallpapers(&mut wallpapers.0);
        sequence.slide_index = 0;
        sequence.incoming_index = usize::from(wallpapers.0.len() > 1);
        set_initial_loading_wallpaper(&wallpapers.0, &mut backgrounds);
        return;
    }
    let Some(pending) = discovery.pending.as_ref() else {
        discovery.pending =
            Some(assets.load(format!("images/bg/bg{}.basisu.ktx2", discovery.next_index)));
        return;
    };

    match assets.get_load_state(pending.id()) {
        Some(bevy::asset::LoadState::Loaded) => {
            wallpapers.0.push(pending.clone());
            discovery.next_index += 1;
            discovery.pending = None;
        },
        Some(bevy::asset::LoadState::Failed(_)) => {
            warn!("Loading wallpaper {} failed", discovery.next_index);
            discovery.next_index = 13;
            discovery.pending = None;
        },
        _ => {},
    }
}

pub(super) fn shuffle_loading_wallpapers(wallpapers: &mut [Handle<Image>]) {
    for index in (1..wallpapers.len()).rev() {
        wallpapers.swap(index, random_range(0..=index));
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn set_initial_loading_wallpaper(
    wallpapers: &[Handle<Image>],
    backgrounds: &mut Query<(&LoadingBackground, &mut Sprite)>,
) {
    for (layer, mut sprite) in backgrounds.iter_mut() {
        sprite.image = wallpapers[0].clone();
        sprite.color = Color::srgba(
            1.0,
            1.0,
            1.0,
            if layer.0 == 0 {
                1.0
            } else {
                0.0
            },
        );
    }
}

pub(super) fn fit_menu_background(
    window: Single<&Window>,
    images: Res<Assets<Image>>,
    mut backgrounds: Query<&mut Sprite, Or<(With<MenuBackground>, With<LoadingBackground>)>>,
) {
    let viewport = Vec2::new(window.width(), window.height());
    for mut sprite in &mut backgrounds {
        if sprite.custom_size != Some(viewport) {
            sprite.custom_size = Some(viewport);
        }
        if let Some(image) = images.get(&sprite.image) {
            let rect = cover_source_rect(image.size_f32(), viewport);
            if sprite.rect != Some(rect) {
                sprite.rect = Some(rect);
            }
        }
    }
}

pub(super) fn cover_source_rect(source: Vec2, viewport: Vec2) -> Rect {
    if source.min_element() <= 0.0 || viewport.min_element() <= 0.0 {
        return Rect::from_corners(Vec2::ZERO, source.max(Vec2::ZERO));
    }
    let source_aspect = source.x / source.y;
    let viewport_aspect = viewport.x / viewport.y;
    if viewport_aspect > source_aspect {
        let height = source.x / viewport_aspect;
        let top = (source.y - height) * 0.5;
        Rect::from_corners(Vec2::new(0.0, top), Vec2::new(source.x, top + height))
    } else {
        let width = source.y * viewport_aspect;
        let left = (source.x - width) * 0.5;
        Rect::from_corners(Vec2::new(left, 0.0), Vec2::new(left + width, source.y))
    }
}

pub(super) fn set_menu_background_visibility(
    state: Res<State<AppState>>,
    loading: Res<LoadingSequence>,
    mut background: Query<&mut Visibility, With<MenuBackground>>,
    mut loading_backgrounds: Query<
        &mut Visibility,
        (With<LoadingBackground>, Without<MenuBackground>),
    >,
) {
    let visibility = if matches!(
        *state.get(),
        AppState::Loading
            | AppState::Map
            | AppState::GameMenu
            | AppState::GameSettings
            | AppState::EmptyScreen
    ) {
        Visibility::Hidden
    } else {
        Visibility::Visible
    };
    if let Ok(mut current) = background.single_mut() {
        if *current != visibility {
            *current = visibility;
        }
    }
    let loading_visibility =
        if *state.get() == AppState::Loading && loading.map_reveal_progress.is_none() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    for mut current in &mut loading_backgrounds {
        if *current != loading_visibility {
            *current = loading_visibility;
        }
    }
}

pub(super) fn revealing_map(state: AppState, loading: &LoadingSequence) -> bool {
    state == AppState::Loading
        && loading.destination == AppState::Map
        && loading.map_reveal_progress.is_some()
}

pub(super) fn game_ui_visible(state: Res<State<AppState>>, loading: Res<LoadingSequence>) -> bool {
    revealing_map(*state.get(), &loading)
        || matches!(
            *state.get(),
            AppState::Map | AppState::EmptyScreen | AppState::GameMenu | AppState::GameSettings
        )
}

pub(super) fn audio_controls_visible(
    state: Res<State<AppState>>,
    loading: Res<LoadingSequence>,
) -> bool {
    !revealing_map(*state.get(), &loading)
        && !matches!(
            *state.get(),
            AppState::Map | AppState::EmptyScreen | AppState::GameMenu | AppState::GameSettings
        )
}

pub(super) fn map_visible(
    state: Res<State<AppState>>,
    game: Res<ActiveGame>,
    loading: Res<LoadingSequence>,
) -> bool {
    *state.get() == AppState::Map
        || revealing_map(*state.get(), &loading)
        || (matches!(*state.get(), AppState::GameMenu | AppState::GameSettings)
            && *game == ActiveGame::LocalPractice)
}

pub(super) fn start_loading_wallpaper(
    mut sequence: ResMut<LoadingSequence>,
    mut wallpapers: ResMut<LoadingWallpapers>,
    mut backgrounds: Query<(&LoadingBackground, &mut Sprite, &mut Transform)>,
) {
    if wallpapers.0.is_empty() {
        return;
    }
    shuffle_loading_wallpapers(&mut wallpapers.0);
    sequence.slide_index = 0;
    sequence.incoming_index = usize::from(wallpapers.0.len() > 1);
    for (layer, mut sprite, mut transform) in &mut backgrounds {
        sprite.image = wallpapers.0[sequence.slide_index].clone();
        sprite.rect = None;
        sprite.color = Color::srgba(1.0, 1.0, 1.0, 0.0);
        transform.translation.z = -9.0 + layer.0 as f32;
    }
}

pub(super) fn advance_loading_wallpaper(
    time: Res<Time>,
    state: Res<State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
    mut sequence: ResMut<LoadingSequence>,
    wallpapers: Res<LoadingWallpapers>,
    #[cfg(target_arch = "wasm32")] discovery: Res<LoadingWallpaperDiscovery>,
    mut backgrounds: Query<(&LoadingBackground, &mut Sprite, &mut Transform)>,
    map_view: Res<MapView>,
) {
    if *state.get() != AppState::Loading {
        return;
    }
    if wallpapers.0.is_empty() {
        return;
    }
    #[cfg(target_arch = "wasm32")]
    if !discovery.complete {
        return;
    }
    let delta = time.delta_secs();
    sequence.elapsed += delta;
    sequence.slide_elapsed += delta;

    if let Some(progress) = sequence.map_reveal_progress {
        let progress = (progress + delta / LOADING_MAP_REVEAL_SECONDS).min(1.0);
        sequence.map_reveal_progress = Some(progress);
        if progress >= 1.0 {
            next.set(AppState::Map);
        }
        return;
    }

    if let Some(progress) = sequence.fade_progress {
        let progress = (progress + delta / 0.8).min(1.0);
        sequence.fade_progress = Some(progress);
        for (layer, mut sprite, _) in &mut backgrounds {
            if layer.0 != sequence.active_layer {
                sprite.color = Color::srgba(1.0, 1.0, 1.0, progress);
            }
        }
        if progress >= 1.0 {
            for (layer, mut sprite, _) in &mut backgrounds {
                if layer.0 == sequence.active_layer {
                    sprite.color = Color::srgba(1.0, 1.0, 1.0, 0.0);
                }
            }
            sequence.active_layer = 1 - sequence.active_layer;
            sequence.slide_index = sequence.incoming_index;
            sequence.fade_progress = None;
        }
    } else if sequence.slide_elapsed >= 5.0 && wallpapers.0.len() > 1 {
        sequence.incoming_index = (sequence.slide_index + 1) % wallpapers.0.len();
        for (layer, mut sprite, mut transform) in &mut backgrounds {
            if layer.0 == sequence.active_layer {
                transform.translation.z = -9.0;
            } else {
                sprite.image = wallpapers.0[sequence.incoming_index].clone();
                sprite.rect = None;
                sprite.color = Color::srgba(1.0, 1.0, 1.0, 0.0);
                transform.translation.z = -8.0;
            }
        }
        sequence.slide_elapsed = 0.0;
        sequence.fade_progress = Some(0.0);
    } else if sequence.elapsed <= LOADING_WALLPAPER_FADE_IN_SECONDS {
        for (layer, mut sprite, _) in &mut backgrounds {
            if layer.0 == sequence.active_layer {
                sprite.color = Color::srgba(
                    1.0,
                    1.0,
                    1.0,
                    (sequence.elapsed / LOADING_WALLPAPER_FADE_IN_SECONDS).min(1.0),
                );
            }
        }
    }

    if sequence.destination == AppState::Map
        && map_view.is_loaded()
        && sequence.fade_progress.is_none()
    {
        sequence.map_reveal_progress = Some(0.0);
    } else if sequence.destination != AppState::Map {
        next.set(sequence.destination);
    }
}
