use anyhow::Result;
use rusqlite::{named_params, Transaction};

use crate::compute_federation::external_pool_provider_activation_candidate::{
    canonical_activation_candidate_json_and_digest,
    canonical_activation_delegation_json_and_digest,
    canonical_activation_delegation_revocation_json_and_digest,
    ExternalPoolProviderActivationCandidateReceipt,
    ExternalPoolProviderActivationDelegationReceipt,
    ExternalPoolProviderActivationDelegationRevocationReceipt,
};

pub(super) fn insert_delegation(
    tx: &Transaction<'_>,
    receipt: &ExternalPoolProviderActivationDelegationReceipt,
) -> Result<()> {
    let d = &receipt.delegation;
    tx.execute(
        "INSERT INTO compute_external_pool_provider_activation_delegations(
          delegation_id,delegation_schema,delegation_digest,delegation_material_digest,
          delegation_json,canonicalization,digest_algorithm,provider_binding_id,
          provider_binding_digest,registry_release_id,registry_release_digest,
          route_adapter_projection_id,provider_id,provider_owner_account_id,
          provider_policy_revision,provider_digest,provider_status,logical_adapter_id,
          release_version,adapter_config_revision,adapter_config_digest,service_actor_id,
          service_actor_kind,allowed_route_kinds_json,allowed_actor_phases_json,
          issued_by_owner_user_id,issued_at,recorded_at,sequence,predecessor_delegation_id,
          predecessor_delegation_digest,idempotency_scope,idempotency_key,confirmation,
          delegation_effect,provider_effect,credential_effect,route_effect,execution_effect,
          market_effect,settlement_effect
        ) VALUES (
          :id,:schema,:digest,:material,:json,:canonicalization,:algorithm,:binding_id,
          :binding_digest,:release_id,:release_digest,:projection_id,:provider_id,:owner_id,
          :provider_revision,:provider_digest,:provider_status,:adapter_id,:release_version,
          :config_revision,:config_digest,:actor_id,:actor_kind,:route_kinds,:actor_phases,
          :issued_by,:issued_at,:recorded_at,:sequence,:predecessor_id,:predecessor_digest,
          :scope,:key,:confirmation,:effect,:provider_effect,:credential_effect,:route_effect,
          :execution_effect,:market_effect,:settlement_effect)",
        named_params! {
            ":id": receipt.delegation_id, ":schema": receipt.schema,
            ":digest": receipt.delegation_digest, ":material": receipt.delegation_material_digest,
            ":json": canonical_activation_delegation_json_and_digest(receipt)?.0,
            ":canonicalization": receipt.canonicalization, ":algorithm": receipt.digest_algorithm,
            ":binding_id": d.provider_binding_id, ":binding_digest": d.provider_binding_digest,
            ":release_id": d.registry_release_id, ":release_digest": d.registry_release_digest,
            ":projection_id": d.route_adapter_projection_id, ":provider_id": d.provider_id,
            ":owner_id": d.provider_owner_account_id, ":provider_revision": d.provider_policy_revision,
            ":provider_digest": d.provider_digest, ":provider_status": d.provider_status,
            ":adapter_id": d.logical_adapter_id, ":release_version": d.release_version,
            ":config_revision": d.adapter_config_revision, ":config_digest": d.adapter_config_digest,
            ":actor_id": d.service_actor_id, ":actor_kind": d.service_actor_kind,
            ":route_kinds": canonical_json(&d.allowed_route_kinds)?,
            ":actor_phases": canonical_json(&d.allowed_actor_phases)?,
            ":issued_by": d.issued_by_owner_user_id, ":issued_at": d.issued_at,
            ":recorded_at": d.recorded_at, ":sequence": i64::try_from(d.sequence)?,
            ":predecessor_id": d.predecessor_delegation_id, ":predecessor_digest": d.predecessor_delegation_digest,
            ":scope": d.idempotency_scope, ":key": d.idempotency_key, ":confirmation": d.confirmation,
            ":effect": d.delegation_effect, ":provider_effect": d.provider_effect,
            ":credential_effect": d.credential_effect, ":route_effect": d.route_effect,
            ":execution_effect": d.execution_effect, ":market_effect": d.market_effect,
            ":settlement_effect": d.settlement_effect,
        },
    )?;
    Ok(())
}

