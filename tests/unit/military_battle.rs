use super::*;

#[path = "egui_capture.rs"]
mod capture;

#[test]
fn live_battle_panel_shows_both_deployments_dice_history_and_result() {
    use crate::game::economy::{EconomicProvince, Terrain};
    let mut world = MilitaryWorld::new(1);
    let own = ForceOwner::Player(0);
    let enemy = ForceOwner::Player(1);
    for owner in [own, enemy] {
        for kind in [UnitType::HeavyInfantry, UnitType::Archers] {
            world.seed_unit(0, owner, kind).unwrap();
        }
    }
    let id = world
        .start_battle(0, &[own], &[enemy], Some(1), None, MilitaryTerrain::Mountains, 1, 71)
        .unwrap();
    let economy = EconomyWorld::new(
        2,
        vec![EconomicProvince::new("Alpes", 50., Terrain::Mountains, false, [10.; 3], [10.; 4], 2)],
        vec![vec![]],
    );
    let ctx = egui::Context::default();
    open_battle_panel(&ctx, id, 0);
    let mut capture = capture::Capture::default();
    let mut time = 0.;
    let mut render = |world: &MilitaryWorld| {
        time += 0.3;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1600., 900.),
                )),
                time: Some(time),
                ..Default::default()
            },
            |_| {
                draw_battle_panel(&ctx, world, &economy, 0);
            },
        );
        capture.frame(&ctx, &output, "battle");
        output.textures_delta.clear();
        output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(t) => Some(t.galley.job.text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    render(&world);
    let text = render(&world);
    assert!(text.iter().any(|s| s == "Battle of Alpes"));
    assert!(text.iter().any(|s| s == "ATTACKERS"));
    assert!(text.iter().any(|s| s == "DEFENDERS"));
    assert!(text.iter().any(|s| s.contains("Awaiting roll")));
    world.battles[0].advance_month(&world.config);
    let text = render(&world);
    assert!(text.iter().any(|s| s.contains("% dice")));
    assert!(text.iter().any(|s| s.contains("4 rounds")));
    assert!(text.iter().any(|s| s == "ROUND HISTORY"));
    world.battles[0].result = Some(BattleResult::DefenderVictory);
    world.history.push(world.battles.remove(0).into_outcome(&world.config).unwrap());
    let text = render(&world);
    assert!(text.iter().any(|s| s.contains("Defenders hold")));
    assert!(selected_battle(&ctx).is_some(), "result remains readable after battle ends");
    assert!(dismiss(&ctx));
}
