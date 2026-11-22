import { createMemo, createSignal, For, Show, onSettled } from "solid-js";
import { Clock } from "@lucide/solid";
import { cn } from "$lib/utils.js";

const hours = Array.from({length: 12}, (_, i) => String(i + 1).padStart(2, "0"));
const minutes = Array.from({length: 60}, (_, i) => String(i).padStart(2, "0"));
const periods = ["AM", "PM"];

export default function TimeInput12h(props: {
  value?: string; disabled?: boolean; className?: string;
  onValueChange?: (value: string) => void; onChange?: (value: string) => void;
}) {
  const [ownValue, setOwnValue] = createSignal("09:00");
  const [open, setOpen] = createSignal(false);
  let root: HTMLDivElement | undefined;
  const value = createMemo(() => props.value ?? ownValue());
  const parsed = createMemo(() => {
    const [h24, m] = (value() || "09:00").split(":").map(Number);
    const period = h24 >= 12 ? "PM" : "AM";
    const h12 = h24 % 12 || 12;
    return { h12: String(h12).padStart(2, "0"), m: String(m).padStart(2, "0"), period };
  });
  const setTime = (h12: string, m: string, period: string) => {
    let h24 = parseInt(h12, 10);
    if (period === "PM" && h24 !== 12) h24 += 12;
    if (period === "AM" && h24 === 12) h24 = 0;
    const next = `${String(h24).padStart(2, "0")}:${m}`;
    setOwnValue(next);
    props.onValueChange?.(next);
    props.onChange?.(next);
  };
  onSettled(() => {
    const outside = (e: PointerEvent) => { if (!root?.contains(e.target as Node)) setOpen(false); };
    const keydown = (e: KeyboardEvent) => { if (e.key === "Escape") setOpen(false); };
    document.addEventListener("pointerdown", outside);
    document.addEventListener("keydown", keydown);
    return () => { document.removeEventListener("pointerdown", outside); document.removeEventListener("keydown", keydown); };
  });
  return (
    <div class="relative" ref={root}>
      <button type="button" disabled={!!props.disabled} aria-haspopup="dialog" aria-expanded={(open()) ? "true" : "false"}
        onClick={() => setOpen(v => !v)}
        class={cn("relative flex items-center w-full h-10 px-3 rounded-xl bg-background/50 border border-border/50 transition-all text-sm font-mono tabular-nums overflow-hidden",
          props.disabled ? "opacity-50 cursor-not-allowed" : "hover:border-primary/30", props.className)}>
        <Clock class="w-4 h-4 text-muted-foreground mr-3" />
        <span>{parsed().h12}:{parsed().m} {parsed().period}</span>
      </button>
      <Show when={open() && !props.disabled}>
        <div role="dialog" aria-label="Select time" class="absolute z-50 top-full left-0 mt-1 w-auto p-0 bg-card border border-border/50 rounded-md shadow-2xl">
          <div class="flex h-[240px] divide-x divide-border/20">
            <div class="w-[64px] overflow-y-auto ace-scrollbar">
              <div class="flex flex-col p-1 gap-1"><For each={hours}>{h =>
                <button type="button" aria-pressed={(h === parsed().h12) ? "true" : "false"} onClick={() => setTime(h, parsed().m, parsed().period)}
                  class={cn("py-2 px-1 text-center text-sm font-mono rounded-md transition-colors",
                    h === parsed().h12 ? "bg-primary text-primary-foreground font-bold" : "text-muted-foreground hover:bg-muted hover:text-foreground")}>{h}</button>
              }</For></div>
            </div>
            <div class="w-[64px] overflow-y-auto ace-scrollbar">
              <div class="flex flex-col p-1 gap-1"><For each={minutes}>{m =>
                <button type="button" aria-pressed={(m === parsed().m) ? "true" : "false"} onClick={() => setTime(parsed().h12, m, parsed().period)}
                  class={cn("py-2 px-1 text-center text-sm font-mono rounded-md transition-colors",
                    m === parsed().m ? "bg-primary text-primary-foreground font-bold" : "text-muted-foreground hover:bg-muted hover:text-foreground")}>{m}</button>
              }</For></div>
            </div>
            <div class="w-[64px] overflow-y-auto ace-scrollbar">
              <div class="flex flex-col p-1 gap-1"><For each={periods}>{p =>
                <button type="button" aria-pressed={(p === parsed().period) ? "true" : "false"} onClick={() => setTime(parsed().h12, parsed().m, p)}
                  class={cn("py-2 px-1 text-center text-xs font-bold rounded-md transition-colors",
                    p === parsed().period ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:bg-muted hover:text-foreground")}>{p}</button>
              }</For></div>
            </div>
          </div>
        </div>
      </Show>
    </div>
  );
}
