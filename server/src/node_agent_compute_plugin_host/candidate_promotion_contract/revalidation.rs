use std::{error::Error as StdError, fmt, time::Instant};

use anyhow::Error;

use crate::node_agent_compute_plugin_host::{
    candidate_health_contract::DurableCandidateHealthPublication,
    trusted_time::ComputePluginTrustedTimeObservation,
};

use super::{
    CandidatePromotionRevalidationCustody, PendingCandidatePromotionRevalidation,
    RevalidatedCandidatePromotion,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidatePromotionRevalidationPhase {
    RetainedContent,
    PostRevalidationTrustedTime,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidatePromotionRevalidationFailure<'root> {
    phase: CandidatePromotionRevalidationPhase,
    error: Error,
    custody: CandidatePromotionRevalidationCustody<'root>,
}

pub(in crate::node_agent_compute_plugin_host) fn begin_candidate_promotion_revalidation<'root>(
    mut publication: DurableCandidateHealthPublication<'root>,
) -> std::result::Result<
    PendingCandidatePromotionRevalidation<'root>,
    CandidatePromotionRevalidationFailure<'root>,
> {
    let guard = publication.staged().archive().snapshot_cancellation_guard();
    let result = guard
        .ensure_current()
        .and_then(|_| publication.revalidate_retained_content())
        .and_then(|_| guard.ensure_current());
    if let Err(error) = result {
        return Err(CandidatePromotionRevalidationFailure {
            phase: CandidatePromotionRevalidationPhase::RetainedContent,
            error,
            custody: CandidatePromotionRevalidationCustody::Durable(publication),
        });
    }
    Ok(PendingCandidatePromotionRevalidation::new(
        publication,
        Instant::now(),
    ))
}

pub(in crate::node_agent_compute_plugin_host) fn complete_candidate_promotion_revalidation<
    'root,
>(
    pending: PendingCandidatePromotionRevalidation<'root>,
    trusted_time: ComputePluginTrustedTimeObservation,
) -> std::result::Result<
    RevalidatedCandidatePromotion<'root>,
    CandidatePromotionRevalidationFailure<'root>,
> {
    match complete(pending, trusted_time) {
        Ok(revalidated) => Ok(revalidated),
        Err((error, pending)) => Err(CandidatePromotionRevalidationFailure {
            phase: CandidatePromotionRevalidationPhase::PostRevalidationTrustedTime,
            error,
            custody: CandidatePromotionRevalidationCustody::Pending(pending),
        }),
    }
}

fn complete<'root>(
    pending: PendingCandidatePromotionRevalidation<'root>,
    trusted_time: ComputePluginTrustedTimeObservation,
) -> std::result::Result<
    RevalidatedCandidatePromotion<'root>,
    (Error, PendingCandidatePromotionRevalidation<'root>),
> {
    let now = Instant::now();
    let key = pending
        .publication()
        .staged()
        .archive()
        .verification_recovery_key();
    let health = pending.publication().receipt().receipt();
    let guard = pending
        .publication()
        .staged()
        .archive()
        .snapshot_cancellation_guard();
    if let Err(error) = trusted_time
        .ensure_live(now)
        .and_then(|_| guard.ensure_current())
        .and_then(|_| {
            if trusted_time.observed_at() <= pending.revalidated_at()
                || trusted_time.installation_id_digest() != key.installation_id_digest()
                || trusted_time.clock_epoch_digest() != key.clock_epoch_digest()
                || health.candidate_token_digest() != key.candidate_token_digest()
                || health.process_owner_epoch() != key.process_owner_epoch()
            {
                anyhow::bail!("COMPUTE_PLUGIN_PROMOTION_POST_REVALIDATION_TIME_INVALID");
            }
            Ok(())
        })
    {
        return Err((error, pending));
    }
    let (publication, revalidated_at) = pending.into_parts();
    Ok(RevalidatedCandidatePromotion::new(
        publication,
        trusted_time,
        revalidated_at,
    ))
}

impl CandidatePromotionRevalidationFailure<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidatePromotionRevalidationPhase {
        self.phase
    }
}

impl<'root> CandidatePromotionRevalidationFailure<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, CandidatePromotionRevalidationCustody<'root>) {
        (self.error, self.custody)
    }
}

impl fmt::Display for CandidatePromotionRevalidationFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

impl fmt::Debug for CandidatePromotionRevalidationFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidatePromotionRevalidationFailure")
            .field("phase", &self.phase)
            .field("error", &self.error)
            .field("custody", &self.custody)
            .finish()
    }
}

impl StdError for CandidatePromotionRevalidationFailure<'_> {}
