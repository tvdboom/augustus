//! Game keyboard shortcuts and clock systems.

use super::*;

pub(super) fn handle_escape(
    mut contexts: Query<&mut bevy_egui::EguiContext, With<bevy_egui::PrimaryEguiContext>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    game: Res<ActiveGame>,
    mut next: ResMut<NextState<AppState>>,
    mut panels: MapPanelParams,
) {
    if !keyboard.just_pressed(KeyCode::Escape) {
        return;
    }
    if *state.get() == AppState::EndGame {
        next.set(AppState::MainMenu);
        return;
    }
    if panels.campaign_ui.dismiss_confirmation() {
        return;
    }
    if *state.get() == AppState::Map
        && contexts
            .single_mut()
            .is_ok_and(|mut context| campaign_military::dismiss_army_panel(context.get_mut()))
    {
        panels.campaign_ui.open = None;
        panels.campaign_ui.close_province_selector();
        panels.province.0 = None;
        panels.governance.0 = false;
        return;
    }
    if panels.campaign_ui.close_province_selector() {
        return;
    }
    if panels.campaign_ui.open.take().is_some() {
        panels.province.0 = None;
        panels.governance.0 = false;
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
        AppState::EndGame => Some(AppState::MainMenu),
        _ => Some(AppState::MainMenu),
    }
}

pub(super) fn handle_game_shortcuts(
    mut contexts: EguiContexts,
    keyboard: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    game: Res<ActiveGame>,
    mut paused: ResMut<GamePaused>,
    mut clock: ResMut<GameClock>,
    mut next: ResMut<NextState<AppState>>,
    campaign_ui: Res<campaign_panel::CampaignUi>,
    practice: Res<LocalPractice>,
    mut campaign: ResMut<campaign::Campaign>,
    mut resources: ResMut<HudResources>,
    mut ownership: ResMut<ProvinceOwnership>,
    terminal: Res<TerminalPresentation>,
) {
    if terminal.spectating || *state.get() == AppState::EndGame {
        return;
    }
    if campaign_ui.confirmation_open() {
        return;
    }
    if contexts.ctx_mut().is_ok_and(|ctx| ctx.egui_wants_keyboard_input()) {
        return;
    }
    if *state.get() == AppState::GameMenu
        && (keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::NumpadEnter))
    {
        next.set(game.screen());
        return;
    }
    if matches!(*state.get(), AppState::Map | AppState::EmptyScreen)
        && keyboard.just_pressed(KeyCode::ArrowUp)
        && (keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight))
        && (keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight))
    {
        apply_practice_boost(practice.active_player, &mut campaign, &mut resources, &mut ownership);
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

pub(super) fn apply_practice_boost(
    player: usize,
    campaign: &mut campaign::Campaign,
    resources: &mut HudResources,
    ownership: &mut ProvinceOwnership,
) {
    if campaign.active {
        let player_count = campaign.economy.players.len();
        let Some(wallet) = campaign.economy.players.get_mut(player) else {
            return;
        };
        for index in 0..3 {
            wallet.resources[index] += 5_000.0;
            wallet.storage[index] += 5_000.0;
            wallet.practice_storage_bonus[index] += 5_000.0;
        }
        wallet.coin += 5_000.0;
        wallet.influence += 1_000.0;
        if let Some(actor) = campaign.actors.get_mut(player) {
            if let Some(requirement) = campaign.senate_config.requirements(actor.rank, player_count)
            {
                wallet.influence = wallet.influence.max(requirement.influence);
                campaign.senate.grant_practice_support(player, requirement.senators);
                actor.promoted_at = None;
                if requirement.rank == crate::game::politics::PoliticalRank::Consul {
                    actor.consul_again_at = campaign.senate.month;
                }
            }
        }
        let owner = crate::game::military::ForceOwner::Player(player);
        if let Some(requirements) =
            crate::game::military::MilitaryRank::Imperator.promotion_requirements()
        {
            let peak = campaign.military.peak_manpower.entry(owner).or_default();
            *peak = (*peak).max(requirements.peak_manpower);
            let victories = campaign.military.victories.entry(owner).or_default();
            *victories = (*victories).max(requirements.victories);
        }
        for (id, province) in campaign.economy.provinces.iter_mut().enumerate() {
            if province.owner == Some(player) {
                let previous_population = province.total_population();
                for population in &mut province.population {
                    *population *= 10.0;
                }
                let added_population = province.total_population() - previous_population;
                province.practice_capacity_bonus += added_population * 100.0;
                for kind in crate::game::military::UnitType::ALL {
                    for _ in 0..3 {
                        campaign
                            .military
                            .seed_unit(id, owner, kind)
                            .expect("owned campaign provinces have military state");
                    }
                }
            }
        }
        campaign.pull_wallets();
    } else {
        let Some(balances) = resources.players.get_mut(player) else {
            return;
        };
        for balance in &mut balances[..4] {
            balance.amount += 5_000.0;
        }
        balances[4].amount += 1_000.0;
        ownership.multiply_owned_population(player, 10.0);
        balances[5].amount = ownership.total_population_for(player);
        resources.refresh_player_rates(player, ownership);
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
    mut campaign: ResMut<campaign::Campaign>,
    mut campaign_ui: ResMut<campaign_panel::CampaignUi>,
    mut terminal: ResMut<TerminalPresentation>,
) {
    paused.0 = false;
    governance_open.0 = false;
    province_open.0 = None;
    toasts.clear();
    *warning_watch = toasts::WarningWatch::default();
    *clock = GameClock::default();
    *resources = HudResources::default();
    *campaign_ui = campaign_panel::CampaignUi::default();
    *terminal = TerminalPresentation::default();
    *campaign = campaign::Campaign::default();
    if *game == ActiveGame::LocalPractice && !practice.players.is_empty() {
        resources.start_players(practice.players.len(), &ownership);
        campaign.start(&ownership, practice.players.len());
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
    mut campaign: ResMut<campaign::Campaign>,
) {
    if !paused.0 && matches!(*state.get(), AppState::Map | AppState::EmptyScreen) {
        if campaign.active {
            let ticks = clock
                .advance_timeline(time.delta_secs(), campaign.military.config.rounds_per_month);
            for monthly in ticks {
                campaign.advance_live_combat();
                if monthly {
                    campaign.advance_live_month();
                }
            }
        } else {
            let months = clock.advance(time.delta_secs());
            resources.advance(months, &mut ownership, *game == ActiveGame::LocalPractice);
        }
    }
}
