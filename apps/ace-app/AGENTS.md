If the Svelte MCP server is connected, use its documentation tools for Svelte 5 and SvelteKit work. If unavailable, rely on the official documentation, repository source and checks; do not claim to have called a missing tool.

## Self-Maintenance (IMPORTANT)

- **Maintain Context**: You MUST automatically edit this `AGENTS.md` file whenever significant changes occur in the project (e.g., new tools added, new core documentation links found, project structure changes, or new patterns established). Keep this document up-to-date as the "Source of Truth" for your behavior in this project.

## Project Description

**Ace Blocker** is a blocker application designed to help users overcome internet addictions. It blocks addictive network sites (porn, social media, betting, etc.) and provides a suite of scripts to make these sites less appealing and more boring.

## Project Structure

This is a **Tauri + SvelteKit** application. The project serves as a UI wrapper for the Ace Engine backend.

### Primary Directories

- `src/` — **Frontend (SvelteKit)**
    - `lib/` — Shared logic, components, and utilities. USE $lib TO ACCESS IT, NO RELATIVE IMPORTS FOR THIS ONE.
        - `engine-client/` — stable TypeScript facade for the typed remoc RPC methods; the Tauri Rust backend owns the transport. Do not regenerate an OpenAPI/HTTP client.
        - `components/` — Application-specific components.
            - `ui/` — Legacy Shadcn Svelte base components. During the approved UI rebuild, do not expand this folder by default just to obtain generic styling; prefer Ace-owned components over headless primitives.
        - `stores/` — Svelte stores for state management.
        - `paraglide/` — Generated i18n logic.
        - `server/` — Server-side logic.
        - `deltas/` — Delta sync logic for pushing changes to the engine (lock, schedule, site-packs connections, user-defined rules deltas). Files: `common.ts`, `lock-delta.svelte.ts`, `schedule-delta.svelte.ts`, `site-packs-connection-delta.svelte.ts`, `user-defined-rules-delta.svelte.ts`.
        - `data/` — Static data files (e.g., `site-packs.json` with pack metadata; ids are the canonical catalog ids such as `pack-adult-free`). Ace-hosts-managed catalog sources use the whole, unsplit `<category>.txt` assets from the latest ace-hosts release.
        - `hooks/` — Reusable Svelte hooks (e.g., `is-mobile.svelte.ts`).
        - `utils/` — Utility functions (e.g., `pack-icons.ts`).
        - `onboarding/` — First-run experience data: `default-blocks.ts` (IDs and helpers for the network blocks seeded by engine migration `0021_seed_default_network_blocks`) and `tour-steps.ts` (guided tour step definitions).
    - `routes/` — Page routes and SvelteKit endpoints.
        - `network-blocks/` — Network block management pages.
        - `resistance-mode/` — Resistance mode configuration pages.
        - `settings/` — Application settings pages.
        - `site-packs/` — Site packs management pages.
        - `getting-started/` — Getting started guide (concepts, setup checklist, FAQ).
        - `help/` — Help and documentation pages.
        - `<route name>/+page.svelte` — Page content.
        - `<route name>/+page.server.ts` — Page server logic.
        - `<route name>/+page.ts` — Page client logic.
        - `<route name>/+page.css` — Page styles.
        - `<route name>/+page.md` — Page documentation.
        - `<route name>/_<component name>.svelte` — Page-specific components
    - `app.css` — Global styles.
- `src-tauri/` — **Backend Wrapper (Rust)**
    - `src/` — Rust source code (e.g., `main.rs`, custom commands).
- `messages/` — Translation files (e.g., `en.json`).
- `docs/` — Reference documentation for LLM context.
- `scripts/` — Utility scripts for building and testing.

### Technology Stack

- **Framework**: SvelteKit 2 + Svelte 5 (Runes)
- **UI**: Tailwind CSS 4 + Bits UI/headless primitives for the approved Ace-owned rebuild. Existing Shadcn Svelte components are legacy implementation and may remain while screens are migrated.
- **Runtime**: Bun
- **Backend Wrapper**: Tauri 2 (Rust)
- **i18n**: Paraglide JS
- **Engine client**: Typed remoc RPC through the Tauri backend; no local REST/HTTP client.

## AI-led refactoring and feature boundaries

