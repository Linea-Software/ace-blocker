# Error Handling Policies

This document defines error-handling rules for Ace Engine.
Use **[eyre](https://docs.rs/eyre)** for error reports.
Use **[thiserror](https://docs.rs/thiserror)** for structured error variants at module boundaries.

---

## 📦 Why `eyre`

| Concern | `eyre` solution |
|---|---|
| **Context** | `.wrap_err("while loading site list")` — attach a human-readable layer at every call site |
| **Root cause** | `.source()` chain preserved through every `wrap_err` / `?` |
| **Backtraces** | `color-eyre` captures a `Backtrace` on every `eyre::Report` at the point of creation |
| **Ad-hoc errors** | `eyre::eyre!("port {} is in use", port)` — no need for a custom error type for one-off failures |
| **Early returns** | `bail!("incompatible config")` and `ensure!(port > 0, "invalid port")` |

---

## 🏗️ Two-layer strategy

```
┌──────────────────────────────────────────────────┐
│  APPLICATION LAYER  (eyre::Result<T>)            │
│  • main(), run(), background tasks               │
│  • Attach context at every ? boundary:           │
│    db.open().wrap_err("failed to open DB")?      │
├──────────────────────────────────────────────────┤
│  DOMAIN / LIBRARY LAYER  (thiserror enums)       │
│  • ApiError, NetworkManagerOpErr, DBError        │
│  • Structured variants for matching & HTTP codes │
│  • impl std::error::Error  →  compose with eyre  │
└──────────────────────────────────────────────────┘
```

- Use **`thiserror`** at **module boundaries** when callers must distinguish error cases.
  Example: map `NetworkBlockIsLocked` → HTTP 423.
- Use **`eyre`** for wiring, glue code, background tasks, and one-shot CLI tools.
  `eyre::Report` wraps any `std::error::Error`, including `thiserror` enums.
  Both error layers can work together.

---

## 🎨 Setup

Install `color-eyre` once in each binary entry point (`main()` or `run()`):

```rust
color_eyre::install()?;
```

This enables automatic `Backtrace` capture and readable error output.
Do **not** call `install()` from library code (`src/lib.rs`). Call it only from binaries.

---

## ✍️ Conventions

### 1. Returning `eyre::Result<T>`

```rust
use eyre::{Result, Context};

fn do_work() -> Result<()> {
    let db = EngineDB::from(&path)
        .wrap_err("failed to open database")?;
    // ...
    Ok(())
}
```

### 2. Adding context at every fallible boundary

Add context to every `?` that crosses an abstraction boundary:

```rust
// ❌  bad — no context, backtrace points at generic "connection refused"
let config = load_config(&path)?;

// ✅  good
let config = load_config(&path)
    .wrap_err_with(|| format!("failed to load config from {}", path.display()))?;
```

### 3. Bailing out

```rust
use eyre::bail;

if port == 0 {
    bail!("no free port available for the proxy");
}
```

### 4. Ensuring invariants

```rust
use eyre::ensure;

ensure!(!path.exists() || overwrite, "refusing to overwrite {}", path.display());
```

### 5. Wrapping `Option<T>`

```rust
use eyre::Context;

let item = maybe_item
    .wrap_err("expected item to exist")?;
```

### 6. Wrapping `thiserror` types

```rust
// DBError implements std::error::Error via thiserror
let result: Result<(), DBError> = db.migrate();

// In application code, just ? and let eyre capture the chain:
db.migrate()
    .wrap_err("database migration failed")?;
```

---

## 🔄 Migration Guide (ongoing)

### Phase 1 — entry points & plumbing

- Replace `Box<dyn std::error::Error>` return types in `run()`, `setup()`, etc.
  with `eyre::Result<T>`.
- Replace `.expect()` and `.unwrap()` with `.wrap_err("...")?`.
  This lets the process exit with a backtrace instead of only a panic message.

### Phase 2 — internal wiring

- Convert functions that currently return `Result<T, String>` or
  `Result<T, Box<dyn Error>>` to `eyre::Result<T>`.
- Preserve existing `thiserror` enums (`ApiError`, `NetworkManagerOpErr`, `DBError`).
  They remain the structured error layer.

### Phase 3 — tool binaries

- Use `eyre::Result<T>` throughout `src/bin/gen_sites_list.rs` and similar one-shot tools.
  Call `color_eyre::install()` at their entry points.

---

## 🧯 Fatal-error policy (unchanged)

When a fatal error occurs:

1. **Log** — Call `error!(%report, "shutting down")`.
   The `%report` specifier captures the full eyre chain and backtrace.
2. **Cleanup** — disable system proxy, stop background tasks, close DB.
3. **User notification** — if running as a service, show a system notification.
4. **Exit** — `std::process::exit(1)`.

---

## 🌐 HTTP API error envelope (machine-readable contract)

Every REST API error response has the shape:

```json
{ "error": "human-readable message", "code": "stable_machine_code" }
```

- Keep `code` **stable across versions**. Clients must use `code` for decisions.
  Do not use message text or HTTP status alone.
- Internal paths, secrets, and stack traces are never included in `error`.
- For malformed JSON, return `code: "invalid_request_body"` through `ValidatedJson`.
  Do not return axum's plain-text 400.

### Stable codes

| Code | HTTP | Meaning |
|---|---|---|
| `not_found` | 404 | Resource not found |
| `bad_request` | 400 | Generic invalid request |
| `invalid_request_body` | 400 | Body failed to parse/validate |
| `invalid_lock_request` | 400 | Lock payload rejected by `Lock::from_raw` |
| `unsupported_lock_state` | 400 | Resistance Mode lock state not in the v1 beta contract (`inactive`, `active_until_lock_end` only) |
| `unsupported_mode` | 400 | Dynamic Resistance Mode requested (not available in v1 beta) |
| `forbidden` | 403 | Generic authorization failure |
| `resistance_locked` | 403 | Action refused because the Resistance Mode lock is active |
| `pro_plan_required` | 403 | Ace Blocker Pro plan required |
| `pro_status_check_failed` | 403 | API key missing/invalid or Pro check failed (e.g. script download without entitlement) |
| `conflict` | 409 | Resource in use — a site pack attached to a network block, or a site pack with a sync already in flight (`SitePackBusy`) |
| `unauthorized` | 401 | Credential required/invalid (including `capability_required`) |
| `capability_required` | 401 | State-changing request without a valid `X-Ace-Capability` token |
| `locked` | 423 | Operation refused while a lock is active (shutdown gate, locked network block, or locked site pack) |
| `internal` | 500 | Unexpected internal failure |
| `scripts_unavailable` | 503 | Script catalog missing/corrupt or reload failed |
| `script_download_failed` | 502 | Download/verify/extract of scripts failed |

---

## 📋 Quick reference

| Situation | Tool |
|---|---|
| I need to return early with a message | `bail!("reason")` |
| I need to wrap an existing error with context | `.wrap_err("context")` |
| I need dynamic context | `.wrap_err_with(|| format!(...))` |
| I need to check a boolean invariant | `ensure!(cond, "message")` |
| My function returns `Option<T>` and I need `Result<T>` | `.wrap_err("expected value")` |
| I'm at a library boundary and callers match on variants | `#[derive(thiserror::Error)]` enum |
| I'm wiring things together (glue code) | `eyre::Result<T>` |
