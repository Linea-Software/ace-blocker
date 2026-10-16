<script lang="ts">
    import * as m from "$lib/paraglide/messages.js";
    import EngineState from "$lib/components/ace/domain/EngineState.svelte";
</script>

<header class="status-strip">
    <p>{m.hold_the_line()}</p>
    <EngineState />
</header>

<style>
    .status-strip {
        min-height: 36px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 18px;
        padding: 0 22px;
        border-bottom: 1px solid var(--border);
        background: color-mix(in srgb, var(--background) 82%, var(--card));
    }

    p {
        margin: 0;
        color: var(--muted-foreground);
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 12px;
        font-weight: 500;
    }

    @media (max-width: 860px) {
        .status-strip {
            justify-content: flex-end;
            padding-inline: 14px;
        }

        p {
            display: none;
        }
    }
</style>
