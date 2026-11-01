//! HTTP/1.x and HTTP/2 forward proxy with TLS interception.
//!
//! The listener speaks HTTP/1.1 to the browser (the classic forward-proxy
//! protocol). Two request shapes arrive:
//!
//! * **Plaintext** (`GET http://host/path`) — forwarded directly to the origin
//!   over cleartext HTTP/1.x.
//! * **`CONNECT host:port`** — we answer `200`, then ask the
//!   [`InterceptDecider`](crate::InterceptDecider) whether to terminate TLS:
//!   approved tunnels are MITM'd with a freshly-minted per-host certificate
//!   and the decrypted inner connection is served (HTTP/1.x *or* HTTP/2,
//!   chosen by ALPN); rejected tunnels are relayed to the origin byte-for-byte
//!   with no TLS setup at all, so no certificate is minted and nothing is
//!   decrypted.

use std::convert::Infallible;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use http::header::{CONTENT_LENGTH, TRANSFER_ENCODING};
use http::uri::Authority;
use http::{Method, Request, Response, StatusCode};
use http_body_util::{BodyExt, Full, Limited, combinators::UnsyncBoxBody};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;
use tracing::{debug, info, warn};

use crate::cert_resolver::TunnelCertResolver;
use crate::error::{ProxyError, Result};
use crate::inspect::{ConnMeta, Protocol, RequestAction, RequestBodyMode, ResponseBodyMode};
use crate::state::SharedState;
use crate::util::{
    absolute_uri, authority_display, ensure_host_header, is_ip_literal, snapshot_request_head,
    strip_hop_by_hop, strip_hop_by_hop_for_upgrade, strip_port,
};
use rustls::pki_types::ServerName;

type BoxError = Box<dyn Error + Send + Sync>;
type ProxyBody = UnsyncBoxBody<Bytes, BoxError>;

// Browser HTTP/2 requests can legitimately exceed Hyper's 16 KiB default once
// cookies and client-hint headers accumulate. Keep the limit bounded while
// leaving enough headroom for header-heavy applications such as ChatGPT.
const CLIENT_HTTP2_MAX_HEADER_LIST_SIZE: u32 = 64 * 1024;
const MAX_BUFFERED_REQUEST_BODY: usize = 16 * 1024 * 1024;
const TUNNEL_POLICY_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Keep the policy future independently polled from connection I/O. Waiting
/// for the engine's policy lock must not pause forwarding on an allowed flow.
async fn wait_for_tunnel_termination(
    authority: &Authority,
    is_intercepted: bool,
    inspector: &dyn crate::inspect::RequestInspector,
) {
    let mut interval = tokio::time::interval(TUNNEL_POLICY_POLL_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    interval.tick().await;
    loop {
        interval.tick().await;
        if inspector
            .should_terminate_tunnel(authority, is_intercepted)
            .await
        {
            debug!(%authority, "closing tunnel for policy transition");
            return;
        }
    }
}

async fn wait_for_upgraded_termination(
    meta: &ConnMeta,
    uri: &http::Uri,
    inspector: &dyn crate::inspect::RequestInspector,
) {
    let mut interval = tokio::time::interval(TUNNEL_POLICY_POLL_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    interval.tick().await;
    loop {
        interval.tick().await;
        if inspector.should_terminate_upgraded(meta, uri).await {
            return;
        }
    }
}

/// Bind and serve the HTTP/1.x + HTTP/2 forward proxy until the process exits.
pub async fn serve(state: SharedState) -> Result<()> {
    let addr = state.config.listen_addr;
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|source| ProxyError::Bind { addr, source })?;
    info!("HTTP/1.x + HTTP/2 proxy listening on http://{addr}");

    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(pair) => pair,
            Err(error) => {
                warn!("accept failed: {error}");
                continue;
            }
        };
        debug!(client = %peer, "accepted proxy client TCP connection");
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(error) = serve_connection(stream, peer, state).await {
                debug!("client connection {peer} ended: {error:#}");
            }
        });
    }
}

/// Serve one browser<->proxy TCP connection (HTTP/1.1, with CONNECT upgrades).
async fn serve_connection(stream: TcpStream, peer: SocketAddr, state: SharedState) -> Result<()> {
    stream.set_nodelay(true).ok();
    let io = TokioIo::new(stream);

    let service = service_fn(move |req: Request<Incoming>| {
        let state = state.clone();
        async move { Ok::<_, Infallible>(outer_handler(req, peer, state).await) }
    });

    hyper::server::conn::http1::Builder::new()
        .serve_connection(io, service)
        .with_upgrades()
        .await
        .map_err(|error| eyre::eyre!("serving client connection: {error}"))
}

