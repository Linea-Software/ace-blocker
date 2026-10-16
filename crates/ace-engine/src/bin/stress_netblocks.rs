//! stress-netblocks — sustained load test for the ace-engine REST API.
//!
//! Hammers the local engine API (`https://127.0.0.1:<port>`) for at least
//! `--duration` seconds: creates network blocks in batches of 5–20, lists
//! them, deletes them, and periodically sends invalid payloads that must be
//! rejected with a 4xx.
//!
//! It never touches the app's default blocks (`01950000-0000-7000-8000-
//! 000000000001..004`) — the only ids it deletes are the ones it created
//! itself (names `stress-<ts>-<batch>-<n>`).
//!
//! TLS: the engine serves a self-signed certificate. This is a local stress
//! tool that only ever talks to the engine on 127.0.0.1, so certificate
//! validation is disabled (`danger_accept_invalid_certs(true)`) instead of
//! pinning `certs\ace_ca.pem` — simpler and equally safe here.
//!
//! CPU discipline: every request is preceded by a 20–100 ms jittered sleep and
//! at most [`MAX_IN_FLIGHT`] requests run concurrently, keeping the average
//! load well under one core.
//!
//! Usage:
//!   stress-netblocks [--duration <secs>] [--port <port>]
//!
//! Exits non-zero if unexpected failures exceed ~1% of operations.

use std::{
    collections::HashSet,
    process::ExitCode,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use rand::RngExt;
use reqwest::Client;
use tokio::sync::Semaphore;

use ace_engine::settings::Settings;

const DEFAULT_DURATION_SECS: u64 = 600;
const MIN_BATCH: usize = 5;
const MAX_BATCH: usize = 20;
const MAX_IN_FLIGHT: usize = 4;
const MIN_GAP_MS: u64 = 20;
const MAX_GAP_MS: u64 = 100;
const NEGATIVE_EVERY: u64 = 7;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Operation counters shared across tasks.
#[derive(Default)]
struct Stats {
    /// Successful expected-2xx operations (create/list/delete).
    ok: AtomicU64,
    /// Unexpected statuses / transport errors on valid operations.
    failures: AtomicU64,
    /// Negative (invalid-input) operations issued.
    neg: AtomicU64,
    /// Negative operations that did NOT get a 4xx.
    neg_failures: AtomicU64,
}

/// Random sleep in the 20–100 ms window enforced between requests.
fn jitter_gap() -> Duration {
    Duration::from_millis(rand::rng().random_range(MIN_GAP_MS..=MAX_GAP_MS))
}

/// Discover the engine port: CLI override, then `port.lock`, then 10567.
fn resolve_port(override_port: Option<u16>) -> u16 {
    if let Some(port) = override_port {
        return port;
    }
    let lock = Settings::get_port_file_path();
    if let Ok(text) = std::fs::read_to_string(&lock)
        && let Ok(port) = text.trim().parse::<u16>()
    {
        return port;
    }
    10567
}

/// POST `/network-blocks` with a valid payload; returns the created block id.
async fn create_block(client: &Client, base: &str, name: &str, stats: &Stats) -> Option<String> {
    let payload = serde_json::json!({ "blockName": name });
    match client
        .post(format!("{base}/network-blocks"))
        .json(&payload)
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();
            if status.is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(body) => {
                        let id = body["networkBlock"]["id"].as_str().map(str::to_string);
                        if id.is_some() {
                            stats.ok.fetch_add(1, Ordering::Relaxed);
                        } else {
                            stats.failures.fetch_add(1, Ordering::Relaxed);
                            eprintln!(
                                "[unexpected] POST /network-blocks: 200 but no networkBlock.id in body"
                            );
                        }
                        id
                    }
                    Err(error) => {
                        stats.failures.fetch_add(1, Ordering::Relaxed);
                        eprintln!("[unexpected] POST /network-blocks: body parse failed: {error}");
                        None
                    }
                }
            } else {
                stats.failures.fetch_add(1, Ordering::Relaxed);
                eprintln!("[unexpected] POST /network-blocks ({name}) -> {status}");
                None
            }
        }
        Err(error) => {
            stats.failures.fetch_add(1, Ordering::Relaxed);
            eprintln!("[unexpected] POST /network-blocks ({name}) failed: {error}");
            None
        }
    }
}

