//! Map HUD, illustrated panels, tooltips, notifications and shared audio controls.

// Child widgets share the application resources without exposing them publicly.
use super::*;

#[path = "audio_controls.rs"]
pub(super) mod audio_controls;
#[path = "campaign_confirmation.rs"]
pub(super) mod campaign_confirmation;
#[path = "campaign_economy.rs"]
pub(super) mod campaign_economy;
#[path = "campaign_military.rs"]
pub(super) mod campaign_military;
#[path = "campaign_notices.rs"]
pub(super) mod campaign_notices;
#[path = "campaign_panel.rs"]
pub(super) mod campaign_panel;
#[path = "campaign_policies.rs"]
pub(super) mod campaign_policies;
#[path = "campaign_politics.rs"]
pub(super) mod campaign_politics;
#[path = "campaign_trade.rs"]
pub(super) mod campaign_trade;
#[path = "campaign_widgets.rs"]
pub(super) mod campaign_widgets;
#[path = "city_panel.rs"]
pub(super) mod city_panel;
#[path = "coin_panel.rs"]
pub(super) mod coin_panel;
#[path = "flow_panel.rs"]
pub(super) mod flow_panel;
#[path = "governance_panel.rs"]
pub(super) mod governance_panel;
#[path = "hud.rs"]
pub(super) mod hud;
#[path = "influence_panel.rs"]
pub(super) mod influence_panel;
#[path = "map_edge.rs"]
pub(super) mod map_edge;
#[path = "map_menu.rs"]
pub(super) mod map_menu;
#[path = "policy_widgets.rs"]
pub(super) mod policy_widgets;
#[path = "population_panel.rs"]
pub(super) mod population_panel;
#[path = "province_intelligence.rs"]
pub(super) mod province_intelligence;
#[path = "province_panel.rs"]
pub(super) mod province_panel;
#[path = "rank_hud.rs"]
pub(super) mod rank_hud;
#[path = "resource_hud.rs"]
pub(super) mod resource_hud;
#[path = "resource_panel.rs"]
pub(super) mod resource_panel;
#[path = "toasts.rs"]
pub(super) mod toasts;
