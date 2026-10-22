<script lang="ts">
    import * as m from "$lib/paraglide/messages.js";
    import * as Dialog from "$lib/components/ui/dialog";
    import { Input } from "$lib/components/ui/input";
    import { Textarea } from "$lib/components/ui/textarea";
    import { Button } from "$lib/components/ui/button";
    import { Badge } from "$lib/components/ui/badge";
    import { Spinner } from "$lib/components/ui/spinner";
    import * as Card from "$lib/components/ui/card";
    import { Label } from "$lib/components/ui/label";
    import * as Select from "$lib/components/ui/select";
    import * as Tabs from "$lib/components/ui/tabs";
    import * as Popover from "$lib/components/ui/popover";
    import { open as openFilePicker } from "@tauri-apps/plugin-dialog";
    import {
        createSitePack,
        type SitePackFormat,
        type SitePackSourceType,
    } from "$engine-client";
    import { toast } from "svelte-sonner";
    import {
        FileText,
        FileUp,
        Globe,
        Info,
        Link2,
        PackagePlus,
        Plus,
        Trash2,
    } from "@lucide/svelte";
    import IconPicker from "$lib/components/IconPicker.svelte";
    import { IncompleteSitePackImportError, waitForSitePackImport } from "./_site-pack-import";

    let {
        open = $bindable(false),
        onSave,
    }: { open: boolean; onSave?: () => void } = $props();

    let packName = $state("");
    let packDescription = $state("");
    let sources = $state<
        Array<{
            type: SitePackSourceType;
            format: SitePackFormat;
            value: string;
        }>
    >([]);

    let currentSourceType = $state<SitePackSourceType>("url");
    let currentSourceFormat = $state<SitePackFormat>("standard");
    let currentUrlInput = $state("");
    let isCreating = $state(false);
    let selectedIcon = $state<string | null>(null);
    let showIconPicker = $state(false);

    const selectedIconLabel = $derived(selectedIcon || m.choose_icon());

    const formatOptions = [
        { value: "standard", label: m.format_default() },
        { value: "wildcarddomains", label: m.format_wildcard() },
    ];

    const currentFormatLabel = $derived(
        formatOptions.find((f) => f.value === currentSourceFormat)?.label ||
            m.pack_format(),
    );
    const isFormReady = $derived(
        packName.trim().length > 0 && sources.length > 0,
    );
    const sourceCountLabel = $derived(
        sources.length === 1
            ? m.one_source_added()
            : m.sources_added({ count: sources.length }),
    );

    function isValidUrl(string: string) {
        try {
            new URL(string);
            return true;
        } catch (_) {
            return false;
        }
    }

    function addUrlSource() {
        const sourceUrl = currentUrlInput.trim();
        if (!isValidUrl(sourceUrl)) {
            toast.error(m.invalid_url());
            return;
        }
        sources.push({
            type: "url",
            format: currentSourceFormat,
            value: sourceUrl,
        });
        currentUrlInput = "";
    }

    function handleUrlKeydown(event: KeyboardEvent) {
        if (event.key !== "Enter" || !currentUrlInput.trim()) return;
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
                sources.push({
                    type: "file",
                    format: currentSourceFormat,
                    value: result,
                });
            }
        } catch (err) {
            console.error(err);
            toast.error(m.failed_to_sync());
        }
    }

    function removeSource(sourceToRemove: (typeof sources)[number]) {
        sources = sources.filter((source) => source !== sourceToRemove);
    }

    async function handleCreate() {
        if (!isFormReady) return;
        isCreating = true;
        try {
            const { stream } = await createSitePack({
                body: {
                    name: packName.trim(),
                    description: packDescription.trim() || null,
                    icon: selectedIcon,
                    sources: sources.map((s) => ({
                        source: s.value,
                        sourceType: s.type,
                        format: s.format,
                    })),
                },
            });
            if (!stream) throw new IncompleteSitePackImportError();
            await waitForSitePackImport(stream);
            toast.success(m.site_pack_created_success());
            onSave?.();
            open = false;
            resetForm();
        } catch (error) {
            toast.error(
                error instanceof IncompleteSitePackImportError
                    ? m.site_pack_import_incomplete()
                    : error instanceof Error ? error.message : m.failed_to_sync(),
            );
        } finally {
            isCreating = false;
        }
    }

        function resetForm() {
        packName = "";
        packDescription = "";
        sources = [];
        currentUrlInput = "";
        currentSourceType = "url";
        currentSourceFormat = "standard";
        selectedIcon = null;
        showIconPicker = false;
    }
