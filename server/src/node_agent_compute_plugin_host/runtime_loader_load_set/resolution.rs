mod grant_ready;

use std::{convert::Infallible, fmt, path::PathBuf};

use crate::{
    node_agent_compute_plugin_host::plugin_manifest::ComputePluginSystemDependency,
    node_agent_managed_fs::{
        ManagedLoaderFileContentLease, ManagedLoaderNamespaceQueryAttemptCustody,
        ManagedLoaderNamespaceQueryReceipt, ManagedLoaderNamespaceSession,
        ManagedLoaderSearchedNameGrant, ManagedLoaderSystemImageContentLeasePositiveOutcomeCustody,
        PinnedWindowsLoaderSearchDirectory,
    },
};

use super::launch_path_discovery::WindowsPreliminaryModuleEdgeLocator;

pub(super) use grant_ready::*;

pub(super) struct WindowsLoaderPackageModuleBinding {
    pub(super) module_request_ordinal: usize,
    pub(super) global_import_edge_ordinal: usize,
    pub(super) edge_locator: WindowsPreliminaryModuleEdgeLocator,
    pub(super) importer_parsed_image_ordinal: usize,
    pub(super) importer: WindowsLoaderModuleNode,
    /// Contiguous resolved-graph order within this exact importer.
    pub(super) importer_graph_edge_ordinal: usize,
    pub(super) edge_kind: WindowsLoaderImportEdgeKind,
    pub(super) normalized_import_name: String,
    pub(super) imported_symbol_name: Option<String>,
    pub(super) imported_symbol_ordinal: Option<u16>,
    pub(super) resolved_module_cache_key: String,
    pub(super) relative_path: String,
    pub(super) resolved_package_file_ordinal: usize,
    pub(super) resolved_search_directory_ordinal: usize,
    pub(super) digest: String,
}

pub(super) enum WindowsLoaderSystemResolutionOrigin {
    Preloaded {
        preloaded_module_ordinal: usize,
    },
    KnownDll {
        section_identity_digest: String,
    },
    ApiSet {
        normalized_contract_name: String,
        host_component_identity_digest: String,
        host_resolution: WindowsLoaderApiSetHostResolution,
    },
    SideBySide {
        assembly_identity_digest: String,
        search_directory_ordinal: usize,
    },
    FilesystemSearch {
        search_directory_ordinal: usize,
    },
}

