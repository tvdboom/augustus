//! Coast-aware march geometry, independent of the campaign's travel clock.
use super::*;

struct Triangle {
    province: usize,
    vertices: [egui::Pos2; 3],
    bounds: egui::Rect,
    center: egui::Pos2,
    neighbors: Vec<(usize, egui::Pos2)>,
}

struct Navigation {
    triangles: Vec<Triangle>,
}

fn point([x, y]: [f32; 2]) -> egui::Pos2 {
    egui::pos2(x * LONGITUDE_SCALE, y)
}

fn geographic(p: egui::Pos2) -> [f32; 2] {
    [p.x / LONGITUDE_SCALE, p.y]
}

fn cross(a: egui::Vec2, b: egui::Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}

/// Clip a segment against a convex land triangle. The covered parameter interval
/// lets shortcuts be checked continuously, including narrow bays and river mouths.
fn interval(triangle: &Triangle, a: egui::Pos2, b: egui::Pos2) -> Option<(f32, f32)> {
    if !triangle.bounds.intersects(egui::Rect::from_two_pos(a, b)) {
        return None;
    }
    let v = triangle.vertices;
    let orientation = cross(v[1] - v[0], v[2] - v[0]).signum();
    let (mut lower, mut upper) = (0_f32, 1_f32);
    for i in 0..3 {
        let edge = v[(i + 1) % 3] - v[i];
        // A geographic → projected → geographic f32 round trip can move a
        // border point by a fraction of a meter. Keep clipping stable there.
        let start = orientation * cross(edge, a - v[i]) + edge.length() * 0.000004;
        let slope = orientation * cross(edge, b - a);
        if slope.abs() < 1e-10 {
            if start < -1e-7 {
                return None;
            }
        } else if slope > 0. {
            lower = lower.max(-start / slope);
        } else {
            upper = upper.min(-start / slope);
        }
    }
    (lower <= upper + 1e-6).then_some((lower, upper))
}

impl Navigation {
    fn new(provinces: &[Province]) -> Self {
        let mut triangles: Vec<Triangle> = Vec::new();
        let mut edges = HashMap::new();
        for (province, shape) in provinces.iter().enumerate() {
            for part in &shape.parts {
                for indices in part.t.chunks_exact(3) {
                    let raw = [indices[0], indices[1], indices[2]].map(|i| part.v[i as usize]);
                    let vertices = raw.map(point);
                    if cross(vertices[1] - vertices[0], vertices[2] - vertices[0]).abs() < 1e-10 {
                        continue;
                    }
                    let index = triangles.len();
                    let mut triangle = Triangle {
                        province,
                        vertices,
                        bounds: egui::Rect::from_points(&vertices).expand(0.000004),
                        center: vertices[0]
                            + ((vertices[1] - vertices[0]) + (vertices[2] - vertices[0])) / 3.,
                        neighbors: Vec::new(),
                    };
                    for i in 0..3 {
                        let a = raw[i].map(f32::to_bits);
                        let b = raw[(i + 1) % 3].map(f32::to_bits);
                        let key = if a < b {
                            (a, b)
                        } else {
                            (b, a)
                        };
                        if let Some(&other) = edges.get(&key) {
                            let portal = vertices[i].lerp(vertices[(i + 1) % 3], 0.5);
                            triangle.neighbors.push((other, portal));
                            triangles[other].neighbors.push((index, portal));
                        } else {
                            edges.insert(key, index);
                        }
                    }
                    triangles.push(triangle);
                }
            }
        }
        Self {
            triangles,
        }
    }

    fn nearest(&self, p: egui::Pos2, allowed: impl Fn(usize) -> bool) -> (usize, egui::Pos2) {
        self.triangles
            .iter()
            .enumerate()
            .filter(|(id, _)| allowed(*id))
            .map(|(id, t)| {
                let closest = if interval(t, p, p).is_some() {
                    p
                } else {
                    (0..3)
                        .map(|i| {
                            let a = t.vertices[i];
                            let delta = t.vertices[(i + 1) % 3] - a;
                            a + delta * ((p - a).dot(delta) / delta.length_sq()).clamp(0., 1.)
                        })
                        .min_by(|a, b| a.distance_sq(p).total_cmp(&b.distance_sq(p)))
                        .unwrap()
                };
                (id, closest)
            })
            .min_by(|a, b| a.1.distance_sq(p).total_cmp(&b.1.distance_sq(p)))
            .unwrap()
    }

    fn visible(&self, a: egui::Pos2, b: egui::Pos2, corridor: &[usize]) -> bool {
        let mut intervals: Vec<_> =
            corridor.iter().filter_map(|&id| interval(&self.triangles[id], a, b)).collect();
        intervals.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut end = 0_f32;
        for (lower, upper) in intervals {
            if lower > end + 1e-5 {
                return false;
            }
            end = end.max(upper);
        }
        end >= 1. - 1e-5
    }

