use anyhow::{bail, Result};
use chrono::Duration;

use super::{
    super::*, support, validate_active_provider_evidence, validate_registering_provider_evidence,
};

pub(crate) fn validate_external_pool_adapter_provider_active_successor_receipt(
    value: &ExternalPoolAdapterProviderActiveSuccessorReceipt,
) -> Result<()> {
    support::identifier(&value.active_successor_receipt_id)?;
    support::digest(&value.receipt_digest)?;
    if value.schema != PROVIDER_ACTIVE_SUCCESSOR_RECEIPT_SCHEMA
        || value.canonicalization != PROVIDER_ACTIVE_SUCCESSOR_CANONICALIZATION
        || value.digest_algorithm != PROVIDER_ACTIVE_SUCCESSOR_DIGEST_ALGORITHM
        || canonical_external_pool_adapter_provider_active_successor_receipt_json_and_digest(value)?
            .1
            != value.receipt_digest
    {
        bail!("provider active successor receipt metadata is not exact")
    }
    validate_external_pool_adapter_provider_active_successor_activation_root(
        &value.successor.activation,
    )?;
    validate_lineage(&value.successor.lineage)?;
    validate_provider_evidence(value)?;
    validate_timestamps(value)?;
    validate_provider_active_successor_boundary(
        &value.successor.effects,
        &value.successor.readiness,
    )?;
    for identifier in [
        &value.successor.credential_evidence.reattestation_receipt_id,
        &value.successor.runtime_observation.runtime_observation_id,
        &value
            .successor
            .task_protocol_evidence
            .task_protocol_conformance_run_receipt_id,
        &value.successor.activation_witness.activation_witness_id,
    ] {
        support::identifier(identifier)?;
    }
    for digest in [
        &value
            .successor
            .credential_evidence
            .reattestation_receipt_digest,
        &value
            .successor
            .runtime_observation
            .runtime_observation_digest,
        &value
            .successor
            .task_protocol_evidence
            .task_protocol_conformance_run_receipt_digest,
        &value.successor.activation_witness.activation_witness_digest,
    ] {
        support::digest(digest)?;
    }
    if provider_active_successor_runtime_observation_digest(&value.successor.runtime_observation)?
        != value
            .successor
            .runtime_observation
            .runtime_observation_digest
    {
        bail!("provider active successor runtime observation digest is not exact")
    }
    Ok(())
}

pub(crate) fn validate_external_pool_adapter_provider_active_successor_revocation(
    value: &ExternalPoolAdapterProviderActiveSuccessorRevocationReceipt,
) -> Result<()> {
    let revocation = &value.revocation;
    for identifier in [
        &value.active_successor_revocation_id,
        &revocation.target_active_successor_receipt_id,
        &revocation.provider_binding_id,
        &revocation.revoked_by_actor_user_id,
        &revocation.idempotency_scope,
        &revocation.idempotency_key,
    ] {
        support::identifier(identifier)?;
    }
    for digest in [
        &value.revocation_digest,
        &revocation.target_active_successor_receipt_digest,
        &revocation.activation_root_digest,
        &revocation.idempotency_digest,
        &revocation.confirmation_digest,
    ] {
        support::digest(digest)?;
    }
    support::reason_code(&revocation.reason_code)?;
    support::canonical_nanos(&revocation.revoked_at)?;
    if value.schema != PROVIDER_ACTIVE_SUCCESSOR_REVOCATION_SCHEMA
        || value.canonicalization != PROVIDER_ACTIVE_SUCCESSOR_CANONICALIZATION
        || value.digest_algorithm != PROVIDER_ACTIVE_SUCCESSOR_DIGEST_ALGORITHM
        || revocation.revoked_by_actor_kind != PROVIDER_ACTIVE_SUCCESSOR_REVOCATION_ACTOR_KIND
        || revocation.confirmation != PROVIDER_ACTIVE_SUCCESSOR_REVOCATION_CONFIRMATION
        || provider_active_successor_idempotency_digest(
            &revocation.revoked_by_actor_kind,
            &revocation.revoked_by_actor_user_id,
            &revocation.idempotency_scope,
            &revocation.idempotency_key,
        )? != revocation.idempotency_digest
        || provider_active_successor_confirmation_digest(&revocation.confirmation)?
            != revocation.confirmation_digest
        || canonical_external_pool_adapter_provider_active_successor_revocation_json_and_digest(
            value,
        )?
        .1 != value.revocation_digest
    {
        bail!("provider active successor revocation is not exact")
    }
    validate_provider_active_successor_boundary(&revocation.effects, &revocation.readiness)
}

fn validate_provider_evidence(
    value: &ExternalPoolAdapterProviderActiveSuccessorReceipt,
) -> Result<()> {
    let successor = &value.successor;
    let root = &successor.activation.activation_root;
    let live = validate_active_provider_evidence(&successor.evidence_provider, root)?;
    let runtime =
        validate_active_provider_evidence(&successor.runtime_observation.observed_provider, root)?;
    if successor.evidence_provider != successor.runtime_observation.observed_provider
        || live != runtime
        || support::canonical_nanos(&live.updated_at)?
            > support::canonical_nanos(&successor.checked_at)?
    {
        bail!("provider active successor runtime observation subject is not live Provider")
    }
    if successor.lineage.successor_sequence == 1 {
        validate_registering_provider_evidence(
            &successor.credential_evidence.observed_provider,
            root,
        )?;
        if successor.evidence_provider.provider_json != root.initial_active_provider_json
            || successor.evidence_provider.provider_digest != root.initial_active_provider_digest
            || live.updated_at != successor.checked_at
        {
            bail!("provider active successor genesis is not the adjacent active Provider")
        }
    } else {
        validate_active_provider_evidence(&successor.credential_evidence.observed_provider, root)?;
        if successor.credential_evidence.observed_provider != successor.evidence_provider {
            bail!("provider active successor refresh credential subject is stale")
        }
    }
    Ok(())
}

fn validate_timestamps(value: &ExternalPoolAdapterProviderActiveSuccessorReceipt) -> Result<()> {
    let successor = &value.successor;
    let observation = &successor.runtime_observation;
    let started = support::canonical_nanos(&observation.observation_started_at)?;
    let completed = support::canonical_nanos(&observation.observation_completed_at)?;
    let expires = support::canonical_nanos(&observation.observation_expires_at)?;
    let protocol_expires = support::canonical_nanos(
        &successor
            .task_protocol_evidence
            .task_protocol_conformance_expires_at,
    )?;
    let checked = support::canonical_nanos(&successor.checked_at)?;
    let created = support::canonical_nanos(&successor.created_at)?;
    if started > completed
        || completed > checked
        || checked >= expires
        || expires
            > completed + Duration::seconds(PROVIDER_ACTIVE_SUCCESSOR_MAX_OBSERVATION_SECONDS)
        || expires > protocol_expires
        || created != checked
    {
        bail!("provider active successor freshness window is invalid")
    }
    Ok(())
}

fn validate_lineage(value: &ExternalPoolAdapterProviderActiveSuccessorLineage) -> Result<()> {
    let id = &value.predecessor_active_successor_receipt_id;
    let digest = &value.predecessor_active_successor_receipt_digest;
    if value.successor_sequence == 0
        || id.is_some() != digest.is_some()
        || (value.successor_sequence == 1) != id.is_none()
    {
        bail!("provider active successor lineage is incomplete")
    }
    if let Some(id) = id {
        support::identifier(id)?;
    }
    if let Some(digest) = digest {
        support::digest(digest)?;
    }
    Ok(())
}
