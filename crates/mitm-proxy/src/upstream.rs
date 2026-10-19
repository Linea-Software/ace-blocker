//! Forwarding decrypted requests to origin servers.
//!
//! Ordinary HTTPS requests reuse a per-origin HTTP/2 connection when the
//! origin negotiates h2. HTTP/1.1 upgrades and HTTP/2 extended CONNECT keep
//! dedicated connections because they turn the request body into a long-lived
//! protocol stream.

use std::collections::HashMap;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex as StdMutex};
use std::task::Poll;
use std::time::Duration;

use bytes::Bytes;
use eyre::{Context, eyre};
use http::header::{HOST, HeaderValue};
use http::{Request, Response, Version};
use http_body_util::{BodyExt, combinators::UnsyncBoxBody};
use hyper::body::{Body, Incoming};
use hyper_util::rt::{TokioExecutor, TokioIo};
use rustls::ClientConfig;
use rustls::pki_types::ServerName;
use tokio::net::TcpStream;
use tokio::sync::Mutex as AsyncMutex;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;
use tracing::debug;

use crate::error::{ProxyError, Result};

type BoxError = Box<dyn Error + Send + Sync>;
type UpstreamBody = UnsyncBoxBody<Bytes, BoxError>;
type H2Sender = hyper::client::conn::http2::SendRequest<UpstreamBody>;

// Hold protocol hints and reusable h2 senders for a bounded number of origins.
// Evicting a slot never aborts already active requests holding sender clones.
const MAX_CACHED_ORIGINS: usize = 1024;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct OriginKey {
    host: String,
    port: u16,
}

impl OriginKey {
    fn new(host: &str, port: u16) -> Self {
        Self {
            host: host.trim_end_matches('.').to_ascii_lowercase(),
            port,
        }
    }
}

enum OriginProtocol {
    Unknown,
    Http1,
    Http2(H2Sender),
}

struct OriginSlot {
    protocol: AsyncMutex<OriginProtocol>,
}

impl OriginSlot {
    fn new() -> Self {
        Self {
            protocol: AsyncMutex::new(OriginProtocol::Unknown),
        }
    }
}

/// Opens connections to upstream origins and relays requests without
/// aggregating their response bodies.
#[derive(Clone)]
pub struct Upstream {
    tls: Arc<ClientConfig>,
    origins: Arc<StdMutex<HashMap<OriginKey, Arc<OriginSlot>>>>,
    client_auth_required: Arc<StdMutex<HashMap<OriginKey, bool>>>,
}

impl Upstream {
    pub fn new(tls: Arc<ClientConfig>) -> Self {
        Self {
            tls,
            origins: Arc::new(StdMutex::new(HashMap::new())),
            client_auth_required: Arc::new(StdMutex::new(HashMap::new())),
        }
    }

    /// Probe whether the origin requires a TLS client certificate. The result is
    /// cached per origin so normal intercepted browsing pays at most one extra
    /// handshake. Only an explicit TLS certificate-required alert selects the
    /// opaque compatibility path; unrelated connection failures remain ordinary
    /// upstream failures.
    pub async fn requires_client_certificate(&self, host: &str, port: u16) -> bool {
        let key = OriginKey::new(host, port);
        if let Some(required) = self
            .client_auth_required
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&key)
            .copied()
        {
            return required;
        }

        let tcp = match TcpStream::connect((host, port)).await {
            Ok(tcp) => tcp,
            Err(_) => return false,
        };
        let server_name = match ServerName::try_from(host.to_string()) {
            Ok(server_name) => server_name,
            Err(_) => return false,
        };
        let connector = TlsConnector::from(self.tls.clone());

        let required = match connector.connect(server_name, tcp).await {
            Ok(_) => false,
            Err(error)
                if error
                    .get_ref()
                    .and_then(|source| source.downcast_ref::<rustls::Error>())
                    .is_some_and(|error| {
                        matches!(
                            error,
                            rustls::Error::AlertReceived(
                                rustls::AlertDescription::CertificateRequired
                            )
                        )
                    }) =>
            {
                true
            }
            Err(_) => return false,
        };

