//! Prepare the small embedded panel icons while compiling, not on the click frame.

use std::path::PathBuf;

#[path = "scripts/wonder-construction.rs"]
mod wonder_construction;
#[path = "src/map/wonder_frames.rs"]
mod wonder_frames;

const PANEL_ICONS: &[(&str, &str)] = &[
    ("images/military/tactics/balanced-owl.png", "balanced-owl"),
    ("images/icons/sestertius.png", "coin"),
    ("images/icons/influence.png", "influence"),
    ("images/icons/manpower.png", "population"),
    ("images/icons/happiness.png", "happiness"),
    ("images/icons/morale.png", "morale"),
    ("images/icons/terrain.png", "terrain"),
    ("images/icons/military_power.png", "military-power"),
    ("images/icons/nobles.png", "nobles"),
    ("images/icons/civilians.png", "civilians"),
    ("images/icons/plebeians.png", "plebeians"),
    ("images/icons/slaves.png", "slaves"),
    ("images/icons/food.png", "food"),
    ("images/icons/metal.png", "metal"),
    ("images/icons/stone.png", "stone"),
    ("images/icons/spy.png", "spy"),
    ("images/icons/spy-build-control.png", "spy-build-control"),
    ("images/icons/spy-improve-relations.png", "spy-improve-relations"),
    ("images/icons/spy-uncover-scandals.png", "spy-uncover-scandals"),
    ("images/icons/spy-undermine-opponents.png", "spy-undermine-opponents"),
    ("images/icons/trade.png", "trade"),
    ("images/icons/control.png", "control"),
    ("images/icons/relation.png", "relation"),
    ("images/icons/diplomacy.png", "diplomacy"),
    ("images/icons/policies.png", "policies"),
    ("images/icons/construction.png", "construction"),
    ("images/icons/recruitment.png", "recruitment"),
    ("images/icons/military-access.png", "military-access"),
    ("images/icons/catapult.png", "catapult"),
    ("images/icons/war-chariots.png", "war-chariots"),
    ("images/icons/recruitment-time.png", "recruitment-time"),
    ("images/icons/offense.png", "offense"),
    ("images/icons/defense.png", "defense"),
    ("images/icons/speed.png", "speed"),
    ("images/icons/maneuver.png", "maneuver"),
    ("images/icons/orders.png", "orders"),
    ("images/icons/amount.png", "amount"),
    ("images/icons/base-amount.png", "base-amount"),
    ("images/icons/delta.png", "delta"),
    ("images/icons/change.png", "change"),
    ("images/icons/notifications.png", "notifications"),
    ("images/icons/cancel.png", "cancel"),
    ("images/icons/notice.png", "notice"),
    ("images/icons/confirm.png", "confirm"),
    ("images/icons/court-nobles.png", "court-nobles"),
    ("images/icons/attack.png", "attack"),
    ("images/ui/spqr-eagle-gold.png", "spqr-eagle-gold"),
    ("images/icons/province.png", "province"),
    ("images/buildings/aqueduct.png", "aqueduct"),
    ("images/buildings/granary.png", "granary"),
    ("images/buildings/granary-rural.png", "granary-rural"),
    ("images/buildings/road.png", "road"),
    ("images/buildings/baths.png", "baths"),
    ("images/buildings/walls.png", "walls"),
    ("images/buildings/forum.png", "forum"),
    ("images/buildings/marketplace.png", "marketplace"),
    ("images/buildings/foundry.png", "foundry"),
    ("images/buildings/academy.png", "academy"),
    ("images/buildings/great-temple.png", "great-temple"),
    ("images/buildings/grand-theater.png", "grand-theater"),
];

