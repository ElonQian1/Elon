use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use super::types::{
    ExpectedExternalPoolAdapterRuntimeBundle, ExternalPoolAdapterRuntimeBundleError,
};
pub(super) const RUNTIME_BUNDLE_SCHEMA: &str =
    "compute_federation.external_pool_adapter_runtime_bundle.v1";
pub(super) const RUNTIME_BUNDLE_PURPOSE: &str = "external_pool_adapter_runtime_v1";
pub(super) const MAX_MANIFEST_BYTES: usize = 16 * 1024;
pub(super) const MAX_CONFIG_BYTES: u64 = 1_048_576;
pub(super) const MAX_CREDENTIAL_BYTES: u64 = 65_536;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExternalPoolAdapterRuntimeBundleManifest {
    // Field order is the RFC 8785 UTF-16 lexical order for this fixed ASCII schema. With only
    // strings and safe integers, serde_json therefore emits the exact JCS representation without
    // building a generic Value tree containing private content hashes.
    pub(super) adapter_config_digest: String,
    pub(super) adapter_config_revision: i64,
    pub(super) bundle_generation: u64,
    pub(super) candidate_digest: String,
    pub(super) candidate_id: String,
    pub(super) config_sha256: String,
    pub(super) config_size_bytes: u64,
    pub(super) credential_locator_commitment: String,
    pub(super) credential_reattestation_material_digest: String,
    pub(super) credential_reattestation_receipt_digest: String,
    pub(super) credential_reattestation_receipt_id: String,
    pub(super) credential_ref_scheme: String,
    pub(super) credential_report_expires_at: String,
    pub(super) credential_sha256: String,
    pub(super) credential_size_bytes: u64,
    pub(super) launch_policy_digest: String,
    pub(super) logical_adapter_id: String,
    pub(super) profile_digest: String,
    pub(super) profile_id: String,
    pub(super) provider_binding_digest: String,
    pub(super) provider_binding_id: String,
    pub(super) provider_id: String,
    pub(super) provider_owner_account_id: String,
    pub(super) purpose: String,
    pub(super) release_version: String,
    pub(super) schema: String,
}

impl Drop for ExternalPoolAdapterRuntimeBundleManifest {
    fn drop(&mut self) {
        // Wipe every heap-backed manifest value, especially content hashes that can serve as
        // offline oracles for low-entropy operator configuration or credential material.
        for value in [
            &mut self.adapter_config_digest,
            &mut self.candidate_digest,
            &mut self.candidate_id,
            &mut self.config_sha256,
            &mut self.credential_locator_commitment,
            &mut self.credential_reattestation_material_digest,
            &mut self.credential_reattestation_receipt_digest,
            &mut self.credential_reattestation_receipt_id,
            &mut self.credential_ref_scheme,
            &mut self.credential_report_expires_at,
            &mut self.credential_sha256,
            &mut self.launch_policy_digest,
            &mut self.logical_adapter_id,
            &mut self.profile_digest,
            &mut self.profile_id,
            &mut self.provider_binding_digest,
            &mut self.provider_binding_id,
            &mut self.provider_id,
            &mut self.provider_owner_account_id,
            &mut self.purpose,
            &mut self.release_version,
            &mut self.schema,
        ] {
            value.zeroize();
        }
    }
}

pub(super) fn parse_and_validate_manifest(
    raw: &[u8],
    expected: &ExpectedExternalPoolAdapterRuntimeBundle,
) -> Result<ExternalPoolAdapterRuntimeBundleManifest, ExternalPoolAdapterRuntimeBundleError> {
    if raw.is_empty() || raw.len() > MAX_MANIFEST_BYTES || raw.starts_with(&[0xef, 0xbb, 0xbf]) {
        return Err(ExternalPoolAdapterRuntimeBundleError::InvalidAuthority);
    }
    let text = std::str::from_utf8(raw)
        .map_err(|_| ExternalPoolAdapterRuntimeBundleError::InvalidAuthority)?;
    let manifest: ExternalPoolAdapterRuntimeBundleManifest = serde_json::from_str(text)
        .map_err(|_| ExternalPoolAdapterRuntimeBundleError::InvalidAuthority)?;
    let mut canonical = serde_json::to_string(&manifest)
        .map_err(|_| ExternalPoolAdapterRuntimeBundleError::InvalidAuthority)?;
    let canonical_matches = canonical.as_bytes() == raw;
    canonical.zeroize();
    if !canonical_matches || !manifest.matches(expected) {
        return Err(ExternalPoolAdapterRuntimeBundleError::InvalidAuthority);
    }
    Ok(manifest)
}

impl ExternalPoolAdapterRuntimeBundleManifest {
    fn matches(&self, expected: &ExpectedExternalPoolAdapterRuntimeBundle) -> bool {
        self.schema == RUNTIME_BUNDLE_SCHEMA
            && self.purpose == RUNTIME_BUNDLE_PURPOSE
            && (1..=9_007_199_254_740_991).contains(&self.bundle_generation)
            && self.profile_id == expected.profile_id
            && self.profile_digest == expected.profile_digest
            && self.launch_policy_digest == expected.launch_policy_digest
            && self.candidate_id == expected.candidate_id
            && self.candidate_digest == expected.candidate_digest
            && self.provider_binding_id == expected.provider_binding_id
            && self.provider_binding_digest == expected.provider_binding_digest
            && self.provider_id == expected.provider_id
            && self.provider_owner_account_id == expected.provider_owner_account_id
            && self.logical_adapter_id == expected.logical_adapter_id
            && self.release_version == expected.release_version
            && self.adapter_config_revision == expected.adapter_config_revision
            && self.adapter_config_digest == expected.adapter_config_digest
            && self.credential_ref_scheme == "vault_ref"
            && self.credential_locator_commitment == expected.credential_locator_commitment
            && self.credential_reattestation_receipt_id
                == expected.credential_reattestation_receipt_id
            && self.credential_reattestation_receipt_digest
                == expected.credential_reattestation_receipt_digest
            && self.credential_reattestation_material_digest
                == expected.credential_reattestation_material_digest
            && self.credential_report_expires_at == expected.credential_report_expires_at
            && (1..=MAX_CONFIG_BYTES).contains(&self.config_size_bytes)
            && (1..=MAX_CREDENTIAL_BYTES).contains(&self.credential_size_bytes)
            && is_sha256(&self.profile_digest)
            && is_sha256(&self.launch_policy_digest)
            && is_sha256(&self.candidate_digest)
            && is_sha256(&self.provider_binding_digest)
            && is_sha256(&self.credential_locator_commitment)
            && is_sha256(&self.credential_reattestation_receipt_digest)
            && is_sha256(&self.credential_reattestation_material_digest)
            && is_sha256(&self.config_sha256)
            && is_sha256(&self.credential_sha256)
            && (1..=9_007_199_254_740_991).contains(&self.adapter_config_revision)
            && bounded_opaque(&self.adapter_config_digest, 512)
            && bounded_opaque(&self.credential_report_expires_at, 128)
            && [
                &self.profile_id,
                &self.candidate_id,
                &self.provider_binding_id,
                &self.provider_id,
                &self.provider_owner_account_id,
                &self.logical_adapter_id,
                &self.release_version,
                &self.credential_reattestation_receipt_id,
            ]
            .into_iter()
            .all(|value| bounded_opaque(value, 256))
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn bounded_opaque(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod tests;
