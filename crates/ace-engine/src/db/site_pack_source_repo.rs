use crate::db::{DBError, EngineDB, models::SitePackSourceModel};
use diesel::prelude::*;

impl EngineDB {
    pub fn get_site_pack_sources(
        &self,
        pack_id: &str,
    ) -> Result<Vec<SitePackSourceModel>, DBError> {
        use crate::schema_main_db::site_pack_sources::dsl::*;
        let conn = &mut self.get_db_connection()?;

        site_pack_sources
            .filter(site_pack_id.eq(pack_id))
            .select(SitePackSourceModel::as_select())
            .load::<SitePackSourceModel>(conn)
            .map_err(|error| DBError::DataFetchError {
                table: "site_pack_sources".to_string(),
                message: error.to_string(),
            })
    }

    pub fn get_site_pack_source_by_id(
        &self,
        source_id: &uuid::Uuid,
    ) -> Result<SitePackSourceModel, DBError> {
        use crate::schema_main_db::site_pack_sources::dsl::*;
        let conn = &mut self.get_db_connection()?;

        site_pack_sources
            .filter(id.eq(source_id.to_string()))
            .select(SitePackSourceModel::as_select())
            .first(conn)
            .map_err(|error| DBError::DataFetchError {
                table: "site_pack_sources".to_string(),
                message: error.to_string(),
            })
    }

    pub fn add_site_pack_source(
        &self,
        id: &uuid::Uuid,
        site_pack_id: &str,
        source_val: &str,
        type_val: &str,
        format_val: &str,
    ) -> Result<SitePackSourceModel, DBError> {
        use crate::schema_main_db::site_pack_sources::dsl;
        let conn = &mut self.get_db_connection()?;

        diesel::insert_into(dsl::site_pack_sources)
            .values((
                dsl::id.eq(id.to_string()),
                dsl::site_pack_id.eq(site_pack_id),
                dsl::source.eq(source_val),
                dsl::source_type.eq(type_val),
                dsl::format.eq(format_val),
            ))
            .get_result::<SitePackSourceModel>(conn)
            .map_err(|error| DBError::InsertionError {
                table: "site_pack_sources".to_string(),
                message: error.to_string(),
            })
    }

    pub fn delete_site_pack_source(&self, source_id: &uuid::Uuid) -> Result<(), DBError> {
        use crate::schema_main_db::site_pack_sources::dsl::*;
        let conn = &mut self.get_db_connection()?;

        diesel::delete(site_pack_sources.filter(id.eq(source_id.to_string())))
            .execute(conn)
            .map_err(|error| DBError::DeletionError {
                table: "site_pack_sources".to_string(),
                message: error.to_string(),
            })?;

        Ok(())
    }
}
