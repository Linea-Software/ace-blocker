import type { JSX } from "@solidjs/web";
import { Show } from "solid-js";
import { cn } from "$lib/utils";

export default function PageCard(props: {
  title?: string;
  class?: string;
  children: JSX.Element;
}) {
  return (
    <div class={cn("rounded-2xl border border-border bg-card p-6", props.class)}>
      <Show when={props.title}>
        <p class="text-xs font-bold uppercase tracking-widest text-muted-foreground">{props.title}</p>
      </Show>
      {props.children}
    </div>
  );
}
