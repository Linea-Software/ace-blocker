/**
 * Formats the engine's `lockEnd` string (local naive datetime
 * `YYYY-MM-DDTHH:MM:SS`) into a human-readable local date/time.
 * Falls back to the raw value when parsing fails.
 */
export function formattedLockEnd(raw: string): string {
    try {
        const parsed = new Date(raw);
        if (Number.isNaN(parsed.getTime())) return raw;
        return parsed.toLocaleString();
    } catch {
        return raw;
    }
}
