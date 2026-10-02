//! Prepare the small embedded panel icons while compiling, not on the click frame.

use std::path::PathBuf;

#[path = "scripts/military-animation.rs"]
mod military_animation;
#[path = "src/map/military_frames.rs"]
mod military_frames;
#[path = "scripts/wonder-construction.rs"]
mod wonder_construction;
#[path = "src/map/wonder_frames.rs"]
mod wonder_frames;

const PANEL_ICONS: &[(&str, &str)] = &[
    ("images/military/tactics/shock-action.png", "tactic-shock-action"),
    ("images/military/tactics/envelopment.png", "tactic-envelopment"),
    ("images/military/tactics/skirmishing.png", "tactic-skirmishing"),
    ("images/military/tactics/deception.png", "tactic-deception"),
    ("images/military/tactics/bottleneck.png", "tactic-bottleneck"),
    ("images/military/tactics/phalanx.png", "tactic-phalanx"),
    ("images/icons/sestertius.png", "coin"),
    ("images/icons/influence.png", "influence"),
    ("images/icons/manpower.png", "population"),
    ("images/icons/happiness.png", "happiness"),
    ("images/icons/morale.png", "morale"),
    ("images/icons/terrain.png", "terrain"),
    ("images/icons/military_power.png", "military-power"),
    ("images/icons/rank-centurion.png", "rank-centurion"),
    ("images/icons/rank-military-tribune.png", "rank-military-tribune"),
    ("images/icons/rank-legate.png", "rank-legate"),
    ("images/icons/rank-imperator.png", "rank-imperator"),
    ("images/icons/cohorts.png", "cohorts"),
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
    ("images/icons/spy-support-revolt.png", "spy-support-revolt"),
    ("images/icons/spy-discredit-rivals.png", "spy-discredit-rivals"),
    ("images/icons/bribe-nobles.png", "bribe-nobles"),
    ("images/icons/senator-petition.png", "senator-petition"),
    ("images/icons/senator-gift.png", "senator-gift"),
    ("images/icons/senator-patronage.png", "senator-patronage"),
    ("images/icons/senator-bribe.png", "senator-bribe"),
    ("images/icons/senator-threaten.png", "senator-threaten"),
    ("images/icons/senator-murder.png", "senator-murder"),
    ("images/icons/senator-banquet.png", "senator-banquet"),
    ("images/icons/senator-discredit.png", "senator-discredit"),
    ("images/icons/senator-lobby.png", "senator-lobby"),
    ("images/icons/insult-player.png", "insult-player"),
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
    ("images/events/events-icon.png", "events"),
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
    ("images/buildings/city-hall.png", "city-hall"),
    ("images/buildings/academy.png", "academy"),
    ("images/buildings/great-temple.png", "great-temple"),
    ("images/buildings/grand-theater.png", "grand-theater"),
];

