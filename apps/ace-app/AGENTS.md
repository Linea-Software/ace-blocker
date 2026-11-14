If the Svelte MCP server is connected, use its documentation tools for Svelte 5 and SvelteKit work.
If it is not connected, use official documentation, repository source, and checks.
Do not report the use of a tool that is not available.

## Browser-prompt-free account linking (2026-11-11)

- The default account and license activation starts in `src/routes/settings/_LicenseSection.svelte`.
  **Connect account** calls the native Tauri `device_link_start` command.
  The app shows a human-readable code and opens `/connect` through ordinary HTTPS.
  The Rust process in `src-tauri/src/device_link.rs` keeps the raw device secret and redeemed key private.
  It polls with a short timeout.
  It sends the key through engine remoc `setApiKey` with the capability held by the operating system.
  The WebView receives only the user code, state, and retry delay.
  It must never receive the API key.
- Do not use a local web HTTP port, a JavaScript-stored API key, or a service stop or restart.
  The engine owns the credential.
  Browser links and completed checkout must not automatically open `aceblocker://`.
  Keep the website fallback manual, and use it only for old desktop versions.
- Rust uses loopback HTTP for native URLs only in debug mode.
  Release mode uses HTTPS at aceblocker.com.
  Do not follow redirects to get a challenge or token.
  The related server schema and endpoints are in `ace-blocker-site/docs/DEVICE_LINKING.md`.
- Run `cargo check -p ace-app --lib`.
  Run `cd apps/ace-app && bun run check && bun run test:unit`.
  Run the server device-link unit and browser tests.
  Account linking requires a valid, verified browser session.

## Self-Maintenance (IMPORTANT)

- **Maintain Context:** Update this `AGENTS.md` automatically when the project changes significantly.
  Examples include new tools, core documentation links, project structure, or established patterns.
  Keep this file current as the source of truth for agent behavior in this project.

## Trusted timed locks (2026-11-06)

Network Block and Resistance Mode timed-lock forms send absolute RFC3339 UTC deadlines.
Convert a local date and time to a UTC ISO string immediately before you submit the RPC.
The engine stores the absolute instant.
Displayed lock deadlines can include a UTC offset.
Parse these values as instants.
Do not cut the value to its first 16 characters.
Schedules are separate civil-time windows.
The engine pins their IANA timezone when it creates a policy.
Do not infer this timezone from the live Windows system timezone after policy creation.
Read `../../docs/TRUSTED_TIME_FUTURE.md`.

## Network Block card icons (2026-11-09)

`NetworkPolicy.svelte` uses the shared `getPackIcon` slug map for saved block icons.
This map also supports legacy icon identifiers.
If no custom icon is selected, show the neutral Lucide Globe.
Use this default in both the card and editor icon picker.
Keep icons small and monochromatic.
Use the amber state rail, activation switch, and lock appearance to show enforcement and protection.
Do not use the icon for that purpose.
The icon is decorative (`aria-hidden`).
It must not replace a readable block name or state.
See `e2e/network-policy-icons.spec.ts` for custom, default, and legacy icon tests.

## Guided Network Block editor (2026-11-09)

The full-page Create/Edit editor at `/network-blocks/editor` has three steps:
**Content → Schedule → Review**.
Users can open each step directly.
Keep draft changes in `UserDefinedRulesDelta`, `SitePacksConnectionDelta`,
and `ScheduleChanges` until Review calls `saveNetworkBlock` once.
Do not save incremental changes during step navigation or Category selection.

Content shows Ace Categories first.
Show available Free Categories immediately.
Show Pro Categories in a collapsed catalog.
Keep selected Pro Categories visible.
Show Custom Lists separately.
**Manage individual websites** opens a subordinate list editor.
It includes domain normalization, bulk input, filtering, and 20-row pagination.
Do not use a tag cloud.
Provide a visible shortcut near the top so users do not need to scroll to find the panel.

Schedule reuses the existing weekly coverage and lock validation components.
Deep links with `section=schedule` open this step directly.
Review contains the block name, complete policy summary, and optional lock entry after saving.
Activation is currently informational.
The atomic save RPC creates enabled blocks and preserves existing enablement.
Changing enablement requires a separate `setNetworkBlockEnabled` operation.
Do not present enablement changes as part of the save transaction.
Do not lock automatically during creation.
Preserve edit safety after partial read failures or uncertain RPC acknowledgments.
Use `e2e/block-editor-redesign.spec.ts` and a native desktop screenshot for tests.
Browser fixtures do not have engine IPC.

