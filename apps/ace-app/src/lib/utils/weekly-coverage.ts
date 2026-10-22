import type { ScheduleEntrySchema } from "$engine-client";

/**
 * Pure model for the weekly coverage grid (no Svelte imports — unit-testable
 * in node).
 *
 * A schedule entry declares a set of weekdays plus one `startTime`–`endTime`
 * range ("HH:MM"). The weekly coverage graph shows, per weekday, which hours
 * of the day are covered by at least one entry:
 *
 *  - overlapping entries combine (union) — the grid never double-counts;
 *  - end at 00:00 denotes the end of the same weekday; 00:00–00:00
 *    represents the full day. Other end-before-start intervals are rejected
 *    by the engine. Equal non-midnight bounds cover a single minute.
 *
 * This module computes minute-resolution ranges and hourly buckets plus
 * human-readable summaries; rendering lives in the component.
 */

export const WEEKDAY_KEYS = [
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
] as const;

export type WeekdayKey = (typeof WEEKDAY_KEYS)[number];

/** Minutes since midnight for an "HH:MM" string; NaN when malformed. */
export function timeToMinutes(time: string): number {
    const [hours, minutes] = time.split(":").map(Number);
    if (
        hours === undefined ||
        minutes === undefined ||
        Number.isNaN(hours) ||
        Number.isNaN(minutes)
    ) {
        return NaN;
    }
    return hours * 60 + minutes;
}

export interface MinuteRange {
    /** Inclusive start minute (0..1439). */
    start: number;
    /** Exclusive end minute (1..1440). */
    end: number;
}

export interface DayCoverage {
    day: WeekdayKey;
    /** Merged, sorted, non-overlapping minute ranges for the day. */
    ranges: MinuteRange[];
}

export type CoverageEditMode = "block" | "allow";
export type CoveragePreset = "always" | "weekdays" | "work_hours" | "clear";

export interface CoverageEntrySpec {
    days: WeekdayKey[];
    start: number;
    end: number;
}

function minutesToTime(minutes: number): string {
    const hour = Math.floor(minutes / 60);
    const minute = minutes % 60;
    return `${hour.toString().padStart(2, "0")}:${minute
        .toString()
        .padStart(2, "0")}`;
}

/**
 * Returns the minute intervals an entry contributes for the given weekday.
 * End at midnight means the end of the selected weekday, and equal
 * non-midnight times represent one selected minute (Rust engine semantics).
 */
function entryIntervalsForDay(
    entry: ScheduleEntrySchema,
    day: WeekdayKey,
): MinuteRange[] {
    const start = timeToMinutes(entry.startTime);
    const end = timeToMinutes(entry.endTime);
    if (!entry[day] || Number.isNaN(start) || Number.isNaN(end)) return [];

    if (end === 0) {
        return [{ start, end: 24 * 60 }];
    }
    if (end === start) {
        return [{ start, end: start + 1 }];
    }
    if (end > start) {
        return [{ start, end }];
    }
    // Untrusted/obsolete overnight ranges are not accepted by the engine.
    return [];
}

/**
 * Computes per-weekday coverage for a set of schedule entries. Ranges from
 * all entries are merged (overlaps combine) and sorted per day.
 */
export function computeWeeklyCoverage(
    entries: ScheduleEntrySchema[],
): DayCoverage[] {
    const result: DayCoverage[] = WEEKDAY_KEYS.map((day) => ({
        day,
        ranges: [],
    }));

    for (const entry of entries) {
        for (const day of WEEKDAY_KEYS) {
            const intervals = entryIntervalsForDay(entry, day);
            if (intervals.length === 0) continue;
            const row = result.find((r) => r.day === day)!;
            row.ranges.push(...intervals);
        }
    }

    for (const row of result) {
        row.ranges = mergeRanges(row.ranges);
    }

    return result;
}

/** Sorts and merges overlapping/adjacent minute ranges into a union. */
export function mergeRanges(ranges: MinuteRange[]): MinuteRange[] {
    const sorted = [...ranges].sort((a, b) => a.start - b.start || a.end - b.end);
    const merged: MinuteRange[] = [];
    for (const range of sorted) {
        const last = merged[merged.length - 1];
        if (last && range.start <= last.end) {
            // Overlapping or adjacent windows combine.
            last.end = Math.max(last.end, range.end);
        } else {
            merged.push({ start: range.start, end: range.end });
        }
    }
    return merged;
}

