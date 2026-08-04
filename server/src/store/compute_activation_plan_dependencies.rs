use anyhow::{anyhow, bail, Result};
use rusqlite::Connection;

use crate::{
    compute_federation::{
        capacity::ComputeCapacityPoolStatus, provider::PROVIDER_STATUS_REGISTERING,
    },
    compute_federation_activation_model::ACTIVATION_REQUEST_STATUS_APPROVED,
    compute_federation_activation_plan_model::ComputeActivationPlan,
};

use super::{
    compute_activation_plans::PrepareComputeActivationPlan,
    compute_activation_requests::request_on,
    compute_capacity_audit::stable_compute_capacity_pool_audit_digest,
    compute_capacity_pool_queries::current_capacity_pool_on,
    compute_provider_registry::current_registered_provider_on, Store,
};

pub(super) fn validate_prepare_dependencies_on(
    conn: &Connection,
    input: &PrepareComputeActivationPlan,
) -> Result<()> {
    let request =
        request_on(conn, input.request_id.trim())?.ok_or_else(|| anyhow!("激活证据申请不存在"))?;
    if request.status != ACTIVATION_REQUEST_STATUS_APPROVED
        || request.request_digest != input.expected_request_digest
        || request.provider_id != input.provider_id
        || request.pool_id != input.pool_id
        || request.expected_provider_policy_revision != input.expected_provider_policy_revision
        || request.expected_provider_digest != input.expected_provider_digest
        || request.expected_capacity_epoch != input.expected_capacity_epoch
        || request.expected_pool_revision != input.expected_pool_revision
        || request.expected_pool_digest != input.expected_pool_digest
    {
        bail!("激活证据申请状态、摘要或依赖版本已变化");
    }

    let provider = current_registered_provider_on(conn, input.provider_id.trim())?
        .ok_or_else(|| anyhow!("算力 Provider 不存在"))?;
    let next_revision = provider
        .provider
        .policy_revision
        .checked_add(1)
        .ok_or_else(|| anyhow!("算力 Provider policy revision 溢出"))?;
    if provider.provider.status != PROVIDER_STATUS_REGISTERING
        || provider.provider.policy_revision != input.expected_provider_policy_revision
        || provider.provider_digest != input.expected_provider_digest
        || input.target_provider.policy_revision != next_revision
        || input.target_provider.provider_id != provider.provider.provider_id
        || input.target_provider.provider_kind != provider.provider.provider_kind
        || input.target_provider.owner_account_id != provider.provider.owner_account_id
        || input.target_provider.created_at != provider.provider.created_at
    {
        bail!("算力 Provider 状态、身份或版本已变化");
    }

    let pool = current_capacity_pool_on(conn, input.pool_id.trim())?
        .ok_or_else(|| anyhow!("容量池不存在"))?;
    if pool.provider_id != input.provider_id
        || pool.status != ComputeCapacityPoolStatus::Registering
        || pool.binding.capacity_epoch != input.expected_capacity_epoch
        || pool.binding.pool_revision != input.expected_pool_revision
        || pool.binding.pool_digest != input.expected_pool_digest
    {
        bail!("容量池归属、状态或版本已变化");
    }
    let audit = Store::audit_compute_capacity_pool_epoch_on(
        conn,
        input.pool_id.trim(),
        input.expected_capacity_epoch,
    )?;
    if !audit.healthy
        || audit.current_capacity_epoch != input.expected_capacity_epoch
        || stable_compute_capacity_pool_audit_digest(&audit)? != request.ledger_audit_digest
    {
        bail!("容量池账本审计结果已变化");
    }
    Ok(())
}

pub(super) fn validate_saved_plan_dependencies_on(
    conn: &Connection,
    plan: &ComputeActivationPlan,
) -> Result<()> {
    validate_prepare_dependencies_on(
        conn,
        &PrepareComputeActivationPlan {
            request_id: plan.request_id.clone(),
            provider_id: plan.provider_id.clone(),
            pool_id: plan.pool_id.clone(),
            expected_request_digest: plan.expected_request_digest.clone(),
            expected_provider_policy_revision: plan.expected_provider_policy_revision,
            expected_provider_digest: plan.expected_provider_digest.clone(),
            expected_capacity_epoch: plan.expected_capacity_epoch,
            expected_pool_revision: plan.expected_pool_revision,
            expected_pool_digest: plan.expected_pool_digest.clone(),
            target_provider: plan.target_provider.clone(),
            idempotency_scope: String::new(),
            idempotency_key: String::new(),
            prepared_by_user_id: plan.prepared_by_user_id.clone(),
        },
    )
}
