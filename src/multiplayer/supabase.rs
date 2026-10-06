//! Reused HTTP client and asynchronous authenticated SQL RPC transport.
use crate::platform::{
    config::SupabaseConfig,
    storage::{self, Session},
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::mpsc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

/// Transport response including a potentially refreshed session.
pub struct Response {
    /// Auth identity to retain even when the SQL call failed.
    pub session: Option<Session>,
    /// Parsed RPC result or a sanitized user-facing failure.
    pub result: Result<Value, String>,
}

/// A single reusable connection pool and native runtime per running client.
pub struct SupabaseLobbyAdapter {
    /// Public configuration; contains no server credentials.
    pub config: SupabaseConfig,
    client: reqwest::Client,
    #[cfg(not(target_arch = "wasm32"))]
    runtime: Arc<tokio::runtime::Runtime>,
}

impl SupabaseLobbyAdapter {
    /// Initializes the HTTP pool without making a network request.
    pub fn configured() -> Result<Self, String> {
        let config = SupabaseConfig::load().map_err(|e| e.to_string())?;
        let builder = reqwest::Client::builder();
        #[cfg(not(target_arch = "wasm32"))]
        let builder = builder.timeout(std::time::Duration::from_secs(15)).pool_max_idle_per_host(1);
        let client = builder
            .build()
            .map_err(|_| "Could not initialize the online connection.".to_string())?;
        #[cfg(not(target_arch = "wasm32"))]
        let runtime = Arc::new(
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|_| "Could not initialize online tasks.".to_string())?,
        );
        Ok(Self {
            config,
            client,
            #[cfg(not(target_arch = "wasm32"))]
            runtime,
        })
    }
    /// True when the fixed public project configuration is complete.
    pub fn is_ready(&self) -> bool {
        !self.config.url.contains("your-project-ref")
    }

    /// Dispatches one request without blocking Bevy. The caller serializes operations.
    pub fn dispatch(
        &self,
        session: Option<Session>,
        rpc: String,
        payload: Value,
    ) -> mpsc::Receiver<Response> {
        let (sender, receiver) = mpsc::channel();
        let client = self.client.clone();
        let config = self.config.clone();
        let future = async move {
            let authenticated = authenticate(&client, &config, session).await;
            let response = match authenticated {
                Ok(session) => {
                    let result = post(
                        &client,
                        &config,
                        &format!("rest/v1/rpc/{rpc}"),
                        Some(&session.access_token),
                        payload,
                    )
                    .await;
                    Response {
                        session: Some(session),
                        result,
                    }
                },
                Err(error) => Response {
                    session: None,
                    result: Err(error),
                },
            };
            let _ = sender.send(response);
        };
        #[cfg(not(target_arch = "wasm32"))]
        {
            let runtime = self.runtime.clone();
            std::thread::spawn(move || runtime.block_on(future));
        }
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(future);
        receiver
    }
}

async fn authenticate(
    client: &reqwest::Client,
    config: &SupabaseConfig,
    previous: Option<Session>,
) -> Result<Session, String> {
    #[derive(Deserialize)]
    struct Auth {
        access_token: String,
        refresh_token: String,
        #[serde(default)]
        expires_in: u64,
    }
    if let Some(session) = &previous {
        if session.expires_at > now().saturating_add(60) {
            return Ok(session.clone());
        }
    }
    let result = if let Some(session) = previous {
        post(
            client,
            config,
            "auth/v1/token?grant_type=refresh_token",
            None,
            json!({"refresh_token":session.refresh_token}),
        )
        .await
    } else {
        post(client, config, "auth/v1/signup", None, json!({})).await
    };
    let auth: Auth =
        serde_json::from_value(result?).map_err(|_| "Invalid sign-in response.".to_string())?;
    let session = Session {
        access_token: auth.access_token,
        refresh_token: auth.refresh_token,
        expires_at: now() + auth.expires_in,
    };
    storage::save(&session)?;
    Ok(session)
}

async fn post(
    client: &reqwest::Client,
    config: &SupabaseConfig,
    path: &str,
    token: Option<&str>,
    payload: Value,
) -> Result<Value, String> {
    let mut request = client
        .post(config.endpoint(path))
        .header("apikey", &config.publishable_key)
        .timeout(std::time::Duration::from_secs(15))
        .json(&payload);
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }
    let response = request
        .send()
        .await
        .map_err(|_| "Connection failed. Check your internet and retry.".to_string())?;
    let status = response.status();
    let value: Value = response
        .json()
        .await
        .map_err(|_| "The online service returned an unreadable response.".to_string())?;
    if status.is_success() {
        return Ok(value);
    }
    let code =
        value.get("code").or_else(|| value.get("error_code")).and_then(Value::as_str).unwrap_or("");
    if code == "PGRST202" || code == "PGRST205" {
        return Err(
            "The database is not set up yet. Run supabase/schema.sql in the Supabase SQL Editor."
                .into(),
        );
    }
    if code == "anonymous_provider_disabled" {
        return Err("Enable Anonymous Sign-Ins in Supabase Authentication settings.".into());
    }
    let message = value
        .get("message")
        .or_else(|| value.get("msg"))
        .and_then(Value::as_str)
        .unwrap_or("The online request failed.");
    // Never include HTTP headers, request bodies, credentials or arbitrary error detail.
    Err(message.chars().take(300).collect())
}

fn now() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
    #[cfg(target_arch = "wasm32")]
    {
        (js_sys::Date::now() / 1000.0) as u64
    }
}

/// Creates an unpredictable UUID for connection leases and idempotent requests.
pub fn request_id() -> String {
    let mut bytes: [u8; 16] = rand::random();
    bytes[6] = (bytes[6] & 15) | 64;
    bytes[8] = (bytes[8] & 63) | 128;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..])
}
