use anyhow::{bail, Result};

use crate::compute_federation::{
    external_pool_adapter_task_protocol_production::{
        derive_external_pool_adapter_task_production_lane_subject,
        validate_task_production_carrier_policy_digest,
        ExternalPoolAdapterTaskProductionLaneSubjectInput,
    },
    provider::{
        ComputeProvider, PROVIDER_KIND_EXTERNAL_POOL, PROVIDER_STATUS_ACTIVE,
        PROVIDER_STATUS_REGISTERING,
    },
};

use super::{super::*, support};

pub(crate) fn validate_external_pool_adapter_provider_active_successor_activation_root(
    value: &ExternalPoolAdapterProviderActiveSuccessorActivationRoot,
) -> Result<()> {
    let root = &value.activation_root;
    identifiers(root)?;
    digests(root)?;
    validate_task_production_carrier_policy_digest(&root.task_production_carrier_policy_digest)?;
    let lane = derive_external_pool_adapter_task_production_lane_subject(
        ExternalPoolAdapterTaskProductionLaneSubjectInput {
            provider_id: root.provider_id.clone(),
            provider_owner_account_id: root.provider_owner_account_id.clone(),
            provider_binding_id: root.provider_binding_id.clone(),
            provider_binding_digest: root.provider_binding_digest.clone(),
            registry_release_id: root.registry_release_id.clone(),
            registry_release_digest: root.registry_release_digest.clone(),
            route_adapter_projection_id: root.route_adapter_projection_id.clone(),
            logical_adapter_binding_digest: root.logical_adapter_binding_digest.clone(),
            logical_projection_compatibility_digest: root
                .logical_projection_compatibility_digest
                .clone(),
        },
    )?;
    if value.schema != PROVIDER_ACTIVE_SUCCESSOR_ACTIVATION_ROOT_SCHEMA
        || value.canonicalization != PROVIDER_ACTIVE_SUCCESSOR_CANONICALIZATION
        || value.digest_algorithm != PROVIDER_ACTIVE_SUCCESSOR_DIGEST_ALGORITHM
        || activation_root_digest(root)? != value.activation_root_digest
        || lane.lane_subject_digest != root.lane_subject_digest
    {
        bail!("provider active successor activation root metadata is not exact")
    }

    let source = exact_provider(
        &root.source_registering_provider_json,
        &root.source_registering_provider_digest,
    )?;
    let initial = exact_provider(
        &root.initial_active_provider_json,
        &root.initial_active_provider_digest,
    )?;
    if source.provider_id != root.provider_id
        || source.provider_id != root.source_registering_provider_id
        || source.owner_account_id != root.provider_owner_account_id
        || source.provider_kind != PROVIDER_KIND_EXTERNAL_POOL
        || source.status != PROVIDER_STATUS_REGISTERING
        || source.policy_revision != root.source_registering_provider_policy_revision
        || source.adapter.as_ref().map(|item| item.adapter_id.as_str())
            != Some(root.logical_adapter_id.as_str())
    {
        bail!("provider active successor registering Provider is not exact")
    }
    let mut expected = source.clone();
    expected.policy_revision = source
        .policy_revision
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("provider policy revision overflow"))?;
    expected.status = PROVIDER_STATUS_ACTIVE.into();
    expected.updated_at = initial.updated_at.clone();
    expected
        .adapter
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("external-pool Provider lacks adapter"))?
        .adapter_id = root.route_adapter_projection_id.clone();
    if initial != expected
        || initial.provider_id != root.initial_active_provider_id
        || initial.policy_revision != root.initial_active_provider_policy_revision
    {
        bail!("provider active successor target is not the adjacent projected Provider")
    }
    Ok(())
}

