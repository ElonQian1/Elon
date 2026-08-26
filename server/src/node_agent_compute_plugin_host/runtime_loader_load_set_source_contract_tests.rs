const HOST_MODULE: &str = include_str!("mod.rs");
const MANAGED_FACADE: &str = include_str!("../node_agent_managed_fs.rs");
const MANAGED_LOADER: &str = include_str!("../node_agent_managed_fs/loader.rs");
const MANAGED_NAME_GRANT_POSITIVE: &str =
    include_str!("../node_agent_managed_fs/loader/name_grant_positive.rs");
const MANAGED_LOADER_SYSTEM_VALIDATION: &str =
    include_str!("../node_agent_managed_fs/loader_system_validation.rs");
const FACADE: &str = include_str!("runtime_loader_load_set.rs");
const DIGEST: &str = include_str!("runtime_loader_load_set/digest.rs");
const FAILURE: &str = include_str!("runtime_loader_load_set/failure.rs");
const LAUNCH_PATH_VALIDATION: &str =
    include_str!("runtime_loader_load_set/launch_path_validation.rs");
const MODEL: &str = include_str!("runtime_loader_load_set/model.rs");
const MODEL_DEBUG: &str = include_str!("runtime_loader_load_set/model_debug.rs");
const NAMESPACE_VALIDATION: &str = include_str!("runtime_loader_load_set/namespace_validation.rs");
const PE_GRAPH_VALIDATION: &str = include_str!("runtime_loader_load_set/pe_graph_validation.rs");
const POLICY: &str = include_str!("runtime_loader_load_set/policy.rs");
const RESOLUTION: &str = include_str!("runtime_loader_load_set/resolution.rs");
const SYSTEM_CLOSURE: &str = include_str!("runtime_loader_load_set/resolution/system_closure.rs");
const SYSTEM_CLOSURE_DIGEST: &str =
    include_str!("runtime_loader_load_set/resolution/system_closure/digest.rs");
const SYSTEM_CLOSURE_EDGE_ORDER: &str =
    include_str!("runtime_loader_load_set/resolution/system_closure/edge_order.rs");
const SYSTEM_CLOSURE_EDGE_PROJECTION: &str =
    include_str!("runtime_loader_load_set/resolution/system_closure/edge_projection.rs");
const SYSTEM_CLOSURE_PROJECTION_DIGEST: &str =
    include_str!("runtime_loader_load_set/resolution/system_closure/projection_digest.rs");
const SYSTEM_CLOSURE_SOURCE_PROJECTION: &str =
    include_str!("runtime_loader_load_set/resolution/system_closure/source_projection.rs");
const SYSTEM_CLOSURE_VALIDATION: &str =
    include_str!("runtime_loader_load_set/resolution/system_closure/validation.rs");
const PE_IMAGE_SOURCE: &str =
    include_str!("runtime_loader_load_set/pe_graph_validation/image_source.rs");
const SYSTEM_RESOLUTION_VALIDATION: &str =
    include_str!("runtime_loader_load_set/system_resolution_validation.rs");
const TRANSITION: &str = include_str!("runtime_loader_load_set/transition.rs");
const VALIDATION: &str = include_str!("runtime_loader_load_set/validation.rs");
const STAGING: &str = include_str!("fetch_file/staging.rs");
const WORK_CAPABILITY: &str = include_str!("work_admission_contract/capability.rs");
const PROMOTION_CAPABILITY: &str = include_str!("candidate_promotion_contract/capability.rs");
const EXTRACTION_TYPES: &str = include_str!("candidate_extraction/zip/types.rs");

fn source_slice() -> String {
    [
        MANAGED_LOADER,
        MANAGED_LOADER_SYSTEM_VALIDATION,
        FACADE,
        DIGEST,
        FAILURE,
        LAUNCH_PATH_VALIDATION,
        MODEL,
        MODEL_DEBUG,
        NAMESPACE_VALIDATION,
        PE_GRAPH_VALIDATION,
        POLICY,
        RESOLUTION,
        SYSTEM_CLOSURE,
        SYSTEM_CLOSURE_DIGEST,
        SYSTEM_CLOSURE_EDGE_ORDER,
        SYSTEM_CLOSURE_EDGE_PROJECTION,
        SYSTEM_CLOSURE_PROJECTION_DIGEST,
        SYSTEM_CLOSURE_SOURCE_PROJECTION,
        SYSTEM_CLOSURE_VALIDATION,
        SYSTEM_RESOLUTION_VALIDATION,
        TRANSITION,
        VALIDATION,
        STAGING,
        WORK_CAPABILITY,
        PROMOTION_CAPABILITY,
        EXTRACTION_TYPES,
    ]
    .join("\n")
}

