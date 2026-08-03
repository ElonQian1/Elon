//! Open Commerce invocation authorization, execution, metering and audit pipeline.

use anyhow::{anyhow, bail, Result};
use chrono::Utc;
use serde_json::{json, Value};

use crate::{
    open_commerce_app_block_service,
    open_commerce_developer_credential_model::AuthenticatedDeveloperCredential,
    open_commerce_grant_budget_service,
    open_commerce_invocation_protocol::{invocation_response, request_digest, request_shape},
    open_commerce_model::{
        normalize_app_id, normalize_idempotency_key, InvokeCapabilityRequest,
        OpenCommerceCapability, OpenCommerceMerchant, ACCESS_AUTHORIZED, CAPABILITY_STATUS_ACTIVE,
        HANDLER_MERCHANT_RUNTIME, MERCHANT_STATUS_ACTIVE,
    },
    open_commerce_rate_limit_service,
    open_commerce_runtime_model::MerchantRuntimeEnvelope,
    open_commerce_service::{self, OpenCommerceActor},
    project_auth::can_edit,
    store::{OpenCommerceInvocationProvenance, OpenCommerceInvocationStart, Store},
};

pub(crate) async fn invoke(
    store: &Store,
    actor: &OpenCommerceActor<'_>,
    request: InvokeCapabilityRequest,
) -> Result<Value> {
    invoke_with_provenance(
        store,
        actor,
        request,
        None,
        None,
        OpenCommerceInvocationProvenance::platform(),
    )
    .await
}

pub(crate) async fn invoke_with_action_confirmation(
    store: &Store,
    actor: &OpenCommerceActor<'_>,
    request: InvokeCapabilityRequest,
    action_confirmation_id: Option<&str>,
) -> Result<Value> {
    invoke_with_provenance(
        store,
        actor,
        request,
        action_confirmation_id,
        None,
        OpenCommerceInvocationProvenance::platform(),
    )
    .await
}

pub(crate) async fn invoke_with_developer_credential(
    store: &Store,
    credential: &AuthenticatedDeveloperCredential,
    actor: &OpenCommerceActor<'_>,
    request: InvokeCapabilityRequest,
    action_confirmation_id: Option<&str>,
) -> Result<Value> {
    credential.ensure_scope(&request.capability_key)?;
    let provenance = OpenCommerceInvocationProvenance::developer(
        credential.environment,
        credential.credential_id.as_deref(),
    )?;
    invoke_with_provenance(
        store,
        actor,
        request,
        action_confirmation_id,
        Some(credential),
        provenance,
    )
    .await
}

