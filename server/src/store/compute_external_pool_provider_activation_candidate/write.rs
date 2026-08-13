use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{Transaction, TransactionBehavior};

use crate::store::{
    compute_external_pool_adapter_registry::current_external_pool_adapter_registry_provider_binding_authority_on,
    Store,
};

use super::{build::*, persistence::*, read::*, replay::*, roots::audit_static_roots, types::*};

impl Store {
    pub(crate) fn create_external_pool_provider_activation_candidate(
        &self,
        input: CreateExternalPoolProviderActivationCandidate,
    ) -> Result<ExternalPoolProviderActivationCandidateWriteReceipt> {
        let CreateExternalPoolProviderActivationCandidate {
            prepared,
            provider_binding_id,
            expected_provider_binding_digest,
            expected_registry_release_digest,
            issued_by_owner_user_id,
            idempotency_scope,
            idempotency_key,
            confirmation,
        } = input;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(delegation) =
            delegation_by_idempotency_on(&tx, &idempotency_scope, &idempotency_key)?
        {
            let candidate = candidate_by_delegation_on(&tx, &delegation.receipt.delegation_id)?
                .ok_or_else(|| anyhow::anyhow!("activation candidate replay lost its candidate"))?;
            ensure_create_replay(
                &tx,
                &prepared,
                &delegation,
                &candidate,
                &provider_binding_id,
                &expected_provider_binding_digest,
                &expected_registry_release_digest,
                &issued_by_owner_user_id,
                &idempotency_scope,
                &idempotency_key,
                &confirmation,
            )?;
            let output = write_output(&delegation, &candidate, true);
            tx.commit()?;
            return Ok(output);
        }
        let (previous_delegation, previous_candidate, sequence) =
            lineage(&tx, &provider_binding_id)?;
        let checked_at =
            lineage_checked_at(previous_delegation.as_ref(), previous_candidate.as_ref());
        let authority = current_external_pool_adapter_registry_provider_binding_authority_on(
            &tx,
            &provider_binding_id,
            prepared,
            &checked_at,
        )?
        .ok_or_else(|| anyhow::anyhow!("current exact V249 Provider binding was not found"))?;
        let delegation = build_delegation(
            &authority,
            previous_delegation.as_ref(),
            sequence,
            &issued_by_owner_user_id,
            &idempotency_scope,
            &idempotency_key,
            &confirmation,
            &checked_at,
        )?;
        let candidate = build_candidate(
            &authority,
            &delegation,
            previous_candidate.as_ref(),
            sequence,
            &checked_at,
        )?;
        if authority.binding().provider_binding_digest != expected_provider_binding_digest
            || authority.release().registry_release_digest != expected_registry_release_digest
        {
            bail!("activation candidate expected V249 roots are not exact");
        }
        audit_static_roots(&authority, &delegation, &candidate)?;
        insert_delegation(&tx, &delegation)?;
        insert_candidate(&tx, &candidate)?;
        let stored_delegation = delegation_by_id_on(&tx, &delegation.delegation_id)?
            .ok_or_else(|| anyhow::anyhow!("activation delegation disappeared after insert"))?;
        let stored_candidate = candidate_by_id_on(&tx, &candidate.candidate_id)?
            .ok_or_else(|| anyhow::anyhow!("activation candidate disappeared after insert"))?;
        let output = write_output(&stored_delegation, &stored_candidate, false);
        tx.commit()?;
        Ok(output)
    }

