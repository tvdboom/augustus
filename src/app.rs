//! Augustus application state, shared resources, and system registration.

#[path = "ui/city_panel.rs"]
mod city_panel;
#[path = "ui/coin_panel.rs"]
mod coin_panel;
#[path = "ui/flow_panel.rs"]
mod flow_panel;
#[path = "ui/governance_panel.rs"]
mod governance_panel;
#[path = "ui/happiness_panel.rs"]
mod happiness_panel;
#[path = "ui/influence_panel.rs"]
mod influence_panel;
#[path = "ui/population_panel.rs"]
mod population_panel;
#[path = "ui/province_panel.rs"]
mod province_panel;
#[path = "ui/resource_panel.rs"]
mod resource_panel;
#[path = "ui/toasts.rs"]
mod toasts;

#[path = "ui/audio_controls.rs"]
mod audio_controls;
#[path = "game/controls.rs"]
mod game_controls;
#[path = "ui/hud.rs"]
mod hud;
#[path = "ui/map_edge.rs"]
mod map_edge;
#[path = "ui/map_menu.rs"]
mod map_menu;
#[path = "menu/audio.rs"]
mod menu_audio;
#[path = "menu/background.rs"]
mod menu_background;
#[path = "menu/controls.rs"]
mod menu_controls;
#[path = "menu/forms.rs"]
mod menu_forms;
#[path = "menu/render.rs"]
mod menu_render;
#[path = "menu/screens.rs"]
mod menu_screens;
#[path = "ui/rank_hud.rs"]
mod rank_hud;
#[path = "ui/resource_hud.rs"]
mod resource_hud;
#[path = "game/resources.rs"]
mod resource_simulation;

use game_controls::*;
use menu_audio::*;

use audio_controls::*;
use hud::*;
use map_edge::*;
pub(crate) use map_menu::map_hud_contains;
use map_menu::*;
use menu_background::*;
use menu_controls::*;
use menu_forms::*;
use menu_render::*;
use menu_screens::*;
use rank_hud::*;
use resource_hud::*;
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use bevy::asset::LoadedFolder;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::WindowResolution;
use bevy_egui::{
    egui, EguiClipboard, EguiContexts, EguiPlugin, EguiPrimaryContextPass, EguiTextureHandle,
};
use bevy_kira_audio::prelude::*;
use egui::epaint::text::{FontInsert, FontPriority, InsertFontFamily};
use rand::random_range;

use crate::basis_texture::BasisTexturePlugin;
use crate::map::{draw_map, Governance, MapView, ProvinceOwnership};
use crate::multiplayer::lobby::{generate_game_code, LobbyPreview};
use crate::TITLE;

/// Native and initial browser width for the menu canvas.
pub const WINDOW_WIDTH: u32 = 1600;
/// Native and initial browser height for the menu canvas.
pub const WINDOW_HEIGHT: u32 = 900;
const MAP_CORNER_CONTROLS_TOP_FRACTION: f32 = 0.012;
const MAP_CORNER_PANEL_BOTTOM_FRACTION: f32 = 0.01;

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
const LOADING_WALLPAPER_FADE_IN_SECONDS: f32 = 0.12;
const LOADING_MAP_REVEAL_SECONDS: f32 = 0.8;

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
    /// Local-practice setup before entering the map.
    PracticeSetup,
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

#[derive(Resource)]
struct LocalPractice {
    color_index: usize,
    player_count: usize,
    active_player: usize,
    players: Vec<PracticePlayer>,
}

#[derive(SystemParam)]
struct PracticeSetupParams<'w> {
    practice: ResMut<'w, LocalPractice>,
    ownership: ResMut<'w, ProvinceOwnership>,
}

impl Default for LocalPractice {
    fn default() -> Self {
        Self {
            color_index: 0,
            player_count: 2,
            active_player: 0,
            players: Vec::new(),
        }
    }
}

#[derive(Clone)]
struct PracticePlayer {
    color_index: usize,
    rank: usize,
}

