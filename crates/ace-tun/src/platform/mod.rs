//! Per-OS backend: adapter lifecycle, addressing, socket pinning, attribution.
//!
//! Everything that touches the OS lives behind this module. The rest of the
//! crate (`lib.rs`, `netstack.rs`, `device.rs`, …) talks only to the seams
//! defined here:
//!
//! * [`Backend`] — create the adapter and install addresses/routes,
//!   discover the physical interface, and report whether the process has the
//!   privileges the adapter needs ([`Error::NotElevated`] contract).
//! * [`AdapterHandle`] — the per-OS adapter object's surface: a session for
//!   the async device, session shutdown, and route removal.
//! * [`ProcessTable`] — per-OS process attribution (`resolve_pid` /
//!   `process_name`).
//! * `pin_socket` / `set_send_buffer` — per-OS socket options for the loop
//!   guard, re-exported by [`crate::dial`].
//!
//! Exactly one OS implementation is compiled, selected by `cfg` below.
//!
//! # Relay escape policy
//!
//! ACE relay sockets must escape the application's capture path without opening
//! a route-level bypass for the application itself. The mechanism differs by OS:
//!
//! | OS | Relay escape | Enforcement boundary |
//! |---|---|---|
//! | Windows | current best non-ACE interface via `IP_UNICAST_IF` / `IPV6_UNICAST_IF` | app-scoped WFP ALE filters |
//! | Linux | `SO_MARK` + inverted RPDB rule | dedicated routing table + nftables OUTPUT guard |
//! | macOS | cached physical interface via `IP_BOUND_IF` / `IPV6_BOUND_IF` | utun routing (unchanged) |
//!
//! Windows still has its byte-order asymmetry: IPv4 takes the interface index in
//! network order while IPv6 takes host order. The per-OS implementation tests
//! this explicitly.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use crate::error::{Error, Result};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub(crate) use windows::process::reset_preexisting_browser_tcp;

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
compile_error!("ace-tun currently supports Windows, Linux, and macOS only");

/// Address assigned to our end of the tunnel. Any traffic we pull in appears to
/// originate here. Chosen from RFC 1918 space that is unusual enough to be
/// unlikely to collide with a corporate LAN or a Docker bridge.
pub const TUN_IPV4: Ipv4Addr = Ipv4Addr::new(10, 63, 7, 1);
/// Prefix length for [`TUN_IPV4`].
pub const TUN_IPV4_PREFIX: u8 = 24;

/// IPv6 counterpart of [`TUN_IPV4`], from the unique-local range (RFC 4193).
pub const TUN_IPV6: Ipv6Addr = Ipv6Addr::new(0xfd00, 0x0ace, 0x0007, 0, 0, 0, 0, 1);
/// Prefix length for [`TUN_IPV6`].
pub const TUN_IPV6_PREFIX: u8 = 64;

/// MTU for the tunnel. The netstack terminates every flow and re-originates it
/// on a relay socket that escapes ACE capture through the platform policy, so
/// this value only governs the local application-to-tunnel segment and never
/// causes on-the-wire fragmentation.
pub const TUN_MTU: u16 = 1500;

/// The two IPv4 prefixes used by the Windows/macOS capture strategy. Linux
/// uses a default route in its dedicated ACE policy-routing table instead.
#[cfg_attr(target_os = "linux", allow(dead_code))]
pub const V4_SPLIT_DEFAULT: [(Ipv4Addr, u8); 2] = [
    (Ipv4Addr::new(0, 0, 0, 0), 1),
    (Ipv4Addr::new(128, 0, 0, 0), 1),
];

/// The IPv6 equivalent of [`V4_SPLIT_DEFAULT`].
#[cfg_attr(target_os = "linux", allow(dead_code))]
pub const V6_SPLIT_DEFAULT: [(Ipv6Addr, u8); 2] = [
    (Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0), 1),
    (Ipv6Addr::new(0x8000, 0, 0, 0, 0, 0, 0, 0), 1),
];

