use std::collections::HashSet;

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{
    identity::ComputePluginReleaseRef,
    keyring::{
        ComputePluginControlPlaneKeyResolver, ComputePluginKeyringBinding,
        ComputePluginPublisherKeyResolver,
    },
    manifest_validation::{is_sha256, verify_and_validate_manifest},
    plugin_manifest::{
        ComputePluginSignature, SignedComputePluginManifest, COMPUTE_PLUGIN_DIGEST_ALGORITHM,
        COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION, COMPUTE_PLUGIN_SIGNATURE_ALGORITHM,
    },
    signed_artifact_verification::{jcs_sha256_hex, verify_control_plane_jcs_signature},
};

pub(in crate::node_agent_compute_plugin_host) const COMPUTE_PLUGIN_MANIFEST_CATALOG_SCHEMA: &str =
    "elon.compute_plugin.manifest_catalog.v1";
pub(in crate::node_agent_compute_plugin_host) const SIGNED_COMPUTE_PLUGIN_MANIFEST_CATALOG_SCHEMA: &str =
    "elon.compute_plugin.signed_manifest_catalog.v1";
pub(in crate::node_agent_compute_plugin_host) const COMPUTE_PLUGIN_MANIFEST_CATALOG_SIGNATURE_DOMAIN: &str =
    "ELON-COMPUTE-PLUGIN-MANIFEST-CATALOG-V1";
const COMPUTE_PLUGIN_MANIFEST_CATALOG_SET_SCHEMA: &str =
    "elon.compute_plugin.manifest_catalog_signed_manifest_set.v1";
pub(in crate::node_agent_compute_plugin_host) const MAX_COMPUTE_PLUGIN_MANIFEST_CATALOG_ENTRIES:
    usize = 256;
pub(in crate::node_agent_compute_plugin_host) const MAX_COMPUTE_PLUGIN_MANIFEST_CATALOG_JSON_BYTES: usize =
    4 * 1024 * 1024;
pub(in crate::node_agent_compute_plugin_host) const MAX_COMPUTE_PLUGIN_MANIFEST_CATALOG_SOURCE_JSON_BYTES: usize =
    4 * 1024 * 1024;
const MAX_REQUEST_ID_BYTES: usize = 160;
const I_JSON_MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

/// A caller-supplied proposal is not authority. The Store must verify every signed Manifest again
/// against the currently active Publisher ring inside the same transaction that binds the
/// catalog. Keeping this DTO separate prevents a pre-validated vector from becoming a capability.
#[derive(Debug)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginManifestCatalogCandidate {
    request_id: String,
    signed_catalog: SignedComputePluginManifestCatalog,
    signed_manifests: Vec<SignedComputePluginManifest>,
}

impl ComputePluginManifestCatalogCandidate {
    pub(in crate::node_agent_compute_plugin_host) fn new(
        request_id: String,
        signed_catalog: SignedComputePluginManifestCatalog,
        signed_manifests: Vec<SignedComputePluginManifest>,
    ) -> Result<Self> {
        if !identifier_is_valid(&request_id, MAX_REQUEST_ID_BYTES)
            || signed_catalog.catalog.catalog_revision <= 0
            || signed_catalog.catalog.catalog_revision > I_JSON_MAX_SAFE_INTEGER
            || signed_manifests.len() > MAX_COMPUTE_PLUGIN_MANIFEST_CATALOG_ENTRIES
        {
            bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_CANDIDATE_INVALID");
        }
        Ok(Self {
            request_id,
            signed_catalog,
            signed_manifests,
        })
    }

    pub(super) fn request_id(&self) -> &str {
        &self.request_id
    }

    pub(super) fn catalog_revision(&self) -> i64 {
        self.signed_catalog.catalog.catalog_revision
    }

    pub(super) fn signed_catalog(&self) -> &SignedComputePluginManifestCatalog {
        &self.signed_catalog
    }