    pub(crate) fn revoke_external_pool_provider_activation_delegation(
        &self,
        input: RevokeExternalPoolProviderActivationDelegation,
    ) -> Result<ExternalPoolProviderActivationDelegationRevocationWriteReceipt> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(revocation) =
            revocation_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)?
        {
            let delegation =
                delegation_by_id_on(&tx, &revocation.receipt.revocation.delegation_id)?
                    .ok_or_else(|| anyhow::anyhow!("revocation replay lost delegation"))?;
            let candidate = candidate_by_delegation_on(&tx, &delegation.receipt.delegation_id)?
                .ok_or_else(|| anyhow::anyhow!("revocation replay lost candidate"))?;
            ensure_revocation_replay(&input, &delegation, &candidate, &revocation)?;
            let output = revocation_output(&delegation, &candidate, &revocation, true);
            tx.commit()?;
            return Ok(output);
        }
        let delegation = delegation_by_id_on(&tx, &input.delegation_id)?
            .ok_or_else(|| anyhow::anyhow!("activation delegation was not found"))?;
        let candidate = candidate_by_delegation_on(&tx, &input.delegation_id)?
            .ok_or_else(|| anyhow::anyhow!("activation delegation lost its candidate"))?;
        ensure_revocation_target(&input, &delegation, &candidate)?;
        let head =
            delegation_head_by_binding_on(&tx, &delegation.receipt.delegation.provider_binding_id)?
                .ok_or_else(|| anyhow::anyhow!("activation delegation lineage head disappeared"))?;
        if head.receipt.delegation_id != delegation.receipt.delegation_id {
            bail!("only the current activation delegation may be revoked");
        }
        if revocation_by_delegation_on(&tx, &input.delegation_id)?.is_some() {
            bail!("activation delegation is already revoked under another idempotency key");
        }
        let revoked_at = std::cmp::max(now(), candidate.receipt.candidate.checked_at.clone());
        let receipt = build_revocation(&input, &delegation, &candidate, &revoked_at)?;
        insert_revocation(&tx, &receipt)?;
        let stored = revocation_by_delegation_on(&tx, &input.delegation_id)?
            .ok_or_else(|| anyhow::anyhow!("activation revocation disappeared after insert"))?;
        let output = revocation_output(&delegation, &candidate, &stored, false);
        tx.commit()?;
        Ok(output)
    }
}

fn lineage(
    tx: &Transaction<'_>,
    binding_id: &str,
) -> Result<(Option<StoredDelegation>, Option<StoredCandidate>, u64)> {
    let delegation = delegation_head_by_binding_on(tx, binding_id)?;
    let candidate = candidate_head_by_binding_on(tx, binding_id)?;
    match (&delegation, &candidate) {
        (None, None) => Ok((None, None, 1)),
        (Some(d), Some(c)) => {
            if c.receipt.candidate.delegation_id != d.receipt.delegation_id
                || c.receipt.candidate.sequence != d.receipt.delegation.sequence
                || revocation_by_delegation_on(tx, &d.receipt.delegation_id)?.is_some()
            {
                bail!("activation lineage head is inconsistent or revoked");
            }
            let sequence = d
                .receipt
                .delegation
                .sequence
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("activation sequence overflow"))?;
            Ok((delegation, candidate, sequence))
        }
        _ => bail!("activation delegation/candidate lineage is incomplete"),
    }
}

fn lineage_checked_at(
    delegation: Option<&StoredDelegation>,
    candidate: Option<&StoredCandidate>,
) -> String {
    let mut checked_at = now();
    if let Some(delegation) = delegation {
        checked_at = std::cmp::max(checked_at, delegation.receipt.delegation.issued_at.clone());
    }
    if let Some(candidate) = candidate {
        checked_at = std::cmp::max(checked_at, candidate.receipt.candidate.checked_at.clone());
    }
    checked_at
}

fn write_output(
    d: &StoredDelegation,
    c: &StoredCandidate,
    replayed: bool,
) -> ExternalPoolProviderActivationCandidateWriteReceipt {
    ExternalPoolProviderActivationCandidateWriteReceipt {
        delegation: d.summary(),
        candidate: c.summary(),
        replayed,
    }
}
fn revocation_output(
    d: &StoredDelegation,
    c: &StoredCandidate,
    r: &StoredRevocation,
    replayed: bool,
) -> ExternalPoolProviderActivationDelegationRevocationWriteReceipt {
    ExternalPoolProviderActivationDelegationRevocationWriteReceipt {
        delegation: d.summary(),
        candidate: c.summary(),
        revocation: r.summary(),
        replayed,
    }
}
fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