## Network Block editor redesign (2026-11-08)

The Create/Edit Network Block editor uses the full-page `/network-blocks/editor` route.
It no longer uses a wide floating Inspector.
`_BlockEditor.svelte` keeps atomic `saveNetworkBlock` drafts and uncertain-result reconciliation.
It shows the editor in a full page with a sticky Save footer.
Show Category selection before website rules.
Let users collapse the Category catalog and long rule lists.
Use Free and Pro badges to distinguish Categories with similar names.
Use `requiresProForConnection` to enforce the engine rule that Custom Lists require Pro.
After a downgrade, users must still be able to remove previously connected Pro packs.
Pass bulk-pasted URLs through `parseWebsiteList` and hostname normalization before staging.
Do not write rule changes during editing.
Save is the only normal action that persists the draft.
Lock controls continue to use `_LockModal` and all engine lock rules.
The Playwright fixture renders this page component directly.
It does not test enforcement.

## Network Block editor and activation (2026-11-07)

The expanded Network Block editor has `Always` (default) and `Scheduled` modes.
These modes are independent of Enabled.
A new block must have websites or Categories.
Pasted URLs are stored as normalized hostnames only.
Schedules use pinned civil-time windows.
Preserve atomic `saveNetworkBlock` behavior, Lock anti-weakening, and uncertain-acknowledgment reconciliation.
`Inspector.svelte` supports a wide editor variant.
Both navigation and no-navigation layouts have a fixed footer.
Read `../../docs/NETWORK_BLOCK_ACTIVATION.md`.
Browser-only screenshots and the Playwright fixture are in `docs/ux-audit-assets/`
and `e2e/block-editor-redesign.spec.ts`.
Native IPC and enforcement need separate validation.

## Desktop localization (2026-11-06)

Italian and English have complete Paraglide message coverage.
Settings has a language preference: System language (default), English, or Italian.
`src/lib/utils/app-language.ts` restores this preference when the root layout mounts.
For automatic selection, use the **unprivileged** Tauri `system_locale` command backed by `sys-locale`.
Do not use the WebView browser locale for this selection.
`settings.json` stores explicit choices in the existing Tauri store.
Configured locales without translations intentionally use English.
Language changes use Paraglide's cookie strategy and reload the document.
Do not add a component-level string-mutation bus.
The non-Tauri browser preview stores a separate localStorage preference.
It cannot test the native operating-system bridge.
For localization changes, run `bun run i18n:audit`, `bun run check`,
`bun run test:unit`, and `cargo check -p ace-app --lib`.
Read `../../docs/LLM_LOCALIZATION_WORKFLOW.md` for editorial review rules.
Use `../../docs/ITALIAN_TERMINOLOGY.md` for approved Italian terms and English source review flags.
For counters that need grammatical agreement, use `src/lib/utils/count-message.ts`.
Provide explicit zero, singular, and plural messages.

## Whole-product updates (pre-release)

Settings includes `_ProductUpdateSection.svelte`.
It uses the existing typed `checkUpdates` engine API.
It shows verified release and MSI staging states.
Do **not** show an Install action until a separate LocalSystem MSI orchestrator is reviewed and integrated.
Keep product update discovery separate from content updates and TrustedClock.
Read `../../docs/PRODUCT_UPDATES.md`.
Do not add Tauri's UI-only updater.
Do not add shutdown commands that bypass locks.

## Project Description

**Ace Blocker** helps users limit addictive internet use.
It blocks addictive websites such as pornography, social media, and betting sites.
It also uses scripts to make these websites less appealing and less stimulating.

## Project Structure

This is a **Tauri + SvelteKit** application.
The app provides the UI for the Ace Engine backend.

### Primary Directories

