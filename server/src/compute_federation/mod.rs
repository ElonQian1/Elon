//! Versioned domain contracts for the task-level distributed compute federation.
//!
//! This first layer is intentionally disconnected from persistence, HTTP, WebSocket and
//! scheduling. Existing node LLM routing remains the active compatibility path.

pub(crate) mod activation_admin_mcp;
pub(crate) mod attempt;
pub(crate) mod attempt_gateway;
pub(crate) mod capacity;
pub(crate) mod capacity_commitment;
pub(crate) mod capacity_commitment_api;
pub(crate) mod capacity_commitment_service;
pub(crate) mod delivery_allocation;
pub(crate) mod delivery_allocation_api;
pub(crate) mod delivery_allocation_service;
pub(crate) mod execution;
pub(crate) mod execution_plan;
pub(crate) mod external_pool_adapter_artifact_source;
pub(crate) mod external_pool_adapter_artifact_source_api;
pub(crate) mod external_pool_adapter_artifact_source_service;
pub(crate) mod external_pool_adapter_release;
pub(crate) mod external_pool_adapter_release_api;
pub(crate) mod external_pool_adapter_release_lifecycle;
pub(crate) mod external_pool_adapter_release_lifecycle_api;
pub(crate) mod external_pool_adapter_release_lifecycle_service;
pub(crate) mod external_pool_adapter_release_mcp;
pub(crate) mod external_pool_adapter_release_service;
pub(crate) mod external_pool_onboarding;
pub(crate) mod external_pool_onboarding_api;
pub(crate) mod external_pool_onboarding_mcp;
pub(crate) mod external_pool_onboarding_service;
pub(crate) mod legacy;
pub(crate) mod management_mcp_support;
pub(crate) mod market;
pub(crate) mod offer;
pub(crate) mod offer_admin_mcp;
pub(crate) mod platform_reference_price_curve;
pub(crate) mod platform_reference_price_curve_api;
pub(crate) mod platform_reference_price_curve_mcp;
pub(crate) mod platform_reference_price_curve_service;
pub(crate) mod provider;
pub(crate) mod receipts;
pub(crate) mod route_authority;
pub(crate) mod start_outbox;
pub(crate) mod workload;
