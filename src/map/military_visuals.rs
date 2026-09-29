//! Lazy close-zoom military sprites, with sixteen-frame relaxed standing animations.
//!
//! The original atlas supplies icons / movement / combat; idle uses a four-by-four sheet.
//! Owner badges fade together with troops, whose anchors stay inside their province.

use super::*;
use crate::game::military::{
    BattlePlan, ForceOwner, MilitaryConfig, MilitaryWorld, Unit, UnitType,
};

/// Geographic anchors survive zoom, panning, clipping and changes to nearby artwork.
#[derive(Default)]
pub(super) struct Anchors {
    positions: std::collections::BTreeMap<(usize, ForceOwner, u8), [f32; 2]>,
}

impl Anchors {
    fn retain_for(&mut self, world: &MilitaryWorld) {
        self.positions.retain(|&(province, owner, side), _| {
            if side == 0 {
                world.provinces.get(province).is_some_and(|state| {
                    state.forces.get(&owner).is_some_and(|units| !units.is_empty())
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

/// Session cache loads no sprite texture until its type is visible at close zoom.
#[derive(Clone)]
struct Textures {
    original: [Option<egui::TextureHandle>; 11],
    idle: [Option<egui::TextureHandle>; 11],
}

impl Default for Textures {
    fn default() -> Self {
        Self {
            original: std::array::from_fn(|_| None),
            idle: std::array::from_fn(|_| None),
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
        data.insert_temp(egui::Id::new("map-army-hit-targets"), Vec::<ArmyHit>::new())
    });
    anchors.retain_for(world);
    let threshold = world.config.sprite_zoom_threshold.max(0.0) as f32;
    if zoom < threshold {
        return vec![];
    }
    let atlas = atlas();
    // Like city and wonder artwork, units keep one map footprint as the camera zooms.
    let size = troop_size(zoom);
    let alpha = troop_alpha(zoom, threshold);
    if alpha == 0 {
        return vec![];
    }
    let cache_id = egui::Id::new("military-sprite-sheet-cache");
    let mut textures =
        painter.ctx().data_mut(|data| data.get_temp::<Textures>(cache_id)).unwrap_or_default();
    let mut occupied = vec![];
    let mut army_hits = vec![];
    for (province, state) in world.provinces.iter().enumerate() {
        let Some(map_province) = atlas.provinces.get(province) else {
            continue;
        };
        let anchor = projection.point(map_province.visual_center);
        if !viewport.expand(size * 2.).contains(anchor) {
            continue;
        }
        let owners: Vec<_> = state.forces.iter().filter(|(_, units)| !units.is_empty()).collect();
        for (cluster, (&owner, units)) in owners.iter().enumerate() {
            let plan = state.plans.get(&owner).copied().unwrap_or_default();
            let types = map_representatives(units, plan, &world.config);
            // Leave the centered province name and its resources room first.
            // This is a placement preference, never a reason to move an existing army.
            let label_center = labels
                .get(province)
                .and_then(Option::as_ref)
                .map_or(anchor, |label| projection.point(label.center));
            let offset = egui::vec2(0., -size * 1.25) + cluster_offset(cluster, owners.len(), size);
            let Some(center) = anchors.get_or_place((province, owner, 0), projection, || {
                let place = |obstacles: &[egui::Rect]| {
                    province_anchor(
                        label_center + offset,
                        size,
                        viewport,
                        obstacles,
                        &occupied,
                        map_province,
                        projection,
                        types.len(),
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
            );
        }
    }
    for movement in &world.movements {
        let types = map_representatives(&movement.units, movement.plan, &world.config);
        let (Some(origin), Some(destination)) = (
            atlas.provinces.get(movement.origin),
            movement.destination().and_then(|p| atlas.provinces.get(p)),
        ) else {
            continue;
        };
        let start = projection.point(origin.visual_center);
        let end = projection.point(destination.visual_center);
        let fraction = movement.interpolation() as f32;
        // Small repeated stride gives movement feedback while the authoritative monthly progress stays discrete.
        let route_anchor = start.lerp(end, fraction);
        let anchor = route_anchor;
        if !viewport.expand(size).contains(anchor) {
            continue;
        }
        let color = owner_color(movement.owner, ownership);
        painter.line_segment([start, end], egui::Stroke::new(1.3, color.gamma_multiply(0.45)));
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
        );
    }
    for battle in &world.battles {
        let Some(map_province) = atlas.provinces.get(battle.province) else {
            continue;
        };
        let anchor = projection.point(map_province.visual_center);
        if !viewport.expand(size * 2.).contains(anchor) {
            continue;
        }
        for (key, direction, side) in [(1, -1., &battle.attackers), (2, 1., &battle.defenders)] {
            let mut owners: Vec<_> = side
                .units
                .iter()
                .map(|unit| unit.owner)
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            owners.sort_unstable();
            for (cluster, owner) in owners.iter().enumerate() {
                let active: Vec<_> = side
                    .units
                    .iter()
                    // Map composition is public even when foreign deployment is redacted.
                    .filter(|u| u.owner == *owner && u.current_manpower > 0.)
                    .cloned()
                    .collect();
                let plan = side.plans.get(owner).copied().unwrap_or_default();
                let types = map_representatives(&active, plan, &world.config);
                let Some(center) =
                    anchors.get_or_place((battle.province, *owner, key), projection, || {
                        province_anchor(
                            anchor
                                + egui::vec2(direction * size * 0.6, cluster as f32 * size * 0.65),
                            size,
                            viewport,
                            landmarks,
                            &occupied,
                            map_province,
                            projection,
                            types.len(),
                        )
                    })
                else {
                    continue;
                };
                draw_cluster(
                    painter,
                    &mut textures,
                    world,
                    ownership,
                    *owner,
                    &active,
                    &types,
                    center,
                    size,
                    3,
                    clock,
                    alpha,
                    &mut occupied,
                    &mut army_hits,
                    battle.province,
                    None,
                );
            }
        }
        painter.text(
            anchor + egui::vec2(0., -size * 0.65),
            egui::Align2::CENTER_CENTER,
            "BATTLE",
            egui::FontId::proportional(10.),
            egui::Color32::from_rgba_unmultiplied(110, 35, 28, alpha),
        );
    }
    painter.ctx().data_mut(|data| data.insert_temp(cache_id, textures));
    if !occupied.is_empty() {
        painter.ctx().request_repaint_after(std::time::Duration::from_millis(80));
    }
    painter
        .ctx()
        .data_mut(|data| data.insert_temp(egui::Id::new("map-army-hit-targets"), army_hits));
    occupied
}

fn troop_alpha(zoom: f32, threshold: f32) -> u8 {
    (smoothstep((zoom - threshold) / 0.5) * 255.).round() as u8
}

fn troop_size(zoom: f32) -> f32 {
    8. * zoom
}

fn cluster_bounds(anchor: egui::Pos2, size: f32, count: usize) -> egui::Rect {
    let width = (size * (2. + count.saturating_sub(1) as f32 * 0.52)).max(27.);
    egui::Rect::from_min_max(
        anchor - egui::vec2(width * 0.5, size * 1.5),
        anchor + egui::vec2(width * 0.5, size * 0.5 + 13.),
    )
}

/// Keep unit feet and their badge inside their own province. Upright artwork can
/// extend above its ground footprint, as city and wonder illustrations do.
/// Narrow provinces hide a group until zoom provides room to show it safely.
fn province_anchor(
    desired: egui::Pos2,
    size: f32,
    _viewport: egui::Rect,
    landmarks: &[egui::Rect],
    troops: &[egui::Rect],
    province: &Province,
    projection: &Projection,
    count: usize,
) -> Option<egui::Pos2> {
    let province_bounds = projection.bounds_rect(province.bounds);
    let valid = |point: egui::Pos2| {
        let bounds = cluster_bounds(point, size, count);
        province_bounds.contains_rect(ground_footprint(point, size, count))
            && !landmarks.iter().chain(troops).any(|other| bounds.intersects(*other))
            && province.contains(projection.inverse(point))
            && footprint_in_province(ground_footprint(point, size, count), province, projection)
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

/// Show the force's selected formation roles in their map order. A missing
/// preferred type is replaced by a surviving type suited to that role.
fn map_representatives(units: &[Unit], plan: BattlePlan, config: &MilitaryConfig) -> Vec<UnitType> {
    let mut available = [false; 11];
    for unit in units.iter().filter(|unit| unit.current_manpower > 0.) {
        available[unit.unit_type as usize] = true;
    }
    let preferred = [plan.primary_unit_type, plan.secondary_unit_type, plan.flank_unit_type];
    let mut roles = [None; 3];
    // Reserve every present preference before choosing fallbacks so a fallback
    // for one role cannot steal another role's selected type.
    for (role, kind) in preferred.into_iter().enumerate() {
        if available[kind as usize] && !roles.contains(&Some(kind)) {
            roles[role] = Some(kind);
        }
    }
    for (role, candidates) in [
        [&config.center_priority[..], &[][..]],
        [&config.support_priority[..], &config.center_priority[..]],
        [&config.flank_priority[..], &[][..]],
    ]
    .into_iter()
    .enumerate()
    {
        if roles[role].is_none() {
            roles[role] = candidates
                .into_iter()
                .flatten()
                .copied()
                .find(|kind| available[*kind as usize] && !roles.contains(&Some(*kind)));
        }
    }
    roles.into_iter().flatten().take(3).collect()
}

/// Draw up to three meaningful troop types, never one sprite per actual cohort.
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
) {
    if types.is_empty() {
        return;
    }
    let owner_color = owner_color(owner, ownership);
    for (index, kind) in types.iter().enumerate() {
        let center =
            anchor + egui::vec2((index as f32 - (types.len() as f32 - 1.) * 0.5) * size * 0.52, 0.);
        // Enlarge the artwork upward, keeping its feet and owner badge at the
        // existing ground anchor so armies still fit in narrow provinces.
        let rect = egui::Rect::from_center_size(
            center - egui::vec2(0., size * 0.5),
            egui::Vec2::splat(size * 2.),
        );
        let (texture, uv) = if row == 1 {
            let seed = units
                .first()
                .map_or(0, |unit| unit.id)
                .wrapping_mul(37)
                .wrapping_add(*kind as u64 * 11);
            let frame = idle_frame(clock, seed);
            (idle_texture(painter.ctx(), textures, *kind), idle_uv(frame))
        } else {
            let frame = ((clock * 5.) as usize + index) % 4;
            (
                texture(painter.ctx(), textures, *kind),
                egui::Rect::from_min_max(
                    egui::pos2(frame as f32 / 4., row as f32 / 4.),
                    egui::pos2((frame + 1) as f32 / 4., (row + 1) as f32 / 4.),
                ),
            )
        };
        painter.image(texture, rect, uv, egui::Color32::from_white_alpha(alpha));
        occupied.push(rect);
        army_hits.push(ArmyHit {
            rect,
            province,
            owner,
            movement,
        });
    }
    let banner = egui::Rect::from_center_size(
        anchor + egui::vec2(0., size * 0.5 + 7.),
        egui::vec2(27., 12.),
    );
    painter.rect_filled(banner, 2., owner_color.gamma_multiply(f32::from(alpha) / 255.));
    let caption = match owner {
        ForceOwner::Player(p) => format!("P{}", p + 1),
        ForceOwner::Local(_) if world.provinces[province].slave_rebellion => "REB".to_owned(),
        ForceOwner::Local(_) => "NPC".to_owned(),
    };
    painter.text(
        banner.center(),
        egui::Align2::CENTER_CENTER,
        caption,
        egui::FontId::proportional(9.),
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

/// Decode at most one modest 768px sheet per type; source art stays full resolution.
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

fn idle_uv(frame: usize) -> egui::Rect {
    let column = frame % 4;
    let row = frame / 4;
    egui::Rect::from_min_max(
        egui::pos2(column as f32 / 4., row as f32 / 4.),
        egui::pos2((column + 1) as f32 / 4., (row + 1) as f32 / 4.),
    )
}

// Hold settled poses, then play short glances and posture adjustments. Different
// soldiers have different phase and pace, including armies of the same unit type.
const IDLE_DURATIONS: [f32; 16] =
    [1.6, 0.18, 0.18, 0.18, 0.7, 0.18, 0.18, 0.18, 1.4, 0.18, 0.18, 0.18, 0.8, 0.18, 0.18, 0.18];

fn idle_frame(clock: f32, seed: u64) -> usize {
    let duration: f32 = IDLE_DURATIONS.iter().sum();
    let pace = 0.85 + (seed % 17) as f32 * 0.018;
    let phase = (seed % 997) as f32 / 997. * duration;
    let mut time = (clock * pace + phase).rem_euclid(duration);
    for (frame, &hold) in IDLE_DURATIONS.iter().enumerate() {
        if time < hold {
            return frame;
        }
        time -= hold;
    }
    0
}

fn idle_texture(context: &egui::Context, cache: &mut Textures, kind: UnitType) -> egui::TextureId {
    cache.idle[kind as usize]
        .get_or_insert_with(|| {
            let decoded = image::load_from_memory(IDLE_SHEETS[kind as usize])
                .expect("military idle sheet must decode")
                .to_rgba8();
            context.load_texture(
                format!("military-idle-{}", kind.abbreviation()),
                egui::ColorImage::from_rgba_unmultiplied([768, 768], decoded.as_raw()),
                egui::TextureOptions::LINEAR,
            )
        })
        .id()
}

/// Reuse the current owner's exact map-banner color with a neutral bronze for NPCs.
fn owner_color(owner: ForceOwner, ownership: &ProvinceOwnership) -> egui::Color32 {
    match owner {
        ForceOwner::Player(player) => ownership
            .player_colors
            .get(player)
            .copied()
            .unwrap_or(egui::Color32::from_rgb(146, 47, 40)),
        ForceOwner::Local(_) => egui::Color32::from_rgb(127, 99, 66),
    }
}

#[cfg(test)]
#[path = "../../tests/unit/map_military.rs"]
mod tests;
