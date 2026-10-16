//! Generated types from OpenAPI specification
//!
//! This file contains all the generated types for the API.
//! Do not edit manually - regenerate using the appropriate script.
#![allow(clippy::large_enum_variant)]
#![allow(clippy::format_in_format_args)]
#![allow(clippy::let_unit_value)]
#![allow(unreachable_patterns)]
use serde::{Deserialize, Serialize};
/// UTF-8 JSON string codec for `bytes::Bytes` model fields
/// produced from `format: binary`. Raw HTTP body and multipart
/// paths use their byte carriers directly and do not invoke it.
mod binary_bytes_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(
        bytes: &bytes::Bytes,
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        let value = std::str::from_utf8(bytes.as_ref())
            .map_err(serde::ser::Error::custom)?;
        ser.serialize_str(value)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(
        de: D,
    ) -> Result<bytes::Bytes, D::Error> {
        String::deserialize(de).map(bytes::Bytes::from)
    }
    pub mod option {
        use serde::{Deserialize, Deserializer, Serializer};
        pub fn serialize<S: Serializer>(
            value: &Option<bytes::Bytes>,
            ser: S,
        ) -> Result<S::Ok, S::Error> {
            match value {
                Some(bytes) => super::serialize(bytes, ser),
                None => ser.serialize_none(),
            }
        }
        pub fn deserialize<'de, D: Deserializer<'de>>(
            de: D,
        ) -> Result<Option<bytes::Bytes>, D::Error> {
            Option::<String>::deserialize(de).map(|value| value.map(bytes::Bytes::from))
        }
    }
}
/// Serde normally maps both a missing `Option<T>` field and an
/// explicit JSON null to `None`. Wrapping the decoded value in
/// `Some` retains the field-presence bit for `Option<Option<T>>`.
mod tri_state_serde {
    use serde::{Deserialize, Deserializer};
    pub fn deserialize<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        T::deserialize(de).map(Some)
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UploadSitePackRequest {
    #[serde(default, with = "binary_bytes_serde")]
    pub file: bytes::Bytes,
    pub id: String,
    pub name: String,
    #[serde(rename = "requiresProSubscription")]
    pub requires_pro_subscription: UploadSitePackRequestRequiresProSubscription,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UploadSitePackRequestRequiresProSubscription {
    Boolean(bool),
    String(String),
}
pub type ListApiKeysResponse = Vec<ListApiKeysResponseItem>;
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListApiKeysResponseItem {
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: Option<String>,
    pub id: String,
    #[serde(rename = "lastRequest")]
    pub last_request: Option<String>,
    pub name: Option<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetProDetailsResponse {
    pub expiration: Option<GetProDetailsResponseExpiration>,
    #[serde(rename = "isPro")]
    pub is_pro: bool,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum GetProDetailsResponseExpiration {
    GetProDetailsResponseExpirationInlineVariant(
        GetProDetailsResponseExpirationInlineVariant,
    ),
    DateTime(chrono::DateTime<chrono::Utc>),
    Date(chrono::NaiveDate),
    Number(f64),
}
pub type GetProDetailsResponseExpirationInlineVariant = serde_json::Value;
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetLatestReleasesVersionDataResponse {
    pub installer: Option<GetLatestReleasesVersionDataResponseInstaller>,
    pub resistance: Option<GetLatestReleasesVersionDataResponseResistance>,
    #[serde(rename = "sitePacks")]
    pub site_packs: Vec<GetLatestReleasesVersionDataResponseSitePacksItem>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetLatestReleasesVersionDataResponseSitePacksItem {
    pub description: String,
    pub id: String,
    pub name: String,
    #[serde(rename = "requiresPro")]
    pub requires_pro: bool,
    pub size: f64,
    #[serde(rename = "updatedAt")]
    pub updated_at: GetLatestReleasesVersionDataResponseSitePacksItemUpdatedAt,
    pub version: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum GetLatestReleasesVersionDataResponseSitePacksItemUpdatedAt {
    GetLatestReleasesVersionDataResponseSitePacksItemUpdatedAtInlineVariant(
        GetLatestReleasesVersionDataResponseSitePacksItemUpdatedAtInlineVariant,
    ),
    DateTime(chrono::DateTime<chrono::Utc>),
    Date(chrono::NaiveDate),
    Number(f64),
}
pub type GetLatestReleasesVersionDataResponseSitePacksItemUpdatedAtInlineVariant = serde_json::Value;
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetLatestReleasesVersionDataResponseResistance {
    #[serde(rename = "createdAt")]
    pub created_at: GetLatestReleasesVersionDataResponseResistanceCreatedAt,
    pub sha256: String,
    pub size: f64,
    pub version: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum GetLatestReleasesVersionDataResponseResistanceCreatedAt {
    GetLatestReleasesVersionDataResponseResistanceCreatedAtInlineVariant(
        GetLatestReleasesVersionDataResponseResistanceCreatedAtInlineVariant,
    ),
    DateTime(chrono::DateTime<chrono::Utc>),
    Date(chrono::NaiveDate),
    Number(f64),
}
pub type GetLatestReleasesVersionDataResponseResistanceCreatedAtInlineVariant = serde_json::Value;
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetLatestReleasesVersionDataResponseInstaller {
    pub architecture: String,
    #[serde(rename = "createdAt")]
    pub created_at: GetLatestReleasesVersionDataResponseInstallerCreatedAt,
    pub os: String,
    pub size: f64,
    pub version: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum GetLatestReleasesVersionDataResponseInstallerCreatedAt {
    GetLatestReleasesVersionDataResponseInstallerCreatedAtInlineVariant(
        GetLatestReleasesVersionDataResponseInstallerCreatedAtInlineVariant,
    ),
    DateTime(chrono::DateTime<chrono::Utc>),
    Date(chrono::NaiveDate),
    Number(f64),
}
pub type GetLatestReleasesVersionDataResponseInstallerCreatedAtInlineVariant = serde_json::Value;
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FetchBackupResponse {
    #[serde(rename = "customSitesLists")]
    pub custom_sites_lists: Vec<FetchBackupResponseCustomSitesListsItem>,
    #[serde(rename = "deactivatedResistanceModeScripts")]
    pub deactivated_resistance_mode_scripts: Vec<String>,
    #[serde(rename = "networkBlocks")]
    pub network_blocks: Vec<FetchBackupResponseNetworkBlocksItem>,
    pub version: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FetchBackupResponseNetworkBlocksItem {
    #[serde(rename = "connectedSitesLists")]
    pub connected_sites_lists: Vec<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "tri_state_serde::deserialize"
    )]
    pub icon: Option<Option<String>>,
    pub id: uuid::Uuid,
    pub name: String,
    pub safe_search: bool,
    pub schedules: Vec<FetchBackupResponseNetworkBlocksItemSchedulesItem>,
    #[serde(rename = "userRules")]
    pub user_rules: Vec<FetchBackupResponseNetworkBlocksItemUserRulesItem>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FetchBackupResponseNetworkBlocksItemUserRulesItem {
    pub rule: String,
    #[serde(rename = "ruleType")]
    pub rule_type: FetchBackupResponseNetworkBlocksItemUserRulesItemRuleType,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
pub enum FetchBackupResponseNetworkBlocksItemUserRulesItemRuleType {
    #[default]
    #[serde(rename = "domain")]
    Domain,
    #[serde(rename = "regex")]
    Regex,
}
impl FetchBackupResponseNetworkBlocksItemUserRulesItemRuleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Domain => "domain",
            Self::Regex => "regex",
        }
    }
}
impl ::std::fmt::Display for FetchBackupResponseNetworkBlocksItemUserRulesItemRuleType {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        f.write_str(self.as_str())
    }
}
impl AsRef<str> for FetchBackupResponseNetworkBlocksItemUserRulesItemRuleType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FetchBackupResponseNetworkBlocksItemSchedulesItem {
    #[serde(rename = "endTime")]
    pub end_time: String,
    pub friday: bool,
    pub monday: bool,
    pub saturday: bool,
    #[serde(rename = "startTime")]
    pub start_time: String,
    pub sunday: bool,
    pub thursday: bool,
    pub tuesday: bool,
    pub wednesday: bool,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FetchBackupResponseCustomSitesListsItem {
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "tri_state_serde::deserialize"
    )]
    pub description: Option<Option<String>>,
    ///Constraint: minLength=1, maxLength=128
    pub id: String,
    pub name: String,
    #[serde(
        rename = "remoteId",
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "tri_state_serde::deserialize"
    )]
    pub remote_id: Option<Option<String>>,
    #[serde(rename = "requiresPro", skip_serializing_if = "Option::is_none")]
    pub requires_pro: Option<bool>,
    pub sources: Vec<FetchBackupResponseCustomSitesListsItemSourcesItem>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "tri_state_serde::deserialize"
    )]
    pub version: Option<Option<String>>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FetchBackupResponseCustomSitesListsItemSourcesItem {
    pub format: String,
    pub source: String,
    #[serde(rename = "sourceType")]
    pub source_type: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateBackupRequest {
    #[serde(rename = "customSitesLists")]
    pub custom_sites_lists: Vec<CreateBackupRequestCustomSitesListsItem>,
    #[serde(rename = "deactivatedResistanceModeScripts")]
    pub deactivated_resistance_mode_scripts: Vec<String>,
    #[serde(rename = "networkBlocks")]
    pub network_blocks: Vec<CreateBackupRequestNetworkBlocksItem>,
    pub version: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateBackupRequestNetworkBlocksItem {
    #[serde(rename = "connectedSitesLists")]
    pub connected_sites_lists: Vec<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "tri_state_serde::deserialize"
    )]
    pub icon: Option<Option<String>>,
    pub id: uuid::Uuid,
    pub name: String,
    pub safe_search: bool,
    pub schedules: Vec<CreateBackupRequestNetworkBlocksItemSchedulesItem>,
    #[serde(rename = "userRules")]
    pub user_rules: Vec<CreateBackupRequestNetworkBlocksItemUserRulesItem>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateBackupRequestNetworkBlocksItemUserRulesItem {
    pub rule: String,
    #[serde(rename = "ruleType")]
    pub rule_type: CreateBackupRequestNetworkBlocksItemUserRulesItemRuleType,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
pub enum CreateBackupRequestNetworkBlocksItemUserRulesItemRuleType {
    #[default]
    #[serde(rename = "domain")]
    Domain,
    #[serde(rename = "regex")]
    Regex,
}
impl CreateBackupRequestNetworkBlocksItemUserRulesItemRuleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Domain => "domain",
            Self::Regex => "regex",
        }
    }
}
impl ::std::fmt::Display for CreateBackupRequestNetworkBlocksItemUserRulesItemRuleType {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        f.write_str(self.as_str())
    }
}
impl AsRef<str> for CreateBackupRequestNetworkBlocksItemUserRulesItemRuleType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateBackupRequestNetworkBlocksItemSchedulesItem {
    #[serde(rename = "endTime")]
    pub end_time: String,
    pub friday: bool,
    pub monday: bool,
    pub saturday: bool,
    #[serde(rename = "startTime")]
    pub start_time: String,
    pub sunday: bool,
    pub thursday: bool,
    pub tuesday: bool,
    pub wednesday: bool,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateBackupRequestCustomSitesListsItem {
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "tri_state_serde::deserialize"
    )]
    pub description: Option<Option<String>>,
    ///Constraint: minLength=1, maxLength=128
    pub id: String,
    pub name: String,
    #[serde(
        rename = "remoteId",
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "tri_state_serde::deserialize"
    )]
    pub remote_id: Option<Option<String>>,
    #[serde(rename = "requiresPro", skip_serializing_if = "Option::is_none")]
    pub requires_pro: Option<bool>,
    pub sources: Vec<CreateBackupRequestCustomSitesListsItemSourcesItem>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "tri_state_serde::deserialize"
    )]
    pub version: Option<Option<String>>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateBackupRequestCustomSitesListsItemSourcesItem {
    pub format: String,
    pub source: String,
    #[serde(rename = "sourceType")]
    pub source_type: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CancelSubscriptionResponse {
    pub success: bool,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateApiKeyResponse {
    #[serde(rename = "apiKey")]
    pub api_key: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateBackupResponse {
    pub id: uuid::Uuid,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeleteAccountRequest {
    pub email: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeleteAccountResponse {
    pub success: bool,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeleteInstallerRequest {
    pub id: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeleteInstallerResponse {
    pub message: String,
    pub success: bool,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeleteInstallerResponse400 {
    pub error: bool,
    pub message: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeleteInstallerResponse500 {
    pub error: bool,
    pub message: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeleteResistanceScriptRequest {
    pub id: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeleteResistanceScriptResponse {
    pub message: String,
    pub success: bool,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeleteResistanceScriptResponse400 {
    pub error: bool,
    pub message: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeleteResistanceScriptResponse500 {
    pub error: bool,
    pub message: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeleteSitePackRequest {
    pub id: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeleteSitePackResponse {
    pub message: String,
    pub success: bool,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeleteSitePackResponse400 {
    pub error: bool,
    pub message: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeleteSitePackResponse500 {
    pub error: bool,
    pub message: String,
}
pub type ExportAccountDataResponse = serde_json::Value;
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetAccountDetailsResponse {
    pub email: String,
    #[serde(rename = "isPro")]
    pub is_pro: bool,
    pub name: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetInstallersResponse {
    pub installers: Vec<serde_json::Value>,
    #[serde(rename = "totalDownloads")]
    pub total_downloads: f64,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetResistanceModeScriptsResponse {
    pub sha256: String,
    pub url: String,
    pub version: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetResistanceScriptsResponse {
    pub releases: Vec<serde_json::Value>,
    #[serde(rename = "totalDownloads")]
    pub total_downloads: f64,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetSitePackBackupResponse {
    pub url: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetSitePacksResponse {
    #[serde(rename = "sitePacks")]
    pub site_packs: Vec<serde_json::Value>,
    #[serde(rename = "totalDownloads")]
    pub total_downloads: f64,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PostApiV1AdminUsersDeleteRequest {
    #[serde(rename = "userId")]
    pub user_id: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PostApiV1AdminUsersGrantProRequest {
    pub duration: serde_json::Value,
    pub unit: String,
    #[serde(rename = "userId")]
    pub user_id: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PostApiV1AdminUsersToggleStatusRequest {
    #[serde(rename = "isBanned")]
    pub is_banned: serde_json::Value,
    #[serde(rename = "userId")]
    pub user_id: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RevokeApiKeyRequest {
    #[serde(rename = "keyId")]
    pub key_id: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RevokeApiKeyResponse {
    pub success: bool,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RevokeSessionRequest {
    #[serde(rename = "sessionId")]
    pub session_id: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RevokeSessionResponse {
    pub success: bool,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SetInstallerDownloadableRequest {
    #[serde(rename = "canBeDownloaded")]
    pub can_be_downloaded: bool,
    pub id: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SetInstallerDownloadableResponse {
    pub message: String,
    pub success: bool,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SetInstallerDownloadableResponse400 {
    pub error: bool,
    pub message: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SetInstallerDownloadableResponse404 {
    pub error: bool,
    pub message: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SetInstallerDownloadableResponse500 {
    pub error: bool,
    pub message: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SetResistanceScriptDownloadableRequest {
    #[serde(rename = "canBeDownloaded")]
    pub can_be_downloaded: bool,
    pub id: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SetResistanceScriptDownloadableResponse {
    pub message: String,
    pub success: bool,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SetResistanceScriptDownloadableResponse400 {
    pub error: bool,
    pub message: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SetResistanceScriptDownloadableResponse404 {
    pub error: bool,
    pub message: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SetResistanceScriptDownloadableResponse500 {
    pub error: bool,
    pub message: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SyncSubscriptionResponse {
    pub changed: bool,
    pub expiration: Option<String>,
    #[serde(rename = "isPro")]
    pub is_pro: bool,
    pub success: bool,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateSitePackRequest {
    pub id: String,
    pub name: String,
    #[serde(rename = "requiresProSubscription")]
    pub requires_pro_subscription: bool,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateSitePackResponse {
    pub message: String,
    pub success: bool,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateSitePackResponse400 {
    pub error: bool,
    pub message: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateSitePackResponse500 {
    pub error: bool,
    pub message: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UploadInstallerRequest {
    pub architecture: String,
    #[serde(default, with = "binary_bytes_serde")]
    pub file: bytes::Bytes,
    pub os: String,
    pub version: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UploadInstallerResponse {
    pub message: String,
    pub success: bool,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UploadInstallerResponse400 {
    pub error: bool,
    pub message: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UploadInstallerResponse500 {
    pub error: bool,
    pub message: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UploadResistanceScriptRequest {
    #[serde(default, with = "binary_bytes_serde")]
    pub file: bytes::Bytes,
    pub version: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UploadResistanceScriptResponse {
    pub message: String,
    pub success: bool,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UploadResistanceScriptResponse400 {
    pub error: bool,
    pub message: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UploadResistanceScriptResponse500 {
    pub error: bool,
    pub message: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UploadSitePackResponse {
    pub message: String,
    pub success: bool,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UploadSitePackResponse400 {
    pub error: bool,
    pub message: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UploadSitePackResponse500 {
    pub error: bool,
    pub message: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VerifyProStatusResponse {
    #[serde(rename = "isActive")]
    pub is_active: bool,
}
