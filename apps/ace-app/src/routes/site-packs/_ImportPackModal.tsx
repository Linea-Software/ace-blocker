import { createMemo, createSignal, For, Show } from "solid-js";
import { open as openFilePicker } from "@tauri-apps/plugin-dialog";
import { toast } from "solid-sonner";
import {
    createSitePack,
    type SitePackFormat,
    type SitePackSourceType,
} from "$engine-client";
import * as m from "$lib/paraglide/messages.js";
import {
    FileText,
    FileUp,
    Globe,
    Info,
    Link2,
    PackagePlus,
    Plus,
    Trash2,
    X,
} from "@lucide/solid";
import { Input } from "$lib/components/ui/input/solid";
import { Textarea } from "$lib/components/ui/textarea/solid";
import { Button } from "$lib/components/ui/button/solid";
import { Badge } from "$lib/components/ui/badge/solid";
import { Spinner } from "$lib/components/ui/spinner/solid";
import * as Card from "$lib/components/ui/card/solid";
import { Label } from "$lib/components/ui/label/solid";
import IconPicker from "./_IconPicker";
import {
    IncompleteSitePackImportError,
    waitForSitePackImport,
} from "./_site-pack-import";

export interface ImportPackModalProps {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    onSave?: () => void | Promise<void>;
}

type Source = {
    type: SitePackSourceType;
    format: SitePackFormat;
    value: string;
};

