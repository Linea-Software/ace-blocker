<script lang="ts">
    import { onMount } from "svelte";
    import * as m from "$lib/paraglide/messages.js";
    import type {
        ScheduleChanges,
        Weekday,
    } from "$lib/deltas/schedule-delta.svelte.ts";
    import type { TempId } from "$lib/deltas/common";
    import type { ScheduleEntrySchema } from "$engine-client";
    import { localizationStore } from "$lib/stores/localization.svelte";
    import SchedulePeriodEditor from "./_SchedulePeriodEditor.svelte";
    import WeeklyCoverage from "./_WeeklyCoverage.svelte";
    import {
        computeWeeklyCoverage,
        coverageEntrySpecs,
        editWeeklyCoverage,
        subtractRange,
        WEEKDAY_KEYS,
        type CoveragePreset,
        type DayCoverage,
        type WeekdayKey,
    } from "$lib/utils/weekly-coverage";

    let {
        delta,
        isLocked = false,
        hasOpenEditor = $bindable(false),
    }: {
        delta: ScheduleChanges;
        isLocked?: boolean;
        hasOpenEditor?: boolean;
    } = $props();

    let editorMode = $state<"add" | "edit" | null>(null);
    let editingEntryId = $state<string | null>(null);
    let editorDays = $state<WeekdayKey[]>([]);
    let editorStartTime = $state("09:00");
    let editorEndTime = $state("17:00");

    const appliedPreviewEntries = $derived.by(() =>
        editingEntryId
            ? delta.currentScheduleEntries.filter(
                  (entry) => entry.id !== editingEntryId,
              )
            : delta.currentScheduleEntries,
    );

    const previewEntry = $derived.by<ScheduleEntrySchema | null>(() => {
        if (!hasOpenEditor || editorDays.length === 0) return null;
        if (!isValidTime(editorStartTime) || !isValidTime(editorEndTime)) {
            return null;
        }

        const weekday = weekdayFromDays(editorDays);
        return {
            networkBlockId: "",
            id: "__period-preview__",
            ...weekday,
            startTime: editorStartTime,
            endTime: editorEndTime,
        };
    });

    onMount(() => {
        localizationStore.detectSystemTimeFormat();
    });

    function isValidTime(value: string): boolean {
        return /^([01]\d|2[0-3]):[0-5]\d$/.test(value);
    }

    function openAddEditor() {
        if (hasOpenEditor) return;
        editorMode = "add";
        editingEntryId = null;
        editorDays = [];
        editorStartTime = "09:00";
        editorEndTime = "17:00";
        hasOpenEditor = true;
    }

    function openEditEditor(entry: ScheduleEntrySchema) {
        if (hasOpenEditor || !canModifyEntry(entry)) return;
        editorMode = "edit";
        editingEntryId = entry.id;
        editorDays = getEntryDays(entry);
        editorStartTime = entry.startTime.slice(0, 5);
        editorEndTime = entry.endTime.slice(0, 5);
        hasOpenEditor = true;
    }

    function closeEditor() {
        editorMode = null;
        editingEntryId = null;
        editorDays = [];
        editorStartTime = "09:00";
        editorEndTime = "17:00";
        hasOpenEditor = false;
    }

    function isTempEntry(entry: ScheduleEntrySchema): boolean {
        return delta.changes.some(
            (change) =>
                change.type === "ADD" && change.scheduleId === entry.id,
        );
    }

    function canModifyEntry(entry: ScheduleEntrySchema): boolean {
        return !isLocked || isTempEntry(entry);
    }

    function handleDeleteEntry(entry: ScheduleEntrySchema) {
        if (hasOpenEditor || !canModifyEntry(entry)) return;

        if (isTempEntry(entry)) {
            delta.removeTempScheduleEntry(entry.id as TempId);
        } else {
            delta.removeScheduleEntry(entry.id);
        }
    }

    function handlePeriodApply() {
        if (
            editorDays.length === 0 ||
            !isValidTime(editorStartTime) ||
            !isValidTime(editorEndTime)
        ) {
            return;
        }

        const [sh, sm] = editorStartTime.split(":").map(Number);
        const [eh, em] = editorEndTime.split(":").map(Number);
        const weekday = weekdayFromDays(editorDays);
        const startPlainTime = { hour: sh, minute: sm };
        const endPlainTime = { hour: eh, minute: em };

        if (editorMode === "edit" && editingEntryId) {
            const entry = delta.currentScheduleEntries.find(
                (candidate) => candidate.id === editingEntryId,
            );
            if (!entry || !canModifyEntry(entry)) return;

            if (isTempEntry(entry)) {
                delta.removeTempScheduleEntry(entry.id as TempId);
                delta.addScheduleEntry(
                    structuredClone(weekday),
                    startPlainTime,
                    endPlainTime,
                );
            } else {
                delta.updateScheduleEntry(
                    entry.id,
                    structuredClone(weekday),
                    {
                        startTime: startPlainTime,
                        endTime: endPlainTime,
                    },
                );
            }
        } else if (editorMode === "add" && isLocked) {
            const startMinute = sh * 60 + sm;
            const endMinute = eh * 60 + em;

            // Preserve the engine's existing same-day overnight semantics.
            if (endMinute <= startMinute) {
                delta.addScheduleEntry(
                    structuredClone(weekday),
                    startPlainTime,
                    endPlainTime,
                );
            } else {
                addLockedCoverage(editorDays, startMinute, endMinute);
            }
        } else if (editorMode === "add") {
            delta.addScheduleEntry(
                structuredClone(weekday),
                startPlainTime,
                endPlainTime,
            );
        }

        closeEditor();
    }

    function coverageEquals(a: DayCoverage[], b: DayCoverage[]): boolean {
        return JSON.stringify(a) === JSON.stringify(b);
    }

    function minuteToPlainTime(totalMinutes: number) {
        const normalized = totalMinutes === 24 * 60 ? 0 : totalMinutes;
        return {
            hour: Math.floor(normalized / 60),
            minute: normalized % 60,
        };
    }

    function weekdayFromDays(days: WeekdayKey[]): Weekday {
        const selected = new Set(days);
        return {
            monday: selected.has("monday"),
            tuesday: selected.has("tuesday"),
            wednesday: selected.has("wednesday"),
            thursday: selected.has("thursday"),
            friday: selected.has("friday"),
            saturday: selected.has("saturday"),
            sunday: selected.has("sunday"),
        };
    }

    function replaceScheduleWithCoverage(nextCoverage: DayCoverage[]) {
        const currentCoverage = computeWeeklyCoverage(delta.currentScheduleEntries);
        if (coverageEquals(currentCoverage, nextCoverage)) return;

        const currentEntries = [...delta.currentScheduleEntries];
        for (const entry of currentEntries) {
            if (isTempEntry(entry)) {
                delta.removeTempScheduleEntry(entry.id as TempId);
            } else {
                delta.removeScheduleEntry(entry.id);
            }
        }

        for (const spec of coverageEntrySpecs(nextCoverage)) {
            delta.addScheduleEntry(
                weekdayFromDays(spec.days),
                minuteToPlainTime(spec.start),
                minuteToPlainTime(spec.end),
            );
        }
    }

    function addLockedCoverage(
        days: WeekdayKey[],
        startMinute: number,
        endMinute: number,
    ) {
        const currentCoverage = computeWeeklyCoverage(delta.currentScheduleEntries);
        const missingCoverage: DayCoverage[] = WEEKDAY_KEYS.map((day) => ({
            day,
            ranges: [],
        }));

        for (const day of days) {
            const existing = currentCoverage.find((row) => row.day === day);
            if (!existing) continue;

            let missingRanges = [{ start: startMinute, end: endMinute }];
            for (const coveredRange of existing.ranges) {
                missingRanges = missingRanges.flatMap((range) =>
                    subtractRange([range], coveredRange),
                );
            }

            const target = missingCoverage.find((row) => row.day === day)!;
            target.ranges = missingRanges;
        }

        for (const spec of coverageEntrySpecs(missingCoverage)) {
            delta.addScheduleEntry(
                weekdayFromDays(spec.days),
                minuteToPlainTime(spec.start),
                minuteToPlainTime(spec.end),
            );
        }
    }

    function handleCoveragePreset(preset: CoveragePreset) {
        if (hasOpenEditor) return;

        const allDays = [...WEEKDAY_KEYS];
        const weekdays = WEEKDAY_KEYS.slice(0, 5);
        const days = preset === "always" ? allDays : weekdays;
        const startMinute = preset === "work_hours" ? 9 * 60 : 0;
        const endMinute = preset === "work_hours" ? 17 * 60 : 24 * 60;

        if (isLocked) {
            if (preset !== "clear") {
                addLockedCoverage(days, startMinute, endMinute);
            }
            return;
        }

        if (preset === "clear") {
            replaceScheduleWithCoverage(computeWeeklyCoverage([]));
            return;
        }

        replaceScheduleWithCoverage(
            editWeeklyCoverage([], days, startMinute, endMinute, "block"),
        );
    }

    function getEntryDays(entry: ScheduleEntrySchema): WeekdayKey[] {
        return WEEKDAY_KEYS.filter((day) => entry[day]);
    }

    function shortDay(day: WeekdayKey): string {
        if (day === "monday") return m.monday_short();
        if (day === "tuesday") return m.tuesday_short();
        if (day === "wednesday") return m.wednesday_short();
        if (day === "thursday") return m.thursday_short();
        if (day === "friday") return m.friday_short();
        if (day === "saturday") return m.saturday_short();
        return m.sunday_short();
    }

    function formatDays(days: WeekdayKey[]): string {
        if (days.length === 0) return m.no_active_days();

        const positions = days.map((day) => WEEKDAY_KEYS.indexOf(day));
        const contiguous = positions.every(
            (position, index) => position === positions[0] + index,
        );

        if (contiguous && days.length > 1) {
            return `${shortDay(days[0])}–${shortDay(days[days.length - 1])}`;
        }

        return days.map(shortDay).join(", ");
    }

    function formatEntryTime(time: string): string {
        return localizationStore.formatTime(time);
    }
