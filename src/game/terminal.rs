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
    paused.0 = true;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_follows_the_local_players_victory_or_last_province_loss() {
        let mut campaign = campaign::Campaign::default();
        campaign.defeated = vec![false; 2];
        assert_eq!(local_outcome(&campaign, 0), None);
        campaign.senate.winner = Some(1);
        assert_eq!(local_outcome(&campaign, 0), Some(TerminalOutcome::Defeat));
        assert_eq!(local_outcome(&campaign, 1), Some(TerminalOutcome::Victory));
        campaign.defeated[1] = true;
        assert_eq!(local_outcome(&campaign, 1), Some(TerminalOutcome::Defeat));
    }

    #[test]
    fn result_opacity_starts_invisible_and_finishes_opaque() {
        let mut presentation = TerminalPresentation::default();
        assert_eq!(presentation.opacity(), 0.0);
        presentation.fade_elapsed = FADE_SECONDS * 0.5;
        assert_eq!(presentation.opacity(), 0.5);
        presentation.fade_elapsed = FADE_SECONDS;
        assert_eq!(presentation.opacity(), 1.0);
    }

    #[test]
    fn entering_spectate_closes_player_controls_and_unpauses_the_map() {
        let mut app = App::new();
        app.insert_resource(TerminalPresentation {
            outcome: Some(TerminalOutcome::Defeat),
            spectating: true,
            fade_elapsed: FADE_SECONDS,
        })
        .insert_resource(GamePaused(true))
        .insert_resource(GovernancePanelOpen(true))
        .insert_resource(ProvincePanelOpen(Some(MapDetail::Province(0))))
        .init_resource::<campaign_panel::CampaignUi>()
        .init_resource::<toasts::ToastQueue>()
        .add_systems(Update, prepare_spectator);
        app.world_mut().resource_mut::<campaign_panel::CampaignUi>().open =
            Some(campaign_panel::CampaignTab::Military);

        app.update();

        assert!(!app.world().resource::<GamePaused>().0);
        assert!(!app.world().resource::<GovernancePanelOpen>().0);
        assert_eq!(app.world().resource::<ProvincePanelOpen>().0, None);
        assert_eq!(app.world().resource::<campaign_panel::CampaignUi>().open, None);
    }
}

/// Keep inspection available after Spectate, while removing the old player controls.
pub(super) fn prepare_spectator(
    terminal: Res<TerminalPresentation>,
    mut campaign_ui: ResMut<campaign_panel::CampaignUi>,
    mut governance: ResMut<GovernancePanelOpen>,
    mut province: ResMut<ProvincePanelOpen>,
    mut paused: ResMut<GamePaused>,
    mut toasts: ResMut<toasts::ToastQueue>,
) {
    if !terminal.spectating {
        return;
    }
    *campaign_ui = campaign_panel::CampaignUi::default();
    governance.0 = false;
    province.0 = None;
    paused.0 = false;
    toasts.clear();
}
