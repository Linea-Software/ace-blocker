//! mitm-proxy based proxy — plugs ace-engine's blocking, safe-search, and
//! resistance-script injection into mitm-proxy's inspect points.
//!
//! Traffic redirection is handled by the `ace-tun` crate: a WinTun virtual
//! adapter feeds a userland TCP/IP stack, which terminates each flow and hands
//! it to this proxy. It replaces an earlier WinDivert packet-interception layer
//! whose per-packet process attribution leaked HTTP/3 traffic and ignored IPv6.

mod constants;
mod csp;
mod injection;

use constants::*;

use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, RwLock as StdRwLock};

use async_trait::async_trait;
use bytes::Bytes;
use eyre::{Context, Result};
use http::StatusCode;
use mitm_proxy::{
    BufferedRequest, BufferedResponse, ConnMeta, InterceptDecider, MitmProxy, ProxyConfig,
    RequestAction, RequestBodyMode, RequestInspector, ResponseAction, ResponseBodyMode,
    ResponseInspector,
};
use tokio::io::AsyncReadExt;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::manager::{AccountManager, NetworkManager};
use crate::service::proxy::csp::{
    csp_headers_block_script_injection, generate_nonce, modify_csp_headers_with_nonce,
};
use crate::service::proxy::injection::inject_scripts_with_nonce;
use crate::settings::Settings;

use async_compression::tokio::bufread::{BrotliDecoder, DeflateDecoder, GzipDecoder, ZstdDecoder};

use ace_tun::{ConnectionInfo, Rule, RuleAction, RuleProtocol, TunRedirect};

// ---------------------------------------------------------------------------
// ProxyAdapter — the inspect-point glue
// ---------------------------------------------------------------------------

/// Holds all state the inspectors need. Shared across all connections.
pub struct ProxyAdapter {
    pub network_manager: Arc<RwLock<NetworkManager>>,
    pub account_manager: AccountManager,
    /// Hosts whose current Ace policy genuinely requires decrypted HTTP.
    /// CONNECT inspection refreshes this before the interception decider runs.
    pub interception_hosts: Arc<StdRwLock<HashSet<String>>>,
}

impl ProxyAdapter {
    /// Keep loopback traffic outside the interception and blocking path.
    fn is_local_host(host: &str) -> bool {
        matches!(
            normalize_proxy_host(host).as_str(),
            "localhost" | "127.0.0.1" | "::1"
        )
    }

    async fn record_blocked_attempt(&self) {
        // Diesel is synchronous. Do not hold the global policy write lock (or
        // block a Tokio worker) while persisting a traffic counter.
        let db = self.network_manager.read().await.get_db();
        let _ = tokio::task::spawn_blocking(move || {
            db.increment_blocked_attempts(chrono::Utc::now().date_naive())
        })
        .await;
    }

    fn set_interception_required(&self, host: &str, required: bool) {
        let host = normalize_proxy_host(host);
        let mut interception_hosts = self
            .interception_hosts
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if required {
            interception_hosts.insert(host);
        } else {
            interception_hosts.remove(&host);
        }
    }

    /// Build a 404 + Clear-Site-Data response matching the mitm-proxy handler.
    fn blocked_response() -> http::Response<Bytes> {
        http::Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("Clear-Site-Data", "\"cache\", \"storage\"")
            .body(Bytes::new())
            .expect("Failed to build block response")
    }
}

fn normalize_proxy_host(host: &str) -> String {
    host.trim_matches(['[', ']'])
        .trim_end_matches('.')
        .to_ascii_lowercase()
}

/// Preserve unrelated query bytes while replacing every decoded occurrence
/// of the enforced key. Substrings and conflicting duplicates are not proof
/// that Safe Search is enabled.
fn enforced_query(query: &str, key: &str, value: &str) -> Option<String> {
    let parameters: Vec<_> = url::form_urlencoded::parse(query.as_bytes()).collect();
    let matching: Vec<_> = parameters.iter().filter(|(name, _)| name == key).collect();
    if matching.len() == 1 && matching[0].1 == value {
        return None;
    }

    let mut retained: Vec<&str> = query
        .split('&')
        .filter(|part| {
            !part.is_empty()
                && !url::form_urlencoded::parse(part.as_bytes()).any(|(name, _)| name == key)
        })
        .collect();
    let enforced = url::form_urlencoded::Serializer::new(String::new())
        .append_pair(key, value)
        .finish();
    retained.push(&enforced);
    Some(retained.join("&"))
}

fn safe_search_redirect(location: &http::Uri) -> BufferedResponse {
    http::Response::builder()
        .status(StatusCode::TEMPORARY_REDIRECT)
        .header(http::header::LOCATION, location.to_string())
        .body(Bytes::new())
        .expect("safe-search redirect is valid")
}

fn is_top_level_document_request(req_head: &http::request::Parts) -> bool {
    req_head.method == http::Method::GET
        && req_head
            .headers
            .get("sec-fetch-dest")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|destination| destination.eq_ignore_ascii_case("document"))
}

#[async_trait]
impl RequestInspector for ProxyAdapter {
    /// Enforce hostname-only rules before TLS decryption and retain the
    /// interception decision for features needing the full HTTP request.
    async fn inspect_connect(
        &self,
        _meta: &ConnMeta,
        authority: &http::uri::Authority,
    ) -> RequestAction {
        let host = authority.host();

        if Self::is_local_host(host) {
            self.set_interception_required(host, false);
            return RequestAction::Continue;
        }

        let (blocking_result, requires_interception) =
            self.network_manager.read().await.connect_host_policy(host);
        self.set_interception_required(host, requires_interception);

        if !matches!(blocking_result, crate::core::BlockingResult::NotBlocked) {
            self.record_blocked_attempt().await;
            return RequestAction::Respond(Self::blocked_response());
        }

        RequestAction::Continue
    }

    /// Recheck established tunnels against the current policy. A newly blocked
    /// host or changed interception requirement invalidates the original flow.
    async fn should_terminate_tunnel(
        &self,
        authority: &http::uri::Authority,
        is_intercepted: bool,
    ) -> bool {
        let host = authority.host();
        if Self::is_local_host(host) {
            return false;
        }

        let (blocking_result, requires_interception) =
            self.network_manager.read().await.connect_host_policy(host);
        blocking_result.is_blocked() || requires_interception != is_intercepted
    }

    async fn should_terminate_upgraded(&self, meta: &ConnMeta, uri: &http::Uri) -> bool {
        if meta.is_tls
            && let Ok(authority) = meta.authority.parse()
            && self.should_terminate_tunnel(&authority, true).await
        {
            return true;
        }
        !matches!(
            self.network_manager.read().await.is_uri_blocked(uri),
            crate::core::BlockingResult::NotBlocked
        )
    }

    async fn inspect_request_head(
        &self,
        meta: &ConnMeta,
        req: &mut BufferedRequest,
    ) -> RequestAction {
        let uri = req.uri().clone();
        let method = req.method().clone();
        let host = uri.host().map(|h| h.to_string());

        debug!(
            proto = %meta.protocol,
            client = %meta.client_addr,
            tls = meta.is_tls,
            authority = %meta.authority,
            method = %method,
            version = ?req.version(),
            path = %uri.path(),
            host = ?host,

            "proxy request"
        );

        // ── Bypass check ───────────────────────────────────────────
        if let Some(ref host) = host
            && Self::is_local_host(host)
        {
            return RequestAction::Continue;
        }

        // ── Safe-search enforcement ─────────────────────────────────
        if let Some(ref _host) = host {
            let (blocking_result, safe_search_enforcement) = {
                let manager = self.network_manager.read().await;
                (
                    manager.is_uri_blocked(&uri),
                    manager.get_safe_search_enforcement(&uri),
                )
            };

            if method != http::Method::CONNECT
                && let Some(enforcement) = safe_search_enforcement
            {
                match enforcement {
                    crate::core::SafeSearchEnforcement::DomainRedirection { target_host } => {
                        if uri.host() != Some(&target_host) {
                            let mut new_uri_parts = uri.clone().into_parts();
                            new_uri_parts.authority =
                                Some(http::uri::Authority::from_str(&target_host).unwrap());
                            let new_uri = http::uri::Uri::from_parts(new_uri_parts).unwrap();

                            info!(
                                "Redirecting safe-search host: {:?} -> {}",
                                host, target_host
                            );

                            return RequestAction::Respond(safe_search_redirect(&new_uri));
                        }
                    }
                    crate::core::SafeSearchEnforcement::QueryParameter { key, value } => {
                        let query = uri.query().unwrap_or("");
                        if let Some(new_query) = enforced_query(query, &key, &value) {
                            let mut new_uri_parts = uri.clone().into_parts();
                            let path = uri.path();
                            new_uri_parts.path_and_query = Some(
                                http::uri::PathAndQuery::from_str(&format!(
                                    "{}?{}",
                                    path, new_query
                                ))
                                .unwrap(),
                            );
                            let new_uri = http::uri::Uri::from_parts(new_uri_parts).unwrap();

                            info!("Applying safe-search query parameters for host={host:?}");

                            return RequestAction::Respond(safe_search_redirect(&new_uri));
                        }
                    }
                }
            }

            // ── Block check ─────────────────────────────────────────
            if !matches!(blocking_result, crate::core::BlockingResult::NotBlocked) {
                self.record_blocked_attempt().await;
                return RequestAction::Respond(Self::blocked_response());
            }
        }

        RequestAction::Continue
    }

