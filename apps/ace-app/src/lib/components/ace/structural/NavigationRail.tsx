import { For } from "solid-js";
import { Dynamic } from "@solidjs/web";
import { useLocation } from "@solidjs/router";
import * as m from "$lib/paraglide/messages.js";
import {
    Gauge,
    ShieldCheck,
    SlidersHorizontal,
    Library,
    BookOpen,
    CircleHelp,
    Settings,
} from "@lucide/solid";

const primaryItems = [
    { href: "/", label: m.nav_control, icon: Gauge, tour: undefined },
    {
        href: "/network-blocks",
        label: m.nav_blocks,
        icon: ShieldCheck,
        tour: "network-blocks",
    },
    {
        href: "/resistance-mode",
        label: m.nav_resistance,
        icon: SlidersHorizontal,
        tour: "resistance-mode",
    },
    {
        href: "/site-packs",
        label: m.nav_packs,
        icon: Library,
        tour: "site-packs",
    },
] as const;
const secondaryItems = [
    {
        href: "/getting-started",
        label: m.nav_guide,
        icon: BookOpen,
        tour: "getting-started",
    },
    { href: "/help", label: m.nav_help, icon: CircleHelp, tour: undefined },
    {
        href: "/settings",
        label: m.nav_settings,
        icon: Settings,
        tour: "settings",
    },
] as const;
export default function NavigationRail() {
    const location = useLocation();
    const isActive = (href: string) =>
        href === "/"
            ? location.pathname === "/"
            : location.pathname.startsWith(href);
    const renderItem = (
        item: (typeof primaryItems)[number] | (typeof secondaryItems)[number],
    ) => (
        <a
            class={["rail-link", { active: isActive(item.href) }]}
            href={item.href}
            aria-current={isActive(item.href) ? "page" : undefined}
            title={item.label()}
            data-tour={item.tour}
        >
            <Dynamic
                component={item.icon}
                size={20}
                strokeWidth={1.8}
                aria-hidden="true"
            />
            <span>{item.label()}</span>
        </a>
    );
    return (
        <nav class="rail" aria-label={m.navigation()}>
            <a
                class="brand"
                href="/"
                aria-label={m.app_name()}
                title={m.app_name()}
            >
                <img
                    src="/ace-icon-light-transparent.svg"
                    alt=""
                    class="brand-icon dark:hidden"
                />
                <img
                    src="/ace-icon-dark-transparent.svg"
                    alt=""
                    class="brand-icon hidden dark:block"
                />
            </a>
            <div class="rail-group">
                <For each={primaryItems}>{renderItem}</For>
            </div>
            <div class="rail-group rail-group-secondary">
                <For each={secondaryItems}>{renderItem}</For>
            </div>
            <style>
                {
                    '\n    .rail {\n        width: 72px;\n        height: 100dvh;\n        flex: 0 0 72px;\n        display: flex;\n        flex-direction: column;\n        align-items: stretch;\n        border-right: 1px solid var(--border);\n        background: var(--sidebar);\n        color: var(--sidebar-foreground);\n        overflow-x: hidden;\n        overflow-y: auto;\n    }\n\n    .brand {\n        height: 62px;\n        flex-shrink: 0;\n        display: grid;\n        place-items: center;\n        border-bottom: 1px solid var(--sidebar-border);\n        color: inherit;\n        text-decoration: none;\n    }\n\n    .brand-icon {\n        width: 46px;\n        height: 46px;\n        object-fit: contain;\n    }\n\n    .rail-group {\n        flex-shrink: 0;\n        display: flex;\n        flex-direction: column;\n        gap: 2px;\n        padding: 8px 6px;\n    }\n\n    .rail-group-secondary {\n        margin-top: auto;\n        border-top: 1px solid var(--sidebar-border);\n    }\n\n    .rail-link {\n        position: relative;\n        min-height: 55px;\n        display: flex;\n        flex-direction: column;\n        align-items: center;\n        justify-content: center;\n        gap: 4px;\n        border-radius: 4px;\n        color: color-mix(in srgb, var(--sidebar-foreground) 68%, transparent);\n        text-decoration: none;\n        transition:\n            background 150ms ease,\n            color 150ms ease;\n    }\n\n    .rail-link::before {\n        content: "";\n        position: absolute;\n        inset: 7px auto 7px -6px;\n        width: 3px;\n        background: transparent;\n        transition: background 150ms ease;\n    }\n\n    .rail-link span {\n        max-width: 60px;\n        overflow: hidden;\n        text-overflow: ellipsis;\n        font-family: "IBM Plex Sans Condensed", sans-serif;\n        font-size: 12px;\n        font-weight: 600;\n        line-height: 1;\n        white-space: nowrap;\n    }\n\n    /* Keep the localized Settings label legible within the compact rail. */\n    .rail-group-secondary .rail-link:last-child span {\n        max-width: 68px;\n        font-size: 10.5px;\n    }\n\n    .rail-link:hover {\n        background: var(--sidebar-accent);\n        color: var(--sidebar-accent-foreground);\n    }\n\n    .rail-link.active {\n        background: var(--sidebar-accent);\n        color: var(--sidebar-accent-foreground);\n    }\n\n    .rail-link.active::before {\n        background: var(--foreground);\n    }\n\n    @media (max-width: 720px) {\n        .rail {\n            width: 72px;\n            flex-basis: 72px;\n        }\n\n        .rail-link span {\n            font-size: 11px;\n        }\n    }\n'
                }
            </style>
        </nav>
    );
}