Read the root `../../AGENTS.md` and, for refactoring, `../../docs/ai/REFACTORING_PLAYBOOK.md`.
Consult `../../docs/ai/BOUNDARIES_AND_ABSTRACTIONS.md` before sharing a
route component, state store, or module across features; consult
`../../docs/ai/DOMAIN_TYPES.md` for untrusted input and typed forms, and
`../../docs/ai/REVIEW_GATES.md` for independent architecture/type reviews.
These documents are authoritative for refactoring when generic skill advice
conflicts; the IPC/security constraints in the root instructions always win.

Keep route-specific components and state local by default. Reuse UI code when
consumers genuinely share behavior and lifecycle, not solely because markup
looks similar. Avoid making a shared store or `$effect` synchronization bus
for otherwise independent routes. The existing `$lib/engine-client/` facade
is the boundary to engine data; Tauri owns native IPC, Rust owns policy and
validation at its trust boundary. Do not bypass authorization or recreate a
local HTTP API. For user input, separate editable form strings from parsed,
validated domain values; do not create branded types or Zod schemas without
a real invariant/trust boundary. Verify relevant `bun run check` and
`bun run test:unit` from this directory; native IPC needs separate tests.

## Brand assets

The official artwork lives in the sibling `ace-blocker-logo` repository. Use the exported **transparent** `ace-icon-light-transparent.svg` / `ace-icon-dark-transparent.svg` in the UI, switching via CSS `.dark`, instead of placing a solid-background icon on a themed surface. `src-tauri/icons` is generated from `PNG/ace-icon-light-transparent.png` using `bun run tauri icon ... --ios-color '#FAF7F1'`; keep required solid platform icon backgrounds only where the operating system needs them. The app shell navigation rail displays the official transparent icon, not a placeholder letter. Never modify the logo source repository when consuming its files.

## UI Identity / Rebuild Direction

**Before any task that creates, redesigns, or materially alters visible UI, read `docs/ui-identity.md`.** It is the local source of truth for the approved Ace App redesign.

The target is not “custom-styled shadcn.” Ace should feel like a **disciplined personal enforcement instrument** rather than a generic web dashboard inside Tauri.

Key rules:

- preserve the engine API as the stable frontend boundary unless the task explicitly authorizes an API change;
- use **Bits UI/headless behavior + Ace-owned styling** for new rebuild components;
- do not add new shadcn components merely because they are convenient;
- Tailwind is an implementation tool, not the visual language;
- Signal amber (`#FFB000`) means intervention/commitment/enforcement, not “every primary action”;
- target warm Ink/Paper neutrals, IBM Plex typography, modest radii, borders/rails/inset surfaces, restrained motion, and structural state;
- do not default to rounded card grids, gradient washes, glow, glass, hover-lift cards, icon tiles, tiny uppercase eyebrows, or cyber/hacker motifs;
- Network Blocks, Resistance, Locks, Schedules, Site Packs, and engine state are the signature visual concepts and should not all share one generic page template;
- verify meaningful visual changes with actual screenshots in representative light/dark and active/inactive/locked/error states.

### Responsive layout convention

- The Tauri window defaults to 800×600, so viewport breakpoints do not represent the usable page width after the navigation rail and workspace gutters.
- `PageLayout` establishes the named CSS container `page`; page-owned components should prefer `@container page (...)` for responsive composition.
- The shell workspace establishes `ace-workspace`, and `Inspector` establishes `inspector`; use those named containers when responsiveness depends on those surfaces rather than on the browser viewport.
- Keep primary facts visible at the default desktop window size; only collapse secondary columns when the actual page container becomes narrow.

### UI skill references (when available)

For material UI work, first check which skills and MCP tools are actually installed
and accessible. A skill listed here or in `skills-lock.json` is not proof that
the implementation is present. Consult relevant verified skills when available;
otherwise follow the Ace UI identity, Svelte docs and repository checks.
No skill may override root security/IPC or domain-ownership rules.

For material UI work, when available:

1. Use `frontend-design`.
2. Use `ui-ux-pro-max`.
3. Use `svelte-code-writer`, `svelte-core-bestpractices`, and `svelte-components` for implementation; add `svelte-runes` when reactive state is involved.
4. Use `web-design-guidelines` as a final review where applicable.
5. `design-taste-frontend` may be used as an anti-template critique pass, but it explicitly is not the authority for dashboard/multi-step product UI.

### Core Rule: Development Context

Most tasks occur within `./src` (Frontend). The Rust backend (`src-tauri`) is primarily a wrapper, though you may occasionally need to interface with its commands (e.g., `is_debug_mode`).

## Package Manager & Workflow

