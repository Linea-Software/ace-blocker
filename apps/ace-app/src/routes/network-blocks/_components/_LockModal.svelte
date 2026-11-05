<script lang="ts">
    import { untrack } from "svelte";
    import * as Dialog from "$lib/components/ui/dialog";
    import { Button } from "$lib/components/ui/button";
    import { Label } from "$lib/components/ui/label";
    import { Switch } from "$lib/components/ui/switch";
    import * as Select from "$lib/components/ui/select";
    import TimePicker from "$lib/components/TimePicker.svelte";

    import * as m from "$lib/paraglide/messages.js";
    import { Lock, Calendar as CalendarIcon } from "@lucide/svelte";
    
    import * as Popover from "$lib/components/ui/popover";
    import { Calendar } from "$lib/components/ui/calendar";
    import { cn } from "$lib/utils";
    import { toast } from "svelte-sonner";
    import { setLock, type LockState } from "$engine-client";
    import { buildLockPayload, isUnchangedLock, validateLockDraft } from "./lock-form";
    import { getLocale } from "$lib/paraglide/runtime";
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
        open = $bindable(false),
        block = $bindable<any>(null),
        onSave,
    }: { open: boolean; block: any; onSave?: () => void } = $props();

    let isActive = $state(false);
    let lockType = $state<LockState["state"]>("soft_unlockable");
    let lockEnd = $state("");
    let isSubmitting = $state(false);

    let calendarValue = $state<CalendarDate | undefined>();
    let timeValue = $state("12:00");

    // The original lock state from the block when the modal opened.
    // null before init / after close; an object once the effect runs.
    let originalLockState: LockState | null = $state(null);

    // Date formatter follows the active app locale (Paraglide), falling
    // back to the system default when no locale is resolved yet.
    const df = $derived(
        new DateFormatter(getLocale(), { dateStyle: "long" }),
    );

    // Init: runs whenever the modal opens.
    $effect(() => {
        if (!block || !open) {
            originalLockState = null;
            return;
        }

        const rawLockState: LockState = {
            state: block.lockState.state,
            end: (block.lockState as any).end ?? undefined,
        };

        originalLockState = rawLockState;
        isActive = rawLockState.state !== "inactive";

        if (rawLockState.state !== "inactive") {
            lockType = rawLockState.state;
            const currentLockEnd = (block.lockState as any).end ?? "";
            untrack(() => {
                lockEnd = currentLockEnd;
                if (currentLockEnd) {
                    try {
                        const dt = parseDateTime(currentLockEnd.slice(0, 16));
                        calendarValue = new CalendarDate(
                            dt.year,
                            dt.month,
                            dt.day,
                        );
                        timeValue = `${dt.hour.toString().padStart(2, "0")}:${dt.minute.toString().padStart(2, "0")}`;
                    } catch {
                        // fallback
                    }
                }
            });
        } else {
            lockType = "soft_unlockable";
            lockEnd = "";
            calendarValue = undefined;
            timeValue = "12:00";
        }
    });

    // Calendar/time → lockEnd sync.
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
                    if (newLockEnd !== lockEnd) lockEnd = newLockEnd;
                } catch {
                    // ignore
                }
            });
        }
    });

    const lockTypes = [
        { value: "soft_unlockable", label: m.soft_unlock() },
        {
            value: "active_during_schedule",
            label: m.lock_active_during_schedule(),
        },
        { value: "active_until_lock_end", label: m.lock_until_specific_date() },
    ];

    let selectedTypeLabel = $derived(
        lockTypes.find((t) => t.value === lockType)?.label ?? "",
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
            const currentLockEnd = (block.lockState as any).end;
            if (
                block.lockState.state === "active_until_lock_end" &&
                currentLockEnd
            ) {
                const currentEnd = parseDateTime(currentLockEnd.slice(0, 16));
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
        } catch {
            return false;
        }
    });

    let lockErrorMessage = $derived.by(() => {
        if (!isTimeInvalid) return "";
        if (
            block?.lockState?.state === "active_until_lock_end" &&
            (block.lockState as any).end
        )
            return m.lock_error_extension();
        return m.lock_error_future();
    });

    const isHardLock = $derived(
        lockType === "active_during_schedule" ||
            lockType === "active_until_lock_end",
    );

    const warningStyles = $derived(
        isHardLock
            ? {
                  bg: "bg-destructive/5",
                  border: "border-destructive/30",
                  iconBg: "bg-destructive/8 text-destructive",

                  switchClass: "data-[state=checked]:bg-destructive",



              }
            : {
                  bg: "bg-[color:color-mix(in_srgb,var(--ace-signal)_7%,transparent)]",
                  border: "border-[color:color-mix(in_srgb,var(--ace-signal)_38%,var(--border))]",
                  iconBg: "bg-[color:color-mix(in_srgb,var(--ace-signal)_10%,transparent)] text-[var(--ace-signal-deep)]",

                  switchClass: "data-[state=checked]:bg-[var(--ace-signal)]",



              },
    );

    async function handleSave() {
        if (!block || isSubmitting) return;

        // No RPC unless the selected state is valid. In particular, the
        // switch being off must never send the selected (hidden) lock type.
        const body = buildLockPayload({
            enabled: isActive,
            type: lockType as Exclude<LockState["state"], "inactive">,
            end: lockEnd,
            original: originalLockState ?? { state: "inactive" },
            scheduleCount: block.schedules?.length ?? 0,
            isCurrentlyLocked: !!block.isLocked,
            now: new Date(),
        });
        if (!body) return;

        // No-op if nothing changed
        if (
            originalLockState &&
            isUnchangedLock(originalLockState, body)
        ) {
            // Nothing changed — close without calling the API
            onSave?.();
            open = false;
            return;
        }

        isSubmitting = true;
        try {
            const { error } = await setLock({
                path: { network_block_id: block.id },
                body,
            });
            if (error) {
                // Surface the refusal — a race can make a lock
                // non-weakenable between the modal opening and the save.
                const code =
                    error && typeof error === "object"
                        ? (error as Record<string, unknown>).code
                        : undefined;
                if (code === "forbidden" || code === "locked") {
                    toast.error(m.block_locked_tooltip());
                } else {
                    toast.error(m.failed_to_sync());
                }
                return;
            }
            onSave?.();
            open = false;
        } catch (err) {
            console.error("Failed to save lock:", err);
            toast.error(m.failed_to_sync());
        } finally {
            isSubmitting = false;
        }
    }

    function resetForm() {
        isActive = false;
        lockType = "soft_unlockable";
        lockEnd = "";
        calendarValue = undefined;
        timeValue = "12:00";
        originalLockState = null;
    }
