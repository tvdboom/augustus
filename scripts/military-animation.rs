//! Bake closed military animation cycles from one coherent painted pose.
//!
//! Every frame samples the same image through a small, periodic deformation
//! field. There is no pose crossfade, frame-dependent crop, scale, or pivot.
//! Phase zero is not repeated at the end: the last frame is one ordinary time
//! step before the first, with matching position and velocity across the seam.

use crate::military_frames::{BASELINE, COLUMNS, COUNT, HEIGHT, SIZE, WIDTH};
use image::{Rgba, RgbaImage};
use std::{f32::consts::TAU, path::Path};

#[path = "military-combat.rs"]
mod combat;
#[path = "military-gait.rs"]
mod gait;

#[derive(Clone, Copy, Debug)]
pub enum Motion {
    Idle,
    Movement,
    Combat,
}

#[derive(Clone, Copy)]
enum Body {
    Infantry,
    Horse,
    Camel,
    Elephant,
    Chariot,
    Engine,
}

struct Pose {
    /// Premultiplied color, filtered once before any animation is applied.
    pixels: RgbaImage,
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    body: Body,
    spear: bool,
    name: String,
}

#[derive(Clone, Copy)]
struct Phase {
    sin: f32,
    cos: f32,
    sin2: f32,
    cos2: f32,
}

impl Phase {
    fn new(angle: f32) -> Self {
        let (sin, cos) = angle.sin_cos();
        let (sin2, cos2) = (angle * 2.).sin_cos();
        Self {
            sin,
            cos,
            sin2,
            cos2,
        }
    }
}

pub fn build(source: &Path, target: &Path, motion: Motion) {
    let pose = prepare_pose(source, motion);
    let mut atlas = RgbaImage::new(WIDTH, HEIGHT);
    for frame in 0..COUNT {
        let angle = TAU * frame as f32 / COUNT as f32;
        let phase = Phase::new(angle);
        let gait = matches!(motion, Motion::Movement)
            .then(|| gait::Rig::new(&pose.name, pose.width, pose.height, angle));
        let combat = matches!(motion, Motion::Combat)
            .then(|| combat::Combat::new(&pose.name, pose.width, pose.height, angle));
        let offset_x = frame as u32 % COLUMNS * SIZE;
        let offset_y = frame as u32 / COLUMNS * SIZE;
        for y in 0..SIZE {
            for x in 0..SIZE {
                let destination = [x as f32, y as f32];
                let mut source_point = destination;
                if let Some(rig) = &gait {
                    let local = rig.source_point(
                        (destination[0] - pose.left) / pose.width,
                        (destination[1] - pose.top) / pose.height,
                    );
                    source_point =
                        [pose.left + local[0] * pose.width, pose.top + local[1] * pose.height];
                } else {
                    let passes = if matches!(motion, Motion::Idle) {
                        2
                    } else {
                        6
                    };
                    for _ in 0..passes {
                        let local_x = (source_point[0] - pose.left) / pose.width;
                        let local_y = (source_point[1] - pose.top) / pose.height;
                        let displacement = if let Some(rig) = &combat {
                            rig.displacement(local_x, local_y)
                        } else {
                            motion_at(&pose, motion, phase, local_x, local_y)
                        };
                        source_point =
                            [destination[0] - displacement[0], destination[1] - displacement[1]];
                    }
                }
                atlas.put_pixel(
                    offset_x + x,
                    offset_y + y,
                    sample(&pose.pixels, source_point[0], source_point[1]),
                );
            }
        }
    }
    atlas.save(target).expect("write closed military animation cycle");
}

