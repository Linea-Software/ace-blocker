//! TUN device lifecycle: create, address, route, and tear down.
//!
//! The device is owned by this process. The TUN interface is non-persistent:
//! the kernel destroys it when the last fd closes, which happens on normal
//! drop *and* on abnormal termination (SIGKILL closes fds for us). Because our
//! routes point at the interface, they die with it. That property is what
//! makes the "hard-kill leaves the machine online" requirement hold without a
//! watchdog process — see [`super::netcfg`] for the routing rationale.

use std::ffi::CString;
use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

use netlink_packet_route::AddressFamily;

use super::firewall::FirewallGuard;
use super::netcfg::{self, RouteHandle, RuleHandle};
use super::{ACE_CAPTURE_PRIORITY, ACE_CAPTURE_TABLE, ACE_EGRESS_MARK};
use crate::device::SessionHandle;
use crate::error::{Error, Result};
use crate::platform::{
    AdapterHandle, PhysicalInterface, TUN_IPV4, TUN_IPV4_PREFIX, TUN_IPV6, TUN_IPV6_PREFIX, TUN_MTU,
};

/// Interface name template. The trailing %d makes the kernel assign
/// ace0, ace1, ... so a leftover interface can never block startup.
pub const ADAPTER_NAME: &str = "ace%d";

const DEFAULT_V4: Ipv4Addr = Ipv4Addr::UNSPECIFIED;
const DEFAULT_V6: Ipv6Addr = Ipv6Addr::UNSPECIFIED;
const LINK_LOCAL_V4: Ipv4Addr = Ipv4Addr::new(169, 254, 0, 0);
const LINK_LOCAL_V4_PREFIX: u8 = 16;
const MULTICAST_V4: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 0);
const MULTICAST_V4_PREFIX: u8 = 4;
const LIMITED_BROADCAST_V4: Ipv4Addr = Ipv4Addr::new(255, 255, 255, 255);
const LINK_LOCAL_V6: Ipv6Addr = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 0);
const LINK_LOCAL_V6_PREFIX: u8 = 10;
const MULTICAST_V6: Ipv6Addr = Ipv6Addr::new(0xff00, 0, 0, 0, 0, 0, 0, 0);
const MULTICAST_V6_PREFIX: u8 = 8;

/// _IOW('T', 202, int) - attach the fd to a TUN interface (TUNSETIFF).
const TUNSETIFF: libc::c_ulong = 0x4004_54ca;
/// _IOW('T', 208, int) - set the TUN offload mask (TUNSETOFFLOAD).
const TUNSETOFFLOAD: libc::c_ulong = 0x4004_54d0;

/// A live TUN adapter with its policy-routing and firewall ownership.
///
/// The capture routes live in a private table. If this process is hard-killed,
/// closing the TUN fd makes those routes unusable, so the inverted policy rule
/// falls through to the ordinary RPDB. The nftables guard has an independent
/// timed heartbeat and also becomes inert after a crash.
pub(crate) struct TunAdapter {
    fd: OwnedFd,
    routes: Vec<RouteHandle>,
    rules: Vec<RuleHandle>,
    firewall: Option<FirewallGuard>,
}

