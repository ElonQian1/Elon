//! Immutable V235 scanner-key roots, four-eyes activation, revocation, and currentness.

mod read;
mod types;
mod write;

#[cfg(test)]
#[path = "compute_external_pool_adapter_scanner_key_tests.rs"]
mod tests;

pub(in crate::store) use read::{
    current_scanner_key_authority_on, scanner_key_record_authority_on,
};
pub(crate) use types::{
    ActivateExternalPoolAdapterScannerKey, ExternalPoolAdapterScannerKeyActivationWriteReceipt,
    ExternalPoolAdapterScannerKeyCurrentnessReceipt,
    ExternalPoolAdapterScannerKeyRegistrationWriteReceipt,
    ExternalPoolAdapterScannerKeyRevocationWriteReceipt, RegisterExternalPoolAdapterScannerKey,
    RevokeExternalPoolAdapterScannerKey,
};