    fn contains(&self, p: egui::Pos2, corridor: &[usize]) -> bool {
        corridor.iter().any(|&id| interval(&self.triangles[id], p, p).is_some())
    }

    /// Prefer one broad arc with coastal clearance over a chain of mesh portals.
    /// The control stays between the ends along the journey, preventing reversals
    /// toward the destination. Validate every segment against the full legal land.
    fn curved_connection(
        &self,
        a: egui::Pos2,
        b: egui::Pos2,
        corridor: &[usize],
    ) -> Option<Vec<egui::Pos2>> {
        let delta = b - a;
        let span = delta.length();
        if span < 0.00001 {
            return Some(vec![a, b]);
        }
        let midpoint = a.lerp(b, 0.5);
        let normal = egui::vec2(-delta.y, delta.x) / span;
        let (weighted, area) = corridor.iter().fold((egui::Vec2::ZERO, 0.), |(sum, area), &id| {
            let t = &self.triangles[id];
            let weight = cross(t.vertices[1] - t.vertices[0], t.vertices[2] - t.vertices[0]).abs();
            (sum + t.center.to_vec2() * weight, area + weight)
        });
        let inland = weighted / area.max(f32::EPSILON) - midpoint.to_vec2();
        let preferred = if inland.dot(normal) >= 0. {
            1.
        } else {
            -1.
        };
        let margin = (span * 0.04).clamp(0.04, 0.25);
        // Narrow terrain can require less clearance; the land constraint always applies.
        for clearance in [true, false] {
            for bend in [0.12, 0.20, 0.30, 0.42, 0.56, 0.74, 0.96, 1.24] {
                // Bays are often asymmetric. Move the deepest part of the bend
                // along the journey instead of adding another corner near shore.
                for along in [0.5, 0.65, 0.35, 0.8, 0.2, 0.9, 0.1] {
                    for sign in [preferred, -preferred] {
                        let control = a.lerp(b, along) + normal * (span * bend * sign);
                        let curve: Vec<_> = (0..=64)
                            .map(|step| {
                                let t = step as f32 / 64.;
                                a.lerp(control, t).lerp(control.lerp(b, t), t)
                            })
                            .collect();
                        if clearance
                            && (4..64).step_by(4).any(|step| {
                                let t = step as f32 / 64.;
                                let tangent = (control - a) * (1. - t) + (b - control) * t;
                                let side = egui::vec2(-tangent.y, tangent.x).normalized();
                                // Army anchors can be close to shore. Ease the
                                // clearance in and out instead of forcing a turn
                                // immediately before reaching the destination.
                                let width = margin * (std::f32::consts::PI * t).sin().powi(2);
                                !self.contains(curve[step] + side * width, corridor)
                                    || !self.contains(curve[step] - side * width, corridor)
                            })
                        {
                            continue;
                        }
                        if curve.windows(2).all(|edge| self.visible(edge[0], edge[1], corridor)) {
                            return Some(curve);
                        }
                    }
                }
            }
        }
        None
    }

