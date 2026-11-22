import ProGatedButton from "$lib/components/ProGatedButton";
import * as m from "$lib/paraglide/messages.js";

export default function UpgradeToProButton(props: { class?: string; text?: string }) {
  return <ProGatedButton isPro={false} label={props.text ?? m.get_pro()} class={props.class ?? ""} />;
}