    fn request_body_mode(&self, _meta: &ConnMeta, _req: &BufferedRequest) -> RequestBodyMode {
        RequestBodyMode::Stream
    }

    async fn inspect_request(&self, meta: &ConnMeta, req: &mut BufferedRequest) -> RequestAction {
        self.inspect_request_head(meta, req).await
    }
}

/// Set on an upstream response head only when decrypted HTML needs scripts.
#[derive(Clone, Copy)]
struct NeedsScriptInjection;

#[async_trait]
impl ResponseInspector for ProxyAdapter {
    async fn inspect_response_head(
        &self,
        meta: &ConnMeta,
        req_head: &http::request::Parts,
        res: &mut BufferedResponse,
    ) -> ResponseAction {
        debug!(
            proto = %meta.protocol,
            client = %meta.client_addr,
            tls = meta.is_tls,
            authority = %meta.authority,
            method = %req_head.method,
            path = %req_head.uri.path(),
            status = %res.status(),
            content_type = ?res.headers().get(http::header::CONTENT_TYPE),
            "proxy response head"
        );

        // Most HTML documents do not need mutation. Decide *before* reading the
        // body so those responses can stream to the browser immediately.
        if is_top_level_document_request(req_head)
            && res
                .headers()
                .get(http::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| value.contains("text/html"))
            && !csp_headers_block_script_injection(res.headers())
        {
            let host = req_head.uri.host().or_else(|| {
                req_head
                    .headers
                    .get("host")
                    .and_then(|value| value.to_str().ok())
            });
            if let Some(host) = host {
                let eligible = {
                    let manager = self.network_manager.read().await;
                    manager.is_resistance_mode_enabled() && manager.has_scripts_of(host)
                };
                if eligible && self.account_manager.is_pro().await {
                    res.extensions_mut().insert(NeedsScriptInjection);
                }
            }
        }

        ResponseAction::Continue
    }

    /// Buffer only top-level HTML documents eligible for script injection;
    /// other response bodies remain streamed to avoid unnecessary allocations.
    fn response_body_mode(
        &self,
        _meta: &ConnMeta,
        req_head: &http::request::Parts,
        res: &BufferedResponse,
    ) -> ResponseBodyMode {
        let _ = req_head;
        if res.extensions().get::<NeedsScriptInjection>().is_some() {
            ResponseBodyMode::Buffer
        } else {
            ResponseBodyMode::Stream
        }
    }

    async fn inspect_response(
        &self,
        _meta: &ConnMeta,
        req_head: &http::request::Parts,
        res: &mut BufferedResponse,
    ) -> ResponseAction {
        let host = req_head.uri.host().map(|h| h.to_string()).or_else(|| {
            req_head
                .headers
                .get("host")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string())
        });

        // Inject only into confirmed top-level documents. Guessing from Accept
        // can target iframe navigations when Fetch Metadata is absent, including
        // sandboxed frames where script execution is intentionally forbidden.
        if !is_top_level_document_request(req_head) {
            return ResponseAction::Continue;
        }

        // ── Content-Type check ──────────────────────────────────────
        let content_type = res
            .headers()
            .get(http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        let is_pro = self.account_manager.is_pro().await;

        if content_type.contains("text/html")
            && is_pro
            && let Some(ref host) = host
        {
            // ── Resistance master switch ──────────────────────────
            // When resistance mode is disabled, no scripts and no
            // window.__BOREDOM_INTENSITY__ may be injected.
            let resistance_enabled = {
                let manager = self.network_manager.read().await;
                manager.is_resistance_mode_enabled()
            };
            if !resistance_enabled {
                return ResponseAction::Continue;
            }

            let (scripts, intensity) = {
                let manager = self.network_manager.read().await;
                (
                    manager.get_resistance_scripts_of(host),
                    manager.get_effective_intensity(),
                )
            };
            if scripts.is_empty() {
                return ResponseAction::Continue;
            }

            if csp_headers_block_script_injection(res.headers()) {
                debug!(
                    %host,
                    "skipping resistance injection because CSP sandbox forbids scripts"
                );
                return ResponseAction::Continue;
            }

            // Detect compression
            let content_encoding = res
                .headers()
                .get(http::header::CONTENT_ENCODING)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_lowercase();

            let body_bytes = res.body().clone();
            let cursor = std::io::Cursor::new(body_bytes);
            let mut decompressed_reader: Box<dyn tokio::io::AsyncRead + Unpin + Send> =
                match content_encoding.as_str() {
                    "gzip" => Box::new(GzipDecoder::new(cursor)),
                    "deflate" => Box::new(DeflateDecoder::new(cursor)),
                    "br" => Box::new(BrotliDecoder::new(cursor)),
                    "zstd" => Box::new(ZstdDecoder::new(cursor)),
                    _ => Box::new(cursor),
                };

            let mut html_string = String::new();
            if let Err(error) = decompressed_reader.read_to_string(&mut html_string).await {
                error!("Failed to decompress/read body: {}", error);
                return ResponseAction::Continue;
            }

            // Generate one nonce and apply it to every enforcing CSP header.
            // Browsers enforce repeated CSP headers independently, so every
            // policy must authorize the injected scripts.
            let nonce = generate_nonce();
            modify_csp_headers_with_nonce(res.headers_mut(), &nonce);

            // Inject scripts
            let new_html = inject_scripts_with_nonce(
                host,
                html_string,
                is_pro,
                &scripts,
                &nonce,
                &intensity.to_string(),
            );

            // The body is now context-specific (Resistance Mode state + CSP
            // nonce). Never let a browser cache this modified top-level response
            // and later reuse it for an iframe navigation, where a sandbox may
            // forbid the embedded Ace scripts. Origin validators also describe
            // the unmodified upstream body and are no longer valid.
            res.headers_mut().remove(http::header::CONTENT_ENCODING);
            res.headers_mut().remove(http::header::CONTENT_LENGTH);
            res.headers_mut().remove(http::header::ETAG);
            res.headers_mut().remove(http::header::LAST_MODIFIED);
            res.headers_mut().insert(
                http::header::CACHE_CONTROL,
                http::HeaderValue::from_static("no-store"),
            );
            res.headers_mut().append(
                http::header::VARY,
                http::HeaderValue::from_static("Sec-Fetch-Dest"),
            );

            *res.body_mut() = Bytes::from(new_html);
        }

        ResponseAction::Continue
    }
}

// ---------------------------------------------------------------------------
// Interception contract
// ---------------------------------------------------------------------------

/// Keep HTTPS opaque by default. TLS is terminated only when the current Ace
/// policy for that host genuinely requires decrypted HTTP (regex/path rules,
/// URL exceptions, Safe Search rewriting, or Resistance script injection).
///
/// Hostname-only blocking is enforced by `inspect_connect`, before this decider
/// runs, so ordinary browsing retains the browser's native TLS/HTTP fingerprint.
#[derive(Debug)]
pub struct ProxyInterceptDecider {
    interception_hosts: Arc<StdRwLock<HashSet<String>>>,
}

impl ProxyInterceptDecider {
    fn new(interception_hosts: Arc<StdRwLock<HashSet<String>>>) -> Self {
        Self { interception_hosts }
    }
}

impl InterceptDecider for ProxyInterceptDecider {
    fn recover_hostname_for_ip_connect(&self) -> bool {
        true
    }

    fn should_intercept(&self, authority: &http::uri::Authority, sni: Option<&str>) -> bool {
        let host = authority.host();
        let policy_requires_interception = self
            .interception_hosts
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .contains(&normalize_proxy_host(host));
        let should_intercept = policy_requires_interception;
        debug!(
            authority = %authority,
            sni = ?sni,
            policy_requires_interception,
            should_intercept,
            "proxy interception decision"
        );
        should_intercept
    }
}

/// Selective HTTP/3 admission while preserving client-certificate sites.
///
/// Hostname-level blocks can be enforced without an origin request; even a
/// client-certificate site receives a block response before the proxy dials
/// upstream. Request/response inspection for otherwise allowed origins still
/// uses TCP CONNECT, which has an asynchronous client-certificate probe before
/// the browser's TLS session is terminated. A QUIC certificate resolver is
/// synchronous and cannot safely perform that probe during the handshake.
/// Those origins are therefore denied H3 so browsers can retry over TCP.
fn http3_host_policy_admits(
    blocking_result: crate::core::BlockingResult,
    _requires_request_inspection: bool,
) -> bool {
    blocking_result.is_blocked()
}

/// Wire the engine's inspection hooks and interception contract onto a
/// [`MitmProxy`].
fn wire_interception(
    config: ProxyConfig,
    request_inspector: Arc<dyn RequestInspector>,
    response_inspector: Arc<dyn ResponseInspector>,
    interception_hosts: Arc<StdRwLock<HashSet<String>>>,
) -> MitmProxy {
    MitmProxy::new(config)
        .with_request_inspector(request_inspector)
        .with_response_inspector(response_inspector)
        .with_intercept_decider(Arc::new(ProxyInterceptDecider::new(interception_hosts)))
}

// ---------------------------------------------------------------------------
// AceProxy — lifecycle wrapper
// ---------------------------------------------------------------------------

/// Runs the mitm-proxy instance wired to the engine's blocking logic and
/// `ace-tun` (WinTun virtual adapter + userland TCP/IP stack).
pub struct AceProxy {
    adapter: Arc<ProxyAdapter>,
    listen_addr: SocketAddr,
    http3_addr: SocketAddr,
    /// Shared with the background start-retry task (see [`AceProxy::start`]).
    tun: Arc<TunRedirect>,
}

