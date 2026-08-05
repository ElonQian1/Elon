use anyhow::{bail, Result};
use serde::Deserialize;

use crate::{
    compute_federation_attempt_service::get_for_participant,
    store::{
        ComputeSettlementChallengeHistoryItem, ComputeSettlementChallengeReceipt,
        ComputeSettlementChallengeResolutionReceipt, ResolveComputeSettlementChallengeRequest,
        Store,
    },
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WithdrawComputeSettlementChallengeBody {
    pub expected_challenge_id: String,
    pub expected_challenge_event_digest: String,
    pub statement: String,
    pub idempotency_key: String,
    pub confirm_balances_unchanged: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ResolveComputeSettlementChallengeBody {
    pub expected_challenge_id: String,
    pub expected_challenge_event_digest: String,
    pub decision: String,
    pub statement: String,
    pub idempotency_key: String,
    pub confirm_no_money_movement: bool,
}

pub(crate) fn withdraw_for_consumer(
    store: &Store,
    consumer_user_id: &str,
    lease_id: &str,
    body: WithdrawComputeSettlementChallengeBody,
) -> Result<ComputeSettlementChallengeResolutionReceipt> {
    if !body.confirm_balances_unchanged {
        bail!("撤回结算挑战前必须确认该操作不会修改任何余额");
    }
    get_for_participant(store, consumer_user_id, lease_id)?;
    store.resolve_compute_settlement_challenge(&ResolveComputeSettlementChallengeRequest {
        lease_id: lease_id.to_string(),
        expected_challenge_id: body.expected_challenge_id,
        expected_challenge_event_digest: body.expected_challenge_event_digest,
        action: "withdrawn".to_string(),
        statement: body.statement,
        actor_user_id: consumer_user_id.to_string(),
        actor_role: "consumer".to_string(),
        idempotency_key: body.idempotency_key,
    })
}

pub(crate) fn resolve_for_platform_admin(
    store: &Store,
    admin_user_id: &str,
    lease_id: &str,
    body: ResolveComputeSettlementChallengeBody,
) -> Result<ComputeSettlementChallengeResolutionReceipt> {
    if !body.confirm_no_money_movement {
        bail!("裁决结算挑战前必须确认该操作不会退款、纠正或移动余额");
    }
    store.resolve_compute_settlement_challenge(&ResolveComputeSettlementChallengeRequest {
        lease_id: lease_id.to_string(),
        expected_challenge_id: body.expected_challenge_id,
        expected_challenge_event_digest: body.expected_challenge_event_digest,
        action: body.decision,
        statement: body.statement,
        actor_user_id: admin_user_id.to_string(),
        actor_role: "platform_admin".to_string(),
        idempotency_key: body.idempotency_key,
    })
}

pub(crate) fn get_for_attempt_participant(
    store: &Store,
    user_id: &str,
    lease_id: &str,
) -> Result<ComputeSettlementChallengeResolutionReceipt> {
    get_for_participant(store, user_id, lease_id)?;
    store.compute_settlement_challenge_resolution(lease_id)
}

pub(crate) fn get_for_platform_admin(
    store: &Store,
    lease_id: &str,
) -> Result<ComputeSettlementChallengeResolutionReceipt> {
    store.compute_settlement_challenge_resolution(lease_id)
}

pub(crate) fn list_open_for_consumer(
    store: &Store,
    consumer_user_id: &str,
    limit: usize,
) -> Result<Vec<ComputeSettlementChallengeReceipt>> {
    store.list_open_compute_settlement_challenges_for_consumer(consumer_user_id, limit)
}

pub(crate) fn list_open_for_platform_admin(
    store: &Store,
    limit: usize,
) -> Result<Vec<ComputeSettlementChallengeReceipt>> {
    store.list_open_compute_settlement_challenges_for_platform_admin(limit)
}

pub(crate) fn list_history_for_consumer(
    store: &Store,
    consumer_user_id: &str,
    limit: usize,
) -> Result<Vec<ComputeSettlementChallengeHistoryItem>> {
    store.list_compute_settlement_challenge_history_for_consumer(consumer_user_id, limit)
}

pub(crate) fn list_history_for_platform_admin(
    store: &Store,
    limit: usize,
) -> Result<Vec<ComputeSettlementChallengeHistoryItem>> {
    store.list_compute_settlement_challenge_history_for_platform_admin(limit)
}

pub(crate) fn list_history_for_provider_owner(
    store: &Store,
    user_id: &str,
    provider_id: &str,
    limit: usize,
) -> Result<Vec<ComputeSettlementChallengeHistoryItem>> {
    let provider = store.compute_provider(provider_id)?;
    if provider.provider.owner_account_id != user_id {
        bail!("算力 Provider 不属于当前登录用户");
    }
    store.list_compute_settlement_challenge_history_for_provider(provider_id, limit)
}