/// Connectivity discovered before ACE installs its capture policy.
///
/// The snapshot is used to decide which address families are viable and remains
/// the macOS relay target. Windows relay sockets re-evaluate the current route
/// per connection; Linux relay sockets use `SO_MARK` and the normal RPDB.
#[derive(Debug, Clone, Copy, Default)]
pub struct PhysicalInterface {
    /// Interface index for IPv4 traffic, if a v4 default route exists.
    pub v4_index: Option<u32>,
    /// Interface index for IPv6 traffic, if a v6 default route exists.
    pub v6_index: Option<u32>,
}

impl PhysicalInterface {
    /// Discover the current internet-facing interface for both families.
    ///
    /// A family with no route simply yields `None`; that is not an error,
    /// it just means the machine has no connectivity of that kind.
    pub fn discover() -> Self {
        BackendImpl::discover_physical_interface()
    }

    /// The interface index to pin an outbound socket to for `dest`.
    pub fn index_for(&self, dest: IpAddr) -> Option<u32> {
        match dest {
            IpAddr::V4(_) => self.v4_index,
            IpAddr::V6(_) => self.v6_index,
        }
    }

    /// Whether any usable outbound interface was found.
    pub fn is_empty(&self) -> bool {
        self.v4_index.is_none() && self.v6_index.is_none()
    }
}

/// A live adapter handle: session for the async device, teardown.
pub(crate) trait AdapterHandle {
    /// A handle to the packet device, for building the async device.
    fn session(&self) -> std::io::Result<crate::device::SessionHandle>;
    /// Stop the session, unblocking the device reader (Windows), or no-op where
    /// the device is a plain fd (Unix).
    fn shutdown_session(&self);
    /// Remove the routes we installed.
    ///
    /// Called before the adapter is dropped so connectivity is restored in the
    /// right order. Failures are logged, not propagated: by the time teardown
    /// runs, a missing route is the outcome we wanted anyway.
    fn remove_routes(&mut self);
}

/// Per-OS adapter lifecycle and privilege contract.
///
/// [`Backend::create`] is the one entry point the engine uses. It maps the
/// per-OS privilege predicate onto [`Error::NotElevated`] so callers can
/// degrade gracefully instead of failing obscurely.
pub(crate) trait Backend {
    type Adapter: AdapterHandle;

    /// Create the adapter, assign addresses, and install routes.
    ///
    /// `iface` is the connectivity snapshot discovered just before this call.
    /// Backends may use it for family availability or legacy interface pinning.
    /// Returns [`Error::NotElevated`] when the process lacks the privileges the
    /// current OS requires, leaving the machine's networking untouched.
    fn create(
        ipv6: bool,
        iface: &PhysicalInterface,
        protected_processes: &[String],
    ) -> Result<Self::Adapter> {
        if !Self::is_privileged() {
            return Err(Error::NotElevated);
        }
        Self::create_privileged(ipv6, iface, protected_processes)
    }

    /// OS-specific creation, called only when [`Backend::is_privileged`] held.
    fn create_privileged(
        ipv6: bool,
        iface: &PhysicalInterface,
        protected_processes: &[String],
    ) -> Result<Self::Adapter>;

    /// Discover the internet-facing interface for both families, before our
    /// own routes exist.
    fn discover_physical_interface() -> PhysicalInterface;

    /// Whether the current process has the privileges adapter creation needs.
    fn is_privileged() -> bool;
}

/// Per-OS process attribution: map a flow's endpoints to a PID, and a PID to
/// an executable name.
pub(crate) trait ProcessTable {
    /// Resolve the PID that owns the flow's local endpoint, or `None`.
    ///
    /// `remote` is the flow's destination — the owning socket's peer. Some
    /// platforms need it: the Linux kernel hashes established TCP sockets by
    /// the full four-tuple, so a local-endpoint-only query cannot find them.
    fn resolve_pid(local: SocketAddr, remote: SocketAddr, is_udp: bool) -> Option<u32>;
    /// Resolve the executable *file name* for a PID, e.g. `chrome.exe`.
    fn process_name(pid: u32) -> Option<String>;
}

