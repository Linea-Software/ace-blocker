import type { JSX } from "@solidjs/web";
import { Portal as SolidPortal } from "@solidjs/web";
import {
    createContext,
    createSignal,
    createUniqueId,
    onSettled,
    useContext,
    Show,
} from "solid-js";
import { cn } from "$lib/utils.js";
type Side = "top" | "bottom" | "left" | "right";
type Align = "start" | "center" | "end";
type Ctx = {
    open: () => boolean;
    setOpen: (v: boolean) => void;
    anchor: () => HTMLElement | undefined;
    setAnchor: (v: HTMLElement) => void;
    id: string;
};
const Context = createContext<Ctx>();
function usePopover() {
    const ctx = useContext(Context);
    if (!ctx) throw Error("Popover.Root required");
    return ctx;
}
export function Root(props: {
    children?: JSX.Element;
    open?: boolean;
    defaultOpen?: boolean;
    onOpenChange?: (v: boolean) => void;
}) {
    const [own, setOwn] = createSignal(props.defaultOpen ?? false);
    const [anchor, setAnchor] = createSignal<HTMLElement>();
    const ctx: Ctx = {
        open: () => props.open ?? own(),
        setOpen: (v) => {
            setOwn(v);
            props.onOpenChange?.(v);
        },
        anchor,
        setAnchor,
        id: createUniqueId(),
    };
    return <Context value={ctx}>{props.children}</Context>;
}
export function Trigger(
    props: JSX.ButtonHTMLAttributes<HTMLButtonElement> & { asChild?: boolean },
) {
    const ctx = usePopover();
    return (
        <button
            {...props}
            ref={(el) => ctx.setAnchor(el)}
            data-slot="popover-trigger"
            type="button"
            aria-haspopup="dialog"
            aria-expanded={ctx.open() ? "true" : "false"}
            aria-controls={ctx.id}
            onClick={(e) => {
                (props.onClick as ((e: MouseEvent) => void) | undefined)?.(e);
                if (!e.defaultPrevented) ctx.setOpen(!ctx.open());
            }}
            class={cn("", props.class)}
        >
            {props.children}
        </button>
    );
}
export function Portal(props: { children?: JSX.Element }) {
    return <>{props.children}</>;
}
function place(
    anchor: HTMLElement | undefined,
    side: Side,
    align: Align,
    offset: number,
    content: HTMLElement,
) {
    if (!anchor) return { top: 0, left: 0 };
    const rect = anchor.getBoundingClientRect(),
        cw = content.offsetWidth,
        ch = content.offsetHeight;
    let top =
        side === "top"
            ? rect.top - ch - offset
            : side === "bottom"
              ? rect.bottom + offset
              : side === "left"
                ? rect.top
                : rect.top;
    let left =
        side === "left"
            ? rect.left - cw - offset
            : side === "right"
              ? rect.right + offset
              : rect.left;
    if (side === "top" || side === "bottom")
        left =
            align === "end"
                ? rect.right - cw
                : align === "center"
                  ? rect.left + (rect.width - cw) / 2
                  : rect.left;
    else
        top =
            align === "end"
                ? rect.bottom - ch
                : align === "center"
                  ? rect.top + (rect.height - ch) / 2
                  : rect.top;
    return {
        top: Math.max(8, Math.min(window.innerHeight - ch - 8, top)),
        left: Math.max(8, Math.min(window.innerWidth - cw - 8, left)),
    };
}
type P = JSX.HTMLAttributes<HTMLDivElement> & {
    side?: Side;
    align?: Align;
    sideOffset?: number;
    collisionPadding?: number;
    portalProps?: unknown;
};
export function Content(props: P) {
    const ctx = usePopover();
    let el!: HTMLDivElement;
    const [pos, setPos] = createSignal({ top: 0, left: 0 });
    onSettled(() => {
        const update = () => {
            if (el)
                setPos(
                    place(
                        ctx.anchor(),
                        props.side ?? "bottom",
                        props.align ?? "center",
                        props.sideOffset ?? 4,
                        el,
                    ),
                );
        };
        const outside = (e: PointerEvent) => {
            if (
                !el?.contains(e.target as Node) &&
                !ctx.anchor()?.contains(e.target as Node)
            )
                ctx.setOpen(false);
        };
        update();
        window.addEventListener("resize", update);
        window.addEventListener("scroll", update, true);
        document.addEventListener("pointerdown", outside);
        return () => {
            window.removeEventListener("resize", update);
            window.removeEventListener("scroll", update, true);
            document.removeEventListener("pointerdown", outside);
        };
    });
    return (
        <Show when={ctx.open()}>
            <SolidPortal>
                <div
                    ref={el}
                    {...props}
                    id={ctx.id}
                    data-slot="popover-content"
                    data-side={props.side ?? "bottom"}
                    data-state="open"
                    role="dialog"
                    tabindex={-1}
                    onKeyDown={(e) => {
                        if (e.key === "Escape") {
                            e.preventDefault();
                            ctx.setOpen(false);
                            ctx.anchor()?.focus();
                        }
                    }}
                    style={{
                        position: "fixed",
                        top: pos().top + "px",
                        left: pos().left + "px",
                    }}
                    class={cn(
                        "bg-popover text-popover-foreground data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-end-2 data-[side=right]:slide-in-from-start-2 data-[side=top]:slide-in-from-bottom-2 z-50 w-72 origin-(--bits-popover-content-transform-origin) rounded-md border p-4 shadow-md outline-hidden",
                        props.class,
                    )}
                >
                    {props.children}
                </div>
            </SolidPortal>
        </Show>
    );
}
export function Close(props: JSX.ButtonHTMLAttributes<HTMLButtonElement>) {
    const ctx = usePopover();
    return (
        <button
            {...props}
            type="button"
            data-slot="popover-close"
            onClick={() => ctx.setOpen(false)}
        >
            {props.children}
        </button>
    );
}
export {
    Root as Popover,
    Trigger as PopoverTrigger,
    Portal as PopoverPortal,
    Content as PopoverContent,
    Close as PopoverClose,
};
