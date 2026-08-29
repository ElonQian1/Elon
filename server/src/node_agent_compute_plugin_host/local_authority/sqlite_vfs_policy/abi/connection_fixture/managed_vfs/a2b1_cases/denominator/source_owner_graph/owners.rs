use super::model::{OwnerSnapshot, SourceOwnerId};

pub(super) const SOURCE_BASELINE_COMMIT: &str = "baa5ad9ab1db404ea9393fcf24e37b4b5a7ed86a";

const fn owner(
    id: SourceOwnerId,
    path: &'static str,
    blob_oid: &'static str,
    normalized_sha256: &'static str,
    symbols: &'static [&'static str],
) -> OwnerSnapshot {
    OwnerSnapshot {
        id,
        path,
        blob_oid,
        normalized_sha256,
        symbols,
    }
}

pub(super) const OWNERS: &[OwnerSnapshot] = &[
    owner(SourceOwnerId::SqliteVfsAbiTable, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi.rs", "91d816d5920aa367645cad64a60d1dc3181f217a", "db123ebc386ed1247b5781c020690fa56175a6a0b0bf90a67589f1328fc84218", &["xShmMap: Some(io_shm::map)", "xShmLock: Some(io_shm::lock)"]),
    owner(SourceOwnerId::AbiBoundary, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/boundary.rs", "efe4d442cb1fc2295ccd18278f1b729c6600b91c", "7e403d878e6607affe9c4aa59985453923dcb3e55218fd0e6144f5e1ae0e626c", &["unsafe fn write_pointer_null"]),
    owner(SourceOwnerId::AbiIoShm, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/io_shm.rs", "73a0cc74331dcbd78165834b679c47591ccc29f0", "68106ca2bc754c27ae4aa6dde33a3e9614c9f379c46abe8c28c688ec8981207e", &["unsafe extern \"C\" fn map", "unsafe extern \"C\" fn lock", "fn sqlite_bool", "fn shm_lock_action"]),
    owner(SourceOwnerId::AbiFileState, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/file_state.rs", "9ec46df270d907d266be3a3534c62db7403b8dd5", "94f4b7642276b59a73cc5ff395346abcd07f8db9c0f506664a892d592163f1aa", &["unsafe fn run_code", "pub(super) fn shm_map", "pub(super) fn shm_lock", "unsafe fn abandon_without_unwind"]),
    owner(SourceOwnerId::AbiRawState, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/raw_state.rs", "c7414aace8c722511de7b4441566f362bbf2e52f", "07657a696fb36549cf9a9876ae043488ee32d675e1df090c312b681931c2a105", &["unsafe fn with_installed_state", "unsafe fn installed_envelope", "fn validate_installed", "unsafe fn abandon_installed_state"]),
    owner(SourceOwnerId::AbiRawCloseWitness, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/raw_state/close_witness.rs", "d754ff75bcd29c39686603ec536609bfab4bbc6c", "53dc1f86b0702221f1a74b75b03fc7a61486f3abac8d94e01cd3c9ffadc7bbe4", &["fn record_state_abandon"]),
    owner(SourceOwnerId::AbiResultCodes, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/result_codes.rs", "ad72c4b592694e5900b69e1ce508eca2fb3c1e32", "5a981032707eea30a9a2f4877aa34a16c872105c9abe0c4f9783fd516c6c3fe4", &["SHM_MAP_UNAVAILABLE", "SHM_LOCK_UNAVAILABLE"]),
    owner(SourceOwnerId::FixtureFaultFile, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/fault_script/file.rs", "f5640040a6da54ef88095e2991090e4be8a92c3b", "df951d65b7ee4e36293048a2f812e57a3b12333eec859a40f0a726b59e0ea0bd", &["struct ManagedTestFaultingFile", "fn shm_map", "fn shm_lock"]),
    owner(SourceOwnerId::FixtureFaultController, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/fault_script.rs", "a5f688bc87e9ca99ba429b4cbbd245ad21f68cf8", "bb635d55670ad5f4ad74736ad5e3f81ec1bd8009b13e739f6f03ebf1aca6cc96", &["fn begin_operation"]),
    owner(SourceOwnerId::FixtureRouteFile, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/route_file.rs", "7df50a671691a9758809e223422f125dd0539af6", "f881373617faf099da69732346e566b160dec1335fb594484d7756d26494aa3c", &["fn prepare_first_main_shm_map", "fn shm_map", "fn shm_lock"]),
    owner(SourceOwnerId::FixtureFaultPlan, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/shm_fault_script.rs", "da70c2068effb0e804e9a0647d824fa570f93124", "3cc338324a33de42852513383cd8a1d8f05b23760e73d1cccf5788f0c312a634", &["fn claim", "fn record_installed", "fn record_promoted"]),
    owner(SourceOwnerId::RegistryTestBridge, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/test_vfs_bridge/file.rs", "8dcd742dde9ada91dfeec9fe1f239ab7538c2b41", "e3738a0b3f3f6fe60d1e547390e058f11bbf3117c26fefab77928bad52a18a9a", &["fn prepare_wal_main_shm_test_fault_script", "fn promote_main_to_wal_for_shm", "fn retain_test_fault_bridge_failure", "fn shm_map", "fn shm_lock"]),
    owner(SourceOwnerId::RegistryAbiFile, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/abi.rs", "c84042f8dfc3062677357aad85f5d4da62bc75f6", "300a1e4d77dac03738d74ed69f4e7495a27117a835ad6787f7357703315ea8d3", &["fn promote_main_to_wal", "fn install_exact_wal_main_shm_test_fault_script", "fn shm_map", "fn shm_lock"]),
    owner(SourceOwnerId::RegistryPromotion, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/promotion.rs", "b209174429aa8815239d59771ed3cd155f561abf", "2374b4a08d66f93f8032de623cc52ba4c65132757f650c9047c83c775d87612a", &["fn promote_main_to_wal", "fn promote_main_custody"]),
    owner(SourceOwnerId::RegistryOperations, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/operations.rs", "d2bdf3c02f5f046cfc93610c89bdbe6c148503db", "4a3601b751a06a1a448866e43c562ee22c6762b6d11eea1fd3783d4a71370180", &["fn shm_map", "fn shm_lock", "fn with_shm", "fn quarantine_unsafe_shm_failure"]),
    owner(SourceOwnerId::RegistryFileCustody, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody.rs", "548fa788b8f1d116107fdaf4fc05248ad0b17e4d", "09db0b999bb367fe1d209a0c2a3d9dae88df371b22b99623dd97e5ce8a0bbebf", &["struct ManagedSqliteRegistryPinnedFile", "impl<Custody, NonceSource> Drop"]),
    owner(SourceOwnerId::RegistryFileFaults, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/test_faults.rs", "348210337ad13da3552904a9b58ce6749b687de0", "acd14e31f9504f4487ce3b0f2a035177c54601202233616f79e888b571d2b97f", &["fn install_exact_wal_main_shm_test_fault_script"]),
    owner(SourceOwnerId::RegistryProcessOwner, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/process_owner.rs", "a8e44edde893867ea0ac7675173df68b4e4051c7", "9c04721712b1d7aa03257648b661cc9470cc0113af23001547024539958a871b", &["fn begin_callback", "fn finish_callback", "fn complete", "fn apply_route<T>", "fn drop"]),
    owner(SourceOwnerId::RegistryProcessLifecycle, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/process_owner/lifecycle.rs", "59e9d509c62d0f91ca58df52383f332728c1c8e6", "47b92af7257b3f8fac914a8237a0873d440e3bcd4f25f89f4da965e694e30153", &["fn retain_terminal_custody", "fn claim_shm"]),
    owner(SourceOwnerId::RegistryOwner, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/owner.rs", "b790c2d82293ab5ec16a634e5d4abf0f7c314989", "fda3a859b2b65c96ff71ed7b4153ab2c8e35a88df48ae5458ce58af006bd0569", &["fn begin_callback", "fn finish_callback", "fn quarantine"]),
    owner(SourceOwnerId::RegistryOwnerLifecycle, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/owner/lifecycle.rs", "b469cfceb4bcfd84dd9444faeffef84533f685e1", "e95c1d6f6125be3a2352e308bb5333b15e04e3aec3918849eb5bbae17031a563", &["fn claim_shm"]),
    owner(SourceOwnerId::RegistryState, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/state.rs", "28195d9035b09e9f46047a3da45016bc2a976c23", "30020293e08dccbfe5703ea33e3c01ecbab5d67b89770a20d33b8c7a4fa53c74", &["fn begin_callback", "fn finish_callback", "fn callback_allowed", "fn ensure_shape", "pub(super) fn claim_shm", "pub(super) fn quarantine"]),
    owner(SourceOwnerId::ManagedNamespace, "src/node_agent_managed_fs/sqlite_namespace.rs", "cec06bb331ef2c7b967b80d61f79a6ef63d23e10", "9d81d759ce4511a44c2fd572eeb5d1b3a93377e0583d0e249205bcb0d320208e", &["fn open_exact"]),
    owner(SourceOwnerId::ManagedFsRoot, "src/node_agent_managed_fs.rs", "4cbf55b9c1983301c799578abf40037a2aa770f6", "01a32fb5b5ac98b12919836ab96b4824eb4a48f030644933a3739997880eaa06", &["#[path = \"node_agent_managed_fs/windows.rs\"]", "mod platform"]),
    owner(SourceOwnerId::ManagedWindowsPlatform, "src/node_agent_managed_fs/windows.rs", "9b3b6798fca43dfc54ef5c212420e7a43905121a", "d941c5d85995d7e6a6b2f088bcf40a9f8bd584c3c8903bf0a95fc066af61f57b", &["mod sqlite_locking", "try_lock_sqlite_byte_range", "unlock_sqlite_byte_range"]),
    owner(SourceOwnerId::ManagedShmRoot, "src/node_agent_managed_fs/sqlite_namespace_shm.rs", "e58c488c08ca7ce15fc0e0625b0e10e970f7efb1", "1d0d1c8b431a6c1d68698bb61628301f4a615b6aad5e2cd4b962e9bd508cfde9", &["fn open_shm_for_wal", "#[path = \"windows_sqlite_shm.rs\"]"]),
    owner(SourceOwnerId::ManagedCoordinator, "src/node_agent_managed_fs/sqlite_namespace_shm/coordinator.rs", "0ec47492fd6d4d90449ff807e7495ddac0347b59", "b88619fd86f426f6e9b7dac450e99733c7838b2d48f7a73af73e44ead9609e7d", &["fn bind_main_file", "fn attach", "fn poisoned_failure", "fn mark_poisoned"]),
    owner(SourceOwnerId::ManagedTypes, "src/node_agent_managed_fs/sqlite_namespace_shm/types.rs", "dee4097de08650a146f2f6fbbcdb22169b05873a", "6bc1e7b4fd3f0a95376a070c7373c970c8357cfd28576a7c08399c7716b708b0", &["struct ManagedSqliteShmBudget", "fn validate_region_size", "fn validate_logical_end", "fn validate_existing_size", "fn validate_mapped_total", "struct ManagedSqliteShmLockRequest", "fn new", "fn mask"]),
    owner(SourceOwnerId::ManagedInitialization, "src/node_agent_managed_fs/sqlite_namespace_shm/node_initialization.rs", "d68c464d47598f910d71317c4615fdb3ced17c96", "c3ce09fbaadd9a217c1ecf72f65d32652f4c065bf3f4efc0f32d5e791d5549c0", &["fn ensure_node", "fn open_node", "fn close_failed_open_file"]),
    owner(SourceOwnerId::ManagedFailureCustody, "src/node_agent_managed_fs/sqlite_namespace_shm/failure_custody.rs", "76a01a132d7ffe31295b126bc488e094063c06fc", "03edfdcca7851208c4eba11b783bcbc15ac8f19a1c7c9c685c1b9248dfe012a5", &["fn consume_open_failure", "fn retain_handle_custody"]),
    owner(SourceOwnerId::ManagedMapping, "src/node_agent_managed_fs/sqlite_namespace_shm/mapping.rs", "78159846a7051bfefcef021683b9f8d2b97ee0f2", "a620de365a5a66d92b34529010346c99402c6db8806e24e126218ae5e9ee6f76", &["fn map_connection", "fn map"]),
    owner(SourceOwnerId::ManagedLocking, "src/node_agent_managed_fs/sqlite_namespace_shm/locking.rs", "80852e1ef7687fe79e9cc2e780125441c04a7f2c", "7d3b6bab18e85f7ce21eeeed120285e8a66c3a408584aa93c1de05b5d2894293", &["fn lock_connection", "fn try_os_lock", "fn unlock_os_range", "fn sibling_masks", "fn require_unlocked"]),
    owner(SourceOwnerId::ManagedFaultApi, "src/node_agent_managed_fs/sqlite_namespace_shm/test_faults/api.rs", "99cabef770ffc0741651ba2fb24eccf8b213d3b6", "8538d0cd4fef6b6d5627d8ef38ab46592577730a0c76ca7b1a27f9d763071c4a", &["fn observe_test_fault", "fn trigger_before_test_fault", "fn trigger_after_test_fault", "fn install_shm_test_fault_script"]),
    owner(SourceOwnerId::ManagedFaultController, "src/node_agent_managed_fs/sqlite_namespace_shm/test_faults/controller.rs", "e33d69678d1853acb54fbff9ea5377c09a27ce71", "5a253bc83a85d3156dadbe9c650878154dde94464fadbdf8bf9c0d5813444b66", &["pub(super) fn install", "pub(super) fn observe", "fn activate_before", "fn activate_after", "fn supports_after_success"]),
    owner(SourceOwnerId::ManagedFaultOperation, "src/node_agent_managed_fs/sqlite_namespace_shm/test_faults/operation.rs", "7528138f68ae1ae640eda2ad57acaae74139f25b", "6a192e526ebf77eaf55ce4b3b471da20e085696dc1ed15e4d32d06d1ea1ade0b", &["fn begin_test_fault", "fn finish_test_fault", "fn terminalize_test_fault"]),
    owner(SourceOwnerId::ManagedFaultMapping, "src/node_agent_managed_fs/sqlite_namespace_shm/test_faults/mapping.rs", "a5d7a5126b6b199f519d38e6569def88b5cb13cd", "4a6ea6ff1664de6e67db06e5dac184e8a606edadc8b94dc7b9f983c04c6ca215", &["fn retain_test_mapping_custody"]),
    owner(SourceOwnerId::ManagedNamespaceIo, "src/node_agent_managed_fs/sqlite_namespace_io.rs", "9b83825826f1e517d751106beead50770337bb45", "4fa2cdd4af3b1d0be21faab118cc233622a4011404aa9241acea95695c8d2e00", &["fn truncate", "fn size"]),
    owner(SourceOwnerId::ManagedNamespaceClose, "src/node_agent_managed_fs/sqlite_namespace_close.rs", "c8970538c916ff700a0feec8df577006cd732c34", "cb8f10c5c1b93bd345bcddab60bac15688bca4a5d5347dd0525f3949b9f28cf1", &["fn close"]),
    owner(SourceOwnerId::WindowsShm, "src/node_agent_managed_fs/windows_sqlite_shm.rs", "ee1ec92f4f903c80653410a415a65c86f54fdd26", "368d96ad83eb5bc080f03171307bb64915fc13dc18e7d69c7887300417c57244", &["fn allocation_granularity", "fn create_mapping", "fn map_view", "fn close_explicit"]),
    owner(SourceOwnerId::WindowsLocking, "src/node_agent_managed_fs/windows_sqlite_locking.rs", "12c27011dac42e786e9bcd7d3b92501847632ded", "4318f5382740dcdb645871e3ddc9d9d62febfab0dd5095279307864817ef64ce", &["fn try_lock_sqlite_byte_range", "fn unlock_sqlite_byte_range"]),
];

macro_rules! source {
    ($path:literal) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path))
    };
}

pub(super) fn source_content(id: SourceOwnerId) -> &'static str {
    match id {
        SourceOwnerId::SqliteVfsAbiTable => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi.rs"),
        SourceOwnerId::AbiBoundary => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/boundary.rs"),
        SourceOwnerId::AbiIoShm => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/io_shm.rs"),
        SourceOwnerId::AbiFileState => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/file_state.rs"),
        SourceOwnerId::AbiRawState => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/raw_state.rs"),
        SourceOwnerId::AbiRawCloseWitness => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/raw_state/close_witness.rs"),
        SourceOwnerId::AbiResultCodes => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/result_codes.rs"),
        SourceOwnerId::FixtureFaultFile => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/fault_script/file.rs"),
        SourceOwnerId::FixtureFaultController => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/fault_script.rs"),
        SourceOwnerId::FixtureRouteFile => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/route_file.rs"),
        SourceOwnerId::FixtureFaultPlan => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/shm_fault_script.rs"),
        SourceOwnerId::RegistryTestBridge => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/test_vfs_bridge/file.rs"),
        SourceOwnerId::RegistryAbiFile => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/abi.rs"),
        SourceOwnerId::RegistryPromotion => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/promotion.rs"),
        SourceOwnerId::RegistryOperations => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/operations.rs"),
        SourceOwnerId::RegistryFileCustody => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody.rs"),
        SourceOwnerId::RegistryFileFaults => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/test_faults.rs"),
        SourceOwnerId::RegistryProcessOwner => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/process_owner.rs"),
        SourceOwnerId::RegistryProcessLifecycle => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/process_owner/lifecycle.rs"),
        SourceOwnerId::RegistryOwner => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/owner.rs"),
        SourceOwnerId::RegistryOwnerLifecycle => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/owner/lifecycle.rs"),
        SourceOwnerId::RegistryState => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/state.rs"),
        SourceOwnerId::ManagedNamespace => source!("src/node_agent_managed_fs/sqlite_namespace.rs"),
        SourceOwnerId::ManagedFsRoot => source!("src/node_agent_managed_fs.rs"),
        SourceOwnerId::ManagedWindowsPlatform => source!("src/node_agent_managed_fs/windows.rs"),
        SourceOwnerId::ManagedShmRoot => source!("src/node_agent_managed_fs/sqlite_namespace_shm.rs"),
        SourceOwnerId::ManagedCoordinator => source!("src/node_agent_managed_fs/sqlite_namespace_shm/coordinator.rs"),
        SourceOwnerId::ManagedTypes => source!("src/node_agent_managed_fs/sqlite_namespace_shm/types.rs"),
        SourceOwnerId::ManagedInitialization => source!("src/node_agent_managed_fs/sqlite_namespace_shm/node_initialization.rs"),
        SourceOwnerId::ManagedFailureCustody => source!("src/node_agent_managed_fs/sqlite_namespace_shm/failure_custody.rs"),
        SourceOwnerId::ManagedMapping => source!("src/node_agent_managed_fs/sqlite_namespace_shm/mapping.rs"),
        SourceOwnerId::ManagedLocking => source!("src/node_agent_managed_fs/sqlite_namespace_shm/locking.rs"),
        SourceOwnerId::ManagedFaultApi => source!("src/node_agent_managed_fs/sqlite_namespace_shm/test_faults/api.rs"),
        SourceOwnerId::ManagedFaultController => source!("src/node_agent_managed_fs/sqlite_namespace_shm/test_faults/controller.rs"),
        SourceOwnerId::ManagedFaultOperation => source!("src/node_agent_managed_fs/sqlite_namespace_shm/test_faults/operation.rs"),
        SourceOwnerId::ManagedFaultMapping => source!("src/node_agent_managed_fs/sqlite_namespace_shm/test_faults/mapping.rs"),
        SourceOwnerId::ManagedNamespaceIo => source!("src/node_agent_managed_fs/sqlite_namespace_io.rs"),
        SourceOwnerId::ManagedNamespaceClose => source!("src/node_agent_managed_fs/sqlite_namespace_close.rs"),
        SourceOwnerId::WindowsShm => source!("src/node_agent_managed_fs/windows_sqlite_shm.rs"),
        SourceOwnerId::WindowsLocking => source!("src/node_agent_managed_fs/windows_sqlite_locking.rs"),
    }
}
