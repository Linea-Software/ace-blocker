# Retired local REST API

Ace Blocker control uses typed remoc RPC over Windows Named Pipes and
Unix sockets, not a local HTTP server or OpenAPI-generated client.

See the unified repository documentation at docs/IPC_ARCHITECTURE.md,
the EngineControl trait in crates/ace-engine/src/service/api/rpc.rs,
and the Svelte facade in apps/ace-app/src/lib/engine-client/sdk.ts.

Blocking/MITM proxy traffic and genuinely external HTTP services remain
separate and supported.
