const HOST_MODULE: &str = include_str!("mod.rs");
const MANAGED_FACADE: &str = include_str!("../node_agent_managed_fs.rs");
const MANAGED_DISCOVERY: &str =
    include_str!("../node_agent_managed_fs/loader_launch_path_discovery.rs");
const WINDOWS_DISCOVERY: &str =
    include_str!("../node_agent_managed_fs/windows_loader_launch_path_discovery.rs");
const WINDOWS_FACADE: &str = include_str!("../node_agent_managed_fs/windows.rs");
const UNSUPPORTED: &str = include_str!("../node_agent_managed_fs/unsupported.rs");
const STAGING: &str = include_str!("fetch_file/staging.rs");
const ARCHIVE: &str = include_str!("candidate_extraction/zip/types.rs");
const LOADER_FACADE: &str = include_str!("runtime_loader_load_set.rs");
const RUNTIME_DISCOVERY: &str = include_str!("runtime_loader_load_set/launch_path_discovery.rs");
const EXACT_CONTEXT: &str =
    include_str!("runtime_loader_load_set/launch_path_discovery/exact_context_plan.rs");
const EXACT_CONTEXT_BINDING: &str =
    include_str!("runtime_loader_load_set/launch_path_discovery/exact_context_plan/binding.rs");
const EXACT_CONTEXT_DIGEST: &str =
    include_str!("runtime_loader_load_set/launch_path_discovery/exact_context_plan/digest.rs");
const EXACT_CONTEXT_EDGE_LOCATOR: &str = include_str!(
    "runtime_loader_load_set/launch_path_discovery/exact_context_plan/edge_locator.rs"
);
const EXACT_CONTEXT_INTENT: &str =
    include_str!("runtime_loader_load_set/launch_path_discovery/exact_context_plan/intent.rs");
const EXACT_CONTEXT_LINEAGE: &str =
    include_str!("runtime_loader_load_set/launch_path_discovery/exact_context_plan/lineage.rs");
const EXACT_CONTEXT_VIEW: &str =
    include_str!("runtime_loader_load_set/launch_path_discovery/exact_context_plan/view.rs");
const PRELEASE_PE: &str =
    include_str!("runtime_loader_load_set/launch_path_discovery/prelease_pe_material.rs");
const PRELEASE_PE_DIGEST: &str =
    include_str!("runtime_loader_load_set/launch_path_discovery/prelease_pe_material/digest.rs");
const PRELEASE_PE_CLOSURE: &str =
    include_str!("runtime_loader_load_set/launch_path_discovery/prelease_pe_material/closure.rs");
const POLICY: &str = include_str!("runtime_loader_load_set/policy.rs");
const RESOLUTION: &str = include_str!("runtime_loader_load_set/resolution.rs");
const GRANT_READY: &str = include_str!("runtime_loader_load_set/resolution/grant_ready.rs");
const GRANT_READY_DIGEST: &str =
    include_str!("runtime_loader_load_set/resolution/grant_ready/digest.rs");
const GRANT_READY_VALIDATION: &str =
    include_str!("runtime_loader_load_set/resolution/grant_ready/validation.rs");
const TRANSITION: &str = include_str!("runtime_loader_load_set/transition.rs");
const LOADER_MODEL: &str = include_str!("runtime_loader_load_set/model.rs");
const FAILURE: &str = include_str!("runtime_loader_load_set/failure.rs");
const SYSTEM_LEASE_FAILURE: &str =
    include_str!("runtime_loader_load_set/failure/system_content_lease.rs");
const MANAGED_LOADER: &str = include_str!("../node_agent_managed_fs/loader.rs");
const MANAGED_SYSTEM_CUSTODY: &str =
    include_str!("../node_agent_managed_fs/loader/system_image_custody.rs");
const MANAGED_SYSTEM_VALIDATION: &str =
    include_str!("../node_agent_managed_fs/loader_system_validation.rs");

