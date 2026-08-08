//! Durable, append-only control receipts for the opt-in compute plugin runtime.

use anyhow::Result;
use rusqlite::Connection;

mod install_plan_preparation;

pub(crate) fn migration_v208(conn: &Connection) -> Result<()> {
    for (column, definition) in [
        (
            "plugin_runtime_requested",
            "plugin_runtime_requested INTEGER NOT NULL DEFAULT 0 CHECK(plugin_runtime_requested IN (0, 1))",
        ),
        (
            "plugin_policy_revision",
            "plugin_policy_revision INTEGER NOT NULL DEFAULT 0 CHECK(plugin_policy_revision >= 0)",
        ),
        ("plugin_policy_digest", "plugin_policy_digest TEXT"),
        ("plugin_consent_schema", "plugin_consent_schema TEXT"),
        (
            "plugin_consent_receipt_id",
            "plugin_consent_receipt_id TEXT",
        ),
        (
            "plugin_installation_identity_digest",
            "plugin_installation_identity_digest TEXT",
        ),
        ("plugin_authorization_ref", "plugin_authorization_ref TEXT"),
        (
            "plugin_authorization_revision",
            "plugin_authorization_revision INTEGER",
        ),
        (
            "plugin_authorization_digest",
            "plugin_authorization_digest TEXT",
        ),
    ] {
        crate::store_migrations::add_column_if_missing(
            conn,
            "node_compute_sharing_policies",
            column,
            definition,
        )?;
    }

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS node_compute_plugin_sharing_consents (
           receipt_id                       TEXT PRIMARY KEY,
           node_id                          TEXT NOT NULL,
           owner_user_id                    TEXT NOT NULL,
           consent_schema                   TEXT NOT NULL,
           installation_identity_digest     TEXT NOT NULL CHECK(length(installation_identity_digest) = 64),
           consent_request_id                TEXT NOT NULL,
           request_facts_digest              TEXT NOT NULL CHECK(length(request_facts_digest) = 64),
           policy_revision                  INTEGER NOT NULL CHECK(policy_revision > 0),
           policy_digest                    TEXT NOT NULL CHECK(length(policy_digest) = 64),
           plugin_runtime_requested          INTEGER NOT NULL CHECK(plugin_runtime_requested IN (0, 1)),
           allowed_model_ids_json            TEXT NOT NULL,
           max_concurrent_runs               INTEGER NOT NULL CHECK(max_concurrent_runs BETWEEN 1 AND 16),
           daily_token_limit                 INTEGER NOT NULL CHECK(daily_token_limit BETWEEN 0 AND 1000000000000),
           authorization_ref                 TEXT,
           authorization_revision            INTEGER,
           authorization_digest              TEXT,
           created_at                        TEXT NOT NULL,
           UNIQUE(node_id, policy_revision),
           UNIQUE(node_id, consent_request_id),
           CHECK (
             (plugin_runtime_requested = 0
              AND authorization_ref IS NULL
              AND authorization_revision IS NULL
              AND authorization_digest IS NULL)
             OR
             (plugin_runtime_requested = 1
              AND authorization_ref IS NOT NULL
              AND authorization_revision = policy_revision
              AND length(authorization_digest) = 64)
           )
         );
         CREATE INDEX IF NOT EXISTS idx_node_compute_plugin_sharing_consent_latest
           ON node_compute_plugin_sharing_consents(node_id, policy_revision DESC);

         CREATE TABLE IF NOT EXISTS node_compute_plugin_sharing_delivery_events (
           id                   TEXT PRIMARY KEY,
           delivery_id          TEXT NOT NULL,
           node_id              TEXT NOT NULL,
           consent_receipt_id   TEXT NOT NULL,
           policy_revision      INTEGER NOT NULL CHECK(policy_revision > 0),
           policy_digest        TEXT NOT NULL CHECK(length(policy_digest) = 64),
           event_sequence       INTEGER NOT NULL CHECK(event_sequence > 0),
           event_kind           TEXT NOT NULL CHECK(event_kind IN (
                                  'intent_committed', 'dispatched',
                                  'capability_missing', 'agent_offline',
                                  'writer_closed', 'ack_timeout', 'dispatch_failed'
                                )),
           detail_code          TEXT,
           created_at           TEXT NOT NULL,
           UNIQUE(delivery_id, event_sequence)
         );
         CREATE INDEX IF NOT EXISTS idx_node_compute_plugin_sharing_delivery_latest
           ON node_compute_plugin_sharing_delivery_events(node_id, created_at DESC, id DESC);

         CREATE TABLE IF NOT EXISTS node_compute_plugin_sharing_observations (
           id                   TEXT PRIMARY KEY,
           delivery_id          TEXT NOT NULL,
           node_id              TEXT NOT NULL,
           consent_receipt_id   TEXT NOT NULL,
           policy_revision      INTEGER NOT NULL CHECK(policy_revision > 0),
           policy_digest        TEXT NOT NULL CHECK(length(policy_digest) = 64),
           accepted             INTEGER NOT NULL CHECK(accepted IN (0, 1)),
           observed_json        TEXT NOT NULL,
           created_at           TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_node_compute_plugin_sharing_observation_latest
           ON node_compute_plugin_sharing_observations(node_id, created_at DESC, id DESC);

         CREATE TRIGGER IF NOT EXISTS trg_node_compute_plugin_sharing_consents_no_update
           BEFORE UPDATE ON node_compute_plugin_sharing_consents BEGIN
             SELECT RAISE(ABORT, 'compute plugin sharing consents are append-only');
           END;
         CREATE TRIGGER IF NOT EXISTS trg_node_compute_plugin_sharing_consents_no_delete
           BEFORE DELETE ON node_compute_plugin_sharing_consents BEGIN
             SELECT RAISE(ABORT, 'compute plugin sharing consents are append-only');
           END;
         CREATE TRIGGER IF NOT EXISTS trg_node_compute_plugin_sharing_delivery_no_update
           BEFORE UPDATE ON node_compute_plugin_sharing_delivery_events BEGIN
             SELECT RAISE(ABORT, 'compute plugin sharing delivery events are append-only');
           END;
         CREATE TRIGGER IF NOT EXISTS trg_node_compute_plugin_sharing_delivery_no_delete
           BEFORE DELETE ON node_compute_plugin_sharing_delivery_events BEGIN
             SELECT RAISE(ABORT, 'compute plugin sharing delivery events are append-only');
           END;
         CREATE TRIGGER IF NOT EXISTS trg_node_compute_plugin_sharing_observation_no_update
           BEFORE UPDATE ON node_compute_plugin_sharing_observations BEGIN
             SELECT RAISE(ABORT, 'compute plugin sharing observations are append-only');
           END;
         CREATE TRIGGER IF NOT EXISTS trg_node_compute_plugin_sharing_observation_no_delete
           BEFORE DELETE ON node_compute_plugin_sharing_observations BEGIN
             SELECT RAISE(ABORT, 'compute plugin sharing observations are append-only');
           END;",
    )?;
    Ok(())
}

pub(crate) fn migration_v209(conn: &Connection) -> Result<()> {
    install_plan_preparation::migration_v209(conn)
}
