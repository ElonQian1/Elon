use super::super::super::ManagedSqliteObservedLock;

/// Read-only observations made by the real Delete authorization gate for one exact target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ManagedSqliteShmTestUnmapDeleteAuthorityReceipt {
    pub(crate) stored_identity_present: bool,
    pub(crate) request_identity_present: bool,
    pub(crate) identity_matches: bool,
    pub(crate) generation_matches: bool,
    pub(crate) lock_level: Option<ManagedSqliteObservedLock>,
    pub(crate) lock_query_unavailable: bool,
    pub(crate) stored_identity_unchanged: bool,
    pub(crate) selected_request_validation_attempted: bool,
    pub(crate) selected_request_validation_succeeded: bool,
    pub(crate) correct_request_recheck_attempted: bool,
    pub(crate) correct_request_recheck_succeeded: bool,
}