</script>

<Dialog.Root bind:open>
    <Dialog.Content
        class="border-border/80 bg-card flex max-h-[min(90dvh,760px)] w-full flex-col gap-0 overflow-hidden rounded-md p-0 shadow-none sm:max-w-2xl"
    >
        <Dialog.Header
            class="border-border/60 flex-row items-start gap-3 border-b p-5 pr-12 text-left"
        >
            <div
                class="bg-primary/10 text-primary flex size-10 shrink-0 items-center justify-center rounded-lg"
            >
                <PackagePlus class="size-5" aria-hidden="true" />
            </div>
            <div class="flex min-w-0 flex-col gap-1">
                <Dialog.Title class="text-xl font-semibold tracking-tight">
                    {m.import_custom_pack()}
                </Dialog.Title>
                <Dialog.Description class="text-sm leading-relaxed">
                    {m.import_custom_pack_description()}
                </Dialog.Description>
            </div>
        </Dialog.Header>

        <div class="bg-background flex-1 overflow-y-auto p-4 sm:p-5">
            <div class="flex flex-col gap-4">
                <Card.Root
                    class="border-border/70 gap-0 rounded-md py-0 shadow-none"
                >
                    <Card.Header class="gap-1 p-4 pb-3">
                        <Card.Title class="text-base font-semibold">
                            {m.pack_details_title()}
                        </Card.Title>
                        <Card.Description>
                            {m.pack_details_description()}
                        </Card.Description>
                    </Card.Header>
                    <Card.Content class="flex flex-col gap-4 p-4 pt-0">
                        <div
                            class="grid gap-4 sm:grid-cols-[minmax(0,1fr)_auto]"
                        >
                            <div class="flex flex-col gap-2">
                                <Label for="name">{m.pack_name()}</Label>
                                <Input
                                    id="name"
                                    bind:value={packName}
                                    placeholder={m.import_pack_name_placeholder()}
                                    autocomplete="off"
                                />
                            </div>
                            <div class="flex min-w-36 flex-col gap-2">
                                <Label>{m.pack_icon()}</Label>
                                <div class="flex items-center gap-2">
                                    <IconPicker
                                        bind:selectedIcon
                                        bind:open={showIconPicker}
                                        label={m.choose_icon()}
                                    />
                                    <span
                                        class="text-muted-foreground truncate text-sm"
                                    >
                                        {selectedIconLabel}
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
                                bind:value={packDescription}
                                placeholder={m.import_pack_description_placeholder()}
                                class="min-h-20 resize-none"
                            />
                        </div>
                    </Card.Content>
                </Card.Root>

                <Card.Root
                    class="border-border/70 gap-0 rounded-md py-0 shadow-none"
                >
                    <Card.Header
                        class="flex-row items-start justify-between gap-4 p-4 pb-3"
                    >
                        <div class="flex min-w-0 flex-col gap-1">
                            <Card.Title class="text-base font-semibold">
                                {m.sources_label()}
                            </Card.Title>
                            <Card.Description>
                                {m.sources_description()}
                            </Card.Description>
                        </div>
                        {#if sources.length > 0}
                            <Badge variant="secondary">
                                {sourceCountLabel}
                            </Badge>
                        {/if}
                    </Card.Header>
                    <Card.Content class="flex flex-col gap-4 p-4 pt-0">
                        <div class="flex flex-col gap-2">
                            <Label for="pack-format">{m.pack_format()}</Label>
                            <Select.Root
                                type="single"
                                bind:value={currentSourceFormat}
                            >
                                <Select.Trigger id="pack-format" class="w-full">
                                    {currentFormatLabel}
                                </Select.Trigger>
                                <Select.Content>
                                    <Select.Group>
                                        {#each formatOptions as format (format.value)}
                                            <Select.Item
                                                value={format.value}
                                                label={format.label}
                                            >
                                                {format.label}
                                            </Select.Item>
                                        {/each}
                                    </Select.Group>
                                </Select.Content>
                            </Select.Root>
                        </div>

                        <Tabs.Root
                            bind:value={currentSourceType}
                            class="w-full"
                        >
                            <Tabs.List class="grid w-full grid-cols-2">
                                <Tabs.Trigger value="url">
                                    <Link2 aria-hidden="true" />
                                    {m.url()}
                                </Tabs.Trigger>
                                <Tabs.Trigger value="file">
                                    <FileText aria-hidden="true" />
                                    {m.local_file()}
                                </Tabs.Trigger>
                            </Tabs.List>
                            <Tabs.Content value="url" class="pt-3">
                                <div class="flex gap-2">
                                    <Input
                                        aria-label={m.source_url_label()}
                                        bind:value={currentUrlInput}
                                        placeholder={m.import_source_url_placeholder()}
                                        onkeydown={handleUrlKeydown}
                                    />
                                    <Button
                                        variant="secondary"
                                        size="icon"
                                        onclick={addUrlSource}
                                        disabled={!currentUrlInput.trim()}
                                        aria-label={m.add_source_url()}
                                        title={m.add_source_url()}
                                    >
                                        <Plus aria-hidden="true" />
                                    </Button>
                                </div>
                            </Tabs.Content>
                            <Tabs.Content value="file" class="pt-3">
                                <Button
                                    variant="outline"
                                    class="text-muted-foreground hover:text-foreground h-auto w-full justify-start whitespace-normal p-4 text-left"
                                    onclick={handleFileSelect}
                                >
                                    <FileUp data-icon="inline-start" />
                                    <span class="flex flex-col gap-0.5">
                                        <span
                                            class="text-foreground font-medium"
                                        >
                                            {m.select_file()}
                                        </span>
                                        <span class="text-xs font-normal">
                                            {m.supported_pack_files()}
                                        </span>
                                    </span>
                                </Button>
                            </Tabs.Content>
                        </Tabs.Root>

                        <div class="flex min-h-20 flex-col gap-2">
                            {#each sources as source (source)}
                                <div
                                    class="border-border/70 bg-muted/30 flex items-center justify-between gap-3 rounded-lg border p-3"
                                >
                                    <div
                                        class="flex min-w-0 items-center gap-3"
                                    >
                                        <div
                                            class="bg-background text-muted-foreground flex size-8 shrink-0 items-center justify-center rounded-md border"
                                        >
                                            {#if source.type === "url"}
                                                <Globe
                                                    class="size-4"
                                                    aria-hidden="true"
                                                />
                                            {:else}
                                                <FileText
                                                    class="size-4"
                                                    aria-hidden="true"
                                                />
                                            {/if}
                                        </div>
                                        <div
                                            class="flex min-w-0 flex-col gap-1"
                                        >
                                            <span
                                                class="truncate text-sm font-medium"
                                                title={source.value}
                                            >
                                                {source.value}
                                            </span>
                                            <div class="flex flex-wrap gap-1.5">
                                                <Badge variant="outline">
                                                    {source.type === "url"
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
                                        onclick={() => removeSource(source)}
                                        aria-label={m.remove_source()}
                                        title={m.remove_source()}
                                    >
                                        <Trash2 aria-hidden="true" />
                                    </Button>
                                </div>
                            {:else}
                                <div
                                    class="border-border/70 bg-muted/20 text-muted-foreground flex min-h-20 items-center justify-center gap-2 rounded-lg border border-dashed px-4 text-center text-sm"
                                >
                                    <Plus class="size-4" aria-hidden="true" />
                                    <span>{m.no_sources_added()}</span>
                                </div>
                            {/each}
                        </div>
                    </Card.Content>
                </Card.Root>
            </div>
        </div>

        <Dialog.Footer
            class="border-border/60 bg-card flex w-full flex-col gap-3 border-t p-4 sm:flex-row sm:items-center sm:justify-between"
        >
            <div
                class="text-muted-foreground flex min-w-0 flex-1 items-center gap-2 text-sm"
                aria-live="polite"
            >
                <Info class="size-4 shrink-0" aria-hidden="true" />
                <span>
                    {#if !packName.trim()}
                        {m.import_name_validation_message()}
                    {:else if sources.length === 0}
                        {m.import_validation_message()}
                    {:else}
                        {m.import_ready_message()}
                    {/if}
                </span>
            </div>

            <div class="flex w-full items-center gap-2 sm:w-auto">
                <Button
                    variant="outline"
                    onclick={() => (open = false)}
                    class="flex-1 sm:flex-none"
                    disabled={isCreating}
                >
                    {m.cancel()}
                </Button>
                <Button
                    class="min-w-32 flex-1 sm:flex-none"
                    onclick={handleCreate}
                    disabled={!isFormReady || isCreating}
                >
                    {#if isCreating}
                        <Spinner data-icon="inline-start" />
                        {m.creating()}
                    {:else}
                        <PackagePlus data-icon="inline-start" />
                        {m.create_pack()}
                    {/if}
                </Button>
            </div>
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>
