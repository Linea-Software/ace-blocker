import { createEffect, createMemo, createSignal, For, Show, onSettled } from "solid-js";
import { Clock } from "@lucide/solid";
import { cn } from "$lib/utils.js";

const hours = Array.from({length: 24}, (_, i) => String(i).padStart(2, "0"));
const minutes = Array.from({length: 60}, (_, i) => String(i).padStart(2, "0"));

export default function TimePicker(props: {
  value?: string; disabled?: boolean; ariaLabel?: string; class?: string;
  onValueChange?: (value: string) => void; onChange?: (value: string) => void;
}) {
  const [ownValue, setOwnValue] = createSignal("09:00");
  const [draft, setDraft] = createSignal("09:00");
  const [open, setOpen] = createSignal(false);
  let root: HTMLDivElement | undefined;
  const value = createMemo(() => props.value ?? ownValue());
  const selectedHour = createMemo(() => value().split(":")[0] || "09");
  const selectedMinute = createMemo(() => value().split(":")[1] || "00");
  createEffect(() => props.value, external => {
    if (external && /^\d{2}:\d{2}$/.test(external)) setDraft(external);
  });
  const publish = (next: string) => {
    setOwnValue(next);
    props.onValueChange?.(next);
    props.onChange?.(next);
  };
  const setTime = (h: string, m: string) => {
    const next = `${h}:${m}`;
    setDraft(next);
    publish(next);
  };
  const handleInput = (event: InputEvent & {currentTarget: HTMLInputElement}) => {
    const el = event.currentTarget;
    let next = el.value.replace(/\D/g, "").slice(0, 4);
    if (next.length >= 3) next = `${next.slice(0, 2)}:${next.slice(2)}`;
    if (next.length === 5) {
      let [h, m] = next.split(":");
      if (+h > 23) h = "23";
      if (+m > 59) m = "59";
      next = `${h}:${m}`;
      publish(next);
    }
    setDraft(next);
    el.value = next;
  };
  const handleBlur = (event: FocusEvent & {currentTarget: HTMLInputElement}) => {
    const input = event.currentTarget;
    if (input.value.length > 0 && input.value.length < 5) {
      const chars = input.value.replace(/\D/g, "").padEnd(4, "0");
      let h = chars.slice(0, 2), m = chars.slice(2, 4);
      if (+h > 23) h = "23";
      if (+m > 59) m = "59";
      const next = `${h}:${m}`;
      setDraft(next);
      input.value = next;
      publish(next);
    }
  };
  onSettled(() => {
    const outside = (e: PointerEvent) => { if (!root?.contains(e.target as Node)) setOpen(false); };
    const escape = (e: KeyboardEvent) => { if (e.key === "Escape") setOpen(false); };
    document.addEventListener("pointerdown", outside);
    document.addEventListener("keydown", escape);
    return () => {document.removeEventListener("pointerdown", outside);document.removeEventListener("keydown", escape);};
  });
  return (
    <div class="relative" ref={root}>
      <div role="button" tabindex={props.disabled ? -1 : 0}
        class={cn("relative flex items-center w-full h-10 rounded-md bg-background/50 border border-border/50 transition-all text-sm font-mono overflow-hidden cursor-text",
          props.disabled ? "opacity-50 cursor-not-allowed" : "focus-within:ring-1 focus-within:ring-ring focus-within:border-ring hover:border-border", props.class)}
        onKeyDown={e => { if (!props.disabled && (e.key === "Enter" || e.key === " ")) {
          e.preventDefault();
          e.currentTarget.querySelector("input")?.focus();
          setOpen(true);
        } }}
        onClick={e => {if (!props.disabled) {e.currentTarget.querySelector("input")?.focus();setOpen(true);} }}>
        <div class="pl-3 flex items-center justify-center shrink-0 pointer-events-none">
          <Clock class="w-4 h-4 text-muted-foreground" />
        </div>
        <input type="text" value={draft()} onInput={handleInput} onBlur={handleBlur}
          class="flex-1 h-full bg-transparent px-3 outline-none tabular-nums text-foreground placeholder-muted-foreground disabled:cursor-not-allowed"
          placeholder="00:00" aria-label={props.ariaLabel} autocomplete="off" maxlength={5} disabled={!!props.disabled}
          aria-haspopup="dialog" aria-expanded={(open()) ? "true" : "false"} onClick={() => {if (!props.disabled) setOpen(true)}} />
      </div>
      <Show when={open() && !props.disabled}>
        <div role="dialog" aria-label={props.ariaLabel ?? "Select time"}
          class="absolute z-50 top-full left-0 mt-1 w-auto p-0 bg-card border border-border/50 rounded-md shadow-2xl">
          <div class="flex h-[240px] divide-x divide-border/20">
            <div class="w-[72px] overflow-y-auto ace-scrollbar">
              <div class="flex flex-col p-1 gap-1"><For each={hours}>{h =>
                <button type="button" class={cn("py-2 px-1 text-center text-sm font-mono rounded-md transition-colors",
                  h === selectedHour() ? "bg-secondary text-secondary-foreground font-bold" : "text-muted-foreground hover:bg-muted hover:text-foreground")}
                  aria-pressed={(h === selectedHour()) ? "true" : "false"} onClick={() => setTime(h, selectedMinute())}>{h}</button>
              }</For></div>
            </div>
            <div class="w-[72px] overflow-y-auto ace-scrollbar">
              <div class="flex flex-col p-1 gap-1"><For each={minutes}>{m =>
                <button type="button" class={cn("py-2 px-1 text-center text-sm font-mono rounded-md transition-colors",
                  m === selectedMinute() ? "bg-secondary text-secondary-foreground font-bold" : "text-muted-foreground hover:bg-muted hover:text-foreground")}
                  aria-pressed={(m === selectedMinute()) ? "true" : "false"} onClick={() => setTime(selectedHour(), m)}>{m}</button>
              }</For></div>
            </div>
          </div>
        </div>
      </Show>
    </div>
  );
}
