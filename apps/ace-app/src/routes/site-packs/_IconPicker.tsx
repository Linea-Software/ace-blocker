import { createMemo, createSignal, For, Show, onSettled } from "solid-js";
import { Dynamic, Portal } from "@solidjs/web";
import type { Component } from "solid-js";
import { X } from "@lucide/solid";
import * as m from "$lib/paraglide/messages.js";
import { cn } from "$lib/utils";
import { LIST_ICONS_GROUPED, FALLBACK_ICON, getPackIcon } from "./_pack-icons";

export interface IconPickerProps {
    selectedIcon: string | null;
    onSelectedIconChange: (icon: string | null) => void;
    open: boolean;
    onOpenChange: (open: boolean) => void;
    fallbackIcon?: Component<{ size?: number }>;
    label?: string;
    size?: "sm" | "md" | "lg";
    disabled?: boolean;
    side?: "bottom" | "left";
}

/** Solid 2 version of the icon grid; an anchored portal avoids clipping inside dialogs. */
export default function IconPicker(props: IconPickerProps) {
    let triggerEl!: HTMLButtonElement;
    let contentEl: HTMLDivElement | undefined;
    const [searchQuery, setSearchQuery] = createSignal("");
    const [position, setPosition] = createSignal({ left: 0, top: 0 });
    const allSlugs = LIST_ICONS_GROUPED.flatMap((group) => group.slugs);
    const filteredSlugs = createMemo(() =>
        searchQuery()
            ? allSlugs.filter((slug) =>
                  slug.includes(searchQuery().toLowerCase()),
              )
            : allSlugs,
    );
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

    function updatePosition() {
        if (!triggerEl) return;
        const rect = triggerEl.getBoundingClientRect();
        const paneWidth = Math.min(288, window.innerWidth - 24);
        const left =
            props.side === "left" ? rect.left - paneWidth - 10 : rect.left;
        setPosition({
            left: Math.max(
                12,
                Math.min(window.innerWidth - paneWidth - 12, left),
            ),
            top: Math.max(
                12,
                Math.min(
                    window.innerHeight - 332,
                    props.side === "left" ? rect.top : rect.bottom + 10,
                ),
            ),
        });
    }

    function openPicker() {
        if (props.disabled) return;
        if (!props.open) updatePosition();
        props.onOpenChange(!props.open);
    }

    function choose(slug: string | null) {
        props.onSelectedIconChange(slug);
        props.onOpenChange(false);
        setSearchQuery("");
    }

    onSettled(() => {
        function dismiss(event: PointerEvent) {
            if (
                props.open &&
                !contentEl?.contains(event.target as Node) &&
                !triggerEl.contains(event.target as Node)
            ) {
                props.onOpenChange(false);
            }
        }
        function keydown(event: KeyboardEvent) {
            if (event.key === "Escape" && props.open) {
                props.onOpenChange(false);
                triggerEl.focus();
            }
        }
        document.addEventListener("pointerdown", dismiss);
        document.addEventListener("keydown", keydown);
        window.addEventListener("resize", updatePosition);
        window.addEventListener("scroll", updatePosition, true);
        return () => {
            document.removeEventListener("pointerdown", dismiss);
            document.removeEventListener("keydown", keydown);
            window.removeEventListener("resize", updatePosition);
            window.removeEventListener("scroll", updatePosition, true);
        };
    });

    const selectedClass = (slug: string | null) =>
        cn(
            itemSize(),
            "rounded-md flex items-center justify-center focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
            props.selectedIcon === slug
                ? "bg-muted text-foreground ring-1 ring-foreground/40"
                : "bg-transparent hover:bg-muted text-muted-foreground hover:text-foreground",
        );

    return (
        <>
            <button
                ref={triggerEl}
                type="button"
                disabled={props.disabled}
                class={cn(
                    triggerSize(),
                    "shrink-0 rounded-sm border border-border bg-card flex items-center justify-center text-muted-foreground hover:border-primary/40 hover:text-foreground hover:bg-muted/50 transition-colors",
                )}
                aria-label={props.label || m.choose_icon()}
                aria-expanded={props.open ? "true" : "false"}
                onClick={openPicker}
            >
                <Dynamic
                    component={
                        props.selectedIcon
                            ? getPackIcon(props.selectedIcon)
                            : (props.fallbackIcon ?? FALLBACK_ICON)
                    }
                    size={size() === "sm" ? 16 : size() === "lg" ? 20 : 18}
                />
            </button>
            <Show when={props.open}>
                <Portal>
                    <div
                        ref={contentEl}
                        data-slot="popover-content"
                        role="dialog"
                        aria-label={props.label || m.choose_icon()}
                        class="fixed z-[80] w-72 max-w-[calc(100dvw-24px)] max-h-[min(320px,calc(100dvh-32px))] overflow-y-auto rounded-sm border border-border bg-card p-3 shadow-none"
                        style={{
                            left: `${position().left}px`,
                            top: `${position().top}px`,
                        }}
                    >
                        <div class="mb-3">
                            <input
                                type="text"
                                placeholder={m.search_icons()}
                                aria-label={m.search_icons()}
                                value={searchQuery()}
                                onInput={(event) =>
                                    setSearchQuery(event.currentTarget.value)
                                }
                                class="w-full h-8 px-2.5 text-sm rounded-md bg-muted border border-border/50 focus:outline-none focus:ring-1 focus:ring-ring"
                            />
                        </div>
                        <div class="space-y-3">
                            <button
                                type="button"
                                class={selectedClass(null)}
                                onClick={() => choose(null)}
                                aria-label={m.none_label()}
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
                                                    class={cn(
                                                        "grid gap-1.5",
                                                        gridCols(),
                                                    )}
                                                >
                                                    <For each={group.slugs}>
                                                        {(slug) => (
                                                            <button
                                                                type="button"
                                                                class={selectedClass(
                                                                    slug,
                                                                )}
                                                                aria-label={
                                                                    slug
                                                                }
                                                                onClick={() =>
                                                                    choose(slug)
                                                                }
                                                            >
                                                                <Dynamic
                                                                    component={getPackIcon(
                                                                        slug,
                                                                    )}
                                                                    size={iconSize()}
                                                                />
                                                            </button>
                                                        )}
                                                    </For>
                                                </div>
                                            </div>
                                        )}
                                    </For>
                                }
                            >
                                <div class={cn("grid gap-1.5", gridCols())}>
                                    <For each={filteredSlugs()}>
                                        {(slug) => (
                                            <button
                                                type="button"
                                                class={selectedClass(slug)}
                                                aria-label={slug}
                                                onClick={() => choose(slug)}
                                            >
                                                <Dynamic
                                                    component={getPackIcon(
                                                        slug,
                                                    )}
                                                    size={iconSize()}
                                                />
                                            </button>
                                        )}
                                    </For>
                                </div>
                            </Show>
                        </div>
                    </div>
                </Portal>
            </Show>
        </>
    );
}
