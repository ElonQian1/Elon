use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat};

use crate::compute_federation::provider::{
    ComputeProvider, ComputeProviderAdapterRef, ComputeProviderCapabilities,
    PROVIDER_KIND_EXTERNAL_POOL, PROVIDER_STATUS_REGISTERING,
};

use super::{
    canonical::{
        canonical_external_pool_onboarding_request_json_and_digest,
        canonical_external_pool_onboarding_request_material_digest,
    },
    types::{
        ComputeExternalPoolOnboardingAdapterIntent, ComputeExternalPoolOnboardingCredentialIntent,
        ComputeExternalPoolOnboardingRequest, ComputeExternalPoolOnboardingRequestEnvelope,
        COMPUTE_EXTERNAL_POOL_ONBOARDING_CANONICALIZATION,
        COMPUTE_EXTERNAL_POOL_ONBOARDING_CONFIRMATION,
        COMPUTE_EXTERNAL_POOL_ONBOARDING_DIGEST_ALGORITHM,
        COMPUTE_EXTERNAL_POOL_ONBOARDING_REQUEST_SCHEMA,
        COMPUTE_EXTERNAL_POOL_ONBOARDING_TRUST_TIER,
    },
};

const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

pub(crate) fn validate_external_pool_onboarding_request_envelope(
    envelope: &ComputeExternalPoolOnboardingRequestEnvelope,
) -> Result<()> {
    validate_identifier(&envelope.request_id, "external-pool request ID", 160)?;
    validate_digest(&envelope.request_digest, "external-pool request digest")?;
    if envelope.schema != COMPUTE_EXTERNAL_POOL_ONBOARDING_REQUEST_SCHEMA
        || envelope.canonicalization != COMPUTE_EXTERNAL_POOL_ONBOARDING_CANONICALIZATION
        || envelope.digest_algorithm != COMPUTE_EXTERNAL_POOL_ONBOARDING_DIGEST_ALGORITHM
    {
        bail!("external-pool request envelope metadata is not supported");
    }
    validate_external_pool_onboarding_request_material(&envelope.request)?;
    let (_, digest) = canonical_external_pool_onboarding_request_json_and_digest(envelope)?;
    if digest != envelope.request_digest {
        bail!("external-pool request digest does not match its canonical envelope");
    }
    Ok(())
}

pub(crate) fn validate_external_pool_onboarding_request_material(
    request: &ComputeExternalPoolOnboardingRequest,
) -> Result<()> {
    validate_identifier(
        &request.requested_by_owner_user_id,
        "external-pool owner user ID",
        160,
    )?;
    validate_identifier(
        &request.idempotency_key,
        "external-pool idempotency key",
        160,
    )?;
    validate_note(&request.owner_note, 2_000)?;
    if request.confirmation != COMPUTE_EXTERNAL_POOL_ONBOARDING_CONFIRMATION {
        bail!("external-pool owner confirmation is not exact");
    }
    parse_timestamp(&request.submitted_at, "external-pool submitted_at")?;
    validate_target_provider(request)?;
    validate_adapter_intent(&request.adapter_intent)?;
    validate_credential_intent(&request.credential_intent)?;
    validate_external_evidence(request)?;
    let _ = canonical_external_pool_onboarding_request_material_digest(request)?;
    Ok(())
}

fn validate_target_provider(request: &ComputeExternalPoolOnboardingRequest) -> Result<()> {
    let provider = &request.target_provider;
    validate_identifier(&provider.provider_id, "external-pool Provider ID", 160)?;
    validate_identifier(
        &provider.owner_account_id,
        "external-pool owner account ID",
        160,
    )?;
    validate_text(
        &provider.display_name,
        "external-pool display name",
        160,
        false,
    )?;
    let settlement = provider
        .settlement_account_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("external-pool settlement account is required"))?;
    validate_identifier(settlement, "external-pool settlement account ID", 160)?;
    let home_region = provider
        .home_region
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("external-pool home region is required"))?;
    validate_identifier(home_region, "external-pool home region", 80)?;
    if provider.schema != crate::compute_federation::provider::COMPUTE_PROVIDER_SCHEMA
        || provider.provider_kind != PROVIDER_KIND_EXTERNAL_POOL
        || provider.status != PROVIDER_STATUS_REGISTERING
        || provider.trust_tier != COMPUTE_EXTERNAL_POOL_ONBOARDING_TRUST_TIER
        || provider.policy_revision != 1
        || provider.owner_account_id != request.requested_by_owner_user_id
        || provider.endpoint.is_some()
        || provider.created_at != request.submitted_at
        || provider.updated_at != request.submitted_at
    {
        bail!("external-pool target Provider is not the exact registering revision-1 shape");
    }
    validate_provider_capabilities(&provider.capabilities, home_region)?;
    validate_provider_evidence(provider)?;
    validate_provider_adapter(provider.adapter.as_ref(), &request.adapter_intent)
}

