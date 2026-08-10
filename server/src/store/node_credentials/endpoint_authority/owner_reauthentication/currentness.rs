use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::node_compute_sharing::endpoint_authority::{
    derive_owner_account_auth_state_digest, derive_owner_account_session_binding_digest,
    derive_owner_google_factor_binding_digest, derive_owner_password_factor_binding_digest,
    PreparedNodeEndpointOwnerReauthentication,
};

struct CurrentAccountSource {
    owner_user_id: String,
    token_hash: String,
    session_created_at: String,
    session_expires_at: String,
    session_revoked_at: Option<String>,
    role: String,
    user_status: String,
    password_login_enabled: bool,
    password_hash: String,
    password_changed_at: Option<String>,
    user_updated_at: String,
}

pub(super) fn require_current_sources_on(
    connection: &Connection,
    prepared: &PreparedNodeEndpointOwnerReauthentication,
    checked_at: DateTime<Utc>,
) -> Result<()> {
    let envelope = prepared.envelope();
    let source = account_source_on(connection, envelope.account_session_id())?
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_OWNER_ACCOUNT_SESSION_MISSING"))?;
    let expires_at = DateTime::parse_from_rfc3339(&source.session_expires_at)
        .map_err(|_| anyhow::anyhow!("NODE_ENDPOINT_OWNER_ACCOUNT_SESSION_EXPIRY_INVALID"))?
        .with_timezone(&Utc);
    if source.owner_user_id != envelope.owner_user_id()
        || source.user_status != "active"
        || source.session_revoked_at.is_some()
        || checked_at >= expires_at
    {
        bail!("NODE_ENDPOINT_OWNER_ACCOUNT_SESSION_NOT_CURRENT");
    }
    let session_digest = derive_owner_account_session_binding_digest(
        envelope.account_session_id(),
        &source.owner_user_id,
        &source.token_hash,
        &source.session_created_at,
        &source.session_expires_at,
    )?;
    let auth_state_digest = derive_owner_account_auth_state_digest(
        &source.owner_user_id,
        &source.role,
        &source.user_status,
        source.password_login_enabled,
        source.password_changed_at.as_deref(),
        &source.user_updated_at,
    )?;
    if session_digest != envelope.session_binding_digest()
        || auth_state_digest != envelope.account_auth_state_digest()
    {
        bail!("NODE_ENDPOINT_OWNER_ACCOUNT_SOURCE_DIGEST_MISMATCH");
    }
    require_factor_on(connection, envelope, &source)?;
    require_target_on(connection, envelope)?;
    Ok(())
}

fn account_source_on(
    connection: &Connection,
    account_session_id: &str,
) -> Result<Option<CurrentAccountSource>> {
    connection
        .query_row(
            "SELECT s.user_id, s.token_hash, s.created_at, s.expires_at, s.revoked_at,
                    u.role, u.status, u.password_login_enabled, u.password_hash,
                    u.password_changed_at, u.updated_at
               FROM sessions s JOIN users u ON u.id=s.user_id WHERE s.id=?1",
            params![account_session_id],
            |row| {
                Ok(CurrentAccountSource {
                    owner_user_id: row.get(0)?,
                    token_hash: row.get(1)?,
                    session_created_at: row.get(2)?,
                    session_expires_at: row.get(3)?,
                    session_revoked_at: row.get(4)?,
                    role: row.get(5)?,
                    user_status: row.get(6)?,
                    password_login_enabled: row.get::<_, i64>(7)? == 1,
                    password_hash: row.get(8)?,
                    password_changed_at: row.get(9)?,
                    user_updated_at: row.get(10)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn require_factor_on(
    connection: &Connection,
    envelope: &crate::node_compute_sharing::endpoint_authority::NodeEndpointOwnerReauthenticationEnvelope,
    source: &CurrentAccountSource,
) -> Result<()> {
    let expected = match envelope.authentication_method() {
        "password" => {
            if envelope.authentication_factor_id() != "password" || !source.password_login_enabled {
                bail!("NODE_ENDPOINT_OWNER_PASSWORD_FACTOR_NOT_CURRENT");
            }
            derive_owner_password_factor_binding_digest(
                envelope.owner_user_id(),
                &source.password_hash,
                source.password_changed_at.as_deref(),
            )?
        }
        "google_oidc" => {
            let identity = connection
                .query_row(
                    "SELECT user_id, provider, issuer, subject, created_at
                       FROM user_identities WHERE id=?1",
                    params![envelope.authentication_factor_id()],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, String>(4)?,
                        ))
                    },
                )
                .optional()?
                .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_OWNER_GOOGLE_FACTOR_MISSING"))?;
            if identity.0 != envelope.owner_user_id() || identity.1 != "google" {
                bail!("NODE_ENDPOINT_OWNER_GOOGLE_FACTOR_NOT_CURRENT");
            }
            derive_owner_google_factor_binding_digest(
                envelope.authentication_factor_id(),
                &identity.0,
                &identity.1,
                &identity.2,
                &identity.3,
                &identity.4,
            )?
        }
        _ => bail!("NODE_ENDPOINT_OWNER_REAUTHENTICATION_METHOD_INVALID"),
    };
    if expected != envelope.authentication_factor_binding_digest() {
        bail!("NODE_ENDPOINT_OWNER_FACTOR_BINDING_MISMATCH");
    }
    Ok(())
}

fn require_target_on(
    connection: &Connection,
    envelope: &crate::node_compute_sharing::endpoint_authority::NodeEndpointOwnerReauthenticationEnvelope,
) -> Result<()> {
    let legacy = connection
        .query_row(
            "SELECT owner_user_id, install_id FROM node_credentials WHERE agent_id=?1",
            params![envelope.agent_id()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .optional()?;
    if !legacy.is_some_and(|value| {
        value.0 == envelope.owner_user_id() && value.1.as_deref() == Some(envelope.install_id())
    }) {
        bail!("NODE_ENDPOINT_OWNER_TARGET_LEGACY_BINDING_MISMATCH");
    }
    let root = connection
        .query_row(
            "SELECT credential_id, current_credential_revision, current_credential_digest,
                    status, owner_user_id, install_id
               FROM node_endpoint_credentials WHERE agent_id=?1",
            params![envelope.agent_id()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()?;
    let owner_install_root_exists = connection.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM node_endpoint_credentials
             WHERE owner_user_id=?1 AND install_id=?2
         )",
        params![envelope.owner_user_id(), envelope.install_id()],
        |row| row.get::<_, bool>(0),
    )?;
    match (envelope.authorization_action(), root) {
        ("initial_registration", None) if !owner_install_root_exists => Ok(()),
        (action, Some(value))
            if value.0 == envelope.expected_credential_id().unwrap_or_default()
                && u64::try_from(value.1)?
                    == envelope.expected_credential_revision().unwrap_or_default()
                && value.2 == envelope.expected_credential_digest().unwrap_or_default()
                && value.4 == envelope.owner_user_id()
                && value.5 == envelope.install_id()
                && ((action == "account_recovery"
                    && matches!(value.3.as_str(), "active" | "revoked"))
                    || (matches!(action, "credential_rotation" | "owner_revocation")
                        && value.3 == "active")) =>
        {
            Ok(())
        }
        _ => bail!("NODE_ENDPOINT_OWNER_TARGET_CREDENTIAL_CURRENTNESS_MISMATCH"),
    }
}
