use anyhow::{ensure, Result};
use chrono::{DateTime, Duration, SecondsFormat};
use rusqlite::Transaction;

use crate::{
    compute_federation::{
        external_pool_adapter_route_renewal::*,
        provider::{PROVIDER_KIND_EXTERNAL_POOL, PROVIDER_STATUS_ACTIVE},
        route_authority::*,
    },
    store::{
        compute_external_pool_adapter_credential_reattestation::CurrentExternalPoolAdapterCredentialReattestationAuthority,
        compute_provider_registry::current_registered_provider_on,
    },
};

mod material;

use material::build_receipt;

use super::{
    read::{
        external_pool_adapter_route_renewal_decision_on, sealed_route_for_receipt_on,
        sealed_route_on,
    },
    receipt::receipt_by_id_on,
    types::{
        BuiltExternalPoolAdapterRouteRenewal, ExternalPoolAdapterRouteRenewalDecision,
        HistoricalExternalPoolAdapterRouteRecoveryAuthority,
    },
};

pub(in crate::store) fn build_external_pool_adapter_route_renewal_receipt<'tx, 'conn>(
    transaction: &'tx Transaction<'conn>,
    historical: &HistoricalExternalPoolAdapterRouteRecoveryAuthority<'tx, 'conn>,
    credential: &CurrentExternalPoolAdapterCredentialReattestationAuthority,
    decision: &ExternalPoolAdapterRouteRenewalDecision,
    evidence_checked_at: &str,
) -> Result<ExternalPoolAdapterRouteRenewalReceipt> {
    Ok(build(
        transaction,
        historical,
        credential,
        decision,
        evidence_checked_at,
    )?
    .receipt)
}

