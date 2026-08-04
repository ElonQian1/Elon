use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use crate::compute_federation::execution::ComputeAttemptLease;

pub(crate) fn migration_v186(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_attempt_lease_states (
           lease_id                    TEXT PRIMARY KEY,
           provider_id                 TEXT NOT NULL,
           consumer_account_id         TEXT NOT NULL,
           lease_revision              INTEGER NOT NULL CHECK(lease_revision > 0),
           lease_digest                TEXT NOT NULL CHECK(length(lease_digest) = 64),
           lease_json                  TEXT NOT NULL CHECK(length(trim(lease_json)) > 0),
           status                      TEXT NOT NULL CHECK(status IN (
                                         'staging', 'running', 'result_reported',
                                         'verifying', 'terminal'
                                       )),
           fencing_generation          INTEGER NOT NULL CHECK(fencing_generation > 0),
           expires_at                  TEXT NOT NULL,
           hard_deadline_at            TEXT NOT NULL,
           last_heartbeat_at           TEXT,
           updated_by_user_id          TEXT NOT NULL CHECK(length(trim(updated_by_user_id)) > 0),
           updated_at                  TEXT NOT NULL,
           FOREIGN KEY(lease_id)
             REFERENCES compute_attempt_activations(lease_id) ON DELETE RESTRICT,
           FOREIGN KEY(provider_id)
             REFERENCES compute_providers(provider_id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_compute_attempt_lease_states_provider
           ON compute_attempt_lease_states(provider_id, status, expires_at, lease_id);
         CREATE INDEX IF NOT EXISTS idx_compute_attempt_lease_states_consumer
           ON compute_attempt_lease_states(consumer_account_id, status, expires_at, lease_id);
         CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_lease_states_no_delete
         BEFORE DELETE ON compute_attempt_lease_states
         BEGIN
           SELECT RAISE(ABORT, 'compute attempt lease states cannot be deleted');
         END;

         CREATE TABLE IF NOT EXISTS compute_attempt_lease_renewals (
           renewal_id                  TEXT PRIMARY KEY,
           lease_id                    TEXT NOT NULL,
           provider_id                 TEXT NOT NULL,
           consumer_account_id         TEXT NOT NULL,
           previous_lease_revision     INTEGER NOT NULL CHECK(previous_lease_revision > 0),
           previous_lease_digest       TEXT NOT NULL CHECK(length(previous_lease_digest) = 64),
           target_lease_revision       INTEGER NOT NULL CHECK(target_lease_revision > 1),
           target_lease_digest         TEXT NOT NULL CHECK(length(target_lease_digest) = 64),
           target_lease_json           TEXT NOT NULL CHECK(length(trim(target_lease_json)) > 0),
           previous_status             TEXT NOT NULL CHECK(previous_status IN ('staging', 'running')),
           target_status               TEXT NOT NULL CHECK(target_status = 'running'),
           fencing_generation          INTEGER NOT NULL CHECK(fencing_generation > 0),
           previous_expires_at         TEXT NOT NULL,
           target_expires_at           TEXT NOT NULL,
           hard_deadline_at            TEXT NOT NULL,
           executor_heartbeat_ref      TEXT NOT NULL CHECK(length(trim(executor_heartbeat_ref)) > 0),
           request_digest              TEXT NOT NULL CHECK(length(request_digest) = 64),
           event_digest                TEXT NOT NULL CHECK(length(event_digest) = 64),
           idempotency_scope           TEXT NOT NULL CHECK(length(trim(idempotency_scope)) > 0),
           idempotency_key             TEXT NOT NULL CHECK(length(trim(idempotency_key)) > 0),
           renewed_by_user_id          TEXT NOT NULL CHECK(length(trim(renewed_by_user_id)) > 0),
           renewed_at                  TEXT NOT NULL,
           created_at                  TEXT NOT NULL,
           CHECK(target_lease_revision = previous_lease_revision + 1),
           UNIQUE(lease_id, target_lease_revision),
           UNIQUE(idempotency_scope, idempotency_key),
           FOREIGN KEY(lease_id)
             REFERENCES compute_attempt_activations(lease_id) ON DELETE RESTRICT,
           FOREIGN KEY(provider_id)
             REFERENCES compute_providers(provider_id) ON DELETE RESTRICT
         );
         CREATE INDEX IF NOT EXISTS idx_compute_attempt_lease_renewals_lease
           ON compute_attempt_lease_renewals(lease_id, target_lease_revision DESC);
         CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_lease_renewals_no_update
         BEFORE UPDATE ON compute_attempt_lease_renewals
         BEGIN
           SELECT RAISE(ABORT, 'compute attempt lease renewals are append-only');
         END;
         CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_lease_renewals_no_delete
         BEFORE DELETE ON compute_attempt_lease_renewals
         BEGIN
           SELECT RAISE(ABORT, 'compute attempt lease renewals are append-only');
         END;",
    )?;

    let activations = {
        let mut statement = conn.prepare(
            "SELECT lease_id, provider_id, consumer_account_id, lease_digest,
                    lease_json, activated_by_user_id, activated_at
               FROM compute_attempt_activations
              WHERE lease_id NOT IN (SELECT lease_id FROM compute_attempt_lease_states)",
        )?;
        statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    for (lease_id, provider_id, consumer_id, digest, lease_json, actor, activated_at) in activations
    {
        let lease: ComputeAttemptLease = serde_json::from_str(&lease_json)
            .with_context(|| format!("Attempt Lease {lease_id} 历史 JSON 无法解析"))?;
        conn.execute(
            "INSERT INTO compute_attempt_lease_states (
                lease_id, provider_id, consumer_account_id, lease_revision,
                lease_digest, lease_json, status, fencing_generation,
                expires_at, hard_deadline_at, last_heartbeat_at,
                updated_by_user_id, updated_at
             ) VALUES (?1, ?2, ?3, 1, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                lease_id,
                provider_id,
                consumer_id,
                digest,
                lease_json,
                lease.status,
                lease.fencing_generation,
                lease.expires_at,
                lease.hard_deadline_at,
                lease.last_heartbeat_at,
                actor,
                activated_at,
            ],
        )?;
    }
    Ok(())
}
