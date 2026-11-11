# Resistance: independent interventions

Resistance makes supported websites less distracting without blocking access. Resistance requires Pro. Free users can see a read-only configuration preview.

## Policy model

There is **no global intensity or severity scale**. Each intervention has one stable catalog `@id`, one defined behavior, and one enabled/disabled state. Each script fixes its behavior parameters, such as scroll caps, video quality, buffering delays, and grayscale strength.

- The `resistance_mode_settings` row stores `enabled`, lock information, and `scripts_version`. It does **not** store a mode or intensity.
- The `script_states` table stores explicit overrides keyed by permanent script ID.
- The catalog field `defaultEnabled` supplies the default when no override exists. Visual degradation, delays, CAPTCHA and scrolling limits start **off**; ordinary distraction-removal actions can start **on**.
- Keep explicit overrides by ID after catalog updates. This includes IDs temporarily absent from the current catalog.
- A Time Lock prevents users from disabling Resistance or turning **off** an existing intervention. Users can enable more interventions.
- Apply intervention changes to newly loaded documents. Do not remove scripts already injected into an open page.

## Enforcement, eligibility and status

Injection requires an enabled master switch, Pro entitlement, and a valid downloaded catalog. The requested host must have at least one enabled script. The proxy injects only relevant, enabled scripts as nonce-bearing inline tags. It rewrites CSP when permitted. The proxy does not inject a global intensity variable.

The `enforcementStatus` reasons, in priority order, are:

1. `disabled` — master switch off.
2. `not_pro` — license ineligible.
3. `scripts_missing` — no catalog installed.
4. `scripts_invalid` — catalog failed to load.
5. `no_scripts_enabled` — all interventions disabled.
6. `operational` — eligible, enabled, valid scripts loaded.

`operational` shows overall readiness. It does not guarantee success on every browser or site. CSP, network tunnels, and site changes can prevent individual scripts from running. The UI distinguishes *configured* from *currently operational*.

## API and backup

The client uses `getResistanceModeSettings`, `getAllScripts`, `setResistanceModeEnabled`, `setScriptEnabled`, `setResistanceModeLock` and `downloadResistanceScripts`. The previous mode/intensity and developer intensity override RPCs are removed.

Local backups preserve `deactivatedResistanceScripts` and `activatedResistanceScripts`. A backup cannot disable an enabled intervention while Resistance is locked. The current cloud-backup contract cannot store enabled opt-in overrides. Reject lossy cloud export and offer local export instead.

## Regression checks

Run `cargo test -p ace-engine --lib` to check persistence, locks, and the proxy. Run `bun run check` and `bun run test:unit` from `apps/ace-app`. Run `bun run build && bun run test:smoke` in `boredom-engine`. Verify categories, per-site toggles, Free previews, and one-way Time Lock rules. Verify content reload after intervention changes.
