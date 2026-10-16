<script lang="ts">
    import { onDestroy } from "svelte";
    import { invalidateAll } from "$app/navigation";
    import * as Dialog from "$lib/components/ui/dialog";
    import { Button } from "$lib/components/ui/button";
    import { Textarea } from "$lib/components/ui/textarea";
    import { toast } from "svelte-sonner";
    import { Lock, Hourglass } from "@lucide/svelte";
    import { initiateUnlock, confirmUnlock } from "$engine-client";
    import type { NetworkBlockSchema } from "$engine-client";
    import * as m from "$lib/paraglide/messages.js";

    let {
        open = $bindable(false),
        block,
    }: {
        open: boolean;
        block: NetworkBlockSchema;
    } = $props();

    // ── View state ──────────────────────────────────────────────
    type View = "reflection" | "countdown";
    let view = $state<View>("reflection");

    // ── View 1 — Reflection ─────────────────────────────────────
    const DEFAULT_MIN_WORDS = 20;
    const DEFAULT_COOLDOWN_SECONDS = 5 * 60;

    let reflectionPrompt = $state(m.soft_unlock_reflection_prompt());
    let minWords = $state(DEFAULT_MIN_WORDS);
    let answer = $state("");
    let isInitiating = $state(false);

    let wordCount = $derived(answer.trim().split(/\s+/).filter(Boolean).length);
    let canSubmit = $derived(wordCount >= minWords && answer.trim().length > 0);

    // ── View 2 — Countdown ──────────────────────────────────────
    let challengeId = $state("");
    let initiatedAt = $state(new Date());
    let cooldownSeconds = $state(DEFAULT_COOLDOWN_SECONDS);
    let cooldownDeadlineMs = $state(0);
    let remainingSeconds = $state(0);
    let isConfirming = $state(false);
    let intervalId: ReturnType<typeof setInterval> | undefined;

    function resetState() {
        stopTimer();
        view = "reflection";
        reflectionPrompt = m.soft_unlock_reflection_prompt();
        minWords = DEFAULT_MIN_WORDS;
        answer = "";
        isInitiating = false;
        isConfirming = false;
        challengeId = "";
        initiatedAt = new Date();
        cooldownSeconds = DEFAULT_COOLDOWN_SECONDS;
        cooldownDeadlineMs = 0;
        remainingSeconds = 0;
    }

    function closeModal() {
        resetState();
        open = false;
    }

    async function startCooldown() {
        isInitiating = true;
        try {
            // The server-side challenge is initiated only after the reflection
            // is complete, so the enforced five-minute cooldown starts here.
            // Re-opening a challenge that is already running remains idempotent.
            const res = await initiateUnlock({
                path: { network_block_id: block.id },
            });

            if (res.error) {
                toast.error(m.soft_unlock_failed_start_cooldown());
                return;
            }

            const data = res.data;
            if (!data?.challengeId) {
                toast.error(m.soft_unlock_failed_start_cooldown());
                return;
            }

            challengeId = data.challengeId;
            cooldownSeconds = Math.max(
                1,
                data.cooldownSeconds ?? DEFAULT_COOLDOWN_SECONDS,
            );

            const serverInitiatedAt = new Date(data.initiatedAt ?? "");
            initiatedAt = Number.isNaN(serverInitiatedAt.getTime())
                ? new Date()
                : serverInitiatedAt;
            cooldownDeadlineMs =
                initiatedAt.getTime() + cooldownSeconds * 1000;
            view = "countdown";
            startTimer();
        } catch (err) {
            console.error("Failed to start cooldown:", err);
            toast.error(m.soft_unlock_could_not_connect());
        } finally {
            isInitiating = false;
        }
    }

    // ── Timer ───────────────────────────────────────────────────
    function startTimer() {
        stopTimer();
        updateRemaining();
        if (remainingSeconds > 0) {
            intervalId = setInterval(updateRemaining, 1000);
        }
    }

    function stopTimer() {
        if (intervalId !== undefined) {
            clearInterval(intervalId);
            intervalId = undefined;
        }
    }

    function updateRemaining() {
        remainingSeconds = Math.max(
            0,
            Math.ceil((cooldownDeadlineMs - Date.now()) / 1000),
        );
        if (remainingSeconds === 0) stopTimer();
    }

    onDestroy(() => stopTimer());

    // ── Timer display ───────────────────────────────────────────
    let timerDisplay = $derived.by(() => {
        const m = Math.floor(remainingSeconds / 60);
        const s = remainingSeconds % 60;
        return `${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
    });

    let unlockLabel = $derived(
        remainingSeconds > 0
            ? m.soft_unlock_available_in({ time: timerDisplay })
            : m.soft_unlock_now(),
    );

    // ── Confirm ─────────────────────────────────────────────────
    async function handleConfirm() {
        if (remainingSeconds > 0 || !challengeId) return;
        isConfirming = true;
        try {
            const res = await confirmUnlock({
                path: { network_block_id: block.id },
                body: { challengeId },
            });

            if (res.error) {
                // Check for 425 Too Early
                if (res.response?.status === 425) {
                    const body = res.error as {
                        retryAfterSeconds?: number;
                        error?: string;
                    };
                    if (body.retryAfterSeconds && body.retryAfterSeconds > 0) {
                        cooldownDeadlineMs =
                            Date.now() + body.retryAfterSeconds * 1000;
                        startTimer();
                    }
                    toast.error(
                        body.retryAfterSeconds
                            ? m.soft_unlock_server_wait({
                                  seconds: body.retryAfterSeconds,
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
            await invalidateAll();
        } catch (err) {
            console.error("Failed to confirm unlock:", err);
            toast.error(m.soft_unlock_could_not_connect());
        } finally {
            isConfirming = false;
        }
    }
</script>

<Dialog.Root
    bind:open
    onOpenChange={(val) => {
        if (!val) resetState();
    }}
>
    <Dialog.Content
        class="flex max-h-[90dvh] w-full flex-col overflow-hidden rounded-md border-border bg-background p-0 shadow-none sm:max-w-lg"
    >
        <Dialog.Header class="border-b border-border bg-card p-5 pb-4">
            <Dialog.Title class="flex items-center gap-2 font-condensed text-2xl font-semibold">
                {#if view === "reflection"}
                    <Lock class="h-5 w-5 text-muted-foreground" />
                    {m.soft_unlock_take_a_moment()}
                {:else}
                    <Hourglass class="h-5 w-5 text-muted-foreground" />
                    {m.soft_unlock_cooldown_in_progress()}
                {/if}
            </Dialog.Title>
            <Dialog.Description class="text-sm text-muted-foreground mt-1">
                {#if view === "reflection"}
                    {m.soft_unlock_reflection_question()}
                {:else}
                    {m.soft_unlock_countdown_hint()}
                {/if}
            </Dialog.Description>
        </Dialog.Header>

        <div class="flex-1 overflow-y-auto px-6 pb-6 space-y-6 scroll-smooth">
            {#if view === "reflection"}
                <!-- Prompt -->
                <div
                    class="rounded-sm border border-border bg-muted/40 px-4 py-3 text-sm text-muted-foreground"
                >
                    {reflectionPrompt || m.soft_unlock_loading()}
                </div>

                <!-- Textarea -->
                <div class="space-y-1.5">
                    <Textarea
                        bind:value={answer}
                        placeholder={m.soft_unlock_answer_placeholder()}
                        rows={4}
                        class="resize-none w-full"
                        disabled={isInitiating}
                    />
                    <div class="flex justify-end">
                        <span
                            class="text-xs {wordCount >= minWords
                                ? 'text-[var(--ace-success)]'
                                : 'text-muted-foreground'}"
                        >
                            {m.soft_unlock_words_count({
                                count: wordCount,
                                minWords,
                            })}
                        </span>
                    </div>
                </div>
            {:else}
                <!-- Timer -->
                <div class="flex flex-col items-center gap-6 py-4">
                    <div
                        class="font-mono text-5xl font-semibold tabular-nums text-foreground"
                    >
                        {timerDisplay}
                    </div>
                    <p
                        class="text-sm text-muted-foreground text-center max-w-xs mx-auto"
                    >
                        {m.soft_unlock_urge_message()}
                    </p>
                </div>
            {/if}
        </div>

        <Dialog.Footer
            class="flex w-full flex-col items-stretch gap-3 border-t border-border bg-card p-4 sm:flex-col sm:space-x-0"
        >
            {#if view === "reflection"}
                <Button
                    class="h-11 w-full rounded-sm border border-border bg-foreground font-semibold text-background hover:bg-foreground/85"
                    onclick={startCooldown}
                    disabled={!canSubmit || isInitiating}
                >
                    {isInitiating
                        ? m.soft_unlock_starting()
                        : m.soft_unlock_start_cooldown()}
                </Button>
                <Button
                    variant="ghost"
                    class="h-10 w-full rounded-sm"
                    onclick={closeModal}
                >
                    {m.cancel()}
                </Button>
            {:else}
                <Button
                    class="h-11 w-full rounded-sm border border-border bg-foreground font-semibold text-background hover:bg-foreground/85 disabled:opacity-60"
                    onclick={handleConfirm}
                    disabled={remainingSeconds > 0 || isConfirming}
                >
                    {isConfirming ? m.soft_unlock_unlocking() : unlockLabel}
                </Button>
                <button
                    class="text-xs text-muted-foreground hover:text-foreground transition-colors mx-auto"
                    onclick={closeModal}
                >
                    {m.soft_unlock_changed_mind()}
                </button>
            {/if}
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>
