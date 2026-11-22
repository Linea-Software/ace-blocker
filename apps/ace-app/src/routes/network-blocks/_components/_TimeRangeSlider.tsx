import { createSignal, For } from "solid-js";
import * as m from "$lib/paraglide/messages.js";

export interface TimeRangeSliderProps {
    startMinutes: number;
    endMinutes: number;
    onStartMinutesChange: (minutes: number) => void;
    onEndMinutesChange: (minutes: number) => void;
}

const TRACK_MINUTES = 24 * 60 - 1;
const SNAP_MINUTES = 15;
const MIN_RANGE = 15;

function formatTime(minutes: number): string {
    const hour = Math.floor(minutes / 60);
    const minute = minutes % 60;
    return `${String(hour).padStart(2, "0")}:${String(minute).padStart(2, "0")}`;
}

export default function TimeRangeSlider(props: TimeRangeSliderProps) {
    let trackEl: HTMLDivElement | undefined;
    const [dragging, setDragging] = createSignal<"start" | "end" | null>(null);
    const startPct = () => (props.startMinutes / TRACK_MINUTES) * 100;
    const endPct = () => (props.endMinutes / TRACK_MINUTES) * 100;

    function clientXToMinutes(clientX: number): number {
        if (!trackEl) return 0;
        const rect = trackEl.getBoundingClientRect();
        const ratio = (clientX - rect.left) / rect.width;
        const snapped = Math.round((ratio * TRACK_MINUTES) / SNAP_MINUTES) * SNAP_MINUTES;
        return Math.max(0, Math.min(TRACK_MINUTES, snapped));
    }

    function onPointerDown(event: PointerEvent, which: "start" | "end") {
        event.preventDefault();
        (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
        setDragging(which);
    }

    function onPointerMove(event: PointerEvent) {
        const active = dragging();
        if (!active) return;
        const minutes = clientXToMinutes(event.clientX);
        if (active === "start") {
            props.onStartMinutesChange(Math.max(0, Math.min(minutes, props.endMinutes - MIN_RANGE)));
        } else {
            props.onEndMinutesChange(Math.min(TRACK_MINUTES, Math.max(minutes, props.startMinutes + MIN_RANGE)));
        }
    }

    function onHandleKeydown(event: KeyboardEvent, which: "start" | "end") {
        if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
        event.preventDefault();
        const step = event.key === "ArrowRight" ? SNAP_MINUTES : -SNAP_MINUTES;
        if (which === "start") {
            props.onStartMinutesChange(Math.max(0, Math.min(props.startMinutes + step, props.endMinutes - MIN_RANGE)));
        } else {
            props.onEndMinutesChange(Math.min(TRACK_MINUTES, Math.max(props.endMinutes + step, props.startMinutes + MIN_RANGE)));
        }
    }

    return (
        <div class="space-y-2">
            <div ref={trackEl} class="relative h-6 flex items-center cursor-pointer select-none touch-none">
                <div class="absolute left-0 right-0 rounded-sm" style={{ height: "6px", background: "#e8e0d4" }} />
                <div class="absolute rounded-sm"
                    style={{ left: `${startPct()}%`, width: `${endPct() - startPct()}%`, height: "6px", background: "#f59e0b" }} />
                <div class="absolute rounded-full cursor-grab active:cursor-grabbing z-10"
                    style={{ left: `calc(${startPct()}% - 10px)`, width: "20px", height: "20px", background: "#f59e0b", border: "2px solid #ffffff", "box-shadow": "0 1px 3px rgba(0,0,0,0.12)" }}
                    role="slider" tabindex={0} aria-label={m.start_time()}
                    aria-valuemin={0} aria-valuemax={TRACK_MINUTES}
                    aria-valuenow={props.startMinutes} aria-valuetext={formatTime(props.startMinutes)}
                    onPointerDown={event => onPointerDown(event, "start")}
                    onPointerMove={onPointerMove} onPointerUp={() => setDragging(null)}
                    onPointerCancel={() => setDragging(null)}
                    onKeyDown={event => onHandleKeydown(event, "start")} />
                <div class="absolute rounded-full cursor-grab active:cursor-grabbing z-10"
                    style={{ left: `calc(${endPct()}% - 10px)`, width: "20px", height: "20px", background: "#f59e0b", border: "2px solid #ffffff", "box-shadow": "0 1px 3px rgba(0,0,0,0.12)" }}
                    role="slider" tabindex={0} aria-label={m.end_time()}
                    aria-valuemin={0} aria-valuemax={TRACK_MINUTES}
                    aria-valuenow={props.endMinutes} aria-valuetext={formatTime(props.endMinutes)}
                    onPointerDown={event => onPointerDown(event, "end")}
                    onPointerMove={onPointerMove} onPointerUp={() => setDragging(null)}
                    onPointerCancel={() => setDragging(null)}
                    onKeyDown={event => onHandleKeydown(event, "end")} />
            </div>
            <div class="flex justify-between px-0.5">
                <For each={[0, 6, 12, 18, 24]}>{hour =>
                    <span class="text-xs font-medium text-muted-foreground/60">{String(hour).padStart(2, "0")}</span>
                }</For>
            </div>
            <div class="flex justify-between px-0.5">
                <span class="text-xs font-semibold font-mono tabular-nums">{formatTime(props.startMinutes)}</span>
                <span class="text-xs font-semibold font-mono tabular-nums">{formatTime(props.endMinutes)}</span>
            </div>
        </div>
    );
}
