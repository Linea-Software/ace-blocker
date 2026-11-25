import { createMemo, createSignal, onSettled, Show } from "solid-js";
import { Button } from "$lib/components/ui/button/solid";
import { Textarea } from "$lib/components/ui/textarea/solid";
import { toast } from "solid-sonner";
import { Lock, Hourglass } from "@lucide/solid";
import {
    initiateUnlock,
    confirmUnlock,
    type NetworkBlockSchema,
} from "$engine-client";
import * as m from "$lib/paraglide/messages.js";

export interface SoftUnlockModalProps {
    open: boolean;
    block: NetworkBlockSchema;
    onOpenChange: (open: boolean) => void;
    onSave?: () => void | Promise<void>;
}

const DEFAULT_MIN_WORDS = 20;
const DEFAULT_COOLDOWN_SECONDS = 5 * 60;

export default function SoftUnlockModal(props: SoftUnlockModalProps) {
    const [view, setView] = createSignal<"reflection" | "countdown">(
        "reflection",
    );
    const [reflectionPrompt, setReflectionPrompt] = createSignal(
        m.soft_unlock_reflection_prompt(),
    );
    const [minWords, setMinWords] = createSignal(DEFAULT_MIN_WORDS);
    const [answer, setAnswer] = createSignal("");
    const [isInitiating, setIsInitiating] = createSignal(false);
    const [challengeId, setChallengeId] = createSignal("");
    const [cooldownDeadlineMs, setCooldownDeadlineMs] = createSignal(0);
    const [remainingSeconds, setRemainingSeconds] = createSignal(0);
    const [isConfirming, setIsConfirming] = createSignal(false);
    let intervalId: ReturnType<typeof setInterval> | undefined;
    const wordCount = createMemo(
        () => answer().trim().split(/\s+/).filter(Boolean).length,
    );
    const canSubmit = () =>
        wordCount() >= minWords() && answer().trim().length > 0;
    const timerDisplay = createMemo(
        () =>
            `${String(Math.floor(remainingSeconds() / 60)).padStart(2, "0")}:${String(remainingSeconds() % 60).padStart(2, "0")}`,
    );
    const unlockLabel = createMemo(() =>
        remainingSeconds() > 0
            ? m.soft_unlock_available_in({ time: timerDisplay() })
            : m.soft_unlock_now(),
    );

    function stopTimer() {
        if (intervalId !== undefined) {
            clearInterval(intervalId);
            intervalId = undefined;
        }
    }
    function updateRemaining() {
        const remaining = Math.max(
            0,
            Math.ceil((cooldownDeadlineMs() - Date.now()) / 1000),
        );
        setRemainingSeconds(remaining);
        if (remaining === 0) stopTimer();
    }
    function startTimer() {
        stopTimer();
        updateRemaining();
        if (remainingSeconds() > 0)
            intervalId = setInterval(updateRemaining, 1000);
    }
    function resetState() {
        stopTimer();
        setView("reflection");
        setReflectionPrompt(m.soft_unlock_reflection_prompt());
        setMinWords(DEFAULT_MIN_WORDS);
        setAnswer("");
        setIsInitiating(false);
        setIsConfirming(false);
        setChallengeId("");
        setCooldownDeadlineMs(0);
        setRemainingSeconds(0);
    }
    function closeModal() {
        resetState();
        props.onOpenChange(false);
    }

    onSettled(() => stopTimer);

    async function startCooldown() {
        if (!canSubmit() || isInitiating()) return;
        setIsInitiating(true);
        try {
            const result = await initiateUnlock({
                path: { network_block_id: props.block.id },
            });
            if (result.error || !result.data?.challengeId) {
                toast.error(m.soft_unlock_failed_start_cooldown());
                return;
            }
            setChallengeId(result.data.challengeId);
            const cooldownSeconds = Math.max(
                1,
                result.data.cooldownSeconds ?? DEFAULT_COOLDOWN_SECONDS,
            );
            const serverInitiatedAt = new Date(result.data.initiatedAt ?? "");
            const initiatedAt = Number.isNaN(serverInitiatedAt.getTime())
                ? new Date()
                : serverInitiatedAt;
            setCooldownDeadlineMs(
                initiatedAt.getTime() + cooldownSeconds * 1000,
            );
            setView("countdown");
            startTimer();
        } catch (error) {
            console.error("Failed to start cooldown:", error);
            toast.error(m.soft_unlock_could_not_connect());
        } finally {
            setIsInitiating(false);
        }
    }

    async function handleConfirm() {
        if (remainingSeconds() > 0 || !challengeId() || isConfirming()) return;
        setIsConfirming(true);
        try {
            const result = await confirmUnlock({
                path: { network_block_id: props.block.id },
                body: { challengeId: challengeId() },
            });
            if (result.error) {
                if (result.error.status === 425) {
                    const error = result.error as {
                        retryAfterSeconds?: number;
                        error?: string;
                    };
                    if (
                        error.retryAfterSeconds &&
                        error.retryAfterSeconds > 0
                    ) {
                        setCooldownDeadlineMs(
                            Date.now() + error.retryAfterSeconds * 1000,
                        );
                        startTimer();
                    }
                    toast.error(
                        error.retryAfterSeconds
                            ? m.soft_unlock_server_wait({
                                  seconds: error.retryAfterSeconds,
                              })
                            : m.soft_unlock_cooldown_not_elapsed(),
                    );
                    return;
                }
                toast.error(m.soft_unlock_failed_confirm());
                return;
            }
            toast.success(m.soft_unlock_success());
            closeModal();
            await props.onSave?.();
        } catch (error) {
            console.error("Failed to confirm unlock:", error);
            toast.error(m.soft_unlock_could_not_connect());
        } finally {
            setIsConfirming(false);
        }
    }

    return (
        <Show when={props.open}>
            <div
                class="fixed inset-0 z-[80] grid place-items-center ace-modal-overlay p-4"
                role="presentation"
                onClick={(event) => {
                    if (event.target === event.currentTarget) closeModal();
                }}
            >
                <section
                    role="dialog"
                    aria-modal="true"
                    aria-labelledby="soft-unlock-title"
                    class="flex max-h-[90dvh] w-full max-w-lg flex-col overflow-hidden rounded-md border border-border bg-background p-0 shadow-none"
                    onKeyDown={(event) => {
                        if (event.key === "Escape") closeModal();
                    }}
                >
                    <header class="border-b border-border bg-card p-5 pb-4">
                        <h2
                            id="soft-unlock-title"
                            class="flex items-center gap-2 font-condensed text-2xl font-semibold"
                        >
                            <Show
                                when={view() === "reflection"}
                                fallback={
                                    <>
                                        <Hourglass class="h-5 w-5 text-muted-foreground" />
                                        {m.soft_unlock_cooldown_in_progress()}
                                    </>
                                }
                            >
                                <Lock class="h-5 w-5 text-muted-foreground" />
                                {m.soft_unlock_take_a_moment()}
                            </Show>
                        </h2>
                        <p class="text-sm text-muted-foreground mt-1">
                            {view() === "reflection"
                                ? m.soft_unlock_reflection_question()
                                : m.soft_unlock_countdown_hint()}
                        </p>
                    </header>
                    <div class="flex-1 overflow-y-auto px-6 pb-6 space-y-6 scroll-smooth">
                        <Show
                            when={view() === "reflection"}
                            fallback={
                                <div class="flex flex-col items-center gap-6 py-4">
                                    <div class="font-mono text-5xl font-semibold tabular-nums text-foreground">
                                        {timerDisplay()}
                                    </div>
                                    <p class="text-sm text-muted-foreground text-center max-w-xs mx-auto">
                                        {m.soft_unlock_urge_message()}
                                    </p>
                                </div>
                            }
                        >
                            <div class="rounded-sm border border-border bg-muted/40 px-4 py-3 text-sm text-muted-foreground">
                                {reflectionPrompt() || m.soft_unlock_loading()}
                            </div>
                            <div class="space-y-1.5">
                                <Textarea
                                    value={answer()}
                                    onInput={(event) =>
                                        setAnswer(event.currentTarget.value)
                                    }
                                    placeholder={m.soft_unlock_answer_placeholder()}
                                    rows={4}
                                    class="resize-none w-full"
                                    disabled={isInitiating()}
                                />
                                <div class="flex justify-end">
                                    <span
                                        class={[
                                            "text-xs",
                                            wordCount() >= minWords()
                                                ? "text-[var(--ace-success)]"
                                                : "text-muted-foreground",
                                        ]}
                                    >
                                        {m.soft_unlock_words_count({
                                            count: wordCount(),
                                            minWords: minWords(),
                                        })}
                                    </span>
                                </div>
                            </div>
                        </Show>
                    </div>
                    <footer class="flex w-full flex-col items-stretch gap-3 border-t border-border bg-card p-4 sm:flex-col">
                        <Show
                            when={view() === "reflection"}
                            fallback={
                                <>
                                    <Button
                                        class="h-11 w-full rounded-sm border border-border bg-foreground font-semibold text-background hover:bg-foreground/85 disabled:opacity-60"
                                        onClick={() => void handleConfirm()}
                                        disabled={
                                            remainingSeconds() > 0 ||
                                            isConfirming()
                                        }
                                    >
                                        {isConfirming()
                                            ? m.soft_unlock_unlocking()
                                            : unlockLabel()}
                                    </Button>
                                    <button
                                        type="button"
                                        class="text-xs text-muted-foreground hover:text-foreground transition-colors mx-auto"
                                        onClick={closeModal}
                                    >
                                        {m.soft_unlock_changed_mind()}
                                    </button>
                                </>
                            }
                        >
                            <Button
                                class="h-11 w-full rounded-sm border border-border bg-foreground font-semibold text-background hover:bg-foreground/85"
                                onClick={() => void startCooldown()}
                                disabled={!canSubmit() || isInitiating()}
                            >
                                {isInitiating()
                                    ? m.soft_unlock_starting()
                                    : m.soft_unlock_start_cooldown()}
                            </Button>
                            <Button
                                variant="ghost"
                                class="h-10 w-full rounded-sm"
                                onClick={closeModal}
                            >
                                {m.cancel()}
                            </Button>
                        </Show>
                    </footer>
                </section>
            </div>
        </Show>
    );
}
