import { createMemo, createSignal, createEffect, For, Show } from "solid-js";
import { Button } from "$lib/components/ui/button/solid";
import { Input } from "$lib/components/ui/input/solid";
import { Badge } from "$lib/components/ui/badge/solid";
import {
    Check,
    CloudDownload,
    CircleCheck,
    Crown,
    ExternalLink,
    Search,
} from "@lucide/solid";
import * as m from "$lib/paraglide/messages.js";
import { cn, openGetProPage } from "$lib/utils.js";
import ProBadge from "$lib/components/ProBadge";
import type { SitePackSchema } from "$engine-client";

export interface AddSitesListDialogProps {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    availableLists: SitePackSchema[];
    selectedListIds: string[];
    isFreeUser?: boolean;
    onAdd: (listId: string) => void;
    onCancel?: () => void;
}

export default function AddSitesListDialog(props: AddSitesListDialogProps) {
    const [searchQuery, setSearchQuery] = createSignal("");
    const [selectedListId, setSelectedListId] = createSignal<string | null>(
        null,
    );
    createEffect(
        () => props.open,
        (open) => {
            if (open) {
                setSearchQuery("");
                setSelectedListId(null);
            }
        },
    );
    const sortedLists = createMemo(() =>
        [...props.availableLists].sort((a, b) =>
            a.requiresPro === b.requiresPro ? 0 : a.requiresPro ? 1 : -1,
        ),
    );
    const filteredLists = createMemo(() =>
        searchQuery().trim()
            ? sortedLists().filter((list) =>
                  list.name.toLowerCase().includes(searchQuery().toLowerCase()),
              )
            : sortedLists(),
    );
    const connectedIds = createMemo(() => new Set(props.selectedListIds));
    function handleListClick(list: SitePackSchema) {
        if (list.requiresPro && props.isFreeUser) {
            void openGetProPage();
            return;
        }
        if (connectedIds().has(list.id)) return;
        setSelectedListId(selectedListId() === list.id ? null : list.id);
    }
    function handleCancel() {
        setSelectedListId(null);
        props.onOpenChange(false);
        props.onCancel?.();
    }
    function handleConfirm() {
        const selected = selectedListId();
        if (!selected) return;
        props.onAdd(selected);
        setSelectedListId(null);
        props.onOpenChange(false);
    }
    return (
        <Show when={props.open}>
            <div
                class="fixed inset-0 z-[80] grid place-items-center bg-black/70 p-4"
                role="presentation"
                onClick={(event) => {
                    if (event.currentTarget === event.target) handleCancel();
                }}
            >
                <section
                    role="dialog"
                    aria-modal="true"
                    aria-labelledby="add-sites-title"
                    class="max-h-[90dvh] w-full max-w-[min(480px,calc(100vw-2rem))] overflow-y-auto rounded-md border border-border bg-background p-6 shadow-none"
                    onKeyDown={(event) => {
                        if (event.key === "Escape") handleCancel();
                    }}
                >
                    <header>
                        <h2
                            id="add-sites-title"
                            class="font-condensed text-xl font-semibold"
                        >
                            {m.add_site_pack_dialog_title()}
                        </h2>
                        <p class="mt-2 text-sm text-muted-foreground">
                            {m.add_site_pack_dialog_description()}
                        </p>
                    </header>
                    <div class="space-y-4 py-4">
                        <div class="relative">
                            <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground/50" />
                            <Input
                                placeholder={m.search_lists_placeholder()}
                                value={searchQuery()}
                                onInput={(event) =>
                                    setSearchQuery(event.currentTarget.value)
                                }
                                class="pl-9 bg-card"
                            />
                        </div>
                        <div class="max-h-[320px] overflow-y-auto space-y-1 -mx-1 px-1">
                            <Show
                                when={filteredLists().length > 0}
                                fallback={
                                    <p class="text-sm text-muted-foreground text-center py-8">
                                        {m.no_lists_found()}
                                    </p>
                                }
                            >
                                <p class="px-2 pb-1 font-mono text-[11px] font-medium text-muted-foreground">
                                    {m.available_lists()}
                                </p>
                                <For each={filteredLists()}>
                                    {(list) => {
                                        const connected = () =>
                                            connectedIds().has(list.id);
                                        const gated = () =>
                                            list.requiresPro &&
                                            !!props.isFreeUser;
                                        const selected = () =>
                                            selectedListId() === list.id;
                                        return (
                                            <button
                                                type="button"
                                                class={cn(
                                                    "w-full flex items-center px-3 py-2.5 rounded-sm border text-left transition-colors duration-150",
                                                    "bg-card",
                                                    connected() &&
                                                        "opacity-60 cursor-default border-border/30",
                                                    !connected() &&
                                                        !gated() &&
                                                        "border-border/50 hover:border-border cursor-pointer",
                                                    selected() &&
                                                        "border-foreground/35 bg-muted/45",
                                                    gated() &&
                                                        "cursor-pointer border-border opacity-50 hover:border-foreground/30",
                                                )}
                                                disabled={connected()}
                                                onClick={() =>
                                                    handleListClick(list)
                                                }
                                            >
                                                <div class="flex items-center w-full gap-3 min-w-0">
                                                    <div
                                                        class={cn(
                                                            "h-4 w-4 shrink-0 rounded-sm border flex items-center justify-center transition-all duration-200",
                                                            selected()
                                                                ? "border-foreground bg-foreground text-background"
                                                                : connected()
                                                                  ? "bg-primary/10 border-primary/40 text-primary"
                                                                  : "border-border",
                                                        )}
                                                    >
                                                        <Show
                                                            when={
                                                                selected() ||
                                                                connected()
                                                            }
                                                        >
                                                            <Check class="h-3 w-3" />
                                                        </Show>
                                                    </div>
                                                    <div class="flex flex-col flex-1 min-w-0">
                                                        <div class="flex items-center gap-2">
                                                            <span class="truncate font-semibold text-sm">
                                                                {list.name}
                                                            </span>
                                                            <Show
                                                                when={gated()}
                                                            >
                                                                <ProBadge />
                                                            </Show>
                                                            <Show
                                                                when={connected()}
                                                            >
                                                                <Badge
                                                                    variant="secondary"
                                                                    class="text-xs h-5 px-1.5 uppercase text-muted-foreground border-none shadow-none bg-muted/50 shrink-0"
                                                                >
                                                                    {m.add_site_pack_dialog_already_connected()}
                                                                </Badge>
                                                            </Show>
                                                        </div>
                                                        <div class="flex items-center justify-between gap-3 mt-1">
                                                            <Show
                                                                when={
                                                                    list.description
                                                                }
                                                            >
                                                                <span class="text-xs text-muted-foreground truncate max-w-[240px]">
                                                                    {
                                                                        list.description
                                                                    }
                                                                </span>
                                                            </Show>
                                                            <div class="flex items-center gap-3 shrink-0 ml-auto">
                                                                <Show
                                                                    when={
                                                                        !list.isSynced
                                                                    }
                                                                    fallback={
                                                                        <CircleCheck class="w-3.5 h-3.5 text-emerald-500/70" />
                                                                    }
                                                                >
                                                                    <Badge
                                                                        variant="secondary"
                                                                        class="text-xs h-5 px-2 uppercase text-muted-foreground border-none shadow-none bg-muted/50"
                                                                    >
                                                                        {m.not_downloaded()}
                                                                    </Badge>
                                                                    <CloudDownload class="w-3.5 h-3.5 text-muted-foreground/40" />
                                                                </Show>
                                                                <Show
                                                                    when={
                                                                        list.entriesCount
                                                                    }
                                                                >
                                                                    <span class="text-xs font-mono text-muted-foreground/60">
                                                                        {new Intl.NumberFormat(
                                                                            "en-US",
                                                                        ).format(
                                                                            list.entriesCount!,
                                                                        )}{" "}
                                                                        {m.domains_lowercase()}
                                                                    </span>
                                                                </Show>
                                                            </div>
                                                        </div>
                                                    </div>
                                                    <Show when={gated()}>
                                                        <div class="flex items-center gap-1.5 shrink-0 ml-2">
                                                            <Crown class="h-3.5 w-3.5 text-[var(--ace-pro)]" />
                                                            <span class="text-xs font-semibold text-[var(--ace-pro)]">
                                                                {m.get_pro()}
                                                            </span>
                                                            <ExternalLink class="h-3.5 w-3.5 text-muted-foreground opacity-70" />
                                                        </div>
                                                    </Show>
                                                </div>
                                            </button>
                                        );
                                    }}
                                </For>
                            </Show>
                        </div>
                    </div>
                    <footer class="flex items-center justify-end gap-2">
                        <Button variant="ghost" onClick={handleCancel}>
                            {m.cancel()}
                        </Button>
                        <Button
                            onClick={handleConfirm}
                            disabled={!selectedListId()}
                            class="rounded-sm px-6 font-semibold shadow-none"
                        >
                            {m.add_site_pack_confirm()}
                        </Button>
                    </footer>
                </section>
            </div>
        </Show>
    );
}
