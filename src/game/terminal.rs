//! Presentation state for a finished local campaign and its read-only spectator view.

use super::*;

const FADE_SECONDS: f32 = 0.65;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TerminalOutcome {
    Victory,
    Defeat,
}

#[derive(Default, Resource)]
pub(crate) struct TerminalPresentation {
    pub outcome: Option<TerminalOutcome>,
    pub spectating: bool,
    pub fade_elapsed: f32,
}

impl TerminalPresentation {
    pub fn opacity(&self) -> f32 {
        let progress = (self.fade_elapsed / FADE_SECONDS).clamp(0.0, 1.0);
        progress * progress * (3.0 - 2.0 * progress)
    }
}

fn local_outcome(campaign: &campaign::Campaign, player: usize) -> Option<TerminalOutcome> {
    if campaign.defeated.get(player).copied().unwrap_or(false) {
        Some(TerminalOutcome::Defeat)
    } else {
        campaign.senate.winner.map(|winner| {
            if winner == player {
                TerminalOutcome::Victory
            } else {
                TerminalOutcome::Defeat
            }
        })
    }
}

/// Resolve the local player's result once, before any more commands can be issued.
pub(super) fn detect_terminal(
    state: Res<State<AppState>>,
    campaign: Res<campaign::Campaign>,
    practice: Res<LocalPractice>,
    mut terminal: ResMut<TerminalPresentation>,
    mut next: ResMut<NextState<AppState>>,
    mut paused: ResMut<GamePaused>,
    game: Res<ActiveGame>,
    sound: Res<MenuAudio>,
    audio: Res<Audio>,
    assets: Res<AssetServer>,
) {
    if *state.get() != AppState::Map || !campaign.active || terminal.outcome.is_some() {
        return;
    }
    let outcome = local_outcome(&campaign, practice.active_player);
    let Some(outcome) = outcome else {
        return;
    };
    terminal.outcome = Some(outcome);
    terminal.fade_elapsed = 0.0;
    if *game != ActiveGame::Online || campaign.senate.winner.is_some() {
        paused.0 = true;
    }
    next.set(AppState::EndGame);
    if sound.mode != AudioMode::Mute && sound.volume > 0.001 {
        let name = match outcome {
            TerminalOutcome::Victory => "victory",
            TerminalOutcome::Defeat => "defeat",
        };
        let decibels = -12.0 + 20.0 * sound.volume.clamp(0.001, 1.0).log10();
        audio.play(assets.load(format!("audio/{name}.ogg"))).with_volume(decibels);
    }
}

/// Keep inspection available after Spectate, while removing the old player controls.
pub(super) fn prepare_spectator(
    mut contexts: Query<&mut bevy_egui::EguiContext, With<bevy_egui::PrimaryEguiContext>>,
    terminal: Res<TerminalPresentation>,
    mut campaign_ui: ResMut<campaign_panel::CampaignUi>,
    mut governance: ResMut<GovernancePanelOpen>,
    mut province: ResMut<ProvincePanelOpen>,
    mut paused: ResMut<GamePaused>,
    mut toasts: ResMut<toasts::ToastQueue>,
    game: Res<ActiveGame>,
) {
    if !terminal.spectating {
        return;
    }
    *campaign_ui = campaign_panel::CampaignUi::default();
    if let Ok(mut context) = contexts.single_mut() {
        while campaign_military::dismiss_army_panel(context.get_mut()) {}
        crate::map::take_army_click(context.get_mut());
        crate::map::take_battle_click(context.get_mut());
        crate::map::take_province_order_click(context.get_mut());
    }
    governance.0 = false;
    province.0 = None;
    if *game != ActiveGame::Online {
        paused.0 = false;
    }
    toasts.clear();
}

#[cfg(test)]
#[path = "../../tests/unit/terminal.rs"]
mod tests;
