//! Local API security middleware.
//!
//! Two layers protect the loopback HTTP API (design note:
//! `docs/local-api-protection-design.md`):
//!
//! 1. **Capability token** — every state-changing request
//!    (`POST`/`PUT`/`DELETE`/`PATCH`) must carry `X-Ace-Capability` matching
//!    the per-install token. Read-only routes (GET/HEAD/OPTIONS) stay open so
//!    health checks and settings reads keep working.
//! 2. **Origin allowlist** — CORS headers are only echoed for known app/site
//!    origins. Non-browser callers (no `Origin` header) are unaffected; the
//!    capability token is their gate.

use axum::{
    Json,
    extract::{Request, State},
    http::{HeaderName, HeaderValue, Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

use super::AppState;

/// Header carrying the per-install capability token.
pub const CAPABILITY_HEADER: HeaderName = HeaderName::from_static("x-ace-capability");

/// Origins that may read/write the local API from a browser context.
const ALLOWED_ORIGINS: [&str; 5] = [
    "tauri://localhost",
    "http://tauri.localhost",
    "https://aceblocker.com",
    "http://localhost:1420",
    "http://localhost:5173",
];

fn is_state_changing(method: &Method) -> bool {
    matches!(
        method,
        &Method::POST | &Method::PUT | &Method::DELETE | &Method::PATCH
    )
}

fn tokens_match(presented: &str, expected: &str) -> bool {
    // Compare SHA-256 digests: constant time in the input length and avoids
    // leaking the expected token through timing or length.
    use sha2::{Digest, Sha256};
    let presented_digest = hex::encode(Sha256::digest(presented.as_bytes()));
    let expected_digest = hex::encode(Sha256::digest(expected.as_bytes()));
    presented_digest == expected_digest
}

/// Rejects state-changing requests that lack (or mismatch) the capability
/// token. See the module docs for the debug-build policy.
pub async fn capability_check(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let Some(expected) = &state.capability_token else {
        // No token configured — enforcement is off (test servers).
        return next.run(request).await;
    };

    let method = request.method().clone();

    if !is_state_changing(&method) {
        return next.run(request).await;
    }

    let presented = request
        .headers()
        .get(&CAPABILITY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    match presented {
        Some(value) if tokens_match(&value, expected) => next.run(request).await,
        // Debug builds accept token-less requests so browser-based
        // development (no Tauri IPC to read the token file) keeps working —
        // same convention as the compile-gated `/dev/*` endpoints. A
        // presented-but-wrong token is still rejected.
        None if !state.enforce_capability => next.run(request).await,
        _ => error_response(
            StatusCode::UNAUTHORIZED,
            "capability_required",
            "This action requires a valid capability token",
        ),
    }
}

/// CORS policy: origin-allowlist for everything. Non-browser callers (no
/// `Origin` header) are unaffected; the capability token is their gate.
pub async fn cors_policy(request: Request, next: Next) -> Response {
    let origin = request.headers().get(header::ORIGIN).cloned();
    let method = request.method().clone();

    let origin_allowed = match &origin {
        Some(value) => value
            .to_str()
            .map(|s| ALLOWED_ORIGINS.contains(&s))
            .unwrap_or(false),
        None => true, // non-browser caller; token gates mutations
    };

    if !origin_allowed {
        // Do not echo CORS headers for unknown origins: browsers will block
        // reading the response, and the capability token blocks writes.
        return next.run(request).await;
    }

    if method == Method::OPTIONS {
        // Preflight: answer without reaching the router.
        let mut response = Response::new(axum::body::Body::empty());
        *response.status_mut() = StatusCode::NO_CONTENT;
        let headers = response.headers_mut();
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_ORIGIN,
            origin
                .clone()
                .unwrap_or_else(|| HeaderValue::from_static("*")),
        );
        headers.insert(header::VARY, HeaderValue::from_static("Origin"));
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_METHODS,
            HeaderValue::from_static("GET, POST, PUT, DELETE, OPTIONS"),
        );
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            HeaderValue::from_static("content-type, x-ace-capability"),
        );
        headers.insert(
            header::ACCESS_CONTROL_MAX_AGE,
            HeaderValue::from_static("600"),
        );
        return response;
    }

    let mut response = next.run(request).await;
    if let Some(origin) = origin {
        let headers = response.headers_mut();
        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
        headers.insert(header::VARY, HeaderValue::from_static("Origin"));
    }
    response
}

