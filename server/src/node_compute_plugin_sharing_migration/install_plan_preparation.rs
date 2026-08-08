//! Append-only cloud ledger for inert InstallPlan context preparation.

use anyhow::Result;
use rusqlite::Connection;

pub(super) fn migration_v209(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS node_compute_plugin_install_plan_preparation_requests (
           preparation_id                 TEXT PRIMARY KEY,
           request_schema                 TEXT NOT NULL,
           request_digest                 TEXT NOT NULL CHECK(length(request_digest) = 64),
           node_id                        TEXT NOT NULL,
           owner_user_id                  TEXT NOT NULL,
           consent_receipt_id             TEXT NOT NULL,
           installation_identity_digest   TEXT NOT NULL CHECK(length(installation_identity_digest) = 64),
           policy_revision                INTEGER NOT NULL CHECK(policy_revision > 0),
           policy_digest                  TEXT NOT NULL CHECK(length(policy_digest) = 64),
           policy_snapshot_digest         TEXT NOT NULL CHECK(length(policy_snapshot_digest) = 64),
           authorization_ref              TEXT NOT NULL,
           authorization_revision         INTEGER NOT NULL CHECK(authorization_revision > 0),
           authorization_digest           TEXT NOT NULL CHECK(length(authorization_digest) = 64),
           created_at                     TEXT NOT NULL,
           UNIQUE(node_id, consent_receipt_id, policy_revision, policy_digest),
           CHECK(authorization_revision = policy_revision),
           CHECK(authorization_digest = policy_digest)
         );
         CREATE INDEX IF NOT EXISTS idx_node_compute_plugin_plan_preparation_request_latest
           ON node_compute_plugin_install_plan_preparation_requests(
             node_id, policy_revision DESC, created_at DESC
           );

         CREATE TABLE IF NOT EXISTS node_compute_plugin_install_plan_preparation_delivery_events (
           id                   TEXT PRIMARY KEY,
           delivery_id          TEXT NOT NULL,
           sharing_delivery_id  TEXT NOT NULL,
           preparation_id       TEXT NOT NULL,
           node_id              TEXT NOT NULL,
           consent_receipt_id   TEXT NOT NULL,
           policy_revision      INTEGER NOT NULL CHECK(policy_revision > 0),
           policy_digest        TEXT NOT NULL CHECK(length(policy_digest) = 64),
           event_sequence       INTEGER NOT NULL CHECK(event_sequence BETWEEN 1 AND 2),
           event_kind           TEXT NOT NULL CHECK(event_kind IN (
                                  'intent_committed', 'dispatched',
                                  'capability_missing', 'agent_offline',
                                  'writer_closed', 'ack_timeout', 'dispatch_failed'
                                )),
           detail_code          TEXT,
           created_at           TEXT NOT NULL,
           UNIQUE(delivery_id, event_sequence),
           CHECK(
             (event_sequence = 1 AND event_kind = 'intent_committed' AND detail_code IS NULL)
             OR
             (event_sequence = 2 AND event_kind = 'dispatched' AND detail_code IS NULL)
             OR
             (event_sequence = 2 AND event_kind NOT IN ('intent_committed', 'dispatched')
              AND detail_code IS NOT NULL)
           )
         );
         CREATE INDEX IF NOT EXISTS idx_node_compute_plugin_plan_preparation_delivery_latest
           ON node_compute_plugin_install_plan_preparation_delivery_events(
             node_id, created_at DESC, id DESC
           );
         CREATE UNIQUE INDEX IF NOT EXISTS idx_node_compute_plugin_plan_preparation_ack_once
           ON node_compute_plugin_install_plan_preparation_delivery_events(sharing_delivery_id)
           WHERE event_sequence = 1;

         CREATE TABLE IF NOT EXISTS node_compute_plugin_install_plan_preparation_observations (
           id                   TEXT PRIMARY KEY,
           delivery_id          TEXT NOT NULL,
           preparation_id       TEXT NOT NULL,
           node_id              TEXT NOT NULL,
           consent_receipt_id   TEXT NOT NULL,
           policy_revision      INTEGER NOT NULL CHECK(policy_revision > 0),
           policy_digest        TEXT NOT NULL CHECK(length(policy_digest) = 64),
           policy_snapshot_digest TEXT NOT NULL CHECK(length(policy_snapshot_digest) = 64),
           accepted             INTEGER NOT NULL CHECK(accepted IN (0, 1)),
           replayed             INTEGER NOT NULL CHECK(replayed IN (0, 1)),
           context_ready        INTEGER NOT NULL CHECK(context_ready IN (0, 1)),
           context_json         TEXT,
           context_digest       TEXT,
           bootstrap_instance_id TEXT NOT NULL,
           observed_json        TEXT NOT NULL,
           observed_digest      TEXT NOT NULL CHECK(length(observed_digest) = 64),
           created_at           TEXT NOT NULL,
           UNIQUE(delivery_id),
           CHECK(
             (context_ready = 0 AND context_json IS NULL AND context_digest IS NULL)
             OR (context_ready = 1 AND context_json IS NOT NULL AND length(context_digest) = 64)
           )
         );
         CREATE INDEX IF NOT EXISTS idx_node_compute_plugin_plan_preparation_observation_latest
           ON node_compute_plugin_install_plan_preparation_observations(
             node_id, created_at DESC, id DESC
           );

         CREATE TRIGGER IF NOT EXISTS trg_node_compute_plugin_plan_preparation_delivery_sequence
           BEFORE INSERT ON node_compute_plugin_install_plan_preparation_delivery_events
           WHEN NEW.event_sequence = 2 AND NOT EXISTS (
             SELECT 1
               FROM node_compute_plugin_install_plan_preparation_delivery_events prior
              WHERE prior.delivery_id = NEW.delivery_id
                AND prior.sharing_delivery_id = NEW.sharing_delivery_id
                AND prior.preparation_id = NEW.preparation_id
                AND prior.node_id = NEW.node_id
                AND prior.consent_receipt_id = NEW.consent_receipt_id
                AND prior.policy_revision = NEW.policy_revision
                AND prior.policy_digest = NEW.policy_digest
                AND prior.event_sequence = 1
                AND prior.event_kind = 'intent_committed'
                AND prior.detail_code IS NULL
           ) BEGIN
             SELECT RAISE(ABORT, 'compute plugin InstallPlan preparation outcome requires intent');
           END;
         CREATE TRIGGER IF NOT EXISTS trg_node_compute_plugin_plan_preparation_observation_dispatch
           BEFORE INSERT ON node_compute_plugin_install_plan_preparation_observations
           WHEN NOT EXISTS (
             SELECT 1
               FROM node_compute_plugin_install_plan_preparation_delivery_events delivery
               JOIN node_compute_plugin_install_plan_preparation_requests request
                 ON request.preparation_id = delivery.preparation_id
                AND request.node_id = delivery.node_id
                AND request.consent_receipt_id = delivery.consent_receipt_id
                AND request.policy_revision = delivery.policy_revision
                AND request.policy_digest = delivery.policy_digest
              WHERE delivery.delivery_id = NEW.delivery_id
                AND delivery.preparation_id = NEW.preparation_id
                AND delivery.node_id = NEW.node_id
                AND delivery.consent_receipt_id = NEW.consent_receipt_id
                AND delivery.policy_revision = NEW.policy_revision
                AND delivery.policy_digest = NEW.policy_digest
                AND delivery.event_sequence = 2
                AND delivery.event_kind = 'dispatched'
                AND delivery.detail_code IS NULL
                AND request.policy_snapshot_digest = NEW.policy_snapshot_digest
           ) BEGIN
             SELECT RAISE(ABORT, 'compute plugin InstallPlan preparation observation requires dispatch');
           END;

         CREATE TRIGGER IF NOT EXISTS trg_node_compute_plugin_plan_preparation_requests_no_update
           BEFORE UPDATE ON node_compute_plugin_install_plan_preparation_requests BEGIN
             SELECT RAISE(ABORT, 'compute plugin InstallPlan preparation requests are append-only');
           END;
         CREATE TRIGGER IF NOT EXISTS trg_node_compute_plugin_plan_preparation_requests_no_delete
           BEFORE DELETE ON node_compute_plugin_install_plan_preparation_requests BEGIN
             SELECT RAISE(ABORT, 'compute plugin InstallPlan preparation requests are append-only');
           END;
         CREATE TRIGGER IF NOT EXISTS trg_node_compute_plugin_plan_preparation_delivery_no_update
           BEFORE UPDATE ON node_compute_plugin_install_plan_preparation_delivery_events BEGIN
             SELECT RAISE(ABORT, 'compute plugin InstallPlan preparation deliveries are append-only');
           END;
         CREATE TRIGGER IF NOT EXISTS trg_node_compute_plugin_plan_preparation_delivery_no_delete
           BEFORE DELETE ON node_compute_plugin_install_plan_preparation_delivery_events BEGIN
             SELECT RAISE(ABORT, 'compute plugin InstallPlan preparation deliveries are append-only');
           END;
         CREATE TRIGGER IF NOT EXISTS trg_node_compute_plugin_plan_preparation_observations_no_update
           BEFORE UPDATE ON node_compute_plugin_install_plan_preparation_observations BEGIN
             SELECT RAISE(ABORT, 'compute plugin InstallPlan preparation observations are append-only');
           END;
         CREATE TRIGGER IF NOT EXISTS trg_node_compute_plugin_plan_preparation_observations_no_delete
           BEFORE DELETE ON node_compute_plugin_install_plan_preparation_observations BEGIN
             SELECT RAISE(ABORT, 'compute plugin InstallPlan preparation observations are append-only');
           END;",
    )?;
    Ok(())
}
