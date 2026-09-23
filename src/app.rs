//! Augustus menu, lobby preview, audio controls, and application state.

use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use bevy::asset::LoadedFolder;
use bevy::prelude::*;
use bevy::window::WindowResolution;
use bevy_egui::{egui, EguiClipboard, EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use bevy_kira_audio::prelude::*;
use egui::epaint::text::{FontInsert, FontPriority, InsertFontFamily};
use rand::random_range;

use crate::basis_texture::BasisTexturePlugin;
use crate::map::{draw_map, MapView};
use crate::multiplayer::lobby::{generate_game_code, LobbyPreview};
use crate::TITLE;

/// Native and initial browser width for the menu canvas.
pub const WINDOW_WIDTH: u32 = 1600;
/// Native and initial browser height for the menu canvas.
pub const WINDOW_HEIGHT: u32 = 900;

const MENU_ACTION_WIDTH: f32 = 308.0;
const MENU_ACTION_HEIGHT: f32 = 55.0;
const MENU_ACTION_TEXT_SIZE: f32 = 23.0;
const MENU_TITLE_TEXT_SIZE: f32 = 38.0;
const MENU_CARD_LABEL_TEXT_SIZE: f32 = 17.0;
const MENU_CONTROL_TEXT_SIZE: f32 = 20.0;
const MENU_TOOLTIP_TEXT_SIZE: f32 = 15.0;
const MENU_CONTROL_HEIGHT: f32 = 36.0;
const MENU_CONTENT_WIDTH: f32 = 452.0;
const RESUME_MENU_WIDTH: f32 = 560.0;
const FORM_CARD_GAP: f32 = 12.0;
const FORM_TITLE_GAP: f32 = 28.0;
const FORM_ACTION_GAP: f32 = 24.0;
const MAIN_MENU_ACTION_COUNT: usize = 5 + cfg!(not(target_arch = "wasm32")) as usize;

const CREAM: egui::Color32 = egui::Color32::from_rgb(239, 220, 188);
const GOLD: egui::Color32 = egui::Color32::from_rgb(195, 145, 87);
const MUTED_TEXT: egui::Color32 = egui::Color32::from_rgb(209, 181, 150);

#[derive(States, Default, Debug, Clone, Copy, Eq, PartialEq, Hash)]
/// Screens in the Augustus menu-first prototype.
pub enum AppState {
    /// Main navigation.
    #[default]
    MainMenu,
    /// New-game player-name form.
    CreateGame,
    /// Join-game form.
    JoinGame,
    /// Placeholder for saved Augustus campaigns.
    ResumeGame,
    /// Multiplayer lobby presentation backed by a local preview until Supabase is connected.
    Lobby,
    /// Audio preferences retained from the reference menu.
    Settings,
    /// Full-screen wallpaper slideshow while entering a playable view.
    Loading,
    /// Local historical map.
    Map,
    /// Pause menu shown over the current game.
    GameMenu,
    /// Audio settings shown over the current game.
    GameSettings,
    /// Blank screen shown after starting the prototype lobby.
    EmptyScreen,
}

#[derive(Default, Clone, Copy, Eq, PartialEq, Resource)]
enum ActiveGame {
    #[default]
    LocalPractice,
    LobbyPreview,
}

impl ActiveGame {
    fn screen(self) -> AppState {
        match self {
            Self::LocalPractice => AppState::Map,
            Self::LobbyPreview => AppState::EmptyScreen,
        }
    }
}

#[derive(Resource, Default)]
struct LocalPractice {
    color_index: usize,
}

impl LocalPractice {
    fn start_new_game(&mut self) {
        self.color_index = random_range(0..PLAYER_COLORS.len());
    }
}

#[derive(Component)]
struct MenuBackground;

#[derive(Component)]
struct LoadingBackground(usize);

#[derive(Resource)]
struct LoadingWallpapers(Vec<Handle<Image>>);

#[derive(Resource)]
#[cfg(not(target_arch = "wasm32"))]
struct LoadingWallpaperFolder(Handle<LoadedFolder>);

#[derive(Resource)]
#[cfg(target_arch = "wasm32")]
struct LoadingWallpaperDiscovery {
    next_index: u32,
    pending: Option<Handle<Image>>,
    complete: bool,
}

#[cfg(target_arch = "wasm32")]
impl Default for LoadingWallpaperDiscovery {
    fn default() -> Self {
        Self {
            next_index: 1,
            pending: None,
            complete: false,
        }
    }
}

#[derive(Resource)]
struct LoadingSequence {
    destination: AppState,
    elapsed: f32,
    slide_elapsed: f32,
    slide_index: usize,
    incoming_index: usize,
    active_layer: usize,
    fade_progress: Option<f32>,
}

impl Default for LoadingSequence {
    fn default() -> Self {
        Self {
            destination: AppState::Map,
            elapsed: 0.0,
            slide_elapsed: 0.0,
            slide_index: 0,
            incoming_index: 1,
            active_layer: 0,
            fade_progress: None,
        }
    }
}

impl LoadingSequence {
    fn begin(&mut self, destination: AppState, next: &mut NextState<AppState>) {
        self.destination = destination;
        self.elapsed = 0.0;
        self.slide_elapsed = 0.0;
        self.slide_index = 0;
        self.incoming_index = 0;
        self.active_layer = 0;
        self.fade_progress = None;
        next.set(AppState::Loading);
    }
}

#[derive(Resource, Default)]
struct MenuDraft {
    display_name: String,
    join_code: String,
}

#[derive(Resource, Default)]
struct GamePaused(bool);

const SECONDS_PER_MONTH: f32 = 3.0;
const MIN_SPEED_STEP: i8 = -2;
const MAX_SPEED_STEP: i8 = 3;
const MONTH_NAMES: [&str; 12] =
    ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

#[derive(Resource)]
struct GameClock {
    // Astronomical year numbering makes the transition from 1 BC to 1 AD seamless.
    year: i32,
    month: usize,
    month_progress: f32,
    speed_step: i8,
}

impl Default for GameClock {
    fn default() -> Self {
        Self {
            year: -449,
            month: 9,
            month_progress: 0.0,
            speed_step: 0,
        }
    }
}

impl GameClock {
    fn speed(&self) -> f32 {
        2.0_f32.powi(self.speed_step.into())
    }

    fn change_speed(&mut self, step: i8) {
        self.speed_step = (self.speed_step + step).clamp(MIN_SPEED_STEP, MAX_SPEED_STEP);
    }

    fn advance(&mut self, seconds: f32) {
        self.month_progress += seconds * self.speed();
        while self.month_progress >= SECONDS_PER_MONTH {
            self.month_progress -= SECONDS_PER_MONTH;
            self.month += 1;
            if self.month == MONTH_NAMES.len() {
                self.month = 0;
                self.year += 1;
            }
        }
    }

    fn date_label(&self) -> String {
        let (year, era) = if self.year <= 0 {
            (1 - self.year, "BC")
        } else {
            (self.year, "AD")
        };
        format!("{}, {year} {era}", MONTH_NAMES[self.month])
    }

    fn speed_label(&self) -> String {
        format!("{}×", self.speed())
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum AudioMode {
    Mute,
    Effects,
    Music,
}

#[derive(Resource)]
struct MenuAudio {
    volume: f32,
    mode: AudioMode,
    restored_mode: AudioMode,
    music: Handle<AudioInstance>,
}

impl MenuAudio {
    fn set_mode(&mut self, mode: AudioMode) {
        if mode != AudioMode::Mute {
            self.restored_mode = mode;
        }
        self.mode = mode;
    }

    fn toggle_mute(&mut self) {
        self.mode = if self.mode == AudioMode::Mute {
            self.restored_mode
        } else {
            self.restored_mode = self.mode;
            AudioMode::Mute
        };
    }
}

/// Plugin for the Augustus menu and local map.
pub struct AugustusPlugin;

impl Plugin for AugustusPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((BasisTexturePlugin, EguiPlugin::default()))
            .init_state::<AppState>()
            .init_resource::<MenuDraft>()
            .init_resource::<GamePaused>()
            .init_resource::<GameClock>()
            .init_resource::<LobbyPreview>()
            .init_resource::<LoadingSequence>()
            .init_resource::<ActiveGame>()
            .init_resource::<LocalPractice>()
            .init_resource::<MapView>()
            .add_systems(Startup, (setup_camera, setup_background, start_music).chain())
            .add_systems(OnEnter(AppState::Loading), (start_loading_wallpaper, reset_game_time))
            .add_systems(
                Update,
                (advance_loading_wallpaper, fit_menu_background, set_menu_background_visibility)
                    .chain(),
            )
            .add_systems(Update, handle_escape)
            .add_systems(Update, handle_game_shortcuts)
            .add_systems(Update, advance_game_time)
            .add_systems(
                EguiPrimaryContextPass,
                draw_menu
                    .run_if(not(in_state(AppState::Map).or_else(in_state(AppState::EmptyScreen)))),
            )
            .add_systems(
                EguiPrimaryContextPass,
                draw_audio_controls.run_if(not(in_state(AppState::Map)
                    .or_else(in_state(AppState::GameMenu))
                    .or_else(in_state(AppState::GameSettings))
                    .or_else(in_state(AppState::EmptyScreen)))),
            )
            .add_systems(EguiPrimaryContextPass, draw_map_hud.run_if(game_ui_visible))
            .add_systems(EguiPrimaryContextPass, draw_map.run_if(map_visible))
            .add_systems(Update, update_music_volume);
        #[cfg(target_arch = "wasm32")]
        app.init_resource::<LoadingWallpaperDiscovery>();
        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(Update, populate_loading_wallpapers);
        #[cfg(target_arch = "wasm32")]
        app.add_systems(Update, discover_loading_wallpapers);
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn setup_background(mut commands: Commands, assets: Res<AssetServer>) {
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
                image: Handle::default(),
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
fn populate_loading_wallpapers(
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
fn discover_loading_wallpapers(
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

fn shuffle_loading_wallpapers(wallpapers: &mut [Handle<Image>]) {
    for index in (1..wallpapers.len()).rev() {
        wallpapers.swap(index, random_range(0..=index));
    }
}

#[cfg(target_arch = "wasm32")]
fn set_initial_loading_wallpaper(
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

fn fit_menu_background(
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

fn cover_source_rect(source: Vec2, viewport: Vec2) -> Rect {
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

fn set_menu_background_visibility(
    state: Res<State<AppState>>,
    mut background: Query<&mut Visibility, With<MenuBackground>>,
    mut loading_backgrounds: Query<
        &mut Visibility,
        (With<LoadingBackground>, Without<MenuBackground>),
    >,
) {
    let visibility = if matches!(
        *state.get(),
        AppState::Map | AppState::GameMenu | AppState::GameSettings | AppState::EmptyScreen
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
    let loading_visibility = if *state.get() == AppState::Loading {
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

fn game_ui_visible(state: Res<State<AppState>>) -> bool {
    matches!(
        *state.get(),
        AppState::Map | AppState::EmptyScreen | AppState::GameMenu | AppState::GameSettings
    )
}

fn map_visible(state: Res<State<AppState>>, game: Res<ActiveGame>) -> bool {
    *state.get() == AppState::Map
        || (matches!(*state.get(), AppState::GameMenu | AppState::GameSettings)
            && *game == ActiveGame::LocalPractice)
}

fn start_loading_wallpaper(
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

fn advance_loading_wallpaper(
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
    } else if sequence.elapsed <= 0.8 {
        for (layer, mut sprite, _) in &mut backgrounds {
            if layer.0 == sequence.active_layer {
                sprite.color = Color::srgba(1.0, 1.0, 1.0, (sequence.elapsed / 0.8).min(1.0));
            }
        }
    }

    if sequence.destination != AppState::Map || map_view.is_loaded() {
        next.set(sequence.destination);
    }
}

fn start_music(mut commands: Commands, assets: Res<AssetServer>, audio: Res<Audio>) {
    let music = audio.play(assets.load("audio/music.ogg")).looped().with_volume(-60.0).handle();
    commands.insert_resource(MenuAudio {
        volume: 0.55,
        mode: AudioMode::Effects,
        restored_mode: AudioMode::Effects,
        music,
    });
}

fn update_music_volume(menu_audio: Res<MenuAudio>, mut instances: ResMut<Assets<AudioInstance>>) {
    if !menu_audio.is_changed() {
        return;
    }
    if let Some(mut instance) = instances.get_mut(&menu_audio.music) {
        let decibels = if menu_audio.mode != AudioMode::Music || menu_audio.volume <= 0.001 {
            -60.0
        } else {
            -20.0 + 20.0 * menu_audio.volume.clamp(0.0, 1.0).log10()
        };
        instance.set_decibels(decibels, AudioTween::linear(Duration::from_millis(90)));
    }
}

fn handle_escape(
    keyboard: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    game: Res<ActiveGame>,
    mut next: ResMut<NextState<AppState>>,
) {
    if !keyboard.just_pressed(KeyCode::Escape) {
        return;
    }
    if let Some(destination) = escape_destination(*state.get(), *game) {
        next.set(destination);
    }
}

fn escape_destination(state: AppState, game: ActiveGame) -> Option<AppState> {
    match state {
        AppState::MainMenu => None,
        AppState::Map | AppState::EmptyScreen => Some(AppState::GameMenu),
        AppState::GameMenu => Some(game.screen()),
        AppState::GameSettings => Some(AppState::GameMenu),
        _ => Some(AppState::MainMenu),
    }
}

fn handle_game_shortcuts(
    keyboard: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    game: Res<ActiveGame>,
    mut paused: ResMut<GamePaused>,
    mut next: ResMut<NextState<AppState>>,
) {
    if *state.get() == AppState::GameMenu
        && (keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::NumpadEnter))
    {
        next.set(game.screen());
        return;
    }
    if !keyboard.just_pressed(KeyCode::Space) {
        return;
    }
    match *state.get() {
        AppState::Map | AppState::EmptyScreen => paused.0 = !paused.0,
        AppState::GameMenu => next.set(game.screen()),
        _ => {},
    }
}

fn reset_game_time(mut paused: ResMut<GamePaused>, mut clock: ResMut<GameClock>) {
    paused.0 = false;
    *clock = GameClock::default();
}

fn advance_game_time(
    time: Res<Time>,
    state: Res<State<AppState>>,
    paused: Res<GamePaused>,
    mut clock: ResMut<GameClock>,
) {
    if !paused.0 && matches!(*state.get(), AppState::Map | AppState::EmptyScreen) {
        clock.advance(time.delta_secs());
    }
}

fn draw_menu(
    mut contexts: EguiContexts,
    mut clipboard: ResMut<EguiClipboard>,
    mut style_initialized: Local<bool>,
    state: Res<State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
    mut draft: ResMut<MenuDraft>,
    mut lobby: ResMut<LobbyPreview>,
    mut loading: ResMut<LoadingSequence>,
    mut game: ResMut<ActiveGame>,
    mut practice: ResMut<LocalPractice>,
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
            egui::FontData::from_static(include_bytes!("../assets/fonts/FiraSans-Bold.ttf")),
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
        let item = if loading.destination == AppState::Map && !map_view.is_loaded() {
            let item = map_view.load_progress().item;
            map_view.load_next(context);
            item
        } else if loading.destination == AppState::EmptyScreen {
            "Local game preview"
        } else {
            "Map ready"
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
            item
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
                AppState::MainMenu => main_menu(
                    ui,
                    &mut next,
                    &mut loading,
                    &mut game,
                    &mut practice,
                    &menu_audio,
                    &audio,
                    &assets,
                ),
                AppState::CreateGame => {
                    create_game(ui, &mut draft, &mut lobby, &mut next, &menu_audio, &audio, &assets)
                },
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

fn draw_loading_progress(
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

fn augustus_ui_style() -> egui::Style {
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

fn viewport_ui_scale(viewport: egui::Vec2) -> f32 {
    let relative =
        (viewport.x / WINDOW_WIDTH as f32).min(viewport.y / WINDOW_HEIGHT as f32).max(0.0);
    relative.sqrt().clamp(0.5, 1.25)
}

fn set_menu_layer_scale(
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

fn logical_content_rect(ui: &egui::Ui) -> egui::Rect {
    let transform = ui
        .ctx()
        .layer_transform_to_global(ui.layer_id())
        .unwrap_or(egui::emath::TSTransform::IDENTITY);
    transform.inverse().mul_rect(ui.ctx().content_rect())
}

fn main_menu_title(context: &egui::Context, viewport: egui::Vec2) -> std::sync::Arc<egui::Galley> {
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

fn main_menu_top(context: &egui::Context, viewport: egui::Vec2) -> f32 {
    let title_height = main_menu_title(context, viewport).size().y;
    let title_bottom = viewport.y * 0.17 + title_height * 0.5;
    let count = MAIN_MENU_ACTION_COUNT as f32;
    let actions_height = MENU_ACTION_HEIGHT * count + FORM_CARD_GAP * (count - 1.0);
    (viewport.y * 0.365).min(viewport.y - 96.0 - actions_height).max(title_bottom + 24.0)
}

fn connection_status_badge(ui: &mut egui::Ui) {
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

fn main_menu(
    ui: &mut egui::Ui,
    next: &mut NextState<AppState>,
    loading: &mut LoadingSequence,
    game: &mut ActiveGame,
    practice: &mut LocalPractice,
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
                    ("Local Practice", AppState::Map),
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
                        if destination == AppState::Map {
                            *game = ActiveGame::LocalPractice;
                            practice.start_new_game();
                            loading.begin(destination, next);
                        } else {
                            next.set(destination);
                        }
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

fn menu_form(ui: &mut egui::Ui, id: &str, title: &str, contents: impl FnOnce(&mut egui::Ui)) {
    ui.spacing_mut().item_spacing.y = 0.0;
    let height = (logical_content_rect(ui).height() - 128.0).max(160.0);
    ui.set_max_height(height);
    egui::ScrollArea::vertical()
        .id_salt(id)
        .auto_shrink([false, true])
        .max_height((height - MENU_ACTION_HEIGHT - FORM_ACTION_GAP).max(80.0))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading(egui::RichText::new(title).size(MENU_TITLE_TEXT_SIZE));
                ui.add_space(FORM_TITLE_GAP);
                ui.scope(|ui| {
                    ui.spacing_mut().item_spacing.y = FORM_CARD_GAP;
                    contents(ui);
                });
            });
        });
    ui.add_space(FORM_ACTION_GAP);
}

fn card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(egui::Color32::from_rgba_unmultiplied(31, 20, 18, 232))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(190, 143, 94, 115)))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(18, 12))
}

fn code_card_heading(
    ui: &mut egui::Ui,
    label: &str,
    tooltip: &str,
    copy_value: Option<&str>,
) -> bool {
    let mut copied = false;
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some(value) = copy_value {
                copied = copy_icon_button(ui, value, label);
            }
            code_info_icon(ui, tooltip);
            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.label(
                    egui::RichText::new(label.to_uppercase())
                        .size(MENU_CARD_LABEL_TEXT_SIZE)
                        .strong()
                        .color(GOLD),
                );
            });
        });
    });
    copied
}

fn copy_icon_button(ui: &mut egui::Ui, value: &str, label: &str) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(28.0, 24.0), egui::Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("Copy {label}"))
    });
    let icon_color = if response.is_pointer_button_down_on() {
        GOLD
    } else if response.hovered() || response.has_focus() {
        CREAM
    } else {
        MUTED_TEXT
    };
    let icon = egui::Rect::from_center_size(rect.center(), egui::vec2(16.0, 17.0));
    let back = egui::Rect::from_min_size(icon.min, egui::vec2(11.0, 12.0));
    let front = egui::Rect::from_min_size(icon.min + egui::vec2(5.0, 5.0), egui::vec2(11.0, 12.0));
    let stroke = egui::Stroke::new(1.5, icon_color);
    ui.painter().rect_stroke(back, 1.0, stroke, egui::StrokeKind::Inside);
    ui.painter().rect_stroke(front, 1.0, stroke, egui::StrokeKind::Inside);
    if response.clicked() {
        #[cfg(target_arch = "wasm32")]
        let _ = call_browser_clipboard_function("augustusCopyText", Some(value));
        ui.ctx().copy_text(value.to_owned());
    }
    response.clicked()
}

fn code_info_icon(ui: &mut egui::Ui, tooltip: &str) {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(28.0, 24.0), egui::Sense::hover());
    let center = rect.center();
    let color = if response.hovered() {
        CREAM
    } else {
        MUTED_TEXT
    };
    ui.painter().circle_stroke(center, 8.0, egui::Stroke::new(1.5, color));
    ui.painter().text(
        center,
        egui::Align2::CENTER_CENTER,
        "i",
        egui::FontId::proportional(14.0),
        color,
    );
    response.on_hover_ui(|ui| {
        ui.set_max_width(300.0);
        ui.label(egui::RichText::new(tooltip).size(MENU_TOOLTIP_TEXT_SIZE));
    });
}

