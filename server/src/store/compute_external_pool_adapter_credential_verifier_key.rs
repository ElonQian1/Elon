//! Immutable V242 signing keys bound to exact V241 credential-verifier implementations.

mod read;
mod types;
mod write;

pub(in crate::store) use read::current_credential_verifier_key_authority_on;
pub(in crate::store) use types::CurrentCredentialVerifierKeyAuthority;
pub(crate) use types::{
    CredentialVerifierKeyCurrentnessReceipt, CredentialVerifierKeyRegistrationWriteReceipt,
    CredentialVerifierKeyRevocationWriteReceipt, RegisterCredentialVerifierKey,
    RevokeCredentialVerifierKey,
};
