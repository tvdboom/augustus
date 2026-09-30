use super::*;
use crate::app::campaign_notifications::NoticeKind;
use crate::game::economy::{BuildingType, EconomicProvince, EconomyWorld, Terrain};
use crate::game::military::{ForceOwner, MilitaryWorld, UnitType};

fn fixture() -> campaign::Campaign {
    let mut p =
        EconomicProvince::new("Italia", 50.0, Terrain::Plains, false, [1.0; 3], [1000.0; 4], 2);
    p.owner = Some(0);
    let mut campaign = campaign::Campaign::default();
    campaign.economy = EconomyWorld::new(2, vec![p], vec![vec![]]);
    campaign.economy.players[0].resources = [100_000.0; 3];
    campaign.military = MilitaryWorld::new(1);
    campaign
}

#[test]
fn province_acquisition_waits_for_yes_and_revalidates_the_target() {
    use crate::game::politics::diplomacy::{PoliticalState, ProvincePolitics};
    for action in [ConfirmationAction::Vassalize, ConfirmationAction::Integrate] {
        let ctx = egui::Context::default();
        let mut campaign = fixture();
        campaign.economy.provinces[0].owner = None;
        campaign.politics = vec![ProvincePolitics::independent(2)];
        campaign.politics[0].state = PoliticalState::Independent {
            local: 0.0,
            shares: vec![100.0, 0.0],
        };
        campaign.politics[0].relations[0] = 80.0;
        campaign.actors = vec![crate::game::politics::PoliticalPlayer::default(); 2];
        campaign.graph = vec![crate::game::military::MilitaryProvince {
            terrain: crate::game::military::MilitaryTerrain::Plains,
            area: 50.0,
            road_level: 0,
            neighbors: vec![],
        }];
        let mut view = CampaignUi::default();
        view.confirmation = PendingConfirmation::new(&campaign, 0, 0, action);
        let pending = view.confirmation.clone().unwrap();
        frame(&ctx, &mut campaign, &mut view, vec![]);
        let output = frame(&ctx, &mut campaign, &mut view, vec![]);
        click(&ctx, &mut campaign, &mut view, text_rect(&output, "No").center());
        assert!(matches!(campaign.politics[0].state, PoliticalState::Independent { .. }));
        assert_eq!(campaign.economy.provinces[0].owner, None);
        view.confirmation = Some(pending.clone());
        frame(&ctx, &mut campaign, &mut view, vec![]);
        let output = frame(&ctx, &mut campaign, &mut view, vec![]);
        click(&ctx, &mut campaign, &mut view, text_rect(&output, "Yes").center());
        assert!(!view.confirmation_open());
        match action {
            ConfirmationAction::Vassalize => {
                assert!(matches!(
                    campaign.politics[0].state,
                    PoliticalState::Vassal {
                        overlord: 0,
                        control: 50.0,
                        ..
                    }
                ));
                assert_eq!(campaign.economy.provinces[0].overlord, Some(0));
            },
            ConfirmationAction::Integrate => {
                assert_eq!(campaign.economy.provinces[0].owner, Some(0));
                assert_eq!(campaign.economy.provinces[0].temporary_happiness, [30.0; 4]);
            },
            _ => unreachable!(),
        }
        campaign.politics[0].state = PoliticalState::Owned {
            owner: 1,
        };
        campaign.politics[0].owned_shares = vec![0.0, 100.0];
        assert!(!pending.apply(&ctx, &mut campaign).contains("Province integrated"));
        assert_eq!(
            campaign.politics[0].state,
            PoliticalState::Owned {
                owner: 1
            }
        );
    }
}

fn frame(
    ctx: &egui::Context,
    campaign: &mut campaign::Campaign,
    view: &mut CampaignUi,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 900.0),
            )),
            events,
            ..Default::default()
        },
        |root| {
            egui::Area::new(egui::Id::new("behind-confirmation"))
                .order(egui::Order::Foreground)
                .fixed_pos(egui::pos2(10.0, 10.0))
                .show(root.ctx(), |ui| {
                    if ui.button("Background action").clicked() {
                        campaign.economy.players[0].resources[0] = 0.0;
                    }
                });
            show(root.ctx(), view, campaign);
        },
    );
    output.textures_delta.clear();
    output
}

fn text_rect(output: &egui::FullOutput, label: &str) -> egui::Rect {
    output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == label => {
                Some(text.galley.rect.translate(text.pos.to_vec2()))
            },
            _ => None,
        })
        .unwrap_or_else(|| panic!("Missing text: {label}"))
}