- `src/` — **Frontend (SvelteKit)**
    - `lib/` — Shared logic, components, and utilities.
      Use `$lib` to import from this directory. Do not use relative imports for it.
        - `engine-client/` — Stable TypeScript interface for typed remoc RPC.
          The Tauri Rust backend owns transport.
          Do not generate an OpenAPI/HTTP client for local control.
        - `components/` — Application-specific components.
            - `ui/` — Legacy Shadcn Svelte components.
              Do not add generic styling components here by default during the approved UI rebuild.
              Prefer Ace-owned components that use headless primitives.
        - `stores/` — Svelte stores for state management.
        - `paraglide/` — Generated localization logic.
        - `server/` — Server-side logic.
        - `deltas/` — Changes sent to the engine for locks, schedules, Site Pack connections, and user-defined rules.
          Files: `common.ts`, `lock-delta.svelte.ts`, `schedule-delta.svelte.ts`,
          `site-packs-connection-delta.svelte.ts`, and `user-defined-rules-delta.svelte.ts`.
        - `data/` — Static data, including `site-packs.json`.
          Pack metadata uses canonical catalog IDs such as `pack-adult-free`.
          Ace-hosts catalog sources use complete, unsplit `<category>.txt` assets from the latest ace-hosts release.
          The anime/manga family has one canonical `pack-anime-manga-pro`.
          It includes curated sources and maintainer-reviewed Anime Discovery additions from ace-hosts.
          For new Ace-managed Categories, align IDs with `ace-hosts/scripts/build_site_packs.py`.
          Use one `releases/latest/download/<category>.txt` source.
          Do not repeat the upstream source URLs.
          Update size and count snapshots from a tested `ace-hosts` build.
          Publish the matching ace-hosts release.
          Update the remote Ace Server catalog before you treat new app entries as deployable.
        - `hooks/` — Reusable Svelte hooks, such as `is-mobile.svelte.ts`.
        - `utils/` — Utility functions, such as `pack-icons.ts`.
        - `onboarding/` — First-run data.
          `default-blocks.ts` contains IDs and helpers for engine migration `0021_seed_default_network_blocks`.
          `tour-steps.ts` defines guided tour steps.
    - `routes/` — SvelteKit pages and endpoints.
        - `network-blocks/` — Network Block management pages.
        - `resistance-mode/` — Resistance Mode configuration pages.
        - `settings/` — App settings pages.
        - `site-packs/` — Site Pack management pages.
        - `getting-started/` — Guide with concepts, setup checklist, and FAQ.
        - `help/` — Help and documentation pages.
        - `<route name>/+page.svelte` — Page content.
        - `<route name>/+page.server.ts` — Server-side page logic.
        - `<route name>/+page.ts` — Client-side page logic.
        - `<route name>/+page.css` — Page styles.
        - `<route name>/+page.md` — Page documentation.
        - `<route name>/_<component name>.svelte` — Page-specific components.
    - `app.css` — Global styles.
- `src-tauri/` — **Rust backend wrapper**
    - `src/` — Rust source, including `main.rs` and custom commands.
- `messages/` — Translation files, including `en.json`.
- `docs/` — Reference documentation for LLM context.
  For AI-assisted translations and review, read `../../docs/LLM_LOCALIZATION_WORKFLOW.md`.
  Use the shared website playbook when it is available.
- `scripts/` — Build and test utility scripts.

### Technology Stack

- **Framework:** SvelteKit 2 + Svelte 5 (Runes)
- **UI:** Tailwind CSS 4 + Bits UI/headless primitives for the Ace-owned rebuild.
  Existing Shadcn Svelte components are legacy and can remain during migration.
- **Runtime:** Bun
- **Backend wrapper:** Tauri 2 (Rust)
- **i18n:** Paraglide JS
- **Engine client:** Typed remoc RPC through Tauri. No local REST/HTTP client.

## AI-led refactoring and feature boundaries

Read root `../../AGENTS.md`.
For refactoring, read `../../docs/ai/REFACTORING_PLAYBOOK.md`.
Before sharing components, stores, or modules across features, read
`../../docs/ai/BOUNDARIES_AND_ABSTRACTIONS.md`.
For untrusted inputs and typed forms, read `../../docs/ai/DOMAIN_TYPES.md`.
For independent architecture and type reviews, read `../../docs/ai/REVIEW_GATES.md`.
Use these documents when generic skill advice conflicts.
Root IPC and security rules always take precedence.

