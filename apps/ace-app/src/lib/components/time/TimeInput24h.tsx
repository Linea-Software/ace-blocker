import { createMemo, createSignal, For, Show, onSettled } from "solid-js";
import { Clock } from "@lucide/solid";
import { cn } from "$lib/utils.js";

const hours = Array.from({ length: 24 }, (_, i) =>
    i.toString().padStart(2, "0"),
);
const minutes = Array.from({ length: 60 }, (_, i) =>
    i.toString().padStart(2, "0"),
);

export default function TimeInput24h(props: {
    value?: string;
    disabled?: boolean;
    className?: string;
    onValueChange?: (value: string) => void;
    onChange?: (value: string) => void;
}) {
    const [ownValue, setOwnValue] = createSignal("09:00");
    const [open, setOpen] = createSignal(false);
    let root: HTMLDivElement | undefined;
    const value = createMemo(() => props.value ?? ownValue());
    const selectedHour = createMemo(() => value().split(":")[0] || "09");
    const selectedMinute = createMemo(() => value().split(":")[1] || "00");
    const setTime = (h: string, m: string) => {
        const next = `${h}:${m}`;
        setOwnValue(next);
        props.onValueChange?.(next);
        props.onChange?.(next);
    };
    onSettled(() => {
        const outside = (e: PointerEvent) => {
            if (!root?.contains(e.target as Node)) setOpen(false);
        };
        const keydown = (e: KeyboardEvent) => {
            if (e.key === "Escape") setOpen(false);
        };
        document.addEventListener("pointerdown", outside);
        document.addEventListener("keydown", keydown);
        return () => {
            document.removeEventListener("pointerdown", outside);
            document.removeEventListener("keydown", keydown);
        };
    });
    return (
        <div class="relative" ref={root}>
            <button
                type="button"
                disabled={!!props.disabled}
                aria-haspopup="dialog"
                aria-expanded={open() ? "true" : "false"}
                onClick={() => setOpen((v) => !v)}
                class={cn(
                    "relative flex items-center w-full h-10 px-3 rounded-xl bg-background/50 border border-border/50 transition-all text-sm font-mono tabular-nums overflow-hidden",
                    props.disabled
                        ? "opacity-50 cursor-not-allowed"
                        : "hover:border-primary/30",
                    props.className,
                )}
            >
                <Clock class="w-4 h-4 text-muted-foreground mr-3" />
                <span>
                    {selectedHour()}:{selectedMinute()}
                </span>
            </button>
            <Show when={open() && !props.disabled}>
                <div
                    role="dialog"
                    aria-label="Select time"
                    class="absolute z-50 top-full left-0 mt-1 w-auto p-0 bg-card border border-border/50 rounded-md shadow-2xl"
                >
                    <div class="flex h-[240px] divide-x divide-border/20">
                        <div class="w-[64px] overflow-y-auto ace-scrollbar">
                            <div class="flex flex-col p-1 gap-1">
                                <For each={hours}>
                                    {(h) => (
                                        <button
                                            type="button"
                                            aria-pressed={
                                                h === selectedHour()
                                                    ? "true"
                                                    : "false"
                                            }
                                            onClick={() =>
                                                setTime(h, selectedMinute())
                                            }
                                            class={cn(
                                                "py-2 px-1 text-center text-sm font-mono rounded-md transition-colors",
                                                h === selectedHour()
                                                    ? "bg-primary text-primary-foreground font-bold"
                                                    : "text-muted-foreground hover:bg-muted hover:text-foreground",
                                            )}
                                        >
                                            {h}
                                        </button>
                                    )}
                                </For>
                            </div>
                        </div>
                        <div class="w-[64px] overflow-y-auto ace-scrollbar">
                            <div class="flex flex-col p-1 gap-1">
                                <For each={minutes}>
                                    {(m) => (
                                        <button
                                            type="button"
                                            aria-pressed={
                                                m === selectedMinute()
                                                    ? "true"
                                                    : "false"
                                            }
                                            onClick={() =>
                                                setTime(selectedHour(), m)
                                            }
                                            class={cn(
                                                "py-2 px-1 text-center text-sm font-mono rounded-md transition-colors",
                                                m === selectedMinute()
                                                    ? "bg-primary text-primary-foreground font-bold"
                                                    : "text-muted-foreground hover:bg-muted hover:text-foreground",
                                            )}
                                        >
                                            {m}
                                        </button>
                                    )}
                                </For>
                            </div>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    );
}
