use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v165(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_capacity_pools (
            pool_id TEXT PRIMARY KEY,
            provider_id TEXT NOT NULL CHECK (length(trim(provider_id)) > 0),
            resource_scope_digest TEXT NOT NULL CHECK (
                length(trim(resource_scope_digest)) > 0
            ),
            status TEXT NOT NULL CHECK (
                status IN ('registering', 'active', 'draining', 'retired', 'quarantined')
            ),
            current_capacity_epoch INTEGER NOT NULL CHECK (current_capacity_epoch > 0),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE (provider_id, resource_scope_digest)
        );

        CREATE TABLE IF NOT EXISTS compute_capacity_pool_versions (
            pool_id TEXT NOT NULL,
            capacity_epoch INTEGER NOT NULL CHECK (capacity_epoch > 0),
            pool_revision INTEGER NOT NULL CHECK (pool_revision > 0),
            pool_digest TEXT NOT NULL CHECK (length(trim(pool_digest)) > 0),
            resource_profile_json TEXT NOT NULL,
            region TEXT,
            supported_meters_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            retired_at TEXT,
            PRIMARY KEY (pool_id, capacity_epoch, pool_revision),
            UNIQUE (pool_id, capacity_epoch, pool_digest),
            FOREIGN KEY (pool_id)
                REFERENCES compute_capacity_pools(pool_id)
                ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS compute_capacity_buckets (
            bucket_id TEXT PRIMARY KEY,
            bucket_digest TEXT NOT NULL CHECK (length(trim(bucket_digest)) > 0),
            pool_id TEXT NOT NULL,
            capacity_epoch INTEGER NOT NULL CHECK (capacity_epoch > 0),
            pool_revision INTEGER NOT NULL CHECK (pool_revision > 0),
            delivery_window_id TEXT NOT NULL CHECK (length(trim(delivery_window_id)) > 0),
            delivery_window_digest TEXT NOT NULL CHECK (length(trim(delivery_window_digest)) > 0),
            delivery_window_starts_at TEXT NOT NULL,
            delivery_window_ends_at TEXT NOT NULL,
            meter TEXT NOT NULL CHECK (length(trim(meter)) > 0),
            meter_mode TEXT NOT NULL CHECK (meter_mode IN ('consumable', 'reusable')),
            quantum_units INTEGER NOT NULL CHECK (quantum_units > 0),
            meter_policy_digest TEXT NOT NULL CHECK (
                length(trim(meter_policy_digest)) > 0
            ),
            status TEXT NOT NULL CHECK (status IN ('open', 'closed', 'retired')),
            issued_units INTEGER NOT NULL CHECK (issued_units >= 0),
            available_units INTEGER NOT NULL CHECK (available_units >= 0),
            held_units INTEGER NOT NULL CHECK (held_units >= 0),
            active_units INTEGER NOT NULL CHECK (active_units >= 0),
            consumed_units INTEGER NOT NULL CHECK (consumed_units >= 0),
            retired_units INTEGER NOT NULL CHECK (retired_units >= 0),
            balance_revision INTEGER NOT NULL CHECK (balance_revision >= 0),
            through_ledger_sequence INTEGER CHECK (
                through_ledger_sequence IS NULL
                OR (
                    typeof(through_ledger_sequence) = 'integer'
                    AND through_ledger_sequence > 0
                )
            ),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            CHECK (delivery_window_starts_at < delivery_window_ends_at),
            CHECK (
                (
                    meter_mode = 'consumable'
                    AND issued_units = available_units + held_units + active_units
                        + consumed_units + retired_units
                )
                OR
                (
                    meter_mode = 'reusable'
                    AND consumed_units = 0
                    AND issued_units = available_units + held_units + active_units
                        + retired_units
                )
            ),
            UNIQUE (pool_id, capacity_epoch, delivery_window_id, meter),
            FOREIGN KEY (pool_id, capacity_epoch, pool_revision)
                REFERENCES compute_capacity_pool_versions(
                    pool_id,
                    capacity_epoch,
                    pool_revision
                )
                ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS compute_capacity_claims (
            claim_id TEXT PRIMARY KEY,
            claim_digest TEXT NOT NULL CHECK (length(trim(claim_digest)) > 0),
            pool_id TEXT NOT NULL,
            capacity_epoch INTEGER NOT NULL CHECK (capacity_epoch > 0),
            delivery_window_id TEXT NOT NULL CHECK (length(trim(delivery_window_id)) > 0),
            claim_kind TEXT NOT NULL CHECK (length(trim(claim_kind)) > 0),
            subject_kind TEXT NOT NULL CHECK (length(trim(subject_kind)) > 0),
            subject_id TEXT NOT NULL CHECK (length(trim(subject_id)) > 0),
            status TEXT NOT NULL CHECK (
                status IN ('pending', 'held', 'active', 'consumed', 'released', 'expired', 'canceled')
            ),
            revision INTEGER NOT NULL CHECK (revision > 0),
            parent_claim_id TEXT,
            idempotency_scope TEXT NOT NULL CHECK (length(trim(idempotency_scope)) > 0),
            idempotency_key TEXT NOT NULL CHECK (length(trim(idempotency_key)) > 0),
            request_digest TEXT NOT NULL CHECK (length(trim(request_digest)) > 0),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            expires_at TEXT,
            terminal_at TEXT,
            CHECK (parent_claim_id IS NULL OR parent_claim_id <> claim_id),
            UNIQUE (idempotency_scope, idempotency_key),
            FOREIGN KEY (pool_id)
                REFERENCES compute_capacity_pools(pool_id)
                ON DELETE RESTRICT,
            FOREIGN KEY (parent_claim_id)
                REFERENCES compute_capacity_claims(claim_id)
                ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS compute_capacity_claim_lines (
            claim_id TEXT NOT NULL,
            line_no INTEGER NOT NULL CHECK (line_no >= 0),
            bucket_id TEXT NOT NULL,
            meter TEXT NOT NULL CHECK (length(trim(meter)) > 0),
            quantity_units INTEGER NOT NULL CHECK (quantity_units > 0),
            created_at TEXT NOT NULL,
            PRIMARY KEY (claim_id, line_no),
            UNIQUE (claim_id, bucket_id),
            UNIQUE (claim_id, meter),
            FOREIGN KEY (claim_id)
                REFERENCES compute_capacity_claims(claim_id)
                ON DELETE RESTRICT,
            FOREIGN KEY (bucket_id)
                REFERENCES compute_capacity_buckets(bucket_id)
                ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS compute_capacity_ledger_transactions (
            transaction_id TEXT PRIMARY KEY,
            transaction_digest TEXT NOT NULL CHECK (length(trim(transaction_digest)) > 0),
            pool_id TEXT NOT NULL,
            capacity_epoch INTEGER NOT NULL CHECK (capacity_epoch > 0),
            delivery_window_id TEXT NOT NULL CHECK (length(trim(delivery_window_id)) > 0),
            ledger_sequence INTEGER NOT NULL CHECK (
                typeof(ledger_sequence) = 'integer' AND ledger_sequence > 0
            ),
            event_kind TEXT NOT NULL CHECK (length(trim(event_kind)) > 0),
            claim_id TEXT,
            claim_effect TEXT,
            claim_effect_key TEXT,
            offer_id TEXT,
            offer_version INTEGER,
            offer_digest TEXT,
            job_id TEXT,
            reservation_id TEXT,
            attempt_lease_id TEXT,
            fencing_generation INTEGER,
            idempotency_scope TEXT NOT NULL CHECK (length(trim(idempotency_scope)) > 0),
            idempotency_key TEXT NOT NULL CHECK (length(trim(idempotency_key)) > 0),
            request_digest TEXT NOT NULL CHECK (length(trim(request_digest)) > 0),
            subject_kind TEXT NOT NULL CHECK (length(trim(subject_kind)) > 0),
            subject_id TEXT NOT NULL CHECK (length(trim(subject_id)) > 0),
            causal_transaction_id TEXT,
            occurred_at TEXT NOT NULL,
            recorded_at TEXT NOT NULL,
            CHECK (
                (
                    claim_id IS NULL
                    AND claim_effect IS NULL
                    AND claim_effect_key IS NULL
                )
                OR
                (
                    claim_id IS NOT NULL
                    AND claim_effect IS NOT NULL
                    AND length(trim(claim_effect)) > 0
                    AND claim_effect_key IS NOT NULL
                    AND length(trim(claim_effect_key)) > 0
                )
            ),
            CHECK (
                (
                    offer_id IS NULL
                    AND offer_version IS NULL
                    AND offer_digest IS NULL
                )
                OR
                (
                    offer_id IS NOT NULL
                    AND length(trim(offer_id)) > 0
                    AND typeof(offer_version) = 'integer'
                    AND offer_version > 0
                    AND offer_digest IS NOT NULL
                    AND length(trim(offer_digest)) > 0
                )
            ),
            CHECK (job_id IS NULL OR length(trim(job_id)) > 0),
            CHECK (
                reservation_id IS NULL
                OR (length(trim(reservation_id)) > 0 AND job_id IS NOT NULL)
            ),
            CHECK (
                (
                    attempt_lease_id IS NULL
                    AND fencing_generation IS NULL
                )
                OR
                (
                    attempt_lease_id IS NOT NULL
                    AND length(trim(attempt_lease_id)) > 0
                    AND reservation_id IS NOT NULL
                    AND typeof(fencing_generation) = 'integer'
                    AND fencing_generation > 0
                )
            ),
            CHECK (
                causal_transaction_id IS NULL
                OR causal_transaction_id <> transaction_id
            ),
            UNIQUE (idempotency_scope, idempotency_key),
            UNIQUE (pool_id, capacity_epoch, ledger_sequence),
            FOREIGN KEY (pool_id)
                REFERENCES compute_capacity_pools(pool_id)
                ON DELETE RESTRICT,
            FOREIGN KEY (claim_id)
                REFERENCES compute_capacity_claims(claim_id)
                ON DELETE RESTRICT,
            FOREIGN KEY (causal_transaction_id)
                REFERENCES compute_capacity_ledger_transactions(transaction_id)
                ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS compute_capacity_ledger_legs (
            leg_id TEXT PRIMARY KEY,
            transaction_id TEXT NOT NULL,
            line_no INTEGER NOT NULL CHECK (line_no >= 0),
            leg_role TEXT NOT NULL CHECK (leg_role IN ('from', 'to')),
            bucket_id TEXT NOT NULL,
            meter TEXT NOT NULL CHECK (length(trim(meter)) > 0),
            account TEXT NOT NULL CHECK (
                account IN ('issuance', 'available', 'held', 'active', 'consumed', 'retired')
            ),
            delta_units INTEGER NOT NULL CHECK (
                (leg_role = 'from' AND delta_units < 0)
                OR (leg_role = 'to' AND delta_units > 0)
            ),
            created_at TEXT NOT NULL,
            UNIQUE (transaction_id, line_no, leg_role),
            FOREIGN KEY (transaction_id)
                REFERENCES compute_capacity_ledger_transactions(transaction_id)
                ON DELETE RESTRICT,
            FOREIGN KEY (bucket_id)
                REFERENCES compute_capacity_buckets(bucket_id)
                ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_compute_capacity_pools_status
            ON compute_capacity_pools(status, updated_at, pool_id);

        CREATE INDEX IF NOT EXISTS idx_compute_capacity_pool_versions_epoch
            ON compute_capacity_pool_versions(pool_id, capacity_epoch, pool_revision DESC);

        CREATE INDEX IF NOT EXISTS idx_compute_capacity_buckets_window_expiry
            ON compute_capacity_buckets(
                status,
                delivery_window_ends_at,
                pool_id,
                capacity_epoch,
                meter
            );

        CREATE INDEX IF NOT EXISTS idx_compute_capacity_claims_status_expiry
            ON compute_capacity_claims(status, expires_at, claim_id)
            WHERE expires_at IS NOT NULL;

        CREATE INDEX IF NOT EXISTS idx_compute_capacity_claims_subject_status
            ON compute_capacity_claims(subject_kind, subject_id, status, updated_at);

        CREATE INDEX IF NOT EXISTS idx_compute_capacity_ledger_transactions_claim
            ON compute_capacity_ledger_transactions(claim_id, occurred_at, transaction_id)
            WHERE claim_id IS NOT NULL;

        CREATE UNIQUE INDEX IF NOT EXISTS idx_compute_capacity_ledger_claim_effect
            ON compute_capacity_ledger_transactions(
                claim_id,
                claim_effect,
                claim_effect_key
            )
            WHERE claim_id IS NOT NULL
                AND claim_effect IS NOT NULL
                AND claim_effect_key IS NOT NULL;

        CREATE TRIGGER IF NOT EXISTS trg_compute_capacity_pool_versions_no_update
        BEFORE UPDATE ON compute_capacity_pool_versions
        BEGIN
            SELECT RAISE(ABORT, 'compute capacity pool versions are append-only');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_capacity_pool_versions_no_delete
        BEFORE DELETE ON compute_capacity_pool_versions
        BEGIN
            SELECT RAISE(ABORT, 'compute capacity pool versions are append-only');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_capacity_claim_lines_no_update
        BEFORE UPDATE ON compute_capacity_claim_lines
        BEGIN
            SELECT RAISE(ABORT, 'compute capacity claim lines are append-only');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_capacity_claim_lines_no_delete
        BEFORE DELETE ON compute_capacity_claim_lines
        BEGIN
            SELECT RAISE(ABORT, 'compute capacity claim lines are append-only');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_capacity_ledger_transactions_no_update
        BEFORE UPDATE ON compute_capacity_ledger_transactions
        BEGIN
            SELECT RAISE(ABORT, 'compute capacity ledger transactions are append-only');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_capacity_ledger_transactions_no_delete
        BEFORE DELETE ON compute_capacity_ledger_transactions
        BEGIN
            SELECT RAISE(ABORT, 'compute capacity ledger transactions are append-only');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_capacity_ledger_legs_no_update
        BEFORE UPDATE ON compute_capacity_ledger_legs
        BEGIN
            SELECT RAISE(ABORT, 'compute capacity ledger legs are append-only');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_capacity_ledger_legs_no_delete
        BEFORE DELETE ON compute_capacity_ledger_legs
        BEGIN
            SELECT RAISE(ABORT, 'compute capacity ledger legs are append-only');
        END;
        "#,
    )?;
    Ok(())
}
