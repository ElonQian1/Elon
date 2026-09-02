use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

use super::{new_id, now};
use crate::esk_asset::{
    EskAccountLedger, EskAllocationInput, EskAllocationReceipt, EskSellbackInput, EskSellbackRecord,
};
use crate::store::Store;

impl Store {
    pub(crate) fn esk_account_ledger(&self, user_id: &str) -> Result<EskAccountLedger> {
        let conn = self.conn()?;
        account_ledger_on(&conn, user_id)
    }

    pub(crate) fn create_esk_paper_allocation(
        &self,
        input: &EskAllocationInput,
    ) -> Result<EskAllocationReceipt> {
        validate_allocation(input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_user_exists(&tx, &input.user_id)?;
        if let Some(existing) = allocation_by_idempotency_on(&tx, &input.idempotency_key)? {
            if existing.user_id != input.user_id
                || existing.amount_base_units != input.amount_base_units
                || existing.reference != input.reference
            {
                bail!("相同 ESK 登记幂等键不能用于不同请求");
            }
            tx.commit()?;
            return Ok(EskAllocationReceipt {
                replayed: true,
                ..existing
            });
        }
        let receipt = EskAllocationReceipt {
            entry_id: new_id("eska"),
            user_id: input.user_id.clone(),
            amount_base_units: input.amount_base_units,
            reference: input.reference.clone(),
            idempotency_key: input.idempotency_key.clone(),
            created_at: now(),
            replayed: false,
        };
        tx.execute(
            "INSERT INTO esk_asset_ledger_entries (
               entry_id, user_id, amount_base_units, entry_kind, reference,
               idempotency_key, actor, created_at
             ) VALUES (?1, ?2, ?3, 'paper_allocation', ?4, ?5, 'platform_admin', ?6)",
            params![
                receipt.entry_id,
                receipt.user_id,
                receipt.amount_base_units,
                receipt.reference,
                receipt.idempotency_key,
                receipt.created_at,
            ],
        )?;
        tx.commit()?;
        Ok(receipt)
    }

    pub(crate) fn create_esk_sellback_request(
        &self,
        input: &EskSellbackInput,
    ) -> Result<EskSellbackRecord> {
        validate_sellback(input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_user_exists(&tx, &input.user_id)?;
        if let Some(existing) =
            sellback_by_idempotency_on(&tx, &input.user_id, &input.idempotency_key)?
        {
            if existing.amount_base_units != input.amount_base_units {
                bail!("相同卖回申请幂等键不能用于不同金额");
            }
            tx.commit()?;
            return Ok(EskSellbackRecord {
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
            bail!("卖回申请金额超过当前可用 ESK");
        }
        let request_id = new_id("eskr");
        let submitted_at = now();
        tx.execute(
            "INSERT INTO esk_sellback_requests (
               request_id, user_id, amount_base_units, idempotency_key, submitted_at
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                request_id,
                input.user_id,
                input.amount_base_units,
                input.idempotency_key,
                submitted_at,
            ],
        )?;
        tx.execute(
            "INSERT INTO esk_sellback_request_events (
               event_id, request_id, revision, status, actor_user_id, created_at
             ) VALUES (?1, ?2, 1, 'submitted', ?3, ?4)",
            params![new_id("eske"), request_id, input.user_id, submitted_at],
        )?;
        let record = sellback_by_id_on(&tx, &input.user_id, &request_id)?
            .ok_or_else(|| anyhow!("卖回申请写入后不可见"))?;
        tx.commit()?;
        Ok(record)
    }

    pub(crate) fn list_esk_sellback_requests(
        &self,
        user_id: &str,
        limit: usize,
    ) -> Result<Vec<EskSellbackRecord>> {
        let conn = self.conn()?;
        let mut statement = conn.prepare(
            "SELECT r.request_id, r.user_id, r.amount_base_units,
                    e.status, e.revision, r.submitted_at, e.created_at
               FROM esk_sellback_requests r
               JOIN esk_sellback_request_events e
                 ON e.request_id = r.request_id
                AND e.revision = (
                  SELECT MAX(latest.revision)
                    FROM esk_sellback_request_events latest
                   WHERE latest.request_id = r.request_id
                )
              WHERE r.user_id = ?1
              ORDER BY r.submitted_at DESC, r.request_id DESC
              LIMIT ?2",
        )?;
        let rows =
            statement.query_map(params![user_id, limit.clamp(1, 100) as i64], map_sellback)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub(crate) fn cancel_esk_sellback_request(
        &self,
        user_id: &str,
        request_id: &str,
    ) -> Result<EskSellbackRecord> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = sellback_by_id_on(&tx, user_id, request_id)?
            .ok_or_else(|| anyhow!("卖回申请不存在"))?;
        if current.status == "canceled" {
            tx.commit()?;
            return Ok(EskSellbackRecord {
                replayed: true,
                ..current
            });
        }
        if current.status != "submitted" {
            bail!("当前卖回申请状态不能取消");
        }
        let updated_at = now();
        tx.execute(
            "INSERT INTO esk_sellback_request_events (
               event_id, request_id, revision, status, actor_user_id, created_at
             ) VALUES (?1, ?2, ?3, 'canceled', ?4, ?5)",
            params![
                new_id("eske"),
                request_id,
                current.revision + 1,
                user_id,
                updated_at,
            ],
        )?;
        let result = sellback_by_id_on(&tx, user_id, request_id)?
            .ok_or_else(|| anyhow!("卖回申请取消后不可见"))?;
        tx.commit()?;
        Ok(result)
    }
}

