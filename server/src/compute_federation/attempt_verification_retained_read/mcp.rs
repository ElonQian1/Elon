use anyhow::{anyhow, bail, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{compute_federation::management_mcp_support as support, store::Store};

use super::service::{self, AttemptVerificationRetainedReadError, ADMIN_FORBIDDEN};

const GET_MY: &str = "compute_get_my_attempt_verification_decision";
const ADMIN_GET: &str = "compute_admin_get_attempt_verification_decision";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LeaseArguments {
    lease_id: String,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![support::tool(
        GET_MY,
        "读取当前消费者或 Provider 所有者可见的 retained v192 Verification 决定；无状态、资金或调度副作用。",
        lease_schema(),
        true,
        false,
    )]
}

pub(crate) fn admin_definitions() -> Vec<Value> {
    vec![support::tool(
        ADMIN_GET,
        "平台管理员按 Attempt Lease 读取 retained v192 Verification 决定；无副作用。",
        lease_schema(),
        true,
        false,
    )]
}

pub(crate) fn call_if_handled(
    store: &Store,
    project_id: &str,
    user_id: &str,
    name: &str,
    arguments: Value,
) -> Result<Option<Value>> {
    if name != GET_MY {
        return Ok(None);
    }
    let input = decode(arguments)?;
    let receipt = service::read_for_participant(store, user_id, &input.lease_id, Some(project_id))
        .map_err(redacted_service_error)?;
    Ok(Some(
        serde_json::to_value(receipt).map_err(|_| anyhow!(service::INTEGRITY_CONFLICT))?,
    ))
}

pub(crate) fn call_admin_if_handled(
    store: &Store,
    platform_role: &str,
    name: &str,
    arguments: Value,
) -> Result<Option<Value>> {
    if name != ADMIN_GET {
        return Ok(None);
    }
    ensure_platform_admin(platform_role)?;
    let input = decode(arguments)?;
    let receipt =
        service::read_for_admin(store, &input.lease_id).map_err(redacted_service_error)?;
    Ok(Some(
        serde_json::to_value(receipt).map_err(|_| anyhow!(service::INTEGRITY_CONFLICT))?,
    ))
}

fn decode(arguments: Value) -> Result<LeaseArguments> {
    serde_json::from_value(arguments).map_err(|_| anyhow!(service::INVALID_REQUEST_INPUT))
}

fn ensure_platform_admin(platform_role: &str) -> Result<()> {
    if !matches!(platform_role, "admin" | "owner") {
        bail!(ADMIN_FORBIDDEN);
    }
    Ok(())
}

fn redacted_service_error(error: AttemptVerificationRetainedReadError) -> anyhow::Error {
    anyhow!(error.code())
}

fn lease_schema() -> Value {
    json!({
        "type":"object",
        "required":["lease_id"],
        "properties":{"lease_id":support::bounded_string(200)},
        "additionalProperties":false
    })
}
