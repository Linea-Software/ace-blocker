# Build performance audit (2026-11-01)

This audit covers the independent `ace-blocker` workspace and Tauri/SvelteKit
app. The numbers below are resolved dependency-graph entries, **not cold-build
seconds**; they are not claims about how many minutes the CI saves.

| Metric | Before | After |
| --- | ---: | ---: |
| Root `Cargo.lock` entries (all targets/features) | 892 | 854 |
| Families with multiple locked versions | 81 | 68 |
| `ace-engine` Linux normal + build graph entries | 470 | 411 |
| `ace-app` Linux normal + build graph entries | 646 | 570 |

Use `cargo tree -p ace-engine --target x86_64-unknown-linux-gnu -e normal,build`
(and the equivalent command for `ace-app`) to inspect the production graph.
`cargo tree -d` shows duplicates; the lockfile contains platform-specific,
optional, and test dependencies that are not all compiled at once.

## Official Cargo guidance and profile decision

- [Optimizing Build Performance](https://doc.rust-lang.org/cargo/guide/build-performance.html)
  recommends pruning dependencies/features, reducing debug symbols and
  measuring the actual critical path.
- [Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)
  describes why dev/test/release outputs differ and why build scripts and
  proc macros already default to quick compilation (`opt-level = 0`).
- [Cargo features](https://doc.rust-lang.org/cargo/reference/features.html)
  describes optional dependencies and feature activation.
- [Cargo timings](https://doc.rust-lang.org/cargo/reference/timings.html)
  shows slow compilation units, duplicate versions and blocked parallelism.

**No Cargo profile settings were changed.** Dev already disables dependency
debug info. Rust's default `opt-level = 0` favors fast debug builds, but ACE
uses `opt-level = 1` for dependencies, which could improve runtime test speed.
Changing it without representative evidence would be speculative. Release's
`opt-level = "z"` and `lto = "thin"` prioritize compact shipped artifacts;
changing them would be a product tradeoff. Windows already uses `lld-link.exe`.

The official guide also suggests `debug = "line-tables-only"` for *workspace*
code when backtraces are useful. Here `debug = false` remains intact; no
nightly compiler flags, experimental backend, or linker tweaks were added.

## CI and measurements

- Frontend assets are built once, uploaded, and reused by both native jobs.
  The prerequisite can increase the critical path even as it reduces duplicate
  work; check GitHub Actions workflow *wall clock*, not aggregate CPU time.
- PRs retain Rust all-target checks and existing selected tests. Release-profile
  validation runs on `main` and manual dispatch instead of every PR.
- A separate non-PR job runs the opt-in proxy CLI and HTTP/3 tests.
- Native checks save `cargo --timings` HTML reports to CI artifacts. Compare
  cold vs warm cache runs on each OS before adjusting profiles.
- [rust-cache](https://github.com/Swatinem/rust-cache) is already configured;
  keep OS and dependency/feature-specific cache keys distinct.

## Deferred architectural decision: separate the Tauri RPC client

Tauri currently imports the entire `ace-engine` package to access the typed
`service::api::bridge`, `RpcError` and site-pack streaming status. This can
pull networking, SQLite and proxy dependencies into the Tauri graph even
though Tauri cannot run the enforcement service. An engine-independent typed
RPC client/contract crate could reduce this cost, but it is not a safe
manifest-only change.

Today, `rpc.rs` combines the remoc trait and privileged server implementation;
`bridge.rs` dispatches typed methods and imports engine core/manager types;
`schemas.rs` re-exports core types; `transport.rs` owns both client connects
and server listeners. Extraction requires explicit ownership, serialization
compatibility and cross-process testing.

**Proposed ownership:** a protocol crate owns wire DTOs and the typed remoc
trait; Tauri owns only the client-side transport, capability-file reading and
WebView operation restrictions; the engine owns all server implementations,
lock policy, ACL enforcement and service lifecycle. No REST, JSON RPC fallback
or shutdown privilege may move into the UI.

Before implementation, inventory domain DTO dependencies, freeze wire-format
contract tests, exercise streaming and error envelopes on Windows named pipes
and Linux sockets, assert WebView shutdown remains denied, and compare
`cargo tree` and `--timings` on both platforms.

Per `docs/ai/BOUNDARIES_AND_ABSTRACTIONS.md`, adding a shared crate/protocol
and reversing dependency direction require a **separate architectural
decision**. This audit records the proposal but does not make that unapproved
high-risk change as part of dependency cleanup.
