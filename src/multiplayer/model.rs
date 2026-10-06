//! Public JSON contract shared with the single Supabase setup script.

use super::patch::Change;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Authenticated player's lobby card; private recovery codes never appear here.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Member {
    /// Stable campaign player index.
    pub player: usize,
    /// Player-selected display name.
    pub display_name: String,
    /// Index in Augustus's house palette.
    pub color: usize,
    /// Whether the presence lease is still live.
    pub connected: bool,
}

/// Game access response. Only opening a game downloads its full snapshot.
#[derive(Clone, Debug, Deserialize)]
pub struct GameRecord {
    /// Stable database identity.
    pub id: String,
    /// Shareable lobby/game code.
    pub code: String,
    /// Lobby, active, or finished.
    pub status: String,
    /// True for a one-player saved campaign.
    pub single_player: bool,
    /// Monotonically increasing state revision.
    pub revision: u64,
    /// Current snapshot, absent in an unstarted lobby.
    pub state: Option<Value>,
    /// Lobby roster ordered by stable player index.
    pub members: Vec<Member>,
    /// Caller index assigned by the database.
    pub player: usize,
    /// Caller's private, stable recovery code.
    pub recovery_code: String,
    /// Hash of public roster cards and live presence.
    pub roster_token: String,
}

/// Lightweight entry in Resume Game; no campaign state or other players' secrets.
#[derive(Clone, Debug, Deserialize)]
pub struct GameSummary {
    /// Stable game identity.
    pub id: String,
    /// Public share code.
    pub code: String,
    /// Active or finished.
    pub status: String,
    /// Last snapshot commit time in database ISO format.
    pub saved_at: String,
    /// Caller's display name.
    pub display_name: String,
    /// Caller's map color, including older summary responses.
    #[serde(default)]
    pub player_color: usize,
    /// Campaign month corresponding to the reference's turn counter.
    #[serde(default)]
    pub turn: u32,
    /// Number of players in this campaign.
    #[serde(default = "default_player_count")]
    pub player_count: usize,
}

fn default_player_count() -> usize {
    1
}

/// Committed state delta for durable catch-up.
#[derive(Debug, Deserialize)]
pub struct StateEvent {
    /// Revision after this update.
    pub revision: u64,
    /// Replacements with old values removed.
    pub changes: Vec<Change>,
}

/// Poll response; unchanged campaigns return no state, events, or roster cards.
#[derive(Debug, Deserialize)]
pub struct SyncResponse {
    /// Host-authorized resume marker; no snapshot is needed to leave the reconnect lobby.
    #[serde(default)]
    pub resume_generation: u64,
    /// Current revision.
    pub revision: u64,
    /// Current lifecycle.
    pub status: String,
    /// New public roster token.
    pub roster_token: String,
    /// Roster only when changed.
    pub members: Option<Vec<Member>>,
    /// Contiguous deltas since caller's revision.
    pub events: Vec<StateEvent>,
    /// Full fallback only when the bounded replay window has expired.
    pub state: Option<Value>,
}
