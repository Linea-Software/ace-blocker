use crate::{
    db::{DBError, EngineDB, models::SitePackModel},
    schema_main_db::site_packs,
};
use diesel::prelude::*;

/// Derives a canonical icon slug from a site pack name.
///
/// The slug vocabulary is the contract shared with
/// `ace-app/src/lib/utils/pack-icons.ts` (see the cross-repository
/// dependency-tracing skill): canonical category slugs are stored in
/// `site_packs.icon` and resolved to Lucide components client-side. Names are
/// matched by category word so tier or wording changes ("— Free"/"— Pro")
/// keep the derivation working. Returns `None` when the name matches no known
/// category.
pub(crate) fn derive_icon_from_name(name: &str) -> Option<String> {
    let name_lower = name.to_lowercase();
    if name_lower.contains("porn") || name_lower.contains("adult") {
        Some("adult".to_string())
    } else if name_lower.contains("social") {
        Some("social".to_string())
    } else if name_lower.contains("gaming") || name_lower.contains("game") {
        Some("gaming".to_string())
    } else if name_lower.contains("gambling")
        || name_lower.contains("betting")
        || name_lower.contains("casino")
    {
        Some("gambling".to_string())
    } else if name_lower.contains("messaging")
        || name_lower.contains("message")
        || name_lower.contains("chat")
    {
        Some("messaging".to_string())
    } else if name_lower.contains("streaming") || name_lower.contains("video") {
        Some("streaming".to_string())
    } else if name_lower.contains("shopping") {
        Some("shopping".to_string())
    } else if name_lower.contains("phishing") {
        Some("phishing".to_string())
    } else if name_lower.contains("piracy") {
        Some("piracy".to_string())
    } else if name_lower.contains("news") {
        Some("news".to_string())
    } else {
        None
    }
}

impl EngineDB {
    pub fn get_all_site_packs(&self) -> Result<Vec<SitePackModel>, DBError> {
        let conn = &mut self.get_db_connection()?;

        site_packs::table
            .select(SitePackModel::as_select())
            .filter(site_packs::is_ignored.eq(false))
            .load::<SitePackModel>(conn)
            .map_err(|error| DBError::DataFetchError {
                table: "site_packs".to_string(),
                message: error.to_string(),
            })
    }

    pub fn get_site_pack(&self, site_pack_id: &str) -> Result<SitePackModel, DBError> {
        use crate::schema_main_db::site_packs::dsl::*;
        let conn = &mut self.get_db_connection()?;

        site_packs
            .filter(id.eq(site_pack_id))
            .select(SitePackModel::as_select())
            .first(conn)
            .map_err(|error| DBError::DataFetchError {
                table: "site_packs".to_string(),
                message: error.to_string(),
            })
    }

    pub fn add_default_site_pack(
        &self,
        id: &str,
        remote_id: &str,
        name: &str,
        description: Option<String>,
        requires_pro: bool,
        version: &str,
    ) -> Result<SitePackModel, DBError> {
        use crate::schema_main_db::site_packs::dsl;
        let conn = &mut self.get_db_connection()?;

        diesel::insert_into(site_packs::table)
            .values((
                dsl::id.eq(id),
                dsl::remote_id.eq(Some(remote_id.to_string())),
                dsl::name.eq(name),
                dsl::description.eq(description),
                dsl::requires_pro.eq(requires_pro),
                dsl::last_updated_at.eq(None::<String>),
                dsl::version.eq(Some(version.to_string())),
                dsl::is_ignored.eq(false),
                dsl::icon.eq(None::<String>),
            ))
            .get_result::<SitePackModel>(conn)
            .map_err(|error| DBError::InsertionError {
                table: "site_packs".to_string(),
                message: error.to_string(),
            })
    }