fn main() {
    let repository = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let source_root = repository.join("assets");
    let output_root = PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("panel-icons");
    std::fs::create_dir_all(&output_root).expect("create panel icon output directory");
    // Original Imperator symbols extracted with transparent parchment from Paradox's
    // published army-interface screenshot. See docs/references/README.md.
    let reference = repository.join("docs/references/imperator-army-tactics.png");
    println!("cargo:rerun-if-changed={}", reference.display());
    let tactics = image::open(reference).expect("Imperator army reference").to_rgba8();
    let tactic_root = output_root.with_file_name("tactic-icons");
    std::fs::create_dir_all(&tactic_root).expect("create tactic icon directory");
    for (name, x, y, size) in [
        ("shock-action", 565, 173, 52),
        ("envelopment", 565, 253, 52),
        ("skirmishing", 565, 333, 52),
        ("deception", 565, 413, 52),
        ("bottleneck", 565, 493, 52),
    ] {
        let mut symbol = image::imageops::crop_imm(&tactics, x, y, size, size.min(46)).to_image();
        // Remove only edge-connected parchment, preserving enclosed animal detail.
        let background = *symbol.get_pixel(0, 0);
        let mut pending = std::collections::VecDeque::new();
        let mut visited = vec![false; (symbol.width() * symbol.height()) as usize];
        for x in 0..symbol.width() {
            pending.push_back((x, 0));
            pending.push_back((x, symbol.height() - 1));
        }
        for y in 0..symbol.height() {
            pending.push_back((0, y));
            pending.push_back((symbol.width() - 1, y));
        }
        while let Some((x, y)) = pending.pop_front() {
            let index = (y * symbol.width() + x) as usize;
            if visited[index] {
                continue;
            }
            visited[index] = true;
            let pixel = symbol.get_pixel_mut(x, y);
            let distance = (0..3)
                .map(|c| (i16::from(pixel[c]) - i16::from(background[c])).unsigned_abs())
                .max()
                .unwrap();
            if distance > 62 {
                continue;
            }
            pixel[3] = 0;
            if x > 0 {
                pending.push_back((x - 1, y));
            }
            if y > 0 {
                pending.push_back((x, y - 1));
            }
            if x + 1 < symbol.width() {
                pending.push_back((x + 1, y));
            }
            if y + 1 < symbol.height() {
                pending.push_back((x, y + 1));
            }
        }
        symbol
            .save(tactic_root.join(format!("{name}.png")))
            .expect("write transparent tactic symbol");
    }

    for &(source, name) in PANEL_ICONS {
        let path = source_root.join(source);
        println!("cargo:rerun-if-changed={}", path.display());
        let mut rgba = image::open(&path).expect("panel icon PNG must be valid").to_rgba8();
        // Keep the same premultiplied Lanczos resize used by the original
        // runtime path, including its transparent-edge handling.
        for pixel in rgba.pixels_mut() {
            let alpha = u16::from(pixel[3]);
            for channel in &mut pixel.0[..3] {
                *channel = ((u16::from(*channel) * alpha + 127) / 255) as u8;
            }
        }
        // Standalone unit art also appears as a large recruitment tooltip illustration.
        // Retain enough detail for that size and high-DPI displays.
        let size = if matches!(name, "catapult" | "war-chariots") {
            256
        } else if name == "military-access" {
            128
        } else {
            64
        };
        let mut small =
            image::imageops::resize(&rgba, size, size, image::imageops::FilterType::Lanczos3);
        for pixel in small.pixels_mut() {
            let alpha = u16::from(pixel[3]);
            for channel in &mut pixel.0[..3] {
                *channel = (u16::from(*channel) * 255 + alpha / 2)
                    .checked_div(alpha)
                    .unwrap_or_default()
                    .min(255) as u8;
            }
        }
        small.save(output_root.join(format!("{name}.png"))).expect("write panel icon PNG");
        if source.starts_with("images/buildings/") || name == "province" {
            let large_root = output_root.with_file_name("building-icons");
            std::fs::create_dir_all(&large_root).expect("create building illustration directory");
            normalize_animation(&path, &large_root.join(format!("{name}.png")), 128);
        }
    }
    // Like the existing embedded panel art, normalize animation sources at build
    // time so opening a map panel never performs a large Lanczos resample.
    let animation_root = PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("animations");
    for (folder, width, height) in [
        ("military", 768, 768),
        ("military/idle", 768, 768),
        (
            "wonders/construction",
            wonder_frames::COLUMNS * wonder_frames::SIZE,
            wonder_frames::ROWS * wonder_frames::SIZE,
        ),
    ] {
        let input = source_root.join("images").join(folder);
        let output = animation_root.join(folder);
        std::fs::create_dir_all(&output).expect("create animation output directory");
        println!("cargo:rerun-if-changed={}", input.display());
        for entry in std::fs::read_dir(input).expect("animation source directory") {
            let path = entry.expect("animation source entry").path();
            if path.extension().and_then(|s| s.to_str()) != Some("png") {
                continue;
            }
            println!("cargo:rerun-if-changed={}", path.display());
            let target = output.join(path.file_name().unwrap());
            if folder == "wonders/construction" {
                let workers = source_root.join("images/wonders/construction-workers.png");
                println!("cargo:rerun-if-changed={}", workers.display());
                wonder_construction::build(&path, &workers, &target);
            } else if folder == "military/idle" {
                normalize_idle(&path, &target);
            } else {
                normalize_animation_size(&path, &target, width, height);
            }
        }
    }
}