    pub(super) fn signed_manifests(&self) -> &[SignedComputePluginManifest] {
        &self.signed_manifests
    }
}

/// Canonical release entry derived from one Publisher-signed Manifest. No local path, download
/// URL, runtime secret or rollback witness is valid catalog content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginManifestCatalogEntry {
    pub release: ComputePluginReleaseRef,
    pub publisher_id: String,
    pub signing_key_id: String,
    pub signing_key_fingerprint: String,
    pub signed_manifest_envelope_digest: String,
}

/// The catalog digest is portable across installations. Installation, node profile, authority
/// revisions and trusted time belong to the activation receipt, not this content payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginManifestCatalog {
    pub schema: String,
    pub catalog_revision: i64,
    pub target_id: String,
    pub host_api_protocol_id: String,
    pub host_api_revision: u32,
    pub keyring_bundle_revision: i64,
    pub publisher_keyring: ComputePluginKeyringBinding,
    pub control_keyring: ComputePluginKeyringBinding,
    pub entries: Vec<ComputePluginManifestCatalogEntry>,
}

/// The Control ring authorizes the exact catalog revision and ordered Publisher envelope set.
/// The signature is outside the canonical payload and uses a catalog-specific domain separator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(in crate::node_agent_compute_plugin_host) struct SignedComputePluginManifestCatalog {
    pub schema: String,
    pub catalog: ComputePluginManifestCatalog,
    pub canonicalization: String,
    pub catalog_digest_algorithm: String,
    pub catalog_digest: String,
    pub signature: ComputePluginSignature,
}

/// Private result of verifying every source envelope against one exact active keyring snapshot.
/// It is intentionally non-serializable and cannot be constructed from loose digest strings.
pub(super) struct ValidatedComputePluginManifestCatalog {
    catalog: ComputePluginManifestCatalog,
    catalog_json: String,
    catalog_digest: String,
    signed_catalog_json: String,
    signed_catalog_envelope_digest: String,
    control_signing_key_id: String,
    control_signing_key_fingerprint: String,
    signed_manifests_json: String,
    signed_manifest_set_digest: String,
}

impl ValidatedComputePluginManifestCatalog {
    pub(super) fn catalog(&self) -> &ComputePluginManifestCatalog {
        &self.catalog
    }

    pub(super) fn catalog_json(&self) -> &str {
        &self.catalog_json
    }

    pub(super) fn catalog_digest(&self) -> &str {
        &self.catalog_digest
    }

    pub(super) fn signed_catalog_json(&self) -> &str {
        &self.signed_catalog_json
    }

    pub(super) fn signed_catalog_envelope_digest(&self) -> &str {
        &self.signed_catalog_envelope_digest
    }

    pub(super) fn control_signing_key_id(&self) -> &str {
        &self.control_signing_key_id
    }

    pub(super) fn control_signing_key_fingerprint(&self) -> &str {
        &self.control_signing_key_fingerprint
    }

    pub(super) fn signed_manifests_json(&self) -> &str {
        &self.signed_manifests_json
    }

    pub(super) fn signed_manifest_set_digest(&self) -> &str {
        &self.signed_manifest_set_digest
    }
}

#[derive(Serialize)]
struct SignedManifestSetDigest<'a> {
    schema: &'static str,
    signed_manifest_envelope_digests: &'a [String],
}

