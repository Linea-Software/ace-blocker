# Ace App — UI Identity and Rebuild Brief

Status: **approved target direction** for the next Ace desktop frontend.

This document is the local source of truth for visual and interaction work in `ace-app`. It complements the project-level brief at `../docs/ai/UI_IDENTITY.md`.

## 1. Objective

Rebuild Ace App so it feels like a dedicated desktop control instrument rather than a web dashboard inside a Tauri shell.

The product thesis is:

> **Configure once. Hold the line.**

Ace exists so a decision made in a rational moment can still be enforced in a weak moment. The UI should make configuration calm and clear, and make enforcement/commitment feel structurally real.

Recognition target:

> Remove the Ace logo and product name from a screenshot. The app should still be recognizable from its palette, typography, geometry, state treatment, and signature domain components.

## 2. Technical foundation

Keep:

- Tauri 2;
- SvelteKit 2;
- Svelte 5;
- TypeScript;
- Bun;
- Tailwind CSS 4;
- Paraglide;
- generated `@hey-api` engine client;
- Vitest;
- Playwright.

Use **Bits UI as the preferred headless/accessibility primitive layer** for new Ace-owned UI.

The existing codebase contains shadcn-svelte components built on top of the same primitive ecosystem. Treat those as **legacy implementation**, not as the visual design system for the rebuild.

Rules:

- Do not add a new shadcn component simply to get a default-styled UI during the rebuild.
- Reusing a legacy shadcn component is acceptable while maintaining an old screen.
- New design-system components should own their Ace styling and API.
- Prefer headless behavior + Ace CSS over copying generic shadcn composition.
- Tailwind is an implementation tool; it is not the design language.

The engine API is a stable boundary for this frontend redesign. Do not change engine contracts merely to make the UI easier unless a task explicitly authorizes an API change.

## 3. Mandatory design skills

For any task that creates, redesigns, or materially alters visible UI:

1. Read this document.
2. Use `frontend-design`.
3. Use `ui-ux-pro-max`.
4. For Svelte implementation, use:
    - `svelte-code-writer`;
    - `svelte-core-bestpractices`;
    - `svelte-components`;
    - `svelte-runes` when reactive state is involved.
5. Use `web-design-guidelines` for a final standards/accessibility review when applicable.
6. Verify meaningful visual changes with actual screenshots/browser/Tauri UI.

`design-taste-frontend` can be used as an anti-template critique pass, but it explicitly is not a dashboard or multi-step product-UI skill. It must not replace product-specific UX reasoning.

## 4. Visual thesis

Read Ace as a **disciplined personal enforcement instrument**.

Qualities:

- decisive;
- physical;
- controlled;
- resistant;
- deliberate;
- durable;
- calm when idle;
- uncompromising at commitment boundaries.

Useful references: industrial controls, measuring equipment, technical signage, durable tools, editorial information design.

Avoid:

- military aesthetics;
- cyber/hacker theater;
- terminal cosplay;
- wellness/meditation softness;
- generic SaaS dashboard patterns.

## 5. Palette

### Core

| Token         | Hex       |
| ------------- | --------- |
| Signal        | `#FFB000` |
| Signal Bright | `#FFC533` |
| Signal Deep   | `#D58B00` |
| Ink           | `#10100E` |
| Graphite      | `#1A1A17` |
| Steel         | `#292925` |
| Ash           | `#8B8A82` |
| Paper         | `#F3F0E8` |
| Paper Strong  | `#FFFDF7` |
| Line          | `#D7D2C7` |

### Semantic

- Success: `#77A86A`
- Danger: `#DF5145`
- Information: `#668DB3`

### Meaning

Signal amber means **attention, intervention, enforcement, or commitment**.

Do not make every primary button amber. Ordinary edits/actions can remain monochrome. Signal is reserved for actions/states where Ace changes what the machine will allow or where the user is committing to enforcement.

Decorative feature colors are not part of the Ace language.

## 6. Typography

Target:

- **IBM Plex Sans** — normal UI;
- **IBM Plex Sans Condensed** — page titles and strong state labels;
- **IBM Plex Mono** — times, durations, counts, schedules, machine/status data.

Do not use monospace as generic “tech” decoration.

Avoid:

- all-caps eyebrow labels;
- excessive tracking;
- giant marketing-style display type inside the desktop app;
- relying on weight alone to create hierarchy.

## 7. Geometry and surfaces

Target radii:

- controls: 4–6 px;
- panels: 6–8 px;
- large structural surfaces: <= 8 px.

Use pills only for true compact states/metadata.

Ace-owned domain components may use a subtle notched/cut-corner signature. It should appear on important product objects, not every container.

Prefer borders, rails, rules, tonal surfaces, and inset regions over floating cards and shadows.

The app should feel **assembled**, not floated.

## 8. Application shell

Target shell:

- a thin navigation rail, roughly 64–72 px;
- main workspace;
- contextual right-side inspector when editing/selecting a domain object;
- global engine/enforcement state that remains easy to see.

Avoid rebuilding the current broad web-style sidebar.

Suggested top-level navigation:

- Control
- Blocks
- Resistance
- Categories
- Guide
- Settings

The exact labels may change after UX testing, but the information architecture should remain compact.

### Contextual inspector

Prefer a contextual inspector for lightweight object inspection and quick edits. The Network Block policy editor is deliberately a full-page route: its contents, schedule, and lock controls need more room than an inspector affords. Categories and Custom Lists retain inspectors.

Reserve dialogs for:

