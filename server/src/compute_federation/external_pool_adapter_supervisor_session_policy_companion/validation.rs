use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat};

use crate::compute_federation::provider::PROVIDER_STATUS_REGISTERING;

use super::*;

pub(crate) fn validate_supervisor_session_policy(
    policy: &ExternalPoolAdapterSupervisorSessionPolicy,
) -> Result<()> {
    let expected = server_policy_without_validation();
    if policy != &expected {
        bail!("supervisor session policy is not the exact server catalog entry");
    }
    Ok(())
}

pub(crate) fn validate_embedded_supervisor_session_policy_shape(
    policy: &ExternalPoolAdapterSupervisorSessionPolicy,
) -> Result<()> {
    let expected = match (policy.policy_id.as_str(), policy.policy_revision) {
        (
            super::policy::SUPERVISOR_SESSION_POLICY_V1_ID,
            super::policy::SUPERVISOR_SESSION_POLICY_V1_REVISION,
        ) => super::policy::policy_v1_for_validation(),
        _ => bail!("embedded supervisor session policy version is unsupported"),
    };
    if policy != &expected {
        bail!("embedded supervisor session policy is not the frozen v1 catalog entry");
    }
    Ok(())
}

pub(crate) fn validate_supervisor_session_companion_receipt(
    receipt: &ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt,
) -> Result<()> {
    metadata(
        &receipt.schema,
        SUPERVISOR_SESSION_COMPANION_SCHEMA,
        &receipt.companion_id,
        &receipt.companion_digest,
        &receipt.companion_material_digest,
        &receipt.canonicalization,
        &receipt.digest_algorithm,
    )?;
    let c = &receipt.companion;
    identifiers([
        &c.profile_id,
        &c.candidate_id,
        &c.provider_binding_id,
        &c.provider_id,
        &c.provider_owner_account_id,
        &c.delegation_id,
        &c.registry_release_id,
        &c.installation_receipt_id,
        &c.route_adapter_projection_id,
        &c.logical_adapter_id,
        &c.release_version,
        &c.service_actor_id,
        &c.entrypoint_capsule_policy_id,
        &c.target_id,
        &c.recorded_by_actor_user_id,
        &c.idempotency_scope,
        &c.idempotency_key,
    ])?;
    digests([
        &c.profile_digest,
        &c.candidate_digest,
        &c.provider_binding_digest,
        &c.provider_digest,
        &c.launch_policy_digest,
        &c.process_isolation_policy_digest,
        &c.resource_policy_digest,
        &c.network_egress_policy_digest,
        &c.delegation_digest,
        &c.registry_release_digest,
        &c.installation_receipt_digest,
        &c.installation_content_digest,
        &c.implementation_digest,
        &c.capability_set_digest,
        &c.credential_verifier_digest,
        &c.entrypoint_capsule_policy_digest,
        &c.target_digest,
        &c.target_policy_digest,
        &c.supervisor_session_policy_digest,
    ])?;
    optional_pair(&c.predecessor_companion_id, &c.predecessor_companion_digest)?;
    if c.provider_status != PROVIDER_STATUS_REGISTERING
        || c.provider_policy_revision <= 0
        || c.adapter_config_revision <= 0
        || c.sequence == 0
        || c.entrypoint_capsule_policy_revision != 1
        || c.process_isolation_policy_revision == 0
        || c.resource_policy_revision == 0
        || c.network_egress_policy_revision == 0
        || c.confirmation != SUPERVISOR_SESSION_COMPANION_CONFIRMATION
        || c.companion_status != SUPERVISOR_SESSION_COMPANION_STATUS
        || c.companion_effect != SUPERVISOR_SESSION_COMPANION_EFFECT
        || !actor(&c.recorded_by_actor_kind)
        || !opaque_digest(&c.adapter_config_digest)
        || !no_effects(c)
        || any_ready(c)
    {
        bail!("supervisor session companion material is not inert and exact");
    }
    canonical_nanos(&c.recorded_at)?;
    validate_embedded_supervisor_session_policy_shape(&c.supervisor_session_policy)?;
    if supervisor_session_policy_digest(&c.supervisor_session_policy)?
        != c.supervisor_session_policy_digest
        || supervisor_session_companion_material_digest(c)? != receipt.companion_material_digest
        || canonical_supervisor_session_companion_json_and_digest(receipt)?.1
            != receipt.companion_digest
    {
        bail!("supervisor session companion digest is not exact");
    }
    Ok(())
}

