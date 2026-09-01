//! Source closure and implementation seals for the q11 Lock raw-state rejection tranche.

use sha2::{Digest, Sha256};

use super::super::super::super::super::source_leaf_authority::Digest32;
use super::super::super::super::lock_local_protocol_rejection_source_scope::lock_local_protocol_rejection_source_scope_entries_v1;
use super::super::abi_scalar_rejection::ABI_SCALAR_REJECTION_PROJECTOR_DELTA_V1;
use super::super::pre_managed_callback_rejection::PRE_MANAGED_CALLBACK_REJECTION_PROJECTOR_DELTA_V1;
use super::case::LockRawStateRejectionCaseV1;

macro_rules! source {
    ($name:literal, $path:literal) => {
        (
            $name,
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), $path)),
        )
    };
}

pub(super) const RAW_STATE_REJECTION_PROJECTOR_DELTA_V1: &[(&str, &str)] = &[
    (
        "dynamic_quotient/runner_admission/lock_program/raw_state_rejection.rs",
        include_str!("../raw_state_rejection.rs"),
    ),
    (
        "dynamic_quotient/runner_admission/lock_program/raw_state_rejection/case.rs",
        include_str!("case.rs"),
    ),
    (
        "dynamic_quotient/runner_admission/lock_program/raw_state_rejection/catalog.rs",
        include_str!("catalog.rs"),
    ),
    (
        "dynamic_quotient/runner_admission/lock_program/raw_state_rejection/expected.rs",
        include_str!("expected.rs"),
    ),
    (
        "dynamic_quotient/runner_admission/lock_program/raw_state_rejection/runtime.rs",
        include_str!("runtime.rs"),
    ),
    (
        "dynamic_quotient/runner_admission/lock_program/raw_state_rejection/source_scope.rs",
        include_str!("source_scope.rs"),
    ),
    (
        "dynamic_quotient/runner_admission/lock_program/raw_state_rejection/raw_state_rejection_members.v1.tsv",
        include_str!("raw_state_rejection_members.v1.tsv"),
    ),
    source!(
        "managed_vfs/a2_dynamic_evidence/child/lock_raw_state_rejection.rs",
        "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/child/lock_raw_state_rejection.rs"
    ),
    source!(
        "managed_vfs/a2_dynamic_evidence/lock_runner/raw_state_rejection.rs",
        "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/lock_runner/raw_state_rejection.rs"
    ),
    source!(
        "managed_vfs/a2_dynamic_evidence/lock_runner/raw_state_rejection/fixture.rs",
        "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/lock_runner/raw_state_rejection/fixture.rs"
    ),
    source!(
        "managed_vfs/a2_dynamic_evidence/lock_runner/raw_state_rejection/payload.rs",
        "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/lock_runner/raw_state_rejection/payload.rs"
    ),
    source!(
        "managed_vfs/connection/lock_raw.rs",
        "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/connection/lock_raw.rs"
    ),
    source!(
        "managed_vfs/lifecycle_faults/pre_managed_lock/raw_rejected.rs",
        "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/lifecycle_faults/pre_managed_lock/raw_rejected.rs"
    ),
    source!(
        "sqlite_vfs_abi/raw_lock_observation.rs",
        "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/raw_lock_observation.rs"
    ),
    source!(
        "sqlite_vfs_abi/raw_lock_observation/events.rs",
        "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/raw_lock_observation/events.rs"
    ),
    source!(
        "sqlite_vfs_abi/raw_lock_observation/expected.rs",
        "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/raw_lock_observation/expected.rs"
    ),
    source!(
        "sqlite_vfs_abi/raw_lock_observation/model.rs",
        "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/raw_lock_observation/model.rs"
    ),
    source!(
        "sqlite_vfs_abi/raw_state/lock_raw_control.rs",
        "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/raw_state/lock_raw_control.rs"
    ),
];

pub(super) fn digest_implementation_v1(case: LockRawStateRejectionCaseV1) -> Digest32 {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-lock-raw-state-rejection-implementation-v1\0");
    // q9 named the shared SHM dispatch split again; it is already present in q1-q8.
    for (name, source) in lock_local_protocol_rejection_source_scope_entries_v1()
        .chain(
            PRE_MANAGED_CALLBACK_REJECTION_PROJECTOR_DELTA_V1
                .iter()
                .copied()
                .filter(|(name, _)| *name != "registry/file_custody/operations/shm.rs"),
        )
        .chain(ABI_SCALAR_REJECTION_PROJECTOR_DELTA_V1.iter().copied())
        .chain(RAW_STATE_REJECTION_PROJECTOR_DELTA_V1.iter().copied())
    {
        hasher.update((name.len() as u64).to_le_bytes());
        hasher.update(name.as_bytes());
        hasher.update((source.len() as u64).to_le_bytes());
        hasher.update(source.as_bytes());
    }
    hasher.update([case.implementation_tag_v1()]);
    Digest32(hasher.finalize().into())
}
