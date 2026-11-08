/**
 * Accept a pasted website URL or hostname, but store only the hostname.
 * Network Block domain rules operate on hosts, not URL paths or queries.
 */
const hostnamePattern = /^(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z](?:[a-z0-9-]{0,61}[a-z0-9])?$/i;

export function websiteHostname(input: string): string | null {
    const text = input.trim();
    if (!text || /\s/.test(text) || text.includes("@")) return null;

    const explicitProtocol = /^[a-z][a-z\d+.-]*:\/\//i.test(text);
    if (text.includes("://") && !explicitProtocol) return null;

    try {
        const url = new URL(explicitProtocol ? text : `https://${text}`);
        if (url.protocol !== "https:" && url.protocol !== "http:") return null;
        const hostname = url.hostname.replace(/\.$/, "").toLowerCase();
        if (!hostnamePattern.test(hostname) || hostname.length > 253) return null;
        return hostname;
    } catch {
        return null;
    }
}

/** Normalize bulk pasted domains/URLs without changing any saved policy. */
export function parseWebsiteList(text: string, existing: readonly string[]) {
    const known = new Set(existing.map((domain) => domain.toLowerCase()));
    const websites: string[] = [];
    const skipped: string[] = [];
    for (const token of text.split(/[\s,;]+/).filter(Boolean)) {
        const domain = websiteHostname(token);
        if (!domain || known.has(domain)) {
            skipped.push(token);
            continue;
        }
        websites.push(domain);
        known.add(domain);
    }
    return { websites, skipped };
}
