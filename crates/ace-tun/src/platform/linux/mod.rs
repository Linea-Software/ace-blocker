//! Linux backend: `/dev/net/tun`, rtnetlink policy routing, `SO_MARK` relay
//! escape, an nftables OUTPUT guard, and sock_diag-based process attribution.
//!
//! Ordinary local traffic selects ACE's dedicated capture table. Relay sockets
//! carry `ACE_EGRESS_MARK`, so they skip only that table and continue through the
//! normal RPDB, including user VPN policy. The nftables guard independently
//! blocks unmarked local egress while ACE is healthy.
//!
//! Implemented in Phase 1 of the cross-platform migration; see
//! `docs/cross-platform-report.md`.

pub(crate) mod adapter;
pub(crate) mod dial;
pub(crate) mod firewall;
pub(crate) mod netcfg;
pub(crate) mod process;

/// Mark placed on sockets opened by ace-tun itself.
///
/// The policy-routing capture rule is inverted against this mark, so these
/// sockets skip only ACE's capture table and continue through the machine's
/// ordinary RPDB. That preserves user VPN policy routing instead of forcing
/// relayed traffic directly onto a physical NIC.
pub(crate) const ACE_EGRESS_MARK: u32 = 0x0ACE_0001;

/// Dedicated routing table used only for local application traffic.
pub(crate) const ACE_CAPTURE_TABLE: u32 = 20_607;

/// RPDB priority for the capture rule. Linux evaluates lower numbers first;
/// the built-in local rule stays ahead of us at priority 0.
pub(crate) const ACE_CAPTURE_PRIORITY: u32 = 10;

use crate::error::Result;
use crate::platform::{Backend, PhysicalInterface, ProcessTable};

/// The Linux implementation of the [`Backend`] seam.
pub struct LinuxBackend;

/// The Linux implementation of the [`ProcessTable`] seam.
pub struct LinuxProcessTable;

impl Backend for LinuxBackend {
    type Adapter = adapter::TunAdapter;

    fn create_privileged(
        ipv6: bool,
        iface: &PhysicalInterface,
        _protected_processes: &[String],
    ) -> Result<Self::Adapter> {
        adapter::TunAdapter::create(ipv6, iface)
    }

    fn discover_physical_interface() -> PhysicalInterface {
        netcfg::discover_physical_interface()
    }

    fn is_privileged() -> bool {
        process::is_privileged()
    }
}

impl ProcessTable for LinuxProcessTable {
    fn resolve_pid(
        local: std::net::SocketAddr,
        remote: std::net::SocketAddr,
        is_udp: bool,
    ) -> Option<u32> {
        process::resolve_pid(local, remote, is_udp)
    }

    fn process_name(pid: u32) -> Option<String> {
        process::process_name(pid)
    }
}