Keep route-specific components and state local by default.
Share UI code only when consumers have the same behavior and lifecycle.
Similar markup alone is not sufficient.
Do not add a shared store or `$effect` synchronization bus for independent routes.
The existing `$lib/engine-client/` facade owns the boundary to engine data.
Tauri owns native IPC.
Rust owns policy and validation at the trust boundary.
Do not bypass authorization or add a local HTTP API.
Keep editable form strings separate from parsed and validated domain values.
Create branded types or Zod schemas only when they enforce a real invariant or trust boundary.
Run `bun run check` and `bun run test:unit` in this directory when applicable.
Native IPC needs separate tests.

## Brand assets

The official artwork is in the sibling `ace-blocker-logo` repository.
Use the exported, **transparent** `ace-icon-light-transparent.svg`
and `ace-icon-dark-transparent.svg` in the UI.
Switch them through CSS `.dark`.
Do not place a solid-background icon on a themed surface.

The native **desktop application icon** uses the dark rounded-square
`ace-icon-dark` artwork: navy background behind the amber octagon and spade,
with transparent pixels only outside its rounded corners. This is different
from the installer EXE's download-badge logo and from the transparent UI logos.
The exact official PNG/SVG exports are checked in to
`src-tauri/branding/ace-icon-dark.{png,svg}`.
To regenerate the Windows, Linux and macOS desktop icons, run:
`pwsh ./scripts/generate-native-icons.ps1` from `apps/ace-app/`.
The script uses Tauri's icon generator and copies desktop-only assets to
`src-tauri/icons`. Do not automatically replace `ios/` or `android/`
platform-specific icon sets. iOS App Store icons must be opaque; Android
adaptive foregrounds use their own platform-specific requirements.
Keep web and in-app artwork transparent and theme-aware.
The app navigation rail must show the official transparent icon, not a placeholder letter.
Do not change the logo source repository when you use its exported files.

## UI Identity / Rebuild Direction

**Read `docs/ui-identity.md` before you create, redesign, or materially change visible UI.**
It defines the approved Ace App design.

Do not make Ace look like custom-styled shadcn.
Make it look like a **disciplined personal enforcement instrument**.
Do not make it look like a generic web dashboard inside Tauri.

Key rules:

- Keep the engine API as the stable UI boundary unless an API change is expressly authorized.
- Use **Bits UI/headless behavior with Ace-owned styles** for new components.
- Do not add new shadcn components only for convenience.
- Use Tailwind for implementation, not as the visual language.
- Use Signal amber (`#FFB000`) for intervention, commitment, and enforcement.
  Do not use amber for every primary action.
- Use warm Ink/Paper neutrals, IBM Plex fonts, and modest corner radii.
  Prefer borders, rails, inset surfaces, controlled motion, and structural state.
- Avoid generic rounded card grids, gradient washes, glow, glass, hover-lift cards,
  icon tiles, small uppercase eyebrow labels, and cyber/hacker motifs.
- Network Blocks, Resistance, Locks, Schedules, Site Packs, and engine state are signature concepts.
  Do not use one generic page template for all of them.
- Check visual changes with screenshots.
  Include light and dark themes and applicable active, inactive, locked, and error states.

### Responsive layout convention

- The default Tauri window is 800×600.
  Viewport breakpoints do not show the space left after navigation rails and workspace gutters.
- `PageLayout` provides the named CSS container `page`.
  Prefer `@container page (...)` for responsive rules in page-owned components.
- The shell workspace provides `ace-workspace`, and `Inspector` provides `inspector`.
  Use these names when layout depends on those surfaces rather than the browser viewport.
- Show primary facts at the default desktop window size.
  Collapse secondary columns only when the actual page container is narrow.
- Keep the navigation rail scrollable in short or zoomed windows.
  Do not hide its bottom links or focus ring.
- Use the Tauri **user-session** opener in `src/lib/utils/account-browser.ts` for account and Pro pages.
  Do not call Engine-side `webbrowser::open`.
  Use the existing allowlisted `aceblocker://` callback.
  Refresh cached account state when focus returns.
  Linux installs `ace-blocker-handler.desktop` for the interactive user.
  During Linux `tauri dev`, `scripts/prepare-linux-auth.ts` builds `ace-redirect` automatically.
  It calls `auth:register-linux` to install user-scoped `ace-blocker-dev-handler.desktop`.
  Read `docs/DEV_AUTOMATION.md`.
  Registration must not increase Engine permissions.
