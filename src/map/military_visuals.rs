//! Lazy close-zoom military sprites, using generated four-by-four animation sheets.
//!
//! The atlas layout is icon / idle / move / combat, with four frames per row.
//! Ownership rings preserve the same player colors as province borders.

use super::*;
use crate::game::military::{representative_types, ForceOwner, MilitaryWorld, Unit, UnitType};

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

/// Session cache loads no sprite texture until its type is visible at close zoom.
#[derive(Clone)]
struct Textures([Option<egui::TextureHandle>; 11]);

impl Default for Textures {
    fn default() -> Self {
        Self(std::array::from_fn(|_| None))
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
/// Returned rectangles reserve label space, preventing map text from covering troops.
pub(super) fn paint(
    painter: &egui::Painter,
    world: &MilitaryWorld,
    ownership: &ProvinceOwnership,
    projection: &Projection,
    zoom: f32,
    clock: f32,
    viewport: egui::Rect,
    landmarks: &[egui::Rect],
) -> Vec<egui::Rect> {
    let threshold = world.config.sprite_zoom_threshold.max(0.0) as f32;
    if zoom < threshold {
        return vec![];
    }
    let atlas = atlas();
    let size = (35. + zoom * 4.).clamp(42., 64.);
    let alpha = (((zoom - threshold) / 0.5).clamp(0., 1.) * 255.) as u8;
    let cache_id = egui::Id::new("military-sprite-sheet-cache");
    let mut textures =
        painter.ctx().data_mut(|data| data.get_temp::<Textures>(cache_id)).unwrap_or_default();
    let mut occupied = vec![];
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
            let offset = cluster_offset(cluster, owners.len(), size);
            let center = clear_anchor(anchor + offset, size, viewport, landmarks, &occupied);
            draw_cluster(
                painter,
                &mut textures,
                world,
                ownership,
                owner,
                units,
                center,
                size,
                1,
                clock,
                alpha,
                &mut occupied,
            );
        }
    }
    for movement in &world.movements {
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
        let anchor = clear_anchor(route_anchor, size, viewport, landmarks, &occupied);
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
            anchor,
            size,
            2,
            clock,
            alpha,
            &mut occupied,
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
        for (direction, side) in [(-1., &battle.attackers), (1., &battle.defenders)] {
            let mut owners: Vec<_> = side.plans.keys().copied().collect();
            owners.sort_unstable();
            for (cluster, owner) in owners.iter().enumerate() {
                let active: Vec<_> = side
                    .units
                    .iter()
                    .filter(|u| {
                        u.owner == *owner
                            && !side.routed.contains(&u.id)
                            && (side.formation.front.contains(&Some(u.id))
                                || side.formation.support.contains(&Some(u.id)))
                    })
                    .cloned()
                    .collect();
                let center = clear_anchor(
                    anchor + egui::vec2(direction * size * 0.6, cluster as f32 * size * 0.65),
                    size,
                    viewport,
                    landmarks,
                    &occupied,
                );
                draw_cluster(
                    painter,
                    &mut textures,
                    world,
                    ownership,
                    *owner,
                    &active,
                    center,
                    size,
                    3,
                    clock,
                    alpha,
                    &mut occupied,
                );
            }
        }
        painter.text(
            anchor + egui::vec2(0., -size * 0.65),
            egui::Align2::CENTER_CENTER,
            "BATTLE",
            egui::FontId::proportional(10.),
            egui::Color32::from_rgb(110, 35, 28),
        );
    }
    painter.ctx().data_mut(|data| data.insert_temp(cache_id, textures));
    if !occupied.is_empty() {
        painter.ctx().request_repaint_after(std::time::Duration::from_millis(160));
    }
    occupied
}

/// Keep representative clusters clear of cities, wonders and earlier troop groups.
/// Select the nearest available screen position; simulation anchors remain geographic.
fn clear_anchor(
    desired: egui::Pos2,
    size: f32,
    viewport: egui::Rect,
    landmarks: &[egui::Rect],
    troops: &[egui::Rect],
) -> egui::Pos2 {
    let extent = egui::vec2(size * 2.1, size * 1.1);
    for ring in 0..=7 {
        for direction in 0..8 {
            let angle = direction as f32 * std::f32::consts::FRAC_PI_4;
            let candidate =
                desired + egui::vec2(angle.cos(), angle.sin()) * ring as f32 * size * 0.65;
            let bounds = egui::Rect::from_center_size(candidate, extent);
            if viewport.contains_rect(bounds)
                && !landmarks.iter().chain(troops).any(|other| bounds.intersects(*other))
            {
                return candidate;
            }
        }
    }
    desired
}

/// Separate peaceful owners around the anchor without laying clusters on top of one another.
fn cluster_offset(index: usize, count: usize, size: f32) -> egui::Vec2 {
    let column = index % 3;
    let row = index / 3;
    egui::vec2(
        (column as f32 - (count.min(3) as f32 - 1.) * 0.5) * size * 1.35,
        size * 0.8 + row as f32 * size * 0.8,
    )
}

/// Draw up to three meaningful troop types, never one sprite per actual cohort.
fn draw_cluster(
    painter: &egui::Painter,
    textures: &mut Textures,
    world: &MilitaryWorld,
    ownership: &ProvinceOwnership,
    owner: ForceOwner,
    units: &[Unit],
    anchor: egui::Pos2,
    size: f32,
    row: usize,
    clock: f32,
    alpha: u8,
    occupied: &mut Vec<egui::Rect>,
) {
    let types = representative_types(units, &world.config);
    if types.is_empty() {
        return;
    }
    let owner_color = owner_color(owner, ownership);
    for (index, kind) in types.iter().enumerate() {
        let center =
            anchor + egui::vec2((index as f32 - (types.len() as f32 - 1.) * 0.5) * size * 0.52, 0.);
        let rect = egui::Rect::from_center_size(center, egui::Vec2::splat(size));
        let ring = center + egui::vec2(0., size * 0.30);
        painter.circle_filled(
            ring,
            size * 0.19,
            egui::Color32::from_rgba_unmultiplied(42, 31, 24, alpha / 2),
        );
        painter.circle_stroke(
            ring,
            size * 0.20,
            egui::Stroke::new(2., owner_color.gamma_multiply(f32::from(alpha) / 255.)),
        );
        let texture = texture(painter.ctx(), textures, *kind);
        let frame = ((clock * 5.) as usize + index) % 4;
        let uv = egui::Rect::from_min_max(
            egui::pos2(frame as f32 / 4., row as f32 / 4.),
            egui::pos2((frame + 1) as f32 / 4., (row + 1) as f32 / 4.),
        );
        painter.image(texture, rect, uv, egui::Color32::from_white_alpha(alpha));
        occupied.push(rect);
    }
    let banner =
        egui::Rect::from_center_size(anchor + egui::vec2(0., size * 0.42), egui::vec2(27., 12.));
    painter.rect_filled(banner, 2., owner_color);
    let caption = match owner {
        ForceOwner::Player(p) => format!("P{}", p + 1),
        ForceOwner::Local(_) => "NPC".to_owned(),
    };
    painter.text(
        banner.center(),
        egui::Align2::CENTER_CENTER,
        caption,
        egui::FontId::proportional(9.),
        egui::Color32::WHITE,
    );
    occupied.push(banner);
}

/// Decode at most one modest 768px sheet per type; source art stays full resolution.
fn texture(context: &egui::Context, cache: &mut Textures, kind: UnitType) -> egui::TextureId {
    cache.0[kind as usize]
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
