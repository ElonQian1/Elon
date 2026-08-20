//! Preserve cleanup-only cancel and poll reachability after runtime-root expiry.

use anyhow::{ensure, Result};
use rusqlite::Connection;

const TRIGGER: &str = "v273_task_exchange_attempt_exact_authority";
const MARKER: &str = "compute_external_pool_adapter_route_renewal_receipts";
const PENDING_UDF: &str = "elon_v278_external_pool_adapter_task_reachability_pending_plan_matches";

pub(super) fn install(connection: &Connection) -> Result<()> {
    let sql = trigger_sql(connection)?;
    if sql.contains(MARKER) {
        return ensure_installed(&sql);
    }
    ensure!(
        sql.contains("WHEN NOT (")
            && sql.contains("compute_external_pool_adapter_task_protocol_conformance_run_receipts")
            && sql.contains("NEW.started_at<run.expires_at")
            && sql.contains("BEGIN SELECT RAISE"),
        "V278 historical poll predecessor authority guard drifted"
    );
    let begin = sql
        .rfind("BEGIN")
        .ok_or_else(|| anyhow::anyhow!("V278 historical poll authority guard lost BEGIN"))?;
    let outer_close = sql[..begin]
        .rfind(')')
        .ok_or_else(|| anyhow::anyhow!("V278 historical poll authority guard lost outer close"))?;
    ensure!(
        sql[outer_close + 1..begin].trim().is_empty(),
        "V278 historical poll authority guard outer shape drifted"
    );
    let replacement = format!(
        "{}\n  OR (\n{}\n  ){}",
        &sql[..outer_close],
        HISTORICAL_POLL_SOURCE,
        &sql[outer_close..]
    );
    connection.execute_batch(&format!(
        "DROP TRIGGER IF EXISTS {TRIGGER};\n{replacement};"
    ))?;
    ensure_installed(&trigger_sql(connection)?)
}

fn ensure_installed(sql: &str) -> Result<()> {
    ensure!(
        sql.contains("NEW.started_at<run.expires_at")
            && sql.contains("original.task_protocol_conformance_run_receipt_digest=NEW.task_protocol_conformance_run_receipt_digest")
            && sql.contains("cancel_outbox.subject_outbox_id=original.outbox_id")
            && sql.contains("successor.successor_sequence=1")
            && sql.matches(MARKER).count() == 1,
        "V278 historical poll authority branch is incomplete"
    );
    ensure!(
        !sql.contains(PENDING_UDF),
        "V278 historical poll authority must not double-consume the exchange pending plan"
    );
    Ok(())
}

