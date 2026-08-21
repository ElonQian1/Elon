use anyhow::{bail, Result};
use rusqlite::TransactionBehavior;

use crate::compute_federation::federation_historical_causal_reference::{
    canonical_federation_historical_causal_reference_json_and_digest,
    validate_federation_historical_causal_reference, FederationHistoricalLineageKindV1,
    UntrustedFederationHistoricalCausalReferenceEnvelopeV1,
};

use super::{
    compute_attempt_execution_receipts::compute_attempt_historical_execution_receipt_by_lease_on,
    compute_attempt_settlement_releases::compute_attempt_historical_settlement_release_by_lease_on,
    compute_attempt_settlements::compute_attempt_historical_settlement_by_lease_on, Store,
};

mod execution;
mod release;
mod release_refs;
mod settlement;
mod source_refs;

#[cfg(test)]
mod source_contract_tests;

pub(crate) struct ValidatedFederationHistoricalLineage {
    canonical_json: String,
    lineage_digest: String,
    kind: FederationHistoricalLineageKindV1,
    access_scope: FederationHistoricalLineageAccessScope,
}

struct FederationHistoricalLineageAccessScope {
    consumer_account_id: String,
    project_id: Option<String>,
    provider_owner_account_id: String,
}

impl FederationHistoricalLineageAccessScope {
    fn from_historical_job_and_provider(
        consumer_account_id: &str,
        project_id: Option<&str>,
        provider_owner_account_id: &str,
    ) -> Result<Self> {
        validate_scope_id("historical consumer account ID", consumer_account_id)?;
        if let Some(project_id) = project_id {
            validate_scope_id("historical project ID", project_id)?;
        }
        validate_scope_id(
            "historical Provider owner account ID",
            provider_owner_account_id,
        )?;
        Ok(Self {
            consumer_account_id: consumer_account_id.to_string(),
            project_id: project_id.map(str::to_string),
            provider_owner_account_id: provider_owner_account_id.to_string(),
        })
    }

    fn ensure_same_as(&self, other: &Self) -> Result<()> {
        if self.consumer_account_id != other.consumer_account_id
            || self.project_id != other.project_id
            || self.provider_owner_account_id != other.provider_owner_account_id
        {
            bail!("historical federation lineage access scope drifted across retained owners");
        }
        Ok(())
    }

    fn ensure_job_matches(
        &self,
        consumer_account_id: &str,
        project_id: Option<&str>,
    ) -> Result<()> {
        if self.consumer_account_id != consumer_account_id
            || self.project_id.as_deref() != project_id
        {
            bail!("historical federation lineage source and terminal Job scopes differ");
        }
        Ok(())
    }

    fn permits_user(&self, user_id: &str) -> bool {
        user_id == self.consumer_account_id || user_id == self.provider_owner_account_id
    }

    fn belongs_to_project(&self, project_id: &str) -> bool {
        self.project_id.as_deref() == Some(project_id)
    }
}

impl ValidatedFederationHistoricalLineage {
    fn from_carrier(
        carrier: UntrustedFederationHistoricalCausalReferenceEnvelopeV1,
        access_scope: FederationHistoricalLineageAccessScope,
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
            access_scope,
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

    pub(crate) fn permits_user(&self, user_id: &str) -> bool {
        self.access_scope.permits_user(user_id)
    }

    pub(crate) fn belongs_to_project(&self, project_id: &str) -> bool {
        self.access_scope.belongs_to_project(project_id)
    }

    fn access_scope(&self) -> &FederationHistoricalLineageAccessScope {
        &self.access_scope
    }

    fn into_lineage_digest_and_access_scope(
        self,
    ) -> (String, FederationHistoricalLineageAccessScope) {
        (self.lineage_digest, self.access_scope)
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

    pub(crate) fn resolve_compute_execution_source_lineage_for_lease(
        &self,
        lease_id: &str,
    ) -> Result<Option<ValidatedFederationHistoricalLineage>> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Deferred)?;
        let Some(receipt) =
            compute_attempt_historical_execution_receipt_by_lease_on(&tx, lease_id)?
        else {
            tx.commit()?;
            return Ok(None);
        };
        let resolved = execution::resolve_execution_source_lineage_on(
            &tx,
            &receipt.receipt.receipt_id,
            &receipt.receipt.receipt_digest,
        )?;
        tx.commit()?;
        Ok(Some(resolved))
    }

    pub(crate) fn resolve_compute_settlement_source_lineage_for_lease(
        &self,
        lease_id: &str,
    ) -> Result<Option<ValidatedFederationHistoricalLineage>> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Deferred)?;
        let Some(receipt) = compute_attempt_historical_settlement_by_lease_on(&tx, lease_id)?
        else {
            tx.commit()?;
            return Ok(None);
        };
        let resolved = settlement::resolve_settlement_source_lineage_on(
            &tx,
            &receipt.settlement.settlement_receipt_id,
            &receipt.settlement.settlement_receipt_digest,
            &receipt.event_digest,
        )?;
        tx.commit()?;
        Ok(Some(resolved))
    }

    pub(crate) fn resolve_compute_settlement_release_source_lineage_for_lease(
        &self,
        lease_id: &str,
    ) -> Result<Option<ValidatedFederationHistoricalLineage>> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Deferred)?;
        let Some(release) =
            compute_attempt_historical_settlement_release_by_lease_on(&tx, lease_id)?
        else {
            tx.commit()?;
            return Ok(None);
        };
        let resolved = release::resolve_settlement_release_source_lineage_on(&tx, &release)?;
        tx.commit()?;
        Ok(Some(resolved))
    }
}

fn validate_scope_id(label: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || value != value.trim()
        || value.len() > 240
        || value.chars().any(char::is_control)
    {
        bail!("{label} is invalid");
    }
    Ok(())
}
