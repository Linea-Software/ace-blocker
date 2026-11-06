import {
    siFacebook,
    siInstagram,
    siNetflix,
    siPinterest,
    siReddit,
    siSnapchat,
    siThreads,
    siTiktok,
    siTwitch,
    siX,
    siYcombinator,
    siYoutube,
} from "simple-icons";
import type { ScriptInfo } from "$engine-client";

// LinkedIn is no longer distributed by simple-icons. Use the bundled
// Tabler filled brand mark (MIT) instead of loading a remote favicon.
const linkedinIcon = {
    hex: "0A66C2",
    svg: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor"><path d="M17 2a5 5 0 0 1 5 5v10a5 5 0 0 1 -5 5h-10a5 5 0 0 1 -5 -5v-10a5 5 0 0 1 5 -5zm-9 8a1 1 0 0 0 -1 1v5a1 1 0 0 0 2 0v-5a1 1 0 0 0 -1 -1m6 0a3 3 0 0 0 -1.168 .236l-.125 .057a1 1 0 0 0 -1.707 .707v5a1 1 0 0 0 2 0v-3a1 1 0 0 1 2 0v3a1 1 0 0 0 2 0v-3a3 3 0 0 0 -3 -3m-6 -3a1 1 0 0 0 -.993 .883l-.007 .127a1 1 0 0 0 1.993 .117l.007 -.127a1 1 0 0 0 -1 -1"/></svg>',
};

/** Bundled brand logos for sites supported by boredom-engine. */
const iconMap: Record<string, { svg: string; hex: string } | undefined> = {
    "facebook.com": siFacebook,
    "instagram.com": siInstagram,
    "linkedin.com": linkedinIcon,
    "netflix.com": siNetflix,
    "pinterest.com": siPinterest,
    "reddit.com": siReddit,
    "snapchat.com": siSnapchat,
    "threads.net": siThreads,
    "tiktok.com": siTiktok,
    "twitch.tv": siTwitch,
    "x.com": siX,
    "news.ycombinator.com": siYcombinator,
    "youtube.com": siYoutube,
};

export function getSiteIcon(domain: string): { svg: string; hex: string } | null {
    const hostname = domain.toLowerCase().replace(/\.$/, "");
    for (const [siteDomain, icon] of Object.entries(iconMap)) {
        // Exact host or real subdomain, never a lookalike (e.g. notyoutube.com).
        if (hostname === siteDomain || hostname.endsWith("." + siteDomain)) {
            return icon ?? null;
        }
    }
    return null;
}

export type ScriptGroup = {
    domain: string;
    scripts: ScriptInfo[];
    activeCount: number;
    totalCount: number;
    icon: { svg: string; hex: string } | null;
};

/** Groups flat script entries by domain (generic scripts land in "Global"). */
export function groupScripts(scripts: ScriptInfo[]): ScriptGroup[] {
    const groups = new Map<string, ScriptInfo[]>();
    for (const script of scripts) {
        const domain = script.domain || "Global";
        const list = groups.get(domain) ?? [];
        list.push(script);
        groups.set(domain, list);
    }
    return [...groups.entries()]
        .map(([domain, items]) => ({
            domain,
            scripts: items,
            activeCount: items.filter((item) => item.isEnabled).length,
            totalCount: items.length,
            icon: getSiteIcon(domain),
        }))
        .sort((a, b) => {
            if (a.domain === "Global") return -1;
            if (b.domain === "Global") return 1;
            return a.domain.localeCompare(b.domain);
        });
}
