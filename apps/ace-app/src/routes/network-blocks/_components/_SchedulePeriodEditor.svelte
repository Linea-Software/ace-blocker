<script lang="ts">
    import * as m from "$lib/paraglide/messages.js";
    import { cn } from "$lib/utils";
    import type { WeekdayKey } from "$lib/utils/weekly-coverage";

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

    let {
        days = $bindable<WeekdayKey[]>([]),
        startTime = $bindable("09:00"),
        endTime = $bindable("17:00"),
        onCancel,
        onApply,
        isEditing = false,
    }: {
        days?: WeekdayKey[];
        startTime?: string;
        endTime?: string;
        onCancel: () => void;
        onApply: () => void;
        isEditing?: boolean;
    } = $props();

    const editorHeadingId = $props.id();
    const hasDays = $derived(days.length > 0);
    const hasTimes = $derived(/^([01]\d|2[0-3]):[0-5]\d$/.test(startTime) && /^([01]\d|2[0-3]):[0-5]\d$/.test(endTime));
    const crossesMidnight = $derived(hasTimes && endTime < startTime);
    const canApply = $derived(hasDays && hasTimes);

    function setDays(nextDays: WeekdayKey[]) {
        days = dayKeys.filter((day) => nextDays.includes(day));
    }

    function toggleDay(day: WeekdayKey) {
        setDays(days.includes(day) ? days.filter((item) => item !== day) : [...days, day]);
    }
</script>

<div class="space-y-3" data-period-editor role="group" aria-labelledby={editorHeadingId}>
    <h4 id={editorHeadingId} class="font-condensed text-[15px] font-semibold text-foreground">
        {isEditing ? m.schedule_period_edit_heading() : m.schedule_period_new_heading()}
    </h4>
    <div class="space-y-2">
        <p class="text-muted-foreground text-xs font-medium" id="schedule-days-label">
            {m.schedule_period_days_label()}
        </p>
        <div class="grid grid-cols-7 gap-1" role="group" aria-labelledby="schedule-days-label">
            {#each dayLabels as day (day.key)}
                <button
                    type="button"
                    aria-pressed={days.includes(day.key)}
                    aria-label={day.label}
                    class={cn(
                        "h-9 min-w-0 rounded-sm border px-1 font-mono text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                        days.includes(day.key)
                            ? "border-foreground bg-foreground text-background"
                            : "border-input bg-background text-muted-foreground hover:border-foreground/60 hover:text-foreground",
                    )}
                    onclick={() => toggleDay(day.key)}
                >
                    {day.short}
                </button>
            {/each}
        </div>
        {#if !hasDays}
            <p class="text-muted-foreground text-xs">{m.schedule_period_choose_day()}</p>
        {/if}
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
                bind:value={startTime}
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
                bind:value={endTime}
                class="border-input bg-background text-foreground focus:border-ring focus:ring-ring/35 h-10 w-full rounded-sm border px-3 font-mono text-sm tabular-nums outline-none focus:ring-2"
            />
        </label>
    </div>

    {#if hasTimes && (crossesMidnight || startTime === endTime)}
        <p class="text-muted-foreground text-xs" role="status">
            {crossesMidnight ? m.schedule_period_overnight() : m.schedule_period_all_day()}
        </p>
    {/if}

    <div class="flex items-center justify-end gap-2">
        <button
            type="button"
            class="text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:ring-ring h-8 rounded-sm px-3 text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2"
            onclick={onCancel}
        >
            {m.cancel()}
        </button>
        <button
            type="button"
            class="bg-foreground text-background hover:bg-foreground/90 focus-visible:ring-ring h-8 rounded-sm px-3 text-xs font-semibold transition-colors focus-visible:outline-none focus-visible:ring-2 disabled:cursor-not-allowed disabled:opacity-40"
            disabled={!canApply}
            onclick={onApply}
        >
            {isEditing ? m.schedule_period_update() : m.schedule_period_add()}
        </button>
    </div>
</div>

