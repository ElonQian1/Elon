use super::model::{OwnerSnapshot, SourceOwnerId};

pub(super) const SOURCE_BASELINE_COMMIT: &str = "2a16dbbe5cb9235a9926ae8b09130a1f7fbaf67a";

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
    owner(SourceOwnerId::SqliteVfsAbiTable, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi.rs", "fdeb5a1b824fdab50301f1140b5ba1625c4ecc5f", "9ad001575eaad3781fdccee7ab681e0bfe8e1c1e5cb7484a0c930b9354864b10", &["xShmMap: Some(io_shm::map)", "xShmLock: Some(io_shm::lock)"]),
    owner(SourceOwnerId::AbiBoundary, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/boundary.rs", "efe4d442cb1fc2295ccd18278f1b729c6600b91c", "7e403d878e6607affe9c4aa59985453923dcb3e55218fd0e6144f5e1ae0e626c", &["unsafe fn write_pointer_null"]),
    owner(SourceOwnerId::AbiIoShm, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/io_shm.rs", "73a0cc74331dcbd78165834b679c47591ccc29f0", "68106ca2bc754c27ae4aa6dde33a3e9614c9f379c46abe8c28c688ec8981207e", &["unsafe extern \"C\" fn map", "unsafe extern \"C\" fn lock", "fn sqlite_bool", "fn shm_lock_action"]),
    owner(SourceOwnerId::AbiFileState, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/file_state.rs", "69ac0d68f20e7de933af154ac6ae3a85c60eb016", "b7b24ce79bddde0e9a9defddeed2f5979467980d4dfd4f861b35b0fe4639050a", &["unsafe fn run_code", "pub(super) fn shm_map", "pub(super) fn shm_lock", "unsafe fn abandon_without_unwind"]),
    owner(SourceOwnerId::AbiRawState, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/raw_state.rs", "db99e3c9c7ac65b0b25073fc6bc5cf0f9db081bd", "4048ef236fa6722780ee43bdbd54c3aac92f0aa42e24c02181072538b42f2ad5", &["unsafe fn with_installed_state", "unsafe fn installed_envelope", "fn validate_installed", "unsafe fn abandon_installed_state"]),
    owner(SourceOwnerId::AbiResultCodes, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/result_codes.rs", "ad72c4b592694e5900b69e1ce508eca2fb3c1e32", "5a981032707eea30a9a2f4877aa34a16c872105c9abe0c4f9783fd516c6c3fe4", &["SHM_MAP_UNAVAILABLE", "SHM_LOCK_UNAVAILABLE"]),
    owner(SourceOwnerId::FixtureFaultFile, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/fault_script/file.rs", "f5640040a6da54ef88095e2991090e4be8a92c3b", "df951d65b7ee4e36293048a2f812e57a3b12333eec859a40f0a726b59e0ea0bd", &["struct ManagedTestFaultingFile", "fn shm_map", "fn shm_lock"]),
    owner(SourceOwnerId::FixtureFaultController, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/fault_script.rs", "9ba7e6986ddcdf028bb4fdf469b1ccacb70a237f", "7e0f61a71163a3014d93ae050e89693e9ace3bf9af3e77371022ced67f923f7d", &["fn begin_operation"]),
    owner(SourceOwnerId::FixtureRouteFile, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/route_file.rs", "5c78b0c8fafa62c434a0c9cbd499d96c1519f427", "81b7cb529f2a238a418f55774ceefd99819b632674d3a29bfd8636511ed65e12", &["fn prepare_first_main_shm_map", "fn shm_map", "fn shm_lock"]),
    owner(SourceOwnerId::FixtureFaultPlan, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/shm_fault_script.rs", "9662db509ca3ca97858950ee8f276b7e02f1e679", "a72c59b8379a1f31376bf6635bd0c021387623effb2e807d77b2e79925314282", &["fn claim", "fn record_installed"]),
    owner(SourceOwnerId::RegistryTestBridge, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/test_vfs_bridge/file.rs", "d01954b4d9ef3f3d76ba475a1f270e8a3acfefcc", "398d1ba0c1dcfc71299e857f31c67d283f4d95cd88dd4b4c4aa10c475bed4af1", &["fn prepare_wal_main_shm_test_fault_script", "fn promote_main_to_wal_for_shm", "fn retain_test_fault_bridge_failure", "fn shm_map", "fn shm_lock"]),
    owner(SourceOwnerId::RegistryAbiFile, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/abi.rs", "16cb57485626550fd2bc18bb54923797ebac1a11", "ed3dabadb4b9f2577bd1e1b0aa3e29a1929100201e61a99b2fc98b95b260722c", &["fn promote_main_to_wal", "fn install_exact_wal_main_shm_test_fault_script", "fn shm_map", "fn shm_lock"]),
    owner(SourceOwnerId::RegistryPromotion, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/promotion.rs", "b209174429aa8815239d59771ed3cd155f561abf", "2374b4a08d66f93f8032de623cc52ba4c65132757f650c9047c83c775d87612a", &["fn promote_main_to_wal", "fn promote_main_custody"]),
    owner(SourceOwnerId::RegistryOperations, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/operations.rs", "ad1cf04ec60aa56f9572c89b86795bd4e6f4fc63", "e29e8bf3b66f0500270d60b04050f3df020bb9fc12f121e216e1daa60b2e999a", &["fn shm_map", "fn shm_lock", "fn with_shm", "fn quarantine_unsafe_shm_failure"]),
    owner(SourceOwnerId::RegistryFileCustody, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody.rs", "5a07c436864990e7d970259fb0e538393f073d73", "e55edc5ed8a02d9541b24db40675c94e6822f92d84ddbeb3e465244325461509", &["struct ManagedSqliteRegistryPinnedFile", "impl<Custody, NonceSource> Drop"]),
    owner(SourceOwnerId::RegistryFileFaults, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/test_faults.rs", "974ffe5dc0c29bb117e46a7bcccbd903bb242e24", "06d00940a1edd8ff9e5df0b477e08b24c5787e76c7c022b9481105def9b2829f", &["fn install_exact_wal_main_shm_test_fault_script"]),
    owner(SourceOwnerId::RegistryProcessOwner, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/process_owner.rs", "226b23b9683afbe90167c87c04eba713a2595548", "585ff5fd05b2cf1ec089b964d0d73c972f29589a1ebaae7dda9e3c3fec1669f2", &["fn begin_callback", "fn finish_callback", "fn complete", "fn apply_route<T>", "fn drop"]),
    owner(SourceOwnerId::RegistryProcessLifecycle, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/process_owner/lifecycle.rs", "8905c18fa5a31356d370b6b551a6d0736b8ef888", "1e34f6cd61f8c600e845bd4f2ebfd0a0bb96ef11d9eb20961910c1e04b47f01d", &["fn retain_terminal_custody", "fn claim_shm"]),
    owner(SourceOwnerId::RegistryOwner, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/owner.rs", "2bc494425ac1975e6394b8d05f2bc3bd6368823d", "06de34d4dcba937c845494b4423b64f19f727a545a7f27e7469ace5c1c73a17e", &["fn begin_callback", "fn finish_callback", "fn quarantine"]),
    owner(SourceOwnerId::RegistryOwnerLifecycle, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/owner/lifecycle.rs", "f3285629bc4048c5be1e68e2f8e7a711d8d72426", "3a3c7782e9d791a7cdf528bd5941a16216c8d499b791c6cc7ff7beb18c7b5423", &["fn claim_shm"]),
    owner(SourceOwnerId::RegistryState, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/state.rs", "e7f727c69d14cfb579393edb014e0bbf2c47723f", "aa2857e0e3092d5facb168f1149d0e8048b169b8907a44ae0415dae695468cf7", &["fn begin_callback", "fn finish_callback", "fn callback_allowed", "fn ensure_shape", "pub(super) fn claim_shm", "pub(super) fn quarantine"]),
    owner(SourceOwnerId::ManagedNamespace, "src/node_agent_managed_fs/sqlite_namespace.rs", "386afa5e1899bec65c81687e528ca14d3ead0a97", "c46624b25e788f7b94692f3f96ad104ef23ccfce4f97c1a96dd4161765f4adff", &["fn open_exact"]),
    owner(SourceOwnerId::ManagedFsRoot, "src/node_agent_managed_fs.rs", "4cbf55b9c1983301c799578abf40037a2aa770f6", "01a32fb5b5ac98b12919836ab96b4824eb4a48f030644933a3739997880eaa06", &["#[path = \"node_agent_managed_fs/windows.rs\"]", "mod platform"]),
    owner(SourceOwnerId::ManagedWindowsPlatform, "src/node_agent_managed_fs/windows.rs", "889aac1d41b5f6e03af3771ead1f840fa2128d09", "e9d617d655ce8e7d85b3659042e0702befe9e0a230739fe5121bea085d42df81", &["mod sqlite_locking", "try_lock_sqlite_byte_range", "unlock_sqlite_byte_range"]),
    owner(SourceOwnerId::ManagedShmRoot, "src/node_agent_managed_fs/sqlite_namespace_shm.rs", "0f5b1ab17b0d16e1fa77b0cc3687173fca120f42", "281b7c9502fe1c6f54b32546aa4f6c676568131ea99caa7aad5f3be277b32877", &["fn open_shm_for_wal", "#[path = \"windows_sqlite_shm.rs\"]"]),
    owner(SourceOwnerId::ManagedCoordinator, "src/node_agent_managed_fs/sqlite_namespace_shm/coordinator.rs", "767543acc8be88a3853847eaa4d86463633f3bd3", "d159963a082b989d51ddb5cdab2d429ba6207db8767b54abb250a25353c8c662", &["fn bind_main_file", "fn attach", "fn poisoned_failure", "fn mark_poisoned"]),
    owner(SourceOwnerId::ManagedTypes, "src/node_agent_managed_fs/sqlite_namespace_shm/types.rs", "dee4097de08650a146f2f6fbbcdb22169b05873a", "6bc1e7b4fd3f0a95376a070c7373c970c8357cfd28576a7c08399c7716b708b0", &["struct ManagedSqliteShmBudget", "fn validate_region_size", "fn validate_logical_end", "fn validate_existing_size", "fn validate_mapped_total", "struct ManagedSqliteShmLockRequest", "fn new", "fn mask"]),
    owner(SourceOwnerId::ManagedInitialization, "src/node_agent_managed_fs/sqlite_namespace_shm/node_initialization.rs", "d68c464d47598f910d71317c4615fdb3ced17c96", "c3ce09fbaadd9a217c1ecf72f65d32652f4c065bf3f4efc0f32d5e791d5549c0", &["fn ensure_node", "fn open_node", "fn close_failed_open_file"]),
    owner(SourceOwnerId::ManagedFailureCustody, "src/node_agent_managed_fs/sqlite_namespace_shm/failure_custody.rs", "76a01a132d7ffe31295b126bc488e094063c06fc", "03edfdcca7851208c4eba11b783bcbc15ac8f19a1c7c9c685c1b9248dfe012a5", &["fn consume_open_failure", "fn retain_handle_custody"]),
    owner(SourceOwnerId::ManagedMapping, "src/node_agent_managed_fs/sqlite_namespace_shm/mapping.rs", "78159846a7051bfefcef021683b9f8d2b97ee0f2", "a620de365a5a66d92b34529010346c99402c6db8806e24e126218ae5e9ee6f76", &["fn map_connection", "fn map"]),
    owner(SourceOwnerId::ManagedLocking, "src/node_agent_managed_fs/sqlite_namespace_shm/locking.rs", "80852e1ef7687fe79e9cc2e780125441c04a7f2c", "7d3b6bab18e85f7ce21eeeed120285e8a66c3a408584aa93c1de05b5d2894293", &["fn lock_connection", "fn try_os_lock", "fn unlock_os_range", "fn sibling_masks", "fn require_unlocked"]),
    owner(SourceOwnerId::ManagedFaultApi, "src/node_agent_managed_fs/sqlite_namespace_shm/test_faults/api.rs", "8e9f478fde7a846728611b24c0dbe5fa1e5ec3a4", "e2763b0bcb8a095e486f9f36a5be851a5db36b5b6cf675ec2739dd25aef41bf6", &["fn observe_test_fault", "fn trigger_before_test_fault", "fn trigger_after_test_fault", "fn install_shm_test_fault_script"]),
    owner(SourceOwnerId::ManagedFaultController, "src/node_agent_managed_fs/sqlite_namespace_shm/test_faults/controller.rs", "cd11041f2cc3e9752d97dce0ec2c6d67cfa95e76", "364420b24c56d1a04c8ea500c1570bab7655ec1bec089f7f991dccf80d85b22d", &["pub(super) fn install", "pub(super) fn observe", "fn activate_before", "fn activate_after", "fn supports_after_success"]),
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
