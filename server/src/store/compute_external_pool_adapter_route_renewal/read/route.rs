use anyhow::Result;
use rusqlite::{named_params, Transaction};

use crate::compute_federation::{
    external_pool_adapter_route_renewal::ExternalPoolAdapterRouteRenewalReceipt,
    route_authority::{
        validated_compute_route_authorization_from_canonical_envelopes,
        AuthorizedComputeRouteAuthorization,
    },
};

pub(in crate::store) fn sealed_route_for_receipt_on(
    transaction: &Transaction<'_>,
    receipt: &ExternalPoolAdapterRouteRenewalReceipt,
) -> Result<AuthorizedComputeRouteAuthorization> {
    let r = &receipt.renewal;
    sealed_route_on(
        transaction,
        &r.stable_binding.route_adapter_projection_id,
        r.stable_binding.route_adapter_revision,
        &r.stable_binding.route_adapter_digest,
        &r.renewed_route.service_actor_authorization_id,
        &r.renewed_route.service_actor_authorization_digest,
        &r.renewed_route.route_credential_id,
        r.renewed_route.route_credential_revision,
        &r.renewed_route.route_credential_digest,
        &r.renewed_route.route_authorization_id,
        &r.renewed_route.route_authorization_digest,
        &r.renewed_route.route_seal_id,
        &r.renewed_route.route_seal_digest,
    )
}

#[allow(clippy::too_many_arguments)]
pub(in crate::store) fn sealed_route_on(
    transaction: &Transaction<'_>,
    adapter_id: &str,
    adapter_revision: i64,
    adapter_digest: &str,
    actor_id: &str,
    actor_digest: &str,
    credential_id: &str,
    credential_revision: i64,
    credential_digest: &str,
    authorization_id: &str,
    authorization_digest: &str,
    seal_id: &str,
    seal_digest: &str,
) -> Result<AuthorizedComputeRouteAuthorization> {
    let json = transaction.query_row(
        "SELECT adapter.adapter_json,credential.credential_json,actor.actor_authorization_json,
                authorization.route_authorization_json,seal.seal_json
           FROM compute_route_adapter_versions adapter
           JOIN compute_route_credential_versions credential
             ON credential.credential_id=:credential_id
            AND credential.credential_revision=:credential_revision
           JOIN compute_service_actor_authorizations actor
             ON actor.actor_authorization_id=:actor_id
           JOIN compute_route_authorization_receipts authorization
             ON authorization.route_authorization_id=:authorization_id
           JOIN compute_route_authorization_seals seal ON seal.seal_id=:seal_id
          WHERE adapter.adapter_id=:adapter_id AND adapter.adapter_revision=:adapter_revision
            AND adapter.adapter_digest=:adapter_digest
            AND credential.credential_digest=:credential_digest
            AND actor.actor_authorization_digest=:actor_digest
            AND authorization.route_authorization_digest=:authorization_digest
            AND seal.seal_digest=:seal_digest",
        named_params! {
            ":adapter_id": adapter_id, ":adapter_revision": adapter_revision,
            ":adapter_digest": adapter_digest, ":actor_id": actor_id,
            ":actor_digest": actor_digest, ":credential_id": credential_id,
            ":credential_revision": credential_revision, ":credential_digest": credential_digest,
            ":authorization_id": authorization_id, ":authorization_digest": authorization_digest,
            ":seal_id": seal_id, ":seal_digest": seal_digest,
        },
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        },
    )?;
    validated_compute_route_authorization_from_canonical_envelopes(
        parse(&json.0)?,
        parse(&json.1)?,
        parse(&json.2)?,
        parse(&json.3)?,
        parse(&json.4)?,
    )
}

