//! Append-only cloud ledger for Planning Snapshot V2 and inert generation requests.

use anyhow::Result;
use rusqlite::Connection;

mod projection;

pub(super) fn migration_v210(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS node_compute_plugin_install_plan_planning_delivery_events_v2 (
           id                                      TEXT PRIMARY KEY,
           planning_delivery_id                    TEXT NOT NULL,
           cloud_session_id                        TEXT NOT NULL CHECK(length(cloud_session_id) BETWEEN 1 AND 256),
           source_sharing_delivery_id               TEXT NOT NULL,
           source_preparation_id                    TEXT NOT NULL,
           source_preparation_delivery_id           TEXT NOT NULL,
           source_preparation_observation_id        TEXT NOT NULL,
           source_preparation_observation_digest    TEXT NOT NULL CHECK(length(source_preparation_observation_digest) = 64),
           source_preparation_request_digest        TEXT NOT NULL CHECK(length(source_preparation_request_digest) = 64),
           source_bootstrap_instance_id             TEXT NOT NULL CHECK(length(source_bootstrap_instance_id) BETWEEN 1 AND 256),
           source_configuration_generation          INTEGER NOT NULL CHECK(source_configuration_generation BETWEEN 0 AND 9007199254740991),
           source_cancellation_generation           INTEGER NOT NULL CHECK(source_cancellation_generation BETWEEN 0 AND 9007199254740991),
           request_schema                           TEXT NOT NULL CHECK(request_schema = 'elon.compute_plugin.install_plan_planning_delivery_request.v2'),
           request_json                             TEXT NOT NULL,
           request_digest                           TEXT NOT NULL CHECK(length(request_digest) = 64),
           node_id                                  TEXT NOT NULL,
           owner_user_id                            TEXT NOT NULL,
           consent_receipt_id                       TEXT NOT NULL,
           installation_identity_digest             TEXT NOT NULL CHECK(length(installation_identity_digest) = 64),
           policy_revision                          INTEGER NOT NULL CHECK(policy_revision BETWEEN 1 AND 9007199254740991),
           policy_digest                            TEXT NOT NULL CHECK(length(policy_digest) = 64),
           policy_snapshot_digest                   TEXT NOT NULL CHECK(length(policy_snapshot_digest) = 64),
           authorization_ref                        TEXT NOT NULL,
           authorization_revision                   INTEGER NOT NULL CHECK(authorization_revision BETWEEN 1 AND 9007199254740991),
           authorization_digest                     TEXT NOT NULL CHECK(length(authorization_digest) = 64),
           event_sequence                           INTEGER NOT NULL CHECK(event_sequence BETWEEN 1 AND 2),
           event_kind                               TEXT NOT NULL CHECK(event_kind IN (
                                                       'intent_committed', 'observed',
                                                       'capability_missing', 'agent_offline',
                                                       'session_replaced', 'writer_closed',
                                                       'ack_timeout', 'dispatch_failed'
                                                     )),
           observed_json                            TEXT,
           observed_digest                          TEXT,
           observed_snapshot_ready                  INTEGER CHECK(observed_snapshot_ready IN (0, 1)),
           observed_snapshot_json                   TEXT,
           observed_snapshot_digest                 TEXT,
           detail_code                              TEXT,
           created_at                               TEXT NOT NULL,
           UNIQUE(planning_delivery_id, event_sequence),
           CHECK(authorization_revision = policy_revision),
           CHECK(authorization_digest = policy_digest),
           CHECK(
              (event_sequence = 1 AND event_kind = 'intent_committed'
               AND observed_json IS NULL AND observed_digest IS NULL
               AND observed_snapshot_ready IS NULL AND observed_snapshot_json IS NULL
               AND observed_snapshot_digest IS NULL AND detail_code IS NULL)
              OR
               (event_sequence = 2 AND event_kind = 'observed'
                AND observed_json IS NOT NULL AND observed_digest IS NOT NULL
                AND length(observed_digest) = 64 AND observed_snapshot_ready IS NOT NULL
                AND observed_snapshot_ready IN (0, 1)
                AND ((observed_snapshot_ready = 1 AND observed_snapshot_json IS NOT NULL
                      AND observed_snapshot_digest IS NOT NULL
                      AND length(observed_snapshot_digest) = 64 AND detail_code IS NULL)
                    OR (observed_snapshot_ready = 0 AND observed_snapshot_json IS NULL
                        AND observed_snapshot_digest IS NULL)))
             OR
              (event_sequence = 2 AND event_kind NOT IN ('intent_committed', 'observed')
               AND observed_json IS NULL AND observed_digest IS NULL
               AND observed_snapshot_ready IS NULL AND observed_snapshot_json IS NULL
               AND observed_snapshot_digest IS NULL AND detail_code IS NOT NULL)
           )
         );
         CREATE UNIQUE INDEX IF NOT EXISTS idx_compute_plugin_plan_v2_source_once
           ON node_compute_plugin_install_plan_planning_delivery_events_v2(
             source_preparation_delivery_id
           ) WHERE event_sequence = 1;
         CREATE INDEX IF NOT EXISTS idx_compute_plugin_plan_v2_delivery_latest
           ON node_compute_plugin_install_plan_planning_delivery_events_v2(
             node_id, created_at DESC, id DESC
           );

         CREATE TABLE IF NOT EXISTS node_compute_plugin_install_plan_planning_snapshots_v2 (
           snapshot_id                              TEXT PRIMARY KEY,
           snapshot_schema                          TEXT NOT NULL CHECK(snapshot_schema = 'elon.compute_plugin.hashed_install_plan_planning_snapshot.v2'),
           snapshot_json                            TEXT NOT NULL,
           snapshot_digest                          TEXT NOT NULL CHECK(length(snapshot_digest) = 64),
           planning_delivery_id                     TEXT NOT NULL UNIQUE,
           cloud_session_id                         TEXT NOT NULL CHECK(length(cloud_session_id) BETWEEN 1 AND 256),
           source_preparation_id                    TEXT NOT NULL,
           source_preparation_delivery_id           TEXT NOT NULL UNIQUE,
           source_preparation_observation_id        TEXT NOT NULL UNIQUE,
           source_preparation_observation_digest    TEXT NOT NULL CHECK(length(source_preparation_observation_digest) = 64),
           source_preparation_request_digest        TEXT NOT NULL CHECK(length(source_preparation_request_digest) = 64),
           node_id                                  TEXT NOT NULL,
           owner_user_id                            TEXT NOT NULL,
           consent_receipt_id                       TEXT NOT NULL,
           installation_identity_digest             TEXT NOT NULL CHECK(length(installation_identity_digest) = 64),
           policy_revision                          INTEGER NOT NULL CHECK(policy_revision BETWEEN 1 AND 9007199254740991),
           policy_digest                            TEXT NOT NULL CHECK(length(policy_digest) = 64),
           policy_snapshot_digest                   TEXT NOT NULL CHECK(length(policy_snapshot_digest) = 64),
           authorization_ref                        TEXT NOT NULL,
           authorization_revision                   INTEGER NOT NULL CHECK(authorization_revision BETWEEN 1 AND 9007199254740991),
           authorization_digest                     TEXT NOT NULL CHECK(length(authorization_digest) = 64),
           bootstrap_instance_id                    TEXT NOT NULL CHECK(length(bootstrap_instance_id) BETWEEN 1 AND 256),
           configuration_generation                 INTEGER NOT NULL CHECK(configuration_generation BETWEEN 0 AND 9007199254740991),
           cancellation_generation                  INTEGER NOT NULL CHECK(cancellation_generation BETWEEN 0 AND 9007199254740991),
           policy_binding_receipt_digest             TEXT NOT NULL CHECK(length(policy_binding_receipt_digest) = 64),
           policy_capability_revocation_receipt_digest TEXT NOT NULL CHECK(length(policy_capability_revocation_receipt_digest) = 64),
           policy_binding_authority_epoch            INTEGER NOT NULL CHECK(policy_binding_authority_epoch BETWEEN 1 AND 9007199254740991),
           policy_binding_process_owner_epoch        INTEGER NOT NULL CHECK(policy_binding_process_owner_epoch BETWEEN 1 AND 9007199254740991),
           authority_state_revision                  INTEGER NOT NULL CHECK(authority_state_revision BETWEEN 1 AND 9007199254740991),
           authority_epoch                           INTEGER NOT NULL CHECK(authority_epoch BETWEEN 1 AND 9007199254740991),
           process_owner_epoch                       INTEGER NOT NULL CHECK(process_owner_epoch BETWEEN 1 AND 9007199254740991),
           clock_epoch_digest                        TEXT NOT NULL CHECK(length(clock_epoch_digest) = 64),
           trusted_time_high_water_ms                INTEGER NOT NULL CHECK(trusted_time_high_water_ms BETWEEN 1 AND 9007199254740991),
           captured_at_ms                            INTEGER NOT NULL CHECK(captured_at_ms BETWEEN 1 AND 9007199254740991),
           expires_at_ms                             INTEGER NOT NULL CHECK(expires_at_ms BETWEEN 1 AND 9007199254740991),
           rollback_anchor_witness_digest            TEXT NOT NULL CHECK(length(rollback_anchor_witness_digest) = 64),
           inventory_revision                        INTEGER NOT NULL CHECK(inventory_revision BETWEEN 0 AND 9007199254740991),
           inventory_digest                          TEXT NOT NULL CHECK(length(inventory_digest) = 64),
           node_profile_digest                       TEXT NOT NULL CHECK(length(node_profile_digest) = 64),
           manifest_catalog_revision                 INTEGER NOT NULL CHECK(manifest_catalog_revision BETWEEN 0 AND 9007199254740991),
           manifest_catalog_digest                   TEXT NOT NULL CHECK(length(manifest_catalog_digest) = 64),
           keyring_bundle_revision                   INTEGER NOT NULL CHECK(keyring_bundle_revision BETWEEN 1 AND 9007199254740991),
           publisher_keyring_revision                INTEGER NOT NULL CHECK(publisher_keyring_revision BETWEEN 1 AND 9007199254740991),
           publisher_keyring_digest                  TEXT NOT NULL CHECK(length(publisher_keyring_digest) = 64),
           control_keyring_revision                  INTEGER NOT NULL CHECK(control_keyring_revision BETWEEN 1 AND 9007199254740991),
           control_keyring_digest                    TEXT NOT NULL CHECK(length(control_keyring_digest) = 64),
           target_id                                 TEXT NOT NULL CHECK(length(target_id) BETWEEN 1 AND 256),
           host_api_protocol_id                      TEXT NOT NULL CHECK(length(host_api_protocol_id) BETWEEN 1 AND 256),
           host_api_revision                         INTEGER NOT NULL CHECK(host_api_revision > 0),
           installed_record_count                    INTEGER NOT NULL CHECK(installed_record_count BETWEEN 0 AND 256),
           created_at                               TEXT NOT NULL,
           CHECK(authorization_revision = policy_revision),
           CHECK(authorization_digest = policy_digest),
           CHECK(captured_at_ms > trusted_time_high_water_ms),
           CHECK(expires_at_ms - captured_at_ms BETWEEN 1 AND 300000),
           CHECK(authority_epoch >= policy_binding_authority_epoch),
           CHECK(process_owner_epoch >= policy_binding_process_owner_epoch),
           CHECK(publisher_keyring_revision != control_keyring_revision
                 OR publisher_keyring_digest != control_keyring_digest)
         );
         CREATE INDEX IF NOT EXISTS idx_compute_plugin_plan_v2_snapshot_latest
           ON node_compute_plugin_install_plan_planning_snapshots_v2(
             node_id, policy_revision DESC, created_at DESC, snapshot_id DESC
           );

         CREATE TABLE IF NOT EXISTS node_compute_plugin_install_plan_generation_requests_v1 (
           generation_request_id                    TEXT PRIMARY KEY,
           request_schema                           TEXT NOT NULL CHECK(request_schema = 'elon.compute_plugin.install_plan_generation_request.v1'),
           request_json                             TEXT NOT NULL,
           request_digest                           TEXT NOT NULL CHECK(length(request_digest) = 64),
           snapshot_id                              TEXT NOT NULL UNIQUE,
           snapshot_digest                          TEXT NOT NULL CHECK(length(snapshot_digest) = 64),
           node_id                                  TEXT NOT NULL,
           owner_user_id                            TEXT NOT NULL,
           installation_identity_digest             TEXT NOT NULL CHECK(length(installation_identity_digest) = 64),
           policy_revision                          INTEGER NOT NULL CHECK(policy_revision BETWEEN 1 AND 9007199254740991),
           policy_digest                            TEXT NOT NULL CHECK(length(policy_digest) = 64),
           authorization_ref                        TEXT NOT NULL,
           authorization_revision                   INTEGER NOT NULL CHECK(authorization_revision BETWEEN 1 AND 9007199254740991),
           authorization_digest                     TEXT NOT NULL CHECK(length(authorization_digest) = 64),
           requested_control_keyring_revision       INTEGER NOT NULL CHECK(requested_control_keyring_revision BETWEEN 1 AND 9007199254740991),
           requested_control_keyring_digest         TEXT NOT NULL CHECK(length(requested_control_keyring_digest) = 64),
           signer_profile                           TEXT NOT NULL CHECK(signer_profile = 'control_install_plan_v2'),
           requested_at_ms                          INTEGER NOT NULL CHECK(requested_at_ms BETWEEN 1 AND 9007199254740991),
           created_at                               TEXT NOT NULL,
           CHECK(authorization_revision = policy_revision),
           CHECK(authorization_digest = policy_digest)
         );
         CREATE INDEX IF NOT EXISTS idx_compute_plugin_plan_generation_request_latest
           ON node_compute_plugin_install_plan_generation_requests_v1(
             node_id, created_at DESC, generation_request_id DESC
           );

         CREATE TABLE IF NOT EXISTS node_compute_plugin_install_plan_generation_outcomes_v1 (
           outcome_id                               TEXT PRIMARY KEY,
           outcome_schema                           TEXT NOT NULL CHECK(outcome_schema = 'elon.compute_plugin.install_plan_generation_outcome.v1'),
           outcome_json                             TEXT NOT NULL,
           outcome_digest                           TEXT NOT NULL CHECK(length(outcome_digest) = 64),
           generation_request_id                    TEXT NOT NULL UNIQUE,
           generation_request_digest                TEXT NOT NULL CHECK(length(generation_request_digest) = 64),
           outcome_kind                             TEXT NOT NULL CHECK(outcome_kind IN ('signer_unavailable', 'rejected')),
           detail_code                              TEXT NOT NULL CHECK(length(detail_code) BETWEEN 1 AND 256),
           retryable                                INTEGER NOT NULL CHECK(retryable IN (0, 1)),
           created_at                               TEXT NOT NULL,
           CHECK(outcome_kind != 'rejected' OR retryable = 0)
         );
         CREATE INDEX IF NOT EXISTS idx_compute_plugin_plan_generation_outcome_latest
           ON node_compute_plugin_install_plan_generation_outcomes_v1(
             created_at DESC, outcome_id DESC
           );

         CREATE TRIGGER IF NOT EXISTS trg_compute_plugin_plan_v2_delivery_source
           BEFORE INSERT ON node_compute_plugin_install_plan_planning_delivery_events_v2
           WHEN NEW.event_sequence = 1 AND NOT EXISTS (
             SELECT 1
               FROM node_compute_plugin_install_plan_preparation_requests request
               JOIN node_compute_plugin_install_plan_preparation_delivery_events delivery
                 ON delivery.preparation_id = request.preparation_id
                AND delivery.node_id = request.node_id
                AND delivery.consent_receipt_id = request.consent_receipt_id
                AND delivery.policy_revision = request.policy_revision
                AND delivery.policy_digest = request.policy_digest
               JOIN node_compute_plugin_install_plan_preparation_observations observation
                 ON observation.delivery_id = delivery.delivery_id
                AND observation.preparation_id = request.preparation_id
                AND observation.node_id = request.node_id
                AND observation.consent_receipt_id = request.consent_receipt_id
                AND observation.policy_revision = request.policy_revision
                AND observation.policy_digest = request.policy_digest
                AND observation.policy_snapshot_digest = request.policy_snapshot_digest
               JOIN node_compute_plugin_sharing_consents consent
                 ON consent.receipt_id = request.consent_receipt_id
                AND consent.node_id = request.node_id
                AND consent.owner_user_id = request.owner_user_id
                AND consent.installation_identity_digest = request.installation_identity_digest
                AND consent.policy_revision = request.policy_revision
                AND consent.policy_digest = request.policy_digest
                AND consent.authorization_ref = request.authorization_ref
                AND consent.authorization_revision = request.authorization_revision
                AND consent.authorization_digest = request.authorization_digest
               JOIN node_compute_plugin_sharing_delivery_events sharing_delivery
                 ON sharing_delivery.delivery_id = delivery.sharing_delivery_id
                AND sharing_delivery.node_id = request.node_id
                AND sharing_delivery.consent_receipt_id = request.consent_receipt_id
                AND sharing_delivery.policy_revision = request.policy_revision
                AND sharing_delivery.policy_digest = request.policy_digest
               JOIN node_compute_plugin_sharing_observations sharing_observation
                 ON sharing_observation.delivery_id = sharing_delivery.delivery_id
                AND sharing_observation.node_id = request.node_id
                AND sharing_observation.consent_receipt_id = request.consent_receipt_id
                AND sharing_observation.policy_revision = request.policy_revision
                AND sharing_observation.policy_digest = request.policy_digest
               WHERE request.preparation_id = NEW.source_preparation_id
                 AND request.request_schema = 'elon.compute_plugin.install_plan_preparation_request.v1'
                 AND request.request_digest = NEW.source_preparation_request_digest
                AND request.node_id = NEW.node_id
                AND request.owner_user_id = NEW.owner_user_id
                AND request.consent_receipt_id = NEW.consent_receipt_id
                AND request.installation_identity_digest = NEW.installation_identity_digest
                AND request.policy_revision = NEW.policy_revision
                AND request.policy_digest = NEW.policy_digest
                AND request.policy_snapshot_digest = NEW.policy_snapshot_digest
                AND request.authorization_ref = NEW.authorization_ref
                AND request.authorization_revision = NEW.authorization_revision
                AND request.authorization_digest = NEW.authorization_digest
                AND delivery.delivery_id = NEW.source_preparation_delivery_id
                AND delivery.sharing_delivery_id = NEW.source_sharing_delivery_id
                AND delivery.event_sequence = 2
                AND delivery.event_kind = 'dispatched'
                AND delivery.detail_code IS NULL
                AND observation.id = NEW.source_preparation_observation_id
                 AND observation.observed_digest = NEW.source_preparation_observation_digest
                 AND observation.accepted = 1
                 AND json_valid(observation.observed_json)
                 AND json_extract(observation.observed_json, '$.bootstrap_instance_id') = NEW.source_bootstrap_instance_id
                 AND json_extract(observation.observed_json, '$.configuration_generation') = NEW.source_configuration_generation
                 AND json_extract(observation.observed_json, '$.cancellation_generation') = NEW.source_cancellation_generation
                AND consent.plugin_runtime_requested = 1
                AND sharing_delivery.event_sequence = 2
                AND sharing_delivery.event_kind = 'dispatched'
                AND sharing_delivery.detail_code IS NULL
                AND sharing_observation.accepted = 1
           ) BEGIN
             SELECT RAISE(ABORT, 'planning V2 requires exact accepted preparation session source');
           END;

         CREATE TRIGGER IF NOT EXISTS trg_compute_plugin_plan_v2_delivery_sequence
           BEFORE INSERT ON node_compute_plugin_install_plan_planning_delivery_events_v2
           WHEN NEW.event_sequence = 2 AND NOT EXISTS (
             SELECT 1
               FROM node_compute_plugin_install_plan_planning_delivery_events_v2 prior
              WHERE prior.planning_delivery_id = NEW.planning_delivery_id
                AND prior.cloud_session_id = NEW.cloud_session_id
                AND prior.source_sharing_delivery_id = NEW.source_sharing_delivery_id
                AND prior.source_preparation_id = NEW.source_preparation_id
                AND prior.source_preparation_delivery_id = NEW.source_preparation_delivery_id
                AND prior.source_preparation_observation_id = NEW.source_preparation_observation_id
                 AND prior.source_preparation_observation_digest = NEW.source_preparation_observation_digest
                 AND prior.source_preparation_request_digest = NEW.source_preparation_request_digest
                 AND prior.source_bootstrap_instance_id = NEW.source_bootstrap_instance_id
                 AND prior.source_configuration_generation = NEW.source_configuration_generation
                 AND prior.source_cancellation_generation = NEW.source_cancellation_generation
                AND prior.request_schema = NEW.request_schema
                AND prior.request_json = NEW.request_json
                AND prior.request_digest = NEW.request_digest
                AND prior.node_id = NEW.node_id
                AND prior.owner_user_id = NEW.owner_user_id
                AND prior.consent_receipt_id = NEW.consent_receipt_id
                AND prior.installation_identity_digest = NEW.installation_identity_digest
                AND prior.policy_revision = NEW.policy_revision
                AND prior.policy_digest = NEW.policy_digest
                AND prior.policy_snapshot_digest = NEW.policy_snapshot_digest
                AND prior.authorization_ref = NEW.authorization_ref
                AND prior.authorization_revision = NEW.authorization_revision
                AND prior.authorization_digest = NEW.authorization_digest
                AND prior.event_sequence = 1
                AND prior.event_kind = 'intent_committed'
           ) BEGIN
             SELECT RAISE(ABORT, 'planning V2 outcome requires exact session intent');
           END;

         CREATE TRIGGER IF NOT EXISTS trg_compute_plugin_plan_v2_observed_projection
           BEFORE INSERT ON node_compute_plugin_install_plan_planning_delivery_events_v2
           WHEN NEW.event_sequence = 2 AND NEW.event_kind = 'observed' AND (
             NOT json_valid(NEW.observed_json)
              OR json_extract(NEW.observed_json, '$.schema') IS NOT 'elon.compute_plugin.install_plan_planning_snapshot_observed.v2'
             OR json_extract(NEW.observed_json, '$.bootstrap_instance_id') IS NOT NEW.source_bootstrap_instance_id
             OR json_extract(NEW.observed_json, '$.configuration_generation') IS NOT NEW.source_configuration_generation
             OR json_extract(NEW.observed_json, '$.cancellation_generation') IS NOT NEW.source_cancellation_generation
             OR json_extract(NEW.observed_json, '$.snapshot_ready') IS NOT NEW.observed_snapshot_ready
             OR json_extract(NEW.observed_json, '$.snapshot') IS NOT NEW.observed_snapshot_json
             OR json_extract(NEW.observed_json, '$.snapshot.snapshot_digest') IS NOT NEW.observed_snapshot_digest
             OR json_extract(NEW.observed_json, '$.error_code') IS NOT NEW.detail_code
           ) BEGIN
             SELECT RAISE(ABORT, 'planning V2 observed projection must exactly match canonical ACK');
           END;

         CREATE TRIGGER IF NOT EXISTS trg_compute_plugin_plan_v2_snapshot_source
           BEFORE INSERT ON node_compute_plugin_install_plan_planning_snapshots_v2
           WHEN NOT EXISTS (
             SELECT 1
               FROM node_compute_plugin_install_plan_planning_delivery_events_v2 delivery
              WHERE delivery.planning_delivery_id = NEW.planning_delivery_id
                AND delivery.cloud_session_id = NEW.cloud_session_id
                AND delivery.source_preparation_id = NEW.source_preparation_id
                AND delivery.source_preparation_delivery_id = NEW.source_preparation_delivery_id
                AND delivery.source_preparation_observation_id = NEW.source_preparation_observation_id
                 AND delivery.source_preparation_observation_digest = NEW.source_preparation_observation_digest
                 AND delivery.source_preparation_request_digest = NEW.source_preparation_request_digest
                 AND delivery.source_bootstrap_instance_id = NEW.bootstrap_instance_id
                 AND delivery.source_configuration_generation = NEW.configuration_generation
                 AND delivery.source_cancellation_generation = NEW.cancellation_generation
                AND delivery.node_id = NEW.node_id
                AND delivery.owner_user_id = NEW.owner_user_id
                AND delivery.consent_receipt_id = NEW.consent_receipt_id
                AND delivery.installation_identity_digest = NEW.installation_identity_digest
                AND delivery.policy_revision = NEW.policy_revision
                AND delivery.policy_digest = NEW.policy_digest
                AND delivery.policy_snapshot_digest = NEW.policy_snapshot_digest
                AND delivery.authorization_ref = NEW.authorization_ref
                AND delivery.authorization_revision = NEW.authorization_revision
                AND delivery.authorization_digest = NEW.authorization_digest
                AND delivery.event_sequence = 2
                 AND delivery.event_kind = 'observed'
                 AND delivery.observed_json IS NOT NULL
                 AND length(delivery.observed_digest) = 64
                 AND delivery.observed_snapshot_ready = 1
                 AND delivery.observed_snapshot_json = NEW.snapshot_json
                 AND delivery.observed_snapshot_digest = NEW.snapshot_digest
                 AND delivery.detail_code IS NULL
           ) BEGIN
             SELECT RAISE(ABORT, 'planning V2 snapshot requires exact observed session delivery');
           END;

         CREATE TRIGGER IF NOT EXISTS trg_compute_plugin_plan_generation_request_source
           BEFORE INSERT ON node_compute_plugin_install_plan_generation_requests_v1
           WHEN NOT EXISTS (
             SELECT 1
               FROM node_compute_plugin_install_plan_planning_snapshots_v2 snapshot
               JOIN node_compute_sharing_policies policy
                 ON policy.node_id = snapshot.node_id
                AND policy.owner_user_id = snapshot.owner_user_id
               JOIN node_compute_plugin_sharing_consents consent
                 ON consent.receipt_id = snapshot.consent_receipt_id
                AND consent.node_id = snapshot.node_id
                AND consent.owner_user_id = snapshot.owner_user_id
                AND consent.installation_identity_digest = snapshot.installation_identity_digest
                AND consent.policy_revision = snapshot.policy_revision
                AND consent.policy_digest = snapshot.policy_digest
                AND consent.authorization_ref = snapshot.authorization_ref
                AND consent.authorization_revision = snapshot.authorization_revision
                AND consent.authorization_digest = snapshot.authorization_digest
               WHERE snapshot.snapshot_id = NEW.snapshot_id
                AND snapshot.snapshot_digest = NEW.snapshot_digest
                AND snapshot.node_id = NEW.node_id
                AND snapshot.owner_user_id = NEW.owner_user_id
                AND snapshot.installation_identity_digest = NEW.installation_identity_digest
                AND snapshot.policy_revision = NEW.policy_revision
                AND snapshot.policy_digest = NEW.policy_digest
                AND snapshot.authorization_ref = NEW.authorization_ref
                AND snapshot.authorization_revision = NEW.authorization_revision
                AND snapshot.authorization_digest = NEW.authorization_digest
                 AND snapshot.control_keyring_revision = NEW.requested_control_keyring_revision
                 AND snapshot.control_keyring_digest = NEW.requested_control_keyring_digest
                 AND NEW.requested_at_ms >= snapshot.captured_at_ms
                 AND NEW.requested_at_ms < snapshot.expires_at_ms
                 AND policy.enabled = 1 AND policy.plugin_runtime_requested = 1
                 AND policy.plugin_consent_receipt_id = consent.receipt_id
                 AND policy.plugin_installation_identity_digest = consent.installation_identity_digest
                 AND policy.plugin_policy_revision = consent.policy_revision
                 AND policy.plugin_policy_digest = consent.policy_digest
                 AND policy.plugin_authorization_ref = consent.authorization_ref
                 AND policy.plugin_authorization_revision = consent.authorization_revision
                 AND policy.plugin_authorization_digest = consent.authorization_digest
                 AND policy.plugin_consent_schema = consent.consent_schema
                 AND policy.allowed_model_ids_json = consent.allowed_model_ids_json
                 AND policy.max_concurrent_runs = consent.max_concurrent_runs
                 AND policy.daily_token_limit = consent.daily_token_limit
                 AND consent.plugin_runtime_requested = 1
                 AND consent.consent_schema = 'elon.node_compute_plugin.sharing_consent.v1'
           ) BEGIN
             SELECT RAISE(ABORT, 'generation request requires exact planning V2 snapshot');
           END;

         CREATE TRIGGER IF NOT EXISTS trg_compute_plugin_plan_generation_outcome_source
           BEFORE INSERT ON node_compute_plugin_install_plan_generation_outcomes_v1
           WHEN NOT EXISTS (
             SELECT 1
               FROM node_compute_plugin_install_plan_generation_requests_v1 request
              WHERE request.generation_request_id = NEW.generation_request_id
                AND request.request_digest = NEW.generation_request_digest
           ) BEGIN
             SELECT RAISE(ABORT, 'generation outcome requires exact request');
           END;",
    )?;

    projection::install_projection_triggers(conn)?;
    install_append_only_triggers(conn)?;
    Ok(())
}

