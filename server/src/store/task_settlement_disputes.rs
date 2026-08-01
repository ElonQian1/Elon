use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::task_settlement::model::{
    CreateSettlementDispute, SettlementDispute, SettlementDisputeDetail, SettlementDisputeEvent,
    DISPUTE_ACCEPTED, DISPUTE_OPEN, DISPUTE_REJECTED, DISPUTE_WITHDRAWN, RECEIPT_RECONCILED,
};

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn create_task_settlement_dispute(
        &self,
        input: CreateSettlementDispute<'_>,
    ) -> Result<SettlementDisputeDetail> {
        let timestamp = now();
        let dispute_id = new_id("settlement_dispute");
        let event_id = new_id("settlement_dispute_event");
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let receipt_status = tx
            .query_row(
                "SELECT status FROM task_settlement_receipts
                  WHERE project_id=?1 AND id=?2",
                params![input.project_id.trim(), input.settlement_receipt_id.trim()],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| anyhow!("影子结算凭证不存在"))?;
        if receipt_status != RECEIPT_RECONCILED {
            bail!("只有已对账影子凭证可以提出争议");
        }
        if has_dispute_status(
            &tx,
            input.project_id,
            input.settlement_receipt_id,
            DISPUTE_ACCEPTED,
        )? {
            bail!("该影子凭证已有已接受争议，不能重复提出");
        }
        if let Some(existing) =
            select_open_dispute(&tx, input.project_id, input.settlement_receipt_id)?
        {
            let same = existing.reason_code == input.reason_code.trim()
                && existing.summary == input.summary.trim()
                && existing.evidence_ref.as_deref() == clean(input.evidence_ref)
                && existing.opened_by_user_id == input.actor_user_id.trim();
            if !same {
                bail!("该影子凭证已有待审核争议，内容发生冲突");
            }
            let existing_id = existing.id.clone();
            tx.commit()?;
            drop(conn);
            return self
                .task_settlement_dispute_detail(input.project_id, &existing_id)?
                .ok_or_else(|| anyhow!("影子结算争议幂等读取失败"));
        }
        tx.execute(
            "INSERT INTO task_settlement_disputes (
               id, project_id, settlement_receipt_id, status, reason_code,
               summary, evidence_ref, opened_by_user_id, resolved_by_user_id,
               resolution_note, opened_at, resolved_at, updated_at
             ) VALUES (
               ?1, ?2, ?3, 'open', ?4, ?5, ?6, ?7, NULL, NULL, ?8, NULL, ?8
             )",
            params![
                dispute_id,
                input.project_id.trim(),
                input.settlement_receipt_id.trim(),
                input.reason_code.trim(),
                input.summary.trim(),
                clean(input.evidence_ref),
                input.actor_user_id.trim(),
                timestamp,
            ],
        )?;
        tx.execute(
            "INSERT INTO task_settlement_dispute_events (
               id, dispute_id, action, previous_status, next_status,
               actor_user_id, note, created_at
             ) VALUES (?1, ?2, 'opened', NULL, 'open', ?3, NULL, ?4)",
            params![event_id, dispute_id, input.actor_user_id.trim(), timestamp],
        )?;
        tx.commit()?;
        drop(conn);
        self.task_settlement_dispute_detail(input.project_id, &dispute_id)?
            .ok_or_else(|| anyhow!("影子结算争议写入后无法读取"))
    }

    pub(crate) fn transition_task_settlement_dispute(
        &self,
        project_id: &str,
        dispute_id: &str,
        actor_user_id: &str,
        target_status: &str,
        note: &str,
    ) -> Result<SettlementDisputeDetail> {
        let action = match target_status.trim() {
            DISPUTE_ACCEPTED => DISPUTE_ACCEPTED,
            DISPUTE_REJECTED => DISPUTE_REJECTED,
            DISPUTE_WITHDRAWN => DISPUTE_WITHDRAWN,
            _ => bail!("未知影子结算争议处理结果"),
        };
        let timestamp = now();
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let dispute = select_dispute(&tx, project_id, dispute_id)?
            .ok_or_else(|| anyhow!("影子结算争议不存在"))?;
        if dispute.status == action {
            tx.commit()?;
            drop(conn);
            return self
                .task_settlement_dispute_detail(project_id, dispute_id)?
                .ok_or_else(|| anyhow!("影子结算争议幂等读取失败"));
        }
        if dispute.status != DISPUTE_OPEN {
            bail!("已结束的影子结算争议不能再次处理");
        }
        tx.execute(
            "UPDATE task_settlement_disputes
                SET status=?3,
                    resolved_by_user_id=?4,
                    resolution_note=?5,
                    resolved_at=?6,
                    updated_at=?6
              WHERE project_id=?1 AND id=?2 AND status='open'",
            params![
                project_id.trim(),
                dispute_id.trim(),
                action,
                actor_user_id.trim(),
                note.trim(),
                timestamp,
            ],
        )?;
        tx.execute(
            "INSERT INTO task_settlement_dispute_events (
               id, dispute_id, action, previous_status, next_status,
               actor_user_id, note, created_at
             ) VALUES (?1, ?2, ?3, 'open', ?4, ?5, ?6, ?7)",
            params![
                new_id("settlement_dispute_event"),
                dispute_id.trim(),
                action,
                action,
                actor_user_id.trim(),
                note.trim(),
                timestamp,
            ],
        )?;
        tx.commit()?;
        drop(conn);
        self.task_settlement_dispute_detail(project_id, dispute_id)?
            .ok_or_else(|| anyhow!("影子结算争议处理后无法读取"))
    }

    pub(crate) fn list_task_settlement_disputes(
        &self,
        project_id: &str,
        receipt_id: &str,
        limit: usize,
    ) -> Result<Vec<SettlementDisputeDetail>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{} WHERE project_id=?1 AND settlement_receipt_id=?2
                ORDER BY opened_at DESC LIMIT ?3",
            dispute_select()
        ))?;
        let disputes = stmt
            .query_map(
                params![
                    project_id.trim(),
                    receipt_id.trim(),
                    limit.clamp(1, 100) as i64
                ],
                read_dispute,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        disputes
            .into_iter()
            .map(|dispute| detail_with_conn(&conn, dispute))
            .collect()
    }

    pub(crate) fn task_settlement_dispute_detail(
        &self,
        project_id: &str,
        dispute_id: &str,
    ) -> Result<Option<SettlementDisputeDetail>> {
        let conn = self.conn()?;
        select_dispute(&conn, project_id, dispute_id)?
            .map(|dispute| detail_with_conn(&conn, dispute))
            .transpose()
    }

    pub(crate) fn task_settlement_has_blocking_dispute(
        &self,
        project_id: &str,
        receipt_id: &str,
    ) -> Result<bool> {
        let conn = self.conn()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM task_settlement_disputes
              WHERE project_id=?1 AND settlement_receipt_id=?2
                AND status IN ('open', 'accepted')",
            params![project_id.trim(), receipt_id.trim()],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }
}

