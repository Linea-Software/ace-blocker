import { Show, onSettled } from "solid-js";
import TimeInput12h from "./TimeInput12h";
import TimeInput24h from "./TimeInput24h";
import { localizationStore } from "$lib/stores/localization";

export default function LocalizedTimeInput(props: {
  value?: string; disabled?: boolean; className?: string;
  onValueChange?: (value: string) => void; onChange?: (value: string) => void;
}) {
  onSettled(() => { localizationStore.detectSystemTimeFormat(); });
  const childProps = () => ({ value: props.value ?? "09:00", disabled: props.disabled,
    className: props.className, onValueChange: (value: string) => {
      props.onValueChange?.(value); props.onChange?.(value);
    } });
  return <Show when={localizationStore.uses12h}
    fallback={<TimeInput24h {...childProps()} />}>
    <TimeInput12h {...childProps()} />
  </Show>;
}
