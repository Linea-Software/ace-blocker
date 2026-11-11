# mitm-proxy — AGENTS.md

## Overview

**mitm-proxy** is the MITM proxy subcrate in the Ace Blocker workspace at `crates/mitm-proxy`.

## Refactoring boundaries

For AI-led cleanup, follow `../../AGENTS.md` and `../../docs/ai/REFACTORING_PLAYBOOK.md`.
For cross-crate changes, read `../../docs/ai/BOUNDARIES_AND_ABSTRACTIONS.md`.
Use `../../docs/ai/REVIEW_GATES.md` for independent review.
The proxy owns transport and interception logic.
It does **not** own engine policy, UI state, or service lifecycle. Do not introduce cross-crate dependencies only because network utilities appear similar.
First, prove ownership and runtime effects.
Preserve opaque passthrough for unintercepted and non-browser traffic.
Preserve TLS/certificate behavior, streaming, cancellation, and error rules.
Write focused characterization tests before refactoring these behaviors.
Never broaden interception or weaken enforcement during structural cleanup.

## Build & test

```sh
# Build the library used by ACE (no standalone CLI or QUIC listener)
cargo build

# Build the standalone proxy CLI
cargo build --features proxy-cli,http3

# Full test suite (hermetic, 71 tests)
cargo test

# Clippy
cargo clippy --all-targets

# Format
cargo fmt
```

**Windows quirk:** A running `target\debug\mitm-proxy.exe` prevents rebuilding ("Access denied").
Stop the process before rebuilding:

```sh
taskkill //F //IM mitm-proxy.exe   # git-bash
Stop-Process -Name mitm-proxy -Force  # PowerShell
```

## Project structure

```
src/
├── lib.rs          — MitmProxy builder (public API entry point)
├── main.rs         — CLI binary (clap)
├── config.rs       — ProxyConfig (pub) — listen addr, CA paths, h3, verify
├── error.rs        — ProxyError enum + Result<T> alias (pub)
├── inspect.rs      — ConnMeta, Protocol, RequestInspector/ResponseInspector traits (pub)
├── ca.rs           — CertAuthority: load/generate root CA, issue leaf certs (pub)
├── cert_resolver.rs— DynamicCertResolver for rustls ResolvesServerCert
├── tls.rs          — server_config / client_config factory functions
├── http.rs         — HTTP/1.x + HTTP/2 forward proxy, CONNECT, relay pipeline
├── http3.rs        — HTTP/3 (QUIC) listener, request handler
├── upstream.rs     — Upstream::send — forward buffered requests to origin
├── state.rs        — ProxyState / SharedState
└── util.rs         — authority_display, absolute_uri, snapshot_request_head, etc.
tests/
├── common/mod.rs   — Shared test helpers: echo server, proxy launcher, client fns
├── ca_tests.rs     — CA lifecycle integration
├── cli_tests.rs    — Compiled binary assertions (assert_cmd)
├── inspector_tests.rs— Observe, mutate, block semantics
├── proxy_http1.rs  — HTTP/1.1 plaintext + CONNECT
├── proxy_http2.rs  — HTTP/2 MITM + multiplexed streams
├── proxy_http3.rs  — HTTP/3 GET/POST + stream FIN regression (Rust h3 client)
└── tunnel_tests.rs — Narrow-interception invariants: opaque tunnel passthrough
docs/
├── llms.full.txt   — Full LLM reference
TESTING.md          — Test suite documentation
README.md           — Root crate documentation
AGENTS.md           — This file
Cargo.toml          — Crate manifest
```

## Public API

`MitmProxy::new(ProxyConfig)` → builder with `with_request_inspector`,
`with_response_inspector`, `with_inspectors`, `with_intercept_decider`,
`.run()`.

For narrow interception, `with_intercept_decider(Arc<dyn InterceptDecider>)` selects CONNECT tunnels for TLS termination.
Relay rejected tunnels byte-for-byte. Do not set up TLS, issue leaf certificates, or decrypt rejected tunnels.
The default `NoInterceptDecider` intercepts nothing.

### ProxyConfig fields