fn between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    source
        .split_once(start)
        .unwrap_or_else(|| panic!("source start marker missing: {start}"))
        .1
        .split_once(end)
        .unwrap_or_else(|| panic!("source end marker missing: {end}"))
        .0
}

#[test]
fn source_routes_a_private_launch_path_candidate_discovery_layer() {
    assert!(HOST_MODULE.contains("mod runtime_loader_launch_path_discovery_source_contract_tests;"));
    assert!(MANAGED_FACADE.contains("mod loader_launch_path_discovery;"));
    assert!(WINDOWS_FACADE.contains("mod loader_launch_path_discovery;"));
    assert!(LOADER_FACADE.contains("mod launch_path_discovery;"));
    assert!(RUNTIME_DISCOVERY.contains("mod exact_context_plan;"));
    assert!(RUNTIME_DISCOVERY.contains("mod prelease_pe_material;"));
    assert_eq!(
        EXACT_CONTEXT
            .lines()
            .filter(|line| line.trim_start().starts_with("mod "))
            .count(),
        6
    );
    assert!(EXACT_CONTEXT.contains("mod lineage;"));
    assert!(EXACT_CONTEXT.contains("mod view;"));
    assert!(GRANT_READY.contains("mod digest;"));
    assert!(GRANT_READY.contains("mod validation;"));
    assert!(MANAGED_LOADER.contains("mod system_image_custody;"));
    assert_eq!(
        PRELEASE_PE
            .lines()
            .filter(|line| line.trim_start().starts_with("mod "))
            .count(),
        2
    );
    for name in [
        "ManagedLoaderLaunchPathComponentDiscovery",
        "ManagedLoaderLaunchPathDiscoveryReceipt",
        "ManagedLoaderLaunchPathDiscoverySet",
        "ManagedLoaderPlanDirectoryLaunchPathDiscovery",
    ] {
        assert!(MANAGED_FACADE.contains(name), "missing managed type {name}");
    }
}

#[test]
fn exact_working_directory_selection_has_no_ambient_default_or_producer() {
    assert!(EXACT_CONTEXT.contains("enum AuthenticatedWindowsRunnerWorkingDirectorySelector"));
    assert!(EXACT_CONTEXT.contains("PackageRoot,"));
    assert!(EXACT_CONTEXT.contains("PlanDirectory {"));
    assert!(EXACT_CONTEXT.contains("directory_ordinal: usize"));
    assert!(EXACT_CONTEXT.contains("relative_path: String"));
    assert!(EXACT_CONTEXT
        .contains("_authenticated_launch_context_source_producer_unavailable: Infallible"));
    assert!(EXACT_CONTEXT_BINDING
        .contains("plan_directories.get(*directory_ordinal) != Some(relative_path)"));
    assert!(EXACT_CONTEXT_BINDING.contains("managed.application()"));
    assert!(EXACT_CONTEXT_BINDING.contains("managed.package_root()"));
    assert!(!EXACT_CONTEXT_BINDING.contains(".first()"));
    assert!(EXACT_CONTEXT_BINDING.contains("fn select_application_directory<'managed>("));
    assert!(EXACT_CONTEXT_BINDING.contains("parent_relative_path == \".\""));
    assert!(EXACT_CONTEXT_BINDING
        .contains("application_file.binding().1 != application_directory_binding.1"));
    assert!(EXACT_CONTEXT_BINDING.contains(".zip(application_directory_components)"));
    assert!(EXACT_CONTEXT_BINDING.contains("from_file.3 != from_directory.3"));
    assert!(EXACT_CONTEXT_BINDING.contains(
        "let application_directory_component_set_digest = application_directory_binding.3"
    ));
    assert!(!EXACT_CONTEXT_DIGEST.contains("application_directory_component_set_digest("));
    assert!(!EXACT_CONTEXT_INTENT.contains("Default"));
}

