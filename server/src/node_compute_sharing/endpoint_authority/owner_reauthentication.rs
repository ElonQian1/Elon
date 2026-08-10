//! Target-bound, recent owner reauthentication facts for future endpoint credential mutations.
//!
//! There is deliberately no constructor wired to account login, HTTP, Google federation, or the
//! legacy owner token. Only a future secure owner-API verifier may assemble the three sealed inputs
//! and the authorized request held by this module.

mod contracts;
mod digests;

pub(crate) use contracts::{
    AuthorizedNodeEndpointOwnerReauthentication, NodeEndpointOwnerReauthenticationEnvelope,
    PreparedNodeEndpointOwnerReauthentication, VerifiedCurrentAccountSession,
    VerifiedRecentOwnerReauthentication, VerifiedSecureOwnerApiTransport,
};

pub(crate) use digests::{
    derive_owner_account_auth_state_digest, derive_owner_account_session_binding_digest,
    derive_owner_google_factor_binding_digest, derive_owner_password_factor_binding_digest,
};