impl TunAdapter {
    /// Create the device and atomically build the Linux enforcement layers.
    pub(crate) fn create(ipv6: bool, _iface: &PhysicalInterface) -> Result<Self> {
        let (fd, ifindex) = open_device()?;
        let mut adapter = Self {
            fd,
            routes: Vec::with_capacity(2),
            rules: Vec::with_capacity(7),
            firewall: None,
        };

        netcfg::add_address(ifindex, IpAddr::V4(TUN_IPV4), TUN_IPV4_PREFIX)
            .map_err(|error| Error::netcfg("add_address(v4)", error))?;
        if ipv6 {
            netcfg::add_address(ifindex, IpAddr::V6(TUN_IPV6), TUN_IPV6_PREFIX)
                .map_err(|error| Error::netcfg("add_address(v6)", error))?;
        }
        netcfg::set_link_mtu_and_up(ifindex, TUN_MTU)
            .map_err(|error| Error::netcfg("set_link_mtu_and_up", error))?;

        adapter.install_capture_route(ifindex, IpAddr::V4(DEFAULT_V4), 0)?;
        if ipv6 {
            adapter.install_capture_route(ifindex, IpAddr::V6(DEFAULT_V6), 0)?;
        }

        // Local-scope traffic should continue using the host's normal topology.
        // These rules are ahead of ACE's catch-all capture rule but behind the
        // kernel's built-in priority-0 local table.
        adapter.install_main_destination_rule(
            IpAddr::V4(LINK_LOCAL_V4),
            LINK_LOCAL_V4_PREFIX,
            2,
        )?;
        adapter.install_main_destination_rule(IpAddr::V4(MULTICAST_V4), MULTICAST_V4_PREFIX, 3)?;
        adapter.install_main_destination_rule(IpAddr::V4(LIMITED_BROADCAST_V4), 32, 4)?;
        if ipv6 {
            adapter.install_main_destination_rule(
                IpAddr::V6(LINK_LOCAL_V6),
                LINK_LOCAL_V6_PREFIX,
                2,
            )?;
            adapter.install_main_destination_rule(
                IpAddr::V6(MULTICAST_V6),
                MULTICAST_V6_PREFIX,
                3,
            )?;
        }

        adapter.install_capture_rule(AddressFamily::Inet)?;
        if ipv6 {
            adapter.install_capture_rule(AddressFamily::Inet6)?;
        }

        // Install the independent output guard only after capture routing is
        // active, so there is never a startup window where the firewall is
        // blocking ordinary traffic before the TUN can carry it.
        adapter.firewall = Some(
            FirewallGuard::install()
                .map_err(|error| Error::netcfg("install_nftables_guard", error))?,
        );

        Ok(adapter)
    }

    fn install_capture_route(&mut self, ifindex: u32, dest: IpAddr, prefix: u8) -> Result<()> {
        let handle = netcfg::add_route_in_table(ifindex, dest, prefix, ACE_CAPTURE_TABLE).map_err(
            |error| {
                Error::netcfg(
                    format!("add_capture_route({dest}/{prefix}, table={ACE_CAPTURE_TABLE})"),
                    error,
                )
            },
        )?;
        self.routes.push(handle);
        Ok(())
    }

    fn install_main_destination_rule(
        &mut self,
        dest: IpAddr,
        prefix: u8,
        priority: u32,
    ) -> Result<()> {
        let handle =
            netcfg::add_main_destination_rule(dest, prefix, priority).map_err(|error| {
                Error::netcfg(
                    format!("add_main_rule({dest}/{prefix}, priority={priority})"),
                    error,
                )
            })?;
        self.rules.push(handle);
        Ok(())
    }

    fn install_capture_rule(&mut self, family: AddressFamily) -> Result<()> {
        let handle = netcfg::add_capture_rule(
            family,
            ACE_CAPTURE_TABLE,
            ACE_EGRESS_MARK,
            ACE_CAPTURE_PRIORITY,
        )
        .map_err(|error| Error::netcfg("add_capture_rule", error))?;
        self.rules.push(handle);
        Ok(())
    }

    fn remove_policy(&mut self) {
        // Remove the firewall first. Once capture rules disappear, ordinary
        // traffic may use physical/VPN interfaces again and must not hit a
        // lingering ACE drop rule.
        self.firewall.take();

        for handle in self.rules.drain(..).rev() {
            if let Err(error) = netcfg::delete_rule(&handle) {
                tracing::debug!("policy-rule removal returned {error} (already gone?)");
            }
        }
        for handle in self.routes.drain(..).rev() {
            if let Err(error) = netcfg::delete_route(&handle) {
                tracing::debug!("capture-route removal returned {error} (already gone?)");
            }
        }
    }
}

impl AdapterHandle for TunAdapter {
    fn session(&self) -> io::Result<SessionHandle> {
        self.fd.try_clone()
    }

    fn shutdown_session(&self) {
        // Linux has no separate WinTun-style session. Closing the adapter fd
        // tears down the non-persistent device.
    }

    fn remove_routes(&mut self) {
        self.remove_policy();
    }
}

impl Drop for TunAdapter {
    fn drop(&mut self) {
        self.remove_policy();
    }
}