/// Keep sixteen generated poses at one scale and foot baseline, with sampling gutters.
fn normalize_idle(source: &std::path::Path, target: &std::path::Path) {
    let rgba = image::open(source).expect("idle PNG must be valid").to_rgba8();
    let mut poses = Vec::with_capacity(16);
    for row in 0..4 {
        for column in 0..4 {
            let left = rgba.width() * column / 4;
            let top = rgba.height() * row / 4;
            let right = rgba.width() * (column + 1) / 4;
            let bottom = rgba.height() * (row + 1) / 4;
            let mut cell =
                image::imageops::crop_imm(&rgba, left, top, right - left, bottom - top).to_image();
            if source.file_name().and_then(|name| name.to_str()) == Some("light-infantry.png") {
                // The third source row contains tips of the next row's spears.
                // They must not determine this soldier's bounds or foot baseline.
                retain_idle_subject(&mut cell);
            }
            let (mut x0, mut y0, mut x1, mut y1) = (cell.width(), cell.height(), 0, 0);
            for (x, y, pixel) in cell.enumerate_pixels() {
                if pixel[3] > 8 {
                    x0 = x0.min(x);
                    y0 = y0.min(y);
                    x1 = x1.max(x + 1);
                    y1 = y1.max(y + 1);
                }
            }
            assert!(x1 > x0 && y1 > y0, "idle frame must be visible");
            // Align the planted feet/wheels, not the entire silhouette: turning
            // a shield, bow, tail or trunk must not translate the subject sideways.
            let mut ground_left = x1;
            let mut ground_right = x0;
            for (x, y, pixel) in cell.enumerate_pixels() {
                if y >= y1 - (y1 - y0) / 6 && pixel[3] > 127 {
                    ground_left = ground_left.min(x);
                    ground_right = ground_right.max(x + 1);
                }
            }
            let ground_center = if ground_right > ground_left {
                (ground_left + ground_right) as f32 * 0.5 - x0 as f32
            } else {
                (x1 - x0) as f32 * 0.5
            };
            poses.push((
                image::imageops::crop_imm(&cell, x0, y0, x1 - x0, y1 - y0).to_image(),
                ground_center,
            ));
        }
    }
    let max_width = poses
        .iter()
        .map(|(pose, ground)| ground.max(pose.width() as f32 - ground) * 2.)
        .fold(0_f32, f32::max);
    let max_height = poses.iter().map(|(pose, _)| pose.height()).max().unwrap() as f32;
    let scale = (154. / max_width).min(154. / max_height);
    let mut sheet = image::RgbaImage::new(768, 768);
    for (frame, (mut pose, ground)) in poses.into_iter().enumerate() {
        premultiply(&mut pose);
        let mut small = image::imageops::resize(
            &pose,
            (pose.width() as f32 * scale).round().max(1.) as u32,
            (pose.height() as f32 * scale).round().max(1.) as u32,
            image::imageops::FilterType::Lanczos3,
        );
        unpremultiply(&mut small);
        let x = frame as u32 % 4 * 192 + (96. - ground * scale).round() as u32;
        let y = frame as u32 / 4 * 192 + 173 - small.height();
        image::imageops::replace(&mut sheet, &small, i64::from(x), i64::from(y));
    }
    sheet.save(target).expect("write normalized sixteen-frame idle sheet");
}

