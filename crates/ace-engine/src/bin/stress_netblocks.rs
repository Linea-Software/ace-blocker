//! Engine-control stress test using typed remoc RPC, not loopback REST.
//! Only deletes blocks it created. Usage: stress-netblocks --duration 120
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use ace_engine::{service::api::bridge, settings::Settings};
use serde_json::{json, Value};
use rand::RngExt;

#[derive(Default)]
struct Stats { ok: AtomicU64, failed: AtomicU64, rejected: AtomicU64 }

fn token() -> Option<String> {
    std::fs::read_to_string(Settings::get_capability_token_path())
        .ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}
async fn call(op: &str, args: Value, token: Option<String>) -> Result<Value, String> {
    tokio::time::timeout(Duration::from_secs(30), bridge::call(op,args,token))
        .await.map_err(|_| format!("{op}: timed out"))?
        .map_err(|e| format!("{op}: {} [{}]: {}", e.status,e.code,e.error))
}
async fn batch(number: u64, stats: &Stats) {
    let token = token();
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
    let count = rand::rng().random_range(5..=20);
    let mut created = Vec::new();
    for index in 0..count {
        tokio::time::sleep(Duration::from_millis(rand::rng().random_range(20..=100))).await;
        let name = format!("stress-{stamp}-{number}-{index}");
        match call("createNetworkBlock", json!({"payload":{"blockName":name}}), token.clone()).await {
            Ok(v) => {
                if let Some(id)=v["networkBlock"]["id"].as_str() {
                    created.push(id.to_owned());
                    stats.ok.fetch_add(1,Ordering::Relaxed);
                } else {
                    stats.failed.fetch_add(1,Ordering::Relaxed);
                    eprintln!("createNetworkBlock succeeded but omitted networkBlock.id");
                }
            }
            Err(error) => {
                stats.failed.fetch_add(1,Ordering::Relaxed);
                eprintln!("{error}");
            }
        }
    }
    match call("getAllNetworkBlocks",json!({}),token.clone()).await {
        Ok(v) => {
            let ids: HashSet<&str> = v["networkBlocks"].as_array().into_iter().flatten()
                .filter_map(|block|block["id"].as_str()).collect();
            let missing=created.iter().filter(|id|!ids.contains(id.as_str())).count() as u64;
            stats.failed.fetch_add(missing,Ordering::Relaxed);
            if missing==0 { stats.ok.fetch_add(1,Ordering::Relaxed); }
        }
        Err(error) => {
            stats.failed.fetch_add(1,Ordering::Relaxed);
            eprintln!("{error}");
        }
    }
    match call("deleteNetworkBlock",json!({"network_block_id":"invalid"}),token.clone()).await {
        Err(error) if error.contains("400") => {
            stats.rejected.fetch_add(1,Ordering::Relaxed);
        }
        other => {
            stats.failed.fetch_add(1,Ordering::Relaxed);
            eprintln!("Invalid-ID RPC was not rejected: {other:?}");
        }
    }
    for id in created {
        tokio::time::sleep(Duration::from_millis(rand::rng().random_range(20..=100))).await;
        match call("deleteNetworkBlock",json!({"network_block_id":id}),token.clone()).await {
            Ok(_) => {stats.ok.fetch_add(1,Ordering::Relaxed);}
            Err(error) => {stats.failed.fetch_add(1,Ordering::Relaxed);eprintln!("{error}");}
        }
    }
}
#[tokio::main]
async fn main() -> std::process::ExitCode {
    let args:Vec<_>=std::env::args().skip(1).collect();
    let duration=if args.is_empty(){600}else if args.len()==2&&args[0]=="--duration"{
        args[1].parse::<u64>().unwrap_or(0)
    } else {0};
    if duration==0 {eprintln!("Usage: stress-netblocks [--duration seconds]");
        return std::process::ExitCode::FAILURE;}
    if call("ping",json!({}),token()).await.is_err() {
        eprintln!("Engine remoc control socket is unavailable.");
        return std::process::ExitCode::FAILURE;
    }
    let stats=Stats::default();
    let start=Instant::now();
    let mut batches=0;
    while start.elapsed()<Duration::from_secs(duration) {
        batch(batches,&stats).await;
        batches+=1;
    }
    let ok=stats.ok.load(Ordering::Relaxed);
    let failed=stats.failed.load(Ordering::Relaxed);
    println!("{batches} batches: {ok} passed, {failed} unexpected failures, {} expected rejections",
        stats.rejected.load(Ordering::Relaxed));
    if failed>(ok/100).max(1){std::process::ExitCode::FAILURE}else{std::process::ExitCode::SUCCESS}
}
