<script lang="ts">
    import { Label } from "$lib/components/ui/label/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import * as m from "$lib/paraglide/messages.js";
    import { Globe } from "@lucide/svelte";
    import IconPicker from "$lib/components/IconPicker.svelte";

    let {
        name = $bindable(),
        icon = $bindable(null),
        disabled = false,
        isLocked = false,
    }: {
        name: string;
        icon: string | null;
        disabled?: boolean;
        isLocked?: boolean;
    } = $props();

    let showIconPicker = $state(false);
</script>

<div class="space-y-2" id="general">
    <Label for="name" class="text-sm font-semibold">{m.block_name()}</Label>
    <div class="flex items-center gap-3">
        <IconPicker
            bind:selectedIcon={icon}
            bind:open={showIconPicker}
            fallbackIcon={Globe}
            label={m.pack_icon()}
            size="lg"
            {disabled}
        />
        <Input
            id="name"
            bind:value={name}
            placeholder={m.block_name_placeholder()}
            disabled={disabled || isLocked}
            class="h-11 min-w-0 flex-1 rounded-sm bg-card"
            autocomplete="off"
        />
    </div>
    {#if isLocked}
        <p class="text-muted-foreground text-xs">
            {m.name_cannot_change_locked()}
        </p>
    {/if}
</div>