async fn invoke_with_provenance(
    store: &Store,
    actor: &OpenCommerceActor<'_>,
    request: InvokeCapabilityRequest,
    action_confirmation_id: Option<&str>,
    developer_credential: Option<&AuthenticatedDeveloperCredential>,
    provenance: OpenCommerceInvocationProvenance<'_>,
) -> Result<Value> {
    let requester_app_id = normalize_app_id(&request.requester_app_id)?;
    if requester_app_id != normalize_app_id(actor.app_id)? {
        bail!("requester_app_id 与当前调用入口不一致");
    }
    let idempotency_key = normalize_idempotency_key(&request.idempotency_key)?;
    let input = crate::open_commerce_model::validate_json_object(&request.input, "调用输入")?;
    let merchant = store.open_commerce_merchant(&request.merchant_id)?;
    let capability =
        store.open_commerce_capability_by_key(&merchant.id, &request.capability_key)?;
    if merchant.status != MERCHANT_STATUS_ACTIVE {
        bail!("商户节点当前不可用");
    }
    if capability.status != CAPABILITY_STATUS_ACTIVE {
        bail!("商业能力当前不可用");
    }
    if let Some(credential) = developer_credential {
        credential.ensure_runtime_access(&capability.handler_type)?;
    }
    crate::open_commerce_capability_schema::validate_input(&capability.input_schema, &input)
        .map_err(anyhow::Error::new)?;
    let target_editor = actor.project_role.is_some_and(can_edit);
    let system_app = matches!(requester_app_id.as_str(), "pc-web" | "mcp-client");
    if !system_app {
        store.ensure_open_commerce_developer_app_owned_by_user(&requester_app_id, actor.user_id)?;
    }
    open_commerce_app_block_service::ensure_app_allowed(
        store,
        &merchant.id,
        &requester_app_id,
        target_editor,
    )?;
    if !target_editor && !store.open_commerce_directory_is_published(&merchant.id)? {
        bail!("商户节点未发布到开放目录");
    }
    if capability.access_level == ACCESS_AUTHORIZED && system_app && !target_editor {
        bail!("受限能力必须使用已注册且已认证的开发者应用身份");
    }
    let grant_id = open_commerce_service::authorize_invocation(
        store,
        actor,
        &merchant,
        &capability,
        request.grant_id.as_deref(),
    )?;
    let request_hash = request_digest(
        &merchant.id,
        &capability.capability_key,
        &requester_app_id,
        &input,
    )?;
    let request_shape = request_shape(&input)?;
    let invocation_start = OpenCommerceInvocationStart {
        project_id: &merchant.project_id,
        merchant_id: &merchant.id,
        capability_id: &capability.id,
        capability_key: &capability.capability_key,
        requester_user_id: actor.user_id,
        requester_app_id: &requester_app_id,
        grant_id: grant_id.as_deref(),
        idempotency_key: &idempotency_key,
        request_hash: &request_hash,
        request_shape: &request_shape,
        unit_price_micros: capability.unit_price_micros,
        currency: &capability.currency,
    };
    let claim = if capability.kind == "action" {
        let confirmation_id = action_confirmation_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow!("动作能力必须先完成服务端一次性确认"))?;
        store.start_confirmed_open_commerce_invocation_with_provenance(
            invocation_start,
            confirmation_id,
            provenance,
        )?
    } else {
        if action_confirmation_id.is_some() {
            bail!("查询能力不接受动作确认凭证");
        }
        store.start_open_commerce_invocation_with_provenance(invocation_start, provenance)?
    };
    if !claim.created {
        crate::open_commerce_capability_contract_service::validate_replayed_output(
            store,
            actor,
            &merchant,
            &capability,
            &requester_app_id,
            &claim.invocation,
        )?;
        return invocation_response(&claim.invocation, true);
    }

    open_commerce_rate_limit_service::enforce_invocation(
        store,
        &merchant,
        &capability,
        actor.user_id,
        &requester_app_id,
        &claim.invocation.id,
        target_editor,
    )?;
    open_commerce_grant_budget_service::enforce_invocation(
        store,
        &merchant,
        &capability,
        actor.user_id,
        &requester_app_id,
        &claim.invocation,
    )?;

    let result = match execute_handler(
        store,
        actor,
        &merchant,
        &capability,
        grant_id.as_deref(),
        &idempotency_key,
        &claim.invocation.id,
        &input,
        provenance,
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            crate::open_commerce_capability_contract_service::record_failed_invocation(
                store,
                actor,
                &merchant,
                &capability,
                &requester_app_id,
                &claim.invocation.id,
                "handler_failed",
                None,
            )?;
            return Err(error);
        }
    };
    if let Err(error) =
        crate::open_commerce_capability_schema::validate_output(&capability.output_schema, &result)
    {
        if capability.handler_type == HANDLER_MERCHANT_RUNTIME {
            let _ = store.mark_open_commerce_runtime_degraded(
                &merchant.id,
                "runtime_output_schema_violation",
            );
        }
        crate::open_commerce_capability_contract_service::record_failed_invocation(
            store,
            actor,
            &merchant,
            &capability,
            &requester_app_id,
            &claim.invocation.id,
            "output_schema_violation",
            Some((&error.path, error.code)),
        )?;
        return Err(anyhow::Error::new(error));
    }
    let invocation =
        store.finish_open_commerce_invocation_success(&claim.invocation.id, &result)?;
    store.record_open_commerce_audit(
        &merchant.project_id,
        actor.user_id,
        Some(&requester_app_id),
        "invocation.succeeded",
        "invocation",
        &invocation.id,
        &json!({
            "merchant_id": merchant.id,
            "capability_key": capability.capability_key,
            "grant_id": grant_id,
            "amount_micros": invocation.amount_micros,
            "settlement_status": invocation.settlement_status,
            "credential_environment": invocation.credential_environment,
            "credential_id": invocation.credential_id
        }),
    )?;
    if let Err(error) = crate::task_settlement::capture_commerce_invocation(
        store,
        &invocation,
        &merchant.owner_user_id,
    ) {
        tracing::warn!(
            project_id = merchant.project_id,
            invocation_id = invocation.id,
            error = %error,
            "failed to capture optional open-commerce shadow usage"
        );
    }
    invocation_response(&invocation, false)
}

async fn execute_handler(
    store: &Store,
    actor: &OpenCommerceActor<'_>,
    merchant: &OpenCommerceMerchant,
    capability: &OpenCommerceCapability,
    grant_id: Option<&str>,
    idempotency_key: &str,
    invocation_id: &str,
    input: &Value,
    provenance: OpenCommerceInvocationProvenance<'_>,
) -> Result<Value> {
    if capability.handler_type != HANDLER_MERCHANT_RUNTIME {
        return open_commerce_service::execute_first_party_handler(merchant, capability, input);
    }
    let binding = store.active_open_commerce_runtime_binding(&merchant.id)?;
    let envelope = MerchantRuntimeEnvelope {
        schema: "merchant_runtime.invoke.v1",
        invocation_id: invocation_id.to_string(),
        merchant_id: merchant.id.clone(),
        capability_key: capability.capability_key.clone(),
        requester_user_id: actor.user_id.to_string(),
        requester_app_id: actor.app_id.to_string(),
        credential_environment: provenance.environment.to_string(),
        credential_id: provenance.credential_id.map(str::to_string),
        grant_id: grant_id.map(str::to_string),
        idempotency_key: idempotency_key.to_string(),
        issued_at_unix: Utc::now().timestamp(),
        input: input.clone(),
    };
    match crate::open_commerce_runtime_client::invoke_runtime(&binding, &envelope).await {
        Ok(result) => Ok(result),
        Err(error) => {
            if error.degrades_binding() {
                let _ = store
                    .mark_open_commerce_runtime_degraded(&merchant.id, "runtime_invocation_failed");
            }
            Err(error.into())
        }
    }
}
