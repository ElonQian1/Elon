const HOST_MODULE: &str = include_str!("mod.rs");
const MANAGED_FACADE: &str = include_str!("../node_agent_managed_fs.rs");
const MANAGED_CUSTODY: &str =
    include_str!("../node_agent_managed_fs/extraction_loader_directory.rs");
const MANAGED_WINDOWS: &str = include_str!("../node_agent_managed_fs/windows.rs");
const MANAGED_WINDOWS_PROBE: &str =
    include_str!("../node_agent_managed_fs/windows_extraction_loader_directory.rs");
const STAGING: &str = include_str!("fetch_file/staging.rs");
const EXTRACTION: &str = include_str!("candidate_extraction/zip/extract.rs");
const TRANSITION: &str = include_str!("runtime_loader_load_set/transition.rs");
const POLICY: &str = include_str!("runtime_loader_load_set/policy.rs");

fn source_between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    source
        .split_once(start)
        .unwrap_or_else(|| panic!("source start marker missing: {start}"))
        .1
        .split_once(end)
        .unwrap_or_else(|| panic!("source end marker missing: {end}"))
        .0
}

#[test]
fn managed_fs_exports_purpose_specific_linear_extraction_loader_directory_custody() {
    assert!(HOST_MODULE.contains("mod runtime_extraction_loader_directory_source_contract_tests;"));
    assert!(MANAGED_FACADE.contains("mod extraction_loader_directory;"));
    for exported in [
        "ManagedExtractionLoaderDirectoryChildFailure",
        "ManagedExtractionLoaderDirectoryFailure",
        "ManagedExtractionLoaderFileFailure",
        "PinnedManagedExtractionLoaderDirectory",
    ] {
        assert!(
            MANAGED_FACADE.contains(exported),
            "missing export {exported}"
        );
    }
    assert!(!MANAGED_FACADE.contains("PlatformExtractionLoaderDirectoryProbe"));
    assert!(MANAGED_CUSTODY.contains(
        "pub(crate) struct PinnedManagedExtractionLoaderDirectory {\n    directory: PinnedManagedDirectory,\n    _share_delete_probe: File,\n    _receipt: ManagedExtractionLoaderDirectoryShareReceipt,\n}"
    ));
    assert!(MANAGED_CUSTODY.contains("fn into_extraction_loader_directory_custody("));
    assert!(MANAGED_CUSTODY.contains("fn create_new_extraction_loader_directory_child("));
    assert!(MANAGED_CUSTODY.contains("fn create_new_extraction_loader_file_child("));
    assert!(MANAGED_CUSTODY.contains("fn create_new_directory_child("));
    assert!(MANAGED_CUSTODY.contains("fn create_new_file_child("));
    assert!(MANAGED_CUSTODY.contains("fn into_loader_parts(self) -> Self"));
    assert!(MANAGED_CUSTODY.contains("fn into_cleanup_directory(self)"));
    assert!(
        MANAGED_CUSTODY.contains("let parent_handle = parent.directory_handles.last().ok_or_else")
    );
    assert!(
        MANAGED_CUSTODY.contains("platform::create_new_directory_relative(parent_handle, name)")
    );
    assert!(!MANAGED_CUSTODY.contains("fn prepare_directory("));
    assert!(!MANAGED_CUSTODY.contains("open_managed_directory_relative"));
    assert!(!MANAGED_CUSTODY.contains("open_directory_relative_deletable"));
    assert!(!MANAGED_CUSTODY.contains("ErrorKind::AlreadyExists"));
    assert!(MANAGED_CUSTODY.contains("same_file_identity(owner_identity, probe_identity)"));
    assert!(MANAGED_CUSTODY.contains("probe_path != owner_path"));
}

#[test]
fn windows_probe_is_share_delete_compatible_and_read_only() {
    assert!(MANAGED_WINDOWS.contains(
        "pub(super) use extraction_loader_directory::probe_extraction_loader_directory_relative;"
    ));
    assert!(MANAGED_WINDOWS_PROBE.contains(
        "const PROBE_DESIRED_ACCESS: u32 = FILE_READ_ATTRIBUTES | FILE_TRAVERSE | SYNCHRONIZE;"
    ));
    assert!(MANAGED_WINDOWS_PROBE.contains(
        "const PROBE_SHARE_ACCESS: u32 = FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE;"
    ));
    assert!(MANAGED_WINDOWS_PROBE.contains("NtQueryInformationFile("));
    assert!(MANAGED_WINDOWS_PROBE.contains("FileAccessInformation"));
    assert!(MANAGED_WINDOWS_PROBE.contains("retained_access & DELETE != DELETE"));
    assert!(MANAGED_WINDOWS_PROBE.contains("probe_access & (DELETE | FILE_WRITE_DATA) != 0"));

    let ordinary_open = source_between(
        MANAGED_WINDOWS,
        "pub(super) fn open_managed_directory_relative(",
        "pub(super) fn create_new_directory_relative(",
    );
    assert!(ordinary_open
        .contains("FILE_READ_ATTRIBUTES | FILE_TRAVERSE | FILE_WRITE_DATA | SYNCHRONIZE"));
    assert!(ordinary_open.contains("FILE_SHARE_READ | FILE_SHARE_WRITE"));
    assert!(!ordinary_open.contains("FILE_SHARE_DELETE"));
}

