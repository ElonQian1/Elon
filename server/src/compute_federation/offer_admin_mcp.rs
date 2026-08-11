//! Administrator MCP surface for governed Offer publication and lifecycle changes.

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    compute_federation_offer_lifecycle_model::{
        DrainComputeOfferRequest, TerminateComputeOfferRequest,
    },
    compute_federation_offer_lifecycle_service,
    compute_federation_offer_publication_model::PublishComputeOfferDraftRequest,
    compute_federation_offer_publication_service, compute_federation_offer_service,
    store::Store,
};

use super::management_mcp_support as support;

const LIST: &str = "compute_admin_list_offer_drafts";
const GET: &str = "compute_admin_get_offer";
const GET_PUBLICATION: &str = "compute_admin_get_offer_publication";
const PUBLISH: &str = "compute_admin_publish_offer";
const GET_DRAIN: &str = "compute_admin_get_offer_drain";
const DRAIN: &str = "compute_admin_drain_offer";
const GET_EXPIRATION: &str = "compute_admin_get_offer_expiration";
const EXPIRE: &str = "compute_admin_expire_offer";
const GET_REVOCATION: &str = "compute_admin_get_offer_revocation";
const REVOKE: &str = "compute_admin_revoke_offer";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OfferArguments {
    offer_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListArguments {
    #[serde(default = "default_limit")]
    limit: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PublishArguments {
    offer_id: String,
    request: PublishComputeOfferDraftRequest,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DrainArguments {
    offer_id: String,
    request: DrainComputeOfferRequest,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TerminateArguments {
    offer_id: String,
    request: TerminateComputeOfferRequest,
}

pub(crate) fn admin_definitions() -> Vec<Value> {
    vec![
        support::tool(LIST, "平台管理员列出待治理的 draft Offer。", list_schema(), true, false),
        support::tool(GET, "平台管理员读取 Offer 当前投影并复核不可变版本。", offer_schema(), true, false),
        support::tool(GET_PUBLICATION, "读取 Offer 的发布回执，不改变市场状态。", offer_schema(), true, false),
        support::tool(PUBLISH, "按精确版本和摘要发布 draft Offer；必须显式确认。不会创建 Price Snapshot、预留容量或移动资金。", publish_schema(), false, false),
        support::tool(GET_DRAIN, "读取 Offer 的排空回执，不改变市场状态。", offer_schema(), true, false),
        support::tool(DRAIN, "按精确版本和摘要将 active Offer 转为 draining；必须显式确认。已有预留保持不变。", drain_schema(), false, false),
        support::tool(GET_EXPIRATION, "读取 Offer 的到期回执，不改变市场状态。", offer_schema(), true, false),
        support::tool(EXPIRE, "在 Offer 有效期结束后按精确版本和摘要转为 expired；必须显式确认。", terminate_schema(), false, false),
        support::tool(GET_REVOCATION, "读取 Offer 的提前撤销回执，不改变市场状态。", offer_schema(), true, false),
        support::tool(REVOKE, "按精确版本和摘要将 draining Offer 提前转为 revoked；必须显式确认。已有预留保持不变。", terminate_schema(), false, false),
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
            json!({"offers":compute_federation_offer_service::list_drafts_for_review(
                store, input.limit
            )?})
        }
        GET => {
            support::ensure_platform_admin(platform_role)?;
            let input: OfferArguments = support::decode(arguments, name)?;
            serde_json::to_value(compute_federation_offer_service::get_for_review(
                store,
                &input.offer_id,
            )?)?
        }
        GET_PUBLICATION => {
            support::ensure_platform_admin(platform_role)?;
            let input: OfferArguments = support::decode(arguments, name)?;
            serde_json::to_value(
                compute_federation_offer_publication_service::get_for_review(
                    store,
                    &input.offer_id,
                )?,
            )?
        }
        PUBLISH => {
            support::ensure_platform_admin(platform_role)?;
            let input: PublishArguments = support::decode(arguments, name)?;
            serde_json::to_value(
                compute_federation_offer_publication_service::publish_for_review(
                    store,
                    user_id,
                    &input.offer_id,
                    input.request,
                )?,
            )?
        }
        GET_DRAIN => {
            support::ensure_platform_admin(platform_role)?;
            let input: OfferArguments = support::decode(arguments, name)?;
            serde_json::to_value(
                compute_federation_offer_lifecycle_service::get_drain_for_review(
                    store,
                    &input.offer_id,
                )?,
            )?
        }
        DRAIN => {
            support::ensure_platform_admin(platform_role)?;
            let input: DrainArguments = support::decode(arguments, name)?;
            serde_json::to_value(
                compute_federation_offer_lifecycle_service::drain_for_review(
                    store,
                    user_id,
                    &input.offer_id,
                    input.request,
                )?,
            )?
        }
        GET_EXPIRATION => {
            support::ensure_platform_admin(platform_role)?;
            terminal_receipt(store, arguments, name, "expired")?
        }
        EXPIRE => {
            support::ensure_platform_admin(platform_role)?;
            let input: TerminateArguments = support::decode(arguments, name)?;
            serde_json::to_value(
                compute_federation_offer_lifecycle_service::expire_for_review(
                    store,
                    user_id,
                    &input.offer_id,
                    input.request,
                )?,
            )?
        }
        GET_REVOCATION => {
            support::ensure_platform_admin(platform_role)?;
            terminal_receipt(store, arguments, name, "revoked")?
        }
        REVOKE => {
            support::ensure_platform_admin(platform_role)?;
            let input: TerminateArguments = support::decode(arguments, name)?;
            serde_json::to_value(
                compute_federation_offer_lifecycle_service::revoke_for_review(
                    store,
                    user_id,
                    &input.offer_id,
                    input.request,
                )?,
            )?
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn terminal_receipt(store: &Store, arguments: Value, name: &str, status: &str) -> Result<Value> {
    let input: OfferArguments = support::decode(arguments, name)?;
    Ok(serde_json::to_value(
        compute_federation_offer_lifecycle_service::get_terminal_for_review(
            store,
            &input.offer_id,
            status,
        )?,
    )?)
}

fn offer_schema() -> Value {
    support::entity_schema("offer_id", 200)
}

fn list_schema() -> Value {
    json!({
        "type":"object",
        "properties":{"limit":{"type":"integer","minimum":1,"maximum":100,"default":20}},
        "additionalProperties":false
    })
}

fn publish_schema() -> Value {
    wrapped_schema(json!({
        "type":"object",
        "required":["expected_offer_version","expected_offer_digest","idempotency_key","confirm_publish"],
        "properties":{
            "expected_offer_version":{"type":"integer","minimum":1},
            "expected_offer_digest":support::bounded_string(256),
            "idempotency_key":support::bounded_string(160),
            "confirm_publish":{"type":"boolean","const":true}
        },
        "additionalProperties":false
    }))
}

fn drain_schema() -> Value {
    wrapped_schema(json!({
        "type":"object",
        "required":["expected_offer_version","expected_offer_digest","reason","idempotency_key","confirm_drain"],
        "properties":{
            "expected_offer_version":{"type":"integer","minimum":1},
            "expected_offer_digest":support::bounded_string(256),
            "reason":support::bounded_string(1000),
            "idempotency_key":support::bounded_string(160),
            "confirm_drain":{"type":"boolean","const":true}
        },
        "additionalProperties":false
    }))
}

fn terminate_schema() -> Value {
    wrapped_schema(json!({
        "type":"object",
        "required":["expected_offer_version","expected_offer_digest","reason","idempotency_key","confirm_terminal"],
        "properties":{
            "expected_offer_version":{"type":"integer","minimum":1},
            "expected_offer_digest":support::bounded_string(256),
            "reason":support::bounded_string(1000),
            "idempotency_key":support::bounded_string(160),
            "confirm_terminal":{"type":"boolean","const":true}
        },
        "additionalProperties":false
    }))
}

fn wrapped_schema(request: Value) -> Value {
    json!({
        "type":"object",
        "required":["offer_id","request"],
        "properties":{"offer_id":support::bounded_string(200),"request":request},
        "additionalProperties":false
    })
}

fn default_limit() -> usize {
    20
}
