use anyhow::Result;
use serde::Serialize;

use super::super::canonical::canonical_domain_json_and_digest;

const ACCOUNT_SESSION_BINDING_DOMAIN: &[u8] =
    b"ELON_NODE_ENDPOINT_OWNER_ACCOUNT_SESSION_BINDING_V1";
const ACCOUNT_AUTH_STATE_DOMAIN: &[u8] = b"ELON_NODE_ENDPOINT_OWNER_ACCOUNT_AUTH_STATE_V1";
const PASSWORD_FACTOR_BINDING_DOMAIN: &[u8] =
    b"ELON_NODE_ENDPOINT_OWNER_PASSWORD_FACTOR_BINDING_V1";
const GOOGLE_FACTOR_BINDING_DOMAIN: &[u8] = b"ELON_NODE_ENDPOINT_OWNER_GOOGLE_FACTOR_BINDING_V1";

#[allow(clippy::too_many_arguments)]
pub(crate) fn derive_owner_account_session_binding_digest(
    account_session_id: &str,
    owner_user_id: &str,
    token_hash: &str,
    created_at: &str,
    expires_at: &str,
) -> Result<String> {
    #[derive(Serialize)]
    struct Binding<'a> {
        account_session_id: &'a str,
        owner_user_id: &'a str,
        token_hash: &'a str,
        created_at: &'a str,
        expires_at: &'a str,
    }
    canonical_domain_json_and_digest(
        ACCOUNT_SESSION_BINDING_DOMAIN,
        &Binding {
            account_session_id,
            owner_user_id,
            token_hash,
            created_at,
            expires_at,
        },
    )
    .map(|(_, digest)| digest)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn derive_owner_account_auth_state_digest(
    owner_user_id: &str,
    role: &str,
    status: &str,
    password_login_enabled: bool,
    password_changed_at: Option<&str>,
    updated_at: &str,
) -> Result<String> {
    #[derive(Serialize)]
    struct Binding<'a> {
        owner_user_id: &'a str,
        role: &'a str,
        status: &'a str,
        password_login_enabled: bool,
        password_changed_at: Option<&'a str>,
        updated_at: &'a str,
    }
    canonical_domain_json_and_digest(
        ACCOUNT_AUTH_STATE_DOMAIN,
        &Binding {
            owner_user_id,
            role,
            status,
            password_login_enabled,
            password_changed_at,
            updated_at,
        },
    )
    .map(|(_, digest)| digest)
}

pub(crate) fn derive_owner_password_factor_binding_digest(
    owner_user_id: &str,
    password_hash: &str,
    password_changed_at: Option<&str>,
) -> Result<String> {
    #[derive(Serialize)]
    struct Binding<'a> {
        owner_user_id: &'a str,
        password_hash: &'a str,
        password_changed_at: Option<&'a str>,
    }
    canonical_domain_json_and_digest(
        PASSWORD_FACTOR_BINDING_DOMAIN,
        &Binding {
            owner_user_id,
            password_hash,
            password_changed_at,
        },
    )
    .map(|(_, digest)| digest)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn derive_owner_google_factor_binding_digest(
    factor_id: &str,
    owner_user_id: &str,
    provider: &str,
    issuer: &str,
    subject: &str,
    created_at: &str,
) -> Result<String> {
    #[derive(Serialize)]
    struct Binding<'a> {
        factor_id: &'a str,
        owner_user_id: &'a str,
        provider: &'a str,
        issuer: &'a str,
        subject: &'a str,
        created_at: &'a str,
    }
    canonical_domain_json_and_digest(
        GOOGLE_FACTOR_BINDING_DOMAIN,
        &Binding {
            factor_id,
            owner_user_id,
            provider,
            issuer,
            subject,
            created_at,
        },
    )
    .map(|(_, digest)| digest)
}