/// Dispatch a request from the browser: CONNECT tunnels vs. plaintext forwards.
async fn outer_handler(
    req: Request<Incoming>,
    peer: SocketAddr,
    state: SharedState,
) -> Response<ProxyBody> {
    if req.method() == Method::CONNECT {
        // e.g. `CONNECT example.com:443`.
        let Some(authority) = req.uri().authority().cloned() else {
            return simple(StatusCode::BAD_REQUEST, "CONNECT requires an authority");
        };
        debug!(client = %peer, authority = %authority, "received CONNECT request");

        let connect_meta = ConnMeta {
            client_addr: peer,
            protocol: Protocol::Http1,
            is_tls: false,
            authority: authority.to_string(),
        };
        match state
            .inspectors
            .request
            .inspect_connect(&connect_meta, &authority)
            .await
        {
            RequestAction::Continue => {}
            RequestAction::Respond(resp) => return into_full(resp),
        }

        tokio::spawn(async move {
            match hyper::upgrade::on(req).await {
                Ok(upgraded) => {
                    let io = TokioIo::new(upgraded);
                    if let Err(error) = mitm_tunnel(io, authority, peer, state).await {
                        debug!("MITM tunnel error: {error:#}");
                    }
                }
                Err(error) => debug!("CONNECT upgrade failed: {error}"),
            }
        });

        // `200` tells the browser the tunnel is established; body is empty.
        Response::new(full_body(Bytes::new()))
    } else {
        forward_plaintext(req, peer, state).await
    }
}

/// Handle a cleartext, absolute-form request (`GET http://host/path`).
async fn forward_plaintext(
    req: Request<Incoming>,
    peer: SocketAddr,
    state: SharedState,
) -> Response<ProxyBody> {
    let uri = req.uri().clone();
    let Some(host) = uri.host().map(str::to_owned) else {
        return simple(
            StatusCode::BAD_REQUEST,
            "expected absolute-form request URI",
        );
    };
    let port = uri.port_u16().unwrap_or(80);

    let meta = ConnMeta {
        client_addr: peer,
        protocol: Protocol::Http1,
        is_tls: false,
        authority: authority_display(&host, port, false),
    };

    relay(req, meta, false, host, port, state).await
}

/// Terminate TLS on an accepted CONNECT tunnel and serve the decrypted stream.
async fn mitm_tunnel<I>(
    io: I,
    authority: Authority,
    peer: SocketAddr,
    state: SharedState,
) -> Result<()>
where
    I: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let port = authority.port_u16().unwrap_or(443);
    if port == 443
        && is_ip_literal(authority.host())
        && state.intercept_decider.recover_hostname_for_ip_connect()
    {
        // ace-tun may not have seen the DNS query (browser cache or DoH).
        // Recover the real TLS target before checking the hostname policy.
        // A missing/invalid SNI must never turn an unknown host into an
        // uninspected, allowed IP tunnel.
        let (io, server_name) = match crate::client_hello::recover_sni(io).await {
            Ok(result) => result,
            Err(error) => {
                warn!(%authority, %error, "rejecting IP CONNECT without usable TLS hostname");
                return Ok(());
            }
        };
        if is_ip_literal(&server_name) || ServerName::try_from(server_name.clone()).is_err() {
            warn!(%authority, %server_name, "rejecting IP CONNECT with invalid TLS hostname");
            return Ok(());
        }
        let policy_authority: Authority = format!("{server_name}:{port}")
            .parse()
            .map_err(|error| eyre::eyre!("invalid recovered TLS authority: {error}"))?;
        let meta = ConnMeta {
            client_addr: peer,
            protocol: Protocol::Http1,
            is_tls: false,
            authority: policy_authority.to_string(),
        };
        if let RequestAction::Respond(_) = state
            .inspectors
            .request
            .inspect_connect(&meta, &policy_authority)
            .await
        {
            debug!(%policy_authority, "blocked TLS SNI recovered from IP CONNECT");
            return Ok(());
        }
        return serve_tls_tunnel(io, authority, policy_authority, peer, state).await;
    }
    serve_tls_tunnel(io, authority.clone(), authority, peer, state).await
}