fn prepare_pose(source: &Path, motion: Motion) -> Pose {
    let atlas = image::open(source).expect("military source PNG must be valid").to_rgba8();
    let name = source.file_stem().and_then(|name| name.to_str()).unwrap();
    let row = match motion {
        Motion::Idle => 0,
        Motion::Movement => 2,
        Motion::Combat => 3,
    };
    let cell_width = atlas.width() / 4;
    let mut cell = image::imageops::crop_imm(
        &atlas,
        0,
        atlas.height() * row / 4,
        cell_width,
        atlas.height() / 4,
    )
    .to_image();
    // Source sheets sometimes include the next row's detached spear point.
    // Clean the one selected pose before measuring its permanent root anchor.
    crate::retain_sprite_subject(&mut cell);
    let (mut left, mut top, mut right, mut bottom) = (cell.width(), cell.height(), 0, 0);
    for (x, y, pixel) in cell.enumerate_pixels() {
        if pixel[3] > 8 {
            left = left.min(x);
            top = top.min(y);
            right = right.max(x + 1);
            bottom = bottom.max(y + 1);
        }
    }
    assert!(right > left && bottom > top, "military pose must be visible");
    let mut ground_left = right;
    let mut ground_right = left;
    for (x, y, pixel) in cell.enumerate_pixels() {
        if y >= bottom - (bottom - top) / 7 && pixel[3] > 127 {
            ground_left = ground_left.min(x);
            ground_right = ground_right.max(x + 1);
        }
    }
    let ground = if ground_right > ground_left {
        (ground_left + ground_right) as f32 * 0.5
    } else {
        (left + right) as f32 * 0.5
    };
    // Keep a transparent fringe and ample motion headroom on every side.
    left = left.saturating_sub(2);
    top = top.saturating_sub(2);
    right = (right + 2).min(cell.width());
    bottom = (bottom + 2).min(cell.height());
    let mut crop =
        image::imageops::crop_imm(&cell, left, top, right - left, bottom - top).to_image();
    let centered_width = (ground - left as f32).max(right as f32 - ground) * 2.;
    let scale = (152. / centered_width).min(152. / crop.height() as f32);
    let width = (crop.width() as f32 * scale).round().max(1.) as u32;
    let height = (crop.height() as f32 * scale).round().max(1.) as u32;
    crate::premultiply(&mut crop);
    let small =
        image::imageops::resize(&crop, width, height, image::imageops::FilterType::Lanczos3);
    let x = (SIZE as f32 * 0.5 - (ground - left as f32) * scale).round() as i64;
    let foot = small
        .enumerate_pixels()
        .filter(|(_, _, pixel)| pixel[3] > 127)
        .map(|(_, y, _)| y)
        .max()
        .expect("normalized military pose must be visible");
    let y = i64::from(BASELINE) - i64::from(foot);
    let mut pixels = RgbaImage::new(SIZE, SIZE);
    image::imageops::replace(&mut pixels, &small, x, y);
    // Keep one soft fringe row below the opaque foot, without a hanging
    // resampling halo that changes the perceived baseline across unit types.
    for row in BASELINE + 2..SIZE {
        for column in 0..SIZE {
            pixels.put_pixel(column, row, Rgba([0; 4]));
        }
    }
    let body = match name {
        "light-infantry" | "heavy-infantry" | "archers" => Body::Infantry,
        "war-camels" => Body::Camel,
        "war-elephants" => Body::Elephant,
        "war-chariots" => Body::Chariot,
        "ballista" | "catapult" => Body::Engine,
        "light-cavalry" | "heavy-cavalry" | "horse-archers" => Body::Horse,
        _ => panic!("unknown military rig {name}"),
    };
    Pose {
        pixels,
        left: x as f32,
        top: y as f32,
        width: width as f32,
        height: height as f32,
        body,
        spear: matches!(name, "light-infantry" | "heavy-infantry"),
        name: name.to_owned(),
    }
}

/// Smoothstep masks describe joint regions in the single cropped source pose.
/// All time terms are integral sine/cosine harmonics, so both displacement and
/// its derivative are periodic. Amplitudes are final tile pixels, never scale.
fn motion_at(pose: &Pose, motion: Motion, p: Phase, x: f32, y: f32) -> [f32; 2] {
    match pose.body {
        Body::Infantry => infantry_motion(pose.spear, motion, p, x, y),
        Body::Engine => engine_motion(motion, p, x, y),
        body => mounted_motion(body, motion, p, x, y),
    }
}

fn infantry_motion(spear: bool, motion: Motion, p: Phase, x: f32, y: f32) -> [f32; 2] {
    // The upper body moves coherently. The waist absorbs the tiny adjustment;
    // lower legs and feet have exactly zero idle/combat displacement.
    let upper = 1. - smooth(0.47, 0.86, y);
    match motion {
        Motion::Idle => {
            // A planted spear remains perfectly straight and fixed on the soil.
            let free = if spear {
                smooth(0.20, 0.32, x)
            } else {
                1.
            };
            [0.65 * p.sin * upper * free, -0.48 * p.cos * upper * free]
        },
        Motion::Movement => {
            let legs = smooth(0.67, 0.98, y);
            let side = 1. - 2. * smooth(0.42, 0.65, x);
            let lift = 0.62 * (1. - p.cos * side);
            [
                0.48 * p.sin * (1. - legs) + 2.45 * p.sin * side * legs,
                -0.65 * p.cos2 * (1. - legs) - lift * legs,
            ]
        },
        Motion::Combat => {
            // A small forward strike/ready/recovery arc keeps the selected
            // weapon and hands together rather than bending a spear or sword.
            [1.75 * p.sin * upper, (-0.50 * p.cos + 0.20 * p.sin2) * upper]
        },
    }
}

