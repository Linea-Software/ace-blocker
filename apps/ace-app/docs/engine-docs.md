# Ace Engine: desktop architecture

Ace Engine is a separate blocking service. The watchdog runs as another service.
Closing the Tauri window does not stop enforcement.

Ace App communicates with the engine through typed **remoc RPC**.
It uses Windows Named Pipes or Unix-domain sockets.
The unprivileged Tauri Rust backend connects to the engine and holds the capability token.
The WebView uses the `apps/ace-app/src/lib/engine-client/` facade.
The WebView never opens the IPC stream.

- Engine RPC contract: `crates/ace-engine/src/service/api/rpc.rs`
- Transport-independent engine operations: `crates/ace-engine/src/service/api/operations_*.rs`
- Local control transport: `crates/ace-engine/src/service/api/transport.rs`
- Tauri bridge: `crates/ace-engine/src/service/api/bridge.rs`
- Complete security and release guidance: `docs/IPC_ARCHITECTURE.md`

The engine sends Site Pack progress as typed Rust events through remoc and Tauri Channels.
This control path does not need a local HTTP server, Swagger UI, generated HTTP client, SSE framing, or port discovery.

External Ace Server calls, downloads, updates, and the blocking/MITM proxy
continue to use HTTP(S) where necessary.
