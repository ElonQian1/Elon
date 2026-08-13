use anyhow::{bail, Result};

use crate::compute_federation::external_pool_adapter_runtime_launch_profile::{
    RUNTIME_LAUNCH_PROFILE_ACTOR_PLATFORM_ADMIN, RUNTIME_LAUNCH_PROFILE_ACTOR_PROVIDER_OWNER,
    RUNTIME_LAUNCH_PROFILE_CONFIRMATION, RUNTIME_LAUNCH_PROFILE_REVOCATION_CONFIRMATION,
};

use super::types::{
    CreateExternalPoolAdapterRuntimeLaunchProfile, RevokeExternalPoolAdapterRuntimeLaunchProfile,
};

pub(super) fn validate_create_actor(
    input: &CreateExternalPoolAdapterRuntimeLaunchProfile,
) -> Result<()> {
    let binding = input.prepared.binding();
    validate_actor_for_owner(
        &input.recorded_by_actor_kind,
        &input.recorded_by_actor_user_id,
        &binding.provider_owner_account_id,
    )
}

pub(super) fn validate_create_input(
    input: &CreateExternalPoolAdapterRuntimeLaunchProfile,
) -> Result<()> {
    validate_actor_kind(&input.recorded_by_actor_kind)?;
    for value in [
        input.candidate_id.as_str(),
        input.recorded_by_actor_user_id.as_str(),
        input.idempotency_scope.as_str(),
        input.idempotency_key.as_str(),
    ] {
        validate_identifier(value)?;
    }
    for value in [
        input.expected_candidate_digest.as_str(),
        input.expected_provider_binding_digest.as_str(),
        input.expected_launch_policy_digest.as_str(),
    ] {
        validate_digest(value)?;
    }
    if input.confirmation != RUNTIME_LAUNCH_PROFILE_CONFIRMATION {
        bail!("runtime launch profile confirmation is invalid");
    }
    if input.predecessor_profile_id.is_some() != input.expected_predecessor_profile_digest.is_some()
    {
        bail!("runtime launch profile predecessor pair is incomplete");
    }
    if let Some(value) = &input.predecessor_profile_id {
        validate_identifier(value)?;
    }
    if let Some(value) = &input.expected_predecessor_profile_digest {
        validate_digest(value)?;
    }
    Ok(())
}

pub(super) fn validate_revoke_input(
    input: &RevokeExternalPoolAdapterRuntimeLaunchProfile,
) -> Result<()> {
    validate_actor_kind(&input.revoked_by_actor_kind)?;
    for value in [
        input.profile_id.as_str(),
        input.revoked_by_actor_user_id.as_str(),
        input.idempotency_scope.as_str(),
        input.idempotency_key.as_str(),
    ] {
        validate_identifier(value)?;
    }
    validate_digest(&input.expected_profile_digest)?;
    validate_digest(&input.expected_candidate_digest)?;
    if input.confirmation != RUNTIME_LAUNCH_PROFILE_REVOCATION_CONFIRMATION {
        bail!("runtime launch profile revocation confirmation is invalid");
    }
    if input.reason.trim() != input.reason
        || !(12..=500).contains(&input.reason.chars().count())
        || input.reason.chars().any(char::is_control)
    {
        bail!("runtime launch profile revocation reason is invalid");
    }
    Ok(())
}

pub(super) fn validate_actor_for_owner(kind: &str, user_id: &str, owner_id: &str) -> Result<()> {
    if (kind == RUNTIME_LAUNCH_PROFILE_ACTOR_PROVIDER_OWNER && user_id == owner_id)
        || kind == RUNTIME_LAUNCH_PROFILE_ACTOR_PLATFORM_ADMIN
    {
        Ok(())
    } else {
        bail!("runtime launch profile actor is not authorized for Provider owner identity")
    }
}

fn validate_actor_kind(value: &str) -> Result<()> {
    if matches!(
        value,
        RUNTIME_LAUNCH_PROFILE_ACTOR_PROVIDER_OWNER | RUNTIME_LAUNCH_PROFILE_ACTOR_PLATFORM_ADMIN
    ) {
        Ok(())
    } else {
        bail!("runtime launch profile actor kind is invalid")
    }
}

pub(super) fn validate_platform_actor_on(
    conn: &rusqlite::Connection,
    kind: &str,
    user_id: &str,
) -> Result<()> {
    if kind != RUNTIME_LAUNCH_PROFILE_ACTOR_PLATFORM_ADMIN {
        return Ok(());
    }
    let authorized: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM users
          WHERE id=?1 AND role IN ('admin','owner') AND status='active')",
        rusqlite::params![user_id],
        |row| row.get(0),
    )?;
    if !authorized {
        bail!("runtime launch profile platform actor is not a current administrator");
    }
    Ok(())
}

fn validate_identifier(value: &str) -> Result<()> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > 240
        || value.chars().any(char::is_control)
    {
        bail!("runtime launch profile identifier is invalid");
    }
    Ok(())
}

fn validate_digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("runtime launch profile digest is invalid");
    }
    Ok(())
}