pub(super) fn validate_active_provider_evidence(
    value: &ExternalPoolAdapterProviderActiveSuccessorProviderEvidence,
    root: &ExternalPoolAdapterProviderActiveSuccessorActivationRootEnvelope,
) -> Result<ComputeProvider> {
    support::identifier(&value.provider_id)?;
    support::digest(&value.provider_digest)?;
    let provider = exact_provider(&value.provider_json, &value.provider_digest)?;
    let adapter = provider
        .adapter
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("active external-pool Provider lacks adapter"))?;
    let initial: ComputeProvider = serde_json::from_str(&root.initial_active_provider_json)?;
    let initial_adapter = initial
        .adapter
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("initial active Provider lacks adapter"))?;
    if provider.provider_id != value.provider_id
        || provider.policy_revision != value.provider_policy_revision
        || provider.provider_id != root.provider_id
        || provider.owner_account_id != root.provider_owner_account_id
        || provider.provider_kind != PROVIDER_KIND_EXTERNAL_POOL
        || provider.status != PROVIDER_STATUS_ACTIVE
        || provider.created_at != initial.created_at
        || provider.policy_revision < root.initial_active_provider_policy_revision
        || adapter.adapter_id != root.route_adapter_projection_id
        || adapter.adapter_version != initial_adapter.adapter_version
        || adapter.config_revision != initial_adapter.config_revision
        || adapter.config_digest != initial_adapter.config_digest
    {
        bail!("provider active successor live Provider evidence is not exact")
    }
    Ok(provider)
}

pub(super) fn validate_registering_provider_evidence(
    value: &ExternalPoolAdapterProviderActiveSuccessorProviderEvidence,
    root: &ExternalPoolAdapterProviderActiveSuccessorActivationRootEnvelope,
) -> Result<()> {
    let provider = exact_provider(&value.provider_json, &value.provider_digest)?;
    if provider.provider_id != value.provider_id
        || provider.policy_revision != value.provider_policy_revision
        || value.provider_id != root.source_registering_provider_id
        || value.provider_policy_revision != root.source_registering_provider_policy_revision
        || value.provider_json != root.source_registering_provider_json
        || value.provider_digest != root.source_registering_provider_digest
        || provider.status != PROVIDER_STATUS_REGISTERING
    {
        bail!("provider active successor registering credential evidence is not exact")
    }
    Ok(())
}

fn exact_provider(json: &str, digest: &str) -> Result<ComputeProvider> {
    if json.len() > PROVIDER_ACTIVE_SUCCESSOR_MAX_JSON_BYTES {
        bail!("provider active successor Provider JSON exceeds the bound")
    }
    support::digest(digest)?;
    let provider: ComputeProvider = serde_json::from_str(json)?;
    if provider_json_and_digest(&provider)? != (json.to_owned(), digest.to_owned()) {
        bail!("provider active successor Provider JSON/digest is not exact")
    }
    Ok(provider)
}

fn identifiers(
    root: &ExternalPoolAdapterProviderActiveSuccessorActivationRootEnvelope,
) -> Result<()> {
    for value in [
        &root.provider_id,
        &root.provider_owner_account_id,
        &root.source_registering_provider_id,
        &root.initial_active_provider_id,
        &root.provider_binding_id,
        &root.registry_release_id,
        &root.installation_receipt_id,
        &root.candidate_id,
        &root.delegation_id,
        &root.service_actor_id,
        &root.logical_adapter_id,
        &root.route_adapter_projection_id,
        &root.profile_id,
        &root.target_id,
        &root.companion_id,
    ] {
        support::identifier(value)?;
    }
    Ok(())
}

fn digests(root: &ExternalPoolAdapterProviderActiveSuccessorActivationRootEnvelope) -> Result<()> {
    for value in [
        &root.source_registering_provider_digest,
        &root.initial_active_provider_digest,
        &root.provider_binding_digest,
        &root.registry_release_digest,
        &root.registry_release_material_digest,
        &root.installation_receipt_digest,
        &root.installation_content_digest,
        &root.candidate_digest,
        &root.delegation_digest,
        &root.logical_adapter_binding_digest,
        &root.logical_projection_compatibility_digest,
        &root.profile_digest,
        &root.launch_policy_digest,
        &root.target_digest,
        &root.target_policy_digest,
        &root.companion_digest,
        &root.supervisor_session_policy_digest,
        &root.entrypoint_capsule_policy_digest,
        &root.launch_image_sha256,
        &root.task_protocol_profile_digest,
        &root.lane_subject_digest,
        &root.task_production_carrier_policy_digest,
    ] {
        support::digest(value)?;
    }
    Ok(())
}
