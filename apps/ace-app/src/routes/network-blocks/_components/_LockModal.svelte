<script lang="ts">
    import { untrack } from "svelte";
    import * as Dialog from "$lib/components/ui/dialog";
    import { Button } from "$lib/components/ui/button";
    import { Label } from "$lib/components/ui/label";
    import * as Popover from "$lib/components/ui/popover";
    import { Calendar } from "$lib/components/ui/calendar";
    import TimePicker from "$lib/components/TimePicker.svelte";
    import {
        CalendarDate,
        DateFormatter,
        getLocalTimeZone,
        parseDateTime,
        today,
    } from "@internationalized/date";
    import {
        CalendarClock,
        CalendarDays,
        Check,
        ChevronRight,
        Clock3,
        Hourglass,
        LockKeyhole,
        ShieldCheck,
        TriangleAlert,
    } from "@lucide/svelte";
    import { toast } from "svelte-sonner";
    import { setLock, type LockState, type NetworkBlockSchema, type ScheduleEntrySchema } from "$engine-client";
    import * as m from "$lib/paraglide/messages.js";
    import { getLocale } from "$lib/paraglide/runtime";
    import {
        buildLockPayload,
        isUnchangedLock,
        parseLockEnd,
        validateLockDraft,
        type LockDraft,
        type LockError,
        type LockKind,
    } from "./lock-form";

    type LockableBlock = NetworkBlockSchema & { schedules?: ScheduleEntrySchema[] };

    let {
        open = $bindable(false),
        block = null,
        onSave,
    }: {
        open: boolean;
        block: LockableBlock | null;
        onSave?: () => Promise<void> | void;
    } = $props();

    const options: LockKind[] = [
        "soft_unlockable",
        "active_during_schedule",
        "active_until_lock_end",
    ];
    let selectedKind = $state<LockKind>("soft_unlockable");
    let original = $state<LockState | null>(null);
    let calendarValue = $state<CalendarDate | undefined>();
    let timeValue = $state("12:00");
    let editingExisting = $state(false);
    let removingScheduledLock = $state(false);
    let confirmationRequested = $state(false);
    let isSubmitting = $state(false);
    let requestError = $state(false);

    const df = $derived(new DateFormatter(getLocale(), { dateStyle: "long" }));
    const timezone = getLocalTimeZone();
    const maxDate = $derived(today(timezone).add({ days: 90 }));
    const configured = $derived(original?.state !== "inactive" && original !== null);
    const editing = $derived(!configured || editingExisting);
    const proposedEnd = $derived.by(() => {
        if (!calendarValue || !/^\d{2}:\d{2}$/.test(timeValue)) return "";
        const [hour, minute] = timeValue.split(":").map(Number);
        if (hour > 23 || minute > 59) return "";
        const p = (n: number) => String(n).padStart(2, "0");
        return `${calendarValue.year}-${p(calendarValue.month)}-${p(calendarValue.day)}T${timeValue}:00`;
    });
    const draft = $derived<LockDraft>({
        enabled: !removingScheduledLock,
        type: selectedKind,
        end: proposedEnd,
        original: original ?? { state: "inactive" },
        scheduleCount: block?.schedules?.length ?? 0,
        isCurrentlyLocked: block?.isLocked ?? false,
        now: new Date(),
    });
    const validationError = $derived(editing ? validateLockDraft(draft) : null);
    const payload = $derived(editing ? buildLockPayload(draft) : null);
    const unchanged = $derived(
        !!original && !!payload && isUnchangedLock(original, payload),
    );
    const canApply = $derived(editing && !validationError && !unchanged && !isSubmitting);

    $effect(() => {
        if (!open || !block) {
            original = null;
            return;
        }
        // Opening another block must never retain the previous block's draft.
        const saved = block.lockState;
        original = { state: saved.state, end: saved.end };
        selectedKind = saved.state === "inactive" ? "soft_unlockable" : saved.state;
        editingExisting = false;
        removingScheduledLock = false;
        confirmationRequested = false;
        requestError = false;
        if (saved.end) {
            try {
                const end = parseDateTime(saved.end.slice(0, 16));
                calendarValue = new CalendarDate(end.year, end.month, end.day);
                timeValue = `${String(end.hour).padStart(2, "0")}:${String(end.minute).padStart(2, "0")}`;
            } catch {
                calendarValue = undefined;
                timeValue = "12:00";
            }
        } else {
            calendarValue = undefined;
            timeValue = "12:00";
        }
    });

    // A changed choice or deadline invalidates the previous confirmation.
    $effect(() => {
        void selectedKind;
        void calendarValue;
        void timeValue;
        void removingScheduledLock;
        untrack(() => {
            confirmationRequested = false;
            requestError = false;
        });
    });

    function optionTitle(kind: LockKind): string {
        if (kind === "soft_unlockable") return m.lock_option_reflection();
        if (kind === "active_during_schedule") return m.lock_option_schedule();
        return m.lock_option_until();
    }

    function optionDescription(kind: LockKind): string {
        if (kind === "soft_unlockable") return m.lock_option_reflection_description();
        if (kind === "active_during_schedule") return m.lock_option_schedule_description();
        return m.lock_option_until_description();
    }

    function errorText(error: LockError): string {
        switch (error) {
            case "schedule_required": return m.lock_error_schedule_required();
            case "date_required": return m.lock_error_date_required();
            case "date_invalid": return m.lock_error_date_invalid();
            case "date_future": return m.lock_error_future();
            case "date_extension": return m.lock_error_extension();
            case "date_limit": return m.lock_error_date_limit();
            case "lock_cannot_be_removed": return m.lock_error_cannot_remove();
        }
    }

    function displayEnd(value: string | null | undefined): string {
        const end = parseLockEnd(value ?? "");
        if (!end) return value ?? "—";
        return end.toLocaleString(getLocale(), {
            dateStyle: "medium",
            timeStyle: "short",
        });
    }

    function choose(kind: LockKind) {
        if (isSubmitting) return;
        selectedKind = kind;
        confirmationRequested = false;
    }

    function startExtension() {
        if (isSubmitting) return;
        editingExisting = true;
        removingScheduledLock = false;
        selectedKind = "active_until_lock_end";
    }

    function startRemoval() {
        if (isSubmitting || block?.isLocked) return;
        editingExisting = true;
        removingScheduledLock = true;
        confirmationRequested = false;
    }

    async function handlePrimary() {
        if (!block || !canApply || !payload || isSubmitting) return;
        const needsConfirmation = removingScheduledLock || selectedKind !== "soft_unlockable";
        if (needsConfirmation && !confirmationRequested) {
            confirmationRequested = true;
            return;
        }

        isSubmitting = true;
        try {
            const { error } = await setLock({
                path: { network_block_id: block.id },
                body: payload,
            });
            if (error) {
                requestError = true;
                toast.error(error.code === "forbidden" || error.code === "locked"
                    ? m.block_locked_tooltip()
                    : m.failed_to_sync());
                return;
            }
            await onSave?.();
            open = false;
        } catch (error) {
            console.error("Failed to update network block lock:", error);
            requestError = true;
            toast.error(m.failed_to_sync());
        } finally {
            isSubmitting = false;
        }
    }