#[allow(clippy::too_many_arguments)]
pub(super) fn verify_manifest_catalog_candidate(
    candidate: &ComputePluginManifestCatalogCandidate,
    target_id: &str,
    host_api_protocol_id: &str,
    host_api_revision: u32,
    keyring_bundle_revision: i64,
    publisher_keyring: &ComputePluginKeyringBinding,
    control_keyring: &ComputePluginKeyringBinding,
    trusted_now: DateTime<Utc>,
    publisher_keys: &dyn ComputePluginPublisherKeyResolver,
    control_keys: &dyn ComputePluginControlPlaneKeyResolver,
) -> Result<ValidatedComputePluginManifestCatalog> {
    if !identifier_is_valid(target_id, 200)
        || !identifier_is_valid(host_api_protocol_id, 200)
        || keyring_bundle_revision <= 0
        || keyring_bundle_revision > I_JSON_MAX_SAFE_INTEGER
        || publisher_keyring.revision <= 0
        || publisher_keyring.revision > I_JSON_MAX_SAFE_INTEGER
        || control_keyring.revision <= 0
        || control_keyring.revision > I_JSON_MAX_SAFE_INTEGER
        || host_api_revision == 0
        || !is_sha256(&publisher_keyring.digest)
        || !is_sha256(&control_keyring.digest)
    {
        bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_AUTHORITY_BINDING_INVALID");
    }

    let mut sources = Vec::with_capacity(candidate.signed_manifests().len());
    for signed in candidate.signed_manifests() {
        let validated = verify_and_validate_manifest(
            signed,
            publisher_keyring,
            trusted_now.clone(),
            publisher_keys,
        )?;
        let manifest = validated.manifest();
        if manifest.target.target_id != target_id
            || manifest.host_api.protocol_id != host_api_protocol_id
            || host_api_revision < manifest.host_api.minimum_revision
            || host_api_revision > manifest.host_api.maximum_revision
        {
            bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_TARGET_MISMATCH");
        }
        sources.push((
            ComputePluginManifestCatalogEntry {
                release: validated.release_ref(),
                publisher_id: manifest.publisher_id.clone(),
                signing_key_id: signed.signature.signing_key_id.clone(),
                signing_key_fingerprint: validated.verification_key_fingerprint().to_string(),
                signed_manifest_envelope_digest: jcs_sha256_hex(signed)?,
            },
            signed.clone(),
        ));
    }
    sources.sort_by(|(left, _), (right, _)| {
        release_identity(&left.release).cmp(&release_identity(&right.release))
    });
    let mut release_identities = HashSet::with_capacity(sources.len());
    let mut envelope_digests = HashSet::with_capacity(sources.len());
    if sources.iter().any(|(entry, _)| {
        !release_identities.insert(release_identity(&entry.release))
            || !envelope_digests.insert(entry.signed_manifest_envelope_digest.clone())
            || !is_sha256(&entry.signing_key_fingerprint)
            || !is_sha256(&entry.signed_manifest_envelope_digest)
    }) {
        bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_DUPLICATE_OR_INVALID_ENTRY");
    }
    let (entries, signed_manifests): (Vec<_>, Vec<_>) = sources.into_iter().unzip();

    let catalog = ComputePluginManifestCatalog {
        schema: COMPUTE_PLUGIN_MANIFEST_CATALOG_SCHEMA.to_string(),
        catalog_revision: candidate.catalog_revision(),
        target_id: target_id.to_string(),
        host_api_protocol_id: host_api_protocol_id.to_string(),
        host_api_revision,
        keyring_bundle_revision,
        publisher_keyring: publisher_keyring.clone(),
        control_keyring: control_keyring.clone(),
        entries,
    };
    let signed_catalog = candidate.signed_catalog();
    if signed_catalog.schema != SIGNED_COMPUTE_PLUGIN_MANIFEST_CATALOG_SCHEMA
        || signed_catalog.catalog != catalog
        || signed_catalog.catalog_digest != jcs_sha256_hex(&catalog)?
    {
        bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_CONTROL_ENVELOPE_MISMATCH");
    }
    let control_signing_key_fingerprint = verify_control_plane_jcs_signature(
        &signed_catalog.catalog,
        &signed_catalog.canonicalization,
        &signed_catalog.catalog_digest_algorithm,
        &signed_catalog.catalog_digest,
        &signed_catalog.signature,
        COMPUTE_PLUGIN_MANIFEST_CATALOG_SIGNATURE_DOMAIN,
        control_keyring,
        trusted_now,
        control_keys,
    )?;
    let catalog_json =
        serde_json::to_string(&catalog).context("COMPUTE_PLUGIN_MANIFEST_CATALOG_JSON")?;
    let signed_catalog_json = serde_json::to_string(signed_catalog)
        .context("COMPUTE_PLUGIN_SIGNED_MANIFEST_CATALOG_JSON")?;
    let signed_manifests_json = serde_json::to_string(&signed_manifests)
        .context("COMPUTE_PLUGIN_MANIFEST_CATALOG_SOURCES_JSON")?;
    if catalog_json.len() > MAX_COMPUTE_PLUGIN_MANIFEST_CATALOG_JSON_BYTES
        || signed_catalog_json.len() > MAX_COMPUTE_PLUGIN_MANIFEST_CATALOG_SOURCE_JSON_BYTES
        || signed_manifests_json.len() > MAX_COMPUTE_PLUGIN_MANIFEST_CATALOG_SOURCE_JSON_BYTES
    {
        bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_OVERSIZED");
    }
    if signed_catalog.signature.algorithm != COMPUTE_PLUGIN_SIGNATURE_ALGORITHM
        || signed_catalog.canonicalization != COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION
        || signed_catalog.catalog_digest_algorithm != COMPUTE_PLUGIN_DIGEST_ALGORITHM
        || !is_sha256(&control_signing_key_fingerprint)
    {
        bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_CONTROL_ENVELOPE_INVALID");
    }
    let catalog_digest = jcs_sha256_hex(&catalog)?;
    let signed_catalog_envelope_digest = jcs_sha256_hex(signed_catalog)?;
    let signed_manifest_envelope_digests = catalog
        .entries
        .iter()
        .map(|entry| entry.signed_manifest_envelope_digest.clone())
        .collect::<Vec<_>>();
    let signed_manifest_set_digest = jcs_sha256_hex(&SignedManifestSetDigest {
        schema: COMPUTE_PLUGIN_MANIFEST_CATALOG_SET_SCHEMA,
        signed_manifest_envelope_digests: &signed_manifest_envelope_digests,
    })?;
    Ok(ValidatedComputePluginManifestCatalog {
        catalog,
        catalog_json,
        catalog_digest,
        signed_catalog_json,
        signed_catalog_envelope_digest,
        control_signing_key_id: signed_catalog.signature.signing_key_id.clone(),
        control_signing_key_fingerprint,
        signed_manifests_json,
        signed_manifest_set_digest,
    })
}