/// Keep the connected soldier, including soft alpha edges, and discard neighboring-cell debris.
fn retain_idle_subject(cell: &mut image::RgbaImage) {
    let width = cell.width() as usize;
    let height = cell.height() as usize;
    let mut visited = vec![false; width * height];
    let mut subject = Vec::new();
    for start in 0..visited.len() {
        if visited[start] || cell.get_pixel((start % width) as u32, (start / width) as u32)[3] <= 8
        {
            continue;
        }
        let mut component = vec![start];
        visited[start] = true;
        let mut cursor = 0;
        while cursor < component.len() {
            let index = component[cursor];
            cursor += 1;
            let x = index % width;
            let y = index / width;
            for next_y in y.saturating_sub(1)..=(y + 1).min(height - 1) {
                for next_x in x.saturating_sub(1)..=(x + 1).min(width - 1) {
                    let next = next_y * width + next_x;
                    if !visited[next] && cell.get_pixel(next_x as u32, next_y as u32)[3] > 8 {
                        visited[next] = true;
                        component.push(next);
                    }
                }
            }
        }
        if component.len() > subject.len() {
            subject = component;
        }
    }
    let mut retained = vec![false; width * height];
    for index in subject {
        // Preserve the subject's faint antialiased fringe without following
        // nearly transparent bridges into another frame's spear tip.
        let x = index % width;
        let y = index / width;
        for next_y in y.saturating_sub(1)..=(y + 1).min(height - 1) {
            for next_x in x.saturating_sub(1)..=(x + 1).min(width - 1) {
                retained[next_y * width + next_x] = true;
            }
        }
    }
    for (index, pixel) in cell.pixels_mut().enumerate() {
        if !retained[index] {
            *pixel = image::Rgba([0; 4]);
        }
    }
}

/// Premultiply before filtering and restore straight alpha to avoid dark sprite fringes.
fn normalize_animation(source: &std::path::Path, target: &std::path::Path, size: u32) {
    normalize_animation_size(source, target, size, size);
}

fn normalize_animation_size(
    source: &std::path::Path,
    target: &std::path::Path,
    width: u32,
    height: u32,
) {
    let mut rgba = image::open(source).expect("animation PNG must be valid").to_rgba8();
    premultiply(&mut rgba);
    let mut small =
        image::imageops::resize(&rgba, width, height, image::imageops::FilterType::Lanczos3);
    unpremultiply(&mut small);
    small.save(target).expect("write optimized animation PNG");
}

fn premultiply(rgba: &mut image::RgbaImage) {
    for pixel in rgba.pixels_mut() {
        let alpha = u16::from(pixel[3]);
        for channel in &mut pixel.0[..3] {
            *channel = ((u16::from(*channel) * alpha + 127) / 255) as u8;
        }
    }
}

fn unpremultiply(rgba: &mut image::RgbaImage) {
    for pixel in rgba.pixels_mut() {
        let alpha = u16::from(pixel[3]);
        for channel in &mut pixel.0[..3] {
            *channel = (u16::from(*channel) * 255 + alpha / 2)
                .checked_div(alpha)
                .unwrap_or_default()
                .min(255) as u8;
        }
    }
}
