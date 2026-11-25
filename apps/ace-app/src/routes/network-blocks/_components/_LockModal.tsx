import { createMemo, createSignal, createEffect, For, Show } from "solid-js";
import { Button } from "$lib/components/ui/button/solid";
import { Label } from "$lib/components/ui/label/solid";
import { toast } from "solid-sonner";
import {
    CalendarClock,
    CalendarDays,
    Check,
    ChevronRight,
    Hourglass,
    LockKeyhole,
    ShieldCheck,
    TriangleAlert,
    X,
} from "@lucide/solid";
import {
    setLock,
    type LockState,
    type NetworkBlockSchema,
    type ScheduleEntrySchema,
} from "$engine-client";
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

export interface LockModalProps {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    block: LockableBlock | null;
    onSave?: () => Promise<void> | void;
    onEditSchedule?: (block: LockableBlock) => void;
}

const OPTIONS: LockKind[] = [
    "soft_unlockable",
    "active_during_schedule",
    "active_until_lock_end",
];
const pad = (n: number) => String(n).padStart(2, "0");
const todayLocal = () => {
    const date = new Date();
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
};
const maxLocal = () => {
    const date = new Date();
    date.setDate(date.getDate() + 90);
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
};

export default function LockModal(props: LockModalProps) {
    const [selectedKind, setSelectedKind] =
        createSignal<LockKind>("soft_unlockable");
    const [original, setOriginal] = createSignal<LockState | null>(null);
    const [selectedDate, setSelectedDate] = createSignal("");
    const [timeValue, setTimeValue] = createSignal("12:00");
    const [editingExisting, setEditingExisting] = createSignal(false);
    const [removingLock, setRemovingLock] = createSignal(false);
    const [confirmationRequested, setConfirmationRequested] =
        createSignal(false);
    const [isSubmitting, setIsSubmitting] = createSignal(false);
    const [requestError, setRequestError] = createSignal(false);
    const configured = createMemo(
        () => original() !== null && original()!.state !== "inactive",
    );
    const editing = createMemo(() => !configured() || editingExisting());
    const proposedEnd = createMemo(() => {
        if (
            !/^\d{4}-\d{2}-\d{2}$/.test(selectedDate()) ||
            !/^\d{2}:\d{2}$/.test(timeValue())
        )
            return "";
        const [hour, minute] = timeValue().split(":").map(Number);
        if (hour > 23 || minute > 59) return "";
        return `${selectedDate()}T${timeValue()}:00`;
    });
    const draft = createMemo<LockDraft>(() => ({
        enabled: !removingLock(),
        type: selectedKind(),
        end: proposedEnd(),
        original: original() ?? { state: "inactive" },
        scheduleCount: props.block?.schedules?.length ?? 0,
        isCurrentlyLocked: props.block?.isLocked ?? false,
        now: new Date(),
    }));
    const validationError = createMemo(() =>
        editing() ? validateLockDraft(draft()) : null,
    );
    const payload = createMemo(() =>
        editing() ? buildLockPayload(draft()) : null,
    );
    const unchanged = createMemo(
        () =>
            !!original() &&
            !!payload() &&
            isUnchangedLock(original()!, payload()!),
    );
    const canApply = createMemo(
        () =>
            editing() && !validationError() && !unchanged() && !isSubmitting(),
    );

    // Opening a different block always resets the lock draft and confirmation.
    createEffect(
        () => [props.open, props.block?.id] as const,
        ([open]) => {
            if (!open || !props.block) {
                setOriginal(null);
                return;
            }
            const saved = props.block.lockState;
            setOriginal({ state: saved.state, end: saved.end });
            setSelectedKind(
                saved.state === "inactive" ? "soft_unlockable" : saved.state,
            );
            setEditingExisting(false);
            setRemovingLock(false);
            setConfirmationRequested(false);
            setRequestError(false);
            if (saved.end) {
                const end = parseLockEnd(saved.end);
                if (end) {
                    setSelectedDate(
                        `${end.getFullYear()}-${pad(end.getMonth() + 1)}-${pad(end.getDate())}`,
                    );
                    setTimeValue(
                        `${pad(end.getHours())}:${pad(end.getMinutes())}`,
                    );
                } else {
                    setSelectedDate("");
                    setTimeValue("12:00");
                }
            } else {
                setSelectedDate("");
                setTimeValue("12:00");
            }
        },
    );

    function updateKind(kind: LockKind) {
        if (isSubmitting()) return;
        setSelectedKind(kind);
        setConfirmationRequested(false);
        setRequestError(false);
    }
    function updateDate(date: string) {
        setSelectedDate(date);
        setConfirmationRequested(false);
        setRequestError(false);
    }
    function updateTime(time: string) {
        setTimeValue(time);
        setConfirmationRequested(false);
        setRequestError(false);
    }
    function updateRemoving(remove: boolean) {
        setRemovingLock(remove);
        setConfirmationRequested(false);
        setRequestError(false);
    }
    function optionTitle(kind: LockKind): string {
        if (kind === "soft_unlockable") return m.lock_option_reflection();
        if (kind === "active_during_schedule") return m.lock_option_schedule();
        return m.lock_option_until();
    }
    function optionDescription(kind: LockKind): string {
        if (kind === "soft_unlockable")
            return m.lock_option_reflection_description();
        if (kind === "active_during_schedule")
            return m.lock_option_schedule_description();
        return m.lock_option_until_description();
    }
    function errorText(error: LockError): string {
        switch (error) {
            case "schedule_required":
                return m.lock_error_schedule_required();
            case "date_required":
                return m.lock_error_date_required();
            case "date_invalid":
                return m.lock_error_date_invalid();
            case "date_future":
                return m.lock_error_future();
            case "date_extension":
                return m.lock_error_extension();
            case "date_limit":
                return m.lock_error_date_limit();
            case "lock_cannot_be_removed":
                return m.lock_error_cannot_remove();
        }
    }
    function displayEnd(value: string | null | undefined): string {
        const end = parseLockEnd(value ?? "");
        return end
            ? end.toLocaleString(getLocale(), {
                  dateStyle: "medium",
                  timeStyle: "short",
              })
            : (value ?? "—");
    }
    function startExtension() {
        if (isSubmitting()) return;
        setEditingExisting(true);
        updateRemoving(false);
        updateKind("active_until_lock_end");
    }
    function startRemoval() {
        if (isSubmitting() || props.block?.isLocked) return;
        setEditingExisting(true);
        updateRemoving(true);
    }

    async function handlePrimary() {
        const block = props.block,
            requestBody = payload();
        if (!block || !canApply() || !requestBody || isSubmitting()) return;
        if (validateLockDraft({ ...draft(), now: new Date() })) return;
        const needsConfirmation =
            removingLock() || selectedKind() !== "soft_unlockable";
        if (needsConfirmation && !confirmationRequested()) {
            setConfirmationRequested(true);
            return;
        }
        setIsSubmitting(true);
        try {
            const { error } = await setLock({
                path: { network_block_id: block.id },
                body: requestBody,
            });
            if (error) {
                setRequestError(true);
                toast.error(
                    error.code === "forbidden" || error.code === "locked"
                        ? m.block_locked_tooltip()
                        : m.failed_to_sync(),
                );
                return;
            }
            await props.onSave?.();
            props.onOpenChange(false);
        } catch (error) {
            console.error("Failed to update network block lock:", error);
            setRequestError(true);
            toast.error(m.failed_to_sync());
        } finally {
            setIsSubmitting(false);
        }
    }

    return (
        <Show when={props.open}>
            <div
                class="fixed inset-0 z-[80] grid place-items-center ace-modal-overlay p-4"
                role="presentation"
                onClick={(event) => {
                    if (event.target === event.currentTarget && !isSubmitting())
                        props.onOpenChange(false);
                }}
            >
                <section
                    role="dialog"
                    aria-modal="true"
                    aria-labelledby="lock-modal-title"
                    class="flex max-h-[90dvh] w-full max-w-2xl flex-col gap-0 overflow-hidden rounded-md border border-border bg-background p-0 shadow-xl"
                    onKeyDown={(event) => {
                        if (event.key === "Escape" && !isSubmitting())
                            props.onOpenChange(false);
                    }}
                >
                    <header class="relative flex flex-col gap-2 border-b border-border bg-card px-5 py-4 text-left sm:px-6">
                        <div class="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                            <ShieldCheck class="size-4" aria-hidden="true" />
                            {m.lock_protection_label()}
                        </div>
                        <h2
                            id="lock-modal-title"
                            class="font-condensed text-[28px] font-semibold leading-tight"
                        >
                            {configured()
                                ? m.lock_review_title()
                                : m.lock_protect_title()}
                        </h2>
                        <p class="max-w-xl text-sm leading-relaxed text-muted-foreground">
                            {m.lock_explanation()}
                        </p>
                        <Show when={props.block}>
                            <p class="mt-1 text-xs font-medium text-foreground">
                                {m.lock_for_block({ name: props.block!.name })}
                            </p>
                        </Show>
                        <Show when={!isSubmitting()}>
                            <button
                                type="button"
                                class="absolute right-4 top-4 text-muted-foreground"
                                aria-label={m.close()}
                                onClick={() => props.onOpenChange(false)}
                            >
                                <X size={18} />
                            </button>
                        </Show>
                    </header>
                    <div class="ace-lock-scroll min-h-0 flex-1 space-y-5 overflow-y-auto px-5 py-5 sm:px-6">
                        <Show when={props.block && original()}>
                            <Show
                                when={configured() && !editing()}
                                fallback={
                                    <>
                                        <Show when={!configured()}>
                                            <fieldset class="space-y-2">
                                                <legend class="mb-3 font-condensed text-lg font-semibold">
                                                    {m.lock_choose_mode()}
                                                </legend>
                                                <For each={OPTIONS}>
                                                    {(kind) => (
                                                        <button
                                                            type="button"
                                                            class={`ace-lock-option flex w-full items-start gap-3 rounded-md border px-4 py-3.5 text-left transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring ${selectedKind() === kind ? "border-foreground bg-muted/50" : "border-border hover:bg-muted/40"}`}
                                                            autofocus={kind === "soft_unlockable"}
                                                            aria-pressed={
                                                                selectedKind() ===
                                                                kind
                                                                    ? "true"
                                                                    : "false"
                                                            }
                                                            disabled={isSubmitting()}
                                                            onClick={() =>
                                                                updateKind(kind)
                                                            }
                                                        >
                                                            <span class="mt-0.5 grid size-9 shrink-0 place-items-center rounded-md border border-border bg-card">
                                                                <Show
                                                                    when={
                                                                        kind ===
                                                                        "soft_unlockable"
                                                                    }
                                                                    fallback={
                                                                        <Show
                                                                            when={
                                                                                kind ===
                                                                                "active_during_schedule"
                                                                            }
                                                                            fallback={
                                                                                <LockKeyhole class="size-4" />
                                                                            }
                                                                        >
                                                                            <CalendarClock class="size-4" />
                                                                        </Show>
                                                                    }
                                                                >
                                                                    <Hourglass class="size-4" />
                                                                </Show>
                                                            </span>
                                                            <span class="min-w-0 flex-1">
                                                                <span class="block text-sm font-semibold">
                                                                    {optionTitle(
                                                                        kind,
                                                                    )}
                                                                </span>
                                                                <span class="mt-1 block text-xs leading-relaxed text-muted-foreground">
                                                                    {optionDescription(
                                                                        kind,
                                                                    )}
                                                                </span>
                                                            </span>
                                                            <Show
                                                                when={
                                                                    selectedKind() ===
                                                                    kind
                                                                }
                                                            >
                                                                <Check class="mt-1 size-4 shrink-0 text-foreground" />
                                                            </Show>
                                                        </button>
                                                    )}
                                                </For>
                                            </fieldset>
                                        </Show>
                                        <Show
                                            when={
                                                selectedKind() ===
                                                    "active_until_lock_end" &&
                                                !removingLock()
                                            }
                                        >
                                            <div class="rounded-md border border-border bg-card p-4">
                                                <Label class="mb-3 block text-sm font-semibold">
                                                    {m.lock_expiration_datetime()}
                                                </Label>
                                                <Show when={configured()}>
                                                    <p class="mb-3 text-xs text-muted-foreground">
                                                        {props.block!.isLocked
                                                            ? m.lock_current_end(
                                                                  {
                                                                      date: displayEnd(
                                                                          original()
                                                                              ?.end,
                                                                      ),
                                                                  },
                                                              )
                                                            : m.lock_expired_on(
                                                                  {
                                                                      date: displayEnd(
                                                                          original()
                                                                              ?.end,
                                                                      ),
                                                                  },
                                                              )}
                                                    </p>
                                                </Show>
                                                <div class="flex flex-wrap gap-2">
                                                    <label class="flex min-w-0 flex-1 items-center gap-2">
                                                        <CalendarDays
                                                            size={16}
                                                            aria-hidden="true"
                                                        />
                                                        <input
                                                            type="date"
                                                            class="min-w-0 w-full rounded-sm border border-border bg-card px-2 py-2"
                                                            aria-label={m.lock_choose_end_date()}
                                                            min={todayLocal()}
                                                            max={maxLocal()}
                                                            value={selectedDate()}
                                                            disabled={isSubmitting()}
                                                            onInput={(event) =>
                                                                updateDate(
                                                                    event
                                                                        .currentTarget
                                                                        .value,
                                                                )
                                                            }
                                                        />
                                                    </label>
                                                    <input
                                                        type="time"
                                                        class="w-32 rounded-sm border border-border bg-card px-2 py-2"
                                                        aria-label={m.lock_choose_end_time()}
                                                        value={timeValue()}
                                                        disabled={isSubmitting()}
                                                        onInput={(event) =>
                                                            updateTime(
                                                                event
                                                                    .currentTarget
                                                                    .value,
                                                            )
                                                        }
                                                    />
                                                </div>
                                                <p class="mt-2 text-xs text-muted-foreground">
                                                    {m.lock_max_duration_hint()}
                                                </p>
                                            </div>
                                        </Show>
                                        <Show when={validationError()}>
                                            <div class="space-y-2">
                                                <p
                                                    class="flex items-start gap-2 text-sm text-destructive"
                                                    role="alert"
                                                >
                                                    <TriangleAlert class="mt-0.5 size-4 shrink-0" />
                                                    {errorText(
                                                        validationError()!,
                                                    )}
                                                </p>
                                                <Show
                                                    when={
                                                        validationError() ===
                                                            "schedule_required" &&
                                                        !!props.onEditSchedule
                                                    }
                                                >
                                                    <Button
                                                        variant="outline"
                                                        class="w-full sm:w-auto"
                                                        disabled={isSubmitting()}
                                                        onClick={() =>
                                                            props.block &&
                                                            props.onEditSchedule?.(
                                                                props.block,
                                                            )
                                                        }
                                                    >
                                                        <CalendarClock
                                                            class="mr-2 size-4"
                                                            aria-hidden="true"
                                                        />
                                                        {m.lock_edit_schedule_action()}
                                                        <ChevronRight
                                                            class="ml-1 size-4"
                                                            aria-hidden="true"
                                                        />
                                                    </Button>
                                                </Show>
                                            </div>
                                        </Show>
                                        <section
                                            class="rounded-md border border-border bg-muted/35 p-4"
                                            aria-labelledby="lock-preview-title"
                                        >
                                            <h3
                                                id="lock-preview-title"
                                                class="font-condensed text-lg font-semibold"
                                            >
                                                {m.lock_preview_title()}
                                            </h3>
                                            <p class="mt-2 text-sm leading-relaxed text-muted-foreground">
                                                {removingLock()
                                                    ? original()?.state ===
                                                      "active_until_lock_end"
                                                        ? m.lock_preview_expired_remove()
                                                        : m.lock_preview_remove()
                                                    : selectedKind() ===
                                                        "soft_unlockable"
                                                      ? m.lock_preview_reflection()
                                                      : selectedKind() ===
                                                          "active_during_schedule"
                                                        ? m.lock_preview_schedule()
                                                        : m.lock_preview_until()}
                                                <Show
                                                    when={
                                                        !removingLock() &&
                                                        selectedKind() ===
                                                            "active_until_lock_end" &&
                                                        !!proposedEnd()
                                                    }
                                                >
                                                    <strong class="mt-2 block font-medium text-foreground">
                                                        {m.lock_preview_deadline(
                                                            {
                                                                date: displayEnd(
                                                                    proposedEnd(),
                                                                ),
                                                            },
                                                        )}
                                                    </strong>
                                                </Show>
                                            </p>
                                        </section>
                                        <Show
                                            when={
                                                confirmationRequested() &&
                                                canApply()
                                            }
                                        >
                                            <section
                                                class="rounded-md border border-border bg-card p-4"
                                                role="status"
                                                aria-live="polite"
                                                aria-label={m.lock_confirmation_title()}
                                            >
                                                <div class="flex items-start gap-2">
                                                    <TriangleAlert class="mt-0.5 size-5 shrink-0" />
                                                    <div>
                                                        <h3 class="font-semibold">
                                                            {m.lock_confirmation_title()}
                                                        </h3>
                                                        <p class="mt-1 text-sm leading-relaxed text-muted-foreground">
                                                            {removingLock()
                                                                ? original()
                                                                      ?.state ===
                                                                  "active_until_lock_end"
                                                                    ? m.lock_confirmation_expired_remove()
                                                                    : m.lock_confirmation_remove()
                                                                : selectedKind() ===
                                                                    "active_until_lock_end"
                                                                  ? m.lock_confirmation_until(
                                                                        {
                                                                            date: displayEnd(
                                                                                proposedEnd(),
                                                                            ),
                                                                        },
                                                                    )
                                                                  : m.lock_confirmation_schedule()}
                                                        </p>
                                                    </div>
                                                </div>
                                            </section>
                                        </Show>
                                    </>
                                }
                            >
                                <section
                                    class="rounded-md border border-border bg-card p-4"
                                    aria-label={m.lock_current_state()}
                                >
                                    <div class="flex items-start gap-3">
                                        <span class="grid size-10 shrink-0 place-items-center rounded-md bg-muted text-foreground">
                                            <LockKeyhole class="size-5" />
                                        </span>
                                        <div class="min-w-0 flex-1">
                                            <h3 class="font-condensed text-xl font-semibold">
                                                {optionTitle(selectedKind())}
                                            </h3>
                                            <p class="mt-1 text-sm leading-relaxed text-muted-foreground">
                                                {optionDescription(
                                                    selectedKind(),
                                                )}
                                            </p>
                                            <Show
                                                when={
                                                    selectedKind() ===
                                                    "active_until_lock_end"
                                                }
                                            >
                                                <p class="mt-3 font-mono text-sm font-semibold">
                                                    {props.block!.isLocked
                                                        ? m.lock_current_end({
                                                              date: displayEnd(
                                                                  original()
                                                                      ?.end,
                                                              ),
                                                          })
                                                        : m.lock_expired_on({
                                                              date: displayEnd(
                                                                  original()
                                                                      ?.end,
                                                              ),
                                                          })}
                                                </p>
                                            </Show>
                                            <Show
                                                when={
                                                    selectedKind() ===
                                                    "active_during_schedule"
                                                }
                                            >
                                                <p class="mt-3 text-sm font-medium">
                                                    {props.block!.isLocked
                                                        ? m.lock_schedule_active_now()
                                                        : m.lock_schedule_outside_now()}
                                                </p>
                                            </Show>
                                        </div>
                                    </div>
                                </section>
                                <Show
                                    when={
                                        selectedKind() ===
                                        "active_until_lock_end"
                                    }
                                >
                                    <div class="flex flex-wrap items-center justify-between gap-3">
                                        <p class="max-w-sm text-xs leading-relaxed text-muted-foreground">
                                            {props.block!.isLocked
                                                ? m.lock_extension_only()
                                                : m.lock_expired_hint()}
                                        </p>
                                        <div class="flex flex-wrap gap-2">
                                            <Show when={!props.block!.isLocked}>
                                                <Button
                                                    variant="outline"
                                                    onClick={startRemoval}
                                                    disabled={isSubmitting()}
                                                >
                                                    {m.lock_remove_expired_action()}
                                                </Button>
                                            </Show>
                                            <Button
                                                variant="outline"
                                                onClick={startExtension}
                                                disabled={isSubmitting()}
                                            >
                                                <CalendarClock class="mr-2 size-4" />
                                                {m.lock_extend_action()}
                                            </Button>
                                        </div>
                                    </div>
                                </Show>
                                <Show
                                    when={
                                        selectedKind() ===
                                        "active_during_schedule"
                                    }
                                >
                                    <div class="space-y-3 rounded-md border border-border p-4">
                                        <p class="text-sm text-muted-foreground">
                                            {m.lock_schedule_review_hint()}
                                        </p>
                                        <Show when={!props.block!.isLocked}>
                                            <Button
                                                variant="outline"
                                                onClick={startRemoval}
                                                disabled={isSubmitting()}
                                            >
                                                {m.lock_remove_scheduled_action()}
                                            </Button>
                                        </Show>
                                    </div>
                                </Show>
                            </Show>
                        </Show>
                        <Show when={requestError()}>
                            <p class="text-sm text-destructive" role="alert">
                                {m.lock_save_failed()}
                            </p>
                        </Show>
                    </div>
                    <footer class="flex flex-row flex-wrap items-center justify-end gap-2 border-t border-border bg-card px-5 py-4 sm:px-6">
                        <Button
                            variant="ghost"
                            disabled={isSubmitting()}
                            onClick={() => props.onOpenChange(false)}
                        >
                            {configured() && !editing()
                                ? m.close()
                                : m.cancel()}
                        </Button>
                        <Show when={editing()}>
                            <Button
                                onClick={() => void handlePrimary()}
                                disabled={!canApply()}
                                class="min-h-10 bg-primary font-semibold text-primary-foreground hover:bg-primary/85"
                            >
                                {isSubmitting()
                                    ? m.saving()
                                    : confirmationRequested()
                                      ? removingLock()
                                          ? m.lock_confirm_remove_action()
                                          : selectedKind() ===
                                              "active_until_lock_end"
                                            ? m.lock_confirm_until_action()
                                            : m.lock_confirm_schedule_action()
                                      : removingLock()
                                        ? original()?.state ===
                                          "active_until_lock_end"
                                            ? m.lock_remove_expired_action()
                                            : m.lock_remove_scheduled_action()
                                        : selectedKind() === "soft_unlockable"
                                          ? m.lock_enable_reflection_action()
                                          : m.lock_continue_action()}
                                <Show
                                    when={
                                        !isSubmitting() &&
                                        !confirmationRequested() &&
                                        !removingLock() &&
                                        selectedKind() !== "soft_unlockable"
                                    }
                                >
                                    <ChevronRight
                                        class="ml-1 size-4"
                                        aria-hidden="true"
                                    />
                                </Show>
                            </Button>
                        </Show>
                    </footer>
                </section>
            </div>
        </Show>
    );
}
