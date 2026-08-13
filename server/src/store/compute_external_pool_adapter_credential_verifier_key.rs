//! Immutable V242 signing keys bound to exact V241 credential-verifier implementations.

mod read;
mod types;
mod write;

pub(in crate::store) use read::{
    credential_verifier_key_is_current_exact_on, credential_verifier_key_record_authority_on,
    current_credential_verifier_key_authority_on,
};
pub(crate) use types::{
    CredentialVerifierKeyCurrentnessReceipt, CredentialVerifierKeyRegistrationWriteReceipt,
    CredentialVerifierKeyRevocationWriteReceipt, RegisterCredentialVerifierKey,
    RevokeCredentialVerifierKey,
};
pub(in crate::store) use types::{
    CredentialVerifierKeyRecordAuthority, CurrentCredentialVerifierKeyAuthority,
};
