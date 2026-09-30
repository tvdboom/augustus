//! Closed combat gestures for the stable painted poses used by the sprite baker.
//!
//! The spear and sword rigs use the raised ready pose in action column zero.
//! Arms, weapons and shoulders prepare and recover together; feet stay planted.
//! Time is evaluated once per frame and consists only of periodic harmonics.

#[derive(Clone, Copy)]
enum Kind {
    Infantry,
    Mounted,
    Archer,
    HorseArcher,
    Elephant,
    Ballista,
    Catapult,
}

#[derive(Clone, Copy)]
struct Joint {
    pivot: [f32; 2],
    sin: f32,
    cos: f32,
}

impl Joint {
    fn new(pivot: [f32; 2], angle: f32) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self {
            pivot,
            sin,
            cos,
        }
    }

    /// Rigid rotation in pixel space preserves the lengths of rigid equipment.
    fn offset(self, x: f32, y: f32, width: f32, height: f32) -> [f32; 2] {
        let dx = (x - self.pivot[0]) * width;
        let dy = (y - self.pivot[1]) * height;
        [dx * (self.cos - 1.) - dy * self.sin, dx * self.sin + dy * (self.cos - 1.)]
    }
}

pub struct Combat {
    kind: Kind,
    width: f32,
    height: f32,
    retract: f32,
    stroke: f32,
    release: f32,
    sin: f32,
    joint: Joint,
    secondary: Joint,
}

impl Combat {
    pub fn new(name: &str, width: f32, height: f32, angle: f32) -> Self {
        let kind = match name {
            "light-infantry" | "heavy-infantry" => Kind::Infantry,
            "archers" => Kind::Archer,
            "horse-archers" => Kind::HorseArcher,
            "war-elephants" => Kind::Elephant,
            "ballista" => Kind::Ballista,
            "catapult" => Kind::Catapult,
            _ => Kind::Mounted,
        };
        // A little more time is spent preparing than at full extension. The
        // phase warp is itself periodic, including its first derivative.
        let sin = angle.sin();
        let action = angle + 0.24 * sin;
        let stroke = 0.5 - 0.5 * action.cos();
        let retract = 1. - stroke;
        let release = (angle * 3.).sin() * retract * retract;
        let (joint, secondary) = match kind {
            Kind::Infantry => {
                (Joint::new([0.43, 0.68], -0.035 * retract), Joint::new([0.38, 0.32], 0.055 * sin))
            },
            Kind::Mounted => {
                (Joint::new([0.45, 0.48], -0.055 * retract), Joint::new([0.72, 0.49], 0.014 * sin))
            },
            Kind::Archer | Kind::HorseArcher => {
                (Joint::new([0.43, 0.60], -0.018 * retract), Joint::new([0.79, 0.31], 0.))
            },
            Kind::Elephant => {
                (Joint::new([0.64, 0.57], -0.027 * stroke), Joint::new([0.84, 0.43], 0.14 * sin))
            },
            Kind::Ballista => {
                (Joint::new([0.20, 0.59], -0.047 * sin), Joint::new([0.27, 0.42], 0.08 * sin))
            },
            Kind::Catapult => {
                (Joint::new([0.68, 0.25], 0.11 * stroke), Joint::new([0.20, 0.61], -0.045 * sin))
            },
        };
        Self {
            kind,
            width,
            height,
            retract,
            stroke,
            release,
            sin,
            joint,
            secondary,
        }
    }

