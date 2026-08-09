//! Redundant-column projections for the append-only Planning Snapshot V2 cloud ledger.

use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install_projection_triggers(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TRIGGER IF NOT EXISTS trg_compute_plugin_plan_v2_request_projection
           BEFORE INSERT ON node_compute_plugin_install_plan_planning_delivery_events_v2
           WHEN NOT json_valid(NEW.request_json) OR (
             json_extract(NEW.request_json, '$.schema') IS NOT NEW.request_schema
             OR json_extract(NEW.request_json, '$.planning_delivery_id') IS NOT NEW.planning_delivery_id
             OR json_extract(NEW.request_json, '$.cloud_session_id') IS NOT NEW.cloud_session_id
             OR json_extract(NEW.request_json, '$.source_sharing_delivery_id') IS NOT NEW.source_sharing_delivery_id
             OR json_extract(NEW.request_json, '$.source_preparation_observation_id') IS NOT NEW.source_preparation_observation_id
             OR json_extract(NEW.request_json, '$.source_preparation_request_digest') IS NOT NEW.source_preparation_request_digest
             OR json_extract(NEW.request_json, '$.source_bootstrap_instance_id') IS NOT NEW.source_bootstrap_instance_id
             OR json_extract(NEW.request_json, '$.source_configuration_generation') IS NOT NEW.source_configuration_generation
             OR json_extract(NEW.request_json, '$.source_cancellation_generation') IS NOT NEW.source_cancellation_generation
             OR json_extract(NEW.request_json, '$.consent_receipt_id') IS NOT NEW.consent_receipt_id
             OR json_extract(NEW.request_json, '$.request.schema') IS NOT 'elon.compute_plugin.install_plan_planning_snapshot_request.v2'
             OR json_extract(NEW.request_json, '$.request.preparation_id') IS NOT NEW.source_preparation_id
             OR json_extract(NEW.request_json, '$.request.cloud_session_id') IS NOT NEW.cloud_session_id
             OR json_extract(NEW.request_json, '$.request.source_preparation_delivery_id') IS NOT NEW.source_preparation_delivery_id
             OR json_extract(NEW.request_json, '$.request.source_preparation_observation_digest') IS NOT NEW.source_preparation_observation_digest
             OR json_extract(NEW.request_json, '$.request.node_id') IS NOT NEW.node_id
             OR json_extract(NEW.request_json, '$.request.owner_user_id') IS NOT NEW.owner_user_id
             OR json_extract(NEW.request_json, '$.request.installation_identity_digest') IS NOT NEW.installation_identity_digest
             OR json_extract(NEW.request_json, '$.request.policy_revision') IS NOT NEW.policy_revision
             OR json_extract(NEW.request_json, '$.request.policy_digest') IS NOT NEW.policy_digest
             OR json_extract(NEW.request_json, '$.request.policy_snapshot_digest') IS NOT NEW.policy_snapshot_digest
             OR json_extract(NEW.request_json, '$.request.authorization.authorization_ref') IS NOT NEW.authorization_ref
             OR json_extract(NEW.request_json, '$.request.authorization.revision') IS NOT NEW.authorization_revision
             OR json_extract(NEW.request_json, '$.request.authorization.digest') IS NOT NEW.authorization_digest
           ) BEGIN
             SELECT RAISE(ABORT, 'planning V2 request projection must match redundant columns');
           END;

         CREATE TRIGGER IF NOT EXISTS trg_compute_plugin_plan_v2_observed_safety_projection
           BEFORE INSERT ON node_compute_plugin_install_plan_planning_delivery_events_v2
           WHEN NEW.event_sequence = 2 AND NEW.event_kind = 'observed' AND (
             NOT json_valid(NEW.observed_json)
             OR NEW.observed_digest IS NULL OR length(NEW.observed_digest) IS NOT 64
             OR NEW.observed_snapshot_ready IS NULL
             OR json_extract(NEW.observed_json, '$.preparation_id') IS NOT NEW.source_preparation_id
             OR json_extract(NEW.observed_json, '$.cloud_session_id') IS NOT NEW.cloud_session_id
             OR json_extract(NEW.observed_json, '$.source_preparation_delivery_id') IS NOT NEW.source_preparation_delivery_id
             OR json_extract(NEW.observed_json, '$.source_preparation_observation_digest') IS NOT NEW.source_preparation_observation_digest
             OR json_extract(NEW.observed_json, '$.node_id') IS NOT NEW.node_id
             OR json_extract(NEW.observed_json, '$.owner_user_id') IS NOT NEW.owner_user_id
             OR (json_extract(NEW.observed_json, '$.installation_identity_digest') IS NOT NULL
                 AND json_extract(NEW.observed_json, '$.installation_identity_digest') IS NOT NEW.installation_identity_digest)
             OR json_extract(NEW.observed_json, '$.bootstrap_instance_id') IS NOT NEW.source_bootstrap_instance_id
             OR json_extract(NEW.observed_json, '$.configuration_generation') IS NOT NEW.source_configuration_generation
             OR json_extract(NEW.observed_json, '$.cancellation_generation') IS NOT NEW.source_cancellation_generation
             OR (json_type(NEW.observed_json, '$.snapshot_ready') IS NOT 'true'
                 AND json_type(NEW.observed_json, '$.snapshot_ready') IS NOT 'false')
             OR (json_type(NEW.observed_json, '$.replayed') IS NOT 'true'
                 AND json_type(NEW.observed_json, '$.replayed') IS NOT 'false')
             OR json_type(NEW.observed_json, '$.local_confirmation_available') IS NOT 'false'
             OR json_type(NEW.observed_json, '$.plan_apply_allowed') IS NOT 'false'
             OR json_type(NEW.observed_json, '$.new_work_admission_enabled') IS NOT 'false'
             OR json_type(NEW.observed_json, '$.downloads_allowed') IS NOT 'false'
             OR json_type(NEW.observed_json, '$.sidecar_launch_allowed') IS NOT 'false'
             OR json_type(NEW.observed_json, '$.side_effects_started') IS NOT 'false'
             OR json_type(NEW.observed_json, '$.blocked_reasons') IS NOT 'array'
             OR json_array_length(NEW.observed_json, '$.blocked_reasons') > 64
             OR (
               NEW.observed_snapshot_ready = 1 AND (
                 json_type(NEW.observed_json, '$.accepted') IS NOT 'true'
                 OR json_extract(NEW.observed_json, '$.installation_identity_digest') IS NOT NEW.installation_identity_digest
                 OR json_extract(NEW.observed_json, '$.observed_policy_revision') IS NOT NEW.policy_revision
                 OR json_extract(NEW.observed_json, '$.observed_policy_digest') IS NOT NEW.policy_digest
                 OR json_extract(NEW.observed_json, '$.observed_policy_snapshot_digest') IS NOT NEW.policy_snapshot_digest
                 OR json_extract(NEW.observed_json, '$.observed_authorization.authorization_ref') IS NOT NEW.authorization_ref
                 OR json_extract(NEW.observed_json, '$.observed_authorization.revision') IS NOT NEW.authorization_revision
                 OR json_extract(NEW.observed_json, '$.observed_authorization.digest') IS NOT NEW.authorization_digest
                 OR json_extract(NEW.observed_json, '$.phase') IS NOT 'planning_snapshot_ready'
                 OR json_array_length(NEW.observed_json, '$.blocked_reasons') != 0
                 OR json_type(NEW.observed_json, '$.compute_plugin_root_lock_acquired') IS NOT 'true'
                 OR json_type(NEW.observed_json, '$.trusted_time_authority_configured') IS NOT 'true'
                 OR json_type(NEW.observed_json, '$.rollback_anchor_witness_configured') IS NOT 'true'
                 OR json_type(NEW.observed_json, '$.root_pinned') IS NOT 'true'
                 OR json_type(NEW.observed_json, '$.authority_opened') IS NOT 'true'
                 OR json_type(NEW.observed_json, '$.process_fence_acquired') IS NOT 'true'
                 OR json_extract(NEW.observed_json, '$.error_code') IS NOT NULL
               )
             )
             OR (
               NEW.observed_snapshot_ready = 0 AND (
                 json_extract(NEW.observed_json, '$.phase') IS NOT 'blocked'
                 OR json_array_length(NEW.observed_json, '$.blocked_reasons') NOT BETWEEN 1 AND 64
                 OR json_type(NEW.observed_json, '$.compute_plugin_root_lock_acquired') IS NOT 'false'
                 OR json_type(NEW.observed_json, '$.trusted_time_authority_configured') IS NOT 'false'
                 OR json_type(NEW.observed_json, '$.rollback_anchor_witness_configured') IS NOT 'false'
                 OR json_type(NEW.observed_json, '$.root_pinned') IS NOT 'false'
                 OR json_type(NEW.observed_json, '$.authority_opened') IS NOT 'false'
                 OR json_type(NEW.observed_json, '$.process_fence_acquired') IS NOT 'false'
                 OR (json_type(NEW.observed_json, '$.accepted') IS 'true' AND (
                       json_extract(NEW.observed_json, '$.installation_identity_digest') IS NOT NEW.installation_identity_digest
                       OR json_extract(NEW.observed_json, '$.observed_policy_revision') IS NOT NEW.policy_revision
                       OR json_extract(NEW.observed_json, '$.observed_policy_digest') IS NOT NEW.policy_digest
                       OR json_extract(NEW.observed_json, '$.observed_policy_snapshot_digest') IS NOT NEW.policy_snapshot_digest
                       OR json_extract(NEW.observed_json, '$.observed_authorization.authorization_ref') IS NOT NEW.authorization_ref
                       OR json_extract(NEW.observed_json, '$.observed_authorization.revision') IS NOT NEW.authorization_revision
                       OR json_extract(NEW.observed_json, '$.observed_authorization.digest') IS NOT NEW.authorization_digest
                       OR json_extract(NEW.observed_json, '$.error_code') IS NOT NULL
                     ))
                 OR (json_type(NEW.observed_json, '$.accepted') IS 'false'
                     AND json_extract(NEW.observed_json, '$.error_code') IS NULL)
                 OR (json_type(NEW.observed_json, '$.accepted') IS NOT 'true'
                     AND json_type(NEW.observed_json, '$.accepted') IS NOT 'false')
               )
             )
           ) BEGIN
             SELECT RAISE(ABORT, 'planning V2 observation crosses its read-only safety projection');
           END;

         CREATE TRIGGER IF NOT EXISTS trg_compute_plugin_plan_v2_snapshot_projection
           BEFORE INSERT ON node_compute_plugin_install_plan_planning_snapshots_v2
           WHEN NOT json_valid(NEW.snapshot_json) OR (
             json_extract(NEW.snapshot_json, '$.schema') IS NOT NEW.snapshot_schema
             OR json_extract(NEW.snapshot_json, '$.canonicalization') IS NOT 'rfc8785_jcs'
             OR json_extract(NEW.snapshot_json, '$.snapshot_digest_algorithm') IS NOT 'sha256'
             OR json_extract(NEW.snapshot_json, '$.snapshot_digest') IS NOT NEW.snapshot_digest
             OR json_extract(NEW.snapshot_json, '$.snapshot.schema') IS NOT 'elon.compute_plugin.install_plan_planning_snapshot.v2'
             OR json_extract(NEW.snapshot_json, '$.snapshot.preparation_id') IS NOT NEW.source_preparation_id
             OR json_extract(NEW.snapshot_json, '$.snapshot.cloud_session_id') IS NOT NEW.cloud_session_id
             OR json_extract(NEW.snapshot_json, '$.snapshot.source_preparation_delivery_id') IS NOT NEW.source_preparation_delivery_id
             OR json_extract(NEW.snapshot_json, '$.snapshot.source_preparation_observation_digest') IS NOT NEW.source_preparation_observation_digest
             OR json_extract(NEW.snapshot_json, '$.snapshot.node_id') IS NOT NEW.node_id
             OR json_extract(NEW.snapshot_json, '$.snapshot.owner_user_id') IS NOT NEW.owner_user_id
             OR json_extract(NEW.snapshot_json, '$.snapshot.installation_identity_digest') IS NOT NEW.installation_identity_digest
             OR json_extract(NEW.snapshot_json, '$.snapshot.policy_revision') IS NOT NEW.policy_revision
             OR json_extract(NEW.snapshot_json, '$.snapshot.policy_digest') IS NOT NEW.policy_digest
             OR json_extract(NEW.snapshot_json, '$.snapshot.policy_snapshot_digest') IS NOT NEW.policy_snapshot_digest
             OR json_type(NEW.snapshot_json, '$.snapshot.sharing_enabled') IS NOT 'true'
             OR json_extract(NEW.snapshot_json, '$.snapshot.authorization.authorization_ref') IS NOT NEW.authorization_ref
             OR json_extract(NEW.snapshot_json, '$.snapshot.authorization.revision') IS NOT NEW.authorization_revision
             OR json_extract(NEW.snapshot_json, '$.snapshot.authorization.digest') IS NOT NEW.authorization_digest
             OR json_extract(NEW.snapshot_json, '$.snapshot.bootstrap_instance_id') IS NOT NEW.bootstrap_instance_id
             OR json_extract(NEW.snapshot_json, '$.snapshot.configuration_generation') IS NOT NEW.configuration_generation
             OR json_extract(NEW.snapshot_json, '$.snapshot.cancellation_generation') IS NOT NEW.cancellation_generation
             OR json_extract(NEW.snapshot_json, '$.snapshot.policy_binding_receipt_digest') IS NOT NEW.policy_binding_receipt_digest
             OR json_extract(NEW.snapshot_json, '$.snapshot.policy_capability_revocation_receipt_digest') IS NOT NEW.policy_capability_revocation_receipt_digest
             OR json_extract(NEW.snapshot_json, '$.snapshot.policy_binding_source_preparation_id') IS NOT NEW.source_preparation_id
             OR json_extract(NEW.snapshot_json, '$.snapshot.policy_binding_authority_epoch') IS NOT NEW.policy_binding_authority_epoch
             OR json_extract(NEW.snapshot_json, '$.snapshot.policy_binding_process_owner_epoch') IS NOT NEW.policy_binding_process_owner_epoch
             OR json_extract(NEW.snapshot_json, '$.snapshot.authority_state_revision') IS NOT NEW.authority_state_revision
             OR json_extract(NEW.snapshot_json, '$.snapshot.authority_epoch') IS NOT NEW.authority_epoch
             OR json_extract(NEW.snapshot_json, '$.snapshot.process_owner_epoch') IS NOT NEW.process_owner_epoch
             OR json_extract(NEW.snapshot_json, '$.snapshot.clock_epoch_digest') IS NOT NEW.clock_epoch_digest
             OR json_extract(NEW.snapshot_json, '$.snapshot.trusted_time_high_water_ms') IS NOT NEW.trusted_time_high_water_ms
             OR json_extract(NEW.snapshot_json, '$.snapshot.captured_at_ms') IS NOT NEW.captured_at_ms
             OR json_extract(NEW.snapshot_json, '$.snapshot.expires_at_ms') IS NOT NEW.expires_at_ms
             OR json_extract(NEW.snapshot_json, '$.snapshot.rollback_anchor_witness_digest') IS NOT NEW.rollback_anchor_witness_digest
             OR json_extract(NEW.snapshot_json, '$.snapshot.inventory_revision') IS NOT NEW.inventory_revision
             OR json_extract(NEW.snapshot_json, '$.snapshot.inventory_digest') IS NOT NEW.inventory_digest
             OR json_extract(NEW.snapshot_json, '$.snapshot.node_profile_digest') IS NOT NEW.node_profile_digest
             OR json_extract(NEW.snapshot_json, '$.snapshot.manifest_catalog_revision') IS NOT NEW.manifest_catalog_revision
             OR json_extract(NEW.snapshot_json, '$.snapshot.manifest_catalog_digest') IS NOT NEW.manifest_catalog_digest
             OR json_extract(NEW.snapshot_json, '$.snapshot.keyring_bundle_revision') IS NOT NEW.keyring_bundle_revision
             OR json_extract(NEW.snapshot_json, '$.snapshot.publisher_keyring.revision') IS NOT NEW.publisher_keyring_revision
             OR json_extract(NEW.snapshot_json, '$.snapshot.publisher_keyring.digest') IS NOT NEW.publisher_keyring_digest
             OR json_extract(NEW.snapshot_json, '$.snapshot.control_keyring.revision') IS NOT NEW.control_keyring_revision
             OR json_extract(NEW.snapshot_json, '$.snapshot.control_keyring.digest') IS NOT NEW.control_keyring_digest
             OR json_extract(NEW.snapshot_json, '$.snapshot.target_id') IS NOT NEW.target_id
             OR json_extract(NEW.snapshot_json, '$.snapshot.host_api_protocol_id') IS NOT NEW.host_api_protocol_id
             OR json_extract(NEW.snapshot_json, '$.snapshot.host_api_revision') IS NOT NEW.host_api_revision
             OR json_type(NEW.snapshot_json, '$.snapshot.installed_records') IS NOT 'array'
             OR json_array_length(NEW.snapshot_json, '$.snapshot.installed_records') IS NOT NEW.installed_record_count
           ) BEGIN
             SELECT RAISE(ABORT, 'planning V2 snapshot projection must match its hashed envelope');
           END;",
    )?;

    install_generation_projection_triggers(conn)
}