// ── Rule constants (ported from proxybridge's configure_for_ace) ──────────

impl AceProxy {
    /// Create a new proxy service backed by `network_manager`.
    ///
    /// The proxy uses its own CA cert/key at
    /// `{cert_dir}/mitm_ca.pem` / `{cert_dir}/mitm_ca.key`.
    pub async fn new(
        network_manager: Arc<RwLock<NetworkManager>>,
        listen_port: u16,
    ) -> Result<Self> {
        let cert_dir = Settings::get_cert_dir();
        if !cert_dir.exists() {
            std::fs::create_dir_all(&cert_dir)?;
        }

        // Select the actual loopback UDP destination before configuring TUN.
        // A later bind race is fatal to proxy startup, never a direct-UDP fallback.
        let http3_socket = std::net::UdpSocket::bind("127.0.0.1:0")
            .wrap_err("Failed to select the HTTP/3 proxy port")?;
        let http3_addr = http3_socket.local_addr()?;
        drop(http3_socket);

        // ── ace-tun: build and configure, but defer start ──────────────
        // The adapter must not exist before the proxy has bound its port, or
        // the first browser connections arrive with nowhere to go.
        let protected_browsers = browser_processes();
        let tun = TunRedirect::builder(format!("127.0.0.1:{}", listen_port))
            .wrap_err("Failed to create ace-tun builder")?
            .quic_policy(ace_tun::QuicPolicy::Intercept)
            .quic_proxy_addr(http3_addr)
            .protect_processes(&protected_browsers)
            // 1. Exempt ace-engine + ace-app from ALL interception
            .add_rule(Rule::new(EXEMPT_PROCESSES.join(";")).action(RuleAction::Direct))
            // 2. Allow all processes to reach localhost directly
            .add_rule(
                Rule::new("*")
                    .hosts("127.0.0.1;::1")
                    .action(RuleAction::Direct),
            )
            // 3. Keep local anonymity/proxy helpers from turning a browser's
            //    loopback connection into an uninspected external relay. Desktop
            //    VPN clients are intentionally allowed; platform enforcement
            //    prevents them from becoming a route-level bypass.
            .add_rule(Rule::new(LOCAL_PROXY_PROCESSES.join(";")).action(RuleAction::Block))
            // 4. Browser VPN/proxy extensions tunnel inside ordinary HTTPS, so
            //    route enforcement alone cannot distinguish them from a normal
            //    website. Keep the narrowly scoped relay-domain denylist as
            //    application-layer defense in depth.
            .add_rule(
                Rule::new("*")
                    .domains(VPN_EXTENSION_DOMAINS)
                    .action(RuleAction::Block),
            )
            // 5. Browser-only MITM: route browsers TCP 80/443 through proxy.
            .add_rule(
                Rule::new(protected_browsers)
                    .ports("80;443")
                    .protocol(RuleProtocol::Tcp)
                    .action(RuleAction::Proxy),
            )
            // Browser QUIC goes only to the local HTTP/3 listener; unsupported
            // or denied handshakes fall back to inspected TCP, never direct UDP.
            // 6. Catch-all: allow remaining traffic directly. On Linux the
            //    relay is SO_MARKed and returns to the normal RPDB (including a
            //    user VPN); on Windows WFP ensures protected applications cannot
            //    escape ACE by selecting another network interface.
            .add_rule(Rule::new("*").action(RuleAction::Direct))
            .proxy_config(ace_tun::ProxyConfig::http("127.0.0.1", listen_port))
            .on_log(|msg| tracing::info!("[ace-tun] {}", msg))
            .on_connection(|info: &ConnectionInfo| {
                tracing::debug!(
                    process = %info.process_name,
                    pid = info.pid,
                    dest_ip = %info.dest_ip,
                    dest_port = info.dest_port,
                    action = ?info.action,
                    proxy = %info.proxy_info,
                    "[ace-tun conn] flow decision"
                );
            })
            .build()
            .wrap_err("Failed to build ace-tun")?;

        info!(
            "ace-tun configured with 6 rules (platform route guard enabled; protected-browser \
             QUIC selectively blocked through H3; IPv4 + available IPv6), forwarding to 127.0.0.1:{} (deferred start)",
            listen_port
        );

        let interception_hosts = Arc::new(StdRwLock::new(HashSet::new()));

        let account_manager = network_manager.read().await.get_account_manager().clone();

        let adapter = Arc::new(ProxyAdapter {
            network_manager,
            account_manager,
            interception_hosts,
        });

        let listen_addr = SocketAddr::from_str(&format!("127.0.0.1:{}", listen_port))?;

        Ok(Self {
            adapter,
            listen_addr,
            http3_addr,
            tun: Arc::new(tun),
        })
    }

    /// Start the proxy and block until it stops or encounters a fatal error.
    pub async fn start(&self) -> Result<()> {
        let cert_dir = Settings::get_cert_dir();

        #[allow(unused_mut)]
        // `run()` provisions both Ace certificates, including Firefox-specific
        // Linux trust policy, before starting either service. Avoid repeating
        // trust-store mutations here after the listener task has been spawned.
        let mut config = ProxyConfig::new(self.listen_addr, cert_dir)
            .install_ca(false)
            .with_http3(self.http3_addr)
            .preserve_client_cert_auth(true);

        // Disable upstream verification in dev (managed certificates may be
        // self-signed during development).
        #[cfg(debug_assertions)]
        {
            config = config.verify_upstream(false);
        }

        let adapter = self.adapter.clone();
        let proxy = wire_interception(
            config,
            adapter.clone() as Arc<dyn RequestInspector>,
            adapter.clone() as Arc<dyn ResponseInspector>,
            adapter.interception_hosts.clone(),
        )
        .with_http3_sni_policy(Arc::new(move |host| {
            if ProxyAdapter::is_local_host(host) {
                return false;
            }
            // TLS certificate resolution is synchronous. Never block a runtime
            // worker or reuse a stale CONNECT decision for QUIC admission.
            let Ok(manager) = adapter.network_manager.try_read() else {
                return false;
            };
            let (blocking_result, requires_inspection) = manager.connect_host_policy(host);
            http3_host_policy_admits(blocking_result, requires_inspection)
        }));

        // Spawn the proxy first so it binds the port before ace-tun starts
        // diverting traffic. Starting in the other order would send the first
        // browser connections to a port nobody is listening on.
        let mut proxy_handle = tokio::spawn(async move { proxy.run().await });

        // Both listeners must be live before any browser traffic is redirected.
        tokio::select! {
            readiness = async {
                wait_for_proxy_listener(self.listen_addr, std::time::Duration::from_secs(10)).await?;
                wait_for_http3_listener(self.http3_addr, std::time::Duration::from_secs(10)).await
            } => {
                if let Err(error) = readiness {
                    proxy_handle.abort();
                    let _ = proxy_handle.await;
                    return Err(error);
                }
            },
            result = &mut proxy_handle => {
                let error = match result {
                    Ok(Ok(())) => eyre::eyre!("mitm-proxy stopped before becoming ready"),
                    Ok(Err(error)) => error.wrap_err("mitm-proxy failed before becoming ready"),
                    Err(error) => eyre::eyre!("mitm-proxy task failed before becoming ready: {error}"),
                };
                return Err(error);
            }
        }

        info!("🚀 Proxy listening on {}", self.listen_addr);

        // NOW start ace-tun — the proxy is already listening.
        //
        // A failure here is deliberately non-fatal. Without administrator
        // rights we cannot create the adapter, and the right response is to
        // keep running unfiltered rather than to take the machine's networking
        // with us. Blocking silently stops working, which is bad; killing the
        // engine would be worse.
        let mut retry_task: Option<tokio::task::JoinHandle<()>> = None;
        let intercepting = match self.tun.start().await {
            Ok(()) => {
                info!(
                    "ace-tun started — intercepting browser traffic \
                     (IPv4 + available IPv6, QUIC selectively admitted)"
                );
                true
            }
            Err(ace_tun::Error::NotElevated) => {
                #[cfg(target_os = "windows")]
                let privilege_hint = "administrator privileges";
                #[cfg(target_os = "linux")]
                let privilege_hint = "root privileges or CAP_NET_ADMIN";
                #[cfg(target_os = "macos")]
                let privilege_hint = "root privileges";
                warn!(
                    "ace-tun needs {privilege_hint}; running WITHOUT traffic interception — sites \
                     will not be blocked. Restart the engine elevated to enable blocking."
                );
                false
            }
            Err(error) => {
                warn!(
                    "ace-tun failed to start ({error}); retrying in the background — sites will not be \
                     blocked until the tunnel comes up."
                );
                // Transient failures (freshly installed wintun.dll while the
                // Windows Installer is still finalizing, AV first-touch scans,
                // slow disks) clear within seconds to minutes. Keep trying in
                // the background so a later service restart is not required;
                // the retry is aborted at shutdown, which drops the adapter
                // and restores the routing table.
                #[cfg(target_os = "linux")]
                warn_if_running_kernel_modules_are_missing();
                retry_task = Some(spawn_tun_retry(self.tun.clone()));
                false
            }
        };

        let proxy_exit_error = tokio::select! {
            result = &mut proxy_handle => {
                Some(match result {
                    Ok(Ok(())) => eyre::eyre!("mitm-proxy stopped unexpectedly"),
                    Ok(Err(error)) => error.wrap_err("mitm-proxy runtime failure"),
                    Err(error) => eyre::eyre!("mitm-proxy task failed: {error}"),
                })
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Received Ctrl+C, shutting down proxy...");
                None
            }
        };

        // Stop the background tunnel retry (if any) so a late success is torn
        // down with the proxy.
        if let Some(task) = retry_task {
            task.abort();
            let _ = task.await;
        }

        info!("\n🛑 Proxy shutting down.");

        // Tear the tunnel down before returning so the routing table is
        // restored even if the process lingers.
        // A delayed retry can start TUN even if the initial attempt failed.
        // Inspect live state after the retry task is joined before teardown.
        if intercepting || self.tun.is_running() {
            let stats = self.tun.stats();
            info!(
                "ace-tun stats: {} tcp / {} udp flows, {} blocked, {} quic dropped, \
                 {} multicast dropped, {} udp send errors, {} proxy connection failures",
                stats.tcp_flows,
                stats.udp_flows,
                stats.blocked,
                stats.quic_dropped,
                stats.group_dropped,
                stats.udp_send_errors,
                stats.proxy_fallbacks
            );
            if let Err(error) = self.tun.stop().await {
                error!("Failed to stop ace-tun: {:?}", error);
            }
        }

        if let Some(error) = proxy_exit_error {
            return Err(error);
        }

        Ok(())
    }
}

