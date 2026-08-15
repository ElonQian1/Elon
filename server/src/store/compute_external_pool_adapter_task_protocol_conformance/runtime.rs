//! Independent startup-owned custody for V272 server-run task conformance.

use std::{path::PathBuf, sync::Arc, sync::OnceLock};

use anyhow::{anyhow, bail, Result};
use thiserror::Error;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use crate::compute_federation::external_pool_adapter_linux_supervisor::ExternalPoolAdapterSupervisorCgroupParent;
use crate::store::compute_external_pool_adapter_runtime_bundle::ExternalPoolAdapterProviderRuntimeReadinessProcessCustody;

const ENABLED_ENV: &str = "ELON_EXTERNAL_POOL_ADAPTER_TASK_PROTOCOL_CONFORMANCE_ENABLED";
const CGROUP_PARENT_PATH_ENV: &str =
    "ELON_EXTERNAL_POOL_ADAPTER_TASK_PROTOCOL_CONFORMANCE_CGROUP_PARENT_PATH";

static TASK_PROTOCOL_CONFORMANCE_RUNTIME: OnceLock<
    Option<Arc<ExternalPoolAdapterTaskProtocolConformanceRuntime>>,
> = OnceLock::new();

pub(crate) struct ExternalPoolAdapterTaskProtocolConformanceRuntime {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    cgroup_parent: ExternalPoolAdapterSupervisorCgroupParent,
    process_custody: ExternalPoolAdapterProviderRuntimeReadinessProcessCustody,
}

#[derive(Debug, Error)]
#[error("external-pool Adapter task protocol conformance is unavailable")]
pub(crate) struct ExternalPoolAdapterTaskProtocolConformanceUnavailable;

pub(crate) fn initialize_external_pool_adapter_task_protocol_conformance_runtime() -> Result<()> {
    let runtime = configured_runtime()?;
    TASK_PROTOCOL_CONFORMANCE_RUNTIME
        .set(runtime)
        .map_err(|_| anyhow!("task protocol conformance runtime initialized more than once"))
}

pub(crate) fn external_pool_adapter_task_protocol_conformance_runtime() -> std::result::Result<
    Arc<ExternalPoolAdapterTaskProtocolConformanceRuntime>,
    ExternalPoolAdapterTaskProtocolConformanceUnavailable,
> {
    TASK_PROTOCOL_CONFORMANCE_RUNTIME
        .get()
        .and_then(Option::as_ref)
        .map(Arc::clone)
        .ok_or(ExternalPoolAdapterTaskProtocolConformanceUnavailable)
}

impl ExternalPoolAdapterTaskProtocolConformanceRuntime {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub(in crate::store) fn cgroup_parent(&self) -> &ExternalPoolAdapterSupervisorCgroupParent {
        &self.cgroup_parent
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

fn configured_runtime() -> Result<Option<Arc<ExternalPoolAdapterTaskProtocolConformanceRuntime>>> {
    let enabled = match std::env::var_os(ENABLED_ENV) {
        None => false,
        Some(value) => match value.to_str() {
            Some("true") => true,
            Some("false") => false,
            _ => bail!("task protocol conformance enabled value is invalid"),
        },
    };
    let cgroup_path = std::env::var_os(CGROUP_PARENT_PATH_ENV);
    if !enabled {
        if cgroup_path.is_some() {
            bail!("disabled task protocol conformance has a cgroup custody path");
        }
        return Ok(None);
    }
    let cgroup_path = required_absolute_path(cgroup_path)?;

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        let cgroup_parent =
            ExternalPoolAdapterSupervisorCgroupParent::from_operator_delegated_path(&cgroup_path)?;
        let process_custody = ExternalPoolAdapterProviderRuntimeReadinessProcessCustody::
            generate_task_protocol_conformance()?;
        return Ok(Some(Arc::new(
            ExternalPoolAdapterTaskProtocolConformanceRuntime {
                cgroup_parent,
                process_custody,
            },
        )));
    }

    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    {
        let _ = cgroup_path;
        bail!("task protocol conformance requires Linux x86-64");
    }
}

fn required_absolute_path(value: Option<std::ffi::OsString>) -> Result<PathBuf> {
    let path = value
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("enabled task protocol conformance lacks its cgroup path"))?;
    if !path.is_absolute() {
        bail!("task protocol conformance cgroup path is not absolute");
    }
    Ok(path)
}
