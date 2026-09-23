//! Named Roman regions over the Augustus geographic map.

use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use serde::Deserialize;

use crate::app::map_hud_contains;

const LONGITUDE_SCALE: f32 = 0.766; // Equirectangular scale at roughly 40° north.
const MIN_ZOOM: f32 = 0.9;
const MAX_ZOOM: f32 = 8.0;
const LABEL_ZOOM_STEP: f32 = 0.25;
const LABEL_ZOOM_LEVELS: usize = ((MAX_ZOOM - MIN_ZOOM) / LABEL_ZOOM_STEP) as usize + 2;
const ZOOM_RESPONSE_RATE: f32 = 18.0;
const CITY_BLEND_START: f32 = 1.35;
const CITY_BLEND_END: f32 = 2.15;
const PAN_MARGIN_SCREEN_FRACTION: f32 = 0.025;
const PAN_OVERSCROLL_SCREEN_FRACTION: f32 = 0.02;
const BOUNDS_RETURN_RATE: f32 = 14.0;
const DEEP_SEA: egui::Color32 = egui::Color32::from_rgb(37, 76, 112);
const CLOSE_SEA: egui::Color32 = egui::Color32::from_rgb(49, 105, 139);
const LAND: egui::Color32 = egui::Color32::from_rgb(189, 192, 171);
const BORDER: egui::Color32 = egui::Color32::from_rgb(145, 106, 82);
const INK: egui::Color32 = egui::Color32::from_rgb(51, 40, 33);
const LABEL_HORIZONTAL_MARGIN: f32 = 0.18;

#[derive(Resource)]
/// Camera position and hovered region on the local map.
pub(crate) struct MapView {
    zoom: f32,
    target_zoom: f32,
    zoom_anchor: egui::Pos2,
    pan: Vec2,
    hovered: Option<usize>,
    atlas_ready: bool,
    wonder_textures: Vec<egui::TextureHandle>,
    city_textures: Vec<egui::TextureHandle>,
    environment_textures: Vec<egui::TextureHandle>,
    wave_offset_a: [f32; 2],
    wave_offset_b: [f32; 2],
    ripple_offset_a: [f32; 2],
    ripple_offset_b: [f32; 2],
    cloud_offset: [f32; 2],
    label_fit: f32,
    label_candidates: Vec<Vec<Vec<LabelPlacement>>>,
    label_anchors: Vec<Option<LabelPlacement>>,
    anchor_candidates: Vec<Vec<LabelPlacement>>,
    anchor_level: Option<usize>,
}

impl Default for MapView {
    fn default() -> Self {
        Self {
            zoom: MIN_ZOOM,
            target_zoom: MIN_ZOOM,
            zoom_anchor: egui::Pos2::ZERO,
            pan: Vec2::ZERO,
            hovered: None,
            atlas_ready: false,
            wonder_textures: Vec::new(),
            city_textures: Vec::new(),
            environment_textures: Vec::new(),
            wave_offset_a: [0.0; 2],
            wave_offset_b: [0.0; 2],
            ripple_offset_a: [0.0; 2],
            ripple_offset_b: [0.0; 2],
            cloud_offset: [0.0; 2],
            label_fit: 0.0,
            label_candidates: Vec::new(),
            label_anchors: Vec::new(),
            anchor_candidates: Vec::new(),
            anchor_level: None,
        }
    }
}

#[derive(Deserialize)]
struct MapAtlas {
    provinces: Vec<Province>,
    land: Vec<MapMesh>,
    marker_land: Vec<MapMesh>,
}

#[derive(Clone)]
struct LabelPlacement {
    center: [f32; 2],
    angle: f32,
    font_size: f32,
    full_name: bool,
}

#[derive(Deserialize)]
struct Province {
    name: String,
    short: String,
    label: [f32; 2],
    bounds: [f32; 4],
    parts: Vec<MapMesh>,
    #[serde(skip)]
    visual_center: [f32; 2],
}

#[derive(Deserialize)]
struct MapMesh {
    /// Longitude and latitude pairs.
    v: Vec<[f32; 2]>,
    /// Exclusive end index of each ring; the first ring is the exterior.
    r: Vec<usize>,
    /// Earcut triangle indices.
    t: Vec<u32>,
    #[serde(skip)]
    bounds: [f32; 4],
}

struct WonderAsset {
    name: &'static str,
    position: [f32; 2],
    png: &'static [u8],
}

struct CityAsset {
    position: [f32; 2],
    // Where the site falls within the image; coastal scenes extend inland.
    hotspot: [f32; 2],
}

// Ancient names at present-day sites. Lutetia is included for the requested Paris location.
const CITIES: [CityAsset; 8] = [
    CityAsset {
        // Rome
        position: [12.50, 41.90],
        hotspot: [0.5, 0.5],
    },
    CityAsset {
        // Lutetia (present-day Paris)
        position: [2.35, 48.86],
        hotspot: [0.5, 0.5],
    },
    CityAsset {
        // Tarraco
        position: [1.24, 41.12],
        hotspot: [0.5, 0.85],
    },
    CityAsset {
        // Carthage
        position: [10.33, 36.85],
        hotspot: [0.5, 0.15],
    },
    CityAsset {
        // Athens
        position: [23.73, 37.98],
        hotspot: [0.8, 0.9],
    },
    CityAsset {
        // Alexandria
        position: [29.92, 31.20],
        hotspot: [0.5, 0.15],
    },
    CityAsset {
        // Ephesus
        position: [27.36, 37.95],
        hotspot: [0.2, 0.5],
    },
    CityAsset {
        // Antioch
        position: [36.16, 36.20],
        hotspot: [0.3, 0.5],
    },
];

const CITY_IMAGES: [(&str, &[u8]); 2] = [
    ("City icon", include_bytes!("../../assets/images/cities/city_icon.png")),
    ("City illustration", include_bytes!("../../assets/images/cities/city.png")),
];

const ENVIRONMENT_IMAGES: [(&str, &[u8]); 4] = [
    ("Sea waves", include_bytes!("../../assets/images/map/waves.png")),
    ("Moving clouds", include_bytes!("../../assets/images/map/clouds.png")),
    ("Coastal gradient", include_bytes!("../../assets/images/map/coastal-gradient.png")),
    ("Close sea ripples", include_bytes!("../../assets/images/map/ripples.png")),
];
const WAVE_TEXTURE: usize = 0;
const CLOUD_TEXTURE: usize = 1;
const COAST_TEXTURE: usize = 2;
const RIPPLE_TEXTURE: usize = 3;
const RIPPLE_TILE_A: [f32; 2] = [0.9, 0.69];
const RIPPLE_TILE_B: [f32; 2] = [1.4, 1.07];