/// GET `/network-blocks` and verify every id we created in this batch is listed.
async fn list_and_verify(client: &Client, base: &str, created: &[String], stats: &Stats) {
    match client.get(format!("{base}/network-blocks")).send().await {
        Ok(response) if response.status().is_success() => {
            let body: serde_json::Value = response.json().await.unwrap_or_default();
            let listed: HashSet<String> = body["networkBlocks"]
                .as_array()
                .map(|blocks| {
                    blocks
                        .iter()
                        .filter_map(|block| block["id"].as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            let missing = created.iter().filter(|id| !listed.contains(*id)).count();
            if missing == 0 {
                stats.ok.fetch_add(1, Ordering::Relaxed);
            } else {
                stats.failures.fetch_add(missing as u64, Ordering::Relaxed);
                eprintln!("[unexpected] GET /network-blocks: {missing} created blocks missing");
            }
        }
        Ok(response) => {
            stats.failures.fetch_add(1, Ordering::Relaxed);
            eprintln!("[unexpected] GET /network-blocks -> {}", response.status());
        }
        Err(error) => {
            stats.failures.fetch_add(1, Ordering::Relaxed);
            eprintln!("[unexpected] GET /network-blocks failed: {error}");
        }
    }
}

/// DELETE `/network-blocks/{id}`; 2xx is the only acceptable outcome.
async fn delete_block(client: &Client, base: &str, id: &str, stats: &Stats) {
    let url = format!("{base}/network-blocks/{id}");
    match client.delete(&url).send().await {
        Ok(response) if response.status().is_success() => {
            stats.ok.fetch_add(1, Ordering::Relaxed);
        }
        Ok(response) => {
            stats.failures.fetch_add(1, Ordering::Relaxed);
            eprintln!("[unexpected] DELETE {url} -> {}", response.status());
        }
        Err(error) => {
            stats.failures.fetch_add(1, Ordering::Relaxed);
            eprintln!("[unexpected] DELETE {url} failed: {error}");
        }
    }
}

/// Negative test: the engine MUST reject this request with a 4xx.
async fn expect_client_error(label: &str, request: reqwest::RequestBuilder, stats: &Stats) {
    stats.neg.fetch_add(1, Ordering::Relaxed);
    match request.send().await {
        Ok(response) if response.status().is_client_error() => {}
        Ok(response) => {
            stats.neg_failures.fetch_add(1, Ordering::Relaxed);
            eprintln!(
                "[negative] {label}: expected 4xx, got {}",
                response.status()
            );
        }
        Err(error) => {
            stats.neg_failures.fetch_add(1, Ordering::Relaxed);
            eprintln!("[negative] {label}: request error: {error}");
        }
    }
}

/// One full create → list → delete batch.
async fn run_batch(
    client: &Client,
    base: &str,
    semaphore: &Arc<Semaphore>,
    stats: &Arc<Stats>,
    batch_no: u64,
) {
    let batch_size = rand::rng().random_range(MIN_BATCH..=MAX_BATCH);
    let epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_secs();

    // --- create ---
    let mut tasks = Vec::new();
    for index in 0..batch_size {
        tokio::time::sleep(jitter_gap()).await;
        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("semaphore closed");
        let name = format!("stress-{epoch}-{batch_no}-{index}");
        let (client, base, stats) = (client.clone(), base.to_string(), stats.clone());
        tasks.push(tokio::spawn(async move {
            let _permit = permit;
            create_block(&client, &base, &name, &stats).await
        }));
    }
    let created: Vec<String> = futures::future::join_all(tasks)
        .await
        .into_iter()
        .filter_map(|result| result.ok().flatten())
        .collect();

    // --- list + verify ---
    tokio::time::sleep(jitter_gap()).await;
    list_and_verify(client, base, &created, stats).await;

    // --- delete ---
    let mut tasks = Vec::new();
    for id in &created {
        tokio::time::sleep(jitter_gap()).await;
        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("semaphore closed");
        let (client, base, id, stats) =
            (client.clone(), base.to_string(), id.clone(), stats.clone());
        tasks.push(tokio::spawn(async move {
            let _permit = permit;
            delete_block(&client, &base, &id, &stats).await
        }));
    }
    futures::future::join_all(tasks).await;

    // --- negative testing ---
    if batch_no.is_multiple_of(NEGATIVE_EVERY) {
        tokio::time::sleep(jitter_gap()).await;
        for payload in [
            serde_json::json!({}),                       // missing blockName
            serde_json::json!({ "blockName": 123 }),     // wrong type
            serde_json::json!({ "blockName": null }),    // null name
            serde_json::json!({ "unexpectedKey": "x" }), // unknown key only
        ] {
            tokio::time::sleep(jitter_gap()).await;
            expect_client_error(
                "POST /network-blocks invalid payload",
                client.post(format!("{base}/network-blocks")).json(&payload),
                stats,
            )
            .await;
        }

        // Malformed JSON body.
        tokio::time::sleep(jitter_gap()).await;
        expect_client_error(
            "POST /network-blocks malformed JSON",
            client
                .post(format!("{base}/network-blocks"))
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body("{not json"),
            stats,
        )
        .await;

        // Deleting a nonexistent block must 404.
        tokio::time::sleep(jitter_gap()).await;
        let fake_id = uuid::Uuid::now_v7().to_string();
        expect_client_error(
            "DELETE /network-blocks/{nonexistent}",
            client.delete(format!("{base}/network-blocks/{fake_id}")),
            stats,
        )
        .await;

        // A non-UUID path parameter must be rejected.
        tokio::time::sleep(jitter_gap()).await;
        expect_client_error(
            "GET /network-blocks/not-a-uuid",
            client.get(format!("{base}/network-blocks/not-a-uuid")),
            stats,
        )
        .await;
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut duration_secs = DEFAULT_DURATION_SECS;
    let mut port_override = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--duration" => {
                index += 1;
                duration_secs = args
                    .get(index)
                    .and_then(|value| value.parse().ok())
                    .filter(|secs| *secs > 0)
                    .unwrap_or_else(|| {
                        eprintln!("invalid --duration; using {DEFAULT_DURATION_SECS}s");
                        DEFAULT_DURATION_SECS
                    });
            }
            "--port" => {
                index += 1;
                port_override = args.get(index).and_then(|value| value.parse().ok());
            }
            other => {
                eprintln!("unknown argument: {other}");
                eprintln!("usage: stress-netblocks [--duration <secs>] [--port <port>]");
                return ExitCode::from(2);
            }
        }
        index += 1;
    }

    let port = resolve_port(port_override);
    let base = format!("https://127.0.0.1:{port}");
    let client = match Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(REQUEST_TIMEOUT)
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            eprintln!("failed to build HTTP client: {error}");
            return ExitCode::from(2);
        }
    };

    // Fail fast if the engine is not reachable.
    match client.get(format!("{base}/ping")).send().await {
        Ok(response) if response.status().is_success() => {
            println!("engine ping OK ({base})");
        }
        other => {
            eprintln!("engine not reachable at {base}: {other:?}");
            eprintln!("(is AceEngine running? check port.lock / --port)");
            return ExitCode::from(2);
        }
    }

    let duration = Duration::from_secs(duration_secs);
    let start = Instant::now();
    let stats = Arc::new(Stats::default());
    let semaphore = Arc::new(Semaphore::new(MAX_IN_FLIGHT));

    // Progress line every 30 seconds.
    {
        let (stats, start) = (stats.clone(), start);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                let elapsed = start.elapsed().as_secs();
                let ok = stats.ok.load(Ordering::Relaxed);
                let failures = stats.failures.load(Ordering::Relaxed);
                let neg = stats.neg.load(Ordering::Relaxed);
                let neg_failures = stats.neg_failures.load(Ordering::Relaxed);
                println!(
                    "[t+{elapsed:>4}s] ok={ok} failures={failures} neg={neg} neg_fail={neg_failures} rate={rate}/s",
                    rate = ok / elapsed.max(1)
                );
            }
        });
    }

    let mut batch_no: u64 = 0;
    while start.elapsed() < duration {
        batch_no += 1;
        run_batch(&client, &base, &semaphore, &stats, batch_no).await;
    }

    let elapsed = start.elapsed().as_secs();
    let ok = stats.ok.load(Ordering::Relaxed);
    let failures = stats.failures.load(Ordering::Relaxed);
    let neg = stats.neg.load(Ordering::Relaxed);
    let neg_failures = stats.neg_failures.load(Ordering::Relaxed);
    let unexpected = failures + neg_failures;
    let ops = ok + failures;

    println!("\n=== stress-netblocks summary ===");
    println!("duration: {elapsed}s (target {duration_secs}s)");
    println!("batches:  {batch_no}");
    println!("ok:       {ok}");
    println!("failures: {failures}");
    println!("neg:      {neg} (neg_failures: {neg_failures})");
    println!("rate:     {}/s", ok / elapsed.max(1));

    // Non-zero exit when unexpected failures exceed ~1% of operations.
    let failed = ops > 0 && unexpected * 100 > ops;
    if failed {
        eprintln!("FAILED: {unexpected} unexpected failures out of {ops} operations");
    } else {
        println!("PASS");
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
