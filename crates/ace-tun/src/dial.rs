//! Outbound relay sockets that are explicitly allowed to escape ACE capture.
//!
//! # Why this exists
//!
//! Once the TUN owns the normal application path, ACE's own relay sockets must
//! be distinguishable from application sockets or they recursively re-enter the
//! netstack. The escape mechanism is platform-specific:
//!
//! * Windows selects the best current non-ACE route for each relay and binds the
//!   socket to that interface with `IP_UNICAST_IF` / `IPV6_UNICAST_IF`.
//! * Linux marks ACE relay sockets with `SO_MARK`; an inverted policy-routing
//!   rule skips only ACE's capture table, preserving NetworkManager/VPN routing.
//! * macOS continues to use `IP_BOUND_IF` / `IPV6_BOUND_IF`.
//!
//! Any failure to establish the escape invariant is fail-closed. An unprepared
//! relay socket is never allowed to fall back to ordinary routing.

use std::io;
use std::net::SocketAddr;
use std::time::Duration;

use tokio::net::{TcpSocket, TcpStream, UdpSocket};

use crate::platform::{PhysicalInterface, pin_socket, set_send_buffer};

/// How long to wait for a pinned outbound TCP connection to complete.
pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Send-buffer size requested for relay sockets.
const UDP_SEND_BUFFER: u32 = 512 * 1024;

/// Native handle of a tokio socket, for the per-OS pinning call.
#[cfg(target_os = "windows")]
fn raw_handle<T: std::os::windows::io::AsRawSocket>(socket: &T) -> u64 {
    socket.as_raw_socket()
}

/// Native handle of a tokio socket, for the per-OS pinning call.
#[cfg(not(target_os = "windows"))]
fn raw_handle<T: std::os::fd::AsRawFd>(socket: &T) -> i32 {
    socket.as_raw_fd()
}

/// Open a TCP connection to `dest` that bypasses the tunnel.
///
/// For non-loopback destinations a platform escape must be established before
/// `connect`. If the required route/interface metadata is unavailable, the dial
/// fails rather than risking recursion or an unintended bypass.
pub(crate) async fn tcp(dest: SocketAddr, iface: &PhysicalInterface) -> io::Result<TcpStream> {
    let socket = if dest.is_ipv4() {
        TcpSocket::new_v4()?
    } else {
        TcpSocket::new_v6()?
    };

    if !dest.ip().is_loopback() {
        let index = iface.index_for(dest.ip()).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotConnected,
                format!("no safe egress interface for {}", dest.ip()),
            )
        })?;
        // Failing to establish the platform escape invariant is security
        // relevant: an unprepared relay socket can recursively enter ACE or
        // bypass the intended policy. Fail closed instead of dialing normally.
        pin_socket(raw_handle(&socket), index, dest.ip())?;
    }

    let stream = tokio::time::timeout(CONNECT_TIMEOUT, socket.connect(dest))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "outbound connect timed out"))??;
    stream.set_nodelay(true).ok();
    Ok(stream)
}

/// Bind a UDP socket that bypasses the tunnel, for relaying a single flow.
///
/// The socket is bound to the wildcard address of `dest`'s family and then
/// connected, so it only exchanges datagrams with that one peer.
pub(crate) async fn udp(dest: SocketAddr, iface: &PhysicalInterface) -> io::Result<UdpSocket> {
    let bind: SocketAddr = if dest.is_ipv4() {
        "0.0.0.0:0".parse().expect("static addr")
    } else {
        "[::]:0".parse().expect("static addr")
    };

    let socket = UdpSocket::bind(bind).await?;

    if !dest.ip().is_loopback() {
        let index = iface.index_for(dest.ip()).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotConnected,
                format!("no safe UDP egress interface for {}", dest.ip()),
            )
        })?;
        pin_socket(raw_handle(&socket), index, dest.ip())?;
    }

    // The OS defaults UDP sockets to a small send buffer and returns
    // WSAENOBUFS the moment it fills. Relaying a whole flow through one socket
    // makes that far more likely than it is for an ordinary application, so ask
    // for more headroom. Best-effort: if the kernel declines, the caller
    // tolerates the resulting send errors anyway.
    if let Err(error) = set_send_buffer(raw_handle(&socket), UDP_SEND_BUFFER) {
        tracing::debug!("could not raise UDP send buffer: {error}");
    }

    socket.connect(dest).await?;
    Ok(socket)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Loopback never needs an escape interface, even when no egress was found.
    #[tokio::test]
    async fn loopback_dial_works_without_an_egress_interface() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = listener.accept().await;
        });

        let iface = PhysicalInterface::default();
        assert!(iface.is_empty());
        let stream = tcp(addr, &iface)
            .await
            .expect("unpinned dial should connect");
        assert_eq!(stream.peer_addr().unwrap(), addr);
    }

    /// Pinning to a bogus interface index must not panic; the dial may fail,
    /// but it fails as an ordinary I/O error.
    #[tokio::test]
    async fn bogus_interface_index_does_not_panic() {
        let iface = PhysicalInterface {
            v4_index: Some(0xDEAD),
            v6_index: None,
        };
        let dest: SocketAddr = "127.0.0.1:9".parse().unwrap();
        let _ = tcp(dest, &iface).await;
    }
}
