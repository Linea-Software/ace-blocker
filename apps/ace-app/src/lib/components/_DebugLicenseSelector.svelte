<script lang="ts">
    import { dev } from "$app/env";
    import { Bug } from "@lucide/svelte";
    import * as Sidebar from "$lib/components/ui/sidebar/index.js";
    import * as Select from "$lib/components/ui/select/index.js";
    import { setDebugProPolicy, getDebugProPolicy } from "$engine-client";
    import type { DebugProPolicy } from "$engine-client";
    import { toast } from "svelte-sonner";
    import { onMount } from "svelte";

    let selectedPolicy = $state<DebugProPolicy>("Default");

    onMount(async () => {
        try {
            const response = await getDebugProPolicy();
            if (response.data) {
                selectedPolicy = response.data;
            }
        } catch (error) {
            console.error("Failed to fetch debug pro policy:", error);
        }
    });

    async function handlePolicyChange(newPolicy: string) {
        const policy = newPolicy as DebugProPolicy;
        if (policy === selectedPolicy) return;

        selectedPolicy = policy;
        try {
            await setDebugProPolicy({
                path: { policy: policy },
            });
            toast.success(`Pro policy set to ${policy}`);
            // Reload to apply changes
            window.location.reload();
        } catch (error) {
            console.error("Failed to set debug pro policy:", error);
            toast.error("Failed to set debug pro policy");
        }
    }
</script>

{#if dev}
    <Sidebar.MenuItem>
        <div
            class="px-2 py-1.5 border-2 border-dashed border-amber-500/30 rounded-md my-1 bg-amber-500/5"
        >
            <div class="flex items-center gap-2 mb-2 px-1">
                <Bug class="size-3 text-amber-500" />
                <span
                    class="text-xs uppercase font-bold text-amber-500 tracking-wider"
                    >Debug Pro Policy</span
                >
            </div>

            <Select.Root
                type="single"
                value={selectedPolicy}
                onValueChange={(v) => handlePolicyChange(v)}
            >
                <Select.Trigger
                    class="h-7 text-xs bg-background/50 border-amber-500/20 focus:ring-amber-500/20"
                >
                    <span class="truncate">{selectedPolicy}</span>
                </Select.Trigger>
                <Select.Content>
                    <Select.Item
                        value="Default"
                        label="Default"
                        class="text-xs"
                    />
                    <Select.Item value="Free" label="Free" class="text-xs" />
                    <Select.Item value="Pro" label="Pro" class="text-xs" />
                </Select.Content>
            </Select.Root>
        </div>
    </Sidebar.MenuItem>
{/if}
