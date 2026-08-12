use anyhow::{bail, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::external_pool_adapter_release::ComputeExternalPoolAdapterReleaseCapability,
    compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256,
};

use super::types::*;

const MAX_JSON_BYTES: usize = 1024 * 1024;
const CHALLENGE_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-SANDBOX-CONFORMANCE-V1";
const TEST_PLAN_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-SANDBOX-TEST-PLAN-V1";
const FIXTURE_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-SANDBOX-FIXTURE-V1";
const OBSERVATION_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-SANDBOX-OBSERVATIONS-V1";
const MATERIAL_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-SANDBOX-MATERIAL-V1";
const RECEIPT_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-SANDBOX-RECEIPT-V1";

#[derive(Serialize)]
struct FixtureProjection<'a> {
    admission_digest: &'a str,
    capability: &'a ComputeExternalPoolAdapterReleaseCapability,
    sandbox_policy_id: &'static str,
    isolation_profile_id: &'static str,
}

pub(crate) fn sandbox_capability_test_plan(
    admission_digest: &str,
    capabilities: &[ComputeExternalPoolAdapterReleaseCapability],
) -> Result<Vec<ExternalPoolAdapterSandboxCapabilityTest>> {
    capabilities
        .iter()
        .map(|capability| {
            let test_case_id = format!(
                "{}-contract-r{}-v1",
                capability.capability_id, capability.capability_revision
            );
            let input_fixture_digest = domain_digest(
                FIXTURE_DOMAIN,
                &FixtureProjection {
                    admission_digest,
                    capability,
                    sandbox_policy_id: SANDBOX_CONFORMANCE_POLICY_ID,
                    isolation_profile_id: SANDBOX_CONFORMANCE_ISOLATION_PROFILE_ID,
                },
            )?;
            Ok(ExternalPoolAdapterSandboxCapabilityTest {
                capability_id: capability.capability_id.clone(),
                capability_revision: capability.capability_revision,
                test_case_id,
                input_fixture_digest,
            })
        })
        .collect()
}

pub(crate) fn sandbox_test_plan_digest<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    domain_digest(TEST_PLAN_DOMAIN, value)
}

pub(crate) fn sandbox_observation_inventory_digest<T: Serialize + ?Sized>(
    value: &T,
) -> Result<String> {
    domain_digest(OBSERVATION_DOMAIN, value)
}

pub(crate) fn sandbox_conformance_challenge(
    binding: ExternalPoolAdapterSandboxConformanceBinding,
) -> Result<ExternalPoolAdapterSandboxConformanceChallenge> {
    let json = canonical_json(&binding)?;
    let mut message = Vec::with_capacity(CHALLENGE_DOMAIN.len() + 1 + json.len());
    message.extend_from_slice(CHALLENGE_DOMAIN);
    message.push(0);
    message.extend_from_slice(json.as_bytes());
    Ok(ExternalPoolAdapterSandboxConformanceChallenge {
        schema: SANDBOX_CONFORMANCE_CHALLENGE_SCHEMA,
        canonicalization: SANDBOX_CONFORMANCE_CANONICALIZATION,
        digest_algorithm: SANDBOX_CONFORMANCE_DIGEST_ALGORITHM,
        signature_algorithm: SANDBOX_CONFORMANCE_SIGNATURE_ALGORITHM,
        signature_message_base64: STANDARD.encode(&message),
        signature_message_digest: hex::encode(Sha256::digest(&message)),
        binding,
    })
}

pub(crate) fn sandbox_conformance_material_digest<T: Serialize + ?Sized>(
    value: &T,
) -> Result<String> {
    domain_digest(MATERIAL_DOMAIN, value)
}

pub(crate) fn canonical_sandbox_conformance_receipt_json_and_digest(
    receipt: &ExternalPoolAdapterSandboxConformanceReceipt,
) -> Result<(String, String)> {
    let value = serde_json::to_value(receipt)?;
    let mut projection = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("sandbox conformance receipt must be an object"))?
        .clone();
    if projection
        .insert(
            "sandbox_conformance_receipt_digest".to_string(),
            serde_json::Value::String(String::new()),
        )
        .is_none()
    {
        bail!("sandbox conformance receipt lacks digest field");
    }
    Ok((
        canonical_json(receipt)?,
        domain_digest(RECEIPT_DOMAIN, &projection)?,
    ))
}

fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    canonical_compute_plugin_ijson_and_sha256(value, MAX_JSON_BYTES).map(|(json, _)| json)
}

fn domain_digest<T: Serialize + ?Sized>(domain: &[u8], value: &T) -> Result<String> {
    let json = canonical_json(value)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(json.as_bytes());
    Ok(hex::encode(digest.finalize()))
}
