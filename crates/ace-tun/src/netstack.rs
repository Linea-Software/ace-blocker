//! Flow dispatch: decide what happens to each connection the netstack accepts.
//!
//! `ipstack` hands us fully-formed flows rather than packets. That is the whole
//! point of the migration: the original destination arrives in the IP header of
//! the SYN, so there is nothing to reverse-map, no NAT table to keep coherent,
//! and no window in which a flow is "not yet attributed". A flow is either
//! accepted here with its true destination, or it does not exist.
//!
//! # Failure posture
//!
//! Unattributed traffic and explicit `Direct` rules remain fail-open. Once a
//! rule selects `Proxy`, however, failure to reach that proxy is fail-closed:
//! silently bypassing inspection would also bypass blocking and resistance.
//!
//! There are two deliberate exceptions — QUIC, and group-addressed traffic.
//! See [`decide`].

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use ipstack::{IpStack, IpStackStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::watch;

use crate::callback::ConnectionInfo;
use crate::dial;
use crate::proxy::{self, Target};
use crate::rule::RuleAction;
use crate::state::{QuicPolicy, Shared};

/// How long a UDP flow may sit idle before we release it.
const UDP_IDLE_TIMEOUT: Duration = Duration::from_secs(30);

/// Largest datagram we will relay. Anything bigger is already beyond the
/// tunnel MTU and would have been fragmented upstream.
const UDP_BUFFER: usize = 65_535;

/// What to do with an accepted flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Decision {
    /// Connect straight to the original destination, bypassing the tunnel.
    Direct,
    /// Relay through the upstream proxy identified by this config id.
    Proxy(u32),
    /// Relay datagrams exclusively to the local HTTP/3 proxy.
    QuicProxy,
    /// Drop the flow.
    Block,
}

/// The facts about a flow that feed [`decide`].
///
/// Bundled into a struct so the decision logic can be exercised in tests
/// without a TUN adapter, a kernel driver, or elevation.
#[derive(Debug, Clone)]
pub(crate) struct FlowFacts<'a> {
    /// Original destination address.
    pub(crate) dest: SocketAddr,
    /// `true` for UDP, `false` for TCP.
    pub(crate) is_udp: bool,
    /// PID that owns the local endpoint, if it could be resolved.
    pub(crate) pid: Option<u32>,
    /// Executable file name of that process, lowercased.
    pub(crate) process_name: &'a str,
    /// Destination hostname from the DNS-snoop cache, if known.
    pub(crate) domain: Option<&'a str>,
}

/// Choose an action for one flow.
///
/// The order of the checks is load-bearing:
///
/// 1. **Our own traffic goes direct, always.** This is the loop guard. The
///    engine's upstream connections re-enter the tunnel; if a rule could send
///    them back to the proxy we would recurse until the process died. The one
///    case where our own flow is dropped instead is when there is no physical
///    interface to pin the outbound socket to, because then "direct" would
///    itself re-enter the tunnel.
/// 2. **Loopback goes direct** unless explicitly configured otherwise.
/// 3. **Captured multicast and broadcast are dropped.** Platform routing keeps
///    normal LAN discovery outside ACE; this remains a fail-safe for group
///    traffic that nevertheless reaches the userland stack, because it cannot
///    be represented by the connected unicast relay.
/// 4. **Protected-browser QUIC is dropped** when the policy says so. Flows
///    whose process identity is unavailable are also dropped to avoid an
///    attribution race leaking browser HTTP/3. Known non-protected applications
///    keep their normal UDP/443 path; UDP/443 is not synonymous with browser
///    QUIC and those applications may not have a TCP fallback.
/// 5. Otherwise the rule set decides, defaulting to direct.
pub(crate) fn decide(shared: &Shared, facts: &FlowFacts<'_>) -> Decision {
    if facts.pid == Some(shared.current_pid) {
        // Our own upstream traffic reaches the internet only because
        // `dial::tcp` pins the socket to the physical NIC, bypassing our own
        // routes. If there is no interface to pin to, relaying this flow would
        // send it straight back into the tunnel and into this function again,
        // forever. Dropping it is the only terminating option — and the flow
        // could not have succeeded anyway.
        if !facts.dest.ip().is_loopback() && shared.iface().index_for(facts.dest.ip()).is_none() {
            tracing::error!(
                "no physical interface for {}; dropping our own flow to avoid a routing loop",
                facts.dest
            );
            return Decision::Block;
        }
        return Decision::Direct;
    }

    if facts.dest.ip().is_loopback() && !shared.localhost_via_proxy {
        return Decision::Direct;
    }

    // Multicast and broadcast cannot be relayed meaningfully. Our outbound
    // sockets are unicast, pinned, and `connect`ed to a single peer; sending a
    // group address down one of those is not what the sender asked for, and
    // replies would arrive on a socket nobody is reading. Windows also punishes
    // the attempt — a socket per mDNS/SSDP packet is how this exhausts buffers
    // and starts failing with WSAENOBUFS.
    if is_group_address(facts.dest.ip()) {
        return Decision::Block;
    }

    if facts.is_udp
        && facts.dest.port() == shared.quic_port
        && shared.quic_policy == QuicPolicy::Drop
        && (facts.process_name.is_empty()
            || shared
                .protected_processes
                .iter()
                .any(|protected| process_names_match(protected, facts.process_name)))
    {
        return Decision::Block;
    }

    let m = shared.rules.evaluate(
        facts.process_name,
        facts.dest.ip(),
        facts.dest.port(),
        facts.is_udp,
        facts.domain,
    );

    match m.action {
        RuleAction::Direct => Decision::Direct,
        RuleAction::Block => Decision::Block,
        RuleAction::Proxy => Decision::Proxy(m.proxy_config_id),
    }
}

/// Recognize a client Initial long header without treating arbitrary UDP as QUIC.
/// Initial datagrams are at least 1200 bytes; version negotiation (zero) and
/// non-Initial long-header types are deliberately excluded. Connection IDs
/// have the invariant-header maximum length of 20 bytes.
fn is_quic_initial(packet: &[u8]) -> bool {
    if packet.len() < 1200 || packet[0] & 0xc0 != 0xc0 {
        return false;
    }
    let version = u32::from_be_bytes(packet[1..5].try_into().unwrap());
    let initial_type = if version == 0x6b3343cf { 0x10 } else { 0x00 };
    if version == 0 || packet[0] & 0x30 != initial_type {
        return false;
    }
    let dcid_len = packet[5] as usize;
    if dcid_len > 20 {
        return false;
    }
    let scid_offset = 6 + dcid_len;
    let Some(&scid_len) = packet.get(scid_offset) else {
        return false;
    };
    scid_len <= 20 && scid_offset + 1 + usize::from(scid_len) < packet.len()
}

