pub const ACE_PROCESS: &str = "ace-engine.exe";
pub const ACE_APP_PROCESS: &str = "ace-app.exe";

/// Processes that must never be intercepted — all traffic passes through
/// directly.
pub const EXEMPT_PROCESSES: &[&str] = &[ACE_PROCESS, ACE_APP_PROCESS];

/// Known browser executables — only these processes have their TCP 80/443
/// traffic routed through mitm-proxy for MITM inspection.
///
/// The names are Windows-style (`chrome.exe`). ace-tun strips the `.exe`
/// suffix and matches case-insensitively, so they already match the
/// equivalent processes on Linux/macOS where no suffix exists. Binary names
/// that differ beyond the suffix (error.g. `msedge.exe` vs Linux's
/// `microsoft-edge`) are supplied per-OS by [`browser_processes`].
pub const DEFAULT_BROWSER_PROCESSES: &[&str] = &[
    "chrome.exe",
    "firefox.exe",
    "msedge.exe",
    "brave.exe",
    "opera.exe",
    "vivaldi.exe",
    "arc.exe",
];

/// Linux-only browser binary names installed by the official packages or the
/// major distributions:
///
/// * `google-chrome` / `google-chrome-stable` — Google's official .deb/.rpm
///   packages install `/usr/bin/google-chrome` (a wrapper around the
///   `google-chrome-stable` ELF).
/// * `chromium` / `chromium-browser` — the Debian/Ubuntu package binaries
///   (`chromium-browser` on older releases, the snap wrapper `chromium` on
///   newer Ubuntu).
/// * `microsoft-edge` / `microsoft-edge-stable` — Microsoft's official Linux
///   packages; `msedge.exe` normalizes to `msedge` which matches nothing on
///   Linux.
#[cfg(target_os = "linux")]
pub const LINUX_BROWSER_PROCESSES: &[&str] = &[
    "google-chrome",
    "google-chrome-stable",
    "chromium",
    "chromium-browser",
    "microsoft-edge",
    "microsoft-edge-stable",
];

/// macOS-only browser process names — the kernel's short process name
/// (`pbi_comm`), which carries the application name with spaces:
///
/// * `google chrome` — Google Chrome.app
/// * `microsoft edge` — Microsoft Edge.app
/// * `brave browser` — Brave.app
/// * `arc` — Arc.app (the existing `arc.exe` also covers it, kept for
///   symmetry)
/// * `safari` — the system browser; without it, blocking would not apply to
///   macOS's default browser
///
/// Firefox, Opera and Vivaldi keep their plain binary names on macOS, so the
/// shared list already matches them.
#[cfg(target_os = "macos")]
pub const MACOS_BROWSER_PROCESSES: &[&str] = &[
    "google chrome",
    "microsoft edge",
    "brave browser",
    "arc",
    "safari",
];

/// The browser-process list for the current OS, joined into a rule token
/// list: the shared Windows-style names plus the per-OS aliases.
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn browser_processes() -> String {
    let mut names: Vec<&str> = DEFAULT_BROWSER_PROCESSES.to_vec();
    #[cfg(target_os = "linux")]
    names.extend_from_slice(LINUX_BROWSER_PROCESSES);
    #[cfg(target_os = "macos")]
    names.extend_from_slice(MACOS_BROWSER_PROCESSES);
    names.join(";")
}

/// Windows (and anything without per-OS aliases): the shared list is the
/// whole list — the `.exe` names are what Windows resolves.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn browser_processes() -> String {
    DEFAULT_BROWSER_PROCESSES.join(";")
}

/// Local anonymity/proxy helpers whose external relay would otherwise inherit
/// a browser connection that reached them over loopback. Desktop VPN clients are
/// intentionally *not* listed here: route-level VPN coexistence is enforced by
/// WFP on Windows and policy-routing/nftables on Linux rather than executable
/// names.
pub const LOCAL_PROXY_PROCESSES: &[&str] = &["tor.exe", "torservice.exe", "obfs4proxy.exe"];

/// Domains used by browser-based VPN/proxy extensions. Blocking these at
/// the network level prevents extensions from connecting to their relay
/// servers.
///
/// Scope note (security/false-positive review): this list must contain only
/// *VPN-extension relay/API* domains, never a parent domain shared by
/// unrelated services of the same vendor. `proton.me` is Proton's shared
/// identity domain — `mail.proton.me`, `calendar.proton.me`, `drive.proton.me`,
/// `pass.proton.me`, `account.proton.me` are NOT VPN infrastructure, and the
/// tun-level block applies to every process (including non-browser Proton
/// desktop apps). It was removed here on purpose. Desktop VPN clients are
/// handled structurally by the platform enforcement layer; the browser-extension
/// relay remains covered by the VPN-specific `*.protonvpn.com` domain.
pub const VPN_EXTENSION_DOMAINS: &str = "*.protonvpn.com;*.windscribe.com;*.browsec.com;\
    *.touchvpn.net;*.setupvpn.com;*.urban-vpn.com;*.hola.org;*.hotspotshield.com;\
    *.tunnelbear.com;*.betternet.co;*.zenmate.com;*.vpnunlimited.com;*.vpn.com;\
    *.ivpn.net;*.airvpn.org;*.hidemyass.com;*.cyberghostvpn.com;*.privatetunnel.com;\
    *.speedify.com;*.vpnsecure.me;*.vpnmaster.com;*.hexatech.com;*.vpn-super.com;\
    *.vpnpro.com";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_list_keeps_windows_names() {
        // The shared list must never lose its Windows entries — they are what
        // matches on Windows, and ace-tun's `.exe` normalization makes them
        // match on Unix too.
        for name in DEFAULT_BROWSER_PROCESSES {
            assert!(browser_processes().contains(name));
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_browser_aliases_are_present() {
        let list = browser_processes();
        for name in LINUX_BROWSER_PROCESSES {
            assert!(
                list.contains(name),
                "Linux browser alias {name} missing from rule list"
            );
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_browser_aliases_are_present() {
        let list = browser_processes();
        for name in MACOS_BROWSER_PROCESSES {
            assert!(
                list.contains(name),
                "macOS browser alias {name} missing from rule list"
            );
        }
    }

    /// Regression (Issue 7): the extension-domain block must NOT contain
    /// `proton.me` — Proton's shared identity parent domain. It is the apex of
    /// mail/calendar/drive/pass/account, none of which are VPN relay
    /// infrastructure; the tun-level rule blocks every process (including
    /// non-browser Proton desktop apps), so blocking it would break unrelated
    /// Proton services. Proton VPN's browser-extension relay remains covered
    /// by the VPN-specific `*.protonvpn.com` entry; desktop VPN clients are
    /// handled structurally by the platform enforcement layer.
    #[test]
    fn extension_domains_do_not_block_shared_proton_identity_domain() {
        assert!(
            !VPN_EXTENSION_DOMAINS.contains("proton.me"),
            "proton.me is shared by non-VPN Proton services (mail/drive/pass/calendar) \
             and must not be blocked wholesale"
        );
        assert!(
            VPN_EXTENSION_DOMAINS.contains("*.protonvpn.com"),
            "the Proton VPN extension domain must stay blocked"
        );
    }

    /// The wildcard form is mandatory: a bare apex entry would be compiled as
    /// an exact match in ace-tun, silently never matching any subdomain.
    #[test]
    fn extension_domains_are_wildcard_only() {
        for token in VPN_EXTENSION_DOMAINS.split(';').filter(|t| !t.is_empty()) {
            assert!(
                token.starts_with("*."),
                "VPN extension domain {token:?} must be a wildcard (subdomains only)"
            );
        }
    }
}
