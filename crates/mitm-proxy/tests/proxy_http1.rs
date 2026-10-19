//! Integration tests for HTTP/1.1 plaintext forwarding and CONNECT MITM.

mod common;

use bytes::Bytes;
use common::*;
use http::{Request, Response, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use std::convert::Infallible;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_rustls::TlsAcceptor;

async fn spawn_streaming_https_origin() -> (
    std::net::SocketAddr,
    SelfSignedCert,
    oneshot::Receiver<()>,
    oneshot::Sender<()>,
) {
    let cert = self_signed_cert();
    let mut tls_cfg = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert.cert_chain.clone(), cert.key_der())
        .unwrap();
    tls_cfg.alpn_protocols = vec![b"http/1.1".to_vec()];
    let acceptor = TlsAcceptor::from(Arc::new(tls_cfg));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (first_sent_tx, first_sent_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();

    tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.unwrap();
        let mut tls = acceptor.accept(tcp).await.unwrap();
        let mut request = Vec::new();
        let mut byte = [0u8; 1];
        while !request.ends_with(b"\r\n\r\n") {
            let n = tls.read(&mut byte).await.unwrap();
            if n == 0 {
                return;
            }
            request.push(byte[0]);
        }

        tls.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nfirst\r\n",
        )
        .await
        .unwrap();
        tls.flush().await.unwrap();
        let _ = first_sent_tx.send(());
        let _ = release_rx.await;
        tls.write_all(b"6\r\nsecond\r\n0\r\n\r\n").await.unwrap();
        tls.flush().await.unwrap();
    });

    (addr, cert, first_sent_rx, release_tx)
}

async fn upgrade_handler(mut req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    let is_upgrade = req
        .headers()
        .get(http::header::UPGRADE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.eq_ignore_ascii_case("ace-echo"));

    if !is_upgrade {
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Full::new(Bytes::new()))
            .unwrap());
    }

    let on_upgrade = hyper::upgrade::on(&mut req);
    tokio::spawn(async move {
        if let Ok(upgraded) = on_upgrade.await {
            let mut io = TokioIo::new(upgraded);
            let mut buffer = [0u8; 4096];
            loop {
                let n = match io.read(&mut buffer).await {
                    Ok(0) | Err(_) => break,
                    Ok(n) => n,
                };
                if io.write_all(&buffer[..n]).await.is_err() {
                    break;
                }
            }
        }
    });

    Ok(Response::builder()
        .status(StatusCode::SWITCHING_PROTOCOLS)
        .header(http::header::CONNECTION, "Upgrade")
        .header(http::header::UPGRADE, "ace-echo")
        .body(Full::new(Bytes::new()))
        .unwrap())
}

async fn spawn_upgrade_https_origin() -> (std::net::SocketAddr, SelfSignedCert) {
    let cert = self_signed_cert();
    let mut tls_cfg = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert.cert_chain.clone(), cert.key_der())
        .unwrap();
    tls_cfg.alpn_protocols = vec![b"http/1.1".to_vec()];
    let acceptor = TlsAcceptor::from(Arc::new(tls_cfg));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.unwrap();
        let tls = acceptor.accept(tcp).await.unwrap();
        let service = service_fn(upgrade_handler);
        let _ = hyper::server::conn::http1::Builder::new()
            .serve_connection(TokioIo::new(tls), service)
            .with_upgrades()
            .await;
    });

    (addr, cert)
}

#[tokio::test]
async fn http1_plaintext_forward() {
    init_tracing();

    let origin = spawn_http1_origin().await;
    let ca_dir = TempDir::new().unwrap();
    let tcp_addr = pick_tcp_addr().await;

    let inspectors = Inspectors::default();
    let (_ca_pem, _handle) = start_proxy_on(tcp_addr, None, ca_dir.path(), inspectors).await;

    let target = format!("http://127.0.0.1:{}/echo", origin.port());
    let (status, body) = proxy_plaintext_get(tcp_addr, &target).await;

    assert_eq!(status, http::StatusCode::OK);
    assert!(body.contains("\"method\":\"GET\""));
    assert!(body.contains("/echo"));
}

#[tokio::test]
async fn http1_connect_mitm() {
    init_tracing();

    let (origin_addr, _cert) = spawn_https_origin().await;
    let ca_dir = TempDir::new().unwrap();
    let tcp_addr = pick_tcp_addr().await;

    let inspectors = Inspectors::default();
    let (ca_pem, _handle) = start_proxy_on(tcp_addr, None, ca_dir.path(), inspectors).await;

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
    assert!(body.contains("/echo"));
}