fn install_append_only_triggers(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TRIGGER IF NOT EXISTS trg_compute_plugin_plan_v2_delivery_no_update
           BEFORE UPDATE ON node_compute_plugin_install_plan_planning_delivery_events_v2 BEGIN
             SELECT RAISE(ABORT, 'planning V2 delivery events are append-only');
           END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_plugin_plan_v2_delivery_no_delete
           BEFORE DELETE ON node_compute_plugin_install_plan_planning_delivery_events_v2 BEGIN
             SELECT RAISE(ABORT, 'planning V2 delivery events are append-only');
           END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_plugin_plan_v2_snapshot_no_update
           BEFORE UPDATE ON node_compute_plugin_install_plan_planning_snapshots_v2 BEGIN
             SELECT RAISE(ABORT, 'planning V2 snapshots are append-only');
           END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_plugin_plan_v2_snapshot_no_delete
           BEFORE DELETE ON node_compute_plugin_install_plan_planning_snapshots_v2 BEGIN
             SELECT RAISE(ABORT, 'planning V2 snapshots are append-only');
           END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_plugin_plan_generation_request_no_update
           BEFORE UPDATE ON node_compute_plugin_install_plan_generation_requests_v1 BEGIN
             SELECT RAISE(ABORT, 'InstallPlan generation requests are append-only');
           END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_plugin_plan_generation_request_no_delete
           BEFORE DELETE ON node_compute_plugin_install_plan_generation_requests_v1 BEGIN
             SELECT RAISE(ABORT, 'InstallPlan generation requests are append-only');
           END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_plugin_plan_generation_outcome_no_update
           BEFORE UPDATE ON node_compute_plugin_install_plan_generation_outcomes_v1 BEGIN
             SELECT RAISE(ABORT, 'InstallPlan generation outcomes are append-only');
           END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_plugin_plan_generation_outcome_no_delete
           BEFORE DELETE ON node_compute_plugin_install_plan_generation_outcomes_v1 BEGIN
             SELECT RAISE(ABORT, 'InstallPlan generation outcomes are append-only');
           END;",
    )?;
    Ok(())
}
