<script lang="ts">
    import * as Popover from "$lib/components/ui/popover/index.js";
    import { ScrollArea } from "$lib/components/ui/scroll-area/index.js";
    import { Clock } from "@lucide/svelte";
    import { cn } from "$lib/utils.js";

    let {
        value = $bindable("09:00"),
        disabled = false,
        className = "",
    }: {
        value?: string;
        disabled?: boolean;
        className?: string;
    } = $props();

    let open = $state(false);

    // Convert HH:mm (24h) to [h, m, period]
    const parsed = $derived.by(() => {
        const [h24, m] = (value || "09:00").split(":").map(Number);
        const period = h24 >= 12 ? "PM" : "AM";
        let h12 = h24 % 12;
        if (h12 === 0) h12 = 12;
        return {
            h12: h12.toString().padStart(2, "0"),
            m: m.toString().padStart(2, "0"),
            period,
        };
    });

    const hours12 = Array.from({ length: 12 }, (_, i) =>
        (i + 1).toString().padStart(2, "0"),
    );
    const minutes = Array.from({ length: 60 }, (_, i) =>
        i.toString().padStart(2, "0"),
    );
    const periods = ["AM", "PM"];

    function setTime(h12: string, m: string, period: string) {
        let h24 = parseInt(h12);
        if (period === "PM" && h24 !== 12) h24 += 12;
        if (period === "AM" && h24 === 12) h24 = 0;
        value = `${h24.toString().padStart(2, "0")}:${m}`;
    }
</script>

<Popover.Root bind:open>
    <Popover.Trigger {disabled}>
        {#snippet child({ props })}
            <button
                {...props}
                class={cn(
                    "relative flex items-center w-full h-10 px-3 rounded-xl bg-background/50 border border-border/50 transition-all text-sm font-mono tabular-nums overflow-hidden",
                    disabled
                        ? "opacity-50 cursor-not-allowed"
                        : "hover:border-primary/30",
                    className,
                )}
            >
                <Clock class="w-4 h-4 text-muted-foreground mr-3" />
                <span>{parsed.h12}:{parsed.m} {parsed.period}</span>
            </button>
        {/snippet}
    </Popover.Trigger>
    <Popover.Content
        class="w-auto p-0 bg-card border-border/50 shadow-2xl"
        align="start"
    >
        <div class="flex h-[240px] divide-x divide-border/20">
            <ScrollArea class="w-[64px]">
                <div class="flex flex-col p-1 gap-1">
                    {#each hours12 as h}
                        <button
                            class={cn(
                                "py-2 px-1 text-center text-sm font-mono rounded-md transition-colors",
                                h === parsed.h12
                                    ? "bg-primary text-primary-foreground font-bold"
                                    : "text-muted-foreground hover:bg-muted hover:text-foreground",
                            )}
                            onclick={() => setTime(h, parsed.m, parsed.period)}
                        >
                            {h}
                        </button>
                    {/each}
                </div>
            </ScrollArea>
            <ScrollArea class="w-[64px]">
                <div class="flex flex-col p-1 gap-1">
                    {#each minutes as m}
                        <button
                            class={cn(
                                "py-2 px-1 text-center text-sm font-mono rounded-md transition-colors",
                                m === parsed.m
                                    ? "bg-primary text-primary-foreground font-bold"
                                    : "text-muted-foreground hover:bg-muted hover:text-foreground",
                            )}
                            onclick={() =>
                                setTime(parsed.h12, m, parsed.period)}
                        >
                            {m}
                        </button>
                    {/each}
                </div>
            </ScrollArea>
            <ScrollArea class="w-[64px]">
                <div class="flex flex-col p-1 gap-1">
                    {#each periods as p}
                        <button
                            class={cn(
                                "py-2 px-1 text-center text-xs font-bold rounded-md transition-colors",
                                p === parsed.period
                                    ? "bg-primary text-primary-foreground"
                                    : "text-muted-foreground hover:bg-muted hover:text-foreground",
                            )}
                            onclick={() => setTime(parsed.h12, parsed.m, p)}
                        >
                            {p}
                        </button>
                    {/each}
                </div>
            </ScrollArea>
        </div>
    </Popover.Content>
</Popover.Root>