/// Check whether a datagram is a conventional DNS query. DNS transaction IDs
/// are random, so looking at the first byte alone can falsely label DNS as
/// migrated QUIC. This parser validates the query header and first question
/// without trusting an arbitrary prefix as proof that the flow is DNS.
fn is_dns_query(packet: &[u8]) -> bool {
    if packet.len() < 17 {
        return false;
    }
    let flags = u16::from_be_bytes([packet[2], packet[3]]);
    let questions = u16::from_be_bytes([packet[4], packet[5]]);
    let answers = u16::from_be_bytes([packet[6], packet[7]]);
    let authorities = u16::from_be_bytes([packet[8], packet[9]]);
    let additionals = u16::from_be_bytes([packet[10], packet[11]]);
    if flags & 0xf800 != 0 || questions != 1 || answers != 0 || authorities != 0 || additionals > 4
    {
        return false;
    }
    let mut position = 12;
    let mut name_length = 0_usize;
    loop {
        let Some(&label_length) = packet.get(position) else {
            return false;
        };
        position += 1;
        if label_length == 0 {
            break;
        }
        let length = label_length as usize;
        if length > 63 || position + length > packet.len() {
            return false;
        }
        name_length += 1 + length;
        if name_length > 253 {
            return false;
        }
        position += length;
    }
    position + 4 <= packet.len()
}

/// Recognize plausible QUIC packets that lack an Initial (including migrated
/// connections). The fixed bit is only a heuristic; unlike actual Initials it
/// cannot prove protocol identity without the QUIC connection's secrets.
fn is_possible_quic_non_initial(packet: &[u8]) -> bool {
    packet.len() >= 22
        && packet.first().is_some_and(|first| {
            let is_short = first & 0xc0 == 0x40;
            let nonzero_long_version = first & 0xc0 == 0xc0 && packet[1..5] != [0, 0, 0, 0];
            is_short || nonzero_long_version
        })
}

/// Security boundary for *every* subsequent datagram on a browser-owned
/// Direct UDP flow. A harmless first packet cannot authorize a later QUIC
/// handshake on the same tuple. Unrelated applications keep opaque UDP.
fn allow_direct_udp_followup(packet: &[u8], is_dns: bool, guard: bool) -> bool {
    if !guard {
        return true;
    }
    if is_dns {
        return is_dns_query(packet) && !is_quic_initial(packet);
    }
    !is_quic_initial(packet) && !is_possible_quic_non_initial(packet)
}

pub(crate) fn decide_udp_packet(
    shared: &Shared,
    facts: &FlowFacts<'_>,
    first_packet: &[u8],
) -> Decision {
    let base = decide(shared, facts);
    // Preserve loop guards, local/multicast routing and explicit Block rules.
    if !facts.is_udp
        || facts.pid == Some(shared.current_pid)
        || (facts.dest.ip().is_loopback() && !shared.localhost_via_proxy)
        || is_group_address(facts.dest.ip())
        || base == Decision::Block
    {
        return base;
    }

    // HTTP CONNECT cannot transport UDP; a rule asking for Proxy must never
    // silently open a direct socket to the origin (regardless of QUIC policy).
    if matches!(base, Decision::Proxy(_)) {
        return Decision::Block;
    }

    let initial = is_quic_initial(first_packet);
    // The QUIC invariant fixed bit can also occur in a random DNS transaction
    // ID. Legitimate browser DNS must remain usable; all following datagrams
    // on a protected DNS flow are validated as DNS by the UDP relay.
    if facts.dest.port() == 53 && !initial && is_dns_query(first_packet) {
        return base;
    }
    // A connection migrating into a fresh TUN tuple can start with a short
    // header OR a non-Initial long header. QUIC's fixed bit is visible in
    // either case. Only classify these as suspicious for protected/unknown
    // flows: UDP owned by unrelated programs stays opaque and unmodified.
    let possible_quic_non_initial = is_possible_quic_non_initial(first_packet);
    let configured_quic_port = facts.dest.port() == shared.quic_port;
    if !initial && !possible_quic_non_initial && !configured_quic_port {
        return base;
    }
    if shared.quic_policy == QuicPolicy::Allow {
        return base;
    }

    if facts.pid.is_none() || facts.process_name.is_empty() {
        return Decision::Block;
    }
    if !shared
        .protected_processes
        .iter()
        .any(|name| process_names_match(name, facts.process_name))
    {
        return base;
    }
    // Only a fresh Initial can be safely forwarded to the local H3 listener.
    // Short/malformed/unknown-version flows cannot rejoin an established
    // direct connection and may not bypass the enforcement boundary.
    if !initial {
        return Decision::Block;
    }
    match shared.quic_policy {
        QuicPolicy::Intercept
            if shared
                .quic_proxy_addr
                .is_some_and(|addr| addr.ip().is_loopback()) =>
        {
            Decision::QuicProxy
        }
        _ => Decision::Block,
    }
}

/// Whether `ip` is a multicast or broadcast address — i.e. addressed to a group
/// rather than to one host.
///
/// Platform-specific route policy is expected to keep these outside the TUN.
/// This check is deliberately retained as a safety net: a captured mDNS/SSDP/
/// LLMNR packet cannot be faithfully relayed by our connected unicast socket.
fn is_group_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_multicast() || v4.is_broadcast(),
        IpAddr::V6(v6) => v6.is_multicast(),
    }
}

fn process_names_match(configured: &str, observed: &str) -> bool {
    let configured = configured.strip_suffix(".exe").unwrap_or(configured);
    let observed = observed.strip_suffix(".exe").unwrap_or(observed);
    configured.eq_ignore_ascii_case(observed)
}

/// Accept flows until the stack closes or `shutdown` fires.
pub(crate) async fn run(
    mut stack: IpStack,
    shared: Arc<Shared>,
    mut shutdown: watch::Receiver<bool>,
) {
    loop {
        let stream = tokio::select! {
            _ = shutdown.changed() => break,
            accepted = stack.accept() => match accepted {
                Ok(s) => s,
                Err(error) => {
                    shared.log(format!("netstack accept ended: {error}"));
                    break;
                }
            },
        };

        let shared = Arc::clone(&shared);
        match stream {
            IpStackStream::Tcp(tcp) => {
                shared.stats.tcp_flows.fetch_add(1, Ordering::Relaxed);
                tokio::spawn(async move {
                    if let Err(error) = handle_tcp(tcp, &shared).await {
                        shared.log(format!("tcp flow ended: {error}"));
                    }
                });
            }
            IpStackStream::Udp(udp) => {
                shared.stats.udp_flows.fetch_add(1, Ordering::Relaxed);
                tokio::spawn(async move {
                    if let Err(error) = handle_udp(udp, &shared).await {
                        shared.log(format!("udp flow ended: {error}"));
                    }
                });
            }
            IpStackStream::UnknownTransport(unknown) => {
                #[cfg(target_os = "windows")]
                {
                    let protocol = unknown.ip_protocol();
                    tokio::spawn(async move {
                        match crate::platform::relay_icmp_echo(unknown).await {
                            Ok(true) => {}
                            Ok(false) => {
                                tracing::trace!("dropping unsupported transport {protocol:?}");
                            }
                            Err(error) => {
                                tracing::debug!("ICMP relay failed for {protocol:?}: {error}");
                            }
                        }
                    });
                }
                #[cfg(not(target_os = "windows"))]
                {
                    tracing::trace!("dropping unsupported transport {:?}", unknown.ip_protocol());
                }
            }
            IpStackStream::UnknownNetwork(bytes) => {
                tracing::trace!("dropping unparseable {}-byte packet", bytes.len());
            }
        }
    }
    shared.log("netstack accept loop stopped");
}

