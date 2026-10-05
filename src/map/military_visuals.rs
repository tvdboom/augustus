//! Lazy close-zoom military sprites with continuous, 48-frame motion cycles.
//!
//! Every motion atlas uses one fixed character and an eight-by-six frame grid.
//! Owner badges fade together with troops, whose anchors stay inside their province.

use super::*;
use crate::game::military::{ForceOwner, MilitaryWorld, MovementOrder, Unit, UnitType};

#[derive(Clone)]
struct MarchPreview {
    origin: usize,
    destination: Option<usize>,
    start_fraction: f64,
}

/// World motion remains tied to army speed, at one quarter of its travel velocity.
const MARCH_ARROW_FLOW: f32 = 0.25;

struct MarchRoute {
    points: Vec<egui::Pos2>,
    start: egui::Pos2,
    travelled: f32,
    size: f32,
    color: egui::Color32,
    target: Option<(usize, ForceOwner)>,
}

/// Preview monthly travel continuously, sharing exactly the same fraction with the army bar.
pub(super) fn movement_visual_progress(ctx: &egui::Context, order: &MovementOrder) -> f32 {
    ctx.data_mut(|data| {
        let fraction = f64::from(
            data.get_temp::<f32>(egui::Id::new("campaign-construction-month-fraction"))
                .unwrap_or(0.),
        );
        let key = egui::Id::new(("march-preview", order.id));
        let previous = data.get_temp::<MarchPreview>(key);
        let start_fraction = match previous {
            Some(previous)
                if previous.origin == order.origin
                    && previous.destination == order.destination() =>
            {
                previous.start_fraction
            },
            Some(_) => 0., // A monthly boundary starts the next edge at the province we just reached.
            None if order.progress == 0. => fraction,
            None => 0.,
        };
        data.insert_temp(
            key,
            MarchPreview {
                origin: order.origin,
                destination: order.destination(),
                start_fraction,
            },
        );
        // The resolver rounds an edge up to whole monthly ticks. Do the same here
        // so a short crossing keeps moving until its actual arrival, without a pause.
        let duration = order.required_progress.ceil().max(1.);
        ((order.progress + fraction - start_fraction) / (duration - start_fraction).max(0.001))
            .clamp(0., 1.) as f32
    })
}

/// Flowing open chevrons follow the remaining route from the army.
/// Using travel progress keeps their motion tied to army speed and the paused clock.
fn paint_march_arrows(
    shapes: &mut Vec<egui::Shape>,
    route: &[egui::Pos2],
    phase: f32,
    size: f32,
    color: egui::Color32,
) {
    let scale = (size / 48.).clamp(0.75, 1.5);
    let spacing = 44. * scale;
    let clearance = size * 0.55;
    let length: f32 = route.windows(2).map(|edge| edge[0].distance(edge[1])).sum();
    let mut distance = clearance + phase.rem_euclid(spacing);
    while distance < length - 8. * scale {
        let mut along = distance;
        for edge in route.windows(2) {
            let delta = edge[1] - edge[0];
            let edge_length = delta.length();
            if edge_length <= f32::EPSILON {
                continue;
            }
            if along > edge_length {
                along -= edge_length;
                continue;
            }
            let forward = delta / edge_length;
            let across = egui::vec2(-forward.y, forward.x);
            let tip = edge[0] + forward * along;
            let fade = ((distance - clearance) / (16. * scale))
                .min((length - distance) / (20. * scale))
                .clamp(0., 1.);
            if fade <= 0. {
                break;
            }
            let ink = color.gamma_multiply(0.95 * fade);
            shapes.push(egui::Shape::line(
                vec![
                    tip - forward * (12. * scale) + across * (8. * scale),
                    tip,
                    tip - forward * (12. * scale) - across * (8. * scale),
                ],
                egui::Stroke::new(3.2 * scale, ink),
            ));
            break;
        }
        distance += spacing;
    }
}

#[path = "military_frames.rs"]
mod frames;

#[path = "military_combat.rs"]
mod combat;

/// Geographic anchors survive zoom, panning, clipping and changes to nearby artwork.
#[derive(Default)]
pub(super) struct Anchors {
    positions: std::collections::BTreeMap<(usize, ForceOwner, u8), [f32; 2]>,
    battles: std::collections::BTreeMap<u64, combat::FieldAnchor>,
}

