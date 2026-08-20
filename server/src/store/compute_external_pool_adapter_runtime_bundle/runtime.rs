//! Startup-owned production custody for Provider-specific runtime readiness.

mod custody;

use std::{
    path::PathBuf,
    sync::{Arc, OnceLock},
};

use anyhow::{anyhow, bail, Result};
use thiserror::Error;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use crate::compute_federation::external_pool_adapter_linux_supervisor::ExternalPoolAdapterSupervisorCgroupParent;

use super::types::ExternalPoolAdapterRuntimeBundleRoot;

pub(crate) use custody::register_external_pool_adapter_atomic_activation_pending_plan_udf;
pub(crate) use custody::register_external_pool_adapter_provider_active_successor_refresh_pending_plan_udf;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(in crate::store) use custody::ExternalPoolAdapterPostCleanupCommitmentInput;
pub(in crate::store) use custody::{
    install_external_pool_adapter_atomic_activation_pending_plan_on,
    install_external_pool_adapter_provider_active_successor_refresh_pending_plan_on,
    ExternalPoolAdapterAtomicActivationPendingPlan,
    ExternalPoolAdapterAtomicActivationPendingPlanGuard,
    ExternalPoolAdapterAtomicActivationPendingWrite,
    ExternalPoolAdapterAtomicActivationPendingWriteKind,
    ExternalPoolAdapterProviderActiveSuccessorProcessSeal,
    ExternalPoolAdapterProviderActiveSuccessorProcessSealInput,
    ExternalPoolAdapterProviderActiveSuccessorRefreshPendingPlan,
    ExternalPoolAdapterProviderActiveSuccessorRefreshPendingPlanGuard,
    ExternalPoolAdapterProviderActiveSuccessorRefreshPendingPlanInput,
    ExternalPoolAdapterProviderRuntimeReadinessProcessCustody,
    ExternalPoolAdapterTaskProtocolConformanceSealInput, TaskProtocolConformanceProcessSeal,
};

const ENABLED_ENV: &str = "ELON_EXTERNAL_POOL_ADAPTER_PROVIDER_RUNTIME_READINESS_ENABLED";
const CGROUP_PARENT_PATH_ENV: &str =
    "ELON_EXTERNAL_POOL_ADAPTER_PROVIDER_RUNTIME_READINESS_CGROUP_PARENT_PATH";
const BUNDLE_ROOT_PATH_ENV: &str =
    "ELON_EXTERNAL_POOL_ADAPTER_PROVIDER_RUNTIME_READINESS_BUNDLE_ROOT_PATH";

static PROVIDER_RUNTIME_READINESS_RUNTIME: OnceLock<
    Option<Arc<ExternalPoolAdapterProviderRuntimeReadinessRuntime>>,
> = OnceLock::new();

/// Process-private startup custody. It exposes no path, descriptor, key, epoch, or generic MAC.
pub(crate) struct ExternalPoolAdapterProviderRuntimeReadinessRuntime {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    cgroup_parent: ExternalPoolAdapterSupervisorCgroupParent,
    bundle_root: ExternalPoolAdapterRuntimeBundleRoot,
    process_custody: ExternalPoolAdapterProviderRuntimeReadinessProcessCustody,
}

#[derive(Debug, Error)]
#[error("external-pool Adapter Provider runtime readiness is unavailable")]
pub(crate) struct ExternalPoolAdapterProviderRuntimeReadinessUnavailable;

pub(crate) fn initialize_external_pool_adapter_provider_runtime_readiness_runtime() -> Result<()> {
    let runtime = configured_runtime()?;
    PROVIDER_RUNTIME_READINESS_RUNTIME
        .set(runtime)
        .map_err(|_| anyhow!("Provider runtime readiness initialized more than once"))
}

pub(crate) fn external_pool_adapter_provider_runtime_readiness_runtime() -> std::result::Result<
    Arc<ExternalPoolAdapterProviderRuntimeReadinessRuntime>,
    ExternalPoolAdapterProviderRuntimeReadinessUnavailable,