impl LocalPractice {
    fn start_new_game(&mut self, ownership: &mut ProvinceOwnership) {
        self.player_count = self.player_count.clamp(1, 4);
        self.color_index = self.color_index.min(PLAYER_COLORS.len() - 1);
        self.active_player = 0;
        self.players = (0..self.player_count)
            .map(|index| PracticePlayer {
                color_index: (self.color_index + index) % PLAYER_COLORS.len(),
                rank: 0,
            })
            .collect();
        let colors: Vec<_> =
            self.players.iter().map(|player| PLAYER_COLORS[player.color_index]).collect();
        ownership.start_game(&colors);
        let map_colors: Vec<_> =
            self.players.iter().map(|player| MAP_PLAYER_COLORS[player.color_index]).collect();
        ownership.set_map_colors(&map_colors);
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
    map_reveal_progress: Option<f32>,
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
            map_reveal_progress: None,
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
        self.map_reveal_progress = None;
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

#[derive(Resource, Default)]
pub(crate) struct GovernancePanelOpen(pub(crate) bool);

#[derive(Resource, Default)]
pub(crate) struct ProvincePanelOpen(pub(crate) Option<MapDetail>);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MapDetail {
    Province(usize),
    City(usize),
}

#[derive(Resource, Default)]
pub(crate) struct MapPanelCloseClick(pub(crate) bool);

#[derive(SystemParam)]
struct MapPanelParams<'w> {
    governance: ResMut<'w, GovernancePanelOpen>,
    province: ResMut<'w, ProvincePanelOpen>,
    close_click: ResMut<'w, MapPanelCloseClick>,
}

const SECONDS_PER_MONTH: f32 = 3.0;
const MIN_SPEED_STEP: i8 = -2;
const MAX_SPEED_STEP: i8 = 2;
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
            year: 60,
            month: 0,
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

    fn advance(&mut self, seconds: f32) -> usize {
        let mut elapsed_months = 0;
        self.month_progress += seconds * self.speed();
        while self.month_progress >= SECONDS_PER_MONTH {
            self.month_progress -= SECONDS_PER_MONTH;
            self.month += 1;
            elapsed_months += 1;
            if self.month == MONTH_NAMES.len() {
                self.month = 0;
                self.year += 1;
            }
        }
        elapsed_months
    }

    fn date_label(&self) -> String {
        let (year, era) = if self.year <= 0 {
            (1 - self.year, "BC")
        } else {
            (self.year, "AD")
        };
        format!("{}, {year} {era}", MONTH_NAMES[self.month])
    }
}

const HUD_RESOURCE_NAMES: [&str; 7] =
    ["Food", "Metal", "Stone", "Sestertius", "Influence", "Population", "Happiness"];
const POP_CLASS_NAMES: [&str; 4] = ["Nobles", "Citizens", "Plebeians", "Slaves"];
const HUD_RESOURCE_WIDTH: f32 = 104.0;
const HUD_RESOURCE_GROUP_PADDING: f32 = 12.0;
const HUD_FIRST_GROUP_WIDTH: f32 = HUD_RESOURCE_WIDTH * 3.0 + HUD_RESOURCE_GROUP_PADDING * 2.0;
const HUD_SECOND_GROUP_WIDTH: f32 = HUD_RESOURCE_WIDTH * 2.0 + HUD_RESOURCE_GROUP_PADDING * 2.0;
const HUD_THIRD_GROUP_WIDTH: f32 = HUD_RESOURCE_WIDTH * 2.0 + HUD_RESOURCE_GROUP_PADDING * 2.0;

#[derive(Clone, Copy)]
struct HudResource {
    amount: f64,
    monthly_delta: f64,
}

