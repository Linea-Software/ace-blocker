# Ace Engine: desktop architecture

Ace Engine is a standalone blocking service; the watchdog is a separate
service, and shutting down the Tauri window cannot stop enforcement.

Ace App talks to the engine through typed **remoc RPC** over Windows Named
Pipes / Unix-domain sockets. The unprivileged Tauri Rust backend connects
to the engine and owns the capability token. The WebView uses the
`apps/ace-app/src/lib/engine-client/` facade; it never opens the IPC stream.

- Engine RPC contract: `crates/ace-engine/src/service/api/rpc.rs`
- Transport-independent engine operations: `crates/ace-engine/src/service/api/operations_*.rs`
- Local control transport: `crates/ace-engine/src/service/api/transport.rs`
- Tauri bridge: `crates/ace-engine/src/service/api/bridge.rs`
- Complete security and release guidance: `docs/IPC_ARCHITECTURE.md`

Site pack progress is transferred as typed Rust events through remoc and
Tauri Channels. No local HTTP server, Swagger UI, generated HTTP client,
SSE framing or port discovery is required.

External Ace Server calls, downloads, updates, and the blocking/MITM proxy
still use HTTP(S) where appropriate.
