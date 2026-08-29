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
mod record;
#[cfg(test)]
mod tests;

pub(super) use child::{
    BoundDynamicChild, ChildLaunchIdentity, DynamicChildFailure, SanitizedActualPayloadCommitment,
    SanitizedChildReport, ValidatedChildProcessReceipt, A2_DYNAMIC_CHILD_NONCE_ENV,
};
pub(super) use cleanup::ValidatedParentCleanupReceipt;
pub(super) use environment::WindowsDynamicEnvironment;
pub(super) use record::{
    UnmapCandidateReportView, ValidatedUnmapCandidateRecord, ValidatedWindowsDynamicRecord,
    WindowsDynamicReportView,
};