    pub fn add_site_pack(
        &self,
        id: &str,
        name: &str,
        description: Option<String>,
        requires_pro: bool,
        icon: Option<String>,
    ) -> Result<SitePackModel, DBError> {
        use crate::schema_main_db::site_packs::dsl;
        let conn = &mut self.get_db_connection()?;

        diesel::insert_into(site_packs::table)
            .values((
                dsl::id.eq(id),
                dsl::name.eq(name),
                dsl::description.eq(description),
                dsl::requires_pro.eq(requires_pro),
                dsl::last_updated_at.eq(None::<String>),
                dsl::version.eq(None::<String>),
                dsl::is_ignored.eq(false),
                dsl::icon.eq(icon),
            ))
            .get_result::<SitePackModel>(conn)
            .map_err(|error| DBError::InsertionError {
                table: "site_packs".to_string(),
                message: error.to_string(),
            })
    }

    /// Create a local pack and all source rows in one main-DB transaction.
    /// The manager must publish the new pack into live state only after commit.
    pub fn add_local_site_pack_with_sources(
        &self,
        pack_id: &str,
        pack_name: &str,
        description: Option<String>,
        icon: Option<String>,
        sources: &[crate::core::importer::SitePackSource],
    ) -> Result<(), DBError> {
        use crate::schema_main_db::{site_pack_sources, site_packs};
        let conn = &mut self.get_db_connection()?;
        conn.transaction::<(), diesel::result::Error, _>(|conn| {
            diesel::insert_into(site_packs::table)
                .values((
                    site_packs::id.eq(pack_id),
                    site_packs::name.eq(pack_name),
                    site_packs::description.eq(description.clone()),
                    site_packs::requires_pro.eq(true),
                    site_packs::last_updated_at.eq(None::<String>),
                    site_packs::version.eq(None::<String>),
                    site_packs::is_ignored.eq(false),
                    site_packs::icon.eq(icon.clone()),
                ))
                .execute(conn)?;

            for source in sources {
                diesel::insert_into(site_pack_sources::table)
                    .values((
                        site_pack_sources::id.eq(source.id.to_string()),
                        site_pack_sources::site_pack_id.eq(pack_id),
                        site_pack_sources::source.eq(&source.source),
                        site_pack_sources::source_type.eq(source.source_type.to_string()),
                        site_pack_sources::format.eq(source.format.to_string()),
                    ))
                    .execute(conn)?;
            }
            Ok(())
        })
        .map_err(|error| DBError::InsertionError {
            table: "site_packs/site_pack_sources".to_string(),
            message: error.to_string(),
        })
    }

    pub fn update_site_pack_last_updated(
        &self,
        site_pack_id: &str,
        last_updated: &chrono::DateTime<chrono::Local>,
    ) -> Result<(), DBError> {
        use crate::schema_main_db::site_packs::dsl::*;
        let conn = &mut self.get_db_connection()?;

        diesel::update(site_packs.filter(id.eq(site_pack_id)))
            .set(last_updated_at.eq(last_updated.to_rfc3339()))
            .execute(conn)
            .map_err(|error| DBError::UpdateError {
                table: "site_packs".to_string(),
                message: error.to_string(),
            })?;

        Ok(())
    }

    pub fn delete_site_pack(&self, site_pack_id: &str) -> Result<(), DBError> {
        use crate::schema_main_db::site_packs::dsl::*;
        let conn = &mut self.get_db_connection()?;

        diesel::delete(site_packs.filter(id.eq(site_pack_id)))
            .execute(conn)
            .map_err(|error| DBError::DeletionError {
                table: "site_packs".to_string(),
                message: error.to_string(),
            })?;

        Ok(())
    }

    pub fn add_remote_site_pack(
        &self,
        id: &str,
        remote_id: &str,
        name: &str,
        description: Option<String>,
        requires_pro: bool,
        version: &str,
    ) -> Result<SitePackModel, DBError> {
        use crate::schema_main_db::site_packs::dsl;
        let conn = &mut self.get_db_connection()?;

        diesel::insert_into(site_packs::table)
            .values((
                dsl::id.eq(id),
                dsl::remote_id.eq(Some(remote_id.to_string())),
                dsl::name.eq(name),
                dsl::description.eq(description),
                dsl::requires_pro.eq(requires_pro),
                dsl::last_updated_at.eq(None::<String>),
                dsl::version.eq(Some(version.to_string())),
                dsl::is_ignored.eq(false),
                dsl::icon.eq(derive_icon_from_name(name)),
            ))
            .get_result::<SitePackModel>(conn)
            .map_err(|error| DBError::InsertionError {
                table: "site_packs".to_string(),
                message: error.to_string(),
            })
    }

