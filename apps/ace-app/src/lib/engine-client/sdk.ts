// Frontend facade for the typed engine RPC interface. No HTTP client.

import type { RequestResult, RpcStreamResult, RpcOptions } from './rpc-types';
import type { SaveNetworkBlockData, SaveNetworkBlockResponses } from './types';
import { rpcRequest, rpcStream } from './tauri-rpc';
import type { AcceptTosData, AcceptTosResponses, AddNetBlockToSitePackConnectionData, AddNetBlockToSitePackConnectionResponses, AddScheduleEntriesData, AddScheduleEntriesResponses, AddSitePackSourceData, AddSitePackSourceResponses, AddUserRulesData, AddUserRulesResponses, CheckUpdatesData, CheckUpdatesResponses, ConfirmUnlockData, ConfirmUnlockErrors, ConfirmUnlockResponses, CreateNetworkBlockData, CreateNetworkBlockErrors, CreateNetworkBlockResponses, CreateSitePackData, CreateSitePackResponse, CreateSitePackResponses, DeleteNetworkBlockData, DeleteNetworkBlockErrors, DeleteNetworkBlockResponses, DeleteScheduleEntryData, DeleteScheduleEntryResponses, DeleteSitePackData, DeleteSitePackErrors, DeleteSitePackResponses, DeleteSitePackSourceData, DeleteSitePackSourceResponses, DeleteUserRuleData, DeleteUserRuleResponses, DownloadResistanceScriptsData, DownloadResistanceScriptsResponses, EditScheduleEntryData, EditScheduleEntryResponses, ExportBackupData, ExportBackupResponses, ExportBackupToCloudData, ExportBackupToCloudResponses, GetAccountInfoData, GetAccountInfoResponses, GetAllNetworkBlocksData, GetAllNetworkBlocksResponses, GetAllScriptsData, GetAllScriptsResponses, GetAllSitePacksData, GetAllSitePacksResponses, GetDashboardSnapshotData, GetDashboardSnapshotResponses, GetDebugProPolicyData, GetDebugProPolicyResponses, GetLockData, GetLockResponses, GetNetBlockToSitePackConnectionsData, GetNetBlockToSitePackConnectionsResponses, GetResistanceModeSettingsData, GetResistanceModeSettingsResponses, GetScheduleEntriesData, GetScheduleEntriesResponses, GetScriptData, GetScriptErrors, GetScriptResponses, GetSitePackSourcesData, GetSitePackSourcesResponses, GetUpdateCheckIntervalData, GetUpdateCheckIntervalResponses, GetUserRulesData, GetUserRulesResponses, GetVersionData, GetVersionResponses, HideFilesData, HideFilesResponses, ImportBackupData, ImportBackupFromCloudData, ImportBackupFromCloudResponses, ImportBackupResponses, InitiateUnlockData, InitiateUnlockErrors, InitiateUnlockResponses, LogoutData, LogoutResponses, OpenProPageData, OpenProPageErrors, OpenProPageResponses, PingData, PingResponses, ReEnableRemoteSitePacksData, ReEnableRemoteSitePacksResponses, RefreshAccountData, RefreshAccountResponses, RemoveNetBlockToSitePackConnectionData, RemoveNetBlockToSitePackConnectionResponses, RenameNetworkBlockData, RenameNetworkBlockErrors, RenameNetworkBlockResponses, ResetTosData, ResetTosResponses, ResyncSitePackData, ResyncSitePackResponse, ResyncSitePackResponses, SetApiKeyData, SetApiKeyResponses, SetDebugProPolicyData, SetDebugProPolicyResponses, SetLockData, SetLockResponses, SetNetworkBlockEnabledData, SetNetworkBlockEnabledErrors, SetNetworkBlockEnabledResponses, SetNetworkBlockIconData, SetNetworkBlockIconErrors, SetNetworkBlockIconResponses, SetNetworkBlockSafeSearchData, SetNetworkBlockSafeSearchErrors, SetNetworkBlockSafeSearchResponses, SetResistanceModeEnabledData, SetResistanceModeEnabledErrors, SetResistanceModeEnabledResponses, SetResistanceModeLockData, SetResistanceModeLockErrors, SetResistanceModeLockResponses, SetScriptEnabledData, SetScriptEnabledErrors, SetScriptEnabledResponses, SetUpdateCheckIntervalData, SetUpdateCheckIntervalResponses, ShowFilesData, ShowFilesResponses, TriggerContentUpdateData, TriggerContentUpdateErrors, TriggerContentUpdateResponses } from './types';

