//! Vehicle-specific sampling in the original painted cell's coordinates.
//!
//! Rims, hubs, axles and timber are rigid. Only the inset spokes turn; walking
//! skinning is confined to exposed legs. Idle/loading gestures belong to the
//! crew, rather than deforming the machinery along with their bodies.

#[derive(Clone, Copy)]
pub enum Motion {
    Idle,
    Movement,
    Combat,
}

#[derive(Clone, Copy)]
struct Wheel {
    center: [f32; 2],
    radius: [f32; 2],
    spokes: bool,
}

impl Wheel {
    const fn new(x: f32, y: f32, rx: f32, ry: f32, spokes: bool) -> Self {
        Self {
            center: [x, y],
            radius: [rx, ry],
            spokes,
        }
    }
}

pub struct Vehicle {
    motion: Motion,
    chariot: bool,
    origin: [f32; 2],
    scale: [f32; 2],
    wheels: [Wheel; 3],
    hip: [f32; 2],
    /// Right edge of exposed crew, from crown to hip. Excludes windlasses,
    /// ropes and timber that overlap the operator in the source painting.
    crew_edge: [[f32; 2]; 5],
}

impl Vehicle {
    pub fn new(name: &str, motion: Motion, origin: [f32; 2], scale: [f32; 2]) -> Option<Self> {
        use Motion::*;
        let hidden = Wheel::new(-100., -100., 1., 1., false);
        let (wheels, hip, crew_edge) = match (name, motion) {
            ("ballista", Idle) => (
                [
                    Wheel::new(113., 267., 35., 44., false),
                    Wheel::new(224., 280., 33., 36., false),
                    Wheel::new(282., 262., 28., 35., false),
                ],
                [69., 183.],
                [[145., 40.], [126., 90.], [112., 130.], [76., 165.], [61., 190.]],
            ),
            ("ballista", Movement) => (
                [
                    Wheel::new(69., 225., 26., 34., true),
                    Wheel::new(174., 244., 29., 35., true),
                    Wheel::new(280., 227., 26., 35., false),
                ],
                [65., 170.],
                [[138., 55.], [125., 85.], [106., 115.], [90., 145.], [57., 180.]],
            ),
            ("ballista", Combat) => (
                [
                    Wheel::new(97., 229., 37., 46., false),
                    Wheel::new(222., 249., 36., 37., false),
                    Wheel::new(286., 224., 26., 33., false),
                ],
                [66., 170.],
                [[145., 24.], [137., 75.], [109., 110.], [87., 143.], [61., 178.]],
            ),
            ("catapult", Idle) => (
                [
                    Wheel::new(88., 264., 35., 34., false),
                    Wheel::new(204., 279., 34., 32., false),
                    Wheel::new(305., 258., 28., 32., false),
                ],
                [78., 181.],
                [[127., 60.], [123., 95.], [116., 126.], [99., 153.], [83., 184.]],
            ),
            ("catapult", Movement) => (
                [
                    Wheel::new(77., 215., 26., 34., true),
                    Wheel::new(180., 234., 26., 32., true),
                    Wheel::new(292., 205., 25., 29., false),
                ],
                [71., 160.],
                [[129., 35.], [123., 78.], [138., 109.], [105., 135.], [64., 176.]],
            ),
            ("catapult", Combat) => (
                [
                    Wheel::new(81., 226., 40., 45., true),
                    Wheel::new(224., 239., 41., 39., true),
                    Wheel::new(289., 214., 25., 29., false),
                ],
                [52., 184.],
                [[105., 75.], [99., 108.], [108., 139.], [75., 165.], [51., 193.]],
            ),
            ("war-chariots", Idle) => (
                [Wheel::new(61., 251., 37., 51., true), hidden, hidden],
                [117., 152.],
                [[160., 0.], [160., 60.], [159., 100.], [156., 130.], [131., 155.]],
            ),
            ("war-chariots", Movement) => (
                [Wheel::new(49., 220., 30., 42., true), hidden, hidden],
                [113., 150.],
                [[170., 0.], [169., 80.], [169., 112.], [165., 134.], [139., 157.]],
            ),
            ("war-chariots", Combat) => (
                [Wheel::new(61., 227., 38., 52., true), hidden, hidden],
                [117., 147.],
                [[172., 0.], [169., 60.], [162., 100.], [157., 124.], [131., 152.]],
            ),
            _ => return None,
        };
        Some(Self {
            motion,
            chariot: name == "war-chariots",
            origin,
            scale,
            wheels,
            hip,
            crew_edge,
        })
    }

