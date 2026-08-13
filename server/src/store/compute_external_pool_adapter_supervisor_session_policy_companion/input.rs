use anyhow::{bail, Result};

use crate::compute_federation::external_pool_adapter_supervisor_session_policy_companion::*;

use super::types::*;

pub(super) fn validate_create_input(
    input: &CreateExternalPoolAdapterSupervisorSessionPolicyCompanion,
) -> Result<()> {
    actor(&input.recorded_by_actor_kind)?;
    identifiers([
        &input.target_id,
        &input.recorded_by_actor_user_id,
        &input.idempotency_scope,
        &input.idempotency_key,
    ])?;
    digests([
        &input.expected_target_digest,
        &input.expected_profile_digest,
        &input.expected_candidate_digest,
        &input.expected_provider_binding_digest,
        &input.expected_supervisor_session_policy_digest,
    ])?;
    if input.confirmation != SUPERVISOR_SESSION_COMPANION_CONFIRMATION {
        bail!("supervisor session companion confirmation is invalid")
    }
    optional_pair(
        &input.predecessor_companion_id,
        &input.expected_predecessor_companion_digest,
    )
}
pub(super) fn validate_revoke_input(
    input: &RevokeExternalPoolAdapterSupervisorSessionPolicyCompanion,
) -> Result<()> {
    actor(&input.revoked_by_actor_kind)?;
    identifiers([
        &input.companion_id,
        &input.revoked_by_actor_user_id,
        &input.idempotency_scope,
        &input.idempotency_key,
    ])?;
    digests([
        &input.expected_companion_digest,
        &input.expected_target_digest,
        &input.expected_profile_digest,
    ])?;
    if !(12..=500).contains(&input.reason.chars().count())
        || input.reason.trim() != input.reason
        || input.confirmation != SUPERVISOR_SESSION_COMPANION_REVOCATION_CONFIRMATION
    {
        bail!("supervisor session companion revocation input is invalid")
    }
    Ok(())
}
pub(super) fn validate_actor_for_owner(kind: &str, user: &str, owner: &str) -> Result<()> {
    if kind == SUPERVISOR_SESSION_COMPANION_ACTOR_PROVIDER_OWNER && user != owner {
        bail!("supervisor session companion owner actor is not exact")
    }
    Ok(())
}
pub(super) fn validate_platform_actor_on(
    conn: &rusqlite::Connection,
    kind: &str,
    user: &str,
) -> Result<()> {
    if kind == SUPERVISOR_SESSION_COMPANION_ACTOR_PLATFORM_ADMIN {
        let role: String =
            conn.query_row("SELECT role FROM users WHERE id=?1", [user], |r| r.get(0))?;
        if !matches!(role.as_str(), "admin" | "owner") {
            bail!("supervisor session companion platform actor is not privileged")
        }
    }
    Ok(())
}
fn actor(v: &str) -> Result<()> {
    if !matches!(
        v,
        SUPERVISOR_SESSION_COMPANION_ACTOR_PROVIDER_OWNER
            | SUPERVISOR_SESSION_COMPANION_ACTOR_PLATFORM_ADMIN
    ) {
        bail!("supervisor session companion actor kind is invalid")
    }
    Ok(())
}
fn identifiers<const N: usize>(v: [&str; N]) -> Result<()> {
    if v.into_iter()
        .any(|x| x.is_empty() || x.trim() != x || x.len() > 240 || x.chars().any(char::is_control))
    {
        bail!("supervisor session companion identifier is invalid")
    }
    Ok(())
}
fn digests<const N: usize>(v: [&str; N]) -> Result<()> {
    if v.into_iter().any(|x| {
        x.len() != 64
            || !x
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    }) {
        bail!("supervisor session companion digest is invalid")
    }
    Ok(())
}
fn optional_pair(id: &Option<String>, digest: &Option<String>) -> Result<()> {
    if id.is_some() != digest.is_some() {
        bail!("supervisor session companion predecessor pair is incomplete")
    }
    if let Some(x) = id {
        identifiers([x])?
    }
    if let Some(x) = digest {
        digests([x])?
    }
    Ok(())
}
