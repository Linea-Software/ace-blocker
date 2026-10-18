// Frontend facade for the typed engine RPC interface. No HTTP client.

import type { RequestResult, RpcStreamResult, RpcOptions } from './rpc-types';
import { rpcRequest, rpcStream } from './tauri-rpc';
import type { AcceptTosData, AcceptTosResponses, AddNetBlockToSitePackConnectionData, AddNetBlockToSitePackConnectionResponses, AddScheduleEntriesData, AddScheduleEntriesResponses, AddSitePackSourceData, AddSitePackSourceResponses, AddUserRulesData, AddUserRulesResponses, CheckUpdatesData, CheckUpdatesResponses, ConfirmUnlockData, ConfirmUnlockErrors, ConfirmUnlockResponses, CreateNetworkBlockData, CreateNetworkBlockErrors, CreateNetworkBlockResponses, CreateSitePackData, CreateSitePackResponse, CreateSitePackResponses, DeleteNetworkBlockData, DeleteNetworkBlockErrors, DeleteNetworkBlockResponses, DeleteScheduleEntryData, DeleteScheduleEntryResponses, DeleteSitePackData, DeleteSitePackErrors, DeleteSitePackResponses, DeleteSitePackSourceData, DeleteSitePackSourceResponses, DeleteUserRuleData, DeleteUserRuleResponses, DownloadResistanceScriptsData, DownloadResistanceScriptsResponses, EditScheduleEntryData, EditScheduleEntryResponses, ExportBackupData, ExportBackupResponses, ExportBackupToCloudData, ExportBackupToCloudResponses, GetAccountInfoData, GetAccountInfoResponses, GetAllNetworkBlocksData, GetAllNetworkBlocksResponses, GetAllScriptsData, GetAllScriptsResponses, GetAllSitePacksData, GetAllSitePacksResponses, GetDashboardSnapshotData, GetDashboardSnapshotResponses, GetDebugProPolicyData, GetDebugProPolicyResponses, GetDebugResistanceLevelData, GetDebugResistanceLevelResponses, GetLockData, GetLockResponses, GetNetBlockToSitePackConnectionsData, GetNetBlockToSitePackConnectionsResponses, GetResistanceModeSettingsData, GetResistanceModeSettingsResponses, GetResistanceModeTypeData, GetResistanceModeTypeResponses, GetScheduleEntriesData, GetScheduleEntriesResponses, GetScriptData, GetScriptErrors, GetScriptResponses, GetSitePackSourcesData, GetSitePackSourcesResponses, GetUpdateCheckIntervalData, GetUpdateCheckIntervalResponses, GetUserRulesData, GetUserRulesResponses, GetVersionData, GetVersionResponses, HideFilesData, HideFilesResponses, ImportBackupData, ImportBackupFromCloudData, ImportBackupFromCloudResponses, ImportBackupResponses, InitiateUnlockData, InitiateUnlockErrors, InitiateUnlockResponses, LogoutData, LogoutResponses, OpenProPageData, OpenProPageErrors, OpenProPageResponses, PingData, PingResponses, ReEnableRemoteSitePacksData, ReEnableRemoteSitePacksResponses, RefreshAccountData, RefreshAccountResponses, RemoveNetBlockToSitePackConnectionData, RemoveNetBlockToSitePackConnectionResponses, RenameNetworkBlockData, RenameNetworkBlockErrors, RenameNetworkBlockResponses, ResetTosData, ResetTosResponses, ResyncSitePackData, ResyncSitePackResponse, ResyncSitePackResponses, SetApiKeyData, SetApiKeyResponses, SetDebugProPolicyData, SetDebugProPolicyResponses, SetDebugResistanceLevelData, SetDebugResistanceLevelResponses, SetLockData, SetLockResponses, SetNetworkBlockEnabledData, SetNetworkBlockEnabledErrors, SetNetworkBlockEnabledResponses, SetNetworkBlockIconData, SetNetworkBlockIconErrors, SetNetworkBlockIconResponses, SetNetworkBlockSafeSearchData, SetNetworkBlockSafeSearchErrors, SetNetworkBlockSafeSearchResponses, SetResistanceModeEnabledData, SetResistanceModeEnabledErrors, SetResistanceModeEnabledResponses, SetResistanceModeLockData, SetResistanceModeLockErrors, SetResistanceModeLockResponses, SetResistanceModeTypeData, SetResistanceModeTypeErrors, SetResistanceModeTypeResponses, SetScriptEnabledData, SetScriptEnabledErrors, SetScriptEnabledResponses, SetUpdateCheckIntervalData, SetUpdateCheckIntervalResponses, ShowFilesData, ShowFilesResponses, ShutdownData, ShutdownErrors, ShutdownResponses, TriggerContentUpdateData, TriggerContentUpdateErrors, TriggerContentUpdateResponses } from './types';

