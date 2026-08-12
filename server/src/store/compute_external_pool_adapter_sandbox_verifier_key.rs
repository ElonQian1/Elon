//! Immutable V237 sandbox-verifier roots, four-eyes activation, and revocation.

mod read;
mod types;
mod write;

pub(in crate::store) use read::{
    current_sandbox_verifier_key_authority_on, sandbox_verifier_key_record_authority_on,
};
pub(crate) use types::{
    ActivateExternalPoolAdapterSandboxVerifierKey,
    ExternalPoolAdapterSandboxVerifierKeyCurrentnessReceipt,
    ExternalPoolAdapterSandboxVerifierKeyRegistrationWriteReceipt,
    ExternalPoolAdapterSandboxVerifierKeyTransitionWriteReceipt,
    RegisterExternalPoolAdapterSandboxVerifierKey, RevokeExternalPoolAdapterSandboxVerifierKey,
};
