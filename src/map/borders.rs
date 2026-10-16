//! Repairs for a small offset in the source atlas's Asia boundary.
use super::*;

/// Real mainland borders whose source vertices differ by roughly 400 meters.
/// These also repair the cached graph in campaigns saved before the atlas fix.
pub(crate) const LAND_BORDER_REPAIRS: [[&str; 2]; 3] =
    [["Asia", "Bithynia"], ["Asia", "Galatia"], ["Asia", "Lycia"]];

/// Align only the known offset borders. A general proximity rule could turn
/// narrow straits into land connections, bypassing the explicit sea crossings.
pub(super) fn align_source_borders(provinces: &mut [Province]) {
    const MAX_OFFSET: f32 = 0.006;
    for [from, to] in LAND_BORDER_REPAIRS {
        let Some(from) = provinces.iter().position(|p| p.name == from) else {
            continue;
        };
        let Some(to) = provinces.iter().position(|p| p.name == to) else {
            continue;
        };
        let targets: Vec<_> =
            provinces[to].parts.iter().flat_map(|part| part.v.iter().copied()).collect();
        for vertex in provinces[from].parts.iter_mut().flat_map(|part| &mut part.v) {
            let distance =
                |other: &[f32; 2]| (vertex[0] - other[0]).powi(2) + (vertex[1] - other[1]).powi(2);
            if let Some(&nearest) =
                targets.iter().min_by(|a, b| distance(a).total_cmp(&distance(b)))
            {
                if distance(&nearest) <= MAX_OFFSET * MAX_OFFSET {
                    *vertex = nearest;
                }
            }
        }
    }
}
