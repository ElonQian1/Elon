//! Canonical sealed genesis route reconstructed from exact typed Store custody.

use anyhow::{ensure, Result};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use rusqlite::Transaction;

use crate::{
    compute_federation::{
        external_pool_adapter_atomic_activation::{
            ExternalPoolProjectedV211AdapterBinding, ExternalPoolStableExecutorBinding,
        },
        provider::PROVIDER_KIND_EXTERNAL_POOL,
        route_authority::*,
    },
    store::{
        compute_external_pool_adapter_credential_reattestation::PreparedExternalPoolAdapterCredentialProjectedActiveTransition,
        compute_external_pool_adapter_runtime_bundle::ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject,
        compute_external_pool_adapter_task_protocol_conformance::PreparedExternalPoolAdapterTaskProtocolPlannedActiveCarrier,
        compute_external_pool_onboarding::historical_external_pool_onboarding_application_authority_on,
        new_id,
    },
};

pub(super) fn build_genesis_route_on<'tx, 'conn>(
    transaction: &'tx Transaction<'conn>,
    no_work: &ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject<'_, 'tx, 'conn>,
    transition: &PreparedExternalPoolAdapterCredentialProjectedActiveTransition<'_, 'tx, 'conn>,
    task_protocol: &PreparedExternalPoolAdapterTaskProtocolPlannedActiveCarrier<'tx, 'conn>,
    stable: &ExternalPoolStableExecutorBinding,
    projected: &ExternalPoolProjectedV211AdapterBinding,
) -> Result<AuthorizedComputeRouteAuthorization> {
    let planned = no_work.preflight();
    let root = &planned.activation_root().activation_root;
    let observation = no_work.observation();
    let candidate = observation.companion().target().profile().candidate();
    let registry = candidate.registry();
    let binding = registry.binding();
    let binding_material = &binding.binding;
    let release = registry.release();
    let release_material = &release.release;
    let delegation = candidate.delegation();
    let delegation_material = &delegation.delegation;
    let onboarding = historical_external_pool_onboarding_application_authority_on(
        transaction,
        &binding_material.application_id,
        &binding_material.application_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("V277 genesis route lost its audited onboarding source"))?;
    ensure!(
        onboarding.provider() == &planned.source().provider
            && onboarding.provider_digest() == planned.source().provider_digest
            && onboarding.approved_by_user_id() == root.provider_owner_account_id
            && onboarding.adapter_id() == root.logical_adapter_id
            && onboarding.adapter_release_version() == release_material.release_version
            && onboarding.adapter_config_revision() == binding_material.adapter_config_revision
            && onboarding.adapter_config_digest() == binding_material.adapter_config_digest
            && release.registry_release_id == root.registry_release_id
            && release.registry_release_digest == root.registry_release_digest
            && candidate.candidate().candidate_id == root.candidate_id
            && candidate.candidate().candidate_digest == root.candidate_digest
            && delegation.delegation_id == root.delegation_id
            && delegation.delegation_digest == root.delegation_digest
            && delegation_material.service_actor_id == root.service_actor_id,
        "V277 genesis route typed source roots are not exact"
    );
    let evidence = transition.credential().receipt();
    let v253 = &evidence.reattestation.binding;
    let verifier = ComputeRouteCredentialVerifierBinding {
        verification_kind: release_material
            .credential_verifier
            .verification_kind
            .clone(),
        verifier_id: release_material.credential_verifier.verifier_id.clone(),
        verifier_revision: release_material.credential_verifier.verifier_revision,
        verifier_digest: release_material.credential_verifier.verifier_digest.clone(),
    };
    ensure!(
        verifier.verification_kind == v253.expected_credential_verifier.verification_kind
            && verifier.verifier_id == v253.expected_credential_verifier.verifier_id
            && verifier.verifier_revision == v253.expected_credential_verifier.verifier_revision
            && verifier.verifier_digest == v253.expected_credential_verifier.verifier_digest,
        "V277 genesis route V253 verifier differs from the registry release"
    );
    let (expires_at, cleanup_expires_at) = expiries(
        no_work.evidence_checked_at(),
        &task_protocol.fresh_expires_at_for(no_work)?,
        &evidence.reattestation.binding.report_expires_at,
    )?;
    let provider = ComputeRouteProviderBinding {
        provider_id: planned.target().provider_id.clone(),
        provider_kind: PROVIDER_KIND_EXTERNAL_POOL.into(),
        provider_owner_account_id: root.provider_owner_account_id.clone(),
    };
    let actor = actor_envelope(
        root,
        delegation_material,
        no_work.evidence_checked_at(),
        &cleanup_expires_at,
    )?;
    let adapter = adapter_envelope(
        root,
        release_material,
        &actor,
        &verifier,
        no_work.evidence_checked_at(),
    )?;
    let shape = ComputeRouteShape {
        route_kind: release_material.route_kind.clone(),
        route_binding_digest: projected.projected_v211_adapter_binding_digest.clone(),
        adapter_binding_digest: projected.projected_v211_adapter_binding_digest.clone(),
        endpoint_id: None,
        endpoint_transport: None,
        adapter: ComputeRouteAdapterBinding {
            adapter_id: root.route_adapter_projection_id.clone(),
            adapter_revision: adapter.adapter_revision,
            adapter_registry_digest: adapter.adapter_digest.clone(),
            adapter_release_version: release_material.release_version.clone(),
            implementation_digest: release_material.implementation_digest.clone(),
            config_revision: binding_material.adapter_config_revision,
            config_digest: binding_material.adapter_config_digest.clone(),
        },
    };
    let credential = credential_envelope(
        provider.clone(),
        shape.clone(),
        onboarding.non_bearer_credential_ref(),
        onboarding.credential_hint(),
        verifier.clone(),
        evidence,
        &actor,
        no_work.evidence_checked_at(),
        &expires_at,
        &cleanup_expires_at,
    )?;
    let capabilities = release_material
        .supported_capabilities
        .iter()
        .enumerate()
        .map(|(ordinal, capability)| ComputeRouteCapabilityBinding {
            ordinal: ordinal as i64,
            capability_id: capability.capability_id.clone(),
            capability_revision: capability.capability_revision,
        })
        .collect::<Vec<_>>();
    let authorization = authorization_envelope(
        provider,
        shape,
        stable,
        &credential,
        &actor,
        capabilities,
        onboarding.application_id(),
        onboarding.application_digest(),
        onboarding.approved_by_user_id(),
        no_work.evidence_checked_at(),
        &expires_at,
        &cleanup_expires_at,
    )?;
    let seal = seal_envelope(
        &adapter,
        &credential,
        &authorization,
        no_work.evidence_checked_at(),
    )?;
    validated_compute_route_authorization_from_canonical_envelopes(
        adapter,
        credential,
        actor,
        authorization,
        seal,
    )
}

