use anyhow::{ensure, Result};
use rusqlite::types::Value;

use crate::{
    compute_federation::{
        external_pool_adapter_atomic_activation::{
            canonical_external_pool_adapter_atomic_activation_receipt_json_and_digest,
            ExternalPoolAdapterAtomicActivationReceipt,
        },
        provider::ComputeProvider,
        route_authority::{
            canonical_route_adapter_version_json_and_digest,
            canonical_route_authorization_json_and_digest,
            canonical_route_authorization_seal_json_and_digest,
            canonical_route_credential_json_and_digest,
            canonical_service_actor_authorization_json_and_digest,
            AuthorizedComputeRouteAuthorization,
        },
    },
    store::{
        compute_external_pool_adapter_runtime_bundle::{
            ExternalPoolAdapterAtomicActivationPendingPlan,
            ExternalPoolAdapterAtomicActivationPendingWrite,
            ExternalPoolAdapterAtomicActivationPendingWriteKind as Kind,
        },
        compute_provider_registry::ComputeProviderRegistrationReceipt,
    },
};

pub(super) fn build_pending_plan(
    source: &ComputeProviderRegistrationReceipt,
    target: &ComputeProvider,
    target_digest: &str,
    route: &AuthorizedComputeRouteAuthorization,
    receipt: &ExternalPoolAdapterAtomicActivationReceipt,
) -> Result<ExternalPoolAdapterAtomicActivationPendingPlan> {
    let target_json = serde_json::to_string(target)?;
    let transition = &receipt.activation.provider_transition;
    ensure!(
        transition.source_registering_provider.provider_id == source.provider.provider_id
            && transition
                .source_registering_provider
                .provider_policy_revision
                == source.provider.policy_revision
            && transition.source_registering_provider.provider_digest == source.provider_digest
            && transition.target_active_provider.provider_id == target.provider_id
            && transition.target_active_provider.provider_policy_revision == target.policy_revision
            && transition.target_active_provider.provider_json == target_json
            && transition.target_active_provider.provider_digest == target_digest,
        "V277 pending plan Provider transition is not exact"
    );
    let inputs = route.inputs();
    let actor = inputs.actor().envelope();
    let adapter = inputs.adapter().envelope();
    let credential = inputs.credential().envelope();
    let authorization = route.envelope();
    let seal = route.seal();
    let (actor_json, actor_digest) = canonical_service_actor_authorization_json_and_digest(actor)?;
    let (adapter_json, adapter_digest) = canonical_route_adapter_version_json_and_digest(adapter)?;
    let (credential_json, credential_digest) =
        canonical_route_credential_json_and_digest(credential)?;
    let (authorization_json, authorization_digest) =
        canonical_route_authorization_json_and_digest(authorization)?;
    let (seal_json, seal_digest) = canonical_route_authorization_seal_json_and_digest(seal)?;
    let closure = &receipt.activation.route_closure;
    let projected = &receipt.activation.projected_v211_binding;
    ensure!(
        actor_digest == actor.actor_authorization_digest
            && adapter_digest == adapter.adapter_digest
            && credential_digest == credential.credential_digest
            && authorization_digest == authorization.route_authorization_digest
            && seal_digest == seal.seal_digest
            && closure.route_adapter_projection_id == adapter.adapter_id
            && closure.route_adapter_revision == adapter.adapter_revision
            && closure.route_adapter_digest == adapter.adapter_digest
            && closure.service_actor_id == actor.authorization.service_actor_id
            && closure.service_actor_authorization_id == actor.actor_authorization_id
            && closure.service_actor_authorization_digest == actor.actor_authorization_digest
            && closure.route_credential_id == credential.credential_id
            && closure.route_credential_revision == credential.credential_revision
            && closure.route_credential_digest == credential.credential_digest
            && closure.route_authorization_id == authorization.route_authorization_id
            && closure.route_authorization_revision == authorization.route_authorization_revision
            && closure.route_authorization_digest == authorization.route_authorization_digest
            && closure.capabilities == authorization.authorization.capabilities
            && usize::try_from(closure.route_capability_count).ok()
                == Some(authorization.authorization.capabilities.len())
            && closure.route_capability_count == seal.capability_count
            && closure.route_capability_set_digest == seal.capability_set_digest
            && closure.route_seal_id == seal.seal_id
            && closure.route_seal_digest == seal.seal_digest
            && seal.route_authorization_id == authorization.route_authorization_id
            && seal.route_authorization_revision == authorization.route_authorization_revision
            && seal.route_authorization_digest == authorization.route_authorization_digest
            && seal.adapter_id == adapter.adapter_id
            && seal.adapter_revision == adapter.adapter_revision
            && seal.adapter_registry_digest
                == authorization
                    .authorization
                    .route
                    .adapter
                    .adapter_registry_digest
            && seal.credential_id == credential.credential_id
            && seal.credential_revision == credential.credential_revision
            && seal.credential_digest == credential.credential_digest
            && projected.projected_v211_adapter_binding_digest
                == authorization.authorization.route.adapter_binding_digest
            && authorization.authorization.route.route_binding_digest
                == authorization.authorization.route.adapter_binding_digest,
        "V277 pending route canonical bytes or receipt closure are not exact"
    );
    let (receipt_json, receipt_digest) =
        canonical_external_pool_adapter_atomic_activation_receipt_json_and_digest(receipt)?;
    ensure!(
        receipt_digest == receipt.activation_receipt_digest,
        "V277 pending receipt digest is not exact"
    );

    let old = &source.provider;
    let new = target;
    let mut writes = vec![
        write(
            Kind::ServiceActorAuthorization,
            vec![
                text(&actor.actor_authorization_id),
                text(&actor.actor_authorization_digest),
                Value::Text(actor_json),
            ],
        )?,
        write(
            Kind::ProjectionAdapterVersion,
            vec![
                text(&adapter.adapter_id),
                integer(adapter.adapter_revision),
                text(&adapter.adapter_digest),
                Value::Text(adapter_json),
            ],
        )?,
        write(
            Kind::ProjectionAdapter,
            vec![
                text(&adapter.adapter_id),
                integer(adapter.adapter_revision),
                text(&adapter.adapter_digest),
                text(&adapter.adapter.status),
                text(&adapter.adapter.registered_at),
                text(&adapter.adapter.registered_at),
            ],
        )?,
        write(
            Kind::RouteCredential,
            vec![
                text(&credential.credential_id),
                integer(credential.credential_revision),
                text(&credential.credential_digest),
                Value::Text(credential_json),
            ],
        )?,
        write(
            Kind::RouteAuthorization,
            vec![
                text(&authorization.route_authorization_id),
                integer(authorization.route_authorization_revision),
                text(&authorization.route_authorization_digest),
                Value::Text(authorization_json),
            ],
        )?,
    ];
    for capability in &authorization.authorization.capabilities {
        writes.push(write(
            Kind::RouteCapability,
            vec![
                text(&authorization.route_authorization_id),
                integer(capability.ordinal),
                text(&capability.capability_id),
                integer(capability.capability_revision),
            ],
        )?);
    }
    writes.extend([
        write(
            Kind::RouteSeal,
            vec![
                text(&seal.seal_id),
                text(&seal.seal_digest),
                Value::Text(seal_json),
            ],
        )?,
        write(
            Kind::ProviderVersion,
            vec![
                text(&new.provider_id),
                integer(new.policy_revision),
                text(target_digest),
                Value::Text(target_json),
                text(&receipt.activation.evidence_checked_at),
            ],
        )?,
        write(
            Kind::ProviderUpdate,
            vec![
                text(&old.provider_id),
                text(&old.provider_kind),
                text(&old.owner_account_id),
                optional_text(old.settlement_account_id.as_deref()),
                text(&old.display_name),
                text(&old.status),
                text(&old.trust_tier),
                optional_text(old.home_region.as_deref()),
                integer(old.policy_revision),
                text(&source.provider_digest),
                text(&old.created_at),
                text(&old.updated_at),
                text(&new.provider_id),
                text(&new.provider_kind),
                text(&new.owner_account_id),
                optional_text(new.settlement_account_id.as_deref()),
                text(&new.display_name),
                text(&new.status),
                text(&new.trust_tier),
                optional_text(new.home_region.as_deref()),
                integer(new.policy_revision),
                text(target_digest),
                text(&new.created_at),
                text(&new.updated_at),
            ],
        )?,
        write(
            Kind::ActivationReceipt,
            vec![
                text(&receipt.activation_receipt_id),
                text(&receipt.activation_receipt_digest),
                text(&receipt.activation.identity.activation_root_digest),
                Value::Text(receipt_json),
            ],
        )?,
    ]);
    ExternalPoolAdapterAtomicActivationPendingPlan::new(writes)
}

fn write(
    kind: Kind,
    values: Vec<Value>,
) -> Result<ExternalPoolAdapterAtomicActivationPendingWrite> {
    ExternalPoolAdapterAtomicActivationPendingWrite::new(kind, values)
}

fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}

fn optional_text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, text)
}

fn integer(value: i64) -> Value {
    Value::Integer(value)
}
