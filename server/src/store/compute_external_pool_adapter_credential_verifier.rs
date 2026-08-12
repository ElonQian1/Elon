//! Immutable V241 credential-verifier roots, four-eyes activation, and revocation.

mod read;
mod types;
mod write;

pub(crate) use types::{
    ActivateExternalPoolAdapterCredentialVerifier,
    ExternalPoolAdapterCredentialVerifierCurrentnessReceipt,
    ExternalPoolAdapterCredentialVerifierRegistrationWriteReceipt,
    ExternalPoolAdapterCredentialVerifierTransitionWriteReceipt,
    RegisterExternalPoolAdapterCredentialVerifier, RevokeExternalPoolAdapterCredentialVerifier,
};