#[test]
fn prelease_pe_material_is_exact_but_predicts_no_grant_or_lease_generation() {
    for required in [
        "imported_symbol_name",
        "imported_symbol_ordinal",
        "descriptor_ordinal",
        "thunk_ordinal",
        "canonical_merge_ordinal",
        "source_export_name",
        "source_export_ordinal",
        "target_symbol_name",
        "target_symbol_ordinal",
        "hop_evidence_digest",
        "reachable_set_digest",
        "module_cache_collision_closure_digest",
        "importer_edge_ordinal",
        "normalized_module_name",
        "process_machine_context_digest",
        "_authenticated_prelease_pe_parser_producer_unavailable: Infallible",
    ] {
        assert!(
            PRELEASE_PE.contains(required),
            "missing PE material {required}"
        );
    }
    assert!(PRELEASE_PE.contains("find(|image| image.package_file_ordinal == runner_file_ordinal)"));
    assert!(PRELEASE_PE.contains("runner_image.parsed_image_ordinal"));
    assert!(PRELEASE_PE
        .contains("normal_then_delay_import_edges_then_forwarder_hops_by_source_edge_and_hop_v2"));
    assert!(PRELEASE_PE.contains("runner_image.parsed_image_ordinal != 0"));
    assert!(PRELEASE_PE.contains("!same_symbol("));
    assert!(PRELEASE_PE.contains("edge.importer_edge_ordinal"));
    assert!(PRELEASE_PE.contains("reachable_set_digest("));
    assert!(PRELEASE_PE_DIGEST.contains("ELON_WINDOWS_PE_REACHABLE_SET_V1"));
    assert!(PRELEASE_PE_DIGEST.contains("ELON_WINDOWS_PE_MODULE_CACHE_COLLISION_CLOSURE_V1"));
    assert!(PRELEASE_PE.contains("self.maximum_observed_forwarder_depth()"));
    assert!(PRELEASE_PE.contains("self.forwarders_are_contiguous()"));
    assert!(PRELEASE_PE.contains("dll_module_name_is_canonical"));
    assert!(PRELEASE_PE
        .contains("self.package_images[hop.source_image_ordinal].normalized_module_name"));
    assert!(PRELEASE_PE_CLOSURE.contains("validate_external_leaf_coverage"));
    assert!(PRELEASE_PE_CLOSURE.contains("cycle_check_receipt_digest"));
    assert!(PRELEASE_PE_CLOSURE.contains("authenticated_module_cache_collision_receipt_digest"));
    assert!(PRELEASE_PE_DIGEST.contains("ELON_WINDOWS_RUNNER_PRELEASE_PE_MATERIAL_V1"));
    assert!(PRELEASE_PE_DIGEST.contains("ELON_WINDOWS_PE_EDGE_MERGE_RULE_V1"));
    assert!(PRELEASE_PE_DIGEST.contains("self.0.update([1])"));
    assert!(PRELEASE_PE_DIGEST.contains("None => self.0.update([0])"));
    for exact_cross_binding in [
        "let planned = envelope",
        "let observed =",
        "let retained = view",
        "image.relative_path() != planned.relative_path",
        "image.sealed_file_digest() != observed.digest",
        "image.file_identity_digest() != retained.identity_digest()",
    ] {
        assert!(
            EXACT_CONTEXT_BINDING.contains(exact_cross_binding),
            "missing package-image cross-binding {exact_cross_binding}"
        );
    }
    for forbidden in [
        "ManagedLoaderSearchedNameGrant",
        "ManagedLoaderFileContentLease",
        "SealedWindowsPeImportGraphAuthority",
        "SealedWindowsLoaderLaunchPathAuthority",
        "SealedWindowsLoaderResolutionAuthority",
        "lease_generation",
    ] {
        assert!(
            !PRELEASE_PE.contains(forbidden),
            "prelease escape: {forbidden}"
        );
        assert!(
            !PRELEASE_PE_DIGEST.contains(forbidden),
            "prelease digest escape: {forbidden}"
        );
    }
}

