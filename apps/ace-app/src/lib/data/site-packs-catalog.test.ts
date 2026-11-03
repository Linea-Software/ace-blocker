import { describe, expect, it } from "vitest";
import sitePacks from "./site-packs.json";

const managedCategories: Record<string, string> = {
    "pack-adult-free": "adult",
    "pack-adult-pro": "adult-extended",
    "pack-social-free": "social",
    "pack-gaming-free": "gaming",
    "pack-gambling-pro": "gambling",
    "pack-messaging-pro": "messaging",
    "pack-streaming-pro": "streaming",
    "pack-shopping-pro": "shopping",
    "pack-anime-manga-pro": "anime-manga",
    "pack-anime-manga-extended": "anime-manga-extended",
    "pack-forums-pro": "forums",
    "pack-news-pro": "news",
    "pack-sports-pro": "sports",
    "pack-dating-pro": "dating",
    "pack-ai-tools-pro": "ai-tools",
};

describe("Ace-managed category catalog", () => {
    it("keeps canonical ids unique and exports whole release files", () => {
        expect(new Set(sitePacks.map((pack) => pack.id)).size).toBe(
            sitePacks.length,
        );
        for (const [id, category] of Object.entries(managedCategories)) {
            const pack = sitePacks.find((candidate) => candidate.id === id);
            expect(pack, id).toBeDefined();
            expect(pack?.sources).toEqual([
                {
                    source: "https://github.com/Linea-Software/ace-hosts/releases/latest/download/" + category + ".txt",
                    source_type: "url",
                    format: "standard",
                },
            ]);
        }
    });

    it("includes anime, manga, manhwa and manhua in the same focused category", () => {
        const anime = sitePacks.find(
            (pack) => pack.id === "pack-anime-manga-pro",
        );
        for (const content of ["anime", "manga", "manhwa", "manhua", "webtoons"]) {
            expect(anime?.description.toLowerCase()).toContain(content);
        }
        expect(anime?.requires_pro).toBe(true);
    });

    it("ships measured counts for the newly built category packs", () => {
        for (const id of [
            "pack-anime-manga-pro",
            "pack-anime-manga-extended",
            "pack-forums-pro",
            "pack-news-pro",
            "pack-sports-pro",
            "pack-messaging-pro",
            "pack-dating-pro",
            "pack-ai-tools-pro",
            "pack-shopping-pro",
        ]) {
            const pack = sitePacks.find((candidate) => candidate.id === id);
            expect(pack?.domains_count, id).toBeGreaterThan(0);
            expect(pack?.uncompressed_size_bytes, id).toBeGreaterThan(0);
            expect(pack?.compressed_size_bytes, id).toBeGreaterThan(0);
        }
    });
});