#[test]
fn source_routes_thirteen_private_loader_authority_modules() {
    assert!(HOST_MODULE.contains("mod runtime_loader_load_set;"));
    assert!(HOST_MODULE.contains("mod runtime_loader_load_set_source_contract_tests;"));
    assert!(MANAGED_FACADE.contains("mod loader;"));
    assert!(MANAGED_FACADE.contains("mod loader_system_validation;"));
    for module in [
        "mod digest;",
        "mod failure;",
        "mod launch_path_discovery;",
        "mod launch_path_validation;",
        "mod model;",
        "mod model_debug;",
        "mod namespace_validation;",
        "mod pe_graph_validation;",
        "mod policy;",
        "mod resolution;",
        "mod system_resolution_validation;",
        "mod transition;",
        "mod validation;",
    ] {
        assert!(FACADE.contains(module), "missing module {module}");
    }
    assert_eq!(FACADE.matches("mod ").count(), 13);
    assert!(MODEL_DEBUG.contains("impl fmt::Debug for SealedComputePluginRunnerImage"));
}

#[test]
fn hard_pe_resolution_launch_path_and_namespace_prerequisites_remain_uninhabited() {
    assert!(RESOLUTION.contains("struct SealedWindowsLoaderResolutionAuthority"));
    assert!(RESOLUTION.contains("struct SealedWindowsLoaderNamespaceAuthority"));
    assert!(RESOLUTION.contains("_producer_unavailable: Infallible"));
    assert!(RESOLUTION.contains("_whole_resolution_fence_backend_unavailable: Infallible"));
    assert!(RESOLUTION.contains("normal and delay imports"));
    assert!(RESOLUTION.contains("struct WindowsLoaderPackageModuleBinding"));
    assert!(RESOLUTION.contains("struct WindowsLoaderSystemModuleBinding"));
    assert!(RESOLUTION.contains("struct WindowsLoaderSearchedNameBinding"));
    assert!(RESOLUTION.contains("enum WindowsLoaderImportEdgeKind"));
    assert!(RESOLUTION.contains("NormalImport"));
    assert!(RESOLUTION.contains("DelayImport"));
    assert!(RESOLUTION.contains("Forwarder"));
    assert!(RESOLUTION.contains("enum WindowsLoaderSearchedNameDisposition"));
    assert!(RESOLUTION.contains("ExpectedPackage"));
    assert!(RESOLUTION.contains("ExpectedSystem"));
    assert!(RESOLUTION.contains("MustRemainAbsent"));
    assert!(RESOLUTION.contains("ShadowedByEarlierName"));
    assert!(RESOLUTION.contains("earlier_searched_name_ordinal: usize"));
    assert!(RESOLUTION.contains("struct WindowsKnownDllResolutionAuthority"));
    assert!(RESOLUTION.contains("struct WindowsApiSetResolutionAuthority"));
    assert!(RESOLUTION.contains("struct WindowsSideBySideResolutionAuthority"));
    assert!(RESOLUTION.contains("struct SealedWindowsPeImportGraphAuthority"));
    assert!(RESOLUTION.contains("_authenticated_pe_parser_producer_unavailable: Infallible"));
    assert!(RESOLUTION.contains("struct SealedWindowsLoaderLaunchPathAuthority"));
    assert!(RESOLUTION.contains("_launch_path_grant_or_share_backend_unavailable: Infallible"));
    assert!(RESOLUTION.contains("pe_import_graph: SealedWindowsPeImportGraphAuthority"));
    assert!(RESOLUTION.contains("launch_path_authority: SealedWindowsLoaderLaunchPathAuthority"));
    assert!(RESOLUTION.contains("searched_name_grants: Vec<WindowsLoaderSearchedNameFenceCustody>"));
    assert!(RESOLUTION
        .contains("launch_path_component_grants: Vec<WindowsLoaderLaunchPathGrantCustody>"));
    assert!(RESOLUTION.contains("package_content_lease_set_digest: String"));
    assert!(RESOLUTION.contains("system_content_lease_set_digest: String"));
    assert!(RESOLUTION.contains("immutable_content_lease_set_digest: String"));
    assert!(DIGEST.contains("\"pe_import_graph\": {"));
    assert!(DIGEST.contains("\"launch_path_component_set_digest\""));
    assert!(SYSTEM_RESOLUTION_VALIDATION.contains("fn validate_system_dependencies"));
    assert!(NAMESPACE_VALIDATION.contains("fn validate_namespace_queries"));
    assert!(NAMESPACE_VALIDATION.contains("final_query_generation <= initial_query_generation"));
    assert!(!source_slice().contains("transition_admitted_runner_to_loader_load_set"));
    assert!(!FACADE.contains("WindowsRunnerLoadSetTransitionFailure"));
}

