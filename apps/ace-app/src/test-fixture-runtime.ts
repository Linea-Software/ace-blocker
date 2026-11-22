import { render, type JSX } from "@solidjs/web";

/**
 * Browser-only Playwright mounting for actual Solid 2 components.
 * This is NEVER used for native engine IPC or production routing.
 * Keeping the fixture API small makes the before/after visual assertions
 * cover the same components without a privileged backend.
 */
export function mount(
    component: (props: any) => JSX.Element,
    options: { target: Element; props?: Record<string, unknown> },
): () => void {
    return render(() => component(options.props ?? {}), options.target);
}
