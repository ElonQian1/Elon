use anyhow::Result;
use sha2::{Digest, Sha256};

use super::*;

const STARTED_AT: &str = "2026-08-15T00:00:00.000000000Z";
const RECEIVED_AT: &str = "2026-08-15T00:00:01.000000000Z";
const RECORDED_AT: &str = "2026-08-15T00:00:02.000000000Z";
const NOT_AFTER: &str = "2026-08-15T00:01:00.000000000Z";

pub(crate) struct ExactEnvelopeFixture {
    pub function: &'static str,
    pub id_field: &'static str,
    pub canonical_json: String,
}

pub(crate) fn exact_envelope_fixtures() -> Result<Vec<ExactEnvelopeFixture>> {
    let attempt = exchange_attempt()?;
    let receipt = exchange_receipt(&attempt)?;
    let reconcile_poll = reconcile_poll(&attempt)?;
    let event_poll = event_poll(&receipt)?;
    let (event_batch, event) = event_batch_and_event(&event_poll, &receipt)?;

    Ok(vec![
        fixture(
            "elon_v273_task_exchange_attempt_is_exact",
            "exchange_attempt_id",
            canonical_task_production_exchange_attempt_json_and_digest(&attempt)?.0,
        ),
        fixture(
            "elon_v273_task_exchange_receipt_is_exact",
            "exchange_receipt_id",
            canonical_task_production_exchange_receipt_json_and_digest(&receipt)?.0,
        ),
        fixture(
            "elon_v273_task_reconcile_poll_is_exact",
            "reconcile_poll_id",
            canonical_task_production_reconcile_poll_json_and_digest(&reconcile_poll)?.0,
        ),
        fixture(
            "elon_v273_task_event_poll_is_exact",
            "event_poll_id",
            canonical_task_production_event_poll_json_and_digest(&event_poll)?.0,
        ),
        fixture(
            "elon_v273_task_event_batch_is_exact",
            "event_batch_id",
            canonical_task_production_event_batch_json_and_digest(&event_batch)?.0,
        ),
        fixture(
            "elon_v273_task_event_is_exact",
            "event_id",
            canonical_task_production_event_json_and_digest(&event)?.0,
        ),
    ])
}

fn fixture(
    function: &'static str,
    id_field: &'static str,
    canonical_json: String,
) -> ExactEnvelopeFixture {
    ExactEnvelopeFixture {
        function,
        id_field,
        canonical_json,
    }
}

fn exchange_attempt() -> Result<ExternalPoolAdapterTaskExchangeAttemptEnvelope> {
    let mut envelope = ExternalPoolAdapterTaskExchangeAttemptEnvelope {
        schema: TASK_PRODUCTION_EXCHANGE_ATTEMPT_SCHEMA.into(),
        exchange_attempt_id: "attempt-1".into(),
        exchange_attempt_digest: digest("attempt-placeholder"),
        canonicalization: TASK_PRODUCTION_CANONICALIZATION.into(),
        digest_algorithm: TASK_PRODUCTION_DIGEST_ALGORITHM.into(),
        attempt: ExternalPoolAdapterTaskExchangeAttemptMaterial {
            identity: exchange_identity()?,
            started_at: STARTED_AT.into(),
            boundary: boundary(),
        },
    };
    envelope.exchange_attempt_digest =
        canonical_task_production_exchange_attempt_json_and_digest(&envelope)?.1;
    validate_task_production_exchange_attempt(&envelope)?;
    Ok(envelope)
}

fn exchange_receipt(
    attempt: &ExternalPoolAdapterTaskExchangeAttemptEnvelope,
) -> Result<ExternalPoolAdapterTaskExchangeReceiptEnvelope> {
    let mut envelope = ExternalPoolAdapterTaskExchangeReceiptEnvelope {
        schema: TASK_PRODUCTION_EXCHANGE_RECEIPT_SCHEMA.into(),
        exchange_receipt_id: "receipt-1".into(),
        exchange_receipt_digest: digest("receipt-placeholder"),
        canonicalization: TASK_PRODUCTION_CANONICALIZATION.into(),
        digest_algorithm: TASK_PRODUCTION_DIGEST_ALGORITHM.into(),
        receipt: ExternalPoolAdapterTaskExchangeReceiptMaterial {
            exchange_attempt_id: attempt.exchange_attempt_id.clone(),
            exchange_attempt_digest: attempt.exchange_attempt_digest.clone(),
            identity: attempt.attempt.identity.clone(),
            exchange_ordinal: 1,
            exchange_nonce_digest: digest("exchange-nonce"),
            upstream_request_bytes: 1,
            upstream_request_sha256: digest("upstream-request"),
            upstream_response_bytes: 1,
            upstream_response_sha256: digest("upstream-response"),
            semantic_observation_bytes: 1,
            semantic_observation_sha256: digest("semantic-observation"),
            session_transcript_digest: attempt
                .attempt
                .identity
                .session
                .session_transcript_digest
                .clone(),
            exchange_root: digest("exchange-root"),
            authenticated_at: STARTED_AT.into(),
            received_at: RECEIVED_AT.into(),
            recorded_at: RECORDED_AT.into(),
            boundary: boundary(),
        },
    };
    envelope.exchange_receipt_digest =
        canonical_task_production_exchange_receipt_json_and_digest(&envelope)?.1;
    validate_task_production_exchange_receipt(&envelope)?;
    Ok(envelope)
}

