//! Lobby preview models used while the Supabase project is being configured.

use bevy::prelude::Resource;
use rand::random_range;

const CODE_ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const GAME_CODE_LENGTH: usize = 6;
const RECOVERY_CODE_LENGTH: usize = 12;

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
    (0..GAME_CODE_LENGTH)
        .map(|_| CODE_ALPHABET[random_range(0..CODE_ALPHABET.len())] as char)
        .collect()
}

/// Accept the reference's six-character share codes and existing hosted codes.
pub fn valid_game_code(value: &str) -> bool {
    let canonical = normalize_code(value);
    (canonical.len() == GAME_CODE_LENGTH && canonical.bytes().all(|b| CODE_ALPHABET.contains(&b)))
        || (canonical.len() == 8 && canonical.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// Validate compact private codes while preserving older grouped and hex saves.
pub fn valid_recovery_code(value: &str) -> bool {
    let canonical = normalize_code(value);
    (matches!(canonical.len(), RECOVERY_CODE_LENGTH | 16)
        && canonical.bytes().all(|b| CODE_ALPHABET.contains(&b)))
        || (canonical.len() == 32 && canonical.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn normalize_code(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_ascii_whitespace() && *c != '-')
        .map(|c| match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            other => other,
        })
        .collect()
}

#[cfg(test)]
#[path = "../../tests/unit/lobby_codes.rs"]
mod tests;