#[test]
fn recursive_system_image_edges_have_disjoint_postlease_provenance_and_fixpoint_seal() {
    assert!(RESOLUTION.contains("enum WindowsLoaderModuleEdgeLocator"));
    assert!(RESOLUTION.contains("BasePrelease"));
    assert!(RESOLUTION.contains("SystemPostLease"));
    assert!(RESOLUTION.contains("source: WindowsPeParsedImageSource"));
    assert!(RESOLUTION
        .contains("recursive_resolution_closure: SealedWindowsRecursiveResolutionClosure"));
    assert!(SYSTEM_CLOSURE.contains("struct WindowsPostLeaseSystemImageParseReceipt"));
    assert!(SYSTEM_CLOSURE.contains("struct WindowsRecursiveResolutionWavePlan"));
    assert!(SYSTEM_CLOSURE.contains("struct SealedWindowsRecursiveResolutionClosure"));
    assert!(SYSTEM_CLOSURE
        .contains("_recursive_system_import_closure_producer_unavailable: Infallible"));
    assert!(SYSTEM_CLOSURE.contains("producer_module_request_ordinal"));
    assert!(SYSTEM_CLOSURE_EDGE_PROJECTION.contains("validate_final_edge_provenance"));
    assert!(SYSTEM_CLOSURE_EDGE_PROJECTION.contains("validate_forwarder_chains"));
    assert!(SYSTEM_CLOSURE_EDGE_PROJECTION.contains("maximum_forwarder_hop_depth"));
    assert!(SYSTEM_CLOSURE_SOURCE_PROJECTION.contains("source_owner_matches_producer_binding"));
    assert!(SYSTEM_CLOSURE_SOURCE_PROJECTION
        .contains("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FRONTIER_DELAYED"));
    assert!(SYSTEM_CLOSURE_EDGE_ORDER.contains("validate_recursive_edge_order"));
    assert!(SYSTEM_CLOSURE_EDGE_ORDER
        .contains("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_WAVE_EDGE_MERGE_CHANGED"));
    assert!(SYSTEM_CLOSURE_PROJECTION_DIGEST.contains("system_resolution_origin_material"));
    assert!(SYSTEM_CLOSURE_DIGEST.contains("producer_module_request_ordinal"));
    assert!(SYSTEM_CLOSURE_VALIDATION.contains("validate_recursive_search_projection"));
    assert!(SYSTEM_CLOSURE_VALIDATION.contains("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_FIXPOINT_CHANGED"));
    assert!(PE_IMAGE_SOURCE.contains("windows_recursive_image_owner_binding.v1"));
    assert!(SYSTEM_CLOSURE.contains("same_owner_parse_receipt_digest"));
    assert!(DIGEST.contains("windows_loader_resolution_profile.v3"));
    assert!(DIGEST.contains("windows_pe_import_edges.v2"));
}

