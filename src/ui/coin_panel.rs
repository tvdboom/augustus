//! Aggregate sestertius income and outflow for the local map HUD.

use super::{campaign::Campaign, flow_panel, ProvinceOwnership};
use crate::game::politics::senate::SenatorAction;

#[cfg(test)]
#[path = "../../tests/unit/senate_flow_ui.rs"]
mod tests;
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
    let taxes = campaign.map_or_else(
        || ownership.coin_taxes_for(player),
        |campaign| {
            campaign
                .economy
                .provinces
                .iter()
                .filter(|p| p.owner == Some(player))
                .map(|p| p.tax_income(&campaign.economy.config))
                .sum()
        },
    );
    let tribute = campaign.map_or(0.0, |campaign| {
        campaign
            .politics
            .iter()
            .enumerate()
            .map(|(id, p)| {
                let market = &campaign.economy.provinces[id].market;
                p.tribute_due(market.monthly_coin_income.min(market.coin_treasury))
                    .filter(|(overlord, _)| *overlord == player)
                    .map_or(0.0, |(_, amount)| amount)
            })
            .sum()
    });
    let noble_wages = campaign
        .map_or_else(|| ownership.noble_wages_for(player), |campaign| campaign.noble_wages(player));
    let army_wages = campaign.map_or_else(
        || ownership.military_wages_for(player),
        |campaign| campaign.army_wages(player),
    );
    let income = [("Taxes", taxes), ("Tributes", tribute)];
    let civic = campaign.map_or(0.0, |campaign| {
        campaign
            .economy
            .provinces
            .iter()
            .filter(|p| p.owner == Some(player))
            .map(|p| p.civic_spending_cost(&campaign.economy.config))
            .sum()
    });
    let recruitment = campaign.map_or(0.0, |campaign| campaign.recruitment_effort_cost(player));
    let spy_upkeep = campaign.map_or(0.0, |campaign| campaign.spy_upkeep(player));
    let senator_bribery =
        campaign.map_or(0.0, |campaign| campaign.senate.spent_on(player, SenatorAction::Bribe));
    let outflow = [
        ("Noble wages", noble_wages),
        ("Army wages", army_wages),
        ("Civic spending", civic),
        ("Recruitment effort", recruitment),
        ("Spy upkeep", spy_upkeep),
        ("Senator bribery", senator_bribery),
        ("Trade deals", 0.0),
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
