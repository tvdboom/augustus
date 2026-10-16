use super::*;
use crate::game::economy::{EconomicProvince, Terrain};

fn outcome(
    id: u64,
    attacker: ForceOwner,
    defender: ForceOwner,
    result: BattleResult,
) -> BattleOutcome {
    let mut world = MilitaryWorld::new(2);
    world.config.base_manpower_damage = 0.;
    world.config.base_morale_damage = 0.;
    for owner in [attacker, defender] {
        world.seed_unit(0, owner, UnitType::HeavyInfantry).unwrap();
    }
    world
        .start_battle(0, &[attacker], &[defender], None, None, MilitaryTerrain::Plains, 0, 7)
        .unwrap();
    let mut battle = world.battles.remove(0);
    battle.advance_round(&world.config);
    battle.id = id;
    battle.attackers.units[0].current_manpower = 8.;
    battle.defenders.units[0].current_manpower = 6.;
    battle.result = Some(result);
    battle.into_outcome(&world.config).unwrap()
}

fn economy() -> EconomyWorld {
    EconomyWorld::new(
        3,
        ["Italia", "Gallia"]
            .into_iter()
            .map(|name| {
                EconomicProvince::new(name, 50., Terrain::Plains, false, [10.; 3], [10.; 4], 3)
            })
            .collect(),
        vec![vec![1], vec![0]],
    )
}

fn render(
    ctx: &egui::Context,
    world: &MilitaryWorld,
    width: f32,
    scale: f32,
    time: f64,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    if time == 0. {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "augustus".into(),
            egui::FontData::from_static(include_bytes!("../../assets/fonts/FiraSans-Bold.ttf"))
                .into(),
        );
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, "augustus".into());
        ctx.set_fonts(fonts);
    }
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 650.))),
            time: Some(time),
            events,
            ..Default::default()
        },
        |ui| {
            *ui.style_mut() = super::super::super::campaign_widgets::map_style(scale);
            overview(ui, world, &economy(), 0, 0., &[], scale, &mut None);
        },
    );
    if time == 0. {
        crate::egui_capture::Capture::default().frame(
            ctx,
            &output,
            &format!("battle-history-{width}-{scale}"),
        );
    }
    output.textures_delta.clear();
    output
}

fn label(output: &egui::FullOutput, value: &str) -> egui::Pos2 {
    output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == value => Some(text.pos),
            _ => None,
        })
        .unwrap_or_else(|| panic!("Missing {value}"))
}

fn click(position: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(position),
        egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        },
    ]
}

