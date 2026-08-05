use std::fmt;

use anyhow::Error;

use super::{
    resolution::validate_authorized_binding,
    types::{
        ComputePluginFetchAbortReason, PreparedComputePluginFetchClaim,
        ValidatedComputePluginFetchAbortPermit,
    },
    ComputePluginFetchAuthorityPort,
};
use crate::node_agent_compute_plugin_host::{
    fetch_contract::ComputePluginFetchClaimRecoveryKey,
    fetch_file::{
        ComputePluginPartCursorDamage, ComputePluginPartCursorDamageKind,
        ComputePluginPinnedFileRecovery,
    },
    install_plan_admission::AdmittedComputePluginInstallPlan,
    local_authority::ComputePluginFetchAuthoritySession,
};

pub(in crate::node_agent_compute_plugin_host) type ComputePluginCursorDamageResult<'authority> =
    std::result::Result<FailedComputePluginDownload, ComputePluginCursorDamageFailure<'authority>>;

#[derive(Debug, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) struct FailedComputePluginDownload {
    pub(crate) ordinal: usize,
    pub(crate) committed_offset: i64,
    pub(crate) damage_kind: ComputePluginPartCursorDamageKind,
    pub(crate) observed_length_bytes: Option<i64>,
}

pub(in crate::node_agent_compute_plugin_host) enum ComputePluginCursorDamageFailure<'authority> {
    BeforeStore {
        error: Error,
        damage: ComputePluginPartCursorDamage,
        authority_session: ComputePluginFetchAuthoritySession<'authority>,
    },
    OutcomeRecoveryRequired {
        error: Error,
        recovery_key: ComputePluginFetchClaimRecoveryKey,
        file: Option<ComputePluginPinnedFileRecovery>,
        damage_kind: ComputePluginPartCursorDamageKind,
        observed_length_bytes: Option<i64>,
        authority_session: ComputePluginFetchAuthoritySession<'authority>,
    },
}

impl fmt::Debug for ComputePluginCursorDamageFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BeforeStore { damage, .. } => formatter
                .debug_struct("ComputePluginCursorDamageFailure")
                .field("phase", &"before_store")
                .field("error", &"<redacted>")
                .field("damage", damage)
                .field("authority_session", &"<retained>")
                .finish(),
            Self::OutcomeRecoveryRequired {
                file,
                damage_kind,
                observed_length_bytes,
                ..
            } => formatter
                .debug_struct("ComputePluginCursorDamageFailure")
                .field("phase", &"store_outcome_uncertain")
                .field("error", &"<redacted>")
                .field("recovery_key", &"<redacted>")
                .field("file", &file.as_ref().map(|_| "<retained>"))
                .field("damage_kind", damage_kind)
                .field("observed_length_bytes", observed_length_bytes)
                .field("authority_session", &"<retained>")
                .finish(),
        }
    }
}

pub(in crate::node_agent_compute_plugin_host) struct ValidatedComputePluginCursorDamagePermit<
    'permit,
> {
    abort: ValidatedComputePluginFetchAbortPermit<'permit>,
}

impl<'permit> ValidatedComputePluginCursorDamagePermit<'permit> {
    fn new(
        claim: &'permit PreparedComputePluginFetchClaim,
        reason: ComputePluginFetchAbortReason,
    ) -> Self {
        debug_assert!(reason.is_cursor_damage());
        Self {
            abort: ValidatedComputePluginFetchAbortPermit::new(claim, reason),
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn abort_permit(
        &self,
    ) -> &ValidatedComputePluginFetchAbortPermit<'permit> {
        &self.abort
    }

    pub(in crate::node_agent_compute_plugin_host) fn reason(
        &self,
    ) -> ComputePluginFetchAbortReason {
        self.abort.reason()
    }
}

/// Consumes one exact cursor/file mismatch and atomically marks both its prepared claim and
/// planned download terminal for this attempt. No writable authorization survives a Store call.
pub(in crate::node_agent_compute_plugin_host) fn fail_cursor_damaged_download<'authority>(
    admitted: &AdmittedComputePluginInstallPlan,
    mut damage: ComputePluginPartCursorDamage,
    authority_session: ComputePluginFetchAuthoritySession<'authority>,
) -> ComputePluginCursorDamageResult<'authority> {
    let validation = (|| {
        damage
            .authorized()
            .validate_recovery_session(&authority_session)?;
        validate_authorized_binding(admitted, damage.authorized())?;
        damage.validate_exact_evidence()
    })();
    if let Err(error) = validation {
        return Err(ComputePluginCursorDamageFailure::BeforeStore {
            error,
            damage,
            authority_session,
        });
    }

    let damage_kind = damage.kind();
    let observed_length_bytes = damage.observed_length_bytes();
    let ordinal = damage.authorized().ordinal();
    let committed_offset = damage.authorized().offset_bytes();
    let reason = match damage_kind {
        ComputePluginPartCursorDamageKind::MissingCommittedFile => {
            ComputePluginFetchAbortReason::CommittedFileMissing
        }
        ComputePluginPartCursorDamageKind::ShorterThanCommittedCursor => {
            ComputePluginFetchAbortReason::CommittedFileShorter
        }
    };
    let permit =
        ValidatedComputePluginCursorDamagePermit::new(damage.authorized().prepared_claim(), reason);
    let authority = ComputePluginFetchAuthorityPort {
        backend: &authority_session,
    };
    if let Err(error) = authority.fail_validated_cursor_damage(permit) {
        let (recovery_key, file) = damage.into_recovery_custody();
        return Err(ComputePluginCursorDamageFailure::OutcomeRecoveryRequired {
            error,
            recovery_key,
            file,
            damage_kind,
            observed_length_bytes,
            authority_session,
        });
    }

    Ok(FailedComputePluginDownload {
        ordinal,
        committed_offset,
        damage_kind,
        observed_length_bytes,
    })
}
