//! MCP management surface for owner-declared external-pool onboarding.

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::store::Store;

use super::{
    external_pool_onboarding_service::{
        self as service, ApplyExternalPoolOnboardingBody, CancelExternalPoolOnboardingBody,
        ReviewExternalPoolOnboardingBody, SubmitExternalPoolOnboardingBody,
    },
    management_mcp_support as support,
};

const SUBMIT: &str = "compute_submit_my_external_pool_onboarding";
const LIST: &str = "compute_list_my_external_pool_onboarding_requests";
const GET: &str = "compute_get_my_external_pool_onboarding_request";
const CANCEL: &str = "compute_cancel_my_external_pool_onboarding_request";
const PREFLIGHT: &str = "compute_preflight_my_external_pool_onboarding_request";
const ADMIN_LIST: &str = "compute_admin_list_external_pool_onboarding_requests";
const ADMIN_GET: &str = "compute_admin_get_external_pool_onboarding_request";
const ADMIN_PREFLIGHT: &str = "compute_admin_preflight_external_pool_onboarding_request";
const ADMIN_REVIEW: &str = "compute_admin_review_external_pool_onboarding_request";
const ADMIN_APPLY: &str = "compute_admin_apply_external_pool_onboarding_request";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EntityArguments {
    request_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListArguments {
    status: Option<String>,
    #[serde(default = "support::default_limit")]
    limit: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CancelArguments {
    request_id: String,
    request: CancelExternalPoolOnboardingBody,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewArguments {
    request_id: String,
    request: ReviewExternalPoolOnboardingBody,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ApplyArguments {
    request_id: String,
    request: ApplyExternalPoolOnboardingBody,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        support::tool(
            SUBMIT,
            "提交当前用户的外部算力池元数据接入申请。只登记自声明材料，不验证凭据、下载 Adapter 或授予 v213 路由权限；必须显式确认。",
            submit_schema(),
            false,
            false,
        ),
        support::tool(LIST, "列出当前用户的外部算力池接入申请。", support::list_schema(), true, false),
        support::tool(GET, "读取当前用户的一份外部算力池接入申请及其复核、应用回执。", entity_schema(), true, false),
        support::tool(CANCEL, "取消当前用户仍处于 submitted 状态的外部算力池接入申请；必须显式确认。", cancel_schema(), false, true),
        support::tool(PREFLIGHT, "检查当前用户外部算力池接入申请的下一步允许操作和阻塞项，不改变状态。", entity_schema(), true, false),
    ]
}

pub(crate) fn admin_definitions() -> Vec<Value> {
    vec![
        support::tool(ADMIN_LIST, "平台管理员列出外部算力池接入申请。", support::list_schema(), true, false),
        support::tool(ADMIN_GET, "平台管理员读取外部算力池接入申请的完整治理回执。", entity_schema(), true, false),
        support::tool(ADMIN_PREFLIGHT, "平台管理员检查外部算力池接入申请的复核和应用条件，不改变状态。", entity_schema(), true, false),
        support::tool(ADMIN_REVIEW, "平台管理员独立复核外部算力池接入申请；必须显式确认。", review_schema(), false, false),
        support::tool(ADMIN_APPLY, "平台管理员应用已批准的接入申请并创建 registering Provider。该操作仍不授予 Adapter、凭据或路由执行权限；必须显式确认。", apply_schema(), false, false),
    ]
}

pub(crate) fn call_if_handled(
    store: &Store,
    user_id: &str,
    name: &str,
    arguments: Value,
) -> Result<Option<Value>> {
    let value = match name {
        SUBMIT => serde_json::to_value(service::submit_for_owner(
            store,
            user_id,
            support::decode::<SubmitExternalPoolOnboardingBody>(arguments, name)?,
        )?)?,
        LIST => {
            let input: ListArguments = support::decode(arguments, name)?;
            json!({"onboarding_requests":service::list_for_owner(
                store, user_id, input.status.as_deref(), input.limit
            )?})
        }
        GET => {
            let input: EntityArguments = support::decode(arguments, name)?;
            serde_json::to_value(service::get_for_owner(store, user_id, &input.request_id)?)?
        }
        CANCEL => {
            let input: CancelArguments = support::decode(arguments, name)?;
            serde_json::to_value(service::cancel_for_owner(
                store,
                user_id,
                &input.request_id,
                input.request,
            )?)?
        }
        PREFLIGHT => {
            let input: EntityArguments = support::decode(arguments, name)?;
            serde_json::to_value(service::preflight_for_owner(
                store,
                user_id,
                &input.request_id,
            )?)?
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}

pub(crate) fn call_admin_if_handled(
    store: &Store,
    user_id: &str,
    platform_role: &str,
    name: &str,
    arguments: Value,
) -> Result<Option<Value>> {
    let value = match name {
        ADMIN_LIST => {
            support::ensure_platform_admin(platform_role)?;
            let input: ListArguments = support::decode(arguments, name)?;
            json!({"onboarding_requests":service::list_for_admin(
                store, input.status.as_deref(), input.limit
            )?})
        }
        ADMIN_GET => {
            support::ensure_platform_admin(platform_role)?;
            let input: EntityArguments = support::decode(arguments, name)?;
            serde_json::to_value(service::get_for_admin(store, &input.request_id)?)?
        }
        ADMIN_PREFLIGHT => {
            support::ensure_platform_admin(platform_role)?;
            let input: EntityArguments = support::decode(arguments, name)?;
            serde_json::to_value(service::preflight_for_admin(store, &input.request_id)?)?
        }
        ADMIN_REVIEW => {
            support::ensure_platform_admin(platform_role)?;
            let input: ReviewArguments = support::decode(arguments, name)?;
            serde_json::to_value(service::review_for_admin(
                store,
                user_id,
                &input.request_id,
                input.request,
            )?)?
        }
        ADMIN_APPLY => {
            support::ensure_platform_admin(platform_role)?;
            let input: ApplyArguments = support::decode(arguments, name)?;
            serde_json::to_value(service::apply_for_admin(
                store,
                user_id,
                &input.request_id,
                input.request,
            )?)?
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn entity_schema() -> Value {
    support::entity_schema("request_id", 200)
}

fn submit_schema() -> Value {
    json!({
        "type":"object",
        "required":[
            "request_id","idempotency_key","submitted_at","provider_id","display_name",
            "home_region","task_kinds","accelerator_kinds","regions","allowed_data_classes",
            "supports_streaming","supports_checkpointing","adapter_intent","credential_intent",
            "confirm_submission"
        ],
        "properties":{
            "request_id":support::bounded_string(200),
            "idempotency_key":support::bounded_string(200),
            "submitted_at":support::bounded_string(64),
            "provider_id":support::bounded_string(160),
            "display_name":support::bounded_string(200),
            "home_region":support::bounded_string(100),
            "task_kinds":string_array(100),
            "accelerator_kinds":string_array(100),
            "regions":string_array(100),
            "allowed_data_classes":string_array(100),
            "supports_streaming":{"type":"boolean"},
            "supports_checkpointing":{"type":"boolean"},
            "declared_hardware_digest":nullable_string(256),
            "adapter_intent":{
                "type":"object",
                "required":["expected_adapter_id","expected_release_version","expected_config_revision","expected_config_digest"],
                "properties":{
                    "expected_adapter_id":support::bounded_string(160),
                    "expected_release_version":support::bounded_string(100),
                    "expected_config_revision":{"type":"integer","minimum":1},
                    "expected_config_digest":support::bounded_string(256)
                },
                "additionalProperties":false
            },
            "credential_intent":{
                "type":"object",
                "properties":{
                    "non_bearer_credential_ref":nullable_string(500),
                    "credential_hint":nullable_string(500)
                },
                "additionalProperties":false
            },
            "external_evidence_ref":nullable_string(500),
            "external_evidence_sha256":nullable_string(256),
            "owner_note":{"type":"string","maxLength":2000,"default":""},
            "confirm_submission":{"type":"boolean","const":true}
        },
        "additionalProperties":false
    })
}

fn cancel_schema() -> Value {
    wrapped_request_schema(json!({
        "type":"object",
        "required":["expected_request_digest","confirm_cancel"],
        "properties":{
            "expected_request_digest":support::bounded_string(256),
            "confirm_cancel":{"type":"boolean","const":true}
        },
        "additionalProperties":false
    }))
}

fn review_schema() -> Value {
    wrapped_request_schema(json!({
        "type":"object",
        "required":["idempotency_key","expected_request_digest","decision","confirm_review"],
        "properties":{
            "idempotency_key":support::bounded_string(200),
            "expected_request_digest":support::bounded_string(256),
            "decision":{"type":"string","enum":["approved","changes_requested","rejected"]},
            "review_reason":nullable_string(2000),
            "confirm_review":{"type":"boolean","const":true}
        },
        "additionalProperties":false
    }))
}

fn apply_schema() -> Value {
    wrapped_request_schema(json!({
        "type":"object",
        "required":["idempotency_key","expected_request_digest","expected_review_digest","confirm_application"],
        "properties":{
            "idempotency_key":support::bounded_string(200),
            "expected_request_digest":support::bounded_string(256),
            "expected_review_digest":support::bounded_string(256),
            "confirm_application":{"type":"boolean","const":true}
        },
        "additionalProperties":false
    }))
}

fn wrapped_request_schema(request: Value) -> Value {
    json!({
        "type":"object",
        "required":["request_id","request"],
        "properties":{"request_id":support::bounded_string(200),"request":request},
        "additionalProperties":false
    })
}

fn string_array(max_length: usize) -> Value {
    json!({
        "type":"array","minItems":1,"maxItems":100,"uniqueItems":true,
        "items":{"type":"string","minLength":1,"maxLength":max_length}
    })
}

fn nullable_string(max_length: usize) -> Value {
    json!({"type":["string","null"],"maxLength":max_length})
}
