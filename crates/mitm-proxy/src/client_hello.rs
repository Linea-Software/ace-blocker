//! Recover TLS SNI from transparent IP-literal CONNECT tunnels.
//!
//! The browser may have resolved the host before ACE started, so ace-tun's
//! passive DNS cache can be empty. This bounded peek keeps every byte available
//! for the subsequent TLS handshake or opaque relay.
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use rustls::server::Acceptor;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, ReadBuf};

const MAX_CLIENT_HELLO_BYTES: usize = 64 * 1024;
const CLIENT_HELLO_TIMEOUT: Duration = Duration::from_secs(5);

pub(super) struct PrefixedIo<S> {
    prefix: Vec<u8>,
    cursor: usize,
    inner: S,
}

impl<S: AsyncRead + Unpin> AsyncRead for PrefixedIo<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.cursor < self.prefix.len() {
            let remaining = &self.prefix[self.cursor..];
            let length = remaining.len().min(buf.remaining());
            buf.put_slice(&remaining[..length]);
            self.cursor += length;
            Poll::Ready(Ok(()))
        } else {
            Pin::new(&mut self.inner).poll_read(context, buf)
        }
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for PrefixedIo<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(context, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(context)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(context)
    }
}

/// Returns the original stream (including the consumed ClientHello bytes) and
/// the advertised server name. Missing, malformed, or stalled TLS must not be
/// treated as an allowed website when the CONNECT destination is only an IP.
pub(super) async fn recover_sni<S: AsyncRead + AsyncWrite + Unpin>(
    mut io: S,
) -> io::Result<(PrefixedIo<S>, String)> {
    let mut acceptor = Acceptor::default();
    let mut prefix = Vec::new();
    let server_name = tokio::time::timeout(CLIENT_HELLO_TIMEOUT, async {
        loop {
            if prefix.len() >= MAX_CLIENT_HELLO_BYTES {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "TLS ClientHello exceeds limit",
                ));
            }
            let mut chunk = [0u8; 4096];
            let capacity = (MAX_CLIENT_HELLO_BYTES - prefix.len()).min(chunk.len());
            let amount = io.read(&mut chunk[..capacity]).await?;
            if amount == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "TLS ClientHello ended before SNI",
                ));
            }
            prefix.extend_from_slice(&chunk[..amount]);
            acceptor.read_tls(&mut io::Cursor::new(&chunk[..amount]))?;
            match acceptor.accept() {
                Ok(Some(accepted)) => {
                    return accepted
                        .client_hello()
                        .server_name()
                        .map(str::to_owned)
                        .ok_or_else(|| {
                            io::Error::new(io::ErrorKind::InvalidData, "TLS ClientHello has no SNI")
                        });
                }
                Ok(None) => {}
                Err((error, _)) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("invalid TLS ClientHello: {error}"),
                    ));
                }
            }
        }
    })
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "TLS ClientHello timeout"))??;
    Ok((
        PrefixedIo {
            prefix,
            cursor: 0,
            inner: io,
        },
        server_name,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn recovers_fragmented_client_hello_without_consuming_it() {
        crate::install_crypto_provider();
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(rustls::RootCertStore::empty())
            .with_no_client_auth();
        let mut client = rustls::ClientConnection::new(
            std::sync::Arc::new(config),
            "blocked.example".try_into().unwrap(),
        )
        .unwrap();
        let mut hello = Vec::new();
        client.write_tls(&mut hello).unwrap();
        let (mut writer, reader) = tokio::io::duplex(hello.len() + 1024);
        let bytes = hello.clone();
        let task = tokio::spawn(async move {
            for part in bytes.chunks(7) {
                writer.write_all(part).await.unwrap();
            }
        });
        let (mut replay, hostname) = recover_sni(reader).await.unwrap();
        assert_eq!(hostname, "blocked.example");
        let mut received = vec![0u8; hello.len()];
        replay.read_exact(&mut received).await.unwrap();
        assert_eq!(received, hello);
        task.await.unwrap();
    }

    #[tokio::test]
    async fn fails_closed_when_the_client_hello_has_no_tls_identity() {
        let (mut writer, reader) = tokio::io::duplex(128);
        writer.write_all(b"not TLS").await.unwrap();
        drop(writer);
        assert!(recover_sni(reader).await.is_err());
    }
}
