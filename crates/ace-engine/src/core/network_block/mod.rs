//! Per-block rule evaluation. Activation (enabled flag and schedule) and
//! mutation guards (locks) are applied by the caller, not by URI matching.

pub mod regex_rules;
pub mod site_pack;
pub mod uri_hash;
pub mod user_defined_rules;

use chrono::{DateTime, Local};
use http;
use moka::sync::Cache;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;
use tracing::{debug, warn};
use uri_hash::UriHash;

use crate::core::common::constants::RESULTS_CACHE_MAX_SIZE;
use crate::core::common::lock::Lock;
use crate::core::common::schedule::{Schedule, ScheduleEntry};
use crate::core::network_block::site_pack::SitePackId;
use regex_rules::RegexRules;
use site_pack::SitePack;
use user_defined_rules::{UserDefinedRule, UserDefinedRules, UserDefinedRulesError};
use uuid::Uuid;

use nutype::nutype;

#[nutype(derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, From, Into, Display, Deref, AsRef
))]
pub struct NetworkBlockId(Uuid);

/// One independently scheduled rule set with shared site-pack handles.
/// URI decisions are cached per block, separately from activation checks.
#[derive(Debug, Clone)]
pub struct NetworkBlock {
    id: NetworkBlockId,
    name: String,
    enabled: bool,
    schedule: Schedule,
    lock: Lock,

    site_packs: HashMap<SitePackId, Arc<RwLock<SitePack>>>,
    user_defined_rules: UserDefinedRules,
    exceptions: RegexRules,
    safe_search: bool,
    icon: Option<String>,

    // Cache only rule decisions; the enabled switch and schedule are checked by callers.
    results_cache: Cache<UriHash, BlockingResult>,
}

