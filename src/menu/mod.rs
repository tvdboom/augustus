//! Menu screens, form controls, loading wallpapers and audio.

// Child widgets share the application resources without exposing them publicly.
use super::*;

#[path = "audio.rs"]
pub(super) mod menu_audio;
#[path = "background.rs"]
pub(super) mod menu_background;
#[path = "controls.rs"]
pub(super) mod menu_controls;
#[path = "forms.rs"]
pub(super) mod menu_forms;
#[path = "render.rs"]
pub(super) mod menu_render;
#[path = "screens.rs"]
pub(super) mod menu_screens;
