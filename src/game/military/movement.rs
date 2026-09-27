//! Size-, terrain-, and road-aware deterministic military routes.

use super::*;

/// Immutable geographic snapshot used for routing during a monthly phase.
#[derive(Clone, Debug)]
pub struct MilitaryProvince {
    /// Terrain selects crossing cost and battlefield frontage.
    pub terrain: MilitaryTerrain,
    /// Map area in any consistent positive square units.
    pub area: f64,
    /// Current completed road level.
    pub road_level: u32,
    /// Land borders and explicit traversable sea crossings.
    pub neighbors: Vec<ProvinceId>,
}

/// Movement permission separates peaceful access from hostile invasion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MilitaryAccess {
    /// Directly owned, own vassal, friendly NPC, or explicitly invited.
    Peaceful,
    /// Hostile entry, which may begin a battle on arrival.
    Invasion,
    /// No diplomatic permission and no declaration of hostility.
    Blocked,
}

/// Transient selected units on one route, consumed when they reach their destination.
#[derive(Clone, Debug)]
pub struct MovementOrder {
    /// Stable order identity.
    pub id: u64,
    /// Military owner of every contained unit.
    pub owner: ForceOwner,
    /// Troops removed from the origin force while traveling.
    pub units: Vec<Unit>,
    /// Province from which the current edge starts.
    pub origin: ProvinceId,
    /// Remaining route, starting with the next adjacent province.
    pub route: Vec<ProvinceId>,
    /// Completed monthly travel on the current edge.
    pub progress: f64,
    /// Required edge travel, always at least one observable monthly tick.
    pub required_progress: f64,
    /// Snapshot, editable only before combat starts.
    pub plan: BattlePlan,
}

impl MovementOrder {
    /// Current adjacent destination.
    pub fn destination(&self) -> Option<ProvinceId> {
        self.route.first().copied()
    }
    /// Animation fraction between province anchors.
    pub fn interpolation(&self) -> f64 {
        (self.progress / self.required_progress.max(0.001)).clamp(0., 1.)
    }
    /// Slowest surviving selected unit sets speed.
    pub fn speed(&self, config: &MilitaryConfig) -> f64 {
        force_speed(&self.units, config)
    }
}

/// Speed of the slowest selected living troop; an empty selection cannot move.
pub fn force_speed(units: &[Unit], config: &MilitaryConfig) -> f64 {
    units
        .iter()
        .filter(|u| u.current_manpower > 0.)
        .map(|u| config.unit(u.unit_type).movement_speed)
        .reduce(f64::min)
        .unwrap_or(0.)
}

/// Median positive map area, cached by the integration layer if desired.
pub fn median_province_area(graph: &[MilitaryProvince]) -> f64 {
    let mut areas: Vec<_> =
        graph.iter().map(|p| p.area).filter(|a| a.is_finite() && *a > 0.).collect();
    areas.sort_by(f64::total_cmp);
    if areas.is_empty() {
        1.
    } else if areas.len() % 2 == 0 {
        (areas[areas.len() / 2 - 1] + areas[areas.len() / 2]) * 0.5
    } else {
        areas[areas.len() / 2]
    }
}

/// Continuous edge duration; the movement resolver allows only one edge per tick.
pub fn edge_travel_months(
    a: &MilitaryProvince,
    b: &MilitaryProvince,
    median_area: f64,
    speed: f64,
    config: &MilitaryConfig,
) -> f64 {
    let crossing = |p: &MilitaryProvince| {
        let road = (1. / (1. + config.road_speed_bonus * f64::from(p.road_level)))
            .max(config.minimum_road_cost);
        (p.area.max(0.001) / median_area.max(0.001)).sqrt()
            * config.terrain_movement[p.terrain as usize]
            * road
    };
    config.movement_scale * 0.5 * (crossing(a) + crossing(b)) * config.reference_speed
        / speed.max(0.001)
}