export default function ImportPackModal(props: ImportPackModalProps) {
    const [packName, setPackName] = createSignal("");
    const [packDescription, setPackDescription] = createSignal("");
    const [sources, setSources] = createSignal<Source[]>([]);
    const [currentSourceType, setCurrentSourceType] =
        createSignal<SitePackSourceType>("url");
    const [currentSourceFormat, setCurrentSourceFormat] =
        createSignal<SitePackFormat>("standard");
    const [currentUrlInput, setCurrentUrlInput] = createSignal("");
    const [isCreating, setIsCreating] = createSignal(false);
    const [selectedIcon, setSelectedIcon] = createSignal<string | null>(null);
    const [showIconPicker, setShowIconPicker] = createSignal(false);

    const formatOptions: Array<{ value: SitePackFormat; label: string }> = [
        { value: "standard", label: m.format_default() },
        { value: "wildcarddomains", label: m.format_wildcard() },
    ];
    const isFormReady = createMemo(
        () => packName().trim().length > 0 && sources().length > 0,
    );
    const sourceCountLabel = createMemo(() =>
        sources().length === 0
            ? m.sources_added_zero()
            : sources().length === 1
              ? m.one_source_added()
              : m.sources_added({ count: sources().length }),
    );

    function isValidUrl(value: string) {
        try {
            const parsed = new URL(value);
            return parsed.protocol === "https:" || parsed.protocol === "http:";
        } catch {
            return false;
        }
    }

    function addUrlSource() {
        const sourceUrl = currentUrlInput().trim();
        if (!isValidUrl(sourceUrl)) {
            toast.error(m.invalid_url());
            return;
        }
        setSources((previous) => [
            ...previous,
            { type: "url", format: currentSourceFormat(), value: sourceUrl },
        ]);
        setCurrentUrlInput("");
    }

    function handleUrlKeydown(event: KeyboardEvent) {
        if (event.key !== "Enter" || !currentUrlInput().trim()) return;
        event.preventDefault();
        addUrlSource();
    }

    async function handleFileSelect() {
        try {
            const result = await openFilePicker({
                multiple: false,
                filters: [{ name: "Text", extensions: ["txt", "hosts"] }],
            });
            if (result && typeof result === "string") {
                setSources((previous) => [
                    ...previous,
                    {
                        type: "file",
                        format: currentSourceFormat(),
                        value: result,
                    },
                ]);
            }
        } catch (error) {
            console.error(error);
            toast.error(m.failed_to_sync());
        }
    }

    function removeSource(sourceToRemove: Source) {
        setSources((previous) =>
            previous.filter((source) => source !== sourceToRemove),
        );
    }

    function resetForm() {
        setPackName("");
        setPackDescription("");
        setSources([]);
        setCurrentUrlInput("");
        setCurrentSourceType("url");
        setCurrentSourceFormat("standard");
        setSelectedIcon(null);
        setShowIconPicker(false);
    }

    async function handleCreate() {
        if (!isFormReady() || isCreating()) return;
        setIsCreating(true);
        try {
            const { stream } = await createSitePack({
                body: {
                    name: packName().trim(),
                    description: packDescription().trim() || null,
                    icon: selectedIcon(),
                    sources: sources().map((source) => ({
                        source: source.value,
                        sourceType: source.type,
                        format: source.format,
                    })),
                },
            });
            if (!stream) throw new IncompleteSitePackImportError();
            await waitForSitePackImport(stream);
            toast.success(m.site_pack_created_success());
            await props.onSave?.();
            props.onOpenChange(false);
            resetForm();
        } catch (error) {
            toast.error(
                error instanceof IncompleteSitePackImportError
                    ? m.site_pack_import_incomplete()
                    : error instanceof Error
                      ? error.message
                      : m.failed_to_sync(),
            );
        } finally {
            setIsCreating(false);
        }
    }

    const dismiss = () => {
        if (!isCreating()) props.onOpenChange(false);
    };

    return (
        <Show when={props.open}>
            <div
                class="fixed inset-0 z-[70] grid place-items-center bg-black/70 p-4"
                role="presentation"
                onClick={(e) => {
                    if (e.target === e.currentTarget) dismiss();
                }}
            >
                <section
                    role="dialog"
                    aria-modal="true"
                    aria-labelledby="import-pack-title"
                    aria-describedby="import-pack-description"
                    class="border-border/80 bg-card flex max-h-[min(90dvh,760px)] w-full max-w-2xl flex-col gap-0 overflow-hidden rounded-md border p-0 shadow-none"
                    onKeyDown={(e) => {
                        if (e.key === "Escape") dismiss();
                    }}
                >
                    <header class="border-border/60 relative flex flex-row items-start gap-3 border-b p-5 pr-12 text-left">
                        <div class="bg-primary/10 text-primary flex size-10 shrink-0 items-center justify-center rounded-lg">
                            <PackagePlus class="size-5" aria-hidden="true" />
                        </div>
                        <div class="flex min-w-0 flex-col gap-1">
                            <h2
                                id="import-pack-title"
                                class="text-xl font-semibold tracking-tight"
                            >
                                {m.import_custom_pack()}
                            </h2>
                            <p
                                id="import-pack-description"
                                class="text-sm leading-relaxed text-muted-foreground"
                            >
                                {m.import_custom_pack_description()}
                            </p>
                        </div>
                        <button
                            type="button"
                            class="absolute right-4 top-4 text-muted-foreground hover:text-foreground"
                            disabled={isCreating()}
                            aria-label={m.close()}
                            onClick={dismiss}
                        >
                            <X size={18} aria-hidden="true" />
                        </button>
                    </header>

                    <div class="bg-background flex-1 overflow-y-auto p-4 sm:p-5">
                        <div class="flex flex-col gap-4">
                            <Card.Root class="border-border/70 gap-0 rounded-md py-0 shadow-none">
                                <Card.Header class="gap-1 p-4 pb-3">
                                    <Card.Title class="text-base font-semibold">
                                        {m.pack_details_title()}
                                    </Card.Title>
                                    <Card.Description>
                                        {m.pack_details_description()}
                                    </Card.Description>
                                </Card.Header>
                                <Card.Content class="flex flex-col gap-4 p-4 pt-0">
                                    <div class="grid gap-4 sm:grid-cols-[minmax(0,1fr)_auto]">
                                        <div class="flex flex-col gap-2">
                                            <Label for="name">
                                                {m.pack_name()}
                                            </Label>
                                            <Input
                                                id="name"
                                                value={packName()}
                                                onInput={(e) =>
                                                    setPackName(
                                                        e.currentTarget.value,
                                                    )
                                                }
                                                placeholder={m.import_pack_name_placeholder()}
                                                autocomplete="off"
                                            />
                                        </div>
                                        <div class="flex min-w-36 flex-col gap-2">
                                            <Label>{m.pack_icon()}</Label>
                                            <div class="flex items-center gap-2">
                                                <IconPicker
                                                    selectedIcon={selectedIcon()}
                                                    onSelectedIconChange={
                                                        setSelectedIcon
                                                    }
                                                    open={showIconPicker()}
                                                    onOpenChange={
                                                        setShowIconPicker
                                                    }
                                                    label={m.choose_icon()}
                                                    side="left"
                                                />
                                                <span class="text-muted-foreground truncate text-sm">
                                                    {selectedIcon() ||
                                                        m.choose_icon()}
                                                </span>
                                            </div>
                                        </div>
                                    </div>
                                    <div class="flex flex-col gap-2">
                                        <Label for="description">
                                            {m.pack_description()}
                                        </Label>
                                        <Textarea
                                            id="description"
                                            value={packDescription()}
                                            onInput={(e) =>
                                                setPackDescription(
                                                    e.currentTarget.value,
                                                )
                                            }
                                            placeholder={m.import_pack_description_placeholder()}
                                            class="min-h-20 resize-none"
                                        />
                                    </div>
                                </Card.Content>
                            </Card.Root>

                            <Card.Root class="border-border/70 gap-0 rounded-md py-0 shadow-none">
                                <Card.Header class="flex-row items-start justify-between gap-4 p-4 pb-3">
                                    <div class="flex min-w-0 flex-col gap-1">
                                        <Card.Title class="text-base font-semibold">
                                            {m.sources_label()}
                                        </Card.Title>
                                        <Card.Description>
                                            {m.sources_description()}
                                        </Card.Description>
                                    </div>
                                    <Show when={sources().length > 0}>
                                        <Badge variant="secondary">
                                            {sourceCountLabel()}
                                        </Badge>
                                    </Show>
                                </Card.Header>
                                <Card.Content class="flex flex-col gap-4 p-4 pt-0">
                                    <div class="flex flex-col gap-2">
                                        <Label for="pack-format">
                                            {m.pack_format()}
                                        </Label>
                                        <select
                                            id="pack-format"
                                            class="border-border bg-card h-10 w-full rounded-md border px-3 text-sm"
                                            value={currentSourceFormat()}
                                            onChange={(e) =>
                                                setCurrentSourceFormat(
                                                    e.currentTarget
                                                        .value as SitePackFormat,
                                                )
                                            }
                                        >
                                            <For each={formatOptions}>
                                                {(format) => (
                                                    <option
                                                        value={format.value}
                                                    >
                                                        {format.label}
                                                    </option>
                                                )}
                                            </For>
                                        </select>
                                    </div>
                                    <div class="w-full">
                                        <div
                                            class="bg-muted text-muted-foreground grid w-full grid-cols-2 rounded-md p-1"
                                            role="tablist"
                                        >
                                            <button
                                                type="button"
                                                role="tab"
                                                aria-selected={
                                                    currentSourceType() ===
                                                    "url"
                                                        ? "true"
                                                        : "false"
                                                }
                                                class={`flex h-9 items-center justify-center gap-2 rounded-sm text-sm ${currentSourceType() === "url" ? "bg-background font-semibold text-foreground shadow-sm" : ""}`}
                                                onClick={() =>
                                                    setCurrentSourceType("url")
                                                }
                                            >
                                                <Link2
                                                    size={16}
                                                    aria-hidden="true"
                                                />
                                                {m.url()}
                                            </button>
                                            <button
                                                type="button"
                                                role="tab"
                                                aria-selected={
                                                    currentSourceType() ===
                                                    "file"
                                                        ? "true"
                                                        : "false"
                                                }
                                                class={`flex h-9 items-center justify-center gap-2 rounded-sm text-sm ${currentSourceType() === "file" ? "bg-background font-semibold text-foreground shadow-sm" : ""}`}
                                                onClick={() =>
                                                    setCurrentSourceType("file")
                                                }
                                            >
                                                <FileText
                                                    size={16}
                                                    aria-hidden="true"
                                                />
                                                {m.local_file()}
                                            </button>
                                        </div>
                                        <Show
                                            when={currentSourceType() === "url"}
                                            fallback={
                                                <div class="pt-3">
                                                    <Button
                                                        variant="outline"
                                                        class="text-muted-foreground hover:text-foreground h-auto w-full justify-start whitespace-normal p-4 text-left"
                                                        onClick={() =>
                                                            void handleFileSelect()
                                                        }
                                                    >
                                                        <FileUp data-icon="inline-start" />
                                                        <span class="flex flex-col gap-0.5">
                                                            <span class="text-foreground font-medium">
                                                                {m.select_file()}
                                                            </span>
                                                            <span class="text-xs font-normal">
                                                                {m.supported_pack_files()}
                                                            </span>
                                                        </span>
                                                    </Button>
                                                </div>
                                            }
                                        >
                                            <div class="flex gap-2 pt-3">
                                                <Input
                                                    aria-label={m.source_url_label()}
                                                    value={currentUrlInput()}
                                                    onInput={(e) =>
                                                        setCurrentUrlInput(
                                                            e.currentTarget
                                                                .value,
                                                        )
                                                    }
                                                    placeholder={m.import_source_url_placeholder()}
                                                    onKeyDown={handleUrlKeydown}
                                                />
                                                <Button
                                                    variant="secondary"
                                                    size="icon"
                                                    onClick={addUrlSource}
                                                    disabled={
                                                        !currentUrlInput().trim()
                                                    }
                                                    aria-label={m.add_source_url()}
                                                    title={m.add_source_url()}
                                                >
                                                    <Plus aria-hidden="true" />
                                                </Button>
                                            </div>
                                        </Show>
                                    </div>
                                    <div class="flex min-h-20 flex-col gap-2">
                                        <Show
                                            when={sources().length > 0}
                                            fallback={
                                                <div class="border-border/70 bg-muted/20 text-muted-foreground flex min-h-20 items-center justify-center gap-2 rounded-lg border border-dashed px-4 text-center text-sm">
                                                    <Plus
                                                        class="size-4"
                                                        aria-hidden="true"
                                                    />
                                                    <span>
                                                        {m.no_sources_added()}
                                                    </span>
                                                </div>
                                            }
                                        >
                                            <For each={sources()}>
                                                {(source) => (
                                                    <div class="border-border/70 bg-muted/30 flex items-center justify-between gap-3 rounded-lg border p-3">
                                                        <div class="flex min-w-0 items-center gap-3">
                                                            <div class="bg-background text-muted-foreground flex size-8 shrink-0 items-center justify-center rounded-md border">
                                                                <Show
                                                                    when={
                                                                        source.type ===
                                                                        "url"
                                                                    }
                                                                    fallback={
                                                                        <FileText
                                                                            class="size-4"
                                                                            aria-hidden="true"
                                                                        />
                                                                    }
                                                                >
                                                                    <Globe
                                                                        class="size-4"
                                                                        aria-hidden="true"
                                                                    />
                                                                </Show>
                                                            </div>
                                                            <div class="flex min-w-0 flex-col gap-1">
                                                                <span
                                                                    class="truncate text-sm font-medium"
                                                                    title={
                                                                        source.value
                                                                    }
                                                                >
                                                                    {
                                                                        source.value
                                                                    }
                                                                </span>
                                                                <div class="flex flex-wrap gap-1.5">
                                                                    <Badge variant="outline">
                                                                        {source.type ===
                                                                        "url"
                                                                            ? m.url()
                                                                            : m.local_file()}
                                                                    </Badge>
                                                                    <Badge variant="outline">
                                                                        {source.format ===
                                                                        "standard"
                                                                            ? m.format_standard()
                                                                            : m.format_wildcard()}
                                                                    </Badge>
                                                                </div>
                                                            </div>
                                                        </div>
                                                        <Button
                                                            variant="ghost"
                                                            size="icon"
                                                            class="text-muted-foreground hover:text-destructive shrink-0"
                                                            onClick={() =>
                                                                removeSource(
                                                                    source,
                                                                )
                                                            }
                                                            aria-label={m.remove_source()}
                                                            title={m.remove_source()}
                                                        >
                                                            <Trash2 aria-hidden="true" />
                                                        </Button>
                                                    </div>
                                                )}
                                            </For>
                                        </Show>
                                    </div>
                                </Card.Content>
                            </Card.Root>
                        </div>
                    </div>

                    <footer class="border-border/60 bg-card flex w-full flex-col gap-3 border-t p-4 sm:flex-row sm:items-center sm:justify-between">
                        <div
                            class="text-muted-foreground flex min-w-0 flex-1 items-center gap-2 text-sm"
                            aria-live="polite"
                        >
                            <Info class="size-4 shrink-0" aria-hidden="true" />
                            <span>
                                {!packName().trim()
                                    ? m.import_name_validation_message()
                                    : sources().length === 0
                                      ? m.import_validation_message()
                                      : m.import_ready_message()}
                            </span>
                        </div>
                        <div class="flex w-full items-center gap-2 sm:w-auto">
                            <Button
                                variant="outline"
                                onClick={dismiss}
                                class="flex-1 sm:flex-none"
                                disabled={isCreating()}
                            >
                                {m.cancel()}
                            </Button>
                            <Button
                                class="min-w-32 flex-1 sm:flex-none"
                                onClick={() => void handleCreate()}
                                disabled={!isFormReady() || isCreating()}
                            >
                                <Show
                                    when={isCreating()}
                                    fallback={
                                        <>
                                            <PackagePlus data-icon="inline-start" />
                                            {m.create_pack()}
                                        </>
                                    }
                                >
                                    <Spinner data-icon="inline-start" />
                                    {m.creating()}
                                </Show>
                            </Button>
                        </div>
                    </footer>
                </section>
            </div>
        </Show>
    );
}