const WONDERS: [WonderAsset; 20] = [
    WonderAsset {
        name: "Great Pyramid of Giza",
        position: [31.13, 29.98],
        png: include_bytes!("../../assets/images/wonders/great_pyramid.png"),
    },
    WonderAsset {
        name: "Temple of Jupiter Optimus Maximus",
        position: [12.49, 41.89],
        png: include_bytes!("../../assets/images/wonders/jupiter_temple.png"),
    },
    WonderAsset {
        name: "Oracle of Dodona",
        position: [20.78, 39.55],
        png: include_bytes!("../../assets/images/wonders/oracle_dodona.png"),
    },
    WonderAsset {
        name: "Sigiriya",
        position: [80.76, 7.95],
        png: include_bytes!("../../assets/images/wonders/sigiriya.png"),
    },
    WonderAsset {
        name: "Stonehenge",
        position: [-1.83, 51.18],
        png: include_bytes!("../../assets/images/wonders/stonehenge.png"),
    },
    WonderAsset {
        name: "Hanging Gardens of Babylon",
        position: [44.42, 32.54],
        png: include_bytes!("../../assets/images/wonders/babylon_gardens.png"),
    },
    WonderAsset {
        name: "Acropolis of Pergamon",
        position: [27.18, 39.13],
        png: include_bytes!("../../assets/images/wonders/pergamon_acropolis.png"),
    },
    WonderAsset {
        name: "Library of Alexandria",
        position: [29.92, 31.20],
        png: include_bytes!("../../assets/images/wonders/alexandria_library.png"),
    },
    WonderAsset {
        name: "Temple of Artemis",
        position: [27.36, 37.95],
        png: include_bytes!("../../assets/images/wonders/artemis_temple.png"),
    },
    WonderAsset {
        name: "Temple of Zeus at Olympia",
        position: [21.63, 37.64],
        png: include_bytes!("../../assets/images/wonders/zeus_temple.png"),
    },
    WonderAsset {
        name: "Palace of the Argeads",
        position: [21.70, 40.44],
        png: include_bytes!("../../assets/images/wonders/argeads_palace.png"),
    },
    WonderAsset {
        name: "Ay Khanum",
        position: [69.42, 37.17],
        png: include_bytes!("../../assets/images/wonders/ay_khanum.png"),
    },
    WonderAsset {
        name: "Mausoleum at Halicarnassus",
        position: [27.42, 37.04],
        png: include_bytes!("../../assets/images/wonders/mausoleum_halicar.png"),
    },
    WonderAsset {
        name: "Taxila",
        position: [72.88, 33.75],
        png: include_bytes!("../../assets/images/wonders/taxila.png"),
    },
    WonderAsset {
        name: "Acropolis of Rhodes",
        position: [28.12, 36.34],
        png: include_bytes!("../../assets/images/wonders/rhodes_acropolis.png"),
    },
    WonderAsset {
        name: "Lighthouse of Alexandria",
        position: [29.92, 31.20],
        png: include_bytes!("../../assets/images/wonders/alexandria_lighthouse.png"),
    },
    WonderAsset {
        name: "Colossus of Rhodes",
        // The coast atlas ends just short of Rhodes's ancient harbour. Keep
        // this marker on the island's northern end at every zoom level.
        position: [28.12, 36.34],
        png: include_bytes!("../../assets/images/wonders/rhodes_colossus.png"),
    },
    WonderAsset {
        name: "Colosseum",
        position: [12.492, 41.890],
        png: include_bytes!("../../assets/images/wonders/colosseum.png"),
    },
    WonderAsset {
        name: "Aqueduct of Segovia",
        position: [-4.117, 40.948],
        png: include_bytes!("../../assets/images/wonders/segovia_aqueduct.png"),
    },
    WonderAsset {
        name: "Pont du Gard",
        position: [4.535, 43.948],
        png: include_bytes!("../../assets/images/wonders/pont_du_gard.png"),
    },
];

pub(crate) struct MapLoadProgress {
    pub completed: usize,
    pub total: usize,
    pub item: &'static str,
}

impl MapView {
    pub(crate) fn is_loaded(&self) -> bool {
        self.atlas_ready
            && self.wonder_textures.len() == WONDERS.len()
            && self.city_textures.len() == CITY_IMAGES.len()
            && self.environment_textures.len() == ENVIRONMENT_IMAGES.len()
            && self.label_candidates.len() == LABEL_ZOOM_LEVELS
    }

    pub(crate) fn load_progress(&self) -> MapLoadProgress {
        let completed = usize::from(self.atlas_ready)
            + self.wonder_textures.len()
            + self.city_textures.len()
            + self.environment_textures.len()
            + self.label_candidates.len();
        let item = if !self.atlas_ready {
            "Historical province map"
        } else if let Some(wonder) = WONDERS.get(self.wonder_textures.len()) {
            wonder.name
        } else if let Some((name, _)) = CITY_IMAGES.get(self.city_textures.len()) {
            name
        } else if let Some((name, _)) = ENVIRONMENT_IMAGES.get(self.environment_textures.len()) {
            name
        } else if self.label_candidates.len() < LABEL_ZOOM_LEVELS {
            "Province labels"
        } else {
            "Map ready"
        };
        MapLoadProgress {
            completed,
            total: WONDERS.len()
                + CITY_IMAGES.len()
                + ENVIRONMENT_IMAGES.len()
                + LABEL_ZOOM_LEVELS
                + 1,
            item,
        }
    }

    pub(crate) fn load_next(&mut self, ctx: &egui::Context) {
        if !self.atlas_ready {
            atlas();
            self.atlas_ready = true;
            return;
        }
        if let Some(wonder) = WONDERS.get(self.wonder_textures.len()) {
            self.wonder_textures.push(load_map_texture(ctx, wonder.name, wonder.png));
        } else if let Some((name, png)) = CITY_IMAGES.get(self.city_textures.len()) {
            let resolution = if self.city_textures.is_empty() {
                64
            } else {
                128
            };
            self.city_textures.push(load_city_texture(ctx, name, png, resolution));
        } else if let Some((name, png)) = ENVIRONMENT_IMAGES.get(self.environment_textures.len()) {
            self.environment_textures.push(load_map_texture(ctx, name, png));
        } else if self.label_candidates.len() < LABEL_ZOOM_LEVELS {
            let atlas = atlas();
            if self.label_candidates.is_empty() {
                self.label_fit = map_geometry(ctx.content_rect(), atlas).3;
            }
            let zoom = label_zoom(self.label_candidates.len());
            let projection = Projection {
                origin: egui::Pos2::ZERO,
                scale: self.label_fit * zoom,
                center: [0.0, 0.0],
            };
            let painter = ctx.layer_painter(egui::LayerId::background());
            self.label_candidates.push(
                atlas
                    .provinces
                    .iter()
                    .map(|province| label_candidates(&painter, province, &projection, zoom))
                    .collect(),
            );
        }
    }
}

fn load_map_texture(ctx: &egui::Context, name: &str, png: &[u8]) -> egui::TextureHandle {
    let rgba = image::load_from_memory(png).expect("map PNG assets must be valid").to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    ctx.load_texture(name, color, egui::TextureOptions::LINEAR)
}

fn load_city_texture(
    ctx: &egui::Context,
    name: &str,
    png: &[u8],
    resolution: u32,
) -> egui::TextureHandle {
    let mut rgba = image::load_from_memory(png).expect("city PNG assets must be valid").to_rgba8();
    // Prefilter the detailed source for small map sizes. Premultiplied alpha
    // keeps the cutout edges clean against both land and sea colors.
    for pixel in rgba.pixels_mut() {
        let alpha = u16::from(pixel[3]);
        for channel in &mut pixel.0[..3] {
            *channel = ((u16::from(*channel) * alpha + 127) / 255) as u8;
        }
    }
    let mut reduced = image::imageops::resize(
        &rgba,
        resolution,
        resolution,
        image::imageops::FilterType::Lanczos3,
    );
    for pixel in reduced.pixels_mut() {
        let alpha = u32::from(pixel[3]);
        for channel in &mut pixel.0[..3] {
            if let Some(value) = (u32::from(*channel) * 255 + alpha / 2).checked_div(alpha) {
                *channel = value.min(255) as u8;
            }
        }
    }
    let size = [resolution as usize, resolution as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, reduced.as_raw());
    ctx.load_texture(name, color, egui::TextureOptions::LINEAR)
}

