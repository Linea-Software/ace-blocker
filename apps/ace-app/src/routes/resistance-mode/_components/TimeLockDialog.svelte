<script lang="ts">
    import * as m from "$lib/paraglide/messages.js";
    import { Button } from "$lib/components/ui/button";
    import * as Dialog from "$lib/components/ui/dialog";
    import * as Popover from "$lib/components/ui/popover";
    import * as AlertDialog from "$lib/components/ui/alert-dialog";
    import { Calendar } from "$lib/components/ui/calendar";
    import { Calendar as CalendarIcon, LockKeyhole } from "@lucide/svelte";
    import TimePicker from "$lib/components/TimePicker.svelte";
    import LockSeal from "$lib/components/ace/domain/LockSeal.svelte";
    import {
        CalendarDate,
        CalendarDateTime,
        DateFormatter,
        getLocalTimeZone,
        today,
        now,
    } from "@internationalized/date";
    import { formattedLockEnd } from "./lock-format";

    let {
        open = $bindable(false),
        isLocked,
        lockEndRaw,
        disabled = false,
        committing = false,
        onCommit,
    }: {
        open: boolean;
        isLocked: boolean;
        lockEndRaw: string | null;
        disabled?: boolean;
        committing?: boolean;
        onCommit: (isoEnd: string) => void;
    } = $props();

    const df = new DateFormatter("en-US", {
        month: "short",
        day: "numeric",
        year: "numeric",
    });

    let calendarValue = $state<CalendarDate | undefined>();
    let timeValue = $state("12:00");
    let confirmOpen = $state(false);

    const expiration = $derived.by(() => {
        if (!calendarValue || !timeValue) return null;
        try {
            const [hour, minute] = timeValue.split(":").map(Number);
            return new CalendarDateTime(
                calendarValue.year,
                calendarValue.month,
                calendarValue.day,
                hour,
                minute,
            );
        } catch {
            return null;
        }
    });

    const maxLockDate = $derived(today(getLocalTimeZone()).add({ days: 90 }));
    const isCommitDisabled = $derived.by(() => {
        if (!expiration) return true;
        try {
            return expiration.compare(now(getLocalTimeZone())) <= 0;
        } catch {
            return true;
        }
    });

    const confirmDatetime = $derived.by(() => {
        if (!expiration) return "";
        return `${df.format(expiration.toDate(getLocalTimeZone()))} at ${timeValue}`;
    });

    function commit() {
        if (isCommitDisabled || !expiration) return;
        const p = (n: number) => n.toString().padStart(2, "0");
        const iso = `${expiration.year}-${p(expiration.month)}-${p(expiration.day)}T${p(expiration.hour)}:${p(expiration.minute)}:00`;
        confirmOpen = false;
        onCommit(iso);
    }

    $effect(() => {
        if (open) {
            calendarValue = undefined;
            timeValue = "12:00";
            confirmOpen = false;
        }
    });
</script>

