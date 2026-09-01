//! Canonical sealing helpers for real Lock execution receipts.

use sha2::{Digest, Sha256};

use super::{Digest32, LockRunnerExecutionReceiptV1};
#[cfg(windows)]
use super::LockProgramV1;
#[cfg(windows)]
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2_dynamic_evidence::LockRunnerEvidenceReceiptV1;

#[cfg(windows)]
pub(super) fn seal_execution_receipt(
    program: LockProgramV1,
    evidence: LockRunnerEvidenceReceiptV1,
) -> LockRunnerExecutionReceiptV1 {
    let (root, child, registration, payload, environment, cleanup, native, exit_code) =
        evidence.into_bindings();
    let mut receipt = LockRunnerExecutionReceiptV1 {
        normalized_descriptor_sha256: program.normalized_descriptor_sha256,
        member: program.member,
        plan_sha256: program.plan_sha256,
        implementation_sha256: program.implementation_sha256,
        root_commitment_sha256: Digest32(root),
        child_fingerprint_sha256: Digest32(child),
        registration_commitment_sha256: Digest32(registration),
        payload_commitment_sha256: Digest32(payload),
        environment_sha256: Digest32(environment),
        cleanup_sha256: Digest32(cleanup),
        native_receipt_sha256: Digest32(native),
        child_exit_code: exit_code,
        execution_sha256: Digest32::ZERO,
    };
    receipt.execution_sha256 = digest_execution_receipt_v1(&receipt);
    receipt
}

pub(super) fn digest_execution_receipt_v1(receipt: &LockRunnerExecutionReceiptV1) -> Digest32 {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-lock-quotient-real-execution-v1\0");
    for digest in [
        receipt.normalized_descriptor_sha256,
        receipt.member.case_key_sha256,
        receipt.member.full_record_sha256,
        receipt.plan_sha256,
        receipt.implementation_sha256,
        receipt.root_commitment_sha256,
        receipt.child_fingerprint_sha256,
        receipt.registration_commitment_sha256,
        receipt.payload_commitment_sha256,
        receipt.environment_sha256,
        receipt.cleanup_sha256,
        receipt.native_receipt_sha256,
    ] {
        hasher.update(digest.0);
    }
    hasher.update(receipt.child_exit_code.to_le_bytes());
    Digest32(hasher.finalize().into())
}