#[tokio::test]
async fn http1_connect_post_body_roundtrip() {
    init_tracing();

    let (origin_addr, _cert) = spawn_https_origin().await;
    let ca_dir = TempDir::new().unwrap();
    let tcp_addr = pick_tcp_addr().await;

    let inspectors = Inspectors::default();
    let (ca_pem, _handle) = start_proxy_on(tcp_addr, None, ca_dir.path(), inspectors).await;

    let (status, body) = proxy_connect_post(
        tcp_addr,
        &ca_pem,
        "localhost",
        origin_addr.port(),
        "/echo",
        r#"{"hello":"world"}"#,
        &[],
    )
    .await;

    assert_eq!(status, http::StatusCode::OK);
    assert!(body.contains("\"method\":\"POST\""));
    assert!(body.contains("hello"));
    assert!(body.contains("world"));
}

#[tokio::test]
async fn http1_connect_with_custom_header() {
    init_tracing();

    let (origin_addr, _cert) = spawn_https_origin().await;
    let ca_dir = TempDir::new().unwrap();
    let tcp_addr = pick_tcp_addr().await;

    let inspectors = Inspectors::default();
    let (ca_pem, _handle) = start_proxy_on(tcp_addr, None, ca_dir.path(), inspectors).await;

    let (status, body) = proxy_connect_post(
        tcp_addr,
        &ca_pem,
        "localhost",
        origin_addr.port(),
        "/echo",
        "test",
        &[("x-test", "hello-from-proxy")],
    )
    .await;

    assert_eq!(status, http::StatusCode::OK);
    assert!(body.contains("\"header_x_test\":\"hello-from-proxy\""));
}

#[tokio::test]
async fn http1_streaming_response_is_not_buffered() {
    init_tracing();

    let (origin_addr, _cert, first_sent, release) = spawn_streaming_https_origin().await;
    let ca_dir = TempDir::new().unwrap();
    let tcp_addr = pick_tcp_addr().await;
    let inspectors = Inspectors::default();
    let (ca_pem, _handle) = start_proxy_on(tcp_addr, None, ca_dir.path(), inspectors).await;

    let tls = connect_tls_through_proxy(
        tcp_addr,
        &ca_pem,
        "localhost",
        origin_addr.port(),
        vec![b"http/1.1".to_vec()],
    )
    .await;
    let (mut sender, conn) = hyper::client::conn::http1::handshake(TokioIo::new(tls))
        .await
        .unwrap();
    tokio::spawn(async move {
        let _ = conn.await;
    });

    let request = Request::builder()
        .method("GET")
        .uri("/events")
        .header("host", "localhost")
        .header(http::header::ACCEPT, "text/event-stream")
        .body(Full::new(Bytes::new()))
        .unwrap();
    let mut response = sender.send_request(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    first_sent.await.unwrap();

    let first = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        response.body_mut().frame(),
    )
    .await
    .expect("first streamed frame must arrive before the origin finishes")
    .expect("stream ended before first frame")
    .expect("first streamed frame must be valid")
    .into_data()
    .expect("first streamed frame must contain data");
    assert_eq!(&first[..], b"first");

    release.send(()).unwrap();
    let rest = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&rest[..], b"second");
}