fn install_generation_projection_triggers(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TRIGGER IF NOT EXISTS trg_compute_plugin_plan_generation_request_projection
           BEFORE INSERT ON node_compute_plugin_install_plan_generation_requests_v1
           WHEN NOT json_valid(NEW.request_json) OR (
             json_extract(NEW.request_json, '$.schema') IS NOT NEW.request_schema
             OR json_extract(NEW.request_json, '$.generation_request_id') IS NOT NEW.generation_request_id
             OR json_extract(NEW.request_json, '$.snapshot_id') IS NOT NEW.snapshot_id
             OR json_extract(NEW.request_json, '$.snapshot_digest') IS NOT NEW.snapshot_digest
             OR json_extract(NEW.request_json, '$.node_id') IS NOT NEW.node_id
             OR json_extract(NEW.request_json, '$.owner_user_id') IS NOT NEW.owner_user_id
             OR json_extract(NEW.request_json, '$.installation_identity_digest') IS NOT NEW.installation_identity_digest
             OR json_extract(NEW.request_json, '$.policy_revision') IS NOT NEW.policy_revision
             OR json_extract(NEW.request_json, '$.policy_digest') IS NOT NEW.policy_digest
             OR json_extract(NEW.request_json, '$.authorization_ref') IS NOT NEW.authorization_ref
             OR json_extract(NEW.request_json, '$.authorization_revision') IS NOT NEW.authorization_revision
             OR json_extract(NEW.request_json, '$.authorization_digest') IS NOT NEW.authorization_digest
             OR json_extract(NEW.request_json, '$.requested_control_keyring_revision') IS NOT NEW.requested_control_keyring_revision
             OR json_extract(NEW.request_json, '$.requested_control_keyring_digest') IS NOT NEW.requested_control_keyring_digest
             OR json_extract(NEW.request_json, '$.signer_profile') IS NOT NEW.signer_profile
             OR json_extract(NEW.request_json, '$.requested_at_ms') IS NOT NEW.requested_at_ms
           ) BEGIN
             SELECT RAISE(ABORT, 'generation request projection must match redundant columns');
           END;

         CREATE TRIGGER IF NOT EXISTS trg_compute_plugin_plan_generation_outcome_projection
           BEFORE INSERT ON node_compute_plugin_install_plan_generation_outcomes_v1
           WHEN NOT json_valid(NEW.outcome_json) OR (
             json_extract(NEW.outcome_json, '$.schema') IS NOT NEW.outcome_schema
             OR json_extract(NEW.outcome_json, '$.outcome_id') IS NOT NEW.outcome_id
             OR json_extract(NEW.outcome_json, '$.generation_request_id') IS NOT NEW.generation_request_id
             OR json_extract(NEW.outcome_json, '$.generation_request_digest') IS NOT NEW.generation_request_digest
             OR json_extract(NEW.outcome_json, '$.outcome_kind') IS NOT NEW.outcome_kind
             OR json_extract(NEW.outcome_json, '$.detail_code') IS NOT NEW.detail_code
             OR json_extract(NEW.outcome_json, '$.retryable') IS NOT NEW.retryable
             OR (NEW.retryable = 1 AND json_type(NEW.outcome_json, '$.retryable') IS NOT 'true')
             OR (NEW.retryable = 0 AND json_type(NEW.outcome_json, '$.retryable') IS NOT 'false')
           ) BEGIN
             SELECT RAISE(ABORT, 'generation outcome projection must match redundant columns');
           END;",
    )?;
    Ok(())
}
