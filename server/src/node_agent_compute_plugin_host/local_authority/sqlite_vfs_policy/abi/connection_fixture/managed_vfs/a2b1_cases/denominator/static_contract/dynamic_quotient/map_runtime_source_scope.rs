//! Shared source commitment for the Map runners' real open/map/close path.
//!
//! The q3 single-region and q4 region-loop programs use the same managed VFS, registry and
//! managed-filesystem cleanup chain. Keeping that transitive closure in one list prevents their
//! implementation digests from silently drifting apart.

macro_rules! source {
    ($name:literal, $path:literal) => {
        (
            $name,
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), $path)),
        )
    };
}

pub(super) const MAP_RUNTIME_DEPENDENCY_SOURCE_SCOPE_V1: &[(&str, &str)] = &[
    source!("managed_vfs/connection/registry_lifecycle.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/connection/registry_lifecycle.rs"),
    source!("managed_vfs/shared_namespace/registration_shutdown.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/shared_namespace/registration_shutdown.rs"),
    source!("managed_vfs/shared_namespace/registry_lifecycle.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/shared_namespace/registry_lifecycle.rs"),
    source!("managed_vfs/lifecycle_faults.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/lifecycle_faults.rs"),
    source!("managed_vfs/lifecycle_faults/native_gate.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/lifecycle_faults/native_gate.rs"),
    source!("managed_vfs/lifecycle_faults/registry_lifecycle.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/lifecycle_faults/registry_lifecycle.rs"),
    source!("managed_vfs/lifecycle_faults/registry_lifecycle/binding.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/lifecycle_faults/registry_lifecycle/binding.rs"),
    source!("managed_vfs/lifecycle_faults/unmap.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/lifecycle_faults/unmap.rs"),
    source!("managed_vfs/lifecycle_faults/joint_close.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/lifecycle_faults/joint_close.rs"),
    source!("managed_vfs/lifecycle_faults/registration_shutdown.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/lifecycle_faults/registration_shutdown.rs"),
    source!("managed_vfs/registration_shutdown_custody.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/registration_shutdown_custody.rs"),
    source!("registry.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry.rs"),
    source!("registry/test_vfs_bridge.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/test_vfs_bridge.rs"),
    source!("registry/file_custody.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody.rs"),
    source!("registry/file_custody/joint_close_runtime.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/joint_close_runtime.rs"),
    source!("registry/file_custody/lifecycle_events.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/lifecycle_events.rs"),
    source!("registry/file_custody/promotion.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/promotion.rs"),
    source!("registry/file_custody/registry_lifecycle.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/registry_lifecycle.rs"),
    source!("registry/file_custody/test_faults.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/test_faults.rs"),
    source!("registry/file_custody/operations/unmap.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/operations/unmap.rs"),
    source!("registry/owner.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/owner.rs"),
    source!("registry/owner/lifecycle.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/owner/lifecycle.rs"),
    source!("registry/owner/vfs.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/owner/vfs.rs"),
    source!("registry/process_owner.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/process_owner.rs"),
    source!("registry/process_owner/joint_close_direct_xclose.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/process_owner/joint_close_direct_xclose.rs"),
    source!("registry/process_owner/joint_close_fault.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/process_owner/joint_close_fault.rs"),
    source!("registry/process_owner/lifecycle.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/process_owner/lifecycle.rs"),
    source!("registry/process_owner/vfs.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/process_owner/vfs.rs"),
    source!("registry/state.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/state.rs"),
    source!("registry/state/owner.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/state/owner.rs"),
    source!("registry/state/test_lifecycle.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/state/test_lifecycle.rs"),
    source!("registry/state/test_snapshot.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/state/test_snapshot.rs"),
    source!("registry/types.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/types.rs"),
    source!("node_agent_managed_fs/windows_sqlite.rs", "/src/node_agent_managed_fs/windows_sqlite.rs"),
    source!("node_agent_managed_fs/sqlite_namespace_close.rs", "/src/node_agent_managed_fs/sqlite_namespace_close.rs"),
    source!("node_agent_managed_fs/sqlite_namespace_close/main_close.rs", "/src/node_agent_managed_fs/sqlite_namespace_close/main_close.rs"),
    source!("node_agent_managed_fs/sqlite_namespace_close/main_close_test_native.rs", "/src/node_agent_managed_fs/sqlite_namespace_close/main_close_test_native.rs"),
    source!("node_agent_managed_fs/sqlite_namespace_close/test_faults.rs", "/src/node_agent_managed_fs/sqlite_namespace_close/test_faults.rs"),
    source!("node_agent_managed_fs/sqlite_namespace_main.rs", "/src/node_agent_managed_fs/sqlite_namespace_main.rs"),
    source!("node_agent_managed_fs/sqlite_namespace_types.rs", "/src/node_agent_managed_fs/sqlite_namespace_types.rs"),
    source!("node_agent_managed_fs/sqlite_namespace_validation.rs", "/src/node_agent_managed_fs/sqlite_namespace_validation.rs"),
    source!("node_agent_managed_fs/sqlite_namespace_lock_domain.rs", "/src/node_agent_managed_fs/sqlite_namespace_lock_domain.rs"),
    source!("node_agent_managed_fs/sqlite_namespace_locking.rs", "/src/node_agent_managed_fs/sqlite_namespace_locking.rs"),
    source!("node_agent_managed_fs/sqlite_namespace_locking/test_native.rs", "/src/node_agent_managed_fs/sqlite_namespace_locking/test_native.rs"),
    source!("node_agent_managed_fs/sqlite_namespace_shm/barrier.rs", "/src/node_agent_managed_fs/sqlite_namespace_shm/barrier.rs"),
    source!("node_agent_managed_fs/sqlite_namespace_shm/close.rs", "/src/node_agent_managed_fs/sqlite_namespace_shm/close.rs"),
    source!("node_agent_managed_fs/sqlite_namespace_shm/unmap.rs", "/src/node_agent_managed_fs/sqlite_namespace_shm/unmap.rs"),
    source!("node_agent_managed_fs/sqlite_namespace_shm/teardown.rs", "/src/node_agent_managed_fs/sqlite_namespace_shm/teardown.rs"),
    source!("node_agent_managed_fs/sqlite_namespace_shm/test_unmap_runtime.rs", "/src/node_agent_managed_fs/sqlite_namespace_shm/test_unmap_runtime.rs"),
    source!("node_agent_managed_fs/sqlite_namespace_shm/test_unmap_runtime/authority.rs", "/src/node_agent_managed_fs/sqlite_namespace_shm/test_unmap_runtime/authority.rs"),
    source!("node_agent_managed_fs/sqlite_namespace_shm/test_unmap_runtime/detach.rs", "/src/node_agent_managed_fs/sqlite_namespace_shm/test_unmap_runtime/detach.rs"),
    source!("node_agent_managed_fs/sqlite_namespace_shm/test_unmap_runtime/native.rs", "/src/node_agent_managed_fs/sqlite_namespace_shm/test_unmap_runtime/native.rs"),
    source!("node_agent_managed_fs/sqlite_namespace_shm/test_unmap_runtime/prestate.rs", "/src/node_agent_managed_fs/sqlite_namespace_shm/test_unmap_runtime/prestate.rs"),
    source!("node_agent_managed_fs/sqlite_namespace_shm/failure_custody.rs", "/src/node_agent_managed_fs/sqlite_namespace_shm/failure_custody.rs"),
    source!("node_agent_managed_fs/sqlite_namespace_shm/test_lock_runtime.rs", "/src/node_agent_managed_fs/sqlite_namespace_shm/test_lock_runtime.rs"),
];

