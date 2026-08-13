use anyhow::Result;
use rusqlite::{named_params, Transaction};

use crate::compute_federation::external_pool_adapter_upstream_transport_target::{
    canonical_upstream_transport_target_json_and_digest,
    canonical_upstream_transport_target_revocation_json_and_digest,
    ExternalPoolAdapterUpstreamTransportTargetReceipt,
    ExternalPoolAdapterUpstreamTransportTargetRevocationReceipt,
};

pub(super) fn insert_target(
    tx: &Transaction<'_>,
    receipt: &ExternalPoolAdapterUpstreamTransportTargetReceipt,
) -> Result<()> {
    let t = &receipt.target;
    let policy_json = canonical_json(&t.target_policy)?;
    tx.execute(
        "INSERT INTO compute_external_pool_adapter_upstream_transport_targets(
          target_id,target_schema,target_digest,target_material_digest,target_json,
          canonicalization,digest_algorithm,profile_id,profile_digest,candidate_id,
          candidate_digest,delegation_id,delegation_digest,provider_binding_id,
          provider_binding_digest,registry_release_id,registry_release_digest,
          installation_receipt_id,installation_receipt_digest,installation_content_digest,
          route_adapter_projection_id,provider_id,provider_owner_account_id,
          provider_policy_revision,provider_digest,provider_status,logical_adapter_id,
          release_version,adapter_config_revision,adapter_config_digest,implementation_digest,
          capability_set_digest,credential_verifier_digest,launch_policy_digest,
          network_egress_policy_id,network_egress_policy_revision,network_egress_policy_digest,
          service_actor_id,target_policy_digest,target_policy_json,dns_hostname,port,
          tls_server_name,expected_tls_leaf_spki_sha256,sequence,predecessor_target_id,
          predecessor_target_digest,recorded_by_actor_kind,recorded_by_actor_user_id,
          recorded_at,idempotency_scope,idempotency_key,confirmation,target_status,
          target_effect,adapter_effect,runtime_effect,provider_effect,credential_effect,
          route_effect,execution_effect,usage_effect,market_effect,settlement_effect,
          broker_connect_ready,upstream_probe_observed,runtime_launch_ready,activation_ready
        ) VALUES (
          :id,:schema,:digest,:material,:json,:canonicalization,:algorithm,:profile_id,
          :profile_digest,:candidate_id,:candidate_digest,:delegation_id,:delegation_digest,
          :binding_id,:binding_digest,:release_id,:release_digest,:installation_id,
          :installation_digest,:content_digest,:projection_id,:provider_id,:owner_id,
          :provider_revision,:provider_digest,:provider_status,:adapter_id,:release_version,
          :config_revision,:config_digest,:implementation,:capability,:verifier,:launch_policy,
          :network_policy_id,:network_policy_revision,:network_policy_digest,:service_actor,
          :target_policy_digest,:target_policy_json,:dns_hostname,:port,:tls_server_name,
          :expected_spki,:sequence,:predecessor_id,:predecessor_digest,:recorded_by_kind,
          :recorded_by_id,:recorded_at,:scope,:key,:confirmation,:target_status,:target_effect,
          :adapter_effect,:runtime_effect,:provider_effect,:credential_effect,:route_effect,
          :execution_effect,:usage_effect,:market_effect,:settlement_effect,
          :broker_connect_ready,:upstream_probe_observed,:runtime_launch_ready,:activation_ready)",
        named_params! {
            ":id": receipt.target_id, ":schema": receipt.schema,
            ":digest": receipt.target_digest, ":material": receipt.target_material_digest,
            ":json": canonical_upstream_transport_target_json_and_digest(receipt)?.0,
            ":canonicalization": receipt.canonicalization, ":algorithm": receipt.digest_algorithm,
            ":profile_id": t.profile_id, ":profile_digest": t.profile_digest,
            ":candidate_id": t.candidate_id, ":candidate_digest": t.candidate_digest,
            ":delegation_id": t.delegation_id, ":delegation_digest": t.delegation_digest,
            ":binding_id": t.provider_binding_id, ":binding_digest": t.provider_binding_digest,
            ":release_id": t.registry_release_id, ":release_digest": t.registry_release_digest,
            ":installation_id": t.installation_receipt_id,
            ":installation_digest": t.installation_receipt_digest,
            ":content_digest": t.installation_content_digest,
            ":projection_id": t.route_adapter_projection_id, ":provider_id": t.provider_id,
            ":owner_id": t.provider_owner_account_id,
            ":provider_revision": t.provider_policy_revision, ":provider_digest": t.provider_digest,
            ":provider_status": t.provider_status, ":adapter_id": t.logical_adapter_id,
            ":release_version": t.release_version, ":config_revision": t.adapter_config_revision,
            ":config_digest": t.adapter_config_digest, ":implementation": t.implementation_digest,
            ":capability": t.capability_set_digest, ":verifier": t.credential_verifier_digest,
            ":launch_policy": t.launch_policy_digest,
            ":network_policy_id": t.network_egress_policy_id,
            ":network_policy_revision": i64::try_from(t.network_egress_policy_revision)?,
            ":network_policy_digest": t.network_egress_policy_digest,
            ":service_actor": t.service_actor_id, ":target_policy_digest": t.target_policy_digest,
            ":target_policy_json": policy_json, ":dns_hostname": t.dns_hostname,
            ":port": i64::from(t.port), ":tls_server_name": t.tls_server_name,
            ":expected_spki": t.expected_tls_leaf_spki_sha256,
            ":sequence": i64::try_from(t.sequence)?,
            ":predecessor_id": t.predecessor_target_id,
            ":predecessor_digest": t.predecessor_target_digest,
            ":recorded_by_kind": t.recorded_by_actor_kind,
            ":recorded_by_id": t.recorded_by_actor_user_id, ":recorded_at": t.recorded_at,
            ":scope": t.idempotency_scope, ":key": t.idempotency_key,
            ":confirmation": t.confirmation, ":target_status": t.target_status,
            ":target_effect": t.target_effect, ":adapter_effect": t.adapter_effect,
            ":runtime_effect": t.runtime_effect, ":provider_effect": t.provider_effect,
            ":credential_effect": t.credential_effect, ":route_effect": t.route_effect,
            ":execution_effect": t.execution_effect, ":usage_effect": t.usage_effect,
            ":market_effect": t.market_effect, ":settlement_effect": t.settlement_effect,
            ":broker_connect_ready": t.broker_connect_ready,
            ":upstream_probe_observed": t.upstream_probe_observed,
            ":runtime_launch_ready": t.runtime_launch_ready,
            ":activation_ready": t.activation_ready,
        },
    )?;
    Ok(())
}