fn form_option_card(
    ui: &mut egui::Ui,
    label: &str,
    tooltip: &str,
    contents: impl FnOnce(&mut egui::Ui),
) {
    let frame = card_frame();
    let inner_width = (ui.available_width() - frame.total_margin().sum().x).max(1.0);
    frame.show(ui, |ui| {
        ui.set_width(inner_width);
        ui.spacing_mut().item_spacing.y = 0.0;
        code_card_heading(ui, label, tooltip, None);
        ui.add_space(FORM_CARD_GAP);
        contents(ui);
    });
}

fn editable_form_card(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut String,
    hint: &str,
    tooltip: &str,
    char_limit: Option<usize>,
) {
    form_option_card(ui, label, tooltip, |ui| {
        let mut editor = egui::TextEdit::singleline(value)
            .id_salt(label)
            .horizontal_align(egui::Align::Center)
            .vertical_align(egui::Align::Center)
            .font(egui::FontId::proportional(MENU_CONTROL_TEXT_SIZE))
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
    });
}

fn menu_submit_pressed(ui: &egui::Ui) -> bool {
    ui.input_mut(|input| {
        let mut pressed = false;
        input.events.retain(|event| {
            if let egui::Event::Key {
                key: egui::Key::Enter,
                pressed: true,
                repeat,
                modifiers,
                ..
            } = event
            {
                if modifiers.is_none() {
                    pressed |= !repeat;
                    return false;
                }
            }
            true
        });
        pressed
    })
}

