# Ace Blocker desktop application

A Tauri 2 desktop client with a **SolidJS 2.0 + TypeScript + Vite** frontend.
The UI runs locally in the WebView (no Node server at runtime). It communicates
with the Ace Engine only through its existing typed Tauri/remoc IPC bridge.

## Setup

From `apps/ace-app/`:

```bash
bun install --frozen-lockfile
bun run check
bun run test:unit
bun run build
bun run tauri dev
```

For development without a running native engine:

```bash
bun run dev
```

The backend Rust crate lives under `src-tauri`; the Windows installer manages
the engine and watchdog service lifecycle outside the frontend.

## Engineering documentation

- [Solid 2 architecture and migration invariants](docs/SOLIDJS_2_PORT.md)
- [AI/agent contributor instructions](AGENTS.md)
- [Solid 2.0 documentation index](https://docs.solidjs.com/llms.txt)
- [Complete Solid 2.0 reference corpus](https://docs.solidjs.com/llms-full.txt)

The existing user-facing design must not change as part of the framework port.
Preserve lock enforcement, device linking, offline behavior, Free/Pro permissions,
trusted deadlines and cloud backup password handling. All unprivileged UI code is
kept separate from the Rust engine's privileged enforcement boundary.
