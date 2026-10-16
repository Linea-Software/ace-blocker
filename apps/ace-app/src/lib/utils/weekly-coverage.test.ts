import { describe, expect, it } from "vitest";
import {
    computeWeeklyCoverage,
    coverageEntrySpecs,
    editWeeklyCoverage,
    formatDayCoverage,
    hasAnyCoverage,
    hourBuckets,
    mergeRanges,
    subtractRange,
    timeToMinutes,
    totalCoverageMinutes,
} from "./weekly-coverage";
import type { ScheduleEntrySchema } from "$engine-client";

function entry(overrides: Partial<ScheduleEntrySchema>): ScheduleEntrySchema {
    return {
        networkBlockId: "block-1",
        id: crypto.randomUUID(),
        monday: false,
        tuesday: false,
        wednesday: false,
        thursday: false,
        friday: false,
        saturday: false,
        sunday: false,
        startTime: "09:00",
        endTime: "17:00",
        ...overrides,
    };
}

describe("timeToMinutes", () => {
    it("parses HH:MM into minutes since midnight", () => {
        expect(timeToMinutes("00:00")).toBe(0);
        expect(timeToMinutes("09:30")).toBe(570);
        expect(timeToMinutes("23:59")).toBe(1439);
    });

    it("returns NaN for malformed times", () => {
        expect(Number.isNaN(timeToMinutes("banana"))).toBe(true);
        expect(Number.isNaN(timeToMinutes("9"))).toBe(true);
    });
});

describe("mergeRanges", () => {
    it("merges overlapping and adjacent windows into a union", () => {
        expect(
            mergeRanges([
                { start: 9 * 60, end: 12 * 60 },
                { start: 11 * 60, end: 15 * 60 },
                { start: 15 * 60, end: 17 * 60 },
            ]),
        ).toEqual([{ start: 9 * 60, end: 17 * 60 }]);
    });

    it("keeps disjoint windows separate", () => {
        expect(
            mergeRanges([
                { start: 9 * 60, end: 12 * 60 },
                { start: 14 * 60, end: 17 * 60 },
            ]),
        ).toEqual([
            { start: 9 * 60, end: 12 * 60 },
            { start: 14 * 60, end: 17 * 60 },
        ]);
    });
});

describe("computeWeeklyCoverage", () => {
    it("covers only the entry's declared weekdays", () => {
        const rows = computeWeeklyCoverage([
            entry({ monday: true, startTime: "09:00", endTime: "17:00" }),
        ]);
        const monday = rows.find((r) => r.day === "monday")!;
        expect(monday.ranges).toEqual([{ start: 9 * 60, end: 17 * 60 }]);
        expect(rows.find((r) => r.day === "tuesday")!.ranges).toEqual([]);
    });

    it("combines overlapping schedules on the same day", () => {
        const rows = computeWeeklyCoverage([
            entry({ monday: true, startTime: "09:00", endTime: "13:00" }),
            entry({ monday: true, startTime: "11:00", endTime: "18:00" }),
        ]);
        const monday = rows.find((r) => r.day === "monday")!;
        expect(monday.ranges).toEqual([{ start: 9 * 60, end: 18 * 60 }]);
    });

    it("keeps disjoint windows on the same day separate", () => {
        const rows = computeWeeklyCoverage([
            entry({ monday: true, startTime: "08:00", endTime: "10:00" }),
            entry({ monday: true, startTime: "19:00", endTime: "21:00" }),
        ]);
        const monday = rows.find((r) => r.day === "monday")!;
        expect(monday.ranges).toHaveLength(2);
    });

    it("handles overnight windows that wrap past midnight on the same weekday", () => {
        // 22:00 -> 02:00 covers 22:00-24:00 and 00:00-02:00.
        const rows = computeWeeklyCoverage([
            entry({ friday: true, startTime: "22:00", endTime: "02:00" }),
        ]);
        const friday = rows.find((r) => r.day === "friday")!;
        expect(friday.ranges).toEqual([
            { start: 0, end: 2 * 60 },
            { start: 22 * 60, end: 24 * 60 },
        ]);
    });

    it("ignores entries with no active day and malformed times", () => {
        const rows = computeWeeklyCoverage([
            entry({ startTime: "09:00", endTime: "17:00" }), // no day
            entry({ monday: true, startTime: "nope", endTime: "17:00" }),
        ]);
        expect(hasAnyCoverage(rows)).toBe(false);
    });
});

describe("hourBuckets / formatDayCoverage", () => {
    it("maps ranges to hourly grid cells", () => {
        const hours = hourBuckets([{ start: 9 * 60, end: 17 * 60 }]);
        expect(hours.slice(0, 9)).toEqual(Array(9).fill(false));
        expect(hours.slice(9, 17)).toEqual(Array(8).fill(true));
        expect(hours.slice(17)).toEqual(Array(7).fill(false));
    });

    it("formats a readable summary with overnight clarity", () => {
        const row = computeWeeklyCoverage([
            entry({ monday: true, startTime: "22:00", endTime: "02:00" }),
        ])[0];
        expect(formatDayCoverage(row)).toBe("Mon 00:00-02:00, 22:00-24:00");
    });

    it("summarizes all-day coverage", () => {
        const row = computeWeeklyCoverage([
            entry({ sunday: true, startTime: "00:00", endTime: "24:00" }),
        ])[6];
        expect(formatDayCoverage(row)).toBe("Sun all day");
    });

    it("returns empty summary for an uncovered day", () => {
        const row = computeWeeklyCoverage([entry({ monday: true })]).find(
            (r) => r.day === "tuesday",
        )!;
        expect(formatDayCoverage(row)).toBe("");
    });
});

describe("interactive coverage editing", () => {
    it("subtracts a range and preserves both sides", () => {
        expect(
            subtractRange(
                [{ start: 8 * 60, end: 18 * 60 }],
                { start: 12 * 60, end: 14 * 60 },
            ),
        ).toEqual([
            { start: 8 * 60, end: 12 * 60 },
            { start: 14 * 60, end: 18 * 60 },
        ]);
    });

    it("adds a dragged block across multiple days", () => {
        const rows = editWeeklyCoverage(
            [],
            ["monday", "tuesday", "wednesday"],
            9 * 60,
            17 * 60,
            "block",
        );

        expect(rows.slice(0, 3).map((row) => row.ranges)).toEqual([
            [{ start: 9 * 60, end: 17 * 60 }],
            [{ start: 9 * 60, end: 17 * 60 }],
            [{ start: 9 * 60, end: 17 * 60 }],
        ]);
        expect(totalCoverageMinutes(rows)).toBe(3 * 8 * 60);
    });

    it("allows one hour without changing minute precision around it", () => {
        const rows = editWeeklyCoverage(
            [entry({ monday: true, startTime: "09:15", endTime: "17:45" })],
            ["monday"],
            12 * 60,
            13 * 60,
            "allow",
        );

        expect(rows[0].ranges).toEqual([
            { start: 9 * 60 + 15, end: 12 * 60 },
            { start: 13 * 60, end: 17 * 60 + 45 },
        ]);
    });

    it("groups identical ranges across weekdays into one schedule spec", () => {
        const rows = editWeeklyCoverage(
            [],
            ["monday", "tuesday", "wednesday", "thursday", "friday"],
            9 * 60,
            17 * 60,
            "block",
        );

        expect(coverageEntrySpecs(rows)).toEqual([
            {
                days: ["monday", "tuesday", "wednesday", "thursday", "friday"],
                start: 9 * 60,
                end: 17 * 60,
            },
        ]);
    });
});
