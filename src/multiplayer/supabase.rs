//! Narrow connection boundary for future Augustus lobby persistence.

use crate::platform::config::{SupabaseConfig, SUPABASE_PUBLISHABLE_KEY, SUPABASE_URL};

/// Public Supabase configuration passed to the future lobby transport.
pub struct SupabaseLobbyAdapter {
    /// Validated public project configuration.
    pub config: SupabaseConfig,
}

impl SupabaseLobbyAdapter {
    /// Builds the adapter from the repository's temporary public placeholders.
    pub fn configured() -> Result<Self, crate::platform::config::ConfigError> {
        Ok(Self {
            config: SupabaseConfig::new(SUPABASE_URL, SUPABASE_PUBLISHABLE_KEY)?,
        })
    }

    /// Returns whether real project credentials have replaced the placeholders.
    pub fn is_ready(&self) -> bool {
        !self.config.url.contains("your-project-ref")
            && !self.config.publishable_key.contains("replace-me")
    }
}
