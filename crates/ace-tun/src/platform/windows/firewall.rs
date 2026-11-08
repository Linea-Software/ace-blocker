//! Windows Filtering Platform guard for route-bypass resistance.
//!
//! WinTun remains the packet transport, while WFP is the security boundary for
//! protected applications. A protected browser may connect through ACE's WinTun
//! interface (or to explicitly local/private destinations), but a VPN cannot
//! make that browser escape ACE by installing a more-specific route.
//!
//! Filters live in a dynamic WFP session owned by a worker thread. The worker
//! periodically discovers newly launched protected executables and adds their
//! path-based ALE filters. Closing the session removes every filter immediately.

use std::collections::HashSet;
use std::io;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use wfp::{
    ActionType, AppIdConditionBuilder, FilterBuilder, FilterEngine, FilterEngineBuilder,
    FilterWeight, GUID, InterfaceConditionBuilder, IpAddressConditionBuilder, Layer,
    SubLayerBuilder, Transaction,
};

use super::process;
use crate::wfp_policy::{FilterStep, filter_steps};

const ACE_SUBLAYER_GUID: GUID = GUID {
    data1: 0x62d9_35e7,
    data2: 0x8f4d,
    data3: 0x4cc3,
    data4: [0x91, 0xe5, 0x34, 0x72, 0x15, 0xa1, 0x6c, 0x0e],
};

const SUBLAYER_WEIGHT: u16 = 0xf000;
const PERMIT_WEIGHT: u64 = 100;
const BLOCK_WEIGHT: u64 = 10;
const PROCESS_REFRESH_INTERVAL: Duration = Duration::from_millis(750);

pub(crate) struct WfpGuard {
    stop_tx: Option<mpsc::Sender<()>>,
    worker: Option<thread::JoinHandle<()>>,
}

impl WfpGuard {
    pub(crate) fn install(
        tun_luid: u64,
        ipv6: bool,
        protected_processes: &[String],
    ) -> io::Result<Self> {
        let protected_names = normalize_protected_names(protected_processes);

        if protected_names.is_empty() {
            // Keep the crate usable for callers that only want transparent TUN
            // transport. ace-engine always supplies its browser list.
            return Ok(Self {
                stop_tx: None,
                worker: None,
            });
        }

        let (stop_tx, stop_rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);

        let worker = thread::Builder::new()
            .name("ace-wfp-guard".into())
            .spawn(move || {
                if let Err(error) = guard_worker(tun_luid, ipv6, protected_names, stop_rx, ready_tx)
                {
                    tracing::error!("WFP enforcement worker stopped: {error}");
                }
            })?;

        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Self {
                stop_tx: Some(stop_tx),
                worker: Some(worker),
            }),
            Ok(Err(error)) => {
                let _ = worker.join();
                Err(error)
            }
            Err(_) => {
                let _ = worker.join();
                Err(io::Error::other(
                    "WFP enforcement worker exited before initialization",
                ))
            }
        }
    }
}