fn reconcile_poll(
    attempt: &ExternalPoolAdapterTaskExchangeAttemptEnvelope,
) -> Result<ExternalPoolAdapterTaskReconcilePollEnvelope> {
    let mut envelope = ExternalPoolAdapterTaskReconcilePollEnvelope {
        schema: TASK_PRODUCTION_RECONCILE_POLL_SCHEMA.into(),
        reconcile_poll_id: "reconcile-poll-1".into(),
        reconcile_poll_digest: digest("reconcile-placeholder"),
        canonicalization: TASK_PRODUCTION_CANONICALIZATION.into(),
        digest_algorithm: TASK_PRODUCTION_DIGEST_ALGORITHM.into(),
        poll: ExternalPoolAdapterTaskReconcilePollIntent {
            lineage: first_poll_lineage(),
            uncertain_exchange_attempt_id: attempt.exchange_attempt_id.clone(),
            uncertain_exchange_attempt_digest: attempt.exchange_attempt_digest.clone(),
            command: poll_command(),
            remote: remote_identity(None, "unknown")?,
            authenticated_subject_sha256: None,
            request_digest: digest("reconcile-request"),
            not_before: STARTED_AT.into(),
            not_after: NOT_AFTER.into(),
            created_at: RECEIVED_AT.into(),
            boundary: boundary(),
        },
    };
    envelope.reconcile_poll_digest =
        canonical_task_production_reconcile_poll_json_and_digest(&envelope)?.1;
    validate_task_production_reconcile_poll(&envelope)?;
    Ok(envelope)
}

fn event_poll(
    receipt: &ExternalPoolAdapterTaskExchangeReceiptEnvelope,
) -> Result<ExternalPoolAdapterTaskEventPollEnvelope> {
    let remote = remote_identity(Some("remote-execution-1"), "running")?;
    let subject = ExternalPoolAdapterTaskAuthenticatedRemoteSubject {
        remote: remote.clone(),
    };
    let cursor = event_cursor(0, None)?;
    let mut envelope =
        ExternalPoolAdapterTaskEventPollEnvelope {
            schema: TASK_PRODUCTION_EVENT_POLL_SCHEMA.into(),
            event_poll_id: "event-poll-1".into(),
            event_poll_digest: digest("event-poll-placeholder"),
            canonicalization: TASK_PRODUCTION_CANONICALIZATION.into(),
            digest_algorithm: TASK_PRODUCTION_DIGEST_ALGORITHM.into(),
            poll: ExternalPoolAdapterTaskEventPollIntent {
                lineage: first_poll_lineage(),
                source_exchange_receipt_id: receipt.exchange_receipt_id.clone(),
                source_exchange_receipt_digest: receipt.exchange_receipt_digest.clone(),
                command: poll_command(),
                remote,
                authenticated_subject_sha256:
                    canonical_task_production_remote_subject_json_and_sha256(&subject)?.1,
                requested_cursor: cursor,
                request_digest: digest("event-poll-request"),
                not_before: STARTED_AT.into(),
                not_after: NOT_AFTER.into(),
                created_at: RECEIVED_AT.into(),
                boundary: boundary(),
            },
        };
    envelope.event_poll_digest = canonical_task_production_event_poll_json_and_digest(&envelope)?.1;
    validate_task_production_event_poll(&envelope)?;
    Ok(envelope)
}

