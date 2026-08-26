//! Authenticated launch-context selection and pre-grant resolution planning.
//!
//! Both authenticated inputs remain uninhabited. The resulting owner is a preliminary request
//! plan, not a grant, lease, final PE graph, launch-path authority or loader successor.

#![allow(dead_code)]

mod binding;
mod digest;
mod edge_locator;
mod intent;
mod lineage;
mod view;

pub(in super::super) use edge_locator::WindowsPreliminaryModuleEdgeLocator;
pub(in crate::node_agent_compute_plugin_host) use lineage::WindowsRunnerLaunchContextPreCreateProjection;
pub(in super::super) use lineage::{
    consume_query_verified_loader_prerequisite, QueryVerifiedWindowsRunnerLaunchLineage,
    QueryVerifiedWindowsRunnerLaunchLineageValidationFailure,
};
pub(in super::super) use view::{
    PreliminaryWindowsRunnerResolutionRequestPlanView, PreliminaryWindowsRunnerSelectedContextView,
};

use std::{convert::Infallible, fmt};

use anyhow::Error;

use crate::node_agent_compute_plugin_host::runtime_loader_load_set::model::WindowsLoaderWorkingDirectoryLocation;

use super::{
    prelease_pe_material::AuthenticatedWindowsPreLeasePeMaterial, LaunchPathDiscoveredWork,
};

pub(super) enum AuthenticatedWindowsRunnerWorkingDirectorySelector {
    PackageRoot,
    PlanDirectory {
        directory_ordinal: usize,
        relative_path: String,
    },
}

pub(super) enum WindowsPreliminarySearchDirectoryRole {
    ApplicationDirectory,
    CurrentDirectory,
    PackageRoot,
    PlanDirectory { directory_ordinal: usize },
    SystemDirectory,
    WindowsDirectory,
    SideBySideAssemblyDirectory,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum WindowsPreliminaryRetainedDirectoryLocation {
    PackageRoot,
    PlanDirectory { directory_ordinal: usize },
}

pub(super) struct AuthenticatedWindowsDllSearchPolicy {
    search_order: Vec<WindowsPreliminarySearchDirectoryRole>,
    resolution_route_order: Vec<String>,
    ambient_path_allowed: bool,
    policy_digest: String,
}

pub(super) struct AuthenticatedWindowsProcessMachineContextExpectation {
    target_architecture: String,
    wow64_mode: String,
    context_policy_digest: String,
}

pub(super) struct AuthenticatedWindowsProcessCreationPolicy {
    inherited_handles: bool,
    environment_policy: String,
    creation_flags: Vec<String>,
    policy_digest: String,
}

pub(super) struct AuthenticatedWindowsRunnerLaunchSecurityExpectation {
    restricted_token_required: bool,
    app_container_required: bool,
    token_profile_policy_digest: String,
}

/// Versioned control-signed intent bound to one admitted work receipt. Manifest/work-admission V1
/// has no CWD selector, so this authority intentionally has no producer in the current source.
pub(in super::super) struct AuthenticatedWindowsRunnerLaunchContextIntent {
    admission_source_digest: String,
    admission_receipt_digest: String,
    manifest_digest: String,
    signed_manifest_envelope_digest: String,
    grant_digest: String,
    target_id: String,
    target_operating_system: String,
    target_architecture: String,
    runner_relative_path: String,
    entrypoint_arguments_digest: String,
    working_directory_selector: AuthenticatedWindowsRunnerWorkingDirectorySelector,
    machine_context: AuthenticatedWindowsProcessMachineContextExpectation,
    dll_search_policy: AuthenticatedWindowsDllSearchPolicy,
    process_creation_policy: AuthenticatedWindowsProcessCreationPolicy,
    launch_security_expectation: AuthenticatedWindowsRunnerLaunchSecurityExpectation,
    control_key_id: String,
    control_keyring_generation: u64,
    selection_payload_digest: String,
    signed_selection_payload_digest: String,
    verified_selection_payload_digest: String,
    signed_selection_envelope_digest: String,
    signature_verification_receipt_digest: String,
    context_intent_digest: String,
    _authenticated_launch_context_source_producer_unavailable: Infallible,
}

struct WindowsRunnerSelectedLaunchContextBinding {
    working_directory_location: WindowsLoaderWorkingDirectoryLocation,
    working_directory_relative_path: String,
    working_directory_identity_digest: String,
    working_directory_component_set_digest: String,
    working_directory_observation_receipt_digest: String,
    application_identity_digest: String,
    application_component_set_digest: String,
    application_observation_receipt_digest: String,
    application_directory_identity_digest: String,
    application_directory_location: WindowsPreliminaryRetainedDirectoryLocation,
    application_directory_component_set_digest: String,
    application_directory_observation_receipt_digest: String,
    context_intent_digest: String,
    selection_binding_digest: String,
}

pub(in super::super) struct WindowsPreliminarySearchDirectoryBinding {
    search_step_ordinal: usize,
    role: String,
    target: WindowsPreliminarySearchDirectoryTarget,
    binding_digest: String,
}

pub(in super::super) enum WindowsPreliminarySearchDirectoryTarget {
    RetainedCandidate {
        location: WindowsPreliminaryRetainedDirectoryLocation,
        identity_digest: String,
        observation_receipt_digest: String,
    },
    ExternalTypedOwnerRequired {
        owner_kind: &'static str,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum WindowsPreliminaryImportEdgeKind {
    Normal,
    Delay,
    Forwarder,
}

pub(in super::super) struct WindowsPreliminaryModuleResolutionRequest {
    request_ordinal: usize,
    global_import_edge_ordinal: usize,
    edge_locator: WindowsPreliminaryModuleEdgeLocator,
    importer_graph_edge_ordinal: usize,
    importer_image_ordinal: usize,
    import_kind: WindowsPreliminaryImportEdgeKind,
    normalized_name: String,
    imported_symbol_name: Option<String>,
    imported_symbol_ordinal: Option<u16>,
    ordered_search_step_ordinals: Vec<usize>,
    grant_ready_resolution_status: &'static str,
}

pub(in super::super) struct WindowsPreliminaryLaunchPathComponentRequest {
    request_ordinal: usize,
    path_kind: &'static str,
    component_ordinal: usize,
    parent_identity_digest: String,
    normalized_component: String,
    expected_object_identity_digest: String,
}

pub(in super::super) enum WindowsPreliminaryContentLeaseRequestRef {
    PackageFile { package_file_ordinal: usize },
}

/// Ordered unresolved request skeleton. A later authenticated resolver must add exact terminal and
/// per-step dispositions before any name grant or system-image lease can be acquired.
pub(in super::super) struct PreliminaryWindowsRunnerResolutionRequestPlan {
    admission_source_digest: String,
    admission_receipt_digest: String,
    extraction_plan_digest: String,
    extraction_evidence_digest: String,
    launch_path_candidate_set_digest: String,
    selected_context: WindowsRunnerSelectedLaunchContextBinding,
    prelease_pe_material_digest: String,
    parser_policy_digest: String,
    authenticated_preloaded_module_set_digest: String,
    resolution_route_order: Vec<String>,
    search_directories: Vec<WindowsPreliminarySearchDirectoryBinding>,
    module_resolution_requests: Vec<WindowsPreliminaryModuleResolutionRequest>,
    launch_path_component_requests: Vec<WindowsPreliminaryLaunchPathComponentRequest>,
    content_lease_requests: Vec<WindowsPreliminaryContentLeaseRequestRef>,
    preliminary_request_plan_digest: String,
}

#[must_use = "preliminary resolution-request custody must advance whole or remain quarantined"]
pub(in super::super) struct PreliminaryResolutionRequestsPlannedWork<'root> {
    discovered: LaunchPathDiscoveredWork<'root>,
    context: AuthenticatedWindowsRunnerLaunchContextIntent,
    pe_material: AuthenticatedWindowsPreLeasePeMaterial,
    plan: PreliminaryWindowsRunnerResolutionRequestPlan,
}

#[must_use = "failed preliminary planning retains every exact source owner"]
pub(super) struct WindowsRunnerLaunchContextPreliminaryResolutionFailure<'root> {
    error: Error,
    discovered: LaunchPathDiscoveredWork<'root>,
    context: AuthenticatedWindowsRunnerLaunchContextIntent,
    pe_material: AuthenticatedWindowsPreLeasePeMaterial,
}

