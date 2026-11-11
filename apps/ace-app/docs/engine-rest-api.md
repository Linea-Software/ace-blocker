# Retired local REST API

Ace Blocker uses typed remoc RPC over Windows Named Pipes and Unix sockets for local control.
It does not use a local HTTP server or an OpenAPI-generated client.

Read `docs/IPC_ARCHITECTURE.md` for the unified repository architecture.
The `EngineControl` trait is in `crates/ace-engine/src/service/api/rpc.rs`.
The Svelte facade is in `apps/ace-app/src/lib/engine-client/sdk.ts`.

Blocking/MITM proxy traffic and external HTTP services are separate from local control.
Both remain supported.
