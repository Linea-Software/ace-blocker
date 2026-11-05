import { describe, expect, it } from "vitest";
import {
    derivePackStatus,
    isPackReady,
    type PackStatus,
} from "./pack-status";
import type { SitePackSchema } from "$engine-client";

function pack(overrides: Partial<SitePackSchema>): SitePackSchema {
    return {
        id: "pack-adult-free",
        name: "Adult Websites — Free",
        description: "Blocks known adult content websites.",
        icon: null,
        isSynced: false,
        isRemote: true,
        isBundled: false,
        isLocked: false,
        entriesCount: 0,
        requiresPro: false,
        lastUpdatedAt: null,
        version: null,
        ...overrides,
    };
}

describe("derivePackStatus", () => {
    it("treats a syncing pack as downloading even when data exists", () => {
        const status = derivePackStatus(pack({ isSynced: true }), true);
        expect(status).toBe<PackStatus>("downloading");
    });

    it("reports bundled data as bundled-ready", () => {
        const status = derivePackStatus(
            pack({ isSynced: true, isBundled: true, entriesCount: 100 }),
            false,
        );
        expect(status).toBe<PackStatus>("bundled-ready");
        expect(isPackReady(pack({ isSynced: true, isBundled: true }))).toBe(true);
    });

    it("reports downloaded data as downloaded-ready", () => {
        const status = derivePackStatus(
            pack({ isSynced: true, entriesCount: 100, version: "1.2.3" }),
            false,
        );
        expect(status).toBe<PackStatus>("downloaded-ready");
        expect(isPackReady(pack({ isSynced: true }))).toBe(true);
    });

    it("never reports a metadata-only pack as ready", () => {
        // The ghost-pack guard: a row with no local rules must be
        // download-required, not installed/enabled/functional.
        const status = derivePackStatus(pack({ isSynced: false }), false);
        expect(status).toBe<PackStatus>("download-required");
        expect(isPackReady(pack({ isSynced: false }))).toBe(false);
    });

    it("keeps a stale 'synced' claim from being treated as ready when it is actually unavailable", () => {
        // Defensive: if the engine ever reports synced with no entries, we
        // classify it as unavailable instead of downloaded-ready.
        const status = derivePackStatus(pack({ isSynced: false, entriesCount: 0 }), false);
        expect(status).not.toBe("downloaded-ready");
    });
});
