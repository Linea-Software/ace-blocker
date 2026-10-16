//! ICMP echo compatibility for traffic captured by WinTun.
//!
//! `ipstack` exposes ICMP as an unknown transport. Windows raw ICMP sockets are
//! used only for echo requests, pinned to the current non-ACE route so the
//! re-originated probe cannot recurse back into the tunnel.

use std::io;
use std::mem::MaybeUninit;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::os::windows::io::AsRawSocket;
use std::time::Duration;

use ipstack::IpStackUnknownTransport;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};

const ICMP_V4: u8 = 1;
const ICMP_V6: u8 = 58;
const ICMP_HEADER_LEN: usize = 8;
const ECHO_TIMEOUT: Duration = Duration::from_secs(3);
const RECEIVE_BUFFER: usize = 65_535;

/// Relay an ICMP echo request. Returns `Ok(false)` for non-echo/unsupported
/// unknown transports so the caller can keep its ordinary drop behavior.
pub(crate) async fn relay_echo(packet: IpStackUnknownTransport) -> io::Result<bool> {
    let protocol = packet.ip_protocol().0;
    if !is_echo_request(protocol, packet.payload()) {
        return Ok(false);
    }

    let client_source = packet.src_addr();
    let destination = packet.dst_addr();
    let request = packet.payload().to_vec();

    let reply = tokio::task::spawn_blocking(move || {
        exchange_echo(client_source, destination, protocol, &request)
    })
    .await
    .map_err(|error| io::Error::other(format!("ICMP relay task failed: {error}")))??;

    if let Some(reply) = reply {
        packet.send(reply)?;
    }
    Ok(true)
}

fn exchange_echo(
    client_source: IpAddr,
    destination: IpAddr,
    protocol: u8,
    request: &[u8],
) -> io::Result<Option<Vec<u8>>> {
    let route = super::netcfg::best_non_tunnel_route(destination).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotConnected,
            format!("no non-ACE ICMP route is available for {destination}"),
        )
    })?;
    if route.source.is_ipv4() != destination.is_ipv4() {
        return Err(io::Error::other("ICMP route source family mismatch"));
    }

    let (domain, socket_protocol) = match protocol {
        ICMP_V4 => (Domain::IPV4, Protocol::ICMPV4),
        ICMP_V6 => (Domain::IPV6, Protocol::ICMPV6),
        _ => return Ok(None),
    };
    let socket = Socket::new(domain, Type::RAW, Some(socket_protocol))?;
    super::dial::pin_socket(socket.as_raw_socket(), route.interface_index, destination)?;
    socket.bind(&SockAddr::from(SocketAddr::new(route.source, 0)))?;
    socket.set_read_timeout(Some(ECHO_TIMEOUT))?;

    let wire_request = request_for_route(protocol, route.source, destination, request)?;
    let destination_addr = SockAddr::from(SocketAddr::new(destination, 0));
    let sent = socket.send_to(&wire_request, &destination_addr)?;
    if sent != wire_request.len() {
        return Err(io::Error::new(
            io::ErrorKind::WriteZero,
            "raw ICMP socket sent a partial echo request",
        ));
    }

    let request_identity = &request[4..8];
    let mut buffer = vec![MaybeUninit::<u8>::uninit(); RECEIVE_BUFFER];
    loop {
        let (received, peer) = match socket.recv_from(&mut buffer) {
            Ok(result) => result,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                return Ok(None);
            }
            Err(error) => return Err(error),
        };

        if peer.as_socket().map(|address| address.ip()) != Some(destination) {
            continue;
        }

        // SAFETY: recv_from initialized exactly the first `received` bytes.
        let bytes = unsafe { std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), received) };
        let Some(icmp) = received_icmp_payload(bytes) else {
            continue;
        };
        if !is_matching_echo_reply(protocol, request_identity, icmp) {
            continue;
        }

        return Ok(Some(build_client_reply(
            protocol,
            destination,
            client_source,
            request_identity,
            &icmp[ICMP_HEADER_LEN..],
        )?));
    }
}

fn is_echo_request(protocol: u8, payload: &[u8]) -> bool {
    payload.len() >= ICMP_HEADER_LEN
        && payload[1] == 0
        && matches!((protocol, payload[0]), (ICMP_V4, 8) | (ICMP_V6, 128))
}

fn is_matching_echo_reply(protocol: u8, identity: &[u8], payload: &[u8]) -> bool {
    payload.len() >= ICMP_HEADER_LEN
        && payload[1] == 0
        && payload[4..8] == *identity
        && matches!((protocol, payload[0]), (ICMP_V4, 0) | (ICMP_V6, 129))
}