pub(super) fn build<'tx, 'conn>(
    transaction: &'tx Transaction<'conn>,
    historical: &HistoricalExternalPoolAdapterRouteRecoveryAuthority<'tx, 'conn>,
    credential: &CurrentExternalPoolAdapterCredentialReattestationAuthority,
    decision: &ExternalPoolAdapterRouteRenewalDecision,
    evidence_checked_at: &str,
) -> Result<BuiltExternalPoolAdapterRouteRenewal> {
    ensure!(
        credential.checked_at() == evidence_checked_at
            && historical.checked_at() == evidence_checked_at,
        "V278 typed authorities do not share checked_at"
    );
    let activation = historical.activation();
    let activation_receipt = activation.receipt();
    let root = &activation.activation_root().activation_root;
    let observed_decision = external_pool_adapter_route_renewal_decision_on(
        transaction,
        &root.provider_binding_id,
        &activation_receipt.activation_receipt_id,
        &activation_receipt.activation_receipt_digest,
        evidence_checked_at,
    )?;
    ensure!(
        &observed_decision == decision,
        "V278 renewal decision was not Store-derived"
    );
    let live = current_registered_provider_on(transaction, &root.provider_id)?
        .ok_or_else(|| anyhow::anyhow!("V278 active Provider disappeared"))?;
    ensure!(
        live.provider.provider_kind == PROVIDER_KIND_EXTERNAL_POOL
            && live.provider.status == PROVIDER_STATUS_ACTIVE
            && live.provider.owner_account_id == root.provider_owner_account_id,
        "V278 live Provider is not the activation-rooted active subject"
    );
    let evidence = credential.receipt();
    let binding = &evidence.reattestation.binding;
    ensure!(
        binding.provider_binding_id == root.provider_binding_id
            && binding.provider_binding_digest == root.provider_binding_digest
            && binding.route_adapter_projection_id == root.route_adapter_projection_id
            && binding.provider_id == live.provider.provider_id
            && binding.provider_kind == live.provider.provider_kind
            && binding.provider_owner_account_id == live.provider.owner_account_id
            && binding.observed_provider_policy_revision == live.provider.policy_revision
            && binding.observed_provider_digest == live.provider_digest
            && binding.observed_provider_status == live.provider.status,
        "V278 current V253 does not attest the live active Provider"
    );
    let (sequence, predecessor_id, predecessor_digest, predecessor_route) = match decision {
        ExternalPoolAdapterRouteRenewalDecision::Current { .. } => {
            anyhow::bail!("V278 builder only accepts RenewalRequired")
        }
        ExternalPoolAdapterRouteRenewalDecision::RenewalRequired {
            predecessor_route_renewal_receipt_id: None,
            predecessor_route_renewal_receipt_digest: None,
        } => {
            let closure = activation.route_closure();
            let route = sealed_route_on(
                transaction,
                &closure.route_adapter_projection_id,
                closure.route_adapter_revision,
                &closure.route_adapter_digest,
                &closure.service_actor_authorization_id,
                &closure.service_actor_authorization_digest,
                &closure.route_credential_id,
                closure.route_credential_revision,
                &closure.route_credential_digest,
                &closure.route_authorization_id,
                &closure.route_authorization_digest,
                &closure.route_seal_id,
                &closure.route_seal_digest,
            )?;
            (1, None, None, route)
        }
        ExternalPoolAdapterRouteRenewalDecision::RenewalRequired {
            predecessor_route_renewal_receipt_id: Some(id),
            predecessor_route_renewal_receipt_digest: Some(digest),
        } => {
            let predecessor = receipt_by_id_on(transaction, id)?
                .ok_or_else(|| anyhow::anyhow!("V278 predecessor disappeared"))?;
            ensure!(
                predecessor.route_renewal_receipt_digest == *digest
                    && predecessor.renewal.identity.provider_binding_id == root.provider_binding_id
                    && predecessor.renewal.identity.activation_root_digest
                        == activation_receipt
                            .activation
                            .identity
                            .activation_root_digest,
                "V278 predecessor identity is not exact"
            );
            let sequence = predecessor
                .renewal
                .identity
                .renewal_sequence
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("V278 sequence exhausted"))?;
            let route = sealed_route_for_receipt_on(transaction, &predecessor)?;
            (sequence, Some(id.clone()), Some(digest.clone()), route)
        }
        ExternalPoolAdapterRouteRenewalDecision::RenewalRequired { .. } => {
            anyhow::bail!("V278 predecessor pair is partial")
        }
    };
    ensure!(
        predecessor_route.envelope().authorization.executor_id
            == activation_receipt.activation.stable_executor.executor_id
            && predecessor_route
                .envelope()
                .authorization
                .route
                .adapter_binding_digest
                == activation_receipt
                    .activation
                    .projected_v211_binding
                    .projected_v211_adapter_binding_digest,
        "V278 predecessor stable executor/binding drifted"
    );
    let adapter_verifier = &predecessor_route
        .inputs()
        .adapter()
        .adapter()
        .credential_verifier;
    let expected_verifier = &binding.expected_credential_verifier;
    ensure!(
        adapter_verifier.verification_kind == expected_verifier.verification_kind
            && adapter_verifier.verifier_id == expected_verifier.verifier_id
            && adapter_verifier.verifier_revision == expected_verifier.verifier_revision
            && adapter_verifier.verifier_digest == expected_verifier.verifier_digest,
        "V278 current V253 verifier does not match the stable Adapter"
    );
    build_from_predecessor(
        activation_receipt,
        activation.genesis(),
        &live.provider,
        &live.provider_digest,
        root,
        evidence,
        sequence,
        predecessor_id,
        predecessor_digest,
        predecessor_route,
        evidence_checked_at,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_from_predecessor(
    activation: &crate::compute_federation::external_pool_adapter_atomic_activation::ExternalPoolAdapterAtomicActivationReceipt,
    genesis: &crate::compute_federation::external_pool_adapter_provider_active_successor::ExternalPoolAdapterProviderActiveSuccessorReceipt,
    active_provider: &crate::compute_federation::provider::ComputeProvider,
    active_provider_digest: &str,
    root: &crate::compute_federation::external_pool_adapter_provider_active_successor::ExternalPoolAdapterProviderActiveSuccessorActivationRootEnvelope,
    credential_evidence: &crate::compute_federation::external_pool_adapter_credential_reattestation::ExternalPoolAdapterCredentialReattestationReceipt,
    sequence: i64,
    predecessor_id: Option<String>,
    predecessor_digest: Option<String>,
    predecessor: AuthorizedComputeRouteAuthorization,
    checked_at: &str,
) -> Result<BuiltExternalPoolAdapterRouteRenewal> {
    let checked = canonical_timestamp(checked_at)?;
    let report_expires =
        canonical_timestamp(&credential_evidence.reattestation.binding.report_expires_at)?;
    let fresh_max = checked
        .checked_add_signed(Duration::seconds(ROUTE_RENEWAL_FRESH_MAX_SECONDS))
        .ok_or_else(|| anyhow::anyhow!("V278 fresh TTL overflow"))?;
    let cleanup = checked
        .checked_add_signed(Duration::seconds(ROUTE_RENEWAL_CLEANUP_MAX_SECONDS))
        .ok_or_else(|| anyhow::anyhow!("V278 cleanup TTL overflow"))?;
    let renew_at = checked
        .checked_add_signed(Duration::seconds(ROUTE_RENEWAL_RENEW_BEFORE_SECONDS))
        .ok_or_else(|| anyhow::anyhow!("V278 renew-before overflow"))?;
    let expires = std::cmp::min(fresh_max, report_expires);
    ensure!(
        renew_at < expires && expires < cleanup,
        "V278 evidence TTL is insufficient"
    );
    let expires_at = canonical(expires);
    let cleanup_expires_at = canonical(cleanup);
    let idempotency = ExternalPoolAdapterRouteRenewalIdempotencyMaterial {
        provider_binding_id: root.provider_binding_id.clone(),
        activation_receipt_id: activation.activation_receipt_id.clone(),
        activation_root_digest: activation
            .activation
            .identity
            .activation_root_digest
            .clone(),
        renewal_sequence: sequence,
        predecessor_route_renewal_receipt_id: predecessor_id.clone(),
        predecessor_route_renewal_receipt_digest: predecessor_digest.clone(),
        credential_reattestation_receipt_id: credential_evidence.reattestation_receipt_id.clone(),
        credential_reattestation_receipt_digest: credential_evidence
            .reattestation_receipt_digest
            .clone(),
        evidence_checked_at: checked_at.to_owned(),
    };
    let (idempotency_json, idempotency_digest) =
        canonical_external_pool_adapter_route_renewal_idempotency_json_and_digest(&idempotency)?;
    let receipt_id = derive_external_pool_adapter_route_renewal_receipt_id(&idempotency_digest)?;
    let actor_id = derive_external_pool_adapter_route_renewal_leaf_id(&receipt_id, "actor")?;
    let authorization_id =
        derive_external_pool_adapter_route_renewal_leaf_id(&receipt_id, "authorization")?;
    let seal_id = derive_external_pool_adapter_route_renewal_leaf_id(&receipt_id, "seal")?;

    let old_actor = predecessor.inputs().actor().envelope();
    let mut actor = old_actor.clone();
    actor.actor_authorization_id = actor_id;
    actor.actor_authorization_revision = old_actor
        .actor_authorization_revision
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("V278 actor revision exhausted"))?;
    actor.actor_authorization_digest.clear();
    actor.authorization.issued_at = checked_at.to_owned();
    actor.authorization.recorded_at = checked_at.to_owned();
    actor.authorization.valid_until = cleanup_expires_at.clone();
    actor.actor_authorization_digest =
        canonical_service_actor_authorization_json_and_digest(&actor)?.1;

    let v253 = &credential_evidence.reattestation.binding;
    let old_credential = predecessor.inputs().credential().envelope();
    let mut route_credential = old_credential.clone();
    route_credential.credential_revision = old_credential
        .credential_revision
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("V278 credential revision exhausted"))?;
    route_credential.credential_digest.clear();
    let renewed_credential = &mut route_credential.credential;
    renewed_credential.verifier = ComputeRouteCredentialVerifierBinding {
        verification_kind: v253.expected_credential_verifier.verification_kind.clone(),
        verifier_id: v253.expected_credential_verifier.verifier_id.clone(),
        verifier_revision: v253.expected_credential_verifier.verifier_revision,
        verifier_digest: v253.expected_credential_verifier.verifier_digest.clone(),
    };
    renewed_credential.verification_receipt_id =
        credential_evidence.reattestation_receipt_id.clone();
    renewed_credential.verification_receipt_digest =
        credential_evidence.reattestation_receipt_digest.clone();
    renewed_credential.verified_by_service_actor_id = actor.authorization.service_actor_id.clone();
    renewed_credential.actor_authorization_id = actor.actor_authorization_id.clone();
    renewed_credential.actor_authorization_digest = actor.actor_authorization_digest.clone();
    renewed_credential.authenticated_at = checked_at.to_owned();
    renewed_credential.recorded_at = checked_at.to_owned();
    renewed_credential.expires_at = expires_at.clone();
    renewed_credential.cleanup_expires_at = cleanup_expires_at.clone();
    route_credential.credential_digest =
        canonical_route_credential_json_and_digest(&route_credential)?.1;

    let old_authorization = predecessor.envelope();
    let mut authorization = old_authorization.clone();
    authorization.route_authorization_id = authorization_id;
    authorization.route_authorization_revision = old_authorization
        .route_authorization_revision
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("V278 authorization revision exhausted"))?;
    authorization.route_authorization_digest.clear();
    let renewed = &mut authorization.authorization;
    renewed.credential = ComputeRouteCredentialBinding {
        credential_id: route_credential.credential_id.clone(),
        credential_revision: route_credential.credential_revision,
        credential_digest: route_credential.credential_digest.clone(),
        expires_at: expires_at.clone(),
        cleanup_expires_at: cleanup_expires_at.clone(),
    };
    renewed.verifier = renewed_credential.verifier.clone();
    renewed.verification_receipt_id = renewed_credential.verification_receipt_id.clone();
    renewed.verification_receipt_digest = renewed_credential.verification_receipt_digest.clone();
    renewed.verified_by_service_actor_id = actor.authorization.service_actor_id.clone();
    renewed.actor_authorization_id = actor.actor_authorization_id.clone();
    renewed.actor_authorization_digest = actor.actor_authorization_digest.clone();
    renewed.authenticated_at = checked_at.to_owned();
    renewed.authorized_at = checked_at.to_owned();
    renewed.recorded_at = checked_at.to_owned();
    renewed.expires_at = expires_at.clone();
    renewed.cleanup_expires_at = cleanup_expires_at.clone();
    authorization.route_authorization_digest =
        canonical_route_authorization_json_and_digest(&authorization)?.1;

    let old_seal = predecessor.seal();
    let mut seal = old_seal.clone();
    seal.seal_id = seal_id;
    seal.seal_digest.clear();
    seal.route_authorization_id = authorization.route_authorization_id.clone();
    seal.route_authorization_revision = authorization.route_authorization_revision;
    seal.route_authorization_digest = authorization.route_authorization_digest.clone();
    seal.credential_id = route_credential.credential_id.clone();
    seal.credential_revision = route_credential.credential_revision;
    seal.credential_digest = route_credential.credential_digest.clone();
    seal.sealed_at = checked_at.to_owned();
    seal.seal_digest = canonical_route_authorization_seal_json_and_digest(&seal)?.1;

    let route = validated_compute_route_authorization_from_canonical_envelopes(
        predecessor.inputs().adapter().envelope().clone(),
        route_credential,
        actor,
        authorization,
        seal,
    )?;
    let receipt = build_receipt(
        activation,
        genesis,
        active_provider,
        active_provider_digest,
        root,
        credential_evidence,
        sequence,
        predecessor_id,
        predecessor_digest,
        &predecessor,
        &route,
        checked_at,
        expires_at,
        cleanup_expires_at,
        idempotency_json,
        idempotency_digest,
        receipt_id,
    )?;
    Ok(BuiltExternalPoolAdapterRouteRenewal { receipt, route })
}

#[allow(clippy::too_many_arguments)]
fn canonical_timestamp(value: &str) -> Result<DateTime<chrono::FixedOffset>> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    ensure!(
        parsed.offset().local_minus_utc() == 0 && canonical(parsed) == value,
        "V278 timestamp is not canonical"
    );
    Ok(parsed)
}

fn canonical(value: DateTime<chrono::FixedOffset>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Nanos, true)
}