pub(super) enum WindowsLoaderApiSetHostResolution {
    Preloaded {
        preloaded_module_ordinal: usize,
    },
    KnownDll {
        section_identity_digest: String,
    },
    FilesystemSearch {
        search_directory_ordinal: usize,
    },
    SideBySide {
        assembly_identity_digest: String,
        search_directory_ordinal: usize,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct WindowsLoaderResolvedFilesystemSystemImageRef {
    pub(super) resolution_request_ordinal: usize,
}

pub(super) struct WindowsLoaderSystemModuleBinding {
    pub(super) module_request_ordinal: usize,
    pub(super) global_import_edge_ordinal: usize,
    pub(super) edge_locator: WindowsPreliminaryModuleEdgeLocator,
    pub(super) importer_parsed_image_ordinal: usize,
    pub(super) importer: WindowsLoaderModuleNode,
    /// Contiguous resolved-graph order shared with package edges for this exact importer.
    pub(super) importer_graph_edge_ordinal: usize,
    pub(super) edge_kind: WindowsLoaderImportEdgeKind,
    pub(super) normalized_import_name: String,
    pub(super) imported_symbol_name: Option<String>,
    pub(super) imported_symbol_ordinal: Option<u16>,
    pub(super) resolved_module_cache_key: String,
    pub(super) resolved_dependency_ordinal: usize,
    pub(super) resolved_component_identity_digest: String,
    pub(super) resolved_image_section_identity_digest: String,
    pub(super) resolution_origin: WindowsLoaderSystemResolutionOrigin,
    pub(super) resolved_search_directory_ordinal: Option<usize>,
    /// Typed borrow key into the unique final system-image custody table. Multiple import edges may
    /// reference one deduplicated FileId owner without cloning its file or content lease.
    pub(super) filesystem_image_ref: Option<WindowsLoaderResolvedFilesystemSystemImageRef>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum WindowsLoaderImportBindingRef {
    Package { binding_ordinal: usize },
    System { binding_ordinal: usize },
}

/// Projection from the verified signed manifest. A self-hashed dependency vector is insufficient:
/// the uninhabited field requires a future producer to consume exact signature-verification
/// evidence and prove that this projection came from that authenticated envelope.
pub(super) struct SealedSignedSystemDependencyAuthority {
    pub(super) manifest_digest: String,
    pub(super) signed_manifest_envelope_digest: String,
    pub(super) dependencies: Vec<ComputePluginSystemDependency>,
    pub(super) projection_digest: String,
    pub(super) _authenticated_manifest_projection_producer_unavailable: Infallible,
}

pub(super) struct WindowsResolvedSystemDependencyAuthority {
    pub(super) dependency_ordinal: usize,
    pub(super) dependency_id: String,
    pub(super) version_requirement: String,
    pub(super) component_identity_digests: Vec<String>,
    pub(super) component_identity_set_digest: String,
    pub(super) resolver_evidence_digest: String,
}

/// Exact immutable image/section projection for a Windows system component. A servicing policy or
/// filesystem path is not byte custody, so this authority has no producer until catalog/CI proof,
/// live generation evidence, and an immutable section identity are bound together.
pub(super) struct WindowsSystemComponentImageBinding {
    pub(super) component_identity_digest: String,
    pub(super) image_file_identity_digest: String,
    pub(super) code_integrity_evidence_digest: String,
    pub(super) servicing_generation_digest: String,
    pub(super) immutable_section_identity_digest: String,
}

pub(super) struct SealedWindowsSystemModuleImageAuthority {
    pub(super) component_images: Vec<WindowsSystemComponentImageBinding>,
    pub(super) component_image_set_digest: String,
    pub(super) _immutable_section_backend_unavailable: Infallible,
}

/// Exact modules already present before Runner import traversal (for example the process bootstrap
/// image set). Their cache keys must participate in the same collision closure as PE-derived edges.
pub(super) struct WindowsLoaderPreloadedModuleBinding {
    pub(super) resolved_module_cache_key: String,
    pub(super) component_identity_digest: String,
    pub(super) immutable_section_identity_digest: String,
    pub(super) preload_evidence_digest: String,
}

pub(super) struct SealedWindowsLoaderPreloadedModuleAuthority {
    pub(super) process_machine_context_digest: String,
    pub(super) modules: Vec<WindowsLoaderPreloadedModuleBinding>,
    pub(super) module_set_digest: String,
    pub(super) _authenticated_process_bootstrap_producer_unavailable: Infallible,
}

#[derive(Clone, PartialEq, Eq)]
pub(super) enum WindowsLoaderModuleNode {
    PackageFile { package_file_ordinal: usize },
    SystemComponent { component_identity_digest: String },
    KnownDllSection { section_identity_digest: String },
    ApiSetHost { component_identity_digest: String },
    SideBySideAssembly { assembly_identity_digest: String },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum WindowsLoaderImportEdgeKind {
    NormalImport,
    DelayImport,
    Forwarder,
}

pub(super) enum WindowsLoaderFilesystemSearchDirectoryTarget {
    PackageRoot,
    PackageWorkingDirectory,
    PackagePlanDirectory {
        directory_ordinal: usize,
    },
    SystemDirectory {
        directory: PinnedWindowsLoaderSearchDirectory,
    },
    WindowsDirectory {
        directory: PinnedWindowsLoaderSearchDirectory,
    },
    SideBySideAssemblyDirectory {
        directory: PinnedWindowsLoaderSearchDirectory,
    },
}

pub(super) struct WindowsLoaderSearchDirectoryBinding {
    pub(super) search_directory_ordinal: usize,
    pub(super) target: WindowsLoaderFilesystemSearchDirectoryTarget,
    pub(super) canonical_path: PathBuf,
    pub(super) canonical_path_digest: String,
    pub(super) directory_identity_digest: String,
    pub(super) policy_source_digest: String,
}

pub(super) struct WindowsKnownDllSectionBinding {
    pub(super) normalized_name: String,
    pub(super) resolved_module_cache_key: String,
    pub(super) section_identity_digest: String,
    pub(super) component_identity_digest: String,
    /// Immutable image section reached through this exact named section object. The mapping
    /// receipt prevents a KnownDLL name/object lookup from being paired with unrelated bytes.
    pub(super) immutable_image_section_identity_digest: String,
    pub(super) section_image_mapping_receipt_digest: String,
}

pub(super) struct WindowsKnownDllResolutionAuthority {
    pub(super) os_build_identity_digest: String,
    pub(super) object_manager_directory_identity_digest: String,
    pub(super) sections: Vec<WindowsKnownDllSectionBinding>,
    pub(super) section_binding_set_digest: String,
    pub(super) section_namespace_generation_digest: String,
}

pub(super) struct WindowsApiSetContractHostBinding {
    pub(super) normalized_contract_name: String,
    pub(super) host_module_cache_key: String,
    pub(super) host_component_identity_digest: String,
}

pub(super) struct WindowsApiSetResolutionAuthority {
    pub(super) os_build_identity_digest: String,
    pub(super) schema_identity_digest: String,
    pub(super) contract_host_bindings: Vec<WindowsApiSetContractHostBinding>,
    pub(super) contract_host_binding_set_digest: String,
}

pub(super) struct WindowsSideBySideAssemblyBinding {
    pub(super) normalized_import_name: String,
    pub(super) resolved_module_cache_key: String,
    pub(super) assembly_identity_digest: String,
    pub(super) component_identity_digest: String,
    pub(super) image_file_identity_digest: String,
    pub(super) immutable_section_identity_digest: String,
    pub(super) activation_context_resolution_receipt_digest: String,
}

pub(super) struct WindowsSideBySideResolutionAuthority {
    pub(super) activation_context_identity_digest: String,
    pub(super) manifest_set_digest: String,
    pub(super) assembly_bindings: Vec<WindowsSideBySideAssemblyBinding>,
    pub(super) assembly_binding_set_digest: String,
}

pub(super) enum WindowsLoaderSearchedNameDisposition {
    ExpectedPackage {
        package_file_ordinal: usize,
        image_file_identity_digest: String,
    },
    ExpectedSystem {
        resolved_component_identity_digest: String,
        image_file_identity_digest: String,
        immutable_section_identity_digest: String,
        servicing_generation_digest: String,
    },
    MustRemainAbsent,
    ShadowedByEarlierName {
        earlier_searched_name_ordinal: usize,
    },
}

pub(super) struct WindowsLoaderSearchedNameBinding {
    pub(super) searched_name_ordinal: usize,
    pub(super) import_binding: WindowsLoaderImportBindingRef,
    /// Position inside this import's exact search sequence.
    pub(super) search_step_ordinal: usize,
    pub(super) normalized_name: String,
    /// Global search-directory plan ordinal selected at this position.
    pub(super) search_directory_ordinal: usize,
    pub(super) search_directory_authority_binding_digest: String,
    pub(super) grant_request_digest: String,
    pub(super) disposition_binding_digest: String,
    pub(super) disposition: WindowsLoaderSearchedNameDisposition,
}

/// Authenticated PE parser projection rooted at the Runner. It binds exact per-image normal/delay
/// imports and forwarders, reachable nodes, and one-to-one search sequences. No parser producer is
/// available in this architecture slice.
pub(super) struct WindowsPeParsedImageBinding {
    pub(super) parsed_image_ordinal: usize,
    pub(super) node: WindowsLoaderModuleNode,
    pub(super) image_material_identity_digest: String,
    pub(super) import_table_digest: String,
    pub(super) normal_import_count: usize,
    pub(super) delay_import_count: usize,
    pub(super) forwarder_count: usize,
}

pub(super) struct WindowsPeReachableNodeBinding {
    pub(super) reachable_node_ordinal: usize,
    pub(super) node: WindowsLoaderModuleNode,
}

pub(super) struct WindowsPeImportSearchSequenceBinding {
    pub(super) sequence_ordinal: usize,
    pub(super) import_binding: WindowsLoaderImportBindingRef,
    pub(super) searched_name_ordinals: Vec<usize>,
}

pub(super) struct WindowsPeParsedImageCrossBinding {
    pub(super) prelease_parsed_image_ordinal: usize,
    pub(super) package_file_ordinal: usize,
    pub(super) file_identity_digest: String,
    pub(super) postlease_parsed_image_ordinal: usize,
    pub(super) postlease_image_material_identity_digest: String,
    pub(super) lease_generation_digest: String,
}

pub(super) struct WindowsPeImportEdgeCrossBinding {
    pub(super) preliminary_request_ordinal: usize,
    pub(super) prelease_importer_parsed_image_ordinal: usize,
    pub(super) edge_locator: WindowsPreliminaryModuleEdgeLocator,
    pub(super) postlease_import_binding: WindowsLoaderImportBindingRef,
    pub(super) postlease_importer_parsed_image_ordinal: usize,
}

/// Purpose-specific receipt emitted only by the postlease same-handle reparse sealer. It binds
/// every prelease package PE input to the final parsed graph and the exact immutable package lease
/// generation set. A digest-only caller cannot construct this receipt.
pub(super) struct SealedWindowsPePrePostCrossBindingReceipt {
    pub(super) prelease_material_set_digest: String,
    pub(super) postlease_parsed_image_set_digest: String,
    pub(super) postlease_import_edge_set_digest: String,
    pub(super) postlease_reachable_node_set_digest: String,
    pub(super) package_content_lease_set_digest: String,
    pub(super) same_retained_file_handle_set_digest: String,
    pub(super) parsed_image_cross_bindings: Vec<WindowsPeParsedImageCrossBinding>,
    pub(super) parsed_image_cross_binding_set_digest: String,
    pub(super) import_edge_cross_bindings: Vec<WindowsPeImportEdgeCrossBinding>,
    pub(super) import_edge_cross_binding_set_digest: String,
    pub(super) receipt_digest: String,
    pub(super) _same_handle_reparse_producer_unavailable: Infallible,
}

pub(super) struct SealedWindowsPeImportGraphAuthority {
    pub(super) root_package_file_ordinal: usize,
    pub(super) parsed_images: Vec<WindowsPeParsedImageBinding>,
    pub(super) reachable_nodes: Vec<WindowsPeReachableNodeBinding>,
    pub(super) search_sequences: Vec<WindowsPeImportSearchSequenceBinding>,
    pub(super) parsed_image_set_digest: String,
    pub(super) import_edge_set_digest: String,
    pub(super) reachable_node_set_digest: String,
    pub(super) search_sequence_set_digest: String,
    pub(super) expected_package_edge_count: usize,
    pub(super) expected_system_edge_count: usize,
    pub(super) expected_search_step_count: usize,
    pub(super) pre_post_cross_binding: SealedWindowsPePrePostCrossBindingReceipt,
    pub(super) _authenticated_pe_parser_producer_unavailable: Infallible,
}

pub(super) struct WindowsLoaderSearchedNameFenceCustody {
    pub(super) searched_name_ordinal: usize,
    pub(super) search_directory_ordinal: usize,
    pub(super) grant: ManagedLoaderSearchedNameGrant,
}

pub(super) enum WindowsLoaderAcquiredNameGrantCustody {
    ImportSearch(WindowsLoaderSearchedNameFenceCustody),
    LaunchPath(WindowsLoaderLaunchPathGrantCustody),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum WindowsLoaderLaunchPathKind {
    Application,
    WorkingDirectory,
}

/// One exact component in the Volume-GUID application/CWD path. The grant is distinct from import
/// search because it binds ancestor traversal and final FileId/directory identity for CreateProcess.
pub(super) struct WindowsLoaderLaunchPathComponentBinding {
    pub(super) path_kind: WindowsLoaderLaunchPathKind,
    pub(super) component_ordinal: usize,
    pub(super) parent_directory_identity_digest: String,
    pub(super) normalized_component: String,
    pub(super) expected_object_identity_digest: String,
}

pub(super) struct WindowsLoaderLaunchPathGrantCustody {
    pub(super) path_kind: WindowsLoaderLaunchPathKind,
    pub(super) component_ordinal: usize,
    pub(super) grant: ManagedLoaderSearchedNameGrant,
}

pub(super) struct SealedWindowsLoaderLaunchPathAuthority {
    pub(super) components: Vec<WindowsLoaderLaunchPathComponentBinding>,
    pub(super) component_set_digest: String,
    pub(super) application_component_set_digest: String,
    pub(super) working_directory_component_set_digest: String,
    pub(super) retained_parent_chain_share_contract_set_digest: String,
    pub(super) _launch_path_grant_or_share_backend_unavailable: Infallible,
}

pub(super) struct WindowsLoaderPackageContentLeaseCustody {
    pub(super) package_file_ordinal: usize,
    pub(super) relative_path: String,
    pub(super) lease: ManagedLoaderFileContentLease,
}

pub(super) enum WindowsLoaderAcquiredImmutableContentLeaseCustody {
    Package(WindowsLoaderPackageContentLeaseCustody),
    ResolvedFilesystemSystemImage {
        resolution_request_ordinal: usize,
        outcome: ManagedLoaderSystemImageContentLeasePositiveOutcomeCustody,
    },
}

/// One final owned system image per GrantReady resolution request/FileId. Import edges only retain
/// typed ordinals into this table, so one physical file/lease owner can serve every exact use.
pub(super) struct WindowsLoaderResolvedFilesystemSystemImageCustody {
    pub(super) resolution_request_ordinal: usize,
    pub(super) outcome: ManagedLoaderSystemImageContentLeasePositiveOutcomeCustody,
}

/// Exact Windows startup/import loader resolution authority required before any share-none handle
/// may close. Runtime-derived module names are outside this set and remain a resume blocker.
///
/// A future producer must bind normal and delay imports, package/system resolution, KnownDLL/API
/// set/SxS policy, immutable system image sections, and every searched present or absent name to
/// the exact work admission. The `Infallible` fields keep every unsupported producer unreachable.
pub(super) struct SealedWindowsLoaderResolutionAuthority {
    pub(super) admission_source_digest: String,
    pub(super) admission_receipt_digest: String,
    pub(super) extraction_plan_digest: String,
    pub(super) extraction_evidence_digest: String,
    pub(super) runner_relative_path: String,
    pub(super) working_directory_relative_path: String,
    pub(super) working_directory_identity_digest: String,
    pub(super) search_directories: Vec<WindowsLoaderSearchDirectoryBinding>,
    pub(super) known_dll_authority: WindowsKnownDllResolutionAuthority,
    pub(super) api_set_authority: WindowsApiSetResolutionAuthority,
    pub(super) side_by_side_authority: WindowsSideBySideResolutionAuthority,
    pub(super) package_module_bindings: Vec<WindowsLoaderPackageModuleBinding>,
    pub(super) system_module_bindings: Vec<WindowsLoaderSystemModuleBinding>,
    pub(super) resolved_filesystem_system_images:
        Vec<WindowsLoaderResolvedFilesystemSystemImageCustody>,
    pub(super) signed_system_dependencies: SealedSignedSystemDependencyAuthority,
    pub(super) resolved_system_dependencies: Vec<WindowsResolvedSystemDependencyAuthority>,
    pub(super) system_module_images: SealedWindowsSystemModuleImageAuthority,
    pub(super) preloaded_module_authority: SealedWindowsLoaderPreloadedModuleAuthority,
    pub(super) searched_names: Vec<WindowsLoaderSearchedNameBinding>,
    pub(super) pe_import_graph: SealedWindowsPeImportGraphAuthority,
    pub(super) launch_path_authority: SealedWindowsLoaderLaunchPathAuthority,
    pub(super) package_content_lease_set_digest: String,
    pub(super) system_content_lease_set_digest: String,
    pub(super) immutable_content_lease_set_digest: String,
    pub(super) resolution_profile_digest: String,
    pub(super) launch_context_selector_digest: String,
    pub(super) selected_context_binding_digest: String,
    pub(super) preliminary_resolution_request_plan_digest: String,
    pub(super) grant_ready_resolution_plan_digest: String,
    pub(super) process_machine_context_digest: String,
    pub(super) _producer_unavailable: Infallible,
}

/// Pre-barrier queryable kernel lease set. It owns only evidence available before the first close;
/// requiring a final receipt here would invert the transition's temporal ownership.
pub(super) struct SealedWindowsLoaderNamespacePrerequisite {
    pub(super) searched_name_grants: Vec<WindowsLoaderSearchedNameFenceCustody>,
    pub(super) launch_path_component_grants: Vec<WindowsLoaderLaunchPathGrantCustody>,
    pub(super) initial_query_attempt: ManagedLoaderNamespaceQueryAttemptCustody,
    pub(super) initial_query_receipt: ManagedLoaderNamespaceQueryReceipt,
    pub(super) session: ManagedLoaderNamespaceSession,
    pub(super) resolution_profile_digest: String,
    pub(super) preliminary_resolution_request_plan_digest: String,
    pub(super) grant_ready_resolution_plan_digest: String,
    pub(super) searched_name_set_digest: String,
    pub(super) fence_generation_set_digest: String,
    pub(super) _whole_resolution_fence_backend_unavailable: Infallible,
}

/// Grants and exact FileId content leases acquired before the consuming pre-barrier currentness
/// query. A query failure retains this owner rather than an impossible already-successful receipt.
pub(super) struct PreFinalWindowsLoaderNamespaceGrantSet<'root> {
    pub(super) resolution_custody: GrantAcquiredWindowsRunnerResolutionLeaseCustody<'root>,
    pub(super) searched_name_grants: Vec<WindowsLoaderSearchedNameFenceCustody>,
    pub(super) launch_path_component_grants: Vec<WindowsLoaderLaunchPathGrantCustody>,
    pub(super) session: ManagedLoaderNamespaceSession,
    pub(super) preliminary_resolution_request_plan_digest: String,
    pub(super) grant_ready_resolution_plan_digest: String,
    pub(super) searched_name_set_digest: String,
    pub(super) fence_generation_set_digest: String,
    pub(super) _whole_resolution_fence_backend_unavailable: Infallible,
}

/// Namespace grant/session lineage after every movable external directory and system candidate
/// has moved exactly once into final resolution/lease custody. It cannot duplicate those owners.
pub(super) struct PostLeaseWindowsLoaderNamespaceGrantSet<'root> {
    pub(super) resolution_lineage: PostLeaseWindowsRunnerResolutionLineage<'root>,
    pub(super) searched_name_grants: Vec<WindowsLoaderSearchedNameFenceCustody>,
    pub(super) launch_path_component_grants: Vec<WindowsLoaderLaunchPathGrantCustody>,
    pub(super) session: ManagedLoaderNamespaceSession,
    pub(super) preliminary_resolution_request_plan_digest: String,
    pub(super) grant_ready_resolution_plan_digest: String,
    pub(super) searched_name_set_digest: String,
    pub(super) fence_generation_set_digest: String,
    pub(super) _postlease_namespace_lineage_producer_unavailable: Infallible,
}

/// Final resolution can exist only after every grant and package/system content lease. Resolved
/// system-image leases have moved into `resolution`; package leases remain linear for the later
/// close/reopen transition. This whole pre-query owner has no producer in the current source.
pub(super) struct PostLeaseSealedWindowsRunnerLoadSetPreQueryPrerequisite<'root> {
    pub(super) namespace: PostLeaseWindowsLoaderNamespaceGrantSet<'root>,
    pub(super) resolution: SealedWindowsLoaderResolutionAuthority,
    pub(super) package_content_leases: Vec<WindowsLoaderPackageContentLeaseCustody>,
    pub(super) _postlease_final_resolution_sealer_unavailable: Infallible,
}

/// Final namespace authority created only after every package file crossed close/reopen and a new
/// authenticated query proved the original grant generation still current. No producer exists.
pub(super) struct SealedWindowsLoaderNamespaceAuthority {
    pub(super) prerequisite: SealedWindowsLoaderNamespacePrerequisite,
    pub(super) final_query_attempt: ManagedLoaderNamespaceQueryAttemptCustody,
    pub(super) final_query_receipt: ManagedLoaderNamespaceQueryReceipt,
    pub(super) namespace_authority_digest: String,
}

/// Both hard prerequisites must be acquired and query-verified before the owned transition enters
/// its irreversible close/reopen barrier. Namespace is declared first so any future explicit
/// release owner can release grants while resolution-directory handles are still retained.
pub(super) struct SealedWindowsRunnerLoadSetPrerequisite<'root> {
    pub(super) postlease_lineage: PostLeaseWindowsRunnerResolutionLineage<'root>,
    pub(super) namespace: SealedWindowsLoaderNamespacePrerequisite,
    pub(super) resolution: SealedWindowsLoaderResolutionAuthority,
    pub(super) package_content_leases: Vec<WindowsLoaderPackageContentLeaseCustody>,
}

/// Query-verified authority after its indexed content leases move linearly into file anchors. It
/// cannot authorize a second transition because the lease vector is no longer present.
pub(super) struct PostLeaseSplitWindowsRunnerLoadSetPrerequisite {
    pub(super) namespace: SealedWindowsLoaderNamespacePrerequisite,
    pub(super) resolution: SealedWindowsLoaderResolutionAuthority,
}

/// Post-barrier authority stored by a successful image. Namespace is first so an eventual explicit
/// release operation can consume it while resolution search-directory handles remain live.
pub(super) struct SealedWindowsRunnerLoadSetAuthority {
    pub(super) namespace: SealedWindowsLoaderNamespaceAuthority,
    pub(super) resolution: SealedWindowsLoaderResolutionAuthority,
}

impl fmt::Debug for SealedWindowsLoaderResolutionAuthority {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SealedWindowsLoaderResolutionAuthority")
            .field("resolution_profile_digest", &"<redacted>")
            .field("package_module_count", &self.package_module_bindings.len())
            .field("system_module_count", &self.system_module_bindings.len())
            .field("searched_name_count", &self.searched_names.len())
            .finish()
    }
}

impl fmt::Debug for SealedWindowsLoaderNamespaceAuthority {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SealedWindowsLoaderNamespaceAuthority")
            .field("grant_count", &self.prerequisite.searched_name_grants.len())
            .field("namespace_authority_digest", &"<redacted>")
            .finish()
    }
}

impl fmt::Debug for SealedWindowsLoaderNamespacePrerequisite {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SealedWindowsLoaderNamespacePrerequisite")
            .field("grant_count", &self.searched_name_grants.len())
            .field("initial_query_receipt", &self.initial_query_receipt)
            .finish()
    }
}

impl fmt::Debug for SealedWindowsRunnerLoadSetPrerequisite<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SealedWindowsRunnerLoadSetPrerequisite")
            .field("namespace", &self.namespace)
            .field("resolution", &self.resolution)
            .finish()
    }
}

impl fmt::Debug for SealedWindowsRunnerLoadSetAuthority {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SealedWindowsRunnerLoadSetAuthority")
            .field("namespace", &self.namespace)
            .field("resolution", &self.resolution)
            .finish()
    }
}