export type Options<TData extends object = object, ThrowOnError extends boolean = boolean, TResponse = unknown> =
    RpcOptions<TData, ThrowOnError, TResponse>;

export const acceptTos = <ThrowOnError extends boolean = false>(options?: Options<AcceptTosData, ThrowOnError>): RequestResult<AcceptTosResponses, unknown, ThrowOnError> => rpcRequest("acceptTos", options) as any;

export const setApiKey = <ThrowOnError extends boolean = false>(options: Options<SetApiKeyData, ThrowOnError>): RequestResult<SetApiKeyResponses, unknown, ThrowOnError> => rpcRequest("setApiKey", options) as any;

export const getAccountInfo = <ThrowOnError extends boolean = false>(options?: Options<GetAccountInfoData, ThrowOnError>): RequestResult<GetAccountInfoResponses, unknown, ThrowOnError> => rpcRequest("getAccountInfo", options) as any;

export const logout = <ThrowOnError extends boolean = false>(options?: Options<LogoutData, ThrowOnError>): RequestResult<LogoutResponses, unknown, ThrowOnError> => rpcRequest("logout", options) as any;

/**
 * Re-read the account's Pro status after an event that may have changed it
 * (login, logout, subscription change, focus regain).
 *
 * The engine never calls the Ace server when the last successful
 * verification is less than one day old: this endpoint returns the cached
 * status in that case and performs exactly one single-flight remote
 * re-verification only when the cache is stale or absent. Transient errors
 * keep the previous entitlement; authorization failures clear the API key
 * and transition the account to free.
 */
export const refreshAccount = <ThrowOnError extends boolean = false>(options?: Options<RefreshAccountData, ThrowOnError>): RequestResult<RefreshAccountResponses, unknown, ThrowOnError> => rpcRequest("refreshAccount", options) as any;

export const exportBackupToCloud = <ThrowOnError extends boolean = false>(options?: Options<ExportBackupToCloudData, ThrowOnError>): RequestResult<ExportBackupToCloudResponses, unknown, ThrowOnError> => rpcRequest("exportBackupToCloud", options) as any;

export const importBackupFromCloud = <ThrowOnError extends boolean = false>(options?: Options<ImportBackupFromCloudData, ThrowOnError>): RequestResult<ImportBackupFromCloudResponses, unknown, ThrowOnError> => rpcRequest("importBackupFromCloud", options) as any;

export const exportBackup = <ThrowOnError extends boolean = false>(options?: Options<ExportBackupData, ThrowOnError>): RequestResult<ExportBackupResponses, unknown, ThrowOnError> => rpcRequest("exportBackup", options) as any;

export const importBackup = <ThrowOnError extends boolean = false>(options: Options<ImportBackupData, ThrowOnError>): RequestResult<ImportBackupResponses, unknown, ThrowOnError> => rpcRequest("importBackup", options) as any;

export const getDebugProPolicy = <ThrowOnError extends boolean = false>(options?: Options<GetDebugProPolicyData, ThrowOnError>): RequestResult<GetDebugProPolicyResponses, unknown, ThrowOnError> => rpcRequest("getDebugProPolicy", options) as any;

export const setDebugProPolicy = <ThrowOnError extends boolean = false>(options: Options<SetDebugProPolicyData, ThrowOnError>): RequestResult<SetDebugProPolicyResponses, unknown, ThrowOnError> => rpcRequest("setDebugProPolicy", options) as any;

export const hideFiles = <ThrowOnError extends boolean = false>(options?: Options<HideFilesData, ThrowOnError>): RequestResult<HideFilesResponses, unknown, ThrowOnError> => rpcRequest("hideFiles", options) as any;

export const showFiles = <ThrowOnError extends boolean = false>(options?: Options<ShowFilesData, ThrowOnError>): RequestResult<ShowFilesResponses, unknown, ThrowOnError> => rpcRequest("showFiles", options) as any;

export const resetTos = <ThrowOnError extends boolean = false>(options?: Options<ResetTosData, ThrowOnError>): RequestResult<ResetTosResponses, unknown, ThrowOnError> => rpcRequest("resetTos", options) as any;

export const getDebugResistanceLevel = <ThrowOnError extends boolean = false>(options?: Options<GetDebugResistanceLevelData, ThrowOnError>): RequestResult<GetDebugResistanceLevelResponses, unknown, ThrowOnError> => rpcRequest("getDebugResistanceLevel", options) as any;