impl Anchors {
    fn retain_for(&mut self, world: &MilitaryWorld) {
        self.battles.retain(|id, _| world.battles.iter().any(|battle| battle.id == *id));
        self.positions.retain(|&(province, owner, side), _| {
            if side == 0 {
                world.provinces.get(province).is_some_and(|state| {
                    state.forces.get(&owner).is_some_and(|units| !units.is_empty())
                }) || world.movements.iter().any(|order| {
                    order.owner == owner
                        && (order.origin == province || order.route.contains(&province))
                })
            } else {
                world.battles.iter().any(|battle| {
                    battle.province == province
                        && (if side == 1 {
                            &battle.attackers
                        } else {
                            &battle.defenders
                        })
                        .units
                        .iter()
                        .any(|unit| unit.owner == owner)
                })
            }
        });
    }

    fn get_or_place(
        &mut self,
        key: (usize, ForceOwner, u8),
        projection: &Projection,
        place: impl FnOnce() -> Option<egui::Pos2>,
    ) -> Option<egui::Pos2> {
        if let Some(&position) = self.positions.get(&key) {
            return Some(projection.point(position));
        }
        let point = place()?;
        self.positions.insert(key, projection.inverse(point));
        Some(point)
    }
}

/// Embedded source sheets follow the existing map-art embedding convention.
const SHEETS: [&[u8]; 11] = [
    include_bytes!(concat!(env!("OUT_DIR"), "/animations/military/light-infantry.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/animations/military/heavy-infantry.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/animations/military/archers.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/animations/military/light-cavalry.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/animations/military/heavy-cavalry.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/animations/military/horse-archers.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/animations/military/war-chariots.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/animations/military/war-camels.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/animations/military/war-elephants.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/animations/military/ballista.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/animations/military/catapult.png")),
];

const IDLE_SHEETS: [&[u8]; 11] = [
    include_bytes!(concat!(env!("OUT_DIR"), "/animations/military/idle/light-infantry.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/animations/military/idle/heavy-infantry.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/animations/military/idle/archers.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/animations/military/idle/light-cavalry.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/animations/military/idle/heavy-cavalry.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/animations/military/idle/horse-archers.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/animations/military/idle/war-chariots.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/animations/military/idle/war-camels.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/animations/military/idle/war-elephants.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/animations/military/idle/ballista.png")),
    include_bytes!(concat!(env!("OUT_DIR"), "/animations/military/idle/catapult.png")),
];

macro_rules! motion_sheets {
    ($motion:literal) => {
        [
            include_bytes!(concat!(
                env!("OUT_DIR"),
                "/animations/military/",
                $motion,
                "/light-infantry.png"
            )) as &[u8],
            include_bytes!(concat!(
                env!("OUT_DIR"),
                "/animations/military/",
                $motion,
                "/heavy-infantry.png"
            )) as &[u8],
            include_bytes!(concat!(
                env!("OUT_DIR"),
                "/animations/military/",
                $motion,
                "/archers.png"
            )) as &[u8],
            include_bytes!(concat!(
                env!("OUT_DIR"),
                "/animations/military/",
                $motion,
                "/light-cavalry.png"
            )) as &[u8],
            include_bytes!(concat!(
                env!("OUT_DIR"),
                "/animations/military/",
                $motion,
                "/heavy-cavalry.png"
            )) as &[u8],
            include_bytes!(concat!(
                env!("OUT_DIR"),
                "/animations/military/",
                $motion,
                "/horse-archers.png"
            )) as &[u8],
            include_bytes!(concat!(
                env!("OUT_DIR"),
                "/animations/military/",
                $motion,
                "/war-chariots.png"
            )) as &[u8],
            include_bytes!(concat!(
                env!("OUT_DIR"),
                "/animations/military/",
                $motion,
                "/war-camels.png"
            )) as &[u8],
            include_bytes!(concat!(
                env!("OUT_DIR"),
                "/animations/military/",
                $motion,
                "/war-elephants.png"
            )) as &[u8],
            include_bytes!(concat!(
                env!("OUT_DIR"),
                "/animations/military/",
                $motion,
                "/ballista.png"
            )) as &[u8],
            include_bytes!(concat!(
                env!("OUT_DIR"),
                "/animations/military/",
                $motion,
                "/catapult.png"
            )) as &[u8],
        ]
    };
}

const MOVEMENT_SHEETS: [&[u8]; 11] = motion_sheets!("movement");
const COMBAT_SHEETS: [&[u8]; 11] = motion_sheets!("combat");

#[derive(Clone, Copy)]
enum Animation {
    Idle,
    Movement,
    Combat,
}

impl Animation {
    fn seconds(self) -> f32 {
        match self {
            Self::Idle => 4.0,
            Self::Movement => 1.6,
            Self::Combat => 2.4,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Movement => "movement",
            Self::Combat => "combat",
        }
    }
}

/// Session cache loads no sprite texture until its type is visible at close zoom.
#[derive(Clone)]
struct Textures {
    original: [Option<egui::TextureHandle>; 11],
    idle: [Option<egui::TextureHandle>; 11],
    movement: [Option<egui::TextureHandle>; 11],
    combat: [Option<egui::TextureHandle>; 11],
}