fn dispute_select() -> &'static str {
    "SELECT id, project_id, settlement_receipt_id, status, reason_code,
            summary, evidence_ref, opened_by_user_id, resolved_by_user_id,
            resolution_note, opened_at, resolved_at, updated_at
       FROM task_settlement_disputes"
}

fn select_dispute(
    conn: &Connection,
    project_id: &str,
    dispute_id: &str,
) -> Result<Option<SettlementDispute>> {
    conn.query_row(
        &format!("{} WHERE project_id=?1 AND id=?2", dispute_select()),
        params![project_id.trim(), dispute_id.trim()],
        read_dispute,
    )
    .optional()
    .map_err(Into::into)
}

fn select_open_dispute(
    conn: &Connection,
    project_id: &str,
    receipt_id: &str,
) -> Result<Option<SettlementDispute>> {
    conn.query_row(
        &format!(
            "{} WHERE project_id=?1 AND settlement_receipt_id=?2 AND status='open'",
            dispute_select()
        ),
        params![project_id.trim(), receipt_id.trim()],
        read_dispute,
    )
    .optional()
    .map_err(Into::into)
}

fn has_dispute_status(
    conn: &Connection,
    project_id: &str,
    receipt_id: &str,
    status: &str,
) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM task_settlement_disputes
          WHERE project_id=?1 AND settlement_receipt_id=?2 AND status=?3",
        params![project_id.trim(), receipt_id.trim(), status],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn detail_with_conn(
    conn: &Connection,
    dispute: SettlementDispute,
) -> Result<SettlementDisputeDetail> {
    let mut stmt = conn.prepare(
        "SELECT id, dispute_id, action, previous_status, next_status,
                actor_user_id, note, created_at
           FROM task_settlement_dispute_events
          WHERE dispute_id=?1 ORDER BY created_at, id",
    )?;
    let events = stmt
        .query_map(params![&dispute.id], read_event)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let blocks_projection = matches!(dispute.status.as_str(), DISPUTE_OPEN | DISPUTE_ACCEPTED);
    Ok(SettlementDisputeDetail {
        dispute,
        events,
        blocks_projection,
    })
}

fn read_dispute(row: &Row<'_>) -> rusqlite::Result<SettlementDispute> {
    Ok(SettlementDispute {
        id: row.get(0)?,
        project_id: row.get(1)?,
        settlement_receipt_id: row.get(2)?,
        status: row.get(3)?,
        reason_code: row.get(4)?,
        summary: row.get(5)?,
        evidence_ref: row.get(6)?,
        opened_by_user_id: row.get(7)?,
        resolved_by_user_id: row.get(8)?,
        resolution_note: row.get(9)?,
        opened_at: row.get(10)?,
        resolved_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

fn read_event(row: &Row<'_>) -> rusqlite::Result<SettlementDisputeEvent> {
    Ok(SettlementDisputeEvent {
        id: row.get(0)?,
        dispute_id: row.get(1)?,
        action: row.get(2)?,
        previous_status: row.get(3)?,
        next_status: row.get(4)?,
        actor_user_id: row.get(5)?,
        note: row.get(6)?,
        created_at: row.get(7)?,
    })
}

fn clean(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
