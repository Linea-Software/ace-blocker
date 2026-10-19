# VPN enforcement architecture

This document describes the route-bypass hardening introduced in September 2026.

## Security invariant

For a protected browser, an external connection must not become usable unless the
flow first crosses ACE's filtering path. Desktop VPN clients are not identified
by vendor executable names or tunnel ports. A VPN may remain the machine's final
egress after ACE has inspected or blocked the browser flow.

The design separates two concerns:

- the virtual adapter transports captured packets into ACE;
- an OS-native policy boundary prevents protected traffic from routing around
  that adapter.

This is deliberately different from maintaining lists such as OpenVPN,
WireGuard, NordVPN, or ProtonVPN process names and ports.

## Windows

Windows retains WinTun as the packet transport. Removing it would require
replacing transparent interception with WFP connect redirection, and connect
redirection requires a signed WFP callout driver. The current design gets the
anti-bypass property without adding an ACE kernel driver.

### WFP enforcement

At startup ACE opens a dynamic WFP session and installs an ACE sublayer. For each
configured protected browser executable path it installs filters at
ALE connect authorization:

1. permit the browser when Windows selected ACE's WinTun interface;
2. permit loopback/private/link-local/multicast destinations;
3. block all other connections from that application.

The policy is application-scoped. VPN daemons and unrelated applications are not
globally blocked, so a VPN can still establish its own transport.

Standard browser installation paths are registered before launch when they can
be found. A process watcher also resolves newly running protected browser
basenames to full paths and adds their WFP application identities to the
dynamic session. Portable/custom browser installs therefore become protected as
soon as their executable path is observed.

Once WinTun routes and WFP filters are installed, ACE resets established
**public IPv4 TCP sockets owned by protected browsers** using the Windows IP
Helper API. This forces a browser started before ACE to reconnect through the
interception path, without restarting it. Local/private sockets and other
applications are untouched. This is best-effort: Windows has no equivalent
public TCP reset API for IPv6 or QUIC UDP, so WFP policy-change reauthorization
remains the protection for those pre-existing flows.

When a browser resolved a hostname before ACE started (or uses DoH), ACE can
recover its hostname from TLS ClientHello SNI before relaying an IP-literal
CONNECT. An unknown or invalid TLS identity is rejected rather than silently
allowed. ECH can conceal the real hostname behind an outer SNI, so encrypted
ClientHello remains a limitation when no authoritative hostname is available.

The WFP session is dynamic: closing the session removes its filters. ACE removes
the WFP guard before restoring ordinary capture routes during graceful teardown.

### Relay egress

A relay socket must never reuse ACE's own WinTun route. Before each outbound
connection ACE enumerates the current Windows route table, excludes the ACE
interface, and chooses the remaining route with the longest matching prefix and
lowest effective metric. It then applies `IP_UNICAST_IF` or
`IPV6_UNICAST_IF`.

This lookup happens for every relay, so Wi-Fi/Ethernet changes and VPN
connect/disconnect events do not leave ACE pinned to the interface discovered at
engine startup. If no safe non-ACE route exists, the relay fails closed.

## Linux

Linux uses the TUN only as packet transport. ACE no longer competes for the
`main` table with NetworkManager or VPN software.

### Policy routing

ACE owns routing table `20607`. It contains a default route for each enabled
address family pointing to the ACE TUN.

An RPDB rule at priority 10 selects that table for ordinary unmarked traffic.
The rule is inverted against fwmark `0x0ace0001`.

ACE applies that mark with `SO_MARK` to every non-loopback relay socket. The
marked relay therefore skips ACE's capture table and continues through the
normal policy-routing database. If the user has a VPN, its rules may then send
the already-filtered ACE relay through that VPN.

Local/link-scope traffic is kept ahead of the capture rule so normal host and
LAN behavior does not depend on the TUN.

### nftables guard

Routing is not treated as the security boundary. ACE also owns an
`inet ace_blocker_guard` nftables OUTPUT chain.

While ACE is healthy the chain permits:

- loopback;
- traffic whose selected output is an ACE TUN;
- packets carrying ACE's egress mark;
- root-owned transport traffic, so privileged VPN daemons can establish their
  underlying tunnel;
- link-local, multicast, and limited-broadcast traffic.

Other local egress is dropped. This catches a non-root browser if another policy
rule or a newly created tunnel tries to route it directly around ACE.

The final drop verdict is held in a timed nftables verdict-map entry. ACE
refreshes it every five seconds and the entry expires after fifteen seconds.
Consequently an ungraceful ACE crash leaves, at worst, inert routing rules and a
firewall guard that automatically stops dropping traffic.

The Linux service needs root or a capability arrangement that gives ACE and its
`nft` helper `CAP_NET_ADMIN`. Packaging must preserve that capability across
the helper exec path.

## macOS

The macOS backend is unchanged by this hardening pass. It continues to use utun,
split-default routes, and `IP_BOUND_IF` / `IPV6_BOUND_IF`. It should receive
a separate PF/Network Extension enforcement pass before being treated as having
the same anti-bypass guarantees as Windows and Linux.

## Remaining trust boundaries

This design is intended to prevent ordinary VPN/tunnel software and route
changes from bypassing ACE. It is not a defense against an administrator who is
actively trying to disable ACE.

On Linux, unrestricted root can alter nftables, RPDB rules, capabilities,
cgroups, the ACE process, or the service definition. On Windows, an
administrator can change WFP state or stop/replace privileged components.

Browser-to-local-proxy tunnelling is also a distinct application-layer problem.
ACE keeps narrowly scoped proxy/extension defenses, but arbitrary localhost
relay protocols cannot be perfectly classified by route enforcement alone.
Solving that comprehensively on Windows would require a WFP connect-redirection
callout or an equivalent local-proxy policy.

## Validation matrix

Before shipping either platform, validate at least:

- VPN active before ACE starts;
- VPN connected after ACE starts;
- VPN reconnect/server change;
- WireGuard, OpenVPN, IKEv2, and at least two commercial clients;
- renamed/custom VPN userspace binaries;
- injected host routes more specific than ACE's normal capture route;
- IPv4 and IPv6;
- Wi-Fi to Ethernet and Ethernet to Wi-Fi changes;
- browser QUIC fallback;
- browser extension proxy/VPN relay domains;
- local SOCKS/HTTP proxy attempts;
- graceful stop;
- forced process kill;
- restart after a forced kill;
- Linux NetworkManager/systemd-networkd route churn;
- Linux nftables coexistence with an existing host firewall.

Windows validation should confirm that the protected application is blocked when
a competing interface wins and that the VPN daemon itself is still able to
connect. Linux validation should inspect both `ip rule`/table 20607 and the
ACE-owned nftables table while the engine is running.