#[test]
fn battle_history_lists_only_participating_battles_newest_first_and_opens_the_selected_record() {
    let mut world = MilitaryWorld::new(2);
    let own = ForceOwner::Player(0);
    world.history = vec![
        outcome(1, own, ForceOwner::Local(1), BattleResult::AttackerVictory),
        outcome(2, ForceOwner::Player(1), own, BattleResult::AttackerVictory),
        outcome(3, own, ForceOwner::Local(0), BattleResult::MutualRout),
        outcome(4, ForceOwner::Player(1), ForceOwner::Local(1), BattleResult::DefenderVictory),
    ];
    // The province is independent now, but belonged to different players in older battles.
    world.history[0].territorial_owner = Some(0);
    world.history[1].territorial_owner = Some(1);
    world.seed_unit(0, own, UnitType::Archers).unwrap();
    let original = serde_json::to_value(&world).unwrap();
    for (width, scale) in [(796., 1.), (380., 1.), (380., 0.85)] {
        let ctx = egui::Context::default();
        let own_color = super::super::super::PLAYER_COLORS[0];
        let opponent_color = super::super::super::PLAYER_COLORS[3];
        ctx.data_mut(|data| {
            data.insert_temp(
                egui::Id::new("campaign-player-colors"),
                vec![own_color, opponent_color],
            );
        });
        let output = render(&ctx, &world, width, scale, 0., vec![]);
        let header = label(&output, "Opponent");
        assert!(header.y > label(&output, "Training").y);
        let names: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text)
                    if text.galley.job.text == "Italia" && text.pos.y > header.y =>
                {
                    Some(text.pos)
                },
                _ => None,
            })
            .collect();
        assert_eq!(names.len(), 3, "Only participating battles belong in history");
        let [newest, middle, oldest] = names.as_slice() else {
            unreachable!()
        };
        assert!(header.y < newest.y && newest.y < middle.y && middle.y < oldest.y);
        let markers: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Rect(marker)
                    if marker.rect.top() > header.y
                        && (marker.rect.width() - 6. * scale).abs() < 0.1 =>
                {
                    Some(marker)
                },
                _ => None,
            })
            .collect();
        assert_eq!(markers.len(), 3);
        for ((marker, name), color) in
            markers.iter().zip(&names).zip([NEUTRAL, opponent_color, own_color])
        {
            assert_eq!(
                marker.fill, color,
                "Use the saved territorial owner, not today's owner or the battle winner"
            );
            assert!((marker.rect.height() / (64. * scale) - 0.68).abs() < 0.001);
            assert_eq!(marker.corner_radius, egui::CornerRadius::from(2. * scale));
            assert!((name.x - marker.rect.right() - 8. * scale).abs() < 0.1);
        }
        assert!(label(&output, "Mutual rout").y < label(&output, "Defeat").y);
        assert!(label(&output, "Defeat").y < label(&output, "Victory").y);
        for text in [
            "Victory",
            "Defeat",
            "Mutual rout",
            "Gallia's army",
            "You: 200",
            "Opponent: 400",
            "You: 400",
            "Opponent: 200",
        ] {
            label(&output, text);
        }
        assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Text(text) if text.galley.job.text.contains('#')
                || matches!(text.galley.job.text.as_str(), "BATTLES" | "Attacker" | "Defender")
                || text.galley.job.text.starts_with("Enemy:"))));
        let loss_icon = super::super::super::campaign_widgets::texture(&ctx, Icon::Cancel);
        assert_eq!(output.shapes.iter().filter(|shape| matches!(&shape.shape,
            egui::Shape::Mesh(mesh) if mesh.texture_id == loss_icon && mesh.calc_bounds().top() > header.y
        )).count(), 6, "Both sides' losses use the battle screen's casualty icon");
        for shape in &output.shapes {
            if let egui::Shape::Text(text) = &shape.shape {
                if text.pos.y > header.y
                    && matches!(
                        text.galley.job.text.as_str(),
                        "Italia" | "Gallia's army" | "Player 2"
                    )
                {
                    for section in &text.galley.job.sections {
                        assert_eq!(section.format.font_id.size, 14. * scale);
                        assert_eq!(
                            section.format.color,
                            if text.galley.job.text == "Player 2" {
                                opponent_color
                            } else {
                                INK
                            },
                        );
                    }
                }
                let visible = egui::Rect::from_min_size(text.pos, text.galley.size())
                    .intersect(shape.clip_rect);
                assert!(visible.right() <= width + 0.5);
            }
        }
        render(&ctx, &world, width, scale, 0.1, click(*middle + egui::vec2(10., 10.), true));
        render(&ctx, &world, width, scale, 0.2, click(*middle + egui::vec2(10., 10.), false));
        assert_eq!(take_battle_click(&ctx), Some(2));
        assert_eq!(take_battle_click(&ctx), None);
    }
    assert_eq!(serde_json::to_value(&world).unwrap(), original);
}

#[test]
fn battles_have_an_empty_state_and_defensive_victories_and_allied_losses_are_personal() {
    let ctx = egui::Context::default();
    let mut world = MilitaryWorld::new(2);
    label(&render(&ctx, &world, 796., 1., 0., vec![]), "You have no completed battles.");
    let own = ForceOwner::Player(0);
    let mut battle = outcome(1, ForceOwner::Local(1), own, BattleResult::DefenderVictory);
    let mut ally = battle.defenders.units[0].clone();
    ally.owner = ForceOwner::Player(1);
    ally.id += 100;
    battle.defenders.initial_manpower.insert(ally.owner, 10.);
    ally.current_manpower = 1.;
    battle.defenders.units.push(ally);
    world.history.push(battle);
    let output = render(&ctx, &world, 796., 1., 0.1, vec![]);
    label(&output, "Victory");
    label(&output, "You: 400");
    label(&output, "Opponent: 200");
}
