# Browser VPN / proxy extensions: interception boundary

Status: **partially mitigated; not validated on Windows with Proton VPN for Chrome**
Date: 2026-11-07

## Two different mechanisms

1. **Route-level desktop VPN**: changing routes can steer packets away from
   ACE's WinTun device. Windows uses app-scoped WFP ALE connect filters to
   admit protected-browser connections over ACE's tunnel (or explicit local
   destinations) and deny other routes. Linux uses policy routing and an
   nftables guard; macOS currently uses utun/split routes. The native platform
   path still needs actual host/driver tests.
2. **Browser proxy/VPN extension**: Chrome can connect to an HTTPS proxy or
   remote relay. ACE observes the browser's connection to that relay, but the
   blocked destination is inside a second encrypted tunnel. Its hostname may
   be absent from the outer CONNECT/TLS context. Enforcing the outer hostname
   cannot, by itself, enforce the hidden inner hostname.

The second case is not solved by catching QUIC alone, by rerouting a VPN, or
by trusting the mitm-proxy CA. Some extensions use IP literals, encrypted DNS,
rotating relays, certificate pinning or nonstandard ports.

## Existing defenses and limits

- ACE captures protected-browser traffic through WinTun or the platform TUN.
  Browser HTTP/3/QUIC is handled conservatively; protected or unattributed
  datagrams with QUIC signatures are not silently sent direct.
- ACE's proxy enforces known blocked hostnames on an HTTP CONNECT **only if**
  the outer request identifies the relevant hostname; it cannot inspect
  tunneled HTTPS destinations the remote relay encapsulates.
- The `VPN_EXTENSION_DOMAINS` list blocks **recognized, DNS-attributed browser
  relay** hosts (for example subdomains under `protonvpn.com`), not a shared
  identity domain such as `proton.me`. Its rule is now scoped to protected
  browsers so native VPN and other applications are not accidentally denied.
- A DNS-snoop cache miss, DoH resolution, new provider hostname or IP-only
  relay may evade the relay-domain rule. Never advertise this list as complete.
- Existing rules send browser TCP 80/443 to the MITM proxy, while other TCP
  ports may be relayed directly inside ACE's netstack. This is a known
  enforcement gap for arbitrary-port proxy extensions; route capture alone
  does not make the encrypted contents observable.
- Existing explicit loopback/local exceptions preserve local connectivity
  but create a separate local-proxy threat model. A nonlisted local helper
  can in principle originate external traffic under a different process.

## Verification matrix before claiming bypass resistance

Run with real Windows elevation and the signed WinTun package. Start a
locked, enabled `Always` Network Block targeting a known HTTPS hostname and
verify the server/ACE counter, not just page appearance.

| Test | Expected observation |
|---|---|
| Chrome direct TCP 443, blocked hostname | Denied by ACE; no origin response |
| Chrome HTTP/3 and IPv6 for same host | Denied or retried on inspected TCP |
| Proton VPN Chrome extension: connect/disconnect; several endpoints | Block must not silently appear bypassed; any bypass is a release blocker |
| Extension under DNS-over-HTTPS and IP-only relay | Log that relay domain identity is unknown; do not claim domain-based deny |
| Extension uses TCP 8443 / 1080 or localhost helper | Exercise the direct-port and local-helper gaps above |
| Chrome restart, existing sessions, WFP route refresh, competing VPN routes | No stale uninspected session that defeats the block |
| Native Proton VPN + Chrome with extension disabled | ACE browser enforcement still works, unrelated VPN egress remains usable |
| Proton Mail/Drive/Pass and nonbrowser apps | No collateral block from the relay denylist |
| Engine/TUN unavailable or WFP refresh failure | Clearly report unavailable/degraded enforcement; no false "protected" UI claim |

Capture the exact remote endpoint, DNS attribution, `ace-tun` flow decision,
HTTP CONNECT authority, and WFP route used in each test. Browser screenshots
alone cannot establish these network guarantees.

**Do not close the Proton extension bypass issue on the strength of the
unit tests in `service::proxy::constants::tests`.** Those tests prove rule
scope for known hostnames, not that the extension's actual relay is recognized.

## Options for a complete solution

Evaluate browser-managed extension controls only with explicit user consent
and a documented supported-browser policy, or design a trustworthy
browser-side origin-enforcement channel. An endpoint blacklist cannot
reliably reveal the inner HTTPS destination. Any stricter network policy
must explicitly balance legitimate VPN usage and local/private networking.
