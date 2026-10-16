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

    const hours = Array.from({ length: 24 }, (_, i) =>
        i.toString().padStart(2, "0"),
    );
    const minutes = Array.from({ length: 60 }, (_, i) =>
        i.toString().padStart(2, "0"),
    );

    let selectedHour = $derived(value?.split(":")[0] || "09");
    let selectedMinute = $derived(value?.split(":")[1] || "00");

    function setTime(h: string, m: string) {
        value = `${h}:${m}`;
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
                <span>{selectedHour}:{selectedMinute}</span>
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
                    {#each hours as h}
                        <button
                            class={cn(
                                "py-2 px-1 text-center text-sm font-mono rounded-md transition-colors",
                                h === selectedHour
                                    ? "bg-primary text-primary-foreground font-bold"
                                    : "text-muted-foreground hover:bg-muted hover:text-foreground",
                            )}
                            onclick={() => setTime(h, selectedMinute)}
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
                                m === selectedMinute
                                    ? "bg-primary text-primary-foreground font-bold"
                                    : "text-muted-foreground hover:bg-muted hover:text-foreground",
                            )}
                            onclick={() => setTime(selectedHour, m)}
                        >
                            {m}
                        </button>
                    {/each}
                </div>
            </ScrollArea>
        </div>
    </Popover.Content>
</Popover.Root>