    fn land_path(
        &self,
        a: egui::Pos2,
        b: egui::Pos2,
        from: usize,
        to: usize,
    ) -> Option<Vec<egui::Pos2>> {
        let (start, a) = self.nearest(a, |id| self.triangles[id].province == from);
        let (finish, b) = self.nearest(b, |id| self.triangles[id].province == to);
        let corridor: Vec<_> = self
            .triangles
            .iter()
            .enumerate()
            .filter_map(|(id, t)| (t.province == from || t.province == to).then_some(id))
            .collect();
        if let Some(curve) = self.curved_connection(a, b, &corridor) {
            return Some(curve);
        }
        let mut distances = vec![f32::INFINITY; self.triangles.len()];
        let mut previous = vec![None; self.triangles.len()];
        let mut open = vec![start];
        distances[start] = 0.;
        while !open.is_empty() {
            let best = (0..open.len())
                .min_by(|&a, &b| {
                    let cost = |i: usize| {
                        distances[open[i]]
                            + self.triangles[open[i]].center.distance(self.triangles[finish].center)
                    };
                    cost(a).total_cmp(&cost(b))
                })
                .unwrap();
            let current = open.swap_remove(best);
            if current == finish {
                break;
            }
            for &(next, portal) in &self.triangles[current].neighbors {
                let province = self.triangles[next].province;
                if province != from && province != to {
                    continue;
                }
                let candidate = distances[current]
                    + self.triangles[current].center.distance(portal)
                    + portal.distance(self.triangles[next].center);
                if candidate < distances[next] {
                    distances[next] = candidate;
                    previous[next] = Some((current, portal));
                    if !open.contains(&next) {
                        open.push(next);
                    }
                }
            }
        }
        if !distances[finish].is_finite() {
            return None;
        }
        let mut reverse = vec![b, self.triangles[finish].center];
        let mut current = finish;
        while current != start {
            let (parent, portal) = previous[current]?;
            reverse.extend([portal, self.triangles[parent].center]);
            current = parent;
        }
        reverse.push(a);
        reverse.reverse();
        let mut path = vec![a];
        let mut index = 0;
        while index + 1 < reverse.len() {
            let next = (index + 1..reverse.len())
                .rev()
                .find(|&next| self.visible(reverse[index], reverse[next], &corridor))
                .unwrap_or(index + 1);
            path.push(reverse[next]);
            index = next;
        }
        // Round bends only where every new segment remains in the land corridor.
        let mut rounded = vec![a];
        for corner in path.windows(3) {
            let entry = corner[1].lerp(corner[0], 0.45);
            let exit = corner[1].lerp(corner[2], 0.45);
            let mut curve = vec![entry];
            for step in 1..=8 {
                let t = step as f32 / 8.;
                curve.push(entry.lerp(corner[1], t).lerp(corner[1].lerp(exit, t), t));
            }
            if self.visible(*rounded.last().unwrap(), entry, &corridor)
                && curve.windows(2).all(|edge| self.visible(edge[0], edge[1], &corridor))
                && self.visible(exit, corner[2], &corridor)
            {
                rounded.extend(curve);
            } else {
                rounded.push(corner[1]);
            }
        }
        rounded.push(b);
        Some(rounded)
    }

    /// Shore anchors attach to the army's connected landmass, even when a named
    /// province also includes offshore islands. Only the marked crossing goes to sea.
    fn shore(&self, anchor: egui::Pos2, shore: [f32; 2], province: usize) -> egui::Pos2 {
        let (start, _) = self.nearest(anchor, |id| self.triangles[id].province == province);
        let mut connected = std::collections::BTreeSet::from([start]);
        let mut pending = vec![start];
        while let Some(id) = pending.pop() {
            for &(next, _) in &self.triangles[id].neighbors {
                if self.triangles[next].province == province && connected.insert(next) {
                    pending.push(next);
                }
            }
        }
        self.nearest(point(shore), |id| connected.contains(&id)).1
    }
}

/// Each visual leg is constrained to its two campaign provinces. A straight
/// shortcut may never skip a province or cut across an unmarked stretch of sea.
pub(super) fn march_leg(from: usize, to: usize, a: [f32; 2], b: [f32; 2]) -> Option<Vec<[f32; 2]>> {
    static NAVIGATION: OnceLock<Navigation> = OnceLock::new();
    let atlas = atlas();
    if from > atlas.provinces.len()
        || to > atlas.provinces.len()
        || (from == atlas.provinces.len() && to == from)
    {
        return None;
    }
    let nav = NAVIGATION.get_or_init(|| Navigation::new(&atlas.provinces));
    // Rome is a city on the three-province junction, rather than a map polygon.
    let (from, to) = (
        if from == atlas.provinces.len() {
            to
        } else {
            from
        },
        if to == atlas.provinces.len() {
            from
        } else {
            to
        },
    );
    let (start, finish) = (point(a), point(b));
    let mut path = nav.land_path(start, finish, from, to).or_else(|| {
        let names = [&atlas.provinces[from].name, &atlas.provinces[to].name];
        let crossing = SEA_CROSSINGS.iter().find(|c| {
            c.provinces == [names[0].as_str(), names[1].as_str()]
                || c.provinces == [names[1].as_str(), names[0].as_str()]
        })?;
        let shores = if crossing.provinces[0] == *names[0] {
            crossing.shores
        } else {
            [crossing.shores[1], crossing.shores[0]]
        };
        let departure = nav.shore(start, shores[0], from);
        let arrival = nav.shore(finish, shores[1], to);
        let mut path = nav.land_path(start, departure, from, from)?;
        path.extend(nav.land_path(arrival, finish, to, to)?);
        Some(path)
    })?;
    // Preserve the exact visible departure/target; sprite centers can sit just
    // beyond a coastal polygon while their planted feet remain on land.
    path.insert(0, start);
    path.push(finish);
    path.dedup_by(|a, b| a.distance_sq(*b) < 1e-12);
    Some(path.into_iter().map(geographic).collect())
}

#[cfg(test)]
#[path = "../../tests/unit/map_routes.rs"]
mod tests;