- Keep gated action labels and entitlements visible at all viewport widths.
  Do not depend on an absolutely positioned label that appears only on hover.
- A downloaded Category or Custom List is coverage data, not an active block.
  Tell users to attach it to a Network Block before it can enforce blocking.

### UI skill references (when available)

Before important UI work, check which skills and MCP tools are installed and accessible.
A listed skill or `skills-lock.json` entry does not prove installation.
Use verified skills when available.
Otherwise, use Ace UI rules, Svelte documentation, and repository checks.
Skills cannot replace root security, IPC, or domain-ownership rules.

For significant UI work, use these skills when available:

1. `frontend-design`.
2. `ui-ux-pro-max`.
3. `svelte-code-writer`, `svelte-core-bestpractices`, and `svelte-components`.
   Also use `svelte-runes` for reactive state.
4. `web-design-guidelines` for a final review, when applicable.
5. `design-taste-frontend` only for anti-template critique.
   It is not the authority for dashboard or multi-step product UI.

### Core Rule: Development Context

Most tasks change `./src` (frontend).
The Rust backend in `src-tauri` is primarily a wrapper.
Some tasks also need its commands, such as `is_debug_mode`.

## Package Manager & Workflow

- **Always** use `bun` for dependencies and scripts.
- Use `bun install`, not `npm install`.
- Use `bun run`, not `npm run`.
- Use `bun`, not `node`.
- Do **not** use `npm`, `pnpm`, or `yarn`.
- Use `bun run <script>` to run scripts.
- Use `bun add <package>` or `bun add -d <package>` to install packages.
- **Local development:** The desktop app and website are always running during development.
    - **App UI:** Open `http://localhost:1420` in a browser.
    - **App (Tauri):** Use the real Tauri runtime or documented disposable VM checks for native behavior.
      The MCP bridge plugin is no longer installed.
- **Git commits:** Make the commits yourself after verification.
  Stage only the reviewed files for the task.
  Do not use `git add .`.
  Follow root `AGENTS.md`.
  Do not ask the user to make the commit.

### Native WebdriverIO automation (debug builds only)

ACE uses WebdriverIO's Tauri service to test the **real native WebView**.
These tests can navigate, interact with the DOM, and take screenshots through the real engine.
Read `docs/DEV_AUTOMATION.md`.

- Rust registers `tauri-plugin-wdio` and `tauri-plugin-wdio-webdriver`
  only under `#[cfg(debug_assertions)]`.
  Release builds must not register them.
  No other feature flag, mode flag, or Tauri configuration is needed.
- The guest JavaScript plugin in `src/hooks.client.ts` loads only in Vite dev mode in Tauri.
  Release bundles do not include it.
- Run `bun run tauri dev` to build the debug binary.
  Then run `bun run test:native` to start a second native instance through the embedded WebDriver server.
  See `e2e/native/read-only.smoke.spec.ts`.
- Browser-only `http://localhost:1420` does not have native IPC.
  Use native WDIO tests for flows that need engine access.
- WDIO can execute JavaScript inside the WebView.
  Keep this automation local.
  Use disposable data for tests that change state.
  Do not bypass locks or engine RPC.
  Do not add a privileged HTTP endpoint.

## Ace Engine Access

The frontend uses `src/lib/engine-client` as its stable engine interface.
The Tauri backend uses typed remoc RPC through a local named pipe.
There is no local REST server, Swagger or OpenAPI generator, API port, or HTTP fallback.

Read root `docs/IPC_ARCHITECTURE.md` before you change IPC or authorization.
Preserve engine lock checks.
Handle explicit error and data results and streaming status updates.
Localize errors that users can see.
The TypeScript interface uses `RpcFailure` from `src/lib/engine-client/rpc-error.ts`.
Do not use Fetch-style `response.status` objects for these errors.
`rpc-contract.test.ts` checks the operation map against the current Rust bridge.

