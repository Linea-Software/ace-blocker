import type { ScriptInfo } from "$engine-client";
import { groupScripts, type ScriptGroup } from "./script-groups";

export type InterventionKind = "feeds" | "video" | "visual" | "interruptions" | "captcha";

export const INTERVENTION_KINDS: readonly InterventionKind[] = [
    "feeds",
    "video",
    "visual",
    "interruptions",
    "captcha",
];

/**
 * Actions are identified by stable catalog IDs, never localized copy.
 * Keep the fallback in "feeds" so every future catalog action is visible,
 * even before its classification has been reviewed.
 */
export function classifyIntervention(script: Pick<ScriptInfo, "id">): InterventionKind {
    const id = script.id.toLowerCase();
    if (/captcha/.test(id)) return "captcha";
    if (/delay|buffer|friction|wait|slow|limit_scrolling|scroll_limit/.test(id)) return "interruptions";
    if (/grayscale|desaturat|low_quality|media_quality|downgrade/.test(id)) return "visual";
    if (/short|reel|spotlight|autoplay|video|stream|playback/.test(id)) return "video";
    return "feeds";
}

export type InterventionSection = {
    kind: InterventionKind;
    sites: ScriptGroup[];
    totalCount: number;
    selectedCount: number;
};

export function groupInterventions(scripts: ScriptInfo[]): InterventionSection[] {
    return INTERVENTION_KINDS.map((kind) => {
        const relevant = scripts.filter((script) => classifyIntervention(script) === kind);
        return {
            kind,
            sites: groupScripts(relevant),
            totalCount: relevant.length,
            selectedCount: relevant.filter((script) => script.isEnabled).length,
        };
    });
}