fn menu_paste_target_id() -> egui::Id {
    egui::Id::new("augustus_menu_paste_target")
}

fn request_menu_paste(ui: &egui::Ui, target: egui::Id) {
    #[cfg(target_arch = "wasm32")]
    if !call_browser_clipboard_function("augustusRequestPaste", None)
        .and_then(|result| result.as_bool())
        .unwrap_or(false)
    {
        return;
    }
    ui.ctx().data_mut(|data| data.insert_temp(menu_paste_target_id(), target));
}

fn apply_pending_menu_paste(context: &egui::Context, _clipboard: &mut EguiClipboard) {
    let Some(target) = context.data(|data| data.get_temp::<egui::Id>(menu_paste_target_id()))
    else {
        return;
    };

    #[cfg(not(target_arch = "wasm32"))]
    let result = Some(_clipboard.get_text().ok_or(()));
    #[cfg(target_arch = "wasm32")]
    let result = take_browser_paste_result();

    let Some(result) = result else {
        return;
    };
    context.data_mut(|data| data.remove::<egui::Id>(menu_paste_target_id()));
    let Ok(text) = result else {
        return;
    };
    context.memory_mut(|memory| memory.request_focus(target));
    context.input_mut(|input| input.events.push(egui::Event::Paste(text)));
}

