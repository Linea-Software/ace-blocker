<script lang="ts">
    import type { PageData } from "./$types";
    import { goto, invalidateAll } from "$app/navigation";
    import { resolve } from "$app/paths";
    import BlockEditor from "../_BlockEditor.svelte";
    import LockModal from "../_components/_LockModal.svelte";
    import * as m from "$lib/paraglide/messages.js";

    let { data }: { data: PageData } = $props();
    let lockOpen = $state(false);
    let editorStep = $state<0 | 1 | 2>(0);
    const returnToList = () => goto(resolve("/network-blocks"));
    // Deep links from the Network Blocks list go directly to the requested section.
    $effect(() => {
        if (typeof window !== "undefined" &&
            new URLSearchParams(window.location.search).get("section") === "schedule") editorStep = 1;
    });

    function handleLock() {
        if (data.block) lockOpen = true;
    }
</script>

<svelte:head>
    <title>{data.block ? m.edit_block() : m.create_block()} · {m.app_name()}</title>
</svelte:head>

{#if data.engineOffline || data.loadError || (data.requestedId && !data.block)}
    <main class="editor-error" role="alert">
        <h1>{m.network_blocks_load_failed_title()}</h1>
        <p>{m.network_blocks_load_failed_description()}</p>
        <a href={resolve("/network-blocks")}>{m.cancel()}</a>
    </main>
{:else}
    <BlockEditor
        block={data.block}
        bind:step={editorStep}
        onClose={returnToList}
        onProtect={handleLock}
        onSave={() => { void returnToList(); }}
    />
{/if}

<LockModal
    bind:open={lockOpen}
    block={data.block}
    onSave={() => invalidateAll()}
    onEditSchedule={() => {
        lockOpen = false;
        editorStep = 1;
    }}
/>

<style>
    .editor-error {
        max-width: 660px;
        margin: 50px auto;
        padding: 26px;
        border: 1px solid var(--border);
        background: var(--card);
    }
    .editor-error h1 {
        font: 600 26px "IBM Plex Sans Condensed", sans-serif;
    }
    .editor-error p { margin: 12px 0; color: var(--muted-foreground); }
    .editor-error a { text-decoration: underline; text-underline-offset: 3px; }
</style>