    pub fn animate(&self, angle: f32) -> Rig<'_> {
        let (sin, cos) = angle.sin_cos();
        Rig {
            vehicle: self,
            sin,
            cos,
            sin2: (angle * 2.).sin(),
        }
    }
}

pub struct Rig<'a> {
    vehicle: &'a Vehicle,
    sin: f32,
    cos: f32,
    sin2: f32,
}

impl Rig<'_> {
    /// None delegates only exposed walking legs to the articulated gait rig.
    pub fn source_point(&self, destination: [f32; 2]) -> Option<[f32; 2]> {
        let v = self.vehicle;
        let p = [
            (destination[0] - v.origin[0]) / v.scale[0],
            (destination[1] - v.origin[1]) / v.scale[1],
        ];
        if let Some(source) = self.wheel(p) {
            return Some(if source == p {
                destination
            } else {
                self.to_tile(source)
            });
        }
        if matches!(v.motion, Motion::Movement) {
            // The chariot and harness end before the exposed horse legs. The
            // siege operator's walking leg is to the left of the carriage.
            let leg = if v.chariot {
                p[0] > 139. && p[1] > 191.
            } else {
                p[0] < 62. && p[1] > v.hip[1]
            };
            return (!leg).then_some(destination);
        }

        // A small lean around the waist keeps head/body proportions intact.
        // Blend only at the hip and the boundary with overlapping equipment.
        let mut edge = v.crew_edge[0][0];
        for pair in v.crew_edge.windows(2) {
            if p[1] >= pair[0][1] {
                let t = ((p[1] - pair[0][1]) / (pair[1][1] - pair[0][1])).clamp(0., 1.);
                edge = pair[0][0] + t * (pair[1][0] - pair[0][0]);
            }
        }
        let crew = (1. - smooth(edge - 8., edge, p[0]))
            * (1. - smooth(v.hip[1] - 24., v.hip[1] + 3., p[1]));
        let angle = if matches!(v.motion, Motion::Combat) {
            0.018 * self.sin + 0.004 * self.sin2
        } else {
            0.014 * self.sin
        };
        let (sin, cos) = angle.sin_cos();
        let dx = p[0] - v.hip[0];
        let dy = p[1] - v.hip[1];
        let mut source = [
            p[0] + ((cos - 1.) * dx + sin * dy) * crew,
            p[1] + (-sin * dx + (cos - 1.) * dy) * crew,
        ];
        if v.chariot && matches!(v.motion, Motion::Idle) {
            // Breathing/gaze motion belongs to the horses; the cart is fixed.
            let horse = smooth(140., 167., p[0])
                * smooth(100., 123., p[1])
                * (1. - smooth(192., 247., p[1]));
            source[0] -= 1.5 * self.sin * horse;
            source[1] += 1.0 * self.sin2 * horse;
        }
        Some(if source == p {
            destination
        } else {
            self.to_tile(source)
        })
    }

    fn to_tile(&self, point: [f32; 2]) -> [f32; 2] {
        let v = self.vehicle;
        [v.origin[0] + point[0] * v.scale[0], v.origin[1] + point[1] * v.scale[1]]
    }

    fn wheel(&self, point: [f32; 2]) -> Option<[f32; 2]> {
        for wheel in &self.vehicle.wheels {
            let radial = [
                (point[0] - wheel.center[0]) / wheel.radius[0],
                (point[1] - wheel.center[1]) / wheel.radius[1],
            ];
            let radius2 = radial[0] * radial[0] + radial[1] * radial[1];
            if radius2 > 1.25 * 1.25 {
                continue;
            }
            // Preserve the complete outline, shadow and axle hub. Projective
            // rotation of inset spokes preserves the painted ellipse exactly.
            if matches!(self.vehicle.motion, Motion::Movement)
                && wheel.spokes
                && radius2 > 0.40 * 0.40
                && radius2 < 0.70 * 0.70
            {
                return Some([
                    wheel.center[0]
                        + (self.cos * radial[0] + self.sin * radial[1]) * wheel.radius[0],
                    wheel.center[1]
                        + (-self.sin * radial[0] + self.cos * radial[1]) * wheel.radius[1],
                ]);
            }
            return Some(point);
        }
        None
    }
}

fn smooth(from: f32, to: f32, value: f32) -> f32 {
    let t = ((value - from) / (to - from)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}
