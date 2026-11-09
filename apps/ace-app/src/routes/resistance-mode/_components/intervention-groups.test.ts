import { describe, expect, it } from "vitest";
import type { ScriptInfo } from "$engine-client";
import { classifyIntervention, groupInterventions } from "./intervention-groups";

function item(id: string, domain: string, isEnabled = false): ScriptInfo {
    return { id, domain, isEnabled, name: id, description: "" };
}

describe("independent Resistance interventions", () => {
    it("uses stable script IDs instead of mutable or localized descriptions", () => {
        expect(classifyIntervention(item("youtube_shorts_remove", "youtube.com"))).toBe("video");
        expect(classifyIntervention(item("youtube_audio_delay", "youtube.com"))).toBe("interruptions");
        expect(classifyIntervention(item("reddit_random_captcha", "reddit.com"))).toBe("captcha");
        expect(classifyIntervention(item("youtube_limit_scrolling", "youtube.com"))).toBe("interruptions");
        expect(classifyIntervention(item("reddit_grayscale", "reddit.com"))).toBe("visual");
        expect(classifyIntervention(item("reddit_hide_recommendations", "reddit.com"))).toBe("feeds");
    });

    it("places every intervention exactly once, with truthful configured counts", () => {
        const scripts = [
            item("youtube_shorts_remove", "youtube.com", true),
            item("youtube_audio_delay", "youtube.com"),
            item("reddit_grayscale", "reddit.com", true),
            item("reddit_hide_recommendations", "reddit.com"),
            item("reddit_random_captcha", "reddit.com"),
        ];
        const sections = groupInterventions(scripts);
        expect(sections).toHaveLength(5);
        expect(sections.find((section) => section.kind === "captcha")?.totalCount).toBe(1);
        expect(sections.reduce((sum, s) => sum + s.totalCount, 0)).toBe(scripts.length);
        expect(sections.reduce((sum, s) => sum + s.selectedCount, 0)).toBe(2);
        expect(sections.flatMap((s) => s.sites.flatMap((site) => site.scripts.map((script) => script.id))).sort())
            .toEqual(scripts.map((s) => s.id).sort());
    });

    it("keeps unknown future IDs visible and does not confuse a configured intervention with enforcement", () => {
        const sections = groupInterventions([item("future_custom_action", "new.example")]);
        expect(sections.find((section) => section.kind === "feeds")?.sites[0].domain).toBe("new.example");
        expect(sections.find((section) => section.kind === "feeds")?.selectedCount).toBe(0);
    });
});
