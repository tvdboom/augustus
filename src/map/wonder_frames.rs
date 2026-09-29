//! Shared construction-atlas geometry for the compiler and map renderer.
pub(crate) const COLUMNS: u32 = 4;
pub(crate) const ROWS: u32 = 3;
pub(crate) const COUNT: u32 = COLUMNS * ROWS;
pub(crate) const SIZE: u32 = 384;
// The foreground activity has fixed bounds, allowing pixel checks everywhere else.
// These constants are shared by the build script and asset regression tests.
#[allow(dead_code)]
pub(crate) const ACTIVITY_TOP: u32 = 276;
#[allow(dead_code)]
pub(crate) const WORKER_CENTERS: [u32; 3] = [76, 190, 306];
#[allow(dead_code)]
pub(crate) const WORKER_FEET: [u32; 3] = [332, 348, 324];
#[allow(dead_code)]
pub(crate) const WORKER_WIDTH: u32 = 52;
#[allow(dead_code)]
pub(crate) const WORKER_HEIGHT: u32 = 48;
