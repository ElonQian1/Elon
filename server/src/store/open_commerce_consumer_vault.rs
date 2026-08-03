use anyhow::{anyhow, bail, Result};
use rusqlite::{params, OptionalExtension, Row, TransactionBehavior};

use crate::open_commerce_consumer_vault_model::{
    ConsumerDataVaultEnvelope, ConsumerDataVaultItem, ConsumerDataVaultItemSummary,
    CONSUMER_DATA_VAULT_ITEM_SCHEMA,
};

use super::{now, Store};

impl Store {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create_open_commerce_consumer_vault_item(
        &self,
        id: &str,
        consumer_project_id: &str,
        consumer_user_id: &str,
        label: &str,
        item_kind: &str,
        envelope: &ConsumerDataVaultEnvelope,
        ciphertext_sha256: &str,
        ciphertext_bytes: i64,
    ) -> Result<ConsumerDataVaultItem> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM open_commerce_consumer_data_vault_items
              WHERE consumer_project_id=?1 AND consumer_user_id=?2",
            params![consumer_project_id, consumer_user_id],
            |row| row.get(0),
        )?;
        if count >= 100 {
            bail!("每个消费者项目最多保存 100 个加密保险箱条目");
        }
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM open_commerce_consumer_data_vault_items WHERE id=?1)",
            params![id],
            |row| row.get(0),
        )?;
        if exists {
            bail!("消费者数据保险箱条目 ID 已存在");
        }
        let timestamp = now();
        tx.execute(
            "INSERT INTO open_commerce_consumer_data_vault_items (
               id, consumer_project_id, consumer_user_id, label, item_kind,
               envelope_json, ciphertext_sha256, ciphertext_bytes,
               revision, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, ?9)",
            params![
                id,
                consumer_project_id,
                consumer_user_id,
                label,
                item_kind,
                serde_json::to_string(envelope)?,
                ciphertext_sha256,
                ciphertext_bytes,
                timestamp,
            ],
        )?;
        let item = tx.query_row(
            &format!("{VAULT_ITEM_SELECT} WHERE id=?1"),
            params![id],
            vault_item_from_row,
        )?;
        tx.commit()?;
        Ok(item)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn update_open_commerce_consumer_vault_item(
        &self,
        id: &str,
        consumer_project_id: &str,
        consumer_user_id: &str,
        expected_revision: i64,
        label: &str,
        item_kind: &str,
        envelope: &ConsumerDataVaultEnvelope,
        ciphertext_sha256: &str,
        ciphertext_bytes: i64,
    ) -> Result<ConsumerDataVaultItem> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let timestamp = now();
        let changed = tx.execute(
            "UPDATE open_commerce_consumer_data_vault_items
                SET label=?5, item_kind=?6, envelope_json=?7,
                    ciphertext_sha256=?8, ciphertext_bytes=?9,
                    revision=revision + 1, updated_at=?10
              WHERE id=?1 AND consumer_project_id=?2 AND consumer_user_id=?3
                AND revision=?4",
            params![
                id,
                consumer_project_id,
                consumer_user_id,
                expected_revision,
                label,
                item_kind,
                serde_json::to_string(envelope)?,
                ciphertext_sha256,
                ciphertext_bytes,
                timestamp,
            ],
        )?;
        if changed == 0 {
            let exists: bool = tx.query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM open_commerce_consumer_data_vault_items
                    WHERE id=?1 AND consumer_project_id=?2 AND consumer_user_id=?3
                 )",
                params![id, consumer_project_id, consumer_user_id],
                |row| row.get(0),
            )?;
            if exists {
                bail!("保险箱条目已变化，请刷新后重试");
            }
            bail!("消费者数据保险箱条目不存在");
        }
        let item = tx.query_row(
            &format!("{VAULT_ITEM_SELECT} WHERE id=?1"),
            params![id],
            vault_item_from_row,
        )?;
        tx.commit()?;
        Ok(item)
    }

    pub(crate) fn open_commerce_consumer_vault_item(
        &self,
        consumer_project_id: &str,
        consumer_user_id: &str,
        id: &str,
    ) -> Result<Option<ConsumerDataVaultItem>> {
        self.conn()?
            .query_row(
                &format!(
                    "{VAULT_ITEM_SELECT}
                      WHERE id=?1 AND consumer_project_id=?2 AND consumer_user_id=?3"
                ),
                params![id, consumer_project_id, consumer_user_id],
                vault_item_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn list_open_commerce_consumer_vault_items(
        &self,
        consumer_project_id: &str,
        consumer_user_id: &str,
        limit: usize,
    ) -> Result<Vec<ConsumerDataVaultItemSummary>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{VAULT_ITEM_SELECT}
              WHERE consumer_project_id=?1 AND consumer_user_id=?2
              ORDER BY updated_at DESC, rowid DESC LIMIT ?3"
        ))?;
        let rows = stmt.query_map(
            params![
                consumer_project_id,
                consumer_user_id,
                limit.clamp(1, 100) as i64,
            ],
            vault_item_from_row,
        )?;
        Ok(rows
            .collect::<rusqlite::Result<Vec<_>>>()?
            .into_iter()
            .map(|item| item.summary())
            .collect())
    }

    pub(crate) fn delete_open_commerce_consumer_vault_item(
        &self,
        consumer_project_id: &str,
        consumer_user_id: &str,
        id: &str,
        expected_revision: i64,
    ) -> Result<ConsumerDataVaultItemSummary> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let item = tx
            .query_row(
                &format!(
                    "{VAULT_ITEM_SELECT}
                      WHERE id=?1 AND consumer_project_id=?2 AND consumer_user_id=?3"
                ),
                params![id, consumer_project_id, consumer_user_id],
                vault_item_from_row,
            )
            .optional()?
            .ok_or_else(|| anyhow!("消费者数据保险箱条目不存在"))?;
        if item.revision != expected_revision {
            bail!("保险箱条目已变化，请刷新后重试");
        }
        tx.execute(
            "DELETE FROM open_commerce_consumer_data_vault_items
              WHERE id=?1 AND consumer_project_id=?2 AND consumer_user_id=?3 AND revision=?4",
            params![id, consumer_project_id, consumer_user_id, expected_revision],
        )?;
        tx.commit()?;
        Ok(item.summary())
    }
}

fn vault_item_from_row(row: &Row<'_>) -> rusqlite::Result<ConsumerDataVaultItem> {
    let envelope_json: String = row.get(3)?;
    let envelope = serde_json::from_str(&envelope_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            envelope_json.len(),
            rusqlite::types::Type::Text,
            error.into(),
        )
    })?;
    Ok(ConsumerDataVaultItem {
        schema: CONSUMER_DATA_VAULT_ITEM_SCHEMA.to_string(),
        id: row.get(0)?,
        label: row.get(1)?,
        item_kind: row.get(2)?,
        envelope,
        ciphertext_sha256: row.get(4)?,
        ciphertext_bytes: row.get(5)?,
        revision: row.get(6)?,
        server_can_decrypt: false,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

const VAULT_ITEM_SELECT: &str = "SELECT id, label, item_kind, envelope_json,
       ciphertext_sha256, ciphertext_bytes, revision, created_at, updated_at
  FROM open_commerce_consumer_data_vault_items";
