use std::collections::BTreeSet;

use super::source_scope_support::is_lower_hex;
use super::{canonical, Digest32, SourceScopeFileV1};

mod git_baseline;
mod validation;

pub(crate) use git_baseline::{validate_baseline_path_blob, validate_baseline_path_blobs};
pub(crate) use validation::{validate_record_source_witnesses, validate_source_witness};

pub(crate) const SOURCE_BASELINE_COMMIT_SHA1: &str = "47cb2652321b42cc9689319075d253fe2275ace1";

/// A hand-reviewed production source owner.  The embedded source is used only to check that the
/// frozen metadata still names the bytes compiled by this checkout; it is deliberately omitted
/// from the canonical manifest representation.
#[derive(Clone, Copy)]
pub(crate) struct ProductionSourceSnapshotV1 {
    pub(crate) owner_id: &'static str,
    pub(crate) repo_relative_path: &'static str,
    pub(crate) git_blob_oid_sha1: &'static str,
    pub(crate) normalized_lf_sha256: &'static str,
    pub(crate) symbol_sentinels: &'static [&'static str],
    source: &'static str,
}

macro_rules! snapshot {
    ($owner_id:literal, $path:literal, $blob:literal, $sha256:literal, $symbols:expr) => {
        ProductionSourceSnapshotV1 {
            owner_id: $owner_id,
            repo_relative_path: $path,
            git_blob_oid_sha1: $blob,
            normalized_lf_sha256: $sha256,
            symbol_sentinels: $symbols,
            source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path)),
        }
    };
}