        let mut cache = self
            .client_auth_required
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !cache.contains_key(&key) && cache.len() >= MAX_CACHED_ORIGINS {
            if let Some(oldest) = cache.keys().next().cloned() {
                cache.remove(&oldest);
            }
        }
        cache.insert(key, required);
        required
    }

    /// Send `req` to `host:port`, using TLS when `secure` is set. The response
    /// body remains streaming; callers decide whether it needs aggregation.
    pub async fn send<B>(
        &self,
        secure: bool,
        host: &str,
        port: u16,
        req: Request<B>,
    ) -> Result<Response<Incoming>>
    where
        B: Body<Data = Bytes> + Send + Unpin + 'static,
        B::Error: Into<BoxError>,
    {
        let req: Request<UpstreamBody> = req.map(|body| {
            body.map_err(|error| -> BoxError { error.into() })
                .boxed_unsync()
        });
        self.send_boxed(secure, host, port, req).await
    }

    async fn send_boxed(
        &self,
        secure: bool,
        host: &str,
        port: u16,
        req: Request<UpstreamBody>,
    ) -> Result<Response<Incoming>> {
        let authority = format!("{host}:{port}");
        let requires_h2 = req.extensions().get::<hyper::ext::Protocol>().is_some();
        let requires_h1_upgrade = req.version() == http::Version::HTTP_11
            && req.headers().contains_key(http::header::UPGRADE)
            && req
                .headers()
                .get_all(http::header::CONNECTION)
                .iter()
                .filter_map(|value| value.to_str().ok())
                .flat_map(|value| value.split(','))
                .any(|token| token.trim().eq_ignore_ascii_case("upgrade"));

        debug!(
            authority = %authority,
            secure,
            method = %req.method(),
            version = ?req.version(),
            path = %req.uri().path(),
            requires_h2,
            requires_h1_upgrade,
            "forwarding upstream request"
        );

        if secure {
            if requires_h1_upgrade {
                let tls = self
                    .connect_tls(host, port, &authority, Some(&[b"http/1.1"]))
                    .await?;
                return self.send_h1(TokioIo::new(tls), &authority, req).await;
            }

            if requires_h2 {
                let tls = self
                    .connect_tls(host, port, &authority, Some(&[b"h2"]))
                    .await?;
                if tls.get_ref().1.alpn_protocol() != Some(b"h2") {
                    return Err(ProxyError::Upstream {
                        authority,
                        source: eyre!("origin did not negotiate HTTP/2 for extended CONNECT"),
                    }
                    .into());
                }
                return self
                    .send_h2_fresh(TokioIo::new(tls), &authority, req, true)
                    .await;
            }

            return self
                .send_secure_reusable_h2(host, port, &authority, req)
                .await;
        }

        if requires_h2 {
            return Err(ProxyError::Upstream {
                authority,
                source: eyre!("extended CONNECT requires an HTTP/2 upstream"),
            }
            .into());
        }

        let tcp = self.connect_tcp(host, port, &authority).await?;
        self.send_h1(TokioIo::new(tcp), &authority, req).await
    }

    /// Use a cached HTTP/2 sender when possible. The per-origin async mutex is
    /// held across only the *first* TCP/TLS/HTTP2 handshake so concurrent first
    /// requests do not each create their own connection. Once h2 is established,
    /// callers clone the sender and multiplex outside the lock.
    async fn send_secure_reusable_h2(
        &self,
        host: &str,
        port: u16,
        authority: &str,
        req: Request<UpstreamBody>,
    ) -> Result<Response<Incoming>> {
        let slot = self.origin_slot(host, port);
        let mut protocol = slot.protocol.lock().await;

        if let OriginProtocol::Http2(sender) = &*protocol {
            if !sender.is_closed() {
                let sender = sender.clone();
                drop(protocol);
                debug!(authority = %authority, "reusing upstream HTTP/2 connection");
                return Self::send_h2_request(sender, authority, req).await;
            }
            *protocol = OriginProtocol::Unknown;
        }

        if matches!(*protocol, OriginProtocol::Http1) {
            drop(protocol);
            let tls = self
                .connect_tls(host, port, authority, Some(&[b"http/1.1"]))
                .await?;
            return self.send_h1(TokioIo::new(tls), authority, req).await;
        }

        // Keep this origin's lock while the first protocol negotiation runs.
        // Other origins are unaffected, and once h2 is cached the lock is only
        // held long enough to clone the sender.
        let tls = self.connect_tls(host, port, authority, None).await?;
        let is_h2 = tls.get_ref().1.alpn_protocol() == Some(b"h2");

        if is_h2 {
            let sender = self
                .open_h2_sender(TokioIo::new(tls), authority, false)
                .await?;
            *protocol = OriginProtocol::Http2(sender.clone());
            drop(protocol);
            debug!(authority = %authority, "cached upstream HTTP/2 connection");
            Self::send_h2_request(sender, authority, req).await
        } else {
            *protocol = OriginProtocol::Http1;
            drop(protocol);
            self.send_h1(TokioIo::new(tls), authority, req).await
        }
    }

    fn origin_slot(&self, host: &str, port: u16) -> Arc<OriginSlot> {
        let key = OriginKey::new(host, port);
        let mut origins = self
            .origins
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !origins.contains_key(&key) && origins.len() >= MAX_CACHED_ORIGINS {
            if let Some(oldest) = origins.keys().next().cloned() {
                origins.remove(&oldest);
            }
        }
        origins
            .entry(key)
            .or_insert_with(|| Arc::new(OriginSlot::new()))
            .clone()
    }

    async fn connect_tcp(&self, host: &str, port: u16, authority: &str) -> Result<TcpStream> {
        debug!(authority = %authority, "opening upstream TCP connection");
        let tcp = TcpStream::connect((host, port))
            .await
            .map_err(|error| ProxyError::Upstream {
                authority: authority.to_string(),
                source: eyre!("tcp connect: {error}"),
            })?;
        tcp.set_nodelay(true).ok();
        debug!(authority = %authority, "upstream TCP connection established");
        Ok(tcp)
    }

    async fn connect_tls(
        &self,
        host: &str,
        port: u16,
        authority: &str,
        forced_alpn: Option<&[&[u8]]>,
    ) -> Result<TlsStream<TcpStream>> {
        let tcp = self.connect_tcp(host, port, authority).await?;
        let tls_config = if let Some(alpn) = forced_alpn {
            let mut config = (*self.tls).clone();
            config.alpn_protocols = alpn.iter().map(|value| value.to_vec()).collect();
            Arc::new(config)
        } else {
            self.tls.clone()
        };
        let connector = TlsConnector::from(tls_config);
        let server_name = ServerName::try_from(host.to_string())
            .map_err(|error| eyre!("invalid upstream server name {host:?}: {error}"))?;
        debug!(authority = %authority, server_name = %host, "starting upstream TLS handshake");
        let tls =
            connector
                .connect(server_name, tcp)
                .await
                .map_err(|error| ProxyError::Upstream {
                    authority: authority.to_string(),
                    source: eyre!("tls handshake: {error}"),
                })?;

        let is_h2 = tls.get_ref().1.alpn_protocol() == Some(b"h2");
        debug!(
            authority = %authority,
            alpn = if is_h2 { "h2" } else { "http/1.1" },
            "upstream TLS handshake completed"
        );
        Ok(tls)
    }

    async fn send_h1<I>(
        &self,
        io: I,
        authority: &str,
        mut req: Request<UpstreamBody>,
    ) -> Result<Response<Incoming>>
    where
        I: hyper::rt::Read + hyper::rt::Write + Send + Unpin + 'static,
    {
        prepare_http1_request(&mut req)?;
        let (mut sender, conn) =
            hyper::client::conn::http1::handshake(io)
                .await
                .map_err(|error| ProxyError::Upstream {
                    authority: authority.to_string(),
                    source: eyre!("http/1 handshake: {error}"),
                })?;

        // `with_upgrades` is required for WebSocket and other HTTP/1.1 protocol
        // switches. Without it, `hyper::upgrade::on(response)` never resolves.
        tokio::spawn(async move {
            if let Err(error) = conn.with_upgrades().await {
                debug!("upstream http/1 connection closed: {error}");
            }
        });

        let response = sender
            .send_request(req)
            .await
            .wrap_err_with(|| format!("http/1 request to {authority} failed"))?;
        debug!(authority = %authority, status = %response.status(), "upstream HTTP/1 response received");
        Ok(response)
    }

    async fn send_h2_fresh<I>(
        &self,
        io: I,
        authority: &str,
        req: Request<UpstreamBody>,
        wait_for_extended_connect: bool,
    ) -> Result<Response<Incoming>>
    where
        I: hyper::rt::Read + hyper::rt::Write + Send + Unpin + 'static,
    {
        let sender = self
            .open_h2_sender(io, authority, wait_for_extended_connect)
            .await?;
        Self::send_h2_request(sender, authority, req).await
    }

    async fn open_h2_sender<I>(
        &self,
        io: I,
        authority: &str,
        wait_for_extended_connect: bool,
    ) -> Result<H2Sender>
    where
        I: hyper::rt::Read + hyper::rt::Write + Send + Unpin + 'static,
    {
        let (sender, mut conn) = hyper::client::conn::http2::handshake(TokioExecutor::new(), io)
            .await
            .map_err(|error| ProxyError::Upstream {
                authority: authority.to_string(),
                source: eyre!("http/2 handshake: {error}"),
            })?;

        if wait_for_extended_connect {
            let wait_for_setting = std::future::poll_fn(|cx| {
                let connection_poll = Pin::new(&mut conn).poll(cx);
                if conn.is_extended_connect_protocol_enabled() {
                    return Poll::Ready(Ok(()));
                }

                match connection_poll {
                    Poll::Ready(Ok(())) => Poll::Ready(Err(eyre!(
                        "HTTP/2 connection closed before enabling extended CONNECT"
                    ))),
                    Poll::Ready(Err(error)) => Poll::Ready(Err(eyre!(
                        "HTTP/2 connection failed before enabling extended CONNECT: {error}"
                    ))),
                    Poll::Pending => Poll::Pending,
                }
            });

            tokio::time::timeout(Duration::from_secs(5), wait_for_setting)
                .await
                .map_err(|_| ProxyError::Upstream {
                    authority: authority.to_string(),
                    source: eyre!("timed out waiting for SETTINGS_ENABLE_CONNECT_PROTOCOL"),
                })?
                .map_err(|source| ProxyError::Upstream {
                    authority: authority.to_string(),
                    source,
                })?;
        }

        tokio::spawn(async move {
            if let Err(error) = conn.await {
                debug!("upstream http/2 connection closed: {error}");
            }
        });

        Ok(sender)
    }

    async fn send_h2_request(
        mut sender: H2Sender,
        authority: &str,
        req: Request<UpstreamBody>,
    ) -> Result<Response<Incoming>> {
        sender
            .ready()
            .await
            .wrap_err_with(|| format!("http/2 connection to {authority} is not ready"))?;
        let response = sender
            .send_request(req)
            .await
            .wrap_err_with(|| format!("http/2 request to {authority} failed"))?;
        debug!(authority = %authority, status = %response.status(), "upstream HTTP/2 response received");
        Ok(response)
    }
}