pub(super) fn insert_candidate(
    tx: &Transaction<'_>,
    receipt: &ExternalPoolProviderActivationCandidateReceipt,
) -> Result<()> {
    let c = &receipt.candidate;
    tx.execute(
        "INSERT INTO compute_external_pool_provider_activation_candidates(
          candidate_id,candidate_schema,candidate_digest,candidate_material_digest,candidate_json,
          canonicalization,digest_algorithm,delegation_id,delegation_digest,provider_binding_id,
          provider_binding_digest,registry_release_id,registry_release_digest,installation_receipt_id,
          installation_receipt_digest,installation_content_digest,route_adapter_projection_id,
          provider_id,provider_owner_account_id,provider_policy_revision,provider_digest,
          provider_status,logical_adapter_id,release_version,adapter_config_revision,
          adapter_config_digest,implementation_digest,capability_set_digest,credential_verifier_digest,
          logical_adapter_binding_digest,logical_projection_compatibility_digest,service_actor_id,
          sequence,predecessor_candidate_id,predecessor_candidate_digest,checked_at,recorded_at,
          candidate_status,activation_closure_status,candidate_effect,provider_effect,
          credential_effect,route_effect,execution_effect,market_effect,settlement_effect
        ) VALUES (
          :id,:schema,:digest,:material,:json,:canonicalization,:algorithm,:delegation_id,
          :delegation_digest,:binding_id,:binding_digest,:release_id,:release_digest,
          :installation_id,:installation_digest,:content_digest,:projection_id,:provider_id,
          :owner_id,:provider_revision,:provider_digest,:provider_status,:adapter_id,
          :release_version,:config_revision,:config_digest,:implementation_digest,
          :capability_digest,:verifier_digest,:binding_shape_digest,:compatibility_digest,
          :actor_id,:sequence,:predecessor_id,:predecessor_digest,:checked_at,:recorded_at,
          :candidate_status,:closure_status,:effect,:provider_effect,:credential_effect,
          :route_effect,:execution_effect,:market_effect,:settlement_effect)",
        named_params! {
            ":id": receipt.candidate_id, ":schema": receipt.schema,
            ":digest": receipt.candidate_digest, ":material": receipt.candidate_material_digest,
            ":json": canonical_activation_candidate_json_and_digest(receipt)?.0,
            ":canonicalization": receipt.canonicalization, ":algorithm": receipt.digest_algorithm,
            ":delegation_id": c.delegation_id, ":delegation_digest": c.delegation_digest,
            ":binding_id": c.provider_binding_id, ":binding_digest": c.provider_binding_digest,
            ":release_id": c.registry_release_id, ":release_digest": c.registry_release_digest,
            ":installation_id": c.installation_receipt_id, ":installation_digest": c.installation_receipt_digest,
            ":content_digest": c.installation_content_digest, ":projection_id": c.route_adapter_projection_id,
            ":provider_id": c.provider_id, ":owner_id": c.provider_owner_account_id,
            ":provider_revision": c.provider_policy_revision, ":provider_digest": c.provider_digest,
            ":provider_status": c.provider_status, ":adapter_id": c.logical_adapter_id,
            ":release_version": c.release_version, ":config_revision": c.adapter_config_revision,
            ":config_digest": c.adapter_config_digest, ":implementation_digest": c.implementation_digest,
            ":capability_digest": c.capability_set_digest, ":verifier_digest": c.credential_verifier_digest,
            ":binding_shape_digest": c.logical_adapter_binding_digest,
            ":compatibility_digest": c.logical_projection_compatibility_digest,
            ":actor_id": c.service_actor_id, ":sequence": i64::try_from(c.sequence)?,
            ":predecessor_id": c.predecessor_candidate_id, ":predecessor_digest": c.predecessor_candidate_digest,
            ":checked_at": c.checked_at, ":recorded_at": c.recorded_at,
            ":candidate_status": c.candidate_status, ":closure_status": c.activation_closure_status,
            ":effect": c.candidate_effect, ":provider_effect": c.provider_effect,
            ":credential_effect": c.credential_effect, ":route_effect": c.route_effect,
            ":execution_effect": c.execution_effect, ":market_effect": c.market_effect,
            ":settlement_effect": c.settlement_effect,
        },
    )?;
    Ok(())
}

pub(super) fn insert_revocation(
    tx: &Transaction<'_>,
    receipt: &ExternalPoolProviderActivationDelegationRevocationReceipt,
) -> Result<()> {
    let r = &receipt.revocation;
    tx.execute(
        "INSERT INTO compute_external_pool_provider_activation_delegation_revocations(
          revocation_id,revocation_schema,revocation_digest,revocation_material_digest,
          revocation_json,canonicalization,digest_algorithm,delegation_id,delegation_digest,
          candidate_id,candidate_digest,provider_binding_id,provider_binding_digest,provider_id,
          revoked_by_owner_user_id,reason,revoked_at,recorded_at,idempotency_scope,idempotency_key,
          confirmation,revocation_effect,provider_effect,credential_effect,route_effect,
          execution_effect,market_effect,settlement_effect
        ) VALUES (:id,:schema,:digest,:material,:json,:canonicalization,:algorithm,:delegation_id,
          :delegation_digest,:candidate_id,:candidate_digest,:binding_id,:binding_digest,
          :provider_id,:revoked_by,:reason,:revoked_at,:recorded_at,:scope,:key,:confirmation,
          :effect,:provider_effect,:credential_effect,:route_effect,:execution_effect,
          :market_effect,:settlement_effect)",
        named_params! {
            ":id": receipt.revocation_id, ":schema": receipt.schema,
            ":digest": receipt.revocation_digest, ":material": receipt.revocation_material_digest,
            ":json": canonical_activation_delegation_revocation_json_and_digest(receipt)?.0,
            ":canonicalization": receipt.canonicalization, ":algorithm": receipt.digest_algorithm,
            ":delegation_id": r.delegation_id, ":delegation_digest": r.delegation_digest,
            ":candidate_id": r.candidate_id, ":candidate_digest": r.candidate_digest,
            ":binding_id": r.provider_binding_id, ":binding_digest": r.provider_binding_digest,
            ":provider_id": r.provider_id, ":revoked_by": r.revoked_by_owner_user_id,
            ":reason": r.reason, ":revoked_at": r.revoked_at, ":recorded_at": r.recorded_at,
            ":scope": r.idempotency_scope, ":key": r.idempotency_key,
            ":confirmation": r.confirmation, ":effect": r.revocation_effect,
            ":provider_effect": r.provider_effect, ":credential_effect": r.credential_effect,
            ":route_effect": r.route_effect, ":execution_effect": r.execution_effect,
            ":market_effect": r.market_effect, ":settlement_effect": r.settlement_effect,
        },
    )?;
    Ok(())
}

fn canonical_json<T: serde::Serialize>(value: &T) -> Result<String> {
    Ok(
        crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256(
            value,
            1024 * 1024,
        )?
        .0,
    )
}
