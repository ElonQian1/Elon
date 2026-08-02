use anyhow::{bail, Result};
use chrono::{Duration, Utc};
use serde_json::json;

use crate::{
    open_commerce_action_confirmation_model::{
        OpenCommerceActionConfirmation, ACTION_CONFIRMATION_PHRASE, ACTION_CONFIRMATION_TTL_SECONDS,
    },
    open_commerce_app_block_service,
    open_commerce_invocation_protocol::{request_digest, request_shape},
    open_commerce_model::{
        normalize_app_id, normalize_idempotency_key, validate_json_object, InvokeCapabilityRequest,
        ACCESS_AUTHORIZED, CAPABILITY_STATUS_ACTIVE, MERCHANT_STATUS_ACTIVE,
    },
    open_commerce_service::{self, OpenCommerceActor},
    project_auth::can_edit,
    store::{CreateOpenCommerceActionConfirmation, Store},
};

pub(crate) fn prepare(
    store: &Store,
    actor: &OpenCommerceActor<'_>,
    request: InvokeCapabilityRequest,
) -> Result<OpenCommerceActionConfirmation> {
    let requester_app_id = normalize_app_id(&request.requester_app_id)?;
    if requester_app_id != normalize_app_id(actor.app_id)? {
        bail!("requester_app_id 与当前调用入口不一致");
    }
    let idempotency_key = normalize_idempotency_key(&request.idempotency_key)?;
    let input = validate_json_object(&request.input, "调用输入")?;
    let merchant = store.open_commerce_merchant(&request.merchant_id)?;
    let capability =
        store.open_commerce_capability_by_key(&merchant.id, &request.capability_key)?;
    if merchant.status != MERCHANT_STATUS_ACTIVE {
        bail!("商户节点当前不可用");
    }
    if capability.status != CAPABILITY_STATUS_ACTIVE {
        bail!("商业能力当前不可用");
    }
    if capability.kind != "action" {
        bail!("查询能力不需要动作确认");
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
    let safe_shape = request_shape(&input)?;
    let expires_at = (Utc::now() + Duration::seconds(ACTION_CONFIRMATION_TTL_SECONDS)).to_rfc3339();
    let confirmation =
        store.create_open_commerce_action_confirmation(CreateOpenCommerceActionConfirmation {
            project_id: &merchant.project_id,
            merchant_id: &merchant.id,
            capability_id: &capability.id,
            capability_key: &capability.capability_key,
            requester_user_id: actor.user_id,
            requester_app_id: &requester_app_id,
            grant_id: grant_id.as_deref(),
            idempotency_key: &idempotency_key,
            request_hash: &request_hash,
            request_shape: &safe_shape,
            expires_at: &expires_at,
        })?;
    store.record_open_commerce_audit(
        &merchant.project_id,
        actor.user_id,
        Some(&requester_app_id),
        "action_confirmation.prepared",
        "action_confirmation",
        &confirmation.id,
        &json!({
            "merchant_id": merchant.id,
            "capability_key": capability.capability_key,
            "idempotency_key": idempotency_key,
            "request_hash": request_hash,
            "expires_at": confirmation.expires_at,
            "status": confirmation.status,
            "contains_raw_values": false
        }),
    )?;
    Ok(confirmation)
}

pub(crate) fn confirm(
    store: &Store,
    actor: &OpenCommerceActor<'_>,
    confirmation_id: &str,
    confirmation_phrase: &str,
) -> Result<OpenCommerceActionConfirmation> {
    if confirmation_phrase.trim() != ACTION_CONFIRMATION_PHRASE {
        bail!("动作确认短语无效");
    }
    let app_id = normalize_app_id(actor.app_id)?;
    let confirmation =
        store.confirm_open_commerce_action_confirmation(confirmation_id, actor.user_id, &app_id)?;
    store.record_open_commerce_audit(
        &confirmation.project_id,
        actor.user_id,
        Some(&app_id),
        "action_confirmation.confirmed",
        "action_confirmation",
        &confirmation.id,
        &json!({
            "merchant_id": confirmation.merchant_id,
            "capability_key": confirmation.capability_key,
            "idempotency_key": confirmation.idempotency_key,
            "request_hash": confirmation.request_hash,
            "status": confirmation.status,
            "contains_raw_values": false
        }),
    )?;
    Ok(confirmation)
}