fn atlas() -> &'static MapAtlas {
    static ATLAS: OnceLock<MapAtlas> = OnceLock::new();
    ATLAS.get_or_init(|| {
        let mut atlas: MapAtlas = serde_json::from_str(include_str!("../../assets/map/atlas.json"))
            .expect("generated map atlas must be valid");
        for part in &mut atlas.land {
            part.cache_bounds();
        }
        for province in &mut atlas.provinces {
            for part in &mut province.parts {
                part.cache_bounds();
            }
            province.visual_center = province_center(province);
        }
        atlas
    })
}

fn province_center(province: &Province) -> [f32; 2] {
    let mut largest = (0.0_f64, province.label);
    for part in &province.parts {
        let mut area = 0.0_f64;
        let mut weighted = [0.0_f64; 2];
        for triangle in part.t.chunks_exact(3) {
            let a = part.v[triangle[0] as usize];
            let b = part.v[triangle[1] as usize];
            let c = part.v[triangle[2] as usize];
            let size = (((b[0] - a[0]) as f64 * (c[1] - a[1]) as f64)
                - ((c[0] - a[0]) as f64 * (b[1] - a[1]) as f64))
                .abs();
            area += size;
            weighted[0] += size * (a[0] as f64 + b[0] as f64 + c[0] as f64) / 3.0;
            weighted[1] += size * (a[1] as f64 + b[1] as f64 + c[1] as f64) / 3.0;
        }
        if area > largest.0 {
            largest = (area, [(weighted[0] / area) as f32, (weighted[1] / area) as f32]);
        }
    }
    largest.1
}

struct Projection {
    origin: egui::Pos2,
    scale: f32,
    center: [f32; 2],
}

impl Projection {
    fn point(&self, [lon, lat]: [f32; 2]) -> egui::Pos2 {
        self.origin
            + egui::vec2(
                (lon - self.center[0]) * LONGITUDE_SCALE * self.scale,
                (self.center[1] - lat) * self.scale,
            )
    }

    fn inverse(&self, point: egui::Pos2) -> [f32; 2] {
        [
            self.center[0] + (point.x - self.origin.x) / (LONGITUDE_SCALE * self.scale),
            self.center[1] - (point.y - self.origin.y) / self.scale,
        ]
    }

    fn bounds_rect(&self, [west, south, east, north]: [f32; 4]) -> egui::Rect {
        egui::Rect::from_min_max(self.point([west, north]), self.point([east, south]))
    }
}

fn settle_position(position: Vec2, target: Vec2, dt: f32) -> Vec2 {
    if position.distance_squared(target) < 0.01 {
        target
    } else {
        let fraction = 1.0 - (-BOUNDS_RETURN_RATE * dt.max(0.0)).exp();
        position.lerp(target, fraction.clamp(0.0, 1.0))
    }
}

fn map_geometry(rect: egui::Rect, atlas: &MapAtlas) -> ([f32; 2], f32, f32, f32) {
    let bounds = atlas.provinces.iter().fold(
        [f32::MAX, f32::MAX, f32::MIN, f32::MIN],
        |mut all, province| {
            all[0] = all[0].min(province.bounds[0]);
            all[1] = all[1].min(province.bounds[1]);
            all[2] = all[2].max(province.bounds[2]);
            all[3] = all[3].max(province.bounds[3]);
            all
        },
    );
    let center = [(bounds[0] + bounds[2]) * 0.5, (bounds[1] + bounds[3]) * 0.5];
    let width = (bounds[2] - bounds[0]) * LONGITUDE_SCALE;
    let height = bounds[3] - bounds[1];
    let fit = (rect.width() / (width + 8.0)).min(rect.height() / (height + 5.0));
    (center, width, height, fit)
}

fn label_zoom(level: usize) -> f32 {
    (MIN_ZOOM + level as f32 * LABEL_ZOOM_STEP).min(MAX_ZOOM)
}

fn label_level(zoom: f32, fit: f32, prepared_fit: f32) -> usize {
    let effective_zoom = zoom.min(zoom * fit / prepared_fit);
    if effective_zoom >= MAX_ZOOM {
        return LABEL_ZOOM_LEVELS - 1;
    }
    (((effective_zoom - MIN_ZOOM) / LABEL_ZOOM_STEP).floor().max(0.0) as usize)
        .min(LABEL_ZOOM_LEVELS - 1)
}

/// The map fills the screen. Escape opens the in-game menu through the app's shared handler.
pub(crate) fn draw_map(
    mut contexts: EguiContexts,
    mut view: ResMut<MapView>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    state: Res<State<crate::app::AppState>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    while !view.is_loaded() {
        view.load_next(ctx);
    }
    let rect = ctx.content_rect();
    let interactions_enabled = *state.get() == crate::app::AppState::Map;
    egui::Area::new(egui::Id::new("augustus_province_map"))
        .order(egui::Order::Background)
        .fixed_pos(rect.min)
        .show(ctx, |ui| {
            ui.set_min_size(rect.size());
            ui.set_max_size(rect.size());
            let sense = if interactions_enabled {
                egui::Sense::drag()
            } else {
                egui::Sense::hover()
            };
            let (map_rect, response) = ui.allocate_exact_size(rect.size(), sense);
            let painter = ui.painter_at(map_rect);
            paint_map(
                &painter,
                map_rect,
                &mut view,
                &response,
                &keyboard,
                time.delta_secs(),
                interactions_enabled,
            );
            response
        });
    ctx.request_repaint_after(std::time::Duration::from_millis(33));
}

