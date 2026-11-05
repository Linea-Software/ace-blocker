import { describe, expect, it } from "vitest";
import { getSiteIcon, groupScripts } from "./script-groups";
import type { ScriptInfo } from "$engine-client";

const sample = (domain: string, name: string): ScriptInfo => ({
    id: domain + ":" + name,
    domain,
    name,
    description: "",
    isEnabled: true,
});

describe("Resistance script site icons", () => {
    it("resolves bundled logos for supported domains and their subdomains", () => {
        for (const domain of [
            "youtube.com",
            "m.youtube.com",
            "instagram.com",
            "reddit.com",
            "tiktok.com",
            "twitch.tv",
            "facebook.com",
        ]) {
            expect(getSiteIcon(domain)?.svg).toContain("<svg");
        }
    });

    it("retains official brand colors for each rendered site logo", () => {
        expect(getSiteIcon("youtube.com")?.hex).toBe("FF0000");
        expect(getSiteIcon("reddit.com")?.hex).toBe("FF4500");
        expect(getSiteIcon("instagram.com")?.hex).toMatch(/^[0-9A-F]{6}$/i);
    });

    it("does not match lookalike domains and uses a neutral fallback for unknown sites", () => {
        expect(getSiteIcon("notyoutube.com")).toBeNull();
        expect(getSiteIcon("youtube.com.evil.example")).toBeNull();
        expect(getSiteIcon("Global")).toBeNull();
    });

    it("attaches the logo to grouped script cards", () => {
        const groups = groupScripts([
            sample("youtube.com", "Shorts"),
            sample("youtube.com", "Comments"),
            sample("other.example", "Unknown"),
        ]);
        expect(groups[0].domain).toBe("other.example");
        expect(groups[0].icon).toBeNull();
        expect(groups[1].icon?.svg).toContain("<svg");
        expect(groups[1].totalCount).toBe(2);
    });
});
