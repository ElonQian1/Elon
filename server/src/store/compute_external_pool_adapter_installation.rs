//! Immutable V246 installed-byte authorities for external-pool Adapters.

mod persistence;
mod read;
mod targets;
mod types;
mod write;

pub(crate) use types::{
    ExternalPoolAdapterInstallationCurrentness, ExternalPoolAdapterInstallationWriteReceipt,
    InstallExternalPoolAdapter,
};
