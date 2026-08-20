use anyhow::{bail, ensure, Result};
use rusqlite::{params_from_iter, types::Value, Connection, OptionalExtension};

use crate::compute_federation::external_pool_adapter_route_renewal::{
    canonical_external_pool_adapter_route_renewal_capabilities_json,
    canonical_external_pool_adapter_route_renewal_receipt_json_and_digest,
    validate_external_pool_adapter_route_renewal_receipt, ExternalPoolAdapterRouteRenewalReceipt,
};

pub(crate) const RECEIPT_COLUMNS: &str = "route_renewal_receipt_id,route_renewal_receipt_schema,route_renewal_receipt_digest,route_renewal_receipt_json,canonicalization,digest_algorithm,provider_binding_id,provider_binding_digest,activation_root_digest,renewal_sequence,predecessor_route_renewal_receipt_id,predecessor_route_renewal_receipt_digest,activation_receipt_id,activation_receipt_digest,active_provider_id,active_provider_policy_revision,active_provider_digest,executor_id,stable_executor_binding_digest,projected_v211_adapter_binding_digest,route_adapter_projection_id,route_adapter_revision,route_adapter_digest,predecessor_service_actor_authorization_id,predecessor_service_actor_authorization_digest,predecessor_route_credential_id,predecessor_route_credential_revision,predecessor_route_credential_digest,predecessor_route_authorization_id,predecessor_route_authorization_revision,predecessor_route_authorization_digest,predecessor_route_seal_id,predecessor_route_seal_digest,activation_genesis_successor_receipt_id,activation_genesis_successor_receipt_digest,credential_reattestation_receipt_id,credential_reattestation_receipt_digest,service_actor_id,service_actor_authorization_id,service_actor_authorization_revision,service_actor_authorization_digest,route_credential_id,route_credential_revision,route_credential_digest,route_authorization_id,route_authorization_revision,route_authorization_digest,route_capabilities_json,route_capability_count,route_capability_set_digest,route_capability_0_id,route_capability_0_revision,route_capability_1_id,route_capability_1_revision,route_capability_2_id,route_capability_2_revision,route_capability_3_id,route_capability_3_revision,route_capability_4_id,route_capability_4_revision,route_capability_5_id,route_capability_5_revision,route_seal_id,route_seal_digest,authenticated_at,authorized_at,expires_at,cleanup_expires_at,evidence_checked_at,created_at,delegation_id,delegation_digest,renewal_policy_digest,renewed_by_actor_kind,renewed_by_service_actor_id,idempotency_material_json,idempotency_digest";

pub(super) fn receipt_by_id_on(
    connection: &Connection,
    receipt_id: &str,
) -> Result<Option<ExternalPoolAdapterRouteRenewalReceipt>> {
    connection
        .query_row(
            &format!(
                "SELECT {RECEIPT_COLUMNS} FROM compute_external_pool_adapter_route_renewal_receipts WHERE route_renewal_receipt_id=?1"
            ),
            [receipt_id],
            |row| {
                let json: String = row.get(3)?;
                let receipt = serde_json::from_str::<ExternalPoolAdapterRouteRenewalReceipt>(&json)
                    .map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            3,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?;
                let stored = (0..77)
                    .map(|index| row.get(index))
                    .collect::<rusqlite::Result<Vec<Value>>>()?;
                Ok((receipt, stored))
            },
        )
        .optional()?
        .map(|(receipt, stored)| audit_receipt(receipt, stored))
        .transpose()
}

pub(super) fn insert_receipt_on(
    connection: &Connection,
    receipt: &ExternalPoolAdapterRouteRenewalReceipt,
) -> Result<()> {
    let values = receipt_values(receipt)?;
    let placeholders = (1..=77)
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(",");
    connection.execute(
        &format!(
            "INSERT INTO compute_external_pool_adapter_route_renewal_receipts ({RECEIPT_COLUMNS}) VALUES ({placeholders})"
        ),
        params_from_iter(values.iter()),
    )?;
    Ok(())
}

pub(super) fn receipt_values(
    receipt: &ExternalPoolAdapterRouteRenewalReceipt,
) -> Result<Vec<Value>> {
    validate_external_pool_adapter_route_renewal_receipt(receipt)?;
    let (json, digest) =
        canonical_external_pool_adapter_route_renewal_receipt_json_and_digest(receipt)?;
    ensure!(
        digest == receipt.route_renewal_receipt_digest,
        "V278 receipt digest is not exact"
    );
    receipt_values_with_json(receipt, json)
}

