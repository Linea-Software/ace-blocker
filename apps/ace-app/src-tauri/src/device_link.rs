//! Non-privileged HTTPS device authorization. This module lives in the
//! interactive Tauri process, not in the LocalSystem engine. The device secret
//! and redeemed API key never enter the webview or any browser URL.
use ace_engine::service::api::bridge;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::State;

#[cfg(debug_assertions)]
const SITE_ORIGIN: &str = "http://localhost:5173";
#[cfg(not(debug_assertions))]
const SITE_ORIGIN: &str = "https://aceblocker.com";

#[derive(Default)]
pub struct DeviceLinkState(pub Mutex<Option<Challenge>>);

#[derive(Clone)]
pub struct Challenge {
    code: String,
    expires: Instant,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServerStart {
    device_code: String,
    user_code: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceLinkStart {
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServerPoll {
    state: String,
    api_key: Option<String>,
    retry_after: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceLinkPoll {
    state: String,
    retry_after: u64,
}

fn http_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "Could not prepare secure device linking".to_owned())
}

#[tauri::command]
pub async fn device_link_start(state: State<'_, DeviceLinkState>) -> Result<DeviceLinkStart, String> {
    let reply = http_client()?
        .post(format!("{SITE_ORIGIN}/api/device-link/start"))
        .json(&json!({}))
        .send()
        .await
        .map_err(|_| "The Ace account server is unreachable".to_owned())?;
    if !reply.status().is_success() {
        return Err("Could not start account linking".into());
    }
    let response: ServerStart = reply
        .json()
        .await
        .map_err(|_| "Invalid account server reply".to_owned())?;
    if response.device_code.len() != 43 || response.expires_in == 0 || response.expires_in > 900 {
        return Err("Invalid account linking challenge".into());
    }
    // Session-only; a restart discards the secret. Never serialize this.
    let mut current = state.0.lock().map_err(|_| "Device link unavailable".to_owned())?;
    *current = Some(Challenge {
        code: response.device_code,
        expires: Instant::now() + Duration::from_secs(response.expires_in),
    });
    Ok(DeviceLinkStart {
        user_code: response.user_code,
        verification_uri: format!("{SITE_ORIGIN}/connect"),
        expires_in: response.expires_in,
        interval: response.interval.max(5),
    })
}

#[tauri::command]
pub async fn device_link_poll(state: State<'_, DeviceLinkState>) -> Result<DeviceLinkPoll, String> {
    let session = {
        state.0.lock().map_err(|_| "Device link unavailable".to_owned())?.clone()
    }.ok_or_else(|| "No active device link".to_owned())?;
    if session.expires <= Instant::now() {
        *state.0.lock().map_err(|_| "Device link unavailable".to_owned())? = None;
        return Ok(DeviceLinkPoll { state: "expired".into(), retry_after: 0 });
    }
    let reply = http_client()?
        .post(format!("{SITE_ORIGIN}/api/device-link/poll"))
        .json(&json!({"deviceCode": session.code}))
        .send()
        .await
        .map_err(|_| "Could not contact the Ace account server".to_owned())?;
    let response: ServerPoll = reply.json().await
        .map_err(|_| "Invalid account server reply".to_owned())?;
    match response.state.as_str() {
        "pending" | "slow_down" => Ok(DeviceLinkPoll {
            state: response.state,
            retry_after: response.retry_after.unwrap_or(5).max(5),
        }),
        "expired" | "consumed" => {
            let mut active = state.0.lock().map_err(|_| "Device link unavailable".to_owned())?;
            if active.as_ref().is_some_and(|active| active.code == session.code) {
                *active = None;
            }
            Ok(DeviceLinkPoll { state: response.state, retry_after: 0 })
        }
        "authorized" => {
            let key = response.api_key.filter(|key| !key.is_empty())
                .ok_or_else(|| "Missing account authorization".to_owned())?;
            {
                let mut active = state.0.lock().map_err(|_| "Device link unavailable".to_owned())?;
                if !active.as_ref().is_some_and(|active| active.code == session.code) {
                    return Err("Device link was cancelled".into());
                }
                *active = None;
            }
            // Only the trusted Tauri process forwards the key to the existing
            // capability-protected remoc RPC. It never returns to JavaScript.
            bridge::call(
                "setApiKey",
                json!({"payload": {"apiKey": key}}),
                super::read_engine_capability(),
            )
            .await
            .map_err(|_| "Could not import the account into Ace Blocker".to_owned())?;
            Ok(DeviceLinkPoll { state: "linked".into(), retry_after: 0 })
        }
        _ => Err("Unexpected device link state".into()),
    }
}

#[tauri::command]
pub fn device_link_cancel(state: State<'_, DeviceLinkState>) -> Result<(), String> {
    *state.0.lock().map_err(|_| "Device link unavailable".to_owned())? = None;
    Ok(())
}
