//! Assemble each loop from one immutable architecture plate and twelve worker poses.
use crate::wonder_frames::{
    ACTIVITY_TOP, COLUMNS, COUNT, ROWS, SIZE, WORKER_CENTERS, WORKER_FEET, WORKER_HEIGHT,
    WORKER_WIDTH,
};
use image::{imageops, RgbaImage};
use std::path::Path;

pub(super) fn build(source: &Path, workers: &Path, target: &Path) {
    let plate = image::open(source).expect("construction plate PNG").to_rgba8();
    let plate = trim(&plate);
    let plate = resize(&plate, (336. / plate.width() as f32).min(300. / plate.height() as f32));
    let mut fixed = RgbaImage::new(SIZE, SIZE);
    let x = (SIZE - plate.width()) / 2;
    let y = 10 + (300 - plate.height()) / 2;
    imageops::replace(&mut fixed, &plate, x.into(), y.into());

    let workers = image::open(workers).expect("twelve-frame workers PNG").to_rgba8();
    let mut poses = Vec::new();
    for frame in 0..COUNT {
        let left = workers.width() * (frame % COLUMNS) / COLUMNS;
        let right = workers.width() * (frame % COLUMNS + 1) / COLUMNS;
        let top = workers.height() * (frame / COLUMNS) / ROWS;
        let bottom = workers.height() * (frame / COLUMNS + 1) / ROWS;
        // Keep the mason, carrier and winch operator as separate foreground figures.
        for actor in 0..3 {
            // The winch operator's stance is wider than the stone carrier's.
            let divisions = [0, 8, 15, 24];
            let actor_left = left + (right - left) * divisions[actor as usize] / 24;
            let actor_right = left + (right - left) * divisions[actor as usize + 1] / 24;
            let cell = imageops::crop_imm(
                &workers,
                actor_left,
                top,
                actor_right - actor_left,
                bottom - top,
            )
            .to_image();
            poses.push(trim(&cell));
        }
    }
    // One shared scale for the entire loop; changing tools/limbs never resizes a worker.
    let scale = poses
        .iter()
        .map(|p| {
            (WORKER_WIDTH as f32 / p.width() as f32).min(WORKER_HEIGHT as f32 / p.height() as f32)
        })
        .fold(f32::INFINITY, f32::min);
    let mut atlas = RgbaImage::new(COLUMNS * SIZE, ROWS * SIZE);
    for frame in 0..COUNT {
        let mut cell = fixed.clone();
        for actor in 0..3 {
            let pose = resize(&poses[(frame * 3 + actor) as usize], scale);
            let x = WORKER_CENTERS[actor as usize] - pose.width() / 2;
            let y = WORKER_FEET[actor as usize] - pose.height();
            assert!(y >= ACTIVITY_TOP, "workers must stay in the foreground");
            imageops::overlay(&mut cell, &pose, x.into(), y.into());
        }
        imageops::replace(
            &mut atlas,
            &cell,
            ((frame % COLUMNS) * SIZE).into(),
            ((frame / COLUMNS) * SIZE).into(),
        );
    }
    atlas.save(target).expect("write twelve-frame construction atlas");
}

fn trim(source: &RgbaImage) -> RgbaImage {
    let (mut left, mut top, mut right, mut bottom) = (source.width(), source.height(), 0, 0);
    for (x, y, p) in source.enumerate_pixels() {
        if p[3] > 8 {
            left = left.min(x);
            top = top.min(y);
            right = right.max(x + 1);
            bottom = bottom.max(y + 1);
        }
    }
    assert!(right > left && bottom > top, "visible construction layer");
    imageops::crop_imm(source, left, top, right - left, bottom - top).to_image()
}

fn resize(source: &RgbaImage, scale: f32) -> RgbaImage {
    let mut source = source.clone();
    for p in source.pixels_mut() {
        let alpha = u16::from(p.0[3]);
        for c in &mut p.0[..3] {
            *c = ((u16::from(*c) * alpha + 127) / 255) as u8;
        }
    }
    let mut result = imageops::resize(
        &source,
        (source.width() as f32 * scale).round().max(1.) as u32,
        (source.height() as f32 * scale).round().max(1.) as u32,
        imageops::FilterType::Lanczos3,
    );
    for p in result.pixels_mut() {
        let a = u16::from(p[3]);
        for c in &mut p.0[..3] {
            *c = (u16::from(*c) * 255 + a / 2).checked_div(a).unwrap_or_default().min(255) as u8;
        }
    }
    result
}
