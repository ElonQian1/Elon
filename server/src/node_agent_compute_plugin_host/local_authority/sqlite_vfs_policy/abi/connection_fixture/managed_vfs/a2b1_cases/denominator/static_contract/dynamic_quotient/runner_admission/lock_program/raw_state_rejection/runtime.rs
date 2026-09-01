//! Windows bridge from an exact q11 source program to the isolated installed-xShmLock runner.

use super::super::super::super::StaticMemberSealV1;
use super::case::LockRawStateRejectionCaseV1;
use super::LockRawStateRejectionProgramSpecV1;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2_dynamic_evidence::{
    run_lock_raw_state_rejection_program_isolated, LockRunnerIsolatedEvidenceV1,
    LockRunnerRawStateRejectionBindingV1, LockRunnerRawStateRejectionV1,
};

pub(super) fn run_isolated_v1(
    exact_test: &str,
    program: LockRawStateRejectionProgramSpecV1,
    member: StaticMemberSealV1,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    run_lock_raw_state_rejection_program_isolated(
        exact_test,
        LockRunnerRawStateRejectionBindingV1 {
            rejection: runtime_rejection_v1(program.rejection),
            normalized_descriptor_sha256: program.normalized_descriptor_sha256.0,
            case_key_sha256: member.case_key_sha256.0,
            full_record_sha256: member.full_record_sha256.0,
            plan_sha256: program.plan_sha256.0,
            implementation_sha256: program.implementation_sha256.0,
        },
    )
}

const fn runtime_rejection_v1(value: LockRawStateRejectionCaseV1) -> LockRunnerRawStateRejectionV1 {
    match value {
        LockRawStateRejectionCaseV1::NullFileDirect => {
            LockRunnerRawStateRejectionV1::NullFileDirect
        }
        LockRawStateRejectionCaseV1::UninstalledDirect => {
            LockRunnerRawStateRejectionV1::UninstalledDirect
        }
        LockRawStateRejectionCaseV1::MethodsNullStatePresentDirect => {
            LockRunnerRawStateRejectionV1::MethodsNullStatePresentDirect
        }
        LockRawStateRejectionCaseV1::ForeignMethodsStateNullDirect => {
            LockRunnerRawStateRejectionV1::ForeignMethodsStateNullDirect
        }
        LockRawStateRejectionCaseV1::ForeignMethodsStatePresentDirect => {
            LockRunnerRawStateRejectionV1::ForeignMethodsStatePresentDirect
        }
        LockRawStateRejectionCaseV1::ExactMethodsStateNullDirect => {
            LockRunnerRawStateRejectionV1::ExactMethodsStateNullDirect
        }
        LockRawStateRejectionCaseV1::OtherTypePayloadMissingDropCompleted => {
            LockRunnerRawStateRejectionV1::OtherTypePayloadMissingDropCompleted
        }
        LockRawStateRejectionCaseV1::OtherTypePayloadPresentDropCompleted => {
            LockRunnerRawStateRejectionV1::OtherTypePayloadPresentDropCompleted
        }
        LockRawStateRejectionCaseV1::OtherTypePayloadPresentDropUnwindCaught => {
            LockRunnerRawStateRejectionV1::OtherTypePayloadPresentDropUnwindCaught
        }
        LockRawStateRejectionCaseV1::ExpectedTypePayloadMissingDropCompleted => {
            LockRunnerRawStateRejectionV1::ExpectedTypePayloadMissingDropCompleted
        }
        LockRawStateRejectionCaseV1::HandleBoundFileMissingDirect => {
            LockRunnerRawStateRejectionV1::HandleBoundFileMissingDirect
        }
    }
}
