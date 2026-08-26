//! Typed exact resolution custody required before any name grant or system-image lease dispatch.
//!
//! The immutable plan is deliberately separate from movable directory/candidate owners. A future
//! backend must consume this module's purpose-specific typestate seams; sibling modules cannot
//! construct, splice or extract its private fields.

mod digest;
mod final_projection;
mod search_projection;
mod validation;

use std::convert::Infallible;

use anyhow::Result;

use crate::node_agent_managed_fs::{
    PinnedWindowsLoaderResolvedSystemImageCandidate, PinnedWindowsLoaderSearchDirectory,
};

use super::super::launch_path_discovery::{
    PreliminaryResolutionRequestsPlannedWork, PreliminaryWindowsRunnerResolutionRequestPlanView,
    WindowsPreliminaryModuleEdgeLocator, WindowsPreliminaryRetainedDirectoryLocation,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum WindowsGrantReadyImportEdgeKind {
    Normal,
    Delay,
    Forwarder,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WindowsGrantReadySystemImageUseRoute {
    OrdinaryFilesystem,
    SideBySide,
}

enum WindowsGrantReadySearchDirectoryTarget {
    RetainedPreliminaryCandidate {
        location: WindowsPreliminaryRetainedDirectoryLocation,
        preliminary_search_step_ordinal: usize,
        preliminary_binding_digest: String,
    },
    ExternalDirectory {
        external_owner_ordinal: usize,
    },
}

struct WindowsGrantReadySearchDirectoryPlanStep {
    search_step_ordinal: usize,
    role: String,
    target: WindowsGrantReadySearchDirectoryTarget,
    authority_binding_digest: String,
}

#[must_use = "external search-directory owner must move into final resolution or failure custody"]
struct WindowsGrantReadyExternalSearchDirectoryCustody {
    external_owner_ordinal: usize,
    search_step_ordinal: usize,
    directory: PinnedWindowsLoaderSearchDirectory,
    handle_chain_authority_digest: String,
    namespace_alias_currentness_receipt_digest: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WindowsGrantReadyNonRecursiveModuleTerminalRef {
    AuthenticatedPreloaded {
        preloaded_authority_record_ordinal: usize,
    },
    PackageFile {
        package_file_ordinal: usize,
        parsed_image_ordinal: usize,
    },
    KnownDllSection {
        known_dll_authority_record_ordinal: usize,
    },
    ResolvedFilesystemSystemImage {
        resolution_request_ordinal: usize,
    },
    SideBySideSystemImage {
        resolution_request_ordinal: usize,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WindowsGrantReadyModuleTerminalRef {
    NonRecursive(WindowsGrantReadyNonRecursiveModuleTerminalRef),
    ApiSetResolution { api_set_resolution_ordinal: usize },
}

struct WindowsGrantReadyAuthenticatedPreloadedTerminalAuthority {
    authority_record_ordinal: usize,
    preloaded_module_ordinal: usize,
    module_cache_key: String,
    component_identity_digest: String,
    immutable_section_identity_digest: String,
    authenticated_evidence_digest: String,
}

struct WindowsGrantReadyKnownDllTerminalAuthority {
    authority_record_ordinal: usize,
    module_cache_key: String,
    section_identity_digest: String,
    component_identity_digest: String,
    immutable_section_identity_digest: String,
    section_image_mapping_receipt_digest: String,
    section_namespace_generation_digest: String,
}

/// API-set host resolution is deliberately non-recursive in this policy version. Nested API-set
/// host redirection remains fail-closed until a future typed DAG contract is introduced.
struct WindowsGrantReadyApiSetResolution {
    api_set_resolution_ordinal: usize,
    normalized_contract_name: String,
    normalized_host_module_cache_key: String,
    host_terminal: WindowsGrantReadyNonRecursiveModuleTerminalRef,
    os_build_identity_digest: String,
    schema_identity_digest: String,
    contract_host_binding_set_digest: String,
    resolution_binding_digest: String,
}

enum WindowsGrantReadySearchedNameDisposition {
    MustRemainAbsent,
    ShadowedByEarlierName {
        earlier_searched_name_ordinal: usize,
    },
    Terminal {
        terminal: WindowsGrantReadyModuleTerminalRef,
    },
}

struct WindowsGrantReadySearchedNameDispositionRecord {
    searched_name_ordinal: usize,
    module_request_ordinal: usize,
    step_position: usize,
    search_step_ordinal: usize,
    normalized_searched_name: String,
    disposition: WindowsGrantReadySearchedNameDisposition,
    grant_request_digest: String,
    disposition_binding_digest: String,
}

struct WindowsGrantReadyModuleResolution {
    request_ordinal: usize,
    global_import_edge_ordinal: usize,
    edge_locator: WindowsPreliminaryModuleEdgeLocator,
    importer_graph_edge_ordinal: usize,
    importer_parsed_image_ordinal: usize,
    import_kind: WindowsGrantReadyImportEdgeKind,
    normalized_requested_name: String,
    imported_symbol_name: Option<String>,
    imported_symbol_ordinal: Option<u16>,
    searched_name_ordinals: Vec<usize>,
    terminal: WindowsGrantReadyModuleTerminalRef,
    resolution_binding_digest: String,
}

struct WindowsGrantReadyResolvedSystemImageUse {
    module_request_ordinal: usize,
    searched_name_ordinal: usize,
    search_step_ordinal: usize,
    normalized_searched_name: String,
    search_directory_authority_binding_digest: String,
    route: WindowsGrantReadySystemImageUseRoute,
}

struct WindowsGrantReadyResolvedFilesystemSystemImageRequest {
    resolution_request_ordinal: usize,
    canonical_dedupe_ordinal: usize,
    candidate_owner_ordinal: usize,
    primary_use_ordinal: usize,
    normalized_name: String,
    search_directory_authority_binding_digest: String,
    resolved_component_identity_digest: String,
    expected_file_identity_digest: String,
    concrete_servicing_generation_digest: String,
    code_integrity_evidence_digest: String,
    servicing_resolution_receipt_digest: String,
    namespace_alias_currentness_receipt_digest: String,
    candidate_binding_digest: String,
    uses: Vec<WindowsGrantReadyResolvedSystemImageUse>,
    lease_request_digest: String,
}

#[must_use = "resolved system candidate must move into one lease attempt or failure custody"]
struct WindowsGrantReadyResolvedSystemImageCandidateCustody {
    candidate_owner_ordinal: usize,
    resolution_request_ordinal: usize,
    candidate: PinnedWindowsLoaderResolvedSystemImageCandidate,
}

pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct GrantReadyWindowsRunnerResolutionPlan
{
    search_directories: Vec<WindowsGrantReadySearchDirectoryPlanStep>,
    preloaded_terminal_authorities: Vec<WindowsGrantReadyAuthenticatedPreloadedTerminalAuthority>,
    known_dll_terminal_authorities: Vec<WindowsGrantReadyKnownDllTerminalAuthority>,
    api_set_resolutions: Vec<WindowsGrantReadyApiSetResolution>,
    searched_name_dispositions: Vec<WindowsGrantReadySearchedNameDispositionRecord>,
    module_resolutions: Vec<WindowsGrantReadyModuleResolution>,
    resolved_filesystem_system_image_requests:
        Vec<WindowsGrantReadyResolvedFilesystemSystemImageRequest>,
    grant_ready_resolution_plan_digest: String,
    exact_terminal_resolution_set_digest: String,
    exact_searched_name_disposition_set_digest: String,
    external_directory_authority_set_digest: String,
    resolved_system_image_request_set_digest: String,
}

impl GrantReadyWindowsRunnerResolutionPlan {
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn digest(
        &self,
    ) -> &str {
        &self.grant_ready_resolution_plan_digest
    }

    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn base_projection_shape(
        &self,
    ) -> (usize, usize, usize) {
        (
            self.module_resolutions.len(),
            self.searched_name_dispositions.len(),
            self.resolved_filesystem_system_image_requests.len(),
        )
    }
}

struct GrantReadyWindowsRunnerMovableOwnerSet {
    external_search_directories: Vec<WindowsGrantReadyExternalSearchDirectoryCustody>,
    pending_system_image_candidates: Vec<WindowsGrantReadyResolvedSystemImageCandidateCustody>,
}

/// Exact terminal/disposition plan plus all linear external owners required before name-grant
/// dispatch. No constructor exists: the unresolved preliminary skeleton cannot fabricate this
/// owner, and all fields stay private to this module's future validator/advancer.
#[must_use = "grant-ready owner must advance whole or remain in failure custody"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct GrantReadyWindowsRunnerResolutionPrerequisite<
    'root,
> {
    preliminary: PreliminaryResolutionRequestsPlannedWork<'root>,
    plan: GrantReadyWindowsRunnerResolutionPlan,
    movable_owners: GrantReadyWindowsRunnerMovableOwnerSet,
    _grant_ready_resolution_producer_unavailable: Infallible,
}

/// Name-grant success consumes the whole grant-ready owner into this lease-acquisition custody.
/// Movable owners remain here until each candidate is consumed by exactly one attempt and each
/// external directory moves into the final resolution.
#[must_use = "post-grant resolution custody must advance through every content lease"]
pub(super) struct GrantAcquiredWindowsRunnerResolutionLeaseCustody<'root> {
    preliminary: PreliminaryResolutionRequestsPlannedWork<'root>,
    plan: GrantReadyWindowsRunnerResolutionPlan,
    movable_owners: GrantReadyWindowsRunnerMovableOwnerSet,
    _grant_acquisition_transition_producer_unavailable: Infallible,
}

/// Lineage left after all movable external directories/candidates have moved into the final
/// resolution. It retains the exact preliminary owner and immutable grant-ready plan, but no
/// handle/file owner duplicated by the final resolution.
#[must_use = "postlease lineage must remain through final namespace query and process custody"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct PostLeaseWindowsRunnerResolutionLineage<
    'root,
> {
    preliminary: PreliminaryResolutionRequestsPlannedWork<'root>,
    plan: GrantReadyWindowsRunnerResolutionPlan,
    _postlease_resolution_sealer_unavailable: Infallible,
}

impl GrantReadyWindowsRunnerResolutionPrerequisite<'_> {
    pub(super) fn borrow_preliminary_requests(
        &self,
    ) -> PreliminaryWindowsRunnerResolutionRequestPlanView<'_> {
        self.preliminary.borrow_resolution_request_plan()
    }

    pub(super) fn validate_whole(&self) -> Result<()> {
        let preliminary = self.borrow_preliminary_requests();
        self.plan.validate_against(&preliminary)?;
        self.movable_owners
            .validate_against(&self.plan, &preliminary)
    }
}

impl<'root> PostLeaseWindowsRunnerResolutionLineage<'root> {
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn borrow_preliminary(
        &self,
    ) -> &PreliminaryResolutionRequestsPlannedWork<'root> {
        &self.preliminary
    }

    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn into_parts(
        self,
    ) -> (
        PreliminaryResolutionRequestsPlannedWork<'root>,
        GrantReadyWindowsRunnerResolutionPlan,
    ) {
        let Self {
            preliminary,
            plan,
            _postlease_resolution_sealer_unavailable: _,
        } = self;
        (preliminary, plan)
    }

    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn grant_ready_plan_digest(
        &self,
    ) -> &str {
        &self.plan.grant_ready_resolution_plan_digest
    }

    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn validate_retained_plans(
        &self,
    ) -> Result<()> {
        self.plan
            .validate_against(&self.preliminary.borrow_resolution_request_plan())
    }

    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn validate_final_system_image_projection(
        &self,
        resolution: &super::SealedWindowsLoaderResolutionAuthority,
    ) -> Result<()> {
        self.plan.validate_final_base_projection(resolution)?;
        let (base_modules, base_names, base_system_images) = self.plan.base_projection_shape();
        let preliminary = self.preliminary.borrow_resolution_request_plan();
        resolution
            .pe_import_graph
            .recursive_resolution_closure
            .validate_against(
                preliminary.package_image_count(),
                base_modules,
                base_names,
                base_system_images,
                preliminary.parser_policy_digest,
                preliminary.selected_context.context_intent_digest,
                resolution,
            )
    }
}
