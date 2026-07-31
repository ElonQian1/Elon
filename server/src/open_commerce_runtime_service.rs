//! Project management and verification for merchant runtime bindings.

use anyhow::{bail, Result};
use chrono::Utc;
use serde_json::{json, Value};

use crate::{
    open_commerce_runtime_client::invoke_runtime,
    open_commerce_runtime_model::{
        MerchantRuntimeEnvelope, OpenCommerceRuntimeBinding, UpsertRuntimeBindingRequest,
    },
    open_commerce_service::OpenCommerceActor,
    project_auth::can_edit,
    store::Store,
};

pub(crate) fn upsert_binding(
    store: &Store,
    project_id: &str,
    merchant_id: &str,
    actor: &OpenCommerceActor<'_>,
    request: UpsertRuntimeBindingRequest,
) -> Result<OpenCommerceRuntimeBinding> {
    require_editor(actor)?;
    let binding = store.upsert_open_commerce_runtime_binding(
        project_id,
        merchant_id,
        actor.user_id,
        request,
    )?;
    store.record_open_commerce_audit(
        project_id,
        actor.user_id,
        Some(actor.app_id),
        "runtime.configured",
        "runtime_binding",
        &binding.id,
        &json!({
            "merchant_id": merchant_id,
            "endpoint_host": reqwest::Url::parse(&binding.endpoint_base_url)
                .ok().and_then(|url| url.host_str().map(str::to_string)),
            "credential_ref": binding.credential_ref,
            "secret_value_recorded": false
        }),
    )?;
    Ok(binding)
}

pub(crate) async fn verify_binding(
    store: &Store,
    project_id: &str,
    merchant_id: &str,
    actor: &OpenCommerceActor<'_>,
) -> Result<OpenCommerceRuntimeBinding> {
    require_editor(actor)?;
    store.open_commerce_merchant_for_project(project_id, merchant_id)?;
    let binding = store.open_commerce_runtime_binding(merchant_id)?;
    let invocation_id = format!("verify-{}", uuid::Uuid::new_v4().simple());
    let envelope = MerchantRuntimeEnvelope {
        schema: "merchant_runtime.invoke.v1",
        invocation_id,
        merchant_id: merchant_id.to_string(),
        capability_key: "system.health".to_string(),
        requester_user_id: actor.user_id.to_string(),
        requester_app_id: actor.app_id.to_string(),
        grant_id: None,
        idempotency_key: format!("verify-{}", uuid::Uuid::new_v4().simple()),
        issued_at_unix: Utc::now().timestamp(),
        input: json!({}),
    };
    let result = match invoke_runtime(&binding, &envelope).await {
        Ok(result) => result,
        Err(error) => {
            let _ = store.mark_open_commerce_runtime_degraded(merchant_id, "verification_failed");
            return Err(error.into());
        }
    };
    if result.get("merchant_id").and_then(Value::as_str) != Some(merchant_id)
        || result.get("status").and_then(Value::as_str) != Some("ok")
    {
        let _ = store.mark_open_commerce_runtime_degraded(merchant_id, "identity_mismatch");
        bail!("商户运行健康响应身份不匹配");
    }
    let manifest_sha256 = result.get("manifest_sha256").and_then(Value::as_str);
    if let Some(expected) = binding.manifest_sha256.as_deref() {
        if manifest_sha256 != Some(expected) {
            let _ = store.mark_open_commerce_runtime_degraded(merchant_id, "manifest_mismatch");
            bail!("商户运行能力清单摘要不匹配");
        }
    }
    let verified = store.mark_open_commerce_runtime_verified(merchant_id, manifest_sha256)?;
    store.record_open_commerce_audit(
        project_id,
        actor.user_id,
        Some(actor.app_id),
        "runtime.verified",
        "runtime_binding",
        &verified.id,
        &json!({
            "merchant_id": merchant_id,
            "manifest_sha256": verified.manifest_sha256,
            "status": verified.status
        }),
    )?;
    Ok(verified)
}

fn require_editor(actor: &OpenCommerceActor<'_>) -> Result<()> {
    if actor.project_role.is_some_and(can_edit) {
        Ok(())
    } else {
        bail!("当前调用方没有项目编辑权限")
    }
}