/** Removes one minute range from a set of merged ranges. */
export function subtractRange(
    ranges: MinuteRange[],
    removal: MinuteRange,
): MinuteRange[] {
    const start = Math.max(0, Math.min(24 * 60, removal.start));
    const end = Math.max(start, Math.min(24 * 60, removal.end));
    if (start === end) return ranges.map((range) => ({ ...range }));

    const result: MinuteRange[] = [];
    for (const range of ranges) {
        if (end <= range.start || start >= range.end) {
            result.push({ ...range });
            continue;
        }

        if (start > range.start) {
            result.push({ start: range.start, end: start });
        }
        if (end < range.end) {
            result.push({ start: end, end: range.end });
        }
    }
    return result;
}

/**
 * Applies a block/allow edit to minute-resolution weekly coverage without
 * mutating the supplied schedule entries. The interactive hour grid uses this
 * model before the parent translates the result back into schedule entries.
 */
export function editWeeklyCoverage(
    entries: ScheduleEntrySchema[],
    days: WeekdayKey[],
    start: number,
    end: number,
    mode: CoverageEditMode,
): DayCoverage[] {
    const selectedDays = new Set(days);
    const editRange = {
        start: Math.max(0, Math.min(24 * 60, start)),
        end: Math.max(0, Math.min(24 * 60, end)),
    };

    return computeWeeklyCoverage(entries).map((row) => {
        if (!selectedDays.has(row.day) || editRange.end <= editRange.start) {
            return {
                day: row.day,
                ranges: row.ranges.map((range) => ({ ...range })),
            };
        }

        return {
            day: row.day,
            ranges:
                mode === "block"
                    ? mergeRanges([...row.ranges, editRange])
                    : subtractRange(row.ranges, editRange),
        };
    });
}

/** Groups identical time ranges across weekdays into compact schedule specs. */
export function coverageEntrySpecs(rows: DayCoverage[]): CoverageEntrySpec[] {
    const grouped = new Map<string, CoverageEntrySpec>();

    for (const day of WEEKDAY_KEYS) {
        const row = rows.find((candidate) => candidate.day === day);
        if (!row) continue;

        for (const range of row.ranges) {
            const key = `${range.start}:${range.end}`;
            const existing = grouped.get(key);
            if (existing) {
                existing.days.push(day);
            } else {
                grouped.set(key, {
                    days: [day],
                    start: range.start,
                    end: range.end,
                });
            }
        }
    }

    return [...grouped.values()].sort(
        (a, b) => a.start - b.start || a.end - b.end,
    );
}

export function totalCoverageMinutes(rows: DayCoverage[]): number {
    return rows.reduce(
        (total, row) =>
            total +
            row.ranges.reduce(
                (dayTotal, range) => dayTotal + (range.end - range.start),
                0,
            ),
        0,
    );
}

/** True when at least one minute of the week is covered. */
export function hasAnyCoverage(rows: DayCoverage[]): boolean {
    return rows.some((row) => row.ranges.length > 0);
}

/**
 * Human-readable summary of a day's coverage, e.g. "Mon 09:00-17:00",
 * "Mon 22:00-02:00" (overnight) or "Mon all day". Used for the accessible
 * text alternative and tooltips so the grid never relies on colour alone.
 * Uses 24-hour times because it is a pure text alternative of the grid.
 */
export function formatDayCoverage(row: DayCoverage): string {
    const day = row.day.charAt(0).toUpperCase() + row.day.slice(1, 3);
    if (row.ranges.length === 0) return "";
    if (
        row.ranges.length === 1 &&
        row.ranges[0].start === 0 &&
        row.ranges[0].end === 24 * 60
    ) {
        return `${day} all day`;
    }
    const parts = row.ranges.map(
        (range) => `${minutesToTime(range.start)}-${minutesToTime(range.end)}`,
    );
    return `${day} ${parts.join(", ")}`;
}

/** Hour (0..23) coverage derived from merged ranges, for the grid cells. */
export function hourBuckets(ranges: MinuteRange[]): boolean[] {
    const hours = Array.from({ length: 24 }, () => false);
    for (const range of ranges) {
        const firstHour = Math.floor(range.start / 60);
        const lastHourExclusive = Math.ceil(range.end / 60);
        for (let hour = firstHour; hour < lastHourExclusive && hour < 24; hour++) {
            hours[hour] = true;
        }
    }
    return hours;
}