</script>

<Dialog.Root bind:open>
    <Dialog.Content
        class="flex max-h-[90dvh] w-full flex-col gap-0 overflow-hidden rounded-md border-border bg-background p-0 shadow-xl sm:max-w-2xl"
        showCloseButton={!isSubmitting}
    >
        <Dialog.Header class="border-b border-border bg-card px-5 py-5 text-left sm:px-6">
            <div class="mb-2 flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                <ShieldCheck class="size-4" aria-hidden="true" />
                {m.lock_protection_label()}
            </div>
            <Dialog.Title class="font-condensed text-[28px] font-semibold leading-tight">
                {configured ? m.lock_review_title() : m.lock_protect_title()}
            </Dialog.Title>
            <Dialog.Description class="mt-2 max-w-xl text-sm leading-relaxed text-muted-foreground">
                {m.lock_explanation()}
            </Dialog.Description>
            {#if block}
                <p class="mt-3 text-xs font-medium text-foreground">
                    {m.lock_for_block({ name: block.name })}
                </p>
            {/if}
        </Dialog.Header>

        <div class="min-h-0 flex-1 space-y-5 overflow-y-auto px-5 py-5 sm:px-6">
            {#if block && original}
                {#if configured && !editing}
                    <section class="rounded-md border border-border bg-card p-4" aria-label={m.lock_current_state()}>
                        <div class="flex items-start gap-3">
                            <span class="grid size-10 shrink-0 place-items-center rounded-md bg-[var(--ace-signal)]/15 text-[var(--ace-signal-deep)]">
                                <LockKeyhole class="size-5" aria-hidden="true" />
                            </span>
                            <div class="min-w-0 flex-1">
                                <h3 class="font-condensed text-xl font-semibold">{optionTitle(selectedKind)}</h3>
                                <p class="mt-1 text-sm leading-relaxed text-muted-foreground">
                                    {optionDescription(selectedKind)}
                                </p>
                                {#if selectedKind === "active_until_lock_end"}
                                    <p class="mt-3 font-mono text-sm font-semibold">{m.lock_current_end({ date: displayEnd(original.end) })}</p>
                                {:else if selectedKind === "active_during_schedule"}
                                    <p class="mt-3 text-sm font-medium">
                                        {block.isLocked ? m.lock_schedule_active_now() : m.lock_schedule_outside_now()}
                                    </p>
                                {/if}
                            </div>
                        </div>
                    </section>

                    {#if selectedKind === "active_until_lock_end"}
                        <div class="flex flex-wrap items-center justify-between gap-3">
                            <p class="max-w-sm text-xs leading-relaxed text-muted-foreground">{m.lock_extension_only()}</p>
                            <Button variant="outline" onclick={startExtension} disabled={isSubmitting}>
                                <CalendarClock class="mr-2 size-4" aria-hidden="true" />
                                {m.lock_extend_action()}
                            </Button>
                        </div>
                    {:else if selectedKind === "active_during_schedule"}
                        <div class="space-y-3 rounded-md border border-border p-4">
                            <p class="text-sm text-muted-foreground">{m.lock_schedule_review_hint()}</p>
                            {#if !block.isLocked}
                                <Button variant="outline" onclick={startRemoval} disabled={isSubmitting}>
                                    {m.lock_remove_scheduled_action()}
                                </Button>
                            {/if}
                        </div>
                    {/if}
                {:else}
                    {#if !configured}
                        <fieldset class="space-y-2">
                            <legend class="mb-3 font-condensed text-lg font-semibold">{m.lock_choose_mode()}</legend>
                            {#each options as kind (kind)}
                                <button
                                    type="button"
                                    class={`flex w-full items-start gap-3 rounded-md border px-4 py-3.5 text-left transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring ${selectedKind === kind
                                        ? "border-[var(--ace-signal-deep)] bg-[var(--ace-signal)]/8"
                                        : "border-border hover:bg-muted/40"}`}
                                    aria-pressed={selectedKind === kind}
                                    disabled={isSubmitting}
                                    onclick={() => choose(kind)}
                                >
                                    <span class="mt-0.5 grid size-9 shrink-0 place-items-center rounded-md border border-border bg-card">
                                        {#if kind === "soft_unlockable"}
                                            <Hourglass class="size-4" aria-hidden="true" />
                                        {:else if kind === "active_during_schedule"}
                                            <CalendarClock class="size-4" aria-hidden="true" />
                                        {:else}
                                            <LockKeyhole class="size-4" aria-hidden="true" />
                                        {/if}
                                    </span>
                                    <span class="min-w-0 flex-1">
                                        <span class="block text-sm font-semibold">{optionTitle(kind)}</span>
                                        <span class="mt-1 block text-xs leading-relaxed text-muted-foreground">
                                            {optionDescription(kind)}
                                        </span>
                                    </span>
                                    {#if selectedKind === kind}
                                        <Check class="mt-1 size-4 shrink-0 text-[var(--ace-signal-deep)]" aria-hidden="true" />
                                    {/if}
                                </button>
                            {/each}
                        </fieldset>
                    {/if}

                    {#if selectedKind === "active_until_lock_end" && !removingScheduledLock}
                        <div class="rounded-md border border-border bg-card p-4">
                            <Label class="mb-3 block text-sm font-semibold">{m.lock_expiration_datetime()}</Label>
                            {#if configured}
                                <p class="mb-3 text-xs text-muted-foreground">{m.lock_current_end({ date: displayEnd(original.end) })}</p>
                            {/if}
                            <div class="flex flex-wrap gap-2">
                                <Popover.Root>
                                    <Popover.Trigger>
                                        {#snippet child({ props })}
                                            <Button
                                                {...props}
                                                variant="outline"
                                                class="min-w-0 flex-1 justify-start gap-2 font-normal"
                                                aria-label={m.lock_choose_end_date()}
                                                disabled={isSubmitting}
                                            >
                                                <CalendarDays class="size-4 shrink-0" aria-hidden="true" />
                                                {calendarValue
                                                    ? df.format(calendarValue.toDate(timezone))
                                                    : m.pick_date()}
                                            </Button>
                                        {/snippet}
                                    </Popover.Trigger>
                                    <Popover.Content class="w-auto p-0" align="start">
                                        <Calendar
                                            type="single"
                                            bind:value={calendarValue}
                                            locale={getLocale()}
                                            minValue={today(timezone)}
                                            maxValue={maxDate}
                                            initialFocus
                                        />
                                    </Popover.Content>
                                </Popover.Root>
                                <div class="w-32" aria-label={m.lock_choose_end_time()}>
                                    <TimePicker bind:value={timeValue} ariaLabel={m.lock_choose_end_time()} disabled={isSubmitting} />
                                </div>
                            </div>
                            <p class="mt-2 text-xs text-muted-foreground">{m.lock_max_duration_hint()}</p>
                        </div>
                    {/if}
                    {#if validationError}
                        <p class="flex items-start gap-2 text-sm text-destructive" role="alert">
                            <TriangleAlert class="mt-0.5 size-4 shrink-0" aria-hidden="true" />
                            {errorText(validationError)}
                        </p>
                    {/if}
                    <section class="rounded-md border border-border bg-muted/35 p-4" aria-labelledby="lock-preview-title">
                        <h3 id="lock-preview-title" class="font-condensed text-lg font-semibold">{m.lock_preview_title()}</h3>
                        <p class="mt-2 text-sm leading-relaxed text-muted-foreground">
                            {#if removingScheduledLock}
                                {m.lock_preview_remove()}
                            {:else if selectedKind === "soft_unlockable"}
                                {m.lock_preview_reflection()}
                            {:else if selectedKind === "active_during_schedule"}
                                {m.lock_preview_schedule()}
                            {:else}
                                {m.lock_preview_until()}
                                {#if proposedEnd}
                                    <strong class="mt-2 block font-medium text-foreground">
                                        {m.lock_preview_deadline({ date: displayEnd(proposedEnd) })}
                                    </strong>
                                {/if}
                            {/if}
                        </p>
                    </section>

                    {#if confirmationRequested && canApply}
                        <section class="rounded-md border border-[var(--ace-signal-deep)] bg-card p-4" role="group" aria-label={m.lock_confirmation_title()}>
                            <div class="flex items-start gap-2">
                                <TriangleAlert class="mt-0.5 size-5 shrink-0" aria-hidden="true" />
                                <div>
                                    <h3 class="font-semibold">{m.lock_confirmation_title()}</h3>
                                    <p class="mt-1 text-sm leading-relaxed text-muted-foreground">
                                        {removingScheduledLock
                                            ? m.lock_confirmation_remove()
                                            : selectedKind === "active_until_lock_end"
                                                ? m.lock_confirmation_until({ date: displayEnd(proposedEnd) })
                                                : m.lock_confirmation_schedule()}
                                    </p>
                                </div>
                            </div>
                        </section>
                    {/if}
                {/if}
            {/if}
            {#if requestError}
                <p class="text-sm text-destructive" role="alert">{m.lock_save_failed()}</p>
            {/if}
        </div>

        <Dialog.Footer class="flex flex-row flex-wrap items-center justify-end gap-2 border-t border-border bg-card px-5 py-4 sm:px-6">
            <Button variant="ghost" disabled={isSubmitting} onclick={() => (open = false)}>
                {configured && !editing ? m.close() : m.cancel()}
            </Button>
            {#if editing}
                <Button
                    onclick={handlePrimary}
                    disabled={!canApply}
                    class="min-h-10 bg-[var(--ace-signal)] font-semibold text-[var(--ace-ink)] hover:bg-[var(--ace-signal-bright)]"
                >
                    {#if isSubmitting}
                        {m.saving()}
                    {:else if confirmationRequested}
                        {removingScheduledLock
                            ? m.lock_confirm_remove_action()
                            : selectedKind === "active_until_lock_end"
                                ? m.lock_confirm_until_action()
                                : m.lock_confirm_schedule_action()}
                    {:else if removingScheduledLock}
                        {m.lock_remove_scheduled_action()}
                    {:else if selectedKind === "soft_unlockable"}
                        {m.lock_enable_reflection_action()}
                    {:else}
                        {m.lock_continue_action()}
                        <ChevronRight class="ml-1 size-4" aria-hidden="true" />
                    {/if}
                </Button>
            {/if}
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>