impl Default for Textures {
    fn default() -> Self {
        Self {
            original: std::array::from_fn(|_| None),
            idle: std::array::from_fn(|_| None),
            movement: std::array::from_fn(|_| None),
            combat: std::array::from_fn(|_| None),
        }
    }
}

/// Share the lazily decoded sheet with military-panel icons (first cell, row zero).
pub(super) fn unit_icon(context: &egui::Context, kind: UnitType) -> egui::TextureId {
    let key = egui::Id::new("military-sprite-sheet-cache");
    let mut textures = context.data_mut(|data| data.get_temp::<Textures>(key)).unwrap_or_default();
    let id = texture(context, &mut textures, kind);
    context.data_mut(|data| data.insert_temp(key, textures));
    id
}

/// Draw owner-separated stationary, traveling, and fighting representatives.
/// Draw above province names; label layout never depends on these rectangles.
pub(super) fn paint(
    painter: &egui::Painter,
    route_layer: Option<egui::layers::ShapeIdx>,
    world: &MilitaryWorld,
    ownership: &ProvinceOwnership,
    projection: &Projection,
    zoom: f32,
    clock: f32,
    viewport: egui::Rect,
    landmarks: &[egui::Rect],
    labels: &[Option<LabelPlacement>],
    label_areas: &[Option<egui::Rect>],
    anchors: &mut Anchors,
) -> Vec<egui::Rect> {
    painter.ctx().data_mut(|data| {
        data.insert_temp(egui::Id::new("map-army-hit-targets"), Vec::<ArmyHit>::new());
        data.insert_temp(egui::Id::new("map-audible-battles"), Vec::<AudibleBattle>::new());
    });
    anchors.retain_for(world);
    let revolts = active_revolt_provinces(world);
    let atlas = atlas();
    // Like city and wonder artwork, units keep one map footprint as the camera zooms.
    let size = troop_size(zoom);
    let alpha = troop_alpha(zoom);
    for movement in &world.movements {
        movement_visual_progress(painter.ctx(), movement);
    }
    if alpha == 0
        && !revolts
            .iter()
            .enumerate()
            .any(|(province, &active)| active && !world.province_in_battle(province))
    {
        return vec![];
    }
    let cache_id = egui::Id::new("military-sprite-sheet-cache");
    let mut textures =
        painter.ctx().data_mut(|data| data.get_temp::<Textures>(cache_id)).unwrap_or_default();
    let mut occupied = vec![];
    let mut army_hits = vec![];
    let mut march_routes = vec![];
    for (province, state) in world.provinces.iter().enumerate() {
        let Some(map_province) = atlas.provinces.get(province) else {
            continue;
        };
        let revolt = revolts[province];
        let (size, alpha) = revolt_troop_style(size, alpha, revolt);
        if alpha == 0 {
            continue;
        }
        let anchor = projection.point(map_province.visual_center);
        if !viewport.expand(size * 2.).contains(anchor) {
            continue;
        }
        let owners: Vec<_> = state.forces.iter().filter(|(_, units)| !units.is_empty()).collect();
        for (cluster, (&owner, units)) in owners.iter().enumerate() {
            let mut types = map_representatives(units);
            if revolt && zoom <= CITY_BLEND_START {
                types.truncate(1);
            }
            // Leave the centered province name and its resources room first.
            // This is a placement preference, never a reason to move an existing army.
            let label_center = labels
                .get(province)
                .and_then(Option::as_ref)
                .map_or(anchor, |label| projection.point(label.center));
            let offset = if map_province.name == "Latium" {
                egui::vec2(0., 0.)
            } else {
                egui::vec2(0., -size * 1.25)
            } + cluster_offset(cluster, owners.len(), size);
            let preferred_center = if map_province.name == "Latium" {
                projection.point([13.30, 41.65])
            } else {
                label_center
            };
            let Some(center) = anchors.get_or_place((province, owner, 0), projection, || {
                let place = |obstacles: &[egui::Rect]| {
                    province_anchor(
                        preferred_center + offset,
                        size,
                        viewport,
                        obstacles,
                        &occupied,
                        map_province,
                        projection,
                        &types,
                    )
                };
                // Labels keep their own center. Only a newly placed army tries
                // to give the whole name/resource group room, with overlap as fallback.
                if let Some(area) = label_areas.get(province).copied().flatten() {
                    let mut preferred = landmarks.to_vec();
                    preferred.push(area.expand(3.));
                    if let Some(point) = place(&preferred) {
                        return Some(point);
                    }
                }
                place(landmarks)
                    .or_else(|| {
                        // Narrow Latium must not shrink or hide its normal army
                        // when nearby artwork leaves no unobstructed rectangle.
                        (map_province.name == "Latium")
                            .then(|| {
                                province_anchor(
                                    preferred_center + offset,
                                    size,
                                    viewport,
                                    &[],
                                    &occupied,
                                    map_province,
                                    projection,
                                    &types,
                                )
                            })
                            .flatten()
                    })
                    .or_else(|| {
                        revolt
                            .then(|| {
                                revolt_anchor(
                                    preferred_center + offset,
                                    size,
                                    map_province,
                                    projection,
                                    &types,
                                    &occupied,
                                )
                            })
                            .flatten()
                    })
            }) else {
                continue;
            };
            draw_cluster(
                painter,
                &mut textures,
                world,
                ownership,
                owner,
                units,
                &types,
                center,
                size,
                1,
                clock,
                alpha,
                &mut occupied,
                &mut army_hits,
                province,
                None,
                false,
            );
        }
    }
    for movement in &world.movements {
        // Record departures even when sprites are hidden at this camera zoom.
        let fraction = movement_visual_progress(painter.ctx(), movement);
        if alpha == 0 {
            continue;
        }
        let types = map_representatives(&movement.units);
        let location = |id: usize| {
            atlas
                .provinces
                .get(id)
                .map(|p| p.visual_center)
                .or_else(|| (id == atlas.provinces.len()).then_some(CITIES[0].position))
        };
        let (Some(origin), Some(destination)) =
            (location(movement.origin), movement.destination().and_then(location))
        else {
            continue;
        };
        // Reuse the stationed army's geographic anchors at both ends of a march.
        // Arrival and the next edge then meet at exactly the same map position.
        let mut endpoint = |id: usize, geographic: [f32; 2]| {
            let fallback = projection.point(geographic);
            let Some(province) = atlas.provinces.get(id) else {
                return fallback;
            };
            let preferred = if province.name == "Latium" {
                projection.point([13.30, 41.65])
            } else {
                labels
                    .get(id)
                    .and_then(Option::as_ref)
                    .map_or(fallback, |label| projection.point(label.center))
                    + egui::vec2(0., -size * 1.25)
            };
            anchors
                .get_or_place((id, movement.owner, 0), projection, || {
                    let mut obstacles = landmarks.to_vec();
                    if let Some(area) = label_areas.get(id).copied().flatten() {
                        obstacles.push(area.expand(3.));
                    }
                    province_anchor(
                        preferred, size, viewport, &obstacles, &occupied, province, projection,
                        &types,
                    )
                    .or_else(|| {
                        province_anchor(
                            preferred, size, viewport, landmarks, &occupied, province, projection,
                            &types,
                        )
                    })
                    .or_else(|| {
                        province_anchor(
                            preferred,
                            size,
                            viewport,
                            &[],
                            &[],
                            province,
                            projection,
                            &types,
                        )
                    })
                })
                .unwrap_or(fallback)
        };
        let start = endpoint(movement.origin, origin);
        let end = endpoint(movement.destination().unwrap(), destination);
        let anchor = start.lerp(end, fraction);
        let mut route = vec![anchor];
        route.extend(
            movement
                .route
                .iter()
                .filter_map(|&id| location(id).map(|position| endpoint(id, position))),
        );
        let color =
            owner_color(movement.owner, world, ownership).gamma_multiply(f32::from(alpha) / 255.);
        march_routes.push(MarchRoute {
            points: route,
            start,
            travelled: fraction * start.distance(end),
            size,
            color,
            target: movement
                .attack_target
                .zip(movement.route.last().copied())
                .map(|(owner, province)| (province, owner)),
        });
        if !viewport.expand(size).contains(anchor) {
            continue;
        }
        draw_cluster(
            painter,
            &mut textures,
            world,
            ownership,
            movement.owner,
            &movement.units,
            &types,
            anchor,
            size,
            2,
            clock,
            alpha,
            &mut occupied,
            &mut army_hits,
            movement.origin,
            Some(movement.id),
            end.x < start.x,
        );
    }
    let mut audible = Vec::new();
    for battle in &world.battles {
        let Some(map_province) = atlas.provinces.get(battle.province).or_else(|| {
            (battle.province == atlas.provinces.len())
                .then(|| atlas.provinces.iter().find(|province| province.name == "Latium"))
                .flatten()
        }) else {
            continue;
        };
        // Fighting armies use the same zoom fade as other units, including rebels.
        if alpha == 0 {
            continue;
        }
        if !viewport.intersects(projection.bounds_rect(map_province.bounds).expand(size * 4.)) {
            continue;
        }
        if let Some(sound) = combat::paint(
            painter,
            &mut textures,
            world,
            ownership,
            battle,
            map_province,
            projection,
            zoom,
            CITY_BLEND_START,
            clock,
            size,
            alpha,
            viewport,
            landmarks,
            label_areas.get(battle.province).copied().flatten(),
            &mut anchors.battles,
            &mut occupied,
            &mut army_hits,
        ) {
            audible.push(sound);
        }
    }
    let mut route_shapes = vec![];
    for mut route in march_routes {
        if let Some((province, owner)) = route.target {
            if let Some(fighter) = army_hits.iter().find(|hit| {
                hit.province == province && hit.owner == owner && hit.movement.is_none()
            }) {
                *route.points.last_mut().unwrap() = fighter.rect.center();
            }
        }
        // Cancel the moving route origin using its distance to the actual next
        // endpoint, including a defender placed away from the arrival anchor.
        let phase = route.travelled * MARCH_ARROW_FLOW + route.points[0].distance(route.points[1])
            - route.start.distance(route.points[1]);
        paint_march_arrows(&mut route_shapes, &route.points, phase, route.size, route.color);
    }
    if let Some(layer) = route_layer {
        painter.set(layer, egui::Shape::Vec(route_shapes));
    } else {
        painter.extend(route_shapes);
    }
    painter.ctx().data_mut(|data| data.insert_temp(egui::Id::new("map-audible-battles"), audible));
    painter.ctx().data_mut(|data| data.insert_temp(cache_id, textures));
    if !occupied.is_empty() {
        painter.ctx().request_repaint_after(std::time::Duration::from_millis(33));
    }
    painter
        .ctx()
        .data_mut(|data| data.insert_temp(egui::Id::new("map-army-hit-targets"), army_hits));
    occupied
}

