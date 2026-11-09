<script lang="ts">
    import { Label } from "$lib/components/ui/label/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import { Switch } from "$lib/components/ui/switch/index.js";
    import * as m from "$lib/paraglide/messages.js";
    import { Globe } from "@lucide/svelte";
    import IconPicker from "$lib/components/IconPicker.svelte";

    let {
        name = $bindable(),
        icon = $bindable(null),
        isSafeSearch = $bindable(false),
        disabled = false,
        isLocked = false,
    }: {
        name: string;
        icon: string | null;
        isSafeSearch: boolean;
        disabled?: boolean;
        isLocked?: boolean;
    } = $props();

    let showIconPicker = $state(false);

    // Stage changes in the editor. Only the parent Save action writes to the engine.
    function handleSafeSearchChange(checked: boolean) {
        if (!isLocked && !disabled) isSafeSearch = checked;
    }
</script>

<div class="space-y-4" id="general">
    <div class="grid gap-2">
        <Label for="name" class="text-sm font-semibold">{m.block_name()}</Label>
        <Input
            id="name"
            bind:value={name}
            placeholder={m.block_name_placeholder()}
            disabled={disabled || isLocked}
            class="h-10 rounded-sm bg-card"
            autocomplete="off"
        />
        {#if isLocked}
            <p class="text-muted-foreground text-xs">
                {m.name_cannot_change_locked()}
            </p>
        {/if}
    </div>

    <details class="border-border group border-t pt-3">
        <summary
            class="text-muted-foreground hover:text-foreground cursor-pointer select-none text-sm font-medium transition-colors"
        >
            {m.more_options()}
        </summary>

        <div class="border-border mt-4 space-y-4 border-l pl-4">
            <div class="grid gap-2">
                <Label class="text-sm font-medium">{m.pack_icon()}</Label>
                <div class="flex items-center gap-3">
                    <IconPicker
                        bind:selectedIcon={icon}
                        bind:open={showIconPicker}
                        fallbackIcon={Globe}
                        size="sm"
                    />
                    <span class="text-muted-foreground min-w-0 truncate text-sm">
                        {icon || m.choose_icon()}
                    </span>
                </div>
            </div>

            <div
                class="border-border flex items-start justify-between gap-4 border-t pt-4"
            >
                <div class="min-w-0">
                    <Label class="text-sm font-medium">{m.force_safe_search()}</Label>
                    <p class="text-muted-foreground mt-0.5 text-xs leading-relaxed">
                        {m.force_safe_search_description()}
                    </p>
                </div>
                <Switch
                    checked={isSafeSearch}
                    onCheckedChange={handleSafeSearchChange}
                    disabled={disabled || isLocked}
                    aria-label={m.force_safe_search()}
                />
            </div>
        </div>
    </details>
</div>