    /// Like [`Self::add_remote_site_pack`] but preserves an absent version
    /// (backup restore of a pack that had no recorded version yet must not
    /// fabricate an empty-string version).
    pub fn add_remote_site_pack_with_optional_version(
        &self,
        id: &str,
        remote_id: &str,
        name: &str,
        description: Option<String>,
        requires_pro: bool,
        version: Option<String>,
    ) -> Result<SitePackModel, DBError> {
        use crate::schema_main_db::site_packs::dsl;
        let conn = &mut self.get_db_connection()?;

        diesel::insert_into(site_packs::table)
            .values((
                dsl::id.eq(id),
                dsl::remote_id.eq(Some(remote_id.to_string())),
                dsl::name.eq(name),
                dsl::description.eq(description),
                dsl::requires_pro.eq(requires_pro),
                dsl::last_updated_at.eq(None::<String>),
                dsl::version.eq(version),
                dsl::is_ignored.eq(false),
                dsl::icon.eq(derive_icon_from_name(name)),
            ))
            .get_result::<SitePackModel>(conn)
            .map_err(|error| DBError::InsertionError {
                table: "site_packs".to_string(),
                message: error.to_string(),
            })
    }

    /// Returns None only for a missing row; query failures propagate as Err.
    pub fn find_site_pack_by_remote_id(
        &self,
        remote_id_val: &str,
    ) -> Result<Option<SitePackModel>, DBError> {
        use crate::schema_main_db::site_packs::dsl::*;
        let conn = &mut self.get_db_connection()?;
        site_packs
            .filter(remote_id.eq(Some(remote_id_val.to_owned())))
            .select(SitePackModel::as_select())
            .first(conn)
            .optional()
            .map_err(|error| DBError::DataFetchError {
                table: "site_packs".to_string(),
                message: error.to_string(),
            })
    }

    pub fn get_site_pack_by_remote_id(
        &self,
        remote_id_val: &str,
    ) -> Result<SitePackModel, DBError> {
        use crate::schema_main_db::site_packs::dsl::*;
        let conn = &mut self.get_db_connection()?;

        site_packs
            .filter(remote_id.eq(Some(remote_id_val.to_owned())))
            .select(SitePackModel::as_select())
            .first(conn)
            .map_err(|error| DBError::DataFetchError {
                table: "site_packs".to_string(),
                message: error.to_string(),
            })
    }

    pub fn update_site_pack_version(
        &self,
        site_pack_id: &str,
        version_val: &str,
    ) -> Result<(), DBError> {
        use crate::schema_main_db::site_packs::dsl::*;
        let conn = &mut self.get_db_connection()?;

        diesel::update(site_packs.filter(id.eq(site_pack_id)))
            .set(version.eq(Some(version_val.to_owned())))
            .execute(conn)
            .map_err(|error| DBError::UpdateError {
                table: "site_packs".to_string(),
                message: error.to_string(),
            })?;

        Ok(())
    }

    pub fn set_site_pack_ignored(&self, site_pack_id: &str, ignored: bool) -> Result<(), DBError> {
        use crate::schema_main_db::site_packs::dsl::*;
        let conn = &mut self.get_db_connection()?;

        diesel::update(site_packs.filter(id.eq(site_pack_id)))
            .set(is_ignored.eq(ignored))
            .execute(conn)
            .map_err(|error| DBError::UpdateError {
                table: "site_packs".to_string(),
                message: error.to_string(),
            })?;

        Ok(())
    }

    pub fn get_ignored_remote_site_packs(&self) -> Result<Vec<SitePackModel>, DBError> {
        use crate::schema_main_db::site_packs::dsl::*;
        let conn = &mut self.get_db_connection()?;

        site_packs
            .filter(remote_id.is_not_null())
            .filter(is_ignored.eq(true))
            .select(SitePackModel::as_select())
            .load(conn)
            .map_err(|error| DBError::DataFetchError {
                table: "site_packs".to_string(),
                message: error.to_string(),
            })
    }
}

#[cfg(test)]
mod tests {}