fn validate_provider_capabilities(
    capabilities: &ComputeProviderCapabilities,
    home_region: &str,
) -> Result<()> {
    validate_sorted_values(&capabilities.task_kinds, "Provider task kinds", 80, true)?;
    validate_sorted_values(
        &capabilities.accelerator_kinds,
        "Provider accelerator kinds",
        80,
        true,
    )?;
    validate_sorted_values(&capabilities.regions, "Provider regions", 80, true)?;
    validate_sorted_values(
        &capabilities.allowed_data_classes,
        "Provider allowed data classes",
        80,
        true,
    )?;
    if !capabilities
        .regions
        .iter()
        .any(|region| region == home_region)
    {
        bail!("external-pool home region is absent from the capability envelope");
    }
    Ok(())
}

fn validate_provider_evidence(provider: &ComputeProvider) -> Result<()> {
    let evidence = &provider.evidence_profile;
    if evidence.observed_hardware_digest.is_some()
        || evidence.verified_hardware_digest.is_some()
        || evidence.last_observed_at.is_some()
        || evidence.last_verified_at.is_some()
    {
        bail!("owner onboarding cannot claim observed or verified Provider evidence");
    }
    if let Some(digest) = evidence.declared_hardware_digest.as_deref() {
        validate_digest(digest, "declared hardware digest")?;
    }
    Ok(())
}

fn validate_provider_adapter(
    provider_adapter: Option<&ComputeProviderAdapterRef>,
    intent: &ComputeExternalPoolOnboardingAdapterIntent,
) -> Result<()> {
    let adapter = provider_adapter
        .ok_or_else(|| anyhow::anyhow!("external-pool target Provider requires an Adapter ref"))?;
    if adapter.adapter_id != intent.expected_adapter_id
        || adapter.adapter_version != intent.expected_release_version
        || adapter.config_revision != intent.expected_config_revision
        || adapter.config_digest != intent.expected_config_digest
    {
        bail!("external-pool target Provider Adapter ref differs from the owner intent");
    }
    Ok(())
}

fn validate_adapter_intent(intent: &ComputeExternalPoolOnboardingAdapterIntent) -> Result<()> {
    validate_identifier(&intent.expected_adapter_id, "expected Adapter ID", 160)?;
    validate_identifier(
        &intent.expected_release_version,
        "expected Adapter release version",
        80,
    )?;
    validate_exact_value(
        &intent.expected_config_digest,
        "expected Adapter config digest",
        512,
    )?;
    if !(1..=MAX_SAFE_INTEGER).contains(&intent.expected_config_revision) {
        bail!("expected Adapter config revision is invalid");
    }
    Ok(())
}

fn validate_credential_intent(
    intent: &ComputeExternalPoolOnboardingCredentialIntent,
) -> Result<()> {
    match (
        intent.non_bearer_credential_ref.as_deref(),
        intent.credential_hint.as_deref(),
    ) {
        (None, None) => {}
        (Some(reference), Some(hint)) => {
            validate_credential_locator(reference)?;
            validate_text(hint, "credential hint", 160, false)?;
        }
        _ => bail!("credential ref and hint must be supplied together"),
    }
    Ok(())
}

fn validate_external_evidence(request: &ComputeExternalPoolOnboardingRequest) -> Result<()> {
    match (
        request.external_evidence_ref.as_deref(),
        request.external_evidence_sha256.as_deref(),
    ) {
        (None, None) => Ok(()),
        (Some(reference), Some(digest)) => {
            validate_server_locator(reference, &["evidence-ref:"], "external evidence ref")?;
            validate_digest(digest, "external evidence digest")
        }
        _ => bail!("external evidence ref and digest must be supplied together"),
    }
}

fn validate_sorted_values(
    values: &[String],
    label: &str,
    item_limit: usize,
    required: bool,
) -> Result<()> {
    if values.len() > 64 || (required && values.is_empty()) {
        bail!("{label} count is invalid");
    }
    let mut previous: Option<&str> = None;
    for value in values {
        validate_identifier(value, label, item_limit)?;
        if previous.is_some_and(|item| item >= value.as_str()) {
            bail!("{label} must be unique and sorted");
        }
        previous = Some(value);
    }
    Ok(())
}

fn validate_credential_locator(value: &str) -> Result<()> {
    validate_server_locator(value, &["vault-ref:", "gateway-ref:"], "credential ref")
}

fn validate_server_locator(value: &str, prefixes: &[&str], label: &str) -> Result<()> {
    let suffix = prefixes
        .iter()
        .find_map(|prefix| value.strip_prefix(prefix))
        .ok_or_else(|| anyhow::anyhow!("{label} does not use a server-issued locator scheme"))?;
    if suffix.is_empty()
        || suffix.len() > 160
        || !suffix
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        bail!("{label} has an invalid server-issued locator ID");
    }
    Ok(())
}

fn validate_identifier(value: &str, label: &str, limit: usize) -> Result<()> {
    validate_text(value, label, limit, false)
}

fn validate_exact_value(value: &str, label: &str, limit: usize) -> Result<()> {
    validate_text(value, label, limit, false)
}

fn validate_note(value: &str, limit: usize) -> Result<()> {
    validate_text(value, "external-pool owner note", limit, true)
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