// ── OS selection ────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
pub use self::windows::WindowsBackend as BackendImpl;
#[cfg(target_os = "windows")]
pub use self::windows::WindowsProcessTable as ProcessTableImpl;
#[cfg(target_os = "windows")]
pub use self::windows::adapter::ADAPTER_NAME;
#[cfg(target_os = "windows")]
pub(crate) use self::windows::dial::{pin_socket, set_send_buffer};
#[cfg(target_os = "windows")]
pub(crate) use self::windows::icmp::relay_echo as relay_icmp_echo;

#[cfg(target_os = "linux")]
pub use self::linux::LinuxBackend as BackendImpl;
#[cfg(target_os = "linux")]
pub use self::linux::LinuxProcessTable as ProcessTableImpl;
#[cfg(target_os = "linux")]
pub use self::linux::adapter::ADAPTER_NAME;
#[cfg(target_os = "linux")]
pub(crate) use self::linux::dial::{pin_socket, set_send_buffer};

#[cfg(target_os = "macos")]
pub use self::macos::MacosBackend as BackendImpl;
#[cfg(target_os = "macos")]
pub use self::macos::MacosProcessTable as ProcessTableImpl;
#[cfg(target_os = "macos")]
pub use self::macos::adapter::ADAPTER_NAME;
#[cfg(target_os = "macos")]
pub(crate) use self::macos::dial::{pin_socket, set_send_buffer};

/// The concrete adapter type of the compiled platform.
pub(crate) type TunAdapter = <BackendImpl as Backend>::Adapter;

/// Resolve the PID that owns the given local endpoint. See [`ProcessTable`].
pub(crate) fn resolve_pid(local: SocketAddr, remote: SocketAddr, is_udp: bool) -> Option<u32> {
    ProcessTableImpl::resolve_pid(local, remote, is_udp)
}

/// Resolve the executable file name for a PID. See [`ProcessTable`].
pub(crate) fn process_name(pid: u32) -> Option<String> {
    ProcessTableImpl::process_name(pid)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tunnel addresses must not fall inside the loopback or link-local
    /// ranges, which are handled specially by the OS and would never route.
    #[test]
    fn tunnel_addresses_are_routable_private_space() {
        assert!(TUN_IPV4.is_private());
        assert!(!TUN_IPV4.is_loopback());
        assert!(!TUN_IPV4.is_link_local());
        // fd00::/8 is unique-local.
        assert_eq!(TUN_IPV6.octets()[0], 0xfd);
    }

    /// The device read buffer is sized from the same constant the adapter is
    /// configured with; if that ever drifts, framing breaks silently.
    #[test]
    fn mtu_is_within_ipstack_limits() {
        const { assert!(TUN_MTU >= 1280, "below IPv6 minimum MTU") };
    }

    /// The split-default prefixes must cover the entire address space, or some
    /// traffic would silently keep using the physical default route.
    #[test]
    fn split_default_covers_whole_v4_space() {
        let [(a, alen), (b, blen)] = V4_SPLIT_DEFAULT;
        assert_eq!((alen, blen), (1, 1));
        assert_eq!(a.octets()[0] >> 7, 0);
        assert_eq!(b.octets()[0] >> 7, 1);
    }

    #[test]
    fn split_default_covers_whole_v6_space() {
        let [(a, alen), (b, blen)] = V6_SPLIT_DEFAULT;
        assert_eq!((alen, blen), (1, 1));
        assert_eq!(a.octets()[0] >> 7, 0);
        assert_eq!(b.octets()[0] >> 7, 1);
    }

    /// Interface discovery must not panic or hang on a machine in any state;
    /// it is allowed to find nothing (e.g. no IPv6 connectivity).
    #[test]
    fn discover_physical_interface_is_infallible() {
        let iface = PhysicalInterface::discover();
        if let Some(idx) = iface.v4_index {
            assert_ne!(idx, 0, "a discovered interface index is never 0");
        }
    }
}
