# Browser-proxy extension enforcement (Windows preview)

**Status:** Implemented as an opt-in policy guard. Windows/browser live tests are
still required before claiming a fully closed VPN-extension bypass.

For the Linux equivalent, see `docs/LINUX_BROWSER_PROXY_ENFORCEMENT.md`.

## Why this exists

HTTPS browser proxy extensions such as Proton VPN, Windscribe and NordVPN send
a browser connection to a TLS-encrypted proxy. The original destination is
inside the encrypted CONNECT tunnel. WinTun and WFP can capture the connection
and still see only the proxy identity, not the originally requested blocked
website. Routing, DNS, TLS SNI, and a blocklist of known VPN domains are not a
complete solution.

Ace also has a narrowly targeted TCP/4443 deny rule for protected browsers
based on audited extension behavior, and WFP retains an IPv6 browser block
even when IPv6 capture was unavailable at startup. Neither alone fixes
generic HTTPS proxies on 443.

## Opt-in strict browser proxy guard

The MSI packages browser-proxy-extension-policy.ps1 in the Ace Blocker
installation directory. It does **not** automatically activate this script:
doing so without consent can break corporate proxies and legitimate privacy
extensions.

From an elevated **64-bit Windows PowerShell** console, run the script in
Program Files / Ace Blocker with:

- Action Status (read-only; no elevation needed)
- Action Enable -IUnderstand (writes managed browser policy)
- Action Disable (removes only exactly matching Ace-owned settings)

Example PowerShell commands:

```powershell
$guard = Join-Path $env:ProgramFiles 'Ace Blocker\browser-proxy-extension-policy.ps1'
& $guard -Action Status
& $guard -Action Enable -IUnderstand
# Restart Chrome, Edge and Firefox and inspect the browser policy pages.
& $guard -Action Disable
```

The Enable action blocks the WebExtension proxy permission through Chrome,
Edge and Firefox ExtensionSettings policy. It also sets Chrome and Edge
ProxyMode to direct and locks Firefox Proxy mode to none. Firefox's
blocked_permissions feature requires Firefox 153 or later; Firefox's Proxy
lock works on older releases.

The script rejects pre-existing extension and proxy management policies
instead of taking ownership of them. It scans loaded user hives, but cannot
prove anything about unloaded hives or cloud-managed policies. Its journal is
stored in protected ProgramData/LineaSoftware/AceBlocker. Interrupted enables
can be undone, while Disable refuses to erase values altered by other tools.
Verify effective policy by restarting browsers and reviewing chrome://policy,
edge://policy and Firefox about:policies.

The MSI automatically invokes DisableIfOwned on normal uninstall, after
the active-lock gate. It does not clear the policy during major upgrades.
A conflicting or unrecognized owned policy makes uninstall fail safe rather
than silently removing other policy values.

Tradeoffs: this affects all users of managed Chrome/Edge/Firefox on the
computer, not just active ACE blocks. It disables browser proxy extensions,
legitimate proxy servers and potentially corporate configurations. It is
currently manual opt-in: the normal MSI installation alone does **not**
eliminate the HTTPS-proxy extension bypass.

## Windows VM verification required

Test Chrome, Edge and Firefox with Proton VPN, NordVPN, Windscribe, Browsec
and Surfshark extensions, connected/disconnected, with the guard enabled
and disabled. Confirm blocked domains fail and unblocked domains still load.
Check that authorized non-proxy extensions work and the managed policy pages
show the intended settings. Include split tunneling, DNS-over-HTTPS, HTTPS
proxy relay ports 443/4443/8443, WSS, IPv6, QUIC, local SOCKS/HTTP helpers,
browser restart, policy pre-existence, interrupted enable and uninstall.

Collect sanitized browser NetLog and ACE WFP/TUN decision metadata.
A live browser test is necessary to distinguish loss of hostname visibility
from any actual route-enforcement bypass. Do not claim proof based only on
unit tests or static extension manifests.

## Remaining limits

- Browser-to-localhost/private-network relay helpers are not comprehensively
  controlled by this implementation; current WFP policy keeps local traffic
  reachable for compatibility.
- Unsupported browsers, Linux/macOS enterprise policy management, elevated
  administrator tampering and uncooperative proxy clients remain outside scope.
- Changing ports or blocking known VPN domains is defense in depth only.
- A trusted browser URL-level companion extension could complement kernel
  filtering, but an easily removable extension is not an anti-tamper boundary.

## Official references

- https://support.google.com/chrome/a/answer/7532015
- https://www.chromium.org/administrators/policy-list-3/extension-settings-full/
- https://learn.microsoft.com/en-us/deployedge/microsoft-edge-manage-extensions-ref-guide
- https://firefox-admin-docs.mozilla.org/reference/policies/extensionsettings/
- https://firefox-admin-docs.mozilla.org/reference/policies/proxy/
- https://chromium.googlesource.com/chromium/src/+/HEAD/net/docs/proxy.md
