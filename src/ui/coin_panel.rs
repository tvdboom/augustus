//! Aggregate sestertius income and outflow for the local map HUD.

use super::{flow_panel, ProvinceOwnership};
use bevy_egui::egui;

pub(super) fn show(
    context: &egui::Context,
    screen: egui::Rect,
    scale: f32,
    date_left: f32,
    player: usize,
    ownership: &ProvinceOwnership,
    icon: &egui::TextureHandle,
    open: &mut bool,
) {
    let income = [("Taxes", ownership.coin_taxes_for(player)), ("Tributes", 0.0)];
    let outflow = [
        ("Wages", ownership.military_wages_for(player)),
        ("Army maintenance", 0.0),
        ("Deals", 0.0),
    ];
    flow_panel::show(
        context,
        screen,
        scale,
        date_left,
        icon,
        &flow_panel::FlowPanel {
            resource_index: 3,
            id: "augustus_coin_flow",
            title: "Sestertius",
            description: "Pays for armies, buildings, and political schemes.",
            income: &income,
            outflow: &outflow,
        },
        open,
    );
}