fn troop_alpha(zoom: f32) -> u8 {
    // Troops and their banners appear throughout the city's icon-to-image transition.
    (city_blend(zoom) * 255.).round() as u8
}

fn is_rebel(owner: ForceOwner, world: &MilitaryWorld) -> bool {
    matches!(owner, ForceOwner::Local(home)
        if world.provinces.get(home).is_some_and(|province| province.slave_rebellion))
}

fn active_revolt_provinces(world: &MilitaryWorld) -> Vec<bool> {
    let mut active = vec![false; world.provinces.len()];
    for (province, unit) in world.units_with_province() {
        if let Some(province) = province {
            if unit.current_manpower > 0.0 && is_rebel(unit.owner, world) {
                active[province] = true;
            }
        }
    }
    active
}

fn revolt_troop_style(size: f32, alpha: u8, active: bool) -> (f32, u8) {
    if active {
        (size.max(14.0), 255)
    } else {
        (size, alpha)
    }
}

/// An uprising must remain visible even when landmarks cover the available ground.
/// Like Latium's narrow ground, keep the feet and badge anchored inside the province.
fn revolt_anchor(
    desired: egui::Pos2,
    size: f32,
    province: &Province,
    projection: &Projection,
    types: &[UnitType],
    troops: &[egui::Rect],
) -> Option<egui::Pos2> {
    let overlap = |point: egui::Pos2| {
        let bounds = cluster_bounds(point, size, types);
        troops
            .iter()
            .filter(|&&rect| bounds.intersects(rect))
            .map(|&rect| bounds.intersect(rect).area())
            .sum::<f32>()
    };
    (0..24)
        .flat_map(|row| (0..24).map(move |column| (row, column)))
        .map(|(row, column)| {
            projection.point([
                province.bounds[0]
                    + (province.bounds[2] - province.bounds[0]) * (column as f32 + 0.5) / 24.,
                province.bounds[1]
                    + (province.bounds[3] - province.bounds[1]) * (row as f32 + 0.5) / 24.,
            ])
        })
        .filter(|&point| ground_anchors_in_province(point, size, types.len(), province, projection))
        // Ignore landmark obstruction for an urgent revolt, but keep opposing
        // army sprites and banners apart wherever the province has room.
        .min_by(|&a, &b| {
            overlap(a)
                .total_cmp(&overlap(b))
                .then_with(|| a.distance_sq(desired).total_cmp(&b.distance_sq(desired)))
        })
}