#[test]
fn pe_dfs_cache_system_sections_and_handle_paths_are_recomputed_and_cross_bound() {
    assert!(RESOLUTION.contains("importer_graph_edge_ordinal: usize"));
    assert!(RESOLUTION.contains("struct WindowsPeImportEdgeCrossBinding"));
    assert!(RESOLUTION.contains("import_edge_cross_binding_set_digest: String"));
    assert!(RESOLUTION.contains("resolved_module_cache_key: String"));
    assert!(RESOLUTION.contains("struct WindowsLoaderPreloadedModuleBinding"));
    assert!(RESOLUTION.contains("struct SealedWindowsLoaderPreloadedModuleAuthority"));
    assert!(RESOLUTION
        .contains("preloaded_module_authority: SealedWindowsLoaderPreloadedModuleAuthority"));
    assert!(
        RESOLUTION.contains("_authenticated_process_bootstrap_producer_unavailable: Infallible")
    );
    assert!(RESOLUTION.contains("struct WindowsPeParsedImageBinding"));
    assert!(RESOLUTION.contains("import_table_digest: String"));
    assert!(RESOLUTION.contains("parsed_images: Vec<WindowsPeParsedImageBinding>"));
    assert!(DIGEST.contains("fn importer_edge_table_digest("));
    assert!(DIGEST.contains("ordered_edges.sort_by_key(|(edge_ordinal, _)| *edge_ordinal)"));
    assert!(PE_GRAPH_VALIDATION.contains("fn validate_pe_import_graph("));
    assert!(PE_GRAPH_VALIDATION.contains("fn importer_graph_edge_ordinals_are_contiguous("));
    assert!(PE_GRAPH_VALIDATION.contains("let mut stack = vec![root.clone()]"));
    assert!(PE_GRAPH_VALIDATION.contains("while let Some(importer) = stack.pop()"));
    assert!(PE_GRAPH_VALIDATION.contains("targets.sort_by_key(|(edge_ordinal, _)| *edge_ordinal)"));
    assert!(PE_GRAPH_VALIDATION.contains("for (_, target) in targets.into_iter().rev()"));
    assert!(PE_GRAPH_VALIDATION
        .contains("canonical_loader_module_basename(&resolution.runner_relative_path)"));
    assert!(PE_GRAPH_VALIDATION
        .contains("for preloaded in &resolution.preloaded_module_authority.modules"));
    assert!(PE_GRAPH_VALIDATION.contains(
        "record_cache_alias(&mut observed, &preloaded.resolved_module_cache_key, &target)"
    ));
    assert!(
        PE_GRAPH_VALIDATION.contains("parsed.import_table_digest != expected_import_table_digest")
    );
    assert!(PE_GRAPH_VALIDATION.contains("loaded_module_cache_targets_are_consistent"));
    assert!(
        PE_IMAGE_SOURCE.contains("elon.compute_plugin.windows_package_parsed_image_material.v1")
    );
    for material in [
        "file_identity_digest",
        "sealed_content_digest",
        "content_lease_generation_digest",
        "immutable_content_policy_digest",
    ] {
        assert!(PE_IMAGE_SOURCE.contains(material));
    }
    assert!(PE_GRAPH_VALIDATION.contains("derived_closure.get(ordinal) != Some(&reachable.node)"));
    assert!(SYSTEM_RESOLUTION_VALIDATION
        .contains("COMPUTE_PLUGIN_LOADER_PRELOADED_MODULE_AUTHORITY_CHANGED"));
    assert!(VALIDATION.contains("COMPUTE_PLUGIN_LOADER_PRELOADED_PROCESS_CONTEXT_CHANGED"));
    assert!(DIGEST.contains("\"preloaded_modules\": {"));
    assert!(DIGEST.contains("resolution.preloaded_module_authority.module_set_digest"));

    assert!(RESOLUTION.contains("struct WindowsKnownDllSectionBinding"));
    assert!(RESOLUTION.contains("immutable_image_section_identity_digest: String"));
    assert!(RESOLUTION.contains("section_image_mapping_receipt_digest: String"));
    assert!(
        SYSTEM_RESOLUTION_VALIDATION.contains("COMPUTE_PLUGIN_LOADER_KNOWN_DLL_AUTHORITY_CHANGED")
    );
    assert!(MANAGED_LOADER.contains("struct PinnedWindowsLoaderSystemImageFile"));
    assert!(MANAGED_LOADER.contains("parent_relative_open_receipt_digest: String"));
    assert!(MANAGED_LOADER.contains("section_mapping_receipt_digest: String"));
    assert!(
        MANAGED_LOADER.contains("_parent_relative_system_image_backend_unavailable: Infallible")
    );
    assert!(SYSTEM_RESOLUTION_VALIDATION.contains("file.matches_resolution("));
    assert!(RESOLUTION.contains("enum WindowsLoaderApiSetHostResolution"));
    assert!(SYSTEM_RESOLUTION_VALIDATION.contains("fn api_set_host_resolution_valid("));
    assert!(SYSTEM_RESOLUTION_VALIDATION.contains("fn side_by_side_terminal_valid("));
    assert!(
        SYSTEM_RESOLUTION_VALIDATION.contains("entry.image_file_identity_digest == file_identity")
    );
    assert!(MANAGED_LOADER.contains("struct ManagedLoaderSystemImageContentLease"));
    assert!(MANAGED_LOADER_SYSTEM_VALIDATION.contains("fn matches_resolution("));

    assert!(MANAGED_LOADER.contains("struct ManagedLoaderHandlePathReceipt"));
    assert!(MANAGED_LOADER.contains("_handle_path_backend_unavailable: Infallible"));
    assert!(MANAGED_LOADER.contains("path_receipt: ManagedLoaderHandlePathReceipt"));
    assert!(LAUNCH_PATH_VALIDATION.contains("fn validate_launch_path_authority("));
    assert!(LAUNCH_PATH_VALIDATION.contains("runner.handle_path_binding()"));
    assert!(LAUNCH_PATH_VALIDATION.contains("working_directory.handle_path_binding()"));
    assert!(LAUNCH_PATH_VALIDATION.contains("retained_parent_chain_share_contract_set_digest"));
}

#[test]
fn success_is_one_successor_not_original_admission_plus_image() {
    assert!(MODEL.contains("struct LoaderLockedWorkAdmittedPluginSlot<'root>"));
    assert!(MODEL.contains("authority: LoaderTransitionAuthorityCustody<'root>"));
    assert!(MODEL.contains("image: SealedComputePluginRunnerImage"));
    assert!(MODEL.contains("load_set_authority: SealedWindowsRunnerLoadSetAuthority"));
    assert!(MODEL.contains("package_files: Vec<WindowsLoaderPackageFileCustody>"));
    assert!(MODEL.contains("runner_ordinal: usize"));
    assert!(MODEL.contains("package_root_directory: PinnedManagedLoaderDirectory"));
    assert!(MODEL.contains("enum WindowsLoaderWorkingDirectoryLocation"));
    assert!(MODEL.contains("working_directory_location: WindowsLoaderWorkingDirectoryLocation"));
    assert!(
        !MODEL.contains("admitted: DurableWorkAdmittedPluginSlot<'root>,\n    pub(super) image")
    );
    assert!(!MODEL.contains("executable: File"));
    assert!(!MODEL.contains("loader_dependency_files: Vec<File>"));
    assert!(!MODEL.contains("impl Clone for LoaderLockedWorkAdmittedPluginSlot"));
    assert!(!MODEL.contains("impl Clone for SealedComputePluginRunnerImage"));
    assert!(!MODEL.contains("Serialize, Deserialize"));
    assert!(VALIDATION.contains("fn validate_internal_binding"));
    assert!(VALIDATION.contains("entry.package_file_ordinal != ordinal"));
    assert!(VALIDATION.contains("entry.directory_ordinal != ordinal"));
}

