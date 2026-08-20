use anyhow::{bail, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::*;

const RECEIPT_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-ATOMIC-ACTIVATION-RECEIPT-V1";
const EXECUTOR_ID_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-STABLE-EXECUTOR-ID-V1";
const EXECUTOR_BINDING_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-STABLE-EXECUTOR-BINDING-V1";
const TRANSITION_PROOF_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-CREDENTIAL-PROJECTED-ACTIVE-TRANSITION-PROOF-V1";
const ACTIVE_CARRIER_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-TASK-PROTOCOL-ACTIVE-CARRIER-V1";
const IDEMPOTENCY_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-ATOMIC-ACTIVATION-IDEMPOTENCY-V1";
const CONFIRMATION_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-ATOMIC-ACTIVATION-CONFIRMATION-V1";

pub(crate) fn derive_external_pool_stable_executor(
    id_material: ExternalPoolStableExecutorIdMaterial,
    logical_projection_compatibility_digest: String,
    projected_v211_adapter_binding_digest: String,
    lane_subject_digest: String,
) -> Result<ExternalPoolStableExecutorBinding> {
    let executor_id_material_json = canonical_json(&id_material)?;
    let executor_id_hash = domain_digest_from_json(EXECUTOR_ID_DOMAIN, &executor_id_material_json);
    let executor_id = format!("external_pool_executor_{executor_id_hash}");
    let binding_material = ExternalPoolStableExecutorBindingMaterial {
        provider_binding_id: id_material.provider_binding_id,
        provider_binding_digest: id_material.provider_binding_digest,
        activation_root_digest: id_material.activation_root_digest,
        route_adapter_projection_id: id_material.route_adapter_projection_id,
        service_actor_id: id_material.service_actor_id,
        task_production_carrier_policy_digest: id_material.task_production_carrier_policy_digest,
        executor_id: executor_id.clone(),
        logical_projection_compatibility_digest,
        projected_v211_adapter_binding_digest,
        lane_subject_digest,
    };
    let executor_binding_material_json = canonical_json(&binding_material)?;
    let stable_executor_binding_digest =
        domain_digest_from_json(EXECUTOR_BINDING_DOMAIN, &executor_binding_material_json);
    Ok(ExternalPoolStableExecutorBinding {
        executor_id,
        executor_id_hash,
        executor_id_material_json,
        executor_binding_material_json,
        stable_executor_binding_digest,
    })
}

pub(crate) fn canonical_projected_active_transition_proof_json_and_digest(
    material: &ExternalPoolAdapterCredentialProjectedActiveTransitionProofMaterial,
) -> Result<(String, String)> {
    if material.schema != PROJECTED_ACTIVE_TRANSITION_PROOF_SCHEMA {
        bail!("V277 projected transition proof schema is not exact")
    }
    canonical_material_json_and_digest(TRANSITION_PROOF_DOMAIN, material)
}

pub(crate) fn canonical_task_protocol_active_carrier_json_and_digest(
    material: &ExternalPoolAdapterTaskProtocolActiveCarrierMaterial,
) -> Result<(String, String)> {
    if material.schema != TASK_PROTOCOL_ACTIVE_CARRIER_SCHEMA {
        bail!("V277 active carrier schema is not exact")
    }
    canonical_material_json_and_digest(ACTIVE_CARRIER_DOMAIN, material)
}

pub(crate) fn canonical_external_pool_adapter_atomic_activation_route_capabilities_json(
    capabilities: &[crate::compute_federation::route_authority::ComputeRouteCapabilityBinding],
) -> Result<String> {
    canonical_json(capabilities)
}

pub(crate) fn canonical_atomic_activation_idempotency_json_and_digest(
    material: &ExternalPoolAdapterAtomicActivationIdempotencyMaterial,
) -> Result<(String, String)> {
    if material.actor_kind != ATOMIC_ACTIVATION_ACTOR_KIND
        || material.scope != ATOMIC_ACTIVATION_IDEMPOTENCY_SCOPE
        || material.key != material.activation_root_digest
    {
        bail!("V277 idempotency material is caller-selectable or drifted")
    }
    canonical_material_json_and_digest(IDEMPOTENCY_DOMAIN, material)
}

pub(crate) fn canonical_atomic_activation_confirmation_json_and_digest(
    material: &ExternalPoolAdapterAtomicActivationConfirmationMaterial,
) -> Result<(String, String)> {
    if material.confirmation != ATOMIC_ACTIVATION_CONFIRMATION
        || material.actor_kind != ATOMIC_ACTIVATION_ACTOR_KIND
    {
        bail!("V277 confirmation material is not exact")
    }
    canonical_material_json_and_digest(CONFIRMATION_DOMAIN, material)
}

pub(crate) fn canonical_external_pool_adapter_atomic_activation_receipt_json_and_digest(
    receipt: &ExternalPoolAdapterAtomicActivationReceipt,
) -> Result<(String, String)> {
    #[derive(Serialize)]
    struct DigestProjection<'a> {
        schema: &'a str,
        activation_receipt_id: &'a str,
        canonicalization: &'a str,
        digest_algorithm: &'a str,
        activation: &'a ExternalPoolAdapterAtomicActivationMaterial,
    }
    let projection = DigestProjection {
        schema: &receipt.schema,
        activation_receipt_id: &receipt.activation_receipt_id,
        canonicalization: &receipt.canonicalization,
        digest_algorithm: &receipt.digest_algorithm,
        activation: &receipt.activation,
    };
    let digest = domain_digest(RECEIPT_DOMAIN, &projection)?;
    Ok((canonical_json(receipt)?, digest))
}

