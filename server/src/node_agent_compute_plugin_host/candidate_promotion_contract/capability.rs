use std::{fmt, time::Instant};

use crate::node_agent_compute_plugin_host::{
    candidate_health_contract::DurableCandidateHealthPublication,
    candidate_staging_contract::StagedComputePluginCandidateArchive,
    local_authority::HashedComputePluginCandidateHealthReceipt,
    trusted_time::ComputePluginTrustedTimeObservation,
};

use super::CandidatePromotionReceiptPair;

/// Healthy publication custody after its pinned staged content has been rehashed. The monotonic
/// barrier forces the trusted-time observation and promotion authority session to be minted later.
#[must_use = "pending promotion revalidation must receive post-revalidation trusted time"]
pub(in crate::node_agent_compute_plugin_host) struct PendingCandidatePromotionRevalidation<'root> {
    publication: DurableCandidateHealthPublication<'root>,
    revalidated_at: Instant,
}

/// Exact healthy candidate custody plus an authenticated time observation emitted strictly after
/// the final retained-content revalidation. This is not Store authority.
#[must_use = "revalidated candidate promotion must be authorized or returned for cleanup"]
pub(in crate::node_agent_compute_plugin_host) struct RevalidatedCandidatePromotion<'root> {
    publication: DurableCandidateHealthPublication<'root>,
    trusted_time: ComputePluginTrustedTimeObservation,
    revalidated_at: Instant,
}

/// Custody retained by a revalidation failure. No variant is an installation or retry permit.
pub(in crate::node_agent_compute_plugin_host) enum CandidatePromotionRevalidationCustody<'root> {
    Durable(DurableCandidateHealthPublication<'root>),
    Pending(PendingCandidatePromotionRevalidation<'root>),
}

/// Installed candidate custody after the exact install and active-slot promotion transaction is
/// durable. Pinned staged handles remain owned here. This cannot prove a started runtime, Ready
/// capability, endpoint, session, or work admission.
#[must_use = "installed slot custody must be retained for activation or cleanup"]
pub(in crate::node_agent_compute_plugin_host) struct DurableInstalledPluginSlot<'root> {
    revalidated: RevalidatedCandidatePromotion<'root>,
    receipts: CandidatePromotionReceiptPair,
}

impl<'root> PendingCandidatePromotionRevalidation<'root> {
    pub(super) fn new(
        publication: DurableCandidateHealthPublication<'root>,
        revalidated_at: Instant,
    ) -> Self {
        Self {
            publication,
            revalidated_at,
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn publication(
        &self,
    ) -> &DurableCandidateHealthPublication<'root> {
        &self.publication
    }

    pub(in crate::node_agent_compute_plugin_host) fn revalidated_at(&self) -> Instant {
        self.revalidated_at
    }

    pub(super) fn into_parts(self) -> (DurableCandidateHealthPublication<'root>, Instant) {
        (self.publication, self.revalidated_at)
    }
}

impl RevalidatedCandidatePromotion<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn publication(
        &self,
    ) -> &DurableCandidateHealthPublication<'_> {
        &self.publication
    }

    pub(in crate::node_agent_compute_plugin_host) fn staged(
        &self,
    ) -> &StagedComputePluginCandidateArchive<'_> {
        self.publication.staged()
    }

    pub(in crate::node_agent_compute_plugin_host) fn health(
        &self,
    ) -> &HashedComputePluginCandidateHealthReceipt {
        self.publication.receipt()
    }

    pub(in crate::node_agent_compute_plugin_host) fn trusted_time(
        &self,
    ) -> &ComputePluginTrustedTimeObservation {
        &self.trusted_time
    }

    pub(in crate::node_agent_compute_plugin_host) fn revalidated_at(&self) -> Instant {
        self.revalidated_at
    }

    pub(super) fn fresh_revalidate_retained_content(&mut self) -> anyhow::Result<()> {
        let guard = self
            .publication
            .staged()
            .archive()
            .snapshot_cancellation_guard();
        guard.ensure_current()?;
        self.publication.revalidate_retained_content()?;
        guard.ensure_current()
    }
}

impl<'root> RevalidatedCandidatePromotion<'root> {
    pub(super) fn new(
        publication: DurableCandidateHealthPublication<'root>,
        trusted_time: ComputePluginTrustedTimeObservation,
        revalidated_at: Instant,
    ) -> Self {
        Self {
            publication,
            trusted_time,
            revalidated_at,
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_publication(
        self,
    ) -> DurableCandidateHealthPublication<'root> {
        self.publication
    }
}

impl<'root> DurableInstalledPluginSlot<'root> {
    pub(super) fn new(
        revalidated: RevalidatedCandidatePromotion<'root>,
        receipts: CandidatePromotionReceiptPair,
    ) -> Self {
        Self {
            revalidated,
            receipts,
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn revalidated(
        &self,
    ) -> &RevalidatedCandidatePromotion<'root> {
        &self.revalidated
    }

    pub(in crate::node_agent_compute_plugin_host) fn receipts(
        &self,
    ) -> &CandidatePromotionReceiptPair {
        &self.receipts
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (
        RevalidatedCandidatePromotion<'root>,
        CandidatePromotionReceiptPair,
    ) {
        (self.revalidated, self.receipts)
    }
}

impl fmt::Debug for PendingCandidatePromotionRevalidation<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PendingCandidatePromotionRevalidation")
            .field("publication", &self.publication)
            .field("revalidated_at", &"<monotonic>")
            .finish()
    }
}

impl fmt::Debug for RevalidatedCandidatePromotion<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RevalidatedCandidatePromotion")
            .field("publication", &self.publication)
            .field("trusted_time", &"<authenticated>")
            .field("revalidated_at", &"<monotonic>")
            .finish()
    }
}

impl fmt::Debug for CandidatePromotionRevalidationCustody<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Durable(_) => formatter.write_str("Durable(<retained-handles>)"),
            Self::Pending(_) => formatter.write_str("Pending(<retained-handles>)"),
        }
    }
}

impl fmt::Debug for DurableInstalledPluginSlot<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DurableInstalledPluginSlot")
            .field("receipts", &self.receipts)
            .field("staged_handles", &"<retained>")
            .finish()
    }
}
