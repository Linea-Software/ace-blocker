import type { ScheduleEntrySchema } from "$engine-client";
import { computeWeeklyCoverage } from "$lib/utils/weekly-coverage";

/**
 * Mirror Rust ScheduleEntry::from_raw's accepted minute-level time shape.
 * An end of 00:00 means midnight at the end of the selected weekday; other
 * end-before-start intervals are rejected by the engine.
 */
export function isSupportedSchedulePeriod(start: string, end: string): boolean {
    const format = /^([01]\d|2[0-3]):[0-5]\d$/;
    return format.test(start) && format.test(end)
        && (end >= start || end === "00:00");
}

/** UI preview only; the privileged engine must independently authorize writes.
 * The engine checks the selected weekday against local time-of-day, not the
 * following weekday. End-at-midnight has special end-of-day semantics.
 */
export function isStrictlyMoreRestrictiveThan(
    oldSchedule: ScheduleEntrySchema[],
    newSchedule: ScheduleEntrySchema[],
): boolean {
    // The Rust ScheduleEntry constructor rejects other overnight ranges.
    // Do not advertise a preview as valid for an engine-invalid draft.
    if (!newSchedule.every((entry) =>
        isSupportedSchedulePeriod(entry.startTime, entry.endTime)
    )) return false;

    const previous = computeWeeklyCoverage(oldSchedule);
    const proposed = computeWeeklyCoverage(newSchedule);
    return previous.every((oldDay, index) =>
        oldDay.ranges.every((needed) =>
            proposed[index].ranges.some(
                (candidate) => candidate.start <= needed.start
                    && candidate.end >= needed.end,
            ),
        ),
    );
}

