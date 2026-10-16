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
    }: {
        days?: WeekdayKey[];
        startTime?: string;
        endTime?: string;
        onCancel: () => void;
        onApply: () => void;
    } = $props();

    const hasDays = $derived(days.length > 0);
    const hasTimes = $derived(startTime.length > 0 && endTime.length > 0);
    const canApply = $derived(hasDays && hasTimes);

    function setDays(nextDays: WeekdayKey[]) {
        days = dayKeys.filter((day) => nextDays.includes(day));
    }

    function toggleDay(day: WeekdayKey) {
        setDays(days.includes(day) ? days.filter((item) => item !== day) : [...days, day]);
    }
</script>

<div class="space-y-3" data-period-editor>
    <div class="space-y-2">
        <div class="grid grid-cols-7 gap-1" role="group" aria-label={m.active_days()}>
            {#each dayLabels as day (day.key)}
                <button
                    type="button"
                    aria-pressed={days.includes(day.key)}
                    aria-label={day.label}
                    class={cn(
                        "h-8 min-w-0 rounded-sm border px-1 font-mono text-[11px] font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-foreground/50 focus-visible:ring-offset-1",
                        days.includes(day.key)
                            ? "border-foreground bg-foreground text-background"
                            : "border-border bg-background text-muted-foreground hover:border-foreground/35 hover:text-foreground",
                    )}
                    onclick={() => toggleDay(day.key)}
                >
                    {day.short}
                </button>
            {/each}
        </div>

        <div class="flex flex-wrap items-center gap-x-3 gap-y-1">
            <button
                type="button"
                class="text-muted-foreground hover:text-foreground focus-visible:ring-foreground/50 rounded-sm px-0.5 py-0.5 text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2"
                onclick={() =>
                    setDays([
                        "monday",
                        "tuesday",
                        "wednesday",
                        "thursday",
                        "friday",
                    ])}
            >
                {m.schedule_quick_weekdays()}
            </button>
            <button
                type="button"
                class="text-muted-foreground hover:text-foreground focus-visible:ring-foreground/50 rounded-sm px-0.5 py-0.5 text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2"
                onclick={() => setDays(["saturday", "sunday"])}
            >
                {m.schedule_quick_weekends()}
            </button>
            <button
                type="button"
                class="text-muted-foreground hover:text-foreground focus-visible:ring-foreground/50 rounded-sm px-0.5 py-0.5 text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2"
                onclick={() => setDays(dayKeys)}
            >
                {m.schedule_quick_every_day()}
            </button>
        </div>
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
                class="border-border bg-background text-foreground focus:border-foreground/50 focus:ring-foreground/35 h-9 w-full rounded-sm border px-3 font-mono text-sm tabular-nums outline-none focus:ring-2"
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
                class="border-border bg-background text-foreground focus:border-foreground/50 focus:ring-foreground/35 h-9 w-full rounded-sm border px-3 font-mono text-sm tabular-nums outline-none focus:ring-2"
            />
        </label>
    </div>

    <div class="flex items-center justify-end gap-2">
        <button
            type="button"
            class="text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:ring-foreground/50 h-8 rounded-sm px-3 text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2"
            onclick={onCancel}
        >
            {m.cancel()}
        </button>
        <button
            type="button"
            class="bg-foreground text-background hover:bg-foreground/90 focus-visible:ring-foreground/50 h-8 rounded-sm px-3 text-xs font-semibold transition-colors focus-visible:outline-none focus-visible:ring-2 disabled:cursor-not-allowed disabled:opacity-40"
            disabled={!canApply}
            onclick={onApply}
        >
            {m.schedule_period_apply()}
        </button>
    </div>
</div>