#[tokio::test]
async fn busy_streamed_http1_response_does_not_delay_other_requests() {
    init_tracing();
    let cert = self_signed_cert();
    let mut tls_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert.cert_chain.clone(), cert.key_der())
        .unwrap();
    tls_config.alpn_protocols = vec![b"http/1.1".to_vec()];
    let acceptor = TlsAcceptor::from(Arc::new(tls_config));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_addr = listener.local_addr().unwrap();
    let (release_tx, release_rx) = oneshot::channel::<()>();
    let release = Arc::new(tokio::sync::Mutex::new(Some(release_rx)));
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                break;
            };
            let acceptor = acceptor.clone();
            let release = release.clone();
            tokio::spawn(async move {
                let Ok(mut tls) = acceptor.accept(stream).await else {
                    return;
                };
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    let mut byte = [0u8; 1];
                    if tls.read_exact(&mut byte).await.is_err() {
                        return;
                    }
                    request.push(byte[0]);
                    if request.len() > 4096 {
                        return;
                    }
                }
                if request
                    .windows(b"/events".len())
                    .any(|part| part == b"/events")
                {
                    if tls.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nfirst\r\n").await.is_err() { return }
                    let Some(rx) = release.lock().await.take() else {
                        return;
                    };
                    let _ = rx.await;
                    let _ = tls.write_all(b"6\r\nsecond\r\n0\r\n\r\n").await;
                } else {
                    let _ = tls
                        .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
                        .await;
                }
                let _ = tls.flush().await;
            });
        }
    });
    let ca_dir = TempDir::new().unwrap();
    let proxy_addr = pick_tcp_addr().await;
    let (ca_pem, _handle) =
        start_proxy_on(proxy_addr, None, ca_dir.path(), Inspectors::default()).await;

    let tls = connect_tls_through_proxy(
        proxy_addr,
        &ca_pem,
        "localhost",
        origin_addr.port(),
        vec![b"http/1.1".to_vec()],
    )
    .await;
    let (mut sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(tls))
        .await
        .unwrap();
    tokio::spawn(async move {
        let _ = connection.await;
    });
    let req = Request::builder()
        .method("GET")
        .uri("/events")
        .header("host", "localhost")
        .body(Full::new(Bytes::new()))
        .unwrap();
    let mut first = sender.send_request(req).await.unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(
        first
            .body_mut()
            .frame()
            .await
            .unwrap()
            .unwrap()
            .into_data()
            .unwrap(),
        Bytes::from_static(b"first")
    );

    let second = tokio::time::timeout(
        std::time::Duration::from_secs(8),
        proxy_connect_get(
            proxy_addr,
            &ca_pem,
            "localhost",
            origin_addr.port(),
            "/ordinary",
        ),
    )
    .await
    .expect("second request must not wait for a never-ending first response");
    assert_eq!(second, (StatusCode::OK, "ok".into()));
    release_tx.send(()).unwrap();
    assert_eq!(
        first.into_body().collect().await.unwrap().to_bytes(),
        Bytes::from_static(b"second")
    );
}

#[tokio::test]
async fn http1_upgrade_relays_bidirectional_bytes_after_101() {
    exercise_http1_upgrade(false).await;
}

#[tokio::test]
async fn http1_upgrade_closes_after_policy_change() {
    exercise_http1_upgrade(true).await;
}

async fn exercise_http1_upgrade(terminate: bool) {
    init_tracing();

    let (origin_addr, _cert) = spawn_upgrade_https_origin().await;
    let ca_dir = TempDir::new().unwrap();
    let tcp_addr = pick_tcp_addr().await;
    let policy = Arc::new(MutableTunnelPolicy::default());
    let inspectors = Inspectors {
        request: policy.clone(),
        ..Inspectors::default()
    };
    let (ca_pem, _handle) = start_proxy_on(tcp_addr, None, ca_dir.path(), inspectors).await;

    let tls = connect_tls_through_proxy(
        tcp_addr,
        &ca_pem,
        "localhost",
        origin_addr.port(),
        vec![b"http/1.1".to_vec()],
    )
    .await;
    let (mut sender, conn) = hyper::client::conn::http1::handshake(TokioIo::new(tls))
        .await
        .unwrap();
    tokio::spawn(async move {
        let _ = conn.with_upgrades().await;
    });

    let request = Request::builder()
        .method("GET")
        .uri("/upgrade")
        .header("host", "localhost")
        .header(http::header::CONNECTION, "Upgrade")
        .header(http::header::UPGRADE, "ace-echo")
        .body(Full::new(Bytes::new()))
        .unwrap();
    let mut response = sender.send_request(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);
    assert_eq!(
        response
            .headers()
            .get(http::header::UPGRADE)
            .and_then(|value| value.to_str().ok()),
        Some("ace-echo")
    );

    let upgraded = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        hyper::upgrade::on(&mut response),
    )
    .await
    .expect("client upgrade must not hang")
    .expect("client upgrade must succeed");
    let mut io = TokioIo::new(upgraded);
    let payload = b"stream-through-upgrade";
    io.write_all(payload).await.unwrap();
    let mut echoed = vec![0u8; payload.len()];
    io.read_exact(&mut echoed).await.unwrap();
    assert_eq!(&echoed, payload);
    if terminate {
        policy
            .terminate
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let mut byte = [0];
        let result = tokio::time::timeout(std::time::Duration::from_secs(3), io.read(&mut byte))
            .await
            .expect("upgraded connection must close after a policy change");
        assert!(matches!(result, Ok(0) | Err(_)));
    }
}