fn paint_map(
    painter: &egui::Painter,
    rect: egui::Rect,
    view: &mut MapView,
    response: &egui::Response,
    keyboard: &ButtonInput<KeyCode>,
    dt: f32,
    interactions_enabled: bool,
) {
    let atlas = atlas();
    let (center, width, height, fit) = map_geometry(rect, atlas);

    let (pointer_over_menu, drag_started_on_menu) = painter.ctx().input(|input| {
        (
            input.pointer.hover_pos().is_some_and(|point| map_hud_contains(rect, point)),
            input.pointer.press_origin().is_some_and(|point| map_hud_contains(rect, point)),
        )
    });

    if interactions_enabled && !pointer_over_menu {
        if let Some(cursor) = response.hover_pos() {
            let scroll = painter.ctx().input(|input| input.smooth_scroll_delta.y);
            if scroll.abs() > 0.0 {
                view.target_zoom =
                    (view.target_zoom * (scroll * 0.0022).exp()).clamp(MIN_ZOOM, MAX_ZOOM);
                view.zoom_anchor = cursor;
            }
        }
    }
    if (view.target_zoom - view.zoom).abs() > 0.0001 {
        painter.ctx().request_repaint();
        let fraction = 1.0 - (-ZOOM_RESPONSE_RATE * dt.max(0.0)).exp();
        let next_zoom = view.zoom + (view.target_zoom - view.zoom) * fraction.clamp(0.0, 1.0);
        let next_zoom = if (view.target_zoom - next_zoom).abs() < 0.001 {
            view.target_zoom
        } else {
            next_zoom
        };
        let old_origin = rect.center() + egui::vec2(view.pan.x, view.pan.y);
        let new_origin =
            view.zoom_anchor - (view.zoom_anchor - old_origin) * (next_zoom / view.zoom);
        let pan = new_origin - rect.center();
        view.pan = Vec2::new(pan.x, pan.y);
        view.zoom = next_zoom;
    }
    let dragging = interactions_enabled
        && !drag_started_on_menu
        && response.dragged_by(egui::PointerButton::Primary);
    let grabbing = dragging
        || (interactions_enabled
            && !drag_started_on_menu
            && response.hovered()
            && painter.ctx().input(|input| input.pointer.primary_down()));
    if dragging {
        let delta = painter.ctx().input(|input| input.pointer.delta());
        view.pan += Vec2::new(delta.x, delta.y);
    }
    let speed = 600.0 * dt.clamp(0.0, 0.1);
    if interactions_enabled
        && (keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft))
    {
        view.pan.x += speed;
    }
    if interactions_enabled
        && (keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight))
    {
        view.pan.x -= speed;
    }
    if interactions_enabled
        && (keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp))
    {
        view.pan.y += speed;
    }
    if interactions_enabled
        && (keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown))
    {
        view.pan.y -= speed;
    }
    let scale = fit * view.zoom;
    let max_pan = Vec2::new(
        ((width * scale - rect.width()) * 0.5).max(0.0) + rect.width() * PAN_MARGIN_SCREEN_FRACTION,
        ((height * scale - rect.height()) * 0.5).max(0.0)
            + rect.height() * PAN_MARGIN_SCREEN_FRACTION,
    );
    let target_pan = view.pan.clamp(-max_pan, max_pan);
    let overscroll = Vec2::new(rect.width(), rect.height()) * PAN_OVERSCROLL_SCREEN_FRACTION;
    let limited = target_pan + (view.pan - target_pan).clamp(-overscroll, overscroll);
    view.pan = if !interactions_enabled {
        view.pan
    } else if grabbing {
        limited
    } else {
        settle_position(limited, target_pan, dt)
    };
    let projection = Projection {
        origin: rect.center() + egui::vec2(view.pan.x, view.pan.y),
        scale,
        center,
    };
    advance_texture_offset(&mut view.wave_offset_a, [1.2, 0.4], [20.0, 15.32], scale, dt);
    advance_texture_offset(&mut view.wave_offset_b, [-0.8, -0.2], [20.0, 15.32], scale, dt);
    advance_texture_offset(&mut view.ripple_offset_a, [2.4, 0.7], RIPPLE_TILE_A, scale, dt);
    advance_texture_offset(&mut view.ripple_offset_b, [-1.6, 0.4], RIPPLE_TILE_B, scale, dt);
    advance_texture_offset(&mut view.cloud_offset, [4.5, 0.7], [84.0, 32.0], scale, dt);
    let wonder_markers = layout_wonders(&projection, rect, view.zoom);
    let city_markers = layout_cities(&projection, rect, view.zoom);

    let close = smoothstep((view.zoom - 1.0) / 2.5);
    painter.rect_filled(rect, 0.0, blend_color(DEEP_SEA, CLOSE_SEA, close));
    if let Some(waves) = view.environment_textures.get(WAVE_TEXTURE) {
        let opacity = (105.0 + close * 20.0) as u8;
        paint_scrolling_texture(
            painter,
            rect,
            &projection,
            waves,
            [20.0, 15.32],
            view.wave_offset_a,
            egui::Color32::from_white_alpha(opacity),
        );
        paint_scrolling_texture(
            painter,
            rect,
            &projection,
            waves,
            [20.0, 15.32],
            view.wave_offset_b,
            egui::Color32::from_white_alpha((opacity as f32 * 0.42) as u8),
        );
    }
    if let Some(ripples) = view.environment_textures.get(RIPPLE_TEXTURE) {
        let opacity = (smoothstep((view.zoom - 1.6) / 2.0) * 210.0) as u8;
        if opacity > 3 {
            paint_scrolling_texture(
                painter,
                rect,
                &projection,
                ripples,
                RIPPLE_TILE_A,
                view.ripple_offset_a,
                egui::Color32::from_white_alpha(opacity),
            );
            paint_scrolling_texture(
                painter,
                rect,
                &projection,
                ripples,
                RIPPLE_TILE_B,
                view.ripple_offset_b,
                egui::Color32::from_white_alpha((opacity as f32 * 0.35) as u8),
            );
        }
    }
    if let Some(coast) = view.environment_textures.get(COAST_TEXTURE) {
        let gradient_rect = egui::Rect::from_min_max(
            projection.point([-18.0, 61.0]),
            projection.point([52.0, 17.0]),
        );
        painter.image(
            coast.id(),
            gradient_rect,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            egui::Color32::from_white_alpha((100.0 + close * 155.0) as u8),
        );
    }
    paint_meshes(painter, &atlas.land, &projection, LAND);

    let over_city = response
        .hover_pos()
        .is_some_and(|point| city_markers.iter().any(|marker| marker.bounds.contains(point)));
    let pointer = if interactions_enabled && !over_city && !pointer_over_menu {
        response.hover_pos().map(|position| projection.inverse(position))
    } else {
        None
    };
    view.hovered = pointer
        .and_then(|point| atlas.provinces.iter().position(|province| province.contains(point)));
    painter.ctx().set_cursor_icon(if dragging {
        egui::CursorIcon::Grabbing
    } else if pointer_over_menu {
        egui::CursorIcon::Default
    } else if view.hovered.is_some() {
        egui::CursorIcon::PointingHand
    } else {
        egui::CursorIcon::Default
    });
    for (index, province) in atlas.provinces.iter().enumerate() {
        if !projection.bounds_rect(province.bounds).intersects(rect) {
            continue;
        }
        let hovered = view.hovered == Some(index);
        let color = if hovered {
            egui::Color32::from_rgb(222, 174, 105)
        } else {
            province_color(index)
        };
        paint_meshes(painter, &province.parts, &projection, color);
        for part in &province.parts {
            paint_rings(
                painter,
                part,
                &projection,
                egui::Stroke::new(
                    if hovered {
                        1.0
                    } else {
                        0.45
                    },
                    BORDER,
                ),
            );
        }
    }

    if let Some(clouds) = view.environment_textures.get(CLOUD_TEXTURE) {
        let opacity = (45.0 + close * 63.0) as u8;
        paint_scrolling_texture(
            painter,
            rect,
            &projection,
            clouds,
            [84.0, 32.0],
            view.cloud_offset,
            egui::Color32::from_white_alpha(opacity),
        );
    }

    paint_wonders(painter, &wonder_markers, view.zoom, &view.wonder_textures);
    paint_cities(painter, &city_markers, view.zoom, &view.city_textures);
    let mut occupied =
        Vec::with_capacity(wonder_markers.len() + city_markers.len() + atlas.provinces.len());
    occupied.extend(wonder_markers.iter().map(|marker| marker.image.expand(1.0)));
    occupied.extend(city_markers.iter().map(|marker| marker.bounds));

    // Keep the first successful geographic position and angle. Refit its text
    // at each prepared zoom level, with other positions available only if a
    // marker blocks it or its readable sizes no longer fit the province.
    let label_level = label_level(view.zoom, fit, view.label_fit);
    if view.label_anchors.len() != atlas.provinces.len() {
        view.label_anchors = vec![None; atlas.provinces.len()];
        view.anchor_level = None;
    }
    if view.anchor_level != Some(label_level) {
        let prepared_projection = Projection {
            origin: egui::Pos2::ZERO,
            scale: view.label_fit * label_zoom(label_level),
            center: [0.0, 0.0],
        };
        view.anchor_candidates = atlas
            .provinces
            .iter()
            .zip(&view.label_anchors)
            .map(|(province, anchor)| {
                anchor.as_ref().map_or_else(Vec::new, |anchor| {
                    anchored_label_candidates(
                        painter,
                        province,
                        &prepared_projection,
                        label_zoom(label_level),
                        anchor,
                    )
                })
            })
            .collect();
        view.anchor_level = Some(label_level);
    }
    for index in
        (0..atlas.provinces.len()).filter(|&index| view.hovered != Some(index)).chain(view.hovered)
    {
        let province = &atlas.provinces[index];
        let anchored = &view.anchor_candidates[index];
        let regular = &view.label_candidates[label_level][index];
        if let Some(anchor) = anchored.first() {
            let name = if anchor.full_name {
                &province.name
            } else {
                &province.short
            };
            let galley = painter.layout_no_wrap(
                name.clone(),
                egui::FontId::proportional(anchor.font_size),
                INK,
            );
            let bounds =
                rotated_bounds(projection.point(anchor.center), galley.size(), anchor.angle)
                    .expand(2.0);
            if !rect.contains_rect(bounds) {
                continue;
            }
        }
        let mut selected = None;
        for candidate in ordered_label_candidates(anchored, regular) {
            let name = if candidate.full_name {
                &province.name
            } else {
                &province.short
            };
            let galley = painter.layout_no_wrap(
                name.clone(),
                egui::FontId::proportional(candidate.font_size),
                INK,
            );
            let center = projection.point(candidate.center);
            let label_rect = rotated_bounds(center, galley.size(), candidate.angle).expand(2.0);
            if occupied.iter().any(|other| other.intersects(label_rect)) {
                continue;
            }
            // The viewport may hide a label, but must not make it relocate.
            if !rect.contains_rect(label_rect) {
                break;
            }
            selected = Some((candidate.clone(), galley, center, label_rect));
            break;
        }
        if let Some((candidate, galley, center, label_rect)) = selected {
            let origin = center - galley.size() * 0.5;
            let shape = egui::epaint::TextShape::new(origin, galley, INK)
                .with_angle_and_anchor(candidate.angle, egui::Align2::CENTER_CENTER);
            painter.add(egui::Shape::Text(shape));
            occupied.push(label_rect);
            if view.label_anchors[index].is_none() {
                view.label_anchors[index] = Some(candidate);
                view.anchor_level = None;
            }
        }
    }
}

