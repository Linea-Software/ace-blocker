use ace_engine::service::api::bridge;
use ace_engine::service::api::rpc::RpcError;
use serde_json::Value;
use std::path::PathBuf;
use tauri::ipc::Channel;

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

pub fn get_capability_token_path() -> Result<PathBuf, String> {
    let mut path =
        get_app_dir(AppDataType::SharedData, &APP_INFO, "").map_err(|error| error.to_string())?;
    path.push("capability.token");
    Ok(path)
}

// Never send the per-install capability back to the WebView. Credentials
// are read only by Rust and checked by the LocalSystem engine on each write.
fn read_engine_capability() -> Option<String> {
    get_capability_token_path()
        .ok()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .map(|value| value.trim().to_string())
}

#[tauri::command]
async fn engine_call(op: String, args: Value) -> Result<Value, RpcError> {
    bridge::call(&op, args, read_engine_capability()).await
}

#[tauri::command]
async fn engine_stream(
    op: String,
    args: Value,
    on_event: Channel<ace_engine::core::network_block::site_pack::SitePackCreationStatus>,
) -> Result<(), RpcError> {
    let mut receiver = bridge::stream(&op, args, read_engine_capability()).await?;
    loop {
        match receiver.recv().await {
            Ok(Some(event)) => on_event.send(event).map_err(|error| RpcError {
                status: 499,
                code: "client_disconnected".into(),
                error: error.to_string(),
            })?,
            Ok(None) => return Ok(()),
            Err(error) => {
                return Err(RpcError {
                    status: 502,
                    code: "stream_failed".into(),
                    error: error.to_string(),
                });
            }
        }
    }
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
            engine_call,
            engine_stream,
            app_version
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