#[test]
fn preliminary_request_plan_retains_whole_linear_owners_without_pretending_to_be_grant_ready() {
    for required in [
        "struct PreliminaryResolutionRequestsPlannedWork<'root>",
        "discovered: LaunchPathDiscoveredWork<'root>",
        "context: AuthenticatedWindowsRunnerLaunchContextIntent",
        "pe_material: AuthenticatedWindowsPreLeasePeMaterial",
        "plan: PreliminaryWindowsRunnerResolutionRequestPlan",
        "match binding::bind_preliminary_request_plan(&discovered, &context, &pe_material)",
    ] {
        assert!(
            EXACT_CONTEXT.contains(required),
            "missing owner edge {required}"
        );
    }
    for required in [
        "WindowsPreliminarySearchDirectoryRole::ApplicationDirectory",
        "WindowsPreliminarySearchDirectoryRole::CurrentDirectory",
        "WindowsPreliminarySearchDirectoryTarget::ExternalTypedOwnerRequired",
        "resolution_route_order",
        "exact_terminal_and_step_dispositions_required_before_grant",
        ".import_edges()",
        "ordered_search_step_ordinals: ordered_search_step_ordinals.clone()",
        "global_import_edge_ordinal: request_ordinal",
        "importer_graph_edge_ordinal",
        "WindowsPreliminaryModuleEdgeLocator::Import",
        "WindowsPreliminaryModuleEdgeLocator::Forwarder",
        "edge_evidence_digest: edge.edge_evidence_digest().to_owned()",
        "hop_evidence_digest: hop.hop_evidence_digest().to_owned()",
        "append_component_requests(&mut requests, \"application\"",
        "append_component_requests(&mut requests, \"working_directory\"",
        "(0..package_file_count)",
    ] {
        assert!(
            EXACT_CONTEXT_BINDING.contains(required),
            "missing plan edge {required}"
        );
    }
    assert!(
        EXACT_CONTEXT_DIGEST.contains("ELON_WINDOWS_RUNNER_PRELIMINARY_RESOLUTION_REQUEST_PLAN_V1")
    );
    assert!(!EXACT_CONTEXT_BINDING.contains("unresolved_present_absent_or_shadow_request"));
    assert!(!EXACT_CONTEXT.contains("SystemLookup"));
    assert!(!EXACT_CONTEXT_BINDING.contains("SystemLookup"));
    for source in [
        EXACT_CONTEXT,
        EXACT_CONTEXT_BINDING,
        EXACT_CONTEXT_DIGEST,
        EXACT_CONTEXT_INTENT,
    ] {
        for forbidden in [
            "ManagedLoaderSearchedNameGrant",
            "ManagedLoaderFileContentLease",
            "SealedWindowsPeImportGraphAuthority",
            "SealedWindowsLoaderLaunchPathAuthority",
            "SealedWindowsLoaderResolutionAuthority",
            "LoaderLockedWorkAdmittedPluginSlot",
            "into_parts(self)",
            "Serialize",
            "Deserialize",
        ] {
            assert!(
                !source.contains(forbidden),
                "preliminary escape: {forbidden}"
            );
        }
    }
}

