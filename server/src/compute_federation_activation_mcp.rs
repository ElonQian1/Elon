use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    compute_federation_activation_service::{
        self, CancelMyComputeActivationEvidenceRequest, SubmitMyComputeActivationEvidenceRequest,
    },
    store::Store,
};

const SUBMIT_TOOL: &str = "compute_submit_my_activation_evidence_request";
const GET_TOOL: &str = "compute_get_my_activation_evidence_request";
const LIST_TOOL: &str = "compute_list_my_activation_evidence_requests";
const CANCEL_TOOL: &str = "compute_cancel_my_activation_evidence_request";
const PREFLIGHT_TOOL: &str = "compute_preflight_my_activation_evidence_request";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SubmitArguments {
    provider_id: String,
    pool_id: String,
    idempotency_key: String,
    node_binding_ref: String,
    ready_capability_digest: String,
    route_proof_digest: String,
    hardware_observation_digest: String,
    confirm_evidence_submission: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestArguments {
    provider_id: String,
    pool_id: String,
    request_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListArguments {
    provider_id: String,
    pool_id: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CancelArguments {
    provider_id: String,
    pool_id: String,
    request_id: String,
    expected_request_digest: String,
    confirm_cancel: bool,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        tool(
            SUBMIT_TOOL,
            "为本人 registering Provider/CapacityPool 提交人工审核证据摘要。必须显式确认；提交不激活资源，也不证明硬件或路由已经验证。",
            submit_schema(),
            false,
        ),
        tool(
            GET_TOOL,
            "读取本人指定 Provider/CapacityPool 的一份激活证据申请。",
            request_schema(),
            true,
        ),
        tool(
            LIST_TOOL,
            "列出本人指定 Provider/CapacityPool 的激活证据申请历史。",
            list_schema(),
            true,
        ),
        tool(
            CANCEL_TOOL,
            "显式取消本人仍处于 submitted 的激活证据申请；不能取消已批准、退回或拒绝的申请。",
            cancel_schema(),
            false,
        ),
        tool(
            PREFLIGHT_TOOL,
            "只读复核本人激活证据申请及当前 Provider/CapacityPool 是否满足后续激活前置条件，返回稳定阻断代码；不会执行激活或写入验证事实。",
            request_schema(),
            true,
        ),
    ]
}

pub(crate) fn call_if_handled(
    store: &Store,
    user_id: &str,
    name: &str,
    arguments: Value,
) -> Result<Option<Value>> {
    match name {
        SUBMIT_TOOL => {
            let input: SubmitArguments = decode(arguments, name)?;
            Ok(Some(serde_json::to_value(
                compute_federation_activation_service::submit_for_user(
                    store,
                    user_id,
                    &input.provider_id,
                    &input.pool_id,
                    SubmitMyComputeActivationEvidenceRequest {
                        idempotency_key: input.idempotency_key,
                        node_binding_ref: input.node_binding_ref,
                        ready_capability_digest: input.ready_capability_digest,
                        route_proof_digest: input.route_proof_digest,
                        hardware_observation_digest: input.hardware_observation_digest,
                        confirm_evidence_submission: input.confirm_evidence_submission,
                    },
                )?,
            )?))
        }
        GET_TOOL => {
            let input: RequestArguments = decode(arguments, name)?;
            Ok(Some(serde_json::to_value(
                compute_federation_activation_service::get_for_user(
                    store,
                    user_id,
                    &input.provider_id,
                    &input.pool_id,
                    &input.request_id,
                )?,
            )?))
        }
        LIST_TOOL => {
            let input: ListArguments = decode(arguments, name)?;
            Ok(Some(json!({
                "activation_evidence_requests":
                    compute_federation_activation_service::list_for_user(
                        store,
                        user_id,
                        &input.provider_id,
                        &input.pool_id,
                        input.limit,
                    )?
            })))
        }
        CANCEL_TOOL => {
            let input: CancelArguments = decode(arguments, name)?;
            Ok(Some(serde_json::to_value(
                compute_federation_activation_service::cancel_for_user(
                    store,
                    user_id,
                    &input.provider_id,
                    &input.pool_id,
                    &input.request_id,
                    CancelMyComputeActivationEvidenceRequest {
                        expected_request_digest: input.expected_request_digest,
                        confirm_cancel: input.confirm_cancel,
                    },
                )?,
            )?))
        }
        PREFLIGHT_TOOL => {
            let input: RequestArguments = decode(arguments, name)?;
            Ok(Some(serde_json::to_value(
                compute_federation_activation_service::preflight_for_user(
                    store,
                    user_id,
                    &input.provider_id,
                    &input.pool_id,
                    &input.request_id,
                )?,
            )?))
        }
        _ => Ok(None),
    }
}

fn submit_schema() -> Value {
    json!({
        "type":"object",
        "required":[
            "provider_id","pool_id","idempotency_key","node_binding_ref",
            "ready_capability_digest","route_proof_digest",
            "hardware_observation_digest","confirm_evidence_submission"
        ],
        "properties":{
            "provider_id":bounded_string(160),
            "pool_id":bounded_string(160),
            "idempotency_key":bounded_string(160),
            "node_binding_ref":bounded_string(160),
            "ready_capability_digest":digest_schema(),
            "route_proof_digest":digest_schema(),
            "hardware_observation_digest":digest_schema(),
            "confirm_evidence_submission":{"type":"boolean","const":true}
        },
        "additionalProperties":false
    })
}

fn request_schema() -> Value {
    json!({
        "type":"object",
        "required":["provider_id","pool_id","request_id"],
        "properties":{
            "provider_id":bounded_string(160),
            "pool_id":bounded_string(160),
            "request_id":bounded_string(160)
        },
        "additionalProperties":false
    })
}

fn list_schema() -> Value {
    json!({
        "type":"object",
        "required":["provider_id","pool_id"],
        "properties":{
            "provider_id":bounded_string(160),
            "pool_id":bounded_string(160),
            "limit":{"type":"integer","minimum":1,"maximum":100,"default":20}
        },
        "additionalProperties":false
    })
}

fn cancel_schema() -> Value {
    json!({
        "type":"object",
        "required":[
            "provider_id","pool_id","request_id","expected_request_digest","confirm_cancel"
        ],
        "properties":{
            "provider_id":bounded_string(160),
            "pool_id":bounded_string(160),
            "request_id":bounded_string(160),
            "expected_request_digest":digest_schema(),
            "confirm_cancel":{"type":"boolean","const":true}
        },
        "additionalProperties":false
    })
}

fn bounded_string(max_length: usize) -> Value {
    json!({"type":"string","minLength":1,"maxLength":max_length})
}

fn digest_schema() -> Value {
    json!({"type":"string","pattern":"^[0-9a-f]{64}$"})
}

fn decode<T: for<'de> Deserialize<'de>>(arguments: Value, name: &str) -> Result<T> {
    serde_json::from_value(arguments).with_context(|| format!("{name} 参数无效"))
}

fn default_limit() -> usize {
    20
}

fn tool(name: &str, description: &str, input_schema: Value, read_only: bool) -> Value {
    json!({
        "name":name,
        "description":description,
        "inputSchema":input_schema,
        "annotations":{
            "readOnlyHint":read_only,
            "destructiveHint":!read_only,
            "idempotentHint":true,
            "openWorldHint":false
        }
    })
}