- **ALWAYS** use `bun` for all package management and script execution tasks.
- Use `bun install` instead of `npm install`
- Use `bun run` instead of `npm run`
- Use `bun` instead of `node`
- Do **NOT** use `npm`, `pnpm`, or `yarn` under any circumstances.
- When running scripts, use `bun run <script>`.
- When installing packages, use `bun add <package>` or `bun add -d <package>`.
- **Running Locally**: Both the desktop app and the website are ALWAYS running during development.
    - **App UI**: Accessible via browser at `http://localhost:1420`. You can connect through your browser to this address to access the app UI.
    - **App (Tauri)**: Verify native behavior with a real Tauri runtime or the documented disposable VM gate; the MCP bridge plugin is no longer installed.
- **Committing Changes**: You **MUST** perform git commits yourself. Once you have finished a task and verified it, stage only the task's reviewed files (never `git add .`) and commit with a descriptive message, following root `AGENTS.md`. Do NOT ask the user to commit for you.

## Ace Engine Access

The frontend uses src/lib/engine-client as its stable interface. Its Tauri
backend calls typed remoc RPC over a local named pipe. There is no local
REST server, Swagger/OpenAPI generator, API port or HTTP fallback.
Read the root docs/IPC_ARCHITECTURE.md before changing IPC or authorization.
Preserve lock policy checks, handle error/data responses and streaming statuses,
and localize user-visible errors. The TypeScript facade uses `RpcFailure`
from `src/lib/engine-client/rpc-error.ts` for explicit error results, not
Fetch-style `response.status` objects; `rpc-contract.test.ts` compares the
operation map against the current Rust bridge. Network Block editing coordinates
independent writes in `src/routes/network-blocks/_save-network-block.ts` and
must never report success or blindly retry after an ambiguous partial commit.
Native IPC/capability tests require the disposable Windows VM gate documented
in `docs/ai/ACE_APP_NATIVE_TEST_GATE.md`. **Desktop WebView shutdown is forbidden**:
`src-tauri/src/lib.rs` denies the `shutdown` operation before capability
loading, while the independent installer/engine maintenance RPC remains intact.
`src/lib/deltas/schedule-coverage.ts` mirrors the current engine interval
shape: end-before-start is invalid except `00:00` as the end of the selected
day; do not restore unsupported overnight editor behavior without an approved
engine policy change.

## Localization (Paraglide JS)

- **CRITICAL**: All user-facing text **MUST** be localized using Paraglide. Hardcoded strings are **FORBIDDEN** in UI components.
- **Workflow**:
    1. Add the string key to `messages/en.json` (format: `"snake_case_key": "Text content"`).
    2. Run `bun run check` (or let the dev server auto-generate fields) to ensure types are updated.
    3. Import `* as m` from `$lib/paraglide/messages.js`.
    4. Use `m.snake_case_key()` in your Svelte components or TS files.
- **Rules**:
    - **Only** edit `en.json` by default. Do **NOT** edit other language files (`de.json`, `fr.json`, etc.) unless the user explicitly instructs you to do so.
    - Variable arguments: Use `{paramName}` in JSON (e.g., `"hello_user": "Hello {name}"`) and pass object used in code: `m.hello_user({ name: 'User' })`.

### Engine Docs Reference

- **Always read** `docs/engine-docs.md` and `docs/IPC_ARCHITECTURE.md` before performing tasks involving the engine.
- The **boredom-engine** (at `../boredom-engine/`) provides the resistance scripts that ace-engine injects into blocked sites.

## Scripts

Use `bun run <script>` for these tasks:

- dev: Starts Vite; actual engine RPC requires Tauri.
- `clear-cache`: Runs a utility script to clear application/build cache.
- `tauri`: Accesses the Tauri CLI (e.g., `bun run tauri dev`).
- `check`: Runs Svelte-check for type safety and linting.

**Current SvelteKit 3 / CI conventions:**
- Use `$app/env` (not removed `$app/environment`) for `dev`, and import `Handle` / `Reroute` types from `@sveltejs/kit/hooks`.
- For navigation to known routes, prefer the generated `RouteId` type from `$app/types`.
- Keep generated `build/` output out of the root TypeScript `include`: otherwise `svelte-check --tsgo` traverses minified output and may crash with `RangeError: Invalid string length` instead of showing useful diagnostics.
- SvelteKit 3's `RequestEvent.request` is readonly; localized request middleware must pass a new event to `resolve` rather than assigning to the original event.

**Note**: ALWAYS use **TypeScript** for any new utility or maintenance scripts created in the `/scripts/` directory.

