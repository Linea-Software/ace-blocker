# Build performance audit (2026-11-01)

This audit covers the independent `ace-blocker` workspace and its Tauri/SvelteKit app.
The values count resolved dependency-graph entries, **not cold-build seconds**.
They do not measure CI time savings.

| Metric | Before | After |
| --- | ---: | ---: |
| Root `Cargo.lock` entries (all targets/features) | 892 | 854 |
| Families with multiple locked versions | 81 | 68 |
| `ace-engine` Linux normal + build graph entries | 470 | 411 |
| `ace-app` Linux normal + build graph entries | 646 | 570 |

Run `cargo tree -p ace-engine --target x86_64-unknown-linux-gnu -e normal,build`
to inspect the engine production graph. Run the equivalent command for `ace-app`.
Use `cargo tree -d` to find duplicate dependencies. The lockfile also lists
platform-specific, optional, and test dependencies. These do not all compile together.

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

**No Cargo profile settings changed.** The dev profile already disables dependency debug information.
Rust's default `opt-level = 0` favors short debug builds. ACE uses `opt-level = 1`
for dependencies, which can improve test runtime. Do not change this setting
without representative measurements. The release settings `opt-level = "z"` and
`lto = "thin"` favor smaller shipped artifacts. Changes require a product decision.
Windows already uses `lld-link.exe`.

The official guide suggests `debug = "line-tables-only"` for *workspace* code
when backtraces are necessary. This audit keeps `debug = false`.
It adds no nightly flags, experimental backends, or linker changes.

## CI and measurements

- Frontend assets are built once, uploaded, and reused by both native jobs.
  This prerequisite can extend the critical path while it reduces duplicate work.
  Measure GitHub Actions workflow *wall clock* time, not total CPU time.
- PRs retain Rust all-target checks and existing selected tests. Release-profile
  validation runs on `main` and manual dispatch instead of every PR.
- A separate non-PR job runs the opt-in proxy CLI and HTTP/3 tests.
- Native checks save `cargo --timings` HTML reports to CI artifacts. Compare
  cold vs warm cache runs on each OS before adjusting profiles.
- [rust-cache](https://github.com/Swatinem/rust-cache) is already configured;
  keep OS and dependency/feature-specific cache keys distinct.

## Deferred architectural decision: separate the Tauri RPC client

Tauri imports the full `ace-engine` package for the typed `service::api::bridge`,
`RpcError`, and site-pack streaming status. This can add networking, SQLite,
and proxy dependencies to Tauri. Tauri cannot run the enforcement service.
A separate typed RPC client/contract crate could reduce the build cost.
This change requires more than a manifest edit.

Today, `rpc.rs` combines the remoc trait and privileged server implementation;
`bridge.rs` dispatches typed methods and imports engine core/manager types;
`schemas.rs` re-exports core types; `transport.rs` owns both client connects
and server listeners. Extraction requires defined ownership, compatible serialization, and cross-process tests.

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
decision**. This audit records the proposal. It does not make the unapproved architectural change.
