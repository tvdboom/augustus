//! A shared battlefield: inward-facing ranks, timed attacks, missiles and impacts.
use super::*;
use crate::game::military::{Battle, BattleSide};

const RELEASE: f32 = 0.5;
const IMPACT: f32 = 0.82;

pub(super) struct FieldAnchor {
    center: [f32; 2],
    map_size: f32,
    spacing: f32,
    composition: Vec<(ForceOwner, UnitType, bool)>,
}

struct Fighter {
    id: u64,
    owner: ForceOwner,
    kind: UnitType,
    seed: u64,
    facing: f32,
    offset: egui::Vec2,
}

fn ranged(kind: UnitType) -> bool {
    matches!(
        kind,
        UnitType::LightInfantry
            | UnitType::Archers
            | UnitType::HorseArchers
            | UnitType::Ballista
            | UnitType::Catapult
    )
}

/// Only public surviving composition is used, never a foreign deployment plan.
fn fighters(side: &BattleSide, facing: f32) -> Vec<Fighter> {
    let owners: std::collections::BTreeSet<_> = side
        .units
        .iter()
        .filter(|unit| unit.current_manpower > 0.)
        .map(|unit| unit.owner)
        .collect();
    let mut fighters = Vec::new();
    for owner in owners {
        let units: Vec<_> = side
            .units
            .iter()
            .filter(|unit| unit.owner == owner && unit.current_manpower > 0.)
            .cloned()
            .collect();
        for (index, kind) in map_representatives(&units).into_iter().enumerate() {
            let seed = units[0]
                .id
                .wrapping_mul(37)
                .wrapping_add(kind as u64 * 11)
                .wrapping_add(index as u64 * 13);
            fighters.push(Fighter {
                id: units[0].id.wrapping_mul(256).wrapping_add(kind as u64 * 16 + index as u64),
                owner,
                kind,
                seed,
                facing,
                offset: egui::vec2(
                    -facing
                        * (if ranged(kind) {
                            1.25
                        } else {
                            0.72
                        }),
                    0.,
                ),
            });
        }
    }
    let count = fighters.len();
    for (index, fighter) in fighters.iter_mut().enumerate() {
        fighter.offset.y = (index as f32 - (count as f32 - 1.) * 0.5) * 0.58;
    }
    fighters
}

fn phase(clock: f32, seed: u64) -> f32 {
    (clock / Animation::Combat.seconds()
        + (seed % frames::COUNT as u64) as f32 / frames::COUNT as f32)
        .rem_euclid(1.)
}

fn strike(clock: f32, fighter: &Fighter) -> BattleStrike {
    use UnitType::*;
    let (cue, moment) = match fighter.kind {
        Archers | HorseArchers => ("battle-archers", RELEASE),
        LightInfantry => ("battle-infantry", IMPACT),
        WarElephants => ("battle-elephants", RELEASE),
        Ballista | Catapult => ("recruit-siege", RELEASE),
        _ => ("battle-infantry", RELEASE),
    };
    BattleStrike {
        actor: fighter.id,
        cycle: (clock / Animation::Combat.seconds()
            + (fighter.seed % frames::COUNT as u64) as f32 / frames::COUNT as f32
            - moment)
            .floor() as i64,
        cue,
    }
}

fn position(field: &FieldAnchor, fighter: &Fighter, projection: &Projection) -> egui::Pos2 {
    projection.point(field.center)
        + fighter.offset * field.map_size * projection.scale * field.spacing
}

/// Place both sides as one group, so obstacle avoidance cannot split a battle.
fn field_anchor(
    fighters: &[Fighter],
    province: &Province,
    desired: egui::Pos2,
    projection: &Projection,
    size: f32,
    obstacles: &[egui::Rect],
) -> Option<FieldAnchor> {
    let map_size = size / projection.scale;
    let valid = |center: egui::Pos2, spacing: f32| {
        fighters.iter().all(|fighter| {
            let feet = center + fighter.offset * size * spacing + egui::vec2(0., size * 0.5);
            // Leave room for a short forward step and the badge beneath the feet.
            [-0.10, 0., 0.10]
                .iter()
                .all(|dx| province.contains(projection.inverse(feet + egui::vec2(dx * size, 0.))))
                && province.contains(projection.inverse(feet + egui::vec2(0., 13.)))
        })
    };
    let mut candidates = vec![desired];
    for row in 0..24 {
        for column in 0..24 {
            candidates.push(projection.point([
                province.bounds[0]
                    + (province.bounds[2] - province.bounds[0]) * (column as f32 + 0.5) / 24.,
                province.bounds[1]
                    + (province.bounds[3] - province.bounds[1]) * (row as f32 + 0.5) / 24.,
            ]));
        }
    }
    for spacing in [1., 0.85, 0.70, 0.55] {
        let cost = |center: egui::Pos2| {
            let overlap: f32 = fighters
                .iter()
                .map(|fighter| {
                    let bounds = troop_rect(
                        center + fighter.offset * size * spacing,
                        size,
                        0,
                        1,
                        fighter.kind,
                    );
                    obstacles
                        .iter()
                        .filter(|other| bounds.intersects(**other))
                        .map(|other| bounds.intersect(*other).area())
                        .sum::<f32>()
                })
                .sum();
            center.distance_sq(desired) + overlap * 8.
        };
        if let Some(center) = candidates
            .iter()
            .copied()
            .filter(|&center| valid(center, spacing))
            .min_by(|&a, &b| cost(a).total_cmp(&cost(b)))
        {
            return Some(FieldAnchor {
                center: projection.inverse(center),
                map_size,
                spacing,
                composition: composition(fighters),
            });
        }
    }
    None
}