## Nomenclature & Design Patterns

- **Svelte Components**: All `.svelte` files MUST use `UpperCamelCase` naming convention (e.g., `Button.svelte`, `UserCard.svelte`).
- **Page-specific components**: These MUST reside in a `./_components` folder within the route directory OR be prefixed with an underscore (e.g., `_Component.svelte`) to differentiate them from routes.
- **Reactive Collections**: Use `SvelteMap` or `SvelteSet` from `svelte/reactivity` for reactive maps and sets. Avoid generic objects for states that require frequent updates or additions.
- `/messages/`: Localization message files for Paraglide. During development, always prioritize English (`en`). Only translate when explicitly requested.

### Safe i18n Patterns

- **CRITICAL**: Avoid indexing the `m` object from Paraglide directly (e.g., `m[someVariable]`). This causes runtime `TypeErrors` and bypasses TypeScript safety.
- **Pattern**: Use explicit helper functions with `switch` or `if/else` to map variables to `m.key()` functions.
- **Example**:
    ```typescript
    const getLabel = (type: string) => {
        if (type === "static") return m.static_mode();
        return "";
    };
    ```

### First-Run Experience

The onboarding flow runs in three parts, all rendered from `src/routes/+layout.svelte`:

1. **`TosModal`** — non-dismissible Terms of Service + Privacy Policy gate. The app shell stays inert until account info is loaded and, when acceptance is required, both acknowledgements are checked and `/account/accept-tos` succeeds. The engine persists the combined legal acceptance.
2. **`OnboardingWizard`** (`lib/components/onboarding/`) — a five-step setup wizard shown once after the ToS. It reads the engine state through `getDashboardSnapshot()`, lets the user switch on the seeded default blocks (which adds a 24/7 schedule entry, since a block with no schedule never matches), and optionally enables resistance mode.
3. **`TutorialTour`** (`lib/components/onboarding/`) — a spotlight tour that navigates between routes and highlights elements by their `data-tour` attribute. Steps live in `lib/onboarding/tour-steps.ts`; add a `data-tour="..."` attribute to any element a step should point at.

- Completion state lives in `lib/stores/onboarding.svelte.ts`, backed by `localStorage` (`ace:onboarding:v1`). It is UI-only state — everything the wizard *configures* is persisted by the engine.
- Both can be replayed from `/getting-started`.

### Resistance Mode UI

- **Intensity**: Static intensity only (v1 beta; dynamic escalation was
  removed on 2026-09-02). The resistance-mode page uses an
  `IntensitySelector` that calls `setResistanceModeType` with
  `{ type: "static", intensity }`.
- **Intensity Mapping**: "Maximum" in the UI corresponds to the `extreme` intensity level in the engine.

## Documentation

**🚨 CRITICAL MANDATORY REQUIREMENT 🚨**
Before performing ANY task related to the technologies below, you MUST read the linked `llms.txt` documentation. Relying solely on internal knowledge is UNACCEPTABLE and leads to outdated patterns.

