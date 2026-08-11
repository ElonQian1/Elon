use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    compute_federation_activation_recovery_service::{
        self, ApplyActivationRecoveryPlanBody, PrepareActivationRecoveryPlanBody,
        ReviewActivationRecoveryPlanBody, SupersedeActivationRecoveryPlanBody,
    },
    store::Store,
};

use super::{request_schema, support, wrapped_schema};

const GET_PLAN: &str = "compute_admin_get_activation_recovery_plan";
const PREPARE_PLAN: &str = "compute_admin_prepare_activation_recovery_plan";
const PREFLIGHT_PLAN: &str = "compute_admin_preflight_activation_recovery_plan";
const GET_SUPERSESSION: &str = "compute_admin_get_activation_recovery_supersession";
const SUPERSEDE_PLAN: &str = "compute_admin_supersede_activation_recovery_plan";
const GET_REVIEW: &str = "compute_admin_get_activation_recovery_review";
const REVIEW_PLAN: &str = "compute_admin_review_activation_recovery_plan";
const GET_APPLICATION: &str = "compute_admin_get_activation_recovery_application";
const APPLY_PLAN: &str = "compute_admin_apply_activation_recovery_plan";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestArguments {
    request_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PrepareArguments {
    request_id: String,
    request: PrepareActivationRecoveryPlanBody,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewArguments {
    request_id: String,
    request: ReviewActivationRecoveryPlanBody,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ApplyArguments {
    request_id: String,
    request: ApplyActivationRecoveryPlanBody,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SupersedeArguments {
    request_id: String,
    request: SupersedeActivationRecoveryPlanBody,
}

pub(super) fn definitions() -> Vec<Value> {
    vec![
        support::tool(
            GET_PLAN,
            "读取隔离恢复计划。",
            request_schema(),
            true,
            false,
        ),
        support::tool(
            PREPARE_PLAN,
            "基于精确隔离摘要准备恢复计划；必须显式确认，不直接恢复。",
            prepare_schema(),
            false,
            false,
        ),
        support::tool(
            PREFLIGHT_PLAN,
            "只读检查恢复计划、独立复核、报价依赖与当前隔离状态。",
            request_schema(),
            true,
            false,
        ),
        support::tool(
            GET_SUPERSESSION,
            "读取恢复计划废止回执。",
            request_schema(),
            true,
            false,
        ),
        support::tool(
            SUPERSEDE_PLAN,
            "废止尚未应用的恢复计划，以便重新准备；必须显式确认。",
            supersede_schema(),
            false,
            true,
        ),
        support::tool(
            GET_REVIEW,
            "读取恢复计划独立复核回执。",
            request_schema(),
            true,
            false,
        ),
        support::tool(
            REVIEW_PLAN,
            "由不同管理员复核恢复计划；必须显式确认。",
            review_schema(),
            false,
            false,
        ),
        support::tool(
            GET_APPLICATION,
            "读取恢复计划应用回执。",
            request_schema(),
            true,
            false,
        ),
        support::tool(
            APPLY_PLAN,
            "在复核和 preflight 通过后原子恢复 Provider 与 Pool；必须显式确认。",
            apply_schema(),
            false,
            false,
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
        GET_PLAN => read(
            store,
            platform_role,
            arguments,
            name,
            |store, request_id| store.compute_activation_recovery_plan_for_request(request_id),
        )?,
        PREPARE_PLAN => {
            support::ensure_platform_admin(platform_role)?;
            let input: PrepareArguments = support::decode(arguments, name)?;
            serde_json::to_value(compute_federation_activation_recovery_service::prepare(
                store,
                user_id,
                &input.request_id,
                input.request,
            )?)?
        }
        PREFLIGHT_PLAN => read(
            store,
            platform_role,
            arguments,
            name,
            |store, request_id| {
                compute_federation_activation_recovery_service::preflight(store, request_id)
            },
        )?,
        GET_SUPERSESSION => read(
            store,
            platform_role,
            arguments,
            name,
            |store, request_id| {
                compute_federation_activation_recovery_service::get_supersession(store, request_id)
            },
        )?,
        SUPERSEDE_PLAN => {
            support::ensure_platform_admin(platform_role)?;
            let input: SupersedeArguments = support::decode(arguments, name)?;
            serde_json::to_value(compute_federation_activation_recovery_service::supersede(
                store,
                user_id,
                &input.request_id,
                input.request,
            )?)?
        }
        GET_REVIEW => read(
            store,
            platform_role,
            arguments,
            name,
            |store, request_id| store.compute_activation_recovery_review_for_request(request_id),
        )?,
        REVIEW_PLAN => {
            support::ensure_platform_admin(platform_role)?;
            let input: ReviewArguments = support::decode(arguments, name)?;
            serde_json::to_value(compute_federation_activation_recovery_service::review(
                store,
                user_id,
                &input.request_id,
                input.request,
            )?)?
        }
        GET_APPLICATION => read(
            store,
            platform_role,
            arguments,
            name,
            |store, request_id| {
                store.compute_activation_recovery_application_for_request(request_id)
            },
        )?,
        APPLY_PLAN => {
            support::ensure_platform_admin(platform_role)?;
            let input: ApplyArguments = support::decode(arguments, name)?;
            serde_json::to_value(compute_federation_activation_recovery_service::apply(
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

fn read<T: serde::Serialize>(
    store: &Store,
    platform_role: &str,
    arguments: Value,
    name: &str,
    operation: impl FnOnce(&Store, &str) -> Result<T>,
) -> Result<Value> {
    support::ensure_platform_admin(platform_role)?;
    let input: RequestArguments = support::decode(arguments, name)?;
    Ok(serde_json::to_value(operation(store, &input.request_id)?)?)
}

fn prepare_schema() -> Value {
    wrapped_schema(json!({
        "type":"object",
        "required":[
            "idempotency_key","expected_quarantine_digest","verified_hardware_digest",
            "trust_tier","verified_at","remediation_summary","evidence_refs","confirm_prepare"
        ],
        "properties":{
            "idempotency_key":support::bounded_string(160),
            "expected_quarantine_digest":super::digest_schema(),
            "endpoint":super::endpoint_schema(),
            "adapter":super::adapter_schema(),
            "verified_hardware_digest":super::digest_schema(),
            "trust_tier":support::bounded_string(80),
            "verified_at":{"type":"string","format":"date-time","maxLength":64},
            "remediation_summary":support::bounded_string(2000),
            "evidence_refs":{
                "type":"array","maxItems":100,
                "items":support::bounded_string(1000)
            },
            "confirm_prepare":{"type":"boolean","const":true}
        },
        "additionalProperties":false
    }))
}

fn supersede_schema() -> Value {
    wrapped_schema(json!({
        "type":"object",
        "required":["idempotency_key","expected_plan_digest","reason","confirm_supersede"],
        "properties":{
            "idempotency_key":support::bounded_string(160),
            "expected_plan_digest":super::digest_schema(),
            "reason":support::bounded_string(1000),
            "confirm_supersede":{"type":"boolean","const":true}
        },
        "additionalProperties":false
    }))
}

fn review_schema() -> Value {
    wrapped_schema(json!({
        "type":"object",
        "required":["idempotency_key","expected_plan_digest","confirm_review"],
        "properties":{
            "idempotency_key":support::bounded_string(160),
            "expected_plan_digest":super::digest_schema(),
            "review_note":{"type":["string","null"],"maxLength":1000},
            "confirm_review":{"type":"boolean","const":true}
        },
        "additionalProperties":false
    }))
}

fn apply_schema() -> Value {
    wrapped_schema(json!({
        "type":"object",
        "required":["idempotency_key","expected_plan_digest","confirm_apply"],
        "properties":{
            "idempotency_key":support::bounded_string(160),
            "expected_plan_digest":super::digest_schema(),
            "confirm_apply":{"type":"boolean","const":true}
        },
        "additionalProperties":false
    }))
}