fn actor_envelope(
    root: &crate::compute_federation::external_pool_adapter_provider_active_successor::ExternalPoolAdapterProviderActiveSuccessorActivationRootEnvelope,
    delegation: &crate::compute_federation::external_pool_provider_activation_candidate::ExternalPoolProviderActivationDelegationMaterial,
    checked_at: &str,
    cleanup_expires_at: &str,
) -> Result<ComputeServiceActorAuthorizationEnvelope> {
    let mut actor = ComputeServiceActorAuthorizationEnvelope {
        schema: COMPUTE_SERVICE_ACTOR_AUTHORIZATION_SCHEMA.into(),
        actor_authorization_id: new_id("external_pool_adapter_service_actor_authorization"),
        actor_authorization_revision: 1,
        actor_authorization_digest: String::new(),
        canonicalization: COMPUTE_ROUTE_CANONICALIZATION.into(),
        digest_algorithm: COMPUTE_ROUTE_DIGEST_ALGORITHM.into(),
        authorization: ComputeServiceActorAuthorization {
            provider_id: root.initial_active_provider_id.clone(),
            provider_owner_account_id: root.provider_owner_account_id.clone(),
            service_actor_id: root.service_actor_id.clone(),
            service_actor_kind: delegation.service_actor_kind.clone(),
            allowed_route_kinds: delegation.allowed_route_kinds.clone(),
            allowed_actor_phases: delegation.allowed_actor_phases.clone(),
            issued_by_user_id: delegation.issued_by_owner_user_id.clone(),
            issued_at: checked_at.into(),
            valid_until: cleanup_expires_at.into(),
            recorded_at: checked_at.into(),
        },
    };
    actor.actor_authorization_digest =
        canonical_service_actor_authorization_json_and_digest(&actor)?.1;
    Ok(actor)
}

