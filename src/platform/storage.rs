//! Small client-local authentication store, separate from cloud campaign data.
use serde::{Deserialize, Serialize};

/// Minimal credentials required to renew an anonymous identity.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    /// Short-lived authenticated user token.
    pub access_token: String,
    /// Token for session renewal; never logged or embedded in a build.
    pub refresh_token: String,
    /// Expiration in Unix seconds.
    #[serde(default)]
    pub expires_at: u64,
}

const STORAGE_KEY: &str = "augustus-auth-bbjsemzdjkgwymzjnlei";

/// Loads only the current project's identity.
pub fn load() -> Option<Session> {
    #[cfg(not(target_arch = "wasm32"))]
    let data = std::fs::read_to_string(path()).ok()?;
    #[cfg(target_arch = "wasm32")]
    let data = web_sys::window()?.local_storage().ok()??.get_item(STORAGE_KEY).ok()??;
    serde_json::from_str(&data).ok()
}

/// Persists a renewed identity, reporting storage errors instead of losing access silently.
pub fn save(session: &Session) -> Result<(), String> {
    let data =
        serde_json::to_string(session).map_err(|_| "Could not store sign-in.".to_string())?;
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|_| "Could not create Augustus settings folder.".to_string())?;
        }
        let temporary = path.with_extension("tmp");
        std::fs::write(&temporary, data)
            .map_err(|_| "Could not save sign-in. Keep your recovery code.".to_string())?;
        std::fs::rename(temporary, path)
            .map_err(|_| "Could not save sign-in. Keep your recovery code.".to_string())?;
    }
    #[cfg(target_arch = "wasm32")]
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .ok_or_else(|| "Browser storage is unavailable. Keep your recovery code.".to_string())?
        .set_item(STORAGE_KEY, &data)
        .map_err(|_| "Could not save sign-in. Keep your recovery code.".to_string())?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn path() -> std::path::PathBuf {
    let root = std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("XDG_DATA_HOME"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(std::env::temp_dir)
                .join(".local/share")
        });
    root.join("Augustus").join(format!("{STORAGE_KEY}.json"))
}
