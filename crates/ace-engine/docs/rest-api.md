# Retired local REST API

Ace Blocker uses typed remoc RPC for local control.
On Windows, it uses Windows Named Pipes.
On Linux, it uses Unix sockets.
It does not use a local HTTP server or an OpenAPI-generated client.

For more information, see these files in the unified repository:

- `docs/IPC_ARCHITECTURE.md`: IPC architecture.
- `crates/ace-engine/src/service/api/rpc.rs`: `EngineControl` trait.
- `apps/ace-app/src/lib/engine-client/sdk.ts`: Svelte interface.

Blocking and MITM proxy traffic still use their existing protocols.
External HTTP services are still supported.