- destructive actions;
- commitment/lock actions;
- unusual multi-step operations.

## 9. Page archetypes

Different product concepts should not all become `PageHeader + Card grid`.

### Control

The home view is a **state board**, not a KPI dashboard.

It should answer:

1. Is Ace working?
2. What is being enforced right now?
3. What changes next?

Active policies and upcoming schedule transitions are more important than generic metric cards.

### Blocks

Network Blocks are **policy objects**.

Each block should visibly combine:

- name;
- current state;
- enforced schedule;
- policy contents;
- lock state;
- next relevant transition.

Use a strong state rail or equivalent structural indicator.

Locked state must visibly reduce/close editability, not merely add a tiny padlock badge.

### Resistance

Resistance is a signature Ace surface built around **independent interventions**.

Never show a global intensity/severity slider: one no longer exists in the engine.
Show a short concrete explanation and **five** compact, equally ranked
categories: feeds and suggestions; videos and playback; visual stimulation;
delays and limits; and simulated verifications (CAPTCHA). All start collapsed.
The five categories are the sole top level; do not group them under additional
headings or add another disclosure layer.

The five categories each expose an always-visible, three-state checkbox on
its header, independent from the expand button. Selecting a whole category
must **not** require opening the category or browsing its sites. Inside the
category, expose additional intervention selection by site. CAPTCHA scripts
default to off in fresh catalog installations but may be enabled as a group
with their checkbox; existing user preferences take precedence. Time Locks prevent clearing
or weakening protected selections, but permit adding more. Distinguish
configured selections from operational enforcement. The scripts update action
and installed version maintenance belong in Settings, not this page.

### Categories

Categories are Ace-provided curated sets of known sites. Custom Lists are user-imported domain collections. Present both as a dense, scannable catalog/inventory with Categories first and Custom Lists clearly secondary; prefer rows/table-like structure with an inspector over marketing cards.

The engine continues to model both as Site Packs. That name belongs to the implementation boundary, not the normal product vocabulary.

### Schedule

Prefer a week/time matrix or timeline that shows enforcement periods spatially and immediately.

## 10. Signature components

The first design-system components to solve well are:

1. `NetworkPolicy`
2. `ScriptSections (independent intervention browser)`
3. `LockSeal`
4. `ScheduleTimeline` / `WeekGrid`
5. `EngineState`
6. `SitePackRow` (internal component name; renders Categories and Custom Lists)
7. `Inspector`

If these are distinctive and coherent, most of the application will inherit identity naturally.

## 11. Lock interaction

Locking is a high-consequence action and deserves deliberate ceremony.

A dedicated commitment surface should clearly state:

- how long the lock lasts;
- what cannot be changed while locked;
- when editability returns.

A hold-to-confirm action is acceptable for a real lock/commitment, approximately long enough to feel intentional but not punitive. Do not apply hold-to-confirm to ordinary actions.

## 12. Motion

Typical duration: 120–180 ms.

Motion should explain structural change:

- inspector opens;
- policy expands;
- lock seal closes;
- schedule snaps;
- state rail changes.

Avoid:

- hover-lift cards;
- bouncy motion;
- ambient pulse;
- decorative section entrances;
- animation for “premium feel.”

Respect reduced-motion.

## 13. Copy and terminology

Clarity wins over brand cleverness.

A small repeated vocabulary can reinforce the product when it remains obvious:

- Armed — possible global enforcement state;
- Engage resistance — possible resistance action;
- Hold to lock — commitment action.

Do not rename familiar actions merely for personality. “Save changes” is better than an obscure branded verb.

## 14. Anti-patterns

Do not introduce as default:

- generic rounded card grids;
- gradient washes;
- blurred blobs;
- glassmorphism;
- giant soft shadows;
- rounded icon tile beside every title;
- tiny uppercase eyebrow labels;
- decorative rainbow feature colors;
- pills everywhere;
- arrows everywhere;
- premium glows;
- cybersecurity shield/hex/terminal motifs.

## 15. Architecture of the component system

Use three layers.

### Primitives

Examples:

- Button
- Input
- Toggle
- Menu
- Dialog
- Tooltip
- Scroller

These should be accessible, restrained, and mostly boring.

### Ace structural components

Examples:

- Workspace
- NavigationRail
- Inspector
- SectionRail
- StatusStrip
- CommitBar
- WeekGrid

These create the recurring visual language.

### Domain components

Examples:

- NetworkPolicy
- ScriptSections (independent intervention browser)
- LockSeal
- SitePackRow
- EngineState
- ScheduleTimeline

These carry the strongest identity.

Do not try to make the generic `Button` component carry the entire brand.

## 16. Accessibility and validation

Distinctive must not mean difficult.

Required:

- keyboard navigation;
- visible focus;
- semantic controls;
- correct labels;
- sufficient contrast in both themes;
- reduced-motion support;
- no state conveyed by color alone;
- target sizes appropriate for desktop input;
- readable zoom/scaling behavior.

Before considering a major visual task complete:

- run the relevant Svelte checks;
- use the Svelte autofixer on changed Svelte files;
- inspect the result visually;
- review representative light and dark states;
- test at least active/inactive/locked/error/offline states when the component supports them.

## 17. Migration principle

The current app may be replaced incrementally or substantially, but preserve the stable engine boundary and business semantics.

Do not create a transitional “new Ace” layer that simply wraps the old generic card language. When a screen is migrated, it should move toward the target system rather than retain the visual assumptions of the old one.
