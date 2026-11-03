use ace_engine::core::network_block::site_pack::{SitePack, importer::SitePackId};
use std::{env, fs, time::Instant};
use uuid::Uuid;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: site_pack_restore_probe <release-zip> <known-blocked-domain>".into());
    }
    let root = env::temp_dir().join(format!("ace-real-restore-{}", Uuid::new_v4()));
    fs::create_dir_all(&root)?;
    let pack_dir = root.join("pack-adult-pro");
    let id = SitePackId::try_new("pack-adult-pro")?;
    let mut pack = SitePack::from(
        id,
        None,
        "Adult Pro".into(),
        None,
        true,
        None,
        None,
        &pack_dir,
        vec![],
        None,
    )?;
    let started = Instant::now();
    pack.restore_from_backup(args[0].clone().into())?;
    let elapsed = started.elapsed();
    let count = pack.get_entry_count()?;
    let result = pack.try_blocking_result(&args[1])?;
    let file_bytes = fs::metadata(pack_dir.join("pack.db"))?.len();
    println!(
        "RESTORE_RESULT duration_ms={} count={} db_bytes={} sample_host={:?} block_result={:?}",
        elapsed.as_millis(),
        count,
        file_bytes,
        args[1],
        result
    );
    if count != 2644748 {
        return Err(format!("unexpected count: {count}").into());
    }
    if !result.is_blocked() {
        return Err("sample host not blocked".into());
    }
    Ok(())
}