Network Block editing sends one complete draft through
`src/routes/network-blocks/_save-network-block.ts`.
The transactional `saveNetworkBlock` remoc operation saves this draft.
Only Save changes block configuration.
If the RPC acknowledgment is lost after COMMIT, the result is uncertain.
Refresh state before you try again.
Do not retry without a check or say that no change was saved.

Use the disposable Windows VM test gate in
`docs/ai/ACE_APP_NATIVE_TEST_GATE.md` for native IPC and capability tests.

**Do not let the desktop WebView shut down the engine.**
`src-tauri/src/lib.rs` rejects `shutdown` before it loads the capability.
Keep the independent installer and engine maintenance RPC.

`src/lib/deltas/schedule-coverage.ts` uses the current engine interval format.
An end time before the start time is invalid.
The exception is `00:00` as the end of the selected day.
Do not restore unsupported overnight editor behavior without an approved engine policy change.

## Localization (Paraglide JS)

- **Mandatory:** Localize all user-facing text through Paraglide.
  Do not put hardcoded UI strings in components.
- **Workflow:**
    1. Add a key to `messages/en.json`. Example: `"snake_case_key": "Text content"`.
    2. Run `bun run check`, or let the dev server generate types.
    3. Import `* as m` from `$lib/paraglide/messages.js`.
    4. Use `m.snake_case_key()` in Svelte or TypeScript.
- **Rules:**
    - By default, edit only `en.json`.
      Do not edit `de.json`, `fr.json`, or other languages unless the user asks.
    - Use `{paramName}` in JSON for variables.
      Example: `"hello_user": "Hello {name}"`.
      Pass an object: `m.hello_user({ name: 'User' })`.

### Engine Docs Reference

- Read `docs/engine-docs.md` and `docs/IPC_ARCHITECTURE.md` before engine work.
- `boredom-engine` at `../boredom-engine/` provides resistance scripts.
  Ace Engine injects these scripts into blocked sites.

## Scripts

Use `bun run <script>` for these tasks:

- `dev`: Start Vite. Actual engine RPC requires Tauri.
- `clear-cache`: Clear app and build caches.
- `tauri`: Run the Tauri CLI, such as `bun run tauri dev`.
- `i18n:compile`: Regenerate Paraglide messages after you edit `messages/en.json`.
- `i18n:audit`: Check all English keys against static Paraglide references.
  This includes function references, not only calls.
  The audit fails for stale, missing, dynamic, or unused keys.
- `check`: Recompile Paraglide, then run Svelte-check for type checking and linting.

**Current SvelteKit 3 / CI conventions:**

- Import `dev` from `$app/env`, not the removed `$app/environment`.
  Import `Handle` and `Reroute` types from `@sveltejs/kit/hooks`.
- Use the generated `RouteId` type from `$app/types` for known routes when possible.
- Do not include generated `build/` output in the root TypeScript `include`.
  Otherwise, `svelte-check --tsgo` can scan minified output.
  It can fail with `RangeError: Invalid string length` instead of useful diagnostics.
- SvelteKit 3 makes `RequestEvent.request` readonly.
  Localized request middleware must pass a new event to `resolve`.
  Do not assign to the original event.

**Use TypeScript** for new utility and maintenance scripts in `/scripts/`.

## Nomenclature & Design Patterns

- **Svelte components:** Name all `.svelte` files in `UpperCamelCase`.
  Examples: `Button.svelte` and `UserCard.svelte`.
- **Page-specific components:** Put them in a route-local `./_components` folder.
  You can also prefix the filename with an underscore, such as `_Component.svelte`.
  This distinguishes them from routes.
- **Reactive collections:** Use `SvelteMap` or `SvelteSet` from `svelte/reactivity`.
  Use them for reactive maps and sets.
  Avoid plain objects when you frequently add or change keys.
- **`/messages/`:** Keep Paraglide translation files here.
  Prefer English (`en`) during development.
  Translate only when the user expressly requests it.

### Safe i18n Patterns

- **Mandatory:** Do not index the Paraglide `m` object with a variable.
  Example of a prohibited pattern: `m[someVariable]`.
  This can cause runtime `TypeErrors` and bypass TypeScript checks.
- Use explicit helper functions with `switch` or `if/else` statements.
  Map values to direct `m.key()` calls.
- Example:

    ```typescript
    const getLabel = (type: string) => {
        if (type === "static") return m.static_mode();
        return "";
    };
    ```

