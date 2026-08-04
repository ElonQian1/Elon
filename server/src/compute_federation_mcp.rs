use anyhow::Result;
use serde_json::Value;

use crate::store::Store;

pub(crate) fn definitions() -> Vec<Value> {
    let mut tools = crate::compute_federation_provider_mcp::definitions();
    tools.extend(crate::compute_federation_capacity_pool_mcp::definitions());
    tools.extend(crate::compute_federation_capacity_bucket_mcp::definitions());
    tools.extend(crate::compute_federation_capacity_supply_mcp::definitions());
    tools.extend(crate::compute_federation_activation_mcp::definitions());
    tools.extend(crate::compute_federation_broker_mcp::definitions());
    tools
}

pub(crate) fn call_if_handled(
    store: &Store,
    project_id: &str,
    user_id: &str,
    name: &str,
    arguments: Value,
) -> Result<Option<Value>> {
    if let Some(value) = crate::compute_federation_provider_mcp::call_if_handled(
        store,
        user_id,
        name,
        arguments.clone(),
    )? {
        return Ok(Some(value));
    }
    if let Some(value) = crate::compute_federation_capacity_pool_mcp::call_if_handled(
        store,
        user_id,
        name,
        arguments.clone(),
    )? {
        return Ok(Some(value));
    }
    if let Some(value) = crate::compute_federation_capacity_bucket_mcp::call_if_handled(
        store,
        user_id,
        name,
        arguments.clone(),
    )? {
        return Ok(Some(value));
    }
    if let Some(value) = crate::compute_federation_capacity_supply_mcp::call_if_handled(
        store,
        user_id,
        name,
        arguments.clone(),
    )? {
        return Ok(Some(value));
    }
    if let Some(value) = crate::compute_federation_activation_mcp::call_if_handled(
        store,
        user_id,
        name,
        arguments.clone(),
    )? {
        return Ok(Some(value));
    }
    crate::compute_federation_broker_mcp::call_if_handled(
        store, project_id, user_id, name, arguments,
    )
}
