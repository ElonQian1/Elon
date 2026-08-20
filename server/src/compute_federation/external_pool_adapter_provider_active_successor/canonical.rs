use anyhow::{bail, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::{
        external_pool_adapter_task_protocol_production::TASK_PRODUCTION_CARRIER_POLICY_DIGEST,
        provider::{
            ComputeProvider, PROVIDER_KIND_EXTERNAL_POOL, PROVIDER_STATUS_ACTIVE,
            PROVIDER_STATUS_REGISTERING,
        },
    },
    compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256,
};

use super::*;

const ACTIVATION_ROOT_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-ACTIVE-SUCCESSOR-ACTIVATION-ROOT-V1";
const RECEIPT_DOMAIN: &[u8] = b"ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-ACTIVE-SUCCESSOR-RECEIPT-V1";
const RUNTIME_OBSERVATION_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-ACTIVE-SUCCESSOR-RUNTIME-OBSERVATION-V1";
const REVOCATION_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-ACTIVE-SUCCESSOR-REVOCATION-V1";
const IDEMPOTENCY_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-ACTIVE-SUCCESSOR-IDEMPOTENCY-V1";
const CONFIRMATION_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-ACTIVE-SUCCESSOR-CONFIRMATION-V1";
const RECEIPT_INTEGRITY_DOMAIN: &[u8] =
    b"ELON-EXTERNAL-POOL-ADAPTER-PROVIDER-ACTIVE-SUCCESSOR-PRIVATE-INTEGRITY-V1";

pub(crate) fn derive_external_pool_adapter_provider_active_successor_activation_root(
    source: &ComputeProvider,
    structural: ExternalPoolAdapterProviderActiveSuccessorStructuralInput,
    activation_target_updated_at: &str,
) -> Result<ExternalPoolAdapterProviderActiveSuccessorActivationRoot> {
    if source.provider_kind != PROVIDER_KIND_EXTERNAL_POOL
        || source.status != PROVIDER_STATUS_REGISTERING
        || source.provider_id != structural.provider_id
        || source.owner_account_id != structural.provider_owner_account_id
        || source
            .adapter
            .as_ref()
            .map(|value| value.adapter_id.as_str())
            != Some(structural.logical_adapter_id.as_str())
        || structural.task_production_carrier_policy_digest != TASK_PRODUCTION_CARRIER_POLICY_DIGEST
    {
        bail!("provider active successor source is not the exact registering subject")
    }
    let mut initial = source.clone();
    initial.policy_revision = source
        .policy_revision
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("provider policy revision overflow"))?;
    initial.status = PROVIDER_STATUS_ACTIVE.into();
    initial.updated_at = activation_target_updated_at.into();
    initial
        .adapter
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("external-pool Provider lacks adapter"))?
        .adapter_id = structural.route_adapter_projection_id.clone();

    let source_json = serde_json::to_string(source)?;
    let initial_json = serde_json::to_string(&initial)?;
    let envelope = ExternalPoolAdapterProviderActiveSuccessorActivationRootEnvelope {
        provider_id: structural.provider_id,
        provider_owner_account_id: structural.provider_owner_account_id,
        source_registering_provider_id: source.provider_id.clone(),
        source_registering_provider_policy_revision: source.policy_revision,
        source_registering_provider_digest: sha256_hex(source_json.as_bytes()),
        source_registering_provider_json: source_json,
        initial_active_provider_id: initial.provider_id.clone(),
        initial_active_provider_policy_revision: initial.policy_revision,
        initial_active_provider_digest: sha256_hex(initial_json.as_bytes()),
        initial_active_provider_json: initial_json,
        provider_binding_id: structural.provider_binding_id,
        provider_binding_digest: structural.provider_binding_digest,
        registry_release_id: structural.registry_release_id,
        registry_release_digest: structural.registry_release_digest,
        registry_release_material_digest: structural.registry_release_material_digest,
        installation_receipt_id: structural.installation_receipt_id,
        installation_receipt_digest: structural.installation_receipt_digest,
        installation_content_digest: structural.installation_content_digest,
        candidate_id: structural.candidate_id,
        candidate_digest: structural.candidate_digest,
        delegation_id: structural.delegation_id,
        delegation_digest: structural.delegation_digest,
        service_actor_id: structural.service_actor_id,
        logical_adapter_id: structural.logical_adapter_id,
        logical_adapter_binding_digest: structural.logical_adapter_binding_digest,
        logical_projection_compatibility_digest: structural.logical_projection_compatibility_digest,
        route_adapter_projection_id: structural.route_adapter_projection_id,
        profile_id: structural.profile_id,
        profile_digest: structural.profile_digest,
        launch_policy_digest: structural.launch_policy_digest,
        target_id: structural.target_id,
        target_digest: structural.target_digest,
        target_policy_digest: structural.target_policy_digest,
        companion_id: structural.companion_id,
        companion_digest: structural.companion_digest,
        supervisor_session_policy_digest: structural.supervisor_session_policy_digest,
        entrypoint_capsule_policy_digest: structural.entrypoint_capsule_policy_digest,
        launch_image_sha256: structural.launch_image_sha256,
        task_protocol_profile_digest: structural.task_protocol_profile_digest,
        lane_subject_digest: structural.lane_subject_digest,
        task_production_carrier_policy_digest: structural.task_production_carrier_policy_digest,
    };
    let activation_root_digest = domain_digest(ACTIVATION_ROOT_DOMAIN, &envelope)?;
    let root = ExternalPoolAdapterProviderActiveSuccessorActivationRoot {
        schema: PROVIDER_ACTIVE_SUCCESSOR_ACTIVATION_ROOT_SCHEMA.into(),
        canonicalization: PROVIDER_ACTIVE_SUCCESSOR_CANONICALIZATION.into(),
        digest_algorithm: PROVIDER_ACTIVE_SUCCESSOR_DIGEST_ALGORITHM.into(),
        activation_root_digest,
        activation_root: envelope,
    };
    validate_external_pool_adapter_provider_active_successor_activation_root(&root)?;
    Ok(root)
}

