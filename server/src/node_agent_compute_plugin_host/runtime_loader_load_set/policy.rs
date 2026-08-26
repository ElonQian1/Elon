/// This architecture-only load-set boundary changes local process custody but writes no runtime,
/// Ready, scheduling, execution, usage, settlement, or money authority.
pub(super) const LOADER_LOAD_SET_ZERO_EFFECTS: &[(&str, &str)] = &[
    ("runtime_phase", "none"),
    ("runtime_generation", "none"),
    ("runtime_start", "none"),
    ("runtime_resume", "none"),
    ("runtime_store", "none"),
    ("health", "none"),
    ("readiness", "none"),
    ("node", "none"),
    ("provider", "none"),
    ("route", "none"),
    ("offer", "none"),
    ("capacity", "none"),
    ("execution", "none"),
    ("attempt", "none"),
    ("lease", "none"),
    ("usage", "none"),
    ("settlement", "none"),
    ("money", "none"),
];

/// A corrected loader owner graph is still not Host runtime or Ready authority.
pub(super) const LOADER_LOAD_SET_AUTHORITY_GAPS: &[(&str, &str)] = &[
    ("node_local_authority_currentness", "missing"),
    ("runtime_transition_authority", "missing"),
    ("host_runtime_authority", "missing"),
    ("v15_authenticated_session", "missing"),
];

/// The sealed set is startup/import resolution only. Runtime-derived `LoadLibrary` names remain a
/// process-resume blocker until a module-load enforcement policy and backend exist.
pub(super) const DYNAMIC_MODULE_LOAD_AUTHORITY: &str = "missing_resume_blocker";

/// Source contracts can advance while production reachability remains blocked on dynamic Windows
/// evidence and every real authenticated producer/backend below.
pub(super) const LOADER_LOAD_SET_REACHABILITY_BLOCKERS: &[(&str, &str)] = &[
    (
        "existing_extraction_directory_access_share_compatibility",
        "source_seam_written_windows_dynamic_unverified",
    ),
    (
        "launch_path_handle_chain_discovery",
        "source_written_windows_dynamic_unverified",
    ),
    (
        "launch_context_selection_contract",
        "source_written_uncompiled_unrun",
    ),
    ("authenticated_launch_context_source_producer", "missing"),
    (
        "prelease_authenticated_pe_material",
        "source_written_uncompiled_unrun",
    ),
    ("authenticated_prelease_pe_parser_producer", "missing"),
    (
        "preliminary_resolution_request_plan",
        "source_written_uncompiled_unrun",
    ),
    (
        "grant_ready_resolution_contract",
        "source_written_uncompiled_unrun",
    ),
    ("grant_ready_resolution_producer", "missing"),
    (
        "authenticated_recursive_resolution_policy_contract",
        "source_written_uncompiled_unrun",
    ),
    ("authenticated_recursive_policy_source_producer", "missing"),
    (
        "recursive_wave_acquisition_custody_contract",
        "source_written_uncompiled_unrun",
    ),
    ("recursive_wave_positive_advancer_backend", "missing"),
    ("recursive_same_owner_parser_backend", "missing"),
    ("external_search_directory_authority", "missing"),
    ("launch_path_component_grant_backend", "missing"),
    ("searched_name_grant_acquisition_backend", "missing"),
    ("searched_name_fence_backend", "missing"),
    ("fileid_immutable_content_lease_backend", "missing"),
    ("postlease_exact_pe_import_graph_sealer", "missing"),
    (
        "postlease_same_owner_lineage_contract",
        "source_written_uncompiled_unrun",
    ),
    ("final_namespace_query_backend", "missing"),
    ("startup_import_resolution_producer", "missing"),
    ("live_windows_resolution_currentness_backend", "missing"),
    ("parent_relative_file_reopen_backend", "missing"),
];

/// Proposed, not dynamically verified, Windows reopen shape for executable images and DLLs.
pub(super) const PROPOSED_WINDOWS_IMAGE_DESIRED_ACCESS: &str =
    "FILE_GENERIC_READ|FILE_GENERIC_EXECUTE";
pub(super) const PROPOSED_WINDOWS_IMAGE_SHARE_ACCESS: &str = "FILE_SHARE_READ";
pub(super) const PROPOSED_WINDOWS_READ_ONLY_ASSET_DESIRED_ACCESS: &str = "FILE_GENERIC_READ";
pub(super) const PROPOSED_WINDOWS_FILE_CREATE_DISPOSITION: &str = "FILE_OPEN";
pub(super) const PROPOSED_WINDOWS_IMAGE_CREATE_OPTIONS: &str =
    "FILE_NON_DIRECTORY_FILE|FILE_OPEN_REPARSE_POINT|FILE_SYNCHRONOUS_IO_NONALERT";

pub(super) const LOADER_TRANSITION_ORDER: &[&str] = &[
    "borrow_only_receipt_and_evidence_preflight",
    "discover_retained_launch_path_candidates_and_prelease_pe_material",
    "authenticate_exact_launch_context_and_preliminary_resolution_requests",
    "resolve_exact_terminals_dispositions_and_external_directory_owners",
    "seal_grant_ready_preliminary_resolution_plan",
    "acquire_base_searched_name_and_launch_path_component_grants",
    "acquire_base_route_specific_owners_and_fileid_leases_after_base_grants",
    "same_owner_parse_base_targets_into_first_recursive_frontier",
    "repeat_recursive_request_resolution_grant_route_owner_lease_and_same_owner_parse_by_producer_wave",
    "prove_terminal_empty_frontier_without_detaching_custody",
    "aggregate_all_base_and_recursive_grants_leases_and_owner_bindings",
    "seal_exact_recursive_pe_graph_under_final_aggregate",
    "query_all_name_grants_and_content_lease_generation_set",
    "validate_and_retain_package_root_and_plan_directory_handles",
    "close_reopen_package_files_runner_last",
    "compare_volume_file_id_type_reparse_link_size_delete_pending",
    "rehash_reopened_files_and_derive_paths_from_handles",
    "final_ordered_identity_hash_path_name_and_content_lease_query",
];
