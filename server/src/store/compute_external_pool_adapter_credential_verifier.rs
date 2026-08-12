//! Immutable V241 credential-verifier roots, four-eyes activation, and revocation.

mod read;
mod types;
mod write;

pub(in crate::store) use read::{
    credential_verifier_is_current_exact_on, current_credential_verifier_authority_on,
};
pub(in crate::store) use types::CurrentExternalPoolAdapterCredentialVerifierAuthority;
pub(crate) use types::{
    ActivateExternalPoolAdapterCredentialVerifier,
    ExternalPoolAdapterCredentialVerifierCurrentnessReceipt,
    ExternalPoolAdapterCredentialVerifierRegistrationWriteReceipt,
    ExternalPoolAdapterCredentialVerifierTransitionWriteReceipt,
    RegisterExternalPoolAdapterCredentialVerifier, RevokeExternalPoolAdapterCredentialVerifier,
};