fn troop_size(zoom: f32) -> f32 {
    8. * zoom
}

fn cluster_bounds(anchor: egui::Pos2, size: f32, types: &[UnitType]) -> egui::Rect {
    let mut bounds = egui::Rect::NOTHING;
    for (index, &kind) in types.iter().enumerate() {
        bounds = bounds.union(troop_rect(anchor, size, index, types.len(), kind));
    }
    bounds.union(egui::Rect::from_center_size(
        anchor + egui::vec2(0., size * 0.5 + 7.),
        egui::vec2(27., 12.),
    ))
}

/// Keep unit feet and their badge inside their own province. Upright artwork can
/// extend above its ground footprint, as city and wonder illustrations do.
/// In tapering Latium, validate each planted anchor rather than requiring the
/// empty rectangle between the figures to fit the province as well.
fn province_anchor(
    desired: egui::Pos2,
    size: f32,
    _viewport: egui::Rect,
    landmarks: &[egui::Rect],
    troops: &[egui::Rect],
    province: &Province,
    projection: &Projection,
    types: &[UnitType],
) -> Option<egui::Pos2> {
    let province_bounds = projection.bounds_rect(province.bounds);
    let valid = |point: egui::Pos2| {
        let bounds = cluster_bounds(point, size, types);
        let ground_inside = if province.name == "Latium" {
            ground_anchors_in_province(point, size, types.len(), province, projection)
        } else {
            let footprint = ground_footprint(point, size, types.len());
            province_bounds.contains_rect(footprint)
                && footprint_in_province(footprint, province, projection)
        };
        ground_inside
            && !landmarks.iter().chain(troops).any(|other| bounds.intersects(*other))
            && province.contains(projection.inverse(point))
    };
    for ring in 0..=7 {
        for direction in 0..16 {
            let angle = direction as f32 * std::f32::consts::TAU / 16.;
            let candidate =
                desired + egui::vec2(angle.cos(), angle.sin()) * ring as f32 * size * 0.35;
            if valid(candidate) {
                return Some(candidate);
            }
        }
    }
    // A centroid can lie outside a concave province. Search its whole interior
    // instead of falling back to an unchecked point in the sea or next province.
    (0..24)
        .flat_map(|row| (0..24).map(move |column| (row, column)))
        .map(|(row, column)| {
            projection.point([
                province.bounds[0]
                    + (province.bounds[2] - province.bounds[0]) * (column as f32 + 0.5) / 24.,
                province.bounds[1]
                    + (province.bounds[3] - province.bounds[1]) * (row as f32 + 0.5) / 24.,
            ])
        })
        .filter(|&point| valid(point))
        .min_by(|a, b| a.distance_sq(desired).total_cmp(&b.distance_sq(desired)))
}

