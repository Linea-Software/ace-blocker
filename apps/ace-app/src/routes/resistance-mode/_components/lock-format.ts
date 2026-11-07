import { getLocale } from "$lib/paraglide/runtime";

/**
 * Format the engine's absolute lock deadline in the selected app locale.
 * Date parses the original instant, including its RFC3339 offset.
 * Falls back to the raw value when parsing fails.
 */
export function formattedLockEnd(raw: string, locale = getLocale()): string {
    try {
        const parsed = new Date(raw);
        if (Number.isNaN(parsed.getTime())) return raw;
        return parsed.toLocaleString(locale);
    } catch {
        return raw;
    }
}