fn mounted_motion(body: Body, motion: Motion, p: Phase, x: f32, y: f32) -> [f32; 2] {
    let elephant = matches!(body, Body::Elephant);
    let chariot = matches!(body, Body::Chariot);
    let leg_start = if elephant {
        0.72
    } else {
        0.66
    };
    let legs = smooth(leg_start, 0.98, y);
    let upper = 1. - smooth(0.50, 0.91, y);
    let rider = (1. - smooth(0.30, 0.47, y)) * (1. - smooth(0.65, 0.78, x));
    let head = smooth(0.57, 0.81, x) * smooth(0.18, 0.36, y) * (1. - smooth(0.59, 0.80, y));
    // Keep the wheeled carriage still; horse and rider regions move separately.
    let animal = if chariot {
        smooth(0.34, 0.49, x)
    } else {
        1.
    };
    match motion {
        Motion::Idle => {
            let tail =
                (1. - smooth(0.08, 0.23, x)) * smooth(0.39, 0.55, y) * (1. - smooth(0.70, 0.89, y));
            let trunk = if elephant {
                smooth(0.78, 0.94, x) * smooth(0.56, 0.76, y)
            } else {
                0.
            };
            [
                0.40 * p.sin * rider
                    + 0.55 * p.sin * head
                    + 0.60 * p.sin2 * tail
                    + 0.65 * p.sin * trunk,
                -0.35 * p.cos * upper * animal - 0.23 * p.sin * rider + 0.30 * p.sin * head,
            ]
        },
        Motion::Movement => {
            let leg_phase = if chariot {
                1. - 2. * smooth(0.63, 0.76, x)
            } else {
                1. - 2. * smooth(0.39, 0.64, x)
            };
            let stride = if elephant {
                1.7
            } else {
                2.65
            };
            [
                animal * (stride * p.sin * leg_phase * legs + 0.38 * p.sin * head)
                    + 0.42 * p.sin * rider,
                animal * (-0.65 * p.cos2 * (1. - legs) - 0.58 * (1. - p.cos * leg_phase) * legs)
                    - 0.28 * p.sin * rider,
            ]
        },
        Motion::Combat => [
            1.05 * p.sin * upper * animal + 0.62 * p.sin * rider,
            -0.42 * p.cos * upper * animal + 0.38 * p.sin * head - 0.2 * p.cos * rider,
        ],
    }
}

fn engine_motion(motion: Motion, p: Phase, x: f32, y: f32) -> [f32; 2] {
    // The crew breathe/work around a solid chassis. Wheels, wooden beams and
    // ropes must not squash with the operator's body.
    let crew = (1. - smooth(0.28, 0.46, x)) * (1. - smooth(0.43, 0.73, y));
    match motion {
        Motion::Idle => [0.42 * p.sin * crew, -0.45 * p.cos * crew],
        Motion::Movement => {
            let feet = (1. - smooth(0.21, 0.34, x)) * smooth(0.64, 0.89, y);
            [1.25 * p.sin * feet + 0.35 * p.sin * crew, -0.40 * p.cos2 - 0.3 * p.sin * crew]
        },
        Motion::Combat => [1.30 * p.sin * crew, (-0.65 * p.cos + 0.2 * p.sin2) * crew],
    }
}

fn smooth(from: f32, to: f32, value: f32) -> f32 {
    let t = ((value - from) / (to - from)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}

/// Bilinear interpolation operates only on premultiplied color, then restores
/// straight alpha for PNG. Transparent samples never contribute black fringes.
fn sample(pixels: &RgbaImage, x: f32, y: f32) -> Rgba<u8> {
    let left = x.floor() as i32;
    let top = y.floor() as i32;
    let fraction_x = x - left as f32;
    let fraction_y = y - top as f32;
    let mut color = [0_f32; 4];
    for (dy, weight_y) in [(0, 1. - fraction_y), (1, fraction_y)] {
        for (dx, weight_x) in [(0, 1. - fraction_x), (1, fraction_x)] {
            let source_x = left + dx;
            let source_y = top + dy;
            if source_x < 0
                || source_y < 0
                || source_x >= pixels.width() as i32
                || source_y >= pixels.height() as i32
            {
                continue;
            }
            let pixel = pixels.get_pixel(source_x as u32, source_y as u32);
            for channel in 0..4 {
                color[channel] += pixel[channel] as f32 * weight_x * weight_y;
            }
        }
    }
    let alpha = color[3].round().clamp(0., 255.) as u8;
    if alpha == 0 {
        return Rgba([0; 4]);
    }
    Rgba([
        (color[0] * 255. / color[3]).round().clamp(0., 255.) as u8,
        (color[1] * 255. / color[3]).round().clamp(0., 255.) as u8,
        (color[2] * 255. / color[3]).round().clamp(0., 255.) as u8,
        alpha,
    ])
}