export const setDebugResistanceLevel = <ThrowOnError extends boolean = false>(options: Options<SetDebugResistanceLevelData, ThrowOnError>): RequestResult<SetDebugResistanceLevelResponses, unknown, ThrowOnError> => rpcRequest("setDebugResistanceLevel", options) as any;

export const getUpdateCheckInterval = <ThrowOnError extends boolean = false>(options?: Options<GetUpdateCheckIntervalData, ThrowOnError>): RequestResult<GetUpdateCheckIntervalResponses, unknown, ThrowOnError> => rpcRequest("getUpdateCheckInterval", options) as any;

export const setUpdateCheckInterval = <ThrowOnError extends boolean = false>(options: Options<SetUpdateCheckIntervalData, ThrowOnError>): RequestResult<SetUpdateCheckIntervalResponses, unknown, ThrowOnError> => rpcRequest("setUpdateCheckInterval", options) as any;

export const getAllNetworkBlocks = <ThrowOnError extends boolean = false>(options?: Options<GetAllNetworkBlocksData, ThrowOnError>): RequestResult<GetAllNetworkBlocksResponses, unknown, ThrowOnError> => rpcRequest("getAllNetworkBlocks", options) as any;

export const createNetworkBlock = <ThrowOnError extends boolean = false>(options: Options<CreateNetworkBlockData, ThrowOnError>): RequestResult<CreateNetworkBlockResponses, CreateNetworkBlockErrors, ThrowOnError> => rpcRequest("createNetworkBlock", options) as any;

export const getDashboardSnapshot = <ThrowOnError extends boolean = false>(options?: Options<GetDashboardSnapshotData, ThrowOnError>): RequestResult<GetDashboardSnapshotResponses, unknown, ThrowOnError> => rpcRequest("getDashboardSnapshot", options) as any;

export const deleteNetworkBlock = <ThrowOnError extends boolean = false>(options: Options<DeleteNetworkBlockData, ThrowOnError>): RequestResult<DeleteNetworkBlockResponses, DeleteNetworkBlockErrors, ThrowOnError> => rpcRequest("deleteNetworkBlock", options) as any;

export const renameNetworkBlock = <ThrowOnError extends boolean = false>(options: Options<RenameNetworkBlockData, ThrowOnError>): RequestResult<RenameNetworkBlockResponses, RenameNetworkBlockErrors, ThrowOnError> => rpcRequest("renameNetworkBlock", options) as any;

export const setNetworkBlockEnabled = <ThrowOnError extends boolean = false>(options: Options<SetNetworkBlockEnabledData, ThrowOnError>): RequestResult<SetNetworkBlockEnabledResponses, SetNetworkBlockEnabledErrors, ThrowOnError> => rpcRequest("setNetworkBlockEnabled", options) as any;

export const setNetworkBlockIcon = <ThrowOnError extends boolean = false>(options: Options<SetNetworkBlockIconData, ThrowOnError>): RequestResult<SetNetworkBlockIconResponses, SetNetworkBlockIconErrors, ThrowOnError> => rpcRequest("setNetworkBlockIcon", options) as any;

export const getLock = <ThrowOnError extends boolean = false>(options: Options<GetLockData, ThrowOnError>): RequestResult<GetLockResponses, unknown, ThrowOnError> => rpcRequest("getLock", options) as any;

export const setLock = <ThrowOnError extends boolean = false>(options: Options<SetLockData, ThrowOnError>): RequestResult<SetLockResponses, unknown, ThrowOnError> => rpcRequest("setLock", options) as any;

export const setNetworkBlockSafeSearch = <ThrowOnError extends boolean = false>(options: Options<SetNetworkBlockSafeSearchData, ThrowOnError>): RequestResult<SetNetworkBlockSafeSearchResponses, SetNetworkBlockSafeSearchErrors, ThrowOnError> => rpcRequest("setNetworkBlockSafeSearch", options) as any;

export const getScheduleEntries = <ThrowOnError extends boolean = false>(options: Options<GetScheduleEntriesData, ThrowOnError>): RequestResult<GetScheduleEntriesResponses, unknown, ThrowOnError> => rpcRequest("getScheduleEntries", options) as any;

export const addScheduleEntries = <ThrowOnError extends boolean = false>(options: Options<AddScheduleEntriesData, ThrowOnError>): RequestResult<AddScheduleEntriesResponses, unknown, ThrowOnError> => rpcRequest("addScheduleEntries", options) as any;

