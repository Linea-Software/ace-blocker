import { createMemo, createSignal, For, Show, onSettled } from "solid-js";
import { Dynamic, Portal } from "@solidjs/web";
import type { Component } from "solid-js";
import { X } from "@lucide/solid";
import * as m from "$lib/paraglide/messages.js";
import {
    LIST_ICONS_GROUPED,
    FALLBACK_ICON,
    getPackIcon,
} from "./ace/solid-pack-icons";

type IconComponent = Component<{ size?: number; class?: string }>;
const allSlugs = LIST_ICONS_GROUPED.flatMap((group) => group.slugs);
export default function IconPicker(props: {
    selectedIcon?: string | null;
    open?: boolean;
    fallbackIcon?: IconComponent;
    label?: string;
    size?: "sm" | "md" | "lg";
    disabled?: boolean;
    side?: "bottom" | "left";
    onSelectedIconChange?: (icon: string | null) => void;
    onValueChange?: (icon: string | null) => void;
    onOpenChange?: (open: boolean) => void;
}) {
    const [localIcon, setLocalIcon] = createSignal<string | null>(null);
    const [localOpen, setLocalOpen] = createSignal(false);
    const [searchQuery, setSearchQuery] = createSignal("");
    const [position, setPosition] = createSignal({ top: 0, left: 0 });
    let trigger: HTMLButtonElement | undefined;
    let popover: HTMLDivElement | undefined;
    const selected = () =>
        props.selectedIcon !== undefined ? props.selectedIcon : localIcon();
    const opened = () => (props.open !== undefined ? props.open : localOpen());
    const size = () => props.size ?? "md";
    const triggerSize = () =>
        size() === "sm" ? "size-8" : size() === "lg" ? "size-11" : "size-9";
    const gridCols = () =>
        size() === "sm"
            ? "grid-cols-8"
            : size() === "lg"
              ? "grid-cols-5"
              : "grid-cols-7";
    const itemSize = () =>
        size() === "sm" ? "size-7" : size() === "lg" ? "size-11" : "size-8";
    const iconSize = () => (size() === "sm" ? 14 : size() === "lg" ? 18 : 16);
    const filteredSlugs = createMemo(() =>
        searchQuery()
            ? allSlugs.filter((slug) =>
                  slug.includes(searchQuery().toLowerCase()),
              )
            : allSlugs,
    );
    const updatePosition = () => {
        const rect = trigger?.getBoundingClientRect();
        if (!rect) return;
        if (props.side === "left") {
            setPosition({
                top: Math.max(12, Math.min(rect.top, window.innerHeight - 332)),
                left: Math.max(12, rect.left - 288 - 10),
            });
        } else {
            setPosition({
                top: Math.max(
                    12,
                    Math.min(rect.bottom + 10, window.innerHeight - 332),
                ),
                left: Math.max(
                    12,
                    Math.min(rect.left, window.innerWidth - 288 - 12),
                ),
            });
        }
    };
    const setOpen = (next: boolean) => {
        setLocalOpen(next);
        props.onOpenChange?.(next);
        if (next) updatePosition();
    };
    const choose = (slug: string | null) => {
        setLocalIcon(slug);
        props.onSelectedIconChange?.(slug);
        props.onValueChange?.(slug);
        setOpen(false);
    };
    const option = (slug: string) => (
        <button
            type="button"
            aria-label={slug}
            aria-pressed={selected() === slug ? "true" : "false"}
            class={`${itemSize()} rounded-md flex items-center justify-center focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${
                selected() === slug
                    ? "bg-muted text-foreground ring-1 ring-foreground/40"
                    : "bg-transparent hover:bg-muted text-muted-foreground hover:text-foreground"
            }`}
            onClick={() => choose(slug)}
        >
            <Dynamic component={getPackIcon(slug)} size={iconSize()} />
        </button>
    );
    onSettled(() => {
        const outside = (event: PointerEvent) => {
            if (
                !popover?.contains(event.target as Node) &&
                !trigger?.contains(event.target as Node)
            )
                setOpen(false);
        };
        const escape = (event: KeyboardEvent) => {
            if (event.key === "Escape" && opened()) setOpen(false);
        };
        document.addEventListener("pointerdown", outside);
        document.addEventListener("keydown", escape);
        window.addEventListener("resize", updatePosition);
        window.addEventListener("scroll", updatePosition, true);
        return () => {
            document.removeEventListener("pointerdown", outside);
            document.removeEventListener("keydown", escape);
            window.removeEventListener("resize", updatePosition);
            window.removeEventListener("scroll", updatePosition, true);
        };
    });
    return (
        <>
            <button
                type="button"
                ref={trigger}
                disabled={!!props.disabled}
                aria-haspopup="dialog"
                aria-expanded={opened() ? "true" : "false"}
                aria-label={props.label || m.choose_icon()}
                class={`${triggerSize()} shrink-0 rounded-sm border border-border bg-card flex items-center justify-center text-muted-foreground hover:border-primary/40 hover:text-foreground hover:bg-muted/50 transition-colors`}
                onClick={() => setOpen(!opened())}
            >
                <Dynamic
                    component={
                        selected()
                            ? getPackIcon(selected())
                            : (props.fallbackIcon ?? FALLBACK_ICON)
                    }
                    size={size() === "sm" ? 16 : size() === "lg" ? 20 : 18}
                />
            </button>
            <Show when={opened() && !props.disabled}>
                <Portal>
                    <div
                        ref={popover}
                        data-slot="popover-content"
                        role="dialog"
                        aria-label={props.label || m.choose_icon()}
                        class="fixed z-[80] w-72 max-h-[min(320px,calc(100dvh-32px))] overflow-y-auto rounded-sm border border-border bg-card p-3 shadow-none"
                        style={{
                            top: position().top + "px",
                            left: position().left + "px",
                        }}
                    >
                        <div class="mb-3">
                            <input
                                type="text"
                                value={searchQuery()}
                                onInput={(e) =>
                                    setSearchQuery(e.currentTarget.value)
                                }
                                placeholder={m.search_icons()}
                                aria-label={m.search_icons()}
                                class="w-full h-8 px-2.5 text-sm rounded-md bg-muted border border-border/50 focus:outline-none focus:ring-1 focus:ring-ring"
                            />
                        </div>
                        <div class="space-y-3">
                            <button
                                type="button"
                                aria-label={m.none_label()}
                                aria-pressed={
                                    selected() === null ? "true" : "false"
                                }
                                onClick={() => choose(null)}
                                class={`${itemSize()} rounded-md flex items-center justify-center focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${
                                    selected() === null
                                        ? "bg-muted text-foreground ring-1 ring-foreground/40"
                                        : "bg-transparent hover:bg-muted text-muted-foreground hover:text-foreground"
                                }`}
                            >
                                <X size={iconSize()} />
                            </button>
                            <Show
                                when={searchQuery()}
                                fallback={
                                    <For each={LIST_ICONS_GROUPED}>
                                        {(group) => (
                                            <div>
                                                <p class="text-xs font-semibold text-muted-foreground mb-1.5">
                                                    {group.category}
                                                </p>
                                                <div
                                                    class={`grid ${gridCols()} gap-1.5`}
                                                >
                                                    <For each={group.slugs}>
                                                        {option}
                                                    </For>
                                                </div>
                                            </div>
                                        )}
                                    </For>
                                }
                            >
                                <div class={`grid ${gridCols()} gap-1.5`}>
                                    <For each={filteredSlugs()}>{option}</For>
                                </div>
                            </Show>
                        </div>
                    </div>
                </Portal>
            </Show>
        </>
    );
}