fn audit_receipt(
    receipt: ExternalPoolAdapterRouteRenewalReceipt,
    stored: Vec<Value>,
) -> Result<ExternalPoolAdapterRouteRenewalReceipt> {
    let (json, digest) =
        canonical_external_pool_adapter_route_renewal_receipt_json_and_digest(&receipt)?;
    if digest != receipt.route_renewal_receipt_digest
        || receipt_values_with_json(&receipt, json)? != stored
    {
        bail!("V278 receipt scalar/canonical readback is not exact")
    }
    validate_external_pool_adapter_route_renewal_receipt(&receipt)?;
    Ok(receipt)
}

fn receipt_values_with_json(
    receipt: &ExternalPoolAdapterRouteRenewalReceipt,
    json: String,
) -> Result<Vec<Value>> {
    let r = &receipt.renewal;
    let id = &r.identity;
    let witness = &r.activation_witness;
    let active = &r.active_subject;
    let stable = &r.stable_binding;
    let predecessor = &r.predecessor_route;
    let evidence = &r.credential_evidence;
    let route = &r.renewed_route;
    ensure!(
        route.route_capabilities.len() == 6,
        "V278 route is not six-wide"
    );
    let capabilities_json =
        canonical_external_pool_adapter_route_renewal_capabilities_json(&route.route_capabilities)?;
    let timing = &r.timing;
    let audit = &r.audit;
    let mut values = vec![
        text(&receipt.route_renewal_receipt_id),
        text(&receipt.schema),
        text(&receipt.route_renewal_receipt_digest),
        Value::Text(json),
        text(&receipt.canonicalization),
        text(&receipt.digest_algorithm),
        text(&id.provider_binding_id),
        text(&id.provider_binding_digest),
        text(&id.activation_root_digest),
        integer(id.renewal_sequence),
        optional(id.predecessor_route_renewal_receipt_id.as_deref()),
        optional(id.predecessor_route_renewal_receipt_digest.as_deref()),
        text(&witness.activation_receipt_id),
        text(&witness.activation_receipt_digest),
        text(&active.active_provider_id),
        integer(active.active_provider_policy_revision),
        text(&active.active_provider_digest),
        text(&stable.executor_id),
        text(&stable.stable_executor_binding_digest),
        text(&stable.projected_v211_adapter_binding_digest),
        text(&stable.route_adapter_projection_id),
        integer(stable.route_adapter_revision),
        text(&stable.route_adapter_digest),
        text(&predecessor.service_actor_authorization_id),
        text(&predecessor.service_actor_authorization_digest),
        text(&predecessor.route_credential_id),
        integer(predecessor.route_credential_revision),
        text(&predecessor.route_credential_digest),
        text(&predecessor.route_authorization_id),
        integer(predecessor.route_authorization_revision),
        text(&predecessor.route_authorization_digest),
        text(&predecessor.route_seal_id),
        text(&predecessor.route_seal_digest),
        text(&witness.activation_genesis_successor_receipt_id),
        text(&witness.activation_genesis_successor_receipt_digest),
        text(&evidence.credential_reattestation_receipt_id),
        text(&evidence.credential_reattestation_receipt_digest),
        text(&route.service_actor_id),
        text(&route.service_actor_authorization_id),
        integer(route.service_actor_authorization_revision),
        text(&route.service_actor_authorization_digest),
        text(&route.route_credential_id),
        integer(route.route_credential_revision),
        text(&route.route_credential_digest),
        text(&route.route_authorization_id),
        integer(route.route_authorization_revision),
        text(&route.route_authorization_digest),
        Value::Text(capabilities_json),
        integer(6),
        text(&route.route_capability_set_digest),
    ];
    for capability in &route.route_capabilities {
        values.push(text(&capability.capability_id));
        values.push(integer(capability.capability_revision));
    }
    values.extend([
        text(&route.route_seal_id),
        text(&route.route_seal_digest),
        text(&timing.authenticated_at),
        text(&timing.authorized_at),
        text(&timing.expires_at),
        text(&timing.cleanup_expires_at),
        text(&timing.evidence_checked_at),
        text(&timing.created_at),
        text(&audit.delegation_id),
        text(&audit.delegation_digest),
        text(&audit.renewal_policy_digest),
        text(&audit.renewed_by_actor_kind),
        text(&audit.renewed_by_service_actor_id),
        text(&audit.idempotency_material_json),
        text(&audit.idempotency_digest),
    ]);
    ensure!(values.len() == 77, "V278 receipt projection is not 77-wide");
    Ok(values)
}

fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}
fn optional(value: Option<&str>) -> Value {
    value.map_or(Value::Null, text)
}
fn integer(value: i64) -> Value {
    Value::Integer(value)
}
