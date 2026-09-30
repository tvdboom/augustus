//! Small articulated walking rigs for the military movement poses.
//! Bone transforms preserve painted limb shape; only the hip and knee joints
//! blend. Feet follow a closed swing, and rigid upper bodies keep gear intact.

use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy)]
struct Leg {
    hip: [f32; 2],
    knee: [f32; 2],
    ankle: [f32; 2],
    thigh_rotation: [f32; 2],
    shin_rotation: [f32; 2],
    knee_offset: [f32; 2],
}

#[derive(Clone, Copy)]
struct Wheel {
    center: [f32; 2],
    radius: [f32; 2],
    rotation: [f32; 2],
}

pub struct Rig {
    legs: Vec<Leg>,
    wheels: Vec<Wheel>,
    size: [f32; 2],
    body_offset: [f32; 2],
    carriage: bool,
}

impl Rig {
    pub fn new(name: &str, width: f32, height: f32, theta: f32) -> Self {
        let mut rig = Self {
            legs: Vec::new(),
            wheels: Vec::new(),
            size: [width, height],
            body_offset: [0.45 * theta.sin(), -1.05 * (theta * 2.).cos()],
            carriage: matches!(name, "ballista" | "catapult" | "war-chariots"),
        };
        // The coordinates belong to the cropped first movement pose (row two).
        // Back/far legs have the opposite phase from the front/near pair.
        match name {
            "light-infantry" => {
                rig.leg([0.40, 0.63], [0.21, 0.79], [0.10, 0.94], theta, 0.36);
                rig.leg([0.54, 0.63], [0.57, 0.82], [0.65, 0.97], theta + PI, 0.36);
            },
            "heavy-infantry" => {
                rig.leg([0.40, 0.70], [0.20, 0.84], [0.12, 0.95], theta, 0.36);
                rig.leg([0.64, 0.71], [0.70, 0.85], [0.82, 0.97], theta + PI, 0.36);
            },
            "archers" => {
                rig.leg([0.39, 0.65], [0.19, 0.80], [0.13, 0.94], theta, 0.36);
                rig.leg([0.64, 0.65], [0.65, 0.83], [0.77, 0.97], theta + PI, 0.36);
            },
            "light-cavalry" | "heavy-cavalry" | "horse-archers" => {
                rig.leg([0.21, 0.69], [0.18, 0.85], [0.20, 0.97], theta, 0.29);
                rig.leg([0.33, 0.70], [0.30, 0.85], [0.36, 0.95], theta + PI, 0.24);
                rig.leg([0.61, 0.70], [0.60, 0.85], [0.62, 0.97], theta + PI, 0.29);
                rig.leg([0.72, 0.69], [0.81, 0.80], [0.75, 0.92], theta, 0.24);
            },
            "war-camels" => {
                rig.leg([0.23, 0.60], [0.12, 0.80], [0.10, 0.95], theta, 0.26);
                rig.leg([0.36, 0.65], [0.34, 0.82], [0.42, 0.94], theta + PI, 0.23);
                rig.leg([0.56, 0.63], [0.54, 0.81], [0.56, 0.94], theta + PI, 0.23);
                rig.leg([0.62, 0.62], [0.72, 0.82], [0.81, 0.98], theta, 0.26);
            },
            "war-elephants" => {
                rig.body_offset = [0.25 * theta.sin(), -0.65 * (theta * 2.).cos()];
                rig.leg([0.23, 0.70], [0.13, 0.83], [0.17, 0.94], theta, 0.25);
                rig.leg([0.38, 0.71], [0.37, 0.84], [0.42, 0.95], theta + PI, 0.20);
                rig.leg([0.55, 0.72], [0.53, 0.85], [0.55, 0.98], theta + PI, 0.25);
                rig.leg([0.70, 0.72], [0.76, 0.81], [0.80, 0.87], theta, 0.20);
            },
            "war-chariots" => {
                rig.leg([0.43, 0.69], [0.41, 0.83], [0.44, 0.95], theta, 0.26);
                rig.leg([0.56, 0.69], [0.55, 0.83], [0.58, 0.93], theta + PI, 0.23);
                rig.leg([0.67, 0.70], [0.72, 0.84], [0.76, 0.97], theta + PI, 0.26);
                rig.leg([0.80, 0.66], [0.88, 0.80], [0.87, 0.94], theta, 0.23);
                rig.wheel([0.13, 0.70], [0.095, 0.15], theta);
            },
            "ballista" | "catapult" => {
                rig.body_offset = [0., -0.40 * (theta * 2.).cos()];
                rig.leg([0.13, 0.61], [0.07, 0.78], [0.05, 0.94], theta, 0.24);
                if name == "ballista" {
                    rig.wheel([0.23, 0.78], [0.07, 0.12], theta);
                    rig.wheel([0.59, 0.88], [0.075, 0.115], theta);
                } else {
                    rig.wheel([0.18, 0.81], [0.065, 0.10], theta);
                    rig.wheel([0.59, 0.91], [0.07, 0.09], theta);
                    rig.wheel([0.94, 0.78], [0.055, 0.10], theta);
                }
            },
            _ => unreachable!("unknown military walking rig"),
        }
        rig
    }

