use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

use super::{
    esk_assets::{account_ledger_on, ensure_user_exists, validate_key},
    new_id, now,
};
use crate::{
    esk_asset::{
        EskQuantAllocationInput, EskQuantAllocationRecord, ESK_QUANT_RISK_DISCLOSURE_REVISION,
    },
    store::Store,
};

impl Store {
    pub(crate) fn create_esk_quant_allocation_request(
        &self,
        input: &EskQuantAllocationInput,
    ) -> Result<EskQuantAllocationRecord> {
        validate_input(input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_user_exists(&tx, &input.user_id)?;
        if let Some(existing) = by_idempotency(&tx, &input.user_id, &input.idempotency_key)? {
            if existing.amount_base_units != input.amount_base_units
                || existing.risk_disclosure_revision != input.risk_disclosure_revision
            {
                bail!("相同量化分配申请幂等键不能用于不同请求");
            }
            tx.commit()?;
            return Ok(EskQuantAllocationRecord {
                replayed: true,
                ..existing
            });
        }
        let ledger = account_ledger_on(&tx, &input.user_id)?;
        let available = ledger
            .total_base_units
            .checked_sub(ledger.reserved_base_units)
            .ok_or_else(|| anyhow!("ESK 余额状态无效"))?;
        if input.amount_base_units > available {
            bail!("量化 Paper 分配申请金额超过当前可用 ESK");
        }

        let request_id = new_id("eskq");
        let submitted_at = now();
        tx.execute(
            "INSERT INTO esk_quant_allocation_requests (
               request_id, user_id, amount_base_units, idempotency_key,
               risk_disclosure_revision, submitted_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                request_id,
                input.user_id,
                input.amount_base_units,
                input.idempotency_key,
                input.risk_disclosure_revision,
                submitted_at,
            ],
        )?;
        tx.execute(
            "INSERT INTO esk_quant_allocation_request_events (
               event_id, request_id, revision, status, actor_user_id, created_at
             ) VALUES (?1, ?2, 1, 'submitted', ?3, ?4)",
            params![new_id("eskqe"), request_id, input.user_id, submitted_at],
        )?;
        let record = by_id(&tx, &input.user_id, &request_id)?
            .ok_or_else(|| anyhow!("量化 Paper 分配申请写入后不可见"))?;
        tx.commit()?;
        Ok(record)
    }

    pub(crate) fn list_esk_quant_allocation_requests(
        &self,
        user_id: &str,
        limit: usize,
    ) -> Result<Vec<EskQuantAllocationRecord>> {
        let conn = self.conn()?;
        let mut statement = conn.prepare(
            "SELECT r.request_id, r.user_id, r.amount_base_units, r.idempotency_key,
                    r.risk_disclosure_revision, e.status, e.revision,
                    r.submitted_at, e.created_at
               FROM esk_quant_allocation_requests r
               JOIN esk_quant_allocation_request_events e
                 ON e.request_id = r.request_id
                AND e.revision = (
                  SELECT MAX(latest.revision)
                    FROM esk_quant_allocation_request_events latest
                   WHERE latest.request_id = r.request_id
                )
              WHERE r.user_id = ?1
              ORDER BY r.submitted_at DESC, r.request_id DESC
              LIMIT ?2",
        )?;
        let rows = statement.query_map(params![user_id, limit.clamp(1, 100) as i64], map_record)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub(crate) fn cancel_esk_quant_allocation_request(
        &self,
        user_id: &str,
        request_id: &str,
    ) -> Result<EskQuantAllocationRecord> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current =
            by_id(&tx, user_id, request_id)?.ok_or_else(|| anyhow!("量化 Paper 分配申请不存在"))?;
        if current.status == "canceled" {
            tx.commit()?;
            return Ok(EskQuantAllocationRecord {
                replayed: true,
                ..current
            });
        }
        if current.status != "submitted" {
            bail!("当前量化 Paper 分配申请状态不能取消");
        }
        let updated_at = now();
        tx.execute(
            "INSERT INTO esk_quant_allocation_request_events (
               event_id, request_id, revision, status, actor_user_id, created_at
             ) VALUES (?1, ?2, ?3, 'canceled', ?4, ?5)",
            params![
                new_id("eskqe"),
                request_id,
                current.revision + 1,
                user_id,
                updated_at,
            ],
        )?;
        let result = by_id(&tx, user_id, request_id)?
            .ok_or_else(|| anyhow!("量化 Paper 分配申请取消后不可见"))?;
        tx.commit()?;
        Ok(result)
    }
}

fn validate_input(input: &EskQuantAllocationInput) -> Result<()> {
    if input.amount_base_units <= 0 {
        bail!("量化 Paper 分配申请金额必须大于 0");
    }
    validate_key(&input.user_id, "用户 ID", 160)?;
    validate_key(&input.idempotency_key, "幂等键", 160)?;
    if input.risk_disclosure_revision != ESK_QUANT_RISK_DISCLOSURE_REVISION {
        bail!("量化 Paper 风险披露版本不匹配");
    }
    Ok(())
}

fn by_idempotency(
    conn: &Connection,
    user_id: &str,
    idempotency_key: &str,
) -> Result<Option<EskQuantAllocationRecord>> {
    let request_id = conn
        .query_row(
            "SELECT request_id FROM esk_quant_allocation_requests
              WHERE user_id = ?1 AND idempotency_key = ?2",
            params![user_id, idempotency_key],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    match request_id {
        Some(request_id) => by_id(conn, user_id, &request_id),
        None => Ok(None),
    }
}

fn by_id(
    conn: &Connection,
    user_id: &str,
    request_id: &str,
) -> Result<Option<EskQuantAllocationRecord>> {
    conn.query_row(
        "SELECT r.request_id, r.user_id, r.amount_base_units, r.idempotency_key,
                r.risk_disclosure_revision, e.status, e.revision,
                r.submitted_at, e.created_at
           FROM esk_quant_allocation_requests r
           JOIN esk_quant_allocation_request_events e
             ON e.request_id = r.request_id
            AND e.revision = (
              SELECT MAX(latest.revision)
                FROM esk_quant_allocation_request_events latest
               WHERE latest.request_id = r.request_id
            )
          WHERE r.user_id = ?1 AND r.request_id = ?2",
        params![user_id, request_id],
        map_record,
    )
    .optional()
    .map_err(Into::into)
}

fn map_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<EskQuantAllocationRecord> {
    Ok(EskQuantAllocationRecord {
        request_id: row.get(0)?,
        user_id: row.get(1)?,
        amount_base_units: row.get(2)?,
        idempotency_key: row.get(3)?,
        risk_disclosure_revision: row.get(4)?,
        status: row.get(5)?,
        revision: row.get(6)?,
        submitted_at: row.get(7)?,
        updated_at: row.get(8)?,
        replayed: false,
    })
}