/// Verify the UDP endpoint responds as QUIC without admitting an SNI or
/// minting a certificate. An unsupported version elicits Version Negotiation.
async fn wait_for_http3_listener(addr: SocketAddr, timeout: std::time::Duration) -> Result<()> {
    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await?;
    socket.connect(addr).await?;
    let connection_ids = uuid::Uuid::new_v4();
    let connection_ids = connection_ids.as_bytes();
    let mut probe = vec![0_u8; 1200];
    probe[..6].copy_from_slice(&[0xc0, 0xfa, 0xce, 0xb0, 0x0c, 8]);
    probe[6..14].copy_from_slice(&connection_ids[..8]);
    probe[14] = 8;
    probe[15..23].copy_from_slice(&connection_ids[8..]);
    let deadline = tokio::time::Instant::now() + timeout;
    let mut response = [0_u8; 1500];
    loop {
        socket.send(&probe).await?;
        if let Ok(Ok(length)) = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            socket.recv(&mut response),
        )
        .await
        {
            if length >= 27
                && response[0] & 0x80 != 0
                && response[1..5] == [0, 0, 0, 0]
                && response[5] == 8
                && response[6..14] == connection_ids[8..]
                && response[14] == 8
                && response[15..23] == connection_ids[..8]
            {
                return Ok(());
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(eyre::eyre!(
                "HTTP/3 listener {addr} did not become ready within {timeout:?}"
            ));
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}

async fn wait_for_proxy_listener(addr: SocketAddr, timeout: std::time::Duration) -> Result<()> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        match tokio::net::TcpStream::connect(addr).await {
            Ok(stream) => {
                drop(stream);
                return Ok(());
            }
            Err(error) if tokio::time::Instant::now() >= deadline => {
                return Err(eyre::eyre!(
                    "proxy listener {addr} did not become ready within {timeout:?}: {error}"
                ));
            }
            Err(_) => {}
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}

#[cfg(test)]
mod http3_readiness_tests {
    use super::*;

    // H3 cannot carry the browser's client certificate to a mutually
    // authenticated upstream origin. A hostname-level block does not connect
    // upstream at all and may be handled natively over H3; request-only rules
    // must retain the established TCP/CONNECT + client-cert probing path.
    #[test]
    fn h3_admission_restricts_unprobed_origins_to_host_level_blocks() {
        assert!(http3_host_policy_admits(
            crate::core::BlockingResult::Blocked,
            false,
        ));
        assert!(!http3_host_policy_admits(
            crate::core::BlockingResult::NotBlocked,
            true,
        ));
        assert!(!http3_host_policy_admits(
            crate::core::BlockingResult::NotBlocked,
            false,
        ));
    }

    #[tokio::test]
    async fn actual_quinn_listener_passes_readiness_without_a_tls_certificate() {
        let tcp = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let tcp_addr = tcp.local_addr().unwrap();
        drop(tcp);
        let udp = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let udp_addr = udp.local_addr().unwrap();
        drop(udp);

        let ca_dir = tempfile::tempdir().unwrap();
        // No SNI is admitted. Readiness must not issue a certificate, require
        // a real account policy, or alter any engine-managed configuration.
        let proxy = MitmProxy::new(ProxyConfig::new(tcp_addr, ca_dir.path()).with_http3(udp_addr))
            .with_http3_sni_policy(Arc::new(|_host| false));
        let server = tokio::spawn(async move { proxy.run().await });
        let ready = wait_for_http3_listener(udp_addr, std::time::Duration::from_secs(3)).await;
        server.abort();
        assert!(ready.is_ok(), "QUIC listener never became ready: {ready:?}");
    }

    #[tokio::test]
    async fn arbitrary_udp_responses_cannot_fake_quic_readiness() {
        let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = socket.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut datagram = [0_u8; 1500];
            while let Ok((_, peer)) = socket.recv_from(&mut datagram).await {
                let _ = socket.send_to(b"not QUIC", peer).await;
            }
        });
        let ready = wait_for_http3_listener(addr, std::time::Duration::from_millis(250)).await;
        server.abort();
        assert!(
            ready.is_err(),
            "arbitrary UDP socket looked like a live QUIC listener"
        );
    }
}

/// Initial backoff; keep retrying every minute after these delays.
const TUN_RETRY_DELAYS_SECS: &[u64] = &[5, 15, 30, 60, 60, 60, 60, 60, 60, 60, 60];

fn tun_retry_delay_secs(attempt_index: usize) -> u64 {
    TUN_RETRY_DELAYS_SECS
        .get(attempt_index)
        .copied()
        .unwrap_or(60)
}

/// Kernel upgrades may leave TUN temporarily unavailable. Keep retrying until
/// it starts; the task is cancelled on proxy shutdown.
fn spawn_tun_retry(tun: Arc<TunRedirect>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        for attempt in 1_u64.. {
            let delay_secs =
                tun_retry_delay_secs(usize::try_from(attempt - 1).unwrap_or(usize::MAX));
            tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
            match tun.start().await {
                Ok(()) => {
                    info!("ace-tun started after background retry (attempt {attempt})");
                    return;
                }
                Err(ace_tun::Error::NotElevated) => {
                    error!("ace-tun cannot recover without privileges; restart elevated");
                    return;
                }
                Err(ace_tun::Error::AlreadyRunning) => return,
                Err(error) => {
                    if attempt <= TUN_RETRY_DELAYS_SECS.len() as u64 || attempt.is_multiple_of(10) {
                        warn!(
                            "ace-tun retry {attempt} failed: {error}; enforcement is OFF; retrying after {delay_secs}s"
                        );
                    }
                }
            }
        }
    })
}

#[cfg(target_os = "linux")]
fn warn_if_running_kernel_modules_are_missing() {
    let Ok(release) = std::fs::read_to_string("/proc/sys/kernel/osrelease") else {
        return;
    };
    let release = release.trim();
    let module_dir = std::path::Path::new("/lib/modules").join(release);
    if !module_dir.is_dir() {
        error!(
            running_kernel = release,
            expected_modules = %module_dir.display(),
            "Running kernel has no matching module tree; TUN may fail after a kernel update. Reboot with a matching kernel."
        );
    }
}

#[cfg(test)]
mod tun_retry_tests {
    use super::tun_retry_delay_secs;
    #[test]
    fn retry_backoff_remains_periodic_after_initial_attempts() {
        assert_eq!(tun_retry_delay_secs(0), 5);
        assert_eq!(tun_retry_delay_secs(1), 15);
        assert_eq!(tun_retry_delay_secs(2), 30);
        assert_eq!(tun_retry_delay_secs(9), 60);
        assert_eq!(tun_retry_delay_secs(10), 60);
        assert_eq!(tun_retry_delay_secs(1_000_000), 60);
    }
}

// ---------------------------------------------------------------------------
// Certificate management (used by startup and the install-cert binary)
// ---------------------------------------------------------------------------

/// Ensure both Ace TLS certificates exist and are trusted before networking starts.
pub fn prepare_certificates() -> Result<()> {
    install_certificate().wrap_err("preparing MITM root certificate")?;
    Ok(())
}

/// Generate a root CA and install it into the system trust store.
///
/// Uses mitm-proxy's `CertAuthority` to generate and manage the root CA.
pub fn install_certificate() -> Result<()> {
    let cert_dir = Settings::get_cert_dir();
    if !cert_dir.exists() {
        std::fs::create_dir_all(&cert_dir)?;
    }

    let cert_path = cert_dir.join("mitm_ca.pem");
    let key_path = cert_dir.join("mitm_ca.key");

    info!("Ensuring MITM Root CA certificate is generated and trusted...");

    // mitm-proxy's CertAuthority::load_or_generate handles creation + persistence.
    mitm_proxy::CertAuthority::load_or_generate(&cert_path, &key_path)?;
    #[cfg(target_os = "linux")]
    mitm_proxy::CertAuthority::install_to_chromium_trust_store(&cert_path);
    install_cert_to_system_trust_store(&cert_path);

    info!("MITM Root CA certificate is ready.");

    Ok(())
}

#[cfg(target_os = "linux")]
const FIREFOX_POLICY_PATH: &str = "/etc/firefox/policies/policies.json";