fn composition(fighters: &[Fighter]) -> Vec<(ForceOwner, UnitType, bool)> {
    fighters.iter().map(|actor| (actor.owner, actor.kind, actor.facing > 0.)).collect()
}

fn camera_gain(zoom: f32, threshold: f32, center: egui::Pos2, viewport: egui::Rect) -> f32 {
    if !viewport.contains(center) {
        return 0.;
    }
    let offset = center - viewport.center();
    let distance =
        (offset.x / (viewport.width() * 0.5)).hypot(offset.y / (viewport.height() * 0.5));
    smoothstep((zoom - threshold) / 1.5) * smoothstep((1. - distance) / 0.8)
}

pub(super) fn paint(
    painter: &egui::Painter,
    textures: &mut Textures,
    world: &MilitaryWorld,
    ownership: &ProvinceOwnership,
    battle: &Battle,
    province: &Province,
    projection: &Projection,
    zoom: f32,
    threshold: f32,
    clock: f32,
    size: f32,
    alpha: u8,
    viewport: egui::Rect,
    landmarks: &[egui::Rect],
    label_area: Option<egui::Rect>,
    anchors: &mut std::collections::BTreeMap<u64, FieldAnchor>,
    occupied: &mut Vec<egui::Rect>,
    hits: &mut Vec<ArmyHit>,
) -> Option<AudibleBattle> {
    let mut actors = fighters(&battle.attackers, 1.);
    let split = actors.len();
    actors.extend(fighters(&battle.defenders, -1.));
    if split == 0 || split == actors.len() {
        return None;
    }
    let changed =
        anchors.get(&battle.id).is_none_or(|field| field.composition != composition(&actors));
    if changed {
        let anchor = if let Some(field) = anchors.get(&battle.id) {
            projection.point(field.center)
        } else if battle.province == atlas().provinces.len() {
            projection.point(CITIES[0].position)
        } else {
            projection.point(province.visual_center) + egui::vec2(0., -size)
        };
        let mut obstacles = landmarks.to_vec();
        obstacles.extend_from_slice(occupied);
        obstacles.extend(label_area);
        let field = field_anchor(&actors, province, anchor, projection, size, &obstacles)?;
        anchors.insert(battle.id, field);
    }
    let field = &anchors[&battle.id];
    let positions: Vec<_> =
        actors.iter().map(|fighter| position(field, fighter, projection)).collect();
    let target = |index: usize| {
        let enemies = if index < split {
            split..actors.len()
        } else {
            0..split
        };
        enemies
            .min_by(|&a, &b| {
                positions[index]
                    .distance_sq(positions[a])
                    .total_cmp(&positions[index].distance_sq(positions[b]))
            })
            .unwrap()
    };
    // Paint from back to front without changing anyone's side or attack target.
    let mut order: Vec<_> = (0..actors.len()).collect();
    order.sort_by(|&a, &b| positions[a].y.total_cmp(&positions[b].y));
    for index in order {
        let actor = &actors[index];
        let p = phase(clock, actor.seed);
        let lunge = if ranged(actor.kind) {
            0.025
        } else {
            0.10
        };
        let advance = (0.5 - 0.5 * (p * std::f32::consts::TAU).cos()) * size * lunge;
        let anchor = positions[index] + egui::vec2(actor.facing * advance, 0.);
        let rect = troop_rect(anchor, size, 0, 1, actor.kind);
        // Marches can target a fighter outside the current camera viewport.
        hits.push(ArmyHit {
            rect,
            province: battle.province,
            owner: actor.owner,
            movement: None,
        });
        if !rect.intersects(viewport) {
            continue;
        }
        let texture = motion_texture(painter.ctx(), textures, actor.kind, Animation::Combat);
        let mut uv = animation_uv(animation_frame(clock, actor.seed, Animation::Combat));
        if actor.facing < 0. {
            std::mem::swap(&mut uv.min.x, &mut uv.max.x);
        }
        painter.image(texture, rect, uv, egui::Color32::from_white_alpha(alpha));
        occupied.push(rect);
    }
    for (index, actor) in actors.iter().enumerate() {
        let enemy = target(index);
        attack_effect(painter, actor, positions[index], positions[enemy], size, clock, alpha);
    }
    for side in [&actors[..split], &actors[split..]] {
        let owners: std::collections::BTreeSet<_> = side.iter().map(|actor| actor.owner).collect();
        for owner in owners {
            let group: Vec<_> = actors
                .iter()
                .enumerate()
                .filter(|(_, actor)| actor.owner == owner && actor.facing == side[0].facing)
                .map(|(index, _)| positions[index])
                .collect();
            let x = group.iter().map(|point| point.x).sum::<f32>() / group.len() as f32;
            let y = group.iter().map(|point| point.y).fold(f32::NEG_INFINITY, f32::max);
            draw_banner(
                painter,
                world,
                owner,
                owner_color(owner, world, ownership),
                egui::pos2(x, y),
                size,
                alpha,
                occupied,
                hits,
                battle.province,
                None,
            );
        }
    }
    let center = projection.point(field.center);
    let gain = camera_gain(zoom, threshold, center, viewport);
    (gain > 0.001).then(|| AudibleBattle {
        battle: battle.id,
        gain,
        strikes: actors.iter().map(|actor| strike(clock, actor)).collect(),
    })
}