#[cfg(target_arch = "wasm32")]
fn take_browser_paste_result() -> Option<Result<String, ()>> {
    let result = call_browser_clipboard_function("augustusTakePaste", None)?;
    if let Some(text) = result.as_string() {
        Some(Ok(text))
    } else if result.as_bool() == Some(false) {
        Some(Err(()))
    } else {
        None
    }
}

#[cfg(target_arch = "wasm32")]
fn call_browser_clipboard_function(
    name: &str,
    argument: Option<&str>,
) -> Option<wasm_bindgen::JsValue> {
    use wasm_bindgen::{JsCast, JsValue};
    let window = web_sys::window()?;
    let function = js_sys::Reflect::get(window.as_ref(), &JsValue::from_str(name))
        .ok()?
        .dyn_into::<js_sys::Function>()
        .ok()?;
    match argument {
        Some(argument) => function.call1(window.as_ref(), &JsValue::from_str(argument)).ok(),
        None => function.call0(window.as_ref()).ok(),
    }
}

fn create_game(
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

fn join_game(
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

fn resume_game(
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

fn lobby_code_card(ui: &mut egui::Ui, code: &str) -> bool {
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

fn show_lobby(
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

fn lobby_players_card(ui: &mut egui::Ui, lobby: &LobbyPreview) {
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

fn settings_screen(
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

fn game_menu(
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
fn game_settings_screen(
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

fn audio_choice_row(ui: &mut egui::Ui, sound: &mut MenuAudio, audio: &Audio, assets: &AssetServer) {
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

fn paint_audio_choice_icon(ui: &egui::Ui, rect: egui::Rect, label: &str) {
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

fn menu_button_metrics(ui: &egui::Ui) -> egui::Vec2 {
    egui::vec2(MENU_ACTION_WIDTH.min(ui.available_width()), MENU_ACTION_HEIGHT)
}

fn menu_button_pair(
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

fn menu_action_button(
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

fn apply_menu_style(ui: &mut egui::Ui) {
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

fn play_click(sound: &MenuAudio, audio: &Audio, assets: &AssetServer) {
    if sound.mode == AudioMode::Mute || sound.volume <= 0.001 {
        return;
    }
    let decibels = -8.0 + 20.0 * sound.volume.clamp(0.001, 1.0).log10();
    audio.play(assets.load("audio/ui-click.ogg")).with_volume(decibels);
}

fn draw_audio_controls(
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

fn draw_map_hud(
    mut contexts: EguiContexts,
    mut standard_textures: Local<Option<[Option<egui::TextureHandle>; 6]>>,
    state: Res<State<AppState>>,
    game: Res<ActiveGame>,
    practice: Res<LocalPractice>,
    lobby: Res<LobbyPreview>,
    mut paused: ResMut<GamePaused>,
    mut clock: ResMut<GameClock>,
    mut next: ResMut<NextState<AppState>>,
    mut sound: ResMut<MenuAudio>,
    audio: Res<Audio>,
    assets: Res<AssetServer>,
) {
    let Ok(context) = contexts.ctx_mut() else {
        return;
    };
    let scale = viewport_ui_scale(context.content_rect().size());
    let color_index = if *game == ActiveGame::LobbyPreview {
        lobby.color_index.min(PLAYER_STANDARDS.len() - 1)
    } else {
        practice.color_index
    };
    let standards = standard_textures.get_or_insert_with(|| std::array::from_fn(|_| None));
    let standard =
        standards[color_index].get_or_insert_with(|| load_player_standard(context, color_index));
    paint_map_edge_frame(context, scale, standard, &clock, paused.0);
    if matches!(*state.get(), AppState::Map | AppState::EmptyScreen) {
        draw_map_menu_hitboxes(context, scale);
        if map_resource_strip_visible(context.content_rect(), scale) {
            if map_date_hitbox(context, scale, "minus", 8.0, 24.0, "Slow down").clicked() {
                let previous = clock.speed_step;
                clock.change_speed(-1);
                if clock.speed_step != previous {
                    play_click(&sound, &audio, &assets);
                }
            }
            if map_date_hitbox(context, scale, "date", 39.0, 144.0, "Pause or resume").clicked() {
                paused.0 = !paused.0;
                play_click(&sound, &audio, &assets);
            }
            if map_date_hitbox(context, scale, "plus", 190.0, 24.0, "Speed up").clicked() {
                let previous = clock.speed_step;
                clock.change_speed(1);
                if clock.speed_step != previous {
                    play_click(&sound, &audio, &assets);
                }
            }
        }
    }
    let responses = egui::Area::new(egui::Id::new("augustus_map_hud"))
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-12.0, 5.0) * scale)
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
}

const MAP_STANDARD_WIDTH: f32 = 146.0;
const MAP_STANDARD_RAIL_IMAGE_WIDTH: f32 = 120.0;
const MAP_STANDARD_RAIL_WIDTH: f32 = 64.0;
const MAP_STANDARD_FLAG_HEIGHT: f32 = 245.0;
const MAP_RESOURCE_STRIP_LEFT: f32 = MAP_STANDARD_WIDTH - 2.0;
const MAP_DATE_SECTION_WIDTH: f32 = 230.0;

fn map_standard_height(screen: egui::Rect, scale: f32) -> f32 {
    (screen.height() / scale - 35.0).clamp(300.0, 842.0)
}

fn map_resource_strip_right(screen: egui::Rect, scale: f32) -> f32 {
    (screen.width() / scale - 170.0).max(310.0)
}

fn map_resource_strip_visible(screen: egui::Rect, scale: f32) -> bool {
    map_resource_strip_right(screen, scale) > MAP_RESOURCE_STRIP_LEFT + MAP_DATE_SECTION_WIDTH
}

pub(crate) fn map_hud_contains(screen: egui::Rect, pointer: egui::Pos2) -> bool {
    if !screen.contains(pointer) {
        return false;
    }
    let scale = viewport_ui_scale(screen.size());
    let local = (pointer - screen.min) / scale;
    (local.x < MAP_STANDARD_WIDTH && local.y < MAP_STANDARD_FLAG_HEIGHT)
        || (local.x < MAP_STANDARD_RAIL_WIDTH && local.y < map_standard_height(screen, scale))
        || (map_resource_strip_visible(screen, scale)
            && (MAP_RESOURCE_STRIP_LEFT..map_resource_strip_right(screen, scale))
                .contains(&local.x)
            && local.y < 37.0)
}

fn draw_map_menu_hitboxes(ctx: &egui::Context, scale: f32) {
    let screen = ctx.content_rect();
    let rail_height = map_standard_height(screen, scale) - MAP_STANDARD_FLAG_HEIGHT;
    let strip_right = map_resource_strip_right(screen, scale);
    let date_left = strip_right - MAP_DATE_SECTION_WIDTH;
    for (id, top, width, height) in [
        ("augustus_standard_flag_hitbox", 0.0, MAP_STANDARD_WIDTH, MAP_STANDARD_FLAG_HEIGHT),
        (
            "augustus_standard_rail_hitbox",
            MAP_STANDARD_FLAG_HEIGHT,
            MAP_STANDARD_RAIL_WIDTH,
            rail_height,
        ),
    ] {
        egui::Area::new(egui::Id::new(id))
            .fixed_pos(screen.min + egui::vec2(0.0, top) * scale)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let (_, response) = ui.allocate_exact_size(
                    egui::vec2(width, height) * scale,
                    egui::Sense::click_and_drag(),
                );
                response.on_hover_cursor(egui::CursorIcon::Default);
            });
    }
    if map_resource_strip_visible(screen, scale) {
        egui::Area::new(egui::Id::new("augustus_resource_strip_hitbox"))
            .fixed_pos(screen.min + egui::vec2(MAP_RESOURCE_STRIP_LEFT, 0.0) * scale)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let (_, response) = ui.allocate_exact_size(
                    egui::vec2(date_left - MAP_RESOURCE_STRIP_LEFT, 37.0) * scale,
                    egui::Sense::click_and_drag(),
                );
                response.on_hover_cursor(egui::CursorIcon::Default);
            });
    }
}

fn map_date_hitbox(
    ctx: &egui::Context,
    scale: f32,
    id: &'static str,
    left: f32,
    width: f32,
    label: &'static str,
) -> egui::Response {
    let screen = ctx.content_rect();
    let date_left = map_resource_strip_right(screen, scale) - MAP_DATE_SECTION_WIDTH;
    egui::Area::new(egui::Id::new(("augustus_map_date", id)))
        .fixed_pos(screen.min + egui::vec2(date_left + left, 3.0) * scale)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let (_, response) =
                ui.allocate_exact_size(egui::vec2(width, 30.0) * scale, egui::Sense::click());
            response
                .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
            response.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(label)
        })
        .inner
}

fn load_player_standard(ctx: &egui::Context, color_index: usize) -> egui::TextureHandle {
    let (name, png) = PLAYER_STANDARDS[color_index];
    let mut rgba =
        image::load_from_memory(png).expect("player standard PNG must be valid").to_rgba8();
    let max_side = ctx.input(|input| input.max_texture_side).max(1) as u32;
    if rgba.width() > max_side || rgba.height() > max_side {
        let fraction = (max_side as f32 / rgba.width().max(rgba.height()) as f32).min(1.0);
        let width = (rgba.width() as f32 * fraction).floor().max(1.0) as u32;
        let height = (rgba.height() as f32 * fraction).floor().max(1.0) as u32;
        rgba = image::imageops::resize(&rgba, width, height, image::imageops::FilterType::Lanczos3);
    }
    ctx.load_texture(
        name,
        egui::ColorImage::from_rgba_unmultiplied(
            [rgba.width() as usize, rgba.height() as usize],
            rgba.as_raw(),
        ),
        egui::TextureOptions::LINEAR,
    )
}

// The edge art stays on the foreground layer while the map uses the full viewport.
fn paint_map_edge_frame(
    ctx: &egui::Context,
    scale: f32,
    standard: &egui::TextureHandle,
    clock: &GameClock,
    paused: bool,
) {
    let screen = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("augustus_map_edge_frame"),
    ));
    let p = |x: f32, y: f32| screen.min + egui::vec2(x, y) * scale;

    // Keep the segmented parchment strip clear for future resources, with the
    // date in its final section.
    let strip_right = map_resource_strip_right(screen, scale);
    let date_left = strip_right - MAP_DATE_SECTION_WIDTH;
    if map_resource_strip_visible(screen, scale) {
        painter.add(egui::Shape::convex_polygon(
            vec![
                p(MAP_RESOURCE_STRIP_LEFT, 0.0),
                p(strip_right, 0.0),
                p(strip_right - 20.0, 37.0),
                p(MAP_RESOURCE_STRIP_LEFT, 37.0),
            ],
            egui::Color32::from_rgb(236, 231, 216),
            egui::Stroke::new(scale, egui::Color32::from_rgb(144, 133, 110)),
        ));
        painter.line_segment(
            [p(MAP_RESOURCE_STRIP_LEFT + 1.0, 34.0), p(strip_right - 22.0, 34.0)],
            egui::Stroke::new(scale, egui::Color32::from_rgb(184, 164, 129)),
        );
        for n in 1..((date_left - MAP_RESOURCE_STRIP_LEFT) / 111.0) as usize {
            let x = MAP_RESOURCE_STRIP_LEFT + n as f32 * 111.0;
            painter.line_segment(
                [p(x, 3.0), p(x, 32.0)],
                egui::Stroke::new(scale, egui::Color32::from_rgb(192, 183, 164)),
            );
        }
        painter.line_segment(
            [p(date_left, 3.0), p(date_left, 32.0)],
            egui::Stroke::new(scale, egui::Color32::from_rgb(192, 183, 164)),
        );
        painter.text(
            p(date_left + 125.0, 13.0),
            egui::Align2::CENTER_CENTER,
            clock.date_label(),
            egui::FontId::proportional(15.0 * scale),
            egui::Color32::from_rgb(35, 35, 32),
        );
        painter.text(
            p(date_left + 125.0, 28.0),
            egui::Align2::CENTER_CENTER,
            clock.speed_label(),
            egui::FontId::proportional(10.0 * scale),
            egui::Color32::from_rgb(104, 88, 69),
        );
        for (offset, symbol, available) in [
            (20.0, "−", clock.speed_step > MIN_SPEED_STEP),
            (202.0, "+", clock.speed_step < MAX_SPEED_STEP),
        ] {
            let center = p(date_left + offset, 18.0);
            painter.rect_filled(
                egui::Rect::from_center_size(center, egui::vec2(22.0, 22.0) * scale),
                3.0 * scale,
                egui::Color32::from_rgb(227, 216, 196),
            );
            painter.rect_stroke(
                egui::Rect::from_center_size(center, egui::vec2(22.0, 22.0) * scale),
                3.0 * scale,
                egui::Stroke::new(scale, egui::Color32::from_rgb(156, 139, 110)),
                egui::StrokeKind::Inside,
            );
            painter.text(
                center,
                egui::Align2::CENTER_CENTER,
                symbol,
                egui::FontId::proportional(18.0 * scale),
                if available {
                    egui::Color32::from_rgb(35, 35, 32)
                } else {
                    egui::Color32::from_rgb(156, 139, 110)
                },
            );
        }
        if paused {
            let center = p(date_left + 50.0, 18.0);
            for offset in [-3.0, 2.0] {
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        center + egui::vec2(offset, -5.0) * scale,
                        egui::vec2(2.0, 10.0) * scale,
                    ),
                    0.5 * scale,
                    egui::Color32::BLACK,
                );
            }
        }
    }

    // Each player color has its own transparent image of the entire standard
    // and rail. No menu glyphs are painted over the cloth.
    let height = map_standard_height(screen, scale);
    let mut mesh = egui::Mesh::with_texture(standard.id());
    let rows = [
        (20.0, MAP_STANDARD_WIDTH),
        (300.0, MAP_STANDARD_WIDTH),
        (440.0, MAP_STANDARD_RAIL_IMAGE_WIDTH),
        (1684.0, MAP_STANDARD_RAIL_IMAGE_WIDTH),
    ];
    for (source_y, width) in rows {
        let y = (source_y - 20.0) / (1684.0 - 20.0) * height;
        for (x, u) in [(0.0, 7.0 / 220.0), (width, 1.0)] {
            mesh.vertices.push(egui::epaint::Vertex {
                pos: p(x, y),
                uv: egui::pos2(u, source_y / 1684.0),
                color: egui::Color32::WHITE,
            });
        }
    }
    for row in 0..3 {
        let top = row * 2;
        mesh.indices.extend_from_slice(&[top, top + 1, top + 2, top + 1, top + 3, top + 2]);
    }
    painter.add(egui::Shape::Mesh(mesh.into()));
}

fn settings_button_destination(state: AppState, game: ActiveGame) -> AppState {
    if state == AppState::GameSettings {
        game.screen()
    } else {
        AppState::GameSettings
    }
}

fn settings_button(ui: &mut egui::Ui, scale: f32) -> egui::Response {
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

fn circular_audio_button(ui: &mut egui::Ui, scale: f32) -> egui::Response {
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

fn audio_mode_button(ui: &mut egui::Ui, mode: AudioMode, scale: f32) -> egui::Response {
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

fn volume_slider(ui: &mut egui::Ui, sound: &mut MenuAudio) -> egui::Response {
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

fn volume_popover(button: &egui::Response, sound: &mut MenuAudio) {
    let id = button.id.with("volume");
    let was_open = button.ctx.data(|data| data.get_temp::<bool>(id).unwrap_or(false));
    let popup = egui::Popup::from_response(button)
        .id(id)
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

const PLAYER_STANDARDS: [(&str, &[u8]); 6] = [
    ("player-standard-red", include_bytes!("../assets/images/ui/player-standard-red.png")),
    ("player-standard-blue", include_bytes!("../assets/images/ui/player-standard-blue.png")),
    ("player-standard-green", include_bytes!("../assets/images/ui/player-standard-green.png")),
    ("player-standard-gold", include_bytes!("../assets/images/ui/player-standard-gold.png")),
    ("player-standard-purple", include_bytes!("../assets/images/ui/player-standard-purple.png")),
    ("player-standard-orange", include_bytes!("../assets/images/ui/player-standard-orange.png")),
];

const PLAYER_COLORS: [egui::Color32; 6] = [
    egui::Color32::from_rgb(158, 57, 45),
    egui::Color32::from_rgb(56, 105, 133),
    egui::Color32::from_rgb(93, 132, 78),
    egui::Color32::from_rgb(195, 161, 72),
    egui::Color32::from_rgb(122, 85, 147),
    egui::Color32::from_rgb(202, 105, 51),
];

/// Screen-size dependent menu window resolution.
pub fn window_resolution() -> WindowResolution {
    WindowResolution::new(WINDOW_WIDTH, WINDOW_HEIGHT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_hud_hit_region_covers_visible_frame() {
        const { assert!(MAP_STANDARD_WIDTH >= MAP_STANDARD_RAIL_IMAGE_WIDTH * 1.1) };
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 900.0));
        assert!(map_hud_contains(screen, egui::pos2(110.0, 40.0)));
        assert!(map_hud_contains(screen, egui::pos2(140.0, 100.0)));
        assert!(map_hud_contains(screen, egui::pos2(40.0, 600.0)));
        assert!(map_hud_contains(screen, egui::pos2(500.0, 20.0)));
        assert!(!map_hud_contains(screen, egui::pos2(100.0, 600.0)));
        assert!(!map_hud_contains(screen, egui::pos2(500.0, 100.0)));
        assert!(!map_hud_contains(screen, egui::pos2(40.0, 870.0)));
    }

    #[test]
    fn all_player_standards_load_with_eguis_texture_limits() {
        for limit in [2048, 1024] {
            let context = egui::Context::default();
            context.begin_pass(egui::RawInput {
                max_texture_side: Some(limit),
                ..Default::default()
            });
            for color_index in 0..PLAYER_STANDARDS.len() {
                let texture = load_player_standard(&context, color_index);
                assert!(texture.size()[0] <= limit);
                assert!(texture.size()[1] <= limit);
            }
            let mut output = context.end_pass();
            output.textures_delta.clear();
        }
    }

    #[test]
    fn escape_opens_and_closes_the_correct_game_menu() {
        for (game, screen) in [
            (ActiveGame::LocalPractice, AppState::Map),
            (ActiveGame::LobbyPreview, AppState::EmptyScreen),
        ] {
            assert_eq!(escape_destination(screen, game), Some(AppState::GameMenu));
            assert_eq!(escape_destination(AppState::GameMenu, game), Some(screen));
            assert_eq!(escape_destination(AppState::GameSettings, game), Some(AppState::GameMenu));
        }
    }

    #[test]
    fn settings_button_closes_settings_to_the_game() {
        for (game, screen) in [
            (ActiveGame::LocalPractice, AppState::Map),
            (ActiveGame::LobbyPreview, AppState::EmptyScreen),
        ] {
            assert_eq!(settings_button_destination(screen, game), AppState::GameSettings);
            assert_eq!(
                settings_button_destination(AppState::GameMenu, game),
                AppState::GameSettings
            );
            assert_eq!(settings_button_destination(AppState::GameSettings, game), screen);
        }
    }

    #[test]
    fn game_clock_advances_months_at_selected_speed() {
        let mut clock = GameClock::default();
        assert_eq!(clock.date_label(), "Oct, 450 BC");
        assert_eq!(clock.speed_label(), "1×");
        clock.advance(1.5);
        assert_eq!(clock.date_label(), "Oct, 450 BC");
        clock.advance(1.5);
        assert_eq!(clock.date_label(), "Nov, 450 BC");

        clock.change_speed(1);
        clock.advance(1.5);
        assert_eq!(clock.date_label(), "Dec, 450 BC");
        clock.change_speed(-10);
        assert_eq!(clock.speed_label(), "0.25×");
        clock.advance(12.0);
        assert_eq!(clock.date_label(), "Jan, 449 BC");
        clock.change_speed(10);
        assert_eq!(clock.speed_label(), "8×");
        clock.advance(0.375);
        assert_eq!(clock.date_label(), "Feb, 449 BC");
    }

    #[test]
    fn game_clock_skips_year_zero() {
        let mut clock = GameClock {
            year: 0,
            month: 11,
            ..Default::default()
        };
        assert_eq!(clock.date_label(), "Dec, 1 BC");
        clock.advance(3.0);
        assert_eq!(clock.date_label(), "Jan, 1 AD");
    }
}
