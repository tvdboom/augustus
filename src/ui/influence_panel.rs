//! Aggregate influence income and outflow for the local map HUD.

use super::{campaign::Campaign, flow_panel, ProvinceOwnership};
use crate::game::economy::happiness_output_multiplier;
use crate::game::politics::diplomacy::{distance_multiplier, PoliticalState};
use crate::game::politics::senate::SenatorAction;
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
                .map(|p| {
                    p.population[0]
                        * campaign.economy.config.influence_per_noble
                        * happiness_output_multiplier(0, p.happiness[0], &campaign.economy.config)
                })
                .sum()
        },
    );
    let (wonders, buildings, office, military, vassals, diplomatic_support) = campaign.map_or(
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
        |campaign| {
            let owned = campaign
                .economy
                .provinces
                .iter()
                .filter(|province| province.owner == Some(player));
            let wonders = owned
                .clone()
                .filter_map(|province| province.completed_wonder)
                .filter_map(|id| campaign.economy.config.wonders.iter().find(|w| w.wonder_id == id))
                .map(|wonder| wonder.monthly_influence)
                .sum();
            let buildings = owned
                .map(|province| province.building_effects(&campaign.economy.config).influence)
                .sum();
            let office = campaign.actors.get(player).map_or(0.0, |actor| {
                campaign.senate_config.rank_income(actor.rank)
            });
            let military = campaign.actors.get(player).map_or(0.0, |_| {
                let rank = campaign.military.rank(crate::game::military::ForceOwner::Player(player));
                campaign.military.config.rank_influence[rank as usize]
            });
            let vassals = campaign
                .politics
                .iter()
                .filter_map(|province| province.vassal_income(&campaign.diplomacy_config))
                .filter(|(overlord, _)| *overlord == player)
                .map(|(_, amount)| amount)
                .sum();
            let diplomatic_support = campaign
                .politics
                .iter()
                .enumerate()
                .filter(|(_, province)| {
                    !matches!(&province.state, PoliticalState::Owned { .. } | PoliticalState::Rome)
                })
                .filter_map(|(id, province)| {
                    let support = province.support.get(player)?;
                    let distance = distance_multiplier(campaign.distance(player, id)).ok()?;
                    let relation = u8::from(support.relation_influence);
                    let control = u8::from(
                        support.control_influence
                            && matches!(&province.state, PoliticalState::Vassal { overlord, .. } if *overlord == player),
                    );
                    Some(f64::from(relation + control) * campaign.diplomacy_config.monthly_influence * distance)
                })
                .sum();
            (wonders, buildings, office, military, vassals, diplomatic_support)
        },
    );
    let income = [
        ("Nobles", nobles),
        ("Wonders", wonders),
        ("City buildings", buildings),
        ("Political office", office),
        ("Military rank", military),
        ("Vassals", vassals),
    ];
    // Influence trading is currently disabled by the default trade configuration.
    let senator_lobbying_upfront =
        campaign.map_or(0.0, |campaign| campaign.senate.spent_on(player, SenatorAction::Lobby));
    let senator_lobbying = campaign
        .map_or(0.0, |campaign| campaign.senate.outreach_upkeep(player, &campaign.senate_config));
    let outflow = [
        ("Diplomatic support", diplomatic_support),
        ("Senator lobbying", senator_lobbying),
        ("Senator lobbying (upfront)", senator_lobbying_upfront),
        ("Trades", 0.0),
    ];
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
