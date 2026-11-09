# Resistance: independent interventions

The Resistance feature makes supported websites less distracting without blocking access to them. This feature is Pro-gated but the configuration can be shown to Free users as a read-only preview.

## Policy model

There is **no global intensity or severity scale**. Every intervention has one stable catalog `@id`, one predictable behavior, and its own enabled/disabled state. Behavior-changing parameters (scroll caps, video quality, buffering delays, grayscale strength) are fixed within the corresponding script.

- The `resistance_mode_settings` row stores `enabled`, lock information and `scripts_version`. It does **not** store a mode or intensity.
- The `script_states` table stores explicit overrides keyed by permanent script ID.
- The catalog field `defaultEnabled` supplies the default when no override exists. Visual degradation, delays, CAPTCHA and scrolling limits start **off**; ordinary distraction-removal actions can start **on**.
- Catalog updates preserve explicit overrides by ID, including IDs temporarily missing from the current catalog.
- A Time Lock prevents disabling Resistance and prevents turning **off** an existing intervention; enabling additional interventions is permitted.
- Changes to enabled interventions take effect on newly loaded documents. They are not retroactively removed from an already injected page.

## Enforcement, eligibility and status

Injection requires an enabled master switch, Pro entitlement, a valid downloaded catalog, and at least one enabled script for the requested host. The proxy injects only the relevant, enabled scripts as nonce-bearing inline tags and rewrites CSP when permitted. It does not inject any global intensity variable.

The `enforcementStatus` reasons, in priority order, are:

1. `disabled` — master switch off.
2. `not_pro` — license ineligible.
3. `scripts_missing` — no catalog installed.
4. `scripts_invalid` — catalog failed to load.
5. `no_scripts_enabled` — all interventions disabled.
6. `operational` — eligible, enabled, valid scripts loaded.

`operational` describes overall readiness, not guaranteed success on every browser/site: CSP, network tunnels, and site changes can prevent an individual script from applying. The UI distinguishes *configured* from *currently operational*.

## API and backup

The client uses `getResistanceModeSettings`, `getAllScripts`, `setResistanceModeEnabled`, `setScriptEnabled`, `setResistanceModeLock` and `downloadResistanceScripts`. The previous mode/intensity and developer intensity override RPCs are removed.

Local backups preserve both `deactivatedResistanceScripts` and `activatedResistanceScripts`; a backup cannot turn off a currently enabled intervention while Resistance is locked. The current remote cloud-backup contract cannot store enabled opt-in overrides: a lossy export is rejected, with local export offered instead.

## Regression checks

Run `cargo test -p ace-engine --lib` for persistence, lock and proxy checks, `bun run check` and `bun run test:unit` from `apps/ace-app`, and `bun run build && bun run test:smoke` in `boredom-engine`. Confirm categories, per-site toggles, Free previews, Time Lock one-way semantics, and content reloading after switching interventions.
