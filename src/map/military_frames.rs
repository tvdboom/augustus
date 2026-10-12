//! Geometry shared by the animation compiler and map renderer.

pub const COLUMNS: u32 = 8;
pub const ROWS: u32 = 6;
pub const SIZE: u32 = 192;
/// Ground anchor within each tile, including the transparent sampling fringe.
pub const BASELINE: u32 = 173;
pub const COUNT: usize = (COLUMNS * ROWS) as usize;
pub const WIDTH: u32 = COLUMNS * SIZE;
pub const HEIGHT: u32 = ROWS * SIZE;

/// Leave enough canvas for the vehicle while keeping its crew at infantry scale.
pub fn vehicle_scale(name: &str) -> Option<f32> {
    match name {
        "ballista" => Some(1.55),
        "catapult" => Some(1.85),
        "war-chariots" => Some(1.90),
        _ => None,
    }
}
