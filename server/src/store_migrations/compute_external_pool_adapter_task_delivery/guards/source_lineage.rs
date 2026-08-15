use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(ATTEMPT_SOURCE)?;
    install_receipt_reproof(conn)?;
    conn.execute_batch(POLL_SOURCE)?;
    Ok(())
}

fn install_receipt_reproof(conn: &Connection) -> Result<()> {
    let columns = [
        "exchange_attempt_digest",
        "operation_kind",
        "source_kind",
        "source_id",
        "source_digest",
        "provider_id",
        "adapter_id",
        "adapter_revision",
        "adapter_registry_digest",
        "adapter_implementation_digest",
        "command_id",
        "command_digest",
        "outbox_id",
        "outbox_digest",
        "send_attempt_id",
        "send_attempt_digest",
        "route_authorization_id",
        "route_authorization_digest",
        "route_credential_id",
        "route_credential_revision",
        "route_credential_digest",
        "credential_verification_receipt_id",
        "credential_verification_receipt_digest",
        "credential_verifier_id",
        "credential_verifier_revision",
        "credential_verifier_digest",
        "executor_binding_digest",
        "fencing_generation",
        "fence_digest",
        "supervisor_session_policy_digest",
        "runtime_launch_profile_digest",
        "task_protocol_profile_digest",
        "upstream_transport_target_id",
        "upstream_transport_target_digest",
        "supervisor_session_policy_companion_digest",
        "launch_image_sha256",
        "ephemeral_task_secret_delivery_root",
        "task_protocol_conformance_run_receipt_id",
        "task_protocol_conformance_run_receipt_digest",
        "session_roots_digest",
        "session_transcript_digest",
        "request_digest",
        "delivery_attempt_digest",
    ];
    let exact = columns
        .iter()
        .map(|column| format!("attempt.{column}=NEW.{column}"))
        .collect::<Vec<_>>()
        .join(" AND ");
    conn.execute_batch(&format!(
        "CREATE TRIGGER IF NOT EXISTS v273_task_exchange_receipt_attempt_reproof
         BEFORE INSERT ON compute_external_pool_adapter_task_exchange_receipts
         WHEN NOT EXISTS (
           SELECT 1 FROM compute_external_pool_adapter_task_exchange_attempts attempt
            WHERE attempt.exchange_attempt_id=NEW.exchange_attempt_id AND {exact})
         BEGIN SELECT RAISE(ABORT,'V273 receipt does not reprove the exact attempt'); END;"
    ))?;
    Ok(())
}

const ATTEMPT_SOURCE: &str = r#"
CREATE TRIGGER IF NOT EXISTS v273_task_exchange_attempt_operation_source
BEFORE INSERT ON compute_external_pool_adapter_task_exchange_attempts
WHEN NOT (
  (NEW.operation_kind IN ('prepare','idempotent_commit','cancel_no_start') AND NEW.source_kind='start_outbox_send_attempt' AND NEW.source_id=NEW.send_attempt_id AND NEW.source_digest=NEW.send_attempt_digest)
  OR (NEW.operation_kind='reconcile' AND NEW.source_kind='reconcile_poll')
  OR (NEW.operation_kind='authenticated_events' AND NEW.source_kind='event_poll')
)
BEGIN SELECT RAISE(ABORT,'V273 exchange operation/source mismatch'); END;

