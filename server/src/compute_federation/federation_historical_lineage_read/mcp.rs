use anyhow::{anyhow, bail, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::store::Store;

use super::service::{self, FederationHistoricalLineageReadError, ADMIN_FORBIDDEN};
use crate::compute_federation::management_mcp_support as support;

const GET_MY_EXECUTION: &str = "compute_get_my_execution_source_lineage";
const GET_MY_SETTLEMENT: &str = "compute_get_my_settlement_source_lineage";
const ADMIN_GET_EXECUTION: &str = "compute_admin_get_execution_source_lineage";
const ADMIN_GET_SETTLEMENT: &str = "compute_admin_get_settlement_source_lineage";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LeaseArguments {
    lease_id: String,
}

pub(crate) fn definitions() -> Vec<Value> {
    vec![
        support::tool(
            GET_MY_EXECUTION,
            "读取当前消费者或 Provider 所有者可见的历史 Execution Source lineage；无状态、资金或调度副作用。",
            lease_schema(),
            true,
            false,
        ),
        support::tool(
            GET_MY_SETTLEMENT,
            "读取当前消费者或 Provider 所有者可见的历史 Settlement Source lineage；无状态、资金或调度副作用。",
            lease_schema(),
            true,
            false,
        ),
    ]
}

pub(crate) fn admin_definitions() -> Vec<Value> {
    vec![
        support::tool(
            ADMIN_GET_EXECUTION,
            "平台管理员按 Attempt Lease 读取历史 Execution Source lineage；无副作用。",
            lease_schema(),
            true,
            false,
        ),
        support::tool(
            ADMIN_GET_SETTLEMENT,
            "平台管理员按 Attempt Lease 读取历史 Settlement Source lineage；无副作用。",
            lease_schema(),
            true,
            false,
        ),
    ]
}

pub(crate) fn call_if_handled(
    store: &Store,
    project_id: &str,
    user_id: &str,
    name: &str,
    arguments: Value,
) -> Result<Option<Value>> {
    let document = match name {
        GET_MY_EXECUTION => {
            let input = decode(arguments)?;
            service::read_execution_for_participant(
                store,
                user_id,
                &input.lease_id,
                Some(project_id),
            )
        }
        GET_MY_SETTLEMENT => {
            let input = decode(arguments)?;
            service::read_settlement_for_participant(
                store,
                user_id,
                &input.lease_id,
                Some(project_id),
            )
        }
        _ => return Ok(None),
    }
    .map_err(redacted_service_error)?;
    Ok(Some(
        serde_json::to_value(document).map_err(|_| anyhow!(service::INTEGRITY_CONFLICT))?,
    ))
}

pub(crate) fn call_admin_if_handled(
    store: &Store,
    platform_role: &str,
    name: &str,
    arguments: Value,
) -> Result<Option<Value>> {
    let document = match name {
        ADMIN_GET_EXECUTION => {
            ensure_platform_admin(platform_role)?;
            let input = decode(arguments)?;
            service::read_execution_for_admin(store, &input.lease_id)
        }
        ADMIN_GET_SETTLEMENT => {
            ensure_platform_admin(platform_role)?;
            let input = decode(arguments)?;
            service::read_settlement_for_admin(store, &input.lease_id)
        }
        _ => return Ok(None),
    }
    .map_err(redacted_service_error)?;
    Ok(Some(
        serde_json::to_value(document).map_err(|_| anyhow!(service::INTEGRITY_CONFLICT))?,
    ))
}

fn decode(arguments: Value) -> Result<LeaseArguments> {
    serde_json::from_value(arguments).map_err(|_| anyhow!(service::INVALID_LEASE_ID))
}

fn ensure_platform_admin(platform_role: &str) -> Result<()> {
    if !matches!(platform_role, "admin" | "owner") {
        bail!(ADMIN_FORBIDDEN);
    }
    Ok(())
}

fn redacted_service_error(error: FederationHistoricalLineageReadError) -> anyhow::Error {
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