export type Options<TData extends object = object, ThrowOnError extends boolean = boolean, TResponse = unknown> =
    RpcOptions<TData, ThrowOnError, TResponse>;

export const acceptTos = <ThrowOnError extends boolean = false>(options?: Options<AcceptTosData, ThrowOnError>): RequestResult<AcceptTosResponses, unknown, ThrowOnError> => rpcRequest<AcceptTosResponses, unknown, ThrowOnError>("acceptTos", options);

export const setApiKey = <ThrowOnError extends boolean = false>(options: Options<SetApiKeyData, ThrowOnError>): RequestResult<SetApiKeyResponses, unknown, ThrowOnError> => rpcRequest<SetApiKeyResponses, unknown, ThrowOnError>("setApiKey", options);

export const getAccountInfo = <ThrowOnError extends boolean = false>(options?: Options<GetAccountInfoData, ThrowOnError>): RequestResult<GetAccountInfoResponses, unknown, ThrowOnError> => rpcRequest<GetAccountInfoResponses, unknown, ThrowOnError>("getAccountInfo", options);

export const logout = <ThrowOnError extends boolean = false>(options?: Options<LogoutData, ThrowOnError>): RequestResult<LogoutResponses, unknown, ThrowOnError> => rpcRequest<LogoutResponses, unknown, ThrowOnError>("logout", options);

/**
 * Re-read the account's Pro status after an event that may have changed it
 * (login, logout, subscription change, focus regain).
 *
 * The explicit refresh path uses a 30-second freshness window (rather
 * than the proxy's 24-hour lazy cache). Concurrent requests are single-flight
 * and transient errors retain the last verified status. Valid Free accounts
 * remain signed in so upgrades can be detected after checkout.
 */
export const refreshAccount = <ThrowOnError extends boolean = false>(options?: Options<RefreshAccountData, ThrowOnError>): RequestResult<RefreshAccountResponses, unknown, ThrowOnError> => rpcRequest<RefreshAccountResponses, unknown, ThrowOnError>("refreshAccount", options);

export const exportBackupToCloud = <ThrowOnError extends boolean = false>(options?: Options<ExportBackupToCloudData, ThrowOnError>): RequestResult<ExportBackupToCloudResponses, unknown, ThrowOnError> => rpcRequest<ExportBackupToCloudResponses, unknown, ThrowOnError>("exportBackupToCloud", options);

export const importBackupFromCloud = <ThrowOnError extends boolean = false>(options?: Options<ImportBackupFromCloudData, ThrowOnError>): RequestResult<ImportBackupFromCloudResponses, unknown, ThrowOnError> => rpcRequest<ImportBackupFromCloudResponses, unknown, ThrowOnError>("importBackupFromCloud", options);

export const exportBackup = <ThrowOnError extends boolean = false>(options?: Options<ExportBackupData, ThrowOnError>): RequestResult<ExportBackupResponses, unknown, ThrowOnError> => rpcRequest<ExportBackupResponses, unknown, ThrowOnError>("exportBackup", options);

export const importBackup = <ThrowOnError extends boolean = false>(options: Options<ImportBackupData, ThrowOnError>): RequestResult<ImportBackupResponses, unknown, ThrowOnError> => rpcRequest<ImportBackupResponses, unknown, ThrowOnError>("importBackup", options);

export const getDebugProPolicy = <ThrowOnError extends boolean = false>(options?: Options<GetDebugProPolicyData, ThrowOnError>): RequestResult<GetDebugProPolicyResponses, unknown, ThrowOnError> => rpcRequest<GetDebugProPolicyResponses, unknown, ThrowOnError>("getDebugProPolicy", options);

export const setDebugProPolicy = <ThrowOnError extends boolean = false>(options: Options<SetDebugProPolicyData, ThrowOnError>): RequestResult<SetDebugProPolicyResponses, unknown, ThrowOnError> => rpcRequest<SetDebugProPolicyResponses, unknown, ThrowOnError>("setDebugProPolicy", options);