/// Gather the facts for a flow: owning process and cached hostname.
///
/// Attribution happens exactly once, here, at flow open — by which point the
/// socket is guaranteed to exist in the kernel's table, because the SYN we are
/// responding to could not have been sent otherwise.
fn attribute(src: SocketAddr, dest: SocketAddr, is_udp: bool) -> (Option<u32>, String) {
    let pid = crate::platform::resolve_pid(src, dest, is_udp);
    let name = pid
        .and_then(crate::platform::process_name)
        .map(|n| n.to_ascii_lowercase())
        .unwrap_or_default();
    if pid.is_none() {
        tracing::trace!("no owning process for {src} -> {dest}");
    }
    (pid, name)
}

/// Look up the destination hostname, but only if some rule actually needs it.
fn domain_for(shared: &Shared, dest: IpAddr) -> Option<String> {
    if shared.rules.needs_domain_resolution() {
        shared.dns_cache.lookup(dest)
    } else {
        None
    }
}

/// Relay one TCP flow.
async fn handle_tcp(mut stream: ipstack::IpStackTcpStream, shared: &Shared) -> std::io::Result<()> {
    let src = stream.local_addr();
    let dest = stream.peer_addr();

    let (pid, process_name) = attribute(src, dest, false);
    let domain = domain_for(shared, dest.ip());
    let facts = FlowFacts {
        dest,
        is_udp: false,
        pid,
        process_name: &process_name,
        domain: domain.as_deref(),
    };
    let decision = decide(shared, &facts);
    report(shared, &facts, decision);

    tracing::debug!(
        source = %src,
        destination = %dest,
        pid = pid.unwrap_or(0),
        process = %if process_name.is_empty() { "<unknown>" } else { &process_name },
        domain = %domain.as_deref().unwrap_or("<unknown>"),
        decision = ?decision,
        "TCP flow accepted"
    );

    match decision {
        Decision::QuicProxy => unreachable!("TCP cannot select the QUIC proxy"),
        Decision::Block => {
            shared.stats.blocked.fetch_add(1, Ordering::Relaxed);
            // Dropping the stream sends a reset, which surfaces to the app as a
            // refused connection rather than a hang.
            drop(stream);
            Ok(())
        }
        Decision::Direct => {
            tracing::debug!(source = %src, destination = %dest, "opening direct TCP connection");
            let mut upstream = match dial::tcp(dest, &shared.iface()).await {
                Ok(upstream) => upstream,
                Err(error) => {
                    tracing::warn!(
                        source = %src,
                        destination = %dest,
                        error = %error,
                        "direct TCP connect failed"
                    );
                    return Err(error);
                }
            };
            let (client_to_upstream, upstream_to_client) =
                tokio::io::copy_bidirectional(&mut stream, &mut upstream).await?;
            tracing::debug!(
                source = %src,
                destination = %dest,
                client_to_upstream,
                upstream_to_client,
                "direct TCP relay completed"
            );
            Ok(())
        }
        Decision::Proxy(config_id) => {
            let Some(cfg) = shared.find_proxy_config(config_id).cloned() else {
                shared.stats.proxy_fallbacks.fetch_add(1, Ordering::Relaxed);
                shared.stats.blocked.fetch_add(1, Ordering::Relaxed);
                shared.log(format!(
                    "proxy configuration {config_id} missing for {dest}; blocking proxied flow"
                ));
                fail_closed(stream).await;
                return Ok(());
            };

            // Prefer the snooped hostname: the MITM proxy needs a name to issue
            // a certificate for and to send as SNI upstream. Falling back to the
            // IP literal still works for plaintext HTTP.
            let target = match (cfg.send_domain_to_proxy, domain) {
                (true, Some(d)) => Target::Domain(d, dest.port()),
                _ => Target::Ip(dest.ip(), dest.port()),
            };

            tracing::debug!(
                source = %src,
                destination = %dest,
                config_id,
                proxy_type = ?cfg.proxy_type(),
                proxy_host = %cfg.host(),
                proxy_port = cfg.port(),
                target = ?target,
                "opening proxied TCP connection"
            );

            match proxy::connect_via_proxy(&cfg, &target).await {
                Ok(mut upstream) => {
                    tracing::debug!(
                        source = %src,
                        destination = %dest,
                        config_id,
                        target = ?target,
                        "proxy CONNECT established"
                    );
                    let (client_to_proxy, proxy_to_client) =
                        tokio::io::copy_bidirectional(&mut stream, &mut upstream).await?;
                    tracing::debug!(
                        source = %src,
                        destination = %dest,
                        config_id,
                        client_to_proxy,
                        proxy_to_client,
                        "proxied TCP relay completed"
                    );
                    Ok(())
                }
                Err(error) => {
                    shared.stats.proxy_fallbacks.fetch_add(1, Ordering::Relaxed);
                    shared.stats.blocked.fetch_add(1, Ordering::Relaxed);
                    shared.log(format!(
                        "proxy unavailable for {dest} ({error}); blocking proxied flow"
                    ));
                    fail_closed(stream).await;
                    Ok(())
                }
            }
        }
    }
}

/// Terminate a flow selected for proxying when its required proxy is absent.
/// Shutting down the intercepted side makes the failure observable to callers
/// without ever opening a direct socket to the destination.
async fn fail_closed<S>(mut stream: S)
where
    S: tokio::io::AsyncWrite + Unpin,
{
    let _ = stream.shutdown().await;
}

