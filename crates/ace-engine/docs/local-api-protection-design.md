# Local API Protection — Design Note (Resistance Mode v1 Beta)

> **Historical security design (2026-08-31):** This document describes the retired local HTTPS/REST control API. Ace Engine now uses typed remoc RPC over Named Pipe/Unix socket. The capability token remains relevant. The HTTP headers, CORS rules, and port-discovery details below are historical. See `docs/IPC_ARCHITECTURE.md` and `src/service/api/{rpc,transport,security}.rs`.

**Date:** 2026-08-31
**Status:** Implemented as part of the Resistance Mode v1 beta scope
**Related:** `docs/engine-communication-audit.md` (finding V1), `docs/resistance-mode-audit-2026-08-31.md` (finding S2)

## Problem

The historical local HTTPS API (`127.0.0.1:<port>`) had no authentication.
Any local process could send state-changing requests. **Any visited website** could also send them.
The API used permissive CORS, a trusted machine certificate, and an `ace-engine.local` hosts entry.
Attackers could create locks, delete network blocks, replace the API key, trigger downloads, or stop the engine.
This security requirement existed before Resistance Mode.
Resistance Mode added targets: `POST /resistance-mode/set-lock` (up to 90 days) and `POST /resistance-mode/download-scripts`.

## Selected protection: per-install capability token + origin allowlist

### 1. Per-install capability token (primary defense)

- The engine generates a 256-bit random token (`getrandom`) on first startup
  and persists it at `{SharedData}\LineaSoftware\AceBlocker\capability.token`
  (the same directory as `port.lock`, which is already hidden at startup by
  `hide_important_files`).
- Every **state-changing request** (method `POST`/`PUT`/`DELETE`/`PATCH`)
  must carry the header `X-Ace-Capability: <token>`.
- Compare SHA-256 digests of the presented and stored tokens.
  This is sufficiently constant-time for the threat model. Never log the token.
- **Legitimate callers and how they obtain the token:**
  - `ace-app` (Tauri WebView): new Tauri command `get_engine_capability`
    reads the token file; the engine client attaches it to every request.
  - `ace-redirect.exe` / `ace-engine.exe` protocol handler: reads the token
    file and attaches the header when forwarding `aceblocker://` key
    deliveries. The site's `linkEngine()` falls back to the deep link
    automatically when the direct browser POST is refused, so key delivery
    keeps working.
  - Injected boredom scripts (`POST /report/usage`, `POST /report/theme`):
    **exempt** — page contexts cannot hold the token. These routes keep their
    existing strict validation (today-only, valid hostname, monotonic
    seconds < 86400) and are the only documented token-exempt mutating
    routes. Forging usage reports can only escalate the reporter's own
    friction (self-harm only); in v1 beta, punishment state is frozen anyway
    (dynamic mode is disabled and static mode never changes from reports).
- **Debug builds** accept requests without a token through a compile-time gate, as with `/dev/*` endpoints.
  This supports browser development at `localhost:1420` against debug port 10567 and the existing e2e suite.
  Release builds enforce token checks.

### 2. Origin allowlist (defense in depth)

- The permissive `CorsLayer` is replaced by a small middleware with two
  behaviors:
  - `/report/*` routes: permissive CORS (required so covered sites can POST
    telemetry from their own origins).
  - All other routes: `Access-Control-Allow-Origin` is echoed only for
    `tauri://localhost`, `http://tauri.localhost`, `https://aceblocker.com`,
    `http://localhost:1420`, `http://localhost:5173` (with `Vary: Origin`);
    preflight (`OPTIONS`) returns 204 only for allowed origins.
- CORS alone is not a security boundary. The token provides authentication.
  The allowlist prevents accidental cross-origin *reads* and reduces the browser attack surface.

### 3. Explicitly not done (and why)

- No per-route ACL table: method-based enforcement is complete and cannot
  drift as new routes are added.
- No rate limiting: This pass excludes rate limiting. The token closes the drive-by attack path.
- No client certificate authentication: the trust-store CA is machine-wide
  and its private key is on disk; a per-install secret file is the smallest
  mechanism that survives engine restart and is readable by the local
  clients that need it.
- No protection for `GET` routes: they are read-only (health, settings,
  scripts). `GET /open-pro` can open a browser tab — accepted residual risk.

## Failure semantics

- Missing/wrong token on a protected route → `401` with
  `{"error": "...", "code": "capability_required"}`. No state is mutated.
- Token file unreadable at startup → engine generates a fresh one (a stale
  client token stops working; the app re-reads it on next refresh).
- Engine offline → unchanged (the app uses its existing ping-based liveness check).

## Tests

See `src/service/api/mod.rs` unit tests and `tests/api_test/`:
- state-changing request without token → 401;
- state-changing request with token → succeeds;
- arbitrary origin (`Origin: https://evil.example`) cannot create a lock
  (token missing → 401; even with a token, CORS preflight is refused);
- `GET /ping` (read-only health) works without a token;
- `/report/usage` remains callable without a token.