#[test]
fn grant_ready_plan_is_private_typed_and_splits_movable_owners_before_final_resolution() {
    for required in [
        "struct WindowsGrantReadySearchDirectoryPlanStep",
        "location: WindowsPreliminaryRetainedDirectoryLocation",
        "external_owner_ordinal: usize",
        "struct WindowsGrantReadySearchedNameDispositionRecord",
        "searched_name_ordinal: usize",
        "ShadowedByEarlierName",
        "struct WindowsGrantReadyApiSetResolution",
        "host_terminal: WindowsGrantReadyNonRecursiveModuleTerminalRef",
        "struct WindowsGrantReadyResolvedFilesystemSystemImageRequest",
        "primary_use_ordinal: usize",
        "uses: Vec<WindowsGrantReadyResolvedSystemImageUse>",
        "struct GrantReadyWindowsRunnerMovableOwnerSet",
        "pending_system_image_candidates",
    ] {
        assert!(
            GRANT_READY.contains(required),
            "missing grant-ready edge {required}"
        );
    }
    let prerequisite = between(
        GRANT_READY,
        "pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct GrantReadyWindowsRunnerResolutionPrerequisite<",
        "/// Name-grant success consumes",
    );
    for required in [
        "preliminary: PreliminaryResolutionRequestsPlannedWork<'root>",
        "plan: GrantReadyWindowsRunnerResolutionPlan",
        "movable_owners: GrantReadyWindowsRunnerMovableOwnerSet",
        "_grant_ready_resolution_producer_unavailable: Infallible",
    ] {
        assert!(prerequisite.contains(required));
    }
    assert!(!prerequisite.contains("pub(in"));
    assert!(GRANT_READY.contains("struct GrantAcquiredWindowsRunnerResolutionLeaseCustody"));
    assert!(RESOLUTION
        .contains("resolution_custody: GrantAcquiredWindowsRunnerResolutionLeaseCustody<'root>"));
    assert!(GRANT_READY.contains("struct PostLeaseWindowsRunnerResolutionLineage<"));
    assert!(GRANT_READY.contains("'root,"));
    assert!(
        RESOLUTION.contains("resolution_lineage: PostLeaseWindowsRunnerResolutionLineage<'root>")
    );
    assert!(
        RESOLUTION.contains("postlease_lineage: PostLeaseWindowsRunnerResolutionLineage<'root>")
    );
    assert!(GRANT_READY_VALIDATION.contains("fn validate_against("));
    assert!(GRANT_READY_VALIDATION.contains("terminal_reaches_system_request"));
    assert!(GRANT_READY_VALIDATION.contains("resolved_file_identities.insert"));
    assert!(GRANT_READY_VALIDATION
        .contains("WindowsGrantReadySearchedNameDisposition::ShadowedByEarlierName"));
    assert!(GRANT_READY_VALIDATION.contains("earlier_searched_name_ordinal: _"));
    assert!(EXACT_CONTEXT_EDGE_LOCATOR.contains("source_export_name: Option<String>"));
    assert!(EXACT_CONTEXT_EDGE_LOCATOR.contains("hop_evidence_digest: String"));
    assert!(GRANT_READY_DIGEST.contains("ELON_WINDOWS_GRANT_READY_RESOLUTION_PLAN_V1"));
    assert!(GRANT_READY_DIGEST.contains("validate_digests("));
}

#[test]
fn query_success_retains_the_same_typed_lineage_through_process_projection() {
    for required in [
        "struct QueryVerifiedWindowsRunnerLaunchLineage",
        "context: AuthenticatedWindowsRunnerLaunchContextIntent",
        "candidates: WindowsRunnerLaunchPathCandidateSet",
        "pe_material: AuthenticatedWindowsPreLeasePeMaterial",
        "plan: PreliminaryWindowsRunnerResolutionRequestPlan",
        "grant_ready_plan: GrantReadyWindowsRunnerResolutionPlan",
        "struct QueryVerifiedWindowsRunnerLaunchLineageValidationFailure",
        "prerequisite: SealedWindowsRunnerLoadSetPrerequisite<'root>",
        "fn consume_query_verified_loader_prerequisite",
        "validate_query_verified_lineage(&prerequisite)",
        "postlease_lineage.into_parts()",
        "validate_loader_image_binding",
        "validate_process_projection",
    ] {
        assert!(
            EXACT_CONTEXT_LINEAGE.contains(required),
            "missing lineage edge {required}"
        );
    }
    assert!(EXACT_CONTEXT_LINEAGE.contains(
        "resolution.launch_context_selector_digest != preliminary.context.context_intent_digest"
    ));
    assert!(EXACT_CONTEXT_LINEAGE.contains("resolution.selected_context_binding_digest"));
    assert!(EXACT_CONTEXT_LINEAGE.contains("pre_post_cross_binding"));
    assert!(TRANSITION.contains("consume_query_verified_loader_prerequisite(prerequisite)?"));
    assert!(LOADER_MODEL.contains("authenticated_launch_lineage:"));
    assert!(!EXACT_CONTEXT_LINEAGE.contains("Clone"));
    assert!(!EXACT_CONTEXT_LINEAGE.contains("Serialize"));
}

