# Ace Blocker App — Agent instructions (SolidJS 2.0)

This repository's desktop frontend is **SolidJS 2.0 + Solid Router 2 + Vite +
TypeScript**, running in a **Tauri 2** WebView. The `solidjs-porting` branch
replaces SvelteKit; **it does not redesign the UI**. Preserve visual layout,
spacing, colors, typography, icons, translations, keyboard behavior, accessibility,
and all engine-facing behavior. Fix defects without weakening product protections.

Before editing, read [the Solid port architecture](docs/SOLIDJS_2_PORT.md),
[the approved UI identity](docs/ui-identity.md), and the relevant root architecture
and product documents.

## Official docs and installed skills

Read the official documentation recursively, not merely the first URL:

- <https://www.solidjs.com/llms.txt>
- <https://docs.solidjs.com/llms.txt>
- <https://docs.solidjs.com/llms-full.txt>
- `node_modules/solid-js/skills/solid-2.0/SKILL.md`
- `node_modules/solid-js/CHEATSHEET.md`
- `node_modules/solid-js/skills/reactivity-diagnostics/SKILL.md`

The official `solidjs/solid` skills (`solid-2.0`,
`reactivity-diagnostics`, `agent-loops`) and SolidJS-v2 migration/reviewer
skills are installed through **skills.sh** under `.agents/skills/`. Reproduce
these selections using `skills-lock.json`. Package-pinned official instructions
outrank third-party prerelease guidance. **Never use Svelte, SvelteKit, Solid 1.x,
React-specific, or otherwise incompatible component APIs** in new code.

Solid 2 specifics:

- Components execute once. Read reactive props without destructuring away
  their reactivity.
- `createSignal` returns a getter and setter; invoke the getter from JSX.
- `createEffect(compute, apply)` uses **two arguments**. A Solid 1 or React
  single-callback effect is not an equivalent.
- Prefer computed values over effects. Use `onSettled(() => { setup();
  return cleanup; })` for mount/dispose resources and listeners.
- `@solidjs/web` owns JSX and DOM rendering, `@solidjs/router` v2 owns
  navigation, and `@solidjs/vite-plugin` compiles the frontend.
- Keep component CSS scoped to a component class; the old Svelte compiler
  performed automatic scoping. Do not let generic `span`, `h2`, `button`,
  `p`, or dialog styles leak globally.
- `src/lib/stores/reactive-fields.ts` bridges existing field-style stores
  into Solid signals; new components may use local signals directly.
- Route components persist across changes to search parameters. React to
  `useLocation()` changes, cancel/ignore stale reads, and reset unsafe drafts
  when a block identity changes. Never trust an outdated query value.
- Modal dialogs must hold keyboard focus, restore focus on close, and prevent
  interaction with the background. Respect reduced motion preferences.

## Project map

| Path | Responsibility |
| --- | --- |
| `src/main.tsx` | Static WebView entry; debug-only native WDIO bridge |
| `src/App.tsx`, `src/router.tsx` | Legal gate, onboarding, shell, lazy routes |
| `src/routes/**/*.tsx` | Dashboard, Network Blocks, Resistance, Categories, Settings, Guide, Help |
| `src/lib/components/**/*.tsx` | Ace-branded components and Solid 2 UI primitives |
| `src/lib/stores/**/*.ts` | Reactive state and engine subscriptions |
| `src/lib/deltas/**/*.ts` | Staged policy and schedule changes |
| `src/lib/engine-client/` | Stable typed remoc RPC interface |
| `src/lib/paraglide/` | **Generated** i18n code; do not edit manually |
| `src/lib/utils/pack-icons.ts` | Canonical icon slug registry, including legacy IDs |
| `src/lib/utils/theme.ts` | `mode-watcher-mode` preference compatibility |
| `src-tauri/` | Rust Tauri wrapper, native commands and capabilities |
| `static/`, `src/app.css` | Official Ace icon assets and global design tokens |
| `vite.config.ts`, `index.html` | Static bundle; `build/` is Tauri's `frontendDist` |
| `messages/*.json` | Paraglide messages; English is the source language |

The installed desktop application has **no NodeJS/SSR process** and no
frontend HTTP API for local policy control. The browser-only development preview
cannot exercise privileged Tauri IPC. Follow root `docs/IPC_ARCHITECTURE.md`
and `docs/ai/BOUNDARIES_AND_ABSTRACTIONS.md` before any IPC change.

## Non-negotiable security and product behavior

**Engine boundary and locks.** The WebView stays unprivileged. The native
Tauri adapter communicates with Ace Engine via authenticated typed **remoc
RPC**, not a localhost HTTP server. Rust owns policy decisions, account
entitlements, capabilities, and lock enforcement. Never expose the device
secret or API key to JavaScript. Never add UI-accessible engine shutdown, stop,
restart, unlock-bypass or LocalSystem maintenance operations.

**Account linking.** Settings `_LicenseSection.tsx` invokes the native
`device_link_start`, `device_link_poll`, and `device_link_cancel` commands.
The UI receives a human-readable code, state and retry delay only. Rust
retains the secret and redeemed key. Open the verified HTTPS confirmation URL
in the system browser; do not reintroduce web prompts, redirect-following
token discovery or a new local HTTP port. Read
`src-tauri/src/device_link.rs` and the adjacent website's
`docs/DEVICE_LINKING.md` before changing this flow.

**Cloud backup.** `_BackupSection.tsx` asks for the password on each cloud
upload or restore. Treat it as ephemeral: do not persist or log it, clear it
after submission/dismissal, and prevent double submissions. Engine-side Rust
owns Argon2id/GCM encryption; the frontend never claims the website has
zero-knowledge authentication. A changed account password cannot decrypt
older backups without their original password. Preserve local import/export
and Pro entitlement gates.