impl Drop for WfpGuard {
    fn drop(&mut self) {
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn guard_worker(
    tun_luid: u64,
    ipv6: bool,
    protected_names: HashSet<String>,
    stop_rx: mpsc::Receiver<()>,
    ready_tx: mpsc::SyncSender<io::Result<()>>,
) -> io::Result<()> {
    let mut engine = match initialize_engine() {
        Ok(engine) => engine,
        Err(error) => {
            let _ = ready_tx.send(Err(copy_error(&error)));
            return Err(error);
        }
    };

    let mut protected_paths = HashSet::new();
    if let Err(error) = refresh_protected_paths(
        &mut engine,
        tun_luid,
        ipv6,
        &protected_names,
        &mut protected_paths,
    ) {
        let _ = ready_tx.send(Err(copy_error(&error)));
        return Err(error);
    }

    let _ = ready_tx.send(Ok(()));

    loop {
        match stop_rx.recv_timeout(PROCESS_REFRESH_INTERVAL) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }

        if let Err(error) = refresh_protected_paths(
            &mut engine,
            tun_luid,
            ipv6,
            &protected_names,
            &mut protected_paths,
        ) {
            tracing::warn!("could not refresh protected WFP applications: {error}");
        }
    }
}

fn initialize_engine() -> io::Result<FilterEngine> {
    let mut engine = FilterEngineBuilder::default().dynamic().open()?;
    let transaction = Transaction::new(&mut engine)?;
    SubLayerBuilder::default()
        .name("Ace Blocker enforcement")
        .description("Dynamic anti-bypass filters for protected applications")
        .guid(ACE_SUBLAYER_GUID)
        .weight(SUBLAYER_WEIGHT)
        .add(&transaction)?;
    transaction.commit()?;
    Ok(engine)
}

fn refresh_protected_paths(
    engine: &mut FilterEngine,
    tun_luid: u64,
    ipv6: bool,
    protected_names: &HashSet<String>,
    protected_paths: &mut HashSet<PathBuf>,
) -> io::Result<()> {
    let mut candidates = known_install_paths(protected_names);
    candidates.extend(process::running_process_paths(protected_names));
    candidates.sort();
    candidates.dedup();

    let new_paths: Vec<PathBuf> = candidates
        .into_iter()
        .filter(|path| !protected_paths.contains(path))
        .collect();

    if new_paths.is_empty() {
        return Ok(());
    }

    let transaction = Transaction::new(engine)?;
    for path in &new_paths {
        add_application_filters(&transaction, path, tun_luid, ipv6)?;
    }
    transaction.commit()?;

    for path in new_paths {
        tracing::info!(path = %path.display(), "WFP protection enabled for application");
        protected_paths.insert(path);
    }
    Ok(())
}

fn add_application_filters(
    transaction: &Transaction<'_>,
    executable: &Path,
    tun_luid: u64,
    ipv6: bool,
) -> io::Result<()> {
    // The flag means "IPv6 is captured by WinTun", NOT "IPv6 needs WFP
    // enforcement". An IPv4-only host may gain a physical IPv6 route after
    // startup. Keep the local IPv6 permits and external IPv6 block even when
    // WinTun has no IPv6 address/routes, so browser traffic fails closed.
    for step in filter_steps(ipv6) {
        match step {
            FilterStep::PermitTunV4 => {
                add_tunnel_permit(transaction, Layer::ConnectV4, executable, tun_luid, "IPv4")?
            }
            FilterStep::PermitLocalV4 => add_local_ipv4_permit(transaction, executable)?,
            FilterStep::BlockOtherV4 => {
                add_application_block(transaction, Layer::ConnectV4, executable, "IPv4")?
            }
            FilterStep::PermitTunV6 => {
                add_tunnel_permit(transaction, Layer::ConnectV6, executable, tun_luid, "IPv6")?
            }
            FilterStep::PermitLocalV6 => add_local_ipv6_permit(transaction, executable)?,
            FilterStep::BlockOtherV6 => {
                add_application_block(transaction, Layer::ConnectV6, executable, "IPv6")?
            }
        }
    }

    Ok(())
}

fn add_tunnel_permit(
    transaction: &Transaction<'_>,
    layer: Layer,
    executable: &Path,
    tun_luid: u64,
    family_name: &str,
) -> io::Result<()> {
    FilterBuilder::default()
        .name(format!("ACE protected tunnel permit {family_name}"))
        .description("Protected application may connect when Windows selected ACE's tunnel")
        .action(ActionType::Permit)
        .layer(layer)
        .sublayer(ACE_SUBLAYER_GUID)
        .weight(FilterWeight::Exact(PERMIT_WEIGHT))
        .condition(AppIdConditionBuilder::default().equal(executable)?.build())
        .condition(InterfaceConditionBuilder::local().luid(tun_luid).build())
        .add(transaction)?;
    Ok(())
}

fn add_local_ipv4_permit(transaction: &Transaction<'_>, executable: &Path) -> io::Result<()> {
    for (network, prefix, label) in [
        (Ipv4Addr::new(127, 0, 0, 0), 8, "loopback"),
        (Ipv4Addr::new(10, 0, 0, 0), 8, "rfc1918-10"),
        (Ipv4Addr::new(172, 16, 0, 0), 12, "rfc1918-172"),
        (Ipv4Addr::new(192, 168, 0, 0), 16, "rfc1918-192"),
        (Ipv4Addr::new(169, 254, 0, 0), 16, "link-local"),
        (Ipv4Addr::new(224, 0, 0, 0), 4, "multicast"),
    ] {
        add_local_ipv4_subnet_permit(transaction, executable, network, prefix, label)?;
    }
    Ok(())
}

fn add_local_ipv4_subnet_permit(
    transaction: &Transaction<'_>,
    executable: &Path,
    network: Ipv4Addr,
    prefix: u8,
    label: &str,
) -> io::Result<()> {
    // WFP filter conditions are ANDed on modern Windows. Keep each destination
    // subnet in its own filter instead of relying on the old same-field OR
    // behavior that only applied to Windows 7 / Server 2008 R2.
    FilterBuilder::default()
        .name(format!("ACE protected local IPv4 permit {label}"))
        .description("Keep explicitly local-scope IPv4 destinations reachable")
        .action(ActionType::Permit)
        .layer(Layer::ConnectV4)
        .sublayer(ACE_SUBLAYER_GUID)
        .weight(FilterWeight::Exact(PERMIT_WEIGHT))
        .condition(AppIdConditionBuilder::default().equal(executable)?.build())
        .condition(
            IpAddressConditionBuilder::remote()
                .subnet_v4(network, prefix)
                .build(),
        )
        .add(transaction)?;
    Ok(())
}

fn add_local_ipv6_permit(transaction: &Transaction<'_>, executable: &Path) -> io::Result<()> {
    for (network, prefix, label) in [
        (Ipv6Addr::LOCALHOST, 128, "loopback"),
        (
            Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, 0),
            7,
            "unique-local",
        ),
        (Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 0), 10, "link-local"),
        (Ipv6Addr::new(0xff00, 0, 0, 0, 0, 0, 0, 0), 8, "multicast"),
    ] {
        add_local_ipv6_subnet_permit(transaction, executable, network, prefix, label)?;
    }
    Ok(())
}

