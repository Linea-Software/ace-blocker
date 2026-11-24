//! Native VM QA client. Run only on disposable, installed test machines.
//! Uses the normal typed engine RPC bridge and the caller's capability.
//! Does not alter locks, service configuration, or credentials.
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
        .ok_or("usage: vm_network_probe <status|enable|remove>")?;
    if !matches!(action.as_str(), "status" | "enable" | "remove") {
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
