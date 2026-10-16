<script lang="ts">
    import * as Popover from "$lib/components/ui/popover/index.js";
    import { ScrollArea } from "$lib/components/ui/scroll-area/index.js";
    import { Clock } from "@lucide/svelte";
    import { cn } from "$lib/utils.js";

    let {
        value = $bindable("09:00"),
        disabled = false,
        class: className = "",
    }: {
        value?: string;
        disabled?: boolean;
        class?: string;
    } = $props();

    let open = $state(false);
    let inputValue = $state(value);

    // Sync external value changes to input value
    $effect(() => {
        if (value && value !== inputValue && /^\d{2}:\d{2}$/.test(value)) {
            inputValue = value;
        }
    });

    const hours = Array.from({ length: 24 }, (_, i) =>
        i.toString().padStart(2, "0"),
    );
    const minutes = Array.from({ length: 60 }, (_, i) =>
        i.toString().padStart(2, "0"),
    );

    let selectedHour = $derived(value?.split(":")[0] || "09");
    let selectedMinute = $derived(value?.split(":")[1] || "00");

    function setTime(h: string, m: string) {
        const newVal = `${h}:${m}`;
        value = newVal;
        inputValue = newVal;
    }

    function handleInput(e: Event) {
        const target = e.target as HTMLInputElement;
        let val = target.value.replace(/\D/g, ""); // Remove non-digits

        if (val.length > 4) {
            val = val.slice(0, 4);
        }

        if (val.length >= 3) {
            val = `${val.slice(0, 2)}:${val.slice(2)}`;
        }

        // Validate hours and minutes when fully typed
        if (val.length === 5) {
            let [h, m] = val.split(":");
            if (parseInt(h) > 23) h = "23";
            if (parseInt(m) > 59) m = "59";
            val = `${h}:${m}`;
            value = val;
        }

        inputValue = val;
        target.value = val; // Force update input in case bindings lag
    }

    function handleBlur(e: Event) {
        // Format properly on blur if incomplete
        if (inputValue.length > 0 && inputValue.length < 5) {
            let val = inputValue.replace(/\D/g, "").padEnd(4, "0");
            let h = val.slice(0, 2);
            let m = val.slice(2, 4);
            if (parseInt(h) > 23) h = "23";
            if (parseInt(m) > 59) m = "59";

            const newVal = `${h}:${m}`;
            inputValue = newVal;
            value = newVal;
        }
    }
</script>

<Popover.Root bind:open>
    <Popover.Trigger {disabled}>
        {#snippet child({ props })}
            <div
                role="button"
                tabindex={disabled ? -1 : 0}
                class={cn(
                    "relative flex items-center w-full h-10 rounded-md bg-background/50 border border-border/50 transition-all text-sm font-mono overflow-hidden cursor-text",
                    disabled
                        ? "opacity-50 cursor-not-allowed"
                        : "focus-within:ring-1 focus-within:ring-ring focus-within:border-ring hover:border-border",
                    className,
                )}
                onkeydown={(e) => {
                    if (disabled) return;
                    if (e.key === "Enter" || e.key === " ") {
                        e.preventDefault();
                        const target = e.currentTarget.querySelector("input");
                        if (target && document.activeElement !== target) {
                            target.focus();
                        }
                    }
                }}
                onclick={(e) => {
                    if (disabled) return;
                    // Focus the input when clicking the wrapper
                    const target = e.currentTarget.querySelector("input");
                    if (target && document.activeElement !== target) {
                        target.focus();
                    }
                }}
            >
                <div
                    class="pl-3 flex items-center justify-center shrink-0 pointer-events-none"
                >
                    <Clock class="w-4 h-4 text-muted-foreground" />
                </div>
                <input
                    {...props}
                    type="text"
                    value={inputValue}
                    oninput={handleInput}
                    onblur={(e) => {
                        handleBlur(e);
                        // Forward blur to popover trigger if needed
                        if (typeof props.onblur === "function") props.onblur(e);
                    }}
                    class="flex-1 h-full bg-transparent px-3 outline-none tabular-nums text-foreground placeholder-muted-foreground disabled:cursor-not-allowed"
                    placeholder="00:00"
                    autocomplete="off"
                    maxlength="5"
                    {disabled}
                />
            </div>
        {/snippet}
    </Popover.Trigger>
    <Popover.Content
        class="w-auto p-0 bg-card border-border/50 shadow-2xl"
        align="start"
        sideOffset={4}
    >
        <div class="flex h-[240px] divide-x divide-border/20">
            <!-- Hours -->
            <ScrollArea class="w-[72px]">
                <div class="flex flex-col p-1 gap-1">
                    {#each hours as h}
                        <button
                            class={cn(
                                "py-2 px-1 text-center text-sm font-mono rounded-md transition-colors",
                                h === selectedHour
                                    ? "bg-secondary text-secondary-foreground font-bold"
                                    : "text-muted-foreground hover:bg-muted hover:text-foreground",
                            )}
                            onclick={() => setTime(h, selectedMinute)}
                        >
                            {h}
                        </button>
                    {/each}
                </div>
            </ScrollArea>
            <!-- Minutes -->
            <ScrollArea class="w-[72px]">
                <div class="flex flex-col p-1 gap-1">
                    {#each minutes as m}
                        <button
                            class={cn(
                                "py-2 px-1 text-center text-sm font-mono rounded-md transition-colors",
                                m === selectedMinute
                                    ? "bg-secondary text-secondary-foreground font-bold"
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
