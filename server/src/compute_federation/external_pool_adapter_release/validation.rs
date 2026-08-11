use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat};

use super::{
    canonical::{
        canonical_external_pool_adapter_release_capability_set_digest,
        canonical_external_pool_adapter_release_request_json_and_digest,
        canonical_external_pool_adapter_release_request_material_digest,
    },
    types::{
        ComputeExternalPoolAdapterReleaseCapability, ComputeExternalPoolAdapterReleaseIntent,
        ComputeExternalPoolAdapterReleaseRequest, ComputeExternalPoolAdapterReleaseRequestEnvelope,
        ComputeExternalPoolAdapterReleaseVerifierIntent,
        COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_CANONICALIZATION,
        COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_CONFIRMATION,
        COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_DIGEST_ALGORITHM,
        COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_PROVIDER_KIND,
        COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_REQUEST_SCHEMA,
        COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_ROUTE_KIND,
    },
};

const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;
const REQUIRED_CAPABILITIES: [&str; 6] = [
    "authenticated_ack",
    "authenticated_events",
    "cancel_no_start",
    "idempotent_commit",
    "prepare",
    "reconcile",
];

pub(crate) fn validate_external_pool_adapter_release_request_envelope(
    envelope: &ComputeExternalPoolAdapterReleaseRequestEnvelope,
) -> Result<()> {
    validate_identifier(&envelope.request_id, "Adapter release request ID", 160)?;
    validate_digest(&envelope.request_digest, "Adapter release request digest")?;
    validate_digest(
        &envelope.request_material_digest,
        "Adapter release request material digest",
    )?;
    if envelope.schema != COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_REQUEST_SCHEMA
        || envelope.canonicalization != COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_CANONICALIZATION
        || envelope.digest_algorithm != COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_DIGEST_ALGORITHM
    {
        bail!("external-pool Adapter release request metadata is not supported");
    }
    validate_external_pool_adapter_release_request_material(&envelope.request)?;
    let material_digest =
        canonical_external_pool_adapter_release_request_material_digest(&envelope.request)?;
    if material_digest != envelope.request_material_digest {
        bail!("external-pool Adapter release request material digest is not canonical");
    }
    let (_, digest) = canonical_external_pool_adapter_release_request_json_and_digest(envelope)?;
    if digest != envelope.request_digest {
        bail!("external-pool Adapter release request digest is not canonical");
    }
    Ok(())
}

pub(crate) fn validate_external_pool_adapter_release_request_material(
    request: &ComputeExternalPoolAdapterReleaseRequest,
) -> Result<()> {
    validate_identifier(
        &request.submitted_by_admin_user_id,
        "Adapter release submitting administrator",
        160,
    )?;
    validate_identifier(
        &request.idempotency_key,
        "Adapter release idempotency key",
        160,
    )?;
    validate_text(
        &request.submission_note,
        "Adapter release submission note",
        2_000,
        true,
    )?;
    if request.confirmation != COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_CONFIRMATION {
        bail!("external-pool Adapter release confirmation is not exact");
    }
    parse_timestamp(&request.submitted_at, "Adapter release submitted_at")?;
    validate_release_intent(&request.release)?;
    let _ = canonical_external_pool_adapter_release_request_material_digest(request)?;
    Ok(())
}

fn validate_release_intent(intent: &ComputeExternalPoolAdapterReleaseIntent) -> Result<()> {
    validate_identifier(&intent.adapter_id, "Adapter ID", 160)?;
    validate_identifier(&intent.release_version, "Adapter release version", 80)?;
    if intent.route_kind != COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_ROUTE_KIND {
        bail!("external-pool Adapter release must use server_adapter");
    }
    if intent.supported_provider_kinds.len() != 1
        || intent.supported_provider_kinds[0] != COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_PROVIDER_KIND
    {
        bail!("external-pool Adapter release must support only external_pool");
    }
    validate_artifact_ref(&intent.candidate_artifact_ref)?;
    validate_digest(
        &intent.declared_implementation_sha256,
        "declared Adapter implementation digest",
    )?;
    validate_capabilities(&intent.supported_capabilities)?;
    validate_digest(
        &intent.capability_set_digest,
        "declared Adapter capability set digest",
    )?;
    let capability_set_digest = canonical_external_pool_adapter_release_capability_set_digest(
        &intent.supported_capabilities,
    )?;
    if capability_set_digest != intent.capability_set_digest {
        bail!("external-pool Adapter release capability set digest is not canonical");
    }
    validate_verifier_intent(&intent.expected_credential_verifier)
}

fn validate_capabilities(
    capabilities: &[ComputeExternalPoolAdapterReleaseCapability],
) -> Result<()> {
    if capabilities.len() != REQUIRED_CAPABILITIES.len() {
        bail!("external-pool Adapter release requires exactly six capabilities");
    }
    for (capability, expected_id) in capabilities.iter().zip(REQUIRED_CAPABILITIES) {
        if capability.capability_id != expected_id
            || !(1..=MAX_SAFE_INTEGER).contains(&capability.capability_revision)
        {
            bail!("external-pool Adapter release capability order or revision is invalid");
        }
    }
    Ok(())
}

fn validate_verifier_intent(
    verifier: &ComputeExternalPoolAdapterReleaseVerifierIntent,
) -> Result<()> {
    validate_identifier(
        &verifier.verification_kind,
        "expected credential verification kind",
        80,
    )?;
    validate_identifier(
        &verifier.verifier_id,
        "expected credential verifier ID",
        160,
    )?;
    if !(1..=MAX_SAFE_INTEGER).contains(&verifier.verifier_revision) {
        bail!("expected credential verifier revision is invalid");
    }
    validate_digest(
        &verifier.verifier_digest,
        "expected credential verifier digest",
    )
}

fn validate_artifact_ref(value: &str) -> Result<()> {
    let suffix = value
        .strip_prefix("artifact-ref:")
        .ok_or_else(|| anyhow::anyhow!("candidate artifact ref must use artifact-ref"))?;
    if suffix.is_empty()
        || suffix.len() > 160
        || !suffix
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        bail!("candidate artifact ref has an invalid opaque ID");
    }
    Ok(())
}

fn validate_identifier(value: &str, label: &str, limit: usize) -> Result<()> {
    validate_text(value, label, limit, false)
}

fn validate_text(value: &str, label: &str, limit: usize, allow_empty: bool) -> Result<()> {
    if (!allow_empty && value.is_empty())
        || value.trim() != value
        || value.len() > limit
        || value.chars().any(char::is_control)
    {
        bail!("{label} is invalid");
    }
    Ok(())
}

fn validate_digest(value: &str, label: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("{label} must be a lowercase SHA-256 digest");
    }
    Ok(())
}

fn parse_timestamp(value: &str, label: &str) -> Result<DateTime<chrono::FixedOffset>> {
    let parsed = DateTime::parse_from_rfc3339(value)
        .map_err(|_| anyhow::anyhow!("{label} is not RFC3339"))?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("{label} must use canonical UTC nanoseconds");
    }
    Ok(parsed)
}
