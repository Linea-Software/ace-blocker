import { reactiveFields } from "$lib/stores/reactive-fields";
import * as m from "$lib/paraglide/messages.js";
import { getLocale } from "$lib/paraglide/runtime";
import { userStore } from "$lib/stores/user-store";
import { exportBackup, importBackup, importBackupFromCloud, exportBackupToCloud } from "$lib/engine-client";
import * as AlertDialog from "$lib/components/ui/alert-dialog/solid";
import { Database, CloudUpload, RefreshCw, FileUp, FileDown, Save, History, ShieldAlert, Cloud } from "@lucide/solid";
import { toast } from "solid-sonner";
import UpgradeToProButton from "$lib/components/UpgradeToProButton";
import { save, open } from "@tauri-apps/plugin-dialog";
import { writeTextFile, readTextFile } from "@tauri-apps/plugin-fs";
import "./_BackupSection.css";
export default function BackupSection() {
 const state = {
 isExporting: false,
 isImporting: false,
 isCloudSyncing: false,
 isCloudRestoring: false,
 isImportConfirmOpen: false,
 lastSyncedDate: null as string | null,
 isCloudPasswordDialogOpen: false,
 cloudOperation: null as "upload" | "restore" | null,
 cloudPassword: "",
 cloudPasswordError: null as string | null,
};
 reactiveFields(state, ["isExporting", "isImporting", "isCloudSyncing", "isCloudRestoring", "isImportConfirmOpen", "lastSyncedDate", "isCloudPasswordDialogOpen", "cloudOperation", "cloudPassword", "cloudPasswordError"]);
    async function handleExportLocal() {
        state.isExporting = true;
        const response = await exportBackup();

        if (response.error || !response.data) {
            console.error(response.error);
            toast.error(m.backup_export_failed());
            state.isExporting = false;
            return;
        }

        try {
            const path = await save({
                filters: [{ name: "Ace Backup", extensions: ["json"] }],
                defaultPath: `ace-backup-${new Date().toISOString().split("T")[0]}.json`,
            });

            if (path) {
                await writeTextFile(
                    path,
                    JSON.stringify(response.data, null, 2),
                );
                toast.success(m.backup_export_success());
            }
        } catch (error) {
            console.error(error);
            toast.error(m.backup_export_failed());
        } finally {
            state.isExporting = false;
        }
    }

    async function handleImportLocal() {
        state.isImporting = true;
        try {
            const path = await open({
                multiple: false,
                filters: [{ name: "Ace Backup", extensions: ["json"] }],
            });

            if (path && typeof path === "string") {
                const content = await readTextFile(path);
                const data = JSON.parse(content);
                const { error } = await importBackup({ body: data });

                if (error) {
                    console.error(error);
                    toast.error(m.backup_import_failed());
                } else {
                    toast.success(m.backup_import_success());
                }
            }
        } catch (error) {
            console.error(error);
            toast.error(m.backup_import_failed());
        } finally {
            state.isImporting = false;
            state.isImportConfirmOpen = false;
        }
    }

    function openCloudPasswordDialog(operation: "upload" | "restore") {
        if (!userStore.isPro || state.isCloudSyncing || state.isCloudRestoring) return;
        state.cloudPassword = "";
        state.cloudPasswordError = null;
        state.cloudOperation = operation;
        state.isCloudPasswordDialogOpen = true;
    }

    function closeCloudPasswordDialog() {
        if (state.isCloudSyncing || state.isCloudRestoring) return;
        state.cloudPassword = "";
        state.cloudPasswordError = null;
        state.cloudOperation = null;
        state.isCloudPasswordDialogOpen = false;
    }

    async function handleConfirmCloud(event: SubmitEvent) {
        event.preventDefault();
        if (
            !userStore.isPro ||
            !state.cloudOperation ||
            !state.cloudPassword ||
            state.isCloudSyncing ||
            state.isCloudRestoring
        ) return;

        const operation = state.cloudOperation;
        state.cloudPasswordError = null;
        if (operation === "upload") {
            state.isCloudSyncing = true;
        } else {
            state.isCloudRestoring = true;
        }

        try {
            // Capture the password in the RPC argument, then immediately wipe the UI.
            // No password is kept for retry or shared with another operation.
            const request = operation === "upload"
                ? exportBackupToCloud({ body: { password: state.cloudPassword } })
                : importBackupFromCloud({ body: { password: state.cloudPassword } });
            state.cloudPassword = "";
            const { error } = await request;
            if (error) {
                state.cloudPasswordError = cloudErrorMessage(
                    error,
                    operation === "upload"
                        ? m.cloud_backup_failed()
                        : m.cloud_restore_failed(),
                );
            } else {
                if (operation === "upload") {
                    toast.success(m.cloud_backup_success());
                    state.lastSyncedDate = new Date().toLocaleString(getLocale());
                } else {
                    toast.success(m.cloud_restore_success());
                }
                state.cloudPassword = "";
                state.cloudPasswordError = null;
                state.cloudOperation = null;
                state.isCloudPasswordDialogOpen = false;
            }
        } catch {
            // Do not log credential-bearing RPC error objects.
            state.cloudPasswordError =
                operation === "upload"
                    ? m.cloud_backup_failed()
                    : m.cloud_restore_failed();
        } finally {
            state.cloudPassword = "";
            state.isCloudSyncing = false;
            state.isCloudRestoring = false;
        }
    }

    function cloudErrorMessage(error: unknown, fallbackMessage: string): string {
        const code =
            error && typeof error === "object"
                ? (error as Record<string, unknown>).code
                : undefined;

        // Only match known server messages; never display or log raw RPC errors.
        const reason =
            error && typeof error === "object"
                ? (error as Record<string, unknown>).error
                : undefined;
        if (reason === "Incorrect password or corrupted cloud backup") {
            return m.cloud_backup_decryption_failed();
        }
        if (code === "unauthorized" || code === "capability_required") {
            return m.cloud_backup_not_signed_in();
        }
        if (code === "forbidden" || code === "pro_plan_required") {
            return m.cloud_backup_needs_pro();
        }
        if (
            code === "invalid_password" ||
            code === "incorrect_password" ||
            code === "wrong_password" ||
            code === "invalid_credentials"
        ) {
            return m.cloud_backup_password_incorrect();
        }
        if (code === "decryption_failed" || code === "backup_decryption_failed") {
            return m.cloud_backup_decryption_failed();
        }
        return fallbackMessage;
    }


 return <>
<section class="settings-section ace-settings-backup">
    <header>
        <span class="section-mark" aria-hidden="true"><Database size={16} /></span>
        <div>
            <h2>{m.backup_and_restore()}</h2>
            <p>{m.backup_and_restore_description()}</p>
        </div>
    </header>

    <div class="backup-ledger">
        <div class="backup-row">
            <span class="row-mark" aria-hidden="true"><Save size={17} /></span>
            <div class="row-copy">
                <h3>{m.local_backup()}</h3>
                <p>{m.local_backup_description()}</p>
            </div>
            <div class="row-actions">
                <button type="button" onClick={handleExportLocal} disabled={state.isExporting}>
                    {state.isExporting ? (<>
                        <RefreshCw class="spinner" size={15} aria-hidden="true" />
                    </>) : (<>
                        <FileUp size={15} aria-hidden="true" />
                    </>)}
                    {m.export()}
                </button>
                <button
                    type="button"
                    onClick={() => (state.isImportConfirmOpen = true)}
                    disabled={state.isImporting}
                >
                    {state.isImporting ? (<>
                        <RefreshCw class="spinner" size={15} aria-hidden="true" />
                    </>) : (<>
                        <FileDown size={15} aria-hidden="true" />
                    </>)}
                    {m.import()}
                </button>
            </div>
        </div>

        <div class={!userStore.isPro ? "backup-row restricted" : "backup-row"}>
            <span class="row-mark" aria-hidden="true"><Cloud size={17} /></span>
            <div class="row-copy">
                <h3>{m.cloud_backup()}</h3>
                <p>
                    {userStore.isPro
                        ? m.cloud_backup_description()
                        : m.cloud_backup_tagline()}
                </p>
                {userStore.isPro && state.lastSyncedDate ? (<>
                    <span class="last-sync">
                        {m.last_backed_up({ date: state.lastSyncedDate })}
                    </span>
                </>) : null}
            </div>
            <div class="row-actions">
                {userStore.isPro ? (<>
                    <button
                        type="button"
                        onClick={() => openCloudPasswordDialog("upload")}
                        disabled={state.isCloudSyncing || state.isCloudRestoring}
                    >
                        {state.isCloudSyncing ? (<>
                            <RefreshCw class="spinner" size={15} aria-hidden="true" />
                        </>) : (<>
                            <CloudUpload size={15} aria-hidden="true" />
                        </>)}
                        {m.upload_to_cloud()}
                    </button>
                    <button
                        type="button"
                        onClick={() => openCloudPasswordDialog("restore")}
                        disabled={state.isCloudSyncing || state.isCloudRestoring}
                    >
                        {state.isCloudRestoring ? (<>
                            <RefreshCw class="spinner" size={15} aria-hidden="true" />
                        </>) : (<>
                            <History size={15} aria-hidden="true" />
                        </>)}
                        {m.restore_from_cloud()}
                    </button>
                </>) : (<>
                    <UpgradeToProButton text={m.cloud_backup()} />
                </>)}
            </div>
        </div>
    </div>
</section>

<AlertDialog.Root open={state.isImportConfirmOpen} onOpenChange={(v: boolean) => { state.isImportConfirmOpen = v; }}>
    <AlertDialog.Content class="ace-settings-backup-dialog rounded-md border-border bg-card shadow-none sm:max-w-md">
        <AlertDialog.Header class="gap-4">
            <div class="import-warning">
                <ShieldAlert size={18} aria-hidden="true" />
            </div>
            <div class="flex flex-col gap-2 text-left">
                <AlertDialog.Title class="font-condensed text-xl font-semibold">
                    {m.import_backup_confirmation_title()}
                </AlertDialog.Title>
                <AlertDialog.Description class="text-sm leading-relaxed">
                    {m.import_backup_confirmation_description()}
                </AlertDialog.Description>
            </div>
        </AlertDialog.Header>
        <AlertDialog.Footer class="mt-2 gap-2 border-t border-border pt-4">
            <AlertDialog.Cancel disabled={state.isImporting}>
                {m.cancel()}
            </AlertDialog.Cancel>
            <AlertDialog.Action
                onClick={handleImportLocal}
                disabled={state.isImporting}
                class="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
                {state.isImporting ? (<>
                    <RefreshCw class="spinner" size={15} aria-hidden="true" />
                    {m.processing()}
                </>) : (<>
                    <FileDown size={15} aria-hidden="true" />
                    {m.confirm()}
                </>)}
            </AlertDialog.Action>
        </AlertDialog.Footer>
    </AlertDialog.Content>
</AlertDialog.Root>

<AlertDialog.Root
    open={state.isCloudPasswordDialogOpen}
    onOpenChange={(open: boolean) => {
        state.isCloudPasswordDialogOpen = open;
        if (!open) {
            state.cloudPassword = "";
            state.cloudPasswordError = null;
            state.cloudOperation = null;
        }
    }}
>
    <AlertDialog.Content
        class="ace-settings-backup-dialog rounded-md border-border bg-card shadow-none sm:max-w-md"
        onEscapeKeyDown={(event: KeyboardEvent) => {
            if (state.isCloudSyncing || state.isCloudRestoring) event.preventDefault();
        }}
        onOpenAutoFocus={(event: Event) => {
            event.preventDefault();
            document.getElementById("cloud-backup-password")?.focus();
        }}
    >
        <form autocomplete="off" onSubmit={handleConfirmCloud} class="grid gap-4">
            <AlertDialog.Header class="gap-3">
                <div class="flex flex-col gap-2 text-left">
                    <AlertDialog.Title class="font-condensed text-xl font-semibold">
                        {state.cloudOperation === "upload"
                            ? m.cloud_backup_password_upload_title()
                            : m.cloud_backup_password_restore_title()}
                    </AlertDialog.Title>
                    <AlertDialog.Description class="text-sm leading-relaxed">
                        {state.cloudOperation === "upload"
                            ? m.cloud_backup_password_upload_description()
                            : m.cloud_backup_password_restore_description()}
                    </AlertDialog.Description>
                </div>
            </AlertDialog.Header>
            <div class="cloud-password-field">
                <label for="cloud-backup-password">{m.cloud_backup_password_label()}</label>
                <input
                    id="cloud-backup-password"
                    type="password"
                    autocomplete="off"
                    autocapitalize="off"
                    spellcheck={false}
                    required
                    value={state.cloudPassword} onInput={(event) => { state.cloudPassword = event.currentTarget.value; }}
                    disabled={state.isCloudSyncing || state.isCloudRestoring}
                    aria-invalid={state.cloudPasswordError !== null ? 'true' : 'false'}
                    aria-describedby={state.cloudPasswordError
                        ? "cloud-backup-password-hint cloud-backup-password-error"
                        : "cloud-backup-password-hint"}
                />
                <p id="cloud-backup-password-hint">{m.cloud_backup_password_hint()}</p>
                {state.cloudPasswordError ? (<>
                    <p id="cloud-backup-password-error" class="cloud-password-error" role="alert">
                        {state.cloudPasswordError}
                    </p>
                </>) : null}
            </div>
            <AlertDialog.Footer class="mt-1 gap-2 border-t border-border pt-4">
                <AlertDialog.Cancel
                    type="button"
                    disabled={state.isCloudSyncing || state.isCloudRestoring}
                    onClick={closeCloudPasswordDialog}
                >
                    {m.cancel()}
                </AlertDialog.Cancel>
                <button
                    class="cloud-password-submit"
                    type="submit"
                    disabled={!state.cloudPassword || state.isCloudSyncing || state.isCloudRestoring}
                >
                    {state.isCloudSyncing || state.isCloudRestoring ? (<>
                        <RefreshCw class="spinner" size={15} aria-hidden="true" />
                        {m.processing()}
                    </>) : state.cloudOperation === "upload" ? (<>
                        <CloudUpload size={15} aria-hidden="true" />
                        {m.upload_to_cloud()}
                    </>) : (<>
                        <History size={15} aria-hidden="true" />
                        {m.restore_from_cloud()}
                    </>)}
                </button>
            </AlertDialog.Footer>
        </form>
    </AlertDialog.Content>
</AlertDialog.Root>
</>;
}
