//! Current public province facts, sharing the domestic inspector layout.

use crate::app::campaign::Campaign;
use crate::game::economy::ConstructionProject;
use bevy_egui::egui;

pub(in crate::app) fn overview_height(campaign: &Campaign, province: usize, _player: usize) -> f32 {
    super::campaign_economy::overview_height(&campaign.economy, province)
}

pub(in crate::app) fn overview(
    ui: &mut egui::Ui,
    campaign: &Campaign,
    province: usize,
    player: usize,
    scale: f32,
) {
    super::campaign_economy::overview_for_player(
        ui,
        &campaign.economy,
        &campaign.military,
        province,
        Some(player),
        scale,
    );
}

/// Foreign buildings and projects are current and visible, with no management controls.
pub(in crate::app) fn buildings(
    ui: &mut egui::Ui,
    campaign: &Campaign,
    province: usize,
    _player: usize,
    scale: f32,
) {
    super::campaign_economy::readonly_buildings(ui, &campaign.economy, province, scale);
    let economic = &campaign.economy.provinces[province];
    if let Some(project) = &economic.construction {
        let name = match project {
            ConstructionProject::Building(building) => {
                format!("{} level {}", building.building.name(), building.target_level)
            },
            ConstructionProject::Wonder(wonder) => {
                crate::map::wonder_name(wonder.wonder_id).unwrap_or("Wonder").to_owned()
            },
        };
        let (progress, required) = project.progress();
        ui.separator();
        ui.strong("Construction");
        ui.label(format!(
            "{name} · {:.0}% · {} months left",
            (progress / required.max(0.001) * 100.0).floor(),
            project.months_remaining(&campaign.economy.config, economic.policies.construction)
        ));
    }
}