#[test]
fn staging_and_loader_transition_retain_the_typed_directory_custody() {
    assert!(STAGING.contains("directory: PinnedManagedExtractionLoaderDirectory"));
    assert!(STAGING.contains("package_root:\n        PinnedManagedExtractionLoaderDirectory"));
    assert!(STAGING.contains(".into_extraction_loader_directory_custody()"));
    assert!(STAGING.contains("self.directory.into_cleanup_directory()"));
    assert!(STAGING.contains("package_root: self.directory.into_loader_parts()"));
    assert!(!STAGING.contains("fn prepare_directory("));
    assert!(!STAGING.contains("fn create_new_file("));

    let root_directory_child = source_between(
        STAGING,
        "fn create_new_directory_child(",
        "fn create_new_file_child(",
    );
    assert!(root_directory_child.contains("self.directory"));
    assert!(root_directory_child.contains(".create_new_directory_child(name)"));
    assert!(!root_directory_child.contains("self.root"));
    assert!(!root_directory_child.contains("pin_existing_directory"));
    assert!(!root_directory_child.contains("prepare_directory"));

    let root_file_child = source_between(
        STAGING,
        "fn create_new_file_child(",
        "fn create_new_seal_file(",
    );
    assert!(root_file_child.contains("self.directory.create_new_file_child(name)"));
    assert!(!root_file_child.contains("self.root"));
    assert!(!root_file_child.contains("pin_existing_directory"));

    let seal_file = source_between(
        STAGING,
        "fn create_new_seal_file(",
        "fn pin_cleanup_ancestors(",
    );
    assert!(seal_file.contains("self.create_new_file_child"));
    assert!(!seal_file.contains("self.root"));
    assert!(!seal_file.contains("pin_existing_directory"));

    assert!(EXTRACTION.contains("let mut directory_indexes = HashMap::with_capacity("));
    assert!(EXTRACTION.contains("directories[parent_index]"));
    assert!(EXTRACTION.contains(".create_new_extraction_loader_directory_child(name)"));
    assert!(EXTRACTION.contains("let retained_directories: HashMap<_, _>"));
    assert!(EXTRACTION.contains(".create_new_extraction_loader_file_child(name)"));
    assert!(!EXTRACTION.contains("staging.prepare_directory(relative)"));
    assert!(!EXTRACTION.contains("staging.create_new_file(&expected.relative_path)"));
    assert!(!EXTRACTION.contains("pin_existing_directory"));

    assert!(TRANSITION.contains("package_root_directory: PinnedManagedExtractionLoaderDirectory"));
    assert!(TRANSITION.contains("package_root_directory: staging.package_root"));
    assert!(POLICY.contains(
        "(\n        \"existing_extraction_directory_access_share_compatibility\",\n        \"source_seam_written_windows_dynamic_unverified\",\n    )"
    ));
}

#[test]
fn typed_custody_has_no_clone_serde_or_raw_handle_escape() {
    for forbidden in ["Clone", "Serialize", "Deserialize"] {
        assert!(
            !MANAGED_CUSTODY.contains(forbidden),
            "linear managed custody must not expose {forbidden}"
        );
    }
    for forbidden in [
        "impl Clone for PinnedManagedExtractionLoaderDirectory",
        "Serialize for PinnedManagedExtractionLoaderDirectory",
        "Deserialize<'de> for PinnedManagedExtractionLoaderDirectory",
        "fn as_raw_handle(",
        "fn raw_handle(",
        "fn into_raw_handle(",
        "fn borrowed_handle(",
        "fn as_file(",
        "fn into_file(",
        "RawHandle",
        "BorrowedHandle",
    ] {
        assert!(
            !MANAGED_CUSTODY.contains(forbidden),
            "typed custody escape present: {forbidden}"
        );
    }
    assert!(!MANAGED_CUSTODY.contains("serde::"));
    assert!(!STAGING.contains("as_raw_handle"));
    assert!(!TRANSITION.contains("as_raw_handle"));
    assert!(!STAGING.contains("impl Clone for PreparedComputePluginCandidateStaging"));
    assert!(!TRANSITION.contains("impl Clone for RawDestructuredLoaderTransitionCustody"));
}
