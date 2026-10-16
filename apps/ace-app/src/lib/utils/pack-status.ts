import type { SitePackSchema } from "$engine-client";

/**
 * The availability states a site pack can be in, used to keep the UI honest
 * about whether a pack can actually block anything:
 *
 * - `downloading` — a sync/resync is in flight (SSE progress visible).
 * - `bundled-ready` — local rules exist and came from the installer bundle
 *   (engine `isBundled`); usable offline.
 * - `downloaded-ready` — local rules exist and were downloaded/updated from
 *   the server; usable.
 * - `download-required` — only metadata exists (no usable local rules); it
 *   must be downloaded before it can be attached to a block. A metadata-only
 *   pack must never be presented as installed/enabled/functional.
 * - `unavailable` — the engine reports local data but it has no usable rules
 *   (defensive; treated like download-required by callers).
 */
export type PackStatus =
    | "downloading"
    | "bundled-ready"
    | "downloaded-ready"
    | "download-required"
    | "unavailable";

/**
 * Derives a pack's availability status from the engine schema plus the
 * transient syncing flag. Pure and unit-testable: no stores are imported.
 */
export function derivePackStatus(
    pack: SitePackSchema,
    isSyncing: boolean,
): PackStatus {
    if (isSyncing) return "downloading";
    if (pack.isSynced) {
        if (pack.isBundled) return "bundled-ready";
        return "downloaded-ready";
    }
    // Metadata-only row: never pretend it is ready.
    return "download-required";
}

/** True when the pack has usable local rules (any "ready" flavour). */
export function isPackReady(pack: SitePackSchema, isSyncing = false): boolean {
    const status = derivePackStatus(pack, isSyncing);
    return status === "bundled-ready" || status === "downloaded-ready";
}
