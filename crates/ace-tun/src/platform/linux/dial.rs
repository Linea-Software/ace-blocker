//! Linux relay-socket escape hatch: SO_MARK + policy routing.
//!
//! Unmarked application traffic is selected into ACE's dedicated capture
//! table. Relay sockets opened by ACE carry the ACE_EGRESS_MARK socket mark,
//! and the capture rule is inverted against that mark. Marked traffic therefore
//! continues through the normal Linux RPDB, preserving NetworkManager and user
//! VPN policy routing instead of forcing traffic directly onto one physical NIC.

use std::io;
use std::net::IpAddr;

use super::ACE_EGRESS_MARK;

/// Mark an outbound socket so ACE's capture rule ignores it.
///
/// The interface index remains in the cross-platform seam but is intentionally
/// unused on Linux. Choosing the current egress path belongs to the kernel's
/// routing-policy database, including any user-selected VPN.
pub(crate) fn pin_socket(raw: i32, _index: u32, dest: IpAddr) -> io::Result<()> {
    if dest.is_loopback() {
        return Ok(());
    }
    let mark = ACE_EGRESS_MARK as libc::c_uint;
    // SAFETY: `raw` is a live socket owned by the caller and `mark` is the
    // native integer payload required by SO_MARK.
    let rc = unsafe {
        libc::setsockopt(
            raw,
            libc::SOL_SOCKET,
            libc::SO_MARK,
            &mark as *const libc::c_uint as *const libc::c_void,
            std::mem::size_of::<libc::c_uint>() as libc::socklen_t,
        )
    };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Set `SO_SNDBUF` on a raw socket fd.
///
/// The kernel defaults UDP sockets to a small send buffer and returns ENOBUFS
/// the moment it fills; relaying a whole flow through one socket makes that
/// far more likely than it is for an ordinary application.
pub(crate) fn set_send_buffer(raw: i32, bytes: u32) -> io::Result<()> {
    let value = bytes as libc::c_int;
    // SAFETY: `raw` is a live socket owned by the caller for this call, and
    // `value` is the 4-byte integer `SO_SNDBUF` expects.
    let rc = unsafe {
        libc::setsockopt(
            raw,
            libc::SOL_SOCKET,
            libc::SO_SNDBUF,
            &value as *const libc::c_int as *const libc::c_void,
            std::mem::size_of::<libc::c_int>() as libc::socklen_t,
        )
    };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    /// Loopback destinations must never be bound: an `oif`-constrained lookup
    /// to 127.0.0.0/8 fails in the local table, and loopback traffic cannot
    /// enter the tunnel anyway.
    #[test]
    fn loopback_destinations_are_not_pinned() {
        // Pinning would fail (ENODEV for a bogus index), so a successful
        // no-op proves the loopback guard ran first.
        let result = pin_socket(-1, 0xDEAD, IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert!(result.is_ok(), "loopback pin must be a no-op");
        let result = pin_socket(-1, 0xDEAD, IpAddr::V6(Ipv6Addr::LOCALHOST));
        assert!(result.is_ok(), "loopback pin must be a no-op");
    }

    /// The Linux escape policy is semantic (SO_MARK), not tied to a cached NIC.
    #[test]
    fn interface_index_is_not_part_of_linux_escape_policy() {
        let err = pin_socket(-1, 0xDEAD, IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)))
            .expect_err("invalid fd must fail at setsockopt");
        assert_eq!(err.raw_os_error(), Some(libc::EBADF));
    }
}
