use std::fmt;

use super::{
    opened_authority::OpenedComputePluginLocalAuthority,
    process_ownership::ComputePluginFetchProcessFence,
};
use crate::node_agent_compute_plugin_host::{
    keyring::ComputePluginBootstrapRootKeyResolver,
    manifest_catalog::ComputePluginManifestCatalogCandidate,
    trusted_time::ComputePluginTrustedTimeObservation,
};

mod recovery;
mod types;
mod validation;
mod write;

use types::ComputePluginManifestCatalogBindingRecoveryKey;
pub(in crate::node_agent_compute_plugin_host) use types::{
    ComputePluginManifestCatalogBindingReceipt, HashedComputePluginManifestCatalogBindingReceipt,
    COMPUTE_PLUGIN_MANIFEST_CATALOG_BINDING_RECEIPT_SCHEMA,
    HASHED_COMPUTE_PLUGIN_MANIFEST_CATALOG_BINDING_RECEIPT_SCHEMA,
};

/// A catalog activation plus the exact already-open authority that committed it. The receipt is
/// planning evidence only; this value contains no InstallPlan, download, runtime or work-admission
/// capability.
#[must_use = "the opened authority and catalog receipt must remain paired"]
pub(in crate::node_agent_compute_plugin_host) struct DurableComputePluginManifestCatalogBinding {
    authority: OpenedComputePluginLocalAuthority,
    receipt: HashedComputePluginManifestCatalogBindingReceipt,
}

impl DurableComputePluginManifestCatalogBinding {
    pub(in crate::node_agent_compute_plugin_host) fn authority(
        &self,
    ) -> &OpenedComputePluginLocalAuthority {
        &self.authority
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority_mut(
        &mut self,
    ) -> &mut OpenedComputePluginLocalAuthority {
        &mut self.authority
    }

    pub(in crate::node_agent_compute_plugin_host) fn receipt(
        &self,
    ) -> &HashedComputePluginManifestCatalogBindingReceipt {
        &self.receipt
    }
}

impl fmt::Debug for DurableComputePluginManifestCatalogBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DurableComputePluginManifestCatalogBinding")
            .field("authority", &self.authority)
            .field("receipt_digest", &self.receipt.receipt_digest)
            .finish()
    }
}

pub(in crate::node_agent_compute_plugin_host) struct RejectedComputePluginManifestCatalogBinding {
    authority: OpenedComputePluginLocalAuthority,
    candidate: ComputePluginManifestCatalogCandidate,
    error: anyhow::Error,
}

impl RejectedComputePluginManifestCatalogBinding {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (
        OpenedComputePluginLocalAuthority,
        ComputePluginManifestCatalogCandidate,
        anyhow::Error,
    ) {
        (self.authority, self.candidate, self.error)
    }
}

impl fmt::Debug for RejectedComputePluginManifestCatalogBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RejectedComputePluginManifestCatalogBinding")
            .field("authority", &self.authority)
            .field("candidate", &self.candidate)
            .field("error", &self.error)
            .finish()
    }
}

#[must_use = "commit uncertainty must be adopted before the authority can be reused"]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginManifestCatalogBindingRecovery {
    authority: OpenedComputePluginLocalAuthority,
    candidate: ComputePluginManifestCatalogCandidate,
    key: ComputePluginManifestCatalogBindingRecoveryKey,
    error: anyhow::Error,
}

impl fmt::Debug for ComputePluginManifestCatalogBindingRecovery {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginManifestCatalogBindingRecovery")
            .field("authority", &self.authority)
            .field("catalog_revision", &self.key.request.catalog_revision)
            .field("receipt_digest", &self.key.hashed_receipt.receipt_digest)
            .field("error", &self.error)
            .finish()
    }
}

pub(in crate::node_agent_compute_plugin_host) enum ComputePluginManifestCatalogBindingStoreResult {
    Durable(DurableComputePluginManifestCatalogBinding),
    Rejected(RejectedComputePluginManifestCatalogBinding),
    Recovery(ComputePluginManifestCatalogBindingRecovery),
}

pub(in crate::node_agent_compute_plugin_host) enum ComputePluginManifestCatalogBindingRecoveryOutcome
{
    Durable(DurableComputePluginManifestCatalogBinding),
    CommittedHistorical(HashedComputePluginManifestCatalogBindingReceipt),
    NotCreated {
        authority: OpenedComputePluginLocalAuthority,
        candidate: ComputePluginManifestCatalogCandidate,
    },
    NotCreatedSuperseded(OpenedComputePluginLocalAuthority),
    Retained(ComputePluginManifestCatalogBindingRecovery),
}

impl OpenedComputePluginLocalAuthority {
    /// Activates one locally verified canonical catalog. This source-only seam is unreachable while
    /// handle-bound SQLite VFS construction remains unavailable.
    pub(in crate::node_agent_compute_plugin_host) fn bind_manifest_catalog(
        self,
        candidate: ComputePluginManifestCatalogCandidate,
        process_fence: &ComputePluginFetchProcessFence,
        observation: ComputePluginTrustedTimeObservation,
        roots: &dyn ComputePluginBootstrapRootKeyResolver,
    ) -> ComputePluginManifestCatalogBindingStoreResult {
        write::bind(self, candidate, process_fence, observation, roots)
    }
}

impl ComputePluginManifestCatalogBindingRecovery {
    pub(in crate::node_agent_compute_plugin_host) fn adopt(
        self,
        process_fence: &ComputePluginFetchProcessFence,
        observation: ComputePluginTrustedTimeObservation,
        roots: &dyn ComputePluginBootstrapRootKeyResolver,
    ) -> ComputePluginManifestCatalogBindingRecoveryOutcome {
        recovery::adopt(self, process_fence, observation, roots)
    }
}

fn durable(
    authority: OpenedComputePluginLocalAuthority,
    receipt: HashedComputePluginManifestCatalogBindingReceipt,
) -> DurableComputePluginManifestCatalogBinding {
    DurableComputePluginManifestCatalogBinding { authority, receipt }
}

fn rejected(
    authority: OpenedComputePluginLocalAuthority,
    candidate: ComputePluginManifestCatalogCandidate,
    error: anyhow::Error,
) -> ComputePluginManifestCatalogBindingStoreResult {
    ComputePluginManifestCatalogBindingStoreResult::Rejected(
        RejectedComputePluginManifestCatalogBinding {
            authority,
            candidate,
            error,
        },
    )
}

fn recovery(
    authority: OpenedComputePluginLocalAuthority,
    candidate: ComputePluginManifestCatalogCandidate,
    key: ComputePluginManifestCatalogBindingRecoveryKey,
    error: anyhow::Error,
) -> ComputePluginManifestCatalogBindingStoreResult {
    ComputePluginManifestCatalogBindingStoreResult::Recovery(
        ComputePluginManifestCatalogBindingRecovery {
            authority,
            candidate,
            key,
            error,
        },
    )
}