| Field | Type | Default |
|-------|------|---------|
| `listen_addr` | `SocketAddr` | (required) |
| `http3_addr` | `Option<SocketAddr>` | `None` |
| `ca_cert_path` | `PathBuf` | `{ca_dir}/mitm_ca.pem` |
| `ca_key_path` | `PathBuf` | `{ca_dir}/mitm_ca.key` |
| `install_ca` | `bool` | `false` |
| `verify_upstream` | `bool` | `true` |

### Inspect module (pub)

| Type | Role |
|------|------|
| `Protocol` | Enum: `Http1`, `Http2`, `Http3` |
| `ConnMeta` | Connection metadata: client_addr, protocol, is_tls, authority |
| `RequestAction` | `Continue` or `Respond(BufferedResponse)` for decrypted HTTP |
| `ConnectAction` | `Continue`, `Respond(BufferedResponse)` or `Intercept(BufferedResponse)` for TLS-local synthetic blocks |
| `ResponseAction` | `Continue` |
| `RequestInspector` trait | `fn inspect_request(&self, meta, req) -> RequestAction` |
| `ResponseInspector` trait | `fn inspect_response(&self, meta, req_head, res) -> ResponseAction` |
| `Inspectors` | Bundle holding `Arc<dyn RequestInspector>` + `Arc<dyn ResponseInspector>` |
| `NoopInspector` | Forwards everything unchanged |
| `LoggingInspector` | Logs one-line summaries at INFO |
| `BufferedRequest` | `Request<Bytes>` |
| `BufferedResponse` | `Response<Bytes>` |

## ACE HTTP/3 admission contract

The engine configures `with_http3_sni_policy`.
Admit QUIC/TLS only for hostnames blocked by the current hostname-level policy.
Reject missing or unauthorized SNI **before** issuing a leaf certificate.
Return HTTP 421 for unapproved HTTP/3 `:authority` values. Never forward them upstream. Ordinary origins requiring path, Safe Search or Resistance inspection
use TCP fallback because the existing TCP CONNECT path probes for TLS client
certificates before attempting decryption. Do not relax this selective
admission without a separately verified pre-handshake client-cert check.

The TUN -> loopback HTTP/3 bridge is described in `crates/ace-tun/README.md`.
Native Chrome, Brave, Firefox, Windows WFP, VPN, and IPv6 checks are release gates.
Local Quinn HTTP/3 tests alone do not prove installed traffic enforcement.

## HTTP/3 stream-FIN bug

**Root cause:** h3 0.0.8 sends GREASE frames during `finish()` on the first request per connection.
Strict clients (aioquic) do not handle the GREASE-frame-then-FIN sequence correctly.

**Fix:** `.send_grease(false)` on the h3 connection builder
(`src/http3.rs:68`). Verified by `tests/proxy_http3.rs::http3_stream_finishes_cleanly`.

## Test conventions

- **Hermetic:** all tests use local echo servers on `127.0.0.1:0`. No network
  required.
- **Proxy runs in-process** via `MitmProxy::new(...)`.
- **CLI tests** launch the compiled binary via `assert_cmd`.
- **H3 tests** use a Rust in-process h3 client (h3 + h3-quinn + quinn).

## Upstream config: testing with self-signed origins

When the proxy needs to reach an origin with a self-signed cert (common in
integration tests), pass `verify_upstream(false)`:

```rust
ProxyConfig::new(listen_addr, ca_dir)
    .verify_upstream(false);
```

This installs a `NoVerify` certificate verifier.
It skips chain validation but still checks cryptographic signatures.

## Key dependency versions

| Crate | Version | Notes |
|-------|---------|-------|
| h3 | 0.0.8 | HTTP/3 protocol |
| h3-quinn | 0.0.10 | h3 transport over quinn |
| quinn | 0.11.11 | QUIC transport |
| rustls | 0.23.42 | ring provider, tls12 |
| rcgen | 0.14.8 | features pem, x509-parser |
| hyper | 1 | features full |
| hyper-util | 0.1.20 | features full |
| tokio-rustls | 0.26.4 | ring, logging, tls12 |
| x509-parser | 0.17 | features verify (dev only) |
| tokio | 1.53.1 | features full |

## Rebuilding while process is running

The compiled binary is `target/debug/mitm-proxy.exe`.
Cargo cannot overwrite this file while it runs.
Stop the process first, or use `cli_tests` to manage its lifecycle.
