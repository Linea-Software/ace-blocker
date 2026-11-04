<script lang="ts">
    import { page } from "$app/state";
    import { resolve } from "$app/paths";
    import * as m from "$lib/paraglide/messages.js";
    import {
        Gauge,
        ShieldCheck,
        SlidersHorizontal,
        Library,
        BookOpen,
        CircleHelp,
        Settings,
    } from "@lucide/svelte";

    const primaryItems = [
        { href: "/", label: m.nav_control, icon: Gauge, tour: undefined },
        { href: "/network-blocks", label: m.nav_blocks, icon: ShieldCheck, tour: "network-blocks" },
        { href: "/resistance-mode", label: m.nav_resistance, icon: SlidersHorizontal, tour: "resistance-mode" },
        { href: "/site-packs", label: m.nav_packs, icon: Library, tour: "site-packs" },
    ] as const;

    const secondaryItems = [
        { href: "/getting-started", label: m.nav_guide, icon: BookOpen, tour: "getting-started" },
        { href: "/help", label: m.nav_help, icon: CircleHelp, tour: undefined },
        { href: "/settings", label: m.settings, icon: Settings, tour: "settings" },
    ] as const;

    function isActive(href: string): boolean {
        const pathname = page.url.pathname;
        return href === "/" ? pathname === "/" : pathname.startsWith(href);
    }
</script>

<nav class="rail" aria-label={m.navigation()}>
    <a class="brand" href={resolve("/")} aria-label={m.app_name()} title={m.app_name()}>
        <img src="/ace-icon-light-transparent.svg" alt="" class="brand-icon dark:hidden" />
        <img src="/ace-icon-dark-transparent.svg" alt="" class="brand-icon hidden dark:block" />
    </a>

    <div class="rail-group">
        {#each primaryItems as item (item.href)}
            <a
                class:active={isActive(item.href)}
                class="rail-link"
                href={resolve(item.href)}
                aria-current={isActive(item.href) ? "page" : undefined}
                title={item.label()}
                data-tour={item.tour}
            >
                <item.icon size={20} strokeWidth={1.8} aria-hidden="true" />
                <span>{item.label()}</span>
            </a>
        {/each}
    </div>

    <div class="rail-group rail-group-secondary">
        {#each secondaryItems as item (item.href)}
            <a
                class:active={isActive(item.href)}
                class="rail-link"
                href={resolve(item.href)}
                aria-current={isActive(item.href) ? "page" : undefined}
                title={item.label()}
                data-tour={item.tour}
            >
                <item.icon size={20} strokeWidth={1.8} aria-hidden="true" />
                <span>{item.label()}</span>
            </a>
        {/each}
    </div>
</nav>

<style>
    .rail {
        width: 72px;
        height: 100dvh;
        flex: 0 0 72px;
        display: flex;
        flex-direction: column;
        align-items: stretch;
        border-right: 1px solid var(--border);
        background: var(--sidebar);
        color: var(--sidebar-foreground);
        overflow-x: hidden;
        overflow-y: auto;
    }

    .brand {
        height: 62px;
        flex-shrink: 0;
        display: grid;
        place-items: center;
        border-bottom: 1px solid var(--sidebar-border);
        color: inherit;
        text-decoration: none;
    }

    .brand-icon {
        width: 46px;
        height: 46px;
        object-fit: contain;
    }

    .rail-group {
        flex-shrink: 0;
        display: flex;
        flex-direction: column;
        gap: 2px;
        padding: 8px 6px;
    }

    .rail-group-secondary {
        margin-top: auto;
        border-top: 1px solid var(--sidebar-border);
    }

    .rail-link {
        position: relative;
        min-height: 55px;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        gap: 4px;
        border-radius: 4px;
        color: color-mix(in srgb, var(--sidebar-foreground) 68%, transparent);
        text-decoration: none;
        transition:
            background 150ms ease,
            color 150ms ease;
    }

    .rail-link::before {
        content: "";
        position: absolute;
        inset: 7px auto 7px -6px;
        width: 3px;
        background: transparent;
        transition: background 150ms ease;
    }

    .rail-link span {
        max-width: 60px;
        overflow: hidden;
        text-overflow: ellipsis;
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 12px;
        font-weight: 600;
        line-height: 1;
        white-space: nowrap;
    }

    .rail-link:hover {
        background: var(--sidebar-accent);
        color: var(--sidebar-accent-foreground);
    }

    .rail-link.active {
        background: var(--sidebar-accent);
        color: var(--sidebar-accent-foreground);
    }

    .rail-link.active::before {
        background: var(--ace-signal);
    }

    @media (max-width: 720px) {
        .rail {
            width: 62px;
            flex-basis: 62px;
        }

        .rail-link span {
            font-size: 11px;
        }
    }
</style>