fn rotated_bounds(center: egui::Pos2, size: egui::Vec2, angle: f32) -> egui::Rect {
    let (sin, cos) = angle.sin_cos();
    let half = egui::vec2(
        (size.x * cos.abs() + size.y * sin.abs()) * 0.5,
        (size.x * sin.abs() + size.y * cos.abs()) * 0.5,
    );
    egui::Rect::from_min_max(center - half, center + half)
}

fn label_candidates(
    painter: &egui::Painter,
    province: &Province,
    projection: &Projection,
    zoom: f32,
) -> Vec<LabelPlacement> {
    let mut centers = Vec::new();
    if province.contains(province.visual_center) {
        centers.push(province.visual_center);
    }
    if province.contains(province.label) && province.label != province.visual_center {
        centers.push(province.label);
    }
    for row in 0..5 {
        for column in 0..5 {
            let center = [
                province.bounds[0]
                    + (province.bounds[2] - province.bounds[0]) * (column as f32 + 0.5) / 5.0,
                province.bounds[1]
                    + (province.bounds[3] - province.bounds[1]) * (row as f32 + 0.5) / 5.0,
            ];
            if province.contains(center) {
                centers.push(center);
            }
        }
    }
    centers.sort_by(|a, b| {
        let distance = |point: &[f32; 2]| {
            let dx = (point[0] - province.visual_center[0]) * LONGITUDE_SCALE;
            let dy = point[1] - province.visual_center[1];
            dx * dx + dy * dy
        };
        distance(a).total_cmp(&distance(b))
    });

    let angles = [
        0.0,
        0.35,
        -0.35,
        0.7,
        -0.7,
        1.05,
        -1.05,
        std::f32::consts::FRAC_PI_2,
        -std::f32::consts::FRAC_PI_2,
    ];
    let preferred_size = (10.5 * zoom.sqrt()).clamp(9.0, 18.0);
    let mut choices = Vec::new();
    let names = if province.name == province.short {
        vec![true]
    } else {
        vec![true, false]
    };
    'sizes: for step in 0..12 {
        let font_size = preferred_size - step as f32 * 1.25;
        if font_size < 6.5 {
            break;
        }
        for &full_name in &names {
            let name = if full_name {
                &province.name
            } else {
                &province.short
            };
            let galley =
                painter.layout_no_wrap(name.clone(), egui::FontId::proportional(font_size), INK);
            let size = label_fit_size(galley.size());
            for &center in &centers {
                for &angle in &angles {
                    if label_fits_province(province, projection, center, size, angle) {
                        choices.push(LabelPlacement {
                            center,
                            angle,
                            font_size,
                            full_name,
                        });
                        if choices.len() >= 16 {
                            break 'sizes;
                        }
                        break;
                    }
                }
            }
        }
    }
    // A slight size reduction is preferable to pushing a name to the edge of
    // its province. Keep widely smaller options behind all readable ones.
    let steps_per_band = if preferred_size >= 13.0 {
        3.0
    } else {
        2.0
    };
    let band = |candidate: &LabelPlacement| {
        ((preferred_size - candidate.font_size) / (1.25 * steps_per_band)).floor() as i32
    };
    let center_distance = |candidate: &LabelPlacement| {
        let dx = (candidate.center[0] - province.visual_center[0]) * LONGITUDE_SCALE;
        let dy = candidate.center[1] - province.visual_center[1];
        dx * dx + dy * dy
    };
    choices.sort_by(|a, b| {
        band(a)
            .cmp(&band(b))
            .then_with(|| center_distance(a).total_cmp(&center_distance(b)))
            .then_with(|| b.full_name.cmp(&a.full_name))
            .then_with(|| b.font_size.total_cmp(&a.font_size))
            .then_with(|| a.angle.abs().total_cmp(&b.angle.abs()))
    });
    choices
}

