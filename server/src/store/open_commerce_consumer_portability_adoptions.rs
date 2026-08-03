use anyhow::{anyhow, bail, Result};
use rusqlite::{params, OptionalExtension, Row, TransactionBehavior};

use crate::{
    open_commerce_consumer_model::ConsumerPreferences,
    open_commerce_portability_adoption_model::{
        ConsumerPortabilityAdoption, CONSUMER_PORTABILITY_ADOPTION_SCHEMA,
    },
};

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn apply_consumer_portability_preferences(
        &self,
        import_id: &str,
        destination_project_id: &str,
        consumer_user_id: &str,
        expected_current_revision: Option<i64>,
        preferences: &ConsumerPreferences,
    ) -> Result<ConsumerPortabilityAdoption> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_owned_import(&tx, import_id, destination_project_id, consumer_user_id)?;
        if active_adoption_exists(&tx, import_id, destination_project_id, consumer_user_id)? {
            bail!("该数据包已有未回滚的偏好采用记录");
        }
        let current = current_preferences(&tx, destination_project_id, consumer_user_id)?;
        let actual_revision = current.as_ref().map(|value| value.1);
        if actual_revision != expected_current_revision {
            bail!("消费者偏好档案已变化，请刷新迁移预演后重试");
        }
        let timestamp = now();
        let preferences_json = serde_json::to_string(preferences)?;
        tx.execute(
            "INSERT INTO open_commerce_consumer_preference_profiles (
               consumer_project_id, consumer_user_id, preferences_json,
               revision, created_at, updated_at
             ) VALUES (?1, ?2, ?3, 1, ?4, ?4)
             ON CONFLICT(consumer_project_id, consumer_user_id) DO UPDATE SET
               preferences_json=excluded.preferences_json,
               revision=open_commerce_consumer_preference_profiles.revision + 1,
               updated_at=excluded.updated_at",
            params![
                destination_project_id,
                consumer_user_id,
                preferences_json,
                timestamp,
            ],
        )?;
        let resulting_revision: i64 = tx.query_row(
            "SELECT revision FROM open_commerce_consumer_preference_profiles
              WHERE consumer_project_id=?1 AND consumer_user_id=?2",
            params![destination_project_id, consumer_user_id],
            |row| row.get(0),
        )?;
        let id = new_id("portability-adoption");
        tx.execute(
            "INSERT INTO open_commerce_consumer_portability_adoptions (
               id, import_id, destination_project_id, consumer_user_id, adoption_kind,
               before_preferences_json, before_revision, applied_preferences_json,
               resulting_revision, status, applied_at
             ) VALUES (?1, ?2, ?3, ?4, 'preferences', ?5, ?6, ?7, ?8, 'applied', ?9)",
            params![
                id,
                import_id,
                destination_project_id,
                consumer_user_id,
                current
                    .as_ref()
                    .map(|value| serde_json::to_string(&value.0))
                    .transpose()?,
                actual_revision,
                serde_json::to_string(preferences)?,
                resulting_revision,
                timestamp,
            ],
        )?;
        let adoption = tx.query_row(
            &format!("{ADOPTION_SELECT} WHERE id=?1"),
            params![id],
            adoption_from_row,
        )?;
        tx.commit()?;
        Ok(adoption)
    }

    pub(crate) fn rollback_consumer_portability_adoption(
        &self,
        adoption_id: &str,
        destination_project_id: &str,
        consumer_user_id: &str,
        expected_current_revision: i64,
    ) -> Result<ConsumerPortabilityAdoption> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let adoption = tx
            .query_row(
                &format!(
                    "{ADOPTION_SELECT}
                      WHERE id=?1 AND destination_project_id=?2 AND consumer_user_id=?3"
                ),
                params![adoption_id, destination_project_id, consumer_user_id],
                adoption_from_row,
            )
            .optional()?
            .ok_or_else(|| anyhow!("消费者数据包采用记录不存在"))?;
        if adoption.status != "applied" {
            bail!("消费者数据包采用记录已经回滚");
        }
        let current_revision: i64 = tx
            .query_row(
                "SELECT revision FROM open_commerce_consumer_preference_profiles
                  WHERE consumer_project_id=?1 AND consumer_user_id=?2",
                params![destination_project_id, consumer_user_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| anyhow!("当前偏好档案不存在，不能安全回滚"))?;
        if current_revision != expected_current_revision
            || current_revision != adoption.resulting_revision
        {
            bail!("偏好档案在采用后已变化，拒绝覆盖后续修改");
        }
        let timestamp = now();
        let rollback_revision = if let Some(before) = &adoption.before_preferences {
            tx.execute(
                "UPDATE open_commerce_consumer_preference_profiles
                    SET preferences_json=?3, revision=revision + 1, updated_at=?4
                  WHERE consumer_project_id=?1 AND consumer_user_id=?2",
                params![
                    destination_project_id,
                    consumer_user_id,
                    serde_json::to_string(before)?,
                    timestamp,
                ],
            )?;
            Some(current_revision + 1)
        } else {
            tx.execute(
                "DELETE FROM open_commerce_consumer_preference_profiles
                  WHERE consumer_project_id=?1 AND consumer_user_id=?2",
                params![destination_project_id, consumer_user_id],
            )?;
            None
        };
        tx.execute(
            "UPDATE open_commerce_consumer_portability_adoptions
                SET status='rolled_back', rolled_back_at=?2, rollback_revision=?3
              WHERE id=?1 AND status='applied'",
            params![adoption_id, timestamp, rollback_revision],
        )?;
        let updated = tx.query_row(
            &format!("{ADOPTION_SELECT} WHERE id=?1"),
            params![adoption_id],
            adoption_from_row,
        )?;
        tx.commit()?;
        Ok(updated)
    }

    pub(crate) fn list_consumer_portability_adoptions(
        &self,
        destination_project_id: &str,
        consumer_user_id: &str,
        limit: usize,
    ) -> Result<Vec<ConsumerPortabilityAdoption>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{ADOPTION_SELECT}
              WHERE destination_project_id=?1 AND consumer_user_id=?2
              ORDER BY applied_at DESC, rowid DESC LIMIT ?3"
        ))?;
        let rows = stmt.query_map(
            params![
                destination_project_id,
                consumer_user_id,
                limit.clamp(1, 100) as i64,
            ],
            adoption_from_row,
        )?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