</script>

<div id="schedule" class="space-y-4">
    <div class="bg-background sticky top-0 z-10 pb-2">
        <WeeklyCoverage entries={appliedPreviewEntries} {previewEntry} />
    </div>

    <div class="border-border border-t py-3">
        <div class="flex flex-wrap items-center gap-1.5">
            <button
                type="button"
                class="border-border bg-background text-foreground hover:bg-muted focus-visible:ring-foreground/50 h-8 rounded-sm border px-2.5 text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 disabled:cursor-not-allowed disabled:opacity-40"
                disabled={hasOpenEditor}
                onclick={() => handleCoveragePreset("always")}
            >
                {m.schedule_coverage_preset_always()}
            </button>
            <button
                type="button"
                class="border-border bg-background text-foreground hover:bg-muted focus-visible:ring-foreground/50 h-8 rounded-sm border px-2.5 text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 disabled:cursor-not-allowed disabled:opacity-40"
                disabled={hasOpenEditor}
                onclick={() => handleCoveragePreset("weekdays")}
            >
                {m.schedule_coverage_preset_weekdays()}
            </button>
            <button
                type="button"
                class="border-border bg-background text-foreground hover:bg-muted focus-visible:ring-foreground/50 h-8 rounded-sm border px-2.5 text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 disabled:cursor-not-allowed disabled:opacity-40"
                disabled={hasOpenEditor}
                onclick={() => handleCoveragePreset("work_hours")}
            >
                {m.schedule_coverage_preset_work_hours()}
            </button>
            <button
                type="button"
                class="text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:ring-foreground/50 h-8 rounded-sm px-2.5 text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 disabled:cursor-not-allowed disabled:opacity-40"
                disabled={isLocked || hasOpenEditor}
                onclick={() => handleCoveragePreset("clear")}
            >
                {m.schedule_coverage_preset_clear()}
            </button>
        </div>
    </div>

    <div class="border-border border-t pt-4">
        <div class="flex items-center justify-between gap-3">
            <p class="font-condensed text-sm font-semibold text-foreground">
                {m.schedule_exact_windows()}
            </p>
            <button
                type="button"
                class="border-border bg-background text-foreground hover:bg-muted focus-visible:ring-foreground/50 h-8 shrink-0 rounded-sm border px-2.5 text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 disabled:cursor-not-allowed disabled:opacity-40"
                disabled={hasOpenEditor}
                onclick={openAddEditor}
            >
                {m.schedule_exact_add()}
            </button>
        </div>

        {#if delta.currentScheduleEntries.length > 0 || editorMode === "add"}
            <div class="border-border/70 divide-border/60 mt-2 divide-y border-y">
                {#each delta.currentScheduleEntries as entry (entry.id)}
                    {@const days = getEntryDays(entry)}
                    {@const modifiable = canModifyEntry(entry)}
                    <div data-period-row={entry.id}>
                        <div class="flex min-w-0 items-center gap-2 px-1 py-2">
                            <p
                                class="text-foreground/90 min-w-0 flex-1 text-xs font-medium tabular-nums"
                            >
                                <span class="text-muted-foreground">
                                    {formatDays(days)}
                                </span>
                                <span class="text-muted-foreground/45 mx-1.5">·</span>
                                <span class="font-mono">
                                    {formatEntryTime(entry.startTime)}–{formatEntryTime(
                                        entry.endTime,
                                    )}
                                </span>
                            </p>
                            <div class="flex shrink-0 items-center gap-1">
                                <button
                                    type="button"
                                    class="text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:ring-foreground/50 h-7 rounded-sm px-2 text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 disabled:cursor-not-allowed disabled:opacity-40"
                                    disabled={hasOpenEditor || !modifiable}
                                    onclick={() => openEditEditor(entry)}
                                >
                                    {m.schedule_exact_edit()}
                                </button>
                                <button
                                    type="button"
                                    class="text-muted-foreground hover:bg-destructive/10 hover:text-destructive focus-visible:ring-foreground/50 h-7 rounded-sm px-2 text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 disabled:cursor-not-allowed disabled:opacity-40"
                                    disabled={hasOpenEditor || !modifiable}
                                    onclick={() => handleDeleteEntry(entry)}
                                >
                                    {m.schedule_exact_remove()}
                                </button>
                            </div>
                        </div>

                        {#if editorMode === "edit" && editingEntryId === entry.id}
                            <div class="border-border/60 bg-muted/20 border-t px-1 py-3">
                                <SchedulePeriodEditor
                                    bind:days={editorDays}
                                    bind:startTime={editorStartTime}
                                    bind:endTime={editorEndTime}
                                    onCancel={closeEditor}
                                    onApply={handlePeriodApply}
                                />
                            </div>
                        {/if}
                    </div>
                {/each}

                {#if editorMode === "add"}
                    <div class="bg-muted/20 px-1 py-3" data-add-period-editor>
                        <SchedulePeriodEditor
                            bind:days={editorDays}
                            bind:startTime={editorStartTime}
                            bind:endTime={editorEndTime}
                            onCancel={closeEditor}
                            onApply={handlePeriodApply}
                        />
                    </div>
                {/if}
            </div>
        {:else}
            <p class="text-muted-foreground mt-2 text-xs">
                {m.schedule_coverage_empty()}
            </p>
        {/if}
    </div>
</div>