fn ground_anchors_in_province(
    anchor: egui::Pos2,
    size: f32,
    count: usize,
    province: &Province,
    projection: &Projection,
) -> bool {
    (0..count).all(|index| {
        let feet = anchor
            + egui::vec2((index as f32 - (count as f32 - 1.) * 0.5) * size * 0.52, size * 0.5);
        province.contains(projection.inverse(feet))
    }) && province.contains(projection.inverse(anchor + egui::vec2(0., size * 0.5 + 7.)))
}

fn ground_footprint(anchor: egui::Pos2, size: f32, count: usize) -> egui::Rect {
    let width = (size * (0.8 + count.saturating_sub(1) as f32 * 0.52)).max(27.);
    egui::Rect::from_min_max(
        anchor + egui::vec2(-width * 0.5, size * 0.30),
        anchor + egui::vec2(width * 0.5, size * 0.50 + 13.),
    )
}

fn footprint_in_province(bounds: egui::Rect, province: &Province, projection: &Projection) -> bool {
    (0..=4).all(|column| {
        (0..=4).all(|row| {
            let point = bounds.min
                + egui::vec2(
                    bounds.width() * column as f32 / 4.,
                    bounds.height() * row as f32 / 4.,
                );
            province.contains(projection.inverse(point))
        })
    })
}

/// Separate peaceful owners around the anchor without laying clusters on top of one another.
fn cluster_offset(index: usize, count: usize, size: f32) -> egui::Vec2 {
    let column = index % 3;
    let row = index / 3;
    egui::vec2(
        (column as f32 - (count.min(3) as f32 - 1.) * 0.5) * size * 1.35,
        row as f32 * size * 0.8,
    )
}

