use anyhow::{bail, Result};
use rusqlite::{params, Connection};
use serde::Serialize;

use super::super::{
    compute_attempt_settlement_challenge_resolutions::{
        history::build_history_item_on, ComputeSettlementChallengeResolutionReceipt,
    },
    compute_attempt_settlement_challenges::{
        compute_settlement_challenge_optional_on, ComputeSettlementChallengeReceipt,
    },
    compute_attempt_settlement_corrections::ComputeSettlementCorrectionReceipt,
    compute_attempt_settlement_releases::{
        compute_settlement_release_optional_on, ComputeSettlementReleaseReceipt,
    },
};
use super::{compute_attempt_settlement_on, ComputeAttemptSettlementReceipt};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeSettlementLifecycleHistoryItem {
    pub settlement: ComputeAttemptSettlementReceipt,
    pub challenge: Option<ComputeSettlementChallengeReceipt>,
    pub resolution: Option<ComputeSettlementChallengeResolutionReceipt>,
    pub correction: Option<ComputeSettlementCorrectionReceipt>,
    pub release: Option<ComputeSettlementReleaseReceipt>,
    pub lifecycle_status: String,
    pub balance_status: String,
    pub external_payment_effect: &'static str,
}

pub(super) fn list_settlement_lifecycle_history_on(
    conn: &Connection,
    consumer_user_id: Option<&str>,
    provider_account_id: Option<&str>,
    limit: usize,
) -> Result<Vec<ComputeSettlementLifecycleHistoryItem>> {
    let mut statement = conn.prepare(
        "SELECT settlement.lease_id
           FROM compute_attempt_settlements settlement
           JOIN compute_settlement_postings posting
             ON posting.settlement_receipt_id=settlement.settlement_receipt_id
           JOIN compute_settlement_ledger_legs consumer_leg
             ON consumer_leg.posting_id=posting.posting_id
            AND consumer_leg.leg_kind='consumer_capture'
           JOIN compute_settlement_ledger_legs provider_leg
             ON provider_leg.posting_id=posting.posting_id
            AND provider_leg.leg_kind='provider_pending'
          WHERE (?1 IS NULL OR consumer_leg.account_id=?1)
            AND (?2 IS NULL OR provider_leg.account_id=?2)
          ORDER BY settlement.settled_at DESC,
                   settlement.settlement_receipt_id DESC
          LIMIT ?3",
    )?;
    let lease_ids = statement
        .query_map(
            params![consumer_user_id, provider_account_id, limit as i64],
            |row| row.get(0),
        )?
        .collect::<rusqlite::Result<Vec<String>>>()?;

    lease_ids
        .into_iter()
        .map(|lease_id| {
            build_lifecycle_item_on(conn, consumer_user_id, provider_account_id, &lease_id)
        })
        .collect()
}

fn build_lifecycle_item_on(
    conn: &Connection,
    expected_consumer_user_id: Option<&str>,
    expected_provider_account_id: Option<&str>,
    lease_id: &str,
) -> Result<ComputeSettlementLifecycleHistoryItem> {
    let settlement = compute_attempt_settlement_on(conn, lease_id)?;
    validate_scope(
        &settlement,
        expected_consumer_user_id,
        expected_provider_account_id,
    )?;

    if compute_settlement_challenge_optional_on(conn, lease_id)?.is_some() {
        let challenged = build_history_item_on(
            conn,
            expected_consumer_user_id,
            expected_provider_account_id,
            lease_id,
        )?;
        return Ok(ComputeSettlementLifecycleHistoryItem {
            settlement: challenged.settlement,
            challenge: Some(challenged.challenge),
            resolution: challenged.resolution,
            correction: challenged.correction,
            release: challenged.release,
            lifecycle_status: challenged.lifecycle_status,
            balance_status: challenged.balance_status,
            external_payment_effect: "not_proven_by_settlement_lifecycle_history",
        });
    }

    let release = compute_settlement_release_optional_on(conn, lease_id)?;
    if let Some(item) = release.as_ref() {
        if item.settlement_receipt_id != settlement.settlement.settlement_receipt_id
            || item.settlement_event_digest != settlement.event_digest
            || item.lease_id != settlement.lease_id
        {
            bail!("结算生命周期历史中的 Release 引用不一致");
        }
    }
    let (lifecycle_status, balance_status) = if release.is_some() {
        ("unchallenged_released", "available")
    } else {
        ("unchallenged_pending", "pending")
    };
    Ok(ComputeSettlementLifecycleHistoryItem {
        settlement,
        challenge: None,
        resolution: None,
        correction: None,
        release,
        lifecycle_status: lifecycle_status.to_string(),
        balance_status: balance_status.to_string(),
        external_payment_effect: "not_proven_by_settlement_lifecycle_history",
    })
}

fn validate_scope(
    settlement: &ComputeAttemptSettlementReceipt,
    expected_consumer_user_id: Option<&str>,
    expected_provider_account_id: Option<&str>,
) -> Result<()> {
    if let Some(expected) = expected_consumer_user_id {
        if settlement.settlement.consumer_account_id != expected {
            bail!("结算生命周期历史返回了其他消费者的记录");
        }
    }
    if let Some(expected) = expected_provider_account_id {
        if settlement.settlement.provider_account_id != expected {
            bail!("结算生命周期历史返回了其他 Provider 的记录");
        }
    }
    Ok(())
}