fn projectile(
    start: egui::Pos2,
    end: egui::Pos2,
    progress: f32,
    arc: f32,
) -> (egui::Pos2, egui::Vec2) {
    let point = start.lerp(end, progress) - egui::vec2(0., 4. * arc * progress * (1. - progress));
    let tangent = end - start - egui::vec2(0., 4. * arc * (1. - 2. * progress));
    (point, tangent.normalized())
}

fn attack_effect(
    painter: &egui::Painter,
    actor: &Fighter,
    anchor: egui::Pos2,
    enemy: egui::Pos2,
    size: f32,
    clock: f32,
    alpha: u8,
) {
    let p = phase(clock, actor.seed);
    let (forward, height) = match actor.kind {
        UnitType::LightInfantry => (0.25, 1.35),
        UnitType::Archers => (0.52, 0.80),
        UnitType::HorseArchers => (0.45, 1.25),
        UnitType::Ballista => (0.65, 0.35),
        UnitType::Catapult => (0.45, 0.65),
        _ => (0.42, 0.60),
    };
    let start = anchor + egui::vec2(actor.facing * size * forward, -size * height);
    let end = enemy + egui::vec2(-actor.facing * size * 0.10, -size * 0.40);
    if ranged(actor.kind) && (RELEASE..IMPACT).contains(&p) {
        let progress = (p - RELEASE) / (IMPACT - RELEASE);
        let arc = size
            * if actor.kind == UnitType::Catapult {
                1.
            } else {
                0.4
            };
        let (tip, direction) = projectile(start, end, progress, arc);
        if actor.kind == UnitType::Catapult {
            painter.circle_filled(
                tip,
                (size * 0.09).max(1.6),
                egui::Color32::from_rgba_unmultiplied(82, 75, 65, alpha),
            );
        } else {
            let spear = actor.kind == UnitType::LightInfantry;
            let length = size
                * if spear {
                    0.65
                } else {
                    0.36
                };
            let shaft = egui::Color32::from_rgba_unmultiplied(92, 55, 29, alpha);
            let steel = egui::Color32::from_rgba_unmultiplied(231, 225, 200, alpha);
            let tail = tip - direction * length;
            painter.line_segment([tail, tip], egui::Stroke::new((size * 0.025).max(1.), shaft));
            let normal = egui::vec2(-direction.y, direction.x);
            let head = (size * 0.065).max(2.);
            painter.add(egui::Shape::convex_polygon(
                vec![
                    tip + direction * head,
                    tip - direction * head + normal * head * 0.45,
                    tip - direction * head - normal * head * 0.45,
                ],
                steel,
                egui::Stroke::NONE,
            ));
            if !spear {
                for sign in [-1., 1.] {
                    painter.line_segment(
                        [tail + direction * head * 1.5, tail + normal * sign * head * 0.65],
                        egui::Stroke::new(1., steel),
                    );
                }
            }
        }
    }
    let hit = if ranged(actor.kind) {
        IMPACT
    } else {
        RELEASE
    };
    if (hit..hit + 0.14).contains(&p) {
        let progress = (p - hit) / 0.14;
        let point = if ranged(actor.kind) {
            end
        } else {
            anchor.lerp(enemy, 0.5) + egui::vec2(0., -size * 0.4)
        };
        let fade = (1. - progress) * f32::from(alpha) / 255.;
        let spark = egui::Color32::from_rgba_unmultiplied(244, 216, 151, (210. * fade) as u8);
        for ray in 0..5 {
            let angle = ray as f32 * std::f32::consts::TAU / 5. + actor.seed as f32;
            let direction = egui::vec2(angle.cos(), angle.sin());
            painter.line_segment(
                [
                    point + direction * size * progress * 0.09,
                    point + direction * size * (0.08 + progress * 0.18),
                ],
                egui::Stroke::new(1.2, spark),
            );
        }
        let ground = enemy + egui::vec2(0., size * 0.5);
        for particle in 0..3 {
            let dx = (particle as f32 - 1.) * size * (0.05 + progress * 0.17);
            painter.circle_filled(
                ground + egui::vec2(dx, -progress * size * 0.17),
                size * (0.03 + progress * 0.07),
                egui::Color32::from_rgba_unmultiplied(146, 123, 87, (75. * fade) as u8),
            );
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/map_combat.rs"]
mod tests;
