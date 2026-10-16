<script lang="ts">
    import * as Sheet from "$lib/components/ui/sheet/index.js";
    import * as Tabs from "$lib/components/ui/tabs/index.js";
    import * as Select from "$lib/components/ui/select/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Label } from "$lib/components/ui/label/index.js";
    import {
        Terminal,
        Bug,
        LoaderCircle,
        Info,
        Sun,
        Moon,
        Monitor,
        Eye,
        EyeOff,
    } from "@lucide/svelte";
    import {
        setDebugProPolicy,
        getDebugProPolicy,
        hideFiles,
        showFiles,
        resetTos,
        type DebugProPolicy,
    } from "$engine-client";
    import { toast } from "svelte-sonner";
    import { setMode } from "mode-watcher";

    let { open = $bindable(false) } = $props();

    // State Overrides
    let selectedPolicy = $state<DebugProPolicy>("Default");
    let selectedTheme = $state("system");

    // Dev tool state
    let isHidingFiles = $state(false);
    let isShowingFiles = $state(false);
    let isResettingTos = $state(false);

    const themeOptions = [
        { value: "light", label: "Light", icon: Sun },
        { value: "dark", label: "Dark", icon: Moon },
        { value: "system", label: "System", icon: Monitor },
    ];

    $effect(() => {
        if (open) {
            loadMetaData();
        }
    });

    async function loadMetaData() {
        const [policyRes] = await Promise.all([getDebugProPolicy()]);

        if (policyRes.data) selectedPolicy = policyRes.data;
    }

    async function handlePolicyChange(v: string) {
        const policy = v as DebugProPolicy;
        selectedPolicy = policy;
        const { error } = await setDebugProPolicy({ path: { policy } });
        if (error) toast.error("Failed to set debug pro policy");
        else {
            toast.success(`Pro policy set to ${policy}`);
            setTimeout(() => window.location.reload(), 500);
        }
    }

    async function handleThemeChange(val: string) {
        selectedTheme = val;
        setMode(val as "light" | "dark" | "system");
        toast.success(`Theme changed to ${val}`);
    }

    async function handleHideFiles() {
        isHidingFiles = true; // UI state
        const response = await hideFiles();

        if (response.error) {
            console.error(response.error);
            toast.error("Failed to hide files");
            return;
        }

        toast.success("Files hidden successfully");
        isHidingFiles = false;
    }

    async function handleShowFiles() {
        isShowingFiles = true; // UI state
        const response = await showFiles();

        if (response.error) {
            console.error(response.error);
            toast.error("Failed to show files");
            return;
        }
        toast.success("Files revealed successfully");
        isShowingFiles = false;
    }

    async function handleResetTos() {
        isResettingTos = true;
        try {
            const { error } = await resetTos();
            if (error) throw new Error("Failed to reset TOS");
            toast.success("TOS acceptance has been reset.");
        } catch (err) {
            console.error("Failed to reset TOS:", err);
            toast.error("Failed to reset TOS acceptance.");
        } finally {
            isResettingTos = false;
        }
    }
</script>