### First-Run Experience

The onboarding flow has three parts in `src/routes/+layout.svelte`:

1. **`TosModal`:** A Terms of Service and Privacy Policy gate that users cannot dismiss.
   The app shell remains inactive until account information loads.
   When acceptance is required, users must check both acknowledgments.
   `/account/accept-tos` must succeed before the gate closes.
   The engine stores the combined legal acceptance.
2. **`OnboardingWizard`** (`lib/components/onboarding/`): A five-step setup wizard.
   It appears once after the Terms of Service.
   It reads engine data through `getDashboardSnapshot()`.
   Users can enable seeded default blocks.
   This adds a 24/7 schedule entry because a block without a schedule never matches.
   Users can also enable Resistance Mode.
3. **`TutorialTour`** (`lib/components/onboarding/`): A tour with spotlights.
   It navigates between routes and highlights elements with `data-tour` attributes.
   Steps are in `lib/onboarding/tour-steps.ts`.
   Add `data-tour="..."` to an element if a tour step must point to it.

- Completion state is in `lib/stores/onboarding.svelte.ts`.
  It uses `localStorage` key `ace:onboarding:v1`.
  This state applies only to the UI.
  The engine stores all changes made by the wizard.
- Users can replay the wizard and tour from `/getting-started`.

### Resistance Mode UI

- Resistance has five independent, top-level intervention Categories:
  **Feeds and suggestions**, **Videos and playback**, **Visual stimulation**,
  **Delays and limits**, and **Verifications (CAPTCHA)**.
  Start all five collapsed.
  Do not add supergroups or a global intensity level.
- `src/routes/resistance-mode/_components/ScriptSections.svelte` has a three-state
  checkbox in the collapsed header of **each of the five** Categories.
  Its states are empty, mixed, and checked.
  Keep the checkbox separate from the disclosure button for mouse and keyboard input.
  Selecting a Category must not expand it.
  CAPTCHA interventions are off by default in the boredom-engine catalog.
  The header checkbox can enable all CAPTCHA scripts explicitly.
  Do not overwrite saved user choices to apply defaults.
- A Time Lock prevents users from disabling protected interventions.
  It also prevents clearing a fully selected Category.
  Users can still enable additional interventions.
- Put Resistance script catalog updates in
  `/settings#resistance-scripts-update` and `_ResistanceScriptsUpdateSection.svelte`.
  The installed version is for information only.
  Do not show an update action on the Resistance configuration page.
- After changes, run `bun run check` and `bun run test:unit`.
  Also run browser fixtures in `e2e/resistance-five-categories.spec.ts`.

## Documentation

**Mandatory:** Before work with the technologies below, read the linked `llms.txt` documentation.
Do not rely only on previous knowledge of these libraries.
Outdated patterns can cause errors.