pub(crate) fn canonical_provider_active_successor_activation_root_json(
    root: &ExternalPoolAdapterProviderActiveSuccessorActivationRoot,
) -> Result<String> {
    canonical_json(root)
}

pub(crate) fn canonical_external_pool_adapter_provider_active_successor_receipt_json_and_digest(
    receipt: &ExternalPoolAdapterProviderActiveSuccessorReceipt,
) -> Result<(String, String)> {
    envelope_digest(
        receipt,
        "receipt_digest",
        RECEIPT_DOMAIN,
        "successor receipt",
    )
}

pub(crate) fn provider_active_successor_runtime_observation_digest(
    value: &ExternalPoolAdapterProviderActiveSuccessorRuntimeObservation,
) -> Result<String> {
    #[derive(Serialize)]
    struct Material<'a> {
        runtime_observation_id: &'a str,
        observed_provider: &'a ExternalPoolAdapterProviderActiveSuccessorProviderEvidence,
        observation_started_at: &'a str,
        observation_completed_at: &'a str,
        observation_expires_at: &'a str,
    }
    domain_digest(
        RUNTIME_OBSERVATION_DOMAIN,
        &Material {
            runtime_observation_id: &value.runtime_observation_id,
            observed_provider: &value.observed_provider,
            observation_started_at: &value.observation_started_at,
            observation_completed_at: &value.observation_completed_at,
            observation_expires_at: &value.observation_expires_at,
        },
    )
}

pub(crate) fn canonical_external_pool_adapter_provider_active_successor_revocation_json_and_digest(
    receipt: &ExternalPoolAdapterProviderActiveSuccessorRevocationReceipt,
) -> Result<(String, String)> {
    envelope_digest(
        receipt,
        "revocation_digest",
        REVOCATION_DOMAIN,
        "successor revocation",
    )
}

pub(crate) fn provider_active_successor_idempotency_digest(
    actor_kind: &str,
    actor_user_id: &str,
    scope: &str,
    key: &str,
) -> Result<String> {
    #[derive(Serialize)]
    struct Material<'a> {
        actor_kind: &'a str,
        actor_user_id: &'a str,
        idempotency_scope: &'a str,
        idempotency_key: &'a str,
    }
    domain_digest(
        IDEMPOTENCY_DOMAIN,
        &Material {
            actor_kind,
            actor_user_id,
            idempotency_scope: scope,
            idempotency_key: key,
        },
    )
}

pub(crate) fn provider_active_successor_confirmation_digest(value: &str) -> Result<String> {
    domain_digest(CONFIRMATION_DOMAIN, value)
}

pub(crate) fn provider_active_successor_private_integrity_digest(
    kind: &str,
    entity_digest: &str,
    custody: &ExternalPoolAdapterProviderActiveSuccessorProcessCustody,
) -> Result<String> {
    #[derive(Serialize)]
    struct Material<'a> {
        kind: &'a str,
        entity_digest: &'a str,
        process_custody_epoch_digest: &'a str,
        process_custody_nonce_digest: &'a str,
        process_custody_seal_digest: &'a str,
    }
    domain_digest(
        RECEIPT_INTEGRITY_DOMAIN,
        &Material {
            kind,
            entity_digest,
            process_custody_epoch_digest: &custody.process_custody_epoch_digest,
            process_custody_nonce_digest: &custody.process_custody_nonce_digest,
            process_custody_seal_digest: &custody.process_custody_seal_digest,
        },
    )
}

pub(super) fn provider_json_and_digest(value: &ComputeProvider) -> Result<(String, String)> {
    let json = serde_json::to_string(value)?;
    let digest = sha256_hex(json.as_bytes());
    Ok((json, digest))
}

pub(super) fn activation_root_digest<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    domain_digest(ACTIVATION_ROOT_DOMAIN, value)
}

fn envelope_digest<T: Serialize>(
    value: &T,
    digest_field: &str,
    domain: &[u8],
    kind: &str,
) -> Result<(String, String)> {
    let object = serde_json::to_value(value)?;
    let mut projection = object
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("{kind} must be an object"))?
        .clone();
    if projection
        .insert(
            digest_field.into(),
            serde_json::Value::String(String::new()),
        )
        .is_none()
    {
        bail!("{kind} lacks its digest field")
    }
    Ok((canonical_json(value)?, domain_digest(domain, &projection)?))
}

fn domain_digest<T: Serialize + ?Sized>(domain: &[u8], value: &T) -> Result<String> {
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(canonical_json(value)?.as_bytes());
    Ok(hex::encode(digest.finalize()))
}

fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    canonical_compute_plugin_ijson_and_sha256(value, PROVIDER_ACTIVE_SUCCESSOR_MAX_JSON_BYTES)
        .map(|item| item.0)
}

fn sha256_hex(value: &[u8]) -> String {
    hex::encode(Sha256::digest(value))
}
