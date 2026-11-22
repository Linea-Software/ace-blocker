import { createMemo, createSignal, For, Show, onSettled } from "solid-js";
import * as m from "$lib/paraglide/messages.js";
import type { ScheduleChanges, Weekday } from "$lib/deltas/schedule-delta";
import type { TempId } from "$lib/deltas/common";
import { isSupportedSchedulePeriod } from "$lib/deltas/schedule-coverage";
import type { ScheduleEntrySchema } from "$engine-client";
import { localizationStore } from "$lib/stores/localization";
import SchedulePeriodEditor from "./_SchedulePeriodEditor";
import WeeklyCoverage from "./_WeeklyCoverage";
import {
    computeWeeklyCoverage, coverageEntrySpecs, editWeeklyCoverage,
    subtractRange, WEEKDAY_KEYS, type CoveragePreset,
    type DayCoverage, type WeekdayKey,
} from "$lib/utils/weekly-coverage";

export interface SectionScheduleProps {
    delta: ScheduleChanges;
    isLocked?: boolean;
    hasOpenEditor: boolean;
    onHasOpenEditorChange: (open: boolean) => void;
}

export default function SectionSchedule(props: SectionScheduleProps) {
    const [editorMode, setEditorMode] = createSignal<"add" | "edit" | null>(null);
    const [confirmClear, setConfirmClear] = createSignal(false);
    const [editingEntryId, setEditingEntryId] = createSignal<string | null>(null);
    const [editorDays, setEditorDays] = createSignal<WeekdayKey[]>([]);
    const [editorStartTime, setEditorStartTime] = createSignal("09:00");
    const [editorEndTime, setEditorEndTime] = createSignal("17:00");
    const appliedPreviewEntries = createMemo(() => editingEntryId()
        ? props.delta.currentScheduleEntries.filter(entry => entry.id !== editingEntryId())
        : props.delta.currentScheduleEntries);
    const previewEntry = createMemo<ScheduleEntrySchema | null>(() => {
        if (!props.hasOpenEditor || editorDays().length === 0
            || !isSupportedSchedulePeriod(editorStartTime(), editorEndTime())) return null;
        return {
            networkBlockId: "", id: "__period-preview__", ...weekdayFromDays(editorDays()),
            startTime: editorStartTime(), endTime: editorEndTime(),
        };
    });
    onSettled(() => localizationStore.detectSystemTimeFormat());

    function focusEditor() {
        queueMicrotask(() => document.querySelector<HTMLButtonElement>(
            "#schedule [data-period-editor] button[aria-pressed]")?.focus());
    }
    function openAddEditor() {
        if (props.hasOpenEditor) return;
        setConfirmClear(false);
        setEditorMode("add");
        setEditingEntryId(null);
        setEditorDays([]);
        setEditorStartTime("09:00");
        setEditorEndTime("17:00");
        props.onHasOpenEditorChange(true);
        focusEditor();
    }
    function openEditEditor(entry: ScheduleEntrySchema) {
        if (props.hasOpenEditor || !canModifyEntry(entry)) return;
        setConfirmClear(false);
        setEditorMode("edit");
        setEditingEntryId(entry.id);
        setEditorDays(getEntryDays(entry));
        setEditorStartTime(entry.startTime.slice(0, 5));
        setEditorEndTime(entry.endTime.slice(0, 5));
        props.onHasOpenEditorChange(true);
        focusEditor();
    }
    function closeEditor() {
        const previousEntryId = editingEntryId();
        setEditorMode(null);
        setEditingEntryId(null);
        setEditorDays([]);
        setEditorStartTime("09:00");
        setEditorEndTime("17:00");
        props.onHasOpenEditorChange(false);
        queueMicrotask(() => {
            const row = [...document.querySelectorAll<HTMLElement>("#schedule [data-period-row]")]
                .find(element => element.dataset.periodRow === previousEntryId);
            const target = row?.querySelector<HTMLButtonElement>("button")
                ?? document.querySelector<HTMLButtonElement>("#schedule section button");
            target?.focus();
        });
    }
    function isTempEntry(entry: ScheduleEntrySchema): boolean {
        return props.delta.changes.some(change => change.type === "ADD" && change.scheduleId === entry.id);
    }
    function canModifyEntry(entry: ScheduleEntrySchema): boolean {
        return !props.isLocked || isTempEntry(entry);
    }
    function handleDeleteEntry(entry: ScheduleEntrySchema) {
        if (props.hasOpenEditor || !canModifyEntry(entry)) return;
        setConfirmClear(false);
        if (isTempEntry(entry)) props.delta.removeTempScheduleEntry(entry.id as TempId);
        else props.delta.removeScheduleEntry(entry.id);
    }
    function weekdayFromDays(days: WeekdayKey[]): Weekday {
        const selected = new Set(days);
        return {
            monday: selected.has("monday"), tuesday: selected.has("tuesday"), wednesday: selected.has("wednesday"),
            thursday: selected.has("thursday"), friday: selected.has("friday"), saturday: selected.has("saturday"),
            sunday: selected.has("sunday"),
        };
    }
    function handlePeriodApply() {
        if (editorDays().length === 0 || !isSupportedSchedulePeriod(editorStartTime(), editorEndTime())) return;
        const [sh, sm] = editorStartTime().split(":").map(Number);
        const [eh, em] = editorEndTime().split(":").map(Number);
        const weekday = weekdayFromDays(editorDays());
        const startPlainTime = {hour: sh, minute: sm};
        const endPlainTime = {hour: eh, minute: em};
        if (editorMode() === "edit" && editingEntryId()) {
            const entry = props.delta.currentScheduleEntries.find(candidate => candidate.id === editingEntryId());
            if (!entry || !canModifyEntry(entry)) return;
            if (isTempEntry(entry)) {
                props.delta.removeTempScheduleEntry(entry.id as TempId);
                props.delta.addScheduleEntry(structuredClone(weekday), startPlainTime, endPlainTime);
            } else {
                props.delta.updateScheduleEntry(entry.id, structuredClone(weekday), {
                    startTime: startPlainTime, endTime: endPlainTime,
                });
            }
        } else if (editorMode() === "add" && props.isLocked) {
            const startMinute = sh * 60 + sm;
            const endMinute = eh * 60 + em;
            if (endMinute <= startMinute) {
                props.delta.addScheduleEntry(structuredClone(weekday), startPlainTime, endPlainTime);
            } else {
                addLockedCoverage(editorDays(), startMinute, endMinute);
            }
        } else if (editorMode() === "add") {
            props.delta.addScheduleEntry(structuredClone(weekday), startPlainTime, endPlainTime);
        }
        closeEditor();
    }
    function coverageEquals(a: DayCoverage[], b: DayCoverage[]): boolean {
        return JSON.stringify(a) === JSON.stringify(b);
    }
    function minuteToPlainTime(totalMinutes: number) {
        const normalized = totalMinutes === 24 * 60 ? 0 : totalMinutes;
        return {hour: Math.floor(normalized / 60), minute: normalized % 60};
    }
    function replaceScheduleWithCoverage(nextCoverage: DayCoverage[]) {
        const currentCoverage = computeWeeklyCoverage(props.delta.currentScheduleEntries);
        if (coverageEquals(currentCoverage, nextCoverage)) return;
        const currentEntries = [...props.delta.currentScheduleEntries];
        for (const entry of currentEntries) {
            if (isTempEntry(entry)) props.delta.removeTempScheduleEntry(entry.id as TempId);
            else props.delta.removeScheduleEntry(entry.id);
        }
        for (const spec of coverageEntrySpecs(nextCoverage)) {
            props.delta.addScheduleEntry(weekdayFromDays(spec.days), minuteToPlainTime(spec.start), minuteToPlainTime(spec.end));
        }
    }
    function addLockedCoverage(days: WeekdayKey[], startMinute: number, endMinute: number) {
        const currentCoverage = computeWeeklyCoverage(props.delta.currentScheduleEntries);
        const missingCoverage: DayCoverage[] = WEEKDAY_KEYS.map(day => ({day, ranges: []}));
        for (const day of days) {
            const existing = currentCoverage.find(row => row.day === day);
            if (!existing) continue;
            let missingRanges = [{start: startMinute, end: endMinute}];
            for (const coveredRange of existing.ranges) {
                missingRanges = missingRanges.flatMap(range => subtractRange([range], coveredRange));
            }
            missingCoverage.find(row => row.day === day)!.ranges = missingRanges;
        }
        for (const spec of coverageEntrySpecs(missingCoverage)) {
            props.delta.addScheduleEntry(weekdayFromDays(spec.days), minuteToPlainTime(spec.start), minuteToPlainTime(spec.end));
        }
    }
    function handleCoveragePreset(preset: CoveragePreset) {
        if (props.hasOpenEditor || props.delta.currentScheduleEntries.length > 0) return;
        const days = preset === "always" ? [...WEEKDAY_KEYS] : WEEKDAY_KEYS.slice(0, 5);
        const startMinute = preset === "work_hours" ? 9 * 60 : 0;
        const endMinute = preset === "work_hours" ? 17 * 60 : 24 * 60;
        if (props.isLocked) {
            if (preset !== "clear") addLockedCoverage(days, startMinute, endMinute);
            return;
        }
        if (preset === "clear") replaceScheduleWithCoverage(computeWeeklyCoverage([]));
        else replaceScheduleWithCoverage(editWeeklyCoverage([], days, startMinute, endMinute, "block"));
    }
    function clearSchedule() {
        if (props.isLocked || props.hasOpenEditor || !confirmClear()) return;
        for (const entry of [...props.delta.currentScheduleEntries]) {
            if (isTempEntry(entry)) props.delta.removeTempScheduleEntry(entry.id as TempId);
            else props.delta.removeScheduleEntry(entry.id);
        }
        setConfirmClear(false);
    }
    function getEntryDays(entry: ScheduleEntrySchema): WeekdayKey[] {
        return WEEKDAY_KEYS.filter(day => entry[day]);
    }
    function shortDay(day: WeekdayKey): string {
        const labels = [m.monday_short(),m.tuesday_short(),m.wednesday_short(),m.thursday_short(),m.friday_short(),m.saturday_short(),m.sunday_short()];
        return labels[WEEKDAY_KEYS.indexOf(day)] ?? day;
    }
    function formatDays(days: WeekdayKey[]): string {
        if (days.length === 0) return m.no_active_days();
        const positions = days.map(day => WEEKDAY_KEYS.indexOf(day));
        if (positions.every((position, index) => position === positions[0] + index) && days.length > 1) {
            return `${shortDay(days[0])}–${shortDay(days[days.length - 1])}`;
        }
        return days.map(shortDay).join(", ");
    }
    function formatPeriodTimes(entry: ScheduleEntrySchema): string {
        if (entry.startTime === entry.endTime) return m.schedule_coverage_all_day();
        return `${localizationStore.formatTime(entry.startTime)}–${localizationStore.formatTime(entry.endTime)}`;
    }
    const periodEditor = () => (
        <SchedulePeriodEditor days={editorDays()} onDaysChange={setEditorDays}
            startTime={editorStartTime()} onStartTimeChange={setEditorStartTime}
            endTime={editorEndTime()} onEndTimeChange={setEditorEndTime}
            onCancel={closeEditor} onApply={handlePeriodApply}
            isEditing={editorMode() === "edit"} />
    );

    return (
        <div id="schedule" class="space-y-6">
            <section class="space-y-3" aria-labelledby="schedule-periods-heading">
                <div class="flex items-start justify-between gap-3">
                    <div class="min-w-0">
                        <h3 id="schedule-periods-heading" class="font-condensed text-lg font-semibold text-foreground">
                            {m.schedule_exact_windows()}
                        </h3>
                        <p class="text-muted-foreground mt-1 text-xs leading-relaxed">{m.schedule_periods_description()}</p>
                    </div>
                    <Show when={!props.hasOpenEditor}>
                        <button type="button"
                            class="border-border bg-card text-foreground hover:bg-muted focus-visible:ring-ring h-9 shrink-0 rounded-sm border px-3 text-xs font-semibold transition-colors focus-visible:outline-none focus-visible:ring-2"
                            onClick={openAddEditor}>{m.schedule_exact_add()}</button>
                    </Show>
                </div>
                <Show when={props.delta.currentScheduleEntries.length > 0 || editorMode() === "add"} fallback={
                    <>
                        <p class="border-border border-l-2 py-1 pl-3 text-sm leading-relaxed text-foreground" role="status">
                            {m.schedule_no_periods_warning()}
                        </p>
                        <div class="space-y-2">
                            <p class="text-muted-foreground text-xs font-medium">{m.schedule_starting_schedules()}</p>
                            <div class="preset-grid">
                                <button type="button" class="preset-button" onClick={() => handleCoveragePreset("always")}>
                                    <strong>{m.schedule_coverage_preset_always()}</strong>
                                    <span>{m.schedule_preset_always_detail()}</span>
                                </button>
                                <button type="button" class="preset-button" onClick={() => handleCoveragePreset("weekdays")}>
                                    <strong>{m.schedule_coverage_preset_weekdays()}</strong>
                                    <span>{m.schedule_preset_weekdays_detail()}</span>
                                </button>
                                <button type="button" class="preset-button" onClick={() => handleCoveragePreset("work_hours")}>
                                    <strong>{m.schedule_coverage_preset_work_hours()}</strong>
                                    <span>{m.schedule_preset_work_hours_detail({
                                        start: localizationStore.formatTime("09:00"), end: localizationStore.formatTime("17:00")})}</span>
                                </button>
                            </div>
                        </div>
                    </>
                }>
                    <div class="border-border bg-card divide-border divide-y overflow-hidden rounded-sm border" data-period-list>
                        <For each={props.delta.currentScheduleEntries}>{entry => (
                            <div data-period-row={entry.id}>
                                <div class="flex min-w-0 items-center gap-2 px-3 py-3">
                                    <p class="text-foreground min-w-0 flex-1 text-[13px] leading-relaxed">
                                        <span class="text-muted-foreground">{formatDays(getEntryDays(entry))}</span>
                                        <span class="text-muted-foreground/70 mx-1">·</span>
                                        <span class="font-mono tabular-nums">{formatPeriodTimes(entry)}</span>
                                        <Show when={entry.startTime !== entry.endTime && entry.endTime === "00:00"}>
                                            <span class="text-muted-foreground ml-1 text-[11px]">({m.schedule_period_ends_at_midnight()})</span>
                                        </Show>
                                    </p>
                                    <div class="flex shrink-0 items-center gap-1">
                                        <button type="button"
                                            class="text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:ring-ring h-8 rounded-sm px-2 text-xs font-medium focus-visible:outline-none focus-visible:ring-2 disabled:cursor-not-allowed disabled:opacity-40"
                                            disabled={props.hasOpenEditor || !canModifyEntry(entry)}
                                            onClick={() => openEditEditor(entry)}>{m.schedule_exact_edit()}</button>
                                        <button type="button"
                                            class="text-muted-foreground hover:bg-destructive/10 hover:text-destructive focus-visible:ring-ring h-8 rounded-sm px-2 text-xs font-medium focus-visible:outline-none focus-visible:ring-2 disabled:cursor-not-allowed disabled:opacity-40"
                                            disabled={props.hasOpenEditor || !canModifyEntry(entry)}
                                            onClick={() => handleDeleteEntry(entry)}>{m.schedule_exact_remove()}</button>
                                    </div>
                                </div>
                                <Show when={editorMode() === "edit" && editingEntryId() === entry.id}>
                                    <div class="border-border bg-muted/30 border-t px-3 py-3" data-edit-period-editor>
                                        {periodEditor()}
                                    </div>
                                </Show>
                            </div>
                        )}</For>
                        <Show when={editorMode() === "add"}>
                            <div class="bg-muted/30 px-3 py-3" data-add-period-editor>{periodEditor()}</div>
                        </Show>
                        <Show when={!props.isLocked && props.delta.currentScheduleEntries.length > 0}>
                            <div class="bg-muted/15 flex flex-wrap items-center gap-2 px-3 py-2.5" data-clear-periods>
                                <Show when={confirmClear()} fallback={
                                    <button type="button" class="text-muted-foreground hover:text-foreground focus-visible:ring-ring h-8 rounded-sm px-1 text-xs focus-visible:outline-none focus-visible:ring-2 disabled:opacity-40"
                                        disabled={props.hasOpenEditor} onClick={() => setConfirmClear(true)}>
                                        {m.schedule_clear_all()}
                                    </button>
                                }>
                                    <p class="text-muted-foreground flex-1 text-xs leading-relaxed" role="status">
                                        {m.schedule_clear_confirmation()}
                                    </p>
                                    <button type="button" class="text-muted-foreground hover:text-foreground focus-visible:ring-ring h-8 rounded-sm px-2 text-xs focus-visible:outline-none focus-visible:ring-2"
                                        onClick={() => setConfirmClear(false)}>{m.cancel()}</button>
                                    <button type="button" class="border-destructive/50 text-destructive hover:bg-destructive/10 focus-visible:ring-ring h-8 rounded-sm border px-2 text-xs font-semibold focus-visible:outline-none focus-visible:ring-2"
                                        onClick={clearSchedule}>{m.schedule_clear_confirm_action()}</button>
                                </Show>
                            </div>
                        </Show>
                    </div>
                </Show>
            </section>
            <section class="border-border space-y-2 border-t pt-4" aria-label={m.schedule_weekly_preview()}>
                <WeeklyCoverage entries={appliedPreviewEntries()} previewEntry={previewEntry()} />
            </section>
            <style>{`
                .preset-grid { display:grid; grid-template-columns:repeat(3,minmax(0,1fr)); gap:7px; }
                .preset-button { min-width:0; min-height:62px; display:flex; flex-direction:column; align-items:flex-start; justify-content:center; gap:4px; padding:9px; border:1px solid var(--border); border-radius:4px; background:var(--card); color:var(--foreground); text-align:left; cursor:pointer; }
                .preset-button strong { font-size:12px; font-weight:600; }
                .preset-button span { color:var(--muted-foreground); font-size:11px; line-height:1.25; }
                .preset-button:hover { border-color:var(--input); background:var(--muted); }
                .preset-button:focus-visible { outline:2px solid var(--ring); outline-offset:2px; }
                @container inspector (max-width:440px) { .preset-grid { grid-template-columns:1fr; } .preset-button { min-height:44px; } }
            `}</style>
        </div>
    );
}