pub(super) fn account_ledger_on(conn: &Connection, user_id: &str) -> Result<EskAccountLedger> {
    let (total, revision, updated_at): (i64, i64, Option<String>) = conn.query_row(
        "SELECT COALESCE(SUM(amount_base_units), 0), COUNT(*), MAX(created_at)
           FROM esk_asset_ledger_entries
          WHERE user_id = ?1",
        params![user_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    let sellback_reserved: i64 = conn.query_row(
        "SELECT COALESCE(SUM(r.amount_base_units), 0)
           FROM esk_sellback_requests r
          WHERE r.user_id = ?1
            AND (SELECT e.status
                   FROM esk_sellback_request_events e
                  WHERE e.request_id = r.request_id
                  ORDER BY e.revision DESC
                  LIMIT 1) = 'submitted'",
        params![user_id],
        |row| row.get(0),
    )?;
    let (quant_reserved, quant_event_count, quant_updated_at): (i64, i64, Option<String>) =
        conn.query_row(
            "SELECT
               COALESCE(SUM(CASE WHEN latest.status = 'submitted' THEN r.amount_base_units ELSE 0 END), 0),
               COALESCE(SUM(latest.revision), 0),
               MAX(latest.created_at)
             FROM esk_quant_allocation_requests r
             JOIN (
               SELECT e.request_id, e.status, e.revision, e.created_at
                 FROM esk_quant_allocation_request_events e
                WHERE e.revision = (
                  SELECT MAX(candidate.revision)
                    FROM esk_quant_allocation_request_events candidate
                   WHERE candidate.request_id = e.request_id
                )
             ) latest ON latest.request_id = r.request_id
            WHERE r.user_id = ?1",
            params![user_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
    let (sellback_event_count, sellback_updated_at): (i64, Option<String>) = conn.query_row(
        "SELECT COUNT(*), MAX(created_at)
           FROM esk_sellback_request_events
          WHERE request_id IN (SELECT request_id FROM esk_sellback_requests WHERE user_id = ?1)",
        params![user_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let sellback_reserved = sellback_reserved.max(0);
    let quant_reserved = quant_reserved.max(0);
    let reserved = sellback_reserved
        .checked_add(quant_reserved)
        .ok_or_else(|| anyhow!("ESK 占用余额超出范围"))?;
    if reserved > total.max(0) {
        bail!("ESK 占用余额超过总余额");
    }
    Ok(EskAccountLedger {
        total_base_units: total.max(0),
        sellback_reserved_base_units: sellback_reserved,
        quant_reserved_base_units: quant_reserved,
        reserved_base_units: reserved,
        revision: revision
            .saturating_add(sellback_event_count)
            .saturating_add(quant_event_count)
            .max(0),
        updated_at: [updated_at, sellback_updated_at, quant_updated_at]
            .into_iter()
            .flatten()
            .max(),
    })
}

fn allocation_by_idempotency_on(
    conn: &Connection,
    idempotency_key: &str,
) -> Result<Option<EskAllocationReceipt>> {
    conn.query_row(
        "SELECT entry_id, user_id, amount_base_units, reference, idempotency_key, created_at
           FROM esk_asset_ledger_entries
          WHERE entry_kind = 'paper_allocation' AND idempotency_key = ?1",
        params![idempotency_key],
        |row| {
            Ok(EskAllocationReceipt {
                entry_id: row.get(0)?,
                user_id: row.get(1)?,
                amount_base_units: row.get(2)?,
                reference: row.get(3)?,
                idempotency_key: row.get(4)?,
                created_at: row.get(5)?,
                replayed: false,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

fn sellback_by_idempotency_on(
    conn: &Connection,
    user_id: &str,
    idempotency_key: &str,
) -> Result<Option<EskSellbackRecord>> {
    let request_id = conn
        .query_row(
            "SELECT request_id FROM esk_sellback_requests
              WHERE user_id = ?1 AND idempotency_key = ?2",
            params![user_id, idempotency_key],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    match request_id {
        Some(request_id) => sellback_by_id_on(conn, user_id, &request_id),
        None => Ok(None),
    }
}

fn sellback_by_id_on(
    conn: &Connection,
    user_id: &str,
    request_id: &str,
) -> Result<Option<EskSellbackRecord>> {
    conn.query_row(
        "SELECT r.request_id, r.user_id, r.amount_base_units,
                e.status, e.revision, r.submitted_at, e.created_at
           FROM esk_sellback_requests r
           JOIN esk_sellback_request_events e
             ON e.request_id = r.request_id
            AND e.revision = (
              SELECT MAX(latest.revision)
                FROM esk_sellback_request_events latest
               WHERE latest.request_id = r.request_id
            )
          WHERE r.user_id = ?1 AND r.request_id = ?2",
        params![user_id, request_id],
        map_sellback,
    )
    .optional()
    .map_err(Into::into)
}

fn map_sellback(row: &rusqlite::Row<'_>) -> rusqlite::Result<EskSellbackRecord> {
    Ok(EskSellbackRecord {
        request_id: row.get(0)?,
        user_id: row.get(1)?,
        amount_base_units: row.get(2)?,
        status: row.get(3)?,
        revision: row.get(4)?,
        submitted_at: row.get(5)?,
        updated_at: row.get(6)?,
        replayed: false,
    })
}

pub(super) fn ensure_user_exists(conn: &Connection, user_id: &str) -> Result<()> {
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

fn validate_allocation(input: &EskAllocationInput) -> Result<()> {
    if input.amount_base_units <= 0 {
        bail!("ESK 登记金额必须大于 0");
    }
    validate_key(&input.user_id, "用户 ID", 160)?;
    validate_key(&input.reference, "登记引用", 240)?;
    validate_key(&input.idempotency_key, "幂等键", 160)
}

fn validate_sellback(input: &EskSellbackInput) -> Result<()> {
    if input.amount_base_units <= 0 {
        bail!("卖回申请金额必须大于 0");
    }
    validate_key(&input.user_id, "用户 ID", 160)?;
    validate_key(&input.idempotency_key, "幂等键", 160)
}

pub(super) fn validate_key(value: &str, label: &str, max_chars: usize) -> Result<()> {
    let value = value.trim();
    let length = value.chars().count();
    if length == 0 || length > max_chars || value.chars().any(char::is_control) {
        bail!("{label} 无效");
    }
    Ok(())
}