/// Dijkstra finds the fastest legal route with stable province-id tie breaks.
/// Hostile provinces may be final destinations but cannot be crossed without
/// stopping, making a battle destination explicit instead of routing through it.
pub fn fastest_route(
    graph: &[MilitaryProvince],
    origin: ProvinceId,
    destination: ProvinceId,
    owner: ForceOwner,
    units: &[Unit],
    access: impl Fn(ForceOwner, ProvinceId) -> MilitaryAccess,
    config: &MilitaryConfig,
) -> Result<Vec<ProvinceId>, MilitaryError> {
    if origin >= graph.len() || destination >= graph.len() {
        return Err(MilitaryError::UnknownProvince);
    }
    if origin == destination || units.is_empty() {
        return Err(MilitaryError::InvalidUnits);
    }
    let speed = force_speed(units, config);
    if speed <= 0. {
        return Err(MilitaryError::InvalidUnits);
    }
    let median = median_province_area(graph);
    let mut distance = vec![f64::INFINITY; graph.len()];
    let mut previous = vec![None; graph.len()];
    let mut visited = vec![false; graph.len()];
    distance[origin] = 0.;
    while let Some(current) = (0..graph.len())
        .filter(|&i| !visited[i] && distance[i].is_finite())
        .min_by(|&a, &b| distance[a].total_cmp(&distance[b]).then(a.cmp(&b)))
    {
        if current == destination {
            break;
        }
        visited[current] = true;
        if current != origin && access(owner, current) == MilitaryAccess::Invasion {
            continue;
        }
        let mut neighbors = graph[current].neighbors.clone();
        neighbors.sort_unstable();
        for next in neighbors {
            if next >= graph.len()
                || visited[next]
                || access(owner, next) == MilitaryAccess::Blocked
            {
                continue;
            }
            // Each edge consumes at least one simulation tick, even with fast cavalry.
            let candidate = distance[current]
                + edge_travel_months(&graph[current], &graph[next], median, speed, config)
                    .max(1.)
                    .ceil();
            if candidate < distance[next] {
                distance[next] = candidate;
                previous[next] = Some(current);
            }
        }
    }
    if !distance[destination].is_finite() {
        return Err(MilitaryError::NoLegalRoute);
    }
    let mut route = vec![destination];
    let mut current = destination;
    while current != origin {
        current = previous[current].ok_or(MilitaryError::NoLegalRoute)?;
        if current != origin {
            route.push(current);
        }
    }
    route.reverse();
    Ok(route)
}

/// Find the fastest legal legs through an ordered list of player-chosen waypoints.
pub fn route_via(
    graph: &[MilitaryProvince],
    origin: ProvinceId,
    destination: ProvinceId,
    waypoints: &[ProvinceId],
    owner: ForceOwner,
    units: &[Unit],
    access: impl Fn(ForceOwner, ProvinceId) -> MilitaryAccess,
    config: &MilitaryConfig,
) -> Result<Vec<ProvinceId>, MilitaryError> {
    let mut route = Vec::new();
    let mut current = origin;
    for &next in waypoints.iter().chain(std::iter::once(&destination)) {
        if next == current {
            continue;
        }
        if next != destination && access(owner, next) != MilitaryAccess::Peaceful {
            return Err(MilitaryError::NoLegalRoute);
        }
        route.extend(fastest_route(graph, current, next, owner, units, &access, config)?);
        current = next;
    }
    if route.is_empty() {
        return Err(MilitaryError::NoLegalRoute);
    }
    validate_route(graph, origin, &route, owner, &access)?;
    Ok(route)
}

/// Validate every selected border and permission before any troops leave the province.
pub fn validate_route(
    graph: &[MilitaryProvince],
    origin: ProvinceId,
    route: &[ProvinceId],
    owner: ForceOwner,
    access: impl Fn(ForceOwner, ProvinceId) -> MilitaryAccess,
) -> Result<(), MilitaryError> {
    if origin >= graph.len() || route.is_empty() {
        return Err(MilitaryError::NoLegalRoute);
    }
    let mut current = origin;
    for (step, &next) in route.iter().enumerate() {
        if next >= graph.len() || !graph[current].neighbors.contains(&next) {
            return Err(MilitaryError::NoLegalRoute);
        }
        let permission = access(owner, next);
        if permission == MilitaryAccess::Blocked
            || (permission == MilitaryAccess::Invasion && step + 1 < route.len())
        {
            return Err(MilitaryError::NoLegalRoute);
        }
        current = next;
    }
    Ok(())
}

/// Legal retreat prefers the attack origin, then the smallest adjacent province ID.
pub fn retreat_destination(
    graph: &[MilitaryProvince],
    province: ProvinceId,
    owner: ForceOwner,
    origin: Option<ProvinceId>,
    access: impl Fn(ForceOwner, ProvinceId) -> MilitaryAccess,
) -> Option<ProvinceId> {
    let neighbors = &graph.get(province)?.neighbors;
    if let Some(origin) = origin {
        if neighbors.contains(&origin) && access(owner, origin) == MilitaryAccess::Peaceful {
            return Some(origin);
        }
    }
    neighbors.iter().copied().filter(|&p| access(owner, p) == MilitaryAccess::Peaceful).min()
}
