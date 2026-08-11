use anyhow::Result;
use serde_json::Value;

use crate::store::Store;

#[cfg(test)]
#[path = "compute_federation_supply_interface_tests.rs"]
mod interface_tests;

#[cfg(test)]
#[path = "compute_federation_broker_interface_tests.rs"]
mod broker_interface_tests;

#[cfg(test)]
#[path = "compute_federation_management_mcp_tests.rs"]
mod management_mcp_tests;

#[cfg(test)]
#[path = "compute_federation_offer_interface_tests.rs"]
mod offer_interface_tests;

#[cfg(test)]
#[path = "compute_federation_activation_interface_tests.rs"]
mod activation_interface_tests;

#[cfg(test)]
#[path = "compute_federation_activation_interface_test_support.rs"]
mod activation_interface_test_support;

pub(crate) fn definitions() -> Vec<Value> {
    let mut tools = crate::compute_federation_provider_mcp::definitions();
    tools.extend(crate::compute_federation_capacity_pool_mcp::definitions());
    tools.extend(crate::compute_federation_capacity_bucket_mcp::definitions());
    tools.extend(crate::compute_federation_capacity_supply_mcp::definitions());
    tools.extend(crate::compute_federation_activation_mcp::definitions());
    tools.extend(crate::compute_federation_offer_mcp::definitions());
    tools.extend(crate::compute_federation_price_snapshot_mcp::definitions());
    tools.extend(crate::compute_federation_broker_mcp::definitions());
    tools.extend(crate::compute_federation::external_pool_onboarding_mcp::definitions());
    tools
}

pub(crate) fn definitions_for_platform_role(platform_role: &str) -> Vec<Value> {
    let mut tools = definitions();
    if matches!(platform_role, "admin" | "owner") {
        tools.extend(admin_definitions());
    }
    tools
}

fn admin_definitions() -> Vec<Value> {
    let mut tools = crate::compute_federation::external_pool_onboarding_mcp::admin_definitions();
    tools.extend(crate::compute_federation::external_pool_adapter_release_mcp::admin_definitions());
    tools.extend(crate::compute_federation::activation_admin_mcp::admin_definitions());
    tools.extend(crate::compute_federation::offer_admin_mcp::admin_definitions());
    tools
        .extend(crate::compute_federation::platform_reference_price_curve_mcp::admin_definitions());
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
    if let Some(value) = crate::compute_federation_offer_mcp::call_if_handled(
        store,
        user_id,
        name,
        arguments.clone(),
    )? {
        return Ok(Some(value));
    }
    if let Some(value) = crate::compute_federation_price_snapshot_mcp::call_if_handled(
        store,
        user_id,
        name,
        arguments.clone(),
    )? {
        return Ok(Some(value));
    }
    if let Some(value) = crate::compute_federation::external_pool_onboarding_mcp::call_if_handled(
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

pub(crate) fn call_admin_if_handled(
    store: &Store,
    user_id: &str,
    platform_role: &str,
    name: &str,
    arguments: Value,
) -> Result<Option<Value>> {
    if let Some(value) =
        crate::compute_federation::external_pool_onboarding_mcp::call_admin_if_handled(
            store,
            user_id,
            platform_role,
            name,
            arguments.clone(),
        )?
    {
        return Ok(Some(value));
    }
    if let Some(value) =
        crate::compute_federation::external_pool_adapter_release_mcp::call_admin_if_handled(
            store,
            user_id,
            platform_role,
            name,
            arguments.clone(),
        )?
    {
        return Ok(Some(value));
    }
    if let Some(value) = crate::compute_federation::offer_admin_mcp::call_admin_if_handled(
        store,
        user_id,
        platform_role,
        name,
        arguments.clone(),
    )? {
        return Ok(Some(value));
    }
    if let Some(value) = crate::compute_federation::activation_admin_mcp::call_admin_if_handled(
        store,
        user_id,
        platform_role,
        name,
        arguments.clone(),
    )? {
        return Ok(Some(value));
    }
    crate::compute_federation::platform_reference_price_curve_mcp::call_admin_if_handled(
        store,
        user_id,
        platform_role,
        name,
        arguments,
    )
}