fn trigger_sql(connection: &Connection) -> Result<String> {
    connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='trigger' AND name=?1",
            [TRIGGER],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

const HISTORICAL_POLL_SOURCE: &str = r#"    ((((NEW.operation_kind='reconcile' AND NEW.source_kind='reconcile_poll')
       OR (NEW.operation_kind='authenticated_events' AND NEW.source_kind='event_poll'))
      AND EXISTS (
      SELECT 1
        FROM compute_external_pool_adapter_task_exchange_attempts original
       WHERE original.operation_kind IN ('prepare','idempotent_commit','cancel_no_start')
         AND original.source_kind='start_outbox_send_attempt'
         AND original.source_id=original.send_attempt_id
         AND original.source_digest=original.send_attempt_digest
         AND original.send_attempt_id=NEW.send_attempt_id
         AND original.send_attempt_digest=NEW.send_attempt_digest
         AND original.provider_id=NEW.provider_id
         AND original.adapter_id=NEW.adapter_id
         AND original.adapter_revision=NEW.adapter_revision
         AND original.adapter_registry_digest=NEW.adapter_registry_digest
         AND original.adapter_implementation_digest=NEW.adapter_implementation_digest
         AND original.command_id=NEW.command_id
         AND original.command_digest=NEW.command_digest
         AND original.outbox_id=NEW.outbox_id
         AND original.outbox_digest=NEW.outbox_digest
         AND original.route_authorization_id=NEW.route_authorization_id
         AND original.route_authorization_digest=NEW.route_authorization_digest
         AND original.route_credential_id=NEW.route_credential_id
         AND original.route_credential_revision=NEW.route_credential_revision
         AND original.route_credential_digest=NEW.route_credential_digest
         AND original.credential_verification_receipt_id=NEW.credential_verification_receipt_id
         AND original.credential_verification_receipt_digest=NEW.credential_verification_receipt_digest
         AND original.credential_verifier_id=NEW.credential_verifier_id
         AND original.credential_verifier_revision=NEW.credential_verifier_revision
         AND original.credential_verifier_digest=NEW.credential_verifier_digest
         AND original.executor_binding_digest=NEW.executor_binding_digest
         AND original.fencing_generation=NEW.fencing_generation
         AND original.fence_digest=NEW.fence_digest
         AND original.supervisor_session_policy_digest=NEW.supervisor_session_policy_digest
         AND original.runtime_launch_profile_digest=NEW.runtime_launch_profile_digest
         AND original.task_protocol_profile_digest=NEW.task_protocol_profile_digest
         AND original.upstream_transport_target_id=NEW.upstream_transport_target_id
         AND original.upstream_transport_target_digest=NEW.upstream_transport_target_digest
         AND original.supervisor_session_policy_companion_digest=NEW.supervisor_session_policy_companion_digest
         AND original.launch_image_sha256=NEW.launch_image_sha256
         AND original.ephemeral_task_secret_delivery_root=NEW.ephemeral_task_secret_delivery_root
         AND original.task_protocol_conformance_run_receipt_id=NEW.task_protocol_conformance_run_receipt_id
         AND original.task_protocol_conformance_run_receipt_digest=NEW.task_protocol_conformance_run_receipt_digest
         AND original.session_roots_digest=NEW.session_roots_digest
         AND original.session_transcript_digest=NEW.session_transcript_digest
         AND original.started_at<=NEW.started_at))
    OR (NEW.operation_kind='cancel_no_start'
      AND NEW.source_kind='start_outbox_send_attempt'
      AND NEW.source_id=NEW.send_attempt_id
      AND NEW.source_digest=NEW.send_attempt_digest
      AND EXISTS (
        SELECT 1
          FROM compute_attempt_start_send_attempts cancel_send
          JOIN compute_attempt_start_outbox cancel_outbox
            ON cancel_outbox.outbox_id=cancel_send.outbox_id
           AND cancel_outbox.outbox_digest=cancel_send.outbox_digest
          JOIN compute_external_pool_adapter_task_exchange_attempts original
            ON original.outbox_id=cancel_outbox.subject_outbox_id
           AND original.command_id=cancel_outbox.command_id
           AND original.command_digest=cancel_outbox.command_digest
         WHERE cancel_send.send_attempt_id=NEW.send_attempt_id
           AND cancel_send.send_attempt_digest=NEW.send_attempt_digest
           AND cancel_send.outbox_id=NEW.outbox_id
           AND cancel_send.outbox_digest=NEW.outbox_digest
           AND cancel_send.command_id=NEW.command_id
           AND cancel_send.command_digest=NEW.command_digest
           AND cancel_send.operation_kind='cancel'
           AND cancel_send.started_at=NEW.started_at
           AND cancel_send.route_authorization_id=NEW.route_authorization_id
           AND cancel_send.route_authorization_digest=NEW.route_authorization_digest
           AND cancel_send.request_digest=NEW.request_digest
           AND cancel_outbox.operation_kind='cancel'
           AND cancel_outbox.command_id=NEW.command_id
           AND cancel_outbox.command_digest=NEW.command_digest
           AND cancel_outbox.provider_id=NEW.provider_id
           AND cancel_outbox.adapter_id=NEW.adapter_id
           AND cancel_outbox.route_authorization_id=NEW.route_authorization_id
           AND cancel_outbox.route_authorization_digest=NEW.route_authorization_digest
           AND original.operation_kind='prepare'
           AND original.source_kind='start_outbox_send_attempt'
           AND original.source_id=original.send_attempt_id
           AND original.source_digest=original.send_attempt_digest
           AND original.provider_id=NEW.provider_id
           AND original.adapter_id=NEW.adapter_id
           AND original.adapter_revision=NEW.adapter_revision
           AND original.adapter_registry_digest=NEW.adapter_registry_digest
           AND original.adapter_implementation_digest=NEW.adapter_implementation_digest
           AND original.command_id=NEW.command_id
           AND original.command_digest=NEW.command_digest
           AND original.route_authorization_id=NEW.route_authorization_id
           AND original.route_authorization_digest=NEW.route_authorization_digest
           AND original.route_credential_id=NEW.route_credential_id
           AND original.route_credential_revision=NEW.route_credential_revision
           AND original.route_credential_digest=NEW.route_credential_digest
           AND original.credential_verification_receipt_id=NEW.credential_verification_receipt_id
           AND original.credential_verification_receipt_digest=NEW.credential_verification_receipt_digest
           AND original.credential_verifier_id=NEW.credential_verifier_id
           AND original.credential_verifier_revision=NEW.credential_verifier_revision
           AND original.credential_verifier_digest=NEW.credential_verifier_digest
           AND original.executor_binding_digest=NEW.executor_binding_digest
           AND original.fencing_generation=NEW.fencing_generation
           AND original.fence_digest=NEW.fence_digest
           AND original.supervisor_session_policy_digest=NEW.supervisor_session_policy_digest
           AND original.runtime_launch_profile_digest=NEW.runtime_launch_profile_digest
           AND original.task_protocol_profile_digest=NEW.task_protocol_profile_digest
           AND original.upstream_transport_target_id=NEW.upstream_transport_target_id
           AND original.upstream_transport_target_digest=NEW.upstream_transport_target_digest
           AND original.supervisor_session_policy_companion_digest=NEW.supervisor_session_policy_companion_digest
           AND original.launch_image_sha256=NEW.launch_image_sha256
           AND original.ephemeral_task_secret_delivery_root=NEW.ephemeral_task_secret_delivery_root
           AND original.task_protocol_conformance_run_receipt_id=NEW.task_protocol_conformance_run_receipt_id
           AND original.task_protocol_conformance_run_receipt_digest=NEW.task_protocol_conformance_run_receipt_digest
           AND original.session_roots_digest=NEW.session_roots_digest
           AND original.session_transcript_digest=NEW.session_transcript_digest
           AND original.started_at<=NEW.started_at)))
    AND ((SELECT COUNT(*)
            FROM compute_external_pool_adapter_route_renewal_receipts renewal
            JOIN compute_external_pool_adapter_atomic_activation_receipts activation
              ON activation.activation_receipt_id=renewal.activation_receipt_id
             AND activation.activation_receipt_digest=renewal.activation_receipt_digest
             AND activation.activation_root_digest=renewal.activation_root_digest
            JOIN compute_external_pool_adapter_provider_active_successor_receipts successor
              ON successor.active_successor_receipt_id=renewal.activation_genesis_successor_receipt_id
             AND successor.receipt_digest=renewal.activation_genesis_successor_receipt_digest
             AND successor.successor_sequence=1
             AND successor.activation_witness_id=activation.activation_receipt_id
             AND successor.activation_witness_digest=activation.activation_receipt_digest
             AND successor.activation_root_digest=activation.activation_root_digest
             JOIN compute_providers provider
               ON provider.provider_id=renewal.active_provider_id
             JOIN compute_provider_versions provider_version
               ON provider_version.provider_id=provider.provider_id
              AND provider_version.policy_revision=provider.current_policy_revision
              AND provider_version.provider_digest=provider.current_provider_digest
           WHERE renewal.route_authorization_id=NEW.route_authorization_id
             AND renewal.route_authorization_digest=NEW.route_authorization_digest
             AND renewal.route_credential_id=NEW.route_credential_id
             AND renewal.route_credential_revision=NEW.route_credential_revision
             AND renewal.route_credential_digest=NEW.route_credential_digest
             AND renewal.active_provider_id=NEW.provider_id
             AND renewal.route_adapter_projection_id=NEW.adapter_id
             AND renewal.route_adapter_revision=NEW.adapter_revision
             AND renewal.route_adapter_digest=NEW.adapter_registry_digest
              AND renewal.stable_executor_binding_digest=NEW.executor_binding_digest
              AND provider.provider_kind='external_pool' AND provider.status='active'
              AND provider.owner_account_id=json_extract(successor.activation_root_json,'$.activation_root.provider_owner_account_id')
              AND provider.current_policy_revision>=activation.target_active_provider_policy_revision
              AND provider.created_at=json_extract(activation.source_registering_provider_json,'$.created_at')
              AND json_extract(provider_version.provider_json,'$.provider_id')=provider.provider_id
              AND json_extract(provider_version.provider_json,'$.provider_kind')=provider.provider_kind
              AND json_extract(provider_version.provider_json,'$.owner_account_id')=provider.owner_account_id
              AND json_extract(provider_version.provider_json,'$.settlement_account_id') IS provider.settlement_account_id
              AND json_extract(provider_version.provider_json,'$.display_name')=provider.display_name
              AND json_extract(provider_version.provider_json,'$.status')=provider.status
              AND json_extract(provider_version.provider_json,'$.trust_tier')=provider.trust_tier
              AND json_extract(provider_version.provider_json,'$.home_region') IS provider.home_region
              AND json_extract(provider_version.provider_json,'$.policy_revision')=provider.current_policy_revision
              AND json_extract(provider_version.provider_json,'$.created_at')=provider.created_at
              AND json_extract(provider_version.provider_json,'$.updated_at')=provider.updated_at
              AND json_extract(provider_version.provider_json,'$.adapter.adapter_id')=activation.route_adapter_projection_id
              AND json_extract(provider_version.provider_json,'$.adapter.adapter_version')=json_extract(activation.target_active_provider_json,'$.adapter.adapter_version')
              AND json_extract(provider_version.provider_json,'$.adapter.config_revision')=json_extract(activation.target_active_provider_json,'$.adapter.config_revision')
              AND json_extract(provider_version.provider_json,'$.adapter.config_digest')=json_extract(activation.target_active_provider_json,'$.adapter.config_digest')
              AND (provider.current_policy_revision<>activation.target_active_provider_policy_revision
                OR (provider.current_provider_digest=activation.target_active_provider_digest
                  AND provider_version.provider_json=activation.target_active_provider_json))
              AND NEW.started_at<renewal.cleanup_expires_at)
       + (SELECT COUNT(*)
            FROM compute_external_pool_adapter_atomic_activation_receipts activation
            JOIN compute_external_pool_adapter_provider_active_successor_receipts successor
              ON successor.successor_sequence=1
             AND successor.activation_witness_id=activation.activation_receipt_id
             AND successor.activation_witness_digest=activation.activation_receipt_digest
             AND successor.activation_root_digest=activation.activation_root_digest
            JOIN compute_route_authorization_receipts authorization
              ON authorization.route_authorization_id=activation.route_authorization_id
             AND authorization.route_authorization_revision=activation.route_authorization_revision
             AND authorization.route_authorization_digest=activation.route_authorization_digest
             AND authorization.credential_id=activation.route_credential_id
             AND authorization.credential_revision=activation.route_credential_revision
             AND authorization.credential_digest=activation.route_credential_digest
             JOIN compute_providers provider
               ON provider.provider_id=activation.target_active_provider_id
             JOIN compute_provider_versions provider_version
               ON provider_version.provider_id=provider.provider_id
              AND provider_version.policy_revision=provider.current_policy_revision
              AND provider_version.provider_digest=provider.current_provider_digest
           WHERE activation.route_authorization_id=NEW.route_authorization_id
             AND activation.route_authorization_digest=NEW.route_authorization_digest
             AND activation.route_credential_id=NEW.route_credential_id
             AND activation.route_credential_revision=NEW.route_credential_revision
             AND activation.route_credential_digest=NEW.route_credential_digest
             AND activation.target_active_provider_id=NEW.provider_id
             AND activation.route_adapter_projection_id=NEW.adapter_id
             AND activation.route_adapter_revision=NEW.adapter_revision
             AND activation.route_adapter_digest=NEW.adapter_registry_digest
              AND activation.stable_executor_binding_digest=NEW.executor_binding_digest
              AND provider.provider_kind='external_pool' AND provider.status='active'
              AND provider.owner_account_id=json_extract(successor.activation_root_json,'$.activation_root.provider_owner_account_id')
              AND provider.current_policy_revision>=activation.target_active_provider_policy_revision
              AND provider.created_at=json_extract(activation.source_registering_provider_json,'$.created_at')
              AND json_extract(provider_version.provider_json,'$.provider_id')=provider.provider_id
              AND json_extract(provider_version.provider_json,'$.provider_kind')=provider.provider_kind
              AND json_extract(provider_version.provider_json,'$.owner_account_id')=provider.owner_account_id
              AND json_extract(provider_version.provider_json,'$.settlement_account_id') IS provider.settlement_account_id
              AND json_extract(provider_version.provider_json,'$.display_name')=provider.display_name
              AND json_extract(provider_version.provider_json,'$.status')=provider.status
              AND json_extract(provider_version.provider_json,'$.trust_tier')=provider.trust_tier
              AND json_extract(provider_version.provider_json,'$.home_region') IS provider.home_region
              AND json_extract(provider_version.provider_json,'$.policy_revision')=provider.current_policy_revision
              AND json_extract(provider_version.provider_json,'$.created_at')=provider.created_at
              AND json_extract(provider_version.provider_json,'$.updated_at')=provider.updated_at
              AND json_extract(provider_version.provider_json,'$.adapter.adapter_id')=activation.route_adapter_projection_id
              AND json_extract(provider_version.provider_json,'$.adapter.adapter_version')=json_extract(activation.target_active_provider_json,'$.adapter.adapter_version')
              AND json_extract(provider_version.provider_json,'$.adapter.config_revision')=json_extract(activation.target_active_provider_json,'$.adapter.config_revision')
              AND json_extract(provider_version.provider_json,'$.adapter.config_digest')=json_extract(activation.target_active_provider_json,'$.adapter.config_digest')
              AND (provider.current_policy_revision<>activation.target_active_provider_policy_revision
                OR (provider.current_provider_digest=activation.target_active_provider_digest
                  AND provider_version.provider_json=activation.target_active_provider_json))
              AND NEW.started_at<authorization.cleanup_expires_at))=1"#;