#[test]
fn purpose_specific_consuming_graph_preserves_every_authority_layer() {
    assert_eq!(
        WORK_CAPABILITY
            .matches("into_loader_transition_parts")
            .count(),
        2
    );
    assert_eq!(
        PROMOTION_CAPABILITY
            .matches("into_loader_transition_parts")
            .count(),
        1
    );
    assert!(EXTRACTION_TYPES.contains("ExtractedComputePluginLoaderTransitionParts"));
    assert!(EXTRACTION_TYPES.contains("completed_at: Instant"));
    for field in [
        "work_admission_receipts",
        "work_admission_trusted_time",
        "work_admission_revalidated_at",
        "promotion_receipts",
        "promotion_trusted_time",
        "promotion_revalidated_at",
        "health_receipt",
        "staging_receipt",
        "staging_recovery_key",
        "extraction_plan",
        "extraction_evidence",
        "verified_artifacts",
        "staging_seal",
        "staging_seal_evidence",
        "extraction_completed_at",
        "staging_root",
        "_staging_root_lock_lease",
        "staging_relative_root",
        "staging_run_digest",
    ] {
        assert!(MODEL.contains(field), "missing authority field {field}");
        assert!(
            TRANSITION.contains(field),
            "missing transition move {field}"
        );
    }
    assert!(!TRANSITION.contains("into_cleanup_parts"));
    assert!(!TRANSITION.contains("into_cleanup_directory"));
    let loader_seam = EXTRACTION_TYPES
        .rsplit_once("fn into_loader_transition_parts")
        .expect("loader consuming seam missing")
        .1;
    assert!(loader_seam.contains("directories: self.directories"));
    assert!(loader_seam.contains("files: self.files"));
    assert!(!loader_seam.contains(".zip("));
    assert!(EXTRACTION_TYPES.contains("directories: Vec<PinnedManagedDirectory>"));
    assert!(EXTRACTION_TYPES.contains("files: Vec<PinnedManagedFile>"));
    assert!(STAGING.contains("struct PreparedComputePluginStagingLoaderParts<'root>"));
    assert!(STAGING.contains("package_root: PinnedManagedDirectory"));
    assert!(TRANSITION.contains("package_root_directory: staging.package_root"));
    assert!(TRANSITION.contains("struct RawDestructuredLoaderTransitionCustody<'root>"));
    assert!(TRANSITION.contains("trait WindowsRunnerLoaderOwnerGraphIndexer"));
    assert!(TRANSITION.contains("fn index_verified_owner_graph<'root>"));
}

#[test]
fn indexed_content_leases_move_once_into_reopen_receipts() {
    assert!(RESOLUTION.contains("struct WindowsLoaderPackageContentLeaseCustody"));
    assert!(RESOLUTION.contains("package_file_ordinal: usize"));
    assert!(
        RESOLUTION.contains("package_content_leases: Vec<WindowsLoaderPackageContentLeaseCustody>")
    );
    assert!(RESOLUTION.contains("struct PostLeaseSplitWindowsRunnerLoadSetPrerequisite"));
    assert!(TRANSITION.contains("consume_query_verified_loader_prerequisite(prerequisite)?"));
    assert!(TRANSITION.contains("struct ValidatedPreBarrierPackageFileCustody"));
    assert!(TRANSITION.contains("content_lease: ManagedLoaderFileContentLease"));
    assert!(TRANSITION.contains("package_files: Vec<ValidatedPreBarrierPackageFileCustody>"));
    assert!(
        TRANSITION.contains("without truncating, cloning, or dropping an unmatched file or lease")
    );
    assert!(MANAGED_LOADER.contains("struct ManagedLoaderFileContentLease"));
    assert!(MANAGED_LOADER.contains("_immutable_content_backend_unavailable: Infallible"));
    assert!(
        MANAGED_LOADER.contains("struct ManagedLoaderFileContentLeaseAuthenticatedNegativeReceipt")
    );
    assert!(MANAGED_LOADER.contains("_authenticated_negative_backend_unavailable: Infallible"));
    assert!(MANAGED_LOADER.contains("struct ManagedLoaderFileReopenReceipt"));
    assert!(MANAGED_LOADER.contains("_anchor_consuming_reopen_backend_unavailable: Infallible"));
    assert!(MANAGED_LOADER.contains("content_lease: ManagedLoaderFileContentLease"));
    assert!(MANAGED_LOADER.contains("reopen_receipt: ManagedLoaderFileReopenReceipt"));
    assert!(MANAGED_LOADER.contains("source_content_lease_generation_digest"));
    assert!(MANAGED_LOADER.contains("fn reopen_receipt_matches("));
    assert!(MANAGED_LOADER.contains("struct ManagedLoaderParentRelativeReopenAttemptCustody"));
    assert!(MANAGED_LOADER
        .contains("struct ManagedLoaderParentRelativeReopenAuthenticatedNegativeReceipt"));
    assert!(
        MANAGED_LOADER.contains("_authenticated_reopen_negative_backend_unavailable: Infallible")
    );
    assert!(FAILURE.contains("struct WindowsRunnerPackageFileReopenFailureCustody"));
    assert!(FAILURE.contains("negative.matches_attempt(&attempt)"));
    assert!(FAILURE.contains(
        "parent_relative_reopen_failure: Option<WindowsRunnerPackageFileReopenFailureCustody>"
    ));
    assert!(VALIDATION.contains("entry.file.content_lease_binding()"));
    assert!(VALIDATION.contains("COMPUTE_PLUGIN_LOADER_PACKAGE_CONTENT_LEASE_SET_CHANGED"));
}

