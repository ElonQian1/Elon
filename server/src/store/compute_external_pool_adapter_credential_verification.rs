//! Immutable short-lived V243 signed credential verification receipts.

mod read;
mod types;
mod write;

pub(in crate::store) use read::current_external_pool_adapter_credential_verification_authority_on;
pub(in crate::store) use types::CurrentExternalPoolAdapterCredentialVerificationAuthority;
pub(crate) use types::{
    CreateExternalPoolAdapterCredentialVerification,
    ExternalPoolAdapterCredentialVerificationCurrentness,
    ExternalPoolAdapterCredentialVerificationWriteReceipt,
    GetExternalPoolAdapterCredentialVerificationChallenge,
};