#[test]
fn system_image_candidate_attempt_and_response_are_one_linear_authenticated_graph() {
    for required in [
        "struct PinnedWindowsLoaderResolvedSystemImageCandidate",
        "resolved_component_identity_digest",
        "concrete_servicing_generation_digest",
        "servicing_resolution_receipt_digest",
        "struct ManagedLoaderSystemImageContentLeaseAcquisitionAttemptCustody",
        "_authenticated_lease_session: File",
        "candidate: PinnedWindowsLoaderResolvedSystemImageCandidate",
        "struct ManagedLoaderSystemImageContentLeasePositiveOutcomeCustody",
    ] {
        assert!(
            MANAGED_SYSTEM_CUSTODY.contains(required),
            "missing system custody {required}"
        );
    }
    for required in [
        "attempt.candidate.binding_is_self_consistent()",
        "self.authenticated_response == attempt.response_buffer",
        "hex::encode(Sha256::digest(&self.authenticated_response))",
        "system_image_negative_receipt_digest(",
        "ELON_MANAGED_LOADER_RESOLVED_SYSTEM_IMAGE_CANDIDATE_V1",
        "ELON_MANAGED_LOADER_SYSTEM_IMAGE_CONTENT_LEASE_NEGATIVE_V1",
    ] {
        assert!(
            MANAGED_SYSTEM_VALIDATION.contains(required),
            "missing response check {required}"
        );
    }
    assert!(FAILURE.contains("ResolvedFilesystemSystemImagePositiveOutcome"));
    assert!(SYSTEM_LEASE_FAILURE.contains("reject_system_image_acquisition"));
    assert!(SYSTEM_LEASE_FAILURE.contains("system_image_positive_outcome_uncertain"));
}

#[test]
fn authenticated_nested_policies_recompute_actual_values_and_freeze_exact_order() {
    for domain in [
        "ELON_WINDOWS_RUNNER_LAUNCH_CONTEXT_PAYLOAD_V1",
        "ELON_WINDOWS_RUNNER_AUTHENTICATED_LAUNCH_CONTEXT_BINDING_V1",
        "ELON_WINDOWS_PROCESS_MACHINE_EXPECTATION_V1",
        "ELON_WINDOWS_DLL_SEARCH_POLICY_V1",
        "ELON_WINDOWS_PROCESS_CREATION_POLICY_V1",
        "ELON_WINDOWS_LAUNCH_SECURITY_EXPECTATION_V1",
    ] {
        assert!(
            EXACT_CONTEXT_INTENT.contains(domain),
            "missing digest domain {domain}"
        );
    }
    assert_eq!(
        EXACT_CONTEXT_INTENT
            .matches("self.recompute_digest() !=")
            .count(),
        4
    );
    assert!(EXACT_CONTEXT_INTENT
        .contains("self.recompute_payload_digest() != self.selection_payload_digest"));
    assert!(EXACT_CONTEXT_INTENT
        .contains("self.recompute_authenticated_binding_digest() != self.context_intent_digest"));
    for exact_value in [
        "routes.as_slice() != REQUIRED_RESOLUTION_ROUTES",
        "self.search_order[0]",
        "self.search_order[self.search_order.len() - 1]",
        "phases.windows(2)",
        "flags.as_slice() != REQUIRED_PROCESS_CREATION_FLAGS",
        "self.environment_policy != EMPTY_ENVIRONMENT_POLICY",
        "digest.text(&self.wow64_mode)",
        "digest.boolean(self.restricted_token_required)",
        "digest.boolean(self.app_container_required)",
    ] {
        assert!(
            EXACT_CONTEXT_INTENT.contains(exact_value),
            "missing authenticated policy value {exact_value}"
        );
    }
}

