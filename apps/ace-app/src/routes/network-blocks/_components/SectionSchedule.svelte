<script lang="ts">
    import { onMount, tick } from "svelte";
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
    let confirmClear = $state(false);
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

    async function openAddEditor() {
        if (hasOpenEditor) return;
        confirmClear = false;
        editorMode = "add";
        editingEntryId = null;
        editorDays = [];
        editorStartTime = "09:00";
        editorEndTime = "17:00";
        hasOpenEditor = true;
        await tick();
        document.querySelector<HTMLButtonElement>("#schedule [data-period-editor] button[aria-pressed]")?.focus();
    }

    async function openEditEditor(entry: ScheduleEntrySchema) {
        if (hasOpenEditor || !canModifyEntry(entry)) return;
        confirmClear = false;
        editorMode = "edit";
        editingEntryId = entry.id;
        editorDays = getEntryDays(entry);
        editorStartTime = entry.startTime.slice(0, 5);
        editorEndTime = entry.endTime.slice(0, 5);
        hasOpenEditor = true;
        await tick();
        document.querySelector<HTMLButtonElement>("#schedule [data-period-editor] button[aria-pressed]")?.focus();
    }

    async function closeEditor() {
        const previousEntryId = editingEntryId;
        editorMode = null;
        editingEntryId = null;
        editorDays = [];
        editorStartTime = "09:00";
        editorEndTime = "17:00";
        hasOpenEditor = false;
        await tick();
        const row = [...document.querySelectorAll<HTMLElement>("#schedule [data-period-row]")]
            .find((element) => element.dataset.periodRow === previousEntryId);
        const target = row?.querySelector<HTMLButtonElement>("button")
            ?? document.querySelector<HTMLButtonElement>("#schedule section button");
        target?.focus();
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
        confirmClear = false;

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
        // Presets are starting points only; never replace populated schedules.
        if (hasOpenEditor || delta.currentScheduleEntries.length > 0) return;

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

    function clearSchedule() {
        if (isLocked || hasOpenEditor || !confirmClear) return;
        for (const entry of [...delta.currentScheduleEntries]) {
            if (isTempEntry(entry)) {
                delta.removeTempScheduleEntry(entry.id as TempId);
            } else {
                delta.removeScheduleEntry(entry.id);
            }
        }
        confirmClear = false;
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

    function formatPeriodTimes(entry: ScheduleEntrySchema): string {
        if (entry.startTime === entry.endTime) return m.schedule_coverage_all_day();
        return `${formatEntryTime(entry.startTime)}–${formatEntryTime(entry.endTime)}`;
    }
</script>

<div id="schedule" class="space-y-6">
    <section class="space-y-3" aria-labelledby="schedule-periods-heading">
        <div class="flex items-start justify-between gap-3">
            <div class="min-w-0">
                <h3 id="schedule-periods-heading" class="font-condensed text-lg font-semibold text-foreground">
                    {m.schedule_exact_windows()}
                </h3>
                <p class="text-muted-foreground mt-1 text-xs leading-relaxed">
                    {m.schedule_periods_description()}
                </p>
            </div>
            {#if !hasOpenEditor}
                <button
                    type="button"
                    class="border-border bg-card text-foreground hover:bg-muted focus-visible:ring-ring h-9 shrink-0 rounded-sm border px-3 text-xs font-semibold transition-colors focus-visible:outline-none focus-visible:ring-2"
                    onclick={openAddEditor}
                >
                    {m.schedule_exact_add()}
                </button>
            {/if}
        </div>

        {#if delta.currentScheduleEntries.length > 0 || editorMode === "add"}
            <div class="border-border bg-card divide-border divide-y overflow-hidden rounded-sm border" data-period-list>
                {#each delta.currentScheduleEntries as entry (entry.id)}
                    {@const days = getEntryDays(entry)}
                    {@const modifiable = canModifyEntry(entry)}
                    <div data-period-row={entry.id}>
                        <div class="flex min-w-0 items-center gap-2 px-3 py-3">
                            <p class="text-foreground min-w-0 flex-1 text-[13px] leading-relaxed">
                                <span class="text-muted-foreground">{formatDays(days)}</span>
                                <span class="text-muted-foreground/70 mx-1">·</span>
                                <span class="font-mono tabular-nums">
                                    {formatPeriodTimes(entry)}
                                </span>
                                {#if entry.startTime !== entry.endTime && entry.endTime < entry.startTime}
                                    <span class="text-muted-foreground ml-1 text-[11px]">
                                        ({m.schedule_period_overnight()})
                                    </span>
                                {/if}
                            </p>
                            <div class="flex shrink-0 items-center gap-1">
                                <button
                                    type="button"
                                    class="text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:ring-ring h-8 rounded-sm px-2 text-xs font-medium focus-visible:outline-none focus-visible:ring-2 disabled:cursor-not-allowed disabled:opacity-40"
                                    disabled={hasOpenEditor || !modifiable}
                                    onclick={() => openEditEditor(entry)}
                                >
                                    {m.schedule_exact_edit()}
                                </button>
                                <button
                                    type="button"
                                    class="text-muted-foreground hover:bg-destructive/10 hover:text-destructive focus-visible:ring-ring h-8 rounded-sm px-2 text-xs font-medium focus-visible:outline-none focus-visible:ring-2 disabled:cursor-not-allowed disabled:opacity-40"
                                    disabled={hasOpenEditor || !modifiable}
                                    onclick={() => handleDeleteEntry(entry)}
                                >
                                    {m.schedule_exact_remove()}
                                </button>
                            </div>
                        </div>

                        {#if editorMode === "edit" && editingEntryId === entry.id}
                            <div class="border-border bg-muted/30 border-t px-3 py-3" data-edit-period-editor>
                                <SchedulePeriodEditor
                                    bind:days={editorDays}
                                    bind:startTime={editorStartTime}
                                    bind:endTime={editorEndTime}
                                    onCancel={closeEditor}
                                    onApply={handlePeriodApply}
                                    isEditing
                                />
                            </div>
                        {/if}
                    </div>
                {/each}

                {#if editorMode === "add"}
                    <div class="bg-muted/30 px-3 py-3" data-add-period-editor>
                        <SchedulePeriodEditor
                            bind:days={editorDays}
                            bind:startTime={editorStartTime}
                            bind:endTime={editorEndTime}
                            onCancel={closeEditor}
                            onApply={handlePeriodApply}
                        />
                    </div>
                {/if}
                {#if !isLocked && delta.currentScheduleEntries.length > 0}
                    <div class="bg-muted/15 flex flex-wrap items-center gap-2 px-3 py-2.5" data-clear-periods>
                        {#if confirmClear}
                            <p class="text-muted-foreground flex-1 text-xs leading-relaxed" role="status">
                                {m.schedule_clear_confirmation()}
                            </p>
                            <button type="button" class="text-muted-foreground hover:text-foreground focus-visible:ring-ring h-8 rounded-sm px-2 text-xs focus-visible:outline-none focus-visible:ring-2" onclick={() => (confirmClear = false)}>
                                {m.cancel()}
                            </button>
                            <button type="button" class="border-destructive/50 text-destructive hover:bg-destructive/10 focus-visible:ring-ring h-8 rounded-sm border px-2 text-xs font-semibold focus-visible:outline-none focus-visible:ring-2" onclick={clearSchedule}>
                                {m.schedule_clear_confirm_action()}
                            </button>
                        {:else}
                            <button type="button" class="text-muted-foreground hover:text-foreground focus-visible:ring-ring h-8 rounded-sm px-1 text-xs focus-visible:outline-none focus-visible:ring-2 disabled:opacity-40" disabled={hasOpenEditor} onclick={() => (confirmClear = true)}>
                                {m.schedule_clear_all()}
                            </button>
                        {/if}
                    </div>
                {/if}
            </div>
        {:else}
            <p class="border-border border-l-2 py-1 pl-3 text-sm leading-relaxed text-foreground" role="status">
                {m.schedule_no_periods_warning()}
            </p>
            <div class="space-y-2">
                <p class="text-muted-foreground text-xs font-medium">{m.schedule_starting_schedules()}</p>
                <div class="preset-grid">
                    <button type="button" class="preset-button" onclick={() => handleCoveragePreset("always")}>
                        <strong>{m.schedule_coverage_preset_always()}</strong>
                        <span>{m.schedule_preset_always_detail()}</span>
                    </button>
                    <button type="button" class="preset-button" onclick={() => handleCoveragePreset("weekdays")}>
                        <strong>{m.schedule_coverage_preset_weekdays()}</strong>
                        <span>{m.schedule_preset_weekdays_detail()}</span>
                    </button>
                    <button type="button" class="preset-button" onclick={() => handleCoveragePreset("work_hours")}>
                        <strong>{m.schedule_coverage_preset_work_hours()}</strong>
                        <span>{m.schedule_preset_work_hours_detail({start:formatEntryTime("09:00"),end:formatEntryTime("17:00")})}</span>
                    </button>
                </div>
            </div>
        {/if}
    </section>

    <section class="border-border space-y-2 border-t pt-4" aria-label={m.schedule_weekly_preview()}>
        <WeeklyCoverage entries={appliedPreviewEntries} {previewEntry} />
    </section>
</div>

<style>
    .preset-grid {
        display: grid;
        grid-template-columns: repeat(3, minmax(0, 1fr));
        gap: 7px;
    }
    .preset-button {
        min-width: 0;
        min-height: 62px;
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        justify-content: center;
        gap: 4px;
        padding: 9px;
        border: 1px solid var(--border);
        border-radius: 4px;
        background: var(--card);
        color: var(--foreground);
        text-align: left;
        cursor: pointer;
    }
    .preset-button strong {
        font-size: 12px;
        font-weight: 600;
    }
    .preset-button span {
        color: var(--muted-foreground);
        font-size: 11px;
        line-height: 1.25;
    }
    .preset-button:hover {
        border-color: var(--input);
        background: var(--muted);
    }
    .preset-button:focus-visible {
        outline: 2px solid var(--ring);
        outline-offset: 2px;
    }
    @container inspector (max-width: 440px) {
        .preset-grid {
            grid-template-columns: 1fr;
        }
        .preset-button {
            min-height: 44px;
        }
    }
</style>
