//! Aggregate influence income and outflow for the local map HUD.

use super::{campaign::Campaign, flow_panel, ProvinceOwnership};
use bevy_egui::egui;

pub(in crate::app) fn show(
    context: &egui::Context,
    screen: egui::Rect,
    scale: f32,
    date_left: f32,
    player: usize,
    ownership: &ProvinceOwnership,
    campaign: Option<&Campaign>,
    icon: &egui::TextureHandle,
    open: &mut bool,
) {
    let nobles = campaign.map_or_else(
        || ownership.influence_delta_for(player),
        |campaign| {
            campaign
                .economy
                .provinces
                .iter()
                .filter(|p| p.owner == Some(player))
                .map(|p| p.population[0] * campaign.economy.config.influence_per_noble)
                .sum()
        },
    );
    let income = [("Nobles", nobles)];
    // Spy missions and trades are not available on the local map yet.
    let outflow = [("Spy missions", 0.0), ("Trades", 0.0)];
    flow_panel::show(
        context,
        screen,
        scale,
        date_left,
        icon,
        &flow_panel::FlowPanel {
            resource_index: 4,
            id: "augustus_influence_flow",
            title: "Influence",
            description: "Political power used for imperial actions.",
            income: &income,
            outflow: &outflow,
        },
        open,
    );
}