export const hideFiles = <ThrowOnError extends boolean = false>(options?: Options<HideFilesData, ThrowOnError>): RequestResult<HideFilesResponses, unknown, ThrowOnError> => rpcRequest<HideFilesResponses, unknown, ThrowOnError>("hideFiles", options);

export const showFiles = <ThrowOnError extends boolean = false>(options?: Options<ShowFilesData, ThrowOnError>): RequestResult<ShowFilesResponses, unknown, ThrowOnError> => rpcRequest<ShowFilesResponses, unknown, ThrowOnError>("showFiles", options);

export const resetTos = <ThrowOnError extends boolean = false>(options?: Options<ResetTosData, ThrowOnError>): RequestResult<ResetTosResponses, unknown, ThrowOnError> => rpcRequest<ResetTosResponses, unknown, ThrowOnError>("resetTos", options);



export const getUpdateCheckInterval = <ThrowOnError extends boolean = false>(options?: Options<GetUpdateCheckIntervalData, ThrowOnError>): RequestResult<GetUpdateCheckIntervalResponses, unknown, ThrowOnError> => rpcRequest<GetUpdateCheckIntervalResponses, unknown, ThrowOnError>("getUpdateCheckInterval", options);

export const setUpdateCheckInterval = <ThrowOnError extends boolean = false>(options: Options<SetUpdateCheckIntervalData, ThrowOnError>): RequestResult<SetUpdateCheckIntervalResponses, unknown, ThrowOnError> => rpcRequest<SetUpdateCheckIntervalResponses, unknown, ThrowOnError>("setUpdateCheckInterval", options);

export const getAllNetworkBlocks = <ThrowOnError extends boolean = false>(options?: Options<GetAllNetworkBlocksData, ThrowOnError>): RequestResult<GetAllNetworkBlocksResponses, unknown, ThrowOnError> => rpcRequest<GetAllNetworkBlocksResponses, unknown, ThrowOnError>("getAllNetworkBlocks", options);

export const createNetworkBlock = <ThrowOnError extends boolean = false>(options: Options<CreateNetworkBlockData, ThrowOnError>): RequestResult<CreateNetworkBlockResponses, CreateNetworkBlockErrors, ThrowOnError> => rpcRequest<CreateNetworkBlockResponses, CreateNetworkBlockErrors, ThrowOnError>("createNetworkBlock", options);

export const saveNetworkBlock = <ThrowOnError extends boolean = false>(options: Options<SaveNetworkBlockData, ThrowOnError>): RequestResult<SaveNetworkBlockResponses, unknown, ThrowOnError> => rpcRequest<SaveNetworkBlockResponses, unknown, ThrowOnError>("saveNetworkBlock", options);

export const getDashboardSnapshot = <ThrowOnError extends boolean = false>(options?: Options<GetDashboardSnapshotData, ThrowOnError>): RequestResult<GetDashboardSnapshotResponses, unknown, ThrowOnError> => rpcRequest<GetDashboardSnapshotResponses, unknown, ThrowOnError>("getDashboardSnapshot", options);

export const deleteNetworkBlock = <ThrowOnError extends boolean = false>(options: Options<DeleteNetworkBlockData, ThrowOnError>): RequestResult<DeleteNetworkBlockResponses, DeleteNetworkBlockErrors, ThrowOnError> => rpcRequest<DeleteNetworkBlockResponses, DeleteNetworkBlockErrors, ThrowOnError>("deleteNetworkBlock", options);

export const renameNetworkBlock = <ThrowOnError extends boolean = false>(options: Options<RenameNetworkBlockData, ThrowOnError>): RequestResult<RenameNetworkBlockResponses, RenameNetworkBlockErrors, ThrowOnError> => rpcRequest<RenameNetworkBlockResponses, RenameNetworkBlockErrors, ThrowOnError>("renameNetworkBlock", options);

export const setNetworkBlockEnabled = <ThrowOnError extends boolean = false>(options: Options<SetNetworkBlockEnabledData, ThrowOnError>): RequestResult<SetNetworkBlockEnabledResponses, SetNetworkBlockEnabledErrors, ThrowOnError> => rpcRequest<SetNetworkBlockEnabledResponses, SetNetworkBlockEnabledErrors, ThrowOnError>("setNetworkBlockEnabled", options);