    fn leg(&mut self, hip: [f32; 2], knee: [f32; 2], ankle: [f32; 2], phase: f32, swing: f32) {
        let hip = self.to_pixels(hip);
        let knee = self.to_pixels(knee);
        let ankle = self.to_pixels(ankle);
        let thigh_angle = swing * phase.sin();
        // More flex on the airborne half; the derivative is zero where a foot
        // leaves/returns to the ground. The knee never snaps at a phase edge.
        let airborne = phase.cos().max(0.).powi(2);
        let shin_angle = thigh_angle + 0.32 * airborne - 0.10;
        let thigh_rotation = rotation(thigh_angle);
        let shin_rotation = rotation(shin_angle);
        let rotated_knee = rotate(subtract(knee, hip), thigh_rotation);
        let knee_offset = subtract(add(hip, rotated_knee), knee);
        self.legs.push(Leg {
            hip,
            knee,
            ankle,
            thigh_rotation,
            shin_rotation,
            knee_offset,
        });
    }

    fn wheel(&mut self, center: [f32; 2], radius: [f32; 2], theta: f32) {
        self.wheels.push(Wheel {
            center: self.to_pixels(center),
            radius: self.to_pixels(radius),
            rotation: rotation(theta.rem_euclid(TAU)),
        });
    }

    fn to_pixels(&self, point: [f32; 2]) -> [f32; 2] {
        [point[0] * self.size[0], point[1] * self.size[1]]
    }

    fn velocity(&self, x: f32, y: f32) -> [f32; 2] {
        let point = self.to_pixels([x, y]);
        let mut displacement = self.body_offset;
        let first_hip = self.legs.iter().map(|leg| leg.hip[1]).fold(f32::INFINITY, f32::min);
        if point[1] < first_hip - 4. {
            return displacement;
        }
        let mut total_weight = 0.;
        let mut joint = [0.; 2];
        for leg in &self.legs {
            let distance = distance_to_segment(point, leg.hip, leg.knee)
                .min(distance_to_segment(point, leg.knee, leg.ankle));
            let weight = 1. / (256. + distance).powi(2);
            let upper =
                subtract(add(leg.hip, rotate(subtract(point, leg.hip), leg.thigh_rotation)), point);
            let lower = subtract(
                add(
                    add(leg.knee, leg.knee_offset),
                    rotate(subtract(point, leg.knee), leg.shin_rotation),
                ),
                point,
            );
            let knee = smooth(leg.knee[1] - 8., leg.knee[1] + 9., point[1]);
            let hip = smooth(leg.hip[1] - 4., leg.hip[1] + 18., point[1]);
            let influence = if self.carriage {
                // Localized legs cannot drag the nearby vehicle body with them.
                1. - smooth(14. * 14., 36. * 36., distance)
            } else {
                1. - smooth(20. * 20., 42. * 42., distance)
            };
            for axis in 0..2 {
                joint[axis] +=
                    (upper[axis] * (1. - knee) + lower[axis] * knee) * hip * weight * influence;
            }
            total_weight += weight;
        }
        if total_weight > 0. {
            displacement[0] += joint[0] / total_weight;
            displacement[1] += joint[1] / total_weight;
        }
        displacement
    }

