use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_delivery_allocation_grants (
            grant_id TEXT PRIMARY KEY CHECK(length(trim(grant_id)) BETWEEN 1 AND 200),
            grant_schema TEXT NOT NULL CHECK(grant_schema=
                'compute_federation.delivery_allocation_grant.v1'),
            grant_revision INTEGER NOT NULL CHECK(grant_revision=1),
            grant_status TEXT NOT NULL CHECK(grant_status='granted'),
            grant_digest TEXT NOT NULL UNIQUE CHECK(
                length(grant_digest)=64 AND grant_digest NOT GLOB '*[^0-9a-f]*'),
            grant_json TEXT NOT NULL CHECK(
                json_valid(grant_json) AND json_type(grant_json)='object'
                AND length(CAST(grant_json AS BLOB))<=1048576),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            commitment_id TEXT NOT NULL UNIQUE CHECK(
                length(trim(commitment_id)) BETWEEN 1 AND 200),
            commitment_revision INTEGER NOT NULL CHECK(commitment_revision=1),
            commitment_digest TEXT NOT NULL CHECK(
                length(commitment_digest)=64
                AND commitment_digest NOT GLOB '*[^0-9a-f]*'),
            provider_owner_account_id TEXT NOT NULL CHECK(
                length(trim(provider_owner_account_id)) BETWEEN 1 AND 200),
            consumer_account_id TEXT NOT NULL CHECK(
                length(trim(consumer_account_id)) BETWEEN 1 AND 200),
            project_id TEXT CHECK(project_id IS NULL OR
                length(trim(project_id)) BETWEEN 1 AND 200),
            job_id TEXT NOT NULL UNIQUE CHECK(length(trim(job_id)) BETWEEN 1 AND 200),
            job_revision INTEGER NOT NULL CHECK(
                job_revision BETWEEN 1 AND 9007199254740991),
            job_digest TEXT NOT NULL CHECK(
                length(job_digest)=64 AND job_digest NOT GLOB '*[^0-9a-f]*'),
            exercise_expires_at TEXT NOT NULL CHECK(
                length(trim(exercise_expires_at))>0
                AND julianday(exercise_expires_at) IS NOT NULL
                AND (exercise_expires_at GLOB '*Z'
                    OR exercise_expires_at GLOB '*+00:00')),
            idempotency_scope TEXT NOT NULL CHECK(
                length(trim(idempotency_scope)) BETWEEN 1 AND 240),
            idempotency_key TEXT NOT NULL CHECK(
                length(trim(idempotency_key)) BETWEEN 1 AND 200),
            request_digest TEXT NOT NULL CHECK(
                length(request_digest)=64 AND request_digest NOT GLOB '*[^0-9a-f]*'),
            created_at TEXT NOT NULL CHECK(
                length(trim(created_at))>0 AND julianday(created_at) IS NOT NULL
                AND (created_at GLOB '*Z' OR created_at GLOB '*+00:00')),
            CHECK(provider_owner_account_id<>consumer_account_id),
            CHECK(julianday(created_at)<julianday(exercise_expires_at)),
            UNIQUE(grant_id, grant_digest),
            UNIQUE(idempotency_scope, idempotency_key),
            FOREIGN KEY(commitment_id, commitment_digest)
                REFERENCES compute_capacity_commitments(commitment_id, commitment_digest)
                ON DELETE RESTRICT,
            FOREIGN KEY(job_id, job_revision)
                REFERENCES compute_job_versions(job_id, revision) ON DELETE RESTRICT,
            FOREIGN KEY(job_id, job_digest)
                REFERENCES compute_job_versions(job_id, job_digest) ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS compute_delivery_allocation_terminal_receipts (
            terminal_receipt_id TEXT PRIMARY KEY CHECK(
                length(trim(terminal_receipt_id)) BETWEEN 1 AND 200),
            terminal_schema TEXT NOT NULL CHECK(terminal_schema=
                'compute_federation.delivery_allocation_terminal_receipt.v1'),
            terminal_revision INTEGER NOT NULL CHECK(terminal_revision=2),
            terminal_status TEXT NOT NULL CHECK(
                terminal_status IN ('exercised','declined','expired')),
            terminal_receipt_digest TEXT NOT NULL UNIQUE CHECK(
                length(terminal_receipt_digest)=64
                AND terminal_receipt_digest NOT GLOB '*[^0-9a-f]*'),
            terminal_receipt_json TEXT NOT NULL CHECK(
                json_valid(terminal_receipt_json)
                AND json_type(terminal_receipt_json)='object'
                AND length(CAST(terminal_receipt_json AS BLOB))<=1048576),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            grant_id TEXT NOT NULL UNIQUE CHECK(length(trim(grant_id)) BETWEEN 1 AND 200),
            grant_digest TEXT NOT NULL CHECK(
                length(grant_digest)=64 AND grant_digest NOT GLOB '*[^0-9a-f]*'),
            commitment_id TEXT NOT NULL UNIQUE CHECK(
                length(trim(commitment_id)) BETWEEN 1 AND 200),
            commitment_revision INTEGER NOT NULL CHECK(commitment_revision=1),
            commitment_digest TEXT NOT NULL CHECK(
                length(commitment_digest)=64
                AND commitment_digest NOT GLOB '*[^0-9a-f]*'),
            actor_kind TEXT NOT NULL CHECK(actor_kind IN ('consumer','admin')),
            actor_id TEXT NOT NULL CHECK(length(trim(actor_id)) BETWEEN 1 AND 200),
            idempotency_scope TEXT NOT NULL CHECK(
                length(trim(idempotency_scope)) BETWEEN 1 AND 240),
            idempotency_key TEXT NOT NULL CHECK(
                length(trim(idempotency_key)) BETWEEN 1 AND 200),
            request_digest TEXT NOT NULL CHECK(
                length(request_digest)=64 AND request_digest NOT GLOB '*[^0-9a-f]*'),
            occurred_at TEXT NOT NULL CHECK(
                length(trim(occurred_at))>0 AND julianday(occurred_at) IS NOT NULL
                AND (occurred_at GLOB '*Z' OR occurred_at GLOB '*+00:00')),
            recorded_at TEXT NOT NULL CHECK(
                length(trim(recorded_at))>0 AND julianday(recorded_at) IS NOT NULL
                AND (recorded_at GLOB '*Z' OR recorded_at GLOB '*+00:00')),
            parent_claim_id TEXT UNIQUE,
            parent_prior_claim_revision INTEGER,
            parent_prior_claim_digest TEXT,
            parent_result_claim_revision INTEGER,
            parent_result_claim_digest TEXT,
            parent_result_claim_state TEXT,
            parent_release_transaction_id TEXT UNIQUE,
            parent_release_transaction_digest TEXT,
            parent_release_ledger_sequence INTEGER,
            parent_release_event_kind TEXT,
            parent_release_causal_transaction_id TEXT,
            reservation_claim_id TEXT UNIQUE,
            reservation_claim_revision INTEGER,
            reservation_claim_digest TEXT,
            reservation_parent_claim_id TEXT,
            reservation_hold_transaction_id TEXT UNIQUE,
            reservation_hold_transaction_digest TEXT,
            reservation_hold_ledger_sequence INTEGER,
            reservation_hold_event_kind TEXT,
            reservation_hold_causal_transaction_id TEXT,
            reservation_id TEXT UNIQUE,
            reservation_revision INTEGER,
            reservation_digest TEXT,
            source_job_revision INTEGER,
            source_job_digest TEXT,
            reserved_job_revision INTEGER,
            reserved_job_digest TEXT,
            budget_reservation_id TEXT UNIQUE,
            reserved_amount_fen INTEGER,
            broker_reserve_request_digest TEXT,
            CHECK(julianday(occurred_at)<=julianday(recorded_at)),
            CHECK((terminal_status='exercised'
                AND parent_claim_id IS NOT NULL
                AND parent_prior_claim_revision IS NOT NULL
                AND parent_prior_claim_digest IS NOT NULL
                AND parent_result_claim_revision IS NOT NULL
                AND parent_result_claim_digest IS NOT NULL
                AND parent_result_claim_state IS NOT NULL
                AND parent_release_transaction_id IS NOT NULL
                AND parent_release_transaction_digest IS NOT NULL
                AND parent_release_ledger_sequence IS NOT NULL
                AND parent_release_event_kind IS NOT NULL
                AND parent_release_causal_transaction_id IS NOT NULL
                AND reservation_claim_id IS NOT NULL
                AND reservation_claim_revision IS NOT NULL
                AND reservation_claim_digest IS NOT NULL
                AND reservation_parent_claim_id IS NOT NULL
                AND reservation_hold_transaction_id IS NOT NULL
                AND reservation_hold_transaction_digest IS NOT NULL
                AND reservation_hold_ledger_sequence IS NOT NULL
                AND reservation_hold_event_kind IS NOT NULL
                AND reservation_hold_causal_transaction_id IS NOT NULL
                AND reservation_id IS NOT NULL AND reservation_revision IS NOT NULL
                AND reservation_digest IS NOT NULL AND source_job_revision IS NOT NULL
                AND source_job_digest IS NOT NULL AND reserved_job_revision IS NOT NULL
                AND reserved_job_digest IS NOT NULL AND budget_reservation_id IS NOT NULL
                AND reserved_amount_fen IS NOT NULL
                AND broker_reserve_request_digest IS NOT NULL
                AND parent_prior_claim_revision=1 AND parent_prior_claim_digest IS NOT NULL
                AND parent_result_claim_revision=2 AND parent_result_claim_digest IS NOT NULL
                AND parent_result_claim_state='released'
                AND parent_release_transaction_id IS NOT NULL
                AND parent_release_transaction_digest IS NOT NULL
                AND parent_release_ledger_sequence BETWEEN 1 AND 9007199254740991
                AND parent_release_event_kind='reservation_released'
                AND parent_release_causal_transaction_id IS NOT NULL
                AND reservation_claim_id IS NOT NULL AND reservation_claim_revision=1
                AND reservation_claim_digest IS NOT NULL
                AND reservation_parent_claim_id=parent_claim_id
                AND reservation_hold_transaction_id IS NOT NULL
                AND reservation_hold_transaction_digest IS NOT NULL
                AND reservation_hold_ledger_sequence BETWEEN 1 AND 9007199254740991
                AND reservation_hold_event_kind='reservation_held'
                AND reservation_hold_causal_transaction_id=parent_release_transaction_id
                AND reservation_hold_ledger_sequence>parent_release_ledger_sequence
                AND reservation_id IS NOT NULL AND reservation_revision=2
                AND reservation_digest IS NOT NULL
                AND source_job_revision BETWEEN 1 AND 9007199254740991
                AND source_job_digest IS NOT NULL
                AND reserved_job_revision=source_job_revision+1
                AND reserved_job_digest IS NOT NULL
                AND budget_reservation_id IS NOT NULL AND reserved_amount_fen>=0
                AND broker_reserve_request_digest IS NOT NULL)
              OR (terminal_status IN ('declined','expired')
                AND parent_claim_id IS NULL AND parent_prior_claim_revision IS NULL
                AND parent_prior_claim_digest IS NULL
                AND parent_result_claim_revision IS NULL
                AND parent_result_claim_digest IS NULL AND parent_result_claim_state IS NULL
                AND parent_release_transaction_id IS NULL
                AND parent_release_transaction_digest IS NULL
                AND parent_release_ledger_sequence IS NULL
                AND parent_release_event_kind IS NULL
                AND parent_release_causal_transaction_id IS NULL
                AND reservation_claim_id IS NULL AND reservation_claim_revision IS NULL
                AND reservation_claim_digest IS NULL AND reservation_parent_claim_id IS NULL
                AND reservation_hold_transaction_id IS NULL
                AND reservation_hold_transaction_digest IS NULL
                AND reservation_hold_ledger_sequence IS NULL
                AND reservation_hold_event_kind IS NULL
                AND reservation_hold_causal_transaction_id IS NULL
                AND reservation_id IS NULL AND reservation_revision IS NULL
                AND reservation_digest IS NULL AND source_job_revision IS NULL
                AND source_job_digest IS NULL AND reserved_job_revision IS NULL
                AND reserved_job_digest IS NULL AND budget_reservation_id IS NULL
                AND reserved_amount_fen IS NULL AND broker_reserve_request_digest IS NULL)),
            UNIQUE(idempotency_scope, idempotency_key),
            FOREIGN KEY(grant_id, grant_digest)
                REFERENCES compute_delivery_allocation_grants(grant_id, grant_digest)
                ON DELETE RESTRICT,
            FOREIGN KEY(commitment_id, commitment_digest)
                REFERENCES compute_capacity_commitments(commitment_id, commitment_digest)
                ON DELETE RESTRICT,
            FOREIGN KEY(parent_claim_id, parent_prior_claim_revision)
                REFERENCES compute_capacity_claim_versions(claim_id, revision)
                ON DELETE RESTRICT,
            FOREIGN KEY(parent_claim_id, parent_prior_claim_digest)
                REFERENCES compute_capacity_claim_versions(claim_id, claim_digest)
                ON DELETE RESTRICT,
            FOREIGN KEY(parent_claim_id, parent_result_claim_revision)
                REFERENCES compute_capacity_claim_versions(claim_id, revision)
                ON DELETE RESTRICT,
            FOREIGN KEY(parent_claim_id, parent_result_claim_digest)
                REFERENCES compute_capacity_claim_versions(claim_id, claim_digest)
                ON DELETE RESTRICT,
            FOREIGN KEY(parent_release_transaction_id)
                REFERENCES compute_capacity_ledger_transactions(transaction_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(parent_release_causal_transaction_id)
                REFERENCES compute_capacity_ledger_transactions(transaction_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(reservation_claim_id, reservation_claim_revision)
                REFERENCES compute_capacity_claim_versions(claim_id, revision)
                ON DELETE RESTRICT,
            FOREIGN KEY(reservation_claim_id, reservation_claim_digest)
                REFERENCES compute_capacity_claim_versions(claim_id, claim_digest)
                ON DELETE RESTRICT,
            FOREIGN KEY(reservation_parent_claim_id)
                REFERENCES compute_capacity_claims(claim_id) ON DELETE RESTRICT,
            FOREIGN KEY(reservation_hold_transaction_id)
                REFERENCES compute_capacity_ledger_transactions(transaction_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(reservation_hold_causal_transaction_id)
                REFERENCES compute_capacity_ledger_transactions(transaction_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(reservation_id, reservation_revision)
                REFERENCES compute_reservation_versions(reservation_id, revision)
                ON DELETE RESTRICT,
            FOREIGN KEY(reservation_id, reservation_digest)
                REFERENCES compute_reservation_versions(reservation_id, reservation_digest)
                ON DELETE RESTRICT,
            FOREIGN KEY(reservation_id)
                REFERENCES compute_broker_reserve_receipts(reservation_id) ON DELETE RESTRICT,
            FOREIGN KEY(budget_reservation_id)
                REFERENCES billing_reservations(id) ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_delivery_allocation_grants_consumer
            ON compute_delivery_allocation_grants(
                consumer_account_id, created_at DESC, grant_id);
        CREATE INDEX IF NOT EXISTS idx_delivery_allocation_grants_expiry
            ON compute_delivery_allocation_grants(exercise_expires_at, grant_id);
        CREATE INDEX IF NOT EXISTS idx_delivery_allocation_terminal_recorded
            ON compute_delivery_allocation_terminal_receipts(
                recorded_at DESC, terminal_receipt_id);
        "#,
    )?;
    Ok(())
}
