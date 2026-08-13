use anyhow::{bail, Result};

use crate::compute_federation::external_pool_adapter_upstream_transport_target::{
    validate_upstream_transport_dns_hostname, UPSTREAM_TRANSPORT_TARGET_ACTOR_PLATFORM_ADMIN,
    UPSTREAM_TRANSPORT_TARGET_ACTOR_PROVIDER_OWNER, UPSTREAM_TRANSPORT_TARGET_CONFIRMATION,
    UPSTREAM_TRANSPORT_TARGET_REVOCATION_CONFIRMATION,
};

use super::types::{
    CreateExternalPoolAdapterUpstreamTransportTarget,
    RevokeExternalPoolAdapterUpstreamTransportTarget,
};

pub(super) fn validate_create_input(
    input: &CreateExternalPoolAdapterUpstreamTransportTarget,
) -> Result<()> {
    validate_actor_kind(&input.recorded_by_actor_kind)?;
    for value in [
        input.profile_id.as_str(),
        input.recorded_by_actor_user_id.as_str(),
        input.idempotency_scope.as_str(),
        input.idempotency_key.as_str(),
    ] {
        validate_identifier(value)?;
    }
    for value in [
        input.expected_profile_digest.as_str(),
        input.expected_candidate_digest.as_str(),
        input.expected_provider_binding_digest.as_str(),
        input.expected_target_policy_digest.as_str(),
        input.target.expected_tls_leaf_spki_sha256.as_str(),
    ] {
        validate_digest(value)?;
    }
    validate_upstream_transport_dns_hostname(&input.target.dns_hostname)?;
    if input.target.port == 0 {
        bail!("upstream transport target port is invalid");
    }
    if input.confirmation != UPSTREAM_TRANSPORT_TARGET_CONFIRMATION {
        bail!("upstream transport target confirmation is invalid");
    }
    if input.predecessor_target_id.is_some() != input.expected_predecessor_target_digest.is_some() {
        bail!("upstream transport target predecessor pair is incomplete");
    }
    if let Some(value) = &input.predecessor_target_id {
        validate_identifier(value)?;
    }
    if let Some(value) = &input.expected_predecessor_target_digest {
        validate_digest(value)?;
    }
    Ok(())
}

pub(super) fn validate_revoke_input(
    input: &RevokeExternalPoolAdapterUpstreamTransportTarget,
) -> Result<()> {
    validate_actor_kind(&input.revoked_by_actor_kind)?;
    for value in [
        input.target_id.as_str(),
        input.revoked_by_actor_user_id.as_str(),
        input.idempotency_scope.as_str(),
        input.idempotency_key.as_str(),
    ] {
        validate_identifier(value)?;
    }
    validate_digest(&input.expected_target_digest)?;
    validate_digest(&input.expected_profile_digest)?;
    if input.confirmation != UPSTREAM_TRANSPORT_TARGET_REVOCATION_CONFIRMATION {
        bail!("upstream transport target revocation confirmation is invalid");
    }
    if input.reason.trim() != input.reason
        || !(12..=500).contains(&input.reason.chars().count())
        || input.reason.chars().any(char::is_control)
    {
        bail!("upstream transport target revocation reason is invalid");
    }
    Ok(())
}

pub(super) fn validate_actor_for_owner(kind: &str, user_id: &str, owner_id: &str) -> Result<()> {
    if (kind == UPSTREAM_TRANSPORT_TARGET_ACTOR_PROVIDER_OWNER && user_id == owner_id)
        || kind == UPSTREAM_TRANSPORT_TARGET_ACTOR_PLATFORM_ADMIN
    {
        Ok(())
    } else {
        bail!("upstream transport target actor is not authorized for Provider owner identity")
    }
}

pub(super) fn validate_platform_actor_on(
    conn: &rusqlite::Connection,
    kind: &str,
    user_id: &str,
) -> Result<()> {
    if kind != UPSTREAM_TRANSPORT_TARGET_ACTOR_PLATFORM_ADMIN {
        return Ok(());
    }
    let authorized: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM users
          WHERE id=?1 AND role IN ('admin','owner') AND status='active')",
        rusqlite::params![user_id],
        |row| row.get(0),
    )?;
    if !authorized {
        bail!("upstream transport target platform actor is not a current administrator");
    }
    Ok(())
}

fn validate_actor_kind(value: &str) -> Result<()> {
    if matches!(
        value,
        UPSTREAM_TRANSPORT_TARGET_ACTOR_PROVIDER_OWNER
            | UPSTREAM_TRANSPORT_TARGET_ACTOR_PLATFORM_ADMIN
    ) {
        Ok(())
    } else {
        bail!("upstream transport target actor kind is invalid")
    }
}

fn validate_identifier(value: &str) -> Result<()> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > 240
        || value.chars().any(char::is_control)
    {
        bail!("upstream transport target identifier is invalid");
    }
    Ok(())
}

fn validate_digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("upstream transport target digest is invalid");
    }
    Ok(())
}
