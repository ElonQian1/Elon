//! Transitive source commitment for exact local protocol rejection programs.

use super::lock_callback_completion_route_unknown_source_scope::LOCK_CALLBACK_COMPLETION_ROUTE_UNKNOWN_SOURCE_SCOPE_V1;

macro_rules! source {
    ($name:literal, $path:literal) => {
        (
            $name,
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), $path)),
        )
    };
}

/// Existing global identities whose current exhaustive-match content participates in q8.
const LOCAL_PROTOCOL_REJECTION_INHERITED_SOURCE_ADDITIONS_V1: &[(&str, &str)] = &[
    source!("managed_vfs/a2_dynamic_evidence/lock_runner/stored_poison/payload.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/lock_runner/stored_poison/payload.rs"),
];

/// New source identities introduced by q8 and absent from the shared projector scope.
pub(super) const LOCAL_PROTOCOL_REJECTION_PROJECTOR_DELTA_V1: &[(&str, &str)] = &[
    source!("dynamic_quotient/lock_local_protocol_rejection_source_scope.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2b1_cases/denominator/static_contract/dynamic_quotient/lock_local_protocol_rejection_source_scope.rs"),
    source!("dynamic_quotient/runner_admission/lock_program/local_protocol_rejection.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2b1_cases/denominator/static_contract/dynamic_quotient/runner_admission/lock_program/local_protocol_rejection.rs"),
    source!("dynamic_quotient/runner_admission/lock_program/local_protocol_rejection/catalog.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2b1_cases/denominator/static_contract/dynamic_quotient/runner_admission/lock_program/local_protocol_rejection/catalog.rs"),
    source!("dynamic_quotient/runner_admission/lock_program/local_protocol_rejection/runtime.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2b1_cases/denominator/static_contract/dynamic_quotient/runner_admission/lock_program/local_protocol_rejection/runtime.rs"),
    source!("dynamic_quotient/runner_admission/lock_program/local_protocol_rejection/source_scope.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2b1_cases/denominator/static_contract/dynamic_quotient/runner_admission/lock_program/local_protocol_rejection/source_scope.rs"),
    source!("dynamic_quotient/runner_admission/lock_program/local_protocol_rejection/local_protocol_own_overlap_or_not_held_completed_members.v1.tsv", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2b1_cases/denominator/static_contract/dynamic_quotient/runner_admission/lock_program/local_protocol_rejection/local_protocol_own_overlap_or_not_held_completed_members.v1.tsv"),
    source!("managed_vfs/a2_dynamic_evidence/child/lock_local_protocol_rejection.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/child/lock_local_protocol_rejection.rs"),
    source!("managed_vfs/a2_dynamic_evidence/child/test_support.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/child/test_support.rs"),
    source!("managed_vfs/a2_dynamic_evidence/lock_runner/local_protocol_rejection.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/lock_runner/local_protocol_rejection.rs"),
    source!("managed_vfs/a2_dynamic_evidence/lock_runner/local_protocol_rejection/fixture.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/lock_runner/local_protocol_rejection/fixture.rs"),
    source!("managed_vfs/a2_dynamic_evidence/lock_runner/local_protocol_rejection/payload.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/lock_runner/local_protocol_rejection/payload.rs"),
    source!("managed_vfs/a2_dynamic_evidence/lock_runner/selector_test_support.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/lock_runner/selector_test_support.rs"),
];

/// Full implementation closure used to seal each exact q8 program.
pub(super) fn lock_local_protocol_rejection_source_scope_entries_v1(
) -> impl Iterator<Item = (&'static str, &'static str)> {
    LOCK_CALLBACK_COMPLETION_ROUTE_UNKNOWN_SOURCE_SCOPE_V1
        .iter()
        .copied()
        .chain(
            LOCAL_PROTOCOL_REJECTION_INHERITED_SOURCE_ADDITIONS_V1
                .iter()
                .copied(),
        )
        .chain(LOCAL_PROTOCOL_REJECTION_PROJECTOR_DELTA_V1.iter().copied())
}
