use anyhow::{bail, ensure, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::{
    types::{
        UntrustedComputeCapacityFutureSettlementLineageEnvelopeV1,
        COMPUTE_CAPACITY_FUTURE_SETTLEMENT_LINEAGE_DIGEST_DOMAIN,
        COMPUTE_CAPACITY_FUTURE_SETTLEMENT_LINEAGE_MAX_JSON_BYTES,
    },
    validation::validate_compute_capacity_future_settlement_lineage,
};

pub(crate) fn canonical_compute_capacity_future_settlement_lineage_json_and_digest(
    envelope: &UntrustedComputeCapacityFutureSettlementLineageEnvelopeV1,
) -> Result<(String, String)> {
    let value = serde_json::to_value(envelope)?;
    let mut projection = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("capacity-future settlement lineage must be an object"))?
        .clone();
    if projection
        .insert(
            "lineage_digest".to_string(),
            serde_json::Value::String(String::new()),
        )
        .is_none()
    {
        bail!("capacity-future settlement lineage lacks lineage_digest");
    }
    let digest = domain_digest(&projection)?;
    let json = canonical_json(envelope)?;
    Ok((json, digest))
}

pub(crate) fn compute_capacity_future_settlement_lineage_from_json(
    value: &str,
) -> Result<UntrustedComputeCapacityFutureSettlementLineageEnvelopeV1> {
    ensure!(
        value.len() <= COMPUTE_CAPACITY_FUTURE_SETTLEMENT_LINEAGE_MAX_JSON_BYTES,
        "capacity-future settlement lineage exceeds its byte limit"
    );
    let envelope =
        serde_json::from_str::<UntrustedComputeCapacityFutureSettlementLineageEnvelopeV1>(value)?;
    validate_compute_capacity_future_settlement_lineage(&envelope)?;
    ensure!(
        canonical_json(&envelope)? == value,
        "capacity-future settlement lineage JSON is not canonical"
    );
    Ok(envelope)
}

fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    canonical_compute_plugin_ijson_and_sha256(
        value,
        COMPUTE_CAPACITY_FUTURE_SETTLEMENT_LINEAGE_MAX_JSON_BYTES,
    )
    .map(|(json, _)| json)
}

fn domain_digest<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    let json = canonical_json(value)?;
    let mut digest = Sha256::new();
    digest.update(COMPUTE_CAPACITY_FUTURE_SETTLEMENT_LINEAGE_DIGEST_DOMAIN.as_bytes());
    digest.update([0]);
    digest.update(json.as_bytes());
    Ok(hex::encode(digest.finalize()))
}