async fn serve_tls_tunnel<I>(
    io: I,
    connect_authority: Authority,
    policy_authority: Authority,
    peer: SocketAddr,
    state: SharedState,
) -> Result<()>
where
    I: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let host = policy_authority.host().to_string();
    let port = policy_authority.port_u16().unwrap_or(443);

    // The effective TLS name, not a DNS-cache-miss IP literal, determines the
    // interception policy; opaque traffic still connects to the original IP.
    let should_intercept = state
        .intercept_decider
        .should_intercept(&policy_authority, Some(&host));
    debug!(
        client = %peer,
        authority = %policy_authority,
        should_intercept,
        "CONNECT interception decision"
    );
    if !should_intercept {
        return opaque_tunnel(
            io,
            &connect_authority,
            &policy_authority,
            connect_authority.host(),
            port,
            state,
        )
        .await;
    }

    if state.config.preserve_client_cert_auth
        && state
            .upstream
            .requires_client_certificate(&host, port)
            .await
    {
        info!(
            client = %peer,
            authority = %policy_authority,
            "origin requires client certificate; keeping CONNECT opaque"
        );
        return opaque_tunnel(
            io,
            &connect_authority,
            &policy_authority,
            connect_authority.host(),
            port,
            state,
        )
        .await;
    }

    let mut server_tls = (*state.server_tls).clone();
    server_tls.cert_resolver = Arc::new(TunnelCertResolver::new(
        state.cert_resolver.clone(),
        host.clone(),
    ));
    let acceptor = TlsAcceptor::from(Arc::new(server_tls));
    debug!(client = %peer, authority = %policy_authority, "starting client TLS handshake");
    let tls = acceptor
        .accept(io)
        .await
        .map_err(|error| ProxyError::ClientHandshake(error.to_string()))?;

    // ALPN tells us which HTTP version the client negotiated inside the tunnel.
    let protocol = match tls.get_ref().1.alpn_protocol() {
        Some(b"h2") => Protocol::Http2,
        _ => Protocol::Http1,
    };
    debug!(
        client = %peer,
        authority = %policy_authority,
        protocol = %protocol,
        "client TLS handshake completed"
    );

    let tunnel_inspector = state.inspectors.request.clone();
    let host = Arc::new(host);
    let hyper_io = TokioIo::new(tls);

    let service = service_fn(move |req: Request<Incoming>| {
        let state = state.clone();
        let host = host.clone();
        async move {
            let meta = ConnMeta {
                client_addr: peer,
                protocol,
                is_tls: true,
                authority: authority_display(&host, port, true),
            };
            Ok::<_, Infallible>(relay(req, meta, true, host.to_string(), port, state).await)
        }
    });

    // `auto` serves either HTTP/1.x or HTTP/2 based on the negotiated protocol.
    let mut builder = auto::Builder::new(TokioExecutor::new());
    builder
        .http2()
        .max_header_list_size(CLIENT_HTTP2_MAX_HEADER_LIST_SIZE)
        .enable_connect_protocol();

    let connection = builder.serve_connection_with_upgrades(hyper_io, service);
    tokio::select! {
        result = connection => {
            result.map_err(|error| eyre::eyre!("serving decrypted connection: {error}"))
        }
        _ = wait_for_tunnel_termination(&policy_authority, true, tunnel_inspector.as_ref()) => Ok(()),
    }
}

/// Relay a CONNECT tunnel to the origin without touching TLS.
///
/// Both directions are copied verbatim: no certificate is minted and no bytes
/// are decrypted. The policy hook can close the tunnel when a schedule/rule
/// transition means the next connection must instead be blocked or inspected.
async fn opaque_tunnel<I>(
    io: I,
    authority: &Authority,
    policy_authority: &Authority,
    host: &str,
    port: u16,
    state: SharedState,
) -> Result<()>
where
    I: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let mut upstream =
        TcpStream::connect((host, port))
            .await
            .map_err(|source| ProxyError::Upstream {
                authority: authority.to_string(),
                source: eyre::eyre!(source),
            })?;
    upstream.set_nodelay(true).ok();

    let mut client = io;
    let relay = tokio::io::copy_bidirectional(&mut client, &mut upstream);
    tokio::select! {
        result = relay => {
            result.map_err(|error| eyre::eyre!("opaque tunnel relay to {authority} failed: {error}"))?;
            Ok(())
        }
        _ = wait_for_tunnel_termination(policy_authority, false, state.inspectors.request.as_ref()) => Ok(()),
    }
}

