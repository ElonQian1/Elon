use anyhow::{bail, Result};
use rusqlite::{Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};

use super::{
    compute_attempt_settlement_challenges::{
        compute_settlement_challenge_on, ComputeSettlementChallengeReceipt,
    },
    Store,
};

pub(super) mod history;
mod pending_queue;
mod support;

use history::list_challenge_history_on;
pub(crate) use history::ComputeSettlementChallengeHistoryItem;
use pending_queue::list_open_challenge_lease_ids_on;
use support::{
    normalize_resolution_request, persist_resolution_on, resolution_by_challenge_on,
    resolution_by_idempotency_on, resolution_by_lease_on, resolution_request_digest,
};

pub(crate) const COMPUTE_SETTLEMENT_CHALLENGE_RESOLUTION_SCHEMA: &str =
    "compute_federation.settlement_challenge_resolution.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ResolveComputeSettlementChallengeRequest {
    pub lease_id: String,
    pub expected_challenge_id: String,
    pub expected_challenge_event_digest: String,
    pub action: String,
    pub statement: String,
    pub actor_user_id: String,
    pub actor_role: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeSettlementChallengeResolutionReceipt {
    pub schema: String,
    pub resolution_id: String,
    pub challenge_id: String,
    pub challenge_event_digest: String,
    pub settlement_receipt_id: String,
    pub settlement_event_digest: String,
    pub lease_id: String,
    pub consumer_account_id: String,
    pub provider_account_id: String,
    pub action: String,
    pub statement: String,
    pub actor_user_id: String,
    pub actor_role: String,
    pub request_digest: String,
    pub event_digest: String,
    pub resolved_at: String,
    pub challenge_status: String,
    pub correction_required: bool,
    pub balance_effect: String,
    pub settlement_release_effect: String,
    pub replayed: bool,
}

impl Store {
    pub(crate) fn resolve_compute_settlement_challenge(
        &self,
        input: &ResolveComputeSettlementChallengeRequest,
    ) -> Result<ComputeSettlementChallengeResolutionReceipt> {
        let input = normalize_resolution_request(input)?;
        let request_digest = resolution_request_digest(&input)?;
        let idempotency_scope = format!(
            "compute_settlement_challenge_resolution:{}:{}",
            input.actor_role, input.actor_user_id
        );
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) =
            resolution_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
        {
            if stored.request_digest != request_digest {
                bail!("相同算力结算挑战决议幂等键不能用于不同请求");
            }
            let receipt = stored.into_receipt(&tx, true)?;
            tx.commit()?;
            return Ok(receipt);
        }

        let receipt = support::resolve_challenge_on(&tx, &input, &request_digest)?;
        if let Some(stored) = resolution_by_challenge_on(&tx, &receipt.challenge_id)? {
            if stored.request_digest != request_digest {
                bail!("同一算力结算挑战已经存在另一份终态决议");
            }
            let receipt = stored.into_receipt(&tx, true)?;
            tx.commit()?;
            return Ok(receipt);
        }
        persist_resolution_on(&tx, &input, &receipt, &idempotency_scope)?;
        let stored = resolution_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
            .ok_or_else(|| anyhow::anyhow!("算力结算挑战决议写入后不可见"))?;
        let receipt = stored.into_receipt(&tx, false)?;
        tx.commit()?;
        Ok(receipt)
    }

    pub(crate) fn compute_settlement_challenge_resolution(
        &self,
        lease_id: &str,
    ) -> Result<ComputeSettlementChallengeResolutionReceipt> {
        support::validate_exact("Attempt Lease ID", lease_id, 200)?;
        let conn = self.conn()?;
        compute_settlement_challenge_resolution_on(&*conn, lease_id)
    }

    pub(crate) fn list_open_compute_settlement_challenges_for_consumer(
        &self,
        consumer_user_id: &str,
        limit: usize,
    ) -> Result<Vec<ComputeSettlementChallengeReceipt>> {
        support::validate_exact("消费者用户 ID", consumer_user_id, 240)?;
        self.list_open_compute_settlement_challenges(Some(consumer_user_id), limit)
    }

    pub(crate) fn list_open_compute_settlement_challenges_for_platform_admin(
        &self,
        limit: usize,
    ) -> Result<Vec<ComputeSettlementChallengeReceipt>> {
        self.list_open_compute_settlement_challenges(None, limit)
    }

    pub(crate) fn list_compute_settlement_challenge_history_for_consumer(
        &self,
        consumer_user_id: &str,
        limit: usize,
    ) -> Result<Vec<ComputeSettlementChallengeHistoryItem>> {
        support::validate_exact("消费者用户 ID", consumer_user_id, 240)?;
        self.list_compute_settlement_challenge_history(Some(consumer_user_id), None, limit)
    }

    pub(crate) fn list_compute_settlement_challenge_history_for_platform_admin(
        &self,
        limit: usize,
    ) -> Result<Vec<ComputeSettlementChallengeHistoryItem>> {
        self.list_compute_settlement_challenge_history(None, None, limit)
    }

    pub(crate) fn list_compute_settlement_challenge_history_for_provider(
        &self,
        provider_id: &str,
        limit: usize,
    ) -> Result<Vec<ComputeSettlementChallengeHistoryItem>> {
        support::validate_exact("Provider ID", provider_id, 240)?;
        self.list_compute_settlement_challenge_history(None, Some(provider_id), limit)
    }

    fn list_compute_settlement_challenge_history(
        &self,
        consumer_user_id: Option<&str>,
        provider_account_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ComputeSettlementChallengeHistoryItem>> {
        let conn = self.conn()?;
        list_challenge_history_on(
            &conn,
            consumer_user_id,
            provider_account_id,
            limit.clamp(1, 100),
        )
    }

    fn list_open_compute_settlement_challenges(
        &self,
        consumer_user_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ComputeSettlementChallengeReceipt>> {
        let conn = self.conn()?;
        list_open_challenge_lease_ids_on(&conn, consumer_user_id, limit.clamp(1, 100))?
            .into_iter()
            .map(|lease_id| {
                let challenge = compute_settlement_challenge_on(&conn, &lease_id)?;
                if settlement_challenge_resolution_by_challenge_on(&conn, &challenge.challenge_id)?
                    .is_some()
                {
                    bail!("待决议挑战队列返回了已有终态决议的挑战");
                }
                if let Some(expected_consumer_user_id) = consumer_user_id {
                    if challenge.consumer_account_id != expected_consumer_user_id {
                        bail!("待决议挑战队列返回了其他消费者的挑战");
                    }
                }
                Ok(challenge)
            })
            .collect()
    }
}

pub(super) fn compute_settlement_challenge_resolution_on(
    conn: &Connection,
    lease_id: &str,
) -> Result<ComputeSettlementChallengeResolutionReceipt> {
    support::validate_exact("Attempt Lease ID", lease_id, 200)?;
    let stored = resolution_by_lease_on(conn, lease_id)?
        .ok_or_else(|| anyhow::anyhow!("Attempt 结算挑战尚无终态决议"))?;
    stored.into_receipt(conn, false)
}

pub(super) fn settlement_challenge_resolution_by_challenge_on(
    conn: &Connection,
    challenge_id: &str,
) -> Result<Option<ComputeSettlementChallengeResolutionReceipt>> {
    support::validate_exact("结算挑战 ID", challenge_id, 240)?;
    resolution_by_challenge_on(conn, challenge_id)?
        .map(|stored| stored.into_receipt(conn, false))
        .transpose()
}
