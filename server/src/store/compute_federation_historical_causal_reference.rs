use anyhow::{bail, Result};
use rusqlite::TransactionBehavior;

use crate::compute_federation::federation_historical_causal_reference::{
    canonical_federation_historical_causal_reference_json_and_digest,
    validate_federation_historical_causal_reference, FederationHistoricalLineageKindV1,
    UntrustedFederationHistoricalCausalReferenceEnvelopeV1,
};

use super::Store;

mod execution;
mod settlement;
mod source_refs;

#[cfg(test)]
mod source_contract_tests;

pub(crate) struct ValidatedFederationHistoricalLineage {
    canonical_json: String,
    lineage_digest: String,
    kind: FederationHistoricalLineageKindV1,
}

impl ValidatedFederationHistoricalLineage {
    fn from_carrier(
        carrier: UntrustedFederationHistoricalCausalReferenceEnvelopeV1,
    ) -> Result<Self> {
        validate_federation_historical_causal_reference(&carrier)?;
        let (canonical_json, recomputed_digest) =
            canonical_federation_historical_causal_reference_json_and_digest(&carrier)?;
        if recomputed_digest != carrier.lineage_digest() {
            bail!("validated federation historical lineage digest drifted after owner resolution");
        }
        Ok(Self {
            canonical_json,
            lineage_digest: recomputed_digest,
            kind: carrier.lineage_kind(),
        })
    }

    pub(crate) fn canonical_json(&self) -> &str {
        &self.canonical_json
    }

    pub(crate) fn lineage_digest(&self) -> &str {
        &self.lineage_digest
    }

    pub(crate) fn kind(&self) -> FederationHistoricalLineageKindV1 {
        self.kind
    }
}

impl Store {
    pub(crate) fn resolve_compute_execution_source_lineage(
        &self,
        execution_receipt_id: &str,
        execution_receipt_digest: &str,
    ) -> Result<ValidatedFederationHistoricalLineage> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Deferred)?;
        let resolved = execution::resolve_execution_source_lineage_on(
            &tx,
            execution_receipt_id,
            execution_receipt_digest,
        )?;
        tx.commit()?;
        Ok(resolved)
    }

    pub(crate) fn resolve_compute_settlement_source_lineage(
        &self,
        settlement_receipt_id: &str,
        settlement_receipt_digest: &str,
        settlement_event_digest: &str,
    ) -> Result<ValidatedFederationHistoricalLineage> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Deferred)?;
        let resolved = settlement::resolve_settlement_source_lineage_on(
            &tx,
            settlement_receipt_id,
            settlement_receipt_digest,
            settlement_event_digest,
        )?;
        tx.commit()?;
        Ok(resolved)
    }
}