/// Show public army composition without exposing its private deployment plan.
/// Prefer infantry, ranged, cavalry, and special types, then fill any vacant
/// places with other surviving types. A fourth figure can show a light flank.
fn map_representatives(units: &[Unit]) -> Vec<UnitType> {
    use UnitType::*;
    let mut counts = [0_usize; 11];
    for unit in units.iter().filter(|unit| unit.current_manpower > 0.) {
        counts[unit.unit_type as usize] += 1;
    }
    let infantry = [HeavyInfantry, LightInfantry];
    let ranged = [Archers, HorseArchers];
    let cavalry = [HeavyCavalry, LightCavalry];
    let special = [WarElephants, WarCamels, WarChariots, Catapult, Ballista];
    let mut selected = Vec::with_capacity(4);
    for category in [&infantry[..], &ranged[..], &cavalry[..], &special[..]] {
        if let Some(&kind) = category.iter().find(|&&kind| counts[kind as usize] > 0) {
            selected.push(kind);
        }
    }
    // Fill spare slots with distinct types, including light cavalry when the
    // heavy cavalry already represents the main mounted force.
    for kind in [
        WarElephants,
        HeavyCavalry,
        HeavyInfantry,
        HorseArchers,
        WarCamels,
        WarChariots,
        Archers,
        LightCavalry,
        LightInfantry,
        Catapult,
        Ballista,
    ] {
        if selected.len() == 4 {
            break;
        }
        if counts[kind as usize] > 0 && !selected.contains(&kind) {
            selected.push(kind);
        }
    }
    // When fewer than four distinct types exist, use other live cohorts of
    // the strongest type rather than leaving an army of many cohorts half empty.
    for kind in [
        WarElephants,
        HeavyCavalry,
        HeavyInfantry,
        HorseArchers,
        WarCamels,
        WarChariots,
        Archers,
        LightCavalry,
        LightInfantry,
        Catapult,
        Ballista,
    ] {
        while selected.len() < 4
            && selected.iter().filter(|&&shown| shown == kind).count() < counts[kind as usize]
        {
            selected.push(kind);
        }
    }
    // A stable per-army shuffle makes the icons a composition sample rather
    // than a left-to-right hint about frontline, rear line, or flanks.
    let mut seed =
        units.iter().fold(0_u64, |seed, unit| seed ^ unit.id.wrapping_mul(0x9e37_79b9_7f4a_7c15));
    for index in (1..selected.len()).rev() {
        seed = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = seed;
        value ^= value >> 30;
        value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value ^= value >> 27;
        value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
        let other = (value ^ (value >> 31)) as usize % (index + 1);
        selected.swap(index, other);
    }
    selected
}

fn troop_scale(kind: UnitType) -> f32 {
    use UnitType::*;
    match kind {
        WarElephants => 2.15,
        WarCamels => 1.75,
        WarChariots => 1.80,
        HeavyCavalry | LightCavalry | HorseArchers => 1.55,
        HeavyInfantry | LightInfantry | Archers => 1.15,
        Ballista | Catapult => 1.35,
    }
}

fn troop_rect(
    anchor: egui::Pos2,
    size: f32,
    index: usize,
    count: usize,
    kind: UnitType,
) -> egui::Rect {
    let center = anchor + egui::vec2((index as f32 - (count as f32 - 1.) * 0.5) * size * 0.52, 0.);
    let extent = size * troop_scale(kind);
    let width = extent * 2.;
    // Align the painted ground anchor, not the transparent bottom of the tile.
    // Otherwise the larger mounts appear to float above the infantry's feet.
    let ground_inset = width * (1. - frames::BASELINE as f32 / frames::SIZE as f32);
    egui::Rect::from_center_size(
        center + egui::vec2(0., size * 0.5 - extent + ground_inset),
        egui::vec2(width, extent * 2.),
    )
}

/// Draw up to four meaningful troop types, never one sprite per actual cohort.
fn draw_cluster(
    painter: &egui::Painter,
    textures: &mut Textures,
    world: &MilitaryWorld,
    ownership: &ProvinceOwnership,
    owner: ForceOwner,
    units: &[Unit],
    types: &[UnitType],
    anchor: egui::Pos2,
    size: f32,
    row: usize,
    clock: f32,
    alpha: u8,
    occupied: &mut Vec<egui::Rect>,
    army_hits: &mut Vec<ArmyHit>,
    province: usize,
    movement: Option<u64>,
    facing_left: bool,
) {
    if types.is_empty() {
        return;
    }
    let owner_color = owner_color(owner, world, ownership);
    for (index, kind) in types.iter().enumerate() {
        let rect = troop_rect(anchor, size, index, types.len(), *kind);
        let animation = match row {
            2 => Animation::Movement,
            3 => Animation::Combat,
            _ => Animation::Idle,
        };
        let seed = units
            .first()
            .map_or(0, |unit| unit.id)
            .wrapping_mul(37)
            .wrapping_add(*kind as u64 * 11)
            .wrapping_add(index as u64 * 13);
        let frame = animation_frame(clock, seed, animation);
        let texture = motion_texture(painter.ctx(), textures, *kind, animation);
        let mut uv = animation_uv(frame);
        if facing_left {
            std::mem::swap(&mut uv.min.x, &mut uv.max.x);
        }
        painter.image(texture, rect, uv, egui::Color32::from_white_alpha(alpha));
        occupied.push(rect);
        army_hits.push(ArmyHit {
            rect,
            province,
            owner,
            movement,
        });
    }
    draw_banner(
        painter,
        world,
        owner,
        owner_color,
        anchor,
        size,
        alpha,
        occupied,
        army_hits,
        province,
        movement,
    );
}

