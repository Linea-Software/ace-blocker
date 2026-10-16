<script lang="ts">
    import * as Popover from "$lib/components/ui/popover/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import ClockIcon from "@lucide/svelte/icons/clock";
    import { cn } from "$lib/utils";

    let {
        value = $bindable("09:00"),
    }: {
        value: string;
    } = $props();

    let open = $state(false);
    let hourCol = $state<HTMLDivElement | null>(null);
    let minCol = $state<HTMLDivElement | null>(null);

    const hours = Array.from({ length: 24 }, (_, i) => i);
    const minutes = Array.from({ length: 12 }, (_, i) => i * 5);

    const hour = $derived(parseInt(value.split(":")[0]!, 10));
    const minute = $derived(parseInt(value.split(":")[1]!, 10));
    const minuteSnapped = $derived(Math.min(55, Math.round(minute / 5) * 5));

    function pad(n: number): string {
        return String(n).padStart(2, "0");
    }

    function selectHour(h: number) {
        value = `${pad(h)}:${pad(minuteSnapped)}`;
    }

    function selectMinute(m: number) {
        value = `${pad(hour)}:${pad(m)}`;
    }

    // Use rAF so the portal has time to paint before we scroll
    $effect(() => {
        if (open) {
            const handle = requestAnimationFrame(() => {
                if (hourCol) {
                    const btns = hourCol.querySelectorAll("button");
                    if (btns.length > 0) {
                        const itemH = (btns[0] as HTMLElement).offsetHeight;
                        hourCol.scrollTop = Math.max(
                            0,
                            hour * itemH - hourCol.clientHeight / 2 + itemH / 2,
                        );
                    }
                }
                if (minCol) {
                    const btns = minCol.querySelectorAll("button");
                    const idx = minutes.indexOf(minuteSnapped);
                    if (btns.length > 0 && idx >= 0) {
                        const itemH = (btns[0] as HTMLElement).offsetHeight;
                        minCol.scrollTop = Math.max(
                            0,
                            idx * itemH - minCol.clientHeight / 2 + itemH / 2,
                        );
                    }
                }
            });
            return () => cancelAnimationFrame(handle);
        }
    });
</script>

<Popover.Root bind:open>
    <Popover.Trigger>
        {#snippet child({ props })}
            <Button
                variant="outline"
                class="h-10 w-full justify-start gap-2 rounded-sm border-border bg-card px-3 font-mono font-normal tabular-nums text-foreground hover:bg-muted/35"
                {...props}
            >
                <ClockIcon class="w-4 h-4 text-muted-foreground/60 shrink-0" />
                <span>{value.slice(0, 5)}</span>
            </Button>
        {/snippet}
    </Popover.Trigger>

    <Popover.Content
        class="w-36 p-0 overflow-hidden"
        align="start"
        sideOffset={6}
    >
        <div class="flex" style="height: 200px;">
            <!-- Hours 00–23 -->
            <div
                bind:this={hourCol}
                class="flex-1 overflow-y-auto flex flex-col no-scrollbar"
            >
                {#each hours as h (h)}
                    <button
                        type="button"
                        class={cn(
                            "py-2 text-sm font-mono text-center transition-colors shrink-0",
                            h === hour
                                ? "bg-foreground text-background font-semibold"
                                : "text-muted-foreground hover:bg-muted hover:text-foreground",
                        )}
                        onclick={() => selectHour(h)}
                    >
                        {pad(h)}
                    </button>
                {/each}
            </div>
            <div class="w-px bg-border/60 shrink-0"></div>
            <!-- Minutes 00, 05, … 55 -->
            <div
                bind:this={minCol}
                class="flex-1 overflow-y-auto flex flex-col no-scrollbar"
            >
                {#each minutes as min (min)}
                    <button
                        type="button"
                        class={cn(
                            "py-2 text-sm font-mono text-center transition-colors shrink-0",
                            min === minuteSnapped
                                ? "bg-foreground text-background font-semibold"
                                : "text-muted-foreground hover:bg-muted hover:text-foreground",
                        )}
                        onclick={() => selectMinute(min)}
                    >
                        {pad(min)}
                    </button>
                {/each}
            </div>
        </div>
    </Popover.Content>
</Popover.Root>

<style>
    .no-scrollbar {
        scrollbar-width: none;
        -ms-overflow-style: none;
    }
    .no-scrollbar::-webkit-scrollbar {
        display: none;
    }
</style>