pub(crate) fn validate_supervisor_session_companion_revocation_receipt(
    receipt: &ExternalPoolAdapterSupervisorSessionPolicyCompanionRevocationReceipt,
) -> Result<()> {
    metadata(
        &receipt.schema,
        SUPERVISOR_SESSION_COMPANION_REVOCATION_SCHEMA,
        &receipt.revocation_id,
        &receipt.revocation_digest,
        &receipt.revocation_material_digest,
        &receipt.canonicalization,
        &receipt.digest_algorithm,
    )?;
    let r = &receipt.revocation;
    identifiers([
        &r.companion_id,
        &r.target_id,
        &r.profile_id,
        &r.provider_binding_id,
        &r.provider_id,
        &r.revoked_by_actor_user_id,
        &r.idempotency_scope,
        &r.idempotency_key,
    ])?;
    digests([
        &r.companion_digest,
        &r.target_digest,
        &r.profile_digest,
        &r.provider_binding_digest,
    ])?;
    if !actor(&r.revoked_by_actor_kind)
        || !(12..=500).contains(&r.reason.chars().count())
        || r.reason.trim() != r.reason
        || r.revoked_at != r.recorded_at
        || r.confirmation != SUPERVISOR_SESSION_COMPANION_REVOCATION_CONFIRMATION
        || r.revocation_effect != SUPERVISOR_SESSION_COMPANION_REVOCATION_EFFECT
        || [
            r.adapter_effect.as_str(),
            r.runtime_effect.as_str(),
            r.provider_effect.as_str(),
            r.credential_effect.as_str(),
            r.route_effect.as_str(),
            r.execution_effect.as_str(),
            r.usage_effect.as_str(),
            r.market_effect.as_str(),
            r.settlement_effect.as_str(),
        ]
        .into_iter()
        .any(|v| v != SUPERVISOR_SESSION_COMPANION_NO_EFFECT)
        || r.process_spawn_ready
        || r.ipc_session_ready
        || r.secret_delivery_ready
        || r.broker_connect_ready
        || r.upstream_probe_observed
        || r.runtime_launch_ready
        || r.activation_ready
    {
        bail!("supervisor session companion revocation is not inert and exact");
    }
    canonical_nanos(&r.revoked_at)?;
    if supervisor_session_companion_revocation_material_digest(r)?
        != receipt.revocation_material_digest
        || canonical_supervisor_session_companion_revocation_json_and_digest(receipt)?.1
            != receipt.revocation_digest
    {
        bail!("supervisor session companion revocation digest is not exact");
    }
    Ok(())
}

fn server_policy_without_validation() -> ExternalPoolAdapterSupervisorSessionPolicy {
    // Kept separate to avoid recursive catalog validation. This is the only exact comparison seam.
    super::policy::policy_for_validation()
}
fn no_effects(c: &ExternalPoolAdapterSupervisorSessionPolicyCompanionMaterial) -> bool {
    [
        &c.adapter_effect,
        &c.runtime_effect,
        &c.provider_effect,
        &c.credential_effect,
        &c.route_effect,
        &c.execution_effect,
        &c.usage_effect,
        &c.market_effect,
        &c.settlement_effect,
    ]
    .into_iter()
    .all(|v| v == SUPERVISOR_SESSION_COMPANION_NO_EFFECT)
}
fn any_ready(c: &ExternalPoolAdapterSupervisorSessionPolicyCompanionMaterial) -> bool {
    c.process_spawn_ready
        || c.ipc_session_ready
        || c.secret_delivery_ready
        || c.broker_connect_ready
        || c.upstream_probe_observed
        || c.runtime_launch_ready
        || c.activation_ready
}
fn actor(v: &str) -> bool {
    matches!(
        v,
        SUPERVISOR_SESSION_COMPANION_ACTOR_PROVIDER_OWNER
            | SUPERVISOR_SESSION_COMPANION_ACTOR_PLATFORM_ADMIN
    )
}
fn identifiers<const N: usize>(values: [&str; N]) -> Result<()> {
    if values.into_iter().any(|v| {
        v.is_empty() || v.trim() != v || v.chars().count() > 240 || v.chars().any(char::is_control)
    }) {
        bail!("supervisor session identifier is invalid")
    }
    Ok(())
}
fn digests<const N: usize>(values: [&str; N]) -> Result<()> {
    if values.into_iter().any(|v| !is_digest(v)) {
        bail!("supervisor session digest is invalid")
    }
    Ok(())
}
fn is_digest(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn opaque_digest(v: &str) -> bool {
    !v.is_empty() && v.trim() == v && v.len() <= 500 && !v.chars().any(char::is_control)
}
fn optional_pair(id: &Option<String>, digest: &Option<String>) -> Result<()> {
    if id.is_some() != digest.is_some() {
        bail!("supervisor session predecessor pair is incomplete")
    }
    if let Some(v) = id {
        identifiers([v])?
    }
    if let Some(v) = digest {
        digests([v])?
    }
    Ok(())
}
fn canonical_nanos(v: &str) -> Result<()> {
    let t = DateTime::parse_from_rfc3339(v)?;
    if t.offset().local_minus_utc() != 0 || t.to_rfc3339_opts(SecondsFormat::Nanos, true) != v {
        bail!("supervisor session timestamp is not canonical UTC nanos")
    }
    Ok(())
}
fn metadata(
    schema: &str,
    expected: &str,
    id: &str,
    digest: &str,
    material: &str,
    canonicalization: &str,
    algorithm: &str,
) -> Result<()> {
    identifiers([id])?;
    digests([digest, material])?;
    if schema != expected
        || canonicalization != SUPERVISOR_SESSION_COMPANION_CANONICALIZATION
        || algorithm != SUPERVISOR_SESSION_COMPANION_DIGEST_ALGORITHM
    {
        bail!("supervisor session receipt metadata is invalid")
    }
    Ok(())
}
