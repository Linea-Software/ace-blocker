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

async fn soft_cycle(token: &str) -> Result<(), String> {
    let name = "Ace VM QA - reflection";
    let blocks = call("getAllNetworkBlocks", json!({}), token).await?;
    if blocks["networkBlocks"]
        .as_array()
        .ok_or("invalid block response")?
        .iter()
        .any(|block| block["name"] == name)
    {
        return Err("existing reflection fixture; restore the snapshot".into());
    }
    let saved = call(
        "saveNetworkBlock",
        json!({"payload": {
            "id": null, "name": name, "icon": null, "safeSearch": false,
            "rules": [{"rule": "example.net", "ruleType": "domain"}],
            "scheduleEntries": [], "scheduleMode": "always", "sitePackIds": []
        }}),
        token,
    )
    .await?;
    let id = saved["networkBlock"]["id"].clone();
    call(
        "setNetworkBlockEnabled",
        json!({"network_block_id": id, "payload": {"enabled": true}}),
        token,
    )
    .await?;
    call("setLock", json!({"network_block_id": id, "payload": {"lockState": "soft_unlockable", "lockEnd": null}}), token).await?;
    let start = std::time::Instant::now();
    let challenge = call("initiateUnlock", json!({"network_block_id": id}), token).await?;
    if challenge["cooldownSeconds"] != 300 || !challenge["challengeId"].is_string() {
        return Err("unexpected challenge".into());
    }
    let confirm =
        json!({"network_block_id": id, "payload": {"challengeId": challenge["challengeId"]}});
    let early = call("confirmUnlock", confirm.clone(), token).await?;
    if early["status"] != 425 || early["body"]["retryAfterSeconds"].as_u64().unwrap_or(0) == 0 {
        return Err("early unlock not rejected".into());
    }
    println!("Reflection cooldown early confirmation rejected: PASS");
    let wrong = json!({"network_block_id": id, "payload": {"challengeId": "00000000-0000-0000-0000-000000000000"}});
    match tokio::time::timeout(
        Duration::from_secs(15),
        bridge::call("confirmUnlock", wrong, Some(token.to_owned())),
    )
    .await
    {
        Ok(Err(e)) if e.code == "bad_request" => println!("Wrong challenge rejected: PASS"),
        _ => return Err("wrong challenge not rejected with expected error".into()),
    }
    let repeated = call("initiateUnlock", json!({"network_block_id": id}), token).await?;
    if repeated["challengeId"] != challenge["challengeId"] {
        return Err("challenge initiation is not idempotent".into());
    }
    tokio::time::sleep(Duration::from_secs(301).saturating_sub(start.elapsed())).await;
    let confirmed = call("confirmUnlock", confirm.clone(), token).await?;
    if confirmed["status"] != 200 {
        return Err(format!(
            "reflection redemption failed: status={}",
            confirmed["status"]
        ));
    }
    let lock = call("getLock", json!({"network_block_id": id}), token).await?;
    if lock["lock"]["isActive"] != false {
        return Err("lock remained active after redemption".into());
    }
    match tokio::time::timeout(
        Duration::from_secs(15),
        bridge::call("confirmUnlock", confirm, Some(token.to_owned())),
    )
    .await
    {
        Ok(Err(e)) if e.code == "not_found" => println!("Consumed challenge replay rejected: PASS"),
        _ => return Err("replay was not rejected".into()),
    }
    call("deleteNetworkBlock", json!({"network_block_id": id}), token).await?;
    println!(
        "Reflection unlock after actual monotonic wait: PASS ({} seconds)",
        start.elapsed().as_secs()
    );
    Ok(())
}

async fn free_policy(token: &str) -> Result<(), String> {
    let account = call("getAccountInfo", json!({}), token).await?;
    if account["isPro"] != false {
        return Err("expected a Free test account".into());
    }
    let settings = call("getResistanceModeSettings", json!({}), token).await?;
    if settings["proEligible"] != false {
        return Err("unexpected Pro eligibility".into());
    }
    println!("Free entitlement: PASS");
    let end = (chrono::Utc::now() + chrono::Duration::minutes(30)).to_rfc3339();
    let args = json!({"payload": {"lockState": "active_until_lock_end", "lockEnd": end}});
    match tokio::time::timeout(
        Duration::from_secs(15),
        bridge::call("setResistanceModeLock", args, Some(token.to_owned())),
    )
    .await
    {
        Ok(Err(e)) if e.code == "pro_plan_required" => {
            println!("Free Resistance Lock rejected: PASS")
        }
        _ => return Err("Free Resistance Lock not rejected as Pro-only".into()),
    }
    match tokio::time::timeout(
        Duration::from_secs(15),
        bridge::call(
            "exportBackup",
            json!({}),
            Some("invalid-qa-capability".into()),
        ),
    )
    .await
    {
        Ok(Err(e)) if e.code == "capability_required" => {
            println!("Invalid capability rejected: PASS")
        }
        _ => return Err("invalid capability unexpectedly accepted or wrong error".into()),
    }
    Ok(())
}

