<script lang="ts">
    import { onMount } from "svelte";
    import * as m from "$lib/paraglide/messages.js";
    import type { ScheduleEntrySchema } from "$engine-client";
    import { localizationStore } from "$lib/stores/localization.svelte";
    import {
        computeWeeklyCoverage,
        totalCoverageMinutes,
        type MinuteRange,
        type WeekdayKey,
    } from "$lib/utils/weekly-coverage";

    interface Props {
        entries: ScheduleEntrySchema[];
        previewEntry?: ScheduleEntrySchema | null;
    }

    let { entries, previewEntry = null }: Props = $props();

    const DAY_ORDER: { key: WeekdayKey; short: string }[] = [
        { key: "monday", short: m.monday_short() },
        { key: "tuesday", short: m.tuesday_short() },
        { key: "wednesday", short: m.wednesday_short() },
        { key: "thursday", short: m.thursday_short() },
        { key: "friday", short: m.friday_short() },
        { key: "saturday", short: m.saturday_short() },
        { key: "sunday", short: m.sunday_short() },
    ];

    let timezone = $state("");

    const appliedCoverage = $derived(computeWeeklyCoverage(entries));
    const editingCoverage = $derived(
        previewEntry ? computeWeeklyCoverage([previewEntry]) : computeWeeklyCoverage([]),
    );
    const combinedCoverage = $derived(
        computeWeeklyCoverage(previewEntry ? [...entries, previewEntry] : entries),
    );
    const totalMinutes = $derived(totalCoverageMinutes(combinedCoverage));

    const coverageSummary = $derived.by(() => {
        const summary = combinedCoverage
            .filter((row) => row.ranges.length > 0)
            .map((row) => `${shortDay(row.day)} ${formatRanges(row.ranges)}`)
            .join(". ");
        return summary || m.schedule_coverage_none_summary();
    });

    onMount(() => {
        localizationStore.detectSystemTimeFormat();
        timezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
    });

    function formatMinute(minutes: number): string {
        if (minutes >= 24 * 60) return m.schedule_coverage_end_marker();
        const hour = Math.floor(minutes / 60);
        const minute = minutes % 60;
        return localizationStore.formatTime(
            `${hour.toString().padStart(2, "0")}:${minute
                .toString()
                .padStart(2, "0")}`,
        );
    }

    function shortDay(day: WeekdayKey): string {
        return DAY_ORDER.find((candidate) => candidate.key === day)?.short ?? day;
    }

    function formatRanges(ranges: MinuteRange[]): string {
        if (
            ranges.length === 1 &&
            ranges[0].start === 0 &&
            ranges[0].end === 24 * 60
        ) {
            return m.schedule_coverage_all_day();
        }

        return ranges
            .map(
                (range) =>
                    `${formatMinute(range.start)}–${formatMinute(range.end)}`,
            )
            .join(", ");
    }

    function formatDuration(minutes: number): string {
        const hours = Math.floor(minutes / 60);
        const remainder = minutes % 60;
        return remainder > 0 ? `${hours}h ${remainder}m` : `${hours}h`;
    }

    function rangeLeft(range: MinuteRange): string {
        return `${(range.start / (24 * 60)) * 100}%`;
    }

    function rangeWidth(range: MinuteRange): string {
        return `${((range.end - range.start) / (24 * 60)) * 100}%`;
    }
</script>

<div class="space-y-1.5">
    <div class="space-y-1.5">
        <div class="flex items-start justify-between gap-3">
            <div class="min-w-0">
                <p class="font-condensed text-sm font-semibold text-foreground">
                    {m.schedule_weekly_preview()}
                </p>
                {#if timezone}
                    <p class="text-muted-foreground/80 mt-0.5 font-mono text-[10px]">
                        {m.schedule_coverage_timezone({ timezone })}
                    </p>
                {/if}
            </div>
            <p class="text-muted-foreground shrink-0 font-mono text-[10px]">
                {m.schedule_coverage_total_exact({
                    duration: formatDuration(totalMinutes),
                })}
            </p>
        </div>

        <div
            class="border-border bg-background border p-1"
            role="img"
            aria-label={m.schedule_weekly_preview_aria({ summary: coverageSummary })}
            data-weekly-preview
        >
            <div
                class="coverage-row mb-0.5 grid grid-cols-[24px_minmax(0,1fr)] items-end gap-1"
            >
                <span></span>
                <div
                    class="text-muted-foreground relative h-2 font-mono text-[7px]"
                >
                    <span class="absolute left-0">{formatMinute(0)}</span>
                    <span
                        class="quarter-label absolute left-1/4 -translate-x-1/2"
                        >{formatMinute(6 * 60)}</span
                    >
                    <span class="absolute left-1/2 -translate-x-1/2"
                        >{formatMinute(12 * 60)}</span
                    >
                    <span
                        class="quarter-label absolute left-3/4 -translate-x-1/2"
                        >{formatMinute(18 * 60)}</span
                    >
                    <span class="absolute right-0">
                        {m.schedule_coverage_end_marker()}
                    </span>
                </div>
            </div>

            <div>
                {#each DAY_ORDER as day (day.key)}
                    {@const appliedRow = appliedCoverage.find(
                        (candidate) => candidate.day === day.key,
                    )!}
                    {@const editingRow = editingCoverage.find(
                        (candidate) => candidate.day === day.key,
                    )!}
                    <div
                        class="coverage-row grid grid-cols-[24px_minmax(0,1fr)] items-center gap-1"
                    >
                        <span
                            class="text-muted-foreground pr-0.5 text-right font-mono text-[7px] font-medium"
                        >
                            {day.short}
                        </span>
                        <div
                            class="border-border/60 bg-muted/65 relative h-2 min-w-0 overflow-hidden border-y"
                            aria-hidden="true"
                            data-preview-track={day.key}
                        >
                            {#each appliedRow.ranges as range (`${range.start}:${range.end}`)}
                                <span
                                    class="absolute inset-y-0 bg-[var(--ace-signal)]"
                                    style:left={rangeLeft(range)}
                                    style:width={rangeWidth(range)}
                                    data-coverage-range="applied"
                                    data-start-minute={range.start}
                                    data-end-minute={range.end}
                                ></span>
                            {/each}

                            {#each editingRow.ranges as range (`${range.start}:${range.end}`)}
                                <span
                                    class="absolute inset-y-0 bg-foreground/15 ring-foreground/60 ring-1 ring-inset"
                                    style:left={rangeLeft(range)}
                                    style:width={rangeWidth(range)}
                                    data-coverage-range="editing"
                                    data-start-minute={range.start}
                                    data-end-minute={range.end}
                                ></span>
                            {/each}

                            <span
                                class="border-border/55 absolute inset-y-0 left-1/4 border-l"
                            ></span>
                            <span
                                class="border-border/55 absolute inset-y-0 left-1/2 border-l"
                            ></span>
                            <span
                                class="border-border/55 absolute inset-y-0 left-3/4 border-l"
                            ></span>
                        </div>
                    </div>
                {/each}
            </div>
        </div>
    </div>

    <p class="sr-only" data-testid="weekly-coverage-summary">
        {coverageSummary}
    </p>
</div>

<style>
    @container inspector (max-width: 420px) {
        .quarter-label {
            display: none;
        }
    }
</style>
