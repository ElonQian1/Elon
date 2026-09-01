//! Source closure and implementation seals for the q10 Lock ABI scalar rejection tranche.

use sha2::{Digest, Sha256};

use super::super::super::super::super::source_leaf_authority::Digest32;
use super::super::super::super::super::terminal_descriptor::{LockAbiScalarV1, ValidityV1};
use super::super::super::super::lock_local_protocol_rejection_source_scope::lock_local_protocol_rejection_source_scope_entries_v1;
use super::super::pre_managed_callback_rejection::PRE_MANAGED_CALLBACK_REJECTION_PROJECTOR_DELTA_V1;

macro_rules! source {
    ($name:literal, $path:literal) => {
        (
            $name,
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), $path)),
        )
    };
}

pub(super) const ABI_SCALAR_REJECTION_PROJECTOR_DELTA_V1: &[(&str, &str)] = &[
    (
        "dynamic_quotient/runner_admission/lock_program/abi_scalar_rejection.rs",
        include_str!("../abi_scalar_rejection.rs"),
    ),
    (
        "dynamic_quotient/runner_admission/lock_program/abi_scalar_rejection/catalog.rs",
        include_str!("catalog.rs"),
    ),
    (
        "dynamic_quotient/runner_admission/lock_program/abi_scalar_rejection/runtime.rs",
        include_str!("runtime.rs"),
    ),
    (
        "dynamic_quotient/runner_admission/lock_program/abi_scalar_rejection/source_scope.rs",
        include_str!("source_scope.rs"),
    ),
    (
        "dynamic_quotient/runner_admission/lock_program/abi_scalar_rejection/abi_scalar_rejection_members.v1.tsv",
        include_str!("abi_scalar_rejection_members.v1.tsv"),
    ),
    source!(
        "managed_vfs/a2_dynamic_evidence/child/lock_abi_scalar_rejection.rs",
        "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/child/lock_abi_scalar_rejection.rs"
    ),
    source!(
        "managed_vfs/a2_dynamic_evidence/lock_runner/abi_scalar_rejection.rs",
        "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/lock_runner/abi_scalar_rejection.rs"
    ),
    source!(
        "managed_vfs/a2_dynamic_evidence/lock_runner/abi_scalar_rejection/payload.rs",
        "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/lock_runner/abi_scalar_rejection/payload.rs"
    ),
    source!(
        "sqlite_vfs_abi/lock_observation.rs",
        "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/lock_observation.rs"
    ),
];

pub(super) fn digest_implementation_v1(scalar: LockAbiScalarV1) -> Digest32 {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-lock-abi-scalar-rejection-implementation-v1\0");
    // q9 named the shared SHM dispatch split again; it is already present in q1-q8.
    for (name, source) in lock_local_protocol_rejection_source_scope_entries_v1()
        .chain(
            PRE_MANAGED_CALLBACK_REJECTION_PROJECTOR_DELTA_V1
                .iter()
                .copied()
                .filter(|(name, _)| *name != "registry/file_custody/operations/shm.rs"),
        )
        .chain(ABI_SCALAR_REJECTION_PROJECTOR_DELTA_V1.iter().copied())
    {
        hasher.update((name.len() as u64).to_le_bytes());
        hasher.update(name.as_bytes());
        hasher.update((source.len() as u64).to_le_bytes());
        hasher.update(source.as_bytes());
    }
    hasher.update([
        validity_tag_v1(scalar.offset),
        validity_tag_v1(scalar.count),
        validity_tag_v1(scalar.flags),
    ]);
    Digest32(hasher.finalize().into())
}

const fn validity_tag_v1(validity: ValidityV1) -> u8 {
    match validity {
        ValidityV1::Invalid => 1,
        ValidityV1::Valid => 2,
    }
}
