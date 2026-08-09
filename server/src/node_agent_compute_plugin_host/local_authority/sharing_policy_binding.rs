use std::fmt;

use anyhow::Result;

use super::{process_ownership::ComputePluginFetchProcessFence, ComputePluginLocalAuthority};
use crate::node_agent_compute_plugin_host::{
    bootstrap::ComputePluginLocalPolicyBindingIntent, fetch_file::PinnedComputePluginRoot,
    root_lock::ComputePluginRootLockLease, trusted_time::ComputePluginTrustedTimeObservation,
};

mod recovery;
mod revocation;
#[cfg(test)]
mod test_support;
mod types;
mod validation;
mod write;

pub(in crate::node_agent_compute_plugin_host) use revocation::{
    ComputePluginSharingPolicyCapabilityRevocationReceipt,
    HashedComputePluginSharingPolicyCapabilityRevocationReceipt,
    COMPUTE_PLUGIN_SHARING_POLICY_CAPABILITY_REVOCATION_RECEIPT_SCHEMA,
    HASHED_COMPUTE_PLUGIN_SHARING_POLICY_CAPABILITY_REVOCATION_RECEIPT_SCHEMA,
};
use types::ComputePluginSharingPolicyBindingRecoveryKey;
pub(in crate::node_agent_compute_plugin_host) use types::{
    ComputePluginSharingPolicyBindingReceipt, HashedComputePluginSharingPolicyBindingReceipt,
    COMPUTE_PLUGIN_SHARING_POLICY_BINDING_RECEIPT_SCHEMA,
    HASHED_COMPUTE_PLUGIN_SHARING_POLICY_BINDING_RECEIPT_SCHEMA,
};

/// A durable local policy binding. It is only one admission prerequisite: this value does not
/// contain a signed InstallPlan, keyring, root pin, local confirmation or download authority.
#[must_use = "the durable policy binding must remain paired with its Bootstrap custody"]
pub(in crate::node_agent_compute_plugin_host) struct DurableComputePluginSharingPolicyBinding {
    intent: ComputePluginLocalPolicyBindingIntent,
    receipt: HashedComputePluginSharingPolicyBindingReceipt,
    revocation_receipt: HashedComputePluginSharingPolicyCapabilityRevocationReceipt,
    _root_lock: ComputePluginRootLockLease,
}

impl DurableComputePluginSharingPolicyBinding {
    pub(in crate::node_agent_compute_plugin_host) fn intent(
        &self,
    ) -> &ComputePluginLocalPolicyBindingIntent {
        &self.intent
    }

    pub(in crate::node_agent_compute_plugin_host) fn receipt(
        &self,
    ) -> &HashedComputePluginSharingPolicyBindingReceipt {
        &self.receipt
    }

    pub(in crate::node_agent_compute_plugin_host) fn revocation_receipt(
        &self,
    ) -> &HashedComputePluginSharingPolicyCapabilityRevocationReceipt {
        &self.revocation_receipt
    }
}

impl fmt::Debug for DurableComputePluginSharingPolicyBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DurableComputePluginSharingPolicyBinding")
            .field("intent", &self.intent)
            .field("receipt_digest", &self.receipt.receipt_digest())
            .field(
                "revocation_receipt_digest",
                &self.revocation_receipt.receipt_digest(),
            )
            .finish()
    }
}

/// A pre-mutation rejection. The exact intent is returned so the caller may correct missing local
/// prerequisites without reconstructing cloud authorization from loose scalar fields.
pub(in crate::node_agent_compute_plugin_host) struct RejectedComputePluginSharingPolicyBinding {
    intent: ComputePluginLocalPolicyBindingIntent,
    error: anyhow::Error,
}

impl RejectedComputePluginSharingPolicyBinding {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (ComputePluginLocalPolicyBindingIntent, anyhow::Error) {
        (self.intent, self.error)
    }
}

impl fmt::Debug for RejectedComputePluginSharingPolicyBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RejectedComputePluginSharingPolicyBinding")
            .field("intent", &self.intent)
            .field("error", &self.error)
            .finish()
    }
}

/// Commit uncertainty retains both the linear Bootstrap intent and the complete expected database
/// transition. It cannot be downgraded into a fresh request until recovery proves `NotCreated`.
#[must_use = "commit uncertainty must be adopted before the intent can be retried"]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginSharingPolicyBindingRecovery {
    intent: ComputePluginLocalPolicyBindingIntent,
    key: ComputePluginSharingPolicyBindingRecoveryKey,
    error: anyhow::Error,
    root_lock: ComputePluginRootLockLease,
}

