use std::path::PathBuf;

//mod api;
//use api::ace_engine::*;
use app_dirs2::*;
pub const APP_INFO: AppInfo = AppInfo {
    name: "AceBlocker",
    author: "LineaSoftware",
};

#[tauri::command]
fn is_debug_mode() -> bool {
    cfg!(debug_assertions)
}

#[tauri::command]
fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn get_port_file_path() -> Result<PathBuf, String> {
    let mut path =
        get_app_dir(AppDataType::SharedData, &APP_INFO, "").map_err(|error| error.to_string())?;
    path.push("port.lock");
    Ok(path)
}

pub fn get_capability_token_path() -> Result<PathBuf, String> {
    let mut path =
        get_app_dir(AppDataType::SharedData, &APP_INFO, "").map_err(|error| error.to_string())?;
    path.push("capability.token");
    Ok(path)
}

#[tauri::command]
fn get_engine_port() -> Result<u16, String> {
    // #[cfg(debug_assertions)]
    // {
    //     10567
    // }
    // #[cfg(not(debug_assertions))]
    // {
    let path = get_port_file_path()?;
    if let Ok(port_str) = std::fs::read_to_string(&path) {
        if let Ok(port) = port_str.trim().parse::<u16>() {
            return Ok(port);
        }
    }
    Err("Failed to get engine port".to_string())
    //10567
    // }
}

/// Reads the engine's per-install capability token. State-changing engine
/// API requests must carry it (`X-Ace-Capability`); see
/// `ace-engine/docs/local-api-protection-design.md`. Returns an error when
/// the file is missing (e.g. the engine has never started) — callers then
/// simply omit the header.
#[tauri::command]
fn get_engine_capability() -> Result<String, String> {
    let path = get_capability_token_path()?;
    std::fs::read_to_string(&path)
        .map(|token| token.trim().to_string())
        .map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build());

    #[cfg(debug_assertions)]
    {
        builder = builder.plugin(tauri_plugin_mcp_bridge::init());
    }

    builder
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(tauri_plugin_log::log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            is_debug_mode,
            get_engine_port,
            get_engine_capability,
            app_version
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
