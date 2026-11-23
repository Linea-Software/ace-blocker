# Ace Blocker desktop — SolidJS 2.0 architecture

## Purpose and constraints

The desktop UI is a static, client-rendered **SolidJS 2.0 + Vite + TypeScript**
application hosted in the existing **Tauri 2** WebView. The port is intentionally
isolated on the `solidjs-porting` branch. This is a framework experiment, **not a
UI redesign**. Retain the existing markup, spacing, CSS variables, iconography,
visual hierarchy, accessibility, keyboard interactions, translations and flows.

The Rust backend remains the only unprivileged-frontend gateway to the
LocalSystem engine over authenticated typed remoc RPC. Do not add a local HTTP
endpoint, client-side API token, browser-held device secret, service shutdown,
or any circumvention of lock enforcement.

## Official Solid 2 documentation (read in this order)

1. <https://www.solidjs.com/llms.txt> (site entry index)
2. <https://docs.solidjs.com/llms.txt> (nested links to each concept, router, Vite and API)
3. <https://docs.solidjs.com/llms-full.txt> (the complete documentation corpus)
4. `node_modules/solid-js/skills/solid-2.0/SKILL.md`
5. `node_modules/solid-js/CHEATSHEET.md`
6. `node_modules/solid-js/skills/reactivity-diagnostics/SKILL.md`

The official `solidjs/solid@solid-2.0`, `reactivity-diagnostics`, and
`agent-loops` skills are installed from skills.sh in `.agents/skills/`.
The SolidJS-v2 migration/reviewer skills are also installed from skills.sh.
Package-pinned official skills always take precedence over third-party guidance.

### Solid 2 rules

- Components run once. Read reactive signals in JSX, a memo or an effect's
  compute phase. Do not destructure reactive props in the component body.
- Signals are getters: `const [n, setN] = createSignal(0)`; JSX reads `n()`.
- `createEffect(compute, apply)` has **two** functions in Solid 2; do not copy
  Solid 1 / React single-callback effects.
- Prefer derived accessors over redundant effects. Use draft updates for
  `createStore`, never Solid 1 path-based setters.
- Imports: `solid-js` for reactive primitives; `@solidjs/web` for rendering
  and DOM JSX types; `@solidjs/router` v2 for navigation; native Vite compiler
  `@solidjs/vite-plugin`. Do not import `solid-js/web`, `solid-js/store` or
  obsolete `onMount` APIs.
- Scoped CSS from Svelte components must keep its effective selectors and
  visual specificity. Extract/import CSS or use correctly isolated styles;
  avoid global selectors that leak into neighboring panels.

## File structure

- `src/main.tsx`: WebView entry and debug-only native test bridge.
- `src/App.tsx`: persistent shell, legal gate, guided tour and notifications.
- `src/router.tsx`: Solid Router 2 lazy routes, retaining previous URLs.
- `src/routes/**/*.tsx`: desktop pages and page-specific components.
- `src/lib/components/**/*.tsx`: reusable presentation and accessible controls.
- `src/lib/stores/**/*.ts`: Solid 2 reactive stores and engine subscriptions.
- `src/lib/engine-client/`: unchanged typed Tauri remoc RPC API boundary.
- `src/lib/paraglide/`: generated localization code; do not edit manually.
- `src/app.css`, `src/app-shell.css`: global design system and layout.
- `src/lib/utils/theme.ts`: the single Solid theme signal and persistence owner. Browser previews restore `mode-watcher-mode` from localStorage; Tauri also loads/saves `settings.json` on application startup and every selection, then synchronizes the local cache to avoid a theme flash on later launches. A late native read never overrides a newer user selection.
- `src-tauri/`: unchanged Rust Tauri backend and native commands.
- `vite.config.ts`: SPA compilation and Paraglide/Tailwind plugins;
  `build/` remains Tauri's `frontendDist`.

No Node/SSR process runs inside the installed desktop application.
Do not introduce server functions for engine control.

## Invariants to preserve

- The ToS gate blocks interaction until the engine verifies account state.
- Account links use `device_link_start`; never expose raw device secrets in
  the WebView.
- Cloud backup passwords are ephemeral user input for each operation; the
  Rust engine performs encryption, not browser JavaScript.
- Network Block drafts are staged until a single atomic `saveNetworkBlock`
  call on Review. Enabled and schedule mode are separate. Never weaken or
  remove a locked connection, bypass Pro entitlements, or auto-lock creation.
- Site Packs maintain Free/Pro coverage labels, selections and Inspector
  semantics; failures must not silently remove saved policies.
- Absolute trusted lock deadlines and pinned schedule time zones retain their
  distinct semantics.
- English/Italian Paraglide messages, native OS language preference and
  persisted theme continue working.

## Verification gates

From `apps/ace-app`: `bun install --frozen-lockfile`, `bun run i18n:audit`,
`bun run check:port`, `bun run check`, `bun run test:unit`,
`bun run test:e2e`, and `bun run build`. The new
`e2e/solid-port-regressions.spec.ts` checks query-only editor navigation,
onboarding focus containment, Inspector CSS isolation, missing routes and
device-link failure styling.
From the repository root: `cargo check -p ace-app --lib`. For Linux
packaging integration, run `bun run tauri build --debug --no-bundle` from
`apps/ace-app`. Native WDIO tests use the debug guest bridge from `src/main.tsx`,
which is present in `tauri dev` (not Vite production bundles). The read-only
WebKitGTK native smoke suite confirms real Tauri IPC, navigation and an internal
route reload. Security enforcement with LocalSystem engine/watchdog and locks
still requires a disposable Windows VM before release.
Run relevant Playwright and native Tauri tests, including offline/engine failure,
lock, account linking, cloud backup, schedule and Pro gating paths.

A successful build alone does not prove UX equivalence. Compare screenshots
against the old app on desktop and narrow viewports, and perform real
keyboard/interaction checks. Verify no active imports from Svelte, SvelteKit,
bits-ui or mode-watcher remain before removing obsolete dependencies.
