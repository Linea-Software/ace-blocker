import { createSignal, onSettled, Show } from "solid-js";
import { Bug } from "@lucide/solid";
import {
    setDebugProPolicy,
    getDebugProPolicy,
    type DebugProPolicy,
} from "$engine-client";
import { toast } from "solid-sonner";

export default function DebugLicenseSelector() {
    const [selectedPolicy, setSelectedPolicy] =
        createSignal<DebugProPolicy>("Default");
    onSettled(() => {
        if (!import.meta.env.DEV) return;
        let active = true;
        void getDebugProPolicy()
            .then((response) => {
                if (active && response.data) setSelectedPolicy(response.data);
            })
            .catch((error) =>
                console.error("Failed to fetch debug pro policy:", error),
            );
        return () => {
            active = false;
        };
    });
    async function handlePolicyChange(value: string) {
        const policy = value as DebugProPolicy;
        if (policy === selectedPolicy()) return;
        setSelectedPolicy(policy);
        try {
            const response = await setDebugProPolicy({ path: { policy } });
            if (response.error) throw response.error;
            toast.success(`Pro policy set to ${policy}`);
            window.location.reload();
        } catch (error) {
            console.error("Failed to set debug pro policy:", error);
            toast.error("Failed to set debug pro policy");
        }
    }
    return (
        <Show when={import.meta.env.DEV}>
            <li class="group/menu-item relative">
                <div class="px-2 py-1.5 border-2 border-dashed border-border rounded-md my-1 bg-muted/50">
                    <div class="flex items-center gap-2 mb-2 px-1">
                        <Bug class="size-3 text-muted-foreground" />
                        <span class="text-xs uppercase font-bold text-muted-foreground tracking-wider">
                            Debug Pro Policy
                        </span>
                    </div>
                    <select
                        class="h-7 w-full text-xs bg-background/50 border border-border rounded-md focus:ring-ring"
                        value={selectedPolicy()}
                        onChange={(e) =>
                            void handlePolicyChange(e.currentTarget.value)
                        }
                    >
                        <option value="Default">Default</option>
                        <option value="Free">Free</option>
                        <option value="Pro">Pro</option>
                    </select>
                </div>
            </li>
        </Show>
    );
}
