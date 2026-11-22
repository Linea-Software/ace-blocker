import type { JSX } from "@solidjs/web";
import { Show } from "solid-js";
import { cn } from "$lib/utils";

export default function PageHeader(props: {
    title: string;
    subtitle?: string;
    align?: "center" | "end";
    mobileCentered?: boolean;
    class?: string;
    children?: JSX.Element;
}) {
    return (
        <div
            class={cn(
                "page-header",
                (props.align ?? "end") === "end"
                    ? "page-header--end"
                    : "page-header--center",
                props.class,
            )}
        >
            <div
                class={cn(
                    "page-header__identity",
                    props.mobileCentered &&
                        "page-header__identity--mobile-centered",
                )}
            >
                <div class="page-header__copy">
                    <h1>{props.title}</h1>
                    <Show when={props.subtitle}>
                        <p>{props.subtitle}</p>
                    </Show>
                </div>
            </div>
            <Show when={props.children}>
                <div class="page-header__actions">{props.children}</div>
            </Show>
            <style>{`
        .page-header { display: flex; justify-content: space-between; gap: 24px; }
        .page-header--end { align-items: flex-end; }
        .page-header--center { align-items: center; }
        .page-header__identity, .page-header__copy { min-width: 0; }
        .page-header h1 { margin: 0; font-family: "IBM Plex Sans Condensed", sans-serif; font-size: clamp(32px, 5cqi, 42px); font-weight: 600; line-height: 0.98; letter-spacing: -0.02em; }
        .page-header p { max-width: 680px; margin: 8px 0 0; color: var(--muted-foreground); font-size: 14px; line-height: 1.5; }
        .page-header__actions { flex: 0 0 auto; display: flex; align-items: center; gap: 8px; }
        @container page (max-width: 740px) {
          .page-header { align-items: stretch; flex-direction: column; gap: 16px; }
          .page-header__actions { width: 100%; flex-wrap: wrap; }
        }
      `}</style>
        </div>
    );
}
