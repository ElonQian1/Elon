//! Immutable V246 installed-byte authorities for external-pool Adapters.

mod current;
mod persistence;
mod read;
mod targets;
mod terminal;
mod types;
mod write;

pub(in crate::store) use current::{
    current_external_pool_adapter_installation_authority_on,
    external_pool_adapter_installation_receipt_authority_on,
};
pub(in crate::store) use types::{
    CurrentExternalPoolAdapterInstallationAuthority,
    HistoricalExternalPoolAdapterInstallationAuthority,
};
pub(crate) use types::{
    ExternalPoolAdapterInstallationCurrentness, ExternalPoolAdapterInstallationTerminalSummary,
    ExternalPoolAdapterInstallationTerminalWriteReceipt,
    ExternalPoolAdapterInstallationWriteReceipt, InstallExternalPoolAdapter,
    RevokeExternalPoolAdapterInstallation,
};
