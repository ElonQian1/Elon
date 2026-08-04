use std::fmt;

use anyhow::Error;

use super::super::{
    recovery::ComputePluginFetchClaimRecoveryKey,
    types::{
        AbortedComputePluginDownloadSegment, AuthorizedComputePluginDownloadSegment,
        CommittedComputePluginDownloadSegment, DurablyWrittenComputePluginSegment,
    },
};
use crate::node_agent_compute_plugin_host::local_authority::ComputePluginFetchAuthoritySession;

/// Describes only whether the SQLite Store mutation was called. The `.part` file may already have
/// been durably changed in either phase; neither phase authorizes repeating file or Store work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host::fetch_contract) enum ComputePluginFetchStoreMutationPhase
{
    StoreNotCalled,
    StoreOutcomeUncertain,
}

pub(in crate::node_agent_compute_plugin_host::fetch_contract) type ComputePluginFetchCommitResult<
    'authority,
> = std::result::Result<
    CommittedComputePluginDownloadSegment,
    ComputePluginFetchCommitFailure<'authority>,
>;

/// Owns the still-open file handle whenever commit does not return a receipt. This prevents a
/// later recovery layer from reopening a path that an attacker or cleanup task may have replaced.
/// The outcome-recovery variant deliberately omits the consumed authorization handle, so an
/// uncertain commit cannot be retried with the old mutation capability.
pub(in crate::node_agent_compute_plugin_host::fetch_contract) enum ComputePluginFetchCommitFailure<
    'authority,
> {
    RecoveryBindingUnavailable {
        error: Error,
        authorized: AuthorizedComputePluginDownloadSegment,
        durable: DurablyWrittenComputePluginSegment<'authority>,
    },
    OutcomeRecoveryRequired {
        store_phase: ComputePluginFetchStoreMutationPhase,
        error: Error,
        recovery_key: ComputePluginFetchClaimRecoveryKey,
        durable: DurablyWrittenComputePluginSegment<'authority>,
    },
}

impl fmt::Debug for ComputePluginFetchCommitFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RecoveryBindingUnavailable { .. } => formatter
                .debug_struct("ComputePluginFetchCommitFailure")
                .field("kind", &"recovery_binding_unavailable")
                .field("error", &"<redacted>")
                .field("authorization", &"<retained>")
                .field("durable_file", &"<retained>")
                .finish(),
            Self::OutcomeRecoveryRequired { store_phase, .. } => formatter
                .debug_struct("ComputePluginFetchCommitFailure")
                .field("kind", &"outcome_recovery_required")
                .field("store_phase", store_phase)
                .field("error", &"<redacted>")
                .field("recovery_key", &"<redacted>")
                .field("durable_file", &"<retained>")
                .finish(),
        }
    }
}

impl<'authority> ComputePluginFetchCommitFailure<'authority> {
    pub(super) fn recovery_binding_unavailable(
        error: Error,
        authorized: AuthorizedComputePluginDownloadSegment,
        durable: DurablyWrittenComputePluginSegment<'authority>,
    ) -> Self {
        Self::RecoveryBindingUnavailable {
            error,
            authorized,
            durable,
        }
    }

    pub(super) fn outcome_recovery_required(
        store_phase: ComputePluginFetchStoreMutationPhase,
        error: Error,
        recovery_key: ComputePluginFetchClaimRecoveryKey,
        durable: DurablyWrittenComputePluginSegment<'authority>,
    ) -> Self {
        Self::OutcomeRecoveryRequired {
            store_phase,
            error,
            recovery_key,
            durable,
        }
    }
}

pub(in crate::node_agent_compute_plugin_host::fetch_contract) type ComputePluginFetchAbortResult<
    'authority,
> = std::result::Result<
    AbortedComputePluginDownloadSegment,
    ComputePluginFetchAbortFailure<'authority>,
>;

/// Retains the sealed authority session on every abort failure. Once a Store mutation was
/// attempted, the old authorization is intentionally discarded and only stable outcome recovery
/// remains possible.
pub(in crate::node_agent_compute_plugin_host::fetch_contract) enum ComputePluginFetchAbortFailure<
    'authority,
> {
    RecoveryBindingUnavailable {
        error: Error,
        authorized: AuthorizedComputePluginDownloadSegment,
        authority_session: ComputePluginFetchAuthoritySession<'authority>,
    },
    OutcomeRecoveryRequired {
        store_phase: ComputePluginFetchStoreMutationPhase,
        error: Error,
        recovery_key: ComputePluginFetchClaimRecoveryKey,
        authority_session: ComputePluginFetchAuthoritySession<'authority>,
    },
}

impl fmt::Debug for ComputePluginFetchAbortFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RecoveryBindingUnavailable { .. } => formatter
                .debug_struct("ComputePluginFetchAbortFailure")
                .field("kind", &"recovery_binding_unavailable")
                .field("error", &"<redacted>")
                .field("authorization", &"<retained>")
                .field("authority_session", &"<retained>")
                .finish(),
            Self::OutcomeRecoveryRequired { store_phase, .. } => formatter
                .debug_struct("ComputePluginFetchAbortFailure")
                .field("kind", &"outcome_recovery_required")
                .field("store_phase", store_phase)
                .field("error", &"<redacted>")
                .field("recovery_key", &"<redacted>")
                .field("authority_session", &"<retained>")
                .finish(),
        }
    }
}

impl<'authority> ComputePluginFetchAbortFailure<'authority> {
    pub(super) fn recovery_binding_unavailable(
        error: Error,
        authorized: AuthorizedComputePluginDownloadSegment,
        authority_session: ComputePluginFetchAuthoritySession<'authority>,
    ) -> Self {
        Self::RecoveryBindingUnavailable {
            error,
            authorized,
            authority_session,
        }
    }

    pub(super) fn outcome_recovery_required(
        store_phase: ComputePluginFetchStoreMutationPhase,
        error: Error,
        recovery_key: ComputePluginFetchClaimRecoveryKey,
        authority_session: ComputePluginFetchAuthoritySession<'authority>,
    ) -> Self {
        Self::OutcomeRecoveryRequired {
            store_phase,
            error,
            recovery_key,
            authority_session,
        }
    }
}
