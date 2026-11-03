# Local IPC architecture

Ace Blocker's local control plane uses **remoc 0.20.4** on OS-native byte
streams: **interprocess 2.4.x Windows Named Pipes** on Windows, a Unix-domain
socket on Linux. The engine and watchdog remain independent privileged
services; the Tauri frontend runs as an unprivileged user-session process.

## Control plane

- Engine listener: `\\.\pipe\AceBlockerEngineControlV1` on Windows;
  `/run/ace-blocker-control/engine-control.sock` on Linux.
- `crates/ace-engine/src/service/api/rpc.rs` declares the 63 typed remoc RPC
  operations. They call transport-independent `operations_*.rs` modules,
  preserving lock checks, cloud authorization, shutdown and backup.
- `service/api/transport.rs` establishes remoc connections and serves
  independent clients. There is **no HTTP control listener, OpenAPI service,
  TLS bypass, port discovery or HTTP fallback** in the production runtime.
- `service/api/bridge.rs` maps the Svelte/Tauri boundary to the typed remoc
  client. `apps/ace-app/src-tauri/src/lib.rs` owns the pipe connection and
  locally reads the capability credential. The WebView never receives the token.
- `apps/ace-app/src/lib/engine-client/` is a stable Svelte facade. Site pack
  progress uses typed Rust events over remoc and Tauri Channels, with no SSE
  framing or bytes-to-JSON decoder.
- `ace-redirect` accepts only the allowlisted account API-key callback.
  Uninstall/upgrade uses `ace-engine --request-shutdown`, which invokes the
  same policy-enforcing RPC and fails closed when the engine refuses.

### Authorization

Windows pipe DACL: LocalSystem full access and authenticated local users
read/write only; no remote clients. Only the pipe owner receives the additional
create-instance permission needed by `interprocess` to accept subsequent
connections. In production, the LocalSystem service owns the pipe; a manual
user-mode launch can likewise accept connections without granting this right
to other authenticated users. **Pipe access is not authorization**: every
modifying RPC verifies the installation capability before entering the existing
handler. A successful handshake must never be treated as proof the client is
the genuine Ace App.
The current install-level token defends against arbitrary WebView requests,
but it does **not** cryptographically attest executable identity: another
user-session process that obtains that token could impersonate the application.
Do not advertise executable attestation as an existing protection.

On Linux the root-owned control directory is traversable (0755), its socket
accepts local clients (0666), and privileged operations still require the
installation capability. It is intentionally distinct from the watchdog
runtime directory (0700) and root-only watchdog socket (0600).

### Watchdog

The independent Windows LocalSystem watchdog service keeps its original
SYSTEM-only pipe ACL and confirms the client PID matches the SCM-owned
AceEngine process. The watchdog and engine use remoc typed heartbeat challenge,
response, and bounded maintenance messages. The separate Linux watchdog
uses the Unix socket and confirms the systemd service MainPID. The original
recovery state machine and watchdog lease validation remain authoritative.

### Verification and release gates

- `cargo check --workspace --all-targets`
- `cargo test -p ace-watchdog-protocol`
- `cargo test -p ace-engine --lib` (including typed remoc stream and unlock tests)
- `cd apps/ace-app; bun run test:unit`
- `cd apps/ace-app; bun install --frozen-lockfile; bun run check; bun run build`
- On a disposable Windows VM, verify clean install, SCM startup order,
  named-pipe ACL from standard user and unauthorized users, concurrent clients,
  service restart/reconnect, watchdog heartbeat/recovery, locked shutdown,
  unlocked maintenance-uninstall, and proxy/Wintun enforcement. These VM
  checks are separate from successful Rust compilation; they are required
  for release.
- External website authentication, update traffic, and the MITM/blocking proxy
  deliberately remain HTTP(S).

Native Svelte debugging requires the Tauri runtime; Vite alone cannot access
the Named Pipe. Obsolete HTTP-mocked browser E2E suites have been removed. The
remaining static browser tests do not validate IPC. Native Tauri/Windows
service and installer validation are still required for release.

Avoid introducing custom framing, serialization, request IDs, RPC dispatch
protocols, or HTTP compatibility endpoints. Extend the remoc trait and its
Tauri facade when adding control operations.