export const getNetBlockToSitePackConnections = <ThrowOnError extends boolean = false>(options: Options<GetNetBlockToSitePackConnectionsData, ThrowOnError>): RequestResult<GetNetBlockToSitePackConnectionsResponses, unknown, ThrowOnError> => rpcRequest("getNetBlockToSitePackConnections", options) as any;

export const addNetBlockToSitePackConnection = <ThrowOnError extends boolean = false>(options: Options<AddNetBlockToSitePackConnectionData, ThrowOnError>): RequestResult<AddNetBlockToSitePackConnectionResponses, unknown, ThrowOnError> => rpcRequest("addNetBlockToSitePackConnection", options) as any;

export const removeNetBlockToSitePackConnection = <ThrowOnError extends boolean = false>(options: Options<RemoveNetBlockToSitePackConnectionData, ThrowOnError>): RequestResult<RemoveNetBlockToSitePackConnectionResponses, unknown, ThrowOnError> => rpcRequest("removeNetBlockToSitePackConnection", options) as any;

export const confirmUnlock = <ThrowOnError extends boolean = false>(options: Options<ConfirmUnlockData, ThrowOnError>): RequestResult<ConfirmUnlockResponses, ConfirmUnlockErrors, ThrowOnError> => rpcRequest("confirmUnlock", options) as any;

export const initiateUnlock = <ThrowOnError extends boolean = false>(options: Options<InitiateUnlockData, ThrowOnError>): RequestResult<InitiateUnlockResponses, InitiateUnlockErrors, ThrowOnError> => rpcRequest("initiateUnlock", options) as any;

export const getUserRules = <ThrowOnError extends boolean = false>(options: Options<GetUserRulesData, ThrowOnError>): RequestResult<GetUserRulesResponses, unknown, ThrowOnError> => rpcRequest("getUserRules", options) as any;

export const addUserRules = <ThrowOnError extends boolean = false>(options: Options<AddUserRulesData, ThrowOnError>): RequestResult<AddUserRulesResponses, unknown, ThrowOnError> => rpcRequest("addUserRules", options) as any;

export const openProPage = <ThrowOnError extends boolean = false>(options?: Options<OpenProPageData, ThrowOnError>): RequestResult<OpenProPageResponses, OpenProPageErrors, ThrowOnError> => rpcRequest("openProPage", options) as any;

export const ping = <ThrowOnError extends boolean = false>(options?: Options<PingData, ThrowOnError>): RequestResult<PingResponses, unknown, ThrowOnError> => rpcRequest("ping", options) as any;

export const downloadResistanceScripts = <ThrowOnError extends boolean = false>(options?: Options<DownloadResistanceScriptsData, ThrowOnError>): RequestResult<DownloadResistanceScriptsResponses, unknown, ThrowOnError> => rpcRequest("downloadResistanceScripts", options) as any;

export const getAllScripts = <ThrowOnError extends boolean = false>(options?: Options<GetAllScriptsData, ThrowOnError>): RequestResult<GetAllScriptsResponses, unknown, ThrowOnError> => rpcRequest("getAllScripts", options) as any;

export const getScript = <ThrowOnError extends boolean = false>(options: Options<GetScriptData, ThrowOnError>): RequestResult<GetScriptResponses, GetScriptErrors, ThrowOnError> => rpcRequest("getScript", options) as any;

export const setScriptEnabled = <ThrowOnError extends boolean = false>(options: Options<SetScriptEnabledData, ThrowOnError>): RequestResult<SetScriptEnabledResponses, SetScriptEnabledErrors, ThrowOnError> => rpcRequest("setScriptEnabled", options) as any;

export const setResistanceModeEnabled = <ThrowOnError extends boolean = false>(options: Options<SetResistanceModeEnabledData, ThrowOnError>): RequestResult<SetResistanceModeEnabledResponses, SetResistanceModeEnabledErrors, ThrowOnError> => rpcRequest("setResistanceModeEnabled", options) as any;

export const setResistanceModeLock = <ThrowOnError extends boolean = false>(options: Options<SetResistanceModeLockData, ThrowOnError>): RequestResult<SetResistanceModeLockResponses, SetResistanceModeLockErrors, ThrowOnError> => rpcRequest("setResistanceModeLock", options) as any;

export const getResistanceModeSettings = <ThrowOnError extends boolean = false>(options?: Options<GetResistanceModeSettingsData, ThrowOnError>): RequestResult<GetResistanceModeSettingsResponses, unknown, ThrowOnError> => rpcRequest("getResistanceModeSettings", options) as any;