pub(super) fn insert_revocation(
    tx: &Transaction<'_>,
    receipt: &ExternalPoolAdapterUpstreamTransportTargetRevocationReceipt,
) -> Result<()> {
    let r = &receipt.revocation;
    tx.execute(
        "INSERT INTO compute_external_pool_adapter_upstream_transport_target_revocations(
          revocation_id,revocation_schema,revocation_digest,revocation_material_digest,
          revocation_json,canonicalization,digest_algorithm,target_id,target_digest,profile_id,
          profile_digest,provider_binding_id,provider_binding_digest,provider_id,
          revoked_by_actor_kind,revoked_by_actor_user_id,reason,revoked_at,recorded_at,
          idempotency_scope,idempotency_key,confirmation,revocation_effect,adapter_effect,
          runtime_effect,provider_effect,credential_effect,route_effect,execution_effect,
          usage_effect,market_effect,settlement_effect,broker_connect_ready,
          upstream_probe_observed,runtime_launch_ready,activation_ready
        ) VALUES (
          :id,:schema,:digest,:material,:json,:canonicalization,:algorithm,:target_id,
          :target_digest,:profile_id,:profile_digest,:binding_id,:binding_digest,:provider_id,
          :revoked_by_kind,:revoked_by_id,:reason,:revoked_at,:recorded_at,:scope,:key,
          :confirmation,:revocation_effect,:adapter_effect,:runtime_effect,:provider_effect,
          :credential_effect,:route_effect,:execution_effect,:usage_effect,:market_effect,
          :settlement_effect,:broker_connect_ready,:upstream_probe_observed,
          :runtime_launch_ready,:activation_ready)",
        named_params! {
            ":id": receipt.revocation_id, ":schema": receipt.schema,
            ":digest": receipt.revocation_digest,
            ":material": receipt.revocation_material_digest,
            ":json": canonical_upstream_transport_target_revocation_json_and_digest(receipt)?.0,
            ":canonicalization": receipt.canonicalization, ":algorithm": receipt.digest_algorithm,
            ":target_id": r.target_id, ":target_digest": r.target_digest,
            ":profile_id": r.profile_id, ":profile_digest": r.profile_digest,
            ":binding_id": r.provider_binding_id, ":binding_digest": r.provider_binding_digest,
            ":provider_id": r.provider_id, ":revoked_by_kind": r.revoked_by_actor_kind,
            ":revoked_by_id": r.revoked_by_actor_user_id, ":reason": r.reason,
            ":revoked_at": r.revoked_at, ":recorded_at": r.recorded_at,
            ":scope": r.idempotency_scope, ":key": r.idempotency_key,
            ":confirmation": r.confirmation, ":revocation_effect": r.revocation_effect,
            ":adapter_effect": r.adapter_effect, ":runtime_effect": r.runtime_effect,
            ":provider_effect": r.provider_effect, ":credential_effect": r.credential_effect,
            ":route_effect": r.route_effect, ":execution_effect": r.execution_effect,
            ":usage_effect": r.usage_effect, ":market_effect": r.market_effect,
            ":settlement_effect": r.settlement_effect,
            ":broker_connect_ready": r.broker_connect_ready,
            ":upstream_probe_observed": r.upstream_probe_observed,
            ":runtime_launch_ready": r.runtime_launch_ready,
            ":activation_ready": r.activation_ready,
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