/// Normalize only after selecting the upstream protocol: the incoming version
/// describes the browser connection, which may use HTTP/2 or HTTP/3 instead.
fn prepare_http1_request<B>(request: &mut Request<B>) -> Result<()> {
    // The effective URI (including inspector changes) is authoritative. Hyper's
    // low-level HTTP/1 sender does not create Host from an absolute URI or from
    // HTTP/2's :authority. Replace conflicting Host values as RFC 9112 requires.
    if let Some(authority) = request.uri().authority() {
        let host = HeaderValue::from_str(authority.as_str())
            .wrap_err("invalid authority for upstream Host header")?;
        request.headers_mut().insert(HOST, host);
    } else if !request.headers().contains_key(HOST) {
        return Err(eyre!(
            "HTTP/1 upstream request has neither authority nor Host"
        ));
    }

    if matches!(request.version(), Version::HTTP_2 | Version::HTTP_3) {
        *request.version_mut() = Version::HTTP_11;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn certificate_required_alert_is_detectable_through_io_error() {
        let error = std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            rustls::Error::AlertReceived(rustls::AlertDescription::CertificateRequired),
        );
        assert!(
            error
                .get_ref()
                .and_then(|source| source.downcast_ref::<rustls::Error>())
                .is_some_and(|error| matches!(
                    error,
                    rustls::Error::AlertReceived(rustls::AlertDescription::CertificateRequired)
                ))
        );
    }
    use http_body_util::Full;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn http2_and_http3_downgrades_emit_http11_with_authoritative_host() {
        for version in [Version::HTTP_2, Version::HTTP_3] {
            for authority in ["example.com", "example.com:8443", "[::1]:8443"] {
                let (client, mut origin) = tokio::io::duplex(4096);
                let origin_task = tokio::spawn(async move {
                    let mut wire = Vec::new();
                    while !wire.ends_with(b"\r\n\r\n") {
                        wire.push(origin.read_u8().await.unwrap());
                        assert!(wire.len() < 4096);
                    }
                    origin
                        .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                        .await
                        .unwrap();
                    String::from_utf8(wire).unwrap()
                });
                let upstream =
                    Upstream::new(crate::tls::client_config(true, crate::tls::TCP_ALPN).unwrap());
                let request = Request::builder()
                    .uri(format!("https://{authority}/path?query=value"))
                    .version(version)
                    .header(HOST, "stale.example")
                    .header(HOST, "duplicate.example")
                    .body(
                        Full::new(Bytes::new())
                            .map_err(|never| -> BoxError { match never {} })
                            .boxed_unsync(),
                    )
                    .unwrap();
                let response = tokio::time::timeout(
                    Duration::from_secs(5),
                    upstream.send_h1(TokioIo::new(client), authority, request),
                )
                .await
                .unwrap()
                .unwrap();
                assert_eq!(response.status(), http::StatusCode::OK);
                let wire = origin_task.await.unwrap();
                assert!(wire.lines().next().unwrap().ends_with(" HTTP/1.1"));
                let hosts: Vec<_> = wire
                    .lines()
                    .filter(|line| line.starts_with("host:"))
                    .collect();
                assert_eq!(hosts, [format!("host: {authority}")]);
            }
        }
    }

    #[test]
    fn http1_upgrade_headers_body_and_version_survive_preparation() {
        let mut request = Request::builder()
            .uri("https://example.com:8443/socket")
            .version(Version::HTTP_11)
            .header("connection", "Upgrade")
            .header("upgrade", "websocket")
            .body("payload")
            .unwrap();
        prepare_http1_request(&mut request).unwrap();
        assert_eq!(request.version(), Version::HTTP_11);
        assert_eq!(request.headers()[HOST], "example.com:8443");
        assert_eq!(request.headers()["connection"], "Upgrade");
        assert_eq!(request.headers()["upgrade"], "websocket");
        assert_eq!(request.body(), &"payload");
    }

    #[test]
    fn origin_form_retains_host_and_http10_version() {
        let mut request = Request::builder()
            .uri("/path")
            .version(Version::HTTP_10)
            .header(HOST, "example.com")
            .body(())
            .unwrap();
        prepare_http1_request(&mut request).unwrap();
        assert_eq!(request.version(), Version::HTTP_10);
        assert_eq!(request.headers()[HOST], "example.com");
        request.headers_mut().remove(HOST);
        assert!(prepare_http1_request(&mut request).is_err());
    }
}