CREATE TRIGGER IF NOT EXISTS v273_task_exchange_attempt_exact_authority
BEFORE INSERT ON compute_external_pool_adapter_task_exchange_attempts
WHEN NOT (
  EXISTS (SELECT 1 FROM compute_attempt_start_send_attempts send
    WHERE send.send_attempt_id=NEW.send_attempt_id AND send.send_attempt_digest=NEW.send_attempt_digest
      AND send.outbox_id=NEW.outbox_id AND send.outbox_digest=NEW.outbox_digest
      AND send.command_id=NEW.command_id AND send.command_digest=NEW.command_digest
      AND send.route_authorization_id=NEW.route_authorization_id AND send.route_authorization_digest=NEW.route_authorization_digest
      AND (NEW.source_kind<>'start_outbox_send_attempt' OR (send.request_digest=NEW.request_digest AND ((NEW.operation_kind='prepare' AND send.operation_kind='prepare') OR (NEW.operation_kind='idempotent_commit' AND send.operation_kind='commit') OR (NEW.operation_kind='cancel_no_start' AND send.operation_kind='cancel')))))
  AND EXISTS (SELECT 1 FROM compute_attempt_start_outbox outbox
    WHERE outbox.outbox_id=NEW.outbox_id AND outbox.outbox_digest=NEW.outbox_digest
      AND outbox.command_id=NEW.command_id AND outbox.command_digest=NEW.command_digest
      AND outbox.provider_id=NEW.provider_id AND outbox.route_authorization_id=NEW.route_authorization_id
      AND outbox.route_authorization_digest=NEW.route_authorization_digest
      AND outbox.fencing_generation=NEW.fencing_generation)
  AND EXISTS (SELECT 1 FROM compute_providers provider
    WHERE provider.provider_id=NEW.provider_id AND provider.provider_kind='external_pool' AND provider.status='active')
  AND EXISTS (SELECT 1 FROM compute_route_authorization_receipts route
    JOIN compute_route_credentials credential ON credential.credential_id=route.credential_id
    JOIN compute_route_credential_versions version ON version.credential_id=route.credential_id AND version.credential_revision=route.credential_revision
    WHERE route.route_authorization_id=NEW.route_authorization_id AND route.route_authorization_digest=NEW.route_authorization_digest
      AND route.provider_id=NEW.provider_id AND route.provider_kind='external_pool' AND route.route_kind='server_adapter'
      AND route.adapter_id=NEW.adapter_id AND route.adapter_revision=NEW.adapter_revision
      AND route.adapter_registry_digest=NEW.adapter_registry_digest AND route.implementation_digest=NEW.adapter_implementation_digest
      AND route.credential_id=NEW.route_credential_id AND route.credential_revision=NEW.route_credential_revision AND route.credential_digest=NEW.route_credential_digest
      AND route.verification_receipt_id=NEW.credential_verification_receipt_id AND route.verification_receipt_digest=NEW.credential_verification_receipt_digest
      AND route.verifier_id=NEW.credential_verifier_id AND route.verifier_revision=NEW.credential_verifier_revision AND route.verifier_digest=NEW.credential_verifier_digest
      AND version.credential_digest=NEW.route_credential_digest
      AND (NEW.operation_kind IN ('cancel_no_start','reconcile','authenticated_events')
        OR (credential.current_credential_revision=NEW.route_credential_revision
            AND credential.current_credential_digest=NEW.route_credential_digest
            AND credential.status='active'))
      AND route.authorized_at<=route.recorded_at AND route.recorded_at<=NEW.started_at
      AND ((NEW.operation_kind IN ('prepare','idempotent_commit')
            AND NEW.started_at<route.expires_at AND NEW.started_at<route.credential_expires_at)
        OR (NEW.operation_kind IN ('cancel_no_start','reconcile','authenticated_events')
            AND NEW.started_at<route.cleanup_expires_at
            AND NEW.started_at<route.credential_cleanup_expires_at)))
  AND EXISTS (SELECT 1 FROM compute_external_pool_adapter_credential_reattestation_receipts verification
    LEFT JOIN compute_external_pool_adapter_credential_reattestation_current current_verification
      ON current_verification.reattestation_receipt_id=verification.reattestation_receipt_id
     AND current_verification.reattestation_receipt_digest=verification.reattestation_receipt_digest
    JOIN compute_external_pool_adapter_supervisor_session_policy_companion_current companion
      ON companion.companion_digest=NEW.supervisor_session_policy_companion_digest
    JOIN compute_external_pool_adapter_registry_releases release
      ON release.registry_release_id=verification.registry_release_id
     AND release.registry_release_digest=verification.registry_release_digest
    JOIN compute_route_authorization_receipts verified_route
      ON verified_route.route_authorization_id=NEW.route_authorization_id
     AND verified_route.route_authorization_digest=NEW.route_authorization_digest
    WHERE verification.reattestation_receipt_id=NEW.credential_verification_receipt_id
      AND verification.reattestation_receipt_digest=NEW.credential_verification_receipt_digest
      AND verification.provider_id=NEW.provider_id AND verification.provider_id=companion.provider_id
      AND verification.provider_binding_id=companion.provider_binding_id
      AND verification.provider_binding_digest=companion.provider_binding_digest
      AND verification.observed_provider_policy_revision=companion.provider_policy_revision
      AND verification.observed_provider_digest=companion.provider_digest
      AND verification.adapter_id=companion.logical_adapter_id
      AND verification.release_version=companion.release_version
      AND verification.adapter_config_revision=companion.adapter_config_revision
      AND verification.adapter_config_digest=companion.adapter_config_digest
      AND companion.route_adapter_projection_id=NEW.adapter_id
      AND companion.current_status='supervisor_session_policy_companion_current_inert'
      AND verification.registry_release_id=companion.registry_release_id
      AND verification.registry_release_digest=companion.registry_release_digest
      AND verification.route_adapter_projection_id=NEW.adapter_id
      AND release.implementation_digest=NEW.adapter_implementation_digest
      AND release.credential_verifier_digest=NEW.credential_verifier_digest
      AND verification.credential_verifier_digest=NEW.credential_verifier_digest
      AND json_extract(verification.expected_credential_verifier_json,'$.verification_kind')=verified_route.verification_kind
      AND json_extract(verification.expected_credential_verifier_json,'$.verifier_id')=NEW.credential_verifier_id
      AND json_extract(verification.expected_credential_verifier_json,'$.verifier_revision')=NEW.credential_verifier_revision
      AND json_extract(verification.expected_credential_verifier_json,'$.verifier_digest')=NEW.credential_verifier_digest
      AND verification.verifier_report_id IS NOT NULL
      AND verification.verified_at<=NEW.started_at
      AND (NEW.operation_kind IN ('cancel_no_start','reconcile','authenticated_events')
        OR (NEW.started_at<verification.report_expires_at
            AND current_verification.current_status='verified_current'
            AND current_verification.head_status='head'
            AND current_verification.provider_binding_status='binding_exact'
            AND current_verification.registry_release_status='release_current'
            AND current_verification.provider_subject_status='subject_exact'
            AND current_verification.provider_revision_status IN ('exact_active','adjacent_active')
            AND current_verification.credential_verifier_key_status='active'
            AND current_verification.report_validity_status='current'
            AND current_verification.revocation_status='none')))
  AND EXISTS (SELECT 1 FROM compute_external_pool_adapter_upstream_transport_targets target
    WHERE target.target_id=NEW.upstream_transport_target_id AND target.target_digest=NEW.upstream_transport_target_digest
      AND target.provider_id=NEW.provider_id AND target.route_adapter_projection_id=NEW.adapter_id
      AND target.implementation_digest=NEW.adapter_implementation_digest AND target.profile_digest=NEW.runtime_launch_profile_digest
      AND target.recorded_at<=NEW.started_at
      AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_upstream_transport_targets successor WHERE successor.predecessor_target_id=target.target_id)
      AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_upstream_transport_target_revocations revocation WHERE revocation.target_id=target.target_id))
  AND EXISTS (SELECT 1 FROM compute_external_pool_adapter_supervisor_session_policy_companion_current companion
    WHERE companion.companion_digest=NEW.supervisor_session_policy_companion_digest
      AND companion.supervisor_session_policy_digest=NEW.supervisor_session_policy_digest
      AND companion.profile_digest=NEW.runtime_launch_profile_digest
      AND companion.target_id=NEW.upstream_transport_target_id
      AND companion.target_digest=NEW.upstream_transport_target_digest
      AND companion.provider_id=NEW.provider_id
      AND companion.route_adapter_projection_id=NEW.adapter_id
      AND companion.implementation_digest=NEW.adapter_implementation_digest
      AND companion.recorded_at<=NEW.started_at
      AND companion.current_status='supervisor_session_policy_companion_current_inert'
      AND companion.head_status='head' AND companion.revocation_status='unrevoked'
      AND companion.target_status='upstream_transport_target_current_inert'
      AND companion.profile_status='launch_profile_current_inert'
      AND companion.policy_status='server_policy_current')
  AND EXISTS (SELECT 1 FROM compute_external_pool_adapter_task_protocol_conformance_run_receipts run
    WHERE run.run_receipt_id=NEW.task_protocol_conformance_run_receipt_id AND run.run_receipt_digest=NEW.task_protocol_conformance_run_receipt_digest
      AND run.supervisor_session_policy_digest=NEW.supervisor_session_policy_digest
      AND run.task_protocol_profile_digest=NEW.task_protocol_profile_digest AND run.launch_image_sha256=NEW.launch_image_sha256
      AND run.implementation_digest=NEW.adapter_implementation_digest
      AND EXISTS (SELECT 1 FROM compute_external_pool_adapter_supervisor_session_policy_companion_current companion
        WHERE companion.companion_digest=NEW.supervisor_session_policy_companion_digest
          AND companion.registry_release_id=run.registry_release_id
          AND companion.registry_release_digest=run.registry_release_digest
          AND companion.logical_adapter_id=run.adapter_id
          AND companion.release_version=run.release_version
          AND companion.implementation_digest=run.implementation_digest
          AND companion.current_status='supervisor_session_policy_companion_current_inert')
      AND run.recorded_at<=NEW.started_at AND NEW.started_at<run.expires_at
      AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_task_protocol_conformance_run_receipts successor WHERE successor.predecessor_run_receipt_id=run.run_receipt_id)
      AND NOT EXISTS (SELECT 1 FROM compute_external_pool_adapter_task_protocol_conformance_revocations revocation WHERE revocation.run_receipt_id=run.run_receipt_id))
)
BEGIN SELECT RAISE(ABORT,'V273 exchange attempt lacks exact current durable authority'); END;