async fn scheduled_cycle(token: &str, proxy_port: u16) -> Result<(), String> {
    use chrono::{Duration as ChronoDuration, Local, Timelike};

    let name = "Ace VM QA - schedule";
    let blocks = call("getAllNetworkBlocks", json!({}), token).await?;
    if blocks["networkBlocks"]
        .as_array()
        .ok_or("invalid block response")?
        .iter()
        .any(|block| block["name"] == name)
    {
        return Err("existing schedule fixture; restore the snapshot".into());
    }
    let now = Local::now();
    let start = now
        .with_second(0)
        .and_then(|t| t.with_nanosecond(0))
        .ok_or("invalid local time")?
        + ChronoDuration::minutes(2);
    let end = start + ChronoDuration::minutes(1);
    if now.date_naive() != end.date_naive() {
        return Err("schedule test requires three minutes before local midnight".into());
    }
    let client = reqwest::Client::builder()
        .proxy(
            reqwest::Proxy::http(format!("http://127.0.0.1:{proxy_port}"))
                .map_err(|_| "invalid proxy")?,
        )
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|_| "build HTTP client")?;
    let traffic = async |expect_block: bool| -> Result<(), String> {
        let status = client
            .get("http://example.net/")
            .send()
            .await
            .map_err(|e| format!("schedule HTTP request: {e}"))?
            .status();
        if (status.as_u16() == 404) != expect_block
            || (!expect_block && !status.is_success() && !status.is_redirection())
        {
            return Err(format!(
                "schedule traffic expected block={expect_block}, got {status}"
            ));
        }
        println!("Schedule traffic expected block={expect_block}: {status} PASS");
        Ok(())
    };
    traffic(false).await?;
    let saved = call(
        "saveNetworkBlock",
        json!({"payload": {
            "id": null, "name": name, "icon": null, "safeSearch": false,
            "rules": [{"rule": "example.net", "ruleType": "domain"}],
            "scheduleEntries": [{"startTime": start.format("%H:%M:%S").to_string(),
                "endTime": end.format("%H:%M:%S").to_string(),
                "monday": true, "tuesday": true, "wednesday": true, "thursday": true,
                "friday": true, "saturday": true, "sunday": true}],
            "scheduleMode": "scheduled", "sitePackIds": []
        }}),
        token,
    )
    .await?;
    let id = saved["networkBlock"]["id"].clone();
    call(
        "setNetworkBlockEnabled",
        json!({"network_block_id": id, "payload": {"enabled": true}}),
        token,
    )
    .await?;
    call("setLock", json!({"network_block_id": id, "payload": {"lockState": "active_during_schedule", "lockEnd": null}}), token).await?;
    let status = async |active: bool| -> Result<(), String> {
        let state = call("getLock", json!({"network_block_id": id}), token).await?;
        let blocks = call("getAllNetworkBlocks", json!({}), token).await?;
        let block = blocks["networkBlocks"]
            .as_array()
            .ok_or("invalid block response")?
            .iter()
            .find(|block| block["id"] == id)
            .ok_or("missing schedule fixture")?;
        if block["isActive"] != active || state["lock"]["isActive"] != active {
            return Err(format!(
                "schedule expected active={active}, got policy={} lock={}",
                block["isActive"], state["lock"]["isActive"]
            ));
        }
        println!("Schedule policy and lock active={active}: PASS");
        Ok(())
    };
    status(false).await?;
    traffic(false).await?;
    tokio::time::sleep(
        (start - Local::now()).to_std().unwrap_or_default() + Duration::from_secs(3),
    )
    .await;
    status(true).await?;
    traffic(true).await?;
    match tokio::time::timeout(
        Duration::from_secs(10),
        bridge::call(
            "setNetworkBlockEnabled",
            json!({"network_block_id": id, "payload": {"enabled": false}}),
            Some(token.to_owned()),
        ),
    )
    .await
    {
        Ok(Err(e)) if e.code == "forbidden" => {
            println!("Schedule lock prevents disabling inside window: PASS")
        }
        _ => return Err("schedule lock did not reject disabling".into()),
    }
    tokio::time::sleep((end - Local::now()).to_std().unwrap_or_default() + Duration::from_secs(3))
        .await;
    status(false).await?;
    traffic(false).await?;
    call("deleteNetworkBlock", json!({"network_block_id": id}), token).await?;
    println!("Scheduled cycle and window-bound lock: PASS");
    Ok(())
}

async fn run() -> Result<(), String> {
    let action = std::env::args()
        .nth(1)
        .ok_or("usage: vm_network_probe <status|enable|remove|packs|lock|verify-lock|soft-cycle|free-policy|schedule-cycle>")?;
    if !matches!(
        action.as_str(),
        "status"
            | "enable"
            | "remove"
            | "packs"
            | "lock"
            | "verify-lock"
            | "soft-cycle"
            | "free-policy"
            | "schedule-cycle"
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
    if action == "soft-cycle" {
        return soft_cycle(token).await;
    }
    if action == "free-policy" {
        return free_policy(token).await;
    }
    if action == "schedule-cycle" {
        let port = std::env::args()
            .nth(2)
            .ok_or("missing proxy port")?
            .parse()
            .map_err(|_| "invalid proxy port")?;
        return scheduled_cycle(token, port).await;
    }
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