fn click(
    ctx: &egui::Context,
    campaign: &mut campaign::Campaign,
    view: &mut CampaignUi,
    pos: egui::Pos2,
) {
    for pressed in [true, false] {
        frame(
            ctx,
            campaign,
            view,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
}

#[test]
fn confirmation_stays_above_foreground_panels() {
    let ctx = egui::Context::default();
    let mut campaign = fixture();
    let mut view = CampaignUi::default();
    campaign.military.seed_unit(0, ForceOwner::Player(0), UnitType::LightInfantry).unwrap();
    view.confirmation = PendingConfirmation::new(&campaign, 0, 0, ConfirmationAction::DisbandArmy);
    for _ in 0..2 {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 900.0),
                )),
                ..Default::default()
            },
            |ui| {
                let panel = egui::Area::new(egui::Id::new("overlapping-army-panel"))
                    .order(egui::Order::Foreground)
                    .fixed_pos(egui::pos2(500.0, 350.0))
                    .show(ui.ctx(), |ui| {
                        ui.set_min_size(egui::vec2(400.0, 400.0));
                    });
                ui.ctx().move_to_top(panel.response.layer_id);
                show(ui.ctx(), &mut view, &mut campaign);
            },
        );
        output.textures_delta.clear();
        assert_eq!(
            ctx.memory(|memory| memory.areas().top_layer_id(egui::Order::Foreground)),
            Some(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("augustus_action_confirmation"),
            )),
        );
    }
}

