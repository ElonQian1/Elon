//! Source closure and implementation seals for the q13 initialization-release tranche.

use sha2::{Digest, Sha256};

use super::super::abi_scalar_rejection::ABI_SCALAR_REJECTION_PROJECTOR_DELTA_V1;
use super::super::native_acquire_created_first_exclusive_release_error::NATIVE_ACQUIRE_CREATED_FIRST_EXCLUSIVE_RELEASE_ERROR_PROJECTOR_DELTA_V1;
use super::super::pre_managed_callback_rejection::PRE_MANAGED_CALLBACK_REJECTION_PROJECTOR_DELTA_V1;
use super::super::raw_state_rejection::RAW_STATE_REJECTION_PROJECTOR_DELTA_V1;
use super::super::super::super::lock_local_protocol_rejection_source_scope::lock_local_protocol_rejection_source_scope_entries_v1;
use super::super::super::super::super::source_leaf_authority::Digest32;
use super::super::super::super::super::terminal_descriptor::LockActionV1;

macro_rules! source {
    ($name:literal, $path:literal) => {
        (
            $name,
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), $path)),
        )
    };
}

/// New source identities introduced by q13. Modified shared roots remain in the inherited closure.
pub(super) const NATIVE_ACQUIRE_EXISTING_FIRST_EXCLUSIVE_RELEASE_ERROR_PROJECTOR_DELTA_V1: &[(&str, &str)] = &[
    (
        "dynamic_quotient/runner_admission/lock_program/native_acquire_existing_first_exclusive_release_error.rs",
        include_str!("../native_acquire_existing_first_exclusive_release_error.rs"),
    ),
    (
        "dynamic_quotient/runner_admission/lock_program/native_acquire_existing_first_exclusive_release_error/catalog.rs",
        include_str!("catalog.rs"),
    ),
    (
        "dynamic_quotient/runner_admission/lock_program/native_acquire_existing_first_exclusive_release_error/runtime.rs",
        include_str!("runtime.rs"),
    ),
    (
        "dynamic_quotient/runner_admission/lock_program/native_acquire_existing_first_exclusive_release_error/source_scope.rs",
        include_str!("source_scope.rs"),
    ),
    (
        "dynamic_quotient/runner_admission/lock_program/native_acquire_existing_first_exclusive_release_error/native_acquire_existing_first_exclusive_release_error_members.v1.tsv",
        include_str!("native_acquire_existing_first_exclusive_release_error_members.v1.tsv"),
    ),
    source!(
        "managed_vfs/a2_dynamic_evidence/child/lock_existing_first_exclusive_release_error.rs",
        "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/child/lock_existing_first_exclusive_release_error.rs"
    ),
    source!(
        "managed_vfs/a2_dynamic_evidence/lock_runner/existing_first_exclusive_release_error.rs",
        "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/lock_runner/existing_first_exclusive_release_error.rs"
    ),
    source!(
        "managed_vfs/a2_dynamic_evidence/lock_runner/existing_first_exclusive_release_error/fixture.rs",
        "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/lock_runner/existing_first_exclusive_release_error/fixture.rs"
    ),
    source!(
        "managed_vfs/a2_dynamic_evidence/lock_runner/existing_first_exclusive_release_error/payload.rs",
        "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/lock_runner/existing_first_exclusive_release_error/payload.rs"
    ),
    source!(
        "node_agent_managed_fs/sqlite_namespace_shm/test_support.rs",
        "/src/node_agent_managed_fs/sqlite_namespace_shm/test_support.rs"
    ),
];

pub(super) fn digest_implementation_v1(
    action: LockActionV1,
    first: u8,
    count: u8,
    mask: u8,
    completion_tag: u8,
) -> Digest32 {
    let mut hasher = Sha256::new();
    hasher.update(
        b"elon-lock-native-acquire-existing-first-exclusive-release-error-implementation-v1\0",
    );
    // Q13 extends the exact q1-q12 closure once, then adds only its new source identities.
    for (name, source) in lock_local_protocol_rejection_source_scope_entries_v1()
        .chain(
            PRE_MANAGED_CALLBACK_REJECTION_PROJECTOR_DELTA_V1
                .iter()
                .copied()
                .filter(|(name, _)| *name != "registry/file_custody/operations/shm.rs"),
        )
        .chain(ABI_SCALAR_REJECTION_PROJECTOR_DELTA_V1.iter().copied())
        .chain(RAW_STATE_REJECTION_PROJECTOR_DELTA_V1.iter().copied())
        .chain(
            NATIVE_ACQUIRE_CREATED_FIRST_EXCLUSIVE_RELEASE_ERROR_PROJECTOR_DELTA_V1
                .iter()
                .copied(),
        )
        .chain(
            NATIVE_ACQUIRE_EXISTING_FIRST_EXCLUSIVE_RELEASE_ERROR_PROJECTOR_DELTA_V1
                .iter()
                .copied(),
        )
    {
        hasher.update((name.len() as u64).to_le_bytes());
        hasher.update(name.as_bytes());
        hasher.update((source.len() as u64).to_le_bytes());
        hasher.update(source.as_bytes());
    }
    hasher.update([
        action_tag_v1(action),
        first,
        count,
        mask,
        completion_tag,
    ]);
    Digest32(hasher.finalize().into())
}

const fn action_tag_v1(action: LockActionV1) -> u8 {
    match action {
        LockActionV1::LockShared => 1,
        LockActionV1::LockExclusive => 2,
        LockActionV1::UnlockShared => 3,
        LockActionV1::UnlockExclusive => 4,
    }
}
