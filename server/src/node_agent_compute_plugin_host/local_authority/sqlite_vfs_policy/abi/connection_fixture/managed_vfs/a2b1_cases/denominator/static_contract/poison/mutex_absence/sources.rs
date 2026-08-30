use std::collections::BTreeSet;

use sha2::{Digest, Sha256};

#[derive(Clone, Copy)]
struct SourceUnit {
    domain: Domain,
    path: &'static str,
    normalized_lf_sha256: &'static str,
    sentinels: &'static [(&'static str, usize)],
    source: &'static str,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Domain {
    Owner,
    Coordinator,
    SharedBoundary,
}

macro_rules! unit {
    ($domain:ident, $path:literal, $sha:literal, $sentinels:expr) => {
        SourceUnit {
            domain: Domain::$domain,
            path: $path,
            normalized_lf_sha256: $sha,
            sentinels: $sentinels,
            source: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path)),
        }
    };
}

const SOURCES: &[SourceUnit] = &[
    unit!(
        SharedBoundary,
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/file_state.rs",
        "a1c67654a2b0cddbdca60945b40c0fc3235ec3b495bf1f3a23d7632013e5eb3f",
        &[("pub(super) unsafe fn run_code", 1), ("catch_unwind(AssertUnwindSafe", 5)]
    ),
    unit!(
        Owner,
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/process_owner.rs",
        "435da236f705e4403d9caf5d7d083ec74b6a2af12a068c69629b59b458f8fa06",
        &[("fn lock_routes", 1), (".routes\n            .lock()", 1)]
    ),
    unit!(
        Owner,
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/process_owner/lifecycle.rs",
        "8153f10db8746ca99ff357d7999441e2e4b039fa9a15cbffe03e9da589bccc19",
        &[("#[cfg(not(test))]", 1), ("self.apply_route(route", 8)]
    ),
    unit!(
        Owner,
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/process_owner/vfs.rs",
        "98e817432b99fd548dce9fb9d9d0be2c2b1a02e5aa2282e77c31b7111d0aa38b",
        &[("fn project_x_open", 1), ("self.apply_route(route", 5)]
    ),
    unit!(
        Owner,
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/owner.rs",
        "fda3a859b2b65c96ff71ed7b4153ab2c8e35a88df48ae5458ce58af006bd0569",
        &[("debug_assert!(", 1), ("validated route must remain present under exclusive owner access", 2)]
    ),
    unit!(
        Owner,
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/owner/lifecycle.rs",
        "e6600610a3cdcba2bbe49576bfc77409d0aab37f86a889caf89540b41575075d",
        &[("fn retire_closed", 2), ("validated route must remain present under exclusive owner access", 1)]
    ),
    unit!(
        Owner,
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/owner/vfs.rs",
        "00a4856a24fdb72ac780ff84f2b00d3b598df41ec174580f6e312c1105538142",
        &[("fn project_x_open", 1), ("fn project_x_full_pathname", 1)]
    ),
    unit!(
        Owner,
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/state.rs",
        "919119f2d9edfa552cff43691c71a67b622f444ef29cdd30a133fa6143b49d09",
        &[("pub(super) fn begin_callback", 1), ("pub(super) fn finish_callback", 1)]
    ),
    unit!(
        Owner,
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/state/owner.rs",
        "67e6dab8b04cf71f4bb53c75951b1c45a3be45de3c22810f20ffb6dd4279a4d4",
        &[("fn new_pending", 1), ("fn phase", 1)]
    ),
    unit!(
        Owner,
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy.rs",
        "f987e79f3a32efbb927e7b82cc90a3d8859e976286b2f5a5ce337a699d39010c",
        &[("struct SealedHandleBoundSqlitePolicy", 1)]
    ),
    unit!(
        Owner,
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/authorizer.rs",
        "22a1daa195ff380f5603e5e5a5bc30221b75703b3cddce582547bc758d1f6882",
        &[("pub(super) fn enter_schema_migration", 1), ("pub(super) fn enter_runtime", 1)]
    ),
    unit!(
        Owner,
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/vfs_requests.rs",
        "3ddba018bfa7969ce4cadef14974003ead6f7d3b83a072464d3c2bc172899792",
        &[("fn project_x_open", 1), ("fn project_x_full_pathname", 1)]
    ),
    unit!(
        Owner,
        "src/node_agent_compute_plugin_host/local_authority/opened_authority.rs",
        "d404fc474ffedda50f09977834d05145acff27d8d3768b9db86238959ccf34d7",
        &[("impl ManagedSqliteRegistryCustody", 1), ("fn ensure_registry_current", 1)]
    ),
    unit!(
        Owner,
        "src/node_agent_compute_plugin_host/bootstrap/authority_controller_open.rs",
        "5fc13a636aff42215f740946aaca0bbe4632124758249acb4714bc4685403f59",
        &[("struct PinnedAuthorityOpenCustody", 1), ("fn retire(&self)", 2)]
    ),
    unit!(
        Owner,
        "src/node_agent_compute_plugin_host/bootstrap/authority_controller_state.rs",
        "5a51c0469d6fcd28f5dc857e3c5e33e57eff83b5e0f2589bee0a83197aa4a521",
        &[("pub(super) fn retire(&self)", 1), ("Ordering::Release", 4)]
    ),
    unit!(
        Coordinator,
        "src/node_agent_managed_fs/sqlite_namespace_shm/coordinator.rs",
        "b88619fd86f426f6e9b7dac450e99733c7838b2d48f7a73af73e44ead9609e7d",
        &[("fn attach(", 1), ("self.state.lock()", 1)]
    ),
    unit!(
        Coordinator,
        "src/node_agent_managed_fs/sqlite_namespace_shm/mapping.rs",
        "a620de365a5a66d92b34529010346c99402c6db8806e24e126218ae5e9ee6f76",
        &[("fn map_connection", 1), ("self.state.lock()", 1)]
    ),
    unit!(
        Coordinator,
        "src/node_agent_managed_fs/sqlite_namespace_shm/locking.rs",
        "7d3b6bab18e85f7ce21eeeed120285e8a66c3a408584aa93c1de05b5d2894293",
        &[("fn lock_connection", 1), ("self.state.lock()", 1)]
    ),
    unit!(
        Coordinator,
        "src/node_agent_managed_fs/sqlite_namespace_shm/node_initialization.rs",
        "c3ce09fbaadd9a217c1ecf72f65d32652f4c065bf3f4efc0f32d5e791d5549c0",
        &[("fn ensure_node", 1), ("fn open_node", 1)]
    ),
    unit!(
        Coordinator,
        "src/node_agent_managed_fs/sqlite_namespace_shm/unmap.rs",
        "5951119177139e9c9a8ab5b4fd5dd9a3bae91ce58538eb24bb1efdf390f8fdce",
        &[("fn unmap_connection", 1), ("self.state.lock()", 2)]
    ),
    unit!(
        Coordinator,
        "src/node_agent_managed_fs/sqlite_namespace_shm/barrier.rs",
        "48f24f35faa2f0b8b39d79269667692de5051eb2d2f8ffa6112d72d98c6a0dfb",
        &[("fn lock_barrier_state", 1), ("self.state.lock()", 1)]
    ),
    unit!(
        Coordinator,
        "src/node_agent_managed_fs/sqlite_namespace_shm/failure_custody.rs",
        "03edfdcca7851208c4eba11b783bcbc15ac8f19a1c7c9c685c1b9248dfe012a5",
        &[("fn consume_open_failure", 1), ("fn retain_handle_custody", 1)]
    ),
    unit!(
        Coordinator,
        "src/node_agent_managed_fs/sqlite_namespace_shm/close.rs",
        "6d18594eb5bafa355fb9cd5902b28ef8cb18712679cc0faee4af42d73a4c21fc",
        &[("pub(crate) fn close(", 1)]
    ),
    unit!(
        Coordinator,
        "src/node_agent_managed_fs/sqlite_namespace_shm/types.rs",
        "6bc1e7b4fd3f0a95376a070c7373c970c8357cfd28576a7c08399c7716b708b0",
        &[("struct ManagedSqliteShmBudget", 1), ("struct ManagedSqliteShmLockRequest", 1)]
    ),
    unit!(
        Coordinator,
        "src/node_agent_managed_fs/sqlite_namespace.rs",
        "0281b462b5e0564a2e550155f55dc49d6292cf9da2c0a790f1133ce52af2bd68",
        &[("fn open_exact", 1), ("fn delete_exact", 2)]
    ),
    unit!(
        Coordinator,
        "src/node_agent_managed_fs/sqlite_namespace_io.rs",
        "4fa2cdd4af3b1d0be21faab118cc233622a4011404aa9241acea95695c8d2e00",
        &[("fn truncate", 1), ("fn size", 1)]
    ),
    unit!(
        Coordinator,
        "src/node_agent_managed_fs/sqlite_namespace_close.rs",
        "a80219cd1436dea5ad13cffe2924eb462c3408969fb3bb97a2f821df36f116b6",
        &[("pub(crate) fn close(", 2)]
    ),
    unit!(
        Coordinator,
        "src/node_agent_managed_fs/windows_sqlite_shm.rs",
        "368d96ad83eb5bc080f03171307bb64915fc13dc18e7d69c7887300417c57244",
        &[("fn create_mapping", 1), ("fn map_view", 1), ("fn close_explicit", 2)]
    ),
    unit!(
        Coordinator,
        "src/node_agent_managed_fs/windows_sqlite_locking.rs",
        "a83861443215aa6e081fc45082d9d60bbd4c102d5ec05c82ae3282ccfb697b0b",
        &[("fn try_lock_sqlite_byte_range", 1), ("fn unlock_sqlite_byte_range(", 1)]
    ),
];