fn anchored_label_candidates(
    painter: &egui::Painter,
    province: &Province,
    projection: &Projection,
    zoom: f32,
    anchor: &LabelPlacement,
) -> Vec<LabelPlacement> {
    let preferred_size = (10.5 * zoom.sqrt()).clamp(9.0, 18.0);
    let mut choices = Vec::new();
    for step in 0..12 {
        let font_size = preferred_size - step as f32 * 1.25;
        if font_size < 6.5 {
            break;
        }
        for full_name in [true, false] {
            if !full_name && province.name == province.short {
                continue;
            }
            let name = if full_name {
                &province.name
            } else {
                &province.short
            };
            let galley =
                painter.layout_no_wrap(name.clone(), egui::FontId::proportional(font_size), INK);
            if label_fits_province(
                province,
                projection,
                anchor.center,
                label_fit_size(galley.size()),
                anchor.angle,
            ) {
                choices.push(LabelPlacement {
                    center: anchor.center,
                    angle: anchor.angle,
                    font_size,
                    full_name,
                });
            }
        }
    }
    choices
}

fn ordered_label_candidates<'a>(
    anchored: &'a [LabelPlacement],
    regular: &'a [LabelPlacement],
) -> impl Iterator<Item = &'a LabelPlacement> {
    let readable_floor =
        regular.iter().map(|candidate| candidate.font_size).fold(6.5_f32, f32::max) - 2.5;
    anchored
        .iter()
        .filter(move |candidate| candidate.font_size >= readable_floor)
        .chain(regular)
        .chain(anchored.iter().filter(move |candidate| candidate.font_size < readable_floor))
}

fn label_fit_size(text_size: egui::Vec2) -> egui::Vec2 {
    text_size + egui::vec2((text_size.x * LABEL_HORIZONTAL_MARGIN).max(8.0), 6.0)
}

fn label_fits_province(
    province: &Province,
    projection: &Projection,
    center: [f32; 2],
    size: egui::Vec2,
    angle: f32,
) -> bool {
    let (sin, cos) = angle.sin_cos();
    let screen_center = projection.point(center);
    let screen_bounds = rotated_bounds(screen_center, size, angle);
    let lower_left = projection.inverse(screen_bounds.left_bottom());
    let upper_right = projection.inverse(screen_bounds.right_top());
    if lower_left[0] < province.bounds[0]
        || lower_left[1] < province.bounds[1]
        || upper_right[0] > province.bounds[2]
        || upper_right[1] > province.bounds[3]
    {
        return false;
    }
    let columns = ((size.x / 6.0).ceil() as usize).clamp(4, 32);
    for column in 0..=columns {
        let x = size.x * (column as f32 / columns as f32 - 0.5);
        for row in 0..=2 {
            let y = size.y * (row as f32 * 0.5 - 0.5);
            let point = screen_center + egui::vec2(x * cos - y * sin, x * sin + y * cos);
            if !province.contains(projection.inverse(point)) {
                return false;
            }
        }
    }
    true
}

struct CityMarker {
    icon: egui::Rect,
    image: egui::Rect,
    bounds: egui::Rect,
}

fn city_blend(zoom: f32) -> f32 {
    let t = ((zoom - CITY_BLEND_START) / (CITY_BLEND_END - CITY_BLEND_START)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn city_rect(anchor: egui::Pos2, size: f32, hotspot: [f32; 2]) -> egui::Rect {
    egui::Rect::from_min_size(
        anchor - egui::vec2(size * hotspot[0], size * hotspot[1]),
        egui::vec2(size, size),
    )
}

fn layout_cities(projection: &Projection, map_rect: egui::Rect, zoom: f32) -> Vec<CityMarker> {
    let blend = city_blend(zoom);
    let icon_size = 24.0 + 1.2 * (zoom - MIN_ZOOM).max(0.0);
    let image_size = (30.0 + 8.0 * (zoom - 2.0)).max(24.0);
    CITIES
        .iter()
        .filter_map(|city| {
            let anchor = projection.point(city.position);
            let icon = city_rect(anchor, icon_size, city.hotspot);
            let image = city_rect(anchor, image_size, city.hotspot);
            let bounds = if blend <= 0.0 {
                icon
            } else if blend >= 1.0 {
                image
            } else {
                icon.union(image)
            }
            .expand(2.0);
            map_rect.intersects(bounds).then_some(CityMarker {
                icon,
                image,
                bounds,
            })
        })
        .collect()
}

fn paint_cities(
    painter: &egui::Painter,
    markers: &[CityMarker],
    zoom: f32,
    textures: &[egui::TextureHandle],
) {
    let (Some(icon), Some(image)) = (textures.first(), textures.get(1)) else {
        return;
    };
    let blend = city_blend(zoom);
    let icon_tint = egui::Color32::from_white_alpha(((1.0 - blend) * 255.0).round() as u8);
    let image_tint = egui::Color32::from_white_alpha((blend * 255.0).round() as u8);
    for marker in markers {
        if blend < 1.0 {
            painter.image(
                icon.id(),
                marker.icon,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                icon_tint,
            );
        }
        if blend > 0.0 {
            painter.image(
                image.id(),
                marker.image,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                image_tint,
            );
        }
    }
}

struct WonderMarker {
    index: usize,
    image: egui::Rect,
}

fn layout_wonders(projection: &Projection, map_rect: egui::Rect, zoom: f32) -> Vec<WonderMarker> {
    let fade = ((zoom - 1.15) / 0.8).clamp(0.0, 1.0);
    let opacity = fade * fade * (3.0 - 2.0 * fade);
    if opacity <= 0.001 {
        return Vec::new();
    }

    let mut markers: Vec<WonderMarker> = Vec::with_capacity(WONDERS.len());

    // The Colosseum shares Rome with Jupiter's temple; the Colossus shares
    // Rhodes with its Acropolis. Keep the iconic monument at each site.
    for index in
        [17, 16].into_iter().chain((0..WONDERS.len()).filter(|index| *index != 17 && *index != 16))
    {
        let wonder = &WONDERS[index];
        let anchor = projection.point(wonder.position);
        if !map_rect.contains(anchor) {
            continue;
        }
        let atlas = atlas();
        // A historical site can sit on a province boundary (Rome does in this
        // atlas). The coastline, rather than the internal border, limits icons.
        // Small islands use their full source outline only for marker fitting;
        // the visible backdrop is trimmed to the province coastline.
        let land_part =
            atlas.marker_land.iter().chain(&atlas.land).find(|part| part.contains(wonder.position));
        let on_land = |point| land_part.is_some_and(|part| part.contains(point));
        if !on_land(wonder.position) {
            continue;
        }
        // Keep a stable size in map coordinates: screen size grows with zoom.
        // The territory check below can still make a coastal marker smaller.
        let desired_size = 13.0 * zoom;
        let mut chosen = None;
        for size in (4..=desired_size.round() as usize).rev() {
            let size = size as f32;
            let image_rect = egui::Rect::from_center_size(anchor, egui::vec2(size, size));
            if !map_rect.contains_rect(image_rect)
                || markers.iter().any(|other| other.image.intersects(image_rect.expand(1.0)))
            {
                continue;
            }
            let fits = [0.0, 0.5, 1.0].into_iter().all(|fraction_x| {
                [0.0, 0.5, 1.0].into_iter().all(|fraction_y| {
                    let point = egui::pos2(
                        image_rect.left() + size * fraction_x,
                        image_rect.top() + size * fraction_y,
                    );
                    on_land(projection.inverse(point))
                })
            });
            if fits {
                chosen = Some(image_rect);
                break;
            }
        }
        let Some(image_rect) = chosen else {
            continue;
        };
        markers.push(WonderMarker {
            index,
            image: image_rect,
        });
    }
    markers
}

fn paint_wonders(
    painter: &egui::Painter,
    markers: &[WonderMarker],
    zoom: f32,
    textures: &[egui::TextureHandle],
) {
    let fade = ((zoom - 1.15) / 0.8).clamp(0.0, 1.0);
    let opacity = fade * fade * (3.0 - 2.0 * fade);
    let tint = egui::Color32::from_white_alpha((opacity * 255.0).round() as u8);
    for marker in markers {
        let Some(texture) = textures.get(marker.index) else {
            continue;
        };
        painter.image(
            texture.id(),
            marker.image,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            tint,
        );
    }
}

impl Province {
    fn contains(&self, [lon, lat]: [f32; 2]) -> bool {
        if lon < self.bounds[0]
            || lon > self.bounds[2]
            || lat < self.bounds[1]
            || lat > self.bounds[3]
        {
            return false;
        }
        self.parts.iter().any(|part| part.contains([lon, lat]))
    }
}

impl MapMesh {
    fn cache_bounds(&mut self) {
        self.bounds = self.v.iter().fold(
            [f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY],
            |mut bounds, &[lon, lat]| {
                bounds[0] = bounds[0].min(lon);
                bounds[1] = bounds[1].min(lat);
                bounds[2] = bounds[2].max(lon);
                bounds[3] = bounds[3].max(lat);
                bounds
            },
        );
    }

    fn contains(&self, point: [f32; 2]) -> bool {
        let mut start = 0;
        let mut inside = false;
        for &end in &self.r {
            if point_in_ring(point, &self.v[start..end]) {
                inside = !inside;
            }
            start = end;
        }
        inside
    }
}

fn point_in_ring([x, y]: [f32; 2], ring: &[[f32; 2]]) -> bool {
    let mut inside = false;
    for index in 0..ring.len() {
        let a = ring[index];
        let b = ring[(index + ring.len() - 1) % ring.len()];
        if (a[1] > y) != (b[1] > y) && x < (b[0] - a[0]) * (y - a[1]) / (b[1] - a[1]) + a[0] {
            inside = !inside;
        }
    }
    inside
}

fn smoothstep(value: f32) -> f32 {
    let t = value.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn blend_color(a: egui::Color32, b: egui::Color32, amount: f32) -> egui::Color32 {
    let mix =
        |left: u8, right: u8| (f32::from(left) * (1.0 - amount) + f32::from(right) * amount) as u8;
    egui::Color32::from_rgb(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()))
}

fn advance_texture_offset(
    offset: &mut [f32; 2],
    screen_pixels_per_second: [f32; 2],
    tile_degrees: [f32; 2],
    scale: f32,
    dt: f32,
) {
    // Accumulate motion in map coordinates so zooming cannot jump the clouds.
    // Dividing by the current scale keeps their screen speed easy to see.
    let dt = dt.clamp(0.0, 0.1);
    offset[0] = (offset[0] + screen_pixels_per_second[0] * dt / (LONGITUDE_SCALE * scale))
        .rem_euclid(tile_degrees[0]);
    offset[1] = (offset[1] - screen_pixels_per_second[1] * dt / scale).rem_euclid(tile_degrees[1]);
}

fn paint_scrolling_texture(
    painter: &egui::Painter,
    rect: egui::Rect,
    projection: &Projection,
    texture: &egui::TextureHandle,
    tile_degrees: [f32; 2],
    offset: [f32; 2],
    tint: egui::Color32,
) {
    // Place tiles in geographic space so panning and zooming do not make the
    // waves or clouds slide against the coast. Only the deliberate drift moves.
    let top_left = projection.inverse(rect.min);
    let bottom_right = projection.inverse(rect.max);
    let shift_lon = offset[0];
    let shift_lat = offset[1];
    let first_x = ((top_left[0] - shift_lon) / tile_degrees[0]).floor() as i32;
    let last_x = ((bottom_right[0] - shift_lon) / tile_degrees[0]).ceil() as i32;
    let first_y = ((bottom_right[1] - shift_lat) / tile_degrees[1]).floor() as i32;
    let last_y = ((top_left[1] - shift_lat) / tile_degrees[1]).ceil() as i32;
    let uv = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0));
    for y in first_y..last_y {
        for x in first_x..last_x {
            let lon = x as f32 * tile_degrees[0] + shift_lon;
            let lat = y as f32 * tile_degrees[1] + shift_lat;
            let image_rect = egui::Rect::from_min_max(
                projection.point([lon, lat + tile_degrees[1]]),
                projection.point([lon + tile_degrees[0], lat]),
            );
            if image_rect.intersects(rect) {
                painter.image(texture.id(), image_rect, uv, tint);
            }
        }
    }
}