#[derive(Resource)]
struct HudResources {
    players: Vec<[HudResource; 7]>,
    happiness: Vec<[HudResource; 4]>,
    happiness_edict_delta: Vec<[f64; 4]>,
    famine_months: Vec<u32>,
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
            .init_resource::<HudResources>()
            .init_resource::<GovernancePanelOpen>()
            .init_resource::<ProvincePanelOpen>()
            .init_resource::<MapPanelCloseClick>()
            .init_resource::<LobbyPreview>()
            .init_resource::<LoadingSequence>()
            .init_resource::<ActiveGame>()
            .init_resource::<LocalPractice>()
            .init_resource::<MapView>()
            .init_resource::<ProvinceOwnership>()
            .init_resource::<toasts::ToastQueue>()
            .init_resource::<toasts::WarningWatch>()
            .add_systems(Startup, (setup_camera, setup_background, start_music).chain())
            .add_systems(OnEnter(AppState::Loading), (start_loading_wallpaper, reset_game_time))
            .add_systems(
                Update,
                (advance_loading_wallpaper, fit_menu_background, set_menu_background_visibility)
                    .chain(),
            )
            .add_systems(Update, handle_escape)
            .add_systems(Update, handle_game_shortcuts)
            .add_systems(
                Update,
                (
                    advance_game_time,
                    toasts::watch_warnings,
                    toasts::play_pending_sounds,
                    toasts::advance,
                )
                    .chain(),
            )
            .add_systems(
                EguiPrimaryContextPass,
                draw_menu
                    .run_if(not(in_state(AppState::Map).or_else(in_state(AppState::EmptyScreen)))),
            )
            .add_systems(EguiPrimaryContextPass, draw_audio_controls.run_if(audio_controls_visible))
            .add_systems(
                EguiPrimaryContextPass,
                (draw_map_hud, draw_governance_panel, draw_province_panel, draw_map_resources)
                    .chain()
                    .run_if(game_ui_visible),
            )
            .add_systems(
                EguiPrimaryContextPass,
                draw_map.run_if(map_visible).after(draw_map_resources),
            )
            .add_systems(EguiPrimaryContextPass, toasts::draw.after(draw_map_resources))
            .add_systems(
                EguiPrimaryContextPass,
                draw_loading_reveal.run_if(in_state(AppState::Loading)),
            )
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

const PLAYER_STANDARDS: [(&str, &[u8]); 6] = [
    ("player-standard-red", include_bytes!("../assets/images/ui/player-standard-red.png")),
    ("player-standard-rose", include_bytes!("../assets/images/ui/player-standard-rose.png")),
    ("player-standard-green", include_bytes!("../assets/images/ui/player-standard-green.png")),
    ("player-standard-gold", include_bytes!("../assets/images/ui/player-standard-gold.png")),
    ("player-standard-purple", include_bytes!("../assets/images/ui/player-standard-purple.png")),
    ("player-standard-orange", include_bytes!("../assets/images/ui/player-standard-orange.png")),
];

const RANK_ICONS: [&[u8]; 6] = [
    include_bytes!("../assets/images/ui/ranks/rank-quaestor.png"),
    include_bytes!("../assets/images/ui/ranks/rank-aedile.png"),
    include_bytes!("../assets/images/ui/ranks/rank-praetor.png"),
    include_bytes!("../assets/images/ui/ranks/rank-censor.png"),
    include_bytes!("../assets/images/ui/ranks/rank-consul.png"),
    include_bytes!("../assets/images/ui/ranks/rank-augustus.png"),
];

const RANK_SCEPTER: &[u8] = include_bytes!("../assets/images/ui/ranks/rank-scepter-v2.png");

const HUD_RESOURCE_ICONS: [&[u8]; 7] = [
    include_bytes!("../assets/images/icons/food.png"),
    include_bytes!("../assets/images/icons/metal.png"),
    include_bytes!("../assets/images/icons/stone.png"),
    include_bytes!("../assets/images/icons/sestertius.png"),
    include_bytes!("../assets/images/icons/influence.png"),
    include_bytes!("../assets/images/icons/manpower.png"),
    include_bytes!("../assets/images/icons/happiness.png"),
];

const POP_CLASS_ICONS: [&[u8]; 4] = [
    include_bytes!("../assets/images/icons/nobles.png"),
    include_bytes!("../assets/images/icons/civilians.png"),
    include_bytes!("../assets/images/icons/plebeians.png"),
    include_bytes!("../assets/images/icons/slaves.png"),
];

const RANK_NAMES_ASSET: [&str; 6] =
    ["rank-quaestor", "rank-aedile", "rank-praetor", "rank-censor", "rank-consul", "rank-augustus"];

const PLAYER_COLORS: [egui::Color32; 6] = [
    // Sampled from plain cloth at (80, 700) in the standards above. These
    // values drive the banner bar, province header, and player markers.
    egui::Color32::from_rgb(112, 18, 17),
    egui::Color32::from_rgb(109, 36, 55),
    egui::Color32::from_rgb(79, 113, 66),
    egui::Color32::from_rgb(107, 77, 24),
    egui::Color32::from_rgb(105, 72, 126),
    egui::Color32::from_rgb(175, 90, 43),
];

// The rose banner cloth is too close to red under the map's translucent
// ownership wash. Only its map tint is brighter; its panel and marker colors
// still use the exact cloth sample above.
const MAP_PLAYER_COLORS: [egui::Color32; 6] = [
    PLAYER_COLORS[0],
    egui::Color32::from_rgb(225, 76, 158),
    PLAYER_COLORS[2],
    PLAYER_COLORS[3],
    PLAYER_COLORS[4],
    PLAYER_COLORS[5],
];

/// Screen-size dependent menu window resolution.
pub fn window_resolution() -> WindowResolution {
    WindowResolution::new(WINDOW_WIDTH, WINDOW_HEIGHT)
}

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
