import { createMemo, createSignal, For, Show, onSettled } from "solid-js";
import * as m from "$lib/paraglide/messages.js";
import type { ScheduleEntrySchema } from "$engine-client";
import { localizationStore } from "$lib/stores/localization";
import {
    computeWeeklyCoverage,
    totalCoverageMinutes,
    type MinuteRange,
    type WeekdayKey,
} from "$lib/utils/weekly-coverage";

export interface WeeklyCoverageProps {
    entries: ScheduleEntrySchema[];
    previewEntry?: ScheduleEntrySchema | null;
}

const DAY_ORDER: { key: WeekdayKey; short: string }[] = [
    { key: "monday", short: m.monday_short() },
    { key: "tuesday", short: m.tuesday_short() },
    { key: "wednesday", short: m.wednesday_short() },
    { key: "thursday", short: m.thursday_short() },
    { key: "friday", short: m.friday_short() },
    { key: "saturday", short: m.saturday_short() },
    { key: "sunday", short: m.sunday_short() },
];

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

export default function WeeklyCoverage(props: WeeklyCoverageProps) {
    const [timezone, setTimezone] = createSignal("");
    const appliedCoverage = createMemo(() => computeWeeklyCoverage(props.entries));
    const editingCoverage = createMemo(() =>
        computeWeeklyCoverage(props.previewEntry ? [props.previewEntry] : []),
    );
    const combinedCoverage = createMemo(() =>
        computeWeeklyCoverage(props.previewEntry ? [...props.entries, props.previewEntry] : props.entries),
    );
    const totalMinutes = createMemo(() => totalCoverageMinutes(combinedCoverage()));

    onSettled(() => {
        localizationStore.detectSystemTimeFormat();
        setTimezone(Intl.DateTimeFormat().resolvedOptions().timeZone);
    });

    function formatMinute(minutes: number): string {
        if (minutes >= 24 * 60) return m.schedule_coverage_end_marker();
        const hour = Math.floor(minutes / 60);
        const minute = minutes % 60;
        return localizationStore.formatTime(
            `${hour.toString().padStart(2, "0")}:${minute.toString().padStart(2, "0")}`,
        );
    }

    function shortDay(day: WeekdayKey): string {
        return DAY_ORDER.find((candidate) => candidate.key === day)?.short ?? day;
    }

    function formatRanges(ranges: MinuteRange[]): string {
        if (ranges.length === 1 && ranges[0].start === 0 && ranges[0].end === 24 * 60) {
            return m.schedule_coverage_all_day();
        }
        return ranges.map(range => `${formatMinute(range.start)}–${formatMinute(range.end)}`).join(", ");
    }

    const coverageSummary = createMemo(() =>
        combinedCoverage()
            .filter(row => row.ranges.length > 0)
            .map(row => `${shortDay(row.day)} ${formatRanges(row.ranges)}`)
            .join(". ") || m.schedule_coverage_none_summary(),
    );

    const shortSummary = createMemo(() => {
        const activeDays = combinedCoverage().filter(row => row.ranges.length > 0);
        if (activeDays.length === 0) return m.schedule_coverage_none_summary();
        const sameRange = activeDays.every(row => formatRanges(row.ranges) === formatRanges(activeDays[0].ranges));
        const indices = activeDays.map(row => DAY_ORDER.findIndex(day => day.key === row.day));
        const consecutive = indices.every((index, i) => i === 0 || index === indices[i - 1] + 1);
        if (sameRange && consecutive) {
            const days = activeDays.length === 7 ? m.schedule_quick_every_day()
                : activeDays.length === 5 && indices[0] === 0 ? m.schedule_quick_weekdays()
                : activeDays.length > 1 ? `${shortDay(activeDays[0].day)}–${shortDay(activeDays[activeDays.length - 1].day)}`
                : shortDay(activeDays[0].day);
            return m.schedule_weekly_summary_uniform({ days, range: formatRanges(activeDays[0].ranges) });
        }
        return m.schedule_weekly_summary_coverage({ days: activeDays.length, duration: formatDuration(totalMinutes()) });
    });

    return (
        <div class="space-y-1.5">
            <div class="space-y-1.5">
                <div class="flex items-start justify-between gap-3">
                    <div class="min-w-0">
                        <p class="font-condensed text-sm font-semibold text-foreground">{m.schedule_weekly_preview()}</p>
                        <Show when={timezone()}>
                            <p class="text-muted-foreground mt-0.5 font-mono text-[11px]">
                                {m.schedule_coverage_timezone({ timezone: timezone() })}
                            </p>
                        </Show>
                    </div>
                    <p class="text-muted-foreground shrink-0 font-mono text-[11px]">
                        {m.schedule_coverage_total_exact({ duration: formatDuration(totalMinutes()) })}
                    </p>
                </div>
                <div class="border-border bg-card border p-2" role="img"
                    aria-label={m.schedule_weekly_preview_aria({ summary: coverageSummary() })}
                    data-weekly-preview>
                    <div class="coverage-row mb-1 grid grid-cols-[34px_minmax(0,1fr)] items-end gap-1.5">
                        <span />
                        <div class="text-muted-foreground relative h-3 font-mono text-[10px]">
                            <span class="absolute left-0">{formatMinute(0)}</span>
                            <span class="quarter-label absolute left-1/4 -translate-x-1/2">{formatMinute(360)}</span>
                            <span class="absolute left-1/2 -translate-x-1/2">{formatMinute(720)}</span>
                            <span class="quarter-label absolute left-3/4 -translate-x-1/2">{formatMinute(1080)}</span>
                            <span class="absolute right-0">{m.schedule_coverage_end_marker()}</span>
                        </div>
                    </div>
                    <div>
                        <For each={DAY_ORDER}>{day => {
                            const appliedRow = createMemo(() => appliedCoverage().find(candidate => candidate.day === day.key));
                            const editingRow = createMemo(() => editingCoverage().find(candidate => candidate.day === day.key));
                            return (
                                <div class="coverage-row mb-1 grid grid-cols-[34px_minmax(0,1fr)] items-center gap-1.5">
                                    <span class="text-muted-foreground pr-0.5 text-right font-mono text-[11px] font-medium">{day.short}</span>
                                    <div class="bg-muted relative h-3 min-w-0 overflow-hidden rounded-[2px]" aria-hidden="true"
                                        data-preview-track={day.key}>
                                        <For each={appliedRow()?.ranges ?? []}>{range =>
                                            <span class="absolute inset-y-0 bg-[var(--ace-signal)]"
                                                style={{ left: rangeLeft(range), width: rangeWidth(range) }}
                                                data-coverage-range="applied" data-start-minute={range.start} data-end-minute={range.end} />
                                        }</For>
                                        <For each={editingRow()?.ranges ?? []}>{range =>
                                            <span class="absolute inset-y-0 bg-foreground/15 ring-foreground/60 ring-1 ring-inset"
                                                style={{ left: rangeLeft(range), width: rangeWidth(range) }}
                                                data-coverage-range="editing" data-start-minute={range.start} data-end-minute={range.end} />
                                        }</For>
                                        <span class="border-border/55 absolute inset-y-0 left-1/4 border-l" />
                                        <span class="border-border/55 absolute inset-y-0 left-1/2 border-l" />
                                        <span class="border-border/55 absolute inset-y-0 left-3/4 border-l" />
                                    </div>
                                </div>
                            );
                        }}</For>
                    </div>
                </div>
            </div>
            <p class="text-muted-foreground text-xs leading-relaxed" data-testid="weekly-human-summary">{shortSummary()}</p>
            <p class="sr-only" data-testid="weekly-coverage-summary">{coverageSummary()}</p>
            <style>{`@container inspector (max-width: 420px) { .quarter-label { display: none; } }`}</style>
        </div>
    );
}
