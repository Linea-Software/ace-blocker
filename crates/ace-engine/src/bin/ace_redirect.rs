//! Browser callback receiver for Ace Blocker authentication.
//! Deep links are parsed through a strict allowlist and forwarded via typed RPC;
//! no local HTTP listener, port discovery, or TLS bypass is involved.
use std::env;
use std::time::Duration;

use ace_engine::service::api::bridge;
use app_dirs2::*;
use serde_json::json;

const APP_INFO: AppInfo = AppInfo {
    name: "AceBlocker",
    author: "LineaSoftware",
};

fn capability_token() -> Option<String> {
    let path = get_app_dir(AppDataType::SharedData, &APP_INFO, "capability.token").ok()?;
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
}

#[tokio::main]
async fn main() {
    let uri = env::args().nth(1).unwrap_or_default();
    if !uri.starts_with("aceblocker://") {
        eprintln!("Usage: ace-redirect aceblocker://account/api-key#apiKey=...");
        std::process::exit(1);
    }
    if let Err(error) = forward(&uri).await {
        eprintln!("Rejected or failed Ace Blocker callback: {error}");
        #[cfg(windows)]
        std::thread::sleep(Duration::from_secs(5));
        std::process::exit(2);
    }
}

async fn forward(uri: &str) -> Result<(), Box<dyn std::error::Error>> {
    // The allowlist prevents arbitrary web pages from invoking shutdown,
    // lock changes, backup import, or privileged maintenance.
    let target =
        ace_engine::protocol::resolve_forward_target(uri).map_err(|error| error.to_string())?;
    if target.path != "/account/api-key" {
        return Err("unsupported callback path".into());
    }

    let fragment = uri.split_once('#').map(|(_, fragment)| fragment);
    let api_key = ace_engine::protocol::api_key_from_fragment(fragment)
        .or_else(|| {
            target.query.as_ref().and_then(|raw| {
                url::form_urlencoded::parse(raw.as_bytes())
                    .find(|(name, _)| name == "apiKey")
                    .map(|(_, value)| value.into_owned())
            })
        })
        .filter(|key| !key.is_empty())
        .ok_or("missing API key in authentication callback")?;

    bridge::call(
        "setApiKey",
        json!({ "payload": { "apiKey": api_key } }),
        capability_token(),
    )
    .await
    .map_err(|error| {
        format!(
            "engine rejected authentication callback ({}): {}",
            error.code, error.error
        )
    })?;

    println!("Ace Blocker authentication callback accepted");
    Ok(())
}
