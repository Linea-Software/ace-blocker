<script lang="ts">
    import * as Sidebar from "$lib/components/ui/sidebar/index.js";
    import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
    import { Badge } from "$lib/components/ui/badge";
    import * as AlertDialog from "$lib/components/ui/alert-dialog";
    import {
        ShieldCheck,
        Settings,
        List,
        Flower2,
        LifeBuoy,
        Sparkles,
        User,
        ChevronsUpDown,
        LogOut,
        Activity,
        Terminal,
        Globe,
        Compass,
    } from "@lucide/svelte";

    import { page } from "$app/state";
    import * as m from "$lib/paraglide/messages.js";
    import { cn } from "$lib/utils.js";
    import { toast } from "svelte-sonner";
    import { openUrl } from "@tauri-apps/plugin-opener";
    import { logout as accountLogout } from "$engine-client";
    import { engineStore } from "$lib/stores/engine.svelte";
    import { userStore } from "$lib/stores/user-store.svelte";
    import { dev } from "$app/env";
    import YinYang from "@tabler/icons-svelte/icons/yin-yang";
    import DevToolsSheet from "./DevToolsSheet.svelte";
    import { Button } from "./ui/button";
    import SidebarAccountWidget from "./SidebarAccountWidget.svelte";

    let isDevToolsOpen = $state(false);

    async function handleLogout() {
        try {
            const { error } = await accountLogout();
            if (error) {
                console.error("Logout failed:", error);
                toast.error(m.failed_to_sync());
                return;
            }
            // The engine cleared its API key; mirror the account transition
            // and close the confirm dialog only on success.
            await userStore.refreshAfterLoginOrEntitlementChange();
            isLogoutConfirmOpen = false;
        } catch (error) {
            console.error("Logout failed:", error);
        }
    }

    let isLogoutConfirmOpen = $state(false);
    let engineActive = $derived(engineStore.isAlive);

    $effect(() => {
        if (engineStore.isAlive) {
            // Engine (re)connected: entitlement may have changed while it was
            // down — force a remote refresh (no-op when logged out).
            userStore.refreshAfterLoginOrEntitlementChange();
        }
    });

    // Navigation items
    const navCategories = [
        {
            label: m.navigation(),
            items: [
                {
                    // Network Blocks is the primary control surface: it is the
                    // first nav item, while the dashboard stays the landing page.
                    // ShieldCheck is the canonical glyph (shared with the page
                    // header, empty state and rows — never a second icon).
                    title: m.network_blocks(),
                    url: "/network-blocks",
                    icon: ShieldCheck,
                    tourId: "nav-network-blocks",
                },
                {
                    title: m.dashboard(),
                    url: "/",
                    icon: Activity,
                    tourId: "nav-dashboard",
                },
                {
                    title: m.resistance_mode(),
                    url: "/resistance-mode",
                    icon: YinYang,
                    tourId: "nav-resistance-mode",
                },
            ],
        },
        {
            label: m.advanced(),
            items: [
                {
                    title: m.site_packs(),
                    url: "/site-packs",
                    icon: Globe,
                    tourId: "nav-site-packs",
                },
            ],
        },
    ];

    const secondaryNav = [
        {
            title: m.getting_started(),
            url: "/getting-started",
            icon: Compass,
            tourId: "nav-getting-started",
        },
        {
            title: m.help_support(),
            url: "/help",
            icon: LifeBuoy,
            tourId: "nav-help",
        },
        {
            title: m.settings(),
            url: "/settings",
            icon: Settings,
            tourId: "nav-settings",
        },
    ];
</script>

