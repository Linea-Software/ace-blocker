//! Integration tests for narrow interception: opaque (non-intercepted) TCP
//! tunnels must relay bytes untouched, and the intercepted path must keep
//! working when a decider opts into it.

mod common;

use bytes::Bytes;
use common::*;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Read exactly `buf.len()` bytes from `stream`, byte-by-byte (kept simple and
/// deterministic for the small payloads used here).
async fn read_exact(stream: &mut tokio::net::TcpStream, buf: &mut [u8]) {
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
async fn raw_connect(
    proxy_addr: std::net::SocketAddr,
    host: &str,
    port: u16,
) -> tokio::net::TcpStream {
    let mut tcp = tokio::net::TcpStream::connect(proxy_addr).await.unwrap();
    tcp.set_nodelay(true).ok();

    let connect_req = format!("CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\n\r\n");
    tcp.write_all(connect_req.as_bytes()).await.unwrap();

    // Read the response head until the double CRLF.
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
    let response_head = String::from_utf8_lossy(&buf[..total]);
    assert!(
        response_head.starts_with("HTTP/1.1 200"),
        "CONNECT must return 200, got: {response_head}"
    );

    tcp
}

struct BlockConnect;

#[derive(Default)]
struct WaitingPolicy {
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
}

#[async_trait::async_trait]
impl RequestInspector for WaitingPolicy {
    async fn inspect_request(
        &self,
        _: &mitm_proxy::ConnMeta,
        _: &mut mitm_proxy::BufferedRequest,
    ) -> RequestAction {
        RequestAction::Continue
    }

    async fn should_terminate_tunnel(&self, _: &http::uri::Authority, _: bool) -> bool {
        self.entered.notify_one();
        self.release.notified().await;
        true
    }
}

#[tokio::test]
async fn opaque_tunnel_forwards_while_policy_is_waiting_then_closes() {
    init_tracing();
    let origin = spawn_raw_tcp_echo(&[]).await;
    let ca_dir = TempDir::new().unwrap();
    let tcp_addr = pick_tcp_addr().await;
    let policy = Arc::new(WaitingPolicy::default());
    let inspectors = Inspectors {
        request: policy.clone(),
        ..Inspectors::default()
    };
    let (_, _handle) = start_proxy_on_with_decider(
        tcp_addr,
        None,
        ca_dir.path(),
        inspectors,
        Arc::new(TunnelAll),
    )
    .await;
    let mut tunnel = raw_connect(tcp_addr, "127.0.0.1", origin.port()).await;
    tokio::time::timeout(std::time::Duration::from_secs(3), policy.entered.notified())
        .await
        .expect("policy hook must run");
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        tunnel.write_all(b"still flowing").await.unwrap();
        let mut echoed = [0; 13];
        tunnel.read_exact(&mut echoed).await.unwrap();
        assert_eq!(&echoed, b"still flowing");
    })
    .await
    .expect("waiting for policy must not stall traffic");
    policy.release.notify_one();
    let mut byte = [0];
    let result = tokio::time::timeout(std::time::Duration::from_secs(2), tunnel.read(&mut byte))
        .await
        .expect("policy rejection must close the tunnel");
    assert!(matches!(result, Ok(0) | Err(_)));
}

#[async_trait::async_trait]
impl RequestInspector for BlockConnect {
    async fn inspect_connect(
        &self,
        _meta: &mitm_proxy::ConnMeta,
        _authority: &http::uri::Authority,
    ) -> RequestAction {
        RequestAction::Respond(
            http::Response::builder()
                .status(http::StatusCode::FORBIDDEN)
                .body(Bytes::new())
                .unwrap(),
        )
    }

    async fn inspect_request(
        &self,
        _meta: &mitm_proxy::ConnMeta,
        _req: &mut mitm_proxy::BufferedRequest,
    ) -> RequestAction {
        RequestAction::Continue
    }
}

#[tokio::test]
async fn opaque_connect_can_be_blocked_before_tunnel_establishment() {
    init_tracing();

    let ca_dir = TempDir::new().unwrap();
    let tcp_addr = pick_tcp_addr().await;
    let mut inspectors = Inspectors::default();
    inspectors.request = Arc::new(BlockConnect);
    let (_ca_pem, _handle) = start_proxy_on_with_decider(
        tcp_addr,
        None,
        ca_dir.path(),
        inspectors,
        Arc::new(TunnelAll),
    )
    .await;

    let mut tcp = tokio::net::TcpStream::connect(tcp_addr).await.unwrap();
    tcp.write_all(b"CONNECT blocked.test:443 HTTP/1.1\r\nHost: blocked.test:443\r\n\r\n")
        .await
        .unwrap();

    let mut response = Vec::new();
    let mut byte = [0u8; 1];
    while response.len() < 4096 {
        let n = tcp.read(&mut byte).await.unwrap();
        if n == 0 {
            break;
        }
        response.push(byte[0]);
        if response.ends_with(b"\r\n\r\n") {
            break;
        }
    }

    let head = String::from_utf8_lossy(&response);
    assert!(
        head.starts_with("HTTP/1.1 403"),
        "CONNECT inspector should reject before opaque tunneling, got: {head}"
    );
}
// ---------------------------------------------------------------------------
// Invariant: byte-for-byte passthrough on the non-intercepted path
// ---------------------------------------------------------------------------