#[cfg(target_os = "linux")]
fn ensure_firefox_certificate_policy(policy_path: &Path, cert_path: &Path) -> Result<bool> {
    if !cert_path.is_absolute() {
        return Err(eyre::eyre!(
            "Firefox certificate policy requires an absolute path: {}",
            cert_path.display()
        ));
    }

    let mut document = match std::fs::read_to_string(policy_path) {
        Ok(contents) => {
            serde_json::from_str::<serde_json::Value>(&contents).wrap_err_with(|| {
                format!("parsing existing Firefox policy {}", policy_path.display())
            })?
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            serde_json::json!({ "policies": {} })
        }
        Err(error) => {
            return Err(error)
                .wrap_err_with(|| format!("reading Firefox policy {}", policy_path.display()));
        }
    };

    let root = document
        .as_object_mut()
        .ok_or_else(|| eyre::eyre!("Firefox policy root must be a JSON object"))?;
    let policies = root
        .entry("policies")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or_else(|| eyre::eyre!("Firefox policy 'policies' value must be a JSON object"))?;
    let certificates = policies
        .entry("Certificates")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or_else(|| eyre::eyre!("Firefox 'Certificates' policy must be a JSON object"))?;
    let install = certificates
        .entry("Install")
        .or_insert_with(|| serde_json::json!([]))
        .as_array_mut()
        .ok_or_else(|| eyre::eyre!("Firefox 'Certificates.Install' policy must be an array"))?;

    let cert_path = cert_path
        .to_str()
        .ok_or_else(|| eyre::eyre!("certificate path is not valid UTF-8"))?;
    if install
        .iter()
        .any(|entry| entry.as_str() == Some(cert_path))
    {
        return Ok(false);
    }
    install.push(serde_json::Value::String(cert_path.to_owned()));

    let parent = policy_path
        .parent()
        .ok_or_else(|| eyre::eyre!("Firefox policy path has no parent directory"))?;
    std::fs::create_dir_all(parent)
        .wrap_err_with(|| format!("creating Firefox policy directory {}", parent.display()))?;

    let file_name = policy_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("policies.json");
    let temporary_path = policy_path.with_file_name(format!(".{file_name}.ace.tmp"));
    let encoded = format!("{}\n", serde_json::to_string_pretty(&document)?);
    std::fs::write(&temporary_path, encoded).wrap_err_with(|| {
        format!(
            "writing Firefox policy temporary file {}",
            temporary_path.display()
        )
    })?;

    if let Ok(metadata) = std::fs::metadata(policy_path) {
        std::fs::set_permissions(&temporary_path, metadata.permissions()).wrap_err_with(|| {
            format!(
                "preserving Firefox policy permissions on {}",
                temporary_path.display()
            )
        })?;
    }

    if let Err(error) = std::fs::rename(&temporary_path, policy_path) {
        let _ = std::fs::remove_file(&temporary_path);
        return Err(error)
            .wrap_err_with(|| format!("replacing Firefox policy {}", policy_path.display()));
    }

    Ok(true)
}

#[cfg(target_os = "linux")]
fn install_cert_to_firefox_trust_store(cert_path: &Path) {
    match ensure_firefox_certificate_policy(Path::new(FIREFOX_POLICY_PATH), cert_path) {
        Ok(true) => info!(
            "Firefox certificate policy updated for {}; restart Firefox to apply it",
            cert_path.display()
        ),
        Ok(false) => debug!(
            "Firefox certificate policy already contains {}",
            cert_path.display()
        ),
        Err(error) => warn!(
            "failed to install {} into Firefox enterprise certificate policy: {error:#}",
            cert_path.display()
        ),
    }
}

