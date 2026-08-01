use anyhow::{bail, Result};
use rusqlite::{params, Transaction, TransactionBehavior};
use serde_json::json;

use crate::open_commerce_model::SETTLEMENT_RECORDED_NOT_CHARGED;

use super::{new_id, now, Store};

pub(crate) const OPEN_COMMERCE_INVOCATION_LEASE_SECONDS: i64 = 120;

struct StartedInvocation {
    invocation_id: String,
    project_id: String,
    merchant_id: String,
    capability_key: String,
    requester_user_id: String,
    requester_app_id: String,
    grant_id: Option<String>,
    reserved_invocations: Option<i64>,
    reserved_amount_micros: Option<i64>,
}

impl Store {
    pub(crate) fn recover_interrupted_open_commerce_invocations(&self) -> Result<usize> {
        self.fail_started_open_commerce_invocations(None, "server_restart_interrupted")
    }

    pub(crate) fn reconcile_expired_open_commerce_invocations(&self) -> Result<usize> {
        self.fail_started_open_commerce_invocations(
            Some(OPEN_COMMERCE_INVOCATION_LEASE_SECONDS),
            "invocation_lease_expired",
        )
    }

    fn fail_started_open_commerce_invocations(
        &self,
        older_than_seconds: Option<i64>,
        error_code: &str,
    ) -> Result<usize> {
        let timestamp = now();
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let started = select_started_invocations(&tx, older_than_seconds)?;
        let recovered_count = started.len();
        for invocation in started {
            let released = release_reservation(&tx, &invocation, &timestamp)?;
            let updated = tx.execute(
                "UPDATE open_commerce_invocations
                    SET status = 'failed', result_json = NULL, error_code = ?1,
                        units = 0, amount_micros = 0, settlement_status = ?2,
                        completed_at = ?3
                  WHERE id = ?4 AND status = 'started'",
                params![
                    error_code,
                    SETTLEMENT_RECORDED_NOT_CHARGED,
                    timestamp,
                    invocation.invocation_id
                ],
            )?;
            if updated != 1 {
                bail!("开放商业孤儿调用恢复发生并发冲突");
            }
            tx.execute(
                "INSERT INTO open_commerce_audit_events (
                    id, project_id, actor_user_id, actor_app_id, action,
                    subject_type, subject_id, metadata_json, created_at
                 ) VALUES (?1, ?2, ?3, ?4, 'invocation.recovered_failed',
                           'invocation', ?5, ?6, ?7)",
                params![
                    new_id("audit"),
                    invocation.project_id,
                    invocation.requester_user_id,
                    invocation.requester_app_id,
                    invocation.invocation_id,
                    serde_json::to_string(&json!({
                        "merchant_id": invocation.merchant_id,
                        "capability_key": invocation.capability_key,
                        "error_code": error_code,
                        "budget_reservation_released": released
                    }))?,
                    timestamp
                ],
            )?;
        }
        tx.commit()?;
        Ok(recovered_count)
    }
}

fn select_started_invocations(
    tx: &Transaction<'_>,
    older_than_seconds: Option<i64>,
) -> Result<Vec<StartedInvocation>> {
    const SELECT: &str = "SELECT i.id, i.project_id, i.merchant_id, i.capability_key,
                i.requester_user_id, i.requester_app_id,
                r.grant_id, r.reserved_invocations, r.reserved_amount_micros
           FROM open_commerce_invocations i
           LEFT JOIN open_commerce_grant_budget_reservations r
             ON r.invocation_id = i.id AND r.status = 'reserved'
          WHERE i.status = 'started'";
    if let Some(seconds) = older_than_seconds {
        let mut stmt = tx.prepare(&format!(
            "{SELECT} AND julianday(i.created_at) < julianday('now', ?1)
              ORDER BY i.created_at"
        ))?;
        let rows = stmt.query_map(params![format!("-{seconds} seconds")], read_started)?;
        return rows
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into);
    }
    let mut stmt = tx.prepare(&format!("{SELECT} ORDER BY i.created_at"))?;
    let rows = stmt.query_map([], read_started)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn read_started(row: &rusqlite::Row<'_>) -> rusqlite::Result<StartedInvocation> {
    Ok(StartedInvocation {
        invocation_id: row.get(0)?,
        project_id: row.get(1)?,
        merchant_id: row.get(2)?,
        capability_key: row.get(3)?,
        requester_user_id: row.get(4)?,
        requester_app_id: row.get(5)?,
        grant_id: row.get(6)?,
        reserved_invocations: row.get(7)?,
        reserved_amount_micros: row.get(8)?,
    })
}

fn release_reservation(
    tx: &Transaction<'_>,
    invocation: &StartedInvocation,
    timestamp: &str,
) -> Result<bool> {
    let Some(grant_id) = invocation.grant_id.as_deref() else {
        return Ok(false);
    };
    let reserved_invocations = invocation.reserved_invocations.unwrap_or_default();
    let reserved_amount_micros = invocation.reserved_amount_micros.unwrap_or_default();
    let updated = tx.execute(
        "UPDATE open_commerce_grants
            SET used_invocations = used_invocations - ?1,
                used_amount_micros = used_amount_micros - ?2,
                updated_at = ?3
          WHERE id = ?4 AND used_invocations >= ?1 AND used_amount_micros >= ?2",
        params![
            reserved_invocations,
            reserved_amount_micros,
            timestamp,
            grant_id
        ],
    )?;
    if updated != 1 {
        bail!("开放商业孤儿调用预算释放失败，计数状态不一致");
    }
    let released = tx.execute(
        "UPDATE open_commerce_grant_budget_reservations
            SET status = 'released', completed_at = ?1
          WHERE invocation_id = ?2 AND status = 'reserved'",
        params![timestamp, invocation.invocation_id],
    )?;
    if released != 1 {
        bail!("开放商业孤儿调用预算预留状态不一致");
    }
    Ok(true)
}
