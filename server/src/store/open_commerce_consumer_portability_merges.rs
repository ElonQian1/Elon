use std::collections::BTreeSet;

use anyhow::{anyhow, bail, Result};
use rusqlite::{params, OptionalExtension, Row, TransactionBehavior};

use crate::{
    open_commerce_consumer_model::ConsumerPreferences,
    open_commerce_portability_merge_model::{
        ConsumerPortabilityFieldSource, ConsumerPortabilityMergeAdoption,
        CONSUMER_PORTABILITY_MERGE_ADOPTION_SCHEMA,
    },
};

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn apply_consumer_portability_preference_merge(
        &self,
        source_import_ids: &[String],
        field_sources: &[ConsumerPortabilityFieldSource],
        destination_project_id: &str,
        consumer_user_id: &str,
        expected_current_revision: Option<i64>,
        preferences: &ConsumerPreferences,
    ) -> Result<ConsumerPortabilityMergeAdoption> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_owned_imports(
            &tx,
            source_import_ids,
            destination_project_id,
            consumer_user_id,
        )?;
        ensure_field_sources_use_selected_imports(source_import_ids, field_sources)?;
        let current = current_preferences(&tx, destination_project_id, consumer_user_id)?;
        let actual_revision = current.as_ref().map(|value| value.1);
        if actual_revision != expected_current_revision {
            bail!("消费者偏好档案已变化，请刷新多来源合并预演后重试");
        }
        let timestamp = now();
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
                serde_json::to_string(preferences)?,
                timestamp,
            ],
        )?;
        let resulting_revision: i64 = tx.query_row(
            "SELECT revision FROM open_commerce_consumer_preference_profiles
              WHERE consumer_project_id=?1 AND consumer_user_id=?2",
            params![destination_project_id, consumer_user_id],
            |row| row.get(0),
        )?;
        let id = new_id("portability-merge");
        tx.execute(
            "INSERT INTO open_commerce_consumer_portability_merge_adoptions (
               id, destination_project_id, consumer_user_id, source_import_ids_json,
               field_sources_json, before_preferences_json, before_revision,
               applied_preferences_json, resulting_revision, status, applied_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'applied', ?10)",
            params![
                id,
                destination_project_id,
                consumer_user_id,
                serde_json::to_string(source_import_ids)?,
                serde_json::to_string(field_sources)?,
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
            &format!("{MERGE_ADOPTION_SELECT} WHERE id=?1"),
            params![id],
            merge_adoption_from_row,
        )?;
        tx.commit()?;
        Ok(adoption)
    }

    pub(crate) fn rollback_consumer_portability_preference_merge(
        &self,
        adoption_id: &str,
        destination_project_id: &str,
        consumer_user_id: &str,
        expected_current_revision: i64,
    ) -> Result<ConsumerPortabilityMergeAdoption> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let adoption = tx
            .query_row(
                &format!(
                    "{MERGE_ADOPTION_SELECT}
                      WHERE id=?1 AND destination_project_id=?2 AND consumer_user_id=?3"
                ),
                params![adoption_id, destination_project_id, consumer_user_id],
                merge_adoption_from_row,
            )
            .optional()?
            .ok_or_else(|| anyhow!("消费者多来源偏好合并记录不存在"))?;
        if adoption.status != "applied" {
            bail!("消费者多来源偏好合并记录已经回滚");
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
            bail!("偏好档案在合并后已变化，拒绝覆盖后续修改");
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
            "UPDATE open_commerce_consumer_portability_merge_adoptions
                SET status='rolled_back', rolled_back_at=?2, rollback_revision=?3
              WHERE id=?1 AND status='applied'",
            params![adoption_id, timestamp, rollback_revision],
        )?;
        let updated = tx.query_row(
            &format!("{MERGE_ADOPTION_SELECT} WHERE id=?1"),
            params![adoption_id],
            merge_adoption_from_row,
        )?;
        tx.commit()?;
        Ok(updated)
    }

    pub(crate) fn list_consumer_portability_preference_merges(
        &self,
        destination_project_id: &str,
        consumer_user_id: &str,
        limit: usize,
    ) -> Result<Vec<ConsumerPortabilityMergeAdoption>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{MERGE_ADOPTION_SELECT}
              WHERE destination_project_id=?1 AND consumer_user_id=?2
              ORDER BY applied_at DESC, rowid DESC LIMIT ?3"
        ))?;
        let rows = stmt.query_map(
            params![
                destination_project_id,
                consumer_user_id,
                limit.clamp(1, 100) as i64,
            ],
            merge_adoption_from_row,
        )?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

fn ensure_owned_imports(
    tx: &rusqlite::Transaction<'_>,
    import_ids: &[String],
    destination_project_id: &str,
    consumer_user_id: &str,
) -> Result<()> {
    for import_id in import_ids {
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
    }
    Ok(())
}

fn ensure_field_sources_use_selected_imports(
    import_ids: &[String],
    field_sources: &[ConsumerPortabilityFieldSource],
) -> Result<()> {
    let selected = import_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if field_sources
        .iter()
        .any(|source| !selected.contains(source.import_id.as_str()))
    {
        bail!("偏好字段来源不属于本次选择的数据包");
    }
    Ok(())
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
            let preferences = parse_json(&json, 0)?;
            Ok((preferences, row.get(1)?))
        },
    )
    .optional()
    .map_err(Into::into)
}

fn merge_adoption_from_row(row: &Row<'_>) -> rusqlite::Result<ConsumerPortabilityMergeAdoption> {
    let import_ids_json: String = row.get(1)?;
    let field_sources_json: String = row.get(2)?;
    let before_json: Option<String> = row.get(3)?;
    let applied_json: String = row.get(5)?;
    Ok(ConsumerPortabilityMergeAdoption {
        schema: CONSUMER_PORTABILITY_MERGE_ADOPTION_SCHEMA.to_string(),
        id: row.get(0)?,
        source_import_ids: parse_json(&import_ids_json, 1)?,
        field_sources: parse_json(&field_sources_json, 2)?,
        before_preferences: before_json
            .as_deref()
            .map(|value| parse_json(value, 3))
            .transpose()?,
        before_revision: row.get(4)?,
        applied_preferences: parse_json(&applied_json, 5)?,
        resulting_revision: row.get(6)?,
        status: row.get(7)?,
        applied_at: row.get(8)?,
        rolled_back_at: row.get(9)?,
        rollback_revision: row.get(10)?,
    })
}

fn parse_json<T: serde::de::DeserializeOwned>(value: &str, index: usize) -> rusqlite::Result<T> {
    serde_json::from_str(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(index, rusqlite::types::Type::Text, error.into())
    })
}

const MERGE_ADOPTION_SELECT: &str = "SELECT id, source_import_ids_json,
       field_sources_json, before_preferences_json, before_revision,
       applied_preferences_json, resulting_revision, status, applied_at,
       rolled_back_at, rollback_revision
  FROM open_commerce_consumer_portability_merge_adoptions";