#[cfg(target_os = "linux")]
fn refresh_linux_trust_bundles() {
    match std::process::Command::new("update-ca-trust")
        .arg("extract")
        .output()
    {
        Ok(output) if output.status.success() => return,
        Ok(output) => warn!(
            "update-ca-trust extract failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => warn!("failed to run update-ca-trust: {error}"),
    }

    match std::process::Command::new("trust")
        .arg("extract-compat")
        .output()
    {
        Ok(output) if output.status.success() => {}
        Ok(output) => warn!(
            "p11-kit trust extract-compat failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        Err(error) => debug!("p11-kit trust extract-compat unavailable: {error}"),
    }
}

#[cfg(target_os = "linux")]
fn install_linux_system_trust(cert_path: &Path) {
    match std::process::Command::new("trust")
        .args(["anchor", "--store"])
        .arg(cert_path)
        .output()
    {
        Ok(output) if output.status.success() => {
            refresh_linux_trust_bundles();
            info!("certificate installed via p11-kit trust");
            return;
        }
        Ok(output) => warn!(
            "p11-kit trust could not install the certificate: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => warn!("failed to run p11-kit trust: {error}"),
    }

    // Debian/Ubuntu (and friends) consume local anchors from this directory.
    const CA_DIR: &str = "/usr/local/share/ca-certificates";
    let stem = cert_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("ace-engine-ca");
    let target = Path::new(CA_DIR).join(format!("{stem}.crt"));

    if let Err(error) = std::fs::create_dir_all(CA_DIR) {
        warn!("failed to create {CA_DIR}: {error}");
        return;
    }
    if let Err(error) = std::fs::copy(cert_path, &target) {
        warn!("failed to copy certificate into {CA_DIR}: {error} (run elevated?)");
        return;
    }

    match std::process::Command::new("update-ca-certificates").output() {
        Ok(output) if output.status.success() => {
            info!("certificate installed via update-ca-certificates");
        }
        Ok(output) => warn!(
            "update-ca-certificates failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        Err(error) => warn!(
            "automatic Linux CA installation failed; neither p11-kit trust nor \
             update-ca-certificates succeeded: {error}"
        ),
    }
}

/// Install a PEM certificate into the OS trust store.
///
/// Best-effort on every platform: a failure is logged, never fatal — a
/// certificate the machine merely does not trust is preferable to the engine
/// refusing to start.
///
/// * **Windows** — `certutil -addstore -f Root` (unchanged).
/// * **Linux** — prefer p11-kit's system trust store (Arch/Fedora family),
///   falling back to `update-ca-certificates` (Debian family), and also register
///   the certificate through Firefox's enterprise `Certificates.Install` policy.
/// * **macOS** — `security add-trusted-cert` into the System keychain.
fn install_cert_to_system_trust_store(cert_path: &Path) {
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("certutil")
            .args([
                "-addstore",
                "-f",
                "Root",
                cert_path.to_str().unwrap_or_default(),
            ])
            .output();
        match output {
            Ok(out) if out.status.success() => {
                info!("certificate installed to the machine Root store");
            }
            Ok(out) => {
                warn!(
                    "certutil failed to install the certificate: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            Err(error) => warn!("failed to run certutil: {error}"),
        }
    }

    #[cfg(target_os = "linux")]
    {
        install_linux_system_trust(cert_path);
        install_cert_to_firefox_trust_store(cert_path);
    }

    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("security")
            .args([
                "add-trusted-cert",
                "-d",
                "-r",
                "trustRoot",
                "-k",
                "/Library/Keychains/System.keychain",
                cert_path.to_str().unwrap_or_default(),
            ])
            .output();
        match output {
            Ok(out) if out.status.success() => {
                info!("certificate installed to the System keychain");
            }
            Ok(out) => {
                warn!(
                    "security add-trusted-cert failed: {} (run elevated?)",
                    String::from_utf8_lossy(&out.stderr).trim()
                );
            }
            Err(error) => warn!("failed to run security: {error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    #[test]
    fn firefox_certificate_policy_preserves_existing_settings() {
        let directory = tempfile::tempdir().unwrap();
        let policy_path = directory.path().join("policies.json");
        std::fs::write(
            &policy_path,
            r#"{"policies":{"DisableTelemetry":true,"Certificates":{"Install":["/existing.pem"]}}}"#,
        )
        .unwrap();
        let cert_path = Path::new("/usr/local/share/AceBlocker/certs/mitm_ca.pem");

        assert!(ensure_firefox_certificate_policy(&policy_path, cert_path).unwrap());

        let policy: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&policy_path).unwrap()).unwrap();
        assert_eq!(policy["policies"]["DisableTelemetry"], true);
        let installs = policy["policies"]["Certificates"]["Install"]
            .as_array()
            .unwrap();
        assert!(installs.iter().any(|entry| entry == "/existing.pem"));
        assert!(
            installs
                .iter()
                .any(|entry| entry == cert_path.to_str().unwrap())
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn firefox_certificate_policy_is_idempotent() {
        let directory = tempfile::tempdir().unwrap();
        let policy_path = directory.path().join("policies.json");
        let cert_path = Path::new("/tmp/ace-test-ca.pem");

        assert!(ensure_firefox_certificate_policy(&policy_path, cert_path).unwrap());
        assert!(!ensure_firefox_certificate_policy(&policy_path, cert_path).unwrap());

        let policy: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&policy_path).unwrap()).unwrap();
        let installs = policy["policies"]["Certificates"]["Install"]
            .as_array()
            .unwrap();
        assert_eq!(
            installs
                .iter()
                .filter(|entry| entry.as_str() == cert_path.to_str())
                .count(),
            1
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn firefox_certificate_policy_does_not_overwrite_incompatible_policy() {
        let directory = tempfile::tempdir().unwrap();
        let policy_path = directory.path().join("policies.json");
        let original = r#"{"policies":{"Certificates":{"Install":"managed elsewhere"}}}"#;
        std::fs::write(&policy_path, original).unwrap();

        assert!(
            ensure_firefox_certificate_policy(&policy_path, Path::new("/tmp/ace-test-ca.pem"))
                .is_err()
        );
        assert_eq!(std::fs::read_to_string(policy_path).unwrap(), original);
    }

    #[test]
    fn safe_search_replaces_conflicting_and_disguised_parameters() {
        for query in [
            "safe=off&not_safe=active",
            "safe=off&safe=active",
            "sa%66e=off",
        ] {
            let rewritten = enforced_query(query, "safe", "active").unwrap();
            let values: Vec<_> = url::form_urlencoded::parse(rewritten.as_bytes())
                .filter(|(key, _)| key == "safe")
                .map(|(_, value)| value.into_owned())
                .collect();
            assert_eq!(values, ["active"]);
            assert_eq!(enforced_query(&rewritten, "safe", "active"), None);
        }
    }

    #[test]
    fn safe_search_preserves_unrelated_query_encoding() {
        assert_eq!(
            enforced_query("q=some%20text&token=a%2Bb&safe=off", "safe", "active"),
            Some("q=some%20text&token=a%2Bb&safe=active".into())
        );
        assert_eq!(enforced_query("safe=active&q=test", "safe", "active"), None);
    }

    #[test]
    fn safe_search_redirect_preserves_non_get_methods() {
        let location: http::Uri = "https://example.com/search?safe=active".parse().unwrap();
        let response = safe_search_redirect(&location);
        assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
        assert_eq!(
            response
                .headers()
                .get(http::header::LOCATION)
                .unwrap()
                .to_str()
                .unwrap(),
            location.to_string()
        );
    }

    #[tokio::test]
    async fn ip_connect_requires_inspection_when_dns_metadata_is_missing() {
        let (_manager, adapter) = adapter_with_resistance(false).await;
        let decider = ProxyInterceptDecider::new(adapter.interception_hosts.clone());
        for target in ["203.0.113.7:443", "[2001:db8::7]:443"] {
            let authority = target.parse().unwrap();
            let meta = ConnMeta {
                client_addr: "127.0.0.1:12345".parse().unwrap(),
                protocol: mitm_proxy::Protocol::Http1,
                is_tls: false,
                authority: target.into(),
            };
            assert!(matches!(
                adapter.inspect_connect(&meta, &authority).await,
                RequestAction::Continue
            ));
            assert!(decider.should_intercept(&authority, None));
            assert!(adapter.should_terminate_tunnel(&authority, false).await);
            assert!(!adapter.should_terminate_tunnel(&authority, true).await);
            let mut upgraded_meta = meta;
            upgraded_meta.is_tls = true;
            assert!(
                !adapter
                    .should_terminate_upgraded(
                        &upgraded_meta,
                        &"https://ordinary.example/socket".parse().unwrap()
                    )
                    .await
            );
        }
    }

    #[test]
    fn interception_is_opaque_by_default_and_enabled_only_by_policy() {
        let interception_hosts = Arc::new(StdRwLock::new(HashSet::new()));
        let decider = ProxyInterceptDecider::new(interception_hosts.clone());
        let ordinary: http::uri::Authority = "ordinary.example:443".parse().unwrap();
        let inspected: http::uri::Authority = "inspect.example:443".parse().unwrap();

        assert!(!decider.should_intercept(&ordinary, None));
        assert!(!decider.should_intercept(&inspected, None));

        interception_hosts
            .write()
            .unwrap()
            .insert("inspect.example".to_string());

        assert!(!decider.should_intercept(&ordinary, None));
        assert!(decider.should_intercept(&inspected, None));
    }

    #[tokio::test]
    async fn opaque_connect_remains_blockable_by_hostname() {
        let (manager, adapter) = adapter_with_resistance(false).await;

        {
            let mut manager = manager.write().await;
            let list_id = manager
                .create_network_block("Opaque transport test".into())
                .unwrap();
            manager
                .add_schedule_entries(
                    &list_id,
                    vec![
                        crate::core::ScheduleEntry::from(
                            crate::core::ScheduleEntryId::from(uuid::Uuid::now_v7()),
                            list_id.clone(),
                            chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
                            chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
                            chrono::WeekdaySet::ALL,
                        )
                        .unwrap(),
                    ],
                )
                .unwrap();
            manager
                .add_user_defined_entries(
                    &list_id,
                    vec![crate::core::UserDefinedRule::new(
                        "example.com".into(),
                        crate::core::UserDefinedRuleType::Domain,
                    )],
                )
                .unwrap();
            manager
                .set_lock(
                    &list_id,
                    crate::core::Lock::from(crate::core::LockState::ActiveUntilLockEnd {
                        end: chrono::Local::now() + chrono::Duration::hours(1),
                    }),
                )
                .unwrap();
        }

        let meta = ConnMeta {
            client_addr: "127.0.0.1:12345".parse().unwrap(),
            protocol: mitm_proxy::Protocol::Http1,
            is_tls: false,
            authority: "example.com:443".into(),
        };
        let authority: http::uri::Authority = "example.com:443".parse().unwrap();

        match adapter.inspect_connect(&meta, &authority).await {
            RequestAction::Respond(response) => {
                assert_eq!(response.status(), StatusCode::NOT_FOUND)
            }
            RequestAction::Continue => panic!("blocked hostname CONNECT must not be tunneled"),
        }
        let ip_meta = ConnMeta {
            authority: "203.0.113.7:443".into(),
            is_tls: true,
            ..meta
        };
        assert!(
            adapter
                .should_terminate_upgraded(&ip_meta, &"https://example.com/socket".parse().unwrap())
                .await,
            "an IP CONNECT upgrade must still obey its recovered request hostname"
        );
    }
    #[tokio::test]
    async fn proxy_readiness_requires_a_bound_listener() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        wait_for_proxy_listener(addr, std::time::Duration::from_millis(100))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn proxy_readiness_times_out_when_listener_is_absent() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        assert!(
            wait_for_proxy_listener(addr, std::time::Duration::from_millis(40))
                .await
                .is_err()
        );
    }
    use crate::db::EngineDB;
    use crate::manager::NetworkManager;
    use http_body_util::BodyExt;

    /// Builds a ProxyAdapter whose network manager has a resistance script for
    /// `example.com` and whose account is Pro (api key + fresh check + is_pro
    /// settings flag, so `AccountManager::is_pro()` never hits the network).
    async fn adapter_with_resistance(
        resistance_enabled: bool,
    ) -> (Arc<RwLock<NetworkManager>>, ProxyAdapter) {
        let temp = tempfile::tempdir().unwrap();
        let db = Arc::new(
            EngineDB::from(&temp.path().join("test.sqlite")).expect("Failed to open test DB"),
        );
        Box::leak(Box::new(temp)); // keep the temp dir alive for the DB file

        let account_manager = AccountManager::from(db.clone()).unwrap();
        account_manager
            .update_account(
                Some(crate::core::ApiKey::try_new("test-key".to_string()).unwrap()),
                Some(chrono::Local::now().to_rfc3339()),
            )
            .await
            .unwrap();
        account_manager
            .update_is_pro_status(true)
            .expect("failed to set pro status");

        let mut manager = NetworkManager::new_empty(db.clone(), account_manager.clone());
        manager.get_resistance_mode_mut().enabled = resistance_enabled;
        manager
            .get_resistance_mode_mut()
            .script_tree
            .insert("example.com", "console.log('boom');".to_string());
        let manager = Arc::new(RwLock::new(manager));

        let adapter = ProxyAdapter {
            network_manager: manager.clone(),
            account_manager,
            interception_hosts: Arc::new(StdRwLock::new(HashSet::new())),
        };
        (manager, adapter)
    }

    fn html_response(body: &str) -> BufferedResponse {
        http::Response::builder()
            .status(StatusCode::OK)
            .header(http::header::CONTENT_TYPE, "text/html")
            .body(Bytes::from(body.to_string()))
            .unwrap()
    }

    fn document_request(host: &str) -> http::request::Parts {
        http::Request::builder()
            .method(http::Method::GET)
            .uri(format!("http://{}/", host))
            .header("sec-fetch-dest", "document")
            .header("host", host)
            .body(())
            .unwrap()
            .into_parts()
            .0
    }

    fn conn_meta(host: &str) -> ConnMeta {
        ConnMeta {
            client_addr: "127.0.0.1:12345".parse().unwrap(),
            protocol: mitm_proxy::Protocol::Http1,
            is_tls: false,
            authority: host.to_string(),
        }
    }

    const HTML: &str = "<!DOCTYPE html><html><head><title>t</title></head><body>hi</body></html>";

    #[tokio::test]
    async fn response_mode_buffers_only_html_documents() {
        let (_manager, adapter) = adapter_with_resistance(true).await;
        let meta = conn_meta("example.com");
        let document = document_request("example.com");
        let mut html = html_response(HTML);
        adapter
            .inspect_response_head(&meta, &document, &mut html)
            .await;
        assert_eq!(
            adapter.response_body_mode(&meta, &document, &html),
            ResponseBodyMode::Buffer
        );

        let events = http::Response::builder()
            .status(StatusCode::OK)
            .header(http::header::CONTENT_TYPE, "text/event-stream")
            .body(Bytes::new())
            .unwrap();
        assert_eq!(
            adapter.response_body_mode(&meta, &document, &events),
            ResponseBodyMode::Stream,
            "SSE must never wait for a complete response body"
        );

        let api_request = http::Request::builder()
            .method(http::Method::POST)
            .uri("https://example.com/api/stream")
            .header("host", "example.com")
            .body(())
            .unwrap()
            .into_parts()
            .0;
        let json = http::Response::builder()
            .status(StatusCode::OK)
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(Bytes::new())
            .unwrap();
        assert_eq!(
            adapter.response_body_mode(&meta, &api_request, &json),
            ResponseBodyMode::Stream
        );
    }

    #[tokio::test]
    async fn resistance_disabled_html_streams_without_buffering() {
        let (_manager, adapter) = adapter_with_resistance(false).await;
        let meta = conn_meta("example.com");
        let document = document_request("example.com");
        let mut html = html_response(HTML);
        adapter
            .inspect_response_head(&meta, &document, &mut html)
            .await;
        assert_eq!(
            adapter.response_body_mode(&meta, &document, &html),
            ResponseBodyMode::Stream
        );
    }

    #[tokio::test]
    async fn test_resistance_disabled_injects_nothing() {
        let (_manager, adapter) = adapter_with_resistance(false).await;

        let mut res = html_response(HTML);
        let action = adapter
            .inspect_response(
                &conn_meta("example.com"),
                &document_request("example.com"),
                &mut res,
            )
            .await;

        assert!(
            matches!(action, ResponseAction::Continue),
            "disabled resistance mode must not be blocked"
        );
        let body = String::from_utf8_lossy(res.body());
        assert!(
            !body.contains("__BOREDOM_INTENSITY__"),
            "disabled resistance mode must not inject the intensity marker"
        );
        assert!(!body.contains("console.log('boom')"));
        assert!(
            body.contains("<title>t</title>"),
            "body should be untouched"
        );
    }

    #[tokio::test]
    async fn test_resistance_enabled_injects_scripts() {
        let (_manager, adapter) = adapter_with_resistance(true).await;

        let mut res = html_response(HTML);
        res.headers_mut().insert(
            http::header::ETAG,
            http::HeaderValue::from_static("\"upstream\""),
        );
        res.headers_mut().insert(
            http::header::LAST_MODIFIED,
            http::HeaderValue::from_static("Wed, 21 Oct 2015 07:28:00 GMT"),
        );
        let action = adapter
            .inspect_response(
                &conn_meta("example.com"),
                &document_request("example.com"),
                &mut res,
            )
            .await;

        assert!(
            matches!(action, ResponseAction::Continue),
            "enabled resistance mode must not be blocked"
        );
        let body = String::from_utf8_lossy(res.body());
        assert!(body.contains("window.__BOREDOM_INTENSITY__"));
        assert!(body.contains("console.log('boom')"));
        assert_eq!(
            res.headers().get(http::header::CACHE_CONTROL).unwrap(),
            "no-store"
        );
        assert!(res.headers().get(http::header::ETAG).is_none());
        assert!(res.headers().get(http::header::LAST_MODIFIED).is_none());
        assert!(
            res.headers()
                .get_all(http::header::VARY)
                .iter()
                .any(|value| value == "Sec-Fetch-Dest")
        );
    }

    #[tokio::test]
    async fn test_resistance_injection_rewrites_every_csp_policy_with_body_nonce() {
        let (_manager, adapter) = adapter_with_resistance(true).await;
        let html = "<!DOCTYPE html><html><head><meta http-equiv=\"Content-Security-Policy\" content=\"script-src 'self'; script-src-elem 'self'\"></head><body>hi</body></html>";
        let mut res = html_response(html);
        let csp_header = http::header::HeaderName::from_static("content-security-policy");
        res.headers_mut()
            .append(csp_header.clone(), "script-src 'self'".parse().unwrap());
        res.headers_mut().append(
            csp_header.clone(),
            "script-src-elem 'self'; sandbox allow-scripts"
                .parse()
                .unwrap(),
        );

        adapter
            .inspect_response(
                &conn_meta("example.com"),
                &document_request("example.com"),
                &mut res,
            )
            .await;

        let body = String::from_utf8_lossy(res.body());
        let nonce = body
            .split("nonce=\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .expect("injected script nonce");
        let nonce_source = format!("'nonce-{nonce}'");

        let csp_values: Vec<&str> = res
            .headers()
            .get_all(&csp_header)
            .iter()
            .map(|value| value.to_str().unwrap())
            .collect();
        assert_eq!(csp_values.len(), 2);
        assert!(
            csp_values
                .iter()
                .all(|policy| policy.contains(&nonce_source))
        );
        assert!(csp_values[1].contains("sandbox allow-scripts"));
        assert!(
            body.contains(&nonce_source),
            "meta CSP must use the same nonce"
        );
    }

    #[tokio::test]
    async fn test_resistance_does_not_relax_csp_when_host_has_no_scripts() {
        let (_manager, adapter) = adapter_with_resistance(true).await;
        let mut res = html_response(HTML);
        let csp_header = http::header::HeaderName::from_static("content-security-policy");
        let original_csp = "script-src 'self'; sandbox";
        res.headers_mut()
            .insert(csp_header.clone(), original_csp.parse().unwrap());

        adapter
            .inspect_response(
                &conn_meta("other.example"),
                &document_request("other.example"),
                &mut res,
            )
            .await;

        assert_eq!(res.headers().get(&csp_header).unwrap(), original_csp);
        assert_eq!(String::from_utf8_lossy(res.body()), HTML);
    }

    // -----------------------------------------------------------------------
    #[tokio::test]
    async fn response_mode_does_not_guess_document_from_accept_header() {
        let (_manager, adapter) = adapter_with_resistance(true).await;
        let meta = conn_meta("example.com");
        let request = http::Request::builder()
            .method(http::Method::GET)
            .uri("https://example.com/frame")
            .header("host", "example.com")
            .header(http::header::ACCEPT, "text/html")
            .body(())
            .unwrap()
            .into_parts()
            .0;
        let mut response = html_response(HTML);

        assert_eq!(
            adapter.response_body_mode(&meta, &request, &response),
            ResponseBodyMode::Stream
        );

        adapter
            .inspect_response(&meta, &request, &mut response)
            .await;
        assert_eq!(String::from_utf8_lossy(response.body()), HTML);
    }

    #[tokio::test]
    async fn resistance_injection_preserves_unsafe_inline_policy() {
        let (_manager, adapter) = adapter_with_resistance(true).await;
        let csp_header = http::header::HeaderName::from_static("content-security-policy");
        let original_csp = "script-src 'self' 'unsafe-inline'";
        let mut response = html_response(HTML);
        response
            .headers_mut()
            .insert(csp_header.clone(), original_csp.parse().unwrap());

        adapter
            .inspect_response(
                &conn_meta("example.com"),
                &document_request("example.com"),
                &mut response,
            )
            .await;

        assert_eq!(response.headers().get(&csp_header).unwrap(), original_csp);
        let body = String::from_utf8_lossy(response.body());
        assert!(body.contains("console.log('boom')"));
    }

    #[tokio::test]
    async fn resistance_injection_skips_blocking_csp_sandbox() {
        let (_manager, adapter) = adapter_with_resistance(true).await;
        let csp_header = http::header::HeaderName::from_static("content-security-policy");
        let original_csp = "script-src 'self'; sandbox allow-forms";
        let mut response = html_response(HTML);
        response
            .headers_mut()
            .insert(csp_header.clone(), original_csp.parse().unwrap());

        adapter
            .inspect_response(
                &conn_meta("example.com"),
                &document_request("example.com"),
                &mut response,
            )
            .await;

        assert_eq!(response.headers().get(&csp_header).unwrap(), original_csp);
        assert_eq!(String::from_utf8_lossy(response.body()), HTML);
    }

    // Interception contract — end-to-end CONNECT regression tests
    // -----------------------------------------------------------------------

    /// Inspector that records whether a request ever reached the inspect
    /// point. A tunnel that is not MITM-terminated never produces a decrypted
    /// request, so the flag is a direct signal that interception ran.
    #[derive(Default)]
    struct RecordingInspector {
        saw_request: Arc<std::sync::atomic::AtomicBool>,
    }

    #[async_trait]
    impl RequestInspector for RecordingInspector {
        async fn inspect_request(
            &self,
            _meta: &ConnMeta,
            _req: &mut BufferedRequest,
        ) -> RequestAction {
            self.saw_request
                .store(true, std::sync::atomic::Ordering::SeqCst);
            RequestAction::Continue
        }
    }

    #[async_trait]
    impl ResponseInspector for RecordingInspector {
        async fn inspect_response(
            &self,
            _meta: &ConnMeta,
            _req_head: &http::request::Parts,
            _res: &mut BufferedResponse,
        ) -> ResponseAction {
            ResponseAction::Continue
        }
    }

    /// Reserve an ephemeral TCP port, then release it so the proxy can bind it.
    async fn ephemeral_addr() -> SocketAddr {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral listener");
        let addr = listener.local_addr().unwrap();
        drop(listener);
        addr
    }

    /// Spawn a local HTTPS origin answering every request with a canned JSON
    /// body. The certificate is self-signed for `localhost`; the test config's
    /// `verify_upstream(false)` lets the proxy accept it.
    async fn spawn_https_echo_origin() -> (SocketAddr, String) {
        let key_pair = rcgen::KeyPair::generate().expect("generate origin key");
        let mut params = rcgen::CertificateParams::default();
        params.is_ca = rcgen::IsCa::NoCa;
        params.subject_alt_names.push(rcgen::SanType::DnsName(
            "localhost".try_into().expect("valid dns name"),
        ));
        let cert = params
            .self_signed(&key_pair)
            .expect("self-sign origin certificate");

        let cert_pem = cert.pem();
        let cert_der = rustls::pki_types::CertificateDer::from(cert.der().to_vec());
        let key_der = rustls::pki_types::PrivatePkcs8KeyDer::from(key_pair.serialize_der());
        let mut server_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert_der], key_der.into())
            .expect("origin tls config");
        // Advertise only http/1.1: the origin speaks raw HTTP/1.1, so the
        // proxy's upstream client must not negotiate h2 here.
        server_config.alpn_protocols = vec![b"http/1.1".to_vec()];
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server_config));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind origin listener");
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let (stream, _) = match listener.accept().await {
                    Ok(pair) => pair,
                    Err(_) => return,
                };
                let acceptor = acceptor.clone();
                tokio::spawn(async move {
                    use tokio::io::{AsyncReadExt, AsyncWriteExt};
                    let mut tls = match acceptor.accept(stream).await {
                        Ok(tls) => tls,
                        Err(_) => return,
                    };
                    // Read the request head, then answer with a canned echo.
                    let mut head = Vec::new();
                    let mut byte = [0u8; 1];
                    while head.len() < 16 * 1024 {
                        match tls.read(&mut byte).await {
                            Ok(0) | Err(_) => return,
                            Ok(_) => {
                                head.push(byte[0]);
                                if head.ends_with(b"\r\n\r\n") {
                                    break;
                                }
                            }
                        }
                    }
                    let body = r#"{"origin":"ace-engine-echo","intercepted":true}"#;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: \
                         {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = tls.write_all(response.as_bytes()).await;
                });
            }
        });
        (addr, cert_pem)
    }

    /// CONNECT through the proxy and issue a GET, trusting the supplied CA.
    ///
    /// Returns `Err` when the tunnel was not MITM-terminated: a client that
    /// trusts only the proxy CA can only complete TLS against the proxy's
    /// per-host leaf, never against the self-signed origin an opaque
    /// (non-intercepted) tunnel would expose.
    async fn connect_get_through_proxy(
        proxy_addr: SocketAddr,
        ca_pem: &str,
        host: &str,
        port: u16,
    ) -> Result<(StatusCode, String), Box<dyn std::error::Error>> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let mut roots = rustls::RootCertStore::empty();
        for cert in rustls_pemfile::certs(&mut ca_pem.as_bytes()).filter_map(Result::ok) {
            roots.add(cert)?;
        }
        let mut tls_config = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        let tls_config = Arc::new(tls_config);

        let mut tcp = tokio::net::TcpStream::connect(proxy_addr).await?;
        tcp.set_nodelay(true)?;

        let connect_req = format!("CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\n\r\n");
        tcp.write_all(connect_req.as_bytes()).await?;

        let mut buf = [0u8; 4096];
        let mut total = 0;
        let mut found_end = false;
        while total < buf.len() {
            let n = tcp.read(&mut buf[total..total + 1]).await?;
            if n == 0 {
                break;
            }
            total += n;
            if total >= 4 && &buf[total - 4..total] == b"\r\n\r\n" {
                found_end = true;
                break;
            }
        }
        if !found_end {
            return Err("CONNECT response was not terminated".into());
        }
        let head = String::from_utf8_lossy(&buf[..total]);
        if !head.contains("200") {
            return Err(format!("CONNECT did not return 200: {head}").into());
        }

        let tls_stream = tokio_rustls::TlsConnector::from(tls_config)
            .connect(
                rustls::pki_types::ServerName::try_from(host.to_string())?,
                tcp,
            )
            .await?;

        let io = hyper_util::rt::TokioIo::new(tls_stream);
        let request = http::Request::builder()
            .method(http::Method::GET)
            .uri("/echo")
            .header(http::header::HOST, host)
            .body(http_body_util::Full::new(Bytes::new()))?;
        let (mut sender, connection) = hyper::client::conn::http1::handshake(io).await?;
        tokio::spawn(async move {
            let _ = connection.await;
        });
        let response = sender.send_request(request).await?;
        let status = response.status();
        let body = response.into_body().collect().await?.to_bytes();
        Ok((status, String::from_utf8_lossy(&body).to_string()))
    }

    #[tokio::test]
    async fn ordinary_connect_tunnel_stays_opaque() {
        let _ = rustls::crypto::ring::default_provider().install_default();

        let (origin_addr, origin_ca_pem) = spawn_https_echo_origin().await;
        let ca_dir = tempfile::tempdir().expect("ca tempdir");
        let listen_addr = ephemeral_addr().await;

        let saw_request = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let recording = Arc::new(RecordingInspector {
            saw_request: saw_request.clone(),
        });
        let proxy = wire_interception(
            ProxyConfig::new(listen_addr, ca_dir.path()).verify_upstream(false),
            recording.clone() as Arc<dyn RequestInspector>,
            recording as Arc<dyn ResponseInspector>,
            Arc::new(StdRwLock::new(HashSet::new())),
        );
        let handle = tokio::spawn(async move {
            let _ = proxy.run().await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(15),
            connect_get_through_proxy(listen_addr, &origin_ca_pem, "localhost", origin_addr.port()),
        )
        .await;

        handle.abort();

        let (status, body) = result
            .expect("opaque CONNECT test must not hang")
            .expect("TLS must complete directly against the origin");
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("ace-engine-echo"));
        assert!(
            !saw_request.load(std::sync::atomic::Ordering::SeqCst),
            "an ordinary HTTPS request must not reach the decrypted request inspector"
        );
    }

    /// The interception contract, end-to-end: a real CONNECT through a proxy
    /// wired exactly like [`AceProxy::start`] (via [`wire_interception`]) must
    /// be MITM-terminated — the client completes TLS against the proxy CA and
    /// the decrypted request reaches the inspect point.
    ///
    /// Fails if interception is disabled, e.g. if a future mitm-proxy whose
    /// default decider intercepts nothing gets pulled in and this explicit
    /// wiring regresses — that would silently disable ALL HTTPS blocking.
    #[tokio::test]
    async fn connect_tunnel_is_mitm_intercepted() {
        let _ = rustls::crypto::ring::default_provider().install_default();

        let (origin_addr, _origin_ca_pem) = spawn_https_echo_origin().await;
        let ca_dir = tempfile::tempdir().expect("ca tempdir");
        let listen_addr = ephemeral_addr().await;

        let saw_request = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let recording = Arc::new(RecordingInspector {
            saw_request: saw_request.clone(),
        });
        let proxy = wire_interception(
            ProxyConfig::new(listen_addr, ca_dir.path()).verify_upstream(false),
            recording.clone() as Arc<dyn RequestInspector>,
            recording as Arc<dyn ResponseInspector>,
            Arc::new(StdRwLock::new(HashSet::from(["localhost".to_string()]))),
        );
        let handle = tokio::spawn(async move {
            let _ = proxy.run().await;
        });
        // Let the proxy bind its listener and generate the CA.
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        let ca_pem = std::fs::read_to_string(ca_dir.path().join("mitm_ca.pem"))
            .expect("proxy CA must be generated on startup");

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(15),
            connect_get_through_proxy(listen_addr, &ca_pem, "localhost", origin_addr.port()),
        )
        .await;

        handle.abort();

        let (status, body) = result
            .expect("CONNECT test must not hang")
            .expect("TLS through the proxy must succeed: the tunnel was not MITM-terminated");
        assert_eq!(status, StatusCode::OK);
        assert!(
            body.contains("intercepted"),
            "expected the origin echo body, got: {body}"
        );
        assert!(
            saw_request.load(std::sync::atomic::Ordering::SeqCst),
            "the decrypted request must reach the inspect point — interception did not happen"
        );
    }

    /// Negative control for [`connect_tunnel_is_mitm_intercepted`]: the same
    /// client setup against a proxy whose decider rejects the tunnel cannot
    /// complete TLS. This proves the positive test really measures
    /// interception and would catch a regressed engine wiring.
    #[tokio::test]
    async fn connect_tunnel_without_interception_fails_tls() {
        let _ = rustls::crypto::ring::default_provider().install_default();

        let (origin_addr, _origin_ca_pem) = spawn_https_echo_origin().await;
        let ca_dir = tempfile::tempdir().expect("ca tempdir");
        let listen_addr = ephemeral_addr().await;

        let recording = Arc::new(RecordingInspector {
            saw_request: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        });
        let proxy =
            MitmProxy::new(ProxyConfig::new(listen_addr, ca_dir.path()).verify_upstream(false))
                .with_request_inspector(recording.clone() as Arc<dyn RequestInspector>)
                .with_response_inspector(recording as Arc<dyn ResponseInspector>)
                .with_intercept_decider(Arc::new(mitm_proxy::NoInterceptDecider));
        let handle = tokio::spawn(async move {
            let _ = proxy.run().await;
        });
        // Let the proxy bind its listener and generate the CA.
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        let ca_pem = std::fs::read_to_string(ca_dir.path().join("mitm_ca.pem"))
            .expect("proxy CA must be generated on startup");

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(15),
            connect_get_through_proxy(listen_addr, &ca_pem, "localhost", origin_addr.port()),
        )
        .await;

        handle.abort();

        let result = result.expect("CONNECT test must not hang");
        assert!(
            result.is_err(),
            "a non-intercepted tunnel must not complete TLS with the proxy CA, got: {result:?}"
        );
    }
}
