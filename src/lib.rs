//! Augustus menu-first game shell.

#![warn(missing_docs)]

pub mod app;
pub(crate) mod basis_texture;
pub(crate) mod map;
pub mod multiplayer;
pub mod platform;

/// Human-readable application title used by native and browser builds.
pub const TITLE: &str = "Augustus";
