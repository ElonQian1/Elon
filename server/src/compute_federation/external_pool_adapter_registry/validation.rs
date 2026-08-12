use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat};

use crate::compute_federation::external_pool_adapter_release::{
    canonical_external_pool_adapter_release_capability_set_digest,
    COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_PROVIDER_KIND,
    COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_ROUTE_KIND,
};

use super::*;

const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;
const REQUIRED_CAPABILITIES: [&str; 6] = [
    "authenticated_ack",
    "authenticated_events",
    "cancel_no_start",
    "idempotent_commit",
    "prepare",
    "reconcile",
];

pub(crate) fn validate_registry_release_receipt(
    receipt: &ExternalPoolAdapterRegistryReleaseReceipt,
) -> Result<()> {
    if receipt.schema != REGISTRY_RELEASE_RECEIPT_SCHEMA
        || receipt.canonicalization != REGISTRY_CANONICALIZATION
        || receipt.digest_algorithm != REGISTRY_DIGEST_ALGORITHM
    {
        bail!("Adapter registry release metadata is unsupported");
    }
    identifier(&receipt.registry_release_id, 200)?;
    digest(&receipt.registry_release_digest)?;
    digest(&receipt.registry_release_material_digest)?;
    let item = &receipt.release;
    for value in [
        &item.admission_id,
        &item.package_receipt_id,
        &item.source_receipt_id,
        &item.adapter_id,
        &item.release_version,
        &item.route_kind,
    ] {
        identifier(value, 200)?;
    }
    for value in [
        &item.admission_digest,
        &item.package_receipt_digest,
        &item.package_material_digest,
        &item.source_receipt_digest,
        &item.implementation_digest,
        &item.declared_implementation_sha256,
        &item.capability_set_digest,
        &item.credential_verifier_digest,
        &item.archive_sha256,
        &item.manifest_digest,
        &item.entry_inventory_digest,
        &item.installation_content_digest,
    ] {
        digest(value)?;
    }
    canonical_nanos(&item.registered_at)?;
    canonical_nanos(&item.recorded_at)?;
    validate_release_contract(item)?;
    if item.registered_at != item.recorded_at
        || item.implementation_digest != item.declared_implementation_sha256
        || item.implementation_digest != item.archive_sha256
        || item.manifest.adapter_id != item.adapter_id
        || item.manifest.release_version != item.release_version
        || item.manifest.supported_capabilities != item.supported_capabilities
        || item.manifest.capability_set_digest != item.capability_set_digest
        || item.manifest.credential_verifier != item.credential_verifier
        || item.credential_verifier.verifier_digest != item.credential_verifier_digest
        || item.archive_size_bytes == 0
        || item.entry_count == 0
        || item.total_uncompressed_bytes == 0
        || item.registry_effect != REGISTRY_RELEASE_EFFECT
        || !no_effects([
            &item.provider_effect,
            &item.credential_effect,
            &item.route_effect,
            &item.execution_effect,
            &item.settlement_effect,
        ])
        || registry_release_material_digest(item)? != receipt.registry_release_material_digest
        || canonical_registry_release_receipt_json_and_digest(receipt)?.1
            != receipt.registry_release_digest
    {
        bail!("Adapter registry release material is not exact");
    }
    Ok(())
}

