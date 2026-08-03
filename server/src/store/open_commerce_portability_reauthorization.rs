use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, Row, TransactionBehavior};

use crate::open_commerce_portability_reauthorization_model::{
    PortabilityRelationshipMapping, PORTABILITY_RELATIONSHIP_MAPPING_SCHEMA,
};

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn save_portability_relationship_mapping(
        &self,
        destination_project_id: &str,
        consumer_user_id: &str,
        import_id: &str,
        source_relationship_id: &str,
        source_merchant_id: &str,
        target_merchant_id: &str,
        target_merchant_project_id: &str,
    ) -> Result<(PortabilityRelationshipMapping, bool)> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing = tx
            .query_row(
                &format!(
                    "{MAPPING_SELECT}
                      WHERE destination_project_id=?1 AND consumer_user_id=?2
                        AND import_id=?3 AND source_relationship_id=?4 AND status='active'"
                ),
                params![
                    destination_project_id,
                    consumer_user_id,
                    import_id,
                    source_relationship_id,
                ],
                mapping_from_row,
            )
            .optional()?;
        if let Some(existing) = existing {
            if existing.target_merchant_id != target_merchant_id {
                anyhow::bail!("该来源关系已有有效目标映射，请先撤销原映射");
            }
            tx.commit()?;
            return Ok((existing, false));
        }
        let id = new_id("relationship-mapping");
        let created_at = now();
        tx.execute(
            "INSERT INTO open_commerce_portability_relationship_mappings (
               id, destination_project_id, consumer_user_id, import_id,
               source_relationship_id, source_merchant_id, target_merchant_id,
               target_merchant_project_id, status, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'active', ?9)",
            params![
                id,
                destination_project_id,
                consumer_user_id,
                import_id,
                source_relationship_id,
                source_merchant_id,
                target_merchant_id,
                target_merchant_project_id,
                created_at,
            ],
        )?;
        let mapping = tx.query_row(
            &format!("{MAPPING_SELECT} WHERE id=?1"),
            params![id],
            mapping_from_row,
        )?;
        tx.commit()?;
        Ok((mapping, true))
    }

    pub(crate) fn list_portability_relationship_mappings(
        &self,
        destination_project_id: &str,
        consumer_user_id: &str,
        limit: usize,
    ) -> Result<Vec<PortabilityRelationshipMapping>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{MAPPING_SELECT}
              WHERE destination_project_id=?1 AND consumer_user_id=?2
              ORDER BY created_at DESC, rowid DESC LIMIT ?3"
        ))?;
        let rows = stmt.query_map(
            params![
                destination_project_id,
                consumer_user_id,
                limit.clamp(1, 200) as i64,
            ],
            mapping_from_row,
        )?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub(crate) fn owned_portability_relationship_mapping(
        &self,
        destination_project_id: &str,
        consumer_user_id: &str,
        mapping_id: &str,
    ) -> Result<Option<PortabilityRelationshipMapping>> {
        self.conn()?
            .query_row(
                &format!(
                    "{MAPPING_SELECT}
                      WHERE id=?1 AND destination_project_id=?2 AND consumer_user_id=?3"
                ),
                params![mapping_id, destination_project_id, consumer_user_id],
                mapping_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn revoke_portability_relationship_mapping(
        &self,
        destination_project_id: &str,
        consumer_user_id: &str,
        mapping_id: &str,
    ) -> Result<PortabilityRelationshipMapping> {
        let timestamp = now();
        self.conn()?.execute(
            "UPDATE open_commerce_portability_relationship_mappings
                SET status='revoked', revoked_at=COALESCE(revoked_at, ?4)
              WHERE id=?1 AND destination_project_id=?2 AND consumer_user_id=?3",
            params![
                mapping_id,
                destination_project_id,
                consumer_user_id,
                timestamp
            ],
        )?;
        self.owned_portability_relationship_mapping(
            destination_project_id,
            consumer_user_id,
            mapping_id,
        )?
        .ok_or_else(|| anyhow!("消费者关系迁移映射不存在"))
    }
}

fn mapping_from_row(row: &Row<'_>) -> rusqlite::Result<PortabilityRelationshipMapping> {
    Ok(PortabilityRelationshipMapping {
        schema: PORTABILITY_RELATIONSHIP_MAPPING_SCHEMA.to_string(),
        id: row.get(0)?,
        import_id: row.get(1)?,
        source_relationship_id: row.get(2)?,
        source_merchant_id: row.get(3)?,
        target_merchant_id: row.get(4)?,
        target_merchant_project_id: row.get(5)?,
        status: row.get(6)?,
        created_at: row.get(7)?,
        revoked_at: row.get(8)?,
    })
}

const MAPPING_SELECT: &str = "SELECT id, import_id, source_relationship_id,
       source_merchant_id, target_merchant_id, target_merchant_project_id,
       status, created_at, revoked_at
  FROM open_commerce_portability_relationship_mappings";