#[test]
fn managed_api_accepts_only_exact_retained_owner_types() {
    let entry = between(
        MANAGED_DISCOVERY,
        "pub(crate) fn discover_loader_launch_path_candidates(",
        ") -> Result<ManagedLoaderLaunchPathDiscoverySet>",
    );
    for required in [
        "application: &PinnedManagedFile",
        "package_root: &PinnedManagedExtractionLoaderDirectory",
        "plan_directories: &[PinnedManagedDirectory]",
    ] {
        assert!(entry.contains(required), "missing typed input {required}");
    }
    for forbidden in ["Path", "&str", "String", ": &File", ": File", "RawHandle"] {
        assert!(
            !entry.contains(forbidden),
            "scalar/raw input escape: {forbidden}"
        );
    }
    assert!(MANAGED_DISCOVERY.contains("require_handle_prefix("));
    assert!(MANAGED_DISCOVERY.contains("Arc::ptr_eq(left, right)"));
    assert!(MANAGED_DISCOVERY.contains("NODE_MANAGED_LOADER_LAUNCH_PATH_ROOT_CHANGED"));
    assert!(MANAGED_DISCOVERY.contains("NODE_MANAGED_LOADER_LAUNCH_PATH_DIRECTORY_DUPLICATED"));
}

#[test]
fn windows_observes_access_identity_and_single_handle_path_components() {
    assert!(WINDOWS_DISCOVERY.contains("NtQueryInformationFile("));
    assert!(WINDOWS_DISCOVERY.contains("FileAccessInformation"));
    assert!(WINDOWS_DISCOVERY.contains("validate_regular_file_identity(identity, volume)"));
    assert!(WINDOWS_DISCOVERY.contains("validate_directory_identity(identity, Some(volume))"));
    assert!(WINDOWS_DISCOVERY.contains("super::canonical_path(handle)"));
    assert!(WINDOWS_DISCOVERY.contains("single_child_component(&parent_path, &canonical)"));
    assert!(WINDOWS_DISCOVERY.contains("Component::Normal(name)"));
    assert!(WINDOWS_DISCOVERY.contains("NODE_MANAGED_LOADER_LAUNCH_PATH_COMPONENT_NOT_SINGLE"));
    assert!(WINDOWS_DISCOVERY.contains("FILE_READ_ATTRIBUTES"));
    assert!(WINDOWS_DISCOVERY.contains("FILE_READ_DATA"));
    assert!(WINDOWS_DISCOVERY.contains("FILE_TRAVERSE"));
    assert!(MANAGED_DISCOVERY.contains("not an exact opener recipe or dynamic share evidence"));
    assert!(UNSUPPORTED.contains("fn discover_loader_directory_launch_path("));
    assert!(UNSUPPORTED.contains("fn discover_loader_file_launch_path("));
}

#[test]
fn runtime_binds_runner_and_returns_admission_custody_on_both_branches() {
    assert!(STAGING.contains("fn loader_launch_path_package_root("));
    assert!(ARCHIVE.contains("struct ExtractedComputePluginLaunchPathDiscoveryView"));
    for input in ["plan", "evidence", "package_root", "directories", "files"] {
        assert!(
            ARCHIVE.contains(input),
            "missing archive discovery input {input}"
        );
    }
    assert!(RUNTIME_DISCOVERY.contains("fn discover_windows_runner_launch_path_candidates<'root>("));
    assert!(RUNTIME_DISCOVERY.contains("admitted: DurableWorkAdmittedPluginSlot<'root>"));
    assert!(RUNTIME_DISCOVERY.contains("LaunchPathDiscoveredWork"));
    assert!(RUNTIME_DISCOVERY.contains("LaunchPathDiscoveryFailure"));
    assert!(RUNTIME_DISCOVERY
        .contains("Err(error) => Err(LaunchPathDiscoveryFailure { error, admitted })"));
    assert!(RUNTIME_DISCOVERY.contains("planned.expected_digest != observed.digest"));
    assert!(
        RUNTIME_DISCOVERY.contains("retained.identity_digest() != observed.file_identity_digest")
    );
    assert!(RUNTIME_DISCOVERY.contains("let mut runner_ordinals"));
    assert!(RUNTIME_DISCOVERY.contains("view.package_root()"));
    assert!(RUNTIME_DISCOVERY.contains("view.directories()"));
    assert!(RUNTIME_DISCOVERY.contains("fn receipt_matches_relative_path("));
    assert!(RUNTIME_DISCOVERY.contains("component.binding().2 == expected"));
    assert!(!RUNTIME_DISCOVERY.contains("selected_working_directory"));
    assert!(!RUNTIME_DISCOVERY.contains("impl<'root> LaunchPathDiscoveredWork"));
}