| Technology | Documentation | Purpose |
| ---------- | ------------- | ------- |
| Svelte 5 & SvelteKit | [llms.txt](https://svelte.dev/llms.txt) | Full-stack framework. **MUST use Svelte 5 runes (`$state`, `$derived`, `$effect`, etc.)**.
| Shadcn Svelte | [llms.txt](https://shadcn-svelte.com/llms.txt) | Legacy UI component library. Consult only when maintaining existing shadcn components; do not use it as the visual foundation of the rebuild.
| Paraglide JS | [llms.txt](https://inlang.com/g/6d900h6s/paraglide-js) | Localization framework for i18n.
| Lucide Svelte | [llms.txt](https://lucide.dev/guide/packages/lucide-svelte) | Icon library. Use `@lucide/svelte` (NOT the deprecated `lucide-svelte`).
| Tauri | [llms.txt](https://tauri.app/llms.txt) | Desktop wrapper, IPC commands, and native APIs.
| Engine IPC | [architecture](../../docs/IPC_ARCHITECTURE.md) | Typed remoc trait, Tauri bridge and frontend RPC facade. |
| Zod | [llms.txt](https://zod.dev/llms.txt) | Schema validation and type safety.
| BitsUI | [llms.txt](https://www.bits-ui.com/llms.txt) | Headless component library for Svelte.


CRITICAL: ALWAYS read `docs/engine-docs.md` to understand what the engine is and how it works before performing any task. This is the source of truth for backend concepts. For any visible UI/design task, also read `docs/ui-identity.md`.

## MCP tool workflows (only when connected)

The tools below are optional integrations and may not exist in every agent
session. If absent, use the official documentation and local compiler/tests
instead. Never block a documentation-only or unrelated refactor solely because
an MCP tool or optional skill is unavailable; never claim a tool was run when
it was not.

### 1. list-sections

Use this FIRST to discover all available documentation sections. Returns a structured list with titles, use_cases, and paths.
When asked about Svelte or SvelteKit topics, ALWAYS use this tool at the start of the chat to find relevant sections.

### 2. get-documentation

Retrieves full documentation content for specific sections. Accepts single or multiple sections.
After calling the list-sections tool, you MUST analyze the returned documentation sections (especially the use_cases field) and then use the get-documentation tool to fetch ALL documentation sections that are relevant for the user's task.

### 3. svelte-autofixer

Analyzes Svelte code and returns issues and suggestions.
You MUST use this tool whenever writing Svelte code before sending it to the user. Keep calling it until no issues or suggestions are returned.

### 4. playground-link

Generates a Svelte Playground link with the provided code.
After completing the code, ask the user if they want a playground link. Only call this tool after user confirmation and NEVER if code was written to files in their project.

### 5. tauri-mcp-server

When a Tauri MCP server is actually connected, use it for native runtime/UI checks. If unavailable, record native verification as not run; Vite/browser tests alone cannot validate remoc or installed Windows services.

## Skill catalog (availability must be checked)

Skill names below record previously selected references. Check for actual
`SKILL.md` files before use; the catalog or lockfile does not guarantee
installation. Inspect their content and executable scripts. Do not fetch or
execute third-party skills automatically. During refactoring, root invariants
and Ace-specific architecture/type guides take precedence.

The following names were previously tracked via `npx skills`. They may or may not be available under `.agents/skills/` in the current workspace or agent environment; verify each relevant skill before use.

### Svelte / SvelteKit

- **svelte-code-writer** (`sveltejs/ai-tools`) — Official Svelte AI tool for writing Svelte components.
- **svelte5-best-practices** (`ejirocodes/agent-skills`) — Best practices for Svelte 5 (Runes, snippets, etc.).
- **svelte-core-bestpractices** (`sveltejs/ai-tools`) — Core Svelte best practices and conventions.
- **sveltekit-structure** (`spences10/svelte-skills-kit`) — SvelteKit project structure conventions.
- **svelte-runes** (`spences10/svelte-skills-kit`) — Svelte 5 Runes ($state, $derived, $effect, etc.) patterns.
- **sveltekit-data-flow** (`spences10/svelte-skills-kit`) — SvelteKit data loading, forms, and actions.
- **svelte-components** (`spences10/svelte-skills-kit`) — Svelte component patterns and composition.

### UI & Styling

- **frontend-design** — Mandatory for creating or materially reshaping visible UI; establishes intentional, non-template visual direction.
- **ui-ux-pro-max** — Mandatory for product-wide UI/UX, accessibility, typography, layout, and interaction reasoning.
- **svelte-components** — Use for Bits UI/headless component patterns and composition.
- **design-taste-frontend** — Anti-template critique only for Ace App; it is not the authority for dashboard/multi-step product UI.
- **web-design-guidelines** — Final UI/accessibility review where applicable.

- **shadcn** (`shadcn/ui`) — Legacy-maintenance reference only; default shadcn styling is not Ace identity.
- **shadcn-svelte** (`vercel-labs/json-render`) — Legacy-maintenance reference only during the rebuild.
- **tailwind-v4-shadcn** (`secondsky/claude-skills`) — Legacy integration reference; do not treat its visual defaults as the target system.
- **suggest-lucide-icons** (`nweii/agent-stuff`) — Use for ordinary utility icons; core Ace concepts may receive bespoke glyphs.

### Tauri

- **tauri-v2** (`nodnarbnitram/claude-code-extensions`) — Tauri v2 development patterns and commands.

### Runtime & Language

- **bun-development** (`sickn33/antigravity-awesome-skills`) — Bun runtime development best practices.
- **typescript-advanced-types** (`wshobson/agents`) — Advanced TypeScript types and patterns.
- **zod** (`pproenca/dot-skills`) — Zod schema validation patterns.

### Utilities

- **i18n-localization** (`sickn33/antigravity-awesome-skills`) — Internationalization and localization best practices.
- **git-commit** (`github/awesome-copilot`) — Git commit message conventions and best practices.