export const getResistanceModeType = <ThrowOnError extends boolean = false>(options?: Options<GetResistanceModeTypeData, ThrowOnError>): RequestResult<GetResistanceModeTypeResponses, unknown, ThrowOnError> => rpcRequest("getResistanceModeType", options) as any;

export const setResistanceModeType = <ThrowOnError extends boolean = false>(options: Options<SetResistanceModeTypeData, ThrowOnError>): RequestResult<SetResistanceModeTypeResponses, SetResistanceModeTypeErrors, ThrowOnError> => rpcRequest("setResistanceModeType", options) as any;

export const deleteScheduleEntry = <ThrowOnError extends boolean = false>(options: Options<DeleteScheduleEntryData, ThrowOnError>): RequestResult<DeleteScheduleEntryResponses, unknown, ThrowOnError> => rpcRequest("deleteScheduleEntry", options) as any;

export const editScheduleEntry = <ThrowOnError extends boolean = false>(options: Options<EditScheduleEntryData, ThrowOnError>): RequestResult<EditScheduleEntryResponses, unknown, ThrowOnError> => rpcRequest("editScheduleEntry", options) as any;

export const getAllSitePacks = <ThrowOnError extends boolean = false>(options?: Options<GetAllSitePacksData, ThrowOnError>): RequestResult<GetAllSitePacksResponses, unknown, ThrowOnError> => rpcRequest("getAllSitePacks", options) as any;

export const createSitePack = <ThrowOnError extends boolean = false>(options: Options<CreateSitePackData, ThrowOnError, CreateSitePackResponse>): Promise<RpcStreamResult<CreateSitePackResponses>> => rpcStream("createSitePack", options) as any;

export const reEnableRemoteSitePacks = <ThrowOnError extends boolean = false>(options?: Options<ReEnableRemoteSitePacksData, ThrowOnError>): RequestResult<ReEnableRemoteSitePacksResponses, unknown, ThrowOnError> => rpcRequest("reEnableRemoteSitePacks", options) as any;

export const deleteSitePackSource = <ThrowOnError extends boolean = false>(options: Options<DeleteSitePackSourceData, ThrowOnError>): RequestResult<DeleteSitePackSourceResponses, unknown, ThrowOnError> => rpcRequest("deleteSitePackSource", options) as any;

export const deleteSitePack = <ThrowOnError extends boolean = false>(options: Options<DeleteSitePackData, ThrowOnError>): RequestResult<DeleteSitePackResponses, DeleteSitePackErrors, ThrowOnError> => rpcRequest("deleteSitePack", options) as any;

export const resyncSitePack = <ThrowOnError extends boolean = false>(options: Options<ResyncSitePackData, ThrowOnError, ResyncSitePackResponse>): Promise<RpcStreamResult<ResyncSitePackResponses>> => rpcStream("resyncSitePack", options) as any;

export const getSitePackSources = <ThrowOnError extends boolean = false>(options: Options<GetSitePackSourcesData, ThrowOnError>): RequestResult<GetSitePackSourcesResponses, unknown, ThrowOnError> => rpcRequest("getSitePackSources", options) as any;

export const addSitePackSource = <ThrowOnError extends boolean = false>(options: Options<AddSitePackSourceData, ThrowOnError>): RequestResult<AddSitePackSourceResponses, unknown, ThrowOnError> => rpcRequest("addSitePackSource", options) as any;

export const shutdown = <ThrowOnError extends boolean = false>(options?: Options<ShutdownData, ThrowOnError>): RequestResult<ShutdownResponses, ShutdownErrors, ThrowOnError> => rpcRequest("shutdown", options) as any;

export const checkUpdates = <ThrowOnError extends boolean = false>(options?: Options<CheckUpdatesData, ThrowOnError>): RequestResult<CheckUpdatesResponses, unknown, ThrowOnError> => rpcRequest("checkUpdates", options) as any;

export const triggerContentUpdate = <ThrowOnError extends boolean = false>(options?: Options<TriggerContentUpdateData, ThrowOnError>): RequestResult<TriggerContentUpdateResponses, TriggerContentUpdateErrors, ThrowOnError> => rpcRequest("triggerContentUpdate", options) as any;

export const getVersion = <ThrowOnError extends boolean = false>(options?: Options<GetVersionData, ThrowOnError>): RequestResult<GetVersionResponses, unknown, ThrowOnError> => rpcRequest("getVersion", options) as any;

export const deleteUserRule = <ThrowOnError extends boolean = false>(options: Options<DeleteUserRuleData, ThrowOnError>): RequestResult<DeleteUserRuleResponses, unknown, ThrowOnError> => rpcRequest("deleteUserRule", options) as any;