fn adapter_envelope(
    root: &crate::compute_federation::external_pool_adapter_provider_active_successor::ExternalPoolAdapterProviderActiveSuccessorActivationRootEnvelope,
    release: &crate::compute_federation::external_pool_adapter_registry::ExternalPoolAdapterRegistryReleaseMaterial,
    actor: &ComputeServiceActorAuthorizationEnvelope,
    verifier: &ComputeRouteCredentialVerifierBinding,
    checked_at: &str,
) -> Result<ComputeRouteAdapterVersionEnvelope> {
    let mut adapter = ComputeRouteAdapterVersionEnvelope {
        schema: COMPUTE_ROUTE_ADAPTER_VERSION_SCHEMA.into(),
        adapter_id: root.route_adapter_projection_id.clone(),
        adapter_revision: 1,
        adapter_digest: String::new(),
        canonicalization: COMPUTE_ROUTE_CANONICALIZATION.into(),
        digest_algorithm: COMPUTE_ROUTE_DIGEST_ALGORITHM.into(),
        adapter: ComputeRouteAdapterVersion {
            release_version: release.release_version.clone(),
            implementation_digest: release.implementation_digest.clone(),
            route_kind: release.route_kind.clone(),
            supported_provider_kinds: release.supported_provider_kinds.clone(),
            credential_verifier: verifier.clone(),
            supported_capabilities: release
                .supported_capabilities
                .iter()
                .map(|capability| ComputeRouteCapabilityRevision {
                    capability_id: capability.capability_id.clone(),
                    capability_revision: capability.capability_revision,
                })
                .collect(),
            status: COMPUTE_ROUTE_ADAPTER_STATUS_ACTIVE.into(),
            registered_by_service_actor_id: root.service_actor_id.clone(),
            actor_authorization_id: actor.actor_authorization_id.clone(),
            actor_authorization_digest: actor.actor_authorization_digest.clone(),
            registered_at: checked_at.into(),
        },
    };
    adapter.adapter_digest = canonical_route_adapter_version_json_and_digest(&adapter)?.1;
    Ok(adapter)
}

#[allow(clippy::too_many_arguments)]
fn credential_envelope(
    provider: ComputeRouteProviderBinding,
    route: ComputeRouteShape,
    credential_ref: &str,
    credential_hint: &str,
    verifier: ComputeRouteCredentialVerifierBinding,
    evidence: &crate::compute_federation::external_pool_adapter_credential_reattestation::ExternalPoolAdapterCredentialReattestationReceipt,
    actor: &ComputeServiceActorAuthorizationEnvelope,
    checked_at: &str,
    expires_at: &str,
    cleanup_expires_at: &str,
) -> Result<ComputeRouteCredentialEnvelope> {
    let mut credential = ComputeRouteCredentialEnvelope {
        schema: COMPUTE_ROUTE_CREDENTIAL_SCHEMA.into(),
        credential_id: new_id("external_pool_adapter_route_credential"),
        credential_revision: 1,
        credential_digest: String::new(),
        canonicalization: COMPUTE_ROUTE_CANONICALIZATION.into(),
        digest_algorithm: COMPUTE_ROUTE_DIGEST_ALGORITHM.into(),
        credential: ComputeRouteCredential {
            provider,
            route,
            non_bearer_credential_ref: credential_ref.into(),
            credential_hint: credential_hint.into(),
            verifier,
            verification_receipt_id: evidence.reattestation_receipt_id.clone(),
            verification_receipt_digest: evidence.reattestation_receipt_digest.clone(),
            verified_by_service_actor_id: actor.authorization.service_actor_id.clone(),
            actor_authorization_id: actor.actor_authorization_id.clone(),
            actor_authorization_digest: actor.actor_authorization_digest.clone(),
            authenticated_at: checked_at.into(),
            expires_at: expires_at.into(),
            cleanup_expires_at: cleanup_expires_at.into(),
            recorded_at: checked_at.into(),
        },
    };
    credential.credential_digest = canonical_route_credential_json_and_digest(&credential)?.1;
    Ok(credential)
}