    pub fn displacement(&self, x: f32, y: f32) -> [f32; 2] {
        let primary = self.joint.offset(x, y, self.width, self.height);
        let secondary = self.secondary.offset(x, y, self.width, self.height);
        match self.kind {
            Kind::Infantry => {
                // The complete upper silhouette, including the weapon, shares
                // one rigid shoulder/hip movement. The waist absorbs it over a
                // broad interval, while the boots remain exactly planted.
                let upper = 1. - smooth(0.46, 0.83, y);
                // Follow the raised hand on the left with a modest shoulder
                // arc, preserving the painted ready silhouette throughout.
                let arm = (1. - smooth(0.30, 0.52, x)) * (1. - smooth(0.28, 0.48, y));
                let weapon = (1. - smooth(0.08, 0.23, y)).max(arm);
                [
                    upper * (primary[0] - 4.8 * self.retract) + secondary[0] * weapon,
                    upper * (primary[1] + 0.8 * self.retract) + secondary[1] * weapon,
                ]
            },
            Kind::Mounted => {
                // The raised weapon and rider recover around the saddle; the
                // animal's legs and chariot chassis remain independently fixed.
                let rider = 1. - smooth(0.29, 0.56, y);
                let head = smooth(0.61, 0.79, x) * band(0.26, 0.38, 0.53, 0.69, y);
                [
                    rider * (primary[0] - 3.6 * self.retract) + secondary[0] * head,
                    rider * (primary[1] + 0.55 * self.retract) + secondary[1] * head,
                ]
            },
            Kind::Archer | Kind::HorseArcher => {
                let mounted = matches!(self.kind, Kind::HorseArcher);
                let upper = if mounted {
                    1. - smooth(0.27, 0.47, y)
                } else {
                    1. - smooth(0.43, 0.81, y)
                };
                let hand = if mounted {
                    band(0.10, 0.17, 0.25, 0.33, y)
                } else {
                    band(0.17, 0.24, 0.34, 0.43, y)
                } * (1. - smooth(0.43, 0.64, x));
                let bow = smooth(0.65, 0.79, x)
                    * (1.
                        - smooth(
                            if mounted {
                                0.30
                            } else {
                                0.56
                            },
                            if mounted {
                                0.43
                            } else {
                                0.70
                            },
                            y,
                        ));
                // Draw hand advances on release, then returns to the cheek.
                // The bow translates as a rigid unit and settles after recoil.
                [
                    primary[0] * upper
                        + 4.2 * self.retract * hand
                        + (1.4 * self.stroke + 0.5 * self.release) * bow,
                    primary[1] * upper - 0.6 * self.stroke * bow,
                ]
            },
            Kind::Elephant => {
                let head = smooth(0.48, 0.70, x) * (1. - smooth(0.53, 0.78, y));
                let trunk = smooth(0.73, 0.87, x) * (1. - smooth(0.33, 0.49, y));
                let rider = (1. - smooth(0.24, 0.40, y)) * (1. - smooth(0.60, 0.75, x));
                [
                    primary[0] * head + secondary[0] * trunk + 1.2 * self.sin * rider,
                    primary[1] * head + secondary[1] * trunk - 0.5 * self.stroke * rider,
                ]
            },
            Kind::Ballista => {
                let crew = (1. - smooth(0.25, 0.42, x)) * (1. - smooth(0.48, 0.77, y));
                let hand = band(0.24, 0.33, 0.43, 0.56, y) * (1. - smooth(0.36, 0.52, x));
                let mechanism = smooth(0.28, 0.47, x) * (1. - smooth(0.48, 0.66, y));
                let recoil = self.stroke * self.stroke * self.stroke;
                [
                    primary[0] * crew + secondary[0] * hand - 2.8 * recoil * mechanism,
                    primary[1] * crew + secondary[1] * hand + 0.55 * recoil * mechanism,
                ]
            },
            Kind::Catapult => {
                // Only the throwing arm pivots around its axle. The broad
                // capsule around its painted diagonal excludes the carriage.
                let arm_line = 0.40 - 0.28 * x;
                let arm = (1. - smooth(0.065, 0.22, (y - arm_line).abs()))
                    * smooth(0.20, 0.33, x)
                    * (1. - smooth(0.72, 0.88, x));
                let crew = (1. - smooth(0.26, 0.42, x)) * (1. - smooth(0.49, 0.79, y));
                [primary[0] * arm + secondary[0] * crew, primary[1] * arm + secondary[1] * crew]
            },
        }
    }
}

fn smooth(from: f32, to: f32, value: f32) -> f32 {
    let t = ((value - from) / (to - from)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}

fn band(start: f32, full_start: f32, full_end: f32, end: f32, value: f32) -> f32 {
    smooth(start, full_start, value) * (1. - smooth(full_end, end, value))
}
