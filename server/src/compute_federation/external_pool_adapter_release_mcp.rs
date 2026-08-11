//! Administrator MCP surface for governed external-pool Adapter release staging.

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::store::Store;

use super::{
    external_pool_adapter_release_service::{
        self as service, ReviewExternalPoolAdapterReleaseBody, StageExternalPoolAdapterReleaseBody,
        SubmitExternalPoolAdapterReleaseBody,
    },
    management_mcp_support as support,
};

const LIST: &str = "compute_admin_list_external_pool_adapter_releases";
const GET: &str = "compute_admin_get_external_pool_adapter_release";
const PREFLIGHT: &str = "compute_admin_preflight_external_pool_adapter_release";
const SUBMIT: &str = "compute_admin_submit_external_pool_adapter_release";
const REVIEW: &str = "compute_admin_review_external_pool_adapter_release";
const STAGE: &str = "compute_admin_stage_external_pool_adapter_release";

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
struct ReviewArguments {
    request_id: String,
    request: ReviewExternalPoolAdapterReleaseBody,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StageArguments {
    request_id: String,
    request: StageExternalPoolAdapterReleaseBody,
}

pub(crate) fn admin_definitions() -> Vec<Value> {
    vec![
        support::tool(LIST, "平台管理员列出外部算力池 Adapter 发布申请。", support::list_schema(), true, false),
        support::tool(GET, "平台管理员读取一份 Adapter 发布申请及治理回执。", entity_schema(), true, false),
        support::tool(PREFLIGHT, "检查当前管理员对 Adapter 发布申请可执行的下一步操作，不改变状态。", entity_schema(), true, false),
        support::tool(SUBMIT, "提交外部算力池 Adapter 候选发布元数据。只登记声明，不下载或验证产物，也不授予 v213 路由权限；必须显式确认。", submit_schema(), false, false),
        support::tool(REVIEW, "由不同平台管理员独立复核 Adapter 候选发布；必须显式确认。", review_schema(), false, false),
        support::tool(STAGE, "暂存已批准的 Adapter 发布元数据。staged 不表示产物、凭据验证器或路由已验证；必须显式确认。", stage_schema(), false, false),
    ]
}

pub(crate) fn call_admin_if_handled(
    store: &Store,
    user_id: &str,
    platform_role: &str,
    name: &str,
    arguments: Value,
) -> Result<Option<Value>> {
    let value = match name {
        LIST => {
            support::ensure_platform_admin(platform_role)?;
            let input: ListArguments = support::decode(arguments, name)?;
            json!({"adapter_release_requests":service::list_for_admin(
                store, input.status.as_deref(), input.limit
            )?})
        }
        GET => {
            support::ensure_platform_admin(platform_role)?;
            let input: EntityArguments = support::decode(arguments, name)?;
            serde_json::to_value(service::get_for_admin(store, &input.request_id)?)?
        }
        PREFLIGHT => {
            support::ensure_platform_admin(platform_role)?;
            let input: EntityArguments = support::decode(arguments, name)?;
            serde_json::to_value(service::preflight_for_admin(
                store,
                user_id,
                &input.request_id,
            )?)?
        }
        SUBMIT => {
            support::ensure_platform_admin(platform_role)?;
            serde_json::to_value(service::submit_for_admin(
                store,
                user_id,
                support::decode::<SubmitExternalPoolAdapterReleaseBody>(arguments, name)?,
            )?)?
        }
        REVIEW => {
            support::ensure_platform_admin(platform_role)?;
            let input: ReviewArguments = support::decode(arguments, name)?;
            serde_json::to_value(service::review_for_admin(
                store,
                user_id,
                &input.request_id,
                input.request,
            )?)?
        }
        STAGE => {
            support::ensure_platform_admin(platform_role)?;
            let input: StageArguments = support::decode(arguments, name)?;
            serde_json::to_value(service::stage_for_admin(
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
            "idempotency_key","adapter_id","release_version","candidate_artifact_ref",
            "declared_implementation_sha256","supported_capabilities",
            "expected_credential_verifier","confirm_submission"
        ],
        "properties":{
            "idempotency_key":support::bounded_string(200),
            "adapter_id":support::bounded_string(160),
            "release_version":support::bounded_string(100),
            "candidate_artifact_ref":support::bounded_string(500),
            "declared_implementation_sha256":support::bounded_string(256),
            "supported_capabilities":{
                "type":"array","minItems":1,"maxItems":100,"uniqueItems":true,
                "items":{
                    "type":"object",
                    "required":["capability_id","capability_revision"],
                    "properties":{
                        "capability_id":support::bounded_string(160),
                        "capability_revision":{"type":"integer","minimum":1}
                    },
                    "additionalProperties":false
                }
            },
            "expected_credential_verifier":{
                "type":"object",
                "required":["verification_kind","verifier_id","verifier_revision","verifier_digest"],
                "properties":{
                    "verification_kind":support::bounded_string(100),
                    "verifier_id":support::bounded_string(160),
                    "verifier_revision":{"type":"integer","minimum":1},
                    "verifier_digest":support::bounded_string(256)
                },
                "additionalProperties":false
            },
            "submission_note":{"type":"string","maxLength":2000,"default":""},
            "confirm_submission":{"type":"boolean","const":true}
        },
        "additionalProperties":false
    })
}

fn review_schema() -> Value {
    wrapped_request_schema(json!({
        "type":"object",
        "required":[
            "idempotency_key","expected_request_digest","expected_request_material_digest",
            "decision","confirm_review"
        ],
        "properties":{
            "idempotency_key":support::bounded_string(200),
            "expected_request_digest":support::bounded_string(256),
            "expected_request_material_digest":support::bounded_string(256),
            "decision":{"type":"string","enum":["approved","changes_requested","rejected"]},
            "review_note":{"type":["string","null"],"maxLength":2000},
            "confirm_review":{"type":"boolean","const":true}
        },
        "additionalProperties":false
    }))
}

fn stage_schema() -> Value {
    wrapped_request_schema(json!({
        "type":"object",
        "required":[
            "idempotency_key","expected_request_digest","expected_request_material_digest",
            "expected_review_digest","confirm_stage"
        ],
        "properties":{
            "idempotency_key":support::bounded_string(200),
            "expected_request_digest":support::bounded_string(256),
            "expected_request_material_digest":support::bounded_string(256),
            "expected_review_digest":support::bounded_string(256),
            "apply_note":{"type":"string","maxLength":2000,"default":""},
            "confirm_stage":{"type":"boolean","const":true}
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
