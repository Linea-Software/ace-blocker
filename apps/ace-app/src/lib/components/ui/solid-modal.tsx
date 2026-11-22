import type { JSX } from "@solidjs/web";
import { Portal } from "@solidjs/web";
import {
    createContext,
    createSignal,
    createUniqueId,
    onSettled,
    useContext,
    Show,
} from "solid-js";
import { tv } from "tailwind-variants";
import { cn } from "$lib/utils.js";
import * as m from "$lib/paraglide/messages.js";

type Kind = "dialog" | "alert-dialog" | "sheet";
type RProps = {
    children?: JSX.Element;
    open?: boolean;
    defaultOpen?: boolean;
    onOpenChange?: (open: boolean) => void;
    modal?: boolean;
};
type ContextValue = {
    open: () => boolean;
    setOpen: (v: boolean) => void;
    id: string;
    kind: Kind;
};
export type Side = "top" | "bottom" | "left" | "right";
export const sheetVariants = tv({
    base: "bg-background data-[state=open]:animate-in data-[state=closed]:animate-out fixed z-50 flex flex-col gap-4 shadow-lg transition ease-in-out data-[state=closed]:duration-300 data-[state=open]:duration-500",
    variants: {
        side: {
            top: "data-[state=closed]:slide-out-to-top data-[state=open]:slide-in-from-top inset-x-0 top-0 h-auto border-b",
            bottom: "data-[state=closed]:slide-out-to-bottom data-[state=open]:slide-in-from-bottom inset-x-0 bottom-0 h-auto border-t",
            left: "data-[state=closed]:slide-out-to-start data-[state=open]:slide-in-from-start inset-y-0 start-0 h-full w-3/4 border-e sm:max-w-sm",
            right: "data-[state=closed]:slide-out-to-end data-[state=open]:slide-in-from-end inset-y-0 end-0 h-full w-3/4 border-s sm:max-w-sm",
        },
    },
    defaultVariants: { side: "right" },
});
type Div = JSX.HTMLAttributes<HTMLDivElement>;
type ContentProps = Div & {
    showCloseButton?: boolean;
    side?: Side;
    portalProps?: unknown;
    onEscapeKeyDown?: (e: KeyboardEvent) => void;
    onOpenAutoFocus?: (e: Event) => void;
    onPointerDownOutside?: (e: PointerEvent) => void;
    preventScroll?: boolean;
};
const normalContent =
    "bg-card data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 fixed top-[50%] left-[50%] z-50 grid w-full max-w-[calc(100%-2rem)] sm:max-w-[min(512px,calc(100vw-2rem))] translate-x-[-50%] translate-y-[-50%] gap-4 rounded-lg border p-6 shadow-lg duration-200 max-h-[90dvh] overflow-y-auto";
const overlayClasses =
    "data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 fixed inset-0 z-50 bg-black/50";

function Trap(props: {
    children: JSX.Element;
    onEscape?: (e: KeyboardEvent) => void;
    onOpenAutoFocus?: (e: Event) => void;
    class?: string;
    role: "dialog" | "alertdialog";
    id: string;
    slot: string;
    onClick?: JSX.EventHandler<HTMLDivElement, MouseEvent>;
    other?: Record<string, unknown>;
}) {
    let element!: HTMLDivElement;
    onSettled(() => {
        const before = document.activeElement as HTMLElement | null;
        const first = element?.querySelector<HTMLElement>(
            '[autofocus],button:not(:disabled),[href],input:not(:disabled),select:not(:disabled),textarea:not(:disabled),[tabindex]:not([tabindex="-1"])',
        );
        const opening = new Event("openAutoFocus", { cancelable: true });
        props.onOpenAutoFocus?.(opening);
        if (!opening.defaultPrevented) (first ?? element)?.focus();
        return () => {
            if (before?.isConnected) before.focus();
        };
    });
    function onKeyDown(e: KeyboardEvent) {
        if (e.key === "Escape") {
            props.onEscape?.(e);
            return;
        }
        if (e.key !== "Tab") return;
        const elems = Array.from(
            element.querySelectorAll<HTMLElement>(
                'a[href],button:not(:disabled),input:not(:disabled),select:not(:disabled),textarea:not(:disabled),[tabindex]:not([tabindex="-1"])',
            ),
        ).filter((el) => el.getClientRects().length > 0);
        if (!elems.length) {
            e.preventDefault();
            element.focus();
            return;
        }
        if (e.shiftKey && document.activeElement === elems[0]) {
            e.preventDefault();
            elems[elems.length - 1].focus();
        } else if (
            !e.shiftKey &&
            document.activeElement === elems[elems.length - 1]
        ) {
            e.preventDefault();
            elems[0].focus();
        }
    }
    return (
        <div
            ref={element}
            {...props.other}
            data-slot={props.slot}
            data-state="open"
            role={props.role}
            aria-modal="true"
            aria-labelledby={props.id + "-title"}
            aria-describedby={props.id + "-description"}
            tabindex={-1}
            class={props.class}
            onKeyDown={onKeyDown}
        >
            {props.children}
        </div>
    );
}

