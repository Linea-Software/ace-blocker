# VPN enforcement architecture

This document explains the route-bypass protection added in September 2026.

## Security invariant

A protected browser must send external traffic through ACE's filtering path before it can use the connection. ACE does not identify desktop VPN clients by vendor executable names or tunnel ports. A VPN can still carry traffic to the final destination after ACE inspects or blocks the browser flow.

The design has two separate controls:

- The virtual adapter carries captured packets into ACE.
- An OS-native policy prevents protected traffic from bypassing the adapter.

ACE does not depend on lists of OpenVPN, WireGuard, NordVPN, or ProtonVPN process names and ports.

## Windows

Windows uses WinTun to carry captured packets. Removing WinTun would require WFP connect redirection for transparent interception. WFP connect redirection requires a signed callout driver. The current design prevents route bypass without an ACE kernel driver.

### WFP enforcement

At startup, ACE opens a dynamic WFP session and installs an ACE sublayer. It installs ALE connect-authorization filters for each configured protected browser executable path.

The filters apply these rules:

1. Permit a browser connection when Windows selects ACE's WinTun interface.
2. Permit loopback, private, link-local, and multicast destinations.
3. Block all other connections from that browser application.

These filters apply only to protected applications. ACE does not block VPN daemons or unrelated applications globally. A VPN can still establish its transport connection.

ACE registers standard browser installation paths before browser startup when it can find them. A process watcher also detects newly running protected browsers. It resolves their executable basenames to full paths. ACE adds those paths as WFP application identities in the dynamic session. This protects portable and custom browser installations after the watcher finds their executable paths.

After ACE installs WinTun routes and WFP filters, it resets established **public IPv4 TCP sockets owned by protected browsers**. It uses the Windows IP Helper API. This makes a browser that started before ACE reconnect through the filtering path. The browser does not need a restart. ACE does not change local or private sockets, or sockets owned by other applications.

This socket reset is best-effort. Windows has no equivalent public TCP reset API for IPv6 or QUIC UDP. For these existing flows, ACE relies on WFP policy-change reauthorization.

A browser can resolve a hostname before ACE starts. It can also use DoH. In these cases, ACE can recover the hostname from TLS ClientHello SNI before it forwards an IP-literal CONNECT. ACE rejects an unknown or invalid TLS identity. It does not silently permit the connection. ECH can hide the real hostname behind an outer SNI. Encrypted ClientHello remains a limitation when ACE has no authoritative hostname.

The WFP session is dynamic. Windows removes the filters when ACE closes the session. During a normal shutdown, ACE removes the WFP guard before it restores ordinary capture routes.

### Relay egress

An outbound relay socket must not use ACE's own WinTun route. Before each outbound connection, ACE reads the current Windows route table. It excludes the ACE interface. From the remaining routes, ACE selects the longest matching prefix and the lowest effective metric. It then sets `IP_UNICAST_IF` or `IPV6_UNICAST_IF`.

ACE checks routes for each relay. Thus, Wi-Fi/Ethernet changes and VPN connection changes do not leave ACE bound to the interface that was available at engine startup. If no safe route outside ACE exists, the relay fails closed.

## Linux

Linux uses the TUN only to carry captured packets. ACE does not compete with NetworkManager or VPN software for the `main` routing table.

### Policy routing

ACE owns routing table `20607`. It has a default route to the ACE TUN for each enabled address family.

An RPDB rule at priority 10 selects this table for ordinary unmarked traffic. The rule uses an inverted match against fwmark `0x0ace0001`.

ACE sets this mark on every non-loopback relay socket with `SO_MARK`. The marked relay bypasses ACE's capture table. It then follows the normal policy-routing database. If the user has a VPN, its rules can carry the already-filtered ACE relay through the VPN.

ACE gives local and link-scope traffic priority over the capture rule. This keeps normal host and LAN traffic independent of the TUN.

### nftables guard

Routing is not the security boundary. ACE also owns an `inet ace_blocker_guard` nftables OUTPUT chain.

When ACE is healthy, the chain permits:

- loopback traffic;
- traffic whose selected output interface is an ACE TUN;
- packets with ACE's egress mark;
- root-owned transport traffic, so privileged VPN daemons can establish their tunnels;
- link-local, multicast, and limited-broadcast traffic.

The chain drops other local egress. Thus, an ordinary non-root browser cannot bypass ACE through a new policy rule or tunnel.

A timed nftables verdict-map entry controls the final drop rule. ACE refreshes the entry every five seconds. The entry expires after fifteen seconds. If ACE crashes without a normal shutdown, the remaining routing rules can be inert. The firewall guard automatically stops dropping traffic when the entry expires.

The Linux service needs root or sufficient capabilities. ACE and its `nft` helper must have `CAP_NET_ADMIN`. Packaging must preserve this capability when it starts the helper.

## macOS

This protection change does not modify the macOS backend. It still uses utun, split-default routes, and `IP_BOUND_IF` / `IPV6_BOUND_IF`. macOS needs a separate PF/Network Extension enforcement review before ACE can claim the same anti-bypass guarantees as Windows and Linux.

## Remaining trust boundaries

This design aims to stop ordinary VPN and tunnel software from routing around ACE. It does not protect against an administrator who intentionally disables ACE.

On Linux, unrestricted root can change nftables, RPDB rules, capabilities, cgroups, the ACE process, and the service definition. On Windows, an administrator can change WFP state or stop or replace privileged components.

A browser that tunnels through a local proxy is a different application-layer problem. ACE retains narrow proxy and extension controls. Route enforcement alone cannot reliably identify every localhost relay protocol. A complete Windows solution would need a WFP connect-redirection callout or an equivalent policy for local proxies.

## Validation matrix

Before release, test at least these cases on each applicable platform:

- VPN active before ACE starts;
- VPN connected after ACE starts;
- VPN reconnect or server change;
- WireGuard, OpenVPN, IKEv2, and at least two commercial clients;
- renamed or custom VPN userspace binaries;
- injected host routes more specific than ACE's usual capture route;
- IPv4 and IPv6;
- Wi-Fi to Ethernet and Ethernet to Wi-Fi changes;
- browser QUIC fallback;
- browser extension proxy or VPN relay domains;
- local SOCKS and HTTP proxy attempts;
- normal shutdown;
- forced process kill;
- restart after a forced kill;
- Linux NetworkManager/systemd-networkd route changes;
- Linux nftables operation with an existing host firewall.

On Windows, verify that ACE blocks a protected application if another interface wins route selection. Verify that the VPN daemon can still connect.

On Linux, inspect `ip rule`, routing table `20607`, and the ACE-owned nftables table while the engine runs.
