import { createEffect } from "solid-js";
import { reactiveFields } from "$lib/stores/reactive-fields";
import * as m from "$lib/paraglide/messages.js";
import { Button } from "$lib/components/ui/button/solid";
import * as Dialog from "$lib/components/ui/dialog/solid";
import * as Popover from "$lib/components/ui/popover/solid";
import * as AlertDialog from "$lib/components/ui/alert-dialog/solid";
import { Calendar } from "$lib/components/ui/calendar/solid";
import { Calendar as CalendarIcon, LockKeyhole } from "@lucide/solid";
import TimePicker from "$lib/components/TimePicker";
import LockSeal from "$lib/components/ace/domain/LockSeal";
import {
    CalendarDate,
    CalendarDateTime,
    DateFormatter,
    getLocalTimeZone,
    today,
    now,
} from "@internationalized/date";
import { formattedLockEnd } from "./lock-format";
import "./TimeLockDialog.css";

type Props = {
    open: boolean;
    onOpenChange?: (open: boolean) => void;
    isLocked: boolean;
    lockEndRaw: string | null;
    disabled?: boolean;
    committing?: boolean;
    onCommit: (isoEnd: string) => void;
};
export default function TimeLockDialog(props: Props) {
    const df = new DateFormatter("en-US", {
        month: "short",
        day: "numeric",
        year: "numeric",
    });
    const state = {
        calendarValue: undefined as CalendarDate | undefined,
        timeValue: "12:00",
        confirmOpen: false,
    };
    reactiveFields(state, ["calendarValue", "timeValue", "confirmOpen"]);
    const expiration = () => {
        if (!state.calendarValue || !state.timeValue) return null;
        try {
            const [hour, minute] = state.timeValue.split(":").map(Number);
            return new CalendarDateTime(
                state.calendarValue.year,
                state.calendarValue.month,
                state.calendarValue.day,
                hour,
                minute,
            );
        } catch {
            return null;
        }
    };
    const maxLockDate = today(getLocalTimeZone()).add({ days: 90 });
    const isCommitDisabled = () => {
        const date = expiration();
        if (!date) return true;
        try {
            return date.compare(now(getLocalTimeZone())) <= 0;
        } catch {
            return true;
        }
    };
    const confirmDatetime = () => {
        const date = expiration();
        return date
            ? `${df.format(date.toDate(getLocalTimeZone()))} at ${state.timeValue}`
            : "";
    };
    function commit() {
        const date = expiration();
        if (isCommitDisabled() || !date) return;
        const iso = date.toDate(getLocalTimeZone()).toISOString();
        state.confirmOpen = false;
        props.onCommit(iso);
    }
    createEffect(
        () => props.open,
        (open) => {
            if (open) {
                state.calendarValue = undefined;
                state.timeValue = "12:00";
                state.confirmOpen = false;
            }
        },
    );
    return (
        <>
            <Dialog.Root
                open={props.open}
                onOpenChange={(next: boolean) => props.onOpenChange?.(next)}
            >
                <Dialog.Content class="max-h-[90dvh] overflow-hidden rounded-md border-border bg-background p-0 shadow-none sm:max-w-[min(520px,calc(100vw-2rem))]">
                    {props.isLocked ? (
                        <>
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
                                        detail={
                                            props.lockEndRaw
                                                ? m.lock_expires_on({
                                                      datetime:
                                                          formattedLockEnd(
                                                              props.lockEndRaw,
                                                          ),
                                                  })
                                                : null
                                        }
                                    />
                                    <p
                                        data-testid="rm-lock-expires"
                                        class="sr-only"
                                    >
                                        {m.lock_expires_on({
                                            datetime: props.lockEndRaw
                                                ? formattedLockEnd(
                                                      props.lockEndRaw,
                                                  )
                                                : "",
                                        })}
                                    </p>
                                </div>
                                <div class="lock-dialog-footer">
                                    <Button
                                        variant="outline"
                                        onClick={() =>
                                            props.onOpenChange?.(false)
                                        }
                                    >
                                        {m.close()}
                                    </Button>
                                </div>
                            </div>
                        </>
                    ) : (
                        <>
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
                                                <Button
                                                    variant="outline"

                                                    class="h-10 w-full justify-start rounded-sm text-left font-normal"
                                                >
                                                    {state.calendarValue
                                                        ? df.format(
                                                              state.calendarValue.toDate(
                                                                  getLocalTimeZone(),
                                                              ),
                                                          )
                                                        : m.pick_date()}
                                                    <CalendarIcon class="ml-auto h-4 w-4 opacity-50" />
                                                </Button>
                                            </Popover.Trigger>
                                            <Popover.Content class="w-auto p-0">
                                                <Calendar
                                                    type="single"
                                                    value={state.calendarValue}
                                                    onValueChange={(
                                                        value: CalendarDate,
                                                    ) => {
                                                        state.calendarValue =
                                                            value;
                                                    }}
                                                    minValue={today(
                                                        getLocalTimeZone(),
                                                    )}
                                                    maxValue={maxLockDate}
                                                />
                                            </Popover.Content>
                                        </Popover.Root>
                                        <TimePicker
                                            value={state.timeValue}
                                            onValueChange={(value: string) => {
                                                state.timeValue = value;
                                            }}
                                        />
                                    </div>

                                    {expiration() ? (
                                        <>
                                            <div
                                                data-testid="rm-lock-resulting-expiration()"
                                                class="resulting-expiration()"
                                            >
                                                <LockKeyhole
                                                    size={14}
                                                    aria-hidden="true"
                                                />
                                                <span>
                                                    {m.lock_expires_on({
                                                        datetime:
                                                            confirmDatetime(),
                                                    })}
                                                </span>
                                            </div>
                                        </>
                                    ) : null}
                                </div>

                                <div class="lock-dialog-footer">
                                    <Button
                                        variant="ghost"
                                        onClick={() =>
                                            props.onOpenChange?.(false)
                                        }
                                    >
                                        {m.cancel()}
                                    </Button>
                                    <Button
                                        data-testid="rm-lock-commit"
                                        class="commit-button"
                                        onClick={() =>
                                            (state.confirmOpen = true)
                                        }
                                        disabled={
                                            isCommitDisabled() ||
                                            props.disabled ||
                                            props.committing
                                        }
                                    >
                                        {props.committing
                                            ? m.processing()
                                            : m.commit_and_lock()}
                                    </Button>
                                </div>
                            </div>
                        </>
                    )}
                </Dialog.Content>
            </Dialog.Root>

            <AlertDialog.Root
                open={state.confirmOpen}
                onOpenChange={(next: boolean) => {
                    state.confirmOpen = next;
                }}
            >
                <AlertDialog.Content class="rounded-md border-border bg-background shadow-none">
                    <AlertDialog.Header>
                        <AlertDialog.Title class="font-condensed text-2xl font-semibold">
                            {m.lock_confirm_title()}
                        </AlertDialog.Title>
                        <AlertDialog.Description class="text-sm leading-relaxed">
                            {m.lock_confirm_description({
                                datetime: confirmDatetime(),
                            })}
                        </AlertDialog.Description>
                    </AlertDialog.Header>
                    <AlertDialog.Footer class="mt-6 gap-3">
                        <AlertDialog.Cancel class="h-11 px-6 font-semibold border-border/60 hover:bg-muted">
                            {m.cancel()}
                        </AlertDialog.Cancel>
                        <AlertDialog.Action
                            data-testid="rm-lock-confirm"
                            onClick={commit}
                            class="h-11 rounded-sm border border-[var(--ace-signal-deep)] bg-[var(--ace-signal)] px-7 font-semibold text-[var(--ace-ink)] hover:bg-[var(--ace-signal-bright)]"
                        >
                            {m.commit_and_lock()}
                        </AlertDialog.Action>
                    </AlertDialog.Footer>
                </AlertDialog.Content>
            </AlertDialog.Root>
        </>
    );
}
