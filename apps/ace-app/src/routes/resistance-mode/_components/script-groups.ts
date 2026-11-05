import {
    siFacebook,
    siInstagram,
    siReddit,
    siTiktok,
    siTwitch,
    siYoutube,
} from "simple-icons";
import type { ScriptInfo } from "$engine-client";

/** Simple-icons brand map for known covered domains. */
const iconMap: Record<string, { svg: string; hex: string } | undefined> = {
    "facebook.com": siFacebook,
    "instagram.com": siInstagram,
    "reddit.com": siReddit,
    "tiktok.com": siTiktok,
    "twitch.tv": siTwitch,
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
