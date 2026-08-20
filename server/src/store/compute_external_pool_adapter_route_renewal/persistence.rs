use anyhow::{ensure, Result};
use rusqlite::{types::Value, Connection, Transaction};

use crate::{
    compute_federation::{
        external_pool_adapter_route_renewal::{
            canonical_external_pool_adapter_route_renewal_receipt_json_and_digest,
            validate_external_pool_adapter_route_renewal_receipt,
            ExternalPoolAdapterRouteRenewalReceipt,
        },
        route_authority::{
            canonical_route_authorization_json_and_digest,
            canonical_route_authorization_seal_json_and_digest,
            canonical_route_credential_json_and_digest,
            canonical_service_actor_authorization_json_and_digest,
        },
    },
    store::compute_external_pool_adapter_credential_reattestation::CurrentExternalPoolAdapterCredentialReattestationAuthority,
};

use super::{
    builder::build,
    pending::{self, ExternalPoolAdapterRouteRenewalPendingPlan, Kind},
    receipt::{insert_receipt_on, receipt_by_id_on, receipt_values},
    types::{
        CommittedExternalPoolAdapterRouteRenewal, ExternalPoolAdapterRouteRenewalDecision,
        ExternalPoolAdapterRouteRenewalDisposition,
        HistoricalExternalPoolAdapterRouteRecoveryAuthority,
        PendingExternalPoolAdapterRouteRenewalCommit,
    },
    writes::{
        cas_credential_root_on, credential_root_on, insert_actor_on, insert_authorization_on,
        insert_capabilities_and_seal_on, insert_credential_on, CredentialRootState,
    },
};

pub(in crate::store) fn renew_external_pool_adapter_route_on<'tx, 'conn>(
    transaction: &'tx Transaction<'conn>,
    historical: &HistoricalExternalPoolAdapterRouteRecoveryAuthority<'tx, 'conn>,
    credential: &CurrentExternalPoolAdapterCredentialReattestationAuthority,
    decision: &ExternalPoolAdapterRouteRenewalDecision,
    receipt: &ExternalPoolAdapterRouteRenewalReceipt,
) -> Result<PendingExternalPoolAdapterRouteRenewalCommit> {
    validate_external_pool_adapter_route_renewal_receipt(receipt)?;
    if let Some(stored) = receipt_by_id_on(transaction, &receipt.route_renewal_receipt_id)? {
        ensure!(stored == *receipt, "V278 immutable replay conflicts");
        return Ok(PendingExternalPoolAdapterRouteRenewalCommit {
            receipt: stored,
            disposition: ExternalPoolAdapterRouteRenewalDisposition::ExactReplay,
            plan_guard: None,
        });
    }
    let built = build(
        transaction,
        historical,
        credential,
        decision,
        &receipt.renewal.timing.evidence_checked_at,
    )?;
    ensure!(
        built.receipt == *receipt,
        "V278 caller receipt is not the exact tx build"
    );
    let route = &built.route;
    let old = credential_root_on(
        transaction,
        &route.inputs().credential().envelope().credential_id,
    )?;
    validate_root_transition(&old, route.inputs().credential().envelope(), &built.receipt)?;
    let plan = build_plan(&built.receipt, route, &old)?;
    let plan_guard = pending::install(transaction, plan)?;

    insert_actor_on(transaction, route.inputs().actor().envelope())?;
    insert_credential_on(transaction, route.inputs().credential().envelope())?;
    cas_credential_root_on(transaction, &old, route.inputs().credential().envelope())?;
    insert_authorization_on(transaction, route)?;
    insert_capabilities_and_seal_on(transaction, route)?;
    insert_receipt_on(transaction, &built.receipt)?;
    plan_guard.ensure_fully_consumed()?;
    let stored = receipt_by_id_on(transaction, &built.receipt.route_renewal_receipt_id)?
        .ok_or_else(|| anyhow::anyhow!("V278 inserted receipt disappeared before commit"))?;
    ensure!(
        stored == built.receipt,
        "V278 inserted receipt readback drifted"
    );

    Ok(PendingExternalPoolAdapterRouteRenewalCommit {
        receipt: stored,
        disposition: ExternalPoolAdapterRouteRenewalDisposition::Inserted,
        plan_guard: Some(plan_guard),
    })
}

