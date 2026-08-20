//! Durable identity-only registering selector; workers never provide receipt IDs.

use anyhow::{ensure, Result};
use rusqlite::{params, OptionalExtension, Transaction};

use crate::store::compute_external_pool_adapter_installation::external_pool_adapter_installation_receipt_authority_on;

use super::super::types::ExternalPoolAdapterRegisteringActivationCandidate;

pub(super) fn select_external_pool_adapter_registering_activation_candidate_on(
    transaction: &Transaction<'_>,
    selection_slot: u64,
) -> Result<Option<ExternalPoolAdapterRegisteringActivationCandidate>> {
    let candidate_count: i64 = transaction.query_row(COUNT_CANDIDATES, [], |row| row.get(0))?;
    if candidate_count == 0 {
        return Ok(None);
    }
    ensure!(
        candidate_count > 0,
        "registering activation candidate count is invalid"
    );
    let offset = i64::try_from(selection_slot % u64::try_from(candidate_count)?)?;
    let selected = transaction
        .query_row(SELECT_CANDIDATE, params![offset], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
            ))
        })
        .optional()?;
    let Some((
        provider_id,
        provider_binding_id,
        provider_binding_digest,
        installation_receipt_id,
        installation_receipt_digest,
        companion_id,
        companion_digest,
        runtime_receipt_id,
        runtime_receipt_digest,
    )) = selected
    else {
        anyhow::bail!("registering activation candidate rotation lost its selected row");
    };
    let installation = external_pool_adapter_installation_receipt_authority_on(
        transaction,
        &installation_receipt_id,
        &installation_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("registering activation lost installation history"))?;
    let receipt = installation.receipt();
    let binding = &receipt.installation.binding;
    ensure!(
        binding.provider_id == provider_id
            && receipt.installation_receipt_id == installation_receipt_id
            && receipt.installation_receipt_digest == installation_receipt_digest,
        "registering activation installation identity changed after selection"
    );
    Ok(Some(ExternalPoolAdapterRegisteringActivationCandidate {
        provider_id,
        provider_binding_id,
        provider_binding_digest,
        companion_id,
        companion_digest,
        runtime_compatibility_verification_receipt_id: runtime_receipt_id,
        runtime_compatibility_verification_receipt_digest: runtime_receipt_digest,
        installation_binding: binding.clone(),
    }))
}

const SELECT_CANDIDATE: &str = "
SELECT binding.provider_id,binding.provider_binding_id,binding.provider_binding_digest,
       binding.installation_receipt_id,binding.installation_receipt_digest,
       companion.companion_id,companion.companion_digest,
       compatibility.verification_receipt_id,compatibility.verification_receipt_digest
  FROM compute_external_pool_adapter_registry_provider_binding_current binding
  JOIN compute_external_pool_adapter_supervisor_session_policy_companion_current companion
    ON companion.provider_binding_id=binding.provider_binding_id
   AND companion.provider_binding_digest=binding.provider_binding_digest
   AND companion.registry_release_id=binding.registry_release_id
   AND companion.registry_release_digest=binding.registry_release_digest
  JOIN compute_external_pool_adapter_runtime_compatibility_verification_current compatibility
    ON compatibility.registry_release_id=binding.registry_release_id
   AND compatibility.registry_release_digest=binding.registry_release_digest
 WHERE binding.current_status='binding_current'
   AND companion.current_status='supervisor_session_policy_companion_current_inert'
   AND compatibility.currentness_status='current_signed_verifier_assertion'
   AND NOT EXISTS(
       SELECT 1 FROM compute_external_pool_adapter_atomic_activation_receipts activation
        WHERE activation.provider_binding_id=binding.provider_binding_id)
 ORDER BY binding.bound_at,binding.provider_binding_id,companion.sequence,companion.companion_id
 LIMIT 1 OFFSET ?1";

const COUNT_CANDIDATES: &str = "
SELECT count(*)
  FROM compute_external_pool_adapter_registry_provider_binding_current binding
  JOIN compute_external_pool_adapter_supervisor_session_policy_companion_current companion
    ON companion.provider_binding_id=binding.provider_binding_id
   AND companion.provider_binding_digest=binding.provider_binding_digest
   AND companion.registry_release_id=binding.registry_release_id
   AND companion.registry_release_digest=binding.registry_release_digest
  JOIN compute_external_pool_adapter_runtime_compatibility_verification_current compatibility
    ON compatibility.registry_release_id=binding.registry_release_id
   AND compatibility.registry_release_digest=binding.registry_release_digest
 WHERE binding.current_status='binding_current'
   AND companion.current_status='supervisor_session_policy_companion_current_inert'
   AND compatibility.currentness_status='current_signed_verifier_assertion'
   AND NOT EXISTS(
       SELECT 1 FROM compute_external_pool_adapter_atomic_activation_receipts activation
        WHERE activation.provider_binding_id=binding.provider_binding_id)";