fn event_batch_and_event(
    poll: &ExternalPoolAdapterTaskEventPollEnvelope,
    receipt: &ExternalPoolAdapterTaskExchangeReceiptEnvelope,
) -> Result<(
    ExternalPoolAdapterTaskEventBatchEnvelope,
    ExternalPoolAdapterTaskEventEnvelope,
)> {
    let mut event_material = ExternalPoolAdapterTaskEventMaterial {
        event_batch_id: "event-batch-1".into(),
        event_batch_digest: digest("event-batch-link-placeholder"),
        remote_identity_digest: poll.poll.remote.remote_identity_digest.clone(),
        event_ordinal: 1,
        remote_event_id: "remote-event-1".into(),
        event_type: "completed".into(),
        remote_sequence: 1,
        previous_event_root: None,
        event_root: digest("event-root-placeholder"),
        canonical_event_digest: digest("canonical-event"),
        observed_at: RECEIVED_AT.into(),
        recorded_at: RECORDED_AT.into(),
        boundary: boundary(),
    };
    event_material.event_root = task_production_event_root(&event_material)?;

    let event_roots = vec![event_material.event_root.clone()];
    let mut batch_material = ExternalPoolAdapterTaskEventBatchMaterial {
        event_poll_id: poll.event_poll_id.clone(),
        event_poll_digest: poll.event_poll_digest.clone(),
        exchange_receipt_id: receipt.exchange_receipt_id.clone(),
        exchange_receipt_digest: receipt.exchange_receipt_digest.clone(),
        predecessor_event_batch_id: None,
        predecessor_event_batch_digest: None,
        remote: poll.poll.remote.clone(),
        authenticated_observation_sha256: digest("observation-placeholder"),
        cursor_before: poll.poll.requested_cursor.clone(),
        cursor_after: event_cursor(1, Some(event_material.event_root.as_str()))?,
        previous_batch_root: None,
        batch_root: digest("batch-root-placeholder"),
        replay_classification: "new".into(),
        event_count: 1,
        event_roots,
        event_inventory_digest: task_production_event_inventory_digest(&[event_material
            .event_root
            .clone()])?,
        authenticated_at: STARTED_AT.into(),
        received_at: RECEIVED_AT.into(),
        recorded_at: RECORDED_AT.into(),
        boundary: boundary(),
    };
    batch_material.batch_root = task_production_event_batch_root(&batch_material)?;
    batch_material.authenticated_observation_sha256 =
        canonical_task_production_authenticated_event_observation_json_and_sha256(
            &task_production_authenticated_event_observation(&batch_material),
        )?
        .1;

    let mut batch = ExternalPoolAdapterTaskEventBatchEnvelope {
        schema: TASK_PRODUCTION_EVENT_BATCH_SCHEMA.into(),
        event_batch_id: event_material.event_batch_id.clone(),
        event_batch_digest: digest("event-batch-placeholder"),
        canonicalization: TASK_PRODUCTION_CANONICALIZATION.into(),
        digest_algorithm: TASK_PRODUCTION_DIGEST_ALGORITHM.into(),
        batch: batch_material,
    };
    batch.event_batch_digest = canonical_task_production_event_batch_json_and_digest(&batch)?.1;
    validate_task_production_event_batch(&batch)?;

    event_material.event_batch_digest = batch.event_batch_digest.clone();
    let mut event = ExternalPoolAdapterTaskEventEnvelope {
        schema: TASK_PRODUCTION_EVENT_SCHEMA.into(),
        event_id: "event-1".into(),
        event_digest: digest("event-placeholder"),
        canonicalization: TASK_PRODUCTION_CANONICALIZATION.into(),
        digest_algorithm: TASK_PRODUCTION_DIGEST_ALGORITHM.into(),
        event: event_material,
    };
    event.event_digest = canonical_task_production_event_json_and_digest(&event)?.1;
    validate_task_production_event(&event)?;
    Ok((batch, event))
}

