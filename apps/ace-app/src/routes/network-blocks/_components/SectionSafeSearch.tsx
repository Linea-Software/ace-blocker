import { ShieldCheck } from "@lucide/solid";
import { Label } from "$lib/components/ui/label/solid";
import { Switch } from "$lib/components/ui/switch/solid";
import * as m from "$lib/paraglide/messages.js";

export interface SectionSafeSearchProps {
    isSafeSearch: boolean;
    onSafeSearchChange: (value: boolean) => void;
    disabled?: boolean;
    isLocked?: boolean;
}

/** Stage Safe Search changes locally; the network-block editor commits them atomically. */
export default function SectionSafeSearch(props: SectionSafeSearchProps) {
    const handleChange = (checked: boolean) => {
        if (!props.disabled && !props.isLocked) props.onSafeSearchChange(checked);
    };
    return (
        <div class="rounded-md border border-border bg-card" data-testid="block-safe-search-setting">
            <Label
                for="block-safe-search"
                class="flex min-h-16 w-full cursor-pointer items-center gap-3 rounded-md p-4 leading-normal has-[:disabled]:cursor-not-allowed"
            >
                <ShieldCheck size={19} strokeWidth={1.75} aria-hidden="true" class="shrink-0 text-muted-foreground" />
                <span class="min-w-0 flex-1">
                    <span class="flex flex-wrap items-center gap-2">
                        <span class="text-sm font-semibold text-foreground">{m.force_safe_search()}</span>
                        <span class="rounded-sm border border-border px-1.5 py-0.5 text-[11px] font-medium text-muted-foreground">{m.pack_access_free()}</span>
                    </span>
                    <span id="block-safe-search-description" class="mt-1 block text-xs leading-relaxed text-muted-foreground">
                        {m.force_safe_search_description()}
                    </span>
                </span>
                <Switch
                    id="block-safe-search"
                    checked={props.isSafeSearch}
                    onCheckedChange={handleChange}
                    disabled={!!props.disabled || !!props.isLocked}
                    aria-describedby="block-safe-search-description"
                    aria-label={m.force_safe_search()}
                />
            </Label>
        </div>
    );
}
