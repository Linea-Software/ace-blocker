# Linux browser proxy/VPN extension guard

**Preview:** this is opt-in and must be tested on actual supported Linux browsers
before claiming that all VPN/proxy extensions are blocked.

## Scope

The network engine already uses the TUN with Linux policy routing and nftables,
but HTTPS browser proxy extensions can conceal the requested website inside the
encrypted connection to a relay. A correct route capture does not reveal the
inner `CONNECT` destination. The engine's TCP/4443 deny rule is only additional
defense, not a universal anti-bypass control.

Ace Blocker ships an administrator-managed browser policy helper as part of the
Linux release at `/opt/ace-blocker/browser-proxy-policy.py`. It does not modify
browser policies during normal installation. The helper supports these
native/system installations:

- Google Chrome: `/etc/opt/chrome/policies/managed/`
- Chromium: `/etc/chromium/policies/managed/`
- Brave: `/etc/brave/policies/managed/`
- Firefox: `/etc/firefox/policies/policies.json`

The guard uses Chromium-family `ExtensionSettings` to deny the `proxy` API
permission and `ProxyMode=direct`. Firefox receives
`policies.ExtensionSettings["*"].blocked_permissions=["proxy"]` and
`policies.Proxy={"Mode":"none","Locked":true}`.

The `blocked_permissions` setting in Firefox requires **Firefox 153+**. The
Firefox Proxy policy also works in older supported versions, but the precise
effect on an existing extension must be checked with that browser version.
Snap, Flatpak, distro-specific Chromium/Firefox builds, portable browsers,
unrecognized forks, and per-profile browser preferences may not honor these
system paths; they are not automatically covered. Firefox Flatpak has a
separate distribution-specific policy mechanism. The guard will not
silently claim compatibility with every Linux packaging variant.

## Usage

After installing Ace Blocker on a Linux system, inspect the policy plan and
existing configuration first. Commands below are intended to be run by the
machine owner and **never automatically by the installer**:

```bash
sudo /opt/ace-blocker/browser-proxy-policy.py --status
sudo /opt/ace-blocker/browser-proxy-policy.py --enable --dry-run
sudo /opt/ace-blocker/browser-proxy-policy.py --enable
# Restart browsers; verify chrome://policy, brave://policy, and about:policies.
sudo /opt/ace-blocker/browser-proxy-policy.py --disable
```

The default `--enable` protects all four browsers. If a browser has a
conflicting managed policy, the operation fails **before writing anything**.
To opt in only for the unaffected browsers, repeat `--browser`, for example:

```bash
sudo /opt/ace-blocker/browser-proxy-policy.py --enable --browser chrome --browser brave
```

This deliberately leaves Chromium and Firefox unprotected in that example.
To change the selected set, first disable the owned policies and then enable
with a new selection. The ownership journal is private to root, so the
`--status` command should also be run with `sudo` after enabling.

This disables *legitimate* browser proxy functionality too, including VPN
extensions, web debugging proxies, and corporate proxies. It affects all
users of the supported browsers and stays active outside Ace Blocker's
scheduled block times. It does not prevent a user with root access from
modifying system policies.

For Chromium-family browsers, the manager writes its own dedicated JSON
file and refuses conflicting extension/proxy policies in other managed or
recommended files. For Firefox, whose system-wide policy is a single file,
it **merges only the two nonconflicting policy keys** and retains the original
file bytes, ownership group and file permissions for exact restoration. It
refuses an existing `ExtensionSettings` or `Proxy` policy rather than
taking it over, and checks known installation-specific Firefox policy files.

The root-only journal is written before browser policy changes. If setup
is interrupted, `--disable` can restore or remove only ACE-owned changes.
If a policy has been changed subsequently, rollback refuses to erase it,
rather than deleting unrelated administrator configuration. Do not manually
edit the guard's ownership journal. Use `--status` to check the result and
`--disable` to revert the owned values before uninstalling Ace Blocker;
the current Linux installer has no automated uninstall path.

## Verification requirements

Test on a disposable Linux VM with Chrome, Chromium, Brave and Firefox,
including the installed browser versions and actual distribution-specific
policy paths. Compare permitted and blocked sites with the extension
disconnected, connected, split tunneling, private browsing, new browser
profiles, and restarted browsers. Use Proton VPN, Windscribe, NordVPN,
Browsec and Surfshark where available; test standard HTTPS proxies on
443/4443, WSS, DoH and local HTTP/SOCKS relays.

Verify that browsers recognize the enterprise policies and refuse proxy
extension traffic while ordinary HTTPS still works. Inspect local
ACE netstack decisions, outbound relay endpoints, route/nftables state,
and sanitized browser logs. Run all policy-manager tests against temporary
filesystem roots only: they must not modify the current workstation's
`/etc` policy directories.

**Known separate gap:** browser-to-localhost/private-network proxy helpers
remain reachable for compatibility and require another narrowly designed
anti-bypass measure. More broad nftables drops alone do not reveal websites
hidden within a TLS proxy tunnel.

## Official documentation

- Chromium enterprise policy files on Linux:
  https://www.chromium.org/administrators/configuring-policy-for-extensions/
- Chrome Enterprise ExtensionSettings:
  https://support.google.com/chrome/a/answer/7517525
- Brave Linux managed policy directory:
  https://support.brave.app/hc/en-us/articles/360039248271-Group-Policy
- Firefox Linux policy directories:
  https://firefox-admin-docs.mozilla.org/guides/policies-configuration/
- Firefox ExtensionSettings (blocked_permissions, Firefox 153+):
  https://firefox-admin-docs.mozilla.org/reference/policies/extensionsettings/
- Firefox Proxy:
  https://firefox-admin-docs.mozilla.org/reference/policies/proxy/
