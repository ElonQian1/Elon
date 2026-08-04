use anyhow::{bail, Context, Result};
use chrono::DateTime;
use rusqlite::{params, Transaction};

use super::keyring_snapshot::ActiveKeyringBinding;
use crate::node_agent_compute_plugin_host::keyring::{
    ComputePluginKeyring, ValidatedComputePluginKeyringBundle,
};

pub(super) struct StoredBundleRow {
    pub bundle_digest: String,
    pub signed_envelope_digest: String,
    pub signed_bundle_json: String,
    pub root_signing_key_id: String,
    pub root_key_fingerprint: String,
    pub publisher_revision: i64,
    pub publisher_digest: String,
    pub control_revision: i64,
    pub control_digest: String,
    pub publisher_key_count: i64,
    pub control_key_count: i64,
    pub generated_at_ms: i64,
    pub expires_at_ms: i64,
    pub installed_at_ms: i64,
}

pub(super) fn verify_bundle_columns(
    active: &ActiveKeyringBinding,
    stored: &StoredBundleRow,
    validated: &ValidatedComputePluginKeyringBundle,
    signed_envelope_digest: &str,
) -> Result<()> {
    let signed = validated.signed();
    let publisher_count = i64::try_from(signed.bundle.publisher_keyring.keys.len())
        .context("COMPUTE_PLUGIN_AUTHORITY_KEYRING_KEY_COUNT")?;
    let control_count = i64::try_from(signed.bundle.control_keyring.keys.len())
        .context("COMPUTE_PLUGIN_AUTHORITY_KEYRING_KEY_COUNT")?;
    if signed.bundle.bundle_revision != active.bundle_revision
        || signed.bundle_digest != stored.bundle_digest
        || signed_envelope_digest != stored.signed_envelope_digest
        || signed.signature.signing_key_id != stored.root_signing_key_id
        || validated.root_key_fingerprint() != stored.root_key_fingerprint
        || validated.publisher_binding() != &active.publisher
        || validated.control_binding() != &active.control
        || stored.publisher_revision != active.publisher.revision
        || stored.publisher_digest != active.publisher.digest
        || stored.control_revision != active.control.revision
        || stored.control_digest != active.control.digest
        || stored.publisher_key_count != publisher_count
        || stored.control_key_count != control_count
        || stored.generated_at_ms != timestamp_millis(&signed.bundle.generated_at)?
        || stored.expires_at_ms != timestamp_millis(&signed.bundle.expires_at)?
    {
        bail!("COMPUTE_PLUGIN_AUTHORITY_KEYRING_BUNDLE_CORRUPT");
    }
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct StoredKeyRow {
    purpose: String,
    subject_id: String,
    signing_key_id: String,
    public_key_base64: String,
    fingerprint_sha256: String,
    status: String,
    not_before_ms: i64,
    not_after_ms: i64,
    revoked_at_ms: Option<i64>,
}

pub(super) fn verify_normalized_keys(
    transaction: &Transaction<'_>,
    bundle_revision: i64,
    validated: &ValidatedComputePluginKeyringBundle,
) -> Result<()> {
    let mut expected = normalized_ring_keys(&validated.signed().bundle.publisher_keyring)?;
    expected.extend(normalized_ring_keys(
        &validated.signed().bundle.control_keyring,
    )?);
    expected.sort_by(|left, right| key_sort_key(left).cmp(&key_sort_key(right)));
    let mut statement = transaction
        .prepare(
            r#"SELECT
                purpose, subject_id, signing_key_id, public_key_base64,
                fingerprint_sha256, status, not_before_ms, not_after_ms, revoked_at_ms
            FROM keyring_keys WHERE bundle_revision = ?1
            ORDER BY purpose, subject_id, signing_key_id"#,
        )
        .context("COMPUTE_PLUGIN_AUTHORITY_KEYRING_KEYS_PREPARE")?;
    let rows = statement
        .query_map(params![bundle_revision], |row| {
            Ok(StoredKeyRow {
                purpose: row.get(0)?,
                subject_id: row.get(1)?,
                signing_key_id: row.get(2)?,
                public_key_base64: row.get(3)?,
                fingerprint_sha256: row.get(4)?,
                status: row.get(5)?,
                not_before_ms: row.get(6)?,
                not_after_ms: row.get(7)?,
                revoked_at_ms: row.get(8)?,
            })
        })
        .context("COMPUTE_PLUGIN_AUTHORITY_KEYRING_KEYS_READ")?;
    let actual = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("COMPUTE_PLUGIN_AUTHORITY_KEYRING_KEYS_DECODE")?;
    if actual != expected {
        bail!("COMPUTE_PLUGIN_AUTHORITY_KEYRING_KEYS_CORRUPT");
    }
    Ok(())
}

fn normalized_ring_keys(ring: &ComputePluginKeyring) -> Result<Vec<StoredKeyRow>> {
    ring.keys
        .iter()
        .map(|key| {
            Ok(StoredKeyRow {
                purpose: key.purpose.clone(),
                subject_id: key.publisher_id.clone().unwrap_or_default(),
                signing_key_id: key.signing_key_id.clone(),
                public_key_base64: key.public_key_base64.clone(),
                fingerprint_sha256: key.fingerprint_sha256.clone(),
                status: key.status.clone(),
                not_before_ms: timestamp_millis(&key.not_before)?,
                not_after_ms: timestamp_millis(&key.not_after)?,
                revoked_at_ms: key
                    .revoked_at
                    .as_deref()
                    .map(timestamp_millis)
                    .transpose()?,
            })
        })
        .collect()
}

fn key_sort_key(key: &StoredKeyRow) -> (&str, &str, &str) {
    (&key.purpose, &key.subject_id, &key.signing_key_id)
}

pub(super) fn timestamp_millis(value: &str) -> Result<i64> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.timestamp_millis())
        .context("COMPUTE_PLUGIN_AUTHORITY_KEYRING_TIME")
}