<Sidebar.Root collapsible="icon">
    <Sidebar.Header>
        <Sidebar.Menu>
            <SidebarAccountWidget bind:isLogoutConfirmOpen />
        </Sidebar.Menu>
    </Sidebar.Header>

    <Sidebar.Content>
        {#each navCategories as category (category.label)}
            <Sidebar.Group>
                <Sidebar.GroupLabel>{category.label}</Sidebar.GroupLabel>
                <Sidebar.GroupContent>
                    <Sidebar.Menu>
                        {#each category.items as item (item.title)}
                            {@const isActive =
                                item.url === "/"
                                    ? page.url.pathname === "/"
                                    : page.url.pathname.startsWith(item.url)}
                            <Sidebar.MenuItem>
                                <Sidebar.MenuButton
                                    {isActive}
                                    tooltipContent={item.title}
                                    class="transition-all duration-200"
                                >
                                    {#snippet child({ props })}
                                        <a
                                            href={item.url}
                                            data-tour={item.tourId}
                                            {...props}
                                        >
                                            <item.icon
                                                class={cn(
                                                    isActive
                                                        ? "text-primary"
                                                        : "text-muted-foreground",
                                                )}
                                            />
                                            <span>{item.title}</span>
                                        </a>
                                    {/snippet}
                                </Sidebar.MenuButton>
                                {#if isActive}
                                    <div
                                        class="absolute left-0 top-1.5 bottom-1.5 w-1 bg-primary rounded-r-full group-data-[collapsible=icon]:hidden"
                                    ></div>
                                {/if}
                            </Sidebar.MenuItem>
                        {/each}
                    </Sidebar.Menu>
                </Sidebar.GroupContent>
            </Sidebar.Group>
        {/each}
    </Sidebar.Content>

    <Sidebar.Footer>
        <!-- Engine Status Indicator -->
        <Sidebar.Menu>
            <Sidebar.MenuItem>
                <div
                    class="px-2 py-1.5 flex items-center gap-2 group-data-[collapsible=icon]:justify-center"
                >
                    <div
                        class="relative flex h-2.5 w-2.5 items-center justify-center"
                    >
                        <span
                            class={cn(
                                "absolute inline-flex h-full w-full animate-ping rounded-full opacity-75 duration-1000",
                                engineActive ? "bg-emerald-400" : "hidden",
                            )}
                        ></span>
                        <span
                            class={cn(
                                "relative inline-flex rounded-full h-2 w-2",
                                engineActive
                                    ? "bg-emerald-500"
                                    : "bg-destructive",
                            )}
                        ></span>
                    </div>
                    <span
                        class="text-xs font-medium text-muted-foreground uppercase tracking-wider group-data-[collapsible=icon]:hidden"
                    >
                        {engineActive ? m.system_active() : m.system_inactive()}
                    </span>
                </div>
            </Sidebar.MenuItem>

            <Sidebar.Separator class="my-2" />

            {#each secondaryNav as item (item.title)}
                <Sidebar.MenuItem>
                    <Sidebar.MenuButton
                        isActive={page.url.pathname === item.url}
                        tooltipContent={item.title}
                    >
                        {#snippet child({ props })}
                            <a
                                href={item.url}
                                data-tour={item.tourId}
                                {...props}
                            >
                                <item.icon class="text-muted-foreground" />
                                <span class="truncate">{item.title}</span>
                            </a>
                        {/snippet}
                    </Sidebar.MenuButton>
                </Sidebar.MenuItem>
            {/each}
        </Sidebar.Menu>

        <div class="mt-auto flex items-center justify-between px-3 py-2">
            {#if dev}
                <Button
                    variant="ghost"
                    size="icon"
                    class="h-8 w-8 text-muted-foreground/40 hover:text-amber-500 hover:bg-amber-500/10 transition-colors"
                    onclick={() => (isDevToolsOpen = true)}
                >
                    <Terminal class="size-4" />
                </Button>
            {:else}
                <div></div>
            {/if}
            <Sidebar.Trigger
                class="text-muted-foreground/50 hover:text-muted-foreground transition-colors"
            />
        </div>
    </Sidebar.Footer>
    <Sidebar.Rail />
</Sidebar.Root>

<DevToolsSheet bind:open={isDevToolsOpen} />

<!-- Logout Confirmation Dialog -->
<AlertDialog.Root bind:open={isLogoutConfirmOpen}>
    <AlertDialog.Content
        class="rounded-3xl border-border/20 shadow-2xl backdrop-blur-2xl"
    >
        <AlertDialog.Header>
            <AlertDialog.Title class="text-2xl font-black tracking-tight"
                >{m.logout_confirmation_title()}</AlertDialog.Title
            >
            <AlertDialog.Description class="text-base font-medium opacity-80">
                {m.logout_confirmation_description()}
            </AlertDialog.Description>
        </AlertDialog.Header>
        <AlertDialog.Footer class="mt-6 gap-3">
            <AlertDialog.Cancel
                class="rounded-2xl h-12 px-6 font-bold hover:bg-muted/50 border-0 transition-colors"
            >
                {m.cancel()}
            </AlertDialog.Cancel>
            <AlertDialog.Action
                onclick={handleLogout}
                class="rounded-2xl h-12 px-8 font-black bg-destructive hover:bg-destructive/90 text-destructive-foreground shadow-lg shadow-destructive/20 transition-all hover:scale-[1.02] active:scale-[0.98]"
            >
                {m.log_out()}
            </AlertDialog.Action>
        </AlertDialog.Footer>
    </AlertDialog.Content>
</AlertDialog.Root>

<style>
    /* Remove resize cursor and click-to-toggle from the sidebar rail.
       The rail's built-in toggle behavior is replaced by the manual
       Sidebar.Trigger button added to the sidebar footer. */
    :global([data-sidebar="rail"]) {
        cursor: default !important;
        pointer-events: none !important;
    }
</style>
