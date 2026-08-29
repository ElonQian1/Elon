//! Test-only ordered Unmap runtime event vocabulary.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super::super) enum ManagedSqliteRegistryUnmapRuntimeEvent {
    CallbackBeginAttempt,
    CallbackBeginSuccess,
    SelectedActionAttempt,
    SelectedActionSuccess,
    CallbackCompletionAttempt,
    CallbackCompletionSuccess,
}