#[test]
fn five_failure_phases_retain_linear_indexed_custody() {
    let transition_failure = FAILURE
        .split_once("pub(super) enum WindowsRunnerLoadSetTransitionFailure<'root> {")
        .expect("transition failure enum missing")
        .1
        .split_once("impl fmt::Debug for WindowsRunnerLoadSetOutcomeUncertainCustody")
        .expect("transition failure enum terminator missing")
        .0;
    for phase in [
        "NameGrantAcquisitionUnusable",
        "ContentLeaseAcquisitionUnusable",
        "BorrowOnlyNotTransitioned",
        "NamespaceQueryUnusable",
        "PostBarrierOutcomeUncertain",
    ] {
        assert!(
            transition_failure.contains(phase),
            "missing failure phase {phase}"
        );
    }
    assert_eq!(transition_failure.matches("custody:").count(), 5);
    for retained in [
        "WindowsRunnerNameGrantAcquisitionUnusableCustody<'root>",
        "WindowsRunnerContentLeaseAcquisitionUnusableCustody<'root>",
        "WindowsRunnerBorrowOnlyNotTransitionedCustody<'root>",
        "WindowsRunnerNamespaceQueryUnusableCustody<'root>",
        "WindowsRunnerLoadSetOutcomeUncertainCustody<'root>",
        "PendingWindowsRunnerPackageFileCustody",
        "TransitionedWindowsRunnerPackageFileCustody",
        "WindowsRunnerPackageFileCloseOutcomeUncertainCustody",
        "WindowsRunnerPackageFileReopenFailureCustody",
        "QuarantinedWindowsRunnerPackageFileReplacementCustody",
        "ValidatedRetainedWindowsRunnerNamespaceDirectoryCustody",
        "package_file_ordinal: usize",
        "directory_ordinal: usize",
        "relative_path: String",
        "transition_schedule: Vec<usize>",
        "next_transition_schedule_index: usize",
        "runner_ordinal: usize",
        "_pending_grants: Vec<WindowsRunnerPendingNameGrantRef>",
        "_pending_package_file_ordinals: Vec<usize>",
    ] {
        assert!(
            FAILURE.contains(retained),
            "missing uncertain custody {retained}"
        );
    }
    for post_barrier_phase in [
        "SourceHandleClose",
        "ParentRelativeReopen",
        "ReplacementIdentity",
        "ReplacementHash",
        "HandleDerivedPath",
        "FinalFenceQuery",
    ] {
        assert!(FAILURE.contains(post_barrier_phase));
    }
    assert!(FAILURE.contains("struct WindowsRunnerFinalFenceQueryFailureCustody"));
    assert!(FAILURE.contains("fn classify("));
    assert_eq!(
        FAILURE
            .matches("_returned_positive: Option<ManagedLoaderNamespaceQueryReceipt>")
            .count(),
        2
    );
    assert!(FAILURE.contains("_returned_positive: Option<ManagedLoaderSearchedNameGrant>"));
    assert!(FAILURE.contains("returned_positive: Option<ManagedLoaderFileContentLease>"));
    assert!(FAILURE.contains("WindowsRunnerActiveContentLeaseAcquisitionCustody::PackageFile"));
    assert!(
        FAILURE.contains("attempt: ManagedLoaderSystemImageContentLeaseAcquisitionAttemptCustody")
    );
    assert!(FAILURE.contains("ResolvedFilesystemSystemImagePositiveOutcome"));
    assert!(FAILURE.contains("outcome: ManagedLoaderSystemImageContentLeasePositiveOutcomeCustody"));
    assert!(!FAILURE.contains("_returned_positive: Option<ManagedLoaderFileContentLease>"));
    assert!(FAILURE.contains("query_attempt.matches_session(&prerequisite.namespace.session)"));
    assert!(FAILURE.contains("query_attempt.matches_session(session)"));
    assert!(FAILURE.contains("negative.matches_query(session, request_digest, query_nonce_digest)"));
    assert!(FAILURE.contains("authenticated_negative.matches_attempt(&active_attempt)"));
    assert!(FAILURE
        .contains("_authenticated_negative: Option<ManagedLoaderAuthenticatedNegativeReceipt>"));
    assert!(FAILURE.contains("QuarantinedManagedLoaderSourceClose"));
    assert!(MANAGED_LOADER.contains("fn returned_positive_is_none(&self) -> bool"));
    assert!(FAILURE.contains("attempt.returned_positive_is_none()"));
    assert_eq!(
        FAILURE
            .matches("_preliminary: PreliminaryResolutionRequestsPlannedWork<'root>")
            .count(),
        1
    );
    assert!(FAILURE.contains("_policy_current_grant_ready:"));
    assert!(FAILURE.contains("PolicyCurrentGrantReadyWindowsRunnerResolutionPrerequisite<'root>"));
    assert!(FAILURE.contains("_policy_current_namespace:"));
    assert!(FAILURE.contains("PolicyCurrentPreFinalWindowsLoaderNamespaceGrantSet<'root>"));
    assert!(FAILURE
        .contains("_acquired_leases: Vec<WindowsLoaderAcquiredImmutableContentLeaseCustody>"));
    assert!(FAILURE.contains("_active: WindowsRunnerActiveContentLeaseAcquisitionCustody"));
    assert!(FAILURE.contains("_pending: Vec<WindowsRunnerPendingContentLeaseRef>"));
    assert!(FAILURE
        .contains("_prerequisite: PostLeaseSealedWindowsRunnerLoadSetPreQueryPrerequisite<'root>"));
    assert!(!FAILURE.contains("_resolution: SealedWindowsLoaderResolutionAuthority"));
    assert!(RESOLUTION.contains("enum WindowsLoaderAcquiredImmutableContentLeaseCustody"));
    assert!(RESOLUTION.contains("ResolvedFilesystemSystemImage"));
    assert!(RESOLUTION.contains("struct PreFinalWindowsLoaderNamespaceGrantSet<'root>"));
    assert!(
        RESOLUTION.contains("struct PolicyCurrentPreFinalWindowsLoaderNamespaceGrantSet<'root>")
    );
    assert!(RESOLUTION.contains("namespace: PreFinalWindowsLoaderNamespaceGrantSet<'root>"));
    assert!(RESOLUTION
        .contains("authenticated_recursive_policy: AuthenticatedWindowsRecursiveResolutionPolicy"));
    assert!(RESOLUTION
        .contains("policy_dispatch_authorization: WindowsRecursivePolicyDispatchAuthorization"));
    assert!(RESOLUTION.contains("struct PostLeaseWindowsLoaderNamespaceGrantSet<'root>"));
    assert!(RESOLUTION.contains("struct PostLeaseSealedWindowsRunnerLoadSetPreQueryPrerequisite"));
    assert!(RESOLUTION.contains("namespace: PostLeaseWindowsLoaderNamespaceGrantSet<'root>"));
    assert!(
        RESOLUTION.contains("postlease_lineage: PostLeaseWindowsRunnerResolutionLineage<'root>")
    );
    assert!(
        RESOLUTION.contains("package_content_leases: Vec<WindowsLoaderPackageContentLeaseCustody>")
    );
    assert!(RESOLUTION.contains("_postlease_final_resolution_sealer_unavailable: Infallible"));
    assert!(!RESOLUTION.contains("struct WindowsRunnerLoadSetBorrowPrerequisite"));
    assert!(DIGEST.contains("windows_loader_resolution_profile.v3"));
    assert!(DIGEST.contains("\"launch_context_selector_digest\""));
    assert!(DIGEST.contains("\"selected_context_binding_digest\""));
    assert!(DIGEST.contains("\"preliminary_resolution_request_plan_digest\""));
    assert!(DIGEST.contains("\"grant_ready_resolution_plan_digest\""));
    assert!(!DIGEST.contains("\"required_launch_context_digest\""));
    assert!(!FAILURE.contains("fn retry"));
    assert!(!FAILURE.contains("fn into_admitted"));
    assert!(!FAILURE.contains("fn into_prerequisite"));
}

