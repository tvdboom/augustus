use super::*;
use crate::game::economy::{EconomicProvince, Terrain};

fn fixture() -> (MilitaryWorld, EconomyWorld, u64) {
    let mut world = MilitaryWorld::new(1);
    world.config.base_manpower_damage = 0.;
    world.config.base_morale_damage = 0.;
    for owner in [ForceOwner::Player(0), ForceOwner::Local(0)] {
        world.seed_unit(0, owner, UnitType::HeavyInfantry).unwrap();
    }
    let id = world
        .start_battle(
            0,
            &[ForceOwner::Player(0)],
            &[ForceOwner::Local(0)],
            None,
            None,
            MilitaryTerrain::Plains,
            0,
            7,
        )
        .unwrap();
    let mut battle = world.battles.remove(0);
    for _ in 0..4 {
        battle.advance_round(&world.config);
    }
    battle.result = Some(BattleResult::AttackerVictory);
    world.history.push(battle.into_outcome(&world.config).unwrap());
    let economy = EconomyWorld::new(
        1,
        vec![EconomicProvince::new("Italia", 50., Terrain::Plains, false, [10.; 3], [10.; 4], 1)],
        vec![vec![]],
    );
    (world, economy, id)
}

fn render(
    ctx: &egui::Context,
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    time: f64,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600., 900.))),
            time: Some(time),
            events,
            ..Default::default()
        },
        |_| {
            assert!(super::super::draw_battle_panel(ctx, world, economy, 0).is_none());
        },
    );
    output.textures_delta.clear();
    output
}

fn click(
    ctx: &egui::Context,
    world: &MilitaryWorld,
    economy: &EconomyWorld,
    position: egui::Pos2,
    time: f64,
) {
    for (offset, pressed) in [(0., true), (0.01, false)] {
        render(
            ctx,
            world,
            economy,
            time + offset,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
}

fn state(ctx: &egui::Context) -> Replay {
    ctx.data(|data| data.get_temp(egui::Id::new(STATE_ID))).unwrap()
}

#[test]
fn replay_controls_start_pause_change_speed_finish_and_reset_without_changing_the_campaign() {
    let (world, economy, id) = fixture();
    let original = serde_json::to_value(&world).unwrap();
    let ctx = egui::Context::default();
    super::super::open_battle_panel(&ctx, id, 0);
    render(&ctx, &world, &economy, 0., vec![]);
    render(&ctx, &world, &economy, 0.1, vec![]);
    assert_eq!(state(&ctx).cursor, 4, "History opens at the finish");
    let (panel, scale) = super::super::panel_rect(ctx.content_rect());
    let origin = egui::pos2(
        panel.left() + panel.width() * 0.75 - CONTROLS_WIDTH * scale / 2.,
        panel.top() + 21. * scale,
    );
    let play = origin + egui::vec2(14. * scale, 0.);
    let speed = origin + egui::vec2(48. * scale, 0.);
    let end = origin + egui::vec2(184. * scale, 0.);
    click(&ctx, &world, &economy, play, 0.2);
    assert!(state(&ctx).playing);
    assert_eq!(state(&ctx).cursor, 0);
    render(&ctx, &world, &economy, 0.97, vec![]);
    assert_eq!(state(&ctx).cursor, 1);
    click(&ctx, &world, &economy, play, 1.0);
    assert!(!state(&ctx).playing);
    render(&ctx, &world, &economy, 8., vec![]);
    assert_eq!(state(&ctx).cursor, 1, "Pause does not accrue elapsed time");
    click(&ctx, &world, &economy, speed, 8.1);
    assert_eq!(state(&ctx).speed, 2);
    click(&ctx, &world, &economy, speed, 8.2);
    assert_eq!(state(&ctx).speed, 3);
    click(&ctx, &world, &economy, play, 8.3);
    render(&ctx, &world, &economy, 8.57, vec![]);
    assert_eq!(state(&ctx).cursor, 2, "3× advances a round in a quarter second");
    click(&ctx, &world, &economy, end, 8.6);
    assert_eq!(state(&ctx).cursor, 4);
    assert!(!state(&ctx).playing);
    click(&ctx, &world, &economy, play, 8.7);
    render(&ctx, &world, &economy, 12., vec![]);
    assert_eq!(state(&ctx).cursor, 4);
    assert!(!state(&ctx).playing, "Playback stops at the finish");
    super::super::dismiss(&ctx);
    assert!(ctx.data(|data| data.get_temp::<Replay>(egui::Id::new(STATE_ID))).is_none());
    super::super::open_battle_panel(&ctx, id, 0);
    render(&ctx, &world, &economy, 13., vec![]);
    assert_eq!(state(&ctx).cursor, 4);
    assert_eq!(state(&ctx).speed, 1);
    assert_eq!(serde_json::to_value(&world).unwrap(), original);
}

#[test]
fn older_battles_still_open_but_do_not_offer_an_invented_replay() {
    let (mut world, economy, id) = fixture();
    world.history[0].replay_start = None;
    for round in &mut world.history[0].round_history {
        round.replay = None;
    }
    assert!(!available(&world.history[0]));
    let ctx = egui::Context::default();
    super::super::open_battle_panel(&ctx, id, 0);
    render(&ctx, &world, &economy, 0., vec![]);
    let (panel, scale) = super::super::panel_rect(ctx.content_rect());
    let play = egui::pos2(
        panel.left() + panel.width() * 0.75 - (CONTROLS_WIDTH / 2. - 14.) * scale,
        panel.top() + 21. * scale,
    );
    click(&ctx, &world, &economy, play, 0.1);
    assert!(!state(&ctx).playing);
    assert_eq!(state(&ctx).cursor, 4);
}
