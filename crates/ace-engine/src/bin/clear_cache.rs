use ace_engine::settings;
use app_dirs2::*;
use std::fs;
use std::io;
use std::path::PathBuf;

fn main() -> io::Result<()> {
    let app_data_dir = app_dir(AppDataType::SharedData, &settings::APP_INFO, "")
        .expect("Failed to get app data directory");

    if app_data_dir.exists() {
        println!("Cleaning cache directory: {:?}", app_data_dir);
        for entry in fs::read_dir(&app_data_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.file_name().is_some_and(|name| name == "certs") {
                println!("Skipping certs directory: {:?}", path);
                continue;
            }
            if path.is_dir() {
                fs::remove_dir_all(&path)?;
            } else {
                fs::remove_file(&path)?;
            }
        }
        println!("Cache cleared successfully (retained certs).");
    } else {
        println!("Cache directory not found, nothing to clear.");
    }

    if cfg!(debug_assertions) {
        let debug_db_path = PathBuf::from("../engine_debug.db");
        if debug_db_path.exists() {
            println!("Removing debug database: {:?}", debug_db_path);
            fs::remove_file(&debug_db_path)?;
            println!("Debug database removed successfully.");
        } else {
            println!("Debug database not found, nothing to clear.");
        }
    }

    Ok(())
}
