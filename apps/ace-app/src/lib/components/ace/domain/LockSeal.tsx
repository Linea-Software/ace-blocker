import "./LockSeal.css";
import { LockKeyhole } from "@lucide/solid";
import { Show } from "solid-js";

export default function LockSeal(props: {
  title: string;
  description: string;
  detail?: string | null;
  active?: boolean;
  emphasis?: "signal" | "danger";
  compact?: boolean;
}) {
  return (
    <section class={["lock-seal", { active: props.active ?? true, danger: props.emphasis === "danger", compact: !!props.compact }]} aria-label={props.title}>
      <span class="seal-rail" aria-hidden="true" />
      <div class="seal-mark" aria-hidden="true"><LockKeyhole size={18} strokeWidth={2} /></div>
      <div class="seal-copy"><strong>{props.title}</strong><p>{props.description}</p></div>
      <Show when={props.detail}><span class="seal-detail">{props.detail}</span></Show>
    </section>
  );
}
