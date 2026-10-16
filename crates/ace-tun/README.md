# ace-tun

The `ace-engine` workspace's transparent redirect for Windows, Linux, and macOS. It creates a virtual
network adapter (WinTun on Windows, a TUN device on Linux/macOS), points the
routing table at it, runs a userland TCP/IP stack over the raw IP packets that
arrive, and hands each terminated flow to a local MITM proxy — or connects it
straight out, or drops it.

```
  app ──routes──▶ TUN adapter ──▶ userland netstack ──┬─▶ MITM proxy ──▶ internet
                                                      └─▶ direct socket ─▶ internet
```

This crate replaces `proxy-redirect`, a WinDivert packet-interception layer.

## Why the rewrite

The WinDivert design intercepted individual packets and reverse-mapped each one
to its owning process to decide what to do with it. Three problems were
structural rather than incidental:

| Problem | Why it happened | How this crate avoids it |
|---|---|---|
| **HTTP/3 leaked past blocking.** Instagram and YouTube loaded despite matching block rules. | The per-packet `GetExtendedUdpTable` lookup raced UDP socket setup. On a miss the flow was passed through *uninspected*. | Attribution happens once per **flow**, at open. The SYN could not exist unless the socket already did, so there is nothing to race. And QUIC is dropped regardless of attribution — see below. |
| **NAT bookkeeping.** Destination rewriting needed a table mapping ephemeral ports back to real destinations, plus checksum fixups. | Packet-level redirection destroys the original destination. | Under a TUN the destination is simply in the IP header. No rewriting, no table. |
| **IPv6 bypassed everything.** | The capture filter was `ip and (...)`, which never matched IPv6. | The netstack handles both families identically; IPv6 addressing and routes are installed by default. |

A later attempt to fix attribution with a second WinDivert handle on the Socket
layer made things worse: QUIC still leaked *and* ordinary sites stopped loading.
That is the failure mode this crate is designed against.

## Design

### Routing and VPN coexistence

Routing is now intentionally platform-specific. The invariant is that a
protected application must cross ACE before it can reach an external route;
VPN software is allowed to remain part of the machine's normal egress path.

**Windows** keeps the WinTun packet transport and the split-default capture
routes (`0.0.0.0/1`, `128.0.0.0/1`, `::/1`, `8000::/1`). Those routes are
no longer the security boundary. A dynamic Windows Filtering Platform session
installs app-scoped ALE connect filters for protected browsers: the browser may
connect through ACE's WinTun interface or to local/private destinations, and a
catch-all filter blocks that browser if another VPN route wins instead.
Unrelated applications and VPN daemons are not globally blocked.

**Linux** does not install split-default routes in `main`. ACE owns a dedicated
routing table with a default route to the TUN and an RPDB rule that selects it
for ordinary local sockets. ACE relay sockets carry `SO_MARK =
0x0ACE0001`; the capture rule is inverted against that mark, so relays skip only
ACE's table and continue through the normal RPDB. That means the safe path is:

```
browser -> ACE TUN/netstack/MITM -> ACE-marked relay -> normal Linux routing -> VPN or NIC
```

An nftables OUTPUT guard backs the routing policy: while ACE is healthy,
unmarked local traffic cannot escape through another interface. Its drop verdict
has a short heartbeat timeout, so a hard crash cannot leave a stale firewall
that strands the host offline.

**macOS** retains the existing utun + split-default design for now.

All Windows route/address configuration uses IP Helper APIs rather than
`netsh`; Linux route/rule configuration uses rtnetlink rather than parsing
`ip` output.

### Relay loop prevention

ACE's own upstream sockets must be distinguishable from application sockets or
they would recursively enter the tunnel. Relay setup therefore fails closed:

| OS | Relay escape |
|---|---|
| Windows | Before every relay connection, enumerate the current routes, exclude ACE's WinTun interface, choose the best remaining route, then apply `IP_UNICAST_IF` / `IPV6_UNICAST_IF`. This follows Wi-Fi/Ethernet/VPN changes without a stale startup-time pin. |
| Linux | Apply `SO_MARK`; policy routing skips ACE's capture table and lets the normal RPDB choose the current egress, including an active VPN. |
| macOS | `IP_BOUND_IF` / `IPV6_BOUND_IF` using the startup connectivity snapshot. |

On Windows, `IP_UNICAST_IF` takes the interface index in **network** byte order
while `IPV6_UNICAST_IF` takes it in **host** byte order. The encoding is tested
because getting it wrong looks like a total outbound hang.

If ACE cannot establish the platform escape invariant for a non-loopback relay,
the connection is rejected rather than retried with ordinary routing.

### Failure posture

Unknown process attribution and an empty ruleset still fail open to direct relay,
but security-boundary failures do not silently downgrade: relay escape setup is
fail-closed, and a flow that explicitly requires a proxy is closed if that proxy
cannot be reached. `StatsSnapshot::proxy_fallbacks` retains its legacy name but
counts these required-proxy failures; they are not sent direct.

If the platform capture layer itself cannot start, `ace-engine` currently keeps
running and reports interception as unavailable rather than taking networking
down. The service layer is responsible for surfacing that degraded state.

The one deliberate exception is **browser QUIC**. UDP/443 from protected
browsers is dropped to force inspected TCP fallback; unattributed UDP/443 is also
dropped to avoid an attribution-race bypass. Known non-protected applications keep
their normal UDP/443 path because not every protocol on that port has a TCP fallback.

