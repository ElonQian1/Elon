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
const POLICY: &str = include_str!("runtime_loader_load_set/policy.rs");
const RESOLUTION: &str = include_str!("runtime_loader_load_set/resolution.rs");

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
        "launch_path_exact_context_selection",
        "launch_path_component_grant_backend",
    ] {
        assert!(POLICY.contains(blocker), "missing blocker {blocker}");
    }
    assert!(POLICY.contains("discover_retained_launch_path_candidates_and_prelease_pe_material"));
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