#[allow(clippy::too_many_arguments)]
fn authorization_envelope(
    provider: ComputeRouteProviderBinding,
    route: ComputeRouteShape,
    stable: &ExternalPoolStableExecutorBinding,
    credential: &ComputeRouteCredentialEnvelope,
    actor: &ComputeServiceActorAuthorizationEnvelope,
    capabilities: Vec<ComputeRouteCapabilityBinding>,
    source_id: &str,
    source_digest: &str,
    approved_by_user_id: &str,
    checked_at: &str,
    expires_at: &str,
    cleanup_expires_at: &str,
) -> Result<ComputeRouteAuthorizationEnvelope> {
    let body = &credential.credential;
    let mut authorization = ComputeRouteAuthorizationEnvelope {
        schema: COMPUTE_ROUTE_AUTHORIZATION_SCHEMA.into(),
        route_authorization_id: new_id("external_pool_adapter_route_authorization"),
        route_authorization_revision: 1,
        route_authorization_digest: String::new(),
        canonicalization: COMPUTE_ROUTE_CANONICALIZATION.into(),
        digest_algorithm: COMPUTE_ROUTE_DIGEST_ALGORITHM.into(),
        authorization: ComputeRouteAuthorization {
            provider,
            executor_id: stable.executor_id.clone(),
            route,
            credential: ComputeRouteCredentialBinding {
                credential_id: credential.credential_id.clone(),
                credential_revision: credential.credential_revision,
                credential_digest: credential.credential_digest.clone(),
                expires_at: expires_at.into(),
                cleanup_expires_at: cleanup_expires_at.into(),
            },
            capabilities,
            source: ComputeRouteAuthorizationSourceBinding {
                source_kind: COMPUTE_ROUTE_SOURCE_EXTERNAL_POOL_ONBOARDING.into(),
                source_id: source_id.into(),
                source_digest: source_digest.into(),
                approved_by_user_id: approved_by_user_id.into(),
            },
            verifier: body.verifier.clone(),
            verification_receipt_id: body.verification_receipt_id.clone(),
            verification_receipt_digest: body.verification_receipt_digest.clone(),
            verified_by_service_actor_id: actor.authorization.service_actor_id.clone(),
            actor_authorization_id: actor.actor_authorization_id.clone(),
            actor_authorization_digest: actor.actor_authorization_digest.clone(),
            authenticated_at: checked_at.into(),
            authorized_at: checked_at.into(),
            expires_at: expires_at.into(),
            cleanup_expires_at: cleanup_expires_at.into(),
            recorded_at: checked_at.into(),
        },
    };
    authorization.route_authorization_digest =
        canonical_route_authorization_json_and_digest(&authorization)?.1;
    Ok(authorization)
}

fn seal_envelope(
    adapter: &ComputeRouteAdapterVersionEnvelope,
    credential: &ComputeRouteCredentialEnvelope,
    authorization: &ComputeRouteAuthorizationEnvelope,
    checked_at: &str,
) -> Result<ComputeRouteAuthorizationSealEnvelope> {
    let capabilities = &authorization.authorization.capabilities;
    let mut seal = ComputeRouteAuthorizationSealEnvelope {
        schema: COMPUTE_ROUTE_AUTHORIZATION_SEAL_SCHEMA.into(),
        seal_id: new_id("external_pool_adapter_route_authorization_seal"),
        seal_digest: String::new(),
        canonicalization: COMPUTE_ROUTE_CANONICALIZATION.into(),
        digest_algorithm: COMPUTE_ROUTE_DIGEST_ALGORITHM.into(),
        route_authorization_id: authorization.route_authorization_id.clone(),
        route_authorization_revision: authorization.route_authorization_revision,
        route_authorization_digest: authorization.route_authorization_digest.clone(),
        adapter_id: adapter.adapter_id.clone(),
        adapter_revision: adapter.adapter_revision,
        adapter_registry_digest: adapter.adapter_digest.clone(),
        credential_id: credential.credential_id.clone(),
        credential_revision: credential.credential_revision,
        credential_digest: credential.credential_digest.clone(),
        capability_count: i64::try_from(capabilities.len())?,
        capability_set_digest: canonical_route_capability_set_digest(capabilities)?,
        sealed_at: checked_at.into(),
    };
    seal.seal_digest = canonical_route_authorization_seal_json_and_digest(&seal)?.1;
    Ok(seal)
}

fn expiries(checked_at: &str, no_work: &str, credential: &str) -> Result<(String, String)> {
    let checked = canonical_time(checked_at)?;
    let expires = canonical_time(no_work)?.min(canonical_time(credential)?);
    let cleanup = checked
        .checked_add_signed(Duration::seconds(1_800))
        .ok_or_else(|| anyhow::anyhow!("V277 route cleanup TTL overflow"))?;
    ensure!(
        checked < expires && expires < cleanup,
        "V277 route fresh evidence TTL is insufficient"
    );
    Ok((canonical(expires), canonical(cleanup)))
}

fn canonical_time(value: &str) -> Result<DateTime<Utc>> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    ensure!(
        parsed.offset().local_minus_utc() == 0
            && parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) == value,
        "V277 route timestamp is not canonical UTC nanoseconds"
    );
    Ok(parsed.with_timezone(&Utc))
}

fn canonical(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Nanos, true)
}