/// Choose the upstream host for a request. When the CONNECT target is an IP
/// literal (ace-tun falls back to the snooped IP when its DNS snoop misses),
/// the real hostname is recovered from the request itself — the URI authority
/// (`:authority` for HTTP/2, also absolute-form HTTP/1.1) first, then the
/// `Host` header with any `:port` suffix stripped — so the upstream SNI
/// carries a domain instead of an IP (CDNs reject SNI literals with a
/// handshake failure). Returns `None` when the CONNECT target is already a
/// domain or the request offers no usable hostname; the caller then keeps the
/// CONNECT-derived host, so IP-literal requests that worked before keep
/// working.
fn upstream_host_for<B>(req: &Request<B>, connect_host: &str) -> Option<String> {
    if !is_ip_literal(connect_host) {
        return None;
    }
    let candidate = req
        .uri()
        .authority()
        .map(|authority| authority.host())
        .or_else(|| {
            req.headers()
                .get(http::header::HOST)
                .and_then(|value| value.to_str().ok())
                .map(strip_port)
        })?;
    if is_ip_literal(candidate) || ServerName::try_from(candidate.to_owned()).is_err() {
        return None;
    }
    Some(candidate.to_owned())
}

/// The shared request pipeline: buffer, inspect, forward upstream, inspect the
/// response, return it. `secure` selects the upstream scheme.
async fn relay(
    req: Request<Incoming>,
    meta: ConnMeta,
    secure: bool,
    host: String,
    port: u16,
    state: SharedState,
) -> Response<ProxyBody> {
    // An IP-literal CONNECT target must not be used as the upstream SNI — CDNs
    // reject SNI literals with a handshake failure. Prefer the hostname the
    // request itself carries (`:authority` for HTTP/2, `Host` for HTTP/1.x).
    let upstream_host = upstream_host_for(&req, &host).unwrap_or(host);
    debug!(
        client = %meta.client_addr,
        protocol = %meta.protocol,
        tls = meta.is_tls,
        method = %req.method(),
        version = ?req.version(),
        path = %req.uri().path(),
        authority = %meta.authority,
        upstream_host = %upstream_host,
        upstream_port = port,
        "relaying request"
    );

    // Rebuild an absolute target URI (origin-form requests inside a tunnel lack
    // scheme/authority; plaintext ones already have them).
    let abs_uri = match absolute_uri(req.uri(), &upstream_host, port, secure) {
        Ok(u) => u,
        Err(error) => {
            return simple(
                StatusCode::BAD_REQUEST,
                &format!("bad request URI: {error}"),
            );
        }
    };

    // Hyper exposes HTTP/2 CONNECT DATA through its upgrade API, not Incoming.
    if req.method() == Method::CONNECT
        && req.version() == http::Version::HTTP_2
        && req.extensions().get::<hyper::ext::Protocol>().is_some()
    {
        return relay_extended_connect(req, meta, secure, upstream_host, port, state, abs_uri)
            .await;
    }

    let mut req = req;
    let client_upgrade = if is_http1_upgrade_request(&req) {
        Some(hyper::upgrade::on(&mut req))
    } else {
        None
    };

    let (mut parts, body) = req.into_parts();
    parts.uri = abs_uri;
    let mut request_head = Request::from_parts(parts, Bytes::new());

    match state
        .inspectors
        .request
        .inspect_request_head(&meta, &mut request_head)
        .await
    {
        RequestAction::Continue => {}
        RequestAction::Respond(resp) => return into_full(resp),
    }

    let body_mode = state
        .inspectors
        .request
        .request_body_mode(&meta, &request_head);
    let (req_head, _) = request_head.into_parts();
    let is_upgrade = client_upgrade.is_some();

    let (head_snapshot, upstream_resp) = match body_mode {
        RequestBodyMode::Stream => {
            let head_snapshot = snapshot_request_head(&req_head);
            let mut ureq = Request::from_parts(req_head, body);
            if is_upgrade {
                strip_hop_by_hop_for_upgrade(ureq.headers_mut());
            } else {
                strip_hop_by_hop(ureq.headers_mut());
            }
            ensure_host_header(&mut ureq, &upstream_host, port, secure);

            let upstream_resp = match state
                .upstream
                .send(secure, &upstream_host, port, ureq)
                .await
            {
                Ok(resp) => resp,
                Err(error) => {
                    warn!("upstream {upstream_host}:{port} failed: {error:#}");
                    return simple(StatusCode::BAD_GATEWAY, "upstream request failed");
                }
            };
            (head_snapshot, upstream_resp)
        }
        RequestBodyMode::Buffer => {
            let body_bytes = match Limited::new(body, MAX_BUFFERED_REQUEST_BODY)
                .collect()
                .await
            {
                Ok(body) => body.to_bytes(),
                Err(error) => {
                    if error
                        .downcast_ref::<http_body_util::LengthLimitError>()
                        .is_some()
                    {
                        return simple(StatusCode::PAYLOAD_TOO_LARGE, "request body too large");
                    }
                    return simple(
                        StatusCode::BAD_REQUEST,
                        &format!("reading request body: {error}"),
                    );
                }
            };
            let mut buffered = Request::from_parts(req_head, body_bytes);
            match state
                .inspectors
                .request
                .inspect_request(&meta, &mut buffered)
                .await
            {
                RequestAction::Continue => {}
                RequestAction::Respond(resp) => return into_full(resp),
            }

            let (req_head, req_body) = buffered.into_parts();
            let head_snapshot = snapshot_request_head(&req_head);
            let mut ureq = Request::from_parts(req_head, Full::new(req_body));
            if is_upgrade {
                strip_hop_by_hop_for_upgrade(ureq.headers_mut());
            } else {
                strip_hop_by_hop(ureq.headers_mut());
            }
            ensure_host_header(&mut ureq, &upstream_host, port, secure);

            let upstream_resp = match state
                .upstream
                .send(secure, &upstream_host, port, ureq)
                .await
            {
                Ok(resp) => resp,
                Err(error) => {
                    warn!("upstream {upstream_host}:{port} failed: {error:#}");
                    return simple(StatusCode::BAD_GATEWAY, "upstream request failed");
                }
            };
            (head_snapshot, upstream_resp)
        }
    };

    finish_upstream_response(upstream_resp, meta, head_snapshot, state, client_upgrade).await
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn is_http1_upgrade_request<B>(req: &Request<B>) -> bool {
    if req.version() != http::Version::HTTP_11 || !req.headers().contains_key(http::header::UPGRADE)
    {
        return false;
    }

    req.headers()
        .get_all(http::header::CONNECTION)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .any(|token| token.trim().eq_ignore_ascii_case("upgrade"))
}

async fn relay_extended_connect(
    mut req: Request<Incoming>,
    meta: ConnMeta,
    secure: bool,
    upstream_host: String,
    port: u16,
    state: SharedState,
    abs_uri: http::Uri,
) -> Response<ProxyBody> {
    let client_upgrade = hyper::upgrade::on(&mut req);
    let (mut parts, _) = req.into_parts();
    let extended_protocol = parts.extensions.get::<hyper::ext::Protocol>().cloned();
    parts.uri = abs_uri;

    // Inspect the handshake before accepting an upgraded stream.
    let mut buffered_head = Request::from_parts(snapshot_request_head(&parts), Bytes::new());
    match state
        .inspectors
        .request
        .inspect_request(&meta, &mut buffered_head)
        .await
    {
        RequestAction::Continue => {}
        RequestAction::Respond(resp) => return into_full(resp),
    }

    let (mut req_head, _) = buffered_head.into_parts();
    if let Some(protocol) = extended_protocol {
        req_head.extensions.insert(protocol);
    }
    let head_snapshot = snapshot_request_head(&req_head);

    let mut ureq = Request::from_parts(req_head, Full::new(Bytes::new()));
    strip_hop_by_hop(ureq.headers_mut());
    ensure_host_header(&mut ureq, &upstream_host, port, secure);

    let upstream_resp = match state
        .upstream
        .send(secure, &upstream_host, port, ureq)
        .await
    {
        Ok(resp) => resp,
        Err(error) => {
            warn!("extended CONNECT upstream {upstream_host}:{port} failed: {error:#}");
            return simple(StatusCode::BAD_GATEWAY, "upstream extended CONNECT failed");
        }
    };

    finish_upstream_response(
        upstream_resp,
        meta,
        head_snapshot,
        state,
        Some(client_upgrade),
    )
    .await
}

async fn finish_upstream_response(
    mut upstream_resp: Response<Incoming>,
    meta: ConnMeta,
    head_snapshot: http::request::Parts,
    state: SharedState,
    client_upgrade: Option<hyper::upgrade::OnUpgrade>,
) -> Response<ProxyBody> {
    let extended_connect =
        head_snapshot.method == Method::CONNECT && head_snapshot.version == http::Version::HTTP_2;
    if let Some(client_upgrade) = client_upgrade
        && (upstream_resp.status() == StatusCode::SWITCHING_PROTOCOLS
            || (extended_connect && upstream_resp.status().is_success()))
    {
        let upstream_upgrade = hyper::upgrade::on(&mut upstream_resp);
        let (parts, _) = upstream_resp.into_parts();
        let mut response_head = Response::from_parts(parts, Bytes::new());
        state
            .inspectors
            .response
            .inspect_response_head(&meta, &head_snapshot, &mut response_head)
            .await;
        let (mut parts, _) = response_head.into_parts();
        if extended_connect {
            strip_hop_by_hop(&mut parts.headers);
        } else {
            strip_hop_by_hop_for_upgrade(&mut parts.headers);
        }

        tokio::spawn(async move {
            match tokio::try_join!(client_upgrade, upstream_upgrade) {
                Ok((client, upstream)) => {
                    let mut client = TokioIo::new(client);
                    let mut upstream = TokioIo::new(upstream);
                    let relay = tokio::io::copy_bidirectional(&mut client, &mut upstream);
                    let result = tokio::select! {
                        result = relay => result,
                        _ = wait_for_upgraded_termination(&meta, &head_snapshot.uri, state.inspectors.request.as_ref()) => return,
                    };
                    if let Err(error) = result {
                        debug!("upgraded protocol relay ended: {error}");
                    }
                }
                Err(error) => debug!("protocol upgrade failed: {error}"),
            }
        });

        return Response::from_parts(parts, full_body(Bytes::new()));
    }

    let (parts, body) = upstream_resp.into_parts();
    let mut response_head = Response::from_parts(parts, Bytes::new());
    state
        .inspectors
        .response
        .inspect_response_head(&meta, &head_snapshot, &mut response_head)
        .await;
    let body_mode =
        state
            .inspectors
            .response
            .response_body_mode(&meta, &head_snapshot, &response_head);
    debug!(
        client = %meta.client_addr,
        protocol = %meta.protocol,
        method = %head_snapshot.method,
        path = %head_snapshot.uri.path(),
        status = %response_head.status(),
        body_mode = ?body_mode,
        "upstream response ready for client"
    );
    let (mut parts, _) = response_head.into_parts();

    match body_mode {
        ResponseBodyMode::Stream => {
            strip_hop_by_hop(&mut parts.headers);
            Response::from_parts(parts, streaming_body(body))
        }
        ResponseBodyMode::Buffer => {
            let body = match body.collect().await {
                Ok(body) => body.to_bytes(),
                Err(error) => {
                    warn!("reading upstream response body failed: {error}");
                    return simple(StatusCode::BAD_GATEWAY, "reading upstream response failed");
                }
            };
            let mut buffered = Response::from_parts(parts, body);
            state
                .inspectors
                .response
                .inspect_response(&meta, &head_snapshot, &mut buffered)
                .await;
            into_full(buffered)
        }
    }
}

fn full_body(bytes: Bytes) -> ProxyBody {
    Full::new(bytes)
        .map_err(|never: Infallible| -> BoxError { match never {} })
        .boxed_unsync()
}

fn streaming_body(body: Incoming) -> ProxyBody {
    body.map_err(|error| Box::new(error) as BoxError)
        .boxed_unsync()
}

/// Convert a buffered response into one hyper can serve, fixing framing headers.
fn into_full(resp: Response<Bytes>) -> Response<ProxyBody> {
    let (mut parts, body) = resp.into_parts();
    strip_hop_by_hop(&mut parts.headers);
    // Body is fully buffered: let hyper set an accurate Content-Length.
    parts.headers.remove(TRANSFER_ENCODING);
    parts.headers.remove(CONTENT_LENGTH);
    Response::from_parts(parts, full_body(body))
}

/// A tiny plaintext response (used for proxy-level errors).
fn simple(status: StatusCode, msg: &str) -> Response<ProxyBody> {
    Response::builder()
        .status(status)
        .header(http::header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(full_body(Bytes::from(msg.to_owned())))
        .expect("static response is always valid")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ca::CertAuthority;
    use crate::cert_resolver::DynamicCertResolver;
    use crate::inspect::Inspectors;
    use crate::state::ProxyState;
    use crate::upstream::Upstream;
    use crate::{NoInterceptDecider, ProxyConfig};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Read exactly `buf.len()` bytes from `stream`.
    async fn read_exact(stream: &mut TcpStream, buf: &mut [u8]) {
        let mut got = 0;
        while got < buf.len() {
            let n = stream.read(&mut buf[got..]).await.unwrap();
            assert!(
                n > 0,
                "connection closed after {} of {} bytes",
                got,
                buf.len()
            );
            got += n;
        }
    }

    /// Raw CONNECT through the proxy, asserting a `200` response head.
    async fn raw_connect(proxy_addr: SocketAddr, host: &str, port: u16) -> TcpStream {
        let mut tcp = TcpStream::connect(proxy_addr).await.unwrap();
        tcp.set_nodelay(true).ok();

        let connect_req = format!("CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\n\r\n");
        tcp.write_all(connect_req.as_bytes()).await.unwrap();

        let mut buf = [0u8; 4096];
        let mut total = 0;
        let mut found_end = false;
        while total < buf.len() {
            let n = tcp.read(&mut buf[total..total + 1]).await.unwrap();
            if n == 0 {
                break;
            }
            total += n;
            if total >= 4 && &buf[total - 4..total] == b"\r\n\r\n" {
                found_end = true;
                break;
            }
        }
        assert!(found_end, "CONNECT response must end with double CRLF");
        let head = String::from_utf8_lossy(&buf[..total]);
        assert!(head.starts_with("HTTP/1.1 200"), "got: {head}");
        tcp
    }

    struct RecoverIpTlsName;

    impl crate::InterceptDecider for RecoverIpTlsName {
        fn should_intercept(&self, _authority: &Authority, _sni: Option<&str>) -> bool {
            false
        }

        fn recover_hostname_for_ip_connect(&self) -> bool {
            true
        }
    }

    #[derive(Default)]
    struct BlockCachedHost {
        observed: std::sync::Mutex<Vec<String>>,
    }

    #[async_trait::async_trait]
    impl crate::inspect::RequestInspector for BlockCachedHost {
        async fn inspect_connect(&self, _meta: &ConnMeta, authority: &Authority) -> RequestAction {
            self.observed
                .lock()
                .unwrap()
                .push(authority.host().to_owned());
            if authority.host() == "blocked.example" {
                RequestAction::Respond(Response::new(Bytes::new()))
            } else {
                RequestAction::Continue
            }
        }

        async fn inspect_request(
            &self,
            _meta: &ConnMeta,
            _request: &mut crate::inspect::BufferedRequest,
        ) -> RequestAction {
            RequestAction::Continue
        }
    }

    #[tokio::test]
    async fn cached_dns_ip_connect_checks_tls_sni_before_opening_tunnel() {
        crate::install_crypto_provider();
        let inspector = Arc::new(BlockCachedHost::default());
        let (cert_pem, key_pem) = CertAuthority::generate_root().unwrap();
        let ca = Arc::new(CertAuthority::from_pem(cert_pem, &key_pem).unwrap());
        let resolver = Arc::new(DynamicCertResolver::new(ca));
        let server_tls = crate::tls::server_config(resolver.clone(), crate::tls::TCP_ALPN);
        let client_tls = crate::tls::client_config(false, crate::tls::TCP_ALPN).unwrap();
        let temp = tempfile::TempDir::new().unwrap();
        let state = Arc::new(ProxyState {
            server_tls,
            cert_resolver: resolver.clone(),
            #[cfg(feature = "http3")]
            h3_tls: crate::tls::server_config(resolver.clone(), crate::tls::H3_ALPN),
            upstream: Upstream::new(client_tls),
            inspectors: Inspectors {
                request: inspector.clone(),
                ..Inspectors::default()
            },
            intercept_decider: Arc::new(RecoverIpTlsName),
            config: Arc::new(ProxyConfig::new(
                "127.0.0.1:0".parse().unwrap(),
                temp.path(),
            )),
        });

        let (mut browser, incoming) = tokio::io::duplex(16384);
        let server = tokio::spawn(mitm_tunnel(
            incoming,
            "127.0.0.1:443".parse().unwrap(),
            "127.0.0.1:1234".parse().unwrap(),
            state,
        ));
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(rustls::RootCertStore::empty())
            .with_no_client_auth();
        let mut browser_tls =
            rustls::ClientConnection::new(Arc::new(config), "blocked.example".try_into().unwrap())
                .unwrap();
        let mut hello = Vec::new();
        browser_tls.write_tls(&mut hello).unwrap();
        browser.write_all(&hello).await.unwrap();

        tokio::time::timeout(Duration::from_secs(3), server)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(
            inspector.observed.lock().unwrap().as_slice(),
            ["blocked.example"]
        );
        assert_eq!(
            resolver.cache_len(),
            0,
            "blocked SNI must not mint a certificate"
        );
    }

    /// Invariant: a tunnel the decider rejects must never mint a leaf
    /// certificate for the host.
    ///
    /// This lives in the crate (rather than `tests/`) because the certificate
    /// cache lives inside `DynamicCertResolver`, which `MitmProxy::run`
    /// constructs internally and does not expose. Here we build the same
    /// [`ProxyState`] `run` would build, but keep the resolver so we can
    /// assert on its cache after the relay completes.
    #[tokio::test]
    async fn tunneled_host_gets_no_certificate_minted() {
        crate::install_crypto_provider();

        // Raw byte origin: pushes a banner, echoes everything back, and
        // signals when the connection closes so the test knows the tunneled
        // relay has fully completed.
        let origin_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = origin_listener.local_addr().unwrap();
        let banner: Vec<u8> = (0..257u32).map(|i| (i % 251) as u8).collect();
        let (close_tx, close_rx) = tokio::sync::oneshot::channel::<()>();
        let banner_for_server = banner.clone();
        tokio::spawn(async move {
            let (mut stream, _) = origin_listener.accept().await.unwrap();
            let _ = stream.write_all(&banner_for_server).await;
            let mut buf = [0u8; 4096];
            loop {
                match stream.read(&mut buf).await {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if stream.write_all(&buf[..n]).await.is_err() {
                            break;
                        }
                    }
                }
            }
            let _ = close_tx.send(());
        });

        // Same state construction `MitmProxy::run` uses, with a resolver the
        // test holds on to.
        let (cert_pem, key_pem) = CertAuthority::generate_root().unwrap();
        let ca = Arc::new(CertAuthority::from_pem(cert_pem, &key_pem).unwrap());
        let resolver = Arc::new(DynamicCertResolver::new(ca));
        let server_tls = crate::tls::server_config(resolver.clone(), crate::tls::TCP_ALPN);
        let client_tls = crate::tls::client_config(false, crate::tls::TCP_ALPN).unwrap();

        let ca_dir = tempfile::TempDir::new().unwrap();
        let proxy_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_addr = proxy_listener.local_addr().unwrap();
        drop(proxy_listener);

        let state = Arc::new(ProxyState {
            server_tls,
            cert_resolver: resolver.clone(),
            #[cfg(feature = "http3")]
            h3_tls: crate::tls::server_config(resolver.clone(), crate::tls::H3_ALPN),
            upstream: Upstream::new(client_tls),
            inspectors: Inspectors::default(),
            intercept_decider: Arc::new(NoInterceptDecider),
            config: Arc::new(ProxyConfig::new(proxy_addr, ca_dir.path())),
        });
        let serve_handle = tokio::spawn(async move {
            let _ = serve(state).await;
        });

        // Let the listener bind before connecting.
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        // The decider (NoInterceptDecider) rejects everything: the tunnel must
        // be relayed raw, so verify bytes flow both ways untouched.
        let mut tunnel = raw_connect(proxy_addr, "127.0.0.1", origin.port()).await;
        let mut got_banner = vec![0u8; banner.len()];
        read_exact(&mut tunnel, &mut got_banner).await;
        assert_eq!(got_banner, banner);

        let payload: Vec<u8> = (0..4096u32).map(|i| (i % 251) as u8).collect();
        tunnel.write_all(&payload).await.unwrap();
        let mut echoed = vec![0u8; payload.len()];
        read_exact(&mut tunnel, &mut echoed).await;
        assert_eq!(echoed, payload);

        // Close the client side and wait for the relay to wind down.
        drop(tunnel);
        let _ = close_rx.await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // The core invariant: no leaf certificate was minted for the tunneled
        // host — in fact the cache must be entirely empty.
        assert!(
            !resolver.has_cached_key("127.0.0.1"),
            "tunneled host must not have a cached certificate"
        );
        assert_eq!(
            resolver.cache_len(),
            0,
            "no certificate may be minted on the opaque tunnel path"
        );

        serve_handle.abort();
    }
}
