# Browser-proxy extension enforcement (Windows preview)

**Status:** The opt-in policy guard is implemented. Live Windows browser tests
are still required. Do not claim that the guard prevents every VPN-extension bypass.

For the Linux equivalent, see `docs/LINUX_BROWSER_PROXY_ENFORCEMENT.md`.

## Why this exists

HTTPS browser proxy extensions, such as Proton VPN, Windscribe, and NordVPN,
connect the browser to a TLS-encrypted proxy. The encrypted CONNECT tunnel hides
the original destination. WinTun and WFP can capture the connection but can
identify only the proxy. They cannot identify the requested blocked website.
Routing, DNS, TLS SNI, and known VPN-domain lists cannot solve this problem alone.

Ace also denies TCP/4443 for protected browsers based on audited extension behavior.
WFP keeps the IPv6 browser block if IPv6 capture is unavailable at startup.
Neither control prevents all HTTPS proxy bypasses on TCP/443.

## Opt-in strict browser proxy guard

The MSI includes `browser-proxy-extension-policy.ps1` in the Ace Blocker
installation directory. It does **not** enable the script automatically.
Without consent, the guard could disrupt corporate proxies and legitimate privacy extensions.

Run the script from **64-bit Windows PowerShell** in the Ace Blocker installation directory.
Elevation is required for changes. Select one of these actions:

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

The script rejects existing extension and proxy management policies.
It does not take ownership of these policies. It scans loaded user hives.
It cannot verify unloaded hives or cloud-managed policies. Its journal is
stored in protected ProgramData/LineaSoftware/AceBlocker. Interrupted enables
can be undone, while Disable refuses to erase values altered by other tools.
Verify effective policy by restarting browsers and reviewing chrome://policy,
edge://policy and Firefox about:policies.

The MSI automatically invokes DisableIfOwned on normal uninstall, after
the active-lock gate. It does not clear the policy during major upgrades.
A conflicting or unrecognized owned policy makes uninstall fail safe rather
than silently removing other policy values.

**Tradeoffs:** The policy affects all users of managed Chrome, Edge, and Firefox.
It applies even when ACE blocks are inactive. It disables browser proxy extensions
and legitimate proxy servers. It can also disrupt corporate configurations.
The guard requires manual opt-in. Installing the MSI alone does **not** prevent
the HTTPS-proxy extension bypass.

## Windows VM verification required

Test Chrome, Edge and Firefox with Proton VPN, NordVPN, Windscribe, Browsec
and Surfshark extensions, connected/disconnected, with the guard enabled
and disabled. Confirm blocked domains fail and unblocked domains still load.
Check that authorized non-proxy extensions work and the managed policy pages
show the intended settings. Include split tunneling, DNS-over-HTTPS, HTTPS
proxy relay ports 443/4443/8443, WSS, IPv6, QUIC, local SOCKS/HTTP helpers,
browser restart, policy pre-existence, interrupted enable and uninstall.

Collect sanitized browser NetLog and ACE WFP/TUN decision metadata.
Use live browser tests to distinguish hidden hostnames from route-enforcement bypasses.
Unit tests and static extension manifests do not provide sufficient proof.

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
