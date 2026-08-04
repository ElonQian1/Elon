use std::collections::HashSet;

use anyhow::{bail, Result};
use chrono::{DateTime, Duration, SecondsFormat, Utc};

use super::{
    keyring::{
        ComputePluginBootstrapRootKeyResolver, ComputePluginControlPlaneKeyResolver,
        ComputePluginKeyring, ComputePluginKeyringBinding, ComputePluginKeyringKey,
        ComputePluginPublisherKeyResolver, ResolvedComputePluginVerificationKey,
        SignedComputePluginKeyringBundle, ValidatedComputePluginKeyringBundle,
        COMPUTE_PLUGIN_KEYRING_BUNDLE_SCHEMA, COMPUTE_PLUGIN_KEYRING_SCHEMA,
        COMPUTE_PLUGIN_KEYRING_SIGNATURE_DOMAIN, KEY_PURPOSE_CONTROL_INSTALL_PLAN,
        KEY_PURPOSE_PUBLISHER_MANIFEST, KEY_STATUS_ACTIVE, KEY_STATUS_REVOKED,
        SIGNED_COMPUTE_PLUGIN_KEYRING_BUNDLE_SCHEMA,
    },
    plugin_manifest::COMPUTE_PLUGIN_SIGNATURE_ALGORITHM,
    signed_artifact_verification::{jcs_sha256_hex, verify_jcs_ed25519},
};

const MAX_KEYS_PER_RING: usize = 4_096;
const MAX_BUNDLE_LIFETIME_DAYS: i64 = 366;
const MAX_GENERATED_AT_SKEW_MINUTES: i64 = 5;
const MAX_PUBLISHER_ID_BYTES: usize = 200;
const MAX_SIGNING_KEY_ID_BYTES: usize = 160;

pub(crate) fn verify_and_validate_keyring_bundle(
    signed: &SignedComputePluginKeyringBundle,
    trusted_now: DateTime<Utc>,
    roots: &dyn ComputePluginBootstrapRootKeyResolver,
) -> Result<ValidatedComputePluginKeyringBundle> {
    if signed.schema != SIGNED_COMPUTE_PLUGIN_KEYRING_BUNDLE_SCHEMA
        || signed.bundle.schema != COMPUTE_PLUGIN_KEYRING_BUNDLE_SCHEMA
        || signed.bundle.bundle_revision <= 0
    {
        bail!("COMPUTE_PLUGIN_KEYRING_SCHEMA: unsupported keyring bundle schema");
    }
    let root_key = roots
        .resolve_bootstrap_root_key(&signed.signature.signing_key_id)?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "COMPUTE_PLUGIN_KEYRING_ROOT_UNTRUSTED: Bootstrap root key is unavailable"
            )
        })?;
    verify_jcs_ed25519(
        &signed.bundle,
        &signed.canonicalization,
        &signed.bundle_digest_algorithm,
        &signed.bundle_digest,
        &signed.signature,
        COMPUTE_PLUGIN_KEYRING_SIGNATURE_DOMAIN,
        &root_key,
    )?;
    validate_bundle_window(signed, trusted_now, true)?;

    let mut fingerprints = HashSet::new();
    validate_ring(
        &signed.bundle.publisher_keyring,
        KEY_PURPOSE_PUBLISHER_MANIFEST,
        &signed.bundle.expires_at,
        &mut fingerprints,
    )?;
    validate_ring(
        &signed.bundle.control_keyring,
        KEY_PURPOSE_CONTROL_INSTALL_PLAN,
        &signed.bundle.expires_at,
        &mut fingerprints,
    )?;
    let publisher_binding = ring_binding(&signed.bundle.publisher_keyring)?;
    let control_binding = ring_binding(&signed.bundle.control_keyring)?;
    let root_key_fingerprint = root_key.fingerprint();
    if fingerprints.contains(&root_key_fingerprint) {
        bail!("COMPUTE_PLUGIN_KEYRING_ROOT_REUSE: Bootstrap root cannot be a leaf signing key");
    }
    Ok(ValidatedComputePluginKeyringBundle::new(
        signed.clone(),
        publisher_binding,
        control_binding,
        root_key_fingerprint,
    ))
}

impl ComputePluginPublisherKeyResolver for ValidatedComputePluginKeyringBundle {
    fn resolve_publisher_key(
        &self,
        publisher_id: &str,
        signing_key_id: &str,
        expected_keyring: &ComputePluginKeyringBinding,
        trusted_now: DateTime<Utc>,
    ) -> Result<Option<ResolvedComputePluginVerificationKey>> {
        resolve_key(
            self,
            &self.signed().bundle.publisher_keyring,
            self.publisher_binding(),
            expected_keyring,
            Some(publisher_id),
            signing_key_id,
            trusted_now,
        )
    }
}

