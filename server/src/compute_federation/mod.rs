//! Versioned domain contracts for the task-level distributed compute federation.
//!
//! This first layer is intentionally disconnected from persistence, HTTP, WebSocket and
//! scheduling. Existing node LLM routing remains the active compatibility path.

pub(crate) mod activation_admin_mcp;
pub(crate) mod attempt;
pub(crate) mod attempt_gateway;
pub(crate) mod attempt_verification_retained_read;
pub(crate) mod capacity;
pub(crate) mod capacity_commitment;
pub(crate) mod capacity_commitment_api;
pub(crate) mod capacity_commitment_service;
pub(crate) mod capacity_future_settlement_lineage;
#[cfg(test)]
mod capacity_future_settlement_lineage_source_contract_tests;
pub(crate) mod capacity_instrument;
pub(crate) mod capacity_instrument_api;
pub(crate) mod capacity_instrument_service;
pub(crate) mod delivery_allocation;
pub(crate) mod delivery_allocation_api;
pub(crate) mod delivery_allocation_expiry_worker;
pub(crate) mod delivery_allocation_service;
pub(crate) mod execution;
pub(crate) mod execution_plan;
pub(crate) mod external_pool_adapter_adoption;
pub(crate) mod external_pool_adapter_adoption_api;
pub(crate) mod external_pool_adapter_adoption_service;
pub(crate) mod external_pool_adapter_artifact_package;
pub(crate) mod external_pool_adapter_artifact_package_api;
pub(crate) mod external_pool_adapter_artifact_package_service;
pub(crate) mod external_pool_adapter_artifact_sandbox_conformance;
pub(crate) mod external_pool_adapter_artifact_sandbox_conformance_api;
pub(crate) mod external_pool_adapter_artifact_sandbox_conformance_service;
pub(crate) mod external_pool_adapter_artifact_security;
pub(crate) mod external_pool_adapter_artifact_security_api;
pub(crate) mod external_pool_adapter_artifact_security_service;
pub(crate) mod external_pool_adapter_artifact_signed_provenance;
pub(crate) mod external_pool_adapter_artifact_signed_provenance_api;
pub(crate) mod external_pool_adapter_artifact_signed_provenance_service;
pub(crate) mod external_pool_adapter_artifact_signing_key;
pub(crate) mod external_pool_adapter_artifact_signing_key_api;
pub(crate) mod external_pool_adapter_artifact_signing_key_service;
pub(crate) mod external_pool_adapter_artifact_source;
pub(crate) mod external_pool_adapter_artifact_source_api;
pub(crate) mod external_pool_adapter_artifact_source_service;
pub(crate) mod external_pool_adapter_artifact_vulnerability_report;
pub(crate) mod external_pool_adapter_artifact_vulnerability_report_api;
pub(crate) mod external_pool_adapter_artifact_vulnerability_report_service;
pub(crate) mod external_pool_adapter_atomic_activation;
pub(crate) mod external_pool_adapter_broker_tls;
#[cfg(test)]
mod external_pool_adapter_broker_tls_source_contract_tests;
pub(crate) mod external_pool_adapter_credential_reattestation;
pub(crate) mod external_pool_adapter_credential_reattestation_api;
pub(crate) mod external_pool_adapter_credential_reattestation_service;
pub(crate) mod external_pool_adapter_credential_verification;
pub(crate) mod external_pool_adapter_credential_verification_api;
pub(crate) mod external_pool_adapter_credential_verification_service;
pub(crate) mod external_pool_adapter_credential_verifier;
pub(crate) mod external_pool_adapter_credential_verifier_api;
pub(crate) mod external_pool_adapter_credential_verifier_key;
pub(crate) mod external_pool_adapter_credential_verifier_key_api;
pub(crate) mod external_pool_adapter_credential_verifier_key_service;
pub(crate) mod external_pool_adapter_credential_verifier_service;
#[cfg(test)]
mod external_pool_adapter_entrypoint_capsule_source_contract_tests;
pub(crate) mod external_pool_adapter_installation;
pub(crate) mod external_pool_adapter_installation_api;
pub(crate) mod external_pool_adapter_installation_service;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) mod external_pool_adapter_linux_supervisor;
#[cfg(test)]
mod external_pool_adapter_linux_supervisor_source_contract_tests;
#[cfg(test)]
mod external_pool_adapter_no_work_probe_source_contract_tests;
pub(crate) mod external_pool_adapter_provider_active_successor;
pub(crate) mod external_pool_adapter_provider_runtime_readiness;
pub(crate) mod external_pool_adapter_provider_runtime_readiness_api;
pub(crate) mod external_pool_adapter_provider_runtime_readiness_service;
mod external_pool_adapter_provider_runtime_readiness_service_redaction;
mod external_pool_adapter_provider_runtime_readiness_service_validation;
pub(crate) mod external_pool_adapter_registry;
pub(crate) mod external_pool_adapter_registry_api;
pub(crate) mod external_pool_adapter_registry_service;
pub(crate) mod external_pool_adapter_release;
pub(crate) mod external_pool_adapter_release_api;
pub(crate) mod external_pool_adapter_release_lifecycle;
pub(crate) mod external_pool_adapter_release_lifecycle_api;
pub(crate) mod external_pool_adapter_release_lifecycle_service;
pub(crate) mod external_pool_adapter_release_mcp;
pub(crate) mod external_pool_adapter_release_service;
pub(crate) mod external_pool_adapter_route_renewal;
#[cfg(test)]
mod external_pool_adapter_runtime_bundle_source_contract_tests;
pub(crate) mod external_pool_adapter_runtime_compatibility;
pub(crate) mod external_pool_adapter_runtime_compatibility_signing_handoff_runtime;
pub(crate) mod external_pool_adapter_runtime_compatibility_signing_handoff_service;
mod external_pool_adapter_runtime_compatibility_signing_handoff_service_validation;
#[cfg(test)]
mod external_pool_adapter_runtime_compatibility_signing_handoff_tests;
pub(crate) mod external_pool_adapter_runtime_compatibility_verification;
pub(crate) mod external_pool_adapter_runtime_compatibility_verification_api;
pub(crate) mod external_pool_adapter_runtime_compatibility_verification_service;
mod external_pool_adapter_runtime_compatibility_verification_service_redaction;
mod external_pool_adapter_runtime_compatibility_verification_service_validation;
pub(crate) mod external_pool_adapter_runtime_launch_profile;
pub(crate) mod external_pool_adapter_runtime_launch_profile_api;
pub(crate) mod external_pool_adapter_runtime_launch_profile_service;
mod external_pool_adapter_runtime_launch_profile_service_redaction;
pub(crate) mod external_pool_adapter_sandbox_reattestation;
pub(crate) mod external_pool_adapter_sandbox_reattestation_api;
pub(crate) mod external_pool_adapter_sandbox_reattestation_service;
pub(crate) mod external_pool_adapter_sandbox_verifier_key;
pub(crate) mod external_pool_adapter_sandbox_verifier_key_api;
pub(crate) mod external_pool_adapter_sandbox_verifier_key_service;
pub(crate) mod external_pool_adapter_scanner_key;
pub(crate) mod external_pool_adapter_scanner_key_api;
pub(crate) mod external_pool_adapter_scanner_key_service;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) mod external_pool_adapter_supervisor_session;
pub(crate) mod external_pool_adapter_supervisor_session_policy_companion;
pub(crate) mod external_pool_adapter_supervisor_session_policy_companion_api;
pub(crate) mod external_pool_adapter_supervisor_session_policy_companion_service;
mod external_pool_adapter_supervisor_session_policy_companion_service_redaction;
mod external_pool_adapter_supervisor_session_policy_companion_service_validation;
#[cfg(test)]
mod external_pool_adapter_supervisor_session_source_contract_tests;
pub(crate) mod external_pool_adapter_task_protocol_conformance;
pub(crate) mod external_pool_adapter_task_protocol_conformance_api;
pub(crate) mod external_pool_adapter_task_protocol_conformance_service;
mod external_pool_adapter_task_protocol_conformance_service_redaction;
mod external_pool_adapter_task_protocol_conformance_service_validation;
pub(crate) mod external_pool_adapter_task_protocol_production;
pub(crate) mod external_pool_adapter_task_worker;
pub(crate) mod external_pool_adapter_upstream_transport_target;
pub(crate) mod external_pool_adapter_upstream_transport_target_api;
pub(crate) mod external_pool_adapter_upstream_transport_target_service;
mod external_pool_adapter_upstream_transport_target_service_redaction;
mod external_pool_adapter_upstream_transport_target_service_validation;
pub(crate) mod external_pool_adapter_vulnerability_reattestation;
pub(crate) mod external_pool_adapter_vulnerability_reattestation_api;
pub(crate) mod external_pool_adapter_vulnerability_reattestation_service;
pub(crate) mod external_pool_onboarding;
pub(crate) mod external_pool_onboarding_api;
pub(crate) mod external_pool_onboarding_mcp;
pub(crate) mod external_pool_onboarding_service;
pub(crate) mod external_pool_provider_activation_candidate;
pub(crate) mod external_pool_provider_activation_candidate_api;
pub(crate) mod external_pool_provider_activation_candidate_service;
mod external_pool_provider_activation_candidate_service_redaction;
pub(crate) mod federation_historical_causal_reference;
pub(crate) mod federation_historical_lineage_read;
pub(crate) mod interactive_desktop;
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
pub(crate) mod user_node_provider_binding;
#[cfg(test)]
mod user_node_provider_binding_source_contract_tests;
pub(crate) mod user_node_ready_source_lineage;
#[cfg(test)]
mod user_node_ready_source_lineage_source_contract_tests;
pub(crate) mod workload;
