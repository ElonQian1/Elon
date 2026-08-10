//! Target-bound, recent owner reauthentication facts for future endpoint credential mutations.
//!
//! There is deliberately no constructor wired to account login, HTTP, Google federation, or the
//! legacy owner token. Only a future secure owner-API verifier may assemble the three sealed inputs
//! and the authorized request held by this module.

mod contracts;
mod digests;
mod direct_tls;

pub(crate) use contracts::{
    authorize_password_owner_reauthentication, AuthorizedNodeEndpointOwnerReauthentication,
    NodeEndpointOwnerReauthenticationEnvelope, PreparedNodeEndpointOwnerReauthentication,
    VerifiedCurrentAccountSession, VerifiedRecentOwnerReauthentication,
};

pub(crate) use direct_tls::{
    bind_direct_tls_owner_api_transport, OwnerApiResponsePermit, VerifiedSecureOwnerApiTransport,
};

pub(crate) use digests::{
    derive_owner_account_auth_state_digest, derive_owner_account_session_binding_digest,
    derive_owner_google_factor_binding_digest, derive_owner_password_factor_binding_digest,
};