impl ComputePluginControlPlaneKeyResolver for ValidatedComputePluginKeyringBundle {
    fn resolve_control_plane_key(
        &self,
        signing_key_id: &str,
        expected_keyring: &ComputePluginKeyringBinding,
        trusted_now: DateTime<Utc>,
    ) -> Result<Option<ResolvedComputePluginVerificationKey>> {
        resolve_key(
            self,
            &self.signed().bundle.control_keyring,
            self.control_binding(),
            expected_keyring,
            None,
            signing_key_id,
            trusted_now,
        )
    }
}

fn resolve_key(
    bundle: &ValidatedComputePluginKeyringBundle,
    ring: &ComputePluginKeyring,
    actual_binding: &ComputePluginKeyringBinding,
    expected_binding: &ComputePluginKeyringBinding,
    publisher_id: Option<&str>,
    signing_key_id: &str,
    trusted_now: DateTime<Utc>,
) -> Result<Option<ResolvedComputePluginVerificationKey>> {
    if actual_binding != expected_binding {
        bail!("COMPUTE_PLUGIN_KEYRING_BINDING_CHANGED: required keyring revision or digest is unavailable");
    }
    validate_bundle_window(bundle.signed(), trusted_now.clone(), false)?;
    let Some(record) = ring.keys.iter().find(|key| {
        key.publisher_id.as_deref() == publisher_id && key.signing_key_id == signing_key_id
    }) else {
        return Ok(None);
    };
    if record.status != KEY_STATUS_ACTIVE {
        return Ok(None);
    }
    let not_before = parse_canonical_utc("COMPUTE_PLUGIN_KEY_NOT_BEFORE", &record.not_before)?;
    let not_after = parse_canonical_utc("COMPUTE_PLUGIN_KEY_NOT_AFTER", &record.not_after)?;
    if trusted_now < not_before || trusted_now >= not_after {
        return Ok(None);
    }
    let key =
        super::signed_artifact_verification::ComputePluginEd25519PublicKey::from_standard_base64(
            &record.public_key_base64,
        )?;
    Ok(Some(ResolvedComputePluginVerificationKey::new(
        key,
        actual_binding.clone(),
        record.purpose.clone(),
        record.publisher_id.clone(),
        record.signing_key_id.clone(),
        record.fingerprint_sha256.clone(),
        not_before,
        not_after,
    )))
}

fn validate_bundle_window(
    signed: &SignedComputePluginKeyringBundle,
    trusted_now: DateTime<Utc>,
    allow_generated_at_skew: bool,
) -> Result<()> {
    let generated = parse_canonical_utc(
        "COMPUTE_PLUGIN_KEYRING_GENERATED_AT",
        &signed.bundle.generated_at,
    )?;
    let expires = parse_canonical_utc(
        "COMPUTE_PLUGIN_KEYRING_EXPIRES_AT",
        &signed.bundle.expires_at,
    )?;
    let earliest_now = if allow_generated_at_skew {
        generated
            .clone()
            .checked_sub_signed(Duration::minutes(MAX_GENERATED_AT_SKEW_MINUTES))
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_KEYRING_TIME_RANGE"))?
    } else {
        generated.clone()
    };
    if generated >= expires
        || expires - generated > Duration::days(MAX_BUNDLE_LIFETIME_DAYS)
        || trusted_now < earliest_now
        || trusted_now >= expires
    {
        bail!("COMPUTE_PLUGIN_KEYRING_EXPIRED: keyring bundle is not currently valid");
    }
    Ok(())
}

fn validate_ring(
    ring: &ComputePluginKeyring,
    expected_purpose: &str,
    bundle_expires_at: &str,
    fingerprints: &mut HashSet<String>,
) -> Result<()> {
    if ring.schema != COMPUTE_PLUGIN_KEYRING_SCHEMA
        || ring.purpose != expected_purpose
        || ring.revision <= 0
        || ring.keys.len() > MAX_KEYS_PER_RING
    {
        bail!("COMPUTE_PLUGIN_KEYRING_SHAPE: keyring is unsupported or oversized");
    }
    let bundle_expires =
        parse_canonical_utc("COMPUTE_PLUGIN_KEYRING_EXPIRES_AT", bundle_expires_at)?;
    let mut identities = HashSet::new();
    let mut previous_identity: Option<String> = None;
    for key in &ring.keys {
        validate_key(
            key,
            expected_purpose,
            &bundle_expires,
            &mut identities,
            fingerprints,
        )?;
        let identity = key_identity(key);
        if previous_identity
            .as_ref()
            .is_some_and(|prior| prior >= &identity)
        {
            bail!("COMPUTE_PLUGIN_KEYRING_ORDER: keys must be strictly identity-sorted");
        }
        previous_identity = Some(identity);
    }
    Ok(())
}