fn ensure_owned_import(
    tx: &rusqlite::Transaction<'_>,
    import_id: &str,
    destination_project_id: &str,
    consumer_user_id: &str,
) -> Result<()> {
    let exists: bool = tx.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM open_commerce_consumer_portability_imports
            WHERE id=?1 AND destination_project_id=?2 AND consumer_user_id=?3
         )",
        params![import_id, destination_project_id, consumer_user_id],
        |row| row.get(0),
    )?;
    if !exists {
        bail!("消费者外部数据包导入记录不存在");
    }
    Ok(())
}

fn active_adoption_exists(
    tx: &rusqlite::Transaction<'_>,
    import_id: &str,
    destination_project_id: &str,
    consumer_user_id: &str,
) -> Result<bool> {
    Ok(tx.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM open_commerce_consumer_portability_adoptions
            WHERE import_id=?1 AND destination_project_id=?2 AND consumer_user_id=?3
              AND adoption_kind='preferences' AND status='applied'
         )",
        params![import_id, destination_project_id, consumer_user_id],
        |row| row.get(0),
    )?)
}

fn current_preferences(
    tx: &rusqlite::Transaction<'_>,
    destination_project_id: &str,
    consumer_user_id: &str,
) -> Result<Option<(ConsumerPreferences, i64)>> {
    tx.query_row(
        "SELECT preferences_json, revision
           FROM open_commerce_consumer_preference_profiles
          WHERE consumer_project_id=?1 AND consumer_user_id=?2",
        params![destination_project_id, consumer_user_id],
        |row| {
            let json: String = row.get(0)?;
            let preferences = serde_json::from_str(&json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    json.len(),
                    rusqlite::types::Type::Text,
                    error.into(),
                )
            })?;
            Ok((preferences, row.get(1)?))
        },
    )
    .optional()
    .map_err(Into::into)
}

fn adoption_from_row(row: &Row<'_>) -> rusqlite::Result<ConsumerPortabilityAdoption> {
    let before_json: Option<String> = row.get(3)?;
    let applied_json: String = row.get(5)?;
    Ok(ConsumerPortabilityAdoption {
        schema: CONSUMER_PORTABILITY_ADOPTION_SCHEMA.to_string(),
        id: row.get(0)?,
        import_id: row.get(1)?,
        kind: row.get(2)?,
        before_preferences: before_json
            .map(|value| serde_json::from_str(&value))
            .transpose()
            .map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    error.into(),
                )
            })?,
        before_revision: row.get(4)?,
        applied_preferences: serde_json::from_str(&applied_json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                applied_json.len(),
                rusqlite::types::Type::Text,
                error.into(),
            )
        })?,
        resulting_revision: row.get(6)?,
        status: row.get(7)?,
        applied_at: row.get(8)?,
        rolled_back_at: row.get(9)?,
        rollback_revision: row.get(10)?,
    })
}

const ADOPTION_SELECT: &str = "SELECT id, import_id, adoption_kind,
       before_preferences_json, before_revision, applied_preferences_json,
       resulting_revision, status, applied_at, rolled_back_at, rollback_revision
  FROM open_commerce_consumer_portability_adoptions";