impl fmt::Debug for ComputePluginSharingPolicyBindingRecovery {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginSharingPolicyBindingRecovery")
            .field("intent", &self.intent)
            .field("policy_revision", &self.key.request.policy_revision)
            .field("receipt_digest", &self.key.hashed_receipt.receipt_digest())
            .field("error", &self.error)
            .finish()
    }
}

pub(in crate::node_agent_compute_plugin_host) enum ComputePluginSharingPolicyBindingStoreResult {
    Durable(DurableComputePluginSharingPolicyBinding),
    Rejected(RejectedComputePluginSharingPolicyBinding),
    Recovery(ComputePluginSharingPolicyBindingRecovery),
}

pub(in crate::node_agent_compute_plugin_host) enum ComputePluginSharingPolicyBindingRecoveryOutcome
{
    Durable(DurableComputePluginSharingPolicyBinding),
    CommittedHistorical {
        binding: HashedComputePluginSharingPolicyBindingReceipt,
        revocation: HashedComputePluginSharingPolicyCapabilityRevocationReceipt,
    },
    NotCreated(ComputePluginLocalPolicyBindingIntent),
    NotCreatedSuperseded,
    Retained(ComputePluginSharingPolicyBindingRecovery),
}

impl ComputePluginLocalAuthority {
    /// Applies only the desired sharing-policy and authorization head. This transaction performs no
    /// download, extraction, keyring installation, PlanApply, candidate mutation or process launch.
    pub(in crate::node_agent_compute_plugin_host) fn bind_sharing_policy(
        &self,
        intent: ComputePluginLocalPolicyBindingIntent,
        root: &PinnedComputePluginRoot,
        process_fence: &ComputePluginFetchProcessFence,
        observation: ComputePluginTrustedTimeObservation,
    ) -> ComputePluginSharingPolicyBindingStoreResult {
        write::bind(self, intent, root, process_fence, observation)
    }

    pub(in crate::node_agent_compute_plugin_host) fn adopt_sharing_policy_binding_recovery(
        &self,
        recovery: ComputePluginSharingPolicyBindingRecovery,
        root: &PinnedComputePluginRoot,
        process_fence: &ComputePluginFetchProcessFence,
        observation: ComputePluginTrustedTimeObservation,
    ) -> ComputePluginSharingPolicyBindingRecoveryOutcome {
        recovery::adopt(self, recovery, root, process_fence, observation)
    }
}

fn rejected(
    intent: ComputePluginLocalPolicyBindingIntent,
    error: anyhow::Error,
) -> ComputePluginSharingPolicyBindingStoreResult {
    ComputePluginSharingPolicyBindingStoreResult::Rejected(
        RejectedComputePluginSharingPolicyBinding { intent, error },
    )
}

fn durable(
    intent: ComputePluginLocalPolicyBindingIntent,
    receipt: HashedComputePluginSharingPolicyBindingReceipt,
    revocation_receipt: HashedComputePluginSharingPolicyCapabilityRevocationReceipt,
    root_lock: ComputePluginRootLockLease,
) -> DurableComputePluginSharingPolicyBinding {
    DurableComputePluginSharingPolicyBinding {
        intent,
        receipt,
        revocation_receipt,
        _root_lock: root_lock,
    }
}

fn recovery(
    intent: ComputePluginLocalPolicyBindingIntent,
    key: ComputePluginSharingPolicyBindingRecoveryKey,
    error: anyhow::Error,
    root_lock: ComputePluginRootLockLease,
) -> ComputePluginSharingPolicyBindingStoreResult {
    ComputePluginSharingPolicyBindingStoreResult::Recovery(
        ComputePluginSharingPolicyBindingRecovery {
            intent,
            key,
            error,
            root_lock,
        },
    )
}

fn retained(
    recovery: ComputePluginSharingPolicyBindingRecovery,
) -> ComputePluginSharingPolicyBindingRecoveryOutcome {
    ComputePluginSharingPolicyBindingRecoveryOutcome::Retained(recovery)
}

fn recovery_error(code: &'static str) -> Result<()> {
    Err(anyhow::anyhow!(code))
}
