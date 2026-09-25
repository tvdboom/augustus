//! Starting population is derived from province area and the presence of a city.
//! A small fresh variation is rolled for each new game.

use rand::random_range;

pub(super) fn starting_total_for(area: f64, urban: bool) -> f64 {
    let base = 220.0 + (35.0 * area.sqrt()).round();
    (base
        + (if urban {
            220.0
        } else {
            0.0
        })
        + random_range(-20.0..=20.0))
        / 10.0
}
