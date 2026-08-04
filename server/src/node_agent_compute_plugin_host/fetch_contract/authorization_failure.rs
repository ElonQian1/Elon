use std::fmt;

use anyhow::Error;

use super::{
    recovery::ComputePluginFetchClaimRecoveryKey, types::AuthorizedComputePluginDownloadSegment,
};

pub(in crate::node_agent_compute_plugin_host) type ComputePluginFetchAuthorizationResult =
    std::result::Result<
        AuthorizedComputePluginDownloadSegment,
        ComputePluginFetchAuthorizationFailure,
    >;

/// Initial claim failures either prove that no claim mutation was called, or retain the exact
/// pre-generated identity required to inspect an uncertain Store outcome. Error details and claim
/// identity stay redacted from Debug output.
pub(in crate::node_agent_compute_plugin_host) enum ComputePluginFetchAuthorizationFailure {
    RejectedBeforeClaim {
        error: Error,
    },
    ClaimOutcomeRecoveryRequired {
        error: Error,
        recovery_key: ComputePluginFetchClaimRecoveryKey,
    },
}

impl fmt::Debug for ComputePluginFetchAuthorizationFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RejectedBeforeClaim { .. } => formatter
                .debug_struct("ComputePluginFetchAuthorizationFailure")
                .field("kind", &"rejected_before_claim")
                .field("error", &"<redacted>")
                .finish(),
            Self::ClaimOutcomeRecoveryRequired { .. } => formatter
                .debug_struct("ComputePluginFetchAuthorizationFailure")
                .field("kind", &"claim_outcome_recovery_required")
                .field("error", &"<redacted>")
                .field("recovery_key", &"<redacted>")
                .finish(),
        }
    }
}

impl ComputePluginFetchAuthorizationFailure {
    pub(super) fn rejected(error: Error) -> Self {
        Self::RejectedBeforeClaim { error }
    }

    pub(super) fn outcome_recovery_required(
        error: Error,
        recovery_key: ComputePluginFetchClaimRecoveryKey,
    ) -> Self {
        Self::ClaimOutcomeRecoveryRequired {
            error,
            recovery_key,
        }
    }
}

pub(in crate::node_agent_compute_plugin_host) type ComputePluginFetchRedirectResult =
    std::result::Result<AuthorizedComputePluginDownloadSegment, ComputePluginFetchRedirectFailure>;

/// Redirect never advances the durable cursor. Before a Store call it returns the complete old
/// authorization; after a Store call it permanently removes that mutation capability and returns
/// only the exact claim recovery identity (whose redirect generation is a lower bound).
pub(in crate::node_agent_compute_plugin_host) enum ComputePluginFetchRedirectFailure {
    StoreNotCalled {
        error: Error,
        authorized: AuthorizedComputePluginDownloadSegment,
    },
    OutcomeRecoveryRequired {
        error: Error,
        recovery_key: ComputePluginFetchClaimRecoveryKey,
    },
}

impl fmt::Debug for ComputePluginFetchRedirectFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StoreNotCalled { .. } => formatter
                .debug_struct("ComputePluginFetchRedirectFailure")
                .field("kind", &"store_not_called")
                .field("error", &"<redacted>")
                .field("authorization", &"<retained>")
                .finish(),
            Self::OutcomeRecoveryRequired { .. } => formatter
                .debug_struct("ComputePluginFetchRedirectFailure")
                .field("kind", &"outcome_recovery_required")
                .field("error", &"<redacted>")
                .field("recovery_key", &"<redacted>")
                .finish(),
        }
    }
}

impl ComputePluginFetchRedirectFailure {
    pub(super) fn store_not_called(
        error: Error,
        authorized: AuthorizedComputePluginDownloadSegment,
    ) -> Self {
        Self::StoreNotCalled { error, authorized }
    }

    pub(super) fn outcome_recovery_required(
        error: Error,
        recovery_key: ComputePluginFetchClaimRecoveryKey,
    ) -> Self {
        Self::OutcomeRecoveryRequired {
            error,
            recovery_key,
        }
    }
}