pub(in crate::store) fn finalize_external_pool_adapter_route_renewal_after_commit_on(
    connection: &Connection,
    pending: PendingExternalPoolAdapterRouteRenewalCommit,
) -> Result<CommittedExternalPoolAdapterRouteRenewal> {
    ensure!(
        connection.is_autocommit(),
        "V278 final readback requires a committed autocommit connection"
    );
    let PendingExternalPoolAdapterRouteRenewalCommit {
        receipt,
        disposition,
        plan_guard,
    } = pending;
    if let Some(guard) = plan_guard {
        guard.ensure_same_connection(connection)?;
        guard.ensure_fully_consumed()?;
        let stored = receipt_by_id_on(connection, &receipt.route_renewal_receipt_id)?
            .ok_or_else(|| anyhow::anyhow!("committed V278 receipt is not visible"))?;
        ensure!(stored == receipt, "committed V278 readback drifted");
        guard.discard()?;
    } else {
        let stored = receipt_by_id_on(connection, &receipt.route_renewal_receipt_id)?
            .ok_or_else(|| anyhow::anyhow!("replayed V278 receipt disappeared"))?;
        ensure!(stored == receipt, "replayed V278 readback drifted");
    }
    Ok(CommittedExternalPoolAdapterRouteRenewal::new(
        receipt,
        disposition,
    ))
}

fn validate_root_transition(
    old: &CredentialRootState,
    new: &crate::compute_federation::route_authority::ComputeRouteCredentialEnvelope,
    receipt: &ExternalPoolAdapterRouteRenewalReceipt,
) -> Result<()> {
    let predecessor = &receipt.renewal.predecessor_route;
    ensure!(
        old.credential_id == new.credential_id
            && old.credential_id == predecessor.route_credential_id
            && old.revision == predecessor.route_credential_revision
            && old.digest == predecessor.route_credential_digest
            && old.status == "active"
            && new.credential_revision == old.revision.checked_add(1).unwrap_or(0)
            && old.digest != new.credential_digest
            && old.updated_at < new.credential.recorded_at,
        "V278 credential-root transition is not adjacent and exact"
    );
    Ok(())
}

fn build_plan(
    receipt: &ExternalPoolAdapterRouteRenewalReceipt,
    route: &crate::compute_federation::route_authority::AuthorizedComputeRouteAuthorization,
    old: &CredentialRootState,
) -> Result<ExternalPoolAdapterRouteRenewalPendingPlan> {
    let actor = route.inputs().actor().envelope();
    let credential = route.inputs().credential().envelope();
    let authorization = route.envelope();
    let seal = route.seal();
    let (actor_json, actor_digest) = canonical_service_actor_authorization_json_and_digest(actor)?;
    let (credential_json, credential_digest) =
        canonical_route_credential_json_and_digest(credential)?;
    let (authorization_json, authorization_digest) =
        canonical_route_authorization_json_and_digest(authorization)?;
    let (seal_json, seal_digest) = canonical_route_authorization_seal_json_and_digest(seal)?;
    let (_, receipt_digest) =
        canonical_external_pool_adapter_route_renewal_receipt_json_and_digest(receipt)?;
    ensure!(
        actor_digest == actor.actor_authorization_digest
            && credential_digest == credential.credential_digest
            && authorization_digest == authorization.route_authorization_digest
            && seal_digest == seal.seal_digest
            && receipt_digest == receipt.route_renewal_receipt_digest,
        "V278 plan canonical roots mismatch"
    );
    let mut writes = vec![
        (
            Kind::ServiceActor,
            vec![
                text(&actor.actor_authorization_id),
                text(&actor_digest),
                Value::Text(actor_json),
            ],
        ),
        (
            Kind::CredentialVersion,
            vec![
                text(&credential.credential_id),
                Value::Integer(credential.credential_revision),
                text(&credential_digest),
                Value::Text(credential_json),
            ],
        ),
        (
            Kind::CredentialRoot,
            vec![
                text(&old.credential_id),
                Value::Integer(old.revision),
                text(&old.digest),
                text(&old.status),
                text(&old.updated_at),
                text(&credential.credential_id),
                Value::Integer(credential.credential_revision),
                text(&credential_digest),
                text(&old.status),
                text(&credential.credential.recorded_at),
            ],
        ),
        (
            Kind::Authorization,
            vec![
                text(&authorization.route_authorization_id),
                Value::Integer(authorization.route_authorization_revision),
                text(&authorization_digest),
                Value::Text(authorization_json),
            ],
        ),
    ];
    writes.extend(
        authorization
            .authorization
            .capabilities
            .iter()
            .map(|capability| {
                (
                    Kind::Capability,
                    vec![
                        text(&authorization.route_authorization_id),
                        Value::Integer(capability.ordinal),
                        text(&capability.capability_id),
                        Value::Integer(capability.capability_revision),
                    ],
                )
            }),
    );
    writes.extend([
        (
            Kind::Seal,
            vec![
                text(&seal.seal_id),
                text(&seal_digest),
                Value::Text(seal_json),
            ],
        ),
        (Kind::Receipt, receipt_values(receipt)?),
    ]);
    ExternalPoolAdapterRouteRenewalPendingPlan::new(writes)
}

fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}
