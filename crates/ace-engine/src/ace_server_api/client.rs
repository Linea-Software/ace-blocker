//! Generated HTTP client for regular API requests
//!
//! This file contains the HTTP client implementation for GET, POST, etc.
//! Do not edit manually - regenerate using the appropriate script.
#![allow(clippy::format_in_format_args)]
#![allow(clippy::let_unit_value)]
use super::types::*;
use thiserror::Error;
/// The generated validation-problem profile based on RFC 9457.
/// The distinctive namespace avoids collisions with user schemas.
pub mod openapi_to_rust_problem {
    #[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
    pub struct ProblemDetails {
        #[serde(rename = "type")]
        pub type_uri: String,
        pub title: String,
        pub status: u16,
        pub code: String,
        #[serde(default)]
        pub errors: Vec<InvalidParameter>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub detail: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub instance: Option<String>,
    }
    #[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
    pub struct InvalidParameter {
        pub code: String,
        pub location: String,
        pub message: String,
    }
}
/// Transport-level errors: failures where no safely inspectable
/// HTTP response is available to the caller.
///
/// HTTP responses with non-2xx status codes are surfaced as
/// [`ApiError`] inside [`ApiOpError::Api`], not here, so callers can
/// always inspect status, headers, and the raw body when the server
/// actually responded.
#[derive(Error, Debug)]
pub enum HttpError {
    /// Network or connection error (from reqwest)
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    /// Middleware error (from reqwest-middleware)
    #[error("Middleware error: {0}")]
    Middleware(#[from] reqwest_middleware::Error),
    /// Request serialization error
    #[error("Failed to serialize request: {0}")]
    Serialization(String),
    /// Authentication error
    #[error("Authentication error: {0}")]
    Auth(String),
    /// Request timeout
    #[error("Request timeout")]
    Timeout,
    /// A response body exceeded the configured in-memory limit
    #[error("Response body exceeded configured limit of {limit} bytes")]
    ResponseTooLarge { limit: usize },
    /// Invalid configuration
    #[error("Configuration error: {0}")]
    Config(String),
    /// Generic error
    #[error("{0}")]
    Other(String),
}
impl HttpError {
    /// Create a serialization error
    pub fn serialization_error(error: impl std::fmt::Display) -> Self {
        Self::Serialization(error.to_string())
    }
    /// Check if this transport error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Network(_) | Self::Middleware(_) | Self::Timeout)
    }
}
/// Envelope returned for any HTTP response that we received but
/// couldn't (or didn't) treat as a successful typed result.
///
/// Includes both non-2xx responses and 2xx responses whose body
/// failed to deserialize into the expected success type. `status`,
/// `headers`, and `raw_body` preserve what the server actually sent,
/// while `body` is a convenient lossy UTF-8 rendering. `typed`
/// carries the parsed per-operation error variant
/// when the body matched a declared schema. Formatting the error
/// limits only the displayed body preview; the public fields
/// retain the complete response and parsing details.
#[derive(Debug, Clone)]
pub struct ApiError<E> {
    pub status: u16,
    pub headers: reqwest::header::HeaderMap,
    pub body: String,
    /// Exact response bytes before lossy UTF-8 conversion.
    pub raw_body: Vec<u8>,
    pub typed: Option<E>,
    pub parse_error: Option<String>,
}
const API_ERROR_BODY_DISPLAY_LIMIT: usize = 500;
const API_ERROR_BODY_TRUNCATION_MARKER: &str = "... [truncated]";
fn display_api_error_body(body: &str) -> std::borrow::Cow<'_, str> {
    let Some((end, _)) = body.char_indices().nth(API_ERROR_BODY_DISPLAY_LIMIT) else {
        return std::borrow::Cow::Borrowed(body);
    };
    let mut displayed = String::with_capacity(
        end + API_ERROR_BODY_TRUNCATION_MARKER.len(),
    );
    displayed.push_str(&body[..end]);
    displayed.push_str(API_ERROR_BODY_TRUNCATION_MARKER);
    std::borrow::Cow::Owned(displayed)
}
impl<E> ApiError<E> {
    pub fn is_client_error(&self) -> bool {
        (400..500).contains(&self.status)
    }
    pub fn is_server_error(&self) -> bool {
        (500..600).contains(&self.status)
    }
    /// Retry guidance for the response. Mirrors the previous
    /// HttpError logic for backwards-compatible retry middleware.
    pub fn is_retryable(&self) -> bool {
        matches!(self.status, 429 | 500 | 502 | 503 | 504)
    }
    /// Decode the generated RFC 9457 validation-problem profile
    /// without replacing a documented per-operation error in `typed`.
    ///
    /// Returns `None` unless the response's `Content-Type` is
    /// `application/problem+json`, which is how RFC 9457 identifies
    /// a problem document. Most third-party APIs return their
    /// errors as plain `application/json`, so this yields `None`
    /// against them by design — use `typed` for a documented
    /// per-operation error body, or `body` for the raw payload.
    /// Servers generated by this tool always emit the problem
    /// media type, so this succeeds against them.
    pub fn problem_details(&self) -> Option<openapi_to_rust_problem::ProblemDetails> {
        let content_type = self
            .headers
            .get(reqwest::header::CONTENT_TYPE)?
            .to_str()
            .ok()?;
        let media_type = content_type.split(';').next()?.trim();
        if !media_type.eq_ignore_ascii_case("application/problem+json") {
            return None;
        }
        serde_json::from_str(&self.body).ok()
    }
}
impl<E: std::fmt::Debug> std::fmt::Display for ApiError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "API error {}: {}", self.status, display_api_error_body(& self.body))?;
        if let Some(typed) = &self.typed {
            write!(f, "; typed: {typed:?}")?;
        }
        if let Some(parse_error) = &self.parse_error {
            write!(f, "; parse error: {parse_error}")?;
        }
        Ok(())
    }
}
impl<E: std::fmt::Debug> std::error::Error for ApiError<E> {}
/// Result error type returned by every generated operation method.
///
/// `Transport` covers failures where we never got an inspectable
/// response (network, timeout, middleware, request-side
/// serialization). `Api` covers any case where the server *did*
/// respond — the envelope always carries status + headers + raw
/// body even when the typed deserialize fails.
#[derive(Debug, Error)]
pub enum ApiOpError<E: std::fmt::Debug> {
    #[error(transparent)]
    Transport(#[from] HttpError),
    #[error(transparent)]
    Api(ApiError<E>),
}
impl<E: std::fmt::Debug> ApiOpError<E> {
    /// Returns the API envelope when this is an `Api` variant.
    pub fn api(&self) -> Option<&ApiError<E>> {
        match self {
            Self::Api(e) => Some(e),
            Self::Transport(_) => None,
        }
    }
    /// True when the underlying error came from the server (i.e.
    /// any `Api` variant) rather than the transport layer.
    pub fn is_api_error(&self) -> bool {
        matches!(self, Self::Api(_))
    }
}
impl<E: std::fmt::Debug> From<reqwest::Error> for ApiOpError<E> {
    fn from(e: reqwest::Error) -> Self {
        Self::Transport(HttpError::Network(e))
    }
}
impl<E: std::fmt::Debug> From<reqwest_middleware::Error> for ApiOpError<E> {
    fn from(e: reqwest_middleware::Error) -> Self {
        Self::Transport(HttpError::Middleware(e))
    }
}
/// Result alias for transport-only error paths (e.g. helpers that
/// don't have a per-operation error type). Generated operation
/// methods use [`ApiOpError`] directly.
pub type HttpResult<T> = Result<T, HttpError>;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use std::collections::BTreeMap;
/// Default upper bound for any response body buffered in memory.
pub const DEFAULT_MAX_RESPONSE_BODY_BYTES: usize = 8 * 1024 * 1024;
/// HTTP client for making API requests
#[derive(Clone)]
pub struct HttpClient {
    base_url: String,
    api_key: Option<String>,
    http_client: ClientWithMiddleware,
    custom_headers: BTreeMap<String, String>,
    max_response_body_bytes: usize,
}
async fn __read_bounded_response_body(
    mut response: reqwest::Response,
    limit: usize,
) -> Result<Vec<u8>, HttpError> {
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(HttpError::Network)? {
        let next_len = body.len().checked_add(chunk.len());
        if next_len.is_none_or(|next_len| next_len > limit) {
            return Err(HttpError::ResponseTooLarge {
                limit,
            });
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}
impl HttpClient {
    /// Create a new HTTP client with default configuration
    pub fn new() -> Self {
        Self::with_config(true)
    }
    /// Create a new HTTP client with custom configuration
    pub fn with_config(enable_tracing: bool) -> Self {
        let reqwest_client = reqwest::Client::new();
        let mut client_builder = ClientBuilder::new(reqwest_client);
        if enable_tracing {
            use reqwest_tracing::TracingMiddleware;
            client_builder = client_builder.with(TracingMiddleware::default());
        }
        let http_client = client_builder.build();
        Self {
            base_url: "".to_string(),
            api_key: None,
            http_client,
            custom_headers: BTreeMap::new(),
            max_response_body_bytes: 8388608usize,
        }
    }
    /// Set the base URL for all requests
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }
    /// Set the API key for authentication
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }
    /// Set the maximum number of response-body bytes buffered in memory.
    pub fn with_max_response_body_bytes(mut self, limit: usize) -> Self {
        self.max_response_body_bytes = limit;
        self
    }
    /// Add a custom header to all requests
    pub fn with_header(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.custom_headers.insert(name.into(), value.into());
        self
    }
    /// Add multiple custom headers
    pub fn with_headers(mut self, headers: BTreeMap<String, String>) -> Self {
        self.custom_headers.extend(headers);
        self
    }
}
impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}
fn __pct_encode_path_segment(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{:02X}", b));
            }
        }
    }
    out
}
///Typed error responses for `deleteInstaller`. One variant per declared non-2xx response.
#[derive(Debug, Clone)]
pub enum DeleteInstallerApiError {
    Status400(DeleteInstallerResponse400),
    Status500(DeleteInstallerResponse500),
}
///Typed error responses for `deleteResistanceScript`. One variant per declared non-2xx response.
#[derive(Debug, Clone)]
pub enum DeleteResistanceScriptApiError {
    Status400(DeleteResistanceScriptResponse400),
    Status500(DeleteResistanceScriptResponse500),
}
///Typed error responses for `deleteSitePack`. One variant per declared non-2xx response.
#[derive(Debug, Clone)]
pub enum DeleteSitePackApiError {
    Status400(DeleteSitePackResponse400),
    Status500(DeleteSitePackResponse500),
}
///Typed error responses for `setInstallerDownloadable`. One variant per declared non-2xx response.
#[derive(Debug, Clone)]
pub enum SetInstallerDownloadableApiError {
    Status400(SetInstallerDownloadableResponse400),
    Status404(SetInstallerDownloadableResponse404),
    Status500(SetInstallerDownloadableResponse500),
}
///Typed error responses for `setResistanceScriptDownloadable`. One variant per declared non-2xx response.
#[derive(Debug, Clone)]
pub enum SetResistanceScriptDownloadableApiError {
    Status400(SetResistanceScriptDownloadableResponse400),
    Status404(SetResistanceScriptDownloadableResponse404),
    Status500(SetResistanceScriptDownloadableResponse500),
}
///Typed error responses for `updateSitePack`. One variant per declared non-2xx response.
#[derive(Debug, Clone)]
pub enum UpdateSitePackApiError {
    Status400(UpdateSitePackResponse400),
    Status500(UpdateSitePackResponse500),
}
///Typed error responses for `uploadInstaller`. One variant per declared non-2xx response.
#[derive(Debug, Clone)]
pub enum UploadInstallerApiError {
    Status400(UploadInstallerResponse400),
    Status500(UploadInstallerResponse500),
}
///Typed error responses for `uploadResistanceScript`. One variant per declared non-2xx response.
#[derive(Debug, Clone)]
pub enum UploadResistanceScriptApiError {
    Status400(UploadResistanceScriptResponse400),
    Status500(UploadResistanceScriptResponse500),
}
///Typed error responses for `uploadSitePack`. One variant per declared non-2xx response.
#[derive(Debug, Clone)]
pub enum UploadSitePackApiError {
    Status400(UploadSitePackResponse400),
    Status500(UploadSitePackResponse500),
}
impl HttpClient {
    /// Cancel the user Pro subscription
    ///
    /// `POST /api/v1/account/cancel-subscription`
    pub async fn cancel_subscription(
        &self,
    ) -> Result<CancelSubscriptionResponse, ApiOpError<serde_json::Value>> {
        let request_url = format!(
            "{}{}", self.base_url, "/api/v1/account/cancel-subscription"
        );
        let mut req = self.http_client.post(request_url);
        req = req.header(reqwest::header::CONTENT_LENGTH, "0");
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<serde_json::Value>;
            let parse_error: Option<String>;
            match status_code {
                _ => {
                    match serde_json::from_str::<serde_json::Value>(&body_text) {
                        Ok(v) => {
                            typed = Some(v);
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// Generate API key for desktop app linking
    ///
    /// `GET /api/v1/account/api-key/create`
    pub async fn create_api_key(
        &self,
    ) -> Result<CreateApiKeyResponse, ApiOpError<serde_json::Value>> {
        let request_url = format!(
            "{}{}", self.base_url, "/api/v1/account/api-key/create"
        );
        let mut req = self.http_client.get(request_url);
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<serde_json::Value>;
            let parse_error: Option<String>;
            match status_code {
                _ => {
                    match serde_json::from_str::<serde_json::Value>(&body_text) {
                        Ok(v) => {
                            typed = Some(v);
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// Create a new engine backup
    ///
    /// `POST /api/v1/backups/`
    pub async fn create_backup(
        &self,
        request: CreateBackupRequest,
    ) -> Result<CreateBackupResponse, ApiOpError<serde_json::Value>> {
        let request_url = format!("{}{}", self.base_url, "/api/v1/backups/");
        let mut req = self.http_client.post(request_url);
        req = req
            .body(serde_json::to_vec(&request).map_err(HttpError::serialization_error)?)
            .header("content-type", "application/json");
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<serde_json::Value>;
            let parse_error: Option<String>;
            match status_code {
                _ => {
                    match serde_json::from_str::<serde_json::Value>(&body_text) {
                        Ok(v) => {
                            typed = Some(v);
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// Permanently delete user account and all associated data
    ///
    /// `POST /api/v1/account/delete`
    pub async fn delete_account(
        &self,
        request: DeleteAccountRequest,
    ) -> Result<DeleteAccountResponse, ApiOpError<serde_json::Value>> {
        let request_url = format!("{}{}", self.base_url, "/api/v1/account/delete");
        let mut req = self.http_client.post(request_url);
        req = req
            .body(serde_json::to_vec(&request).map_err(HttpError::serialization_error)?)
            .header("content-type", "application/json");
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<serde_json::Value>;
            let parse_error: Option<String>;
            match status_code {
                _ => {
                    match serde_json::from_str::<serde_json::Value>(&body_text) {
                        Ok(v) => {
                            typed = Some(v);
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// Delete an installer
    ///
    /// `POST /api/v1/admin/installers/delete`
    pub async fn delete_installer(
        &self,
        request: DeleteInstallerRequest,
    ) -> Result<DeleteInstallerResponse, ApiOpError<DeleteInstallerApiError>> {
        let request_url = format!(
            "{}{}", self.base_url, "/api/v1/admin/installers/delete"
        );
        let mut req = self.http_client.post(request_url);
        req = req
            .body(serde_json::to_vec(&request).map_err(HttpError::serialization_error)?)
            .header("content-type", "application/json");
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<DeleteInstallerApiError>;
            let parse_error: Option<String>;
            match status_code {
                400u16 => {
                    match serde_json::from_str::<
                        DeleteInstallerResponse400,
                    >(&body_text) {
                        Ok(v) => {
                            typed = Some(DeleteInstallerApiError::Status400(v));
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
                500u16 => {
                    match serde_json::from_str::<
                        DeleteInstallerResponse500,
                    >(&body_text) {
                        Ok(v) => {
                            typed = Some(DeleteInstallerApiError::Status500(v));
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
                _ => {
                    typed = None;
                    parse_error = None;
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// Delete a resistance mode script
    ///
    /// `POST /api/v1/admin/resistance/delete`
    pub async fn delete_resistance_script(
        &self,
        request: DeleteResistanceScriptRequest,
    ) -> Result<
        DeleteResistanceScriptResponse,
        ApiOpError<DeleteResistanceScriptApiError>,
    > {
        let request_url = format!(
            "{}{}", self.base_url, "/api/v1/admin/resistance/delete"
        );
        let mut req = self.http_client.post(request_url);
        req = req
            .body(serde_json::to_vec(&request).map_err(HttpError::serialization_error)?)
            .header("content-type", "application/json");
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<DeleteResistanceScriptApiError>;
            let parse_error: Option<String>;
            match status_code {
                400u16 => {
                    match serde_json::from_str::<
                        DeleteResistanceScriptResponse400,
                    >(&body_text) {
                        Ok(v) => {
                            typed = Some(DeleteResistanceScriptApiError::Status400(v));
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
                500u16 => {
                    match serde_json::from_str::<
                        DeleteResistanceScriptResponse500,
                    >(&body_text) {
                        Ok(v) => {
                            typed = Some(DeleteResistanceScriptApiError::Status500(v));
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
                _ => {
                    typed = None;
                    parse_error = None;
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// Delete a site pack
    ///
    /// `POST /api/v1/admin/site-packs/delete`
    pub async fn delete_site_pack(
        &self,
        request: DeleteSitePackRequest,
    ) -> Result<DeleteSitePackResponse, ApiOpError<DeleteSitePackApiError>> {
        let request_url = format!(
            "{}{}", self.base_url, "/api/v1/admin/site-packs/delete"
        );
        let mut req = self.http_client.post(request_url);
        req = req
            .body(serde_json::to_vec(&request).map_err(HttpError::serialization_error)?)
            .header("content-type", "application/json");
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<DeleteSitePackApiError>;
            let parse_error: Option<String>;
            match status_code {
                400u16 => {
                    match serde_json::from_str::<DeleteSitePackResponse400>(&body_text) {
                        Ok(v) => {
                            typed = Some(DeleteSitePackApiError::Status400(v));
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
                500u16 => {
                    match serde_json::from_str::<DeleteSitePackResponse500>(&body_text) {
                        Ok(v) => {
                            typed = Some(DeleteSitePackApiError::Status500(v));
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
                _ => {
                    typed = None;
                    parse_error = None;
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// Export user account data as JSON download
    ///
    /// `GET /api/v1/account/export`
    pub async fn export_account_data(
        &self,
    ) -> Result<ExportAccountDataResponse, ApiOpError<serde_json::Value>> {
        let request_url = format!("{}{}", self.base_url, "/api/v1/account/export");
        let mut req = self.http_client.get(request_url);
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<serde_json::Value>;
            let parse_error: Option<String>;
            match status_code {
                _ => {
                    match serde_json::from_str::<serde_json::Value>(&body_text) {
                        Ok(v) => {
                            typed = Some(v);
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// Fetch the latest engine backup
    ///
    /// `GET /api/v1/backups/`
    pub async fn fetch_backup(
        &self,
    ) -> Result<FetchBackupResponse, ApiOpError<serde_json::Value>> {
        let request_url = format!("{}{}", self.base_url, "/api/v1/backups/");
        let mut req = self.http_client.get(request_url);
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<serde_json::Value>;
            let parse_error: Option<String>;
            match status_code {
                _ => {
                    match serde_json::from_str::<serde_json::Value>(&body_text) {
                        Ok(v) => {
                            typed = Some(v);
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// Get current user account details
    ///
    /// `GET /api/v1/account/`
    pub async fn get_account_details(
        &self,
    ) -> Result<GetAccountDetailsResponse, ApiOpError<serde_json::Value>> {
        let request_url = format!("{}{}", self.base_url, "/api/v1/account/");
        let mut req = self.http_client.get(request_url);
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<serde_json::Value>;
            let parse_error: Option<String>;
            match status_code {
                _ => {
                    match serde_json::from_str::<serde_json::Value>(&body_text) {
                        Ok(v) => {
                            typed = Some(v);
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// Get all installers with download stats
    ///
    /// `GET /api/v1/admin/installers/`
    pub async fn get_installers(
        &self,
    ) -> Result<GetInstallersResponse, ApiOpError<serde_json::Value>> {
        let request_url = format!("{}{}", self.base_url, "/api/v1/admin/installers/");
        let mut req = self.http_client.get(request_url);
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<serde_json::Value>;
            let parse_error: Option<String>;
            match status_code {
                _ => {
                    match serde_json::from_str::<serde_json::Value>(&body_text) {
                        Ok(v) => {
                            typed = Some(v);
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// Get metadata for the latest software releases directly from cache
    ///
    /// `GET /api/v1/releases/latest`
    pub async fn get_latest_releases_version_data(
        &self,
    ) -> Result<GetLatestReleasesVersionDataResponse, ApiOpError<serde_json::Value>> {
        let request_url = format!("{}{}", self.base_url, "/api/v1/releases/latest");
        let mut req = self.http_client.get(request_url);
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<serde_json::Value>;
            let parse_error: Option<String>;
            match status_code {
                _ => {
                    match serde_json::from_str::<serde_json::Value>(&body_text) {
                        Ok(v) => {
                            typed = Some(v);
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// Get user pro status
    ///
    /// `GET /api/v1/account/pro`
    pub async fn get_pro_details(
        &self,
    ) -> Result<GetProDetailsResponse, ApiOpError<serde_json::Value>> {
        let request_url = format!("{}{}", self.base_url, "/api/v1/account/pro");
        let mut req = self.http_client.get(request_url);
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<serde_json::Value>;
            let parse_error: Option<String>;
            match status_code {
                _ => {
                    match serde_json::from_str::<serde_json::Value>(&body_text) {
                        Ok(v) => {
                            typed = Some(v);
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// Get download URL for resistance mode scripts
    ///
    /// `GET /api/v1/scripts/resistance-mode`
    pub async fn get_resistance_mode_scripts(
        &self,
    ) -> Result<GetResistanceModeScriptsResponse, ApiOpError<serde_json::Value>> {
        let request_url = format!(
            "{}{}", self.base_url, "/api/v1/scripts/resistance-mode"
        );
        let mut req = self.http_client.get(request_url);
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<serde_json::Value>;
            let parse_error: Option<String>;
            match status_code {
                _ => {
                    match serde_json::from_str::<serde_json::Value>(&body_text) {
                        Ok(v) => {
                            typed = Some(v);
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// Get all resistance mode scripts with download stats
    ///
    /// `GET /api/v1/admin/resistance/`
    pub async fn get_resistance_scripts(
        &self,
    ) -> Result<GetResistanceScriptsResponse, ApiOpError<serde_json::Value>> {
        let request_url = format!("{}{}", self.base_url, "/api/v1/admin/resistance/");
        let mut req = self.http_client.get(request_url);
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<serde_json::Value>;
            let parse_error: Option<String>;
            match status_code {
                _ => {
                    match serde_json::from_str::<serde_json::Value>(&body_text) {
                        Ok(v) => {
                            typed = Some(v);
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// Get download URL for a site pack backup
    ///
    /// `GET /api/v1/site-packs/{sitePackId}/backup`
    pub async fn get_site_pack_backup(
        &self,
        site_pack_id: impl AsRef<str>,
    ) -> Result<GetSitePackBackupResponse, ApiOpError<serde_json::Value>> {
        let request_url = format!(
            "{}{}", self.base_url, format!("/api/v1/site-packs/{}/backup",
            __pct_encode_path_segment(site_pack_id.as_ref()))
        );
        let mut req = self.http_client.get(request_url);
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<serde_json::Value>;
            let parse_error: Option<String>;
            match status_code {
                _ => {
                    match serde_json::from_str::<serde_json::Value>(&body_text) {
                        Ok(v) => {
                            typed = Some(v);
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// Get all site packs with download stats
    ///
    /// `GET /api/v1/admin/site-packs/`
    pub async fn get_site_packs(
        &self,
    ) -> Result<GetSitePacksResponse, ApiOpError<serde_json::Value>> {
        let request_url = format!("{}{}", self.base_url, "/api/v1/admin/site-packs/");
        let mut req = self.http_client.get(request_url);
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<serde_json::Value>;
            let parse_error: Option<String>;
            match status_code {
                _ => {
                    match serde_json::from_str::<serde_json::Value>(&body_text) {
                        Ok(v) => {
                            typed = Some(v);
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// List user API keys
    ///
    /// `GET /api/v1/account/api-key/list`
    pub async fn list_api_keys(
        &self,
    ) -> Result<ListApiKeysResponse, ApiOpError<serde_json::Value>> {
        let request_url = format!("{}{}", self.base_url, "/api/v1/account/api-key/list");
        let mut req = self.http_client.get(request_url);
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<serde_json::Value>;
            let parse_error: Option<String>;
            match status_code {
                _ => {
                    match serde_json::from_str::<serde_json::Value>(&body_text) {
                        Ok(v) => {
                            typed = Some(v);
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// `POST /api/v1/admin/users/delete`
    pub async fn post_api_v1_admin_users_delete(
        &self,
        request: PostApiV1AdminUsersDeleteRequest,
    ) -> Result<(), ApiOpError<serde_json::Value>> {
        let request_url = format!("{}{}", self.base_url, "/api/v1/admin/users/delete");
        let mut req = self.http_client.post(request_url);
        req = req
            .body(serde_json::to_vec(&request).map_err(HttpError::serialization_error)?)
            .header("content-type", "application/json");
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            req = req.header(name, value);
        }
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if status.is_success() {
            let _ = body_text;
            let _ = raw_body;
            let _ = headers;
            Ok(())
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "any declared 2xx response",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<serde_json::Value>;
            let parse_error: Option<String>;
            match status_code {
                _ => {
                    match serde_json::from_str::<serde_json::Value>(&body_text) {
                        Ok(v) => {
                            typed = Some(v);
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// `POST /api/v1/admin/users/grant-pro`
    pub async fn post_api_v1_admin_users_grant_pro(
        &self,
        request: PostApiV1AdminUsersGrantProRequest,
    ) -> Result<(), ApiOpError<serde_json::Value>> {
        let request_url = format!(
            "{}{}", self.base_url, "/api/v1/admin/users/grant-pro"
        );
        let mut req = self.http_client.post(request_url);
        req = req
            .body(serde_json::to_vec(&request).map_err(HttpError::serialization_error)?)
            .header("content-type", "application/json");
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            req = req.header(name, value);
        }
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if status.is_success() {
            let _ = body_text;
            let _ = raw_body;
            let _ = headers;
            Ok(())
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "any declared 2xx response",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<serde_json::Value>;
            let parse_error: Option<String>;
            match status_code {
                _ => {
                    match serde_json::from_str::<serde_json::Value>(&body_text) {
                        Ok(v) => {
                            typed = Some(v);
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// `POST /api/v1/admin/users/toggle-status`
    pub async fn post_api_v1_admin_users_toggle_status(
        &self,
        request: PostApiV1AdminUsersToggleStatusRequest,
    ) -> Result<(), ApiOpError<serde_json::Value>> {
        let request_url = format!(
            "{}{}", self.base_url, "/api/v1/admin/users/toggle-status"
        );
        let mut req = self.http_client.post(request_url);
        req = req
            .body(serde_json::to_vec(&request).map_err(HttpError::serialization_error)?)
            .header("content-type", "application/json");
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            req = req.header(name, value);
        }
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if status.is_success() {
            let _ = body_text;
            let _ = raw_body;
            let _ = headers;
            Ok(())
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "any declared 2xx response",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<serde_json::Value>;
            let parse_error: Option<String>;
            match status_code {
                _ => {
                    match serde_json::from_str::<serde_json::Value>(&body_text) {
                        Ok(v) => {
                            typed = Some(v);
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// Revoke an API key
    ///
    /// `POST /api/v1/account/api-key/revoke`
    pub async fn revoke_api_key(
        &self,
        request: RevokeApiKeyRequest,
    ) -> Result<RevokeApiKeyResponse, ApiOpError<serde_json::Value>> {
        let request_url = format!(
            "{}{}", self.base_url, "/api/v1/account/api-key/revoke"
        );
        let mut req = self.http_client.post(request_url);
        req = req
            .body(serde_json::to_vec(&request).map_err(HttpError::serialization_error)?)
            .header("content-type", "application/json");
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<serde_json::Value>;
            let parse_error: Option<String>;
            match status_code {
                _ => {
                    match serde_json::from_str::<serde_json::Value>(&body_text) {
                        Ok(v) => {
                            typed = Some(v);
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// Revoke one of the current user sessions
    ///
    /// `POST /api/v1/account/revoke-session`
    pub async fn revoke_session(
        &self,
        request: RevokeSessionRequest,
    ) -> Result<RevokeSessionResponse, ApiOpError<serde_json::Value>> {
        let request_url = format!(
            "{}{}", self.base_url, "/api/v1/account/revoke-session"
        );
        let mut req = self.http_client.post(request_url);
        req = req
            .body(serde_json::to_vec(&request).map_err(HttpError::serialization_error)?)
            .header("content-type", "application/json");
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<serde_json::Value>;
            let parse_error: Option<String>;
            match status_code {
                _ => {
                    match serde_json::from_str::<serde_json::Value>(&body_text) {
                        Ok(v) => {
                            typed = Some(v);
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// Withdraw or restore an installer release from the latest selection
    ///
    /// `POST /api/v1/admin/installers/set-downloadable`
    pub async fn set_installer_downloadable(
        &self,
        request: SetInstallerDownloadableRequest,
    ) -> Result<
        SetInstallerDownloadableResponse,
        ApiOpError<SetInstallerDownloadableApiError>,
    > {
        let request_url = format!(
            "{}{}", self.base_url, "/api/v1/admin/installers/set-downloadable"
        );
        let mut req = self.http_client.post(request_url);
        req = req
            .body(serde_json::to_vec(&request).map_err(HttpError::serialization_error)?)
            .header("content-type", "application/json");
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<SetInstallerDownloadableApiError>;
            let parse_error: Option<String>;
            match status_code {
                400u16 => {
                    match serde_json::from_str::<
                        SetInstallerDownloadableResponse400,
                    >(&body_text) {
                        Ok(v) => {
                            typed = Some(SetInstallerDownloadableApiError::Status400(v));
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
                404u16 => {
                    match serde_json::from_str::<
                        SetInstallerDownloadableResponse404,
                    >(&body_text) {
                        Ok(v) => {
                            typed = Some(SetInstallerDownloadableApiError::Status404(v));
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
                500u16 => {
                    match serde_json::from_str::<
                        SetInstallerDownloadableResponse500,
                    >(&body_text) {
                        Ok(v) => {
                            typed = Some(SetInstallerDownloadableApiError::Status500(v));
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
                _ => {
                    typed = None;
                    parse_error = None;
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// Withdraw or restore a resistance release from the latest selection
    ///
    /// `POST /api/v1/admin/resistance/set-downloadable`
    pub async fn set_resistance_script_downloadable(
        &self,
        request: SetResistanceScriptDownloadableRequest,
    ) -> Result<
        SetResistanceScriptDownloadableResponse,
        ApiOpError<SetResistanceScriptDownloadableApiError>,
    > {
        let request_url = format!(
            "{}{}", self.base_url, "/api/v1/admin/resistance/set-downloadable"
        );
        let mut req = self.http_client.post(request_url);
        req = req
            .body(serde_json::to_vec(&request).map_err(HttpError::serialization_error)?)
            .header("content-type", "application/json");
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<SetResistanceScriptDownloadableApiError>;
            let parse_error: Option<String>;
            match status_code {
                400u16 => {
                    match serde_json::from_str::<
                        SetResistanceScriptDownloadableResponse400,
                    >(&body_text) {
                        Ok(v) => {
                            typed = Some(
                                SetResistanceScriptDownloadableApiError::Status400(v),
                            );
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
                404u16 => {
                    match serde_json::from_str::<
                        SetResistanceScriptDownloadableResponse404,
                    >(&body_text) {
                        Ok(v) => {
                            typed = Some(
                                SetResistanceScriptDownloadableApiError::Status404(v),
                            );
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
                500u16 => {
                    match serde_json::from_str::<
                        SetResistanceScriptDownloadableResponse500,
                    >(&body_text) {
                        Ok(v) => {
                            typed = Some(
                                SetResistanceScriptDownloadableApiError::Status500(v),
                            );
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
                _ => {
                    typed = None;
                    parse_error = None;
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// Sync subscription status from Dodo Payments
    ///
    /// `POST /api/v1/account/sync-subscription`
    pub async fn sync_subscription(
        &self,
    ) -> Result<SyncSubscriptionResponse, ApiOpError<serde_json::Value>> {
        let request_url = format!(
            "{}{}", self.base_url, "/api/v1/account/sync-subscription"
        );
        let mut req = self.http_client.post(request_url);
        req = req.header(reqwest::header::CONTENT_LENGTH, "0");
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<serde_json::Value>;
            let parse_error: Option<String>;
            match status_code {
                _ => {
                    match serde_json::from_str::<serde_json::Value>(&body_text) {
                        Ok(v) => {
                            typed = Some(v);
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// Update an existing site pack
    ///
    /// `POST /api/v1/admin/site-packs/update`
    pub async fn update_site_pack(
        &self,
        request: UpdateSitePackRequest,
    ) -> Result<UpdateSitePackResponse, ApiOpError<UpdateSitePackApiError>> {
        let request_url = format!(
            "{}{}", self.base_url, "/api/v1/admin/site-packs/update"
        );
        let mut req = self.http_client.post(request_url);
        req = req
            .body(serde_json::to_vec(&request).map_err(HttpError::serialization_error)?)
            .header("content-type", "application/json");
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<UpdateSitePackApiError>;
            let parse_error: Option<String>;
            match status_code {
                400u16 => {
                    match serde_json::from_str::<UpdateSitePackResponse400>(&body_text) {
                        Ok(v) => {
                            typed = Some(UpdateSitePackApiError::Status400(v));
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
                500u16 => {
                    match serde_json::from_str::<UpdateSitePackResponse500>(&body_text) {
                        Ok(v) => {
                            typed = Some(UpdateSitePackApiError::Status500(v));
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
                _ => {
                    typed = None;
                    parse_error = None;
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// Upload a new installer release
    ///
    /// `POST /api/v1/admin/installers/upload`
    pub async fn upload_installer(
        &self,
        request: UploadInstallerRequest,
    ) -> Result<UploadInstallerResponse, ApiOpError<UploadInstallerApiError>> {
        let request_url = format!(
            "{}{}", self.base_url, "/api/v1/admin/installers/upload"
        );
        let mut req = self.http_client.post(request_url);
        req = req
            .body(serde_json::to_vec(&request).map_err(HttpError::serialization_error)?)
            .header("content-type", "application/json");
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<UploadInstallerApiError>;
            let parse_error: Option<String>;
            match status_code {
                400u16 => {
                    match serde_json::from_str::<
                        UploadInstallerResponse400,
                    >(&body_text) {
                        Ok(v) => {
                            typed = Some(UploadInstallerApiError::Status400(v));
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
                500u16 => {
                    match serde_json::from_str::<
                        UploadInstallerResponse500,
                    >(&body_text) {
                        Ok(v) => {
                            typed = Some(UploadInstallerApiError::Status500(v));
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
                _ => {
                    typed = None;
                    parse_error = None;
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// Upload a new resistance mode script
    ///
    /// `POST /api/v1/admin/resistance/upload`
    pub async fn upload_resistance_script(
        &self,
        request: UploadResistanceScriptRequest,
    ) -> Result<
        UploadResistanceScriptResponse,
        ApiOpError<UploadResistanceScriptApiError>,
    > {
        let request_url = format!(
            "{}{}", self.base_url, "/api/v1/admin/resistance/upload"
        );
        let mut req = self.http_client.post(request_url);
        req = req
            .body(serde_json::to_vec(&request).map_err(HttpError::serialization_error)?)
            .header("content-type", "application/json");
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<UploadResistanceScriptApiError>;
            let parse_error: Option<String>;
            match status_code {
                400u16 => {
                    match serde_json::from_str::<
                        UploadResistanceScriptResponse400,
                    >(&body_text) {
                        Ok(v) => {
                            typed = Some(UploadResistanceScriptApiError::Status400(v));
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
                500u16 => {
                    match serde_json::from_str::<
                        UploadResistanceScriptResponse500,
                    >(&body_text) {
                        Ok(v) => {
                            typed = Some(UploadResistanceScriptApiError::Status500(v));
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
                _ => {
                    typed = None;
                    parse_error = None;
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// Upload a new site pack release
    ///
    /// `POST /api/v1/admin/site-packs/upload`
    pub async fn upload_site_pack(
        &self,
        request: UploadSitePackRequest,
    ) -> Result<UploadSitePackResponse, ApiOpError<UploadSitePackApiError>> {
        let request_url = format!(
            "{}{}", self.base_url, "/api/v1/admin/site-packs/upload"
        );
        let mut req = self.http_client.post(request_url);
        req = req
            .body(serde_json::to_vec(&request).map_err(HttpError::serialization_error)?)
            .header("content-type", "application/json");
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<UploadSitePackApiError>;
            let parse_error: Option<String>;
            match status_code {
                400u16 => {
                    match serde_json::from_str::<UploadSitePackResponse400>(&body_text) {
                        Ok(v) => {
                            typed = Some(UploadSitePackApiError::Status400(v));
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
                500u16 => {
                    match serde_json::from_str::<UploadSitePackResponse500>(&body_text) {
                        Ok(v) => {
                            typed = Some(UploadSitePackApiError::Status500(v));
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
                _ => {
                    typed = None;
                    parse_error = None;
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
    /// Force a live DodoPayments check of the current user pro status
    ///
    /// `POST /api/v1/account/verify-pro`
    pub async fn verify_pro_status(
        &self,
    ) -> Result<VerifyProStatusResponse, ApiOpError<serde_json::Value>> {
        let request_url = format!("{}{}", self.base_url, "/api/v1/account/verify-pro");
        let mut req = self.http_client.post(request_url);
        req = req.header(reqwest::header::CONTENT_LENGTH, "0");
        if let Some(api_key) = &self.api_key {
            req = req.bearer_auth(api_key);
        }
        for (name, value) in &self.custom_headers {
            if !name.eq_ignore_ascii_case("accept") {
                req = req.header(name, value);
            }
        }
        req = req.header(reqwest::header::ACCEPT, "application/json");
        let response = req.send().await?;
        let status = response.status();
        let status_code = status.as_u16();
        let headers = response.headers().clone();
        let body_bytes = __read_bounded_response_body(
                response,
                self.max_response_body_bytes,
            )
            .await?;
        let raw_body = body_bytes;
        let body_text = String::from_utf8_lossy(&raw_body).into_owned();
        if false || status_code == 200u16 {
            match serde_json::from_str(&body_text) {
                Ok(body) => Ok(body),
                Err(e) => {
                    Err(
                        ApiOpError::Api(ApiError {
                            status: status_code,
                            headers: headers,
                            body: body_text,
                            raw_body,
                            typed: None,
                            parse_error: Some(
                                format!("failed to deserialize 2xx response body: {}", e),
                            ),
                        }),
                    )
                }
            }
        } else if status.is_success() {
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed: None,
                    parse_error: Some(
                        format!(
                            "unexpected successful status {}; generated return type selects `{}`",
                            status_code, "200",
                        ),
                    ),
                }),
            )
        } else {
            let typed: Option<serde_json::Value>;
            let parse_error: Option<String>;
            match status_code {
                _ => {
                    match serde_json::from_str::<serde_json::Value>(&body_text) {
                        Ok(v) => {
                            typed = Some(v);
                            parse_error = None;
                        }
                        Err(e) => {
                            typed = None;
                            parse_error = Some(e.to_string());
                        }
                    }
                }
            }
            Err(
                ApiOpError::Api(ApiError {
                    status: status_code,
                    headers,
                    body: body_text,
                    raw_body,
                    typed,
                    parse_error,
                }),
            )
        }
    }
}
