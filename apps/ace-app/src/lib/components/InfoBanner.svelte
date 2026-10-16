<script lang="ts">
    import type { Component, ComponentType, Snippet } from "svelte";
    import { TriangleAlert, CircleAlert } from "@lucide/svelte";
    import { cn } from "$lib/utils";

    let {
        icon: Icon,
        variant = "warning",
        title,
        class: className,
        children,
        actions,
    }: {
        /** Icon rendered inside the tinted circle. Defaults to a variant icon. */
        icon?: Component<{ class?: string }> | ComponentType;
        /** Visual tone: amber warning or red danger notice. */
        variant?: "warning" | "danger";
        title?: string;
        class?: string;
        /** Banner body text/content. */
        children: Snippet;
        /** Optional right-side actions (e.g. a call-to-action button). */
        actions?: Snippet;
    } = $props();

    const variantStyles = {
        warning: {
            container: "border-amber-500/20 bg-amber-500/5",
            icon: "bg-amber-500/10 text-amber-600 dark:text-amber-400",
            title: "text-amber-600 dark:text-amber-400",
        },
        danger: {
            container: "border-destructive/20 bg-destructive/5",
            icon: "bg-destructive/10 text-destructive",
            title: "text-destructive",
        },
    } as const;

    const ResolvedIcon = $derived(
        Icon ?? (variant === "warning" ? TriangleAlert : CircleAlert),
    );
</script>

<div
    class={cn(
        "flex flex-col md:flex-row items-center justify-between gap-6 rounded-2xl border p-8",
        variantStyles[variant].container,
        className,
    )}
>
    <div class="flex items-center gap-6">
        <div
            class={cn(
                "flex items-center justify-center w-14 h-14 rounded-full shrink-0",
                variantStyles[variant].icon,
            )}
        >
            <ResolvedIcon class="size-7" />
        </div>
        <div class="space-y-1">
            {#if title}
                <h3
                    class={cn(
                        "text-xl font-bold",
                        variantStyles[variant].title,
                    )}
                >
                    {title}
                </h3>
            {/if}
            <div
                class="text-sm text-muted-foreground max-w-xl leading-relaxed"
            >
                {@render children()}
            </div>
        </div>
    </div>
    {#if actions}
        <div class="shrink-0">{@render actions()}</div>
    {/if}
</div>
