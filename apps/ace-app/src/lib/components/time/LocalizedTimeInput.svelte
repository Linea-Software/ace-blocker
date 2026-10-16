<script lang="ts">
    import TimeInput12h from "./TimeInput12h.svelte";
    import TimeInput24h from "./TimeInput24h.svelte";
    import { onMount } from "svelte";
    import { localizationStore } from "$lib/stores/localization.svelte";

    let {
        value = $bindable("09:00"),
        disabled = false,
        className = ""
    }: {
        value: string;
        disabled?: boolean;
        className?: string;
    } = $props();

    onMount(() => {
        localizationStore.detectSystemTimeFormat();
    });

    const is12h = $derived(localizationStore.uses12h);
</script>

{#if is12h}
    <TimeInput12h bind:value {disabled} {className} />
{:else}
    <TimeInput24h bind:value {disabled} {className} />
{/if}
