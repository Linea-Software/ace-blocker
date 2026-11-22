import type { JSX } from "@solidjs/web";
import { Portal as SolidPortal } from "@solidjs/web";
import {
    createContext,
    createSignal,
    createUniqueId,
    useContext,
    Show,
    onSettled,
} from "solid-js";
import { cn } from "$lib/utils.js";
type Ctx = {
    open: () => boolean;
    setOpen: (v: boolean) => void;
    anchor: () => HTMLElement | undefined;
    setAnchor: (e: HTMLElement) => void;
    id: string;
    delay: number;
};
const Context = createContext<Ctx>(),
    ProviderContext = createContext(700);
function useTooltip() {
    const v = useContext(Context);
    if (!v) throw Error("Tooltip.Root required");
    return v;
}
export function Provider(props: {
    children?: JSX.Element;
    delayDuration?: number;
    skipDelayDuration?: number;
}) {
    return (
        <ProviderContext value={props.delayDuration ?? 700}>
            {props.children}
        </ProviderContext>
    );
}
export function Root(props: {
    children?: JSX.Element;
    open?: boolean;
    defaultOpen?: boolean;
    onOpenChange?: (v: boolean) => void;
}) {
    const delay = useContext(ProviderContext),
        [own, setOwn] = createSignal(props.defaultOpen ?? false),
        [anchor, setAnchor] = createSignal<HTMLElement>();
    const ctx: Ctx = {
        open: () => props.open ?? own(),
        setOpen: (v) => {
            setOwn(v);
            props.onOpenChange?.(v);
        },
        anchor,
        setAnchor,
        id: createUniqueId(),
        delay,
    };
    return <Context value={ctx}>{props.children}</Context>;
}
export function Trigger(props: JSX.ButtonHTMLAttributes<HTMLButtonElement>) {
    const ctx = useTooltip();
    let timer: ReturnType<typeof setTimeout> | undefined;
    const dismiss = () => {
        clearTimeout(timer);
        ctx.setOpen(false);
    };
    onSettled(() => () => clearTimeout(timer));
    return (
        <button
            {...props}
            ref={(el) => ctx.setAnchor(el)}
            data-slot="tooltip-trigger"
            type="button"
            aria-describedby={ctx.open() ? ctx.id : undefined}
            onMouseEnter={() => {
                timer = setTimeout(() => ctx.setOpen(true), ctx.delay);
            }}
            onMouseLeave={dismiss}
            onFocus={() => ctx.setOpen(true)}
            onBlur={dismiss}
            onKeyDown={(e) => {
                if (e.key === "Escape") dismiss();
            }}
        >
            {props.children}
        </button>
    );
}
export function Portal(props: { children?: JSX.Element }) {
    return <>{props.children}</>;
}
export function Content(
    props: JSX.HTMLAttributes<HTMLDivElement> & {
        side?: "top" | "bottom" | "left" | "right";
        align?: "start" | "center" | "end";
        sideOffset?: number;
        arrowClasses?: string;
        portalProps?: unknown;
    },
) {
    const ctx = useTooltip(),
        [pos, setPos] = createSignal({ top: 0, left: 0 });
    let node!: HTMLDivElement;
    const reposition = () => {
        const trigger = ctx.anchor();
        if (!trigger || !node) return;
        const rect = trigger.getBoundingClientRect(),
            side = props.side ?? "top",
            offset = props.sideOffset ?? 6,
            w = node.offsetWidth,
            h = node.offsetHeight;
        const top =
            side === "bottom"
                ? rect.bottom + offset
                : side === "top"
                  ? rect.top - h - offset
                  : rect.top + (rect.height - h) / 2;
        const left =
            side === "left"
                ? rect.left - w - offset
                : side === "right"
                  ? rect.right + offset
                  : rect.left + (rect.width - w) / 2;
        setPos({
            top: Math.min(Math.max(8, top), innerHeight - h - 8),
            left: Math.min(Math.max(8, left), innerWidth - w - 8),
        });
    };
    onSettled(() => {
        window.addEventListener("scroll", reposition, true);
        window.addEventListener("resize", reposition);
        return () => {
            window.removeEventListener("scroll", reposition, true);
            window.removeEventListener("resize", reposition);
        };
    });
    return (
        <Show when={ctx.open() && !props.hidden}>
            <SolidPortal>
                <div
                    ref={(el) => {
                        node = el;
                        requestAnimationFrame(reposition);
                    }}
                    {...props}
                    id={ctx.id}
                    data-slot="tooltip-content"
                    role="tooltip"
                    data-side={props.side ?? "top"}
                    data-state="open"
                    style={{
                        position: "fixed",
                        top: pos().top + "px",
                        left: pos().left + "px",
                    }}
                    class={cn(
                        "bg-foreground text-background animate-in fade-in-0 zoom-in-95 data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95 data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-end-2 data-[side=right]:slide-in-from-start-2 data-[side=top]:slide-in-from-bottom-2 z-50 w-fit origin-(--bits-tooltip-content-transform-origin) rounded-md px-3 py-1.5 text-xs text-balance",
                        props.class,
                    )}
                >
                    {props.children}
                    <div
                        aria-hidden="true"
                        class={cn(
                            "bg-foreground absolute size-2.5 rotate-45 rounded-[2px]",
                            props.arrowClasses,
                        )}
                    />
                </div>
            </SolidPortal>
        </Show>
    );
}
export {
    Root as Tooltip,
    Trigger as TooltipTrigger,
    Content as TooltipContent,
    Provider as TooltipProvider,
    Portal as TooltipPortal,
};