pub(super) const MAP_REGION_LOOP_SOURCE_SCOPE_V1: &[(&str, &str)] = &[
    source!("dynamic_quotient/runner_admission/map_program/region_loop.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2b1_cases/denominator/static_contract/dynamic_quotient/runner_admission/map_program/region_loop.rs"),
    source!("dynamic_quotient/runner_admission/map_program/region_loop/catalog.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2b1_cases/denominator/static_contract/dynamic_quotient/runner_admission/map_program/region_loop/catalog.rs"),
    source!("dynamic_quotient/runner_admission/map_program/region_loop/region_loop_members.v1.tsv", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2b1_cases/denominator/static_contract/dynamic_quotient/runner_admission/map_program/region_loop/region_loop_members.v1.tsv"),
    source!("dynamic_quotient/runner_admission/map_program/region_loop/source_scope.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2b1_cases/denominator/static_contract/dynamic_quotient/runner_admission/map_program/region_loop/source_scope.rs"),
    source!("managed_vfs/a2_dynamic_evidence/child/map_region_loop.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/child/map_region_loop.rs"),
    source!("managed_vfs/a2_dynamic_evidence/map_runner/region_loop.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/map_runner/region_loop.rs"),
    source!("managed_vfs/a2_dynamic_evidence/map_runner/region_loop/fixture.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/map_runner/region_loop/fixture.rs"),
    source!("managed_vfs/a2_dynamic_evidence/map_runner/region_loop/payload.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/map_runner/region_loop/payload.rs"),
];
