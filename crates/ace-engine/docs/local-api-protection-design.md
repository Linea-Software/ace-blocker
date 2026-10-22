# Local API Protection — Design Note (Resistance Mode v1 Beta)

> **Historical security design (2026-08-31):** This document describes the retired local HTTPS/REST control API. Ace Engine now uses typed remoc RPC over Named Pipe/Unix socket; the capability token remains relevant, but HTTP headers, CORS and port-discovery descriptions below are not the current boundary. See `docs/IPC_ARCHITECTURE.md` and `src/service/api/{rpc,transport,security}.rs`.

**Date:** 2026-08-31
**Status:** Implemented as part of the Resistance Mode v1 beta scope
**Related:** `docs/engine-communication-audit.md` (finding V1), `docs/resistance-mode-audit-2026-08-31.md` (finding S2)

## Problem

The engine's local HTTPS API (`127.0.0.1:<port>`) is unauthenticated. Any
local process — and, because CORS is permissive and the API certificate is in
the machine trust store with a `ace-engine.local` hosts entry, **any website
the user visits** — can issue state-changing requests: create locks, delete
network blocks, replace the API key, trigger downloads, or shut the engine
down. This is a public-launch security requirement and pre-dates Resistance
Mode, but Resistance Mode adds attractive targets (`POST /resistance-mode/set-lock`
up to 90 days, `POST /resistance-mode/download-scripts`).

## Selected protection: per-install capability token + origin allowlist

### 1. Per-install capability token (primary defense)

- The engine generates a 256-bit random token (`getrandom`) on first startup
  and persists it at `{SharedData}\LineaSoftware\AceBlocker\capability.token`
  (the same directory as `port.lock`, which is already hidden at startup by
  `hide_important_files`).
- Every **state-changing request** (method `POST`/`PUT`/`DELETE`/`PATCH`)
  must carry the header `X-Ace-Capability: <token>`.
- Comparison is done on SHA-256 digests of the presented vs. stored token
  (constant-time enough for this threat model; the token itself is never
  logged).
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
- **Debug builds** accept requests without a token (compile-time gate, same
  convention as the existing `/dev/*` endpoints) so browser-based development
  (`localhost:1420` against a debug engine on the fixed port 10567) and the
  existing e2e suite keep working. Release builds enforce strictly.

### 2. Origin allowlist (defense in depth)

- The permissive `CorsLayer` is replaced by a small middleware with two
  behaviors:
  - `/report/*` routes: permissive CORS (required so covered sites can POST
    telemetry from their own origins).
  - All other routes: `Access-Control-Allow-Origin` is echoed only for
    `tauri://localhost`, `http://tauri.localhost`, `https://aceblocker.com`,
    `http://localhost:1420`, `http://localhost:5173` (with `Vary: Origin`);
    preflight (`OPTIONS`) returns 204 only for allowed origins.
- CORS was never a security boundary by itself; the token is. The allowlist
  only removes accidental cross-origin *reads* and shrinks the browser
  attack surface.

### 3. Explicitly not done (and why)

- No per-route ACL table: method-based enforcement is complete and cannot
  drift as new routes are added.
- No rate limiting: out of scope for this pass; the token closes the
  drive-by vector that made rate limiting urgent.
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
- Engine offline → unchanged (app's existing ping-based liveness).

## Tests

See `src/service/api/mod.rs` unit tests and `tests/api_test/`:
- state-changing request without token → 401;
- state-changing request with token → succeeds;
- arbitrary origin (`Origin: https://evil.example`) cannot create a lock
  (token missing → 401; even with a token, CORS preflight is refused);
- `GET /ping` (read-only health) works without a token;
- `/report/usage` remains callable without a token.
