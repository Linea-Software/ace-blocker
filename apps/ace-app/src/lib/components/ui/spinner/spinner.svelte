<script lang="ts">
	import * as m from "$lib/paraglide/messages.js";
	import { cn } from "$lib/utils.js";
	import Loader2Icon from "@lucide/svelte/icons/loader-2";
	import type { ComponentProps } from "svelte";

	let { class: className, ...restProps }: ComponentProps<typeof Loader2Icon> = $props();
</script>

<Loader2Icon
	role="status"
	aria-label={m.ui_loading()}
	class={cn("size-4 animate-spin", className)}
	{...restProps}
/>