/// Rebuilds every relationship that can be proven from the immutable stored source envelopes.
/// Cryptographic key validity is checked during binding/replay under the live keyring; this helper
/// makes later receipt reads fail closed on JSON, ordering, digest or source-set drift.
#[allow(clippy::too_many_arguments)]
pub(super) fn validate_persisted_manifest_catalog_sources(
    catalog_json: &str,
    signed_catalog_json: &str,
    signed_catalog_envelope_digest: &str,
    control_signing_key_id: &str,
    control_signing_key_fingerprint: &str,
    signed_manifests_json: &str,
    signed_manifest_set_digest: &str,
) -> Result<ComputePluginManifestCatalog> {
    if catalog_json.len() > MAX_COMPUTE_PLUGIN_MANIFEST_CATALOG_JSON_BYTES
        || signed_catalog_json.len() > MAX_COMPUTE_PLUGIN_MANIFEST_CATALOG_SOURCE_JSON_BYTES
        || signed_manifests_json.len() > MAX_COMPUTE_PLUGIN_MANIFEST_CATALOG_SOURCE_JSON_BYTES
        || !is_sha256(signed_catalog_envelope_digest)
        || !is_sha256(control_signing_key_fingerprint)
        || !is_sha256(signed_manifest_set_digest)
    {
        bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_STORED_SOURCE_INVALID");
    }
    let catalog: ComputePluginManifestCatalog = serde_json::from_str(catalog_json)
        .context("COMPUTE_PLUGIN_MANIFEST_CATALOG_STORED_JSON")?;
    let signed_catalog: SignedComputePluginManifestCatalog =
        serde_json::from_str(signed_catalog_json)
            .context("COMPUTE_PLUGIN_SIGNED_MANIFEST_CATALOG_STORED_JSON")?;
    let signed_manifests: Vec<SignedComputePluginManifest> =
        serde_json::from_str(signed_manifests_json)
            .context("COMPUTE_PLUGIN_MANIFEST_CATALOG_SOURCES_STORED_JSON")?;
    if catalog.schema != COMPUTE_PLUGIN_MANIFEST_CATALOG_SCHEMA
        || signed_catalog.schema != SIGNED_COMPUTE_PLUGIN_MANIFEST_CATALOG_SCHEMA
        || signed_catalog.catalog != catalog
        || signed_catalog.canonicalization != COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION
        || signed_catalog.catalog_digest_algorithm != COMPUTE_PLUGIN_DIGEST_ALGORITHM
        || signed_catalog.catalog_digest != jcs_sha256_hex(&catalog)?
        || signed_catalog.signature.algorithm != COMPUTE_PLUGIN_SIGNATURE_ALGORITHM
        || signed_catalog.signature.signing_key_id != control_signing_key_id
        || jcs_sha256_hex(&signed_catalog)? != signed_catalog_envelope_digest
        || signed_manifests.len() != catalog.entries.len()
        || catalog.entries.len() > MAX_COMPUTE_PLUGIN_MANIFEST_CATALOG_ENTRIES
    {
        bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_STORED_SOURCE_MISMATCH");
    }
    let mut previous_release = None;
    let mut release_identities = HashSet::with_capacity(catalog.entries.len());
    let mut envelope_digests = HashSet::with_capacity(catalog.entries.len());
    for (entry, signed) in catalog.entries.iter().zip(&signed_manifests) {
        let identity = release_identity(&entry.release);
        let envelope_digest = jcs_sha256_hex(signed)?;
        let source_release = ComputePluginReleaseRef {
            plugin_id: signed.manifest.plugin_id.clone(),
            plugin_version: signed.manifest.plugin_version.clone(),
            target_id: signed.manifest.target.target_id.clone(),
            manifest_digest: signed.manifest_digest.clone(),
            package_digest: signed.manifest.package.package_digest.clone(),
        };
        if previous_release
            .as_ref()
            .is_some_and(|previous| previous >= &identity)
            || !release_identities.insert(identity.clone())
            || !envelope_digests.insert(envelope_digest.clone())
            || entry.release != source_release
            || entry.publisher_id != signed.manifest.publisher_id
            || entry.signing_key_id != signed.signature.signing_key_id
            || entry.signed_manifest_envelope_digest != envelope_digest
            || !is_sha256(&entry.signing_key_fingerprint)
        {
            bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_STORED_ENTRY_MISMATCH");
        }
        previous_release = Some(identity);
    }
    let ordered_digests = catalog
        .entries
        .iter()
        .map(|entry| entry.signed_manifest_envelope_digest.clone())
        .collect::<Vec<_>>();
    if jcs_sha256_hex(&SignedManifestSetDigest {
        schema: COMPUTE_PLUGIN_MANIFEST_CATALOG_SET_SCHEMA,
        signed_manifest_envelope_digests: &ordered_digests,
    })? != signed_manifest_set_digest
    {
        bail!("COMPUTE_PLUGIN_MANIFEST_CATALOG_STORED_SET_DIGEST_MISMATCH");
    }
    Ok(catalog)
}

fn release_identity(release: &ComputePluginReleaseRef) -> String {
    format!(
        "{}\0{}\0{}\0{}\0{}",
        release.plugin_id,
        release.plugin_version,
        release.target_id,
        release.manifest_digest,
        release.package_digest,
    )
}

fn identifier_is_valid(value: &str, maximum_bytes: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum_bytes
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}
