use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    compute_federation_activation_lifecycle_service::{
        self, SupersedeComputeActivationEvidenceRequestBody,
    },
    compute_federation_activation_service::{self, ReviewComputeActivationEvidenceRequestBody},
    store::Store,
};

use super::{request_schema, support, wrapped_schema};

const LIST: &str = "compute_admin_list_activation_evidence_requests";
const PREFLIGHT: &str = "compute_admin_preflight_activation_evidence_request";
const REVIEW: &str = "compute_admin_review_activation_evidence_request";
const SUPERSEDE: &str = "compute_admin_supersede_activation_evidence_request";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListArguments {
    #[serde(default = "default_status")]
    status: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestArguments {
    request_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewArguments {
    request_id: String,
    request: ReviewComputeActivationEvidenceRequestBody,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SupersedeArguments {
    request_id: String,
    request: SupersedeComputeActivationEvidenceRequestBody,
}

pub(super) fn definitions() -> Vec<Value> {
    vec![
        support::tool(
            LIST,
            "列出指定状态的激活证据申请治理队列。",
            list_schema(),
            true,
            false,
        ),
        support::tool(
            PREFLIGHT,
            "只读复核激活证据申请与当前 Provider、Pool 和账本事实。",
            request_schema(),
            true,
            false,
        ),
        support::tool(
            REVIEW,
            "按精确摘要批准、退回或拒绝激活证据申请；必须显式确认。",
            review_schema(),
            false,
            false,
        ),
        support::tool(
            SUPERSEDE,
            "废止已批准但尚未应用的激活证据申请；必须显式确认。",
            supersede_schema(),
            false,
            true,
        ),
    ]
}

pub(super) fn call_if_handled(
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
            json!({"activation_evidence_requests":
                compute_federation_activation_service::list_for_review(
                    store, &input.status, input.limit
                )?})
        }
        PREFLIGHT => {
            support::ensure_platform_admin(platform_role)?;
            let input: RequestArguments = support::decode(arguments, name)?;
            serde_json::to_value(compute_federation_activation_service::preflight_for_review(
                store,
                &input.request_id,
            )?)?
        }
        REVIEW => {
            support::ensure_platform_admin(platform_role)?;
            let input: ReviewArguments = support::decode(arguments, name)?;
            serde_json::to_value(compute_federation_activation_service::review(
                store,
                user_id,
                &input.request_id,
                input.request,
            )?)?
        }
        SUPERSEDE => {
            support::ensure_platform_admin(platform_role)?;
            let input: SupersedeArguments = support::decode(arguments, name)?;
            serde_json::to_value(
                compute_federation_activation_lifecycle_service::supersede_for_review(
                    store,
                    user_id,
                    &input.request_id,
                    input.request,
                )?,
            )?
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn list_schema() -> Value {
    json!({
        "type":"object",
        "properties":{
            "status":{
                "type":"string",
                "enum":["submitted","changes_requested","approved","activated","rejected","canceled","superseded"],
                "default":"submitted"
            },
            "limit":{"type":"integer","minimum":1,"maximum":100,"default":20}
        },
        "additionalProperties":false
    })
}

fn review_schema() -> Value {
    wrapped_schema(json!({
        "type":"object",
        "required":["expected_request_digest","decision","confirm_review"],
        "properties":{
            "expected_request_digest":super::digest_schema(),
            "decision":{"type":"string","enum":["approved","changes_requested","rejected"]},
            "review_note":{"type":["string","null"],"maxLength":1000},
            "confirm_review":{"type":"boolean","const":true}
        },
        "additionalProperties":false
    }))
}

fn supersede_schema() -> Value {
    wrapped_schema(json!({
        "type":"object",
        "required":["expected_request_digest","reason","confirm_supersede"],
        "properties":{
            "expected_request_digest":super::digest_schema(),
            "reason":support::bounded_string(1000),
            "confirm_supersede":{"type":"boolean","const":true}
        },
        "additionalProperties":false
    }))
}

fn default_status() -> String {
    "submitted".to_string()
}

fn default_limit() -> usize {
    20
}