pub(crate) fn build_external_pool_adapter_atomic_activation_receipt(
    activation_receipt_id: String,
    activation: ExternalPoolAdapterAtomicActivationMaterial,
) -> Result<ExternalPoolAdapterAtomicActivationReceipt> {
    let mut receipt = ExternalPoolAdapterAtomicActivationReceipt {
        schema: ATOMIC_ACTIVATION_RECEIPT_SCHEMA.into(),
        activation_receipt_id,
        activation_receipt_digest: String::new(),
        canonicalization: ATOMIC_ACTIVATION_CANONICALIZATION.into(),
        digest_algorithm: ATOMIC_ACTIVATION_DIGEST_ALGORITHM.into(),
        activation,
    };
    receipt.activation_receipt_digest =
        canonical_external_pool_adapter_atomic_activation_receipt_json_and_digest(&receipt)?.1;
    validate_external_pool_adapter_atomic_activation_receipt(&receipt)?;
    Ok(receipt)
}

pub(crate) fn atomic_activation_json_is_canonical<T>(json: &str) -> bool
where
    T: serde::de::DeserializeOwned + Serialize,
{
    if json.len() > ATOMIC_ACTIVATION_MAX_JSON_BYTES {
        return false;
    }
    serde_json::from_str::<T>(json).is_ok_and(|value| {
        canonical_compute_plugin_ijson_and_sha256(&value, ATOMIC_ACTIVATION_MAX_JSON_BYTES)
            .is_ok_and(|(canonical, _)| canonical == json)
    })
}

fn canonical_material_json_and_digest<T: Serialize>(
    domain: &[u8],
    material: &T,
) -> Result<(String, String)> {
    let json = canonical_json(material)?;
    let digest = domain_digest_from_json(domain, &json);
    Ok((json, digest))
}

fn domain_digest<T: Serialize + ?Sized>(domain: &[u8], value: &T) -> Result<String> {
    Ok(domain_digest_from_json(domain, &canonical_json(value)?))
}

fn domain_digest_from_json(domain: &[u8], json: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(json.as_bytes());
    hex::encode(digest.finalize())
}

fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    canonical_compute_plugin_ijson_and_sha256(value, ATOMIC_ACTIVATION_MAX_JSON_BYTES)
        .map(|(json, _)| json)
}
