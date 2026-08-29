use super::model::{OwnerSnapshot, SourceOwnerId};

pub(super) const SOURCE_BASELINE_COMMIT: &str = "df38ff849d2b402bb818be51c01a11912f293a09";

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
    owner(SourceOwnerId::FixtureRouteFile, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/route_file.rs", "cd63ff94a2c7f8315cf4de5bfc9b1538fec4028d", "e2c86db3244a39b68e2ace38eeef4bea329a57db2cacfb100a49c5d8230c811f", &["fn prepare_first_main_shm_map", "fn shm_map", "fn shm_lock"]),
    owner(SourceOwnerId::FixtureFaultPlan, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/shm_fault_script.rs", "bfecd43918c2693e921b84b6d47ec3214fa1eb17", "4c24629c9246c05243c7986e46a1e3e20c5f8e598469db6e76ad80e06fae1e0b", &["fn claim", "fn record_installed", "fn record_promoted"]),
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
    owner(SourceOwnerId::ManagedNamespace, "src/node_agent_managed_fs/sqlite_namespace.rs", "0de5ecbec9123e5e6e3a4a36b4b404f1be25bc4a", "29c328639728f57a62312f053abfc7288567b3e0510ec846b2198cab35cb70cb", &["fn open_exact"]),
    owner(SourceOwnerId::ManagedFsRoot, "src/node_agent_managed_fs.rs", "4cbf55b9c1983301c799578abf40037a2aa770f6", "01a32fb5b5ac98b12919836ab96b4824eb4a48f030644933a3739997880eaa06", &["#[path = \"node_agent_managed_fs/windows.rs\"]", "mod platform"]),
    owner(SourceOwnerId::ManagedWindowsPlatform, "src/node_agent_managed_fs/windows.rs", "889aac1d41b5f6e03af3771ead1f840fa2128d09", "e9d617d655ce8e7d85b3659042e0702befe9e0a230739fe5121bea085d42df81", &["mod sqlite_locking", "try_lock_sqlite_byte_range", "unlock_sqlite_byte_range"]),
    owner(SourceOwnerId::ManagedShmRoot, "src/node_agent_managed_fs/sqlite_namespace_shm.rs", "71e13e317356b31014f9e144d96d4a7a98f54cfb", "3a17e4db53679ba80da41718100071dce0ccb1d661b57e1903897de5e9a9be3b", &["fn open_shm_for_wal", "#[path = \"windows_sqlite_shm.rs\"]"]),
    owner(SourceOwnerId::ManagedCoordinator, "src/node_agent_managed_fs/sqlite_namespace_shm/coordinator.rs", "767543acc8be88a3853847eaa4d86463633f3bd3", "d159963a082b989d51ddb5cdab2d429ba6207db8767b54abb250a25353c8c662", &["fn bind_main_file", "fn attach", "fn poisoned_failure", "fn mark_poisoned"]),
    owner(SourceOwnerId::ManagedTypes, "src/node_agent_managed_fs/sqlite_namespace_shm/types.rs", "dee4097de08650a146f2f6fbbcdb22169b05873a", "6bc1e7b4fd3f0a95376a070c7373c970c8357cfd28576a7c08399c7716b708b0", &["struct ManagedSqliteShmBudget", "fn validate_region_size", "fn validate_logical_end", "fn validate_existing_size", "fn validate_mapped_total", "struct ManagedSqliteShmLockRequest", "fn new", "fn mask"]),
    owner(SourceOwnerId::ManagedInitialization, "src/node_agent_managed_fs/sqlite_namespace_shm/node_initialization.rs", "d68c464d47598f910d71317c4615fdb3ced17c96", "c3ce09fbaadd9a217c1ecf72f65d32652f4c065bf3f4efc0f32d5e791d5549c0", &["fn ensure_node", "fn open_node", "fn close_failed_open_file"]),
    owner(SourceOwnerId::ManagedFailureCustody, "src/node_agent_managed_fs/sqlite_namespace_shm/failure_custody.rs", "76a01a132d7ffe31295b126bc488e094063c06fc", "03edfdcca7851208c4eba11b783bcbc15ac8f19a1c7c9c685c1b9248dfe012a5", &["fn consume_open_failure", "fn retain_handle_custody"]),
    owner(SourceOwnerId::ManagedMapping, "src/node_agent_managed_fs/sqlite_namespace_shm/mapping.rs", "78159846a7051bfefcef021683b9f8d2b97ee0f2", "a620de365a5a66d92b34529010346c99402c6db8806e24e126218ae5e9ee6f76", &["fn map_connection", "fn map"]),
    owner(SourceOwnerId::ManagedLocking, "src/node_agent_managed_fs/sqlite_namespace_shm/locking.rs", "80852e1ef7687fe79e9cc2e780125441c04a7f2c", "7d3b6bab18e85f7ce21eeeed120285e8a66c3a408584aa93c1de05b5d2894293", &["fn lock_connection", "fn try_os_lock", "fn unlock_os_range", "fn sibling_masks", "fn require_unlocked"]),
    owner(SourceOwnerId::ManagedFaultApi, "src/node_agent_managed_fs/sqlite_namespace_shm/test_faults/api.rs", "480fd477e9fe6ecab060f7446b6236b8aa9d43fe", "b2d06bb54b3e0146a352e1be9a012235b14c92c6b7f6924e90cc9dac698edb07", &["fn observe_test_fault", "fn trigger_before_test_fault", "fn trigger_after_test_fault", "fn install_shm_test_fault_script"]),
    owner(SourceOwnerId::ManagedFaultController, "src/node_agent_managed_fs/sqlite_namespace_shm/test_faults/controller.rs", "f24befcc407e38c8ef8933f42f4727b39142c1cd", "faa6534f3ef075667cd7fcdb5ec9e4f0f6991c14ab66d42897a3a094e39a7b95", &["pub(super) fn install", "pub(super) fn observe", "fn activate_before", "fn activate_after", "fn supports_after_success"]),
    owner(SourceOwnerId::ManagedFaultOperation, "src/node_agent_managed_fs/sqlite_namespace_shm/test_faults/operation.rs", "7528138f68ae1ae640eda2ad57acaae74139f25b", "6a192e526ebf77eaf55ce4b3b471da20e085696dc1ed15e4d32d06d1ea1ade0b", &["fn begin_test_fault", "fn finish_test_fault", "fn terminalize_test_fault"]),
    owner(SourceOwnerId::ManagedFaultMapping, "src/node_agent_managed_fs/sqlite_namespace_shm/test_faults/mapping.rs", "a5d7a5126b6b199f519d38e6569def88b5cb13cd", "4a6ea6ff1664de6e67db06e5dac184e8a606edadc8b94dc7b9f983c04c6ca215", &["fn retain_test_mapping_custody"]),
    owner(SourceOwnerId::ManagedNamespaceIo, "src/node_agent_managed_fs/sqlite_namespace_io.rs", "9b83825826f1e517d751106beead50770337bb45", "4fa2cdd4af3b1d0be21faab118cc233622a4011404aa9241acea95695c8d2e00", &["fn truncate", "fn size"]),
    owner(SourceOwnerId::ManagedNamespaceClose, "src/node_agent_managed_fs/sqlite_namespace_close.rs", "5a7cbe33337085a49b8484020f6e0cda95f47c21", "9915f3288555c8fa68ff60b1f003b9af0624362dc687070e5365dcad197bfb31", &["fn close"]),
    owner(SourceOwnerId::WindowsShm, "src/node_agent_managed_fs/windows_sqlite_shm.rs", "8f417d20bfe6d177a95772d486809bac52e1ae6a", "5a39254f15b08036f1cfdd5c011ae9cd41e40ecfb76c5f7cb6984f70adf7b481", &["fn allocation_granularity", "fn create_mapping", "fn map_view", "fn close_explicit"]),
    owner(SourceOwnerId::WindowsLocking, "src/node_agent_managed_fs/windows_sqlite_locking.rs", "ebce7d95b0816c593ebea8fdda08133b40d634d3", "53d61f1f7390934bfee84f3e517f367d5ad880458b7b2a28ec1bde493e0a7fb2", &["fn try_lock_sqlite_byte_range", "fn unlock_sqlite_byte_range"]),
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
