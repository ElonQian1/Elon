//! Windows-only bridge from an exact q8 source program to the isolated Lock runner.

use super::{LockLocalProtocolRejectionPathSpecV1, LockLocalProtocolRejectionProgramSpecV1};
use super::super::super::super::StaticMemberSealV1;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2_dynamic_evidence::{
    run_lock_local_protocol_rejection_program_isolated, LocalProtocolRejectionPathV1,
    LockRunnerIsolatedEvidenceV1, LockRunnerLocalProtocolRejectionBindingV1,
};

pub(super) fn run_isolated_v1(
    exact_test: &str,
    program: LockLocalProtocolRejectionProgramSpecV1,
    member: StaticMemberSealV1,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    run_lock_local_protocol_rejection_program_isolated(
        exact_test,
        LockRunnerLocalProtocolRejectionBindingV1 {
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
    path: LockLocalProtocolRejectionPathSpecV1,
) -> LocalProtocolRejectionPathV1 {
    match path {
        LockLocalProtocolRejectionPathSpecV1::OwnOverlap => {
            LocalProtocolRejectionPathV1::OwnOverlap
        }
        LockLocalProtocolRejectionPathSpecV1::NotHeld => LocalProtocolRejectionPathV1::NotHeld,
    }
}
