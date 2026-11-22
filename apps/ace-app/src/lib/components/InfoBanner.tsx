import type { JSX } from "@solidjs/web";
import type { Component } from "solid-js";
import { Dynamic } from "@solidjs/web";
import { Show } from "solid-js";
import { TriangleAlert, CircleAlert } from "@lucide/solid";
import { cn } from "$lib/utils";

const variantStyles = {
  warning: { container: "border-amber-500/20 bg-amber-500/5", icon: "bg-amber-500/10 text-amber-600 dark:text-amber-400", title: "text-amber-600 dark:text-amber-400" },
  danger: { container: "border-destructive/20 bg-destructive/5", icon: "bg-destructive/10 text-destructive", title: "text-destructive" },
} as const;

export default function InfoBanner(props: {
  icon?: Component<{ class?: string }>;
  variant?: "warning" | "danger";
  title?: string;
  class?: string;
  children: JSX.Element;
  actions?: JSX.Element;
}) {
  const style = () => variantStyles[props.variant ?? "warning"];
  return (
    <div class={cn("flex flex-col md:flex-row items-center justify-between gap-6 rounded-2xl border p-8", style().container, props.class)}>
      <div class="flex items-center gap-6">
        <div class={cn("flex items-center justify-center w-14 h-14 rounded-full shrink-0", style().icon)}>
          <Dynamic component={props.icon ?? (props.variant === "danger" ? CircleAlert : TriangleAlert)} class="size-7" />
        </div>
        <div class="space-y-1">
          <Show when={props.title}><h3 class={cn("text-xl font-bold", style().title)}>{props.title}</h3></Show>
          <div class="text-sm text-muted-foreground max-w-xl leading-relaxed">{props.children}</div>
        </div>
      </div>
      <Show when={props.actions}><div class="shrink-0">{props.actions}</div></Show>
    </div>
  );
}
