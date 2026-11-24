//! Native VM QA client. Run only on disposable, installed test machines.
//! Uses the normal typed engine RPC bridge and the caller's capability.
//! Only mutates the named synthetic policy; lock tests require a VM snapshot.
//! Does not alter service configuration or credentials.
use ace_engine::{service::api::bridge, settings::Settings};
use serde_json::{Value, json};
use std::time::Duration;

const TEST_NAME: &str = "Ace VM QA - example.org";

async fn call(op: &str, args: Value, token: &str) -> Result<Value, String> {
    tokio::time::timeout(
        Duration::from_secs(15),
        bridge::call(op, args, Some(token.to_owned())),
    )
    .await
    .map_err(|_| format!("{op}: timeout"))?
    .map_err(|error| format!("{op}: HTTP-style status {} ({})", error.status, error.code))
}

async fn run() -> Result<(), String> {
    let action = std::env::args()
        .nth(1)
        .ok_or("usage: vm_network_probe <status|enable|remove|packs|lock|verify-lock>")?;
    if !matches!(
        action.as_str(),
        "status" | "enable" | "remove" | "packs" | "lock" | "verify-lock"
    ) {
        return Err("unknown action".into());
    }
    let token = std::fs::read_to_string(Settings::get_capability_token_path())
        .map_err(|_| "cannot read caller capability")?;
    let token = token.trim();
    if token.is_empty() {
        return Err("empty capability".into());
    }
    call("ping", json!({}), token).await?;
    println!("RPC ping: PASS");
    if action == "packs" {
        let response = call("getAllSitePacks", json!({}), token).await?;
        let packs = response["sitePacks"]
            .as_array()
            .ok_or("invalid pack response")?;
        let bundled: Vec<_> = packs
            .iter()
            .filter(|p| {
                p["isBundled"] == true
                    && p["isSynced"] == true
                    && p["requiresPro"] == false
                    && p["entriesCount"].as_u64().unwrap_or(0) > 0
            })
            .collect();
        println!("Usable bundled Free packs: {}", bundled.len());
        if bundled.len() < 5 {
            return Err("expected five offline starter packs".into());
        }
        return Ok(());
    }
    let blocks = call("getAllNetworkBlocks", json!({}), token).await?;
    let blocks = blocks["networkBlocks"]
        .as_array()
        .ok_or("invalid block response")?;
    let matches: Vec<_> = blocks
        .iter()
        .filter(|block| block["name"] == TEST_NAME)
        .collect();
    if matches.len() > 1 {
        return Err("ambiguous QA blocks; refusing mutation".into());
    }
    if action == "status" {
        println!("Network block read: PASS ({} blocks)", blocks.len());
        if let Some(block) = matches.first() {
            println!(
                "QA block enabled={} active={}",
                block["enabled"], block["isActive"]
            );
        }
        return Ok(());
    }
    if action == "lock" || action == "verify-lock" {
        let block = matches
            .first()
            .ok_or("missing QA block; run enable first")?;
        let id = block["id"].clone();
        if block["enabled"] != true || block["isActive"] != true {
            return Err("QA block not active".into());
        }
        if action == "lock" {
            let end = (chrono::Utc::now() + chrono::Duration::minutes(30)).to_rfc3339();
            call(
                "setLock",
                json!({"network_block_id": id, "payload": {
                    "lockState": "active_until_lock_end", "lockEnd": end
                }}),
                token,
            )
            .await?;
        }
        let state = call("getLock", json!({"network_block_id": id}), token).await?;
        if state["lock"]["isActive"] != true
            || state["lock"]["lockState"]["state"] != "active_until_lock_end"
        {
            return Err("expected active timed lock".into());
        }
        for (op, args) in [
            (
                "setNetworkBlockEnabled",
                json!({"network_block_id": id, "payload": {"enabled": false}}),
            ),
            ("deleteNetworkBlock", json!({"network_block_id": id})),
            (
                "setLock",
                json!({"network_block_id": id, "payload": {"lockState": "inactive", "lockEnd": null}}),
            ),
        ] {
            let result = tokio::time::timeout(
                Duration::from_secs(10),
                bridge::call(op, args, Some(token.to_owned())),
            )
            .await
            .map_err(|_| format!("{op}: timeout"))?;
            match result {
                Err(e) if e.code == "forbidden" => println!("{op} rejected: PASS"),
                Err(e) => return Err(format!("{op}: unexpected error {} ({})", e.status, e.code)),
                Ok(_) => return Err(format!("{op}: unexpectedly allowed")),
            }
        }
        let result = call("getLock", json!({"network_block_id": id}), token).await?;
        if result["lock"]["isActive"] != true
            || result["lock"]["lockState"] != state["lock"]["lockState"]
        {
            return Err("QA lock unexpectedly changed".into());
        }
        println!("Timed lock preserved; verify actual traffic separately");
        return Ok(());
    }
    if action == "remove" {
        if let Some(block) = matches.first() {
            call(
                "deleteNetworkBlock",
                json!({"network_block_id": block["id"]}),
                token,
            )
            .await?;
            println!("QA block removed");
        }
        return Ok(());
    }
    let id = match matches.first() {
        Some(block) => block["id"].clone(),
        None => {
            let saved = call(
                "saveNetworkBlock",
                json!({"payload": {
                    "id": null, "name": TEST_NAME, "icon": null, "safeSearch": false,
                    "rules": [{"rule": "example.org", "ruleType": "domain"}],
                    "scheduleEntries": [], "scheduleMode": "always", "sitePackIds": []
                }}),
                token,
            )
            .await?;
            saved["networkBlock"]["id"].clone()
        }
    };
    if !id.is_string() {
        return Err("missing QA block ID".into());
    }
    call(
        "setNetworkBlockEnabled",
        json!({
            "network_block_id": id, "payload": {"enabled": true}
        }),
        token,
    )
    .await?;
    let verified = call("getAllNetworkBlocks", json!({}), token).await?;
    let active = verified["networkBlocks"]
        .as_array()
        .ok_or("invalid block response")?
        .iter()
        .any(|block| block["id"] == id && block["enabled"] == true && block["isActive"] == true);
    if !active {
        return Err("QA policy not active".into());
    }
    println!(
        "QA policy enabled and active; browser enforcement still requires independent verification"
    );
    Ok(())
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}