export const setNetworkBlockIcon = <ThrowOnError extends boolean = false>(options: Options<SetNetworkBlockIconData, ThrowOnError>): RequestResult<SetNetworkBlockIconResponses, SetNetworkBlockIconErrors, ThrowOnError> => rpcRequest<SetNetworkBlockIconResponses, SetNetworkBlockIconErrors, ThrowOnError>("setNetworkBlockIcon", options);

export const getLock = <ThrowOnError extends boolean = false>(options: Options<GetLockData, ThrowOnError>): RequestResult<GetLockResponses, unknown, ThrowOnError> => rpcRequest<GetLockResponses, unknown, ThrowOnError>("getLock", options);

export const setLock = <ThrowOnError extends boolean = false>(options: Options<SetLockData, ThrowOnError>): RequestResult<SetLockResponses, unknown, ThrowOnError> => rpcRequest<SetLockResponses, unknown, ThrowOnError>("setLock", options);

export const setNetworkBlockSafeSearch = <ThrowOnError extends boolean = false>(options: Options<SetNetworkBlockSafeSearchData, ThrowOnError>): RequestResult<SetNetworkBlockSafeSearchResponses, SetNetworkBlockSafeSearchErrors, ThrowOnError> => rpcRequest<SetNetworkBlockSafeSearchResponses, SetNetworkBlockSafeSearchErrors, ThrowOnError>("setNetworkBlockSafeSearch", options);

export const getScheduleEntries = <ThrowOnError extends boolean = false>(options: Options<GetScheduleEntriesData, ThrowOnError>): RequestResult<GetScheduleEntriesResponses, unknown, ThrowOnError> => rpcRequest<GetScheduleEntriesResponses, unknown, ThrowOnError>("getScheduleEntries", options);

export const addScheduleEntries = <ThrowOnError extends boolean = false>(options: Options<AddScheduleEntriesData, ThrowOnError>): RequestResult<AddScheduleEntriesResponses, unknown, ThrowOnError> => rpcRequest<AddScheduleEntriesResponses, unknown, ThrowOnError>("addScheduleEntries", options);

export const getNetBlockToSitePackConnections = <ThrowOnError extends boolean = false>(options: Options<GetNetBlockToSitePackConnectionsData, ThrowOnError>): RequestResult<GetNetBlockToSitePackConnectionsResponses, unknown, ThrowOnError> => rpcRequest<GetNetBlockToSitePackConnectionsResponses, unknown, ThrowOnError>("getNetBlockToSitePackConnections", options);

export const addNetBlockToSitePackConnection = <ThrowOnError extends boolean = false>(options: Options<AddNetBlockToSitePackConnectionData, ThrowOnError>): RequestResult<AddNetBlockToSitePackConnectionResponses, unknown, ThrowOnError> => rpcRequest<AddNetBlockToSitePackConnectionResponses, unknown, ThrowOnError>("addNetBlockToSitePackConnection", options);

export const removeNetBlockToSitePackConnection = <ThrowOnError extends boolean = false>(options: Options<RemoveNetBlockToSitePackConnectionData, ThrowOnError>): RequestResult<RemoveNetBlockToSitePackConnectionResponses, unknown, ThrowOnError> => rpcRequest<RemoveNetBlockToSitePackConnectionResponses, unknown, ThrowOnError>("removeNetBlockToSitePackConnection", options);

export const confirmUnlock = <ThrowOnError extends boolean = false>(options: Options<ConfirmUnlockData, ThrowOnError>): RequestResult<ConfirmUnlockResponses, ConfirmUnlockErrors, ThrowOnError> => rpcRequest<ConfirmUnlockResponses, ConfirmUnlockErrors, ThrowOnError>("confirmUnlock", options);

export const initiateUnlock = <ThrowOnError extends boolean = false>(options: Options<InitiateUnlockData, ThrowOnError>): RequestResult<InitiateUnlockResponses, InitiateUnlockErrors, ThrowOnError> => rpcRequest<InitiateUnlockResponses, InitiateUnlockErrors, ThrowOnError>("initiateUnlock", options);

