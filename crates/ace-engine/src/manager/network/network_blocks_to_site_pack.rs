use std::sync::{Arc, RwLock};

use super::{NetworkManager, error::NetworkManagerOpErr};
use crate::core::network_block::site_pack::extended_category_remote_id;
use crate::core::{NetworkBlockId, SitePack, SitePackId};

impl NetworkManager {
    pub fn get_network_block_to_site_packs(
        &self,
        network_block_id: &NetworkBlockId,
    ) -> Result<Vec<Arc<RwLock<SitePack>>>, NetworkManagerOpErr> {
        Ok(self
            .get_network_block_from(network_block_id)
            .ok_or(NetworkManagerOpErr::NetworkBlockNotFound {
                id: network_block_id.clone(),
            })?
            .get_site_packs()
            .values()
            .cloned()
            .collect())
    }

    /// Number of site packs attached to a block that currently have usable
    /// local rules (synced). An unsynced pack — a metadata row whose data was
    /// never downloaded, or whose database is missing — enforces nothing, so
    /// reporting it as block coverage would be a "ghost" claim (dashboard,
    /// onboarding).
    pub fn count_synced_site_packs_of(
        &self,
        network_block_id: &NetworkBlockId,
    ) -> Result<usize, NetworkManagerOpErr> {
        let packs = self.get_network_block_to_site_packs(network_block_id)?;
        let mut count = 0;
        for pack_arc in packs {
            let pack = pack_arc
                .read()
                .map_err(|_| NetworkManagerOpErr::RwLockPoison)?;
            if pack.is_synced() {
                count += 1;
            }
        }
        Ok(count)
    }

    /// Find synchronized Extended companions of canonical Basic packs.
    /// Never infer an entitlement or a family from a user-controlled name.
    pub(super) fn ready_extended_category_ids(
        &self,
        pack_ids: &[SitePackId],
    ) -> Result<Vec<SitePackId>, NetworkManagerOpErr> {
        let mut wanted = std::collections::HashSet::new();
        for id in pack_ids {
            let pack = self.get_site_pack(id)?;
            if !pack.requires_pro()
                && let Some(remote_id) = pack.get_remote_id()
                && let Some(extended) = extended_category_remote_id(remote_id.as_ref())
            {
                wanted.insert(extended);
            }
        }
        let mut ready = Vec::new();
        for (id, pack) in &self.site_packs {
            let pack = pack.read().map_err(|_| NetworkManagerOpErr::RwLockPoison)?;
            if pack.requires_pro()
                && pack
                    .get_remote_id()
                    .is_some_and(|remote| wanted.contains(remote.as_ref() as &str))
                && pack.is_synced()
                && pack.get_db_path().join("pack.db").is_file()
            {
                ready.push(id.clone());
            }
        }
        Ok(ready)
    }

    /// Extend configured categories without removing any previously enforced
    /// rules. Recheck entitlement after the network/download phase.
    pub(crate) fn attach_ready_extended_categories(&mut self) -> Result<(), NetworkManagerOpErr> {
        if !self.is_pro_user_cached() {
            return Ok(());
        }
        let mut additions = Vec::new();
        for block in self.get_all_network_blocks() {
            let ids: Vec<_> = block.get_site_packs().keys().cloned().collect();
            for extended in self.ready_extended_category_ids(&ids)? {
                if !ids.contains(&extended) {
                    additions.push((block.get_id(), extended));
                }
            }
        }
        let mut first_error = None;
        for (block, pack) in additions {
            if let Err(error) = self.add_network_block_to_site_pack(&block, &pack) {
                tracing::warn!("Failed to extend category {pack} for block {block}: {error}");
                first_error.get_or_insert(error);
            }
        }
        first_error.map_or(Ok(()), Err)
    }

    pub fn add_network_block_to_site_pack(
        &mut self,
        network_block_id: &NetworkBlockId,
        site_pack_id: &SitePackId,
    ) -> Result<(), NetworkManagerOpErr> {
        let site_pack_arc = self
            .site_packs
            .get(site_pack_id)
            .ok_or(NetworkManagerOpErr::SitePackNotFound {
                id: site_pack_id.clone(),
            })?
            .clone();

        // Reject invalid targets before a potentially expensive lazy SQLite open.
        self.get_network_block_from(network_block_id).ok_or(
            NetworkManagerOpErr::NetworkBlockNotFound {
                id: network_block_id.clone(),
            },
        )?;
        if site_pack_arc.is_poisoned() {
            return Err(NetworkManagerOpErr::RwLockPoison);
        }
        {
            let pack = site_pack_arc.read().unwrap();
            if !pack.is_synced() {
                return Err(NetworkManagerOpErr::SitePackNotSynced {
                    id: site_pack_id.clone(),
                });
            }
            pack.ensure_db_open()
                .map_err(|error| NetworkManagerOpErr::DatabaseError {
                    message: format!(
                        "failed to open site pack {site_pack_id} for enforcement: {error}"
                    ),
                })?;
        }

        self.db
            .add_network_block_to_site_pack(network_block_id, site_pack_id)
            .map_err(|error| NetworkManagerOpErr::DatabaseError {
                message: format!(
                    "failed to add network block to site pack connection: {}",
                    error
                ),
            })?;

        self.get_mut_network_block_from(network_block_id)
            .expect("network block was validated before persistence")
            .add_site_pack(site_pack_id.clone(), site_pack_arc)
            .map_err(|error| NetworkManagerOpErr::InternalError {
                message: error.to_string(),
            })?;

        Ok(())
    }

    pub fn remove_network_block_to_site_pack(
        &mut self,
        network_block_id: &NetworkBlockId,
        site_pack_id: &SitePackId,
    ) -> Result<(), NetworkManagerOpErr> {
        let network_block = self.get_network_block_from(network_block_id).ok_or(
            NetworkManagerOpErr::NetworkBlockNotFound {
                id: network_block_id.clone(),
            },
        )?;

        if network_block.is_locked(&crate::trusted_time::locking_now()) {
            return Err(NetworkManagerOpErr::NetworkBlockIsLocked {
                id: network_block_id.clone(),
            });
        }

        self.db
            .remove_network_block_to_site_pack(network_block_id, site_pack_id)
            .map_err(|error| NetworkManagerOpErr::DatabaseError {
                message: format!(
                    "failed to remove network block to site pack connection: {}",
                    error
                ),
            })?;

        self.get_mut_network_block_from(network_block_id)
            .expect("network block was validated before persistence")
            .remove_site_pack(site_pack_id);
        self.release_unused_site_pack_pools();

        Ok(())
    }
}