fn paint_meshes(
    painter: &egui::Painter,
    parts: &[MapMesh],
    projection: &Projection,
    color: egui::Color32,
) {
    let mut mesh = egui::Mesh::default();
    for part in parts {
        if !projection.bounds_rect(part.bounds).intersects(painter.clip_rect()) {
            continue;
        }
        let offset = mesh.vertices.len() as u32;
        mesh.vertices.extend(part.v.iter().map(|&point| egui::epaint::Vertex {
            pos: projection.point(point),
            uv: egui::Pos2::ZERO,
            color,
        }));
        mesh.indices.extend(part.t.iter().map(|index| index + offset));
    }
    if !mesh.indices.is_empty() {
        painter.add(egui::Shape::Mesh(mesh.into()));
    }
}

fn paint_rings(
    painter: &egui::Painter,
    mesh: &MapMesh,
    projection: &Projection,
    stroke: egui::Stroke,
) {
    if !projection.bounds_rect(mesh.bounds).intersects(painter.clip_rect().expand(stroke.width)) {
        return;
    }
    let mut start = 0;
    for &end in &mesh.r {
        if end - start >= 3 {
            painter.add(egui::Shape::closed_line(
                mesh.v[start..end].iter().map(|&point| projection.point(point)).collect(),
                stroke,
            ));
        }
        start = end;
    }
}