| Technology | Documentation | Purpose |
| ---------- | ------------- | ------- |
| Svelte 5 & SvelteKit | [llms.txt](https://svelte.dev/llms.txt) | Full-stack framework. **Use Svelte 5 runes (`$state`, `$derived`, `$effect`, etc.).** |
| Shadcn Svelte | [llms.txt](https://shadcn-svelte.com/llms.txt) | Legacy UI components. Consult only to maintain existing shadcn components. Do not use them as the new visual foundation. |
| Paraglide JS | [llms.txt](https://inlang.com/g/6d900h6s/paraglide-js) | Localization framework. |
| Lucide Svelte | [llms.txt](https://lucide.dev/guide/packages/lucide-svelte) | Icon library. Use `@lucide/svelte`, **not** deprecated `lucide-svelte`. |
| Tauri | [llms.txt](https://tauri.app/llms.txt) | Desktop wrapper, IPC commands, and native APIs. |
| Engine IPC | [architecture](../../docs/IPC_ARCHITECTURE.md) | Typed remoc trait, Tauri bridge, and frontend RPC interface. |
| Zod | [llms.txt](https://zod.dev/llms.txt) | Schema validation and type checks. |
| BitsUI | [llms.txt](https://www.bits-ui.com/llms.txt) | Headless Svelte component library. |

**Always read `docs/engine-docs.md` before a task.**
This explains the engine and its operation.
For visible UI or design tasks, also read `docs/ui-identity.md`.

## MCP tool workflows (only when connected)

These integrations are optional.
Some agent sessions do not have them.
If a tool is missing, use official documentation and local compiler or tests.
Do not stop unrelated work or documentation-only tasks only because an optional tool is missing.
Do not report a tool run unless it actually ran.

### 1. list-sections

Use this tool first to list available documentation sections.
It returns titles, `use_cases`, and paths.
For Svelte or SvelteKit questions, use it at the start when it is available.

### 2. get-documentation

Use this tool to read complete documentation for the selected sections.
It accepts one or more sections.
First review the `use_cases` values from `list-sections`.
Then request all sections applicable to the task.

### 3. svelte-autofixer

This tool checks Svelte code and suggests corrections.
Use it before you deliver Svelte code.
Run it again until no issues or suggestions remain.

### 4. playground-link

This tool makes a Svelte Playground link from code.
After you finish code, ask whether the user wants the link.
Use this tool only after the user agrees.
Do **not** use it for code that you already wrote to project files.

### 5. tauri-mcp-server

When connected, use this tool for native UI and runtime checks.
If it is not connected, report that native verification was not run.
Vite or browser tests alone cannot verify remoc or installed Windows services.

## Skill catalog (check availability)

The names below are references selected earlier.
Check that an actual `SKILL.md` exists before you use a skill.
A catalog or lockfile entry does not prove that a skill is installed.
Read the skill instructions and executable scripts.
Do not automatically download or run third-party skills.
During refactoring, root requirements and Ace-specific architecture and type guides take precedence.

These skills were previously tracked through `npx skills`.
They can be absent from `.agents/skills/` or from the current environment.
Check each applicable skill before use.

### Svelte / SvelteKit

- **svelte-code-writer** (`sveltejs/ai-tools`) — Official Svelte AI tool for writing Svelte components.
- **svelte5-best-practices** (`ejirocodes/agent-skills`) — Svelte 5 patterns, including runes and snippets.
- **svelte-core-bestpractices** (`sveltejs/ai-tools`) — Core Svelte rules and conventions.
- **sveltekit-structure** (`spences10/svelte-skills-kit`) — SvelteKit project structure rules.
- **svelte-runes** (`spences10/svelte-skills-kit`) — Svelte 5 `$state`, `$derived`, and `$effect` patterns.
- **sveltekit-data-flow** (`spences10/svelte-skills-kit`) — SvelteKit data loading, forms, and actions.
- **svelte-components** (`spences10/svelte-skills-kit`) — Svelte component patterns and composition.

### UI & Styling

- **frontend-design** — Required for creating or materially changing visible UI.
  It helps set a deliberate visual direction.
- **ui-ux-pro-max** — Required for product-wide UI/UX, accessibility, typography, layout, and interaction.
- **svelte-components** — Patterns for Bits UI, headless components, and composition.
- **design-taste-frontend** — Use only to identify generic design patterns in Ace App.
  It is not the authority for dashboard or multi-step product UI.
- **web-design-guidelines** — Final UI and accessibility review when applicable.

- **shadcn** (`shadcn/ui`) — Reference for legacy maintenance only.
  Its default visual style is not Ace identity.
- **shadcn-svelte** (`vercel-labs/json-render`) — Reference for legacy maintenance only.
- **tailwind-v4-shadcn** (`secondsky/claude-skills`) — Legacy integration reference.
  Its default visual style is not the target system.
- **suggest-lucide-icons** (`nweii/agent-stuff`) — Ordinary utility icons.
  Core Ace concepts can use custom symbols.

### Tauri

- **tauri-v2** (`nodnarbnitram/claude-code-extensions`) — Tauri v2 patterns and commands.

### Runtime & Language

- **bun-development** (`sickn33/antigravity-awesome-skills`) — Bun runtime practices.
- **typescript-advanced-types** (`wshobson/agents`) — Advanced TypeScript types and patterns.
- **zod** (`pproenca/dot-skills`) — Zod schema validation patterns.

### Utilities

- **i18n-localization** (`sickn33/antigravity-awesome-skills`) — Localization practices.
- **git-commit** (`github/awesome-copilot`) — Git commit messages and conventions.
