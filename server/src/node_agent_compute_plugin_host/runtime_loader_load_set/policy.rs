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

/// The existing extraction producer retains directory handles opened with DELETE access while
/// later traversals omit FILE_SHARE_DELETE. Windows compatibility is unverified and may keep even
/// the share-none predecessor unreachable; this slice does not change that runtime contract.
pub(super) const LOADER_LOAD_SET_REACHABILITY_BLOCKERS: &[(&str, &str)] = &[
    (
        "existing_extraction_directory_access_share_compatibility",
        "missing_windows_verification_and_fix",
    ),
    ("startup_import_resolution_producer", "missing"),
    ("fileid_immutable_content_lease_backend", "missing"),
    ("searched_name_grant_acquisition_backend", "missing"),
    ("searched_name_fence_backend", "missing"),
    (
        "launch_path_parent_chain_share_or_grant_authority",
        "missing",
    ),
    ("authenticated_pe_import_graph_projection", "missing"),
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
    "borrow_only_receipt_evidence_resolution_preflight",
    "acquire_all_searched_name_and_launch_path_component_grants",
    "acquire_indexed_fileid_content_leases_after_all_name_grants",
    "same_handle_full_package_rehash_under_content_leases_and_name_grants",
    "query_all_name_grants_and_content_lease_generation_set",
    "validate_and_retain_package_root_and_plan_directory_handles",
    "close_reopen_package_files_runner_last",
    "compare_volume_file_id_type_reparse_link_size_delete_pending",
    "rehash_reopened_files_and_derive_paths_from_handles",
    "final_ordered_identity_hash_path_name_and_content_lease_query",
];
