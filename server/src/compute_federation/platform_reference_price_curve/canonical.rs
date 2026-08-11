use anyhow::Result;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::types::{
    ComputePlatformReferencePriceCurveBatch, ComputePlatformReferencePriceCurveBatchEnvelope,
    ComputePlatformReferencePriceCurveEntryEnvelope, ComputePlatformReferencePriceCurveEntryIntent,
};

const MAX_PLATFORM_REFERENCE_PRICE_CURVE_JSON_BYTES: usize = 1024 * 1024;
const BATCH_ENVELOPE_DIGEST_DOMAIN: &[u8] = b"ELON-COMPUTE-PLATFORM-REFERENCE-PRICE-CURVE-BATCH-V1";
const BATCH_MATERIAL_DIGEST_DOMAIN: &[u8] =
    b"ELON-COMPUTE-PLATFORM-REFERENCE-PRICE-CURVE-MATERIAL-V1";
const ENTRY_SET_DIGEST_DOMAIN: &[u8] = b"ELON-COMPUTE-PLATFORM-REFERENCE-PRICE-CURVE-ENTRY-SET-V1";
const ENTRY_ENVELOPE_DIGEST_DOMAIN: &[u8] = b"ELON-COMPUTE-PLATFORM-REFERENCE-PRICE-CURVE-ENTRY-V1";

/// Returns full RFC 8785/JCS JSON and the domain-separated digest excluding `batch_digest`.
pub(crate) fn canonical_platform_reference_price_curve_batch_json_and_digest(
    envelope: &ComputePlatformReferencePriceCurveBatchEnvelope,
) -> Result<(String, String)> {
    #[derive(Serialize)]
    struct DigestProjection<'a> {
        schema: &'a str,
        batch_id: &'a str,
        batch_material_digest: &'a str,
        canonicalization: &'a str,
        digest_algorithm: &'a str,
        batch: &'a ComputePlatformReferencePriceCurveBatch,
    }

    let digest = domain_digest(
        BATCH_ENVELOPE_DIGEST_DOMAIN,
        &DigestProjection {
            schema: &envelope.schema,
            batch_id: &envelope.batch_id,
            batch_material_digest: &envelope.batch_material_digest,
            canonicalization: &envelope.canonicalization,
            digest_algorithm: &envelope.digest_algorithm,
            batch: &envelope.batch,
        },
    )?;
    let (json, _) = canonical_compute_plugin_ijson_and_sha256(
        envelope,
        MAX_PLATFORM_REFERENCE_PRICE_CURVE_JSON_BYTES,
    )?;
    Ok((json, digest))
}

/// Stable idempotency material digest. Server-assigned submission time remains outside it.
pub(crate) fn canonical_platform_reference_price_curve_batch_material_digest(
    batch: &ComputePlatformReferencePriceCurveBatch,
) -> Result<String> {
    #[derive(Serialize)]
    struct MaterialProjection<'a> {
        submitted_by_admin_user_id: &'a str,
        curve_id: &'a str,
        curve_version: i64,
        methodology_kind: &'a str,
        valid_from: &'a str,
        valid_until: &'a str,
        quote_ttl_seconds: i64,
        rounding_mode: &'a str,
        entries: &'a [ComputePlatformReferencePriceCurveEntryIntent],
        entry_set_digest: &'a str,
        idempotency_key: &'a str,
        confirmation: &'a str,
        submission_note: &'a str,
    }

    domain_digest(
        BATCH_MATERIAL_DIGEST_DOMAIN,
        &MaterialProjection {
            submitted_by_admin_user_id: &batch.submitted_by_admin_user_id,
            curve_id: &batch.curve_id,
            curve_version: batch.curve_version,
            methodology_kind: &batch.methodology_kind,
            valid_from: &batch.valid_from,
            valid_until: &batch.valid_until,
            quote_ttl_seconds: batch.quote_ttl_seconds,
            rounding_mode: &batch.rounding_mode,
            entries: &batch.entries,
            entry_set_digest: &batch.entry_set_digest,
            idempotency_key: &batch.idempotency_key,
            confirmation: &batch.confirmation,
            submission_note: &batch.submission_note,
        },
    )
}

/// Canonical binding of the strictly ordered entry intents; it carries no application authority.
pub(crate) fn canonical_platform_reference_price_curve_entry_set_digest(
    entries: &[ComputePlatformReferencePriceCurveEntryIntent],
) -> Result<String> {
    domain_digest(ENTRY_SET_DIGEST_DOMAIN, entries)
}

/// Returns full entry JSON and its digest excluding only `entry_digest`.
pub(crate) fn canonical_platform_reference_price_curve_entry_json_and_digest(
    envelope: &ComputePlatformReferencePriceCurveEntryEnvelope,
) -> Result<(String, String)> {
    #[derive(Serialize)]
    struct DigestProjection<'a> {
        schema: &'a str,
        batch_id: &'a str,
        batch_digest: &'a str,
        entry_id: &'a str,
        ordinal: i64,
        entry: &'a ComputePlatformReferencePriceCurveEntryIntent,
    }

    let digest = domain_digest(
        ENTRY_ENVELOPE_DIGEST_DOMAIN,
        &DigestProjection {
            schema: &envelope.schema,
            batch_id: &envelope.batch_id,
            batch_digest: &envelope.batch_digest,
            entry_id: &envelope.entry_id,
            ordinal: envelope.ordinal,
            entry: &envelope.entry,
        },
    )?;
    let (json, _) = canonical_compute_plugin_ijson_and_sha256(
        envelope,
        MAX_PLATFORM_REFERENCE_PRICE_CURVE_JSON_BYTES,
    )?;
    Ok((json, digest))
}

fn domain_digest<T: Serialize + ?Sized>(domain: &[u8], value: &T) -> Result<String> {
    let (json, _) = canonical_compute_plugin_ijson_and_sha256(
        value,
        MAX_PLATFORM_REFERENCE_PRICE_CURVE_JSON_BYTES,
    )?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(json.as_bytes());
    Ok(hex::encode(digest.finalize()))
}
