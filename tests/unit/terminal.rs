use super::*;

#[test]
fn result_follows_the_local_players_victory_or_last_province_loss() {
    let mut campaign = campaign::Campaign {
        defeated: vec![false; 2],
        ..Default::default()
    };
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
    let entity = app
        .world_mut()
        .spawn((bevy_egui::EguiContext::default(), bevy_egui::PrimaryEguiContext))
        .id();
    let ctx = app.world_mut().get_mut::<bevy_egui::EguiContext>(entity).unwrap().get_mut().clone();
    campaign_military::open_army_panel(&ctx, 0, 0, None);

    app.update();

    assert!(!app.world().resource::<GamePaused>().0);
    assert!(!app.world().resource::<GovernancePanelOpen>().0);
    assert_eq!(app.world().resource::<ProvincePanelOpen>().0, None);
    assert_eq!(app.world().resource::<campaign_panel::CampaignUi>().open, None);
    assert!(!campaign_military::dismiss_army_panel(&ctx));
}
