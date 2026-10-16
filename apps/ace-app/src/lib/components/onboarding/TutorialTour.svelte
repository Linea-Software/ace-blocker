<script lang="ts">
    /**
     * Guided tour overlay.
     *
     * Walks the user through the real UI, spotlighting one element per step and
     * navigating between routes as needed. Rendered from the root layout while
     * `onboardingStore.tourRunning` is set.
     */
    import { untrack } from "svelte";
    import { goto } from "$app/navigation";
    import { resolve } from "$app/paths";
    import { page } from "$app/state";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Progress } from "$lib/components/ui/progress/index.js";
    import { X, ArrowLeft, ArrowRight, Check } from "@lucide/svelte";
    import * as m from "$lib/paraglide/messages.js";
    import { onboardingStore } from "$lib/stores/onboarding.svelte";
    import { TOUR_STEPS, type TourStep } from "$lib/onboarding/tour-steps";

    type Rect = { top: number; left: number; width: number; height: number };

    /** Gap between the spotlight and the tooltip, in pixels. */
    const TOOLTIP_GAP = 16;
    /** Padding added around the spotlighted element. */
    const SPOTLIGHT_PADDING = 8;
    /** Minimum distance the tooltip keeps from the viewport edges. */
    const VIEWPORT_MARGIN = 16;
    const TOOLTIP_WIDTH = 340;
    /** How long we wait for a step's target element to appear before giving up. */
    const TARGET_TIMEOUT_MS = 2000;

    let stepIndex = $state(0);
    let targetElement = $state<HTMLElement | null>(null);
    let targetRect = $state<Rect | null>(null);
    let tooltipHeight = $state(200);

    const step = $derived<TourStep | undefined>(TOUR_STEPS[stepIndex]);
    const isLastStep = $derived(stepIndex === TOUR_STEPS.length - 1);
    const progress = $derived(((stepIndex + 1) / TOUR_STEPS.length) * 100);

    /** Polls for the step's target until it shows up or we time out. */
    async function waitForElement(
        selector: string,
    ): Promise<HTMLElement | null> {
        const deadline = performance.now() + TARGET_TIMEOUT_MS;

        while (performance.now() < deadline) {
            const element = document.querySelector<HTMLElement>(selector);
            if (element) return element;
            await new Promise((resolve) => setTimeout(resolve, 50));
        }

        return null;
    }

    function measure() {
        if (!targetElement) {
            targetRect = null;
            return;
        }

        const box = targetElement.getBoundingClientRect();
        targetRect = {
            top: box.top - SPOTLIGHT_PADDING,
            left: box.left - SPOTLIGHT_PADDING,
            width: box.width + SPOTLIGHT_PADDING * 2,
            height: box.height + SPOTLIGHT_PADDING * 2,
        };
    }

    // Resolve the current step: navigate if needed, then find and reveal its target.
    $effect(() => {
        const currentStep = TOUR_STEPS[stepIndex];
        if (!currentStep) return;

        let cancelled = false;

        (async () => {
            // Read the pathname untracked: navigating must not re-trigger this effect.
            const currentPath = untrack(() => page.url.pathname);
            if (currentPath !== currentStep.route) {
                await goto(resolve(currentStep.route));
            }
            if (cancelled) return;

            const element = await waitForElement(currentStep.selector);
            if (cancelled) return;

            targetElement = element;

            if (element) {
                element.scrollIntoView({ block: "center", behavior: "smooth" });
                // Let the smooth scroll settle before measuring.
                await new Promise((resolve) => setTimeout(resolve, 350));
                if (cancelled) return;
            }

            measure();
        })();

        return () => {
            cancelled = true;
        };
    });

    // Keep the spotlight glued to its target while the window moves around.
    $effect(() => {
        if (!targetElement) return;

        const handleViewportChange = () => measure();

        window.addEventListener("resize", handleViewportChange);
        window.addEventListener("scroll", handleViewportChange, true);

        return () => {
            window.removeEventListener("resize", handleViewportChange);
            window.removeEventListener("scroll", handleViewportChange, true);
        };
    });

    const tooltipPosition = $derived.by(() => {
        if (typeof window === "undefined") return { top: 0, left: 0 };

        const viewportWidth = window.innerWidth;
        const viewportHeight = window.innerHeight;

        // No target on screen: park the tooltip in the middle.
        if (!targetRect || !step) {
            return {
                top: Math.max(
                    VIEWPORT_MARGIN,
                    viewportHeight / 2 - tooltipHeight / 2,
                ),
                left: Math.max(
                    VIEWPORT_MARGIN,
                    viewportWidth / 2 - TOOLTIP_WIDTH / 2,
                ),
            };
        }

        let top: number;
        let left: number;

        switch (step.placement) {
            case "right":
                left = targetRect.left + targetRect.width + TOOLTIP_GAP;
                top = targetRect.top + targetRect.height / 2 - tooltipHeight / 2;
                break;
            case "left":
                left = targetRect.left - TOOLTIP_WIDTH - TOOLTIP_GAP;
                top = targetRect.top + targetRect.height / 2 - tooltipHeight / 2;
                break;
            case "top":
                left =
                    targetRect.left + targetRect.width / 2 - TOOLTIP_WIDTH / 2;
                top = targetRect.top - tooltipHeight - TOOLTIP_GAP;
                break;
            default:
                left =
                    targetRect.left + targetRect.width / 2 - TOOLTIP_WIDTH / 2;
                top = targetRect.top + targetRect.height + TOOLTIP_GAP;
                break;
        }

        const maxLeft = viewportWidth - TOOLTIP_WIDTH - VIEWPORT_MARGIN;
        const maxTop = viewportHeight - tooltipHeight - VIEWPORT_MARGIN;

        return {
            top: Math.min(Math.max(top, VIEWPORT_MARGIN), Math.max(maxTop, VIEWPORT_MARGIN)),
            left: Math.min(Math.max(left, VIEWPORT_MARGIN), Math.max(maxLeft, VIEWPORT_MARGIN)),
        };
    });

    function goToPrevious() {
        if (stepIndex > 0) stepIndex -= 1;
    }

    function goToNext() {
        if (isLastStep) {
            finish();
            return;
        }
        stepIndex += 1;
    }

    function finish() {
        onboardingStore.completeTour();
    }

    function handleKeydown(event: KeyboardEvent) {
        if (event.key === "Escape") {
            event.preventDefault();
            finish();
        }
    }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if step}
    <!-- Backdrop: swallows clicks so the user can't wander off mid-tour. -->
    <div
        class="fixed inset-0 z-[100]"
        role="presentation"
        aria-hidden="true"
    ></div>

    <!-- Spotlight. The huge spread shadow is what dims everything else. -->
    {#if targetRect}
        <div
            class="pointer-events-none fixed z-[101] rounded-md ring-2 ring-[var(--ace-signal)] transition-all duration-150"
            style:top="{targetRect.top}px"
            style:left="{targetRect.left}px"
            style:width="{targetRect.width}px"
            style:height="{targetRect.height}px"
            style:box-shadow="0 0 0 9999px rgba(0, 0, 0, 0.65)"
        ></div>
    {:else}
        <div
            class="fixed inset-0 z-[101] bg-black/65 pointer-events-none"
        ></div>
    {/if}

    <!-- Tooltip -->
    <div
        class="fixed z-[102] space-y-4 rounded-md border border-border bg-card p-5 shadow-none transition-all duration-150"
        style:top="{tooltipPosition.top}px"
        style:left="{tooltipPosition.left}px"
        style:width="{TOOLTIP_WIDTH}px"
        bind:clientHeight={tooltipHeight}
        role="dialog"
        aria-modal="true"
        aria-label={step.title()}
    >
        <div class="flex items-start justify-between gap-3">
            <div class="space-y-1">
                <p
                    class="font-mono text-[11px] font-medium text-muted-foreground"
                >
                    {m.tour_step_counter({
                        current: stepIndex + 1,
                        total: TOUR_STEPS.length,
                    })}
                </p>
                <h2 class="font-condensed text-xl font-semibold">{step.title()}</h2>
            </div>
            <Button
                variant="ghost"
                size="icon"
                class="size-8 shrink-0 text-muted-foreground"
                aria-label={m.tour_skip()}
                onclick={finish}
            >
                <X class="size-4" />
            </Button>
        </div>

        <p class="text-sm text-muted-foreground leading-relaxed">
            {step.body()}
        </p>

        <Progress value={progress} class="h-1" />

        <div class="flex items-center justify-between gap-2">
            <Button
                variant="ghost"
                size="sm"
                class="text-muted-foreground"
                onclick={finish}
            >
                {m.tour_skip()}
            </Button>

            <div class="flex items-center gap-2">
                {#if stepIndex > 0}
                    <Button variant="outline" size="sm" onclick={goToPrevious}>
                        <ArrowLeft class="size-4 mr-1" />
                        {m.onboarding_back()}
                    </Button>
                {/if}
                <Button size="sm" onclick={goToNext}>
                    {#if isLastStep}
                        {m.tour_finish()}
                        <Check class="size-4 ml-1" />
                    {:else}
                        {m.onboarding_next()}
                        <ArrowRight class="size-4 ml-1" />
                    {/if}
                </Button>
            </div>
        </div>
    </div>
{/if}
