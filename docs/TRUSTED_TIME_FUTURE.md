# Trusted network time for locks — approved architecture and release gates

**Status:** APPROVED (2026-11-06); initial implementation in `crates/ace-engine/src/trusted_time.rs`. This document separates the decisions now approved from the remaining release verification and deferred possibilities. The feature is **not yet Windows-VM validated**.

## Approved decisions (binding)

- **Primary authority:** Cloudflare Roughtime over UDP, `roughtime.cloudflare.com:2003`; pin the published Ed25519 root `0GD7c3yP8xEc4Zl2zeuN2SlLvDVVocjsPSL8/Rl/7zg=`. Cloudflare describes the service as beta and may rotate the root. Updates to the root require an authenticated, reviewed application release, not downloading and trusting a DNS-supplied key.
- **Authenticated protocol:** the `roughtime 0.2.1` client verifies delegation, Ed25519 signatures, nonce/Merkle binding, delegation validity and its anti-rollback floor. Neither unauthenticated NTP nor HTTP `Date` is an alternative authority. No Ace-hosted endpoint is involved.
- **Cadence:** after a successful sample, refresh every **60 minutes**, not every 5 or 10. Start with one due request on engine startup. Failed attempts use bounded timeout, exponential backoff (starting at 120 seconds, capped near 60 minutes) and jitter. A minute-local wakeup checks whether enough monotonic time has elapsed, including after resume; it does *not* make network requests when not due. Never query per site, network flow, lock check or editor visit. The independent installer `--check-locks` process may perform **one** separately authenticated sample per uninstall check (not an engine polling mechanism).
- **Time model:** every authenticated sample is an interval, not an exact instant. Include signed uncertainty radius, measured total request/response time and bounded oscillator drift (500 ppm) while advancing with monotonic uptime. A lower bound is used to decide whether a lock *has certainly expired*; optimistic timestamps never authorize early unlock. `GetTickCount64` is selected on Windows for time elapsed across sleep/hibernation; the process `Instant` is used on other platforms. Neither is persisted across reboots.
- **Persistence:** save a high-water/rollback floor under shared engine data, independent of local wall time. Updates are serialized across engine and installer with a separate advisory lock file and replace-on-rename, avoiding truncation and stale-process rollback. On the next boot it is only a floor, **never** a reboot-spanning elapsed clock. There is no trusted current time until a fresh verified response. An outage cannot silently substitute an unauthenticated source.
- **Failure behavior:** existing filtering and services continue. Existing timed locks remain active when their expiry cannot be verified; the uninstall gate makes the same conservative decision, while an explicitly inactive state remains inactive. Creating new timed locks requires a valid trusted reference for the 90-day cap. An outage does not globally disable the tunnel, proxy or Internet.
- **Cooldowns:** the five-minute soft-unlock delay uses an in-memory monotonic `Instant`; challenges disappear on service restart. A new timed deadline must be strictly later than the authenticated interval; upgrading a soft lock to a timed lock with less than five minutes remaining is rejected.
- **Time zones:** schedules carry a persisted IANA zone in `network_blocks.schedule_timezone`, captured at block creation; matching uses authenticated UTC converted to that zone via `chrono-tz`, including daylight-saving transitions. Changing Windows time zone does not silently change existing windows. Timed lock deadlines persist as RFC3339 absolute instants and compare as UTC, separately from civil schedule time. Non-authenticated/stale time must not be used to weaken scheduled locks.
- **No change:** never set the Windows clock. Engine, watchdog, Tauri, typed Named Pipe/remoc IPC, installer gates, WinTun/WFP/nftables, and browser-only TLS decryption remain separate.

## Current integration and verification

- `trusted_time.rs`: pin, sampling, refresh/backoff, quality and UTC interval, persisted floor.
- `core/common/lock.rs`, `core/network_block/mod.rs`, `manager/network/`, `manager/resistance/`: lock guards use the shared trusted reference, not local wall time, including Resistance Mode and policy mutations.
- `core/common/unlock_challenge.rs`: monotonic cooldown.
- `lock_check.rs` and `main.rs`: separate read-only SQLite uninstall gate with an independent Roughtime query; the shared *time floor* may be updated separately, but no database writes are made.
- `core/common/schedule.rs`, `db/`, migration `0026_schedule_iana_timezone`: pinned zone and DST semantics; local JSON backup exports and restores it. **Existing rows** initially carry a migration sentinel, which the engine resolves to the host's IANA zone at first upgrade before loading policy. Legacy offset-less timed deadlines are converted once to RFC3339 using that pinned zone, conservatively choosing the later instant for ambiguous DST times. Subsequent Windows timezone changes cannot rewrite the pinned zone or deadlines. If the host timezone was already changed before the first upgrade, the historical zone cannot be recovered; validate such installations before release.
- RPC and frontend lock forms now use offset-bearing UTC timestamps. A new timezone-aware cloud backup schema is **not** yet available: cloud export refuses non-UTC scheduled blocks, and cloud import refuses scheduled blocks when the source zone cannot be established. Local backup remains available.
- Unit/regression tests use synthetic times and forged invalid packets, not Cloudflare production load. Before shipping: native Windows test of GetTickCount64 suspend/hibernate/reboot behavior, named-pipe/installer access and ACL, clean MSI upgrade, Cloudflare UDP blocked, key rotation and affected browser/TUN flows.

## Still deferred or deliberately excluded

1. **Ace backend time endpoint:** not added. A future signed endpoint may be justified by an operational review; Cloudflare would remain an authenticated fallback.
2. **NTS / other authenticated time sources:** not added as hidden fallbacks. NTS bootstrapping requires explicit certificate-time decisions.
3. **OS notifications for resume/reconnect:** no native Windows power/network event subscription yet; the local minute wakeup accounts for elapsed uptime and the failure retry cadence. Event-driven refresh needs native verification, deduplication and network usage review before release.
4. **Complete protection against privileged OS administrators:** out of scope. Public beta-key rotation, unstable UDP paths, memory/disk tampering by administrators, and unsupported IPv6-only DNS resolution must be documented to users and release engineering.
5. **Full cloud backup IANA metadata:** requires a separate Ace Server API/schema change, not a handwritten edit of generated clients. Do not silently reinterpret civil schedules in another zone.

## References

- Cloudflare Roughtime: https://developers.cloudflare.com/time-services/roughtime/
- Published server key and endpoint (UDP/2003): https://developers.cloudflare.com/time-services/roughtime/usage/
- Deprecated older UDP/2002 endpoint: https://developers.cloudflare.com/time-services/roughtime/deprecation/
- Usage recipes: https://developers.cloudflare.com/time-services/roughtime/recipes/
- Windows GetTickCount64: https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-gettickcount64

**Release decision:** code and Linux tests are not proof of native Windows anti-bypass guarantees. Run the bounded Windows VM security, MSI and clock-suspend scenarios before enabling this feature for release.