pub(crate) fn validate_registry_provider_binding_receipt(
    receipt: &ExternalPoolAdapterRegistryProviderBindingReceipt,
) -> Result<()> {
    if receipt.schema != REGISTRY_PROVIDER_BINDING_RECEIPT_SCHEMA
        || receipt.canonicalization != REGISTRY_CANONICALIZATION
        || receipt.digest_algorithm != REGISTRY_DIGEST_ALGORITHM
    {
        bail!("Adapter registry provider binding metadata is unsupported");
    }
    identifier(&receipt.provider_binding_id, 200)?;
    digest(&receipt.provider_binding_digest)?;
    digest(&receipt.provider_binding_material_digest)?;
    let item = &receipt.binding;
    for value in [
        &item.registry_release_id,
        &item.route_adapter_projection_id,
        &item.installation_receipt_id,
        &item.application_id,
        &item.adoption_receipt_id,
        &item.provider_id,
        &item.provider_owner_account_id,
        &item.adapter_id,
        &item.release_version,
        &item.admission_id,
        &item.package_receipt_id,
        &item.source_receipt_id,
        &item.sandbox_conformance_receipt_id,
        &item.credential_verification_receipt_id,
        &item.bound_by_admin_user_id,
    ] {
        identifier(value, 200)?;
    }
    for value in [&item.idempotency_scope, &item.idempotency_key] {
        identifier(value, 240)?;
    }
    for value in [
        &item.registry_release_digest,
        &item.installation_receipt_digest,
        &item.installation_material_digest,
        &item.installation_content_digest,
        &item.application_digest,
        &item.adoption_receipt_digest,
        &item.adoption_material_digest,
        &item.provider_digest,
        &item.admission_digest,
        &item.package_receipt_digest,
        &item.package_material_digest,
        &item.source_receipt_digest,
        &item.sandbox_conformance_receipt_digest,
        &item.credential_verification_receipt_digest,
        &item.credential_locator_commitment,
    ] {
        digest(value)?;
    }
    canonical_nanos(&item.checked_at)?;
    canonical_nanos(&item.bound_at)?;
    canonical_nanos(&item.recorded_at)?;
    if item.checked_at != item.bound_at
        || item.bound_at != item.recorded_at
        || item.provider_policy_revision < 1
        || item.adapter_config_revision < 1
        || item.adapter_config_digest.is_empty()
        || item.adapter_config_digest.trim() != item.adapter_config_digest
        || item.adapter_config_digest.chars().count() > 512
        || item.adapter_config_digest.chars().any(char::is_control)
        || item.confirmation != REGISTRY_BINDING_CONFIRMATION
        || item.registry_effect != REGISTRY_BINDING_EFFECT
        || !no_effects([
            &item.provider_effect,
            &item.credential_effect,
            &item.route_effect,
            &item.execution_effect,
            &item.settlement_effect,
        ])
        || registry_provider_binding_material_digest(item)?
            != receipt.provider_binding_material_digest
        || canonical_registry_provider_binding_receipt_json_and_digest(receipt)?.1
            != receipt.provider_binding_digest
    {
        bail!("Adapter registry provider binding material is not exact");
    }
    Ok(())
}

fn no_effects<const N: usize>(values: [&String; N]) -> bool {
    values.into_iter().all(|value| value == REGISTRY_NO_EFFECT)
}

fn validate_release_contract(item: &ExternalPoolAdapterRegistryReleaseMaterial) -> Result<()> {
    if item.route_kind != COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_ROUTE_KIND
        || item.supported_provider_kinds.len() != 1
        || item.supported_provider_kinds[0] != COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_PROVIDER_KIND
        || item.supported_capabilities.len() != REQUIRED_CAPABILITIES.len()
    {
        bail!("Adapter registry release contract is unsupported");
    }
    for (capability, expected_id) in item
        .supported_capabilities
        .iter()
        .zip(REQUIRED_CAPABILITIES)
    {
        if capability.capability_id != expected_id
            || !(1..=MAX_SAFE_INTEGER).contains(&capability.capability_revision)
        {
            bail!("Adapter registry capability set is invalid");
        }
    }
    if canonical_external_pool_adapter_release_capability_set_digest(&item.supported_capabilities)?
        != item.capability_set_digest
    {
        bail!("Adapter registry capability set digest is not canonical");
    }
    identifier(&item.credential_verifier.verification_kind, 80)?;
    identifier(&item.credential_verifier.verifier_id, 160)?;
    if !(1..=MAX_SAFE_INTEGER).contains(&item.credential_verifier.verifier_revision) {
        bail!("Adapter registry credential verifier revision is invalid");
    }
    digest(&item.credential_verifier.verifier_digest)
}
fn identifier(value: &str, maximum: usize) -> Result<()> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > maximum
        || value.chars().any(char::is_control)
    {
        bail!("Adapter registry identifier is invalid");
    }
    Ok(())
}
fn digest(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        bail!("Adapter registry digest is invalid");
    }
    Ok(())
}
fn canonical_nanos(value: &str) -> Result<()> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("Adapter registry timestamp is not canonical UTC nanoseconds");
    }
    Ok(())
}
