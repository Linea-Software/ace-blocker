import { describe, expect, it } from "vitest";
import type { SitePackSchema } from "$engine-client";
import catalog from "$lib/data/site-packs.json";
import {
    getCategoryCatalogPair,
    getOfficialCatalogEntry,
    getSitePackCoverage,
    getSitePackIconSlug,
    groupSitePackCategories,
} from "./site-pack-presentation";

function pack(id: string, changes: Partial<SitePackSchema> = {}): SitePackSchema {
    const source = catalog.find((item) => item.id === id);
    if (!source) throw new Error("Missing test catalog entry " + id);
    return {
        id: source.id,
        name: source.name,
        description: source.description,
        isRemote: true,
        isSynced: false,
        entriesCount: 0,
        requiresPro: source.requires_pro,
        isBundled: false,
        isLocked: false,
        ...changes,
    };
}

describe("official site-pack presentation", () => {
    it("supplies a renderable icon for every curated category", () => {
        for (const item of catalog) {
            const entry = pack(item.id);
            expect(getSitePackIconSlug(entry), item.id).toBe(item.icon);
            expect(item.icon.length, item.id).toBeGreaterThan(0);
        }
    });

    it("finds remote UUID categories by exact tier-specific names", () => {
        const category = pack("pack-adult-pro", { id: "019bd28a-9417-7aef-a30e-44a95a7e7e2c" });
        expect(getOfficialCatalogEntry(category)?.id).toBe("pack-adult-pro");
        expect(getSitePackIconSlug(category)).toBe("adult");
        expect(getCategoryCatalogPair(category)).toEqual({
            free: 153512,
            pro: 2644357,
        });
    });

    it("never treats a user-created list as a curated category based on its name", () => {
        const own = pack("pack-adult-pro", {
            id: "custom-id",
            isRemote: false,
            icon: "photo",
        });
        expect(getOfficialCatalogEntry(own)).toBeUndefined();
        expect(getSitePackIconSlug(own)).toBe("photo");
        expect(getCategoryCatalogPair(own)).toBeNull();
        expect(getSitePackIconSlug({ ...own, icon: null })).toBe("globe");
    });

    it("uses measured installed coverage before an offline catalog estimate", () => {
        const remote = pack("pack-gambling-pro");
        expect(getSitePackCoverage(remote)).toEqual({
            count: 454090,
            source: "catalog",
        });
        expect(getSitePackCoverage({ ...remote, isSynced: true, entriesCount: 9 })).toEqual({
            count: 9,
            source: "installed",
        });
        const custom = { ...remote, id: "custom", isRemote: false };
        expect(getSitePackCoverage(custom)).toEqual({ count: null, source: "unknown" });
    });

    it("groups Free and Pro under one name without changing underlying IDs", () => {
        const groups = groupSitePackCategories([
            pack("pack-gambling-pro"), pack("pack-gambling-free"),
            pack("pack-streaming-free"), pack("pack-anime-manga-pro"),
        ]);
        const gambling = groups.find((group) => group.name === "Gambling");
        expect(gambling?.packs.map((entry) => entry.id)).toEqual([
            "pack-gambling-free", "pack-gambling-pro",
        ]);
        expect(gambling?.catalogPair).toEqual({ free: 14, pro: 454090 });
        expect(groups.length).toBe(3);
    });
});