pub(in crate::store) fn route_leaf_is_current_on(
    transaction: &Transaction<'_>,
    receipt: &ExternalPoolAdapterRouteRenewalReceipt,
    checked_at: &str,
    usable_through: &str,
) -> Result<bool> {
    let r = &receipt.renewal;
    transaction
        .query_row(
            "SELECT EXISTS(
                 SELECT 1 FROM compute_route_credentials root
                 JOIN compute_route_credential_versions credential
                   ON credential.credential_id=root.credential_id
                  AND credential.credential_revision=root.current_credential_revision
                 JOIN compute_service_actor_authorizations actor
                   ON actor.actor_authorization_id=:actor_id
                 JOIN compute_route_authorization_receipts authorization
                   ON authorization.route_authorization_id=:authorization_id
                 JOIN compute_route_authorization_seals seal ON seal.seal_id=:seal_id
                 JOIN compute_providers provider ON provider.provider_id=:provider_id
                 JOIN compute_external_pool_adapter_credential_reattestation_receipts evidence
                   ON evidence.reattestation_receipt_id=:evidence_id
                  AND evidence.reattestation_receipt_digest=:evidence_digest
                WHERE root.credential_id=:credential_id AND root.status='active'
                  AND provider.status='active'
                  AND provider.current_policy_revision=:provider_revision
                  AND provider.current_provider_digest=:provider_digest
                  AND root.current_credential_revision=:credential_revision
                  AND root.current_credential_digest=:credential_digest
                  AND credential.credential_digest=:credential_digest
                  AND credential.verification_receipt_id=:evidence_id
                  AND credential.verification_receipt_digest=:evidence_digest
                  AND actor.actor_authorization_digest=:actor_digest
                  AND authorization.route_authorization_digest=:authorization_digest
                  AND seal.seal_digest=:seal_digest
                  AND evidence.provider_binding_id=:binding_id
                  AND evidence.provider_binding_digest=:binding_digest
                  AND evidence.provider_id=:provider_id
                  AND evidence.observed_provider_policy_revision=:provider_revision
                  AND evidence.observed_provider_digest=:provider_digest
                  AND evidence.observed_provider_status='active'
                  AND evidence.route_adapter_projection_id=:adapter_id
                  AND :checked_at>=actor.issued_at AND :checked_at>=credential.authenticated_at
                  AND :checked_at>=authorization.authorized_at AND :checked_at>=authorization.recorded_at
                  AND :usable_through<actor.valid_until
                  AND :usable_through<credential.expires_at
                  AND :usable_through<authorization.expires_at
                  AND :usable_through<authorization.credential_expires_at
                  AND :usable_through<evidence.report_expires_at
                  AND :usable_through<:receipt_expires_at
                  AND NOT EXISTS(SELECT 1 FROM compute_route_credential_revocations revoked
                                  WHERE revoked.credential_id=root.credential_id
                                    AND revoked.credential_revision=root.current_credential_revision)
                  AND NOT EXISTS(SELECT 1 FROM compute_external_pool_adapter_credential_reattestation_revocations revoked
                                  WHERE revoked.reattestation_receipt_id=evidence.reattestation_receipt_id)
                  AND NOT EXISTS(SELECT 1 FROM compute_external_pool_adapter_credential_reattestation_receipts successor
                                  WHERE successor.predecessor_receipt_id=evidence.reattestation_receipt_id))",
            named_params! {
                ":actor_id": r.renewed_route.service_actor_authorization_id,
                ":actor_digest": r.renewed_route.service_actor_authorization_digest,
                ":credential_id": r.renewed_route.route_credential_id,
                ":credential_revision": r.renewed_route.route_credential_revision,
                ":credential_digest": r.renewed_route.route_credential_digest,
                ":authorization_id": r.renewed_route.route_authorization_id,
                ":authorization_digest": r.renewed_route.route_authorization_digest,
                ":seal_id": r.renewed_route.route_seal_id,
                ":seal_digest": r.renewed_route.route_seal_digest,
                ":provider_id": r.active_subject.active_provider_id,
                ":provider_revision": r.active_subject.active_provider_policy_revision,
                ":provider_digest": r.active_subject.active_provider_digest,
                ":binding_id": r.identity.provider_binding_id,
                ":binding_digest": r.identity.provider_binding_digest,
                ":adapter_id": r.stable_binding.route_adapter_projection_id,
                ":evidence_id": r.credential_evidence.credential_reattestation_receipt_id,
                ":evidence_digest": r.credential_evidence.credential_reattestation_receipt_digest,
                ":checked_at": checked_at,
                ":usable_through": usable_through,
                ":receipt_expires_at": r.timing.expires_at,
            },
            |row| row.get(0),
        )
        .map_err(Into::into)
}

pub(in crate::store) fn effective_expires_at(
    route: &AuthorizedComputeRouteAuthorization,
    receipt: &ExternalPoolAdapterRouteRenewalReceipt,
) -> String {
    [
        route.inputs().actor().authorization().valid_until.as_str(),
        route.inputs().credential().credential().expires_at.as_str(),
        route.envelope().authorization.expires_at.as_str(),
        receipt.renewal.timing.expires_at.as_str(),
    ]
    .into_iter()
    .min()
    .expect("fixed non-empty expiry set")
    .to_owned()
}

fn parse<T: serde::de::DeserializeOwned>(json: &str) -> Result<T> {
    Ok(serde_json::from_str(json)?)
}