#[test]
fn discovery_types_do_not_clone_serialize_or_expose_raw_handles() {
    for forbidden in [
        "#[derive(Clone",
        "#[derive(Copy",
        "Serialize",
        "Deserialize",
        "fn as_raw_handle(",
        "fn into_raw_handle(",
        "fn as_file(",
        "fn into_file(",
        "pub path:",
        "pub file:",
    ] {
        assert!(
            !MANAGED_DISCOVERY.contains(forbidden),
            "managed escape: {forbidden}"
        );
        assert!(
            !RUNTIME_DISCOVERY.contains(forbidden),
            "runtime escape: {forbidden}"
        );
    }
    assert!(!RUNTIME_DISCOVERY.contains("serde::"));
    assert!(!MANAGED_DISCOVERY.contains("serde::"));
}

#[test]
fn exact_authority_gaps_and_zero_effects_remain_closed() {
    assert!(RESOLUTION.contains("struct SealedWindowsLoaderLaunchPathAuthority"));
    assert!(RESOLUTION.contains("_launch_path_grant_or_share_backend_unavailable: Infallible"));
    for blocker in [
        "launch_path_handle_chain_discovery",
        "source_written_windows_dynamic_unverified",
        "launch_context_selection_contract",
        "source_written_uncompiled_unrun",
        "authenticated_launch_context_source_producer",
        "prelease_authenticated_pe_material",
        "authenticated_prelease_pe_parser_producer",
        "preliminary_resolution_request_plan",
        "grant_ready_resolution_contract",
        "grant_ready_resolution_producer",
        "external_search_directory_authority",
        "launch_path_component_grant_backend",
        "postlease_exact_pe_import_graph_sealer",
        "postlease_same_owner_lineage_contract",
        "final_namespace_query_backend",
    ] {
        assert!(POLICY.contains(blocker), "missing blocker {blocker}");
    }
    assert!(POLICY.contains("discover_retained_launch_path_candidates_and_prelease_pe_material"));
    assert!(
        POLICY.contains("authenticate_exact_launch_context_and_preliminary_resolution_requests")
    );
    assert!(POLICY.contains("resolve_exact_terminals_dispositions_and_external_directory_owners"));
    assert!(POLICY.contains("seal_grant_ready_preliminary_resolution_plan"));
    assert!(POLICY.contains(
        "same_handle_full_package_rehash_and_reparse_under_content_leases_and_name_grants"
    ));
    assert!(POLICY
        .contains("seal_exact_pe_graph_launch_path_and_startup_import_resolution_under_leases"));
    let zero_effects = between(
        POLICY,
        "pub(super) const LOADER_LOAD_SET_ZERO_EFFECTS",
        "pub(super) const LOADER_LOAD_SET_AUTHORITY_GAPS",
    );
    let authority_gaps = between(
        POLICY,
        "pub(super) const LOADER_LOAD_SET_AUTHORITY_GAPS",
        "pub(super) const DYNAMIC_MODULE_LOAD_AUTHORITY",
    );
    assert_eq!(zero_effects.matches("(\"runtime_").count(), 5);
    assert_eq!(zero_effects.matches("\", \"none\")").count(), 18);
    assert_eq!(authority_gaps.matches("\", \"missing\")").count(), 4);
}