pub(super) fn validate() -> Result<(), String> {
    if SOURCES.len() != 29 {
        return Err(format!("source width changed: {}", SOURCES.len()));
    }
    if SOURCES
        .iter()
        .filter(|unit| unit.domain == Domain::Owner)
        .count()
        != 14
        || SOURCES
            .iter()
            .filter(|unit| unit.domain == Domain::Coordinator)
            .count()
            != 14
        || SOURCES
            .iter()
            .filter(|unit| unit.domain == Domain::SharedBoundary)
            .count()
            != 1
    {
        return Err("source domain width changed".to_owned());
    }
    let mut paths = BTreeSet::new();
    for unit in SOURCES {
        if !paths.insert(unit.path) {
            return Err(format!("duplicate source path: {}", unit.path));
        }
        validate_parts(
            unit.path,
            unit.normalized_lf_sha256,
            unit.sentinels,
            unit.source,
        )?;
    }
    let reviewed = SOURCES
        .iter()
        .map(|unit| super::candidates::SourceView {
            path: unit.path,
            source: unit.source,
        })
        .collect::<Vec<_>>();
    super::candidates::validate(&reviewed)?;
    Ok(())
}

fn validate_parts(
    path: &str,
    expected_sha256: &str,
    sentinels: &[(&str, usize)],
    source: &str,
) -> Result<(), String> {
    if expected_sha256.len() != 64
        || !expected_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("invalid normalized SHA-256 for {path}"));
    }
    let observed = normalized_sha256(source);
    if observed != expected_sha256 {
        return Err(format!(
            "normalized source SHA-256 changed for {path}: expected {expected_sha256}, observed {observed}"
        ));
    }
    for (needle, count) in sentinels {
        require_occurrences(path, source, needle, *count)?;
    }
    Ok(())
}

fn require_occurrences(
    label: &str,
    source: &str,
    needle: &str,
    expected: usize,
) -> Result<(), String> {
    let observed = source.matches(needle).count();
    if observed != expected {
        return Err(format!(
            "source sentinel drift for {label}: {needle:?} expected {expected}, observed {observed}"
        ));
    }
    Ok(())
}

fn normalized_sha256(source: &str) -> String {
    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    format!("{:x}", Sha256::digest(normalized.as_bytes()))
}

#[cfg(test)]
pub(super) fn validate_parts_for_test(
    path: &str,
    expected_sha256: &str,
    sentinels: &[(&str, usize)],
    source: &str,
) -> Result<(), String> {
    validate_parts(path, expected_sha256, sentinels, source)
}

#[cfg(test)]
pub(super) fn normalized_sha256_for_test(source: &str) -> String {
    normalized_sha256(source)
}
