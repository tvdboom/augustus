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
                    for _ in 0..6 {
                        let local_x = (source_point[0] - pose.left) / pose.width;
                        let local_y = (source_point[1] - pose.top) / pose.height;
                        let displacement = if let Some(rig) = &combat {
                            rig.displacement(local_x, local_y)
                        } else {
                            idle_motion(&pose, phase, local_x, local_y)
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
    let scale = (152. / centered_width).min(152. / crop.height() as f32)
        // The upright spear adds empty height above the light infantry's head.
        // Calibrate his actual crown-to-sole stature against the archer, rather
        // than letting the weapon make an adult soldier look shorter.
        * if name == "light-infantry" { 1.06 } else { 1.0 };
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
        spear: name == "light-infantry",
        name: name.to_owned(),
    }
}

/// Visible, continuous idle gestures. Amplitudes are tile pixels: at close map
/// zoom the previous subpixel offsets disappeared after the atlas was scaled.
/// Feet remain fixed, while bodies shift weight and equipment follows the hands.
/// Integral harmonics make position and velocity agree across the loop boundary.
fn idle_motion(pose: &Pose, p: Phase, x: f32, y: f32) -> [f32; 2] {
    match pose.body {
        Body::Infantry => infantry_idle(pose, p, x, y),
        Body::Engine => {
            // Only the operator leans over the windlass. The wheels and wooden
            // frame are fixed, as are the operator's planted lower legs.
            let crew = (1. - smooth(0.28, 0.49, x)) * (1. - smooth(0.40, 0.73, y));
            [(3.5 * p.sin + 0.5 * p.sin2) * crew, (-1.8 * p.cos + 0.35 * p.cos2) * crew]
        },
        body => mounted_idle(body, p, x, y),
    }
}

fn infantry_idle(pose: &Pose, p: Phase, x: f32, y: f32) -> [f32; 2] {
    let upper = 1. - smooth(0.50, 0.89, y);
    let free = if pose.spear {
        smooth(0.20, 0.38, x)
    } else {
        1.
    };
    let mut displacement = [
        (2.8 * p.sin + 0.65 * p.sin2) * upper * free,
        (-1.65 * p.cos + 0.25 * p.cos2) * upper * free,
    ];
    if pose.spear {
        // The planted spear turns as one rigid length around its ground end.
        // Its upper end visibly follows the hand during the weight shift.
        let angle = 0.027 * p.sin;
        let planted = (1. - free) * (1. - smooth(0.88, 0.95, y));
        displacement[0] += -(y - 0.95) * pose.height * angle * planted;
        displacement[1] += (x - 0.14) * pose.width * angle * planted;
    }
    // A small neck rotation makes the gaze change independently of breathing.
    // The pivot accommodates helmets/crests and the archer's uncovered head.
    let neck_y = if pose.name == "light-infantry" {
        0.31
    } else {
        0.25
    };
    let head = smooth(0.24, 0.39, x)
        * (1. - smooth(0.76, 0.93, x))
        * (1. - smooth(neck_y - 0.04, neck_y + 0.11, y));
    let turn = 0.034 * p.sin2;
    displacement[0] += -(y - neck_y) * pose.height * turn * head;
    displacement[1] += (x - 0.56) * pose.width * turn * head;
    displacement
}

fn mounted_idle(body: Body, p: Phase, x: f32, y: f32) -> [f32; 2] {
    let elephant = matches!(body, Body::Elephant);
    let chariot = matches!(body, Body::Chariot);
    let upper = 1. - smooth(0.50, 0.90, y);
    let animal = if chariot {
        smooth(0.34, 0.50, x)
    } else {
        1.
    };
    let rider = (1. - smooth(0.31, 0.53, y)) * (1. - smooth(0.62, 0.80, x));
    let head = smooth(0.56, 0.83, x) * smooth(0.18, 0.35, y) * (1. - smooth(0.58, 0.80, y));
    let tail = (1. - smooth(0.08, 0.24, x)) * smooth(0.39, 0.54, y) * (1. - smooth(0.68, 0.88, y));
    let trunk = if elephant {
        smooth(0.77, 0.94, x) * smooth(0.56, 0.78, y)
    } else {
        0.
    };
    [
        1.25 * p.sin * upper * animal
            + (2.35 * p.sin + 0.70 * p.sin2) * rider
            + 2.1 * p.sin * head
            + (4.2 * p.sin2 + 0.6 * p.sin) * tail
            + 4.8 * p.sin * trunk,
        (-1.2 * p.cos + 0.2 * p.cos2) * upper * animal - 1.1 * p.sin * rider
            + 2.4 * p.sin * head
            + 1.1 * p.cos2 * tail
            - 1.6 * (1. - p.cos) * trunk,
    ]
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
