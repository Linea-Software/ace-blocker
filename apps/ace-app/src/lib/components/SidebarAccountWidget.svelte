<script lang="ts">
    import * as Sidebar from "$lib/components/ui/sidebar";
    import * as Dialog from "$lib/components/ui/dialog";
    import { Button } from "$lib/components/ui/button";
    import {
        Crown,
        CircleUserRound,
        CircleHelp,
        LoaderCircle,
        Sparkles,
        CircleCheck,
    } from "@lucide/svelte";
    import { userStore } from "$lib/stores/user-store.svelte";
    import * as m from "$lib/paraglide/messages.js";
    import { cn } from "$lib/utils.js";
    import { dev } from "$app/env";
    import { openUrl } from "@tauri-apps/plugin-opener";
    import { engineStore } from "$lib/stores/engine.svelte";

    let { isLogoutConfirmOpen = $bindable(false) } = $props();

    let isPro = $derived(userStore.isPro);

    let isActivationModalOpen = $state(false);

    async function handleSignIn() {
        const baseUrl = dev
            ? "http://localhost:5173"
            : "https://aceblocker.com";
        const url = `${baseUrl}/signin?origin=ace-engine`;
        await openUrl(url);
        isActivationModalOpen = false;
    }

    async function handleUpgrade() {
        const baseUrl = dev
            ? "http://localhost:5173"
            : "https://aceblocker.com";
        const url = `${baseUrl}/get-pro`;
        await openUrl(url);
        isActivationModalOpen = false;
    }

    const proFeatures = [
        m.pro_feature_cloud_backup(),
        m.pro_feature_premium_lists(),
        m.pro_feature_advanced_friction(),
    ];

    // The product does not keep personal profile data, so this chip is a
    // licence/plan entry point, not a user profile: the icon and its
    // accessible name must say so (the widget opens Pro activation or the
    // plan status, never an "account" surface).
    const accountState = $derived.by(() => {
        if (!engineStore.isReady) {
            return {
                label: m.plan_status_checking(),
                description: m.plan_status_loading(),
                tooltip: m.plan_status_loading(),
                icon: LoaderCircle,
                iconClass: "animate-spin text-muted-foreground",
            };
        }
        if (isPro) {
            return {
                label: m.plan_premium(),
                description: engineStore.isAlive
                    ? m.all_features_available()
                    : m.plan_status_offline_cached(),
                tooltip: m.plan_status_pro_tooltip(),
                icon: Crown,
                iconClass: "text-amber-500",
            };
        }
        if (!engineStore.isAlive) {
            return {
                label: m.plan_status_unavailable(),
                description: m.plan_status_offline(),
                tooltip: m.plan_status_offline(),
                icon: CircleHelp,
                iconClass: "text-muted-foreground",
            };
        }
        return {
            label: m.plan_free(),
            description: m.sidebar_upgrade_text(),
            tooltip: m.plan_status_free_tooltip(),
            icon: CircleUserRound,
            iconClass: "text-muted-foreground",
        };
    });
</script>

<Sidebar.MenuItem>
    <Sidebar.MenuButton
        size="lg"
        tooltipContent={accountState.tooltip}
        onclick={() => {
            if (engineStore.isReady && !isPro) isActivationModalOpen = true;
        }}
        class="hover:bg-sidebar-accent hover:text-sidebar-accent-foreground transition-colors"
        aria-label={`${m.plan_and_activation()}: ${accountState.label}`}
    >
        <div
            class="border-border bg-card flex aspect-square size-8 items-center justify-center rounded-lg border"
        >
            <accountState.icon
                class={cn("size-4", accountState.iconClass)}
                aria-hidden="true"
            />
        </div>
        <div
            class="grid flex-1 text-left text-sm leading-tight group-data-[collapsible=icon]:hidden"
        >
            <span
                class={cn("truncate font-semibold", isPro && "text-amber-500")}
            >
                {accountState.label}
            </span>
            <span class="text-muted-foreground truncate text-xs">
                {accountState.description}
            </span>
        </div>
    </Sidebar.MenuButton>
</Sidebar.MenuItem>

<!-- Activation Modal -->
<Dialog.Root bind:open={isActivationModalOpen}>
    <Dialog.Content
        class="border-border/20 max-h-[90dvh] rounded-3xl shadow-2xl backdrop-blur-2xl sm:max-w-[min(420px,calc(100vw-2rem))]"
    >
        <Dialog.Header class="items-center pt-4">
            <div
                class="mb-4 flex h-12 w-12 items-center justify-center rounded-2xl bg-amber-500/10 text-amber-500"
            >
                <Sparkles class="size-6" />
            </div>
            <Dialog.Title
                class="text-center text-2xl font-black tracking-tight"
            >
                {m.unlock_pro_title()}
            </Dialog.Title>
        </Dialog.Header>

        <div class="space-y-4 py-4">
            <ul class="space-y-3">
                {#each proFeatures as feature (feature)}
                    <li class="flex items-center gap-3 text-sm font-medium">
                        <div
                            class="flex h-5 w-5 items-center justify-center rounded-full bg-emerald-500/10 text-emerald-500"
                        >
                            <CircleCheck class="size-3.5" />
                        </div>
                        {feature}
                    </li>
                {/each}
            </ul>

            <div class="space-y-3 pt-4">
                <Button
                    class="h-12 w-full rounded-2xl bg-amber-500 font-black text-white shadow-lg shadow-amber-500/20 transition-all hover:scale-[1.02] hover:bg-amber-600 active:scale-[0.98]"
                    onclick={handleUpgrade}
                >
                    {m.get_pro()}
                </Button>

                <div class="text-center">
                    <Button
                        variant="ghost"
                        size="sm"
                        class="text-muted-foreground hover:text-foreground text-xs transition-colors"
                        onclick={handleSignIn}
                    >
                        {m.already_purchased()}
                    </Button>
                </div>
            </div>
        </div>
    </Dialog.Content>
</Dialog.Root>
