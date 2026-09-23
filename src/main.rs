#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy::asset::AssetMetaCheck;
use bevy::prelude::*;
use bevy::window::{WindowMode, WindowPlugin, WindowPosition};
use bevy_kira_audio::AudioPlugin;

#[cfg(target_os = "windows")]
use bevy::ecs::system::NonSendMarker;
#[cfg(target_os = "windows")]
use bevy::winit::WINIT_WINDOWS;
#[cfg(target_os = "windows")]
use winit::window::Icon;

use augustus::app::{window_resolution, AugustusPlugin};
use augustus::TITLE;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_linear())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: TITLE.into(),
                        mode: WindowMode::Windowed,
                        position: WindowPosition::Centered(MonitorSelection::Primary),
                        resolution: window_resolution(),
                        #[cfg(target_arch = "wasm32")]
                        canvas: Some("#bevy".to_string()),
                        fit_canvas_to_parent: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: "assets-runtime".to_string(),
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                }),
        )
        .add_plugins((AudioPlugin, AugustusPlugin))
        .add_systems(Startup, set_windows_taskbar_icon)
        .run();
}

#[cfg(target_os = "windows")]
fn set_windows_taskbar_icon(_: NonSendMarker) {
    let Ok(image) = image::open("assets-runtime/images/icons/augustus.png") else {
        warn!("could not load the packaged Windows icon");
        return;
    };
    let image = image.into_rgba8();
    let (width, height) = image.dimensions();
    let rgba = image.into_raw();

    let Ok(icon) = Icon::from_rgba(rgba, width, height) else {
        warn!("packaged Windows icon dimensions or pixels are invalid");
        return;
    };

    WINIT_WINDOWS.with_borrow(|windows| {
        for window in windows.windows.values() {
            window.set_window_icon(Some(icon.clone()));
        }
    });
}

#[cfg(not(target_os = "windows"))]
fn set_windows_taskbar_icon() {}
