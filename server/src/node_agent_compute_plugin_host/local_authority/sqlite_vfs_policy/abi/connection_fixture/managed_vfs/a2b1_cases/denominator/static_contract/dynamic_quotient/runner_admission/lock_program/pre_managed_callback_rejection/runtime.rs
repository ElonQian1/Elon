//! Windows bridge from an exact q9 source program to the isolated installed-xShmLock runner.

use super::super::super::super::StaticMemberSealV1;
use super::{
    LockPreManagedCallbackRejectionFamilyV1, LockPreManagedCallbackRejectionProgramSpecV1,
};
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2_dynamic_evidence::{
    run_lock_pre_managed_rejection_program_isolated, LockRunnerIsolatedEvidenceV1,
    LockRunnerPreManagedCompletionV1, LockRunnerPreManagedRejectionBindingV1,
    LockRunnerPreManagedRejectionV1,
};

pub(super) fn run_isolated_v1(
    exact_test: &str,
    program: LockPreManagedCallbackRejectionProgramSpecV1,
    member: StaticMemberSealV1,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    let (rejection, completion) = runtime_family_v1(program.family);
    run_lock_pre_managed_rejection_program_isolated(
        exact_test,
        LockRunnerPreManagedRejectionBindingV1 {
            rejection,
            completion,
            action: super::super::runner_action_v1(program.action),
            first: program.first,
            count: program.count,
            mask: program.mask,
            normalized_descriptor_sha256: program.normalized_descriptor_sha256.0,
            case_key_sha256: member.case_key_sha256.0,
            full_record_sha256: member.full_record_sha256.0,
            plan_sha256: program.plan_sha256.0,
            implementation_sha256: program.implementation_sha256.0,
        },
    )
}

const fn runtime_family_v1(
    family: LockPreManagedCallbackRejectionFamilyV1,
) -> (
    LockRunnerPreManagedRejectionV1,
    LockRunnerPreManagedCompletionV1,
) {
    use LockPreManagedCallbackRejectionFamilyV1 as F;
    match family {
        F::AdmissionRouteUnknownDirect => (
            LockRunnerPreManagedRejectionV1::AdmissionRouteUnknown,
            LockRunnerPreManagedCompletionV1::Direct,
        ),
        F::AdmissionCounterOverflowDirect => (
            LockRunnerPreManagedRejectionV1::AdmissionCounterOverflow,
            LockRunnerPreManagedCompletionV1::Direct,
        ),
        F::UnsupportedFileRoleCompleted => (
            LockRunnerPreManagedRejectionV1::UnsupportedFileRole,
            LockRunnerPreManagedCompletionV1::Completed,
        ),
        F::UnsupportedFileRoleRouteUnknown => (
            LockRunnerPreManagedRejectionV1::UnsupportedFileRole,
            LockRunnerPreManagedCompletionV1::RouteUnknown,
        ),
        F::ShmDetachedCompleted => (
            LockRunnerPreManagedRejectionV1::ShmDetached,
            LockRunnerPreManagedCompletionV1::Completed,
        ),
        F::ShmDetachedRouteUnknown => (
            LockRunnerPreManagedRejectionV1::ShmDetached,
            LockRunnerPreManagedCompletionV1::RouteUnknown,
        ),
    }
}
