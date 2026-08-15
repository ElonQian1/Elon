//! Startup-owned delegated cgroup custody for the V269 administrator signing handoff.

use std::{path::PathBuf, sync::Arc, sync::OnceLock};

use anyhow::{anyhow, bail, Result};
use thiserror::Error;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use super::external_pool_adapter_linux_supervisor::ExternalPoolAdapterSupervisorCgroupParent;

const ENABLED_ENV: &str =
    "ELON_EXTERNAL_POOL_ADAPTER_RUNTIME_COMPATIBILITY_SIGNING_HANDOFF_ENABLED";
const CGROUP_PARENT_PATH_ENV: &str =
    "ELON_EXTERNAL_POOL_ADAPTER_RUNTIME_COMPATIBILITY_CGROUP_PARENT_PATH";

static SIGNING_HANDOFF_RUNTIME: OnceLock<
    Option<Arc<ExternalPoolAdapterRuntimeCompatibilitySigningHandoffRuntime>>,
> = OnceLock::new();

pub(crate) struct ExternalPoolAdapterRuntimeCompatibilitySigningHandoffRuntime {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    cgroup_parent: ExternalPoolAdapterSupervisorCgroupParent,
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl ExternalPoolAdapterRuntimeCompatibilitySigningHandoffRuntime {
    pub(crate) fn cgroup_parent(&self) -> &ExternalPoolAdapterSupervisorCgroupParent {
        &self.cgroup_parent
    }
}

#[derive(Debug, Error)]
#[error("external-pool Adapter runtime compatibility signing handoff is unavailable")]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilitySigningHandoffUnavailable;

pub(crate) fn initialize_external_pool_adapter_runtime_compatibility_signing_handoff_runtime(
) -> Result<()> {
    let runtime = configured_runtime()?;
    SIGNING_HANDOFF_RUNTIME
        .set(runtime)
        .map_err(|_| anyhow!("runtime compatibility signing handoff initialized more than once"))
}

pub(crate) fn external_pool_adapter_runtime_compatibility_signing_handoff_runtime() -> Result<
    Arc<ExternalPoolAdapterRuntimeCompatibilitySigningHandoffRuntime>,
    ExternalPoolAdapterRuntimeCompatibilitySigningHandoffUnavailable,
> {
    SIGNING_HANDOFF_RUNTIME
        .get()
        .and_then(Option::as_ref)
        .map(Arc::clone)
        .ok_or(ExternalPoolAdapterRuntimeCompatibilitySigningHandoffUnavailable)
}

fn configured_runtime(
) -> Result<Option<Arc<ExternalPoolAdapterRuntimeCompatibilitySigningHandoffRuntime>>> {
    let enabled = match std::env::var_os(ENABLED_ENV) {
        None => false,
        Some(value) => match value.to_str() {
            Some("true") => true,
            Some("false") => false,
            _ => bail!("runtime compatibility signing handoff enabled value is invalid"),
        },
    };
    let path = std::env::var_os(CGROUP_PARENT_PATH_ENV);
    if !enabled {
        if path.is_some() {
            bail!("disabled runtime compatibility signing handoff has a cgroup path");
        }
        return Ok(None);
    }
    let path = path
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| {
            anyhow!("enabled runtime compatibility signing handoff lacks a cgroup path")
        })?;
    if !path.is_absolute() {
        bail!("runtime compatibility signing handoff cgroup path is not absolute");
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        let cgroup_parent =
            ExternalPoolAdapterSupervisorCgroupParent::from_operator_delegated_path(&path)?;
        return Ok(Some(Arc::new(
            ExternalPoolAdapterRuntimeCompatibilitySigningHandoffRuntime { cgroup_parent },
        )));
    }

    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    {
        let _ = path;
        bail!("runtime compatibility signing handoff requires Linux x86-64");
    }
}
