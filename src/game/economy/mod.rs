//! Deterministic monthly economy, demographic simulation, construction, and trade.
//!
//! Resource indexes are Food, Metal, Stone; class indexes are Nobles, Citizens,
//! Plebeians, Slaves. Province/player IDs are stable indexes supplied by the map.
//! Politics owns sovereignty and diplomacy; this module consumes their snapshots
//! and returns trade effects for the political resolver.

mod buildings;
mod config;
mod market;
mod model;
mod population;
mod simulation;
mod trade;

pub use buildings::*;
pub use config::*;
pub use market::*;
pub use model::*;
pub use population::{
    birth_modifier, happiness_output_multiplier, normalized_capacity_area, UNHAPPINESS_THRESHOLDS,
};
pub use trade::*;

#[cfg(test)]
#[path = "../../../tests/unit/economy.rs"]
mod tests;
