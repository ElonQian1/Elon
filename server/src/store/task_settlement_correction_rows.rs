use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};
use sha2::{Digest, Sha256};

use crate::task_settlement::model::{
    SettlementCorrection, SettlementCorrectionDetail, SettlementCorrectionEvent, SettlementReceipt,
};

use super::{
    new_id,
    task_settlement_rows::{read_settlement, settlement_select},
};

pub(super) fn correction_select() -> &'static str {
    "SELECT correction.id, correction.project_id, correction.dispute_id,
            correction.original_settlement_receipt_id, correction.correction_matter_id,
            correction.status, correction.corrected_compute_amount_micros,
            correction.corrected_provider_amount_micros,
            correction.corrected_platform_amount_micros, correction.summary,
            correction.evidence_ref, correction.created_by_user_id,
            correction.posted_by_user_id, correction.reversal_receipt_id,
            correction.replacement_receipt_id, matter.status, matter.final_decision,
            correction.created_at, correction.posted_at, correction.updated_at
       FROM task_settlement_corrections correction
       JOIN project_ai_matters matter ON matter.id=correction.correction_matter_id"
}

pub(super) fn select_correction(
    conn: &Connection,
    project_id: &str,
    correction_id: &str,
) -> Result<Option<SettlementCorrection>> {
    conn.query_row(
        &format!(
            "{} WHERE correction.project_id=?1 AND correction.id=?2",
            correction_select()
        ),
        params![project_id.trim(), correction_id.trim()],
        read_correction,
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn select_active_correction(
    conn: &Connection,
    project_id: &str,
    dispute_id: &str,
) -> Result<Option<SettlementCorrection>> {
    conn.query_row(
        &format!(
            "{} WHERE correction.project_id=?1 AND correction.dispute_id=?2
                  AND correction.status IN ('matter_pending', 'posted')
                ORDER BY CASE correction.status WHEN 'posted' THEN 0 ELSE 1 END LIMIT 1",
            correction_select()
        ),
        params![project_id.trim(), dispute_id.trim()],
        read_correction,
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn correction_detail_with_conn(
    conn: &Connection,
    correction: SettlementCorrection,
) -> Result<SettlementCorrectionDetail> {
    let mut stmt = conn.prepare(
        "SELECT id, correction_id, action, previous_status, next_status,
                actor_user_id, note, created_at
           FROM task_settlement_correction_events
          WHERE correction_id=?1 ORDER BY created_at, id",
    )?;
    let events = stmt
        .query_map([&correction.id], read_correction_event)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let original_receipt = select_receipt(
        conn,
        &correction.project_id,
        &correction.original_settlement_receipt_id,
    )?
    .ok_or_else(|| anyhow!("纠正关联的原凭证不存在"))?;
    let reversal_receipt =
        optional_receipt(conn, &correction, correction.reversal_receipt_id.as_deref())?;
    let replacement_receipt = optional_receipt(
        conn,
        &correction,
        correction.replacement_receipt_id.as_deref(),
    )?;
    Ok(SettlementCorrectionDetail {
        correction,
        events,
        original_receipt,
        reversal_receipt,
        replacement_receipt,
    })
}

fn optional_receipt(
    conn: &Connection,
    correction: &SettlementCorrection,
    receipt_id: Option<&str>,
) -> Result<Option<SettlementReceipt>> {
    receipt_id
        .map(|id| select_receipt(conn, &correction.project_id, id))
        .transpose()
        .map(Option::flatten)
}

pub(super) fn select_receipt(
    conn: &Connection,
    project_id: &str,
    receipt_id: &str,
) -> Result<Option<SettlementReceipt>> {
    conn.query_row(
        &format!("{} WHERE project_id=?1 AND id=?2", settlement_select()),
        params![project_id.trim(), receipt_id.trim()],
        read_settlement,
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn read_correction(row: &Row<'_>) -> rusqlite::Result<SettlementCorrection> {
    Ok(SettlementCorrection {
        id: row.get(0)?,
        project_id: row.get(1)?,
        dispute_id: row.get(2)?,
        original_settlement_receipt_id: row.get(3)?,
        correction_matter_id: row.get(4)?,
        status: row.get(5)?,
        corrected_compute_amount_micros: row.get(6)?,
        corrected_provider_amount_micros: row.get(7)?,
        corrected_platform_amount_micros: row.get(8)?,
        summary: row.get(9)?,
        evidence_ref: row.get(10)?,
        created_by_user_id: row.get(11)?,
        posted_by_user_id: row.get(12)?,
        reversal_receipt_id: row.get(13)?,
        replacement_receipt_id: row.get(14)?,
        matter_status: row.get(15)?,
        matter_final_decision: row.get(16)?,
        created_at: row.get(17)?,
        posted_at: row.get(18)?,
        updated_at: row.get(19)?,
    })
}

fn read_correction_event(row: &Row<'_>) -> rusqlite::Result<SettlementCorrectionEvent> {
    Ok(SettlementCorrectionEvent {
        id: row.get(0)?,
        correction_id: row.get(1)?,
        action: row.get(2)?,
        previous_status: row.get(3)?,
        next_status: row.get(4)?,
        actor_user_id: row.get(5)?,
        note: row.get(6)?,
        created_at: row.get(7)?,
    })
}

pub(super) fn insert_correction_event(
    conn: &Connection,
    correction_id: &str,
    action: &str,
    previous_status: Option<&str>,
    next_status: &str,
    actor_user_id: &str,
    note: Option<&str>,
    timestamp: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO task_settlement_correction_events (
           id, correction_id, action, previous_status, next_status,
           actor_user_id, note, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            new_id("settlement_correction_event"),
            correction_id.trim(),
            action.trim(),
            clean(previous_status),
            next_status.trim(),
            actor_user_id.trim(),
            clean(note),
            timestamp,
        ],
    )?;
    Ok(())
}

pub(super) fn digest_text(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

pub(super) fn clean(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
