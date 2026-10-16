<script lang="ts">
    import { untrack } from "svelte";
    import { Label } from "$lib/components/ui/label/index.js";
    import { Switch } from "$lib/components/ui/switch/index.js";
    import * as Select from "$lib/components/ui/select/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import TimePicker from "$lib/components/TimePicker.svelte";
    import * as m from "$lib/paraglide/messages.js";
    import type { LockChanges } from "$lib/deltas/lock-delta.svelte.ts";
    import type { NetworkListLockSchema, LockState } from "$engine-client";
    import {
        ShieldAlert,
        Calendar as CalendarIcon,
        Clock,
        Lock,
        TriangleAlert,
    } from "@lucide/svelte";
    import * as Popover from "$lib/components/ui/popover/index.js";
    import { Calendar } from "$lib/components/ui/calendar/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { cn } from "$lib/utils.js";
    import {
        CalendarDate,
        CalendarDateTime,
        DateFormatter,
        getLocalTimeZone,
        parseDateTime,
        today,
        now,
    } from "@internationalized/date";

    let {
        delta,
        existingLock,
        isValid = $bindable(true),
    }: {
        delta: LockChanges;
        existingLock: NetworkListLockSchema;
        isValid?: boolean;
    } = $props();

    let isActive = $state(false);
    let lockType = $state<LockState["state"]>("active_during_schedule");
    let lockEnd = $state("");

    // Date Picker state
    let calendarValue = $state<CalendarDate | undefined>();
    let timeValue = $state("12:00");

    $effect(() => {
        isValid = !isTimeInvalid;
    });

    const df = new DateFormatter("en-US", {
        dateStyle: "long",
    });

    $effect(() => {
        isActive = existingLock.lockState.state !== "inactive";
        if (existingLock.lockState.state !== "inactive") {
            lockType = existingLock.lockState.state;
            const currentLockEnd = (existingLock.lockState as any).end ?? "";

            untrack(() => {
                if (currentLockEnd !== lockEnd) {
                    lockEnd = currentLockEnd;
                    if (currentLockEnd) {
                        try {
                            const dt = parseDateTime(
                                currentLockEnd.slice(0, 16),
                            );
                            calendarValue = new CalendarDate(
                                dt.year,
                                dt.month,
                                dt.day,
                            );
                            timeValue = `${dt.hour.toString().padStart(2, "0")}:${dt.minute.toString().padStart(2, "0")}`;
                        } catch (e) {
                            // Fallback defaults
                        }
                    }
                }
            });
        } else {
            // Reset to defaults for new locks
            lockType = "active_during_schedule";
            lockEnd = "";
            calendarValue = undefined;
            timeValue = "12:00";
        }
    });

    $effect(() => {
        if (calendarValue && timeValue) {
            untrack(() => {
                try {
                    const [h, min] = timeValue.split(":").map(Number);
                    const dt = new CalendarDateTime(
                        calendarValue!.year,
                        calendarValue!.month,
                        calendarValue!.day,
                        isNaN(h) ? 12 : h,
                        isNaN(min) ? 0 : min,
                    );
                    const p = (n: number) => n.toString().padStart(2, "0");
                    const newLockEnd = `${dt.year}-${p(dt.month)}-${p(dt.day)}T${p(dt.hour)}:${p(dt.minute)}:00`;
                    if (newLockEnd !== lockEnd) {
                        lockEnd = newLockEnd;
                    }
                } catch (e) {}
            });
        }
    });

    $effect(() => {
        let newState: any;
        if (!isActive) {
            newState = { state: "inactive" };
        } else if (lockType === "active_until_lock_end") {
            newState = { state: "active_until_lock_end", end: lockEnd };
        } else {
            newState = { state: "active_during_schedule" };
        }
        delta.setLock(newState);
    });

    const lockTypes = [
        {
            value: "active_during_schedule",
            label: m.lock_active_during_schedule(),
        },
        { value: "active_until_lock_end", label: m.lock_until_specific_date() },
    ];

    let selectedTypeLabel = $derived(
        lockTypes.find((t) => t.value === lockType)?.label,
    );

    let isTimeInvalid = $derived.by(() => {
        if (
            !isActive ||
            lockType !== "active_until_lock_end" ||
            !calendarValue ||
            !timeValue
        )
            return false;

        try {
            const [h, min] = timeValue.split(":").map(Number);
            const selectedEnd = new CalendarDateTime(
                calendarValue.year,
                calendarValue.month,
                calendarValue.day,
                h,
                min,
            );

            if (
                existingLock.isActive &&
                existingLock.lockState.state === "active_until_lock_end" &&
                existingLock.lockState.end
            ) {
                const currentEnd = parseDateTime(
                    existingLock.lockState.end.slice(0, 16),
                );
                return selectedEnd.compare(currentEnd) < 0;
            } else {
                const dtNow = now(getLocalTimeZone());
                const currentNowCDT = new CalendarDateTime(
                    dtNow.year,
                    dtNow.month,
                    dtNow.day,
                    dtNow.hour,
                    dtNow.minute,
                );
                return selectedEnd.compare(currentNowCDT) <= 0;
            }
        } catch (e) {
            return false;
        }
    });

    let lockErrorMessage = $derived.by(() => {
        if (!isTimeInvalid) return "";
        if (
            existingLock.isActive &&
            existingLock.lockState.state === "active_until_lock_end"
        ) {
            return m.lock_error_extension();
        }
        return m.lock_error_future();
    });
</script>

