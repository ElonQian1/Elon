use std::path::Path;

use thiserror::Error;

use super::PreparedExternalPoolAdapterInstallation;

mod audit;
mod extract;
mod paths;

pub(crate) use audit::audit_external_pool_adapter_installation;
pub(crate) use extract::prepare_external_pool_adapter_installation;

#[derive(Debug, Error)]
pub(crate) enum ExternalPoolAdapterInstallationFsError {
    #[error("external-pool Adapter installation authority is incompatible")]
    Authority(#[source] anyhow::Error),
    #[error("external-pool Adapter installation content address is invalid")]
    InvalidContentAddress,
    #[error("external-pool Adapter installation package is malformed or drifted")]
    Package(#[source] anyhow::Error),
    #[error("external-pool Adapter installation target is missing")]
    Missing,
    #[error("external-pool Adapter installation target is not a safe inert tree")]
    UnsafeTarget,
    #[error("external-pool Adapter installed bytes drifted from their exact receipt")]
    ContentDrift,
    #[error("external-pool Adapter installation storage failed")]
    Storage(#[source] std::io::Error),
}

pub(super) fn with_storage_context(
    error: ExternalPoolAdapterInstallationFsError,
    operation: impl std::fmt::Display,
) -> ExternalPoolAdapterInstallationFsError {
    match error {
        ExternalPoolAdapterInstallationFsError::Storage(source) => {
            ExternalPoolAdapterInstallationFsError::Storage(std::io::Error::new(
                source.kind(),
                format!("{operation}: {source}"),
            ))
        }
        other => other,
    }
}

impl PreparedExternalPoolAdapterInstallation {
    /// Reopens the final tree instead of trusting retained metadata or path strings.
    pub(crate) fn audit_current(
        self,
        data_dir: &Path,
    ) -> Result<Self, ExternalPoolAdapterInstallationFsError> {
        audit_external_pool_adapter_installation(data_dir, self.binding)
    }
}