fn add_local_ipv6_subnet_permit(
    transaction: &Transaction<'_>,
    executable: &Path,
    network: Ipv6Addr,
    prefix: u8,
    label: &str,
) -> io::Result<()> {
    FilterBuilder::default()
        .name(format!("ACE protected local IPv6 permit {label}"))
        .description("Keep explicitly local-scope IPv6 destinations reachable")
        .action(ActionType::Permit)
        .layer(Layer::ConnectV6)
        .sublayer(ACE_SUBLAYER_GUID)
        .weight(FilterWeight::Exact(PERMIT_WEIGHT))
        .condition(AppIdConditionBuilder::default().equal(executable)?.build())
        .condition(
            IpAddressConditionBuilder::remote()
                .subnet_v6(network, prefix)
                .build(),
        )
        .add(transaction)?;
    Ok(())
}

fn add_application_block(
    transaction: &Transaction<'_>,
    layer: Layer,
    executable: &Path,
    family_name: &str,
) -> io::Result<()> {
    FilterBuilder::default()
        .name(format!("ACE protected bypass block {family_name}"))
        .description("Protected application cannot select a path outside ACE")
        .action(ActionType::Block)
        .layer(layer)
        .sublayer(ACE_SUBLAYER_GUID)
        .weight(FilterWeight::Exact(BLOCK_WEIGHT))
        .condition(AppIdConditionBuilder::default().equal(executable)?.build())
        .add(transaction)?;
    Ok(())
}

fn normalize_protected_names(processes: &[String]) -> HashSet<String> {
    processes
        .iter()
        .map(|name| name.trim())
        .filter(|name| !name.is_empty() && !name.contains(['*', '?']))
        .map(str::to_ascii_lowercase)
        .collect()
}

/// Resolve common installed-browser locations before the processes start.
///
/// The process watcher covers portable/custom installs after launch; these
/// candidates close the startup race for standard installations.
fn known_install_paths(protected_names: &HashSet<String>) -> Vec<PathBuf> {
    let program_files = std::env::var_os("ProgramFiles").map(PathBuf::from);
    let program_files_x86 = std::env::var_os("ProgramFiles(x86)").map(PathBuf::from);
    let local_app_data = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);

    let mut paths = Vec::new();
    for name in protected_names {
        let relative_candidates: &[(&Option<PathBuf>, &str)] = match name.as_str() {
            "chrome.exe" => &[
                (&program_files, r"Google\Chrome\Application\chrome.exe"),
                (&program_files_x86, r"Google\Chrome\Application\chrome.exe"),
                (&local_app_data, r"Google\Chrome\Application\chrome.exe"),
            ],
            "msedge.exe" => &[
                (&program_files, r"Microsoft\Edge\Application\msedge.exe"),
                (&program_files_x86, r"Microsoft\Edge\Application\msedge.exe"),
                (&local_app_data, r"Microsoft\Edge\Application\msedge.exe"),
            ],
            "firefox.exe" => &[
                (&program_files, r"Mozilla Firefox\firefox.exe"),
                (&program_files_x86, r"Mozilla Firefox\firefox.exe"),
            ],
            "brave.exe" => &[
                (
                    &program_files,
                    r"BraveSoftware\Brave-Browser\Application\brave.exe",
                ),
                (
                    &program_files_x86,
                    r"BraveSoftware\Brave-Browser\Application\brave.exe",
                ),
                (
                    &local_app_data,
                    r"BraveSoftware\Brave-Browser\Application\brave.exe",
                ),
            ],
            "vivaldi.exe" => &[
                (&program_files, r"Vivaldi\Application\vivaldi.exe"),
                (&local_app_data, r"Vivaldi\Application\vivaldi.exe"),
            ],
            "opera.exe" => &[
                (&local_app_data, r"Programs\Opera\opera.exe"),
                (&local_app_data, r"Programs\Opera GX\opera.exe"),
                (&program_files, r"Opera\opera.exe"),
            ],
            "arc.exe" => &[(&local_app_data, r"Programs\Arc\Arc.exe")],
            _ => &[],
        };

        for (root, relative) in relative_candidates {
            if let Some(root) = root {
                let path = root.join(relative);
                if path.is_file() {
                    paths.push(path);
                }
            }
        }
    }

    paths
}

fn copy_error(error: &io::Error) -> io::Error {
    match error.raw_os_error() {
        Some(code) => io::Error::from_raw_os_error(code),
        None => io::Error::new(error.kind(), error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protected_names_are_case_insensitive_and_skip_globs() {
        let names = normalize_protected_names(&[
            "Chrome.EXE".to_string(),
            "fire*.exe".to_string(),
            "msedge.exe".to_string(),
        ]);
        assert!(names.contains("chrome.exe"));
        assert!(names.contains("msedge.exe"));
        assert!(!names.contains("fire*.exe"));
    }
}
