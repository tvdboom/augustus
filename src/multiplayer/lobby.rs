//! Lobby preview models used while the Supabase project is being configured.

use bevy::prelude::Resource;
use rand::random_range;

#[derive(Resource, Debug, Clone, Default)]
/// One local host's editable details in the menu-only lobby.
pub struct LobbyPreview {
    /// Display name shown to other players.
    pub display_name: String,
    /// Share code generated for the local preview.
    pub code: String,
    /// Index into the Augustus house-color palette.
    pub color_index: usize,
}

impl LobbyPreview {
    /// Replaces the current draft after creating or joining a preview lobby.
    pub fn reset(&mut self, display_name: &str, code: String) {
        self.display_name = display_name.to_string();
        self.code = code;
        self.color_index = 0;
    }
}

/// Creates a short readable code for a local lobby preview.
pub fn generate_game_code() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let code =
        (0..8).map(|_| ALPHABET[random_range(0..ALPHABET.len())] as char).collect::<String>();
    format!("{}-{}", &code[..4], &code[4..])
}
