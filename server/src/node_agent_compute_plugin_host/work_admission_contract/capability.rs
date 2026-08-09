use std::{fmt, time::Instant};

use crate::node_agent_compute_plugin_host::{
    candidate_promotion_contract::DurableInstalledPluginSlot,
    trusted_time::ComputePluginTrustedTimeObservation,
};

use super::ComputePluginWorkAdmissionReceiptPair;

#[must_use = "pending work admission revalidation needs post-rehash trusted time"]
pub(in crate::node_agent_compute_plugin_host) struct PendingInstalledWorkAdmissionRevalidation<
    'root,
> {
    installed: DurableInstalledPluginSlot<'root>,
    revalidated_at: Instant,
}

#[must_use = "revalidated installed custody must be authorized or retained"]
pub(in crate::node_agent_compute_plugin_host) struct RevalidatedInstalledWorkAdmission<'root> {
    installed: DurableInstalledPluginSlot<'root>,
    trusted_time: ComputePluginTrustedTimeObservation,
    revalidated_at: Instant,
}

pub(in crate::node_agent_compute_plugin_host) enum InstalledWorkAdmissionRevalidationCustody<'root>
{
    Installed(DurableInstalledPluginSlot<'root>),
    Pending(PendingInstalledWorkAdmissionRevalidation<'root>),
}

/// Durable authorization to launch only under the sealed source ceilings. This is not proof of a
/// running process, health, endpoint, Ready capability, session, attempt, or completed work.
#[must_use = "work-admitted installed custody must be retained for a later runtime contract"]
pub(in crate::node_agent_compute_plugin_host) struct DurableWorkAdmittedPluginSlot<'root> {
    revalidated: RevalidatedInstalledWorkAdmission<'root>,
    receipts: ComputePluginWorkAdmissionReceiptPair,
}

impl<'root> PendingInstalledWorkAdmissionRevalidation<'root> {
    pub(super) fn new(installed: DurableInstalledPluginSlot<'root>, at: Instant) -> Self {
        Self {
            installed,
            revalidated_at: at,
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn installed(
        &self,
    ) -> &DurableInstalledPluginSlot<'root> {
        &self.installed
    }

    pub(in crate::node_agent_compute_plugin_host) fn revalidated_at(&self) -> Instant {
        self.revalidated_at
    }

    pub(super) fn into_parts(self) -> (DurableInstalledPluginSlot<'root>, Instant) {
        (self.installed, self.revalidated_at)
    }
}

impl RevalidatedInstalledWorkAdmission<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn installed(
        &self,
    ) -> &DurableInstalledPluginSlot<'_> {
        &self.installed
    }

    pub(in crate::node_agent_compute_plugin_host) fn trusted_time(
        &self,
    ) -> &ComputePluginTrustedTimeObservation {
        &self.trusted_time
    }

    pub(in crate::node_agent_compute_plugin_host) fn revalidated_at(&self) -> Instant {
        self.revalidated_at
    }
}

impl<'root> RevalidatedInstalledWorkAdmission<'root> {
    pub(super) fn new(
        installed: DurableInstalledPluginSlot<'root>,
        trusted_time: ComputePluginTrustedTimeObservation,
        revalidated_at: Instant,
    ) -> Self {
        Self {
            installed,
            trusted_time,
            revalidated_at,
        }
    }

    pub(super) fn fresh_revalidate_for_recovery(&mut self) -> anyhow::Result<Instant> {
        self.installed.fresh_revalidate_installed_content()
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_installed(
        self,
    ) -> DurableInstalledPluginSlot<'root> {
        self.installed
    }
}

impl<'root> DurableWorkAdmittedPluginSlot<'root> {
    pub(super) fn new(
        revalidated: RevalidatedInstalledWorkAdmission<'root>,
        receipts: ComputePluginWorkAdmissionReceiptPair,
    ) -> Self {
        Self {
            revalidated,
            receipts,
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn installed(
        &self,
    ) -> &DurableInstalledPluginSlot<'root> {
        self.revalidated.installed()
    }

    pub(in crate::node_agent_compute_plugin_host) fn receipts(
        &self,
    ) -> &ComputePluginWorkAdmissionReceiptPair {
        &self.receipts
    }
}

impl fmt::Debug for PendingInstalledWorkAdmissionRevalidation<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PendingInstalledWorkAdmissionRevalidation")
            .field("installed", &self.installed)
            .field("revalidated_at", &"<monotonic>")
            .finish()
    }
}

impl fmt::Debug for RevalidatedInstalledWorkAdmission<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RevalidatedInstalledWorkAdmission")
            .field("installed", &self.installed)
            .field("trusted_time", &"<authenticated>")
            .field("revalidated_at", &"<monotonic>")
            .finish()
    }
}

impl fmt::Debug for InstalledWorkAdmissionRevalidationCustody<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Installed(_) => f.write_str("Installed(<retained-handles>)"),
            Self::Pending(_) => f.write_str("Pending(<retained-handles>)"),
        }
    }
}

impl fmt::Debug for DurableWorkAdmittedPluginSlot<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DurableWorkAdmittedPluginSlot")
            .field("receipts", &self.receipts)
            .field("runtime", &"<not-started>")
            .finish()
    }
}
