use ace_engine::service::api::bridge;
use ace_engine::service::api::rpc::RpcError;
use serde_json::Value;
use std::path::PathBuf;
use tauri::ipc::Channel;

//mod api;
//use api::ace_engine::*;
#[cfg(not(target_os = "linux"))]
use app_dirs2::*;
#[cfg(not(target_os = "linux"))]
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
    #[cfg(target_os = "linux")]
    {
        // The service must not disagree with this normal user's XDG_DATA_DIRS.
        Ok(PathBuf::from(ace_engine::settings::LINUX_SHARED_DATA_DIR).join("capability.token"))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let mut path = get_app_dir(AppDataType::SharedData, &APP_INFO, "")
            .map_err(|error| error.to_string())?;
        path.push("capability.token");
        Ok(path)
    }
}

// Never send the per-install capability back to the WebView. Credentials
// are read only by Rust and checked by the LocalSystem engine on each write.
fn read_engine_capability() -> Option<String> {
    get_capability_token_path()
        .ok()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .map(|value| value.trim().to_string())
}

// A WebView may invoke registered commands with arbitrary JSON, regardless of
// what the TypeScript SDK exports. Privileged service lifecycle belongs to the
// independent installer/maintenance client, never to the desktop UI.
fn authorize_webview_operation(op: &str) -> Result<(), RpcError> {
    if op == "shutdown" {
        return Err(RpcError {
            status: 403,
            code: "desktop_operation_forbidden".into(),
            error: "Engine shutdown is reserved for authorized maintenance".into(),
        });
    }
    Ok(())
}

#[tauri::command]
async fn engine_call(op: String, args: Value) -> Result<Value, RpcError> {
    authorize_webview_operation(&op)?;
    bridge::call(&op, args, read_engine_capability()).await
}

#[tauri::command]
async fn engine_stream(
    op: String,
    args: Value,
    on_event: Channel<ace_engine::core::network_block::site_pack::SitePackCreationStatus>,
) -> Result<(), RpcError> {
    authorize_webview_operation(&op)?;
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
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
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

#[cfg(test)]
mod tests {
    use super::authorize_webview_operation;

    #[test]
    fn webview_shutdown_is_denied_before_capability_or_pipe_connection() {
        let error = authorize_webview_operation("shutdown").unwrap_err();
        assert_eq!(error.status, 403);
        assert_eq!(error.code, "desktop_operation_forbidden");
    }

    #[test]
    fn ordinary_desktop_and_stream_operations_remain_forwardable() {
        assert!(authorize_webview_operation("ping").is_ok());
        assert!(authorize_webview_operation("createNetworkBlock").is_ok());
        assert!(authorize_webview_operation("createSitePack").is_ok());
        assert!(authorize_webview_operation("resyncSitePack").is_ok());
    }
}

#[cfg(all(test, target_os = "linux"))]
mod linux_shared_data_tests {
    #[test]
    fn app_and_engine_use_the_same_capability_file() {
        assert_eq!(
            super::get_capability_token_path().unwrap(),
            ace_engine::settings::Settings::get_capability_token_path()
        );
    }
}