</script>

<Dialog.Root
    bind:open
    onOpenChange={(val) => {
        if (!val) resetForm();
    }}
>
    <Dialog.Content
        class="w-full sm:max-w-lg p-0 overflow-hidden flex flex-col max-h-[90dvh] rounded-md border-border bg-background shadow-none"
    >
        <Dialog.Header class="border-b border-border bg-card p-5 pb-4">
            <Dialog.Title class="flex items-center gap-2 font-condensed text-2xl font-semibold">
                <Lock class="w-5 h-5" />
                {m.lock_block()}
            </Dialog.Title>
            <Dialog.Description class="text-sm text-muted-foreground mt-1">
                {m.lock_block_description()}
            </Dialog.Description>
        </Dialog.Header>

        <div
            class="flex-1 overflow-y-auto px-6 pb-6 space-y-6 scroll-smooth custom-scrollbar"
        >
            {#if block && originalLockState}
                <div
                    class={cn(
                        "border-y border-border bg-background px-5 py-5 space-y-5",
                        warningStyles.border,
                        warningStyles.bg,
                    )}
                >
                    <!-- Active switch + security icon -->
                    <div class="flex items-center justify-between gap-4">
                        <div class="flex items-center gap-4">
                            <div
                                class={cn(
                                    "flex h-10 w-10 shrink-0 items-center justify-center rounded-sm border",
                                    warningStyles.iconBg,
                                    warningStyles.border,
                                )}
                            >
                                <Lock class="h-5 w-5" aria-hidden="true" />
                            </div>
                            <div class="space-y-0.5">
                                <h4 class="font-bold text-base text-foreground">
                                    {m.security_overwrite()}
                                </h4>
                                <p
                                    class="text-xs text-muted-foreground max-w-[240px] leading-relaxed"
                                >
                                    {m.section_lock_warning()}
                                </p>
                            </div>
                        </div>
                        <Switch
                            bind:checked={isActive}
                            disabled={block.lockState.state !== "inactive"}
                            class={warningStyles.switchClass}
                        />
                    </div>

                    {#if isActive}
                        <div
                            class="space-y-4 rounded-md border border-border bg-card p-4"
                        >
                            <!-- Lock mode selector -->
                            <div class="space-y-2">
                                <Label
                                    class="text-xs font-semibold text-muted-foreground"
                                >
                                    {m.lock_mode()}
                                </Label>
                                <Select.Root
                                    type="single"
                                    value={lockType}
                                    onValueChange={(v) =>
                                        (lockType = v as LockState["state"])}
                                    disabled={block.lockState.state !==
                                        "inactive"}
                                >
                                    <Select.Trigger
                                        class="w-full bg-card border-input"
                                    >
                                        {selectedTypeLabel}
                                    </Select.Trigger>
                                    <Select.Content>
                                        {#each lockTypes as type (type.value)}
                                            <Select.Item value={type.value}>
                                                {type.label}
                                            </Select.Item>
                                        {/each}
                                    </Select.Content>
                                </Select.Root>
                            </div>

                            <!-- Date/time picker (only for active_until_lock_end) -->
                            {#if lockType === "active_until_lock_end"}
                                <div class="grid gap-2 pt-2">
                                    <Label
                                        class="text-xs font-semibold text-muted-foreground"
                                    >
                                        {m.lock_expiration_datetime()}
                                    </Label>
                                    <div class="flex gap-2">
                                        <Popover.Root>
                                            <Popover.Trigger
                                                disabled={block.lockState
                                                    .state !== "inactive" &&
                                                    block.lockState.state !==
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
                                                        disabled={block
                                                            .lockState.state !==
                                                            "inactive" &&
                                                            block.lockState
                                                                .state !==
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
                                                    minValue={block.lockState
                                                        .state ===
                                                        "active_until_lock_end" &&
                                                    (block.lockState as any).end
                                                        ? (() => {
                                                              try {
                                                                  const dt =
                                                                      parseDateTime(
                                                                          (
                                                                              block.lockState as any
                                                                          ).end.slice(
                                                                              0,
                                                                              16,
                                                                          ),
                                                                      );
                                                                  return new CalendarDate(
                                                                      dt.year,
                                                                      dt.month,
                                                                      dt.day,
                                                                  );
                                                              } catch {
                                                                  return today(
                                                                      getLocalTimeZone(),
                                                                  );
                                                              }
                                                          })()
                                                        : today(
                                                              getLocalTimeZone(),
                                                          )}
                                                />
                                            </Popover.Content>
                                        </Popover.Root>
                                        <div class="w-32">
                                            <TimePicker
                                                bind:value={timeValue}
                                                disabled={block.lockState
                                                    .state !== "inactive" &&
                                                    block.lockState.state !==
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

                            <!-- Mode description -->
                            {#if lockType === "soft_unlockable"}
                                <div
                                    class="rounded-sm border border-[color:color-mix(in_srgb,var(--ace-signal)_38%,var(--border))] bg-[color:color-mix(in_srgb,var(--ace-signal)_7%,transparent)] p-3 text-xs leading-relaxed text-foreground"
                                >
                                    <p class="mb-1 font-condensed font-semibold">
                                        {m.lock_reflection_pause()}
                                    </p>
                                    <p>
                                        {m.lock_reflection_pause_description()}
                                    </p>
                                </div>
                            {:else if lockType === "active_during_schedule"}
                                <div
                                    class="rounded-sm border border-destructive/30 bg-destructive/5 p-3 text-xs leading-relaxed text-destructive"
                                >
                                    <p class="mb-1 font-condensed font-semibold">
                                        {m.lock_schedule_bound()}
                                    </p>
                                    <p>
                                        {m.lock_schedule_bound_description()}
                                    </p>
                                </div>
                            {:else if lockType === "active_until_lock_end"}
                                <div
                                    class="rounded-sm border border-destructive/30 bg-destructive/5 p-3 text-xs leading-relaxed text-destructive"
                                >
                                    <p class="mb-1 font-condensed font-semibold">
                                        {m.lock_time_bound()}
                                    </p>
                                    <p>
                                        {m.lock_time_bound_description()}
                                    </p>
                                </div>
                            {/if}
                        </div>
                    {/if}
                </div>
            {/if}
        </div>

        <Dialog.Footer
            class="flex w-full items-center justify-end gap-2 border-t border-border bg-card p-4"
        >
            <Button
                variant="ghost"
                onclick={() => (open = false)}
                class="h-10 rounded-sm px-5 font-medium"
            >
                {m.cancel()}
            </Button>
            <Button
                class="h-10 rounded-sm border border-[var(--ace-signal-deep)] bg-[var(--ace-signal)] px-6 font-semibold text-[var(--ace-ink)] hover:bg-[var(--ace-signal-bright)]"
                onclick={handleSave}
                disabled={isSubmitting || isTimeInvalid || (isActive && !!validateLockDraft({
                    enabled: isActive,
                    type: lockType as Exclude<LockState["state"], "inactive">,
                    end: lockEnd,
                    original: originalLockState ?? { state: "inactive" },
                    scheduleCount: block?.schedules?.length ?? 0,
                    isCurrentlyLocked: !!block?.isLocked,
                    now: new Date(),
                }))}
            >
                {#if isSubmitting}
                    {m.saving()}
                {:else}
                    {m.save()}
                {/if}
            </Button>
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>
