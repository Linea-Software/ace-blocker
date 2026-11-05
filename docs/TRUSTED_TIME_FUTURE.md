# Trusted network time for locks — future design note

**Status:** DEFERRED / DESIGN REFERENCE ONLY (2026-11-05). No implementation or polling frequency is approved by this document.
**Scope:** Desktop engine on Windows first; adapt separately for Linux/macOS. This is not a task to implement now.

## Product direction and rationale

Ace Blocker should eventually evaluate time-sensitive lock decisions using authenticated external UTC time, rather than blindly trusting user-adjustable OS wall time. This would address clock/date changes and related schedule/lock bypasses without a proprietary kernel driver.

**Do not run frequent per-user time requests against aceblocker.com on the current small VPS.** The earlier suggestion of polling the Ace backend every ten minutes was rejected as too expensive for a simple time service. A hosted public time service, preferably Cloudflare, should be evaluated first.

The preferred future architecture is:

1. **Initially:** an authenticated Cloudflare time source used directly by the desktop engine, with a cached trusted-time anchor and a monotonic clock between network samples.
2. **Possibly later:** an authenticated/signed time endpoint on aceblocker.com, if operational capacity justifies it, with Cloudflare retained as fallback. The order of precedence would change only in that later phase.
3. **Currently:** existing behavior remains unchanged. Neither Cloudflare integration nor an Ace endpoint is authorized or implemented.

## Candidate Cloudflare protocols (verify before implementation)

- **Roughtime (preferred candidate):** Cloudflare documents `roughtime.cloudflare.com:2003` over UDP, returning cryptographically signed time. Its signature verification can bootstrap trustworthy time even if the local OS clock is sufficiently wrong to break ordinary HTTPS certificate validation. The protocol provides approximate time with an uncertainty interval, not a guaranteed exact UTC instant. Validate nonce/response binding, signature/delegation, uncertainty, and replay resistance.
- **Cloudflare NTS:** `time.cloudflare.com` supports Network Time Security (RFC 8915). NTS authenticates NTP, but its TLS-based key exchange raises a bootstrapping question when the Windows clock is badly wrong. Evaluate as an alternative or second authenticated source rather than assuming it solves the initial bad-clock problem.
- **Plain NTP:** `time.cloudflare.com` serves ordinary NTP, but unauthenticated NTP is not sufficient on its own for a security-sensitive unlock/expiry decision.

Cloudflare Roughtime is **beta**: its published root public key can change. A future client needs an explicit, authenticated key trust/rotation policy; do not silently accept arbitrary public keys fetched through untrusted DNS. The documented endpoint and port must also be rechecked before shipping. Public accessibility is broad, but UDP/2003 may be blocked on captive portals, enterprise networks or some firewalls; connectivity to the web does not prove connectivity to Roughtime.

Official references (checked 2026-11-05):
- https://developers.cloudflare.com/time-services/roughtime/usage/
- https://developers.cloudflare.com/time-services/roughtime/recipes/
- https://developers.cloudflare.com/time-services/roughtime/
- https://developers.cloudflare.com/time-services/nts/
- https://www.cloudflare.com/time/

## Future time model (proposal, not an approved contract)

- Never change Windows' system clock. The engine should maintain its **own** trusted UTC estimate.
- On a validated time reply, record the authenticated UTC interval (including uncertainty), a monotonic-clock reading and the relevant boot/session identity. Within the same boot, advance the estimate using elapsed monotonic time, not `chrono::Local::now()`. On Windows, evaluate `GetTickCount64` for elapsed time across sleep/hibernation; verify its behavior and limitations in tests.
- Preserve a protected last-known-good time/high-watermark as an additional anti-rollback signal. **A monotonic counter cannot bridge a restart on its own.** A new boot with no trustworthy fresh time must not conclude that an existing lock has expired based only on the adjustable Windows clock.
- Represent lock expiry in UTC. For wall-clock schedules, explicitly choose an IANA time zone at configuration time and evaluate day/time in that zone, rather than accepting arbitrary user-initiated system time-zone changes.
- Differentiate **lock expiry** (security decision) from ordinary **network availability**. A time-service outage must not automatically disable blocking or permanently cut off the network. Any fail-closed behavior should be narrowly scoped to time-dependent unlock/expiry and uninstall checks, and must specify user recovery and support consequences.
- Respect time uncertainty and network RTT, especially near a lock deadline; design deterministic handling for indeterminate boundary intervals and large forward/backward jumps.
- Recognize the stated limit: an administrator with control over system binaries/network can defeat a local app; the goal is to remove easy clock-based bypasses, not to claim resistance against an omnipotent local admin.

## Query cadence: decide by measurement, not by a fixed ten-minute timer

**No final polling cadence is selected.** The ten-minute VPS-polling proposal should not be used as a requirement.

Candidate policy for later prototyping:
- Refresh once at service start when network access permits.
- Prefer event-triggered refresh after reconnect, resume, abnormal clock change, or when an expiry/unlock decision requires higher confidence.
- Between validated responses, use the monotonic clock; there is no need to poll every few minutes merely to defend against manual changes to the OS clock.
- Consider infrequent periodic refresh (e.g. a configurable, jittered 1–6 hour interval) only after testing drift, uptime, data cost, reliability and provider guidance. This range is a study input, **not** a shipping default.
- Use bounded timeouts, limited retries with exponential backoff/jitter, no query storms and no requirement that Cloudflare always be reachable. Re-evaluate provider usage limits/terms before a public rollout.
- If the public source is unavailable, preserve the last trusted anchor where meaningful; on restart without validated elapsed time, do not treat an adjustable wall-clock reading as authorization to remove an active lock.

## Existing code touchpoints for any future design

- `crates/ace-engine/src/core/common/lock.rs`: `ActiveUntilLockEnd` is checked against a caller-provided local time.
- `crates/ace-engine/src/core/common/schedule.rs`: schedule windows are based on local wall time.
- `crates/ace-engine/src/core/common/unlock_challenge.rs`: the five-minute cooldown uses `Local::now()` and is in-memory only.
- `crates/ace-engine/src/manager/resistance/manager.rs`: Resistance Mode lock expiry uses `Local::now()`.
- `crates/ace-engine/src/lock_check.rs`: the installer's independent read-only uninstall gate checks timed-lock end against `Local::now()`; it must not diverge from the engine's trusted-time decision model.
- `crates/ace-engine/src/manager/account.rs`: Pro entitlements already implement a persisted clock high-watermark, but that is **not** a general trusted clock for locks.
- `crates/ace-watchdog/src/main.rs`: existing `Instant` use for watchdog heartbeat is useful precedent, but it does not establish a trusted clock for the engine.

Because timed locks and the uninstall gate are security/runtime contracts, any future implementation needs an explicit design approval covering persistence across restart, offline behavior, time-zone semantics, failure recovery and testing.

## Tests required before enabling

Use an isolated Windows VM with synthetic blocks and disabled external production dependencies where possible:
- Advance/rollback the wall clock and change the system time zone while locks/schedules are active.
- Suspend/hibernate, resume, reboot with/without network, and reconnect after a clock jump.
- Test network available but Cloudflare UDP/2003 unavailable, high RTT, malformed/unsigned/stale Roughtime replies, and published-key rotation.
- Check lock expiry/unlock boundaries, Resistance Mode, schedule activation and the MSI uninstall gate all agree.
- Confirm failed time sync does not disable filtering or trap users in an unrecoverable state.

**Non-goal now:** add dependencies, alter engine logic, deploy Ace backend endpoints, require kernel access, or change user-facing lock semantics.