> {
    PROVIDER_RUNTIME_READINESS_RUNTIME
        .get()
        .and_then(Option::as_ref)
        .map(Arc::clone)
        .ok_or(ExternalPoolAdapterProviderRuntimeReadinessUnavailable)
}

/// SQLite's V274 verifier-only UDF calls this registry lookup. It never opens SQLite, mints a
/// seal, or turns a pending tuple into committed authority.
#[allow(clippy::too_many_arguments)]
pub(crate) fn verify_pending_external_pool_adapter_provider_active_successor_process_seal(
    kind: &str,
    entity_id: &str,
    entity_digest: &str,
    process_custody_epoch_digest: &str,
    process_custody_nonce_digest: &str,
    process_custody_seal_digest: &str,
    receipt_integrity_digest: &str,
) -> bool {
    external_pool_adapter_provider_runtime_readiness_runtime()
        .ok()
        .and_then(|runtime| {
            runtime
                .process_custody()
                .attests_pending_provider_active_successor_process_seal(
                    kind,
                    entity_id,
                    entity_digest,
                    process_custody_epoch_digest,
                    process_custody_nonce_digest,
                    process_custody_seal_digest,
                    receipt_integrity_digest,
                )
                .ok()
        })
        .unwrap_or(false)
}

impl ExternalPoolAdapterProviderRuntimeReadinessRuntime {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub(in crate::store) fn cgroup_parent(&self) -> &ExternalPoolAdapterSupervisorCgroupParent {
        &self.cgroup_parent
    }

    pub(in crate::store) fn bundle_root(&self) -> &ExternalPoolAdapterRuntimeBundleRoot {
        &self.bundle_root
    }

    pub(in crate::store) fn process_custody(
        &self,
    ) -> &ExternalPoolAdapterProviderRuntimeReadinessProcessCustody {
        &self.process_custody
    }

    pub(in crate::store) fn custody_epoch_digest(&self) -> &str {
        self.process_custody.custody_epoch_digest()
    }
}

fn configured_runtime() -> Result<Option<Arc<ExternalPoolAdapterProviderRuntimeReadinessRuntime>>> {
    let enabled = match std::env::var_os(ENABLED_ENV) {
        None => false,
        Some(value) => match value.to_str() {
            Some("true") => true,
            Some("false") => false,
            _ => bail!("Provider runtime readiness enabled value is invalid"),
        },
    };
    let cgroup_path = std::env::var_os(CGROUP_PARENT_PATH_ENV);
    let bundle_path = std::env::var_os(BUNDLE_ROOT_PATH_ENV);
    if !enabled {
        if cgroup_path.is_some() || bundle_path.is_some() {
            bail!("disabled Provider runtime readiness has a custody path");
        }
        return Ok(None);
    }
    let cgroup_path = required_absolute_path(cgroup_path, "cgroup")?;
    let bundle_path = required_absolute_path(bundle_path, "bundle root")?;

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        let cgroup_parent =
            ExternalPoolAdapterSupervisorCgroupParent::from_operator_delegated_path(&cgroup_path)?;
        let bundle_root = ExternalPoolAdapterRuntimeBundleRoot::new(bundle_path)
            .map_err(|_| anyhow!("Provider runtime readiness bundle root custody is unsafe"))?;
        let process_custody =
            ExternalPoolAdapterProviderRuntimeReadinessProcessCustody::generate()?;
        return Ok(Some(Arc::new(
            ExternalPoolAdapterProviderRuntimeReadinessRuntime {
                cgroup_parent,
                bundle_root,
                process_custody,
            },
        )));
    }

    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    {
        let _ = (cgroup_path, bundle_path);
        bail!("Provider runtime readiness requires Linux x86-64");
    }
}

fn required_absolute_path(value: Option<std::ffi::OsString>, label: &str) -> Result<PathBuf> {
    let path = value
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("enabled Provider runtime readiness lacks its {label} path"))?;
    if !path.is_absolute() {
        bail!("Provider runtime readiness {label} path is not absolute");
    }
    Ok(path)
}