fn exchange_identity() -> Result<ExternalPoolAdapterTaskExchangeIdentity> {
    let roots = ExternalPoolAdapterTaskProductionSessionRoots {
        supervisor_session_policy_digest: digest("session-policy"),
        runtime_launch_profile_digest: digest("runtime-profile"),
        task_protocol_profile_digest: digest("protocol-profile"),
        upstream_transport_target_digest: digest("transport-target"),
        supervisor_session_policy_companion_digest: digest("policy-companion"),
        launch_image_sha256: digest("launch-image"),
        ephemeral_task_secret_delivery_root: digest("secret-delivery"),
        task_protocol_conformance_run_receipt_digest: digest("conformance-receipt"),
    };
    let session_roots_digest = task_production_session_roots_digest(&roots)?;
    let command = command_binding();
    Ok(ExternalPoolAdapterTaskExchangeIdentity {
        operation_kind: "prepare".into(),
        source: ExternalPoolAdapterTaskExchangeSource {
            source_kind: TASK_PRODUCTION_SOURCE_START_SEND.into(),
            source_id: command.send_attempt_id.clone(),
            source_digest: command.send_attempt_digest.clone(),
        },
        adapter: ExternalPoolAdapterTaskAdapterBinding {
            provider_id: "provider-1".into(),
            adapter_id: "adapter-1".into(),
            adapter_revision: 1,
            adapter_registry_digest: digest("adapter-registry"),
            adapter_implementation_digest: digest("adapter-implementation"),
        },
        command,
        route: ExternalPoolAdapterTaskRouteBinding {
            route_authorization_id: "route-authorization-1".into(),
            route_authorization_digest: digest("route-authorization"),
            route_credential_id: "route-credential-1".into(),
            route_credential_revision: 1,
            route_credential_digest: digest("route-credential"),
            credential_verification_receipt_id: "credential-receipt-1".into(),
            credential_verification_receipt_digest: digest("credential-receipt"),
            credential_verifier_id: "credential-verifier-1".into(),
            credential_verifier_revision: 1,
            credential_verifier_digest: digest("credential-verifier"),
        },
        executor_binding_digest: digest("executor-binding"),
        fencing_generation: 1,
        fence_digest: digest("fence"),
        session: ExternalPoolAdapterTaskSessionBinding {
            roots,
            session_roots_digest: session_roots_digest.clone(),
            session_transcript_digest: session_roots_digest,
            upstream_transport_target_id: "transport-target-1".into(),
            task_protocol_conformance_run_receipt_id: "conformance-receipt-1".into(),
        },
        request_digest: digest("request"),
        delivery_attempt_digest: digest("delivery-attempt"),
    })
}

fn command_binding() -> ExternalPoolAdapterTaskCommandBinding {
    ExternalPoolAdapterTaskCommandBinding {
        command_id: "command-1".into(),
        command_digest: digest("command"),
        outbox_id: "outbox-1".into(),
        outbox_digest: digest("outbox"),
        send_attempt_id: "send-attempt-1".into(),
        send_attempt_digest: digest("send-attempt"),
    }
}

fn poll_command() -> ExternalPoolAdapterTaskPollCommandBinding {
    let command = command_binding();
    ExternalPoolAdapterTaskPollCommandBinding {
        command_id: command.command_id,
        command_digest: command.command_digest,
        outbox_id: command.outbox_id,
        outbox_digest: command.outbox_digest,
        send_attempt_id: command.send_attempt_id,
        send_attempt_digest: command.send_attempt_digest,
        route_authorization_id: "route-authorization-1".into(),
        route_authorization_digest: digest("route-authorization"),
        executor_binding_digest: digest("executor-binding"),
        fencing_generation: 1,
        fence_digest: digest("fence"),
    }
}

fn remote_identity(
    remote_execution_id: Option<&str>,
    remote_execution_state: &str,
) -> Result<ExternalPoolAdapterTaskRemoteIdentity> {
    let executor_binding_digest = digest("executor-binding");
    Ok(ExternalPoolAdapterTaskRemoteIdentity {
        remote_identity_digest: task_production_remote_identity_digest(
            &executor_binding_digest,
            remote_execution_id,
        )?,
        executor_binding_digest,
        remote_execution_id: remote_execution_id.map(str::to_owned),
        remote_execution_state: remote_execution_state.into(),
    })
}

fn first_poll_lineage() -> ExternalPoolAdapterTaskPollLineage {
    ExternalPoolAdapterTaskPollLineage {
        predecessor_id: None,
        predecessor_digest: None,
        poll_ordinal: 1,
    }
}

fn event_cursor(
    remote_sequence: u64,
    previous_event_root: Option<&str>,
) -> Result<ExternalPoolAdapterTaskEventCursor> {
    Ok(ExternalPoolAdapterTaskEventCursor {
        remote_sequence,
        previous_event_root: previous_event_root.map(str::to_owned),
        cursor_digest: task_production_event_cursor_digest(remote_sequence, previous_event_root)?,
    })
}

fn boundary() -> ExternalPoolAdapterTaskProductionBoundary {
    ExternalPoolAdapterTaskProductionBoundary {
        authority_status: TASK_PRODUCTION_NO_V213_AUTHORITY.into(),
        effects: ExternalPoolAdapterTaskProductionEffects::none(),
        readiness: ExternalPoolAdapterTaskProductionReadiness::none(),
    }
}

fn digest(label: &str) -> String {
    hex::encode(Sha256::digest(label.as_bytes()))
}