/// Open `/dev/net/tun`, create the device node if it is missing (containers
/// often lack it), attach with `IFF_TUN | IFF_NO_PI`, and read back the
/// kernel-assigned interface index.
fn open_device() -> Result<(OwnedFd, u32)> {
    // Create the device node if missing. mknod needs CAP_MKNOD; errors are
    // ignored and the open below reports the real problem.
    if std::fs::metadata("/dev/net/tun").is_err() {
        let _ = std::fs::create_dir_all("/dev/net");
        // SAFETY: static NUL-terminated path; mode and device are plain
        // values.
        unsafe {
            libc::mknod(
                c"/dev/net/tun".as_ptr(),
                0o666 | libc::S_IFCHR,
                libc::makedev(10, 200),
            );
        }
    }

    // SAFETY: static NUL-terminated path; flags are plain values.
    let raw = unsafe { libc::open(c"/dev/net/tun".as_ptr(), libc::O_RDWR | libc::O_CLOEXEC) };
    if raw < 0 {
        // SAFETY: `open` just failed, so errno is set.
        let error = io::Error::last_os_error();
        return Err(if is_privilege_error(&error) {
            Error::NotElevated
        } else {
            Error::netcfg("open(/dev/net/tun)", error)
        });
    }
    // SAFETY: `raw` is a fresh fd with no other owner.
    let fd = unsafe { OwnedFd::from_raw_fd(raw) };

    // SAFETY: `request` is a live, zeroed ifreq; TUNSETIFF fills it with the
    // assigned interface name.
    let mut request: libc::ifreq = unsafe { std::mem::zeroed() };
    let name = CString::new(ADAPTER_NAME).expect("static name has no NUL");
    for (slot, byte) in request.ifr_name.iter_mut().zip(name.as_bytes_with_nul()) {
        *slot = *byte as libc::c_char;
    }
    request.ifr_ifru.ifru_flags = (libc::IFF_TUN | libc::IFF_NO_PI) as libc::c_short;

    // SAFETY: `request` is fully initialised above and outlives the call.
    let rc = unsafe {
        libc::ioctl(
            fd.as_raw_fd(),
            TUNSETIFF,
            &request as *const libc::ifreq as *const libc::c_void,
        )
    };
    if rc != 0 {
        let error = io::Error::last_os_error();
        return Err(if is_privilege_error(&error) {
            Error::NotElevated
        } else {
            Error::netcfg("TUNSETIFF", error)
        });
    }

    // A previous process may have left the TUN_F_* offload mask set on a
    // persistent interface. Without IFF_VNET_HDR we must clear it, or reads
    // return GSO aggregates as oversized single packets. No-op on a fresh
    // device; failure is logged, not fatal.
    // SAFETY: passing the address of a zero int; TUNSETOFFLOAD copies it.
    let zero = 0;
    unsafe {
        libc::ioctl(fd.as_raw_fd(), TUNSETOFFLOAD, &zero as *const libc::c_int);
    }

    // The device must be nonblocking for the async wrapper.
    // SAFETY: `fd` is live; F_GETFL/F_SETFL take plain values.
    let flags = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFL) };
    if flags < 0
        || unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
    {
        return Err(Error::netcfg(
            "fcntl(O_NONBLOCK)",
            io::Error::last_os_error(),
        ));
    }

    // SAFETY: the kernel wrote a NUL-terminated name into `request`.
    let assigned = unsafe { std::ffi::CStr::from_ptr(request.ifr_name.as_ptr()) }
        .to_string_lossy()
        .into_owned();
    let name = CString::new(assigned.clone()).expect("kernel interface name has no NUL");
    // SAFETY: `name` is a live C string.
    let ifindex = unsafe { libc::if_nametoindex(name.as_ptr()) };
    if ifindex == 0 {
        return Err(Error::netcfg("if_nametoindex", io::Error::last_os_error()));
    }

    tracing::debug!("tun device '{assigned}' up (ifindex {ifindex})");
    Ok((fd, ifindex))
}

/// A privilege failure from the tun setup — `EPERM` without `CAP_NET_ADMIN`.
fn is_privilege_error(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::PermissionDenied
}
