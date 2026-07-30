use anyhow::{bail, Result};
use rusqlite::{params, OptionalExtension, Row};

use crate::task_settlement::model::{
    CreateSettlementReceipt, SettlementIntent, SettlementReceipt, TaskEconomyProjectSetting,
    UsageReceipt,
};

pub(super) fn read_project_setting(row: &Row<'_>) -> rusqlite::Result<TaskEconomyProjectSetting> {
    Ok(TaskEconomyProjectSetting {
        project_id: row.get(0)?,
        enabled: row.get::<_, i64>(1)? != 0,
        shadow_only: row.get::<_, i64>(2)? != 0,
        updated_by_user_id: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

pub(super) fn read_usage(row: &Row<'_>) -> rusqlite::Result<UsageReceipt> {
    Ok(UsageReceipt {
        id: row.get(0)?,
        project_id: row.get(1)?,
        subject_type: row.get(2)?,
        subject_id: row.get(3)?,
        source_type: row.get(4)?,
        source_id: row.get(5)?,
        source_digest: row.get(6)?,
        consumer_user_id: row.get(7)?,
        provider_user_id: row.get(8)?,
        units: row.get(9)?,
        amount_micros: row.get(10)?,
        provider_amount_micros: row.get(11)?,
        currency: row.get(12)?,
        billing_source: row.get(13)?,
        source_status: row.get(14)?,
        occurred_at: row.get(15)?,
        created_at: row.get(16)?,
    })
}

pub(super) fn read_intent(row: &Row<'_>) -> rusqlite::Result<SettlementIntent> {
    Ok(SettlementIntent {
        id: row.get(0)?,
        project_id: row.get(1)?,
        matter_id: row.get(2)?,
        assignment_id: row.get(3)?,
        payer_user_id: row.get(4)?,
        payee_user_id: row.get(5)?,
        idempotency_key: row.get(6)?,
        policy_version: row.get(7)?,
        policy_digest: row.get(8)?,
        status: row.get(9)?,
        shadow_only: row.get::<_, i64>(10)? != 0,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

pub(super) fn read_settlement(row: &Row<'_>) -> rusqlite::Result<SettlementReceipt> {
    Ok(SettlementReceipt {
        id: row.get(0)?,
        project_id: row.get(1)?,
        intent_id: row.get(2)?,
        posting_key: row.get(3)?,
        status: row.get(4)?,
        compute_amount_micros: row.get(5)?,
        provider_amount_micros: row.get(6)?,
        platform_amount_micros: row.get(7)?,
        outcome_reward_micros: row.get(8)?,
        review_reward_micros: row.get(9)?,
        currency: row.get(10)?,
        shadow_only: row.get::<_, i64>(11)? != 0,
        accepted_matter_id: row.get(12)?,
        reason: row.get(13)?,
        created_at: row.get(14)?,
    })
}

pub(super) fn usage_select() -> &'static str {
    "SELECT id, project_id, subject_type, subject_id, source_type, source_id,
            source_digest, consumer_user_id, provider_user_id, units,
            amount_micros, provider_amount_micros, currency, billing_source,
            source_status, occurred_at, created_at
       FROM task_usage_receipts"
}

pub(super) fn usage_select_alias() -> &'static str {
    "SELECT receipt.id, receipt.project_id, receipt.subject_type, receipt.subject_id,
            receipt.source_type, receipt.source_id, receipt.source_digest,
            receipt.consumer_user_id, receipt.provider_user_id, receipt.units,
            receipt.amount_micros, receipt.provider_amount_micros, receipt.currency,
            receipt.billing_source, receipt.source_status, receipt.occurred_at,
            receipt.created_at
       FROM task_usage_receipts receipt"
}

pub(super) fn intent_select() -> &'static str {
    "SELECT id, project_id, matter_id, assignment_id, payer_user_id, payee_user_id,
            idempotency_key, policy_version, policy_digest, status, shadow_only,
            created_at, updated_at
       FROM task_settlement_intents"
}

pub(super) fn settlement_select() -> &'static str {
    "SELECT id, project_id, intent_id, posting_key, status,
            compute_amount_micros, provider_amount_micros, platform_amount_micros,
            outcome_reward_micros, review_reward_micros, currency, shadow_only,
            accepted_matter_id, reason, created_at
       FROM task_settlement_receipts"
}

pub(super) fn select_usage_by_source(
    conn: &rusqlite::Connection,
    project_id: &str,
    source_type: &str,
    source_id: &str,
) -> Result<Option<UsageReceipt>> {
    conn.query_row(
        &format!(
            "{} WHERE project_id = ?1 AND source_type = ?2 AND source_id = ?3",
            usage_select()
        ),
        params![project_id.trim(), source_type.trim(), source_id.trim()],
        read_usage,
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn select_intent_by_key(
    conn: &rusqlite::Connection,
    project_id: &str,
    idempotency_key: &str,
) -> Result<Option<SettlementIntent>> {
    conn.query_row(
        &format!(
            "{} WHERE project_id = ?1 AND idempotency_key = ?2",
            intent_select()
        ),
        params![project_id.trim(), idempotency_key.trim()],
        read_intent,
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn select_intent_by_id(
    conn: &rusqlite::Connection,
    intent_id: &str,
) -> Result<Option<SettlementIntent>> {
    conn.query_row(
        &format!("{} WHERE id = ?1", intent_select()),
        params![intent_id.trim()],
        read_intent,
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn select_settlement_by_intent(
    conn: &rusqlite::Connection,
    intent_id: &str,
) -> Result<Option<SettlementReceipt>> {
    conn.query_row(
        &format!("{} WHERE intent_id = ?1", settlement_select()),
        params![intent_id.trim()],
        read_settlement,
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn ensure_same_settlement(
    existing: &SettlementReceipt,
    input: &CreateSettlementReceipt<'_>,
) -> Result<()> {
    if existing.posting_key != input.posting_key.trim()
        || existing.status != input.status.trim()
        || existing.compute_amount_micros != input.compute_amount_micros
        || existing.provider_amount_micros != input.provider_amount_micros
        || existing.platform_amount_micros != input.platform_amount_micros
    {
        bail!("同一结算意图不能映射到不同影子凭证");
    }
    Ok(())
}
