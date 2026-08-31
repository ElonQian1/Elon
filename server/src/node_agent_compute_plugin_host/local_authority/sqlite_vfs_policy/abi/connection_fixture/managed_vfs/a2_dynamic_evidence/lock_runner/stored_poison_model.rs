//! Shared identity model for the additive q3/q4 stored-poison Lock receipts.

use super::LockRunnerActionV1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) enum LockRunnerStoredPoisonCompletionV1 {
    RetentionSucceeded,
    RetentionRouteUnknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) enum LockRunnerStoredPoisonProfileV1 {
    GateNoMutation,
    FileCloseNoMutation,
    ExactSiblingDeleteNoMutation,
    ExactSiblingOpenUncertain,
    DmsTruncateUncertain,
    FileCloseUncertain,
    ExactSiblingDeleteUncertain,
    FileGrowUncertain,
    MappingCloseUncertain,
    ViewUnmapUncertain,
    LockReleaseUncertain,
    ConnectionDetachUncertain,
    DeleteAuthorizationUncertain,
    DmsExclusiveReleaseUncertain,
    DmsSharedReleaseUncertain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) struct LockRunnerStoredPoisonBindingV1 {
    pub(in super::super::super) action: LockRunnerActionV1,
    pub(in super::super::super) first: u8,
    pub(in super::super::super) count: u8,
    pub(in super::super::super) mask: u8,
    pub(in super::super::super) profile: LockRunnerStoredPoisonProfileV1,
    pub(in super::super::super) completion: LockRunnerStoredPoisonCompletionV1,
    pub(in super::super::super) normalized_descriptor_sha256: [u8; 32],
    pub(in super::super::super) case_key_sha256: [u8; 32],
    pub(in super::super::super) full_record_sha256: [u8; 32],
    pub(in super::super::super) plan_sha256: [u8; 32],
    pub(in super::super::super) implementation_sha256: [u8; 32],
}
