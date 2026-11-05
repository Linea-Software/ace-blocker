//! # mitm-proxy
//!
//! A standalone man-in-the-middle proxy that terminates and inspects every
//! major HTTP version and its secure counterpart:
//!
//! * **HTTP/1.0 / HTTP/1.1** — forward proxy (plaintext forwarding and CONNECT
//!   tunnelling), TLS terminated with a dynamically-issued per-host cert.
//! * **HTTP/2** — served over the decrypted CONNECT tunnel, chosen by ALPN,
//!   with multiplexed streams handled by hyper.
//! * **HTTP/3 (QUIC)** — a QUIC endpoint that terminates the transport and
//!   decodes HTTP/3 streams (see [`mod@http3`]).
//!
//! At the centre is a [`transparent inspect point`](inspect): every decrypted
//! request and response passes through caller-supplied
//! [`RequestInspector`](inspect::RequestInspector) /
//! [`ResponseInspector`](inspect::ResponseInspector) hooks before being
//! forwarded, where it can be observed, modified, or blocked.
//!
//! ## Quick start
//!
//! ```no_run
//! use std::sync::Arc;
//! use mitm_proxy::{MitmProxy, ProxyConfig};
//! use mitm_proxy::inspect::LoggingInspector;
//!
//! # async fn run() -> mitm_proxy::Result<()> {
//! let config = ProxyConfig::new("127.0.0.1:8080".parse().unwrap(), "./ca");
//! MitmProxy::new(config)
//!     .with_request_inspector(Arc::new(LoggingInspector))
//!     .with_response_inspector(Arc::new(LoggingInspector))
//!     .run()
//!     .await
//! # }
//! ```
//!
//! The proxy is independently usable. Ace Engine wires its own inspectors and
//! a restrictive HTTP/3 SNI policy at the integration boundary.

pub mod ca;
pub mod config;
pub mod error;
pub mod inspect;
pub mod intercept;

mod cert_resolver;
mod client_hello;
mod http;
#[cfg(feature = "http3")]
mod http3;
mod state;
mod tls;
mod upstream;
mod util;

use std::sync::Arc;

use tracing::info;

pub use ca::CertAuthority;
pub use config::ProxyConfig;
pub use error::{ProxyError, Result};
pub use inspect::{
    BufferedRequest, BufferedResponse, ConnMeta, Inspectors, Protocol, RequestAction,
    RequestBodyMode, RequestInspector, ResponseAction, ResponseBodyMode, ResponseInspector,
};
pub use intercept::{InterceptDecider, NoInterceptDecider};

use cert_resolver::DynamicCertResolver;
use inspect::NoopInspector;
use state::ProxyState;
use upstream::Upstream;

/// A configured, ready-to-run proxy.
///
/// Build one with [`MitmProxy::new`], attach inspectors, then [`run`](MitmProxy::run).
pub struct MitmProxy {
    config: ProxyConfig,
    inspectors: Inspectors,
    intercept_decider: Arc<dyn InterceptDecider>,
    #[cfg(feature = "http3")]
    http3_sni_policy: Option<Arc<dyn Fn(&str) -> bool + Send + Sync>>,
}

impl MitmProxy {
    /// Create a proxy from `config` with no-op inspectors (forwards everything
    /// unchanged).
    pub fn new(config: ProxyConfig) -> Self {
        Self {
            config,
            inspectors: Inspectors::default(),
            intercept_decider: Arc::new(NoInterceptDecider),
            #[cfg(feature = "http3")]
            http3_sni_policy: None,
        }
    }

    /// Set the request inspection hook.
    pub fn with_request_inspector(mut self, inspector: Arc<dyn RequestInspector>) -> Self {
        self.inspectors.request = inspector;
        self
    }

    /// Set the response inspection hook.
    pub fn with_response_inspector(mut self, inspector: Arc<dyn ResponseInspector>) -> Self {
        self.inspectors.response = inspector;
        self
    }

