use anyhow::{bail, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::*;

const PROFILE_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-RUNTIME-COMPATIBILITY-PROFILE-V2";
const RUNNER_POLICY_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-RUNTIME-COMPATIBILITY-RUNNER-POLICY-V1";
const FIXTURE_CATALOG_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-RUNTIME-COMPATIBILITY-FIXTURE-CATALOG-V1";
const CHALLENGE_MATERIAL_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-RUNTIME-COMPATIBILITY-CHALLENGE-MATERIAL-V1";
const CHALLENGE_RECEIPT_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-RUNTIME-COMPATIBILITY-CHALLENGE-RECEIPT-V1";
const OBSERVATION_MATERIAL_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-RUNTIME-COMPATIBILITY-RUN-OBSERVATION-MATERIAL-V1";
const OBSERVATION_RECEIPT_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-RUNTIME-COMPATIBILITY-RUN-OBSERVATION-RECEIPT-V1";
const SIGNATURE_MESSAGE_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-RUNTIME-COMPATIBILITY-SIGNATURE-MESSAGE-V1";
const VERIFICATION_MATERIAL_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-RUNTIME-COMPATIBILITY-VERIFICATION-MATERIAL-V1";
const VERIFICATION_RECEIPT_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-RUNTIME-COMPATIBILITY-VERIFICATION-RECEIPT-V1";
const REVOCATION_MATERIAL_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-RUNTIME-COMPATIBILITY-REVOCATION-MATERIAL-V1";
const REVOCATION_RECEIPT_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-RUNTIME-COMPATIBILITY-REVOCATION-RECEIPT-V1";

pub(crate) fn runtime_compatibility_profile_digest(
    value: &ExternalPoolAdapterRuntimeCompatibilityProfileV2,
) -> Result<String> {
    domain_digest(PROFILE_DOMAIN, value)
}

pub(crate) fn runtime_compatibility_runner_policy_digest(
    value: &ExternalPoolAdapterRuntimeCompatibilityRunnerPolicy,
) -> Result<String> {
    domain_digest(RUNNER_POLICY_DOMAIN, value)
}

pub(crate) fn runtime_compatibility_fixture_catalog_digest(
    value: &ExternalPoolAdapterRuntimeCompatibilityPublicFixtureCatalog,
) -> Result<String> {
    domain_digest(FIXTURE_CATALOG_DOMAIN, value)
}

pub(crate) fn runtime_compatibility_challenge_material_digest(
    value: &ExternalPoolAdapterRuntimeCompatibilityChallengeMaterial,
) -> Result<String> {
    domain_digest(CHALLENGE_MATERIAL_DOMAIN, value)
}

pub(crate) fn runtime_compatibility_challenge_json_and_digest(
    value: &ExternalPoolAdapterRuntimeCompatibilityChallengeReceipt,
) -> Result<(String, String)> {
    receipt_digest(value, "challenge_digest", CHALLENGE_RECEIPT_DOMAIN)
}

pub(crate) fn runtime_compatibility_observation_material_digest(
    value: &ExternalPoolAdapterRuntimeCompatibilityServerRunObservationMaterial,
) -> Result<String> {
    domain_digest(OBSERVATION_MATERIAL_DOMAIN, value)
}

pub(crate) fn runtime_compatibility_observation_json_and_digest(
    value: &ExternalPoolAdapterRuntimeCompatibilityRunObservationReceipt,
) -> Result<(String, String)> {
    receipt_digest(value, "run_observation_digest", OBSERVATION_RECEIPT_DOMAIN)
}

pub(crate) fn runtime_compatibility_verification_material_digest(
    value: &ExternalPoolAdapterRuntimeCompatibilityVerificationMaterial,
) -> Result<String> {
    domain_digest(VERIFICATION_MATERIAL_DOMAIN, value)
}

pub(crate) fn runtime_compatibility_verification_receipt_json_and_digest(
    value: &ExternalPoolAdapterRuntimeCompatibilityVerificationReceipt,
) -> Result<(String, String)> {
    receipt_digest(
        value,
        "verification_receipt_digest",
        VERIFICATION_RECEIPT_DOMAIN,
    )
}

pub(crate) fn runtime_compatibility_revocation_material_digest(
    value: &ExternalPoolAdapterRuntimeCompatibilityRevocationMaterial,
) -> Result<String> {
    domain_digest(REVOCATION_MATERIAL_DOMAIN, value)
}

pub(crate) fn runtime_compatibility_revocation_receipt_json_and_digest(
    value: &ExternalPoolAdapterRuntimeCompatibilityRevocationReceipt,
) -> Result<(String, String)> {
    receipt_digest(
        value,
        "revocation_receipt_digest",
        REVOCATION_RECEIPT_DOMAIN,
    )
}

pub(crate) fn runtime_compatibility_signature_challenge(
    challenge: &ExternalPoolAdapterRuntimeCompatibilityChallengeReceipt,
    observation: &ExternalPoolAdapterRuntimeCompatibilityRunObservationReceipt,
) -> Result<ExternalPoolAdapterRuntimeCompatibilitySignatureChallenge> {
    validate_runtime_compatibility_challenge_receipt(challenge)?;
    validate_runtime_compatibility_run_observation_receipt(observation)?;
    validate_runtime_compatibility_observation_against_challenge(
        &observation.observation,
        challenge,
    )?;
    let message = runtime_compatibility_signature_message(challenge, observation)?;
    Ok(ExternalPoolAdapterRuntimeCompatibilitySignatureChallenge {
        schema: RUNTIME_COMPATIBILITY_VERIFICATION_SIGNATURE_CHALLENGE_SCHEMA,
        canonicalization: RUNTIME_COMPATIBILITY_VERIFICATION_CANONICALIZATION,
        digest_algorithm: RUNTIME_COMPATIBILITY_VERIFICATION_DIGEST_ALGORITHM,
        signature_algorithm: RUNTIME_COMPATIBILITY_VERIFICATION_SIGNATURE_ALGORITHM,
        signature_message_base64: STANDARD.encode(&message),
        signature_message_digest: hex::encode(Sha256::digest(&message)),
    })
}

pub(super) fn runtime_compatibility_signature_message(
    challenge: &ExternalPoolAdapterRuntimeCompatibilityChallengeReceipt,
    observation: &ExternalPoolAdapterRuntimeCompatibilityRunObservationReceipt,
) -> Result<Vec<u8>> {
    let selected = &challenge.challenge;
    let run = &observation.observation;
    let release = &selected.registry_release;
    let mut message = Vec::with_capacity(1024);
    message.extend_from_slice(SIGNATURE_MESSAGE_DOMAIN);
    message.push(0);
    for (name, value) in [
        ("challenge_digest", challenge.challenge_digest.as_str()),
        ("runner_execution_id", run.runner_execution_id.as_str()),
        (
            "run_observation_digest",
            observation.run_observation_digest.as_str(),
        ),
        (
            "challenge_nonce_digest",
            selected.challenge_nonce_digest.as_str(),
        ),
        ("profile_digest", selected.profile_digest.as_str()),
        (
            "runner_policy_digest",
            selected.runner_policy.policy_digest.as_str(),
        ),
        (
            "fixture_catalog_digest",
            selected.fixture_catalog.policy_digest.as_str(),
        ),
        (
            "sandbox_verifier_key_record_digest",
            selected.sandbox_verifier_key_record_digest.as_str(),
        ),
        (
            "sandbox_verifier_key_id",
            selected.sandbox_verifier_key_id.as_str(),
        ),
        (
            "sandbox_verifier_operator",
            selected.sandbox_verifier_operator.as_str(),
        ),
        (
            "sandbox_verifier_product",
            selected.sandbox_verifier_product.as_str(),
        ),
        ("signature_algorithm", selected.signature_algorithm.as_str()),
        (
            "registry_release_digest",
            release.registry_release_digest.as_str(),
        ),
        (
            "registry_release_material_digest",
            release.registry_release_material_digest.as_str(),
        ),
        (
            "installation_content_digest",
            release.release.installation_content_digest.as_str(),
        ),
        ("source_capsule_sha256", run.source_capsule_sha256.as_str()),
        (
            "source_capsule_policy_digest",
            run.source_capsule_policy_digest.as_str(),
        ),
        ("launch_image_sha256", run.launch_image_sha256.as_str()),
        (
            "public_fixture_delivery_root",
            run.public_fixture_delivery_root.as_str(),
        ),
    ] {
        append_frame(&mut message, name.as_bytes())?;
        append_frame(&mut message, value.as_bytes())?;
    }
    for (name, value) in [
        ("source_capsule_size_bytes", run.source_capsule_size_bytes),
        ("launch_image_size_bytes", run.launch_image_size_bytes),
    ] {
        append_frame(&mut message, name.as_bytes())?;
        append_frame(&mut message, &value.to_be_bytes())?;
    }
    Ok(message)
}

pub(crate) fn canonical_runtime_compatibility_verification_json<T: Serialize + ?Sized>(
    value: &T,
) -> Result<String> {
    canonical_json(value)
}

fn append_frame(target: &mut Vec<u8>, value: &[u8]) -> Result<()> {
    let length = u32::try_from(value.len())?;
    target.extend_from_slice(&length.to_be_bytes());
    target.extend_from_slice(value);
    Ok(())
}

fn receipt_digest<T: Serialize>(value: &T, field: &str, domain: &[u8]) -> Result<(String, String)> {
    let json = canonical_json(value)?;
    let mut projection = serde_json::to_value(value)?
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("runtime compatibility receipt must be an object"))?
        .clone();
    if projection
        .insert(field.into(), serde_json::Value::String(String::new()))
        .is_none()
    {
        bail!("runtime compatibility receipt lacks digest field");
    }
    Ok((json, domain_digest(domain, &projection)?))
}

fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    canonical_compute_plugin_ijson_and_sha256(
        value,
        RUNTIME_COMPATIBILITY_VERIFICATION_MAX_RECEIPT_JSON_BYTES,
    )
    .map(|(json, _)| json)
}

fn domain_digest<T: Serialize + ?Sized>(domain: &[u8], value: &T) -> Result<String> {
    let json = canonical_json(value)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(json.as_bytes());
    Ok(hex::encode(digest.finalize()))
}