CREATE TRIGGER IF NOT EXISTS v273_task_exchange_attempt_poll_source
BEFORE INSERT ON compute_external_pool_adapter_task_exchange_attempts
WHEN (NEW.source_kind='reconcile_poll' AND NOT EXISTS (
  SELECT 1 FROM compute_external_pool_adapter_task_reconcile_polls poll
   WHERE poll.reconcile_poll_id=NEW.source_id AND poll.reconcile_poll_digest=NEW.source_digest AND poll.claim_status='claimed'
     AND poll.not_before<=NEW.started_at AND NEW.started_at<poll.not_after AND NEW.started_at<poll.claim_expires_at
     AND poll.command_id=NEW.command_id AND poll.command_digest=NEW.command_digest AND poll.outbox_id=NEW.outbox_id AND poll.outbox_digest=NEW.outbox_digest
     AND poll.send_attempt_id=NEW.send_attempt_id AND poll.send_attempt_digest=NEW.send_attempt_digest AND poll.route_authorization_id=NEW.route_authorization_id
     AND poll.route_authorization_digest=NEW.route_authorization_digest AND poll.executor_binding_digest=NEW.executor_binding_digest
     AND poll.fencing_generation=NEW.fencing_generation AND poll.fence_digest=NEW.fence_digest AND poll.request_digest=NEW.request_digest))
