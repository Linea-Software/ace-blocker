import ProGatedButton from "$lib/components/ProGatedButton";
import { Crown } from "@lucide/solid";
import * as m from "$lib/paraglide/messages.js";

export default function UpgradeToProButton(props: {
    class?: string;
}) {
    return (
        <ProGatedButton
            isPro={false}
            label={m.get_pro()}
            upgradeOnly={true}
            icon={Crown}
            class={props.class ?? ""}
        />
    );
}