fn province_color(index: usize) -> egui::Color32 {
    const COLORS: [(u8, u8, u8); 6] = [
        (185, 125, 91),
        (175, 113, 82),
        (197, 145, 105),
        (168, 115, 89),
        (188, 131, 86),
        (179, 123, 99),
    ];
    let (r, g, b) = COLORS[index % COLORS.len()];
    egui::Color32::from_rgb(r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotated_label_fits_a_narrow_province() {
        let province = Province {
            name: "Narrow".into(),
            short: "Narrow".into(),
            label: [0.5, 2.0],
            bounds: [0.0, 0.0, 1.0, 4.0],
            parts: vec![MapMesh {
                v: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 4.0], [0.0, 4.0]],
                r: vec![4],
                t: Vec::new(),
                bounds: [0.0, 0.0, 1.0, 4.0],
            }],
            visual_center: [0.5, 2.0],
        };
        let projection = Projection {
            origin: egui::Pos2::ZERO,
            scale: 40.0,
            center: [0.5, 2.0],
        };
        let text_size = egui::vec2(50.0, 12.0);
        assert!(!label_fits_province(&province, &projection, province.label, text_size, 0.0));
        assert!(label_fits_province(
            &province,
            &projection,
            province.label,
            text_size,
            std::f32::consts::FRAC_PI_2,
        ));
    }

    #[test]
    fn label_candidates_leave_room_and_offer_marker_fallbacks() {
        let province = Province {
            name: "Tarraconensis".into(),
            short: "Tarraconensis".into(),
            label: [10.0, 5.0],
            bounds: [0.0, 0.0, 20.0, 10.0],
            parts: vec![MapMesh {
                v: vec![[0.0, 0.0], [20.0, 0.0], [20.0, 10.0], [0.0, 10.0]],
                r: vec![4],
                t: vec![0, 1, 2, 0, 2, 3],
                bounds: [0.0, 0.0, 20.0, 10.0],
            }],
            visual_center: [10.0, 5.0],
        };
        let projection = Projection {
            origin: egui::Pos2::ZERO,
            scale: 10.0,
            center: [10.0, 5.0],
        };
        let context = egui::Context::default();
        context.begin_pass(Default::default());
        let painter = context.layer_painter(egui::LayerId::background());
        let candidates = label_candidates(&painter, &province, &projection, 2.0);
        assert!(candidates.len() > 1);
        let first = &candidates[0];
        assert_eq!(first.center, province.visual_center);
        let alternate = candidates.iter().find(|candidate| {
            candidate.center != first.center && candidate.font_size == first.font_size
        });
        assert!(alternate.is_some(), "a blocked center must have a readable fallback");
        let text_size = painter
            .layout_no_wrap(province.name.clone(), egui::FontId::proportional(first.font_size), INK)
            .size();
        assert!(label_fit_size(text_size).x >= text_size.x * 1.18);
        for zoom in [1.0, 4.0] {
            let fitted = anchored_label_candidates(&painter, &province, &projection, zoom, first);
            assert!(!fitted.is_empty());
            assert!(fitted.iter().all(|candidate| {
                candidate.center == first.center && candidate.angle == first.angle
            }));
        }
        let mut output = context.end_pass();
        output.textures_delta.clear();
    }

    #[test]
    fn tarraconensis_has_readable_placements_away_from_its_marker() {
        let province =
            atlas().provinces.iter().find(|province| province.name == "Tarraconensis").unwrap();
        let projection = Projection {
            origin: egui::Pos2::ZERO,
            scale: 48.0,
            center: province.label,
        };
        let context = egui::Context::default();
        context.begin_pass(Default::default());
        let painter = context.layer_painter(egui::LayerId::background());
        let candidates = label_candidates(&painter, province, &projection, 3.7);
        let preferred_size = candidates[0].font_size;
        let marker = egui::Rect::from_center_size(
            projection.point([-4.117, 40.948]),
            egui::vec2(42.0, 42.0),
        );
        assert!(candidates.iter().any(|candidate| {
            let galley = painter.layout_no_wrap(
                province.name.clone(),
                egui::FontId::proportional(candidate.font_size),
                INK,
            );
            let bounds =
                rotated_bounds(projection.point(candidate.center), galley.size(), candidate.angle);
            candidate.full_name
                && candidate.font_size == preferred_size
                && !marker.intersects(bounds)
        }));
        let anchor = LabelPlacement {
            center: province.label,
            angle: 0.0,
            font_size: preferred_size,
            full_name: true,
        };
        let anchored = anchored_label_candidates(&painter, province, &projection, 3.7, &anchor);
        let blocked_center =
            egui::Rect::from_center_size(projection.point(anchor.center), egui::vec2(42.0, 42.0));
        let readable_floor =
            candidates.iter().map(|candidate| candidate.font_size).fold(6.5_f32, f32::max) - 2.5;
        let selected = ordered_label_candidates(&anchored, &candidates)
            .find(|candidate| {
                let name = if candidate.full_name {
                    &province.name
                } else {
                    &province.short
                };
                let galley = painter.layout_no_wrap(
                    name.clone(),
                    egui::FontId::proportional(candidate.font_size),
                    INK,
                );
                let bounds = rotated_bounds(
                    projection.point(candidate.center),
                    galley.size(),
                    candidate.angle,
                );
                !blocked_center.intersects(bounds)
            })
            .expect("a blocked Tarraconensis label needs a fallback");
        assert!(selected.font_size >= readable_floor);
        let mut output = context.end_pass();
        output.textures_delta.clear();
    }

    #[test]
    fn rhodes_wonder_anchor_is_on_the_island() {
        let rhodes = WONDERS.iter().find(|wonder| wonder.name == "Colossus of Rhodes").unwrap();
        let province = atlas()
            .provinces
            .iter()
            .find(|province| province.contains(rhodes.position))
            .expect("the Colossus should be on Rhodes, within Asia's island geometry");
        assert_eq!(province.name, "Asia");
        assert!(atlas().land.iter().any(|part| part.contains(rhodes.position)));
    }

    #[test]
    fn mismatched_coastal_backdrop_does_not_tint_sea() {
        for point in [[-6.235, 37.013], [-1.273, 44.175], [4.521, 51.682]] {
            assert!(
                !atlas().land.iter().any(|part| part.contains(point)),
                "coastal water at {point:?} should have the sea color"
            );
        }
        assert!(atlas().land.iter().any(|part| part.contains([10.0, 60.0])));
    }

    #[test]
    fn colossus_grows_with_zoom_without_leaving_rhodes() {
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 545.0));
        let marker_at_zoom = |zoom| {
            let projection = Projection {
                origin: rect.center(),
                scale: 14.7 * zoom,
                center: WONDERS[16].position,
            };
            let marker = layout_wonders(&projection, rect, zoom)
                .into_iter()
                .find(|marker| marker.index == 16)
                .expect("the Colossus should appear on Rhodes at close zoom");
            assert_eq!(marker.image.center(), projection.point(WONDERS[16].position));
            marker.image.width()
        };
        assert!(marker_at_zoom(8.0) > marker_at_zoom(3.6));
    }

    #[test]
    fn new_wonders_are_on_land_and_cities_stay_at_sites() {
        for wonder in &WONDERS[17..] {
            assert!(
                atlas().land.iter().any(|part| part.contains(wonder.position)),
                "{} must be on the map's land geometry",
                wonder.name,
            );
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 545.0));
            let projection = Projection {
                origin: rect.center(),
                scale: 14.7 * 8.0,
                center: wonder.position,
            };
            assert!(
                layout_wonders(&projection, rect, 8.0)
                    .iter()
                    .any(|marker| WONDERS[marker.index].name == wonder.name),
                "{} should be visible at close zoom",
                wonder.name,
            );
        }
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 545.0));
        let projection = Projection {
            origin: rect.center(),
            scale: 12.0,
            center: [20.0, 40.0],
        };
        let overview = layout_cities(&projection, rect, MIN_ZOOM);
        let close = layout_cities(&projection, rect, MAX_ZOOM);
        assert_eq!(overview.len(), CITIES.len());
        assert_eq!(close.len(), CITIES.len());
        for ((city, icon), illustration) in CITIES.iter().zip(&overview).zip(&close) {
            let anchor = projection.point(city.position);
            for image in [icon.icon, illustration.image] {
                let hotspot = image.min
                    + egui::vec2(image.width() * city.hotspot[0], image.height() * city.hotspot[1]);
                assert!(hotspot.distance(anchor) < 0.001);
            }
            assert!(illustration.image.width() > icon.image.width());
        }
        assert_eq!(city_blend(MIN_ZOOM), 0.0);
        assert_eq!(city_blend(MAX_ZOOM), 1.0);
    }
}