fn request_for_route(
    protocol: u8,
    source: IpAddr,
    destination: IpAddr,
    request: &[u8],
) -> io::Result<Vec<u8>> {
    if !is_echo_request(protocol, request) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "not an echo request",
        ));
    }

    let mut request = request.to_vec();
    request[2] = 0;
    request[3] = 0;
    let checksum = match (protocol, source, destination) {
        (ICMP_V4, IpAddr::V4(_), IpAddr::V4(_)) => internet_checksum(&request),
        (ICMP_V6, IpAddr::V6(source), IpAddr::V6(destination)) => {
            icmpv6_checksum(source, destination, &request)
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "ICMP family mismatch",
            ));
        }
    };
    request[2..4].copy_from_slice(&checksum.to_be_bytes());
    Ok(request)
}

fn build_client_reply(
    protocol: u8,
    source: IpAddr,
    destination: IpAddr,
    identity: &[u8],
    data: &[u8],
) -> io::Result<Vec<u8>> {
    let reply_type = match protocol {
        ICMP_V4 => 0,
        ICMP_V6 => 129,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unsupported ICMP protocol",
            ));
        }
    };
    let mut reply = Vec::with_capacity(ICMP_HEADER_LEN + data.len());
    reply.extend_from_slice(&[reply_type, 0, 0, 0]);
    reply.extend_from_slice(identity);
    reply.extend_from_slice(data);

    let checksum = match (protocol, source, destination) {
        (ICMP_V4, IpAddr::V4(_), IpAddr::V4(_)) => internet_checksum(&reply),
        (ICMP_V6, IpAddr::V6(source), IpAddr::V6(destination)) => {
            icmpv6_checksum(source, destination, &reply)
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "ICMP family mismatch",
            ));
        }
    };
    reply[2..4].copy_from_slice(&checksum.to_be_bytes());
    Ok(reply)
}

fn received_icmp_payload(bytes: &[u8]) -> Option<&[u8]> {
    if bytes.len() < ICMP_HEADER_LEN {
        return None;
    }
    match bytes[0] >> 4 {
        4 => {
            let header_len = usize::from(bytes[0] & 0x0f) * 4;
            (header_len >= 20 && bytes.len() >= header_len + ICMP_HEADER_LEN)
                .then_some(&bytes[header_len..])
        }
        6 => (bytes.len() >= 40 + ICMP_HEADER_LEN).then_some(&bytes[40..]),
        _ => Some(bytes),
    }
}

fn internet_checksum(bytes: &[u8]) -> u16 {
    let mut sum = 0u32;
    for chunk in bytes.chunks(2) {
        let word = if chunk.len() == 2 {
            u16::from_be_bytes([chunk[0], chunk[1]])
        } else {
            u16::from_be_bytes([chunk[0], 0])
        };
        sum += u32::from(word);
    }
    while sum > 0xffff {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

fn icmpv6_checksum(source: Ipv6Addr, destination: Ipv6Addr, payload: &[u8]) -> u16 {
    let mut pseudo = Vec::with_capacity(40 + payload.len());
    pseudo.extend_from_slice(&source.octets());
    pseudo.extend_from_slice(&destination.octets());
    pseudo.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    pseudo.extend_from_slice(&[0, 0, 0, ICMP_V6]);
    pseudo.extend_from_slice(payload);
    internet_checksum(&pseudo)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn recognizes_echo_requests_only() {
        assert!(is_echo_request(ICMP_V4, &[8, 0, 0, 0, 0, 1, 0, 2]));
        assert!(is_echo_request(ICMP_V6, &[128, 0, 0, 0, 0, 1, 0, 2]));
        assert!(!is_echo_request(ICMP_V4, &[3, 0, 0, 0, 0, 1, 0, 2]));
        assert!(!is_echo_request(ICMP_V6, &[129, 0, 0, 0, 0, 1, 0, 2]));
    }

    #[test]
    fn v4_reply_has_valid_checksum_and_preserves_identity() {
        let reply = build_client_reply(
            ICMP_V4,
            IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
            IpAddr::V4(Ipv4Addr::new(10, 63, 7, 1)),
            &[0x12, 0x34, 0x00, 0x07],
            b"payload",
        )
        .unwrap();
        assert_eq!(&reply[4..8], &[0x12, 0x34, 0x00, 0x07]);
        assert_eq!(internet_checksum(&reply), 0);
    }

    #[test]
    fn v6_reply_has_valid_pseudoheader_checksum() {
        let source: Ipv6Addr = "2001:4860:4860::8888".parse().unwrap();
        let destination: Ipv6Addr = "fd00:ace:7::1".parse().unwrap();
        let reply = build_client_reply(
            ICMP_V6,
            IpAddr::V6(source),
            IpAddr::V6(destination),
            &[0xab, 0xcd, 0x00, 0x09],
            b"payload",
        )
        .unwrap();
        assert_eq!(icmpv6_checksum(source, destination, &reply), 0);
    }

    #[test]
    fn strips_ipv4_header_from_raw_socket_receive() {
        let mut packet = vec![0u8; 20];
        packet[0] = 0x45;
        packet.extend_from_slice(&[0, 0, 0, 0, 0x12, 0x34, 0, 1]);
        assert_eq!(
            received_icmp_payload(&packet).unwrap(),
            &[0, 0, 0, 0, 0x12, 0x34, 0, 1]
        );
    }
}