#[derive(Error, Debug)]
pub enum NetworkBlockOpErr {
    #[error("Network block is locked and cannot be modified.")]
    ListIsLocked,
    #[error("Failed to add entries: {0}. Context: {1}")]
    EntriesAdditionError(#[source] regex::Error, String),
    #[error("Failed to remove entries: {0}. Context: {1}")]
    EntriesRemovalError(#[source] regex::Error, String),
    #[error("Failed to add user-defined rules: {0}")]
    UserRulesAdditionError(#[source] UserDefinedRulesError),
    #[error("Failed to remove user-defined rule: {0}")]
    UserRulesRemovalError(#[source] UserDefinedRulesError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockingResult {
    NotBlocked,
    Blocked,
}

impl BlockingResult {
    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Blocked)
    }
}

impl NetworkBlock {
    // Constructor with explicit fields; a builder would add boilerplate for
    // no current benefit.
    #[expect(clippy::too_many_arguments)]
    pub fn from(
        id: NetworkBlockId,
        name: String,
        enabled: bool,
        schedule: Schedule,
        lock: Lock,
        site_packs: HashMap<SitePackId, Arc<RwLock<SitePack>>>,
        user_defined_rules: UserDefinedRules,
        exceptions: RegexRules,
        safe_search: bool,
    ) -> NetworkBlock {
        NetworkBlock {
            id,
            name,
            enabled,
            schedule,
            lock,

            site_packs,
            user_defined_rules,
            exceptions,
            safe_search,
            icon: None,
            results_cache: Cache::builder()
                .max_capacity(RESULTS_CACHE_MAX_SIZE)
                .build(),
        }
    }

    // Constructor with explicit fields; a builder would add boilerplate for
    // no current benefit.
    #[expect(clippy::too_many_arguments)]
    pub fn from_with_icon(
        id: NetworkBlockId,
        name: String,
        enabled: bool,
        schedule: Schedule,
        lock: Lock,
        site_packs: HashMap<SitePackId, Arc<RwLock<SitePack>>>,
        user_defined_rules: UserDefinedRules,
        exceptions: RegexRules,
        safe_search: bool,
        icon: Option<String>,
    ) -> NetworkBlock {
        NetworkBlock {
            id,
            name,
            enabled,
            schedule,
            lock,

            site_packs,
            user_defined_rules,
            exceptions,
            safe_search,
            icon,
            results_cache: Cache::builder()
                .max_capacity(RESULTS_CACHE_MAX_SIZE)
                .build(),
        }
    }

    pub fn get_id(&self) -> NetworkBlockId {
        self.id.clone()
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn get_icon(&self) -> Option<String> {
        self.icon.clone()
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Changes the explicit switch without modifying schedule entries.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn set_icon(&mut self, icon: Option<String>) {
        self.icon = icon;
    }

    pub fn get_schedule(&self) -> &Schedule {
        &self.schedule
    }

    pub fn get_lock(&self) -> &Lock {
        &self.lock
    }

    pub fn get_site_packs(&self) -> &HashMap<SitePackId, Arc<RwLock<SitePack>>> {
        &self.site_packs
    }

    pub fn get_user_defined_rules(&self) -> &UserDefinedRules {
        &self.user_defined_rules
    }

    pub fn get_exceptions(&self) -> &RegexRules {
        &self.exceptions
    }

    pub fn is_safe_search_enabled(&self) -> bool {
        self.safe_search
    }

    pub fn set_safe_search(&mut self, enabled: bool) {
        self.safe_search = enabled;
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn add_user_defined_entries(
        &mut self,
        rules: Vec<UserDefinedRule>,
    ) -> Result<(), NetworkBlockOpErr> {
        self.user_defined_rules
            .add_rules(rules)
            .map_err(NetworkBlockOpErr::UserRulesAdditionError)?;
        self.results_cache.invalidate_all();
        Ok(())
    }

    pub fn remove_user_defined_entry(
        &mut self,
        rule: &UserDefinedRule,
    ) -> Result<(), NetworkBlockOpErr> {
        self.user_defined_rules
            .remove_rule(rule)
            .map_err(NetworkBlockOpErr::UserRulesRemovalError)?;
        self.results_cache.invalidate_all();
        Ok(())
    }

    pub fn add_exceptions(&mut self, patterns: Vec<String>) -> Result<(), NetworkBlockOpErr> {
        let patterns_clone = patterns.clone();
        self.exceptions.add_patterns(patterns).map_err(|error| {
            NetworkBlockOpErr::EntriesAdditionError(error, format!("{:?}", patterns_clone))
        })?;
        self.results_cache.invalidate_all();
        Ok(())
    }

    pub fn remove_exception(&mut self, pattern: &str) -> Result<(), NetworkBlockOpErr> {
        self.exceptions
            .remove_pattern(pattern)
            .map_err(|error| NetworkBlockOpErr::EntriesRemovalError(error, pattern.to_string()))?;
        self.results_cache.invalidate_all();
        Ok(())
    }

    pub fn add_site_pack(
        &mut self,
        site_pack_id: SitePackId,
        site_pack: Arc<RwLock<SitePack>>,
    ) -> Result<(), NetworkBlockOpErr> {
        self.site_packs.insert(site_pack_id, site_pack);
        self.results_cache.invalidate_all();
        Ok(())
    }

    pub fn has_site_pack(&self, site_pack_id: &SitePackId) -> bool {
        self.site_packs.contains_key(site_pack_id)
    }

    pub fn remove_site_pack(&mut self, site_pack_id: &SitePackId) {
        self.site_packs.remove(site_pack_id);
        self.results_cache.invalidate_all();
    }

    /// Existing packs can change their membership independently of this block.
    /// Call after activating a new pack database.
    pub(crate) fn invalidate_results_cache(&self) {
        self.results_cache.invalidate_all();
    }

    pub(crate) fn results_cache_handle(&self) -> Cache<UriHash, BlockingResult> {
        self.results_cache.clone()
    }

    pub fn set_schedule(&mut self, schedule_entries: Vec<ScheduleEntry>) {
        self.schedule = Schedule::from(schedule_entries);
    }

    pub fn set_lock(&mut self, lock: Lock) {
        self.lock = lock;
    }

    /// Evaluates the URI against rules and exceptions without checking whether
    /// the block is enabled or scheduled. Exceptions take precedence.
    /// Cached decisions must be considered when changing the underlying rules.
    pub fn is_content_blocked(&self, entry: &http::uri::Uri) -> BlockingResult {
        let uri_hash = UriHash::from_uri(entry);

        if let Some(blocking_result) = self.results_cache.get(&uri_hash) {
            return blocking_result;
        }

        let blocking_result = self.get_internal_content_blocking_result(entry);
        self.results_cache.insert(uri_hash, blocking_result.clone());

        blocking_result
    }

    fn blocks_host_without_request_path(&self, host: &str) -> bool {
        if self.user_defined_rules.matches_host(host) {
            return true;
        }

        for pack in self.site_packs.values() {
            if pack.is_poisoned() {
                continue;
            }

            if pack.read().unwrap().blocking_result(host) == BlockingResult::Blocked {
                return true;
            }
        }

        false
    }

    /// A CONNECT request only exposes the destination host, never the eventual
    /// HTTP path. Host-only rules can therefore be enforced before TLS without
    /// decrypting browser traffic, but path exceptions make that unsafe.
    pub(crate) fn blocks_connect_host(&self, host: &str) -> bool {
        self.exceptions.is_empty() && self.blocks_host_without_request_path(host)
    }

    /// Returns whether this block needs the decrypted HTTP request to make a
    /// correct decision for `host`.
    ///
    /// Regex rules can target arbitrary paths/queries, so while one is active we
    /// must inspect requests. Exceptions only require inspection for hosts that
    /// this block would otherwise block at hostname level.
    pub(crate) fn requires_request_inspection_for_host(&self, host: &str) -> bool {
        self.user_defined_rules.has_regex_rules()
            || (!self.exceptions.is_empty() && self.blocks_host_without_request_path(host))
    }

    /// Checks exceptions first, then user rules, then site-pack membership.
    /// The caller handles caching of the resulting decision.
    fn get_internal_content_blocking_result(&self, entry: &http::uri::Uri) -> BlockingResult {
        let entry_string = entry.to_string();
        if self.exceptions.contains(entry_string.as_str()) {
            return BlockingResult::NotBlocked;
        }

        if self.user_defined_rules.matches_uri(entry) {
            return BlockingResult::Blocked;
        }

        if let Some(host) = entry.host() {
            for pack in self.site_packs.values() {
                if pack.is_poisoned() {
                    warn!("Site pack is poisoned: {}", host);
                    continue;
                }

                let pack = pack.read().unwrap();

                if pack.blocking_result(host) == BlockingResult::Blocked {
                    debug!("Site pack matched: {}", host);
                    return BlockingResult::Blocked;
                }
            }
        }
        BlockingResult::NotBlocked
    }

    pub fn is_locked(&self, now: &DateTime<Local>) -> bool {
        self.lock.is_active(&self.schedule, now)
    }

    /// Enforcement requires both the enabled switch and a matching schedule.
    /// A lock constrains edits; it does not activate an otherwise inactive block.
    pub fn is_active(&self, now: &DateTime<Local>) -> bool {
        self.enabled && self.schedule.is_active_at(now)
    }
}
#[cfg(test)]
mod decision_cache_tests {
    use super::*;

    fn block() -> NetworkBlock {
        NetworkBlock::from(
            NetworkBlockId::from(Uuid::new_v4()),
            "test".into(),
            true,
            Schedule::default(),
            Lock::default(),
            HashMap::new(),
            UserDefinedRules::default(),
            RegexRules::default(),
            false,
        )
    }

    #[test]
    fn cached_allow_changes_after_adding_and_removing_rules() {
        let mut block = block();
        let uri: http::Uri = "https://example.com/path".parse().unwrap();
        let rule = UserDefinedRule::new(
            "example.com".into(),
            user_defined_rules::UserDefinedRuleType::Domain,
        );
        assert!(!block.is_content_blocked(&uri).is_blocked());
        block.add_user_defined_entries(vec![rule.clone()]).unwrap();
        assert!(block.is_content_blocked(&uri).is_blocked());
        block.remove_user_defined_entry(&rule).unwrap();
        assert!(!block.is_content_blocked(&uri).is_blocked());
    }

    #[test]
    fn cached_block_changes_after_adding_and_removing_exceptions() {
        let mut block = block();
        let uri: http::Uri = "https://example.com/path".parse().unwrap();
        block
            .add_user_defined_entries(vec![UserDefinedRule::new(
                "example.com".into(),
                user_defined_rules::UserDefinedRuleType::Domain,
            )])
            .unwrap();
        assert!(block.is_content_blocked(&uri).is_blocked());
        block.add_exceptions(vec!["example\\.com".into()]).unwrap();
        assert!(!block.is_content_blocked(&uri).is_blocked());
        block.remove_exception("example\\.com").unwrap();
        assert!(block.is_content_blocked(&uri).is_blocked());
    }
}
