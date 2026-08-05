use anyhow::{bail, Result};
use rusqlite::{params, Connection};
use serde::Serialize;

use super::super::{
    compute_attempt_settlement_challenges::{
        compute_settlement_challenge_on, ComputeSettlementChallengeReceipt,
    },
    compute_attempt_settlement_corrections::{
        compute_settlement_correction_by_resolution_on, ComputeSettlementCorrectionReceipt,
    },
    compute_attempt_settlement_releases::{
        compute_settlement_release_optional_on, ComputeSettlementReleaseReceipt,
    },
    compute_attempt_settlements::{compute_attempt_settlement_on, ComputeAttemptSettlementReceipt},
};
use super::{
    settlement_challenge_resolution_by_challenge_on, ComputeSettlementChallengeResolutionReceipt,
};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeSettlementChallengeHistoryItem {
    pub settlement: ComputeAttemptSettlementReceipt,
    pub challenge: ComputeSettlementChallengeReceipt,
    pub resolution: Option<ComputeSettlementChallengeResolutionReceipt>,
    pub correction: Option<ComputeSettlementCorrectionReceipt>,
    pub release: Option<ComputeSettlementReleaseReceipt>,
    pub lifecycle_status: String,
    pub balance_status: String,
    pub external_payment_effect: &'static str,
}

pub(super) fn list_challenge_history_on(
    conn: &Connection,
    consumer_user_id: Option<&str>,
    provider_account_id: Option<&str>,
    limit: usize,
) -> Result<Vec<ComputeSettlementChallengeHistoryItem>> {
    let mut statement = conn.prepare(
        "SELECT challenge.lease_id
           FROM compute_settlement_challenges challenge
          WHERE (?1 IS NULL OR challenge.consumer_account_id=?1)
            AND (?2 IS NULL OR challenge.provider_account_id=?2)
          ORDER BY challenge.opened_at DESC,
                   challenge.challenge_id DESC
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
            build_history_item_on(conn, consumer_user_id, provider_account_id, &lease_id)
        })
        .collect()
}

pub(in crate::store) fn build_history_item_on(
    conn: &Connection,
    expected_consumer_user_id: Option<&str>,
    expected_provider_account_id: Option<&str>,
    lease_id: &str,
) -> Result<ComputeSettlementChallengeHistoryItem> {
    let settlement = compute_attempt_settlement_on(conn, lease_id)?;
    let challenge = compute_settlement_challenge_on(conn, lease_id)?;
    if challenge.settlement_receipt_id != settlement.settlement.settlement_receipt_id
        || challenge.settlement_event_digest != settlement.event_digest
        || challenge.consumer_account_id != settlement.settlement.consumer_account_id
        || challenge.provider_account_id != settlement.settlement.provider_account_id
    {
        bail!("结算挑战历史中的 Settlement 与 Challenge 引用不一致");
    }
    if let Some(expected) = expected_consumer_user_id {
        if challenge.consumer_account_id != expected {
            bail!("结算挑战历史返回了其他消费者的记录");
        }
    }
    if let Some(expected) = expected_provider_account_id {
        if challenge.provider_account_id != expected {
            bail!("结算挑战历史返回了其他 Provider 的记录");
        }
    }

    let resolution =
        settlement_challenge_resolution_by_challenge_on(conn, &challenge.challenge_id)?;
    let correction = resolution
        .as_ref()
        .map(|item| compute_settlement_correction_by_resolution_on(conn, &item.resolution_id))
        .transpose()?
        .flatten();
    let release = compute_settlement_release_optional_on(conn, lease_id)?;
    validate_descendants(
        &challenge,
        resolution.as_ref(),
        correction.as_ref(),
        release.as_ref(),
    )?;
    let (lifecycle_status, balance_status) =
        derive_status(resolution.as_ref(), correction.as_ref(), release.as_ref())?;

    Ok(ComputeSettlementChallengeHistoryItem {
        settlement,
        challenge,
        resolution,
        correction,
        release,
        lifecycle_status: lifecycle_status.to_string(),
        balance_status: balance_status.to_string(),
        external_payment_effect: "not_proven_by_settlement_challenge_history",
    })
}

fn validate_descendants(
    challenge: &ComputeSettlementChallengeReceipt,
    resolution: Option<&ComputeSettlementChallengeResolutionReceipt>,
    correction: Option<&ComputeSettlementCorrectionReceipt>,
    release: Option<&ComputeSettlementReleaseReceipt>,
) -> Result<()> {
    if let Some(item) = resolution {
        if item.challenge_id != challenge.challenge_id
            || item.challenge_event_digest != challenge.event_digest
            || item.lease_id != challenge.lease_id
        {
            bail!("结算挑战历史中的 Resolution 引用不一致");
        }
    }
    if correction.is_some() && resolution.map(|item| item.action.as_str()) != Some("accepted") {
        bail!("非 accepted 挑战不能包含 Correction Receipt");
    }
    if let Some(item) = correction {
        let expected_resolution =
            resolution.ok_or_else(|| anyhow::anyhow!("Correction 缺少上游 Resolution"))?;
        if item.challenge_id != challenge.challenge_id
            || item.resolution_id != expected_resolution.resolution_id
            || item.lease_id != challenge.lease_id
        {
            bail!("结算挑战历史中的 Correction 引用不一致");
        }
    }
    if let Some(item) = release {
        if item.settlement_receipt_id != challenge.settlement_receipt_id
            || item.settlement_event_digest != challenge.settlement_event_digest
            || item.lease_id != challenge.lease_id
        {
            bail!("结算挑战历史中的 Release 引用不一致");
        }
    }
    Ok(())
}

fn derive_status(
    resolution: Option<&ComputeSettlementChallengeResolutionReceipt>,
    correction: Option<&ComputeSettlementCorrectionReceipt>,
    release: Option<&ComputeSettlementReleaseReceipt>,
) -> Result<(&'static str, &'static str)> {
    match (
        resolution.map(|item| item.action.as_str()),
        correction,
        release,
    ) {
        (None, None, None) => Ok(("open", "pending_blocked")),
        (Some("withdrawn"), None, None) => Ok(("withdrawn", "release_pending")),
        (Some("rejected"), None, None) => Ok(("rejected", "release_pending")),
        (Some("accepted"), None, None) => Ok(("accepted_pending_correction", "pending_blocked")),
        (Some("accepted"), Some(_), None) => Ok(("accepted_corrected", "corrected_pending")),
        (Some("withdrawn"), None, Some(_)) => Ok(("withdrawn_released", "available")),
        (Some("rejected"), None, Some(_)) => Ok(("rejected_released", "available")),
        (Some("accepted"), Some(_), Some(_)) => {
            Ok(("accepted_corrected_released", "corrected_available"))
        }
        _ => bail!("结算挑战历史包含不允许的 Resolution/Correction/Release 组合"),
    }
}