export const getUserRules = <ThrowOnError extends boolean = false>(options: Options<GetUserRulesData, ThrowOnError>): RequestResult<GetUserRulesResponses, unknown, ThrowOnError> => rpcRequest<GetUserRulesResponses, unknown, ThrowOnError>("getUserRules", options);

export const addUserRules = <ThrowOnError extends boolean = false>(options: Options<AddUserRulesData, ThrowOnError>): RequestResult<AddUserRulesResponses, unknown, ThrowOnError> => rpcRequest<AddUserRulesResponses, unknown, ThrowOnError>("addUserRules", options);

export const openProPage = <ThrowOnError extends boolean = false>(options?: Options<OpenProPageData, ThrowOnError>): RequestResult<OpenProPageResponses, OpenProPageErrors, ThrowOnError> => rpcRequest<OpenProPageResponses, OpenProPageErrors, ThrowOnError>("openProPage", options);

export const ping = <ThrowOnError extends boolean = false>(options?: Options<PingData, ThrowOnError>): RequestResult<PingResponses, unknown, ThrowOnError> => rpcRequest<PingResponses, unknown, ThrowOnError>("ping", options);

export const downloadResistanceScripts = <ThrowOnError extends boolean = false>(options?: Options<DownloadResistanceScriptsData, ThrowOnError>): RequestResult<DownloadResistanceScriptsResponses, unknown, ThrowOnError> => rpcRequest<DownloadResistanceScriptsResponses, unknown, ThrowOnError>("downloadResistanceScripts", options);

export const getAllScripts = <ThrowOnError extends boolean = false>(options?: Options<GetAllScriptsData, ThrowOnError>): RequestResult<GetAllScriptsResponses, unknown, ThrowOnError> => rpcRequest<GetAllScriptsResponses, unknown, ThrowOnError>("getAllScripts", options);

export const getScript = <ThrowOnError extends boolean = false>(options: Options<GetScriptData, ThrowOnError>): RequestResult<GetScriptResponses, GetScriptErrors, ThrowOnError> => rpcRequest<GetScriptResponses, GetScriptErrors, ThrowOnError>("getScript", options);

export const setScriptEnabled = <ThrowOnError extends boolean = false>(options: Options<SetScriptEnabledData, ThrowOnError>): RequestResult<SetScriptEnabledResponses, SetScriptEnabledErrors, ThrowOnError> => rpcRequest<SetScriptEnabledResponses, SetScriptEnabledErrors, ThrowOnError>("setScriptEnabled", options);

export const setResistanceModeEnabled = <ThrowOnError extends boolean = false>(options: Options<SetResistanceModeEnabledData, ThrowOnError>): RequestResult<SetResistanceModeEnabledResponses, SetResistanceModeEnabledErrors, ThrowOnError> => rpcRequest<SetResistanceModeEnabledResponses, SetResistanceModeEnabledErrors, ThrowOnError>("setResistanceModeEnabled", options);

export const setResistanceModeLock = <ThrowOnError extends boolean = false>(options: Options<SetResistanceModeLockData, ThrowOnError>): RequestResult<SetResistanceModeLockResponses, SetResistanceModeLockErrors, ThrowOnError> => rpcRequest<SetResistanceModeLockResponses, SetResistanceModeLockErrors, ThrowOnError>("setResistanceModeLock", options);

export const getResistanceModeSettings = <ThrowOnError extends boolean = false>(options?: Options<GetResistanceModeSettingsData, ThrowOnError>): RequestResult<GetResistanceModeSettingsResponses, unknown, ThrowOnError> => rpcRequest<GetResistanceModeSettingsResponses, unknown, ThrowOnError>("getResistanceModeSettings", options);



export const deleteScheduleEntry = <ThrowOnError extends boolean = false>(options: Options<DeleteScheduleEntryData, ThrowOnError>): RequestResult<DeleteScheduleEntryResponses, unknown, ThrowOnError> => rpcRequest<DeleteScheduleEntryResponses, unknown, ThrowOnError>("deleteScheduleEntry", options);