/// Relay one UDP flow, snooping DNS responses on the way back.
async fn handle_udp(mut stream: ipstack::IpStackUdpStream, shared: &Shared) -> std::io::Result<()> {
    let src = stream.local_addr();
    let dest = stream.peer_addr();

    let mut first_packet = vec![0u8; UDP_BUFFER];
    let first_len =
        match tokio::time::timeout(UDP_IDLE_TIMEOUT, stream.read(&mut first_packet)).await {
            Ok(result) => result?,
            Err(_) => return Ok(()),
        };

    let (pid, process_name) = attribute(src, dest, true);
    let domain = domain_for(shared, dest.ip());
    let facts = FlowFacts {
        dest,
        is_udp: true,
        pid,
        process_name: &process_name,
        domain: domain.as_deref(),
    };
    let decision = decide_udp_packet(shared, &facts, &first_packet[..first_len]);
    report(shared, &facts, decision);

    match decision {
        Decision::Block => {
            if dest.port() == shared.quic_port || is_quic_initial(&first_packet[..first_len]) {
                shared.stats.quic_dropped.fetch_add(1, Ordering::Relaxed);
            } else if is_group_address(dest.ip()) {
                shared.stats.group_dropped.fetch_add(1, Ordering::Relaxed);
            } else {
                shared.stats.blocked.fetch_add(1, Ordering::Relaxed);
            }
            drop(stream);
            return Ok(());
        }
        Decision::Direct | Decision::QuicProxy => {}
        // `decide_udp_packet` rejects UDP Proxy selections before relaying.
        Decision::Proxy(_) => unreachable!("UDP Proxy decisions must fail closed"),
    }

    let socket = if decision == Decision::QuicProxy {
        // Do not use the physical-interface dialer for the local proxy, and
        // never retry this flow against its original destination.
        let addr = shared
            .quic_proxy_addr
            .filter(|addr| addr.ip().is_loopback())
            .ok_or_else(|| std::io::Error::other("local QUIC proxy unavailable"))?;
        let bind_addr: SocketAddr = if addr.is_ipv4() {
            "127.0.0.1:0"
        } else {
            "[::1]:0"
        }
        .parse()
        .unwrap();
        let socket = tokio::net::UdpSocket::bind(bind_addr).await?;
        socket.connect(addr).await?;
        socket
    } else {
        dial::udp(dest, &shared.iface()).await?
    };
    let is_dns = dest.port() == 53 && decision != Decision::QuicProxy;
    let guard_direct_udp = decision == Decision::Direct
        && !dest.ip().is_loopback()
        && shared.quic_policy != QuicPolicy::Allow
        && (process_name.is_empty()
            || shared
                .protected_processes
                .iter()
                .any(|name| process_names_match(name, &process_name)));
    relay_udp_datagrams(
        stream,
        socket,
        &first_packet[..first_len],
        shared,
        is_dns,
        guard_direct_udp,
    )
    .await
}