fn error_response(status: StatusCode, code: &'static str, message: &str) -> Response {
    (
        status,
        Json(serde_json::json!({ "error": message, "code": code })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::api::AppState;
    use axum_test::TestServer;
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::TempDir;

    /// Builds a full test server with the real router and a capability token
    /// so the security layers are exercised end to end (token + CORS).
    fn test_server(token: Option<&str>) -> (TempDir, TestServer) {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("test.sqlite");
        // One shared process-wide sites dir for all unit tests (TESTINFRA-01).
        crate::settings::Settings::set_test_sites_dir(
            crate::settings::Settings::shared_test_sites_dir(),
        );

        let db = Arc::new(crate::db::EngineDB::from(&db_path).expect("failed to open test DB"));
        let account_manager =
            crate::manager::AccountManager::from(db.clone()).expect("account manager");

        let state = AppState {
            network_manager: Arc::new(tokio::sync::RwLock::new(
                crate::manager::NetworkManager::new_empty(db.clone(), account_manager.clone()),
            )),
            account_manager,
            content_update_service: Arc::new(tokio::sync::Mutex::new(
                crate::service::content_updater::ContentUpdateService::from(db.clone()),
            )),
            update_check_interval: Arc::new(tokio::sync::RwLock::new(Duration::from_secs(
                24 * 60 * 60,
            ))),
            shutdown_tx: tokio::sync::watch::channel(false).0,
            unlock_challenges: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            capability_token: token.map(str::to_string),
            enforce_capability: token.is_some(),
        };

        let server = TestServer::builder()
            .transport(axum_test::Transport::HttpRandomPort)
            .build(crate::service::api::create_router(state));
        (temp, server)
    }

    #[tokio::test]
    async fn test_state_changing_without_token_is_rejected() {
        let (_temp, server) = test_server(Some("expected-token"));
        let response = server
            .post("/resistance-mode/set-lock")
            .json(&serde_json::json!({ "lockState": "inactive" }))
            .await;
        assert_eq!(response.status_code(), 401);
        let json: serde_json::Value = response.json();
        assert_eq!(json["code"], "capability_required");
    }

    #[tokio::test]
    async fn test_state_changing_with_wrong_token_is_rejected() {
        let (_temp, server) = test_server(Some("expected-token"));
        let response = server
            .post("/resistance-mode/set-lock")
            .add_header("x-ace-capability", "wrong-token")
            .json(&serde_json::json!({ "lockState": "inactive" }))
            .await;
        assert_eq!(response.status_code(), 401);
    }

    #[tokio::test]
    async fn test_state_changing_with_valid_token_succeeds() {
        let (_temp, server) = test_server(Some("expected-token"));
        let response = server
            .post("/resistance-mode/set-enabled")
            .add_header("x-ace-capability", "expected-token")
            .json(&serde_json::json!({ "enabled": true }))
            .await;
        assert_eq!(response.status_code(), 200);
    }

    #[tokio::test]
    async fn test_read_only_health_works_without_token() {
        let (_temp, server) = test_server(Some("expected-token"));
        let response = server.get("/ping").await;
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.text(), "pong");
    }

    #[tokio::test]
    async fn test_unknown_origin_gets_no_cors_headers() {
        let (_temp, server) = test_server(Some("expected-token"));
        let response = server
            .get("/resistance-mode/settings")
            .add_header("origin", "https://evil.example")
            .await;
        assert!(
            response
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .is_none(),
            "unknown origin must not receive CORS headers"
        );
    }

    #[tokio::test]
    async fn test_known_origin_gets_cors_headers() {
        let (_temp, server) = test_server(Some("expected-token"));
        let response = server
            .get("/resistance-mode/settings")
            .add_header("origin", "tauri://localhost")
            .await;
        assert_eq!(
            response
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .unwrap(),
            "tauri://localhost"
        );
    }

    #[tokio::test]
    async fn test_arbitrary_web_origin_cannot_create_a_lock() {
        // Drive-by scenario: a malicious page tries to create a resistance
        // lock. It has no capability token → 401, no state change.
        let (_temp, server) = test_server(Some("expected-token"));
        let later = (chrono::Local::now() + chrono::Duration::hours(1))
            .format(crate::core::constants::DATETIME_FORMAT)
            .to_string();
        let response = server
            .post("/resistance-mode/set-lock")
            .add_header("origin", "https://evil.example")
            .json(&serde_json::json!({
                "lockState": "active_until_lock_end",
                "lockEnd": later
            }))
            .await;
        assert_eq!(response.status_code(), 401, "drive-by lock must fail");

        // And even the preflight for that origin is refused. axum-test has no
        // OPTIONS helper, so send the preflight over plain reqwest to the
        // test server's real HTTP socket.
        let url = server
            .server_url("/resistance-mode/set-lock")
            .expect("server url");
        let preflight = reqwest::Client::new()
            .request(reqwest::Method::OPTIONS, url)
            .header("origin", "https://evil.example")
            .send()
            .await
            .expect("preflight request");
        assert!(
            preflight
                .headers()
                .get("access-control-allow-origin")
                .is_none(),
            "preflight for unknown origin must not be granted"
        );
        assert!(
            preflight
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .is_none(),
            "preflight for unknown origin must not be granted"
        );
    }

    #[tokio::test]
    async fn test_no_token_configured_means_no_enforcement() {
        // Test-server configuration (no token) must not block state changes.
        let (_temp, server) = test_server(None);
        let response = server
            .post("/resistance-mode/set-enabled")
            .json(&serde_json::json!({ "enabled": true }))
            .await;
        assert_eq!(response.status_code(), 200);
    }
}