<Sheet.Root bind:open>
    <Sheet.Content
        side="right"
        class="w-[400px] sm:w-[540px] overflow-y-auto flex flex-col bg-background/95 backdrop-blur-xl border-l border-white/10"
    >
        <Sheet.Header class="mb-6">
            <Sheet.Title class="text-2xl font-black flex items-center gap-3">
                <Terminal class="size-6 text-amber-500" />
                Developer Tools
            </Sheet.Title>
            <Sheet.Description>
                Advanced tools for testing engine behavior and state
                transitions.
            </Sheet.Description>
        </Sheet.Header>

        <Tabs.Root value="overrides" class="flex-1">
            <Tabs.List
                class="grid w-full grid-cols-1 mb-8 bg-muted/50 p-1 rounded-xl"
            >
                <Tabs.Trigger value="overrides" class="rounded-lg font-bold"
                    >State Overrides</Tabs.Trigger
                >
            </Tabs.List>

            <Tabs.Content value="overrides" class="space-y-6">
                <div
                    class="space-y-4 p-6 rounded-2xl border border-white/5 bg-white/5"
                >
                    <div class="flex items-center gap-2 mb-2">
                        <Bug class="size-4 text-amber-500" />
                        <h3
                            class="text-sm font-bold uppercase tracking-widest text-amber-500"
                        >
                            Global Overrides
                        </h3>
                    </div>

                    <div class="space-y-2">
                        <Label
                            class="text-xs font-bold text-muted-foreground uppercase"
                            >Debug Pro Policy</Label
                        >
                        <Select.Root
                            type="single"
                            value={selectedPolicy}
                            onValueChange={handlePolicyChange}
                        >
                            <Select.Trigger
                                class="h-11 bg-background/50 border-white/10"
                            >
                                {selectedPolicy}
                            </Select.Trigger>
                            <Select.Content>
                                <Select.Item value="Default" label="Default" />
                                <Select.Item value="Free" label="Free" />
                                <Select.Item value="Pro" label="Pro" />
                            </Select.Content>
                        </Select.Root>
                        <p class="text-xs text-muted-foreground italic">
                            Forces the license state. Page will reload upon
                            change.
                        </p>
                    </div>

                    <div class="space-y-2 pt-4">
                        <Label
                            class="text-xs font-bold text-muted-foreground uppercase"
                            >Site Theme</Label
                        >
                        <Select.Root
                            type="single"
                            value={selectedTheme}
                            onValueChange={handleThemeChange}
                        >
                            <Select.Trigger
                                class="h-11 bg-background/50 border-white/10"
                            >
                                <div class="flex items-center gap-2">
                                    {#if selectedTheme === "light"}<Sun
                                            class="w-4 h-4"
                                        />{/if}
                                    {#if selectedTheme === "dark"}<Moon
                                            class="w-4 h-4"
                                        />{/if}
                                    {#if selectedTheme === "system"}<Monitor
                                            class="w-4 h-4"
                                        />{/if}
                                    {themeOptions.find(
                                        (t) => t.value === selectedTheme,
                                    )?.label}
                                </div>
                            </Select.Trigger>
                            <Select.Content>
                                {#each themeOptions as option}
                                    <Select.Item value={option.value}>
                                        <div class="flex items-center gap-2">
                                            <option.icon class="w-4 h-4" />
                                            {option.label}
                                        </div>
                                    </Select.Item>
                                {/each}
                            </Select.Content>
                        </Select.Root>
                        <p class="text-xs text-muted-foreground italic">
                            Changes the application theme immediately.
                        </p>
                    </div>

                    <div class="space-y-2 pt-4">
                        <Label
                            class="text-xs font-bold text-muted-foreground uppercase"
                            >File Visibility</Label
                        >
                        <div class="grid grid-cols-2 gap-2">
                            <Button
                                variant="outline"
                                class="h-10 text-xs font-bold uppercase tracking-wider"
                                disabled={isHidingFiles || isShowingFiles}
                                onclick={handleHideFiles}
                            >
                                {#if isHidingFiles}
                                    <LoaderCircle
                                        class="size-3 animate-spin mr-1"
                                    />
                                    Hiding
                                {:else}
                                    <EyeOff class="size-3 mr-1" />
                                    Hide Files
                                {/if}
                            </Button>
                            <Button
                                variant="outline"
                                class="h-10 text-xs font-bold uppercase tracking-wider"
                                disabled={isHidingFiles || isShowingFiles}
                                onclick={handleShowFiles}
                            >
                                {#if isShowingFiles}
                                    <LoaderCircle
                                        class="size-3 animate-spin mr-1"
                                    />
                                    Showing
                                {:else}
                                    <Eye class="size-3 mr-1" />
                                    Show Files
                                {/if}
                            </Button>
                        </div>
                        <p class="text-xs text-muted-foreground italic">
                            Toggles file visibility in the engine.
                        </p>
                    </div>

                    <div class="space-y-2 pt-4">
                        <Label
                            class="text-xs font-bold text-muted-foreground uppercase"
                            >TOS &amp; Privacy</Label
                        >
                        <Button
                            variant="outline"
                            class="h-10 text-xs font-bold uppercase tracking-wider w-full"
                            disabled={isResettingTos}
                            onclick={handleResetTos}
                        >
                            {#if isResettingTos}
                                <LoaderCircle
                                    class="size-3 animate-spin mr-1"
                                />
                                Resetting...
                            {:else}
                                Reset TOS Acceptance
                            {/if}
                        </Button>
                        <p class="text-xs text-muted-foreground italic">
                            Clears the Terms of Service and Privacy Policy
                            acceptance. The blocking modal will appear again on
                            next app launch.
                        </p>
                    </div>
                </div>

                <div
                    class="p-4 rounded-xl bg-blue-500/10 border border-blue-500/20 flex gap-3"
                >
                    <Info class="size-5 text-blue-500 shrink-0" />
                    <p class="text-xs text-blue-500/80 leading-relaxed">
                        Overrides force engine state for UI layout testing
                        without waiting for normal conditions.
                    </p>
                </div>
            </Tabs.Content>
        </Tabs.Root>
    </Sheet.Content>
</Sheet.Root>