OR (NEW.source_kind='event_poll' AND NOT EXISTS (
  SELECT 1 FROM compute_external_pool_adapter_task_event_polls poll
   WHERE poll.event_poll_id=NEW.source_id AND poll.event_poll_digest=NEW.source_digest AND poll.claim_status='claimed'
     AND poll.not_before<=NEW.started_at AND NEW.started_at<poll.not_after AND NEW.started_at<poll.claim_expires_at
     AND poll.command_id=NEW.command_id AND poll.command_digest=NEW.command_digest AND poll.outbox_id=NEW.outbox_id AND poll.outbox_digest=NEW.outbox_digest
     AND poll.send_attempt_id=NEW.send_attempt_id AND poll.send_attempt_digest=NEW.send_attempt_digest AND poll.route_authorization_id=NEW.route_authorization_id
     AND poll.route_authorization_digest=NEW.route_authorization_digest AND poll.executor_binding_digest=NEW.executor_binding_digest
     AND poll.fencing_generation=NEW.fencing_generation AND poll.fence_digest=NEW.fence_digest AND poll.request_digest=NEW.request_digest))
BEGIN SELECT RAISE(ABORT,'V273 exchange attempt lacks exact claimed poll source'); END;
"#;

const POLL_SOURCE: &str = r#"
CREATE TRIGGER IF NOT EXISTS v273_task_reconcile_poll_source
BEFORE INSERT ON compute_external_pool_adapter_task_reconcile_polls
WHEN NOT (
  (NEW.predecessor_reconcile_poll_id IS NULL
   AND NEW.poll_ordinal=1
   AND (EXISTS (
     SELECT 1 FROM compute_external_pool_adapter_task_exchange_attempts attempt
      WHERE attempt.exchange_attempt_id=NEW.uncertain_exchange_attempt_id
        AND attempt.exchange_attempt_digest=NEW.uncertain_exchange_attempt_digest
        AND attempt.operation_kind IN ('prepare','idempotent_commit','cancel_no_start')
        AND attempt.source_kind='start_outbox_send_attempt'
        AND attempt.command_id=NEW.command_id AND attempt.command_digest=NEW.command_digest
        AND attempt.outbox_id=NEW.outbox_id AND attempt.outbox_digest=NEW.outbox_digest
        AND attempt.send_attempt_id=NEW.send_attempt_id AND attempt.send_attempt_digest=NEW.send_attempt_digest
        AND attempt.route_authorization_id=NEW.route_authorization_id
        AND attempt.route_authorization_digest=NEW.route_authorization_digest
        AND attempt.executor_binding_digest=NEW.executor_binding_digest
        AND attempt.fencing_generation=NEW.fencing_generation AND attempt.fence_digest=NEW.fence_digest
        AND NOT EXISTS (
          SELECT 1 FROM compute_external_pool_adapter_task_exchange_receipts receipt
           WHERE receipt.exchange_attempt_id=attempt.exchange_attempt_id
             AND receipt.exchange_attempt_digest=attempt.exchange_attempt_digest)
        AND ((attempt.operation_kind='prepare'
              AND NEW.remote_execution_id IS NULL
              AND NEW.remote_execution_state='unknown'
              AND NEW.authenticated_subject_sha256 IS NULL)
          OR (attempt.operation_kind IN ('idempotent_commit','cancel_no_start')
              AND NEW.remote_execution_id IS NOT NULL
              AND NEW.remote_execution_state='prepared'
              AND NEW.authenticated_subject_sha256 IS NOT NULL
              AND EXISTS (
                SELECT 1 FROM compute_external_pool_adapter_task_exchange_receipts prepared
                 WHERE prepared.operation_kind='prepare'
                   AND prepared.source_kind='start_outbox_send_attempt'
                   AND prepared.command_id=NEW.command_id
                    AND prepared.command_digest=NEW.command_digest
                    AND prepared.provider_id=attempt.provider_id
                    AND prepared.route_authorization_id=NEW.route_authorization_id
                    AND prepared.route_authorization_digest=NEW.route_authorization_digest
                    AND prepared.executor_binding_digest=NEW.executor_binding_digest
                    AND prepared.fencing_generation=NEW.fencing_generation
                    AND prepared.fence_digest=NEW.fence_digest
                    AND prepared.semantic_observation_sha256=NEW.authenticated_subject_sha256))))
     OR (NEW.remote_execution_id IS NOT NULL
         AND NEW.remote_execution_state='prepared'
         AND NEW.authenticated_subject_sha256 IS NOT NULL
         AND EXISTS (
           SELECT 1 FROM compute_external_pool_adapter_task_exchange_attempts attempt
           JOIN compute_external_pool_adapter_task_exchange_receipts receipt
             ON receipt.exchange_attempt_id=attempt.exchange_attempt_id
            AND receipt.exchange_attempt_digest=attempt.exchange_attempt_digest
            WHERE attempt.exchange_attempt_id=NEW.uncertain_exchange_attempt_id
              AND attempt.exchange_attempt_digest=NEW.uncertain_exchange_attempt_digest
              AND attempt.operation_kind='cancel_no_start'
              AND attempt.source_kind='start_outbox_send_attempt'
              AND attempt.command_id=NEW.command_id AND attempt.command_digest=NEW.command_digest
              AND attempt.outbox_id=NEW.outbox_id AND attempt.outbox_digest=NEW.outbox_digest
              AND attempt.send_attempt_id=NEW.send_attempt_id AND attempt.send_attempt_digest=NEW.send_attempt_digest
              AND attempt.route_authorization_id=NEW.route_authorization_id
              AND attempt.route_authorization_digest=NEW.route_authorization_digest
              AND attempt.executor_binding_digest=NEW.executor_binding_digest
              AND attempt.fencing_generation=NEW.fencing_generation AND attempt.fence_digest=NEW.fence_digest
              AND receipt.operation_kind='cancel_no_start'
              AND receipt.source_kind='start_outbox_send_attempt'
              AND receipt.source_id=NEW.send_attempt_id AND receipt.source_digest=NEW.send_attempt_digest
              AND receipt.command_id=NEW.command_id AND receipt.command_digest=NEW.command_digest
              AND receipt.outbox_id=NEW.outbox_id AND receipt.outbox_digest=NEW.outbox_digest
              AND receipt.route_authorization_id=NEW.route_authorization_id
              AND receipt.route_authorization_digest=NEW.route_authorization_digest
              AND receipt.executor_binding_digest=NEW.executor_binding_digest
              AND receipt.fencing_generation=NEW.fencing_generation AND receipt.fence_digest=NEW.fence_digest
              AND receipt.semantic_observation_sha256=NEW.authenticated_subject_sha256))))
  OR
  (NEW.predecessor_reconcile_poll_id IS NOT NULL
   AND EXISTS (
     SELECT 1 FROM compute_external_pool_adapter_task_reconcile_polls predecessor
     JOIN compute_external_pool_adapter_task_exchange_attempts attempt
       ON attempt.exchange_attempt_id=NEW.uncertain_exchange_attempt_id
      AND attempt.exchange_attempt_digest=NEW.uncertain_exchange_attempt_digest
      AND attempt.operation_kind='reconcile'
      AND attempt.source_kind='reconcile_poll'
      AND attempt.source_id=predecessor.reconcile_poll_id
      AND attempt.source_digest=predecessor.reconcile_poll_digest
      WHERE predecessor.reconcile_poll_id=NEW.predecessor_reconcile_poll_id
        AND predecessor.reconcile_poll_digest=NEW.predecessor_reconcile_poll_digest
        AND predecessor.poll_ordinal+1=NEW.poll_ordinal
        AND predecessor.command_id=NEW.command_id AND predecessor.command_digest=NEW.command_digest
        AND predecessor.outbox_id=NEW.outbox_id AND predecessor.outbox_digest=NEW.outbox_digest
        AND predecessor.send_attempt_id=NEW.send_attempt_id AND predecessor.send_attempt_digest=NEW.send_attempt_digest
        AND predecessor.route_authorization_id=NEW.route_authorization_id
        AND predecessor.route_authorization_digest=NEW.route_authorization_digest
        AND predecessor.executor_binding_digest=NEW.executor_binding_digest
        AND predecessor.fencing_generation=NEW.fencing_generation AND predecessor.fence_digest=NEW.fence_digest
        AND attempt.command_id=NEW.command_id AND attempt.command_digest=NEW.command_digest
        AND attempt.outbox_id=NEW.outbox_id AND attempt.outbox_digest=NEW.outbox_digest
        AND attempt.send_attempt_id=NEW.send_attempt_id AND attempt.send_attempt_digest=NEW.send_attempt_digest
        AND attempt.route_authorization_id=NEW.route_authorization_id
        AND attempt.route_authorization_digest=NEW.route_authorization_digest
        AND attempt.executor_binding_digest=NEW.executor_binding_digest
        AND attempt.fencing_generation=NEW.fencing_generation AND attempt.fence_digest=NEW.fence_digest
        AND ((predecessor.claim_status='in_flight_unknown'
              AND predecessor.remote_execution_id IS NEW.remote_execution_id
              AND predecessor.remote_identity_digest=NEW.remote_identity_digest
              AND predecessor.remote_execution_state=NEW.remote_execution_state
              AND predecessor.authenticated_subject_sha256 IS NEW.authenticated_subject_sha256
              AND NOT EXISTS (
                SELECT 1 FROM compute_external_pool_adapter_task_exchange_receipts receipt
                 WHERE receipt.exchange_attempt_id=attempt.exchange_attempt_id
                   AND receipt.exchange_attempt_digest=attempt.exchange_attempt_digest))
          OR (predecessor.claim_status='delivery_observed'
              AND NEW.remote_execution_state='unknown'
              AND NEW.authenticated_subject_sha256 IS NOT NULL
              AND EXISTS (
                SELECT 1 FROM compute_external_pool_adapter_task_exchange_receipts receipt
                 WHERE receipt.exchange_attempt_id=attempt.exchange_attempt_id
                   AND receipt.exchange_attempt_digest=attempt.exchange_attempt_digest
                   AND receipt.operation_kind='reconcile'
                   AND receipt.source_kind='reconcile_poll'
                   AND receipt.source_id=predecessor.reconcile_poll_id
                   AND receipt.source_digest=predecessor.reconcile_poll_digest
                   AND receipt.command_id=NEW.command_id AND receipt.command_digest=NEW.command_digest
                   AND receipt.outbox_id=NEW.outbox_id AND receipt.outbox_digest=NEW.outbox_digest
                   AND receipt.send_attempt_id=NEW.send_attempt_id AND receipt.send_attempt_digest=NEW.send_attempt_digest
                   AND receipt.route_authorization_id=NEW.route_authorization_id
                   AND receipt.route_authorization_digest=NEW.route_authorization_digest
                   AND receipt.executor_binding_digest=NEW.executor_binding_digest
                   AND receipt.fencing_generation=NEW.fencing_generation AND receipt.fence_digest=NEW.fence_digest
                   AND receipt.semantic_observation_sha256=NEW.authenticated_subject_sha256))))))