export function makeModal(kind: Kind) {
    const Ctx = createContext<ContextValue>();
    function useModal() {
        const ctx = useContext(Ctx);
        if (!ctx) throw Error(kind + " must be wrapped in Root");
        return ctx;
    }
    function Root(props: RProps) {
        const [own, setOwn] = createSignal(props.defaultOpen ?? false);
        const ctx: ContextValue = {
            id: createUniqueId(),
            kind,
            open: () => props.open ?? own(),
            setOpen: (v) => {
                setOwn(v);
                props.onOpenChange?.(v);
            },
        };
        return <Ctx value={ctx}>{props.children}</Ctx>;
    }
    function Trigger(
        props: JSX.HTMLAttributes<HTMLSpanElement> & { asChild?: boolean },
    ) {
        const ctx = useModal();
        return (
            <span
                {...props}
                data-slot={kind + "-trigger"}
                role="button"
                tabindex={0}
                style={{ display: "contents" }}
                onClick={() => ctx.setOpen(!ctx.open())}
                onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                        e.preventDefault();
                        ctx.setOpen(!ctx.open());
                    }
                }}
            >
                {props.children}
            </span>
        );
    }
    function Close(props: JSX.HTMLAttributes<HTMLSpanElement>) {
        const ctx = useModal();
        return (
            <span
                {...props}
                data-slot={kind + "-close"}
                role="button"
                tabindex={0}
                style={{ display: "contents" }}
                onClick={() => ctx.setOpen(false)}
                onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                        e.preventDefault();
                        ctx.setOpen(false);
                    }
                }}
            >
                {props.children}
            </span>
        );
    }
    function ModalPortal(props: { children?: JSX.Element }) {
        return <Portal>{props.children}</Portal>;
    }
    function Overlay(props: Div) {
        const ctx = useModal();
        return (
            <Show when={ctx.open()}>
                <div
                    {...props}
                    data-slot={kind + "-overlay"}
                    data-state="open"
                    class={cn(overlayClasses, props.class)}
                    onPointerDown={(e) => {
                        if (
                            kind !== "alert-dialog" &&
                            e.currentTarget === e.target
                        )
                            ctx.setOpen(false);
                    }}
                />
            </Show>
        );
    }
    function Content(props: ContentProps) {
        const ctx = useModal();
        const sheet = () => kind === "sheet";
        const close = () => props.showCloseButton ?? kind !== "alert-dialog";
        const contentClass = () =>
            cn(
                sheet()
                    ? sheetVariants({ side: props.side ?? "right" })
                    : normalContent,
                kind === "alert-dialog" && !sheet() ? "bg-background" : "",
                props.class,
            );
        return (
            <Show when={ctx.open()}>
                <Portal>
                    <div
                        data-slot={kind + "-overlay"}
                        data-state="open"
                        class={overlayClasses}
                        onPointerDown={(e) => {
                            props.onPointerDownOutside?.(e);
                            if (!e.defaultPrevented && kind !== "alert-dialog")
                                ctx.setOpen(false);
                        }}
                    />
                    <Trap
                        role={
                            kind === "alert-dialog" ? "alertdialog" : "dialog"
                        }
                        id={ctx.id}
                        slot={kind + "-content"}
                        class={contentClass()}
                        onEscape={(e) => {
                            props.onEscapeKeyDown?.(e);
                            if (!e.defaultPrevented) ctx.setOpen(false);
                        }}
                        onOpenAutoFocus={props.onOpenAutoFocus}
                        other={{ "data-side": props.side ?? "right" }}
                    >
                        {props.children}
                        <Show when={close()}>
                            <button
                                type="button"
                                data-slot={kind + "-close"}
                                aria-label={m.ui_close()}
                                onClick={() => ctx.setOpen(false)}
                                class={
                                    kind === "sheet"
                                        ? "ring-offset-background focus-visible:ring-ring absolute end-4 top-4 rounded-xs opacity-70 transition-opacity hover:opacity-100 focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-hidden disabled:pointer-events-none"
                                        : "ring-offset-background focus:ring-ring absolute end-4 top-4 rounded-xs opacity-70 transition-opacity hover:opacity-100 focus:ring-2 focus:ring-offset-2 focus:outline-hidden disabled:pointer-events-none [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4"
                                }
                            >
                                <svg
                                    class="size-4"
                                    width="16"
                                    height="16"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="2"
                                >
                                    <path d="M18 6 6 18M6 6l12 12" />
                                </svg>
                                <span class="sr-only">{m.ui_close()}</span>
                            </button>
                        </Show>
                    </Trap>
                </Portal>
            </Show>
        );
    }
    function Header(props: Div) {
        return (
            <div
                {...props}
                data-slot={kind + "-header"}
                class={cn(
                    kind === "sheet"
                        ? "flex flex-col gap-1.5 p-4"
                        : "flex flex-col gap-2 text-center sm:text-start",
                    props.class,
                )}
            >
                {props.children}
            </div>
        );
    }
    function Footer(props: Div) {
        return (
            <div
                {...props}
                data-slot={kind + "-footer"}
                class={cn(
                    kind === "sheet"
                        ? "mt-auto flex flex-col gap-2 p-4"
                        : "flex flex-col-reverse gap-2 sm:flex-row sm:justify-end",
                    props.class,
                )}
            >
                {props.children}
            </div>
        );
    }
    function Title(props: JSX.HTMLAttributes<HTMLHeadingElement>) {
        const ctx = useModal();
        return (
            <h2
                {...props}
                id={ctx.id + "-title"}
                data-slot={kind + "-title"}
                class={cn(
                    kind === "sheet"
                        ? "text-foreground font-semibold"
                        : kind === "dialog"
                          ? "text-lg leading-none font-semibold"
                          : "text-lg font-semibold",
                    props.class,
                )}
            >
                {props.children}
            </h2>
        );
    }
    function Description(props: JSX.HTMLAttributes<HTMLParagraphElement>) {
        const ctx = useModal();
        return (
            <p
                {...props}
                id={ctx.id + "-description"}
                data-slot={kind + "-description"}
                class={cn("text-muted-foreground text-sm", props.class)}
            >
                {props.children}
            </p>
        );
    }
    function Action(props: JSX.ButtonHTMLAttributes<HTMLButtonElement>) {
        const ctx = useModal();
        return (
            <button
                {...props}
                type={props.type ?? "button"}
                data-slot={kind + "-action"}
                onClick={(e) => {
                    (props.onClick as ((ev: MouseEvent) => void) | undefined)?.(
                        e,
                    );
                    if (!e.defaultPrevented) ctx.setOpen(false);
                }}
            >
                {props.children}
            </button>
        );
    }
    function Cancel(props: JSX.ButtonHTMLAttributes<HTMLButtonElement>) {
        const ctx = useModal();
        return (
            <button
                {...props}
                type="button"
                data-slot={kind + "-cancel"}
                onClick={(e) => {
                    (props.onClick as ((ev: MouseEvent) => void) | undefined)?.(
                        e,
                    );
                    if (!e.defaultPrevented) ctx.setOpen(false);
                }}
            >
                {props.children}
            </button>
        );
    }
    return {
        Root,
        Trigger,
        Close,
        Portal: ModalPortal,
        Overlay,
        Content,
        Header,
        Footer,
        Title,
        Description,
        Action,
        Cancel,
    };
}
