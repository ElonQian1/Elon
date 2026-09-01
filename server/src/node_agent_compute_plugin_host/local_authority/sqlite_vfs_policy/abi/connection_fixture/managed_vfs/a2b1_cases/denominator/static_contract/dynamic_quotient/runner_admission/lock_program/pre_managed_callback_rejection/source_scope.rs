//! Source closure and implementation seals for the q9 pre-managed rejection tranche.

use sha2::{Digest, Sha256};

use super::super::super::super::super::source_leaf_authority::Digest32;
use super::super::super::super::super::terminal_descriptor::LockActionV1;
use super::super::super::super::lock_local_protocol_rejection_source_scope::lock_local_protocol_rejection_source_scope_entries_v1;
use super::LockPreManagedCallbackRejectionFamilyV1;

macro_rules! source {
    ($name:literal, $path:literal) => {
        (
            $name,
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), $path)),
        )
    };
}

pub(super) const PRE_MANAGED_CALLBACK_REJECTION_PROJECTOR_DELTA_V1: &[(&str, &str)] = &[
    ("dynamic_quotient/runner_admission/lock_program/pre_managed_callback_rejection.rs", include_str!("../pre_managed_callback_rejection.rs")),
    ("dynamic_quotient/runner_admission/lock_program/pre_managed_callback_rejection/catalog.rs", include_str!("catalog.rs")),
    ("dynamic_quotient/runner_admission/lock_program/pre_managed_callback_rejection/runtime.rs", include_str!("runtime.rs")),
    ("dynamic_quotient/runner_admission/lock_program/pre_managed_callback_rejection/source_scope.rs", include_str!("source_scope.rs")),
    ("dynamic_quotient/runner_admission/lock_program/pre_managed_callback_rejection/admission_route_unknown_direct_members.v1.tsv", include_str!("admission_route_unknown_direct_members.v1.tsv")),
    ("dynamic_quotient/runner_admission/lock_program/pre_managed_callback_rejection/admission_counter_overflow_direct_members.v1.tsv", include_str!("admission_counter_overflow_direct_members.v1.tsv")),
    ("dynamic_quotient/runner_admission/lock_program/pre_managed_callback_rejection/unsupported_file_role_completed_members.v1.tsv", include_str!("unsupported_file_role_completed_members.v1.tsv")),
    ("dynamic_quotient/runner_admission/lock_program/pre_managed_callback_rejection/unsupported_file_role_route_unknown_members.v1.tsv", include_str!("unsupported_file_role_route_unknown_members.v1.tsv")),
    ("dynamic_quotient/runner_admission/lock_program/pre_managed_callback_rejection/shm_detached_completed_members.v1.tsv", include_str!("shm_detached_completed_members.v1.tsv")),
    ("dynamic_quotient/runner_admission/lock_program/pre_managed_callback_rejection/shm_detached_route_unknown_members.v1.tsv", include_str!("shm_detached_route_unknown_members.v1.tsv")),
    source!("registry/state/test_lock_callback_admission.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/state/test_lock_callback_admission.rs"),
    source!("registry/owner/test_lock_callback_admission.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/owner/test_lock_callback_admission.rs"),
    source!("registry/process_owner/test_lock_callback_admission.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/process_owner/test_lock_callback_admission.rs"),
    source!("registry/test_vfs_bridge/lock_callback_admission.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/test_vfs_bridge/lock_callback_admission.rs"),
    source!("registry/file_custody/operations/shm.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/operations/shm.rs"),
    source!("registry/file_custody/operations/pre_managed_lock.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/operations/pre_managed_lock.rs"),
    source!("managed_vfs/lifecycle_faults/pre_managed_lock.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/lifecycle_faults/pre_managed_lock.rs"),
    source!("managed_vfs/connection/lock_pre_managed.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/connection/lock_pre_managed.rs"),
    source!("managed_vfs/a2_dynamic_evidence/child/lock_pre_managed_rejection.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/child/lock_pre_managed_rejection.rs"),
    source!("managed_vfs/a2_dynamic_evidence/lock_runner/pre_managed_rejection.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/lock_runner/pre_managed_rejection.rs"),
    source!("managed_vfs/a2_dynamic_evidence/lock_runner/pre_managed_rejection/fixture.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/lock_runner/pre_managed_rejection/fixture.rs"),
    source!("managed_vfs/a2_dynamic_evidence/lock_runner/pre_managed_rejection/payload.rs", "/src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/abi/connection_fixture/managed_vfs/a2_dynamic_evidence/lock_runner/pre_managed_rejection/payload.rs"),
];

pub(super) fn digest_implementation_v1(
    family: LockPreManagedCallbackRejectionFamilyV1,
    action: LockActionV1,
    first: u8,
    count: u8,
    mask: u8,
) -> Digest32 {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-lock-pre-managed-callback-rejection-implementation-v1\0");
    // The shared SHM dispatch split is already present in the inherited q1-q8 closure.
    for (name, source) in lock_local_protocol_rejection_source_scope_entries_v1().chain(
        PRE_MANAGED_CALLBACK_REJECTION_PROJECTOR_DELTA_V1
            .iter()
            .copied()
            .filter(|(name, _)| *name != "registry/file_custody/operations/shm.rs"),
    ) {
        hasher.update((name.len() as u64).to_le_bytes());
        hasher.update(name.as_bytes());
        hasher.update((source.len() as u64).to_le_bytes());
        hasher.update(source.as_bytes());
    }
    hasher.update([
        family_tag_v1(family),
        action_tag_v1(action),
        first,
        count,
        mask,
    ]);
    Digest32(hasher.finalize().into())
}

const fn family_tag_v1(family: LockPreManagedCallbackRejectionFamilyV1) -> u8 {
    use LockPreManagedCallbackRejectionFamilyV1 as F;
    match family {
        F::AdmissionRouteUnknownDirect => 1,
        F::AdmissionCounterOverflowDirect => 2,
        F::UnsupportedFileRoleCompleted => 3,
        F::UnsupportedFileRoleRouteUnknown => 4,
        F::ShmDetachedCompleted => 5,
        F::ShmDetachedRouteUnknown => 6,
    }
}

const fn action_tag_v1(action: LockActionV1) -> u8 {
    match action {
        LockActionV1::LockShared => 1,
        LockActionV1::LockExclusive => 2,
        LockActionV1::UnlockShared => 3,
        LockActionV1::UnlockExclusive => 4,
    }
}