<div class="space-y-4" id="lock">
    <div class="space-y-1.5">
        <h3
            class="flex items-center gap-2 font-condensed text-lg font-semibold text-destructive"
        >
            <Lock class="w-5 h-5" />
            {m.lock()}
        </h3>
        <p class="text-sm text-muted-foreground">
            {m.section_lock_description()}
        </p>
    </div>

    <div
        class="space-y-6 rounded-md border border-destructive/30 bg-destructive/5 p-5"
    >
        <div class="flex items-center justify-between gap-4">
            <div class="flex items-center gap-4">
                <div
                    class="flex h-10 w-10 shrink-0 items-center justify-center rounded-sm border border-destructive/30 bg-destructive/8 text-destructive"
                >
                    <ShieldAlert class="w-6 h-6" />
                </div>
                <div class="space-y-0.5">
                    <h4 class="font-bold text-base text-foreground">
                        {m.security_overwrite()}
                    </h4>
                    <p
                        class="text-xs text-muted-foreground max-w-[280px] leading-relaxed"
                    >
                        {m.section_lock_warning()}
                    </p>
                </div>
            </div>
            <Switch
                bind:checked={isActive}
                disabled={existingLock.lockState.state !== "inactive"}
                class="data-[state=checked]:bg-destructive"
            />
        </div>

        {#if isActive}
            <div
                class="space-y-4 rounded-sm border border-border bg-card p-4"
            >
                <div class="space-y-2">
                    <Label
                        class="text-xs font-semibold text-muted-foreground"
                        >{m.lock_mode()}</Label
                    >
                    <Select.Root
                        type="single"
                        value={lockType}
                        onValueChange={(v) =>
                            (lockType = v as LockState["state"])}
                        disabled={existingLock.lockState.state !== "inactive"}
                    >
                        <Select.Trigger class="w-full bg-card border-input">
                            {selectedTypeLabel}
                        </Select.Trigger>
                        <Select.Content>
                            {#each lockTypes as type (type.value)}
                                <Select.Item value={type.value}
                                    >{type.label}</Select.Item
                                >
                            {/each}
                        </Select.Content>
                    </Select.Root>
                </div>

                {#if lockType === "active_until_lock_end"}
                    <div class="grid gap-2 pt-2">
                        <Label
                            class="text-xs font-semibold text-muted-foreground"
                            >{m.lock_expiration_datetime()}</Label
                        >
                        <div class="flex gap-2">
                            <Popover.Root>
                                <Popover.Trigger
                                    disabled={existingLock.lockState.state !==
                                        "inactive" &&
                                        existingLock.lockState.state !==
                                            "active_until_lock_end"}
                                >
                                    {#snippet child({ props })}
                                        <Button
                                            variant="outline"
                                            class={cn(
                                                "flex-1 justify-start text-left font-normal transition-colors",
                                                !calendarValue &&
                                                    "text-muted-foreground",
                                                isTimeInvalid &&
                                                    "border-destructive text-destructive bg-destructive/5 hover:bg-destructive/10 hover:text-destructive",
                                            )}
                                            {...props}
                                            disabled={existingLock.lockState
                                                .state !== "inactive" &&
                                                existingLock.lockState.state !==
                                                    "active_until_lock_end"}
                                        >
                                            <CalendarIcon
                                                class="mr-2 h-4 w-4"
                                            />
                                            {calendarValue
                                                ? df.format(
                                                      calendarValue.toDate(
                                                          getLocalTimeZone(),
                                                      ),
                                                  )
                                                : m.pick_date()}
                                        </Button>
                                    {/snippet}
                                </Popover.Trigger>
                                <Popover.Content class="w-auto p-0">
                                    <Calendar
                                        type="single"
                                        bind:value={calendarValue}
                                        initialFocus
                                        minValue={existingLock.isActive &&
                                        existingLock.lockState.state ===
                                            "active_until_lock_end" &&
                                        existingLock.lockState.end
                                            ? (() => {
                                                  try {
                                                      const dt = parseDateTime(
                                                          existingLock.lockState.end.slice(
                                                              0,
                                                              16,
                                                          ),
                                                      );
                                                      return new CalendarDate(
                                                          dt.year,
                                                          dt.month,
                                                          dt.day,
                                                      );
                                                  } catch (e) {
                                                      return today(
                                                          getLocalTimeZone(),
                                                      );
                                                  }
                                              })()
                                            : today(getLocalTimeZone())}
                                    />
                                </Popover.Content>
                            </Popover.Root>
                            <div class="w-32">
                                <TimePicker
                                    bind:value={timeValue}
                                    disabled={existingLock.lockState.state !==
                                        "inactive" &&
                                        existingLock.lockState.state !==
                                            "active_until_lock_end"}
                                    class={isTimeInvalid
                                        ? "border-destructive text-destructive bg-destructive/5"
                                        : ""}
                                />
                            </div>
                        </div>

                        {#if isTimeInvalid}
                            <p
                                class="text-xs font-medium text-destructive mt-1 animate-in fade-in slide-in-from-top-1"
                            >
                                {lockErrorMessage}
                            </p>
                        {/if}
                    </div>
                {/if}

                <div
                    class="flex gap-2 items-start text-xs text-destructive bg-destructive/5 p-2 rounded border border-destructive/10"
                >
                    <TriangleAlert class="w-3.5 h-3.5 mt-0.5 shrink-0" />
                    <div>
                        <p class="font-semibold">{m.security_warning()}</p>
                        <p>
                            {m.lock_irreversible_warning()}
                        </p>
                    </div>
                </div>
            </div>
        {/if}
    </div>
</div>
