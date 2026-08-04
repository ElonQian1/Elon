use anyhow::{bail, Result};
use rusqlite::{Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};

use super::{compute_attempt_settlement_challenges::ComputeSettlementChallengeGate, Store};

mod support;

use support::{
    normalize_release_request, persist_release_on, release_by_idempotency_on, release_by_lease_on,
    release_request_digest,
};

pub(crate) const COMPUTE_SETTLEMENT_RELEASE_SCHEMA: &str =
    "compute_federation.settlement_release.v1";
pub(crate) const COMPUTE_SETTLEMENT_RELEASE_POLICY_ID: &str = "settlement_pending_release_72h_v1";
pub(crate) const COMPUTE_SETTLEMENT_RELEASE_POLICY_VERSION: i64 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReleaseComputeAttemptSettlementRequest {
    pub lease_id: String,
    pub expected_settlement_receipt_id: String,
    pub expected_settlement_event_digest: String,
    pub expected_posting_id: String,
    pub expected_posting_digest: String,
    pub idempotency_key: String,
    pub released_by_user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeSettlementReleaseReceipt {
    pub schema: String,
    pub release_id: String,
    pub settlement_receipt_id: String,
    pub settlement_event_digest: String,
    pub source_posting_id: String,
    pub source_posting_digest: String,
    pub lease_id: String,
    pub consumer_account_id: String,
    pub provider_account_id: String,
    pub platform_account_id: String,
    pub currency: String,
    pub provider_released_micros: i64,
    pub platform_released_micros: i64,
    pub provider_pending_balance_after_micros: i64,
    pub provider_available_balance_after_micros: i64,
    pub provider_account_revision_after: i64,
    pub platform_pending_balance_after_micros: i64,
    pub platform_available_balance_after_micros: i64,
    pub platform_account_revision_after: i64,
    pub challenge_deadline: String,
    pub challenge_gate: ComputeSettlementChallengeGate,
    pub challenge_gate_digest: String,
    pub policy_id: String,
    pub policy_version: i64,
    pub release_posting_id: String,
    pub release_posting_digest: String,
    pub request_digest: String,
    pub event_digest: String,
    pub released_by_user_id: String,
    pub released_at: String,
    pub balance_effect: String,
    pub withdrawal_effect: String,
    pub replayed: bool,
}

impl Store {
    pub(crate) fn release_compute_attempt_settlement(
        &self,
        input: &ReleaseComputeAttemptSettlementRequest,
    ) -> Result<ComputeSettlementReleaseReceipt> {
        let input = normalize_release_request(input)?;
        let request_digest = release_request_digest(&input)?;
        let idempotency_scope = format!("compute_settlement_release:{}", input.released_by_user_id);
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) =
            release_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
        {
            if stored.request_digest != request_digest {
                bail!("相同待结算释放幂等键不能用于不同请求");
            }
            let receipt = stored.into_receipt(&tx, true)?;
            tx.commit()?;
            return Ok(receipt);
        }
        if let Some(stored) = release_by_lease_on(&tx, &input.lease_id)? {
            if stored.request_digest != request_digest {
                bail!("同一 Attempt Lease 已绑定另一份待结算释放回执");
            }
            let receipt = stored.into_receipt(&tx, true)?;
            tx.commit()?;
            return Ok(receipt);
        }

        let receipt = support::release_settlement_on(&tx, &input, &request_digest)?;
        persist_release_on(&tx, &input, &receipt, &idempotency_scope)?;
        let stored = release_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
            .ok_or_else(|| anyhow::anyhow!("待结算释放回执写入后不可见"))?;
        let receipt = stored.into_receipt(&tx, false)?;
        tx.commit()?;
        Ok(receipt)
    }

    pub(crate) fn compute_settlement_release(
        &self,
        lease_id: &str,
    ) -> Result<ComputeSettlementReleaseReceipt> {
        support::validate_exact("Attempt Lease ID", lease_id, 200)?;
        let conn = self.conn()?;
        compute_settlement_release_on(&*conn, lease_id)
    }
}

pub(super) fn compute_settlement_release_on(
    conn: &Connection,
    lease_id: &str,
) -> Result<ComputeSettlementReleaseReceipt> {
    support::validate_exact("Attempt Lease ID", lease_id, 200)?;
    let stored = release_by_lease_on(conn, lease_id)?
        .ok_or_else(|| anyhow::anyhow!("Attempt 结算尚未释放到 available"))?;
    stored.into_receipt(conn, false)
}

pub(super) fn compute_settlement_release_optional_on(
    conn: &Connection,
    lease_id: &str,
) -> Result<Option<ComputeSettlementReleaseReceipt>> {
    support::validate_exact("Attempt Lease ID", lease_id, 200)?;
    release_by_lease_on(conn, lease_id)?
        .map(|stored| stored.into_receipt(conn, false))
        .transpose()
}
