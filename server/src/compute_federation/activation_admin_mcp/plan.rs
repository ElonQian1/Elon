use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    compute_federation_activation_application_service::{self, ApplyComputeActivationPlanBody},
    compute_federation_activation_plan_review_service::{self, ReviewComputeActivationPlanBody},
    compute_federation_activation_plan_service::{self, PrepareComputeActivationPlanBody},
    compute_federation_activation_quarantine_service::{
        self, QuarantineComputeActivationApplicationBody,
    },
    store::Store,
};

use super::{request_schema, support, wrapped_schema};

const GET_PLAN: &str = "compute_admin_get_activation_plan";
const PREPARE_PLAN: &str = "compute_admin_prepare_activation_plan";
const PREFLIGHT_PLAN: &str = "compute_admin_preflight_activation_plan";
const GET_REVIEW: &str = "compute_admin_get_activation_plan_review";
const REVIEW_PLAN: &str = "compute_admin_review_activation_plan";
const GET_APPLICATION: &str = "compute_admin_get_activation_application";
const APPLY_PLAN: &str = "compute_admin_apply_activation_plan";
const GET_QUARANTINE: &str = "compute_admin_get_activation_quarantine";
const QUARANTINE: &str = "compute_admin_quarantine_activation_application";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestArguments {
    request_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PrepareArguments {
    request_id: String,
    request: PrepareComputeActivationPlanBody,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewArguments {
    request_id: String,
    request: ReviewComputeActivationPlanBody,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ApplyArguments {
    request_id: String,
    request: ApplyComputeActivationPlanBody,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QuarantineArguments {
    request_id: String,
    request: QuarantineComputeActivationApplicationBody,
}

pub(super) fn definitions() -> Vec<Value> {
    vec![
        support::tool(
            GET_PLAN,
            "读取激活计划，不改变 Provider 或 Pool。",
            request_schema(),
            true,
            false,
        ),
        support::tool(
            PREPARE_PLAN,
            "为已批准申请准备精确激活计划；必须显式确认，不直接激活。",
            prepare_schema(),
            false,
            false,
        ),
        support::tool(
            PREFLIGHT_PLAN,
            "只读校验激活计划、独立复核和当前依赖。",
            request_schema(),
            true,
            false,
        ),
        support::tool(
            GET_REVIEW,
            "读取激活计划的独立复核回执。",
            request_schema(),
            true,
            false,
        ),
        support::tool(
            REVIEW_PLAN,
            "由不同管理员复核激活计划；必须显式确认。",
            review_schema(),
            false,
            false,
        ),
        support::tool(
            GET_APPLICATION,
            "读取激活计划应用回执。",
            request_schema(),
            true,
            false,
        ),
        support::tool(
            APPLY_PLAN,
            "在复核与 preflight 通过后原子激活 Provider 和 Pool；必须显式确认。",
            apply_schema(),
            false,
            false,
        ),
        support::tool(
            GET_QUARANTINE,
            "读取激活结果隔离回执。",
            request_schema(),
            true,
            false,
        ),
        support::tool(
            QUARANTINE,
            "按精确应用摘要隔离 Provider 和 Pool；必须显式确认。",
            quarantine_schema(),
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
        GET_PLAN => read(
            store,
            platform_role,
            arguments,
            name,
            |store, request_id| {
                compute_federation_activation_plan_service::get_for_review(store, request_id)
            },
        )?,
        PREPARE_PLAN => {
            support::ensure_platform_admin(platform_role)?;
            let input: PrepareArguments = support::decode(arguments, name)?;
            serde_json::to_value(
                compute_federation_activation_plan_service::prepare_for_review(
                    store,
                    user_id,
                    &input.request_id,
                    input.request,
                )?,
            )?
        }
        PREFLIGHT_PLAN => read(
            store,
            platform_role,
            arguments,
            name,
            |store, request_id| {
                compute_federation_activation_plan_service::preflight_for_review(store, request_id)
            },
        )?,
        GET_REVIEW => read(
            store,
            platform_role,
            arguments,
            name,
            |store, request_id| {
                compute_federation_activation_plan_review_service::get_for_admin(store, request_id)
            },
        )?,
        REVIEW_PLAN => {
            support::ensure_platform_admin(platform_role)?;
            let input: ReviewArguments = support::decode(arguments, name)?;
            serde_json::to_value(
                compute_federation_activation_plan_review_service::review_for_admin(
                    store,
                    user_id,
                    &input.request_id,
                    input.request,
                )?,
            )?
        }
        GET_APPLICATION => read(
            store,
            platform_role,
            arguments,
            name,
            |store, request_id| {
                compute_federation_activation_application_service::get_for_review(store, request_id)
            },
        )?,
        APPLY_PLAN => {
            support::ensure_platform_admin(platform_role)?;
            let input: ApplyArguments = support::decode(arguments, name)?;
            serde_json::to_value(
                compute_federation_activation_application_service::apply_for_review(
                    store,
                    user_id,
                    &input.request_id,
                    input.request,
                )?,
            )?
        }
        GET_QUARANTINE => read(
            store,
            platform_role,
            arguments,
            name,
            |store, request_id| {
                compute_federation_activation_quarantine_service::get_for_review(store, request_id)
            },
        )?,
        QUARANTINE => {
            support::ensure_platform_admin(platform_role)?;
            let input: QuarantineArguments = support::decode(arguments, name)?;
            serde_json::to_value(
                compute_federation_activation_quarantine_service::quarantine_for_review(
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
            "idempotency_key","expected_request_digest","endpoint",
            "verified_hardware_digest","trust_tier","verified_at","confirm_prepare"
        ],
        "properties":{
            "idempotency_key":support::bounded_string(160),
            "expected_request_digest":super::digest_schema(),
            "endpoint":super::endpoint_schema(),
            "verified_hardware_digest":super::digest_schema(),
            "trust_tier":support::bounded_string(80),
            "verified_at":{"type":"string","format":"date-time","maxLength":64},
            "confirm_prepare":{"type":"boolean","const":true}
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

fn quarantine_schema() -> Value {
    wrapped_schema(json!({
        "type":"object",
        "required":["idempotency_key","expected_application_digest","reason","confirm_quarantine"],
        "properties":{
            "idempotency_key":support::bounded_string(160),
            "expected_application_digest":super::digest_schema(),
            "reason":support::bounded_string(1000),
            "confirm_quarantine":{"type":"boolean","const":true}
        },
        "additionalProperties":false
    }))
}
