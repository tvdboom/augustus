//! Exercise the real coin and Influence HUD cards after personal Senate actions.
use super::*;
use crate::game::politics::PoliticalPlayer;

fn hover(ctx: &egui::Context, campaign: &Campaign, influence: bool) -> egui::FullOutput {
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1000.0));
    let icon = ctx.load_texture(
        "flow-test",
        egui::ColorImage::filled([1, 1], egui::Color32::WHITE),
        egui::TextureOptions::LINEAR,
    );
    let pointer = egui::pos2(
        super::super::hud_resource_positions()[if influence {
            4
        } else {
            3
        }] + 10.0,
        10.0,
    );
    let mut open = false;
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(screen),
            events: vec![egui::Event::PointerMoved(pointer)],
            ..Default::default()
        },
        |root| {
            if influence {
                super::super::influence_panel::show(
                    root.ctx(),
                    screen,
                    1.0,
                    1600.0,
                    0,
                    &ProvinceOwnership::default(),
                    Some(campaign),
                    &icon,
                    &mut open,
                );
            } else {
                show(
                    root.ctx(),
                    screen,
                    1.0,
                    1600.0,
                    0,
                    &ProvinceOwnership::default(),
                    Some(campaign),
                    &icon,
                    &mut open,
                );
            }
        },
    );
    output.textures_delta.clear();
    output
}

fn label_y(output: &egui::FullOutput, label: &str) -> f32 {
    output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.text == label => Some(t.pos.y),
            _ => None,
        })
        .unwrap_or_else(|| panic!("Missing resource flow row: {label}"))
}

fn has_amount_at(output: &egui::FullOutput, amount: &str, y: f32) -> bool {
    output.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.job.text==amount && (t.pos.y-y).abs()<1.0))
}

#[test]
fn influence_flow_shows_political_and_military_rank_income() {
    let mut c = Campaign {
        actors: vec![PoliticalPlayer {
            rank: crate::game::politics::PoliticalRank::Praetor,
            ..Default::default()
        }],
        ..Default::default()
    };
    c.military.ranks.insert(
        crate::game::military::ForceOwner::Player(0),
        crate::game::military::MilitaryRank::Legate,
    );
    let ctx = egui::Context::default();
    hover(&ctx, &c, true);
    let output = hover(&ctx, &c, true);
    for row in ["Political office", "Military rank"] {
        let y = label_y(&output, row);
        assert!(y > label_y(&output, "INCOME"));
        assert!(has_amount_at(&output, "10", y));
    }
}

#[test]
fn senate_flows_show_bribery_in_coin_and_lobbying_only_in_influence() {
    let mut c = Campaign {
        actors: vec![PoliticalPlayer {
            coin: 1000.0,
            influence: 1000.0,
            ..Default::default()
        }],
        ..Default::default()
    };
    c.senate_config.action_risks = [0.0; 9];
    c.senate.act_on_senator(0, 0, SenatorAction::Bribe, &mut c.actors, &c.senate_config).unwrap();
    c.senate.act_on_senator(0, 1, SenatorAction::Lobby, &mut c.actors, &c.senate_config).unwrap();
    let ctx = egui::Context::default();
    hover(&ctx, &c, false);
    let coin = hover(&ctx, &c, false);
    let y = label_y(&coin, "Senator bribery");
    assert!(y > label_y(&coin, "OUTFLOW"));
    assert!(has_amount_at(&coin, "-8", y));
    assert!(has_amount_at(&coin, "-80", label_y(&coin, "Senator bribery (upfront)")));
    assert!(!coin.shapes.iter().any(
        |s| matches!(&s.shape,egui::Shape::Text(t) if t.galley.job.text.contains("lobbying"))
    ));
    hover(&ctx, &c, true);
    let influence = hover(&ctx, &c, true);
    assert!(has_amount_at(&influence, "-4", label_y(&influence, "Senator lobbying")));
    assert!(has_amount_at(&influence, "-20", label_y(&influence, "Senator lobbying (upfront)")));
    assert!(!influence
        .shapes
        .iter()
        .any(|s| matches!(&s.shape,egui::Shape::Text(t) if t.galley.job.text.contains("bribery"))));
}
