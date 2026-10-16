<script lang="ts">
    import * as m from "$lib/paraglide/messages.js";

    let {
        startMinutes = $bindable(540),
        endMinutes = $bindable(1020),
    }: {
        startMinutes: number;
        endMinutes: number;
    } = $props();

    const TRACK_MINUTES = 24 * 60 - 1; // 1439 = 23:59
    const SNAP_MINUTES = 15;
    const MIN_RANGE = 15;

    let trackEl = $state<HTMLDivElement | null>(null);
    let dragging = $state<"start" | "end" | null>(null);

    function startPct(): number {
        return (startMinutes / TRACK_MINUTES) * 100;
    }

    function endPct(): number {
        return (endMinutes / TRACK_MINUTES) * 100;
    }

    function formatTime(minutes: number): string {
        const h = Math.floor(minutes / 60);
        const m = minutes % 60;
        return `${String(h).padStart(2, "0")}:${String(m).padStart(2, "0")}`;
    }

    function clientXToMinutes(clientX: number): number {
        if (!trackEl) return 0;
        const rect = trackEl.getBoundingClientRect();
        const ratio = (clientX - rect.left) / rect.width;
        const raw = ratio * TRACK_MINUTES;
        const snapped = Math.round(raw / SNAP_MINUTES) * SNAP_MINUTES;
        return Math.max(0, Math.min(TRACK_MINUTES, snapped));
    }

    function onPointerDown(e: PointerEvent, which: "start" | "end") {
        e.preventDefault();
        (e.target as HTMLElement).setPointerCapture(e.pointerId);
        dragging = which;
    }

    function onPointerMove(e: PointerEvent) {
        if (!dragging) return;
        const mins = clientXToMinutes(e.clientX);
        if (dragging === "start") {
            startMinutes = Math.min(mins, endMinutes - MIN_RANGE);
        } else {
            endMinutes = Math.max(mins, startMinutes + MIN_RANGE);
        }
    }

    function onPointerUp() {
        dragging = null;
    }

    function onHandleKeydown(e: KeyboardEvent, which: "start" | "end") {
        if (e.key !== "ArrowLeft" && e.key !== "ArrowRight") return;
        e.preventDefault();
        const step = e.key === "ArrowRight" ? SNAP_MINUTES : -SNAP_MINUTES;
        if (which === "start") {
            startMinutes = Math.max(
                0,
                Math.min(startMinutes + step, endMinutes - MIN_RANGE),
            );
        } else {
            endMinutes = Math.min(
                TRACK_MINUTES,
                Math.max(endMinutes + step, startMinutes + MIN_RANGE),
            );
        }
    }
</script>

<div class="space-y-2">
    <!-- Track -->
    <div
        bind:this={trackEl}
        class="relative h-6 flex items-center cursor-pointer select-none touch-none"
    >
        <!-- Background track -->
        <div
            class="absolute left-0 right-0 rounded-sm"
            style="height: 6px; background: #e8e0d4;"
        ></div>

        <!-- Amber fill -->
        <div
            class="absolute rounded-sm"
            style="left: {startPct()}%; width: {endPct() -
                startPct()}%; height: 6px; background: #f59e0b;"
        ></div>

        <!-- Start handle -->
        <div
            class="absolute rounded-full cursor-grab active:cursor-grabbing z-10"
            style="left: calc({startPct()}% - 10px); width: 20px; height: 20px; background: #f59e0b; border: 2px solid #ffffff; box-shadow: 0 1px 3px rgba(0,0,0,0.12);"
            role="slider"
            tabindex="0"
            aria-label={m.start_time()}
            aria-valuemin={0}
            aria-valuemax={TRACK_MINUTES}
            aria-valuenow={startMinutes}
            aria-valuetext={formatTime(startMinutes)}
            onpointerdown={(e) => onPointerDown(e, "start")}
            onpointermove={onPointerMove}
            onpointerup={onPointerUp}
            onpointercancel={onPointerUp}
            onkeydown={(e) => onHandleKeydown(e, "start")}
        ></div>

        <!-- End handle -->
        <div
            class="absolute rounded-full cursor-grab active:cursor-grabbing z-10"
            style="left: calc({endPct()}% - 10px); width: 20px; height: 20px; background: #f59e0b; border: 2px solid #ffffff; box-shadow: 0 1px 3px rgba(0,0,0,0.12);"
            role="slider"
            tabindex="0"
            aria-label={m.end_time()}
            aria-valuemin={0}
            aria-valuemax={TRACK_MINUTES}
            aria-valuenow={endMinutes}
            aria-valuetext={formatTime(endMinutes)}
            onpointerdown={(e) => onPointerDown(e, "end")}
            onpointermove={onPointerMove}
            onpointerup={onPointerUp}
            onpointercancel={onPointerUp}
            onkeydown={(e) => onHandleKeydown(e, "end")}
        ></div>
    </div>

    <!-- Hour labels -->
    <div class="flex justify-between px-0.5">
        {#each [0, 6, 12, 18, 24] as hour}
            <span class="text-xs font-medium text-muted-foreground/60">
                {String(hour).padStart(2, "0")}
            </span>
        {/each}
    </div>

    <!-- Current times -->
    <div class="flex justify-between px-0.5">
        <span class="text-xs font-semibold font-mono tabular-nums">
            {formatTime(startMinutes)}
        </span>
        <span class="text-xs font-semibold font-mono tabular-nums">
            {formatTime(endMinutes)}
        </span>
    </div>
</div>