/// This list is intentionally independent from `static_contract::source::ProductionOwner`.
/// Adding an owner there does not silently update this authority: this frozen list and its digest
/// must be reviewed explicitly, which is the desired omission detector.
pub(crate) const PRODUCTION_SOURCE_SCOPE: &[ProductionSourceSnapshotV1; 29] = &[
    snapshot!(
        "sqlite-vfs-abi-table",
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi.rs",
        "16dbe67ed4ef85711e1cd4b01848b3dd4cdd73e0",
        "40d2c3054addb14b382c1480373d94bfb137381686207a7e8c5f62248df2a096",
        &["xShmMap: Some(io_shm::map)", "xShmLock: Some(io_shm::lock)"]
    ),
    snapshot!(
        "abi-boundary",
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/boundary.rs",
        "efe4d442cb1fc2295ccd18278f1b729c6600b91c",
        "7e403d878e6607affe9c4aa59985453923dcb3e55218fd0e6144f5e1ae0e626c",
        &["unsafe fn write_pointer_null"]
    ),
    snapshot!(
        "abi-io-shm",
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/io_shm.rs",
        "73a0cc74331dcbd78165834b679c47591ccc29f0",
        "68106ca2bc754c27ae4aa6dde33a3e9614c9f379c46abe8c28c688ec8981207e",
        &[
            "unsafe extern \"C\" fn map",
            "unsafe extern \"C\" fn lock",
            "fn sqlite_bool",
            "fn shm_lock_action",
        ]
    ),
    snapshot!(
        "abi-file-state",
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/file_state.rs",
        "1043664fa6bf563ddfdf2444440c8e72286b6216",
        "a1c67654a2b0cddbdca60945b40c0fc3235ec3b495bf1f3a23d7632013e5eb3f",
        &[
            "unsafe fn run_code",
            "pub(super) fn shm_map",
            "pub(super) fn shm_lock",
            "unsafe fn abandon_without_unwind",
        ]
    ),
    snapshot!(
        "abi-raw-state",
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/raw_state.rs",
        "f35e6bcd750d8b6a150375b980e8f9e33858bb41",
        "e720b32c811b806d7edf1b57a0a4cc39c19c6b224b8e1c2ebf223f916968a346",
        &[
            "unsafe fn with_installed_state",
            "unsafe fn installed_envelope",
            "fn validate_installed",
            "unsafe fn abandon_installed_state",
        ]
    ),
    snapshot!(
        "abi-result-codes",
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/result_codes.rs",
        "ad72c4b592694e5900b69e1ce508eca2fb3c1e32",
        "5a981032707eea30a9a2f4877aa34a16c872105c9abe0c4f9783fd516c6c3fe4",
        &["SHM_MAP_UNAVAILABLE", "SHM_LOCK_UNAVAILABLE"]
    ),
    snapshot!(
        "registry-abi-file",
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/abi.rs",
        "67d9c0a6b3b27bdc9128d823ef424dfc3ecf37fa",
        "73da3e9cddc44db475ebbec961d74b7655f0f7faeedc5e7bc7b743d84930bc61",
        &[
            "fn promote_main_to_wal",
            "fn install_exact_wal_main_shm_test_fault_script",
            "fn shm_map",
            "fn shm_lock",
        ]
    ),
    snapshot!(
        "registry-operations",
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/operations.rs",
        "d2bdf3c02f5f046cfc93610c89bdbe6c148503db",
        "4a3601b751a06a1a448866e43c562ee22c6762b6d11eea1fd3783d4a71370180",
        &[
            "fn shm_map",
            "fn shm_lock",
            "fn with_shm",
            "fn quarantine_unsafe_shm_failure",
        ]
    ),
    snapshot!(
        "registry-file-custody",
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody.rs",
        "e8dac8ea5612800d864339f38f3d8e3d97971b1d",
        "00b761728eb368258686325f35adaebe0d1c354b7b5b075ca0a9f9bd30613220",
        &[
            "struct ManagedSqliteRegistryPinnedFile",
            "impl<Custody, NonceSource> Drop",
        ]
    ),
    snapshot!(
        "registry-process-owner",
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/process_owner.rs",
        "f6fabb33e273a0541920b1dc48b7ce98171a41e4",
        "435da236f705e4403d9caf5d7d083ec74b6a2af12a068c69629b59b458f8fa06",
        &[
            "fn begin_callback",
            "fn finish_callback",
            "fn complete",
            "fn apply_route<T>",
            "fn drop",
        ]
    ),
    snapshot!(
        "registry-process-lifecycle",
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/process_owner/lifecycle.rs",
        "2ccdaa2d49563579db3a1c9c71c755a0c872823e",
        "8153f10db8746ca99ff357d7999441e2e4b039fa9a15cbffe03e9da589bccc19",
        &["fn retain_terminal_custody", "fn claim_shm"]
    ),
    snapshot!(
        "registry-owner",
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/owner.rs",
        "b790c2d82293ab5ec16a634e5d4abf0f7c314989",
        "fda3a859b2b65c96ff71ed7b4153ab2c8e35a88df48ae5458ce58af006bd0569",
        &["fn begin_callback", "fn finish_callback", "fn quarantine"]
    ),
    snapshot!(
        "registry-owner-lifecycle",
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/owner/lifecycle.rs",
        "eed2b7baa529d20c926c76cc74c4d48b2e33aa43",
        "e6600610a3cdcba2bbe49576bfc77409d0aab37f86a889caf89540b41575075d",
        &["fn claim_shm"]
    ),
    snapshot!(
        "registry-state",
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/state.rs",
        "7f5d71f3cf9ad4f22eda8650ff89a68928728a60",
        "919119f2d9edfa552cff43691c71a67b622f444ef29cdd30a133fa6143b49d09",
        &[
            "fn begin_callback",
            "fn finish_callback",
            "fn callback_allowed",
            "fn ensure_shape",
            "pub(super) fn claim_shm",
            "pub(super) fn quarantine",
        ]
    ),
    snapshot!(
        "managed-namespace",
        "src/node_agent_managed_fs/sqlite_namespace.rs",
        "0cef92af7a00847860585bac8954c0fa59fdf628",
        "0281b462b5e0564a2e550155f55dc49d6292cf9da2c0a790f1133ce52af2bd68",
        &["fn open_exact"]
    ),
    snapshot!(
        "managed-namespace-types",
        "src/node_agent_managed_fs/sqlite_namespace_types.rs",
        "c7e8672ab8639b70b1577df5d04f7669fc50794c",
        "25fe5b2f880a002448f038517cc6741dc778d54907edfb021e1a1824f15d84cd",
        &[
            "enum ManagedSqliteFileKind",
            "struct PinnedManagedSqliteFile",
            "struct ManagedSqliteFileOpenFailure",
            "struct ManagedSqliteDeleteFailure",
        ]
    ),
    snapshot!(
        "managed-fs-root",
        "src/node_agent_managed_fs.rs",
        "4cbf55b9c1983301c799578abf40037a2aa770f6",
        "01a32fb5b5ac98b12919836ab96b4824eb4a48f030644933a3739997880eaa06",
        &[
            "#[path = \"node_agent_managed_fs/windows.rs\"]",
            "mod platform",
        ]
    ),
    snapshot!(
        "managed-windows-platform",
        "src/node_agent_managed_fs/windows.rs",
        "40f5e1b4af58a503559705928e55693c9fcc72a1",
        "05abf0d86dbf2b97932181de34884bbfc32a0a0b2078d11686fa4ed4bbe95059",
        &[
            "mod sqlite_locking",
            "try_lock_sqlite_byte_range",
            "unlock_sqlite_byte_range",
        ]
    ),
    snapshot!(
        "managed-shm-root",
        "src/node_agent_managed_fs/sqlite_namespace_shm.rs",
        "255e47f26ce85681e43d2dcff536b532b867a36a",
        "563e32ffb3c8d8cc7506f040fdee53819483b2b1d33d328ede702abaaec684c2",
        &["fn open_shm_for_wal", "#[path = \"windows_sqlite_shm.rs\"]"]
    ),
    snapshot!(
        "managed-coordinator",
        "src/node_agent_managed_fs/sqlite_namespace_shm/coordinator.rs",
        "0ec47492fd6d4d90449ff807e7495ddac0347b59",
        "b88619fd86f426f6e9b7dac450e99733c7838b2d48f7a73af73e44ead9609e7d",
        &[
            "fn bind_main_file",
            "fn attach",
            "fn poisoned_failure",
            "fn mark_poisoned",
        ]
    ),
    snapshot!(
        "managed-types",
        "src/node_agent_managed_fs/sqlite_namespace_shm/types.rs",
        "dee4097de08650a146f2f6fbbcdb22169b05873a",
        "6bc1e7b4fd3f0a95376a070c7373c970c8357cfd28576a7c08399c7716b708b0",
        &[
            "struct ManagedSqliteShmBudget",
            "fn validate_region_size",
            "fn validate_logical_end",
            "fn validate_existing_size",
            "fn validate_mapped_total",
            "struct ManagedSqliteShmLockRequest",
            "fn new",
            "fn mask",
        ]
    ),
    snapshot!(
        "managed-initialization",
        "src/node_agent_managed_fs/sqlite_namespace_shm/node_initialization.rs",
        "d68c464d47598f910d71317c4615fdb3ced17c96",
        "c3ce09fbaadd9a217c1ecf72f65d32652f4c065bf3f4efc0f32d5e791d5549c0",
        &["fn ensure_node", "fn open_node", "fn close_failed_open_file"]
    ),
    snapshot!(
        "managed-failure-custody",
        "src/node_agent_managed_fs/sqlite_namespace_shm/failure_custody.rs",
        "76a01a132d7ffe31295b126bc488e094063c06fc",
        "03edfdcca7851208c4eba11b783bcbc15ac8f19a1c7c9c685c1b9248dfe012a5",
        &["fn consume_open_failure", "fn retain_handle_custody"]
    ),
    snapshot!(
        "managed-mapping",
        "src/node_agent_managed_fs/sqlite_namespace_shm/mapping.rs",
        "78159846a7051bfefcef021683b9f8d2b97ee0f2",
        "a620de365a5a66d92b34529010346c99402c6db8806e24e126218ae5e9ee6f76",
        &["fn map_connection", "fn map"]
    ),
    snapshot!(
        "managed-locking",
        "src/node_agent_managed_fs/sqlite_namespace_shm/locking.rs",
        "80852e1ef7687fe79e9cc2e780125441c04a7f2c",
        "7d3b6bab18e85f7ce21eeeed120285e8a66c3a408584aa93c1de05b5d2894293",
        &[
            "fn lock_connection",
            "fn try_os_lock",
            "fn unlock_os_range",
            "fn sibling_masks",
            "fn require_unlocked",
        ]
    ),
    snapshot!(
        "managed-namespace-io",
        "src/node_agent_managed_fs/sqlite_namespace_io.rs",
        "9b83825826f1e517d751106beead50770337bb45",
        "4fa2cdd4af3b1d0be21faab118cc233622a4011404aa9241acea95695c8d2e00",
        &["fn truncate", "fn size"]
    ),
    snapshot!(
        "managed-namespace-close",
        "src/node_agent_managed_fs/sqlite_namespace_close.rs",
        "930e791b0b679d3e34f5b760fb94f76696cd6aa4",
        "a80219cd1436dea5ad13cffe2924eb462c3408969fb3bb97a2f821df36f116b6",
        &["fn close"]
    ),
    snapshot!(
        "windows-shm",
        "src/node_agent_managed_fs/windows_sqlite_shm.rs",
        "ee1ec92f4f903c80653410a415a65c86f54fdd26",
        "368d96ad83eb5bc080f03171307bb64915fc13dc18e7d69c7887300417c57244",
        &[
            "fn allocation_granularity",
            "fn create_mapping",
            "fn map_view",
            "fn close_explicit",
        ]
    ),
    snapshot!(
        "windows-locking",
        "src/node_agent_managed_fs/windows_sqlite_locking.rs",
        "7b70c7e63317f851eef0b7e5cc92516edba905c5",
        "a83861443215aa6e081fc45082d9d60bbd4c102d5ec05c82ae3282ccfb697b0b",
        &["fn try_lock_sqlite_byte_range", "fn unlock_sqlite_byte_range"]
    ),
];

