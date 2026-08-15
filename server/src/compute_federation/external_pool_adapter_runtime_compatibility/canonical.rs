use anyhow::Result;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::{
    ExternalPoolAdapterRuntimeCompatibilityCandidateMaterial,
    ExternalPoolAdapterRuntimeCompatibilityChallengeMaterial,
    ExternalPoolAdapterRuntimeCompatibilityProfile,
};

const PROFILE_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-RUNTIME-COMPATIBILITY-PROFILE-V1";
const CHALLENGE_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-RUNTIME-COMPATIBILITY-CHALLENGE-V1";
const REPORT_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-RUNTIME-COMPATIBILITY-CANDIDATE-REPORT-V1";
const ELNW_ROOT_DOMAIN: &[u8] = b"elon.external_pool_adapter.no_work_probe.root.v1\0";
const MAX_COMPATIBILITY_JSON_BYTES: usize = 128 * 1024;

pub(crate) fn runtime_compatibility_profile_digest(
    profile: &ExternalPoolAdapterRuntimeCompatibilityProfile,
) -> Result<String> {
    canonical_digest(PROFILE_DOMAIN, profile)
}

pub(crate) fn runtime_compatibility_challenge_digest(
    challenge: &ExternalPoolAdapterRuntimeCompatibilityChallengeMaterial,
) -> Result<String> {
    canonical_digest(CHALLENGE_DOMAIN, challenge)
}

pub(crate) fn runtime_compatibility_candidate_report_digest(
    report: &ExternalPoolAdapterRuntimeCompatibilityCandidateMaterial,
) -> Result<String> {
    canonical_digest(REPORT_DOMAIN, report)
}

pub(crate) fn runtime_compatibility_profile_json<T: Serialize + ?Sized>(
    value: &T,
) -> Result<String> {
    canonical_compute_plugin_ijson_and_sha256(value, MAX_COMPATIBILITY_JSON_BYTES)
        .map(|(json, _)| json)
}

pub(crate) fn runtime_compatibility_elnw_root(
    nonce: &[u8; 32],
    request_bytes: u32,
    response_bytes: u32,
    request_sha256: &[u8; 32],
    response_sha256: &[u8; 32],
) -> String {
    let mut digest = Sha256::new();
    digest.update(ELNW_ROOT_DOMAIN);
    digest.update(nonce);
    digest.update(request_bytes.to_be_bytes());
    digest.update(response_bytes.to_be_bytes());
    digest.update(request_sha256);
    digest.update(response_sha256);
    hex::encode(digest.finalize())
}

fn canonical_digest<T: Serialize + ?Sized>(domain: &[u8], value: &T) -> Result<String> {
    let (json, _) = canonical_compute_plugin_ijson_and_sha256(value, MAX_COMPATIBILITY_JSON_BYTES)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(json.as_bytes());
    Ok(hex::encode(digest.finalize()))
}
