//! Menu music setup and volume updates.

use super::*;

pub(super) fn start_music(mut commands: Commands, assets: Res<AssetServer>, audio: Res<Audio>) {
    let music = audio.play(assets.load("audio/music.ogg")).looped().with_volume(-60.0).handle();
    commands.insert_resource(MenuAudio {
        volume: 0.55,
        mode: AudioMode::Effects,
        restored_mode: AudioMode::Effects,
        music,
    });
}

pub(super) fn update_music_volume(
    menu_audio: Res<MenuAudio>,
    mut instances: ResMut<Assets<AudioInstance>>,
) {
    if !menu_audio.is_changed() {
        return;
    }
    if let Some(mut instance) = instances.get_mut(&menu_audio.music) {
        let decibels = if menu_audio.mode != AudioMode::Music || menu_audio.volume <= 0.001 {
            -60.0
        } else {
            -20.0 + 20.0 * menu_audio.volume.clamp(0.0, 1.0).log10()
        };
        instance.set_decibels(decibels, AudioTween::linear(Duration::from_millis(90)));
    }
}