<Dialog.Root bind:open>
    <Dialog.Content
        class="max-h-[90dvh] overflow-hidden rounded-md border-border bg-background p-0 shadow-none sm:max-w-[min(520px,calc(100vw-2rem))]"
    >
        {#if isLocked}
            <div class="lock-dialog-shell">
                <div class="lock-dialog-heading">
                    <Dialog.Title class="font-condensed text-2xl font-semibold">
                        {m.time_lock_title()}
                    </Dialog.Title>
                </div>
                <div class="lock-dialog-body">
                    <LockSeal
                        title={m.lock_time_bound()}
                        description={m.time_lock_description()}
                        detail={lockEndRaw
                            ? m.lock_expires_on({
                                  datetime: formattedLockEnd(lockEndRaw),
                              })
                            : null}
                    />
                    <p
                        data-testid="rm-lock-expires"
                        class="sr-only"
                    >
                        {m.lock_expires_on({
                            datetime: lockEndRaw
                                ? formattedLockEnd(lockEndRaw)
                                : "",
                        })}
                    </p>
                </div>
                <div class="lock-dialog-footer">
                    <Button variant="outline" onclick={() => (open = false)}>
                        {m.close()}
                    </Button>
                </div>
            </div>
        {:else}
            <div class="lock-dialog-shell">
                <div class="lock-dialog-heading">
                    <Dialog.Title class="font-condensed text-2xl font-semibold">
                        {m.time_lock_title()}
                    </Dialog.Title>
                    <Dialog.Description class="text-sm leading-relaxed text-muted-foreground">
                        {m.time_lock_description()}
                    </Dialog.Description>
                </div>

                <div class="lock-dialog-body">
                    <LockSeal
                        title={m.lock_time_bound()}
                        description={m.lock_one_way_warning()}
                        detail={m.lock_support_note()}
                        emphasis="danger"
                    />

                    <div class="commit-fields">
                        <Popover.Root>
                            <Popover.Trigger>
                                {#snippet child({ props })}
                                    <Button
                                        variant="outline"
                                        {...props}
                                        class="h-10 w-full justify-start rounded-sm text-left font-normal"
                                    >
                                        {calendarValue
                                            ? df.format(
                                                  calendarValue.toDate(
                                                      getLocalTimeZone(),
                                                  ),
                                              )
                                            : m.pick_date()}
                                        <CalendarIcon class="ml-auto h-4 w-4 opacity-50" />
                                    </Button>
                                {/snippet}
                            </Popover.Trigger>
                            <Popover.Content class="w-auto p-0">
                                <Calendar
                                    type="single"
                                    bind:value={calendarValue}
                                    minValue={today(getLocalTimeZone())}
                                    maxValue={maxLockDate}
                                />
                            </Popover.Content>
                        </Popover.Root>
                        <TimePicker bind:value={timeValue} />
                    </div>

                    {#if expiration}
                        <div
                            data-testid="rm-lock-resulting-expiration"
                            class="resulting-expiration"
                        >
                            <LockKeyhole size={14} aria-hidden="true" />
                            <span>{m.lock_expires_on({ datetime: confirmDatetime })}</span>
                        </div>
                    {/if}
                </div>

                <div class="lock-dialog-footer">
                    <Button variant="ghost" onclick={() => (open = false)}>
                        {m.cancel()}
                    </Button>
                    <Button
                        data-testid="rm-lock-commit"
                        class="commit-button"
                        onclick={() => (confirmOpen = true)}
                        disabled={isCommitDisabled || disabled || committing}
                    >
                        {committing ? m.processing() : m.commit_and_lock()}
                    </Button>
                </div>
            </div>
        {/if}
    </Dialog.Content>
</Dialog.Root>

<AlertDialog.Root bind:open={confirmOpen}>
    <AlertDialog.Content class="rounded-md border-border bg-background shadow-none">
        <AlertDialog.Header>
            <AlertDialog.Title class="font-condensed text-2xl font-semibold"
                >{m.lock_confirm_title()}</AlertDialog.Title
            >
            <AlertDialog.Description class="text-sm leading-relaxed"
                >{m.lock_confirm_description({
                    datetime: confirmDatetime,
                })}</AlertDialog.Description
            >
        </AlertDialog.Header>
        <AlertDialog.Footer class="mt-6 gap-3">
            <AlertDialog.Cancel
                class="h-11 px-6 font-semibold border-border/60 hover:bg-muted"
                >{m.cancel()}</AlertDialog.Cancel
            >
            <AlertDialog.Action
                data-testid="rm-lock-confirm"
                onclick={commit}
                class="h-11 rounded-sm border border-[var(--ace-signal-deep)] bg-[var(--ace-signal)] px-7 font-semibold text-[var(--ace-ink)] hover:bg-[var(--ace-signal-bright)]"
                >{m.commit_and_lock()}</AlertDialog.Action
            >
        </AlertDialog.Footer>
    </AlertDialog.Content>
</AlertDialog.Root>

<style>
    .lock-dialog-shell {
        display: grid;
        grid-template-rows: auto minmax(0, 1fr) auto;
        max-height: 90dvh;
    }

    .lock-dialog-heading {
        display: grid;
        gap: 6px;
        padding: 20px 22px 16px;
        border-bottom: 1px solid var(--border);
        background: var(--card);
    }

    .lock-dialog-body {
        display: grid;
        gap: 16px;
        min-height: 0;
        overflow: auto;
        padding: 20px 22px;
    }

    .commit-fields {
        display: grid;
        grid-template-columns: minmax(0, 1fr) 132px;
        gap: 10px;
    }

    .resulting-expiration {
        display: flex;
        align-items: center;
        gap: 8px;
        min-height: 42px;
        padding: 10px 12px;
        border: 1px solid color-mix(in srgb, var(--ace-signal) 42%, var(--border));
        border-left: 4px solid var(--ace-signal);
        border-radius: 4px;
        background: color-mix(in srgb, var(--ace-signal) 8%, var(--card));
        font-family: "IBM Plex Mono", monospace;
        font-size: 11px;
    }

    .lock-dialog-footer {
        display: flex;
        justify-content: flex-end;
        gap: 8px;
        padding: 14px 18px;
        border-top: 1px solid var(--border);
        background: var(--card);
    }

    :global(.commit-button) {
        border: 1px solid var(--ace-signal-deep) !important;
        border-radius: 4px !important;
        background: var(--ace-signal) !important;
        color: var(--ace-ink) !important;
        font-weight: 600 !important;
    }

    :global(.commit-button:hover:not(:disabled)) {
        background: var(--ace-signal-bright) !important;
    }

    @media (max-width: 560px) {
        .commit-fields {
            grid-template-columns: 1fr;
        }
    }
</style>
