<script lang="ts">
    import { LockKeyhole } from "@lucide/svelte";

    let {
        title,
        description,
        detail,
        active = true,
        emphasis = "signal",
        compact = false,
    }: {
        title: string;
        description: string;
        detail?: string | null;
        active?: boolean;
        emphasis?: "signal" | "danger";
        compact?: boolean;
    } = $props();
</script>

<section
    class:active
    class:danger={emphasis === "danger"}
    class:compact
    class="lock-seal"
    aria-label={title}
>
    <span class="seal-rail" aria-hidden="true"></span>
    <div class="seal-mark" aria-hidden="true">
        <LockKeyhole size={18} strokeWidth={2} />
    </div>
    <div class="seal-copy">
        <strong>{title}</strong>
        <p>{description}</p>
    </div>
    {#if detail}
        <span class="seal-detail">{detail}</span>
    {/if}
</section>

<style>
    .lock-seal {
        position: relative;
        display: grid;
        grid-template-columns: 6px 40px minmax(0, 1fr) auto;
        align-items: center;
        gap: 12px;
        min-height: 72px;
        overflow: hidden;
        border: 1px solid var(--border);
        border-radius: 5px;
        background: color-mix(in srgb, var(--muted) 30%, var(--card));
    }

    .seal-rail {
        align-self: stretch;
        background: var(--ace-ash);
    }

    .lock-seal.active {
        border-top-color: var(--ace-signal-deep);
        border-bottom-color: var(--ace-signal-deep);
    }

    .lock-seal.active .seal-rail {
        background: var(--ace-signal);
    }

    .lock-seal.danger.active {
        border-top-color: color-mix(in srgb, var(--ace-danger) 58%, var(--border));
        border-bottom-color: color-mix(in srgb, var(--ace-danger) 58%, var(--border));
    }

    .lock-seal.danger.active .seal-rail {
        background: var(--ace-danger);
    }

    .seal-mark {
        width: 36px;
        height: 36px;
        display: grid;
        place-items: center;
        border: 1px solid var(--border);
        border-radius: 4px;
        background: var(--background);
        color: var(--muted-foreground);
    }

    .active .seal-mark {
        border-color: color-mix(in srgb, var(--ace-signal-deep) 40%, var(--border));
        color: var(--ace-signal-deep);
    }

    .danger.active .seal-mark {
        border-color: color-mix(in srgb, var(--ace-danger) 45%, var(--border));
        color: var(--ace-danger);
    }

    .seal-copy {
        min-width: 0;
        padding-block: 12px;
    }

    .seal-copy strong {
        display: block;
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 15px;
        font-weight: 600;
        line-height: 1.1;
    }

    .seal-copy p {
        margin: 4px 0 0;
        color: var(--muted-foreground);
        font-size: 11px;
        line-height: 1.45;
    }

    .seal-detail {
        margin-right: 14px;
        color: var(--muted-foreground);
        font-family: "IBM Plex Mono", monospace;
        font-size: 11px;
        white-space: nowrap;
    }

    .lock-seal.compact {
        grid-template-columns: 28px minmax(0, 1fr) auto;
        gap: 8px;
        min-height: 36px;
        border-inline: 0;
        border-top: 0;
        border-bottom-color: color-mix(in srgb, var(--ace-signal-deep) 42%, var(--border));
        border-radius: 0;
        background: color-mix(in srgb, var(--ace-signal) 8%, var(--card));
        box-shadow: none;
    }

    .compact .seal-rail {
        display: none;
    }

    .compact .seal-mark {
        width: 26px;
        height: 26px;
        border: 0;
        background: transparent;
    }

    .compact .seal-copy {
        padding-block: 6px;
    }

    .compact .seal-copy strong {
        font-size: 12px;
    }

    .compact .seal-copy p {
        margin-top: 1px;
        font-size: 11px;
    }

    .compact .seal-detail {
        margin-right: 14px;
        font-size: 11px;
    }

    @media (max-width: 620px) {
        .lock-seal {
            grid-template-columns: 6px 36px minmax(0, 1fr);
        }

        .seal-detail {
            grid-column: 3;
            margin: -4px 0 12px;
            white-space: normal;
        }
    }

    @container page (max-width: 560px) {
        .lock-seal.compact {
            grid-template-columns: 28px minmax(0, 1fr);
        }

        .compact .seal-detail {
            grid-column: 2;
            margin: -3px 0 8px;
            white-space: normal;
        }
    }
</style>
