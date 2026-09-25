//! Game keyboard shortcuts and clock systems.

use super::*;

pub(super) fn handle_escape(
    keyboard: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    game: Res<ActiveGame>,
    mut next: ResMut<NextState<AppState>>,
    mut panels: MapPanelParams,
) {
    if !keyboard.just_pressed(KeyCode::Escape) {
        return;
    }
    if dismiss_map_panel_on_escape(*state.get(), &mut panels.governance, &mut panels.province) {
        return;
    }
    if let Some(destination) = escape_destination(*state.get(), *game) {
        next.set(destination);
    }
}

pub(super) fn dismiss_map_panel_on_escape(
    state: AppState,
    governance: &mut GovernancePanelOpen,
    province: &mut ProvincePanelOpen,
) -> bool {
    if state != AppState::Map {
        return false;
    }
    if province.0.take().is_some() {
        return true;
    }
    std::mem::take(&mut governance.0)
}

pub(super) fn escape_destination(state: AppState, game: ActiveGame) -> Option<AppState> {
    match state {
        AppState::MainMenu => None,
        AppState::Map | AppState::EmptyScreen => Some(AppState::GameMenu),
        AppState::GameMenu => Some(game.screen()),
        AppState::GameSettings => Some(AppState::GameMenu),
        _ => Some(AppState::MainMenu),
    }
}

pub(super) fn handle_game_shortcuts(
    keyboard: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    game: Res<ActiveGame>,
    mut paused: ResMut<GamePaused>,
    mut clock: ResMut<GameClock>,
    mut next: ResMut<NextState<AppState>>,
) {
    if *state.get() == AppState::GameMenu
        && (keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::NumpadEnter))
    {
        next.set(game.screen());
        return;
    }
    if matches!(*state.get(), AppState::Map | AppState::EmptyScreen)
        && (keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight))
    {
        let speed_change = if keyboard.just_pressed(KeyCode::ArrowLeft) {
            Some(-1)
        } else if keyboard.just_pressed(KeyCode::ArrowRight) {
            Some(1)
        } else {
            None
        };
        if let Some(step) = speed_change {
            let previous = clock.speed_step;
            clock.change_speed(step);
            if clock.speed_step != previous {
                paused.0 = false;
            }
        }
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

pub(super) fn reset_game_time(
    mut paused: ResMut<GamePaused>,
    mut clock: ResMut<GameClock>,
    mut resources: ResMut<HudResources>,
    mut governance_open: ResMut<GovernancePanelOpen>,
    mut province_open: ResMut<ProvincePanelOpen>,
    mut toasts: ResMut<toasts::ToastQueue>,
    mut warning_watch: ResMut<toasts::WarningWatch>,
    game: Res<ActiveGame>,
    practice: Res<LocalPractice>,
    ownership: Res<ProvinceOwnership>,
) {
    paused.0 = false;
    governance_open.0 = false;
    province_open.0 = None;
    toasts.clear();
    *warning_watch = toasts::WarningWatch::default();
    *clock = GameClock::default();
    *resources = HudResources::default();
    if *game == ActiveGame::LocalPractice && !practice.players.is_empty() {
        resources.start_players(practice.players.len(), &ownership);
    }
}

pub(super) fn advance_game_time(
    time: Res<Time>,
    state: Res<State<AppState>>,
    paused: Res<GamePaused>,
    mut clock: ResMut<GameClock>,
    mut resources: ResMut<HudResources>,
    game: Res<ActiveGame>,
    mut ownership: ResMut<ProvinceOwnership>,
) {
    if !paused.0 && matches!(*state.get(), AppState::Map | AppState::EmptyScreen) {
        let months = clock.advance(time.delta_secs());
        if months > 0 {
            resources.advance(months, &mut ownership, *game == ActiveGame::LocalPractice);
        }
    }
}
