//! Illustrated nationwide civic events inside the Overview parchment.

use super::campaign_widgets::{self, Icon};
use crate::app::campaign::Campaign;
use crate::app::campaign_events::{CivicEvent, EventError};
use bevy_egui::egui;

const INK: egui::Color32 = egui::Color32::from_rgb(57, 43, 37);
const RULE: egui::Color32 = egui::Color32::from_rgb(191, 171, 143);

fn image_bytes(event: CivicEvent) -> &'static [u8] {
    match event {
        CivicEvent::Theater => include_bytes!("../../assets/images/events/theater.png"),
        CivicEvent::Feast => include_bytes!("../../assets/images/events/feast.png"),
        CivicEvent::Games => include_bytes!("../../assets/images/events/games.png"),
        CivicEvent::GrainDole => include_bytes!("../../assets/images/events/grain-dole.png"),
        CivicEvent::Patronage => include_bytes!("../../assets/images/events/patronage.png"),
    }
}

fn artwork(ctx: &egui::Context, event: CivicEvent, side: u32) -> egui::TextureId {
    let key = egui::Id::new(("civic-event-image", event));
    if let Some((cached_side, texture)) =
        ctx.data(|data| data.get_temp::<(u32, egui::TextureHandle)>(key))
    {
        if cached_side == side {
            return texture.id();
        }
    }
    let image = image::load_from_memory(image_bytes(event))
        .expect("valid civic event artwork")
        .resize_exact(side, side, image::imageops::FilterType::Lanczos3)
        .to_rgba8();
    let texture = ctx.load_texture(
        format!("civic-event-{event:?}"),
        egui::ColorImage::from_rgba_unmultiplied(
            [image.width() as usize, image.height() as usize],
            image.as_raw(),
        ),
        egui::TextureOptions::LINEAR,
    );
    let id = texture.id();
    ctx.data_mut(|data| data.insert_temp(key, (side, texture)));
    id
}

pub(super) fn show(
    ui: &mut egui::Ui,
    campaign: &Campaign,
    player: usize,
    scale: f32,
) -> Option<CivicEvent> {
    let mut selected = None;
    let image_side = (102.0 * scale * ui.ctx().pixels_per_point()).round().max(1.0) as u32;
    for event in CivicEvent::ALL {
        let availability = campaign.event_availability(player, event);
        let enabled = availability.is_ok();
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), 112.0 * scale),
            egui::Sense::click(),
        );
        campaign_widgets::paint_purchase_background(
            ui,
            rect,
            enabled,
            response.hovered(),
            response.is_pointer_button_down_on(),
            4.0 * scale,
            scale,
        );
        let painter = ui.painter();
        painter.rect_stroke(
            rect,
            4.0 * scale,
            egui::Stroke::new(
                scale,
                if enabled && response.hovered() {
                    INK
                } else {
                    RULE
                },
            ),
            egui::StrokeKind::Inside,
        );
        let image = egui::Rect::from_min_size(
            rect.min + egui::vec2(5.0, 5.0) * scale,
            egui::vec2(102.0, 102.0) * scale,
        );
        painter.image(
            artwork(ui.ctx(), event, image_side),
            image,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            if enabled {
                egui::Color32::WHITE
            } else {
                egui::Color32::from_gray(160)
            },
        );
        let x = image.right() + 10.0 * scale;
        let label_ink = if enabled {
            INK
        } else {
            campaign_widgets::UNAVAILABLE_PURCHASE_INK
        };
        let text = |y: f32, label: &str, size: f32, color: egui::Color32| {
            painter.text(
                egui::pos2(x, rect.top() + y * scale),
                egui::Align2::LEFT_TOP,
                label,
                egui::FontId::proportional(size * scale),
                color,
            );
        };
        text(6.0, event.name(), 16.0, label_ink);
        if let Err(EventError::Cooldown {
            ready_month,
        }) = availability
        {
            let months_left = ready_month.saturating_sub(campaign.economy.month);
            let unit = if months_left == 1 {
                "month"
            } else {
                "months"
            };
            painter.text(
                egui::pos2(rect.right() - 8.0 * scale, rect.top() + 8.0 * scale),
                egui::Align2::RIGHT_TOP,
                format!("{months_left} {unit} left"),
                egui::FontId::proportional(11.0 * scale),
                egui::Color32::from_rgb(168, 45, 40),
            );
        }
        text(29.0, event.description(), 11.0, label_ink);
        let quote = campaign.event_quote(player, event);
        let cost = quote.cost;
        let mut cost_x = x;
        for (icon, amount) in
            [(Icon::Coin, cost.coin), (Icon::Food, cost.food), (Icon::Influence, cost.influence)]
        {
            if amount <= 0.0 {
                continue;
            }
            campaign_widgets::paint_icon(
                ui,
                icon,
                egui::Rect::from_min_size(
                    egui::pos2(cost_x, rect.top() + 49.0 * scale),
                    egui::vec2(19.0, 19.0) * scale,
                ),
            );
            painter.text(
                egui::pos2(cost_x + 23.0 * scale, rect.top() + 58.0 * scale),
                egui::Align2::LEFT_CENTER,
                format!("{amount:.0}"),
                egui::FontId::proportional(13.0 * scale),
                label_ink,
            );
            cost_x += 58.0 * scale;
        }
        campaign_widgets::paint_icon(
            ui,
            if event == CivicEvent::Patronage {
                Icon::Coin
            } else {
                Icon::Happiness
            },
            egui::Rect::from_min_size(
                egui::pos2(x, rect.top() + 79.0 * scale),
                egui::vec2(19.0, 19.0) * scale,
            ),
        );
        painter.text(
            egui::pos2(x + 23.0 * scale, rect.top() + 88.0 * scale),
            egui::Align2::LEFT_CENTER,
            event.reward_label(quote.coin_reward),
            egui::FontId::proportional(11.0 * scale),
            if enabled {
                egui::Color32::from_rgb(43, 103, 67)
            } else {
                label_ink
            },
        );
        // The entire illustrated card, including its image, is the button.
        let response = response.on_hover_cursor(if enabled {
            egui::CursorIcon::PointingHand
        } else {
            egui::CursorIcon::Default
        });
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, event.name())
        });
        if enabled && response.clicked() {
            selected = Some(event);
        }
        ui.add_space(6.0 * scale);
    }
    selected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::economy::{EconomicProvince, EconomyWorld, Terrain};
    use crate::game::politics::PoliticalPlayer;

    #[test]
    fn clicking_an_event_illustration_selects_its_action() {
        let mut campaign = Campaign::default();
        campaign.active = true;
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
}
