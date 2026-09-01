//! Windows bridge from an exact q10 source program to the isolated installed-xShmLock runner.

use super::super::super::super::StaticMemberSealV1;
use super::super::super::super::super::terminal_descriptor::ValidityV1;
use super::LockAbiScalarRejectionProgramSpecV1;
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2_dynamic_evidence::{
    run_lock_abi_scalar_rejection_program_isolated, LockRunnerAbiScalarRejectionBindingV1,
    LockRunnerAbiScalarValidityV1, LockRunnerIsolatedEvidenceV1,
};

pub(super) fn run_isolated_v1(
    exact_test: &str,
    program: LockAbiScalarRejectionProgramSpecV1,
    member: StaticMemberSealV1,
) -> anyhow::Result<LockRunnerIsolatedEvidenceV1> {
    run_lock_abi_scalar_rejection_program_isolated(
        exact_test,
        LockRunnerAbiScalarRejectionBindingV1 {
            offset: runtime_validity_v1(program.scalar.offset),
            count: runtime_validity_v1(program.scalar.count),
            flags: runtime_validity_v1(program.scalar.flags),
            normalized_descriptor_sha256: program.normalized_descriptor_sha256.0,
            case_key_sha256: member.case_key_sha256.0,
            full_record_sha256: member.full_record_sha256.0,
            plan_sha256: program.plan_sha256.0,
            implementation_sha256: program.implementation_sha256.0,
        },
    )
}

const fn runtime_validity_v1(value: ValidityV1) -> LockRunnerAbiScalarValidityV1 {
    match value {
        ValidityV1::Invalid => LockRunnerAbiScalarValidityV1::Invalid,
        ValidityV1::Valid => LockRunnerAbiScalarValidityV1::Valid,
    }
}
