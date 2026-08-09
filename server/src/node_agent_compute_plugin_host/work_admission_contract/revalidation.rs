use std::{error::Error as StdError, fmt, time::Instant};

use anyhow::Error;

use crate::node_agent_compute_plugin_host::{
    candidate_promotion_contract::DurableInstalledPluginSlot,
    trusted_time::ComputePluginTrustedTimeObservation,
};

use super::{
    InstalledWorkAdmissionRevalidationCustody, PendingInstalledWorkAdmissionRevalidation,
    RevalidatedInstalledWorkAdmission,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum InstalledWorkAdmissionRevalidationPhase {
    RetainedInstalledContent,
    PostRevalidationTrustedTime,
}

pub(in crate::node_agent_compute_plugin_host) struct InstalledWorkAdmissionRevalidationFailure<
    'root,
> {
    phase: InstalledWorkAdmissionRevalidationPhase,
    error: Error,
    custody: InstalledWorkAdmissionRevalidationCustody<'root>,
}

/// Must run before the successor `reauthorize_existing` PlanApply closes the candidate source.
/// The pending/revalidated custody remains linear across PlanApply; PlanApply never consumes it.
pub(in crate::node_agent_compute_plugin_host) fn begin_installed_work_admission_revalidation<
    'root,
>(
    mut installed: DurableInstalledPluginSlot<'root>,
) -> Result<
    PendingInstalledWorkAdmissionRevalidation<'root>,
    InstalledWorkAdmissionRevalidationFailure<'root>,
> {
    match installed.fresh_revalidate_initial_work_content() {
        Ok(at) => Ok(PendingInstalledWorkAdmissionRevalidation::new(
            installed, at,
        )),
        Err(error) => Err(InstalledWorkAdmissionRevalidationFailure::new(
            InstalledWorkAdmissionRevalidationPhase::RetainedInstalledContent,
            error,
            InstalledWorkAdmissionRevalidationCustody::Installed(installed),
        )),
    }
}

pub(in crate::node_agent_compute_plugin_host) fn complete_installed_work_admission_revalidation<
    'root,
>(
    pending: PendingInstalledWorkAdmissionRevalidation<'root>,
    trusted_time: ComputePluginTrustedTimeObservation,
) -> Result<
    RevalidatedInstalledWorkAdmission<'root>,
    InstalledWorkAdmissionRevalidationFailure<'root>,
> {
    let (installed, barrier) = pending.into_parts();
    let install = installed.receipts().install().receipt();
    let valid = trusted_time.observed_at() > barrier
        && trusted_time.installation_id_digest() == install.installation_id_digest()
        && trusted_time.ensure_live(Instant::now()).is_ok();
    if !valid {
        return Err(InstalledWorkAdmissionRevalidationFailure::new(
            InstalledWorkAdmissionRevalidationPhase::PostRevalidationTrustedTime,
            anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_TRUSTED_TIME_NOT_POST_REVALIDATION"),
            InstalledWorkAdmissionRevalidationCustody::Pending(
                PendingInstalledWorkAdmissionRevalidation::new(installed, barrier),
            ),
        ));
    }
    Ok(RevalidatedInstalledWorkAdmission::new(
        installed,
        trusted_time,
        barrier,
    ))
}

impl<'root> InstalledWorkAdmissionRevalidationFailure<'root> {
    fn new(
        phase: InstalledWorkAdmissionRevalidationPhase,
        error: Error,
        custody: InstalledWorkAdmissionRevalidationCustody<'root>,
    ) -> Self {
        Self {
            phase,
            error,
            custody,
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> InstalledWorkAdmissionRevalidationPhase {
        self.phase
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, InstalledWorkAdmissionRevalidationCustody<'root>) {
        (self.error, self.custody)
    }
}

impl fmt::Display for InstalledWorkAdmissionRevalidationFailure<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#}", self.error)
    }
}

impl fmt::Debug for InstalledWorkAdmissionRevalidationFailure<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InstalledWorkAdmissionRevalidationFailure")
            .field("phase", &self.phase)
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl StdError for InstalledWorkAdmissionRevalidationFailure<'_> {}