#[test]
fn managed_loader_typestates_expose_no_raw_or_reconstructing_authority() {
    for owner in [
        "struct ManagedLoaderFileContentLease",
        "struct ManagedLoaderFileContentLeaseAcquisitionAttemptCustody",
        "struct ManagedLoaderFileIdentityAnchor",
        "struct ManagedLoaderFileReopenReceipt",
        "struct ManagedLoaderHandlePathReceipt",
        "struct ManagedLoaderParentRelativeReopenAttemptCustody",
        "struct ManagedLoaderParentRelativeReopenAuthenticatedNegativeReceipt",
        "struct PinnedManagedLoaderFile",
        "struct PinnedManagedLoaderDirectory",
        "struct PinnedWindowsLoaderSearchDirectory",
        "struct PinnedWindowsLoaderSystemImageFile",
        "struct QuarantinedManagedLoaderFile",
        "struct QuarantinedManagedLoaderSourceClose",
    ] {
        assert!(
            MANAGED_LOADER.contains(owner),
            "missing managed owner {owner}"
        );
    }
    assert!(!MANAGED_LOADER.contains("pub(crate) fn from_"));
    assert!(!MANAGED_LOADER.contains("pub(crate) fn file"));
    assert!(!MANAGED_LOADER.contains("AsRawHandle"));
    assert!(!MANAGED_LOADER.contains("try_clone"));
    assert!(MANAGED_LOADER.contains("ManuallyDrop<PinnedManagedFile>"));
    assert!(MANAGED_LOADER.contains("handle_derived_canonical_path"));
    assert!(MANAGED_LOADER.contains("authenticated_response_digest: String"));
    assert!(MANAGED_LOADER.contains("authenticated_response_is_bound"));
    assert!(MANAGED_LOADER.contains("_authenticated_positive_backend_unavailable: Infallible"));
    assert!(MANAGED_NAME_GRANT_POSITIVE.contains("fn matches_attempt("));
    assert!(MANAGED_NAME_GRANT_POSITIVE.contains("Arc::ptr_eq(&self.owner, &attempt.owner)"));
    assert!(MANAGED_NAME_GRANT_POSITIVE
        .contains("self.authenticated_response == attempt.response_buffer"));
    assert!(MANAGED_LOADER_SYSTEM_VALIDATION.contains("path_currentness_binding"));
    assert!(MANAGED_LOADER_SYSTEM_VALIDATION.contains("namespace_alias_currentness_receipt_digest"));
}