    /// Set both inspection hooks at once.
    pub fn with_inspectors(mut self, inspectors: Inspectors) -> Self {
        self.inspectors = inspectors;
        self
    }

    /// Set the interception decision hook.
    ///
    /// Tunnels for which `should_intercept` returns `false` are relayed to the
    /// origin byte-for-byte without any TLS termination; only hosts the
    /// decider approves ever get a leaf certificate minted. Without this the
    /// proxy intercepts nothing (see [`NoInterceptDecider`]).
    pub fn with_intercept_decider(mut self, decider: Arc<dyn InterceptDecider>) -> Self {
        self.intercept_decider = decider;
        self
    }

    /// Restrict HTTP/3 interception to approved SNI and request authority hosts.
    ///
    /// Missing or denied SNI fails the QUIC TLS handshake without minting a
    /// certificate, allowing clients to fall back to TCP. Denied request
    /// authorities receive 421 to prevent cross-origin connection coalescing.
    /// Without a policy, standalone HTTP/3 behavior is unchanged.
    #[cfg(feature = "http3")]
    pub fn with_http3_sni_policy(
        mut self,
        policy: Arc<dyn Fn(&str) -> bool + Send + Sync>,
    ) -> Self {
        self.http3_sni_policy = Some(policy);
        self
    }

    /// Run the proxy until a listener fails or the process is terminated.
    ///
    /// Starts the HTTP/1.x + HTTP/2 TCP listener and, if
    /// [`ProxyConfig::http3_addr`] is set, the HTTP/3 QUIC listener.
    pub async fn run(self) -> Result<()> {
        install_crypto_provider();

        // --- certificate authority --------------------------------------
        let ca = Arc::new(CertAuthority::load_or_generate(
            &self.config.ca_cert_path,
            &self.config.ca_key_path,
        )?);
        if self.config.install_ca {
            ca.install_to_trust_store(&self.config.ca_cert_path)?;
        }
        info!(
            "CA ready ({}); clients must trust this certificate to avoid warnings",
            self.config.ca_cert_path.display()
        );

        // --- TLS configs ------------------------------------------------
        let resolver = Arc::new(DynamicCertResolver::new(ca));
        let server_tls = tls::server_config(resolver.clone(), tls::TCP_ALPN);
        #[cfg(feature = "http3")]
        let h3_tls = match &self.http3_sni_policy {
            Some(policy) => tls::server_config(
                Arc::new(cert_resolver::Http3CertResolver::new(
                    resolver.clone(),
                    policy.clone(),
                )),
                tls::H3_ALPN,
            ),
            None => tls::server_config(resolver.clone(), tls::H3_ALPN),
        };
        let client_tls = tls::client_config(self.config.verify_upstream, tls::TCP_ALPN)?;

        // --- shared state -----------------------------------------------
        let state = Arc::new(ProxyState {
            server_tls,
            cert_resolver: resolver,
            #[cfg(feature = "http3")]
            h3_tls,
            upstream: Upstream::new(client_tls),
            inspectors: self.inspectors,
            config: Arc::new(self.config.clone()),
            intercept_decider: self.intercept_decider,
        });

        // --- listeners --------------------------------------------------
        let tcp = http::serve(state.clone());

        #[cfg(not(feature = "http3"))]
        return tcp.await;

        #[cfg(feature = "http3")]
        match self.config.http3_addr {
            Some(addr) => {
                let quic = http3::serve(state, addr, self.http3_sni_policy);
                // Run both; return as soon as either fails.
                tokio::try_join!(tcp, quic).map(|_| ())
            }
            None => tcp.await,
        }
    }
}

/// Convenience re-export of a no-op inspector for callers that only want to
/// observe one side of the exchange.
pub fn noop_inspector() -> Arc<NoopInspector> {
    Arc::new(NoopInspector)
}

/// Install the ring-based rustls crypto provider as the process default, unless
/// one is already installed. Safe to call more than once.
fn install_crypto_provider() {
    // Ignore the error: it only means a provider was already installed.
    let _ = rustls::crypto::ring::default_provider().install_default();
}