fn main() {
    println!("cargo:rerun-if-changed=scripts/military-animation.rs");
    println!("cargo:rerun-if-changed=scripts/military-gait.rs");
    println!("cargo:rerun-if-changed=scripts/military-combat.rs");
    println!("cargo:rerun-if-changed=src/map/military_frames.rs");
    let repository = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let source_root = repository.join("assets");
    let output_root = PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("panel-icons");
    std::fs::create_dir_all(&output_root).expect("create panel icon output directory");
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
        } else if matches!(
            name,
            "military-access"
                | "events"
                | "rank-centurion"
                | "rank-military-tribune"
                | "rank-legate"
                | "rank-imperator"
        ) {
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
    let banner_source = source_root.join("images/cities/policies-panel-banner.png");
    println!("cargo:rerun-if-changed={}", banner_source.display());
    let banner_root = PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("panel-banners");
    std::fs::create_dir_all(&banner_root).expect("create panel banner output directory");
    let banner = image::open(&banner_source).expect("overview banner PNG must be valid");
    banner
        .resize(1024, 1024, image::imageops::FilterType::Lanczos3)
        .save(banner_root.join("policies.png"))
        .expect("write prepared overview banner PNG");
    // Like the existing embedded panel art, normalize animation sources at build
    // time so opening a map panel never performs a large Lanczos resample.
    let animation_root = PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("animations");
    let available = std::thread::available_parallelism().map_or(1, usize::from);
    let jobs = std::env::var("NUM_JOBS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(available)
        .clamp(1, available);
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
        let mut paths = Vec::new();
        for entry in std::fs::read_dir(input).expect("animation source directory") {
            let path = entry.expect("animation source entry").path();
            if path.extension().and_then(|s| s.to_str()) != Some("png") {
                continue;
            }
            println!("cargo:rerun-if-changed={}", path.display());
            paths.push(path);
        }
        if folder == "wonders/construction" {
            println!(
                "cargo:rerun-if-changed={}",
                source_root.join("images/wonders/construction-workers.png").display()
            );
        }
        // Each sheet has independent input and output paths. Bound parallel
        // preparation by Cargo's requested jobs and the host's available cores.
        std::thread::scope(|scope| {
            for chunk in paths.chunks(paths.len().div_ceil(jobs).max(1)) {
                let output = &output;
                let source_root = &source_root;
                scope.spawn(move || {
                    for path in chunk {
                        let target = output.join(path.file_name().unwrap());
                        if folder == "wonders/construction" {
                            let workers =
                                source_root.join("images/wonders/construction-workers.png");
                            wonder_construction::build(path, &workers, &target);
                        } else if folder == "military/idle" {
                            military_animation::build(
                                path,
                                &target,
                                military_animation::Motion::Idle,
                            );
                        } else if folder == "military" {
                            // Panel icons retain their original four-by-four source layout.
                            normalize_animation_size(path, &target, width, height);
                            for (state, motion) in [
                                ("movement", military_animation::Motion::Movement),
                                ("combat", military_animation::Motion::Combat),
                            ] {
                                let output = output.join(state);
                                std::fs::create_dir_all(&output)
                                    .expect("create military cycle directory");
                                military_animation::build(
                                    path,
                                    &output.join(path.file_name().unwrap()),
                                    motion,
                                );
                            }
                        } else {
                            normalize_animation_size(path, &target, width, height);
                        }
                    }
                });
            }
        });
    }
}

/// Keep the connected subject, including soft alpha edges, and discard neighboring-cell debris.
fn retain_sprite_subject(cell: &mut image::RgbaImage) {
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
    // Some generated icon cells contain a detached spear tip or plume from the
    // animation row below. Keep the connected icon before downsampling, so the
    // panel icon UVs cannot show that neighboring-frame debris.
    let clean_icon = if source
        .parent()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        == Some("military")
        && matches!(
            source.file_name().and_then(|name| name.to_str()),
            Some(
                "light-infantry.png"
                    | "light-cavalry.png"
                    | "heavy-cavalry.png"
                    | "war-camels.png"
                    | "heavy-infantry.png"
            )
        ) {
        let mut icon =
            image::imageops::crop_imm(&rgba, 0, 0, rgba.width() / 4, rgba.height() / 4).to_image();
        retain_sprite_subject(&mut icon);
        Some(icon)
    } else {
        None
    };
    premultiply(&mut rgba);
    let mut small =
        image::imageops::resize(&rgba, width, height, image::imageops::FilterType::Lanczos3);
    unpremultiply(&mut small);
    if let Some(mut icon) = clean_icon {
        // Full-sheet filtering can pull the next row back across the icon UV.
        premultiply(&mut icon);
        let mut small_icon = image::imageops::resize(
            &icon,
            width / 4,
            height / 4,
            image::imageops::FilterType::Lanczos3,
        );
        unpremultiply(&mut small_icon);
        image::imageops::replace(&mut small, &small_icon, 0, 0);
    }
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