fn draw_banner(
    painter: &egui::Painter,
    world: &MilitaryWorld,
    owner: ForceOwner,
    color: egui::Color32,
    anchor: egui::Pos2,
    size: f32,
    alpha: u8,
    occupied: &mut Vec<egui::Rect>,
    army_hits: &mut Vec<ArmyHit>,
    province: usize,
    movement: Option<u64>,
) {
    let banner = egui::Rect::from_center_size(
        anchor + egui::vec2(0., size * 0.5 + 7.),
        egui::vec2(27., 12.),
    );
    painter.rect_filled(banner, 2., color.gamma_multiply(f32::from(alpha) / 255.));
    let caption = match owner {
        ForceOwner::Player(p) => format!("P{}", p + 1),
        ForceOwner::Local(_) if is_rebel(owner, world) => "Revolt".to_owned(),
        ForceOwner::Local(_) => "NPC".to_owned(),
    };
    painter.text(
        banner.center(),
        egui::Align2::CENTER_CENTER,
        caption,
        egui::FontId::proportional(if is_rebel(owner, world) {
            8.
        } else {
            9.
        }),
        egui::Color32::from_white_alpha(alpha),
    );
    occupied.push(banner);
    army_hits.push(ArmyHit {
        rect: banner,
        province,
        owner,
        movement,
    });
}

/// Panel icons retain the original four-by-four source atlas.
fn texture(context: &egui::Context, cache: &mut Textures, kind: UnitType) -> egui::TextureId {
    cache.original[kind as usize]
        .get_or_insert_with(|| {
            let decoded = image::load_from_memory(SHEETS[kind as usize])
                .expect("military sprite sheet must decode")
                .to_rgba8();
            context.load_texture(
                format!("military-{}", kind.abbreviation()),
                egui::ColorImage::from_rgba_unmultiplied([768, 768], decoded.as_raw()),
                egui::TextureOptions::LINEAR,
            )
        })
        .id()
}

fn animation_uv(frame: usize) -> egui::Rect {
    let column = frame as u32 % frames::COLUMNS;
    let row = frame as u32 / frames::COLUMNS;
    egui::Rect::from_min_max(
        egui::pos2(column as f32 / frames::COLUMNS as f32, row as f32 / frames::ROWS as f32),
        egui::pos2(
            (column + 1) as f32 / frames::COLUMNS as f32,
            (row + 1) as f32 / frames::ROWS as f32,
        ),
    )
}

/// Uniform samples of a closed cycle: no duplicate endpoint or special pause.
fn animation_frame(clock: f32, seed: u64, animation: Animation) -> usize {
    let phase = (seed % frames::COUNT as u64) as f64;
    let frame = f64::from(clock) / f64::from(animation.seconds()) * frames::COUNT as f64 + phase;
    frame.floor().rem_euclid(frames::COUNT as f64) as usize
}

fn motion_texture(
    context: &egui::Context,
    cache: &mut Textures,
    kind: UnitType,
    animation: Animation,
) -> egui::TextureId {
    let (sheets, handles) = match animation {
        Animation::Idle => (&IDLE_SHEETS, &mut cache.idle),
        Animation::Movement => (&MOVEMENT_SHEETS, &mut cache.movement),
        Animation::Combat => (&COMBAT_SHEETS, &mut cache.combat),
    };
    handles[kind as usize]
        .get_or_insert_with(|| {
            let decoded = image::load_from_memory(sheets[kind as usize])
                .expect("military motion sheet must decode")
                .to_rgba8();
            context.load_texture(
                format!("military-{}-{}", animation.name(), kind.abbreviation()),
                egui::ColorImage::from_rgba_unmultiplied(
                    [frames::WIDTH as usize, frames::HEIGHT as usize],
                    decoded.as_raw(),
                ),
                egui::TextureOptions::LINEAR,
            )
        })
        .id()
}

/// Reuse the current owner's exact map-banner color with a neutral bronze for NPCs.
fn owner_color(
    owner: ForceOwner,
    world: &MilitaryWorld,
    ownership: &ProvinceOwnership,
) -> egui::Color32 {
    match owner {
        ForceOwner::Player(player) => ownership
            .player_colors
            .get(player)
            .copied()
            .unwrap_or(egui::Color32::from_rgb(146, 47, 40)),
        ForceOwner::Local(_) if is_rebel(owner, world) => egui::Color32::from_rgb(176, 45, 35),
        ForceOwner::Local(_) => egui::Color32::from_rgb(127, 99, 66),
    }
}

#[cfg(test)]
#[path = "../../tests/unit/map_military.rs"]
mod tests;
