use ace_engine::core::network_block::site_pack::{
    importer::{
        Importer, SitePackCreationStatus, SitePackFormat, SitePackId, SitePackSource,
        SitePackSourceId, SitePackSourceType,
    },
    site_pack_db::SitePackDB,
};
use futures::StreamExt;
use std::{env, fs, time::Instant};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .without_time()
        .init();
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: site_pack_release_probe <file-or-url> <normal|staging> <label>".into());
    }
    let root = env::temp_dir().join(format!(
        "ace-release-benchmark-{}-{}",
        args[2],
        Uuid::new_v4()
    ));
    fs::create_dir_all(&root)?;
    let db_path = root.join("pack.db");
    let db = if args[1] == "staging" {
        SitePackDB::from_staging(&db_path)?
    } else {
        SitePackDB::from(&db_path)?
    };
    let id = SitePackId::try_new("bench-pack")?;
    let source = SitePackSource {
        id: SitePackSourceId::from(Uuid::new_v4()),
        site_pack_id: id.clone(),
        source: args[0].clone(),
        source_type: if args[0].starts_with("https://") {
            SitePackSourceType::Url
        } else {
            SitePackSourceType::File
        },
        format: SitePackFormat::Standard,
    };
    let started = Instant::now();
    let mut completed = false;
    let mut events = 0u64;
    let mut last_pct = 0.0f64;
    let mut stream = Importer::import_sources(&db, vec![source], root.clone(), id);
    while let Some(status) = stream.next().await {
        events += 1;
        match status {
            SitePackCreationStatus::Completed { .. } => completed = true,
            SitePackCreationStatus::Failed { phase, error } => {
                eprintln!("FAILED phase={phase:?} {error}");
                return Err("import failed".into());
            }
            SitePackCreationStatus::InProgress { progress, .. } => {
                let pct = progress.into_inner();
                if pct < last_pct {
                    return Err(format!("progress regressed: {last_pct} -> {pct}").into());
                }
                last_pct = pct;
            }
            _ => {}
        }
    }
    drop(stream);
    if !completed {
        return Err("missing Completed event".into());
    }
    let elapsed_ms = started.elapsed().as_millis();
    let count = db.count_entries()?;
    let metadata_count = db.get_entry_count()?;
    if count as i32 != metadata_count {
        return Err("metadata count mismatch".into());
    }
    if args[1] == "staging" {
        db.finish_staging(&db_path)?;
    }
    let db_size = fs::metadata(&db_path)?.len();
    println!(
        "BENCH_RESULT label={} mode={} elapsed_ms={} count={} metadata_count={} db_bytes={} events={} dir={}",
        args[2],
        args[1],
        elapsed_ms,
        count,
        metadata_count,
        db_size,
        events,
        root.display()
    );
    drop(db);
    Ok(())
}
