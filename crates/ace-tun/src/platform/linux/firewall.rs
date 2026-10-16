//! Linux nftables guard for route-bypass resistance.
//!
//! Routing sends ordinary local traffic into ACE's TUN. This guard is the
//! independent security boundary underneath that routing policy: while ACE is
//! healthy, non-root local traffic may leave only through loopback, the ACE TUN,
//! explicitly local-scope destinations, or an ACE-marked relay socket.
//!
//! The final drop verdict is stored in a timed verdict map. A heartbeat refresh
//! renews it periodically. If the process is killed before cleanup can run, the
//! element expires and the otherwise-accepting table becomes inert, preventing
//! a stale firewall from stranding the machine offline.

use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use super::ACE_EGRESS_MARK;

const TABLE_NAME: &str = "ace_blocker_guard";
const HEARTBEAT_KEY_EXPR: &str = "numgen inc mod 1";
const HEARTBEAT_TIMEOUT_SECS: u64 = 15;
const HEARTBEAT_REFRESH_SECS: u64 = 5;

/// Live ownership of ACE's nftables guard.
///
/// Dropping this value stops the heartbeat and removes the ACE-owned table.
pub(crate) struct FirewallGuard {
    stop_tx: Option<mpsc::Sender<()>>,
    heartbeat: Option<thread::JoinHandle<()>>,
}

impl FirewallGuard {
    /// Install the guard and start its crash-safety heartbeat.
    pub(crate) fn install() -> io::Result<Self> {
        // A previous process may have crashed and left an inert table behind.
        // Deleting a missing table is expected, so perform this as a best-effort
        // operation separate from the transactional install.
        let _ = run_nft(&format!("delete table inet {TABLE_NAME}\n"));

        run_nft(&install_script())?;

        let (stop_tx, stop_rx) = mpsc::channel();
        let heartbeat = thread::Builder::new()
            .name("ace-nft-heartbeat".into())
            .spawn(move || heartbeat_loop(stop_rx))?;

        Ok(Self {
            stop_tx: Some(stop_tx),
            heartbeat: Some(heartbeat),
        })
    }
}

impl Drop for FirewallGuard {
    fn drop(&mut self) {
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }
        if let Some(heartbeat) = self.heartbeat.take() {
            let _ = heartbeat.join();
        }
        if let Err(error) = run_nft(&format!("delete table inet {TABLE_NAME}\n")) {
            tracing::debug!("nftables guard cleanup returned {error}");
        }
    }
}

fn heartbeat_loop(stop_rx: mpsc::Receiver<()>) {
    loop {
        match stop_rx.recv_timeout(Duration::from_secs(HEARTBEAT_REFRESH_SECS)) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => return,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }

        if let Err(error) = run_nft(&refresh_script()) {
            // Do not spin or attempt to reconstruct arbitrary system firewall
            // state. If refresh remains unavailable, the timed verdict expires
            // and the guard deliberately fails open while ACE's capture routing
            // continues to provide the primary path.
            tracing::error!("failed to refresh nftables guard heartbeat: {error}");
        }
    }
}

fn install_script() -> String {
    format!(
        "table inet {TABLE_NAME} {{\n\
           map heartbeat {{ typeof {HEARTBEAT_KEY_EXPR} : verdict; flags timeout; timeout {HEARTBEAT_TIMEOUT_SECS}s; \
             elements = {{ 0 timeout {HEARTBEAT_TIMEOUT_SECS}s : drop }}; }}\n\
           chain output {{\n\
             type filter hook output priority -150; policy accept;\n\
             oifname \"lo\" accept\n\
             oifname \"ace*\" accept\n\
             meta skuid 0 accept\n\
             meta mark {ACE_EGRESS_MARK:#x} accept\n\
             ip daddr 169.254.0.0/16 accept\n\
             ip daddr 224.0.0.0/4 accept\n\
             ip daddr 255.255.255.255 accept\n\
             ip6 daddr fe80::/10 accept\n\
             ip6 daddr ff00::/8 accept\n\
             {HEARTBEAT_KEY_EXPR} vmap @heartbeat\n\
           }}\n\
         }}\n"
    )
}

fn refresh_script() -> String {
    format!(
        "destroy element inet {TABLE_NAME} heartbeat {{ 0 }}\n\
         add element inet {TABLE_NAME} heartbeat {{ 0 timeout {HEARTBEAT_TIMEOUT_SECS}s : drop }}\n"
    )
}

fn run_nft(script: &str) -> io::Result<()> {
    let mut child = Command::new("nft")
        .args(["-f", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("failed to launch nftables command: {error}"),
            )
        })?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(script.as_bytes())?;
    }
    drop(child.stdin.take());

    let output = child.wait_with_output()?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(io::Error::other(format!(
        "nftables transaction failed ({}): {}",
        output.status,
        stderr.trim()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_script_has_fail_open_timed_verdict() {
        let script = install_script();
        assert!(script.contains(&format!("typeof {HEARTBEAT_KEY_EXPR} : verdict")));
        assert!(script.contains("flags timeout"));
        assert!(script.contains(&format!("{HEARTBEAT_KEY_EXPR} vmap @heartbeat")));
        assert!(!script.contains("type integer : verdict"));
        assert!(script.contains(&format!("meta mark {ACE_EGRESS_MARK:#x} accept")));
    }

    #[test]
    fn refresh_is_one_transaction_with_destroy_then_add() {
        let script = refresh_script();
        assert!(script.starts_with("destroy element"));
        assert!(script.contains("add element"));
        assert!(script.contains(&format!("timeout {HEARTBEAT_TIMEOUT_SECS}s")));
    }
}
