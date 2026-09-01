//! Windows bridge from an exact q12 source program to the isolated controlled-fault runner.

use super::super::super::super::super::terminal_descriptor::LockActionV1;
use super::super::super::super::StaticMemberSealV1;
use super::{
    LockCreatedFirstExclusiveReleaseCompletionV1,
    LockNativeAcquireCreatedFirstExclusiveReleaseErrorProgramSpecV1,
};
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2_dynamic_evidence::{
    run_lock_native_acquire_created_first_exclusive_release_error_program_isolated,
    LockRunnerActionV1, LockRunnerCreatedFirstExclusiveReleaseCompletionV1,
    LockRunnerIsolatedEvidenceV1,
    LockRunnerNativeAcquireCreatedFirstExclusiveReleaseErrorBindingV1,
};

pub(super) fn run_isolated_v1(
    exact_test: &str,
    program: LockNativeAcquireCreatedFirstExclusiveReleaseErrorProgramSpecV1,
    member: StaticMemberSealV1,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    run_lock_native_acquire_created_first_exclusive_release_error_program_isolated(
        exact_test,
        LockRunnerNativeAcquireCreatedFirstExclusiveReleaseErrorBindingV1 {
            action: runner_action_v1(program.action),
            first: program.first,
            count: program.count,
            mask: program.mask,
            completion: runner_completion_v1(program.completion),
            normalized_descriptor_sha256: program.normalized_descriptor_sha256.0,
            case_key_sha256: member.case_key_sha256.0,
            full_record_sha256: member.full_record_sha256.0,
            plan_sha256: program.plan_sha256.0,
            implementation_sha256: program.implementation_sha256.0,
        },
    )
}

const fn runner_action_v1(action: LockActionV1) -> LockRunnerActionV1 {
    match action {
        LockActionV1::LockShared => LockRunnerActionV1::LockShared,
        LockActionV1::LockExclusive => LockRunnerActionV1::LockExclusive,
        LockActionV1::UnlockShared => LockRunnerActionV1::UnlockShared,
        LockActionV1::UnlockExclusive => LockRunnerActionV1::UnlockExclusive,
    }
}

const fn runner_completion_v1(
    completion: LockCreatedFirstExclusiveReleaseCompletionV1,
) -> LockRunnerCreatedFirstExclusiveReleaseCompletionV1 {
    match completion {
        LockCreatedFirstExclusiveReleaseCompletionV1::RetentionSucceeded => {
            LockRunnerCreatedFirstExclusiveReleaseCompletionV1::RetentionSucceeded
        }
        LockCreatedFirstExclusiveReleaseCompletionV1::RetentionRouteUnknown => {
            LockRunnerCreatedFirstExclusiveReleaseCompletionV1::RetentionRouteUnknown
        }
    }
}
