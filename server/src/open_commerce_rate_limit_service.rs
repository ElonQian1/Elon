use anyhow::{bail, Result};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{
    open_commerce_model::{normalize_app_id, OpenCommerceCapability, OpenCommerceMerchant},
    open_commerce_rate_limit_model::{
        validate_rate_limit_bounds, OpenCommerceRateLimitDecision, OpenCommerceRateLimitExceeded,
        OpenCommerceRateLimitPolicy, SetOpenCommerceRateLimitEnabledRequest,
        UpsertOpenCommerceRateLimitRequest,
    },
    project_auth::can_edit,
    store::Store,
};

#[allow(clippy::too_many_arguments)]
pub(crate) fn upsert_policy(
    store: &Store,
    project_id: &str,
    actor_user_id: &str,
    actor_app_id: &str,
    project_role: &str,
    request: UpsertOpenCommerceRateLimitRequest,
) -> Result<OpenCommerceRateLimitPolicy> {
    require_editor(project_role)?;
    validate_rate_limit_bounds(request.window_seconds, request.max_requests)?;
    let requester_app_id = request
        .requester_app_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(normalize_app_id)
        .transpose()?;
    let merchant = store.open_commerce_merchant(&request.merchant_id)?;
    if merchant.project_id != project_id.trim() {
        bail!("商户节点不属于当前项目");
    }
    let capability =
        store.open_commerce_capability_by_key(&merchant.id, &request.capability_key)?;
    let policy = store.upsert_open_commerce_rate_limit(
        project_id,
        &merchant.id,
        &capability.id,
        &capability.capability_key,
        requester_app_id.as_deref(),
        request.window_seconds,
        request.max_requests,
        request.enabled,
        actor_user_id,
    )?;
    store.record_open_commerce_audit(
        project_id,
        actor_user_id,
        Some(actor_app_id),
        "rate_limit.upserted",
        "rate_limit_policy",
        &policy.id,
        &json!({
            "merchant_id": policy.merchant_id,
            "capability_key": policy.capability_key,
            "requester_app_id": policy.requester_app_id,
            "window_seconds": policy.window_seconds,
            "max_requests": policy.max_requests,
            "status": policy.status
        }),
    )?;
    Ok(policy)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn set_policy_enabled(
    store: &Store,
    project_id: &str,
    policy_id: &str,
    actor_user_id: &str,
    actor_app_id: &str,
    project_role: &str,
    request: SetOpenCommerceRateLimitEnabledRequest,
) -> Result<OpenCommerceRateLimitPolicy> {
    require_editor(project_role)?;
    let policy =
        store.set_open_commerce_rate_limit_enabled(project_id, policy_id, request.enabled)?;
    store.record_open_commerce_audit(
        project_id,
        actor_user_id,
        Some(actor_app_id),
        "rate_limit.status_changed",
        "rate_limit_policy",
        &policy.id,
        &json!({
            "merchant_id": policy.merchant_id,
            "capability_key": policy.capability_key,
            "status": policy.status
        }),
    )?;
    Ok(policy)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn enforce_invocation(
    store: &Store,
    merchant: &OpenCommerceMerchant,
    capability: &OpenCommerceCapability,
    requester_user_id: &str,
    requester_app_id: &str,
    invocation_id: &str,
    bypass_for_project_editor: bool,
) -> Result<Option<OpenCommerceRateLimitDecision>> {
    if bypass_for_project_editor {
        return Ok(None);
    }
    let decision = store.claim_open_commerce_rate_limit(
        &merchant.project_id,
        &merchant.id,
        &capability.id,
        requester_app_id,
        &counter_subject(requester_app_id, requester_user_id),
    )?;
    let Some(decision) = decision else {
        return Ok(None);
    };
    if decision.allowed {
        return Ok(Some(decision));
    }

    let failed = store.finish_open_commerce_invocation_failure(invocation_id, "rate_limited")?;
    let retry_after_seconds = (decision.reset_at_unix - chrono::Utc::now().timestamp()).max(1);
    store.record_open_commerce_audit(
        &merchant.project_id,
        requester_user_id,
        Some(requester_app_id),
        "invocation.rate_limited",
        "invocation",
        &failed.id,
        &json!({
            "merchant_id": merchant.id,
            "capability_key": capability.capability_key,
            "policy_id": decision.policy_id,
            "max_requests": decision.max_requests,
            "window_seconds": decision.window_seconds,
            "retry_after_seconds": retry_after_seconds,
            "error_code": "rate_limited"
        }),
    )?;
    Err(OpenCommerceRateLimitExceeded {
        retry_after_seconds,
        max_requests: decision.max_requests,
        window_seconds: decision.window_seconds,
    }
    .into())
}

fn counter_subject(requester_app_id: &str, requester_user_id: &str) -> String {
    if matches!(requester_app_id, "pc-web" | "mcp-client") {
        let digest = hex::encode(Sha256::digest(requester_user_id.as_bytes()));
        format!("{requester_app_id}:{}", &digest[..16])
    } else {
        requester_app_id.to_string()
    }
}

fn require_editor(role: &str) -> Result<()> {
    if !can_edit(role) {
        bail!("当前调用方没有项目编辑权限");
    }
    Ok(())
}
