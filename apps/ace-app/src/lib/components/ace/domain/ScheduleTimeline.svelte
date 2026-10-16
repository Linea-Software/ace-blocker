<script lang="ts">
    import type { ScheduleEntrySchema } from "$engine-client";
    import * as m from "$lib/paraglide/messages.js";
    import {
        computeWeeklyCoverage,
        type WeekdayKey,
    } from "$lib/utils/weekly-coverage";

    let {
        entries = [],
        active = true,
    }: {
        entries?: ScheduleEntrySchema[];
        active?: boolean;
    } = $props();

    const dayKeys: WeekdayKey[] = [
        "sunday",
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
    ];

    const days = [
        { key: "monday", label: m.monday_short },
        { key: "tuesday", label: m.tuesday_short },
        { key: "wednesday", label: m.wednesday_short },
        { key: "thursday", label: m.thursday_short },
        { key: "friday", label: m.friday_short },
        { key: "saturday", label: m.saturday_short },
        { key: "sunday", label: m.sunday_short },
    ] as const satisfies ReadonlyArray<{ key: WeekdayKey; label: () => string }>;

    const coverage = $derived(computeWeeklyCoverage(entries));
    const now = new Date();
    const currentDay = dayKeys[now.getDay()] ?? "sunday";
    const currentMinute = now.getHours() * 60 + now.getMinutes();

    const summary = $derived.by(() => {
        const activeDays = coverage.filter((row) => row.ranges.length > 0);
        if (activeDays.length === 0) return m.schedule_coverage_none_summary();
        return activeDays
            .map((row) => {
                const day = days.find((candidate) => candidate.key === row.day);
                return (day?.label() ?? row.day) + ": " + row.ranges.length;
            })
            .join(", ");
    });

    function left(start: number): string {
        return String((start / (24 * 60)) * 100) + "%";
    }

    function width(start: number, end: number): string {
        return String(((end - start) / (24 * 60)) * 100) + "%";
    }
</script>

<div
    class:inactive={!active}
    class="schedule-timeline"
    role="img"
    aria-label={m.schedule_weekly_preview_aria({ summary })}
>
    <div class="scale" aria-hidden="true">
        <span>00</span>
        <span>06</span>
        <span>12</span>
        <span>18</span>
        <span>24</span>
    </div>

    <div class="rows" aria-hidden="true">
        {#each days as day (day.key)}
            {@const row = coverage.find((candidate) => candidate.day === day.key)}
            <div class="day-row">
                <span class="day">{day.label()}</span>
                <span class="track">
                    {#if row}
                        {#each row.ranges as range (range.start)}
                            <span
                                class="window"
                                style:left={left(range.start)}
                                style:width={width(range.start, range.end)}
                            ></span>
                        {/each}
                    {/if}
                    {#if day.key === currentDay}
                        <span
                            class="now-marker"
                            style:left={left(currentMinute)}
                        ></span>
                    {/if}
                </span>
            </div>
        {/each}
    </div>
</div>

<style>
    .schedule-timeline {
        display: grid;
        gap: 5px;
        min-width: 0;
    }

    .scale {
        margin-left: 31px;
        display: flex;
        justify-content: space-between;
        color: var(--muted-foreground);
        font-family: "IBM Plex Mono", monospace;
        font-size: 11px;
        line-height: 1;
    }

    .rows {
        display: grid;
        gap: 5px;
    }

    .day-row {
        min-width: 0;
        display: grid;
        grid-template-columns: 26px minmax(0, 1fr);
        align-items: center;
        gap: 5px;
    }

    .day {
        color: var(--muted-foreground);
        font-family: "IBM Plex Mono", monospace;
        font-size: 11px;
        font-weight: 500;
        line-height: 1;
    }

    .track {
        position: relative;
        height: 8px;
        overflow: hidden;
        border-radius: 2px;
        background: var(--muted);
    }

    .track::before,
    .track::after {
        content: "";
        position: absolute;
        inset-block: 0;
        width: 1px;
        background: color-mix(in srgb, var(--foreground) 12%, transparent);
    }

    .track::before {
        left: 50%;
    }

    .track::after {
        left: 75%;
    }

    .window {
        position: absolute;
        inset-block: 0;
        min-width: 2px;
        background: var(--ace-signal);
    }

    .inactive .window {
        background: var(--ace-ash);
    }

    .now-marker {
        position: absolute;
        z-index: 2;
        inset-block: -2px;
        width: 2px;
        background: var(--foreground);
        opacity: 0.72;
    }
</style>
