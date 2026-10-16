import type { ScheduleEntrySchema } from "$engine-client";

const DAY_KEYS = [
    "sunday",
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
] as const;

type DayKey = (typeof DAY_KEYS)[number];

export type ScheduleTransition = {
    kind: "start" | "end";
    at: Date;
};

function parseMinute(value: string | null | undefined): number {
    const [hours = "0", minutes = "0"] = (value ?? "00:00").split(":");
    return (Number.parseInt(hours, 10) || 0) * 60 + (Number.parseInt(minutes, 10) || 0);
}

function dayKey(date: Date): DayKey {
    return DAY_KEYS[date.getDay()] ?? "sunday";
}

function previousDayKey(date: Date): DayKey {
    return DAY_KEYS[(date.getDay() + 6) % 7] ?? "saturday";
}

export function isScheduleActiveAt(
    entries: ScheduleEntrySchema[],
    date: Date,
): boolean {
    const minute = date.getHours() * 60 + date.getMinutes();
    const today = dayKey(date);
    const yesterday = previousDayKey(date);

    return entries.some((entry) => {
        const start = parseMinute(entry.startTime);
        const end = parseMinute(entry.endTime);

        if (start === end) {
            return Boolean(entry[today]);
        }

        if (end > start) {
            return Boolean(entry[today]) && minute >= start && minute < end;
        }

        return (
            (Boolean(entry[today]) && minute >= start) ||
            (Boolean(entry[yesterday]) && minute < end)
        );
    });
}

export function findNextScheduleTransition(
    entries: ScheduleEntrySchema[],
    from = new Date(),
    searchDays = 8,
): ScheduleTransition | null {
    if (entries.length === 0) return null;

    let previous = isScheduleActiveAt(entries, from);
    const cursor = new Date(from);
    cursor.setSeconds(0, 0);
    cursor.setMinutes(cursor.getMinutes() + 1);

    const totalMinutes = searchDays * 24 * 60;
    for (let offset = 0; offset < totalMinutes; offset += 1) {
        const active = isScheduleActiveAt(entries, cursor);
        if (active !== previous) {
            return {
                kind: active ? "start" : "end",
                at: new Date(cursor),
            };
        }

        previous = active;
        cursor.setMinutes(cursor.getMinutes() + 1);
    }

    return null;
}
