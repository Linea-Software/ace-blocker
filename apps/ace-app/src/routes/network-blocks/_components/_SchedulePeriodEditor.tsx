import { createUniqueId, For, Show } from "solid-js";
import * as m from "$lib/paraglide/messages.js";
import { cn } from "$lib/utils";
import type { WeekdayKey } from "$lib/utils/weekly-coverage";
import { isSupportedSchedulePeriod } from "$lib/deltas/schedule-coverage";

const dayKeys: WeekdayKey[] = [
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
];
const dayLabels: { key: WeekdayKey; label: string; short: string }[] = [
    { key: "monday", label: m.monday(), short: m.monday_short() },
    { key: "tuesday", label: m.tuesday(), short: m.tuesday_short() },
    { key: "wednesday", label: m.wednesday(), short: m.wednesday_short() },
    { key: "thursday", label: m.thursday(), short: m.thursday_short() },
    { key: "friday", label: m.friday(), short: m.friday_short() },
    { key: "saturday", label: m.saturday(), short: m.saturday_short() },
    { key: "sunday", label: m.sunday(), short: m.sunday_short() },
];

export interface SchedulePeriodEditorProps {
    days: WeekdayKey[];
    startTime: string;
    endTime: string;
    onDaysChange: (days: WeekdayKey[]) => void;
    onStartTimeChange: (time: string) => void;
    onEndTimeChange: (time: string) => void;
    onCancel: () => void;
    onApply: () => void;
    isEditing?: boolean;
}

export default function SchedulePeriodEditor(props: SchedulePeriodEditorProps) {
    const headingId = createUniqueId();
    const hasDays = () => props.days.length > 0;
    const validTimes = () =>
        /^([01]\d|2[0-3]):[0-5]\d$/.test(props.startTime) &&
        /^([01]\d|2[0-3]):[0-5]\d$/.test(props.endTime);
    const invalidOvernight = () =>
        validTimes() &&
        props.endTime < props.startTime &&
        props.endTime !== "00:00";
    const isFullDay = () =>
        validTimes() &&
        props.startTime === "00:00" &&
        props.endTime === "00:00";
    const endsAtMidnight = () =>
        validTimes() &&
        props.startTime !== "00:00" &&
        props.endTime === "00:00";
    const canApply = () =>
        hasDays() && isSupportedSchedulePeriod(props.startTime, props.endTime);
    const toggleDay = (day: WeekdayKey) =>
        props.onDaysChange(
            dayKeys.filter((key) =>
                key === day
                    ? !props.days.includes(key)
                    : props.days.includes(key),
            ),
        );

    return (
        <div
            class="space-y-3"
            data-period-editor
            role="group"
            aria-labelledby={headingId}
        >
            <h4
                id={headingId}
                class="font-condensed text-[15px] font-semibold text-foreground"
            >
                {props.isEditing
                    ? m.schedule_period_edit_heading()
                    : m.schedule_period_new_heading()}
            </h4>
            <div class="space-y-2">
                <p
                    class="text-muted-foreground text-xs font-medium"
                    id="schedule-days-label"
                >
                    {m.schedule_period_days_label()}
                </p>
                <div
                    class="grid grid-cols-7 gap-1"
                    role="group"
                    aria-labelledby="schedule-days-label"
                >
                    <For each={dayLabels}>
                        {(day) => (
                            <button
                                type="button"
                                aria-pressed={
                                    props.days.includes(day.key)
                                        ? "true"
                                        : "false"
                                }
                                aria-label={day.label}
                                class={cn(
                                    "h-9 min-w-0 rounded-sm border px-1 font-mono text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                                    props.days.includes(day.key)
                                        ? "border-foreground bg-foreground text-background"
                                        : "border-input bg-background text-muted-foreground hover:border-foreground/60 hover:text-foreground",
                                )}
                                onClick={() => toggleDay(day.key)}
                            >
                                {day.short}
                            </button>
                        )}
                    </For>
                </div>
                <Show when={!hasDays()}>
                    <p class="text-muted-foreground text-xs">
                        {m.schedule_period_choose_day()}
                    </p>
                </Show>
            </div>
            <div class="grid grid-cols-2 gap-2.5">
                <label class="space-y-1" for="schedule-period-start">
                    <span class="text-muted-foreground text-xs font-medium">
                        {m.schedule_from_label()}
                    </span>
                    <input
                        id="schedule-period-start"
                        type="time"
                        name="schedule-period-start"
                        autocomplete="off"
                        step="60"
                        value={props.startTime}
                        onInput={(e) =>
                            props.onStartTimeChange(e.currentTarget.value)
                        }
                        class="border-input bg-background text-foreground focus:border-ring focus:ring-ring/35 h-10 w-full rounded-sm border px-3 font-mono text-sm tabular-nums outline-none focus:ring-2"
                    />
                </label>
                <label class="space-y-1" for="schedule-period-end">
                    <span class="text-muted-foreground text-xs font-medium">
                        {m.schedule_to_label()}
                    </span>
                    <input
                        id="schedule-period-end"
                        type="time"
                        name="schedule-period-end"
                        autocomplete="off"
                        step="60"
                        value={props.endTime}
                        onInput={(e) =>
                            props.onEndTimeChange(e.currentTarget.value)
                        }
                        class="border-input bg-background text-foreground focus:border-ring focus:ring-ring/35 h-10 w-full rounded-sm border px-3 font-mono text-sm tabular-nums outline-none focus:ring-2"
                    />
                </label>
            </div>
            <Show
                when={invalidOvernight()}
                fallback={
                    <Show when={isFullDay() || endsAtMidnight()}>
                        <p class="text-muted-foreground text-xs" role="status">
                            {isFullDay()
                                ? m.schedule_period_all_day()
                                : m.schedule_period_ends_at_midnight()}
                        </p>
                    </Show>
                }
            >
                <p class="text-destructive text-xs" role="alert">
                    {m.schedule_period_invalid_overnight()}
                </p>
            </Show>
            <div class="flex items-center justify-end gap-2">
                <button
                    type="button"
                    class="text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:ring-ring h-8 rounded-sm px-3 text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2"
                    onClick={props.onCancel}
                >
                    {m.cancel()}
                </button>
                <button
                    type="button"
                    class="bg-foreground text-background hover:bg-foreground/90 focus-visible:ring-ring h-8 rounded-sm px-3 text-xs font-semibold transition-colors focus-visible:outline-none focus-visible:ring-2 disabled:cursor-not-allowed disabled:opacity-40"
                    disabled={!canApply()}
                    onClick={props.onApply}
                >
                    {props.isEditing
                        ? m.schedule_period_update()
                        : m.schedule_period_add()}
                </button>
            </div>
        </div>
    );
}
