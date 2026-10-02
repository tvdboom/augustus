use super::*;
use crate::game::economy::{EconomicProvince, EconomyWorld, Terrain};
use crate::game::politics::PoliticalPlayer;

#[test]
fn clicking_an_event_illustration_selects_its_action() {
    let mut campaign = Campaign {
        active: true,
        ..Default::default()
    };
    let mut province =
        EconomicProvince::new("Home", 20.0, Terrain::Plains, false, [1.0; 3], [10.0; 4], 1);
    province.owner = Some(0);
    campaign.economy = EconomyWorld::new(1, vec![province], vec![vec![]]);
    campaign.actors = vec![PoliticalPlayer::default()];
    let ctx = egui::Context::default();
    let mut time = 0.0;
    let mut render = |events| {
        time += 0.1;
        let mut selected = None;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 900.0),
                )),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ui| selected = show(ui, &campaign, 0, 1.0),
        );
        output.textures_delta.clear();
        (output, selected)
    };
    render(vec![]);
    let (output, _) = render(vec![]);
    let title = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == "Organize Theater" => Some(text),
            _ => None,
        })
        .expect("the theater card is visible");
    let image_center = egui::pos2(title.pos.x - 55.0, title.pos.y + 30.0);
    render(vec![
        egui::Event::PointerMoved(image_center),
        egui::Event::PointerButton {
            pos: image_center,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
    ]);
    let (_, selected) = render(vec![egui::Event::PointerButton {
        pos: image_center,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    }]);
    assert_eq!(selected, Some(CivicEvent::Theater));
}
