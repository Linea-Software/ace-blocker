//! Regression coverage for HTTP/2 requests forwarded to HTTP/1.1 origins.
//! The UniBo live check remains opt-in because it requires internet access.
#[allow(dead_code)]
mod common;

use bytes::Bytes;
use common::*;
use http::{Request, StatusCode, Version};
use http_body_util::{BodyExt, Full};
use hyper_util::rt::{TokioExecutor, TokioIo};
use mitm_proxy::{BufferedRequest, ConnMeta};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

/// Experimental control only: reconstruct the missing header after h2 decoding.
struct SupplyHost;

#[async_trait::async_trait]
impl RequestInspector for SupplyHost {
    async fn inspect_request(&self, _: &ConnMeta, request: &mut BufferedRequest) -> RequestAction {
        let authority = request.uri().authority().unwrap().as_str().to_owned();
        request
            .headers_mut()
            .insert("host", authority.parse().unwrap());
        RequestAction::Continue
    }
}

async fn request_through_proxy(
    host: &str,
    port: u16,
    use_http2: bool,
    supply_host: bool,
) -> StatusCode {
    let ca_dir = TempDir::new().unwrap();
    let proxy_addr = pick_tcp_addr().await;
    let mut inspectors = Inspectors::default();
    if supply_host {
        inspectors.request = Arc::new(SupplyHost);
    }
    let (ca_pem, proxy_task) = start_proxy_on(proxy_addr, None, ca_dir.path(), inspectors).await;
    let protocol = if use_http2 {
        b"h2".to_vec()
    } else {
        b"http/1.1".to_vec()
    };
    let tls = connect_tls_through_proxy(proxy_addr, &ca_pem, host, port, vec![protocol]).await;
    let authority = if port == 443 {
        host.to_owned()
    } else {
        format!("{host}:{port}")
    };
    let mut request = Request::builder()
        .uri(format!("https://{authority}/it/studiare/vivere-luniversita-e-la-citta/wi-fi/faq-almawifi"))
        .version(if use_http2 { Version::HTTP_2 } else { Version::HTTP_11 })
        .header("accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8")
        .header("accept-language", "en-US,en;q=0.8")
        .header("cache-control", "max-age=0")
        .header("priority", "u=0, i")
        .header("upgrade-insecure-requests", "1")
        .header("referer", "https://safe.search.brave.com/");
    if !use_http2 {
        request = request.header("host", authority);
    }
    let request = request.body(Full::new(Bytes::new())).unwrap();
    let response = if use_http2 {
        let (mut sender, connection) =
            hyper::client::conn::http2::handshake(TokioExecutor::new(), TokioIo::new(tls))
                .await
                .unwrap();
        tokio::spawn(async move {
            connection.await.unwrap();
        });
        sender.send_request(request).await.unwrap()
    } else {
        let (mut sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(tls))
            .await
            .unwrap();
        tokio::spawn(async move {
            connection.await.unwrap();
        });
        sender.send_request(request).await.unwrap()
    };
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    println!(
        "client_http2={use_http2}, supply_host={supply_host}, status={status}, body_bytes={}",
        body.len()
    );
    proxy_task.abort();
    status
}

#[tokio::test]
async fn http2_to_http1_preserves_authority_as_host() {
    let cert = self_signed_cert();
    let mut config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert.cert_chain.clone(), cert.key_der())
        .unwrap();
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let origin_task = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.unwrap();
        let mut tls = acceptor.accept(tcp).await.unwrap();
        let mut wire = Vec::new();
        while !wire.ends_with(b"\r\n\r\n") {
            wire.push(tls.read_u8().await.unwrap());
            assert!(wire.len() < 65536);
        }
        tls.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        String::from_utf8(wire).unwrap()
    });
    request_through_proxy("localhost", port, true, false).await;
    let wire = origin_task.await.unwrap();
    println!("Upstream wire request:\n{wire}");
    assert!(
        wire.to_ascii_lowercase()
            .contains(&format!("\r\nhost: localhost:{port}\r\n")),
        "HTTP/1.1 requires Host reconstructed from HTTP/2 :authority"
    );
}

#[tokio::test]
#[ignore = "contacts public www.unibo.it; opt-in reproduction, requires network"]
async fn unibo_http2_and_http1_both_succeed() {
    let http1_status = request_through_proxy("www.unibo.it", 443, false, false).await;
    let http2_status = request_through_proxy("www.unibo.it", 443, true, false).await;
    let control_status = request_through_proxy("www.unibo.it", 443, true, true).await;
    assert_eq!(http1_status, StatusCode::OK);
    assert_eq!(control_status, StatusCode::OK);
    assert_eq!(http2_status, StatusCode::OK);
}
