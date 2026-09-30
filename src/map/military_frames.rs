//! Geometry shared by the animation compiler and map renderer.

pub const COLUMNS: u32 = 8;
pub const ROWS: u32 = 6;
pub const SIZE: u32 = 192;
/// Ground anchor within each tile, including the transparent sampling fringe.
pub const BASELINE: u32 = 173;
pub const COUNT: usize = (COLUMNS * ROWS) as usize;
pub const WIDTH: u32 = COLUMNS * SIZE;
pub const HEIGHT: u32 = ROWS * SIZE;
