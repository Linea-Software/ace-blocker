import { createSignal, Show } from "solid-js";
import { Globe } from "@lucide/solid";
import { Label } from "$lib/components/ui/label/solid";
import { Input } from "$lib/components/ui/input/solid";
import * as m from "$lib/paraglide/messages.js";
import IconPicker from "../../site-packs/_IconPicker";

export interface SectionGeneralProps {
    name: string;
    icon: string | null;
    onNameChange: (name: string) => void;
    onIconChange: (icon: string | null) => void;
    disabled?: boolean;
    isLocked?: boolean;
}

export default function SectionGeneral(props: SectionGeneralProps) {
    const [showIconPicker, setShowIconPicker] = createSignal(false);
    return (
        <div class="space-y-2" id="general">
            <Label for="name" class="text-sm font-semibold">
                {m.block_name()}
            </Label>
            <div class="flex items-center gap-3">
                <IconPicker
                    selectedIcon={props.icon}
                    onSelectedIconChange={props.onIconChange}
                    open={showIconPicker()}
                    onOpenChange={setShowIconPicker}
                    fallbackIcon={Globe}
                    label={m.pack_icon()}
                    size="lg"
                    disabled={props.disabled}
                />
                <Input
                    id="name"
                    value={props.name}
                    onInput={(event) =>
                        props.onNameChange(event.currentTarget.value)
                    }
                    placeholder={m.block_name_placeholder()}
                    disabled={!!props.disabled || !!props.isLocked}
                    class="h-11 min-w-0 flex-1 rounded-sm bg-card"
                    autocomplete="off"
                />
            </div>
            <Show when={props.isLocked}>
                <p class="text-muted-foreground text-xs">
                    {m.name_cannot_change_locked()}
                </p>
            </Show>
        </div>
    );
}
