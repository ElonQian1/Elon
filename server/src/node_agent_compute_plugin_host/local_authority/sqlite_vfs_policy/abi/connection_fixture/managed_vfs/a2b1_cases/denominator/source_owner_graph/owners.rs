use super::model::{OwnerSnapshot, SourceOwnerId};

pub(super) const SOURCE_BASELINE_COMMIT: &str = "2cdbd70f75e18a16352d7cd1002e8b711bc11375";

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
    owner(SourceOwnerId::SqliteVfsAbiTable, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi.rs", "16dbe67ed4ef85711e1cd4b01848b3dd4cdd73e0", "40d2c3054addb14b382c1480373d94bfb137381686207a7e8c5f62248df2a096", &["xShmMap: Some(io_shm::map)", "xShmLock: Some(io_shm::lock)"]),
    owner(SourceOwnerId::AbiBoundary, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/boundary.rs", "efe4d442cb1fc2295ccd18278f1b729c6600b91c", "7e403d878e6607affe9c4aa59985453923dcb3e55218fd0e6144f5e1ae0e626c", &["unsafe fn write_pointer_null"]),
    owner(SourceOwnerId::AbiIoShm, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/io_shm.rs", "73a0cc74331dcbd78165834b679c47591ccc29f0", "68106ca2bc754c27ae4aa6dde33a3e9614c9f379c46abe8c28c688ec8981207e", &["unsafe extern \"C\" fn map", "unsafe extern \"C\" fn lock", "fn sqlite_bool", "fn shm_lock_action"]),
    owner(SourceOwnerId::AbiFileState, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/file_state.rs", "1043664fa6bf563ddfdf2444440c8e72286b6216", "a1c67654a2b0cddbdca60945b40c0fc3235ec3b495bf1f3a23d7632013e5eb3f", &["unsafe fn run_code", "pub(super) fn shm_map", "pub(super) fn shm_lock", "unsafe fn abandon_without_unwind"]),
    owner(SourceOwnerId::AbiRawState, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/raw_state.rs", "f35e6bcd750d8b6a150375b980e8f9e33858bb41", "e720b32c811b806d7edf1b57a0a4cc39c19c6b224b8e1c2ebf223f916968a346", &["unsafe fn with_installed_state", "unsafe fn installed_envelope", "fn validate_installed", "unsafe fn abandon_installed_state"]),
    owner(SourceOwnerId::AbiRawCloseWitness, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/raw_state/close_witness.rs", "72fd66d333def8a1b0bb12995cafeba3011097e9", "a89d0680f9a600baf6441c10cd2b129f56ebe13e4ec201f9c9fef48ab3339d17", &["fn record_state_abandon"]),
    owner(SourceOwnerId::AbiResultCodes, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/result_codes.rs", "ad72c4b592694e5900b69e1ce508eca2fb3c1e32", "5a981032707eea30a9a2f4877aa34a16c872105c9abe0c4f9783fd516c6c3fe4", &["SHM_MAP_UNAVAILABLE", "SHM_LOCK_UNAVAILABLE"]),
    owner(SourceOwnerId::FixtureFaultFile, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/fault_script/file.rs", "f5640040a6da54ef88095e2991090e4be8a92c3b", "df951d65b7ee4e36293048a2f812e57a3b12333eec859a40f0a726b59e0ea0bd", &["struct ManagedTestFaultingFile", "fn shm_map", "fn shm_lock"]),
    owner(SourceOwnerId::FixtureFaultController, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/fault_script.rs", "a5f688bc87e9ca99ba429b4cbbd245ad21f68cf8", "bb635d55670ad5f4ad74736ad5e3f81ec1bd8009b13e739f6f03ebf1aca6cc96", &["fn begin_operation"]),
    owner(SourceOwnerId::FixtureRouteFile, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/route_file.rs", "7df50a671691a9758809e223422f125dd0539af6", "f881373617faf099da69732346e566b160dec1335fb594484d7756d26494aa3c", &["fn prepare_first_main_shm_map", "fn shm_map", "fn shm_lock"]),
    owner(SourceOwnerId::FixtureFaultPlan, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/shm_fault_script.rs", "5c083ec59e7ac747ff0b55ab3384bce2a4991cf2", "5822ed864a733f1dda6e74d9e0f84c1e0107a59e055339764b3704c656635211", &["pub(super) fn exact_target_presence", "fn claim", "fn record_installed", "fn record_promoted"]),
    owner(SourceOwnerId::RegistryTestBridge, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/test_vfs_bridge/file.rs", "63756e8e172fd4207fbbb46ccfc2153ab7d4a739", "f372d6efee40f7ceabe0e782c2be46e7f7a5d2f1f41fbdaaf0daee934b6b9272", &["fn prepare_wal_main_shm_test_fault_script", "fn promote_main_to_wal_for_shm", "fn retain_test_fault_bridge_failure", "fn shm_map", "fn shm_lock"]),
    owner(SourceOwnerId::RegistryAbiFile, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/abi.rs", "67d9c0a6b3b27bdc9128d823ef424dfc3ecf37fa", "73da3e9cddc44db475ebbec961d74b7655f0f7faeedc5e7bc7b743d84930bc61", &["fn promote_main_to_wal", "fn install_exact_wal_main_shm_test_fault_script", "fn shm_map", "fn shm_lock"]),
    owner(SourceOwnerId::RegistryPromotion, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/promotion.rs", "b209174429aa8815239d59771ed3cd155f561abf", "2374b4a08d66f93f8032de623cc52ba4c65132757f650c9047c83c775d87612a", &["fn promote_main_to_wal", "fn promote_main_custody"]),
    owner(SourceOwnerId::RegistryOperations, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/operations.rs", "d2bdf3c02f5f046cfc93610c89bdbe6c148503db", "4a3601b751a06a1a448866e43c562ee22c6762b6d11eea1fd3783d4a71370180", &["fn shm_map", "fn shm_lock", "fn with_shm", "fn quarantine_unsafe_shm_failure"]),
    owner(SourceOwnerId::RegistryFileCustody, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody.rs", "e8dac8ea5612800d864339f38f3d8e3d97971b1d", "00b761728eb368258686325f35adaebe0d1c354b7b5b075ca0a9f9bd30613220", &["struct ManagedSqliteRegistryPinnedFile", "impl<Custody, NonceSource> Drop"]),
    owner(SourceOwnerId::RegistryFileFaults, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/test_faults.rs", "348210337ad13da3552904a9b58ce6749b687de0", "acd14e31f9504f4487ce3b0f2a035177c54601202233616f79e888b571d2b97f", &["fn install_exact_wal_main_shm_test_fault_script"]),
    owner(SourceOwnerId::RegistryProcessOwner, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/process_owner.rs", "f6fabb33e273a0541920b1dc48b7ce98171a41e4", "435da236f705e4403d9caf5d7d083ec74b6a2af12a068c69629b59b458f8fa06", &["fn begin_callback", "fn finish_callback", "fn complete", "fn apply_route<T>", "fn drop"]),
    owner(SourceOwnerId::RegistryProcessLifecycle, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/process_owner/lifecycle.rs", "2ccdaa2d49563579db3a1c9c71c755a0c872823e", "8153f10db8746ca99ff357d7999441e2e4b039fa9a15cbffe03e9da589bccc19", &["fn retain_terminal_custody", "fn claim_shm"]),
    owner(SourceOwnerId::RegistryOwner, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/owner.rs", "b790c2d82293ab5ec16a634e5d4abf0f7c314989", "fda3a859b2b65c96ff71ed7b4153ab2c8e35a88df48ae5458ce58af006bd0569", &["fn begin_callback", "fn finish_callback", "fn quarantine"]),
    owner(SourceOwnerId::RegistryOwnerLifecycle, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/owner/lifecycle.rs", "eed2b7baa529d20c926c76cc74c4d48b2e33aa43", "e6600610a3cdcba2bbe49576bfc77409d0aab37f86a889caf89540b41575075d", &["fn claim_shm"]),
    owner(SourceOwnerId::RegistryState, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/state.rs", "7f5d71f3cf9ad4f22eda8650ff89a68928728a60", "919119f2d9edfa552cff43691c71a67b622f444ef29cdd30a133fa6143b49d09", &["fn begin_callback", "fn finish_callback", "fn callback_allowed", "fn ensure_shape", "pub(super) fn claim_shm", "pub(super) fn quarantine"]),
    owner(SourceOwnerId::ManagedNamespace, "src/node_agent_managed_fs/sqlite_namespace.rs", "52f0dc807798e121bd009ad637012a8c71e1524f", "731f40a95f4e5d69a65da0bec91685ef7ff91577244aec0060edc6e25939689b", &["fn open_exact"]),
    owner(SourceOwnerId::ManagedFsRoot, "src/node_agent_managed_fs.rs", "4cbf55b9c1983301c799578abf40037a2aa770f6", "01a32fb5b5ac98b12919836ab96b4824eb4a48f030644933a3739997880eaa06", &["#[path = \"node_agent_managed_fs/windows.rs\"]", "mod platform"]),
    owner(SourceOwnerId::ManagedWindowsPlatform, "src/node_agent_managed_fs/windows.rs", "40f5e1b4af58a503559705928e55693c9fcc72a1", "05abf0d86dbf2b97932181de34884bbfc32a0a0b2078d11686fa4ed4bbe95059", &["mod sqlite_locking", "try_lock_sqlite_byte_range", "unlock_sqlite_byte_range"]),
    owner(SourceOwnerId::ManagedShmRoot, "src/node_agent_managed_fs/sqlite_namespace_shm.rs", "3165a8e0f7dc8efd7fcdd846cbd526dd1623b4b6", "78d0c91bb6648637c89066c5f136a2c4d351363f909812e66f791f93cac560a9", &["fn open_shm_for_wal", "#[path = \"windows_sqlite_shm.rs\"]"]),
    owner(SourceOwnerId::ManagedCoordinator, "src/node_agent_managed_fs/sqlite_namespace_shm/coordinator.rs", "b5ac83def2d2a8edbfd8ce7bec23525eb6efa510", "d201d46c05faa5599372136dae120b402b60e1d5daa894a08e8b1394014b075e", &["fn bind_main_file", "fn attach", "fn poisoned_failure", "fn mark_poisoned"]),
    owner(SourceOwnerId::ManagedTypes, "src/node_agent_managed_fs/sqlite_namespace_shm/types.rs", "dee4097de08650a146f2f6fbbcdb22169b05873a", "6bc1e7b4fd3f0a95376a070c7373c970c8357cfd28576a7c08399c7716b708b0", &["struct ManagedSqliteShmBudget", "fn validate_region_size", "fn validate_logical_end", "fn validate_existing_size", "fn validate_mapped_total", "struct ManagedSqliteShmLockRequest", "fn new", "fn mask"]),
    owner(SourceOwnerId::ManagedInitialization, "src/node_agent_managed_fs/sqlite_namespace_shm/node_initialization.rs", "d68c464d47598f910d71317c4615fdb3ced17c96", "c3ce09fbaadd9a217c1ecf72f65d32652f4c065bf3f4efc0f32d5e791d5549c0", &["fn ensure_node", "fn open_node", "fn close_failed_open_file"]),
    owner(SourceOwnerId::ManagedFailureCustody, "src/node_agent_managed_fs/sqlite_namespace_shm/failure_custody.rs", "76a01a132d7ffe31295b126bc488e094063c06fc", "03edfdcca7851208c4eba11b783bcbc15ac8f19a1c7c9c685c1b9248dfe012a5", &["fn consume_open_failure", "fn retain_handle_custody"]),
    owner(SourceOwnerId::ManagedMapping, "src/node_agent_managed_fs/sqlite_namespace_shm/mapping.rs", "48b1a467a3076bb7dd278ede8c245ebc7260e9b3", "1dc5a7eac8e997a010ceebd8136ab4845ce09f2f5480015e749c792cf6b1e7ab", &["fn map_connection", "fn map"]),
    owner(SourceOwnerId::ManagedLocking, "src/node_agent_managed_fs/sqlite_namespace_shm/locking.rs", "dc81d3ee80a261c98f11a70a77cdcf18e6a1cf9a", "608fd31846690268947915955630476e46863f41dc66f11dc6114f38a0b1b0d4", &["fn lock_connection", "fn try_os_lock", "fn unlock_os_range", "fn sibling_masks", "fn require_unlocked"]),
    owner(SourceOwnerId::ManagedFaultApi, "src/node_agent_managed_fs/sqlite_namespace_shm/test_faults/api.rs", "27f52a7280f06a6dee122fd2f301ee1298770ff5", "1b9a0c31d8513c470e9bc230ffed110323f34426ece0929f2a4f7cd005310422", &["fn observe_test_fault", "fn trigger_before_test_fault", "fn trigger_after_test_fault", "fn install_shm_test_fault_script"]),
    owner(SourceOwnerId::ManagedFaultController, "src/node_agent_managed_fs/sqlite_namespace_shm/test_faults/controller.rs", "e33d69678d1853acb54fbff9ea5377c09a27ce71", "5a253bc83a85d3156dadbe9c650878154dde94464fadbdf8bf9c0d5813444b66", &["pub(super) fn install", "pub(super) fn observe", "fn activate_before", "fn activate_after", "fn supports_after_success"]),
    owner(SourceOwnerId::ManagedFaultOperation, "src/node_agent_managed_fs/sqlite_namespace_shm/test_faults/operation.rs", "7528138f68ae1ae640eda2ad57acaae74139f25b", "6a192e526ebf77eaf55ce4b3b471da20e085696dc1ed15e4d32d06d1ea1ade0b", &["fn begin_test_fault", "fn finish_test_fault", "fn terminalize_test_fault"]),
    owner(SourceOwnerId::ManagedFaultMapping, "src/node_agent_managed_fs/sqlite_namespace_shm/test_faults/mapping.rs", "a5d7a5126b6b199f519d38e6569def88b5cb13cd", "4a6ea6ff1664de6e67db06e5dac184e8a606edadc8b94dc7b9f983c04c6ca215", &["fn retain_test_mapping_custody"]),
    owner(SourceOwnerId::ManagedNamespaceIo, "src/node_agent_managed_fs/sqlite_namespace_io.rs", "9b83825826f1e517d751106beead50770337bb45", "4fa2cdd4af3b1d0be21faab118cc233622a4011404aa9241acea95695c8d2e00", &["fn truncate", "fn size"]),
    owner(SourceOwnerId::ManagedNamespaceClose, "src/node_agent_managed_fs/sqlite_namespace_close.rs", "930e791b0b679d3e34f5b760fb94f76696cd6aa4", "a80219cd1436dea5ad13cffe2924eb462c3408969fb3bb97a2f821df36f116b6", &["fn close"]),
    owner(SourceOwnerId::WindowsShm, "src/node_agent_managed_fs/windows_sqlite_shm.rs", "ee1ec92f4f903c80653410a415a65c86f54fdd26", "368d96ad83eb5bc080f03171307bb64915fc13dc18e7d69c7887300417c57244", &["fn allocation_granularity", "fn create_mapping", "fn map_view", "fn close_explicit"]),
    owner(SourceOwnerId::WindowsLocking, "src/node_agent_managed_fs/windows_sqlite_locking.rs", "7b70c7e63317f851eef0b7e5cc92516edba905c5", "a83861443215aa6e081fc45082d9d60bbd4c102d5ec05c82ae3282ccfb697b0b", &["fn try_lock_sqlite_byte_range", "fn unlock_sqlite_byte_range"]),
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