**Network Blocks.** The full-page editor has three steps,
**Content → Schedule → Review**. Stage changes in
`UserDefinedRulesDelta`, `SitePacksConnectionDelta`, and `ScheduleChanges`.
Commit a full draft using **one atomic `saveNetworkBlock` RPC on Review**.
Never persist during step navigation, schedule editing, category selection or
individual site entry. An uncertain acknowledgment may follow a successful
COMMIT: reconcile with canonical engine state instead of blindly retrying.
A failed read of a pre-existing block must fail closed, not overwrite its
rules with empty defaults. Enabled state and schedule mode are independent;
activation is a separate operation. Do not auto-lock a newly created block.
Safe Search is a valid stand-alone target, but clearly describe its limited
search-engine coverage. Keep the 20-row website editor and hostname
normalization.

**Anti-weakening.** Never remove a preexisting locked Site Pack connection,
reduce a locked schedule, circumvent Free/Pro gates, or relax protected
Resistance scripts. Users may still add restrictions. Engine remains the final
authority. Timed lock submissions are absolute RFC3339 UTC instants; never
truncate offsets. Schedules are separate civil-time windows pinned to an IANA
timezone by the engine. Do not reinterpret them using the current OS
timezone. See `../../docs/TRUSTED_TIME_FUTURE.md`,
`../../docs/NETWORK_BLOCK_ACTIVATION.md` and
`../../docs/IPC_ARCHITECTURE.md`.

**Site Packs.** Use canonical `src/lib/utils/site-pack-presentation.ts`
for official category grouping, icons and counts. Paired Free/Pro categories
appear once and expand into editions; Custom Lists are separate. Respect
engine entitlements and never silently erase connections on downgrade.
Downloaded pack counts are real; noninstalled counts are estimates.
Catalog IDs must align with `ace-hosts` release metadata.

**Resistance Mode.** The five top-level categories are Feeds, Video,
Visual Stimulation, Delays/Limits, and CAPTCHA. They start collapsed;
CAPTCHA interventions default off. Checkboxes and disclosures must be
independently accessible. A Time Lock forbids turning off protected scripts
or clearing a fully selected category; it still allows adding restrictions.
Script updates live in Settings, not in Resistance. Do not restore a global
intensity level or silently reset saved choices.

**First run.** The Terms of Service gate is non-dismissible until engine
account state and acceptance are confirmed. The workspace must remain inert
while the gate is active. The five-step `OnboardingWizard` follows ToS;
its actual changes go through engine RPC, not localStorage. Both the wizard
and `TutorialTour` can be replayed. UI-only completion flags use
`ace:onboarding:v1`. During the wizard, hold keyboard focus within the
dialog and keep the background inert. Tour targets use `data-tour`.

**Product updates.** The Settings updater may check and stage signed
releases; do not offer an Install or shutdown action until the privileged MSI
orchestrator is separately approved. Never add a frontend-only updater that
bypasses the engine/watchdog. See `../../docs/PRODUCT_UPDATES.md`.

## UI and localization

Follow `docs/ui-identity.md`: warm Paper/Ink neutrals, IBM Plex fonts,
Signal amber (`#FFB000`) for enforcement/commitment, structural rails,
modest corner radii, and no generic glass/gradient/glow dashboards. The
official transparent logo lives in `static/`, and native icon assets
live in `src-tauri/branding/`. Do not replace either with placeholder art.
Preserve light/dark themes, 800×600 minimum usability and 320px narrow
browser test layouts. Prefer CSS containers `page`, `ace-workspace` and
`inspector` over raw viewport breakpoints.

All visible text belongs in Paraglide. Import
`* as m from "$lib/paraglide/messages.js"` and call `m.key()`
**directly**: never use dynamic `m[someKey]` indexing. Change
`messages/en.json` first, check grammatical plural forms with
`src/lib/utils/count-message.ts`, and follow
`../../docs/ITALIAN_TERMINOLOGY.md` and
`../../docs/LLM_LOCALIZATION_WORKFLOW.md` for translations.

Native locale auto-detection comes from the unprivileged Rust
`system_locale` command, never from the browser's reported language.
Explicit choices persist in the existing Tauri `settings.json`; the
browser preview has its own localStorage preference. Paraglide reloads
this static SPA on a locale change without changing the path.

## Workflow and verification

Always use **Bun** for frontend package installation and scripts, rather
than npm/pnpm/yarn. From `apps/ace-app`:

```bash
bun install --frozen-lockfile
bun run i18n:audit
bun run check:port
bun run check
bun run test:unit
bun run test:e2e
bun run build
bun run tauri build --debug --no-bundle
```

From the repo root, check Rust with `cargo check -p ace-app --lib`.
Native smoke checks use `bun run test:native` when the debug Tauri WebView
and `tauri-driver` are available. The debug-only WebDriver bridge is loaded
from `src/main.tsx` and **must never be included in release builds**.
Read `docs/DEV_AUTOMATION.md` and
`../../docs/ai/ACE_APP_NATIVE_TEST_GATE.md`.
Browser Playwright fixtures exercise UI/UX and simulated stores only; they
do **not** prove Windows service or policy enforcement.

After a change, compare native/browser screenshots in light and dark themes,
test keyboard and narrow-screen operation, ensure no legacy Svelte source
or runtime imports remain, inspect `git diff --check` and source diffs, then
commit reviewed files in focused commits. Do not commit `.temp/`, generated
`build/` artifacts or private credentials. Update this document and the
porting guide whenever core architecture or workflow changes.
