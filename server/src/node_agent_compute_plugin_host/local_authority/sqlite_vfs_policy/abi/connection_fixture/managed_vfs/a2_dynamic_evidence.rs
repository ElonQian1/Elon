//! Linear envelope for future A2 Windows dynamic evidence.
//!
//! The types in this module are only reachable from the Windows test target. A record can be
//! created only after a parent process has revalidated a sanitized child observation, consumed a
//! real child exit status, and removed the same child-bound root. Merely compiling this
//! module or finding an A2 source case never creates Windows dynamic evidence.

mod capture;
mod child;
mod cleanup;
mod environment;
mod joint_close_family;
mod map_runner;
mod record;
#[cfg(test)]
mod tests;
mod unmap_family;

pub(super) use child::{
    BoundDynamicChild, ChildLaunchIdentity, DynamicChildFailure, SanitizedActualPayloadCommitment,
    SanitizedChildReport, ValidatedChildProcessReceipt, A2_DYNAMIC_CHILD_NONCE_ENV,
};
pub(super) use cleanup::ValidatedParentCleanupReceipt;
pub(super) use environment::WindowsDynamicEnvironment;
pub(super) use joint_close_family::{
    JointCloseCandidateReportView, JointCloseFamilyCohort, RenderedJointCloseFamilyReport,
    ValidatedJointCloseCandidateRecord, ValidatedJointCloseCleanCheckoutReceipt,
    ValidatedJointCloseFamily, ValidatedJointCloseFamilyMemberReceipt,
};
pub(super) use map_runner::{
    run_map_program_isolated, MapRunnerEvidenceReceiptV1, MapRunnerIsolatedEvidenceV1,
    MapRunnerModeV1, MapRunnerProgramBindingV1, MapRunnerRequestBudgetV1,
};
pub(super) use record::{
    UnmapCandidateReportView, ValidatedUnmapCandidateRecord, ValidatedWindowsDynamicRecord,
    WindowsDynamicReportView,
};
pub(super) use unmap_family::{
    RenderedUnmapFamilyReport, UnmapFamilyCohort, ValidatedUnmapCleanCheckoutReceipt,
    ValidatedUnmapFamily, ValidatedUnmapFamilyMemberReceipt,
};