/// Carry an already-decided UDP flow bidirectionally without opening any
/// other egress socket. The first payload was consumed to classify QUIC, so
/// forward it once, then relay subsequent datagrams and replies as usual.
async fn relay_udp_datagrams(
    mut stream: ipstack::IpStackUdpStream,
    socket: tokio::net::UdpSocket,
    first_packet: &[u8],
    shared: &Shared,
    is_dns: bool,
    guard_direct_udp: bool,
) -> std::io::Result<()> {
    let dest = stream.peer_addr();
    // A protected browser's direct flow must retain its original non-QUIC
    // protocol for its entire lifetime. Never use a single benign datagram
    // as permanent authorization to send QUIC to the origin.
    if !allow_direct_udp_followup(first_packet, is_dns, guard_direct_udp) {
        return Ok(());
    }
    // A redirected browser packet must never fall back to the origin if the
    // local QUIC proxy becomes unavailable. Direct UDP retains the existing
    // best-effort first-datagram semantics.
    let must_reach_local_proxy = socket.peer_addr()?.ip().is_loopback() && !dest.ip().is_loopback();
    if let Err(error) = socket.send(first_packet).await {
        shared.stats.udp_send_errors.fetch_add(1, Ordering::Relaxed);
        if must_reach_local_proxy {
            return Err(error);
        }
    }

    let mut from_client = vec![0u8; UDP_BUFFER];
    let mut from_server = vec![0u8; UDP_BUFFER];

    loop {
        tokio::select! {
            read = tokio::time::timeout(UDP_IDLE_TIMEOUT, stream.read(&mut from_client)) => {
                match read {
                    Err(_) => break,                       // idle timeout
                    Ok(Ok(0)) | Ok(Err(_)) => break,       // client closed
                    Ok(Ok(n)) => {
                        if !allow_direct_udp_followup(&from_client[..n], is_dns, guard_direct_udp) {
                            if is_dns {
                                shared.stats.blocked.fetch_add(1, Ordering::Relaxed);
                            } else {
                                shared.stats.quic_dropped.fetch_add(1, Ordering::Relaxed);
                            }
                            tracing::debug!("closing protected direct UDP flow after unexpected QUIC or invalid DNS");
                            break;
                        }
                        // A failed send loses one datagram; it must not end the
                        // flow. UDP is unreliable by contract, so the sender
                        // already has to cope with loss — whereas tearing the
                        // flow down turns a dropped packet into a broken
                        // connection. Windows returns WSAENOBUFS here whenever
                        // the socket's send queue is momentarily full, which is
                        // routine under load and entirely recoverable.
                        if let Err(error) = socket.send(&from_client[..n]).await {
                            shared.stats.udp_send_errors.fetch_add(1, Ordering::Relaxed);
                            tracing::debug!("dropping {n}-byte datagram to {dest}: {error}");
                        }
                    }
                }
            }
            recv = tokio::time::timeout(UDP_IDLE_TIMEOUT, socket.recv(&mut from_server)) => {
                match recv {
                    Err(_) => break,
                    // A receive error means this socket is done; unlike a send
                    // there is nothing to retry, so end the flow.
                    Ok(Err(_)) => break,
                    Ok(Ok(n)) => {
                        if is_dns {
                            // Build the IP -> hostname map that lets proxied
                            // flows be CONNECTed by name and lets domain rules
                            // match. This replaces the packet-layer DNS snoop
                            // the WinDivert build did.
                            shared.dns_cache.snoop_response(&from_server[..n]);
                        }
                        if stream.write_all(&from_server[..n]).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Emit the per-connection callback for a decided flow.
fn report(shared: &Shared, facts: &FlowFacts<'_>, decision: Decision) {
    if shared.conn_cb.is_none() {
        return;
    }
    let (action, proxy_info) = match decision {
        Decision::QuicProxy => (
            RuleAction::Proxy,
            format!("QUIC proxy {:?}", shared.quic_proxy_addr),
        ),
        Decision::Direct => (RuleAction::Direct, "Direct".to_string()),
        Decision::Block => (RuleAction::Block, "Blocked".to_string()),
        Decision::Proxy(id) => {
            let desc = shared
                .find_proxy_config(id)
                .map(|c| format!("Proxy {:?}://{}:{}", c.proxy_type(), c.host(), c.port()))
                .unwrap_or_else(|| "Proxy (unconfigured)".to_string());
            (RuleAction::Proxy, desc)
        }
    };

    shared.report_connection(&ConnectionInfo {
        process_name: facts.process_name.to_string(),
        pid: facts.pid.unwrap_or(0),
        dest_ip: facts.dest.ip(),
        dest_port: facts.dest.port(),
        action,
        proxy_info,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProxyConfig;
    use crate::rule::{Rule, RuleProtocol};
    use std::net::{Ipv4Addr, Ipv6Addr};

    fn dest(ip: &str, port: u16) -> SocketAddr {
        format!("{ip}:{port}").parse().unwrap()
    }

    fn facts<'a>(d: SocketAddr, is_udp: bool, pid: Option<u32>, name: &'a str) -> FlowFacts<'a> {
        FlowFacts {
            dest: d,
            is_udp,
            pid,
            process_name: name,
            domain: None,
        }
    }

    #[tokio::test]
    async fn unavailable_proxy_closes_intercepted_flow_instead_of_falling_back() {
        let (mut browser, intercepted) = tokio::io::duplex(64);
        fail_closed(intercepted).await;

        let mut byte = [0_u8; 1];
        assert_eq!(browser.read(&mut byte).await.unwrap(), 0);
    }

    /// QUIC relay must preserve the first datagram exactly once, and carry the
    /// proxy's response back to the original TUN-side client tuple. This test
    /// uses an in-memory ipstack UDP stream and a loopback echo endpoint; it
    /// never requires a physical adapter or reaches a remote network.
    #[tokio::test]
    async fn quic_udp_bridge_preserves_first_datagram_and_returns_response() {
        use tokio::net::UdpSocket;
        use tokio::sync::mpsc;
        let first = quic_initial(1);
        let udp_server = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let relay_socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        relay_socket
            .connect(udp_server.local_addr().unwrap())
            .await
            .unwrap();

        let (from_tun, mut back_to_tun) = mpsc::unbounded_channel();
        let mut tun_flow = ipstack::IpStackUdpStream::new(
            "10.63.7.1:55123".parse().unwrap(),
            "203.0.113.17:8443".parse().unwrap(),
            first.clone(),
            from_tun,
            1500,
            Duration::from_secs(10),
            None,
        );
        let mut first_read = vec![0; UDP_BUFFER];
        let len = tun_flow.read(&mut first_read).await.unwrap();
        assert_eq!(&first_read[..len], first);

        let shared = Shared::new_for_test(&[], vec![]);
        let task = tokio::spawn(async move {
            relay_udp_datagrams(
                tun_flow,
                relay_socket,
                &first_read[..len],
                &shared,
                false,
                false,
            )
            .await
        });

        let mut received = vec![0; UDP_BUFFER];
        let (length, peer) =
            tokio::time::timeout(Duration::from_secs(2), udp_server.recv_from(&mut received))
                .await
                .unwrap()
                .unwrap();
        assert_eq!(&received[..length], first);
        assert!(
            tokio::time::timeout(
                Duration::from_millis(80),
                udp_server.recv_from(&mut received)
            )
            .await
            .is_err(),
            "the first QUIC Initial was replayed more than once",
        );

        udp_server.send_to(b"quic response", peer).await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_secs(2), back_to_tun.recv())
                .await
                .unwrap()
                .is_some(),
            "local H3 server reply was not returned into the TUN UDP stream",
        );
        task.abort();
    }

    /// Browser TCP 443 must reach the proxy — the core happy path.
    #[test]
    fn browser_tcp_443_goes_to_proxy() {
        let rules = [
            Rule::new("chrome.exe;brave.exe")
                .ports("80;443")
                .protocol(RuleProtocol::Tcp)
                .action(RuleAction::Proxy),
            Rule::new("*").action(RuleAction::Direct),
        ];
        let mut cfg = ProxyConfig::http("127.0.0.1", 8080);
        cfg.config_id = 1;
        let shared = Shared::new_for_test(&rules, vec![cfg]);

        let d = dest("140.82.113.4", 443);
        assert!(matches!(
            decide(&shared, &facts(d, false, Some(999), "brave.exe")),
            Decision::Proxy(_)
        ));
    }

    /// A non-browser doing TCP 443 falls through to the catch-all: direct.
    #[test]
    fn non_browser_tcp_is_direct() {
        let rules = [
            Rule::new("chrome.exe")
                .ports("443")
                .protocol(RuleProtocol::Tcp)
                .action(RuleAction::Proxy),
            Rule::new("*").action(RuleAction::Direct),
        ];
        let shared = Shared::new_for_test(&rules, vec![]);
        let d = dest("140.82.113.4", 443);
        assert_eq!(
            decide(&shared, &facts(d, false, Some(999), "curl.exe")),
            Decision::Direct
        );
    }

    /// The regression that motivated this rewrite: a flow whose owning process
    /// cannot be identified must still reach the internet, not be dropped.
    #[test]
    fn unattributed_tcp_flow_fails_open() {
        let rules = [
            Rule::new("chrome.exe")
                .ports("443")
                .protocol(RuleProtocol::Tcp)
                .action(RuleAction::Proxy),
            Rule::new("*").action(RuleAction::Direct),
        ];
        let shared = Shared::new_for_test(&rules, vec![]);
        let d = dest("140.82.113.4", 443);
        assert_eq!(
            decide(&shared, &facts(d, false, None, "")),
            Decision::Direct
        );
    }

    /// With no rules at all, everything still flows.
    #[test]
    fn empty_ruleset_fails_open() {
        let shared = Shared::new_for_test(&[], vec![]);
        let d = dest("1.1.1.1", 443);
        assert_eq!(
            decide(&shared, &facts(d, false, None, "")),
            Decision::Direct
        );
    }

    /// The other half of the regression: QUIC must be dropped even when the
    /// process is unknown, which is exactly the case that leaked before.
    #[test]
    fn unattributed_quic_is_dropped() {
        let rules = [Rule::new("*").action(RuleAction::Direct)];
        let shared = Shared::new_for_test(&rules, vec![]);
        let d = dest("157.240.1.35", 443);
        assert_eq!(decide(&shared, &facts(d, true, None, "")), Decision::Block);
    }

    /// QUIC over IPv6 is dropped too — IPv6 was a total bypass before.
    #[test]
    fn quic_over_ipv6_is_dropped() {
        let rules = [Rule::new("*").action(RuleAction::Direct)];
        let shared = Shared::new_for_test(&rules, vec![]);
        let d = SocketAddr::new(
            IpAddr::V6(Ipv6Addr::new(0x2a03, 0x2880, 0xf10c, 0, 0, 0, 0, 0x35)),
            443,
        );
        assert_eq!(decide(&shared, &facts(d, true, None, "")), Decision::Block);
    }

    /// A wildcard Direct rule must not resurrect protected-browser QUIC.
    #[test]
    fn quic_drop_overrides_a_matching_direct_rule_for_protected_browser() {
        let rules = [Rule::new("*").action(RuleAction::Direct)];
        let mut shared = Shared::new_for_test(&rules, vec![]);
        shared.protected_processes = vec!["chrome.exe".to_string()];
        let d = dest("157.240.1.35", 443);
        assert_eq!(
            decide(&shared, &facts(d, true, Some(99), "chrome.exe")),
            Decision::Block
        );
    }

    // HTTP/3 security contract. These tests are intentionally added BEFORE the
    // QUIC router/bridge, so they fail against the original direct-UDP fallback.
    fn quic_initial(version: u32) -> Vec<u8> {
        let mut payload = vec![0_u8; 1200];
        payload[0] = 0xc0; // long header + fixed bit + v1 Initial type
        payload[1..5].copy_from_slice(&version.to_be_bytes());
        payload[5] = 8; // destination connection ID length
        payload[6..14].copy_from_slice(b"initial1");
        payload
    }

    fn protected_quic_shared() -> Shared {
        let rules = [Rule::new("*").action(RuleAction::Direct)];
        let mut shared = Shared::new_for_test(&rules, vec![]);
        shared.protected_processes = vec!["brave.exe".into()];
        shared
    }

    #[test]
    fn quic_on_alternative_ports_must_not_escape_even_when_unattributed() {
        let shared = protected_quic_shared();
        for d in [dest("203.0.113.17", 8443), dest("[2001:db8::123]", 10443)] {
            assert_eq!(
                decide_udp_packet(&shared, &facts(d, true, None, ""), &quic_initial(1)),
                Decision::Block,
                "unattributed QUIC on {d} cannot escape through the direct UDP fallback"
            );
        }
    }

    #[test]
    fn protected_browser_quic_on_an_alternative_port_must_not_go_direct() {
        let shared = protected_quic_shared();
        let d = dest("203.0.113.17", 8443);
        assert_eq!(
            decide_udp_packet(
                &shared,
                &facts(d, true, Some(71), "brave"),
                &quic_initial(1)
            ),
            Decision::Block
        );
    }

    #[test]
    fn short_non_quic_udp_remains_compatible_on_other_ports() {
        let shared = protected_quic_shared();
        assert_eq!(
            decide_udp_packet(
                &shared,
                &facts(dest("203.0.113.17", 8443), true, Some(71), "brave"),
                b"ordinary UDP"
            ),
            Decision::Direct,
        );
    }

    // DNS transaction IDs are random and may begin with QUIC's fixed bit.
    // Normal DNS/53 (including EDNS) must never be classified as migrated QUIC.
    #[test]
    fn dns_query_with_quic_like_transaction_id_remains_direct() {
        let shared = protected_quic_shared();
        let mut dns = vec![0x43, 0x01, 0x01, 0x00, 0, 1, 0, 0, 0, 0, 0, 1];
        dns.extend_from_slice(b"\x07example\x03com\x00");
        dns.extend_from_slice(&[0, 1, 0, 1]);
        dns.extend_from_slice(&[0, 0, 41, 16, 0, 0, 0, 0, 0, 0, 0]);
        for (pid, name) in [(Some(71), "brave"), (None, "")] {
            assert_eq!(
                decide_udp_packet(&shared, &facts(dest("1.1.1.1", 53), true, pid, name), &dns,),
                Decision::Direct,
                "DNS query with a QUIC-like transaction ID must not be blocked",
            );
        }
    }

    #[test]
    fn quic_version_negotiation_must_not_be_mistaken_for_a_client_initial() {
        let shared = protected_quic_shared();
        assert_eq!(
            decide_udp_packet(
                &shared,
                &facts(dest("203.0.113.17", 8443), true, Some(71), "brave"),
                &quic_initial(0)
            ),
            Decision::Direct,
        );
    }

    #[test]
    fn known_non_browser_quic_is_not_intercepted() {
        let shared = protected_quic_shared();
        assert_eq!(
            decide_udp_packet(
                &shared,
                &facts(dest("203.0.113.17", 8443), true, Some(100), "wireguard"),
                &quic_initial(1)
            ),
            Decision::Direct,
        );
    }

    #[test]
    fn enabled_h3_routing_targets_the_local_proxy_for_ipv4_and_ipv6() {
        let mut shared = protected_quic_shared();
        shared.quic_policy = QuicPolicy::Intercept;
        shared.quic_proxy_addr = Some("127.0.0.1:9443".parse().unwrap());
        for d in [
            dest("203.0.113.17", 443),
            dest("203.0.113.17", 8443),
            dest("[2001:db8::123]", 443),
            dest("[2001:db8::123]", 8443),
        ] {
            assert_eq!(
                decide_udp_packet(
                    &shared,
                    &facts(d, true, Some(71), "brave"),
                    &quic_initial(1)
                ),
                Decision::QuicProxy,
                "protected QUIC to {d} must use the in-process MITM endpoint"
            );
        }
    }

    #[test]
    fn interception_without_a_local_h3_listener_fails_closed() {
        let mut shared = protected_quic_shared();
        shared.quic_policy = QuicPolicy::Intercept;
        shared.quic_proxy_addr = None;
        assert_eq!(
            decide_udp_packet(
                &shared,
                &facts(dest("203.0.113.17", 443), true, Some(71), "brave"),
                &quic_initial(1)
            ),
            Decision::Block,
        );
    }

    #[test]
    fn unattributed_quic_stays_blocked_even_with_h3_interception_enabled() {
        let mut shared = protected_quic_shared();
        shared.quic_policy = QuicPolicy::Intercept;
        shared.quic_proxy_addr = Some("127.0.0.1:9443".parse().unwrap());
        assert_eq!(
            decide_udp_packet(
                &shared,
                &facts(dest("203.0.113.17", 8443), true, None, ""),
                &quic_initial(1)
            ),
            Decision::Block,
        );
    }

    #[test]
    fn explicit_block_rules_override_h3_interception() {
        let rules = [
            Rule::new("brave.exe")
                .ports("8443")
                .protocol(RuleProtocol::Udp)
                .action(RuleAction::Block),
            Rule::new("*").action(RuleAction::Direct),
        ];
        let mut shared = Shared::new_for_test(&rules, vec![]);
        shared.protected_processes = vec!["brave.exe".into()];
        shared.quic_policy = QuicPolicy::Intercept;
        shared.quic_proxy_addr = Some("127.0.0.1:9443".parse().unwrap());
        assert_eq!(
            decide_udp_packet(
                &shared,
                &facts(dest("203.0.113.17", 8443), true, Some(71), "brave"),
                &quic_initial(1)
            ),
            Decision::Block,
        );
    }

    // Independent-security-review regression: an already-opened or migrated
    // QUIC flow can begin with a short header, not a new Initial. It must not
    // be relayed to the origin when browser H3 interception is enabled.
    #[test]
    fn h3_intercept_rejects_non_initial_udp443_from_a_browser() {
        let mut shared = protected_quic_shared();
        shared.quic_policy = QuicPolicy::Intercept;
        shared.quic_proxy_addr = Some("127.0.0.1:9443".parse().unwrap());
        let short_header = [0x43_u8; 64];
        for process in ["brave", ""] {
            assert_eq!(
                decide_udp_packet(
                    &shared,
                    &facts(dest("203.0.113.17", 443), true, Some(71), process),
                    &short_header
                ),
                Decision::Block,
                "{process:?} must not use origin-direct UDP on port 443",
            );
        }
    }

    #[test]
    fn migrated_quic_short_header_on_alternative_port_cannot_escape() {
        let mut shared = protected_quic_shared();
        shared.quic_policy = QuicPolicy::Intercept;
        shared.quic_proxy_addr = Some("127.0.0.1:9443".parse().unwrap());
        let short_header = [0x43_u8; 64];
        for process in ["brave", ""] {
            assert_eq!(
                decide_udp_packet(
                    &shared,
                    &facts(dest("203.0.113.17", 8443), true, Some(71), process),
                    &short_header
                ),
                Decision::Block,
                "{process:?} must not relay a migrated connection directly",
            );
        }
        assert_eq!(
            decide_udp_packet(
                &shared,
                &facts(dest("203.0.113.17", 8443), true, Some(99), "wireguard"),
                &short_header
            ),
            Decision::Direct,
            "unprotected applications retain opaque UDP",
        );
    }

    #[test]
    fn migrated_quic_non_initial_long_header_on_an_alternative_port_is_not_direct() {
        let mut shared = protected_quic_shared();
        shared.quic_policy = QuicPolicy::Intercept;
        shared.quic_proxy_addr = Some("127.0.0.1:9443".parse().unwrap());
        // QUIC v1 0-RTT or Handshake packets carry a long header with a
        // nonzero version, and can arrive before the first captured Initial.
        let mut packet = quic_initial(1);
        packet[0] = 0xd0;
        assert_eq!(
            decide_udp_packet(
                &shared,
                &facts(dest("203.0.113.17", 8443), true, Some(71), "brave"),
                &packet
            ),
            Decision::Block,
        );
    }

    #[test]
    fn protected_direct_udp_flow_rejects_later_quic_without_a_new_flow_decision() {
        let initial = quic_initial(1);
        let mut short_header = vec![0x43_u8; 64];
        let normal_udp = b"ordinary UDP datagram";
        assert!(allow_direct_udp_followup(normal_udp, false, true));
        assert!(!allow_direct_udp_followup(&initial, false, true));
        assert!(!allow_direct_udp_followup(&short_header, false, true));
        // Non-protected programs retain their own opaque UDP.
        assert!(allow_direct_udp_followup(&initial, false, false));
        short_header[0] = 0xd0;
        short_header[1..5].copy_from_slice(&1_u32.to_be_bytes());
        assert!(!allow_direct_udp_followup(&short_header, false, true));
    }

    #[test]
    fn explicit_udp_proxy_rule_fails_closed_if_no_udp_capable_proxy_selected() {
        let rules = [
            Rule::new("brave.exe")
                .ports("5354")
                .protocol(RuleProtocol::Udp)
                .action(RuleAction::Proxy),
            Rule::new("*").action(RuleAction::Direct),
        ];
        let shared = Shared::new_for_test(&rules, vec![]);
        assert_eq!(
            decide_udp_packet(
                &shared,
                &facts(dest("203.0.113.17", 5354), true, Some(71), "brave"),
                b"udp protocol data"
            ),
            Decision::Block,
            "an unsupported UDP Proxy rule may not silently relay to the origin",
        );
    }

    #[test]
    fn known_non_protected_process_can_use_udp_443() {
        let rules = [Rule::new("*").action(RuleAction::Direct)];
        let mut shared = Shared::new_for_test(&rules, vec![]);
        shared.protected_processes = vec!["chrome.exe".to_string()];
        let d = dest("203.0.113.20", 443);
        assert_eq!(
            decide(&shared, &facts(d, true, Some(100), "wireguard.exe")),
            Decision::Direct
        );
    }

    /// mDNS, SSDP and LLMNR are dropped rather than relayed. Relaying them
    /// burned a pinned unicast socket per packet, which is what exhausted
    /// Windows' socket buffers (WSAENOBUFS) on a busy LAN.
    #[test]
    fn multicast_destinations_are_dropped() {
        let rules = [Rule::new("*").action(RuleAction::Direct)];
        let shared = Shared::new_for_test(&rules, vec![]);

        for (ip, port) in [
            ("224.0.0.251", 5353u16),  // mDNS
            ("239.255.255.250", 1900), // SSDP
            ("224.0.0.252", 5355),     // LLMNR
        ] {
            assert_eq!(
                decide(&shared, &facts(dest(ip, port), true, Some(9), "")),
                Decision::Block,
                "{ip}:{port} should be dropped"
            );
        }
    }

    /// Broadcast (DHCP, NetBIOS) is dropped for the same reason.
    #[test]
    fn broadcast_destinations_are_dropped() {
        let rules = [Rule::new("*").action(RuleAction::Direct)];
        let shared = Shared::new_for_test(&rules, vec![]);
        assert_eq!(
            decide(
                &shared,
                &facts(dest("255.255.255.255", 67), true, Some(9), "")
            ),
            Decision::Block
        );
    }

    /// IPv6 multicast (ff00::/8) too — neighbour discovery, mDNS over v6.
    #[test]
    fn ipv6_multicast_is_dropped() {
        let rules = [Rule::new("*").action(RuleAction::Direct)];
        let shared = Shared::new_for_test(&rules, vec![]);
        let d = SocketAddr::new(
            IpAddr::V6(Ipv6Addr::new(0xff02, 0, 0, 0, 0, 0, 0, 0xfb)),
            5353,
        );
        assert_eq!(
            decide(&shared, &facts(d, true, Some(9), "")),
            Decision::Block
        );
    }

    /// Ordinary unicast must not be caught by the group-address check — that
    /// would black-hole normal traffic.
    #[test]
    fn unicast_is_not_treated_as_a_group_address() {
        assert!(!is_group_address("140.82.113.4".parse().unwrap()));
        assert!(!is_group_address("8.8.8.8".parse().unwrap()));
        assert!(!is_group_address("2606:50c0:8000::153".parse().unwrap()));
        // 223.x is the last unicast /8 before the 224.0.0.0/4 multicast range.
        assert!(!is_group_address("223.255.255.255".parse().unwrap()));
        assert!(is_group_address("224.0.0.0".parse().unwrap()));
    }

    /// Non-QUIC UDP (e.g. DNS) is unaffected by the QUIC policy.
    #[test]
    fn dns_udp_is_not_dropped() {
        let rules = [Rule::new("*").action(RuleAction::Direct)];
        let shared = Shared::new_for_test(&rules, vec![]);
        let d = dest("1.1.1.1", 53);
        assert_eq!(decide(&shared, &facts(d, true, None, "")), Decision::Direct);
    }

    /// Allowing QUIC must actually let it through, so the policy is real.
    #[test]
    fn quic_allow_policy_lets_it_through() {
        let rules = [Rule::new("*").action(RuleAction::Direct)];
        let mut shared = Shared::new_for_test(&rules, vec![]);
        shared.quic_policy = QuicPolicy::Allow;
        let d = dest("157.240.1.35", 443);
        assert_eq!(decide(&shared, &facts(d, true, None, "")), Decision::Direct);
    }

    /// The loop guard: our own traffic is never sent back to the proxy, even
    /// when a rule says every process should be proxied.
    #[test]
    fn own_process_always_goes_direct() {
        let rules = [Rule::new("*")
            .protocol(RuleProtocol::Tcp)
            .action(RuleAction::Proxy)];
        let mut cfg = ProxyConfig::http("127.0.0.1", 8080);
        cfg.config_id = 1;
        let shared = Shared::new_for_test(&rules, vec![cfg]);

        let me = shared.current_pid;
        let d = dest("140.82.113.4", 443);
        assert_eq!(
            decide(&shared, &facts(d, false, Some(me), "ace-engine.exe")),
            Decision::Direct
        );
    }

    /// Our own process is exempt from Block rules as well, or a VPN-process
    /// rule matching our name would sever the engine from the internet.
    #[test]
    fn own_process_bypasses_block_rules() {
        let rules = [Rule::new("*").action(RuleAction::Block)];
        let shared = Shared::new_for_test(&rules, vec![]);
        let me = shared.current_pid;
        let d = dest("140.82.113.4", 443);
        assert_eq!(
            decide(&shared, &facts(d, false, Some(me), "")),
            Decision::Direct
        );
    }

    /// Even our own QUIC goes direct — the loop guard runs before the QUIC drop.
    #[test]
    fn own_process_quic_is_not_dropped() {
        let shared = Shared::new_for_test(&[], vec![]);
        let me = shared.current_pid;
        let d = dest("1.1.1.1", 443);
        assert_eq!(
            decide(&shared, &facts(d, true, Some(me), "")),
            Decision::Direct
        );
    }

    /// With an interface to pin to, our own traffic is relayed directly —
    /// the normal case, and the counterpart to the drop test below.
    #[test]
    fn own_process_goes_direct_when_an_interface_exists() {
        let shared = Shared::new_for_test(&[], vec![]);
        let me = shared.current_pid;
        let d = dest("140.82.113.4", 443);
        assert_eq!(
            decide(&shared, &facts(d, false, Some(me), "")),
            Decision::Direct
        );
    }

    /// Without one, relaying our own flow would loop forever, so it is dropped.
    #[test]
    fn own_process_is_dropped_when_there_is_nothing_to_pin_to() {
        let shared = Shared::new_for_test_without_interface(&[], vec![]);
        let me = shared.current_pid;
        let d = dest("140.82.113.4", 443);
        assert_eq!(
            decide(&shared, &facts(d, false, Some(me), "")),
            Decision::Block
        );
    }

    /// Our own loopback traffic (e.g. reaching the local proxy) is never at
    /// risk of looping, so it is relayed even with no interface known.
    #[test]
    fn own_process_loopback_is_direct_without_an_interface() {
        let shared = Shared::new_for_test_without_interface(&[], vec![]);
        let me = shared.current_pid;
        let d = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);
        assert_eq!(
            decide(&shared, &facts(d, false, Some(me), "")),
            Decision::Direct
        );
    }

    /// Loopback stays local regardless of rules.
    #[test]
    fn loopback_is_direct() {
        let rules = [Rule::new("*")
            .protocol(RuleProtocol::Tcp)
            .action(RuleAction::Proxy)];
        let mut cfg = ProxyConfig::http("127.0.0.1", 8080);
        cfg.config_id = 1;
        let shared = Shared::new_for_test(&rules, vec![cfg]);
        let d = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);
        assert_eq!(
            decide(&shared, &facts(d, false, Some(1), "chrome.exe")),
            Decision::Direct
        );
    }

    /// Explicit process block rules remain transport-agnostic.
    #[test]
    fn explicit_process_block_rule_still_works() {
        let rules = [
            Rule::new("blocked-helper.exe").action(RuleAction::Block),
            Rule::new("*").action(RuleAction::Direct),
        ];
        let shared = Shared::new_for_test(&rules, vec![]);
        let d = dest("203.0.113.9", 8443);
        assert_eq!(
            decide(&shared, &facts(d, false, Some(42), "blocked-helper.exe")),
            Decision::Block
        );
        assert_eq!(
            decide(&shared, &facts(d, true, Some(42), "blocked-helper.exe")),
            Decision::Block
        );
    }

    /// Domain rules still work, now fed by the UDP-flow DNS snoop.
    #[test]
    fn domain_rule_blocks_by_snooped_hostname() {
        let rules = [
            Rule::new("*")
                .domains("*.protonvpn.com")
                .action(RuleAction::Block),
            Rule::new("*").action(RuleAction::Direct),
        ];
        let shared = Shared::new_for_test(&rules, vec![]);
        let d = dest("185.159.157.1", 443);

        let mut f = facts(d, false, Some(7), "chrome.exe");
        f.domain = Some("api.protonvpn.com");
        assert_eq!(decide(&shared, &f), Decision::Block);

        let mut f2 = facts(d, false, Some(7), "chrome.exe");
        f2.domain = Some("github.com");
        assert_eq!(decide(&shared, &f2), Decision::Direct);
    }

    /// A `*.proton.me` wildcard matches every Proton *service* subdomain
    /// (mail/calendar/drive/pass/account), not just VPN infrastructure —
    /// documented so nobody reintroduces the overbroad rule that broke
    /// unrelated Proton services (Issue 7). With only `*.protonvpn.com`
    /// blocked, `mail.proton.me` must be Direct.
    #[test]
    fn proton_shared_services_are_not_blocked_by_vpn_domain_rule() {
        let rules = [
            Rule::new("*")
                .domains("*.protonvpn.com")
                .action(RuleAction::Block),
            Rule::new("*").action(RuleAction::Direct),
        ];
        let shared = Shared::new_for_test(&rules, vec![]);
        let d = dest("185.159.157.1", 443);

        // A non-VPN Proton service on the shared identity domain stays
        // reachable — the exact collateral damage the old rule caused.
        let mut mail = facts(d, false, Some(7), "chrome.exe");
        mail.domain = Some("mail.proton.me");
        assert_eq!(decide(&shared, &mail), Decision::Direct);
        let mut pass = facts(d, false, Some(7), "chrome.exe");
        pass.domain = Some("pass.proton.me");
        assert_eq!(decide(&shared, &pass), Decision::Direct);

        // The VPN-specific domain is still blocked for every process.
        let mut vpn = facts(d, false, Some(7), "chrome.exe");
        vpn.domain = Some("api.protonvpn.com");
        assert_eq!(decide(&shared, &vpn), Decision::Block);
    }

    /// IPv6 browser traffic is proxied exactly like IPv4 — under WinDivert it
    /// bypassed interception entirely.
    #[test]
    fn ipv6_browser_traffic_is_proxied() {
        let rules = [
            Rule::new("chrome.exe")
                .ports("443")
                .protocol(RuleProtocol::Tcp)
                .action(RuleAction::Proxy),
            Rule::new("*").action(RuleAction::Direct),
        ];
        let mut cfg = ProxyConfig::http("127.0.0.1", 8080);
        cfg.config_id = 1;
        let shared = Shared::new_for_test(&rules, vec![cfg]);

        let d = SocketAddr::new(
            IpAddr::V6(Ipv6Addr::new(0x2606, 0x50c0, 0x8000, 0, 0, 0, 0, 0x153)),
            443,
        );
        assert!(matches!(
            decide(&shared, &facts(d, false, Some(3), "chrome.exe")),
            Decision::Proxy(_)
        ));
    }
}
