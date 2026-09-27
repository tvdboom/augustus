//! Prepare the small embedded panel icons while compiling, not on the click frame.

use std::path::PathBuf;

const PANEL_ICONS: &[(&str, &str)] = &[
    ("images/icons/sestertius.png", "coin"),
    ("images/icons/influence.png", "influence"),
    ("images/icons/manpower.png", "population"),
    ("images/icons/happiness.png", "happiness"),
    ("images/icons/morale.png", "morale"),
    ("images/icons/military_power.png", "military-power"),
    ("images/icons/nobles.png", "nobles"),
    ("images/icons/civilians.png", "civilians"),
    ("images/icons/plebeians.png", "plebeians"),
    ("images/icons/slaves.png", "slaves"),
    ("images/icons/food.png", "food"),
    ("images/icons/metal.png", "metal"),
    ("images/icons/stone.png", "stone"),
    ("images/icons/spy.png", "spy"),
    ("images/icons/trade.png", "trade"),
    ("images/icons/court-nobles.png", "court-nobles"),
    ("images/icons/attack.png", "attack"),
    ("images/ui/spqr-eagle-gold.png", "spqr-eagle-gold"),
    ("images/icons/province.png", "province"),
    ("images/buildings/aqueduct.png", "aqueduct"),
    ("images/buildings/granary.png", "granary"),
    ("images/buildings/forum.png", "forum"),
    ("images/buildings/marketplace.png", "marketplace"),
    ("images/buildings/foundry.png", "foundry"),
    ("images/buildings/academy.png", "academy"),
    ("images/buildings/great-temple.png", "great-temple"),
    ("images/buildings/grand-theater.png", "grand-theater"),
];

fn main() {
    let source_root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("assets");
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
        let mut small =
            image::imageops::resize(&rgba, 64, 64, image::imageops::FilterType::Lanczos3);
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
    }
    // Like the existing embedded panel art, normalize animation sources at build
    // time so opening a map panel never performs a large Lanczos resample.
    let animation_root = PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("animations");
    for (folder, size) in [("military", 768), ("wonders/construction", 512)] {
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
            normalize_animation(&path, &output.join(path.file_name().unwrap()), size);
        }
    }
}

/// Premultiply before filtering and restore straight alpha to avoid dark sprite fringes.
fn normalize_animation(source: &std::path::Path, target: &std::path::Path, size: u32) {
    let mut rgba = image::open(source).expect("animation PNG must be valid").to_rgba8();
    for pixel in rgba.pixels_mut() {
        let alpha = u16::from(pixel[3]);
        for channel in &mut pixel.0[..3] {
            *channel = ((u16::from(*channel) * alpha + 127) / 255) as u8;
        }
    }
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
    small.save(target).expect("write optimized animation PNG");
}
