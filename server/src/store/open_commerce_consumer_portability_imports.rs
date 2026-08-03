use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, Row, TransactionBehavior};

use crate::{
    open_commerce_portability_import_model::{
        ConsumerPortabilityImport, CONSUMER_PORTABILITY_IMPORT_MERGE_STATUS,
        CONSUMER_PORTABILITY_IMPORT_SCHEMA, CONSUMER_PORTABILITY_IMPORT_TRUST_STATUS,
    },
    open_commerce_portability_model::ConsumerPortabilityExport,
};

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn save_consumer_portability_import(
        &self,
        destination_project_id: &str,
        consumer_user_id: &str,
        source_operator: &str,
        package: &ConsumerPortabilityExport,
        package_json: &str,
        envelope_sha256: &str,
    ) -> Result<(ConsumerPortabilityImport, bool)> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing = tx
            .query_row(
                &format!(
                    "{PORTABILITY_IMPORT_SELECT}
                      WHERE destination_project_id=?1 AND consumer_user_id=?2
                        AND envelope_sha256=?3"
                ),
                params![
                    destination_project_id.trim(),
                    consumer_user_id.trim(),
                    envelope_sha256,
                ],
                portability_import_from_row,
            )
            .optional()?;
        if let Some(existing) = existing {
            tx.commit()?;
            return Ok((existing, false));
        }

        let id = new_id("portability-import");
        let imported_at = now();
        tx.execute(
            "INSERT INTO open_commerce_consumer_portability_imports (
               id, destination_project_id, consumer_user_id, source_operator,
               source_project_id, source_package_id, source_package_schema,
               envelope_sha256, payload_sha256, package_json, imported_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                id,
                destination_project_id.trim(),
                consumer_user_id.trim(),
                source_operator,
                package.source_project_id,
                package.id,
                package.schema,
                envelope_sha256,
                package.payload_sha256,
                package_json,
                imported_at,
            ],
        )?;
        tx.commit()?;
        drop(conn);
        let saved = self
            .consumer_portability_import(destination_project_id, consumer_user_id, &id)?
            .ok_or_else(|| anyhow!("消费者外部数据包导入记录不存在"))?;
        Ok((saved, true))
    }

    pub(crate) fn list_consumer_portability_imports(
        &self,
        destination_project_id: &str,
        consumer_user_id: &str,
        limit: usize,
    ) -> Result<Vec<ConsumerPortabilityImport>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{PORTABILITY_IMPORT_SELECT}
              WHERE destination_project_id=?1 AND consumer_user_id=?2
              ORDER BY imported_at DESC, rowid DESC LIMIT ?3"
        ))?;
        let rows = stmt.query_map(
            params![
                destination_project_id.trim(),
                consumer_user_id.trim(),
                limit.clamp(1, 100) as i64,
            ],
            portability_import_from_row,
        )?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub(crate) fn consumer_portability_import(
        &self,
        destination_project_id: &str,
        consumer_user_id: &str,
        import_id: &str,
    ) -> Result<Option<ConsumerPortabilityImport>> {
        self.conn()?
            .query_row(
                &format!(
                    "{PORTABILITY_IMPORT_SELECT}
                      WHERE id=?1 AND destination_project_id=?2 AND consumer_user_id=?3"
                ),
                params![
                    import_id.trim(),
                    destination_project_id.trim(),
                    consumer_user_id.trim(),
                ],
                portability_import_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn delete_consumer_portability_import(
        &self,
        destination_project_id: &str,
        consumer_user_id: &str,
        import_id: &str,
    ) -> Result<Option<ConsumerPortabilityImport>> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing = tx
            .query_row(
                &format!(
                    "{PORTABILITY_IMPORT_SELECT}
                      WHERE id=?1 AND destination_project_id=?2 AND consumer_user_id=?3"
                ),
                params![
                    import_id.trim(),
                    destination_project_id.trim(),
                    consumer_user_id.trim(),
                ],
                portability_import_from_row,
            )
            .optional()?;
        if existing.is_some() {
            tx.execute(
                "DELETE FROM open_commerce_consumer_portability_imports
                  WHERE id=?1 AND destination_project_id=?2 AND consumer_user_id=?3",
                params![
                    import_id.trim(),
                    destination_project_id.trim(),
                    consumer_user_id.trim(),
                ],
            )?;
        }
        tx.commit()?;
        Ok(existing)
    }
}

fn portability_import_from_row(row: &Row<'_>) -> rusqlite::Result<ConsumerPortabilityImport> {
    let package_json: String = row.get(8)?;
    let package: ConsumerPortabilityExport =
        serde_json::from_str(&package_json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                package_json.len(),
                rusqlite::types::Type::Text,
                error.into(),
            )
        })?;
    let source_project_id: String = row.get(3)?;
    let source_package_id: String = row.get(4)?;
    let source_package_schema: String = row.get(5)?;
    let payload_sha256: String = row.get(7)?;
    if package.source_project_id != source_project_id
        || package.id != source_package_id
        || package.schema != source_package_schema
        || package.payload_sha256 != payload_sha256
    {
        return Err(rusqlite::Error::FromSqlConversionFailure(
            package_json.len(),
            rusqlite::types::Type::Text,
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "导入记录元数据与数据包不一致",
            )
            .into(),
        ));
    }
    Ok(ConsumerPortabilityImport {
        schema: CONSUMER_PORTABILITY_IMPORT_SCHEMA.to_string(),
        id: row.get(0)?,
        destination_project_id: row.get(1)?,
        source_operator: row.get(2)?,
        source_project_id,
        source_package_id,
        source_package_schema,
        envelope_sha256: row.get(6)?,
        payload_sha256,
        package_json,
        package,
        trust_status: CONSUMER_PORTABILITY_IMPORT_TRUST_STATUS.to_string(),
        merge_status: CONSUMER_PORTABILITY_IMPORT_MERGE_STATUS.to_string(),
        imported_at: row.get(9)?,
    })
}

const PORTABILITY_IMPORT_SELECT: &str = "SELECT id, destination_project_id,
       source_operator, source_project_id, source_package_id, source_package_schema,
       envelope_sha256, payload_sha256, package_json, imported_at
  FROM open_commerce_consumer_portability_imports";