fn validate_key(
    key: &ComputePluginKeyringKey,
    expected_purpose: &str,
    bundle_expires: &DateTime<Utc>,
    identities: &mut HashSet<String>,
    fingerprints: &mut HashSet<String>,
) -> Result<()> {
    let publisher_shape_matches = match expected_purpose {
        KEY_PURPOSE_PUBLISHER_MANIFEST => key
            .publisher_id
            .as_deref()
            .is_some_and(|value| identifier_is_valid(value, MAX_PUBLISHER_ID_BYTES)),
        KEY_PURPOSE_CONTROL_INSTALL_PLAN => key.publisher_id.is_none(),
        _ => false,
    };
    let revoked_shape_matches = match key.status.as_str() {
        KEY_STATUS_ACTIVE => key.revoked_at.is_none(),
        KEY_STATUS_REVOKED => key.revoked_at.is_some(),
        _ => false,
    };
    if key.purpose != expected_purpose
        || key.algorithm != COMPUTE_PLUGIN_SIGNATURE_ALGORITHM
        || !publisher_shape_matches
        || !identifier_is_valid(&key.signing_key_id, MAX_SIGNING_KEY_ID_BYTES)
        || !revoked_shape_matches
        || !sha256_is_valid(&key.fingerprint_sha256)
    {
        bail!("COMPUTE_PLUGIN_KEYRING_KEY_SHAPE: key record is invalid");
    }
    let not_before = parse_canonical_utc("COMPUTE_PLUGIN_KEY_NOT_BEFORE", &key.not_before)?;
    let not_after = parse_canonical_utc("COMPUTE_PLUGIN_KEY_NOT_AFTER", &key.not_after)?;
    if not_before >= not_after || not_after > bundle_expires.clone() {
        bail!("COMPUTE_PLUGIN_KEYRING_KEY_WINDOW: key validity is outside bundle bounds");
    }
    if let Some(revoked_at) = &key.revoked_at {
        let revoked = parse_canonical_utc("COMPUTE_PLUGIN_KEY_REVOKED_AT", revoked_at)?;
        if revoked < not_before || revoked > not_after {
            bail!("COMPUTE_PLUGIN_KEYRING_REVOKED_AT: revocation time is outside key validity");
        }
    }
    let parsed =
        super::signed_artifact_verification::ComputePluginEd25519PublicKey::from_standard_base64(
            &key.public_key_base64,
        )?;
    if parsed.fingerprint() != key.fingerprint_sha256
        || !identities.insert(key_identity(key))
        || !fingerprints.insert(key.fingerprint_sha256.clone())
    {
        bail!(
            "COMPUTE_PLUGIN_KEYRING_KEY_DUPLICATE: key identity or public key fingerprint is reused"
        );
    }
    Ok(())
}

fn ring_binding(ring: &ComputePluginKeyring) -> Result<ComputePluginKeyringBinding> {
    Ok(ComputePluginKeyringBinding {
        revision: ring.revision,
        digest: jcs_sha256_hex(ring)?,
    })
}

fn key_identity(key: &ComputePluginKeyringKey) -> String {
    format!(
        "{}\0{}",
        key.publisher_id.as_deref().unwrap_or_default(),
        key.signing_key_id
    )
}

fn identifier_is_valid(value: &str, maximum_bytes: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum_bytes
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn sha256_is_valid(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn parse_canonical_utc(code: &str, value: &str) -> Result<DateTime<Utc>> {
    let parsed = DateTime::parse_from_rfc3339(value)
        .map_err(|_| anyhow::anyhow!("{code}: timestamp is not RFC 3339"))?;
    let utc = parsed.with_timezone(&Utc);
    if parsed.offset().local_minus_utc() != 0
        || utc.to_rfc3339_opts(SecondsFormat::Secs, true) != value
    {
        bail!("{code}: timestamp must be canonical UTC seconds");
    }
    Ok(utc)
}