### Teardown

Routes are removed on stop, on drop, and on netstack panic (a watchdog task
supervises the netstack and tears down whichever way it ends).

If the process dies without running any of that, the adapter is destroyed —
Windows closes the WinTun handle and the routes bound to its LUID go with it;
on Linux and macOS the non-persistent TUN/utun interface dies with its last
fd, taking its routes along. **Hard-kill restores normal connectivity by
itself** — the safety property does not depend on our cleanup code running.

## Requirements

* **Windows:** administrator privileges (WinTun and the WFP dynamic policy both
  require elevation) and `wintun.dll` next to the executable — see below.
* **Linux:** root or `CAP_NET_ADMIN`, plus the `nft` userspace command. The same
  capability covers TUN setup, policy routing, `SO_MARK`, and nftables changes.
* **macOS:** root (utun creation requires root or the packet-tunnel
  entitlement).

`TunRedirect::start` returns `Error::NotElevated` so callers can degrade
gracefully instead of failing obscurely.

## Bundling `wintun.dll` (Windows only)

The official WinTun distribution is vendored at `thirdparty/wintun/` —
**version 0.14.1**, Authenticode-signed by `CN=WireGuard LLC`. It ships all four
architectures (`amd64`, `x86`, `arm`, `arm64`) plus the header and license.

`build.rs` copies the architecture-appropriate DLL next to every binary cargo
produces (`target/<profile>/`, `deps/`, `examples/`). Because it runs as a build
script of a path dependency, this also covers **`ace-engine`'s** output
directory, so the engine binary gets the DLL beside it with no extra step.

The `vendored_wintun_dll_loads` test loads the DLL and resolves its exports,
which catches a truncated, wrong-architecture, or mis-copied artifact at
`cargo test` time instead of as an opaque adapter-creation failure at runtime.

### Shipping it

`ace-installer/build.ps1` copies the DLL from this vendored tree — not from the
build output, so the shipped artifact is the reviewed one — verifies its
Authenticode signature is still `Valid`, and fails the build if it is not.
`installer.wxs` installs it beside `ace-engine.exe`.

**There is no separate driver to register.** Unlike WinDivert, WinTun's kernel
component is carried inside the DLL and installed on demand when the first
adapter is created, so the installer's `sc create WinDivert64` / `sc delete`
custom actions are gone rather than replaced.

Do not repack, patch, or recompress the DLL — the signature is what lets the
kernel component load.

### Licensing

WinTun is by WireGuard LLC / Jason A. Donenfeld and is released under the
**GPLv2 *or* the MIT license** — see `thirdparty/wintun/LICENSE.txt`. We
redistribute under the MIT option, which requires only that the copyright and
permission notice accompany the distribution. The installer ships that file as
`WINTUN-LICENSE.txt`; `build.ps1` fails if it is missing, so the obligation
cannot be dropped by accident.

## Usage

```rust
use ace_tun::{TunRedirect, ProxyConfig, Rule, RuleAction, RuleProtocol};

let redirect = TunRedirect::builder("127.0.0.1:8080")?
    .protect_processes("chrome.exe;brave.exe")
    .add_rule(Rule::new("chrome.exe;brave.exe")
        .ports("80;443")
        .protocol(RuleProtocol::Tcp)
        .action(RuleAction::Proxy))
    .add_rule(Rule::new("*").action(RuleAction::Direct))
    .proxy_config(ProxyConfig::http("127.0.0.1", 8080))
    .build()?;

redirect.start().await?;   // needs elevation
// ...
redirect.stop().await?;
```

Flows selected by a `Proxy` rule are handed to the upstream proxy as an HTTP
`CONNECT` (or SOCKS5), using the hostname from the DNS-snoop cache when one is
known. That matters: the MITM proxy needs a name to issue a certificate for and
to send as SNI upstream, so DNS snooping is load-bearing, not just diagnostic.

## Known gaps

* **Only ICMP echo is relayed on Windows.** Ordinary `ping` works through a raw
  socket pinned to the current non-ACE route, but other ICMP/unknown transport
  messages are still not proxied through the userland stack.
* **UDP `Proxy` rules degrade to direct.** UDP cannot be tunnelled through an
  HTTP `CONNECT` proxy, so a `Proxy` action on a UDP flow relays it directly
  rather than dropping it.
* **Captured multicast/broadcast is a fallback drop.** Windows demotes WinTun's
  auto-created group routes so mDNS/SSDP/LLMNR stay on the LAN; Linux and macOS
  likewise keep group traffic on the host's normal topology. If such traffic
  nevertheless reaches the netstack, it is dropped rather than incorrectly
  relayed through a connected unicast socket.

## Testing

`cargo test` covers the parts that do not need a kernel driver: rule evaluation,
the full flow-decision matrix (including the QUIC and loop-guard cases), DNS
parsing, the proxy client, socket-option encoding, and builder validation.

Adapter creation, routing, and packet forwarding cannot be exercised without
elevation and a real driver. Use `cargo run --example live_check` (elevated) for
those — it runs the whole stack against a self-contained blocking proxy and
prints what it sees.

The macOS backend is implemented and compiles for both Apple targets but has
not been run on real hardware from the development environment; see
`docs/cross-platform-report.md` §8.4 for what needs a macOS machine.