#[tokio::test]
async fn opaque_tunnel_relays_bytes_unchanged_both_directions() {
    init_tracing();

    // Raw byte origin: pushes a banner, then echoes everything back verbatim.
    let banner: Vec<u8> = (0..257u32).map(|i| (i % 251) as u8).collect();
    let origin = spawn_raw_tcp_echo(&banner).await;

    let ca_dir = TempDir::new().unwrap();
    let tcp_addr = pick_tcp_addr().await;

    let inspectors = Inspectors::default();
    let (_ca_pem, _handle) = start_proxy_on_with_decider(
        tcp_addr,
        None,
        ca_dir.path(),
        inspectors,
        Arc::new(TunnelAll),
    )
    .await;

    let mut tunnel = raw_connect(tcp_addr, "127.0.0.1", origin.port()).await;

    // Upstream -> client: the banner must arrive untouched (no TLS in the way).
    let mut got_banner = vec![0u8; banner.len()];
    read_exact(&mut tunnel, &mut got_banner).await;
    assert_eq!(
        got_banner, banner,
        "upstream -> client bytes must be unchanged"
    );

    // Client -> upstream -> client: echo round-trip must be byte-for-byte.
    let payload: Vec<u8> = (0..8192u32).map(|i| (i % 251) as u8).collect();
    tunnel.write_all(&payload).await.unwrap();
    let mut echoed = vec![0u8; payload.len()];
    read_exact(&mut tunnel, &mut echoed).await;
    assert_eq!(
        echoed, payload,
        "client -> upstream -> client bytes must be unchanged"
    );
}

/// A CONNECT URI encloses an IPv6 literal in brackets; the origin socket
/// must receive the bare address, not the bracketed HTTP authority host.
#[tokio::test]
async fn opaque_connect_to_bracketed_ipv6_literal_relays_bytes() {
    init_tracing();

    let origin = tokio::net::TcpListener::bind("[::1]:0").await.unwrap();
    let origin_port = origin.local_addr().unwrap().port();
    let upstream = tokio::spawn(async move {
        let (mut stream, _) = origin.accept().await.unwrap();
        stream.write_all(b"ipv6-ready").await.unwrap();
        let mut request = [0u8; 4];
        stream.read_exact(&mut request).await.unwrap();
        assert_eq!(&request, b"ping");
        stream.write_all(b"pong").await.unwrap();
    });

    let ca_dir = TempDir::new().unwrap();
    let proxy_addr = pick_tcp_addr().await;
    let (_, _proxy) = start_proxy_on_with_decider(
        proxy_addr,
        None,
        ca_dir.path(),
        Inspectors::default(),
        Arc::new(TunnelAll),
    )
    .await;

    let mut tunnel = raw_connect(proxy_addr, "[::1]", origin_port).await;
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        let mut banner = [0u8; 10];
        tunnel.read_exact(&mut banner).await.unwrap();
        assert_eq!(&banner, b"ipv6-ready");
        tunnel.write_all(b"ping").await.unwrap();
        let mut response = [0u8; 4];
        tunnel.read_exact(&mut response).await.unwrap();
        assert_eq!(&response, b"pong");
    })
    .await
    .expect("IPv6 CONNECT must establish and relay an opaque connection");
    upstream.await.unwrap();
}

// ---------------------------------------------------------------------------
// Invariant: no leaf certificate is minted for a tunneled host
// ---------------------------------------------------------------------------
//
// The certificate cache lives inside `DynamicCertResolver`, which is created
// in `MitmProxy::run` and is not reachable through the public API, so this
// test lives in the crate (`src/http.rs`) where it can hold the resolver and
// assert on its cache directly.

// ---------------------------------------------------------------------------
// Regression guard: the intercepted path still works when opted in
// ---------------------------------------------------------------------------

#[tokio::test]
async fn intercepted_path_still_works_with_decider() {
    init_tracing();

    let (origin_addr, _cert) = spawn_https_origin().await;
    let ca_dir = TempDir::new().unwrap();
    let tcp_addr = pick_tcp_addr().await;

    let inspectors = Inspectors::default();
    let (ca_pem, _handle) = start_proxy_on_with_decider(
        tcp_addr,
        None,
        ca_dir.path(),
        inspectors,
        Arc::new(InterceptAll),
    )
    .await;

    // Same flow as the classic `http1_connect_mitm` test, but with an
    // explicit intercept-all decider: TLS must be terminated and the request
    // must round-trip through the MITM pipeline.
    let (status, body) = proxy_connect_get(
        tcp_addr,
        &ca_pem,
        "localhost",
        origin_addr.port(),
        "/echo?x=1",
    )
    .await;

    assert_eq!(status, http::StatusCode::OK);
    assert!(body.contains("\"method\":\"GET\""));
    assert!(body.contains("/echo?x=1"));
}
