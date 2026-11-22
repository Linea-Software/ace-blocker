import "./Inspector.css";
import { Show, onSettled, createUniqueId } from "solid-js";
import type { JSX } from "@solidjs/web";
import { X } from "@lucide/solid";

export default function Inspector(props: {
  title: string; description?: string; closeLabel: string; onClose: () => void;
  wide?: boolean; navigation?: JSX.Element; children: JSX.Element; footer?: JSX.Element;
}) {
  const uid = createUniqueId();
  let closeButton: HTMLButtonElement | undefined;
  onSettled(() => {
    const previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    closeButton?.focus();
    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.defaultPrevented &&
        document.activeElement?.closest(".inspector")) {
        event.preventDefault();
        props.onClose();
      }
    };
    window.addEventListener("keydown", handleEscape);
    return () => {
      window.removeEventListener("keydown", handleEscape);
      if (previousFocus?.isConnected) previousFocus.focus();
    };
  });
  return (
    <aside class={["inspector", { wide: !!props.wide, "with-navigation": !!props.navigation }]}
      role="region" aria-labelledby={uid + "-title"}>
      <header class="inspector-header">
        <div class="title-block"><h2 id={uid + "-title"}>{props.title}</h2>
          <Show when={props.description}><p>{props.description}</p></Show>
        </div>
        <button type="button" class="close-button" ref={closeButton} onClick={props.onClose}
          aria-label={props.closeLabel} title={props.closeLabel}><X size={18} aria-hidden="true" /></button>
      </header>
      <Show when={props.navigation}><div class="inspector-nav">{props.navigation}</div></Show>
      <div class="inspector-scroll ace-scrollbar">{props.children}</div>
      <Show when={props.footer}><footer class="inspector-footer">{props.footer}</footer></Show>

    </aside>
  );
}
