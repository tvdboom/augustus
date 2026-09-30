//! Starting population is derived from province area and the presence of a city.
//! A small fresh variation is rolled for each new game.

use rand::random_range;

/// Number of displayed residents represented by one old aggregate population unit.
pub(crate) const POPULATION_SCALE: f64 = 10.0;

pub(super) fn starting_total_for(area: f64, urban: bool) -> f64 {
    let base = 220.0 + (35.0 * area.sqrt()).round();
    base + (if urban {
        220.0
    } else {
        0.0
    }) + random_range(-20.0..=20.0)
}
