use anyhow::{anyhow, bail, ensure, Result};
use chrono::{DateTime, SecondsFormat};

use crate::compute_federation::start_outbox::{
    canonical_attempt_dispatch_actor_receipt_json_and_digest,
    canonical_lease_authority_binding_json_and_digest, canonical_lease_authority_scopes_digest,
    canonical_start_outbox_operation_json_and_digest, ComputeAttemptDispatchActorReceiptEnvelope,
    ComputeLeaseAuthorityBindingEnvelope, ComputeStartOutboxOperationEnvelope,
    COMPUTE_ACTOR_RECEIPT_PHASE_APPLICATION, COMPUTE_ATTEMPT_DISPATCH_ACTOR_RECEIPT_SCHEMA,
    COMPUTE_LEASE_AUTHORITY_BINDING_SCHEMA, COMPUTE_START_OPERATION_COMMIT,
    COMPUTE_START_OUTBOX_CANONICALIZATION, COMPUTE_START_OUTBOX_DIGEST_ALGORITHM,
    COMPUTE_START_OUTBOX_OPERATION_SCHEMA,
};

use super::{AcceptedCommitSource, DerivedAcceptedCommitClosure};

pub(super) fn derive_closure(
    source: &AcceptedCommitSource,
    closure_at: &str,
) -> Result<DerivedAcceptedCommitClosure> {
    parse_canonical_time(closure_at, "accepted closure time")?;
    let base = &source.base;
    let route = &base.route.authorization;
    let actor_authority = &base.actor_authority.authorization;
    let application = &source.application.envelope;
    let actor_valid_until = std::cmp::min(&route.expires_at, &actor_authority.valid_until).clone();
    ensure!(
        source.ack.received_at.as_str() <= closure_at && closure_at < actor_valid_until.as_str(),
        "accepted closure application actor is outside its authority window"
    );
    let mut actor = ComputeAttemptDispatchActorReceiptEnvelope {
        schema: COMPUTE_ATTEMPT_DISPATCH_ACTOR_RECEIPT_SCHEMA.to_string(),
        actor_receipt_id: format!(
            "attempt_application_actor_{}",
            application.application_digest
        ),
        actor_receipt_digest: String::new(),
        canonicalization: COMPUTE_START_OUTBOX_CANONICALIZATION.to_string(),
        digest_algorithm: COMPUTE_START_OUTBOX_DIGEST_ALGORITHM.to_string(),
        actor_phase: COMPUTE_ACTOR_RECEIPT_PHASE_APPLICATION.to_string(),
        command_id: base.command.command_id.clone(),
        command_digest: base.command.command_digest.clone(),
        provider_id: route.provider.provider_id.clone(),
        provider_owner_account_id: route.provider.provider_owner_account_id.clone(),
        service_actor_id: route.verified_by_service_actor_id.clone(),
        actor_authorization_id: route.actor_authorization_id.clone(),
        actor_authorization_digest: route.actor_authorization_digest.clone(),
        route_authorization_id: base.route.route_authorization_id.clone(),
        route_authorization_digest: base.route.route_authorization_digest.clone(),
        ack_id: Some(source.ack.ack_id.clone()),
        ack_digest: Some(source.ack.ack_digest.clone()),
        application_id: Some(application.application_id.clone()),
        application_digest: Some(application.application_digest.clone()),
        issued_at: closure_at.to_string(),
        valid_until: actor_valid_until,
        recorded_at: closure_at.to_string(),
    };
    let (_, actor_digest) = canonical_attempt_dispatch_actor_receipt_json_and_digest(&actor)?;
    actor.actor_receipt_digest = actor_digest;
    let (actor_json, actor_digest) =
        canonical_attempt_dispatch_actor_receipt_json_and_digest(&actor)?;
    ensure!(
        actor_digest == actor.actor_receipt_digest,
        "accepted actor digest drift"
    );

    let plan_authority = &base.plan.plan.lease_authority;
    let scopes_digest = canonical_lease_authority_scopes_digest(&plan_authority.required_scopes)?;
    ensure!(
        closure_at < plan_authority.valid_until.as_str(),
        "accepted lease authority is already expired"
    );
    let mut authority = ComputeLeaseAuthorityBindingEnvelope {
        schema: COMPUTE_LEASE_AUTHORITY_BINDING_SCHEMA.to_string(),
        lease_authority_id: format!("attempt_lease_authority_{}", application.application_digest),
        authority_revision: 1,
        lease_authority_digest: String::new(),
        canonicalization: COMPUTE_START_OUTBOX_CANONICALIZATION.to_string(),
        digest_algorithm: COMPUTE_START_OUTBOX_DIGEST_ALGORITHM.to_string(),
        authority_kind: plan_authority.authority_kind.clone(),
        delivery_mode: plan_authority.delivery_mode.clone(),
        non_bearer_authority_ref: base.lease_credential_ref.clone(),
        authority_hint: base.lease_credential_hint.clone(),
        command_id: base.command.command_id.clone(),
        command_digest: base.command.command_digest.clone(),
        plan_id: base.plan.plan_id.clone(),
        plan_digest: base.plan.plan_digest.clone(),
        ack_id: source.ack.ack_id.clone(),
        ack_digest: source.ack.ack_digest.clone(),
        application_id: application.application_id.clone(),
        application_digest: application.application_digest.clone(),
        application_actor_receipt_id: actor.actor_receipt_id.clone(),
        application_actor_receipt_digest: actor.actor_receipt_digest.clone(),
        lease_id: source.activation.lease.lease_id.clone(),
        lease_digest: source.activation.lease_digest.clone(),
        provider_id: source.activation.lease.provider_id.clone(),
        executor_id: source.activation.lease.executor_id.clone(),
        fencing_generation: source.activation.lease.fencing_generation,
        route_authorization_id: base.route.route_authorization_id.clone(),
        route_authorization_digest: base.route.route_authorization_digest.clone(),
        audience: plan_authority.audience.clone(),
        scopes: plan_authority.required_scopes.clone(),
        scopes_digest,
        issued_at: closure_at.to_string(),
        expires_at: plan_authority.valid_until.clone(),
        recorded_at: closure_at.to_string(),
    };
    let (_, authority_digest) = canonical_lease_authority_binding_json_and_digest(&authority)?;
    authority.lease_authority_digest = authority_digest;
    let (authority_json, authority_digest) =
        canonical_lease_authority_binding_json_and_digest(&authority)?;
    ensure!(
        authority_digest == authority.lease_authority_digest,
        "accepted lease authority digest drift"
    );

    let not_after = [
        source.activation.lease.expires_at.as_str(),
        route.expires_at.as_str(),
        actor.valid_until.as_str(),
        authority.expires_at.as_str(),
    ]
    .into_iter()
    .min()
    .ok_or_else(|| anyhow!("accepted Commit window is missing"))?
    .to_string();
    ensure!(
        closure_at < not_after.as_str(),
        "accepted Commit window is closed"
    );
    let mut commit = ComputeStartOutboxOperationEnvelope {
        schema: COMPUTE_START_OUTBOX_OPERATION_SCHEMA.to_string(),
        outbox_id: format!("attempt_start_commit_{}", application.application_digest),
        outbox_digest: String::new(),
        canonicalization: COMPUTE_START_OUTBOX_CANONICALIZATION.to_string(),
        digest_algorithm: COMPUTE_START_OUTBOX_DIGEST_ALGORITHM.to_string(),
        operation_kind: COMPUTE_START_OPERATION_COMMIT.to_string(),
        operation_generation: 1,
        subject_outbox_id: Some(base.prepare.envelope.outbox_id.clone()),
        command_id: base.command.command_id.clone(),
        command_digest: base.command.command_digest.clone(),
        adapter_binding_digest: base.prepare.envelope.adapter_binding_digest.clone(),
        route_authorization_id: base.route.route_authorization_id.clone(),
        route_authorization_digest: base.route.route_authorization_digest.clone(),
        plan_id: base.plan.plan_id.clone(),
        plan_digest: base.plan.plan_digest.clone(),
        lease_id: source.activation.lease.lease_id.clone(),
        fencing_generation: source.activation.lease.fencing_generation,
        ack_id: Some(source.ack.ack_id.clone()),
        ack_digest: Some(source.ack.ack_digest.clone()),
        application_id: Some(application.application_id.clone()),
        application_digest: Some(application.application_digest.clone()),
        lease_authority_id: Some(authority.lease_authority_id.clone()),
        lease_authority_revision: Some(authority.authority_revision),
        lease_authority_digest: Some(authority.lease_authority_digest.clone()),
        actor_receipt_id: actor.actor_receipt_id.clone(),
        actor_receipt_digest: actor.actor_receipt_digest.clone(),
        issued_at: closure_at.to_string(),
        not_before: closure_at.to_string(),
        not_after,
    };
    let (_, commit_digest) = canonical_start_outbox_operation_json_and_digest(&commit)?;
    commit.outbox_digest = commit_digest;
    let (commit_json, commit_digest) = canonical_start_outbox_operation_json_and_digest(&commit)?;
    ensure!(
        commit_digest == commit.outbox_digest,
        "accepted Commit digest drift"
    );
    Ok(DerivedAcceptedCommitClosure {
        actor,
        actor_json,
        authority,
        authority_json,
        commit,
        commit_json,
        provider_id: base.adapter.provider_id.clone(),
        adapter_id: base.adapter.adapter_id.clone(),
    })
}

fn parse_canonical_time(value: &str, label: &str) -> Result<DateTime<chrono::FixedOffset>> {
    let parsed =
        DateTime::parse_from_rfc3339(value).map_err(|_| anyhow!("{label} is not RFC3339"))?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("{label} must use canonical UTC nanoseconds");
    }
    Ok(parsed)
}