export const editScheduleEntry = <ThrowOnError extends boolean = false>(options: Options<EditScheduleEntryData, ThrowOnError>): RequestResult<EditScheduleEntryResponses, unknown, ThrowOnError> => rpcRequest<EditScheduleEntryResponses, unknown, ThrowOnError>("editScheduleEntry", options);

export const getAllSitePacks = <ThrowOnError extends boolean = false>(options?: Options<GetAllSitePacksData, ThrowOnError>): RequestResult<GetAllSitePacksResponses, unknown, ThrowOnError> => rpcRequest<GetAllSitePacksResponses, unknown, ThrowOnError>("getAllSitePacks", options);

export const createSitePack = <ThrowOnError extends boolean = false>(options: Options<CreateSitePackData, ThrowOnError, CreateSitePackResponse>): Promise<RpcStreamResult<CreateSitePackResponses>> => rpcStream<CreateSitePackResponses>("createSitePack", options);

export const reEnableRemoteSitePacks = <ThrowOnError extends boolean = false>(options?: Options<ReEnableRemoteSitePacksData, ThrowOnError>): RequestResult<ReEnableRemoteSitePacksResponses, unknown, ThrowOnError> => rpcRequest<ReEnableRemoteSitePacksResponses, unknown, ThrowOnError>("reEnableRemoteSitePacks", options);

export const deleteSitePackSource = <ThrowOnError extends boolean = false>(options: Options<DeleteSitePackSourceData, ThrowOnError>): RequestResult<DeleteSitePackSourceResponses, unknown, ThrowOnError> => rpcRequest<DeleteSitePackSourceResponses, unknown, ThrowOnError>("deleteSitePackSource", options);

export const deleteSitePack = <ThrowOnError extends boolean = false>(options: Options<DeleteSitePackData, ThrowOnError>): RequestResult<DeleteSitePackResponses, DeleteSitePackErrors, ThrowOnError> => rpcRequest<DeleteSitePackResponses, DeleteSitePackErrors, ThrowOnError>("deleteSitePack", options);

export const resyncSitePack = <ThrowOnError extends boolean = false>(options: Options<ResyncSitePackData, ThrowOnError, ResyncSitePackResponse>): Promise<RpcStreamResult<ResyncSitePackResponses>> => rpcStream<ResyncSitePackResponses>("resyncSitePack", options);

export const getSitePackSources = <ThrowOnError extends boolean = false>(options: Options<GetSitePackSourcesData, ThrowOnError>): RequestResult<GetSitePackSourcesResponses, unknown, ThrowOnError> => rpcRequest<GetSitePackSourcesResponses, unknown, ThrowOnError>("getSitePackSources", options);

export const addSitePackSource = <ThrowOnError extends boolean = false>(options: Options<AddSitePackSourceData, ThrowOnError>): RequestResult<AddSitePackSourceResponses, unknown, ThrowOnError> => rpcRequest<AddSitePackSourceResponses, unknown, ThrowOnError>("addSitePackSource", options);

export const checkUpdates = <ThrowOnError extends boolean = false>(options?: Options<CheckUpdatesData, ThrowOnError>): RequestResult<CheckUpdatesResponses, unknown, ThrowOnError> => rpcRequest<CheckUpdatesResponses, unknown, ThrowOnError>("checkUpdates", options);

export const triggerContentUpdate = <ThrowOnError extends boolean = false>(options?: Options<TriggerContentUpdateData, ThrowOnError>): RequestResult<TriggerContentUpdateResponses, TriggerContentUpdateErrors, ThrowOnError> => rpcRequest<TriggerContentUpdateResponses, TriggerContentUpdateErrors, ThrowOnError>("triggerContentUpdate", options);

export const getVersion = <ThrowOnError extends boolean = false>(options?: Options<GetVersionData, ThrowOnError>): RequestResult<GetVersionResponses, unknown, ThrowOnError> => rpcRequest<GetVersionResponses, unknown, ThrowOnError>("getVersion", options);

export const deleteUserRule = <ThrowOnError extends boolean = false>(options: Options<DeleteUserRuleData, ThrowOnError>): RequestResult<DeleteUserRuleResponses, unknown, ThrowOnError> => rpcRequest<DeleteUserRuleResponses, unknown, ThrowOnError>("deleteUserRule", options);
