use serde::Serialize;

use crate::{
    compute_federation::federation_historical_causal_reference::FederationHistoricalLineageKindV1,
    store::compute_federation_historical_causal_reference::ValidatedFederationHistoricalLineage,
};

pub(super) const FEDERATION_HISTORICAL_LINEAGE_READ_SCHEMA: &str =
    "compute_federation.core_historical_causal_reference.read.v1";

#[derive(Serialize)]
pub(super) struct FederationHistoricalLineageReadDocument {
    schema: &'static str,
    lineage_kind: FederationHistoricalLineageKindV1,
    lineage_digest: String,
    canonical_carrier_json: String,
    read_effect: &'static str,
}

impl FederationHistoricalLineageReadDocument {
    pub(super) fn from_validated(lineage: ValidatedFederationHistoricalLineage) -> Self {
        Self {
            schema: FEDERATION_HISTORICAL_LINEAGE_READ_SCHEMA,
            lineage_kind: lineage.kind(),
            lineage_digest: lineage.lineage_digest().to_string(),
            canonical_carrier_json: lineage.canonical_json().to_string(),
            read_effect: "none",
        }
    }
}
