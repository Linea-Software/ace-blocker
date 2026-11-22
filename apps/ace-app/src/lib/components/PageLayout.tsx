import type { JSX } from "@solidjs/web";
import { cn } from "$lib/utils";

export default function PageLayout(props: {
  width?: "wide" | "narrow" | "dashboard";
  header: JSX.Element;
  children: JSX.Element;
  class?: string;
  [key: string]: unknown;
}) {
  return (
    <div class={cn("page-layout", props.width === "narrow" && "page-layout--narrow", props.width === "dashboard" && "page-layout--dashboard", props.class)}>
      <div class="page-header-slot">{props.header}</div>
      <div class="page-content">{props.children}</div>
      <style>{`
        .page-layout { width: min(1120px, calc(100% - 44px)); margin: 0 auto; padding: 30px 0 64px; container-type: inline-size; container-name: page; }
        .page-layout--narrow { max-width: 920px; }
        .page-layout--dashboard { max-width: 1280px; }
        .page-header-slot { padding-bottom: 18px; border-bottom: 1px solid var(--border); }
        .page-content { padding-top: 18px; }
        @container ace-workspace (max-width: 760px) {
          .page-layout { width: calc(100% - 28px); padding-top: 24px; }
        }
      `}</style>
    </div>
  );
}
