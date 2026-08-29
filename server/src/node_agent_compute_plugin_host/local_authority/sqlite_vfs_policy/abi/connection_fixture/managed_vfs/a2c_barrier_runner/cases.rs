//! Fixed libtest selectors; no Barrier case identity is accepted from the environment.

pub(super) const ADMISSION_REJECTED: &str = "node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2c_barrier_runner::barrier_admission_rejected";
pub(super) const WRAPPER_BEFORE: &str = "node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2c_barrier_runner::barrier_wrapper_before";
pub(super) const FENCE_BEFORE: &str = "node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2c_barrier_runner::barrier_fence_before";
pub(super) const FENCE_AFTER: &str = "node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2c_barrier_runner::barrier_fence_after";
pub(super) const COMPLETION_BEFORE: &str = "node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2c_barrier_runner::barrier_completion_before";
pub(super) const COMPLETION_NATIVE_UNCERTAIN: &str = "node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2c_barrier_runner::barrier_completion_native_uncertain";
pub(super) const COMPLETION_AFTER_SUCCESS_KNOWN: &str = "node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2c_barrier_runner::barrier_completion_after_success_known";
pub(super) const SUCCESS: &str = "node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2c_barrier_runner::barrier_success";
