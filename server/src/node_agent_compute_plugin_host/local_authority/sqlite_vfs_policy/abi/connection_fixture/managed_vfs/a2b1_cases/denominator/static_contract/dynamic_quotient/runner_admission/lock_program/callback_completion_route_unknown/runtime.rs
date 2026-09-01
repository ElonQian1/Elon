//! Windows-only bridge from an exact q7 source program to the isolated Lock runner.

use super::{
    LockCallbackCompletionRouteUnknownPathV1, LockCallbackCompletionRouteUnknownProgramSpecV1,
};
use super::super::super::super::StaticMemberSealV1;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2_dynamic_evidence::{
    run_lock_callback_route_unknown_program_isolated,
    LockRunnerCallbackRouteUnknownBindingV1, LockRunnerCallbackRouteUnknownPathV1,
    LockRunnerIsolatedEvidenceV1,
};

pub(super) fn run_isolated_v1(
    exact_test: &str,
    program: LockCallbackCompletionRouteUnknownProgramSpecV1,
    member: StaticMemberSealV1,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    run_lock_callback_route_unknown_program_isolated(
        exact_test,
        LockRunnerCallbackRouteUnknownBindingV1 {
            path: runtime_path_v1(program.path),
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

const fn runtime_path_v1(
    path: LockCallbackCompletionRouteUnknownPathV1,
) -> LockRunnerCallbackRouteUnknownPathV1 {
    match path {
        LockCallbackCompletionRouteUnknownPathV1::LocalSiblingContention => {
            LockRunnerCallbackRouteUnknownPathV1::LocalSiblingContention
        }
        LockCallbackCompletionRouteUnknownPathV1::NativeRelease => {
            LockRunnerCallbackRouteUnknownPathV1::NativeRelease
        }
        LockCallbackCompletionRouteUnknownPathV1::NativeAcquireAcquired => {
            LockRunnerCallbackRouteUnknownPathV1::NativeAcquireAcquired
        }
        LockCallbackCompletionRouteUnknownPathV1::NativeAcquireBusy => {
            LockRunnerCallbackRouteUnknownPathV1::NativeAcquireBusy
        }
        LockCallbackCompletionRouteUnknownPathV1::SharedLocalAcquire => {
            LockRunnerCallbackRouteUnknownPathV1::SharedLocalAcquire
        }
        LockCallbackCompletionRouteUnknownPathV1::SharedLocalRelease => {
            LockRunnerCallbackRouteUnknownPathV1::SharedLocalRelease
        }
    }
}