pub(super) fn plan_authenticated_windows_runner_preliminary_resolution_requests<'root>(
    discovered: LaunchPathDiscoveredWork<'root>,
    context: AuthenticatedWindowsRunnerLaunchContextIntent,
    pe_material: AuthenticatedWindowsPreLeasePeMaterial,
) -> std::result::Result<
    PreliminaryResolutionRequestsPlannedWork<'root>,
    WindowsRunnerLaunchContextPreliminaryResolutionFailure<'root>,
> {
    match binding::bind_preliminary_request_plan(&discovered, &context, &pe_material) {
        Ok(plan) => Ok(PreliminaryResolutionRequestsPlannedWork {
            discovered,
            context,
            pe_material,
            plan,
        }),
        Err(error) => Err(WindowsRunnerLaunchContextPreliminaryResolutionFailure {
            error,
            discovered,
            context,
            pe_material,
        }),
    }
}

impl fmt::Debug for PreliminaryResolutionRequestsPlannedWork<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreliminaryResolutionRequestsPlannedWork")
            .field("discovered", &"<retained>")
            .field("context", &"<authenticated-uninhabited>")
            .field("pe_material", &self.pe_material)
            .field(
                "search_directory_count",
                &self.plan.search_directories.len(),
            )
            .field(
                "module_resolution_request_count",
                &self.plan.module_resolution_requests.len(),
            )
            .field("plan_digest", &"<redacted>")
            .finish()
    }
}

impl fmt::Debug for WindowsRunnerLaunchContextPreliminaryResolutionFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WindowsRunnerLaunchContextPreliminaryResolutionFailure")
            .field("error", &self.error)
            .field("owners", &"<discovery-context-pe-retained>")
            .finish()
    }
}
