import {
    createEffect,
    createMemo,
    createSignal,
    Show,
    onSettled,
} from "solid-js";
import { useLocation, useNavigate } from "@solidjs/router";
import { X, ArrowLeft, ArrowRight, Check } from "@lucide/solid";
import * as m from "$lib/paraglide/messages.js";
import { onboardingStore } from "$lib/stores/onboarding";
import { TOUR_STEPS, type TourStep } from "$lib/onboarding/tour-steps";

type Rect = { top: number; left: number; width: number; height: number };
const TOOLTIP_GAP = 16,
    SPOTLIGHT_PADDING = 8,
    VIEWPORT_MARGIN = 16,
    TOOLTIP_WIDTH = 340,
    TARGET_TIMEOUT_MS = 2000;
const button =
    "inline-flex items-center justify-center gap-2 rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground hover:bg-primary/90";
const ghost =
    "inline-flex items-center justify-center gap-2 rounded-md px-3 py-1.5 text-sm font-medium text-muted-foreground hover:bg-muted";
const outline =
    "inline-flex items-center justify-center gap-2 rounded-md border border-input bg-background px-3 py-1.5 text-sm font-medium hover:bg-muted";

export default function TutorialTour() {
    const navigate = useNavigate();
    const location = useLocation();
    const [stepIndex, setStepIndex] = createSignal(0);
    const [targetElement, setTargetElement] = createSignal<HTMLElement | null>(
        null,
    );
    const [targetRect, setTargetRect] = createSignal<Rect | null>(null);
    const [tooltipHeight, setTooltipHeight] = createSignal(200);
    const [viewport, setViewport] = createSignal({
        width: typeof window === "undefined" ? 0 : window.innerWidth,
        height: typeof window === "undefined" ? 0 : window.innerHeight,
    });
    let tooltip: HTMLDivElement | undefined;
    const step = createMemo<TourStep | undefined>(
        () => TOUR_STEPS[stepIndex()],
    );
    const isLastStep = createMemo(() => stepIndex() === TOUR_STEPS.length - 1);
    const progress = createMemo(
        () => ((stepIndex() + 1) / TOUR_STEPS.length) * 100,
    );
    const finish = () => onboardingStore.completeTour();
    const previous = () => {
        if (stepIndex() > 0) setStepIndex((i) => i - 1);
    };
    const next = () => {
        if (isLastStep()) finish();
        else setStepIndex((i) => i + 1);
    };
    function measure() {
        const element = targetElement();
        if (!element) {
            setTargetRect(null);
            return;
        }
        const box = element.getBoundingClientRect();
        setTargetRect({
            top: box.top - SPOTLIGHT_PADDING,
            left: box.left - SPOTLIGHT_PADDING,
            width: box.width + SPOTLIGHT_PADDING * 2,
            height: box.height + SPOTLIGHT_PADDING * 2,
        });
    }
    async function waitForElement(selector: string) {
        const deadline = performance.now() + TARGET_TIMEOUT_MS;
        while (performance.now() < deadline) {
            const el = document.querySelector<HTMLElement>(selector);
            if (el) return el;
            await new Promise<void>((resolve) => setTimeout(resolve, 50));
        }
        return null;
    }
    createEffect(
        () => stepIndex(),
        (index) => {
            const current = TOUR_STEPS[index];
            if (!current) return;
            let cancelled = false;
            setTargetElement(null);
            setTargetRect(null);
            void (async () => {
                if (location.pathname !== current.route)
                    await navigate(current.route);
                if (cancelled) return;
                const element = await waitForElement(current.selector);
                if (cancelled) return;
                setTargetElement(element);
                if (element) {
                    element.scrollIntoView({
                        block: "center",
                        behavior: "smooth",
                    });
                    await new Promise<void>((resolve) =>
                        setTimeout(resolve, 350),
                    );
                    if (cancelled) return;
                }
                measure();
            })();
            return () => {
                cancelled = true;
            };
        },
    );
    const tooltipPosition = createMemo(() => {
        const { width, height } = viewport();
        const rect = targetRect(),
            s = step(),
            tipHeight = tooltipHeight();
        if (!rect || !s)
            return {
                top: Math.max(VIEWPORT_MARGIN, height / 2 - tipHeight / 2),
                left: Math.max(VIEWPORT_MARGIN, width / 2 - TOOLTIP_WIDTH / 2),
            };
        let top: number, left: number;
        switch (s.placement) {
            case "right":
                left = rect.left + rect.width + TOOLTIP_GAP;
                top = rect.top + rect.height / 2 - tipHeight / 2;
                break;
            case "left":
                left = rect.left - TOOLTIP_WIDTH - TOOLTIP_GAP;
                top = rect.top + rect.height / 2 - tipHeight / 2;
                break;
            case "top":
                left = rect.left + rect.width / 2 - TOOLTIP_WIDTH / 2;
                top = rect.top - tipHeight - TOOLTIP_GAP;
                break;
            default:
                left = rect.left + rect.width / 2 - TOOLTIP_WIDTH / 2;
                top = rect.top + rect.height + TOOLTIP_GAP;
        }
        return {
            top: Math.min(
                Math.max(top, VIEWPORT_MARGIN),
                Math.max(height - tipHeight - VIEWPORT_MARGIN, VIEWPORT_MARGIN),
            ),
            left: Math.min(
                Math.max(left, VIEWPORT_MARGIN),
                Math.max(
                    width - TOOLTIP_WIDTH - VIEWPORT_MARGIN,
                    VIEWPORT_MARGIN,
                ),
            ),
        };
    });
    onSettled(() => {
        const onViewportChange = () => {
            setViewport({
                width: window.innerWidth,
                height: window.innerHeight,
            });
            measure();
        };
        const onKeydown = (event: KeyboardEvent) => {
            if (event.key === "Escape") {
                event.preventDefault();
                finish();
            }
        };
        window.addEventListener("resize", onViewportChange);
        window.addEventListener("scroll", onViewportChange, true);
        window.addEventListener("keydown", onKeydown);
        const observer = new ResizeObserver((entries) => {
            const box = entries[0]?.contentRect;
            if (box) setTooltipHeight(box.height);
        });
        if (tooltip) observer.observe(tooltip);
        return () => {
            observer.disconnect();
            window.removeEventListener("resize", onViewportChange);
            window.removeEventListener("scroll", onViewportChange, true);
            window.removeEventListener("keydown", onKeydown);
        };
    });
    return (
        <Show when={step()}>
            {(current) => (
                <>
                    <div
                        class="fixed inset-0 z-[100]"
                        role="presentation"
                        aria-hidden="true"
                    />
                    <Show
                        when={targetRect()}
                        fallback={
                            <div class="fixed inset-0 z-[101] bg-black/65 pointer-events-none" />
                        }
                    >
                        {(rect) => (
                            <div
                                class="pointer-events-none fixed z-[101] rounded-md ring-2 ring-[var(--foreground)] transition-all duration-150"
                                style={{
                                    top: rect().top + "px",
                                    left: rect().left + "px",
                                    width: rect().width + "px",
                                    height: rect().height + "px",
                                    "box-shadow":
                                        "0 0 0 9999px rgba(0, 0, 0, 0.65)",
                                }}
                            />
                        )}
                    </Show>
                    <div
                        ref={tooltip}
                        role="dialog"
                        aria-modal="true"
                        aria-label={current().title()}
                        class="fixed z-[102] space-y-4 rounded-md border border-border bg-card p-5 shadow-none transition-all duration-150"
                        style={{
                            top: tooltipPosition().top + "px",
                            left: tooltipPosition().left + "px",
                            width: TOOLTIP_WIDTH + "px",
                        }}
                    >
                        <div class="flex items-start justify-between gap-3">
                            <div class="space-y-1">
                                <p class="font-mono text-[11px] font-medium text-muted-foreground">
                                    {m.tour_step_counter({
                                        current: stepIndex() + 1,
                                        total: TOUR_STEPS.length,
                                    })}
                                </p>
                                <h2 class="font-condensed text-xl font-semibold">
                                    {current().title()}
                                </h2>
                            </div>
                            <button
                                type="button"
                                class={ghost + " size-8 shrink-0"}
                                aria-label={m.tour_skip()}
                                onClick={finish}
                            >
                                <X class="size-4" />
                            </button>
                        </div>
                        <p class="text-sm text-muted-foreground leading-relaxed">
                            {current().body()}
                        </p>
                        <div
                            role="progressbar"
                            aria-valuemin={0}
                            aria-valuemax={100}
                            aria-valuenow={progress()}
                            class="relative h-1 w-full overflow-hidden rounded-full bg-primary/20"
                        >
                            <div
                                class="h-full bg-primary transition-all"
                                style={{ width: progress() + "%" }}
                            />
                        </div>
                        <div class="flex items-center justify-between gap-2">
                            <button
                                type="button"
                                class={ghost}
                                onClick={finish}
                            >
                                {m.tour_skip()}
                            </button>
                            <div class="flex items-center gap-2">
                                <Show when={stepIndex() > 0}>
                                    <button
                                        type="button"
                                        class={outline}
                                        onClick={previous}
                                    >
                                        <ArrowLeft class="size-4 mr-1" />
                                        {m.onboarding_back()}
                                    </button>
                                </Show>
                                <button
                                    type="button"
                                    class={button}
                                    onClick={next}
                                >
                                    <Show
                                        when={isLastStep()}
                                        fallback={
                                            <>
                                                {m.onboarding_next()}
                                                <ArrowRight class="size-4 ml-1" />
                                            </>
                                        }
                                    >
                                        {m.tour_finish()}
                                        <Check class="size-4 ml-1" />
                                    </Show>
                                </button>
                            </div>
                        </div>
                    </div>
                </>
            )}
        </Show>
    );
}
