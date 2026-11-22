import { createEffect, createSignal, For, Show, onSettled } from "solid-js";
import { Portal } from "@solidjs/web";
import { Button } from "$lib/components/ui/button/solid";
import { Clock } from "@lucide/solid";
import { cn } from "$lib/utils";

export interface TimePickerProps {
    value: string;
    onValueChange: (value: string) => void;
    ariaLabel?: string;
    disabled?: boolean;
}

const hours = Array.from({length: 24}, (_, index) => index);
const minutes = Array.from({length: 12}, (_, index) => index * 5);
const pad = (n: number) => String(n).padStart(2, "0");

export default function TimePicker(props: TimePickerProps) {
    const [open, setOpen] = createSignal(false);
    const [position, setPosition] = createSignal({left: 0, top: 0});
    let trigger!: HTMLButtonElement;
    let content: HTMLDivElement | undefined;
    let hourCol: HTMLDivElement | undefined;
    let minCol: HTMLDivElement | undefined;
    const hour = () => parseInt(props.value.split(":")[0] ?? "0", 10);
    const minute = () => parseInt(props.value.split(":")[1] ?? "0", 10);
    const minuteSnapped = () => Math.min(55, Math.round(minute() / 5) * 5);
    const selectHour = (nextHour: number) => props.onValueChange(`${pad(nextHour)}:${pad(minuteSnapped())}`);
    const selectMinute = (nextMinute: number) => props.onValueChange(`${pad(hour())}:${pad(nextMinute)}`);

    function toggle() {
        if (props.disabled) return;
        if (!open()) {
            const rect = trigger.getBoundingClientRect();
            setPosition({
                left: Math.max(12, Math.min(window.innerWidth - 158, rect.left)),
                top: Math.max(12, Math.min(window.innerHeight - 218, rect.bottom + 6)),
            });
        }
        setOpen(!open());
    }
    createEffect(
        () => open(),
        expanded => {
            if (!expanded) return;
            const frame = requestAnimationFrame(() => {
                const hourButtons = hourCol?.querySelectorAll("button") ?? [];
                if (hourCol && hourButtons.length) {
                    const height = (hourButtons[0] as HTMLElement).offsetHeight;
                    hourCol.scrollTop = Math.max(0, hour() * height - hourCol.clientHeight / 2 + height / 2);
                }
                const minuteButtons = minCol?.querySelectorAll("button") ?? [];
                const index = minutes.indexOf(minuteSnapped());
                if (minCol && minuteButtons.length && index >= 0) {
                    const height = (minuteButtons[0] as HTMLElement).offsetHeight;
                    minCol.scrollTop = Math.max(0, index * height - minCol.clientHeight / 2 + height / 2);
                }
            });
            return () => cancelAnimationFrame(frame);
        },
    );
    onSettled(() => {
        function outside(event: PointerEvent) {
            if (open() && !trigger.contains(event.target as Node) && !content?.contains(event.target as Node)) {
                setOpen(false);
            }
        }
        function keydown(event: KeyboardEvent) {
            if (open() && event.key === "Escape") {
                event.preventDefault();
                setOpen(false);
                trigger.focus();
            }
        }
        document.addEventListener("pointerdown", outside);
        document.addEventListener("keydown", keydown);
        return () => {
            document.removeEventListener("pointerdown", outside);
            document.removeEventListener("keydown", keydown);
        };
    });

    return (
        <>
            <Button ref={trigger} variant="outline"
                class="h-10 w-full justify-start gap-2 rounded-sm border-border bg-card px-3 font-mono font-normal tabular-nums text-foreground hover:bg-muted/35"
                aria-label={props.ariaLabel} aria-expanded={open() ? "true" : "false"}
                disabled={props.disabled} onClick={toggle}>
                <Clock class="w-4 h-4 text-muted-foreground/60 shrink-0" />
                <span>{props.value.slice(0,5)}</span>
            </Button>
            <Show when={open()}>
                <Portal>
                    <div ref={content} role="dialog" aria-label={props.ariaLabel ?? "Time"}
                        class="fixed z-[90] w-36 overflow-hidden rounded-md border border-border bg-card p-0 shadow-none"
                        style={{left: `${position().left}px`, top: `${position().top}px`}}>
                        <div class="flex" style={{height: "200px"}}>
                            <div ref={hourCol} class="flex-1 overflow-y-auto flex flex-col no-scrollbar">
                                <For each={hours}>{item =>
                                    <button type="button"
                                        class={cn("py-2 text-sm font-mono text-center transition-colors shrink-0",
                                            item === hour() ? "bg-foreground text-background font-semibold"
                                                : "text-muted-foreground hover:bg-muted hover:text-foreground")}
                                        onClick={() => selectHour(item)}>{pad(item)}</button>
                                }</For>
                            </div>
                            <div class="w-px bg-border/60 shrink-0" />
                            <div ref={minCol} class="flex-1 overflow-y-auto flex flex-col no-scrollbar">
                                <For each={minutes}>{item =>
                                    <button type="button"
                                        class={cn("py-2 text-sm font-mono text-center transition-colors shrink-0",
                                            item === minuteSnapped() ? "bg-foreground text-background font-semibold"
                                                : "text-muted-foreground hover:bg-muted hover:text-foreground")}
                                        onClick={() => selectMinute(item)}>{pad(item)}</button>
                                }</For>
                            </div>
                        </div>
                    </div>
                </Portal>
            </Show>
            <style>{`
                .no-scrollbar { scrollbar-width: none; -ms-overflow-style:none; }
                .no-scrollbar::-webkit-scrollbar { display:none; }
            `}</style>
        </>
    );
}
