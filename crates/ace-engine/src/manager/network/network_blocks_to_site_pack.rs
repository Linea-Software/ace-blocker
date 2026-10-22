use std::sync::{Arc, RwLock};

use super::{NetworkManager, error::NetworkManagerOpErr};
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
    pub fn count_synced_site_packs_of(&self, network_block_id: &NetworkBlockId) -> usize {
        self.get_network_block_to_site_packs(network_block_id)
            .map(|packs| {
                packs
                    .iter()
                    .filter(|pack_arc| {
                        pack_arc
                            .read()
                            .map(|pack| pack.is_synced())
                            .unwrap_or_else(|error| {
                                tracing::warn!(network_block_id = %network_block_id, %error,
                                    "Site-pack read lock poisoned while counting coverage");
                                false
                            })
                    })
                    .count()
            })
            .unwrap_or_else(|error| {
                tracing::warn!(network_block_id = %network_block_id, %error,
                    "Unable to count synced site packs; reporting zero coverage");
                0
            })
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

        if site_pack_arc.is_poisoned() {
            return Err(NetworkManagerOpErr::RwLockPoison);
        }
        if !site_pack_arc.read().unwrap().is_synced() {
            return Err(NetworkManagerOpErr::SitePackNotSynced {
                id: site_pack_id.clone(),
            });
        }

        // Validate every precondition before publishing a live policy change.
        self.get_network_block_from(network_block_id).ok_or(
            NetworkManagerOpErr::NetworkBlockNotFound {
                id: network_block_id.clone(),
            },
        )?;

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

        if network_block.is_locked(&chrono::Local::now()) {
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

        Ok(())
    }
}