#[test]
fn no_outside_and_escape_preserve_construction_and_block_background_actions() {
    for dismissal in ["No", "Outside", "Outside right-click", "Escape"] {
        let ctx = egui::Context::default();
        let mut campaign = fixture();
        campaign.economy.start_building(0, 0, BuildingType::Granary).unwrap();
        let resources = campaign.economy.players[0].resources;
        let mut view = CampaignUi::default();
        view.confirmation =
            PendingConfirmation::construction(&campaign, 0, 0, WorkQueueAction::CancelActive);
        frame(&ctx, &mut campaign, &mut view, vec![]);
        frame(&ctx, &mut campaign, &mut view, vec![]);
        let output = frame(&ctx, &mut campaign, &mut view, vec![]);
        let no = text_rect(&output, "No");
        let yes = text_rect(&output, "Yes");
        let title = text_rect(&output, "Cancel construction");
        let modal_rect = ctx
            .memory(|memory| memory.area_rect(egui::Id::new("augustus_action_confirmation")))
            .expect("Modal has a centered area");
        assert!(
            modal_rect.center().distance(egui::pos2(600.0, 450.0)) < 1.0,
            "Centered above the map: {modal_rect:?}"
        );
        assert!((title.center().x - modal_rect.center().x).abs() < 1.0);
        assert!(
            ((yes.center().x + no.center().x) / 2.0 - modal_rect.center().x).abs() < 1.0,
            "Yes and No should be centered together"
        );
        assert!(title.bottom() < yes.top());
        click(&ctx, &mut campaign, &mut view, modal_rect.left_top() + egui::vec2(8.0, 8.0));
        assert!(view.confirmation_open(), "Clicking inside the card must not dismiss it");
        match dismissal {
            "No" => click(&ctx, &mut campaign, &mut view, no.center()),
            "Outside" => click(
                &ctx,
                &mut campaign,
                &mut view,
                text_rect(&output, "Background action").center(),
            ),
            "Outside right-click" => {
                let pos = text_rect(&output, "Background action").center();
                for pressed in [true, false] {
                    frame(
                        &ctx,
                        &mut campaign,
                        &mut view,
                        vec![
                            egui::Event::PointerMoved(pos),
                            egui::Event::PointerButton {
                                pos,
                                button: egui::PointerButton::Secondary,
                                pressed,
                                modifiers: egui::Modifiers::NONE,
                            },
                        ],
                    );
                }
            },
            _ => {
                frame(
                    &ctx,
                    &mut campaign,
                    &mut view,
                    vec![egui::Event::Key {
                        key: egui::Key::Escape,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                );
            },
        }
        assert!(!view.confirmation_open(), "{dismissal} must dismiss");
        assert!(campaign.economy.provinces[0].construction.is_some());
        assert_eq!(campaign.economy.players[0].resources, resources);
    }
}

#[test]
fn yes_is_required_for_active_queued_recruitment_and_army_disbanding() {
    use campaign_military::MilitaryUiAction;
    for operation in 0..4 {
        let ctx = egui::Context::default();
        let mut campaign = fixture();
        let mut view = CampaignUi::default();
        let initial_population = campaign.economy.provinces[0].population;
        let action = match operation {
            0 | 1 => {
                for _ in 0..2 {
                    apply_military_action(
                        &mut campaign,
                        0,
                        0,
                        MilitaryUiAction::Recruit(UnitType::LightInfantry),
                    );
                }
                if operation == 0 {
                    MilitaryUiAction::CancelRecruitment
                } else {
                    MilitaryUiAction::CancelQueuedRecruitment(0)
                }
            },
            _ => {
                let id = campaign
                    .military
                    .seed_unit(0, ForceOwner::Player(0), UnitType::LightInfantry)
                    .unwrap();
                campaign.military.seed_unit(0, ForceOwner::Player(0), UnitType::Archers).unwrap();
                if operation == 2 {
                    MilitaryUiAction::Disband(id)
                } else {
                    MilitaryUiAction::DisbandArmy
                }
            },
        };
        let population = campaign.economy.provinces[0].population;
        let resources = campaign.economy.players[0].resources;
        super::super::campaign_panel::dispatch_military_action(
            &mut view,
            &mut campaign,
            0,
            0,
            action,
        );
        assert!(view.confirmation_open());
        assert_eq!(campaign.economy.provinces[0].population, population);
        assert_eq!(campaign.economy.players[0].resources, resources);
        assert_eq!(
            campaign.military.all_units().count(),
            if operation < 2 {
                0
            } else {
                2
            }
        );
        frame(&ctx, &mut campaign, &mut view, vec![]);
        let output = frame(&ctx, &mut campaign, &mut view, vec![]);
        let pending = view.confirmation.clone();
        if operation == 3 {
            text_rect(&output, "Disband army");
            text_rect(&output, "Are you sure you want to disband your entire army in Italia?");
        }
        click(&ctx, &mut campaign, &mut view, text_rect(&output, "No").center());
        assert!(!view.confirmation_open());
        if operation == 3 {
            assert!(!campaign
                .notifications
                .history_for(0)
                .any(|notice| notice.kind == NoticeKind::ArmyDisbanded));
        }
        assert_eq!(campaign.economy.provinces[0].population, population);
        assert_eq!(campaign.economy.players[0].resources, resources);
        assert_eq!(
            campaign.military.all_units().count(),
            if operation < 2 {
                0
            } else {
                2
            }
        );
        view.confirmation = pending;
        frame(&ctx, &mut campaign, &mut view, vec![]);
        let output = frame(&ctx, &mut campaign, &mut view, vec![]);
        click(&ctx, &mut campaign, &mut view, text_rect(&output, "Yes").center());
        assert!(!view.confirmation_open());
        match operation {
            0 => {
                assert!(
                    campaign.military.provinces[0].recruitment.is_some(),
                    "The next recruit starts"
                );
                assert!(campaign.military.provinces[0].recruitment_queue.is_empty());
                assert_eq!(campaign.economy.players[0].resources, resources);
                assert_eq!(campaign.economy.provinces[0].population, population);
            },
            1 => {
                assert!(campaign.military.provinces[0].recruitment_queue.is_empty());
                assert!(campaign.economy.players[0].resources[1] > resources[1]);
                assert!(campaign.economy.provinces[0].population[2] > population[2]);
                assert_ne!(campaign.economy.provinces[0].population, initial_population);
            },
            _ => {
                assert_eq!(
                    campaign.military.all_units().count(),
                    if operation == 2 {
                        1
                    } else {
                        0
                    }
                );
                assert_eq!(campaign.economy.players[0].resources, resources);
                if operation == 3 {
                    let notice = campaign
                        .notifications
                        .history_for(0)
                        .find(|notice| notice.kind == NoticeKind::ArmyDisbanded)
                        .expect("confirmed army disband creates a notification");
                    assert_eq!(notice.title, "Army disbanded");
                    assert_eq!(notice.province, Some(0));
                }
            },
        }
        let after = campaign.economy.provinces[0].population;
        frame(&ctx, &mut campaign, &mut view, vec![]);
        assert_eq!(campaign.economy.provinces[0].population, after, "Apply only once");
    }
}

#[test]
fn confirmation_keeps_the_clock_running_with_existing_speed_and_pause_state() {
    let mut campaign = fixture();
    campaign.economy.start_building(0, 0, BuildingType::Granary).unwrap();
    let mut view = CampaignUi::default();
    view.confirmation =
        PendingConfirmation::construction(&campaign, 0, 0, WorkQueueAction::CancelActive);
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_secs_f32(SECONDS_PER_MONTH));
    let mut app = App::new();
    app.insert_resource(time)
        .insert_resource(State::new(AppState::Map))
        .insert_resource(campaign)
        .insert_resource(view)
        .insert_resource(GamePaused(false))
        .insert_resource(GameClock {
            speed_step: 1,
            ..Default::default()
        })
        .init_resource::<HudResources>()
        .init_resource::<ActiveGame>()
        .init_resource::<ProvinceOwnership>()
        .add_systems(Update, crate::app::game_controls::advance_game_time);
    app.update();
    assert_eq!(app.world().resource::<GameClock>().month, 2);
    assert_eq!(app.world().resource::<GameClock>().month_progress, 0.0);
    assert!(!app.world().resource::<GamePaused>().0);
    assert!(app.world().resource::<CampaignUi>().confirmation_open());
    app.world_mut().resource_mut::<CampaignUi>().dismiss_confirmation();
    app.update();
    assert_eq!(app.world().resource::<GameClock>().month, 4);
    assert_eq!(app.world().resource::<GameClock>().speed_step, 1);
    assert!(!app.world().resource::<GamePaused>().0);
}

#[test]
fn confirmation_survives_a_month_if_its_order_is_still_active() {
    let ctx = egui::Context::default();
    let mut campaign = fixture();
    campaign.economy.start_building(0, 0, BuildingType::Granary).unwrap();
    let pending =
        PendingConfirmation::construction(&campaign, 0, 0, WorkQueueAction::CancelActive).unwrap();
    campaign.economy.month += 1;
    assert_eq!(pending.apply(&ctx, &mut campaign), "Construction cancelled.");
    assert!(campaign.economy.provinces[0].construction.is_none());
}

#[test]
fn completed_order_closes_confirmation_before_it_can_cancel_its_successor() {
    let ctx = egui::Context::default();
    let mut campaign = fixture();
    campaign.economy.start_building(0, 0, BuildingType::Granary).unwrap();
    campaign.economy.start_building(0, 0, BuildingType::Granary).unwrap();
    let mut view = CampaignUi::default();
    view.confirmation =
        PendingConfirmation::construction(&campaign, 0, 0, WorkQueueAction::CancelActive);
    let successor = campaign.economy.provinces[0].construction_queue.pop_front().unwrap();
    campaign.economy.provinces[0].construction = Some(successor);
    frame(&ctx, &mut campaign, &mut view, vec![]);
    assert!(!view.confirmation_open());
    assert!(campaign.economy.provinces[0].construction.is_some());
}

#[test]
fn finished_building_and_recruitment_close_their_confirmations() {
    use campaign_military::MilitaryUiAction;
    let ctx = egui::Context::default();
    let mut campaign = fixture();
    campaign.economy.start_building(0, 0, BuildingType::Granary).unwrap();
    let mut view = CampaignUi::default();
    view.confirmation =
        PendingConfirmation::construction(&campaign, 0, 0, WorkQueueAction::CancelActive);
    campaign.economy.provinces[0].construction = None;
    frame(&ctx, &mut campaign, &mut view, vec![]);
    assert!(!view.confirmation_open());

    apply_military_action(&mut campaign, 0, 0, MilitaryUiAction::Recruit(UnitType::LightInfantry));
    view.confirmation =
        PendingConfirmation::new(&campaign, 0, 0, ConfirmationAction::CancelRecruitment);
    assert!(view.confirmation_open());
    campaign.military.provinces[0].recruitment = None;
    frame(&ctx, &mut campaign, &mut view, vec![]);
    assert!(!view.confirmation_open());
}

#[test]
fn game_escape_dismisses_only_the_confirmation_and_keeps_the_panel_open() {
    let mut campaign = fixture();
    campaign.economy.start_building(0, 0, BuildingType::Granary).unwrap();
    let mut view = CampaignUi::default();
    view.open = Some(campaign_panel::CampaignTab::Province);
    view.province = Some(0);
    view.confirmation =
        PendingConfirmation::construction(&campaign, 0, 0, WorkQueueAction::CancelActive);
    let mut keyboard = ButtonInput::<KeyCode>::default();
    keyboard.press(KeyCode::Escape);
    let mut app = App::new();
    app.insert_resource(keyboard)
        .insert_resource(State::new(AppState::Map))
        .insert_resource(view)
        .insert_resource(ProvincePanelOpen(Some(MapDetail::Province(0))))
        .init_resource::<ActiveGame>()
        .init_resource::<NextState<AppState>>()
        .init_resource::<GovernancePanelOpen>()
        .init_resource::<MapPanelCloseClick>()
        .add_systems(Update, crate::app::game_controls::handle_escape);
    app.update();
    let view = app.world().resource::<CampaignUi>();
    assert!(!view.confirmation_open());
    assert_eq!(view.open, Some(campaign_panel::CampaignTab::Province));
    assert_eq!(app.world().resource::<ProvincePanelOpen>().0, Some(MapDetail::Province(0)));
    assert!(matches!(app.world().resource::<NextState<AppState>>(), NextState::Unchanged));
}
