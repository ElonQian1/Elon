use std::collections::HashSet;

use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

use super::{new_id, now};
use crate::{
    esk_asset::{EskAllocationBatchInput, EskAllocationBatchReceipt, EskAllocationReceipt},
    store::Store,
};

const MAX_BATCH_ENTRIES: usize = 100;

impl Store {
    pub(crate) fn validate_esk_paper_allocation_batch(
        &self,
        input: &EskAllocationBatchInput,
    ) -> Result<()> {
        validate_batch_shape(input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        validate_batch_on(&tx, input)?;
        tx.commit()?;
        Ok(())
    }

    pub(crate) fn create_esk_paper_allocation_batch(
        &self,
        input: &EskAllocationBatchInput,
    ) -> Result<EskAllocationBatchReceipt> {
        validate_batch_shape(input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(existing) = validate_batch_on(&tx, input)? {
            tx.commit()?;
            return Ok(EskAllocationBatchReceipt {
                replayed: true,
                ..existing
            });
        }

        let created_at = now();
        tx.execute(
            "INSERT INTO esk_paper_allocation_batches (
               batch_id, request_digest, entry_count, total_base_units, actor, created_at
             ) VALUES (?1, ?2, ?3, ?4, 'platform_admin', ?5)",
            params![
                input.batch_id,
                input.request_digest,
                input.entries.len() as i64,
                input.total_base_units,
                created_at,
            ],
        )?;

        for (ordinal, entry) in input.entries.iter().enumerate() {
            let entry_id = new_id("eska");
            tx.execute(
                "INSERT INTO esk_asset_ledger_entries (
                   entry_id, user_id, amount_base_units, entry_kind, reference,
                   idempotency_key, actor, created_at
                 ) VALUES (?1, ?2, ?3, 'paper_allocation', ?4, ?5, 'platform_admin', ?6)",
                params![
                    entry_id,
                    entry.user_id,
                    entry.amount_base_units,
                    entry.reference,
                    entry.idempotency_key,
                    created_at,
                ],
            )?;
            tx.execute(
                "INSERT INTO esk_paper_allocation_batch_entries (
                   batch_id, ordinal, ledger_entry_id
                 ) VALUES (?1, ?2, ?3)",
                params![input.batch_id, ordinal as i64, entry_id],
            )?;
        }

        let receipt = batch_by_id_on(&tx, &input.batch_id)?
            .ok_or_else(|| anyhow!("ESK Paper 批次写入后不可见"))?;
        tx.commit()?;
        Ok(receipt)
    }
}

fn validate_batch_on(
    conn: &Connection,
    input: &EskAllocationBatchInput,
) -> Result<Option<EskAllocationBatchReceipt>> {
    if let Some(existing) = batch_by_id_on(conn, &input.batch_id)? {
        if existing.request_digest != input.request_digest
            || existing.total_base_units != input.total_base_units
            || existing.entries.len() != input.entries.len()
        {
            bail!("相同 ESK Paper 批次 ID 不能用于不同请求");
        }
        return Ok(Some(existing));
    }

    for entry in &input.entries {
        ensure_user_exists(conn, &entry.user_id)?;
        let occupied: bool = conn.query_row(
            "SELECT EXISTS(
               SELECT 1 FROM esk_asset_ledger_entries
                WHERE entry_kind = 'paper_allocation' AND idempotency_key = ?1
             )",
            params![entry.idempotency_key],
            |row| row.get(0),
        )?;
        if occupied {
            bail!("ESK Paper 批次条目幂等键已被使用");
        }
    }
    Ok(None)
}

fn batch_by_id_on(conn: &Connection, batch_id: &str) -> Result<Option<EskAllocationBatchReceipt>> {
    let header = conn
        .query_row(
            "SELECT request_digest, entry_count, total_base_units, created_at
               FROM esk_paper_allocation_batches
              WHERE batch_id = ?1",
            params![batch_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()?;
    let Some((request_digest, expected_count, total_base_units, created_at)) = header else {
        return Ok(None);
    };

    let mut statement = conn.prepare(
        "SELECT l.entry_id, l.user_id, l.amount_base_units, l.reference,
                l.idempotency_key, l.created_at
           FROM esk_paper_allocation_batch_entries b
           JOIN esk_asset_ledger_entries l ON l.entry_id = b.ledger_entry_id
          WHERE b.batch_id = ?1
          ORDER BY b.ordinal ASC",
    )?;
    let entries = statement
        .query_map(params![batch_id], |row| {
            Ok(EskAllocationReceipt {
                entry_id: row.get(0)?,
                user_id: row.get(1)?,
                amount_base_units: row.get(2)?,
                reference: row.get(3)?,
                idempotency_key: row.get(4)?,
                created_at: row.get(5)?,
                replayed: false,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let actual_total = entries.iter().try_fold(0_i64, |total, entry| {
        total
            .checked_add(entry.amount_base_units)
            .ok_or_else(|| anyhow!("ESK Paper 批次回执总额溢出"))
    })?;
    if entries.len() as i64 != expected_count || actual_total != total_base_units {
        bail!("ESK Paper 批次回执完整性校验失败");
    }
    Ok(Some(EskAllocationBatchReceipt {
        batch_id: batch_id.to_string(),
        request_digest,
        total_base_units,
        entries,
        created_at,
        replayed: false,
    }))
}

fn ensure_user_exists(conn: &Connection, user_id: &str) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM users WHERE id = ?1)",
        params![user_id],
        |row| row.get(0),
    )?;
    if !exists {
        bail!("ESK 登记用户不存在");
    }
    Ok(())
}

fn validate_batch_shape(input: &EskAllocationBatchInput) -> Result<()> {
    validate_key(&input.batch_id, "批次 ID", 160)?;
    if input.entries.is_empty() || input.entries.len() > MAX_BATCH_ENTRIES {
        bail!("ESK Paper 批次条目数量必须为 1..={MAX_BATCH_ENTRIES}");
    }
    if input.request_digest.len() != 64
        || input
            .request_digest
            .bytes()
            .any(|byte| !byte.is_ascii_hexdigit() || byte.is_ascii_uppercase())
    {
        bail!("ESK Paper 批次请求摘要无效");
    }
    let mut references = HashSet::with_capacity(input.entries.len());
    let mut idempotency_keys = HashSet::with_capacity(input.entries.len());
    let mut total = 0_i64;
    for entry in &input.entries {
        validate_key(&entry.user_id, "用户 ID", 160)?;
        validate_key(&entry.reference, "登记引用", 240)?;
        validate_key(&entry.idempotency_key, "幂等键", 160)?;
        if entry.amount_base_units <= 0 {
            bail!("ESK 登记金额必须大于 0");
        }
        if !references.insert(entry.reference.as_str()) {
            bail!("ESK Paper 批次包含重复登记引用");
        }
        if !idempotency_keys.insert(entry.idempotency_key.as_str()) {
            bail!("ESK Paper 批次包含重复幂等键");
        }
        total = total
            .checked_add(entry.amount_base_units)
            .ok_or_else(|| anyhow!("ESK Paper 批次总金额超出范围"))?;
    }
    if total != input.total_base_units {
        bail!("ESK Paper 批次总金额校验失败");
    }
    Ok(())
}

fn validate_key(value: &str, label: &str, max_chars: usize) -> Result<()> {
    let value = value.trim();
    let length = value.chars().count();
    if length == 0 || length > max_chars || value.chars().any(char::is_control) {
        bail!("{label} 无效");
    }
    Ok(())
}