    /// Integrating a smooth velocity field gives an invertible skin map even
    /// between overlapping near/far legs. Directly averaging large transforms
    /// there can fold the image. Small midpoint steps preserve orientation and
    /// coherent silhouettes throughout the full gait.
    fn flow(&self, x: f32, y: f32, direction: f32) -> [f32; 2] {
        const STEPS: usize = 12;
        let step = direction / STEPS as f32;
        let mut point = self.to_pixels([x, y]);
        for _ in 0..STEPS {
            let velocity = self.velocity(point[0] / self.size[0], point[1] / self.size[1]);
            let midpoint =
                [point[0] + velocity[0] * step * 0.5, point[1] + velocity[1] * step * 0.5];
            let velocity = self.velocity(midpoint[0] / self.size[0], midpoint[1] / self.size[1]);
            point[0] += velocity[0] * step;
            point[1] += velocity[1] * step;
        }
        point
    }

    /// The baker samples through the backward flow directly, so it does not
    /// need an iterative inverse that can jump between adjacent moving limbs.
    pub fn source_point(&self, x: f32, y: f32) -> [f32; 2] {
        if let Some(point) = self.wheel_source(x, y) {
            return point;
        }
        let point = self.flow(x, y, -1.);
        [point[0] / self.size[0], point[1] / self.size[1]]
    }

    #[allow(dead_code)]
    pub fn displacement(&self, x: f32, y: f32) -> [f32; 2] {
        let initial = self.to_pixels([x, y]);
        subtract(self.flow(x, y, 1.), initial)
    }

    /// A wheel uses an exact inverse rigid rotation, rather than iterating a
    /// deformation field through a half-turn. Preserve its elliptical outline
    /// and leave the metal rim outside the rotating inset untouched.
    pub fn wheel_source(&self, x: f32, y: f32) -> Option<[f32; 2]> {
        let point = subtract(self.to_pixels([x, y]), self.body_offset);
        for wheel in &self.wheels {
            let radial = [
                (point[0] - wheel.center[0]) / wheel.radius[0],
                (point[1] - wheel.center[1]) / wheel.radius[1],
            ];
            if radial[0] * radial[0] + radial[1] * radial[1] < 0.85 * 0.85 {
                let rotated = rotate(radial, [wheel.rotation[0], -wheel.rotation[1]]);
                return Some([
                    (wheel.center[0] + rotated[0] * wheel.radius[0]) / self.size[0],
                    (wheel.center[1] + rotated[1] * wheel.radius[1]) / self.size[1],
                ]);
            }
        }
        None
    }
}

fn rotation(angle: f32) -> [f32; 2] {
    let (sin, cos) = angle.sin_cos();
    [cos, sin]
}
fn rotate(point: [f32; 2], rotation: [f32; 2]) -> [f32; 2] {
    [
        point[0] * rotation[0] - point[1] * rotation[1],
        point[0] * rotation[1] + point[1] * rotation[0],
    ]
}
fn add(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] + b[0], a[1] + b[1]]
}
fn subtract(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}
fn smooth(from: f32, to: f32, value: f32) -> f32 {
    let t = ((value - from) / (to - from)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}
fn distance_to_segment(point: [f32; 2], start: [f32; 2], end: [f32; 2]) -> f32 {
    let edge = subtract(end, start);
    let relative = subtract(point, start);
    let fraction = ((relative[0] * edge[0] + relative[1] * edge[1])
        / (edge[0] * edge[0] + edge[1] * edge[1]))
        .clamp(0., 1.);
    let dx = relative[0] - edge[0] * fraction;
    let dy = relative[1] - edge[1] * fraction;
    dx * dx + dy * dy
}