pub(crate) fn source_scope_files() -> Vec<SourceScopeFileV1> {
    PRODUCTION_SOURCE_SCOPE
        .iter()
        .map(|snapshot| SourceScopeFileV1 {
            owner_id: snapshot.owner_id.to_owned(),
            repo_relative_path: snapshot.repo_relative_path.to_owned(),
            git_blob_oid_sha1: snapshot.git_blob_oid_sha1.to_owned(),
            normalized_lf_sha256: snapshot.normalized_lf_sha256.to_owned(),
            symbol_sentinels: snapshot
                .symbol_sentinels
                .iter()
                .map(|symbol| (*symbol).to_owned())
                .collect(),
        })
        .collect()
}

pub(crate) fn source_scope_sha256() -> Result<Digest32, String> {
    validate_source_scope()?;
    canonical::digest_source_scope(&source_scope_files())
}

pub(crate) fn validate_source_scope() -> Result<(), String> {
    if !is_lower_hex(SOURCE_BASELINE_COMMIT_SHA1, 40) {
        return Err("source baseline commit is not a full lowercase SHA-1".to_owned());
    }
    if PRODUCTION_SOURCE_SCOPE.len() != 29 {
        return Err("production source scope must contain exactly 29 owners".to_owned());
    }

    let mut owner_ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for snapshot in PRODUCTION_SOURCE_SCOPE {
        validation::validate_snapshot_shape(snapshot)?;
        if !owner_ids.insert(snapshot.owner_id) {
            return Err(format!(
                "duplicate production source owner: {}",
                snapshot.owner_id
            ));
        }
        if !paths.insert(snapshot.repo_relative_path) {
            return Err(format!(
                "duplicate production source path: {}",
                snapshot.repo_relative_path
            ));
        }
        validation::validate_snapshot_bytes(snapshot)?;
    }

    validate_baseline_path_blobs(
        PRODUCTION_SOURCE_SCOPE
            .iter()
            .map(|snapshot| (snapshot.repo_relative_path, snapshot.git_blob_oid_sha1)),
    )?;

    canonical::digest_source_scope(&source_scope_files())?;
    Ok(())
}
