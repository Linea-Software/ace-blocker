import type { ScheduleEntrySchema } from "$engine-client";
import { computeWeeklyCoverage } from "$lib/utils/weekly-coverage";

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
    for (const entry of newSchedule) {
        const [sh, sm] = entry.startTime.split(":").map(Number);
        const [eh, em] = entry.endTime.split(":").map(Number);
        const start = sh * 60 + sm;
        const end = eh * 60 + em;
        if (!Number.isFinite(start) || !Number.isFinite(end)) return false;
        if (end < start && end !== 0) return false;
    }

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