BEGIN SELECT RAISE(ABORT,'V273 reconcile poll lacks exact remote-unknown lineage'); END;

CREATE TRIGGER IF NOT EXISTS v273_task_event_poll_source
BEFORE INSERT ON compute_external_pool_adapter_task_event_polls
WHEN NOT (
  (NEW.predecessor_event_poll_id IS NULL
   AND NEW.poll_ordinal=1
   AND NEW.requested_remote_sequence=0
   AND EXISTS (
     SELECT 1 FROM compute_external_pool_adapter_task_exchange_receipts receipt
      WHERE receipt.exchange_receipt_id=NEW.source_exchange_receipt_id
        AND receipt.exchange_receipt_digest=NEW.source_exchange_receipt_digest
        AND receipt.operation_kind IN ('idempotent_commit','reconcile')
        AND receipt.semantic_observation_sha256=NEW.authenticated_subject_sha256
        AND receipt.command_id=NEW.command_id AND receipt.command_digest=NEW.command_digest
        AND receipt.outbox_id=NEW.outbox_id AND receipt.outbox_digest=NEW.outbox_digest
        AND receipt.send_attempt_id=NEW.send_attempt_id AND receipt.send_attempt_digest=NEW.send_attempt_digest
        AND receipt.route_authorization_id=NEW.route_authorization_id
        AND receipt.route_authorization_digest=NEW.route_authorization_digest
        AND receipt.executor_binding_digest=NEW.executor_binding_digest
        AND receipt.fencing_generation=NEW.fencing_generation AND receipt.fence_digest=NEW.fence_digest))
  OR
  (NEW.predecessor_event_poll_id IS NOT NULL
   AND EXISTS (
     SELECT 1 FROM compute_external_pool_adapter_task_event_polls predecessor
      WHERE predecessor.event_poll_id=NEW.predecessor_event_poll_id
        AND predecessor.event_poll_digest=NEW.predecessor_event_poll_digest
        AND predecessor.poll_ordinal+1=NEW.poll_ordinal
        AND predecessor.source_exchange_receipt_id=NEW.source_exchange_receipt_id
        AND predecessor.source_exchange_receipt_digest=NEW.source_exchange_receipt_digest
        AND predecessor.command_id=NEW.command_id AND predecessor.command_digest=NEW.command_digest
        AND predecessor.outbox_id=NEW.outbox_id AND predecessor.outbox_digest=NEW.outbox_digest
        AND predecessor.send_attempt_id=NEW.send_attempt_id AND predecessor.send_attempt_digest=NEW.send_attempt_digest
        AND predecessor.route_authorization_id=NEW.route_authorization_id
        AND predecessor.route_authorization_digest=NEW.route_authorization_digest
        AND predecessor.executor_binding_digest=NEW.executor_binding_digest
        AND predecessor.fencing_generation=NEW.fencing_generation AND predecessor.fence_digest=NEW.fence_digest
        AND ((predecessor.claim_status='in_flight_unknown'
              AND predecessor.remote_execution_id=NEW.remote_execution_id
              AND predecessor.remote_identity_digest=NEW.remote_identity_digest
              AND predecessor.remote_execution_state=NEW.remote_execution_state
              AND predecessor.authenticated_subject_sha256=NEW.authenticated_subject_sha256
              AND predecessor.requested_remote_sequence=NEW.requested_remote_sequence
              AND predecessor.requested_previous_event_root IS NEW.requested_previous_event_root
              AND predecessor.requested_cursor_digest=NEW.requested_cursor_digest)
          OR (predecessor.claim_status='delivery_observed'
              AND EXISTS (
                SELECT 1 FROM compute_external_pool_adapter_task_event_batches batch
                 WHERE batch.event_poll_id=predecessor.event_poll_id
                   AND batch.event_poll_digest=predecessor.event_poll_digest
                   AND batch.remote_execution_id=NEW.remote_execution_id
                   AND batch.remote_identity_digest=NEW.remote_identity_digest
                   AND batch.remote_execution_state=NEW.remote_execution_state
                   AND batch.cursor_after_remote_sequence=NEW.requested_remote_sequence
                   AND batch.cursor_after_previous_event_root IS NEW.requested_previous_event_root
                   AND batch.cursor_after_digest=NEW.requested_cursor_digest))))))
BEGIN SELECT RAISE(ABORT,'V273 event poll lacks exact committed cursor lineage'); END;
"#;