#[test]
fn source_freezes_missing_authorities_eighteen_zero_effects_and_transition_order() {
    for gap in [
        "node_local_authority_currentness",
        "runtime_transition_authority",
        "host_runtime_authority",
        "v15_authenticated_session",
    ] {
        assert!(POLICY.contains(&format!("(\"{gap}\", \"missing\")")));
    }
    let zero_effects = POLICY
        .split_once("LOADER_LOAD_SET_ZERO_EFFECTS")
        .expect("zero-effect table missing")
        .1
        .split_once("];")
        .expect("zero-effect table terminator missing")
        .0;
    for effect in [
        "runtime_phase",
        "runtime_generation",
        "runtime_start",
        "runtime_resume",
        "runtime_store",
        "health",
        "readiness",
        "node",
        "provider",
        "route",
        "offer",
        "capacity",
        "execution",
        "attempt",
        "lease",
        "usage",
        "settlement",
        "money",
    ] {
        assert!(zero_effects.contains(&format!("(\"{effect}\", \"none\")")));
    }
    assert_eq!(zero_effects.matches("(\"").count(), 18);
    for proposed in [
        "FILE_GENERIC_READ|FILE_GENERIC_EXECUTE",
        "PROPOSED_WINDOWS_READ_ONLY_ASSET_DESIRED_ACCESS",
        "PROPOSED_WINDOWS_FILE_CREATE_DISPOSITION",
        "FILE_SHARE_READ",
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
    ] {
        assert!(
            POLICY.contains(proposed),
            "missing proposed recipe {proposed}"
        );
    }
    assert!(POLICY.contains("DYNAMIC_MODULE_LOAD_AUTHORITY"));
    assert!(POLICY.contains("missing_resume_blocker"));
    assert!(POLICY.contains("existing_extraction_directory_access_share_compatibility"));
    assert!(POLICY.contains("fileid_immutable_content_lease_backend"));
    assert!(POLICY.contains("prelease_authenticated_pe_material"));
    assert!(POLICY.contains("authenticated_prelease_pe_parser_producer"));
    assert!(POLICY.contains("postlease_exact_pe_import_graph_sealer"));
    assert!(POLICY.contains("launch_path_handle_chain_discovery"));
    assert!(POLICY.contains("launch_context_selection_contract"));
    assert!(POLICY.contains("authenticated_launch_context_source_producer"));
    assert!(POLICY.contains("preliminary_resolution_request_plan"));
    assert!(POLICY.contains("grant_ready_preliminary_resolution_plan"));
    assert!(POLICY.contains("authenticated_recursive_resolution_policy_contract"));
    assert!(POLICY.contains("authenticated_recursive_policy_source_producer"));
    assert!(POLICY.contains("recursive_wave_acquisition_custody_contract"));
    assert!(POLICY.contains("recursive_wave_positive_advancer_backend"));
    assert!(POLICY.contains("recursive_same_owner_parser_backend"));
    assert!(POLICY.contains("external_search_directory_authority"));
    assert!(POLICY.contains("launch_path_component_grant_backend"));
    assert!(!POLICY.contains("PROPOSED_WINDOWS_DIRECTORY_DESIRED_ACCESS"));
    assert!(!POLICY.contains("PROPOSED_WINDOWS_DIRECTORY_SHARE_ACCESS"));

    let transition_positions = [
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
    ]
    .map(|step| POLICY.find(step).expect("transition step missing"));
    assert!(transition_positions
        .windows(2)
        .all(|positions| positions[0] < positions[1]));
}

#[test]
fn loader_contract_has_no_runtime_store_or_market_side_effect() {
    let combined = source_slice();
    for forbidden in [
        "ResumeThread",
        "CreateProcessAsUserW",
        "Command::new",
        "rusqlite",
        "INSERT INTO",
        "UPDATE compute_plugin",
        "ComputeReadyCapability",
        "ComputeOffer",
        "ComputeLease",
    ] {
        assert!(
            !combined.contains(forbidden),
            "forbidden source {forbidden}"
        );
    }
}
