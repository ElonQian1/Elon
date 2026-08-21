use anyhow::{anyhow, bail, Context, Result};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::types::{
    ExecutionSourceLineageV1, ExecutionVerificationSourceLineageV1,
    FederationHistoricalLineageKindV1, FederationHistoricalLineageV1,
    SettlementReleaseSourceLineageV1, SettlementSourceLineageV1,
    UntrustedFederationHistoricalCausalReferenceEnvelopeV1,
    FEDERATION_HISTORICAL_CAUSAL_REFERENCE_CANONICALIZATION,
    FEDERATION_HISTORICAL_CAUSAL_REFERENCE_DIGEST_ALGORITHM,
    FEDERATION_HISTORICAL_CAUSAL_REFERENCE_DIGEST_DOMAIN,
    FEDERATION_HISTORICAL_CAUSAL_REFERENCE_MAX_JSON_BYTES,
    FEDERATION_HISTORICAL_CAUSAL_REFERENCE_SCHEMA,
};
use super::validation::validate_federation_historical_causal_reference;

pub(crate) fn build_execution_source_carrier(
    lineage: ExecutionSourceLineageV1,
) -> Result<UntrustedFederationHistoricalCausalReferenceEnvelopeV1> {
    build_carrier(
        FederationHistoricalLineageKindV1::ExecutionSourceV1,
        FederationHistoricalLineageV1::ExecutionSource(lineage),
    )
}

pub(crate) fn build_execution_verification_source_carrier(
    lineage: ExecutionVerificationSourceLineageV1,
) -> Result<UntrustedFederationHistoricalCausalReferenceEnvelopeV1> {
    build_carrier(
        FederationHistoricalLineageKindV1::ExecutionVerificationSourceV1,
        FederationHistoricalLineageV1::ExecutionVerificationSource(lineage),
    )
}

pub(crate) fn build_settlement_source_carrier(
    lineage: SettlementSourceLineageV1,
) -> Result<UntrustedFederationHistoricalCausalReferenceEnvelopeV1> {
    build_carrier(
        FederationHistoricalLineageKindV1::SettlementSourceV1,
        FederationHistoricalLineageV1::SettlementSource(lineage),
    )
}

pub(crate) fn build_settlement_release_source_carrier(
    lineage: SettlementReleaseSourceLineageV1,
) -> Result<UntrustedFederationHistoricalCausalReferenceEnvelopeV1> {
    build_carrier(
        FederationHistoricalLineageKindV1::SettlementReleaseSourceV1,
        FederationHistoricalLineageV1::SettlementReleaseSource(lineage),
    )
}

pub(crate) fn canonical_federation_historical_causal_reference_json_and_digest(
    envelope: &UntrustedFederationHistoricalCausalReferenceEnvelopeV1,
) -> Result<(String, String)> {
    let mut digest_projection = serde_json::to_value(envelope)
        .context("serialize federation historical causal reference digest projection")?;
    let lineage_digest = digest_projection
        .as_object_mut()
        .and_then(|object| object.get_mut("lineage_digest"))
        .ok_or_else(|| anyhow!("federation historical causal reference digest key is missing"))?;
    *lineage_digest = Value::String(String::new());

    let (projection_json, _) = canonical_compute_plugin_ijson_and_sha256(
        &digest_projection,
        FEDERATION_HISTORICAL_CAUSAL_REFERENCE_MAX_JSON_BYTES,
    )?;
    let mut digest = Sha256::new();
    digest.update(FEDERATION_HISTORICAL_CAUSAL_REFERENCE_DIGEST_DOMAIN.as_bytes());
    digest.update([0]);
    digest.update(projection_json.as_bytes());
    let digest = hex::encode(digest.finalize());

    let (canonical_json, _) = canonical_compute_plugin_ijson_and_sha256(
        envelope,
        FEDERATION_HISTORICAL_CAUSAL_REFERENCE_MAX_JSON_BYTES,
    )?;
    Ok((canonical_json, digest))
}

pub(crate) fn federation_historical_causal_reference_from_json(
    json: &str,
) -> Result<UntrustedFederationHistoricalCausalReferenceEnvelopeV1> {
    federation_historical_causal_reference_from_json_bytes(json.as_bytes())
}

pub(crate) fn federation_historical_causal_reference_from_json_bytes(
    bytes: &[u8],
) -> Result<UntrustedFederationHistoricalCausalReferenceEnvelopeV1> {
    if bytes.len() > FEDERATION_HISTORICAL_CAUSAL_REFERENCE_MAX_JSON_BYTES {
        bail!("federation historical causal reference exceeds its JSON byte limit");
    }
    let json = std::str::from_utf8(bytes)
        .context("federation historical causal reference is not valid UTF-8")?;
    let envelope: UntrustedFederationHistoricalCausalReferenceEnvelopeV1 =
        serde_json::from_str(json)
            .context("parse federation historical causal reference exact envelope")?;
    validate_federation_historical_causal_reference(&envelope)?;
    let (canonical_json, _) =
        canonical_federation_historical_causal_reference_json_and_digest(&envelope)?;
    if canonical_json.as_bytes() != bytes {
        bail!("federation historical causal reference bytes are not canonical JCS");
    }
    Ok(envelope)
}

fn build_carrier(
    lineage_kind: FederationHistoricalLineageKindV1,
    lineage: FederationHistoricalLineageV1,
) -> Result<UntrustedFederationHistoricalCausalReferenceEnvelopeV1> {
    let mut envelope = UntrustedFederationHistoricalCausalReferenceEnvelopeV1 {
        schema: FEDERATION_HISTORICAL_CAUSAL_REFERENCE_SCHEMA.to_string(),
        lineage_kind,
        lineage_digest: String::new(),
        canonicalization: FEDERATION_HISTORICAL_CAUSAL_REFERENCE_CANONICALIZATION.to_string(),
        digest_algorithm: FEDERATION_HISTORICAL_CAUSAL_REFERENCE_DIGEST_ALGORITHM.to_string(),
        lineage,
    };
    let (_, digest) = canonical_federation_historical_causal_reference_json_and_digest(&envelope)?;
    envelope.lineage_digest = digest;
    validate_federation_historical_causal_reference(&envelope)?;
    Ok(envelope)
}
