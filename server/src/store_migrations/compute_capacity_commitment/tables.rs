use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_capacity_commitments (
            commitment_id TEXT PRIMARY KEY CHECK(
                length(trim(commitment_id)) BETWEEN 1 AND 200),
            commitment_schema TEXT NOT NULL CHECK(commitment_schema=
                'compute_federation.capacity_commitment.v1'),
            commitment_revision INTEGER NOT NULL CHECK(commitment_revision=1),
            commitment_status TEXT NOT NULL CHECK(commitment_status='committed'),
            commitment_digest TEXT NOT NULL UNIQUE CHECK(
                length(commitment_digest)=64
                AND commitment_digest NOT GLOB '*[^0-9a-f]*'),
            commitment_json TEXT NOT NULL CHECK(
                json_valid(commitment_json)
                AND json_type(commitment_json)='object'
                AND length(CAST(commitment_json AS BLOB))<=1048576),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            owner_account_id TEXT NOT NULL CHECK(
                length(trim(owner_account_id)) BETWEEN 1 AND 200),
            provider_id TEXT NOT NULL CHECK(
                length(trim(provider_id)) BETWEEN 1 AND 160),
            provider_policy_revision INTEGER NOT NULL CHECK(
                provider_policy_revision BETWEEN 1 AND 9007199254740991),
            provider_digest TEXT NOT NULL CHECK(
                length(provider_digest)=64
                AND provider_digest NOT GLOB '*[^0-9a-f]*'),
            offer_id TEXT NOT NULL CHECK(length(trim(offer_id)) BETWEEN 1 AND 200),
            offer_version INTEGER NOT NULL CHECK(
                offer_version BETWEEN 1 AND 9007199254740991),
            offer_digest TEXT NOT NULL CHECK(
                length(offer_digest)=64 AND offer_digest NOT GLOB '*[^0-9a-f]*'),
            pool_id TEXT NOT NULL CHECK(length(trim(pool_id)) BETWEEN 1 AND 200),
            capacity_epoch INTEGER NOT NULL CHECK(
                capacity_epoch BETWEEN 1 AND 9007199254740991),
            pool_revision INTEGER NOT NULL CHECK(
                pool_revision BETWEEN 1 AND 9007199254740991),
            pool_digest TEXT NOT NULL CHECK(
                length(pool_digest)=64 AND pool_digest NOT GLOB '*[^0-9a-f]*'),
            delivery_window_id TEXT NOT NULL CHECK(
                length(trim(delivery_window_id)) BETWEEN 1 AND 200),
            delivery_window_digest TEXT NOT NULL CHECK(
                length(delivery_window_digest)=64
                AND delivery_window_digest NOT GLOB '*[^0-9a-f]*'),
            delivery_window_starts_at TEXT NOT NULL CHECK(
                length(trim(delivery_window_starts_at))>0
                AND julianday(delivery_window_starts_at) IS NOT NULL
                AND (delivery_window_starts_at GLOB '*Z'
                    OR delivery_window_starts_at GLOB '*+00:00')),
            delivery_window_ends_at TEXT NOT NULL CHECK(
                length(trim(delivery_window_ends_at))>0
                AND julianday(delivery_window_ends_at) IS NOT NULL),
            price_snapshot_id TEXT NOT NULL CHECK(
                length(trim(price_snapshot_id)) BETWEEN 1 AND 200),
            price_snapshot_digest TEXT NOT NULL CHECK(
                length(price_snapshot_digest)=64
                AND price_snapshot_digest NOT GLOB '*[^0-9a-f]*'),
            reference_binding_id TEXT NOT NULL CHECK(
                length(trim(reference_binding_id)) BETWEEN 1 AND 200),
            reference_binding_digest TEXT NOT NULL CHECK(
                length(reference_binding_digest)=64
                AND reference_binding_digest NOT GLOB '*[^0-9a-f]*'),
            instrument_id TEXT NOT NULL CHECK(
                length(trim(instrument_id)) BETWEEN 1 AND 200),
            claim_id TEXT NOT NULL UNIQUE CHECK(
                length(trim(claim_id)) BETWEEN 1 AND 200),
            claim_revision INTEGER NOT NULL CHECK(claim_revision=1),
            claim_digest TEXT NOT NULL CHECK(
                length(claim_digest)=64 AND claim_digest NOT GLOB '*[^0-9a-f]*'),
            hold_transaction_id TEXT NOT NULL UNIQUE CHECK(
                length(trim(hold_transaction_id)) BETWEEN 1 AND 200),
            hold_transaction_digest TEXT NOT NULL CHECK(
                length(hold_transaction_digest)=64
                AND hold_transaction_digest NOT GLOB '*[^0-9a-f]*'),
            hold_ledger_sequence INTEGER NOT NULL CHECK(
                hold_ledger_sequence BETWEEN 1 AND 9007199254740991),
            hold_event_kind TEXT NOT NULL CHECK(hold_event_kind='reservation_held'),
            idempotency_scope TEXT NOT NULL CHECK(
                length(trim(idempotency_scope)) BETWEEN 1 AND 240),
            idempotency_key TEXT NOT NULL CHECK(
                length(trim(idempotency_key)) BETWEEN 1 AND 200),
            request_digest TEXT NOT NULL CHECK(
                length(request_digest)=64 AND request_digest NOT GLOB '*[^0-9a-f]*'),
            created_at TEXT NOT NULL CHECK(
                length(trim(created_at))>0 AND julianday(created_at) IS NOT NULL
                AND (created_at GLOB '*Z' OR created_at GLOB '*+00:00')),
            expires_at TEXT NOT NULL CHECK(
                length(trim(expires_at))>0 AND julianday(expires_at) IS NOT NULL),
            CHECK(julianday(created_at)<julianday(delivery_window_starts_at)),
            CHECK(julianday(delivery_window_starts_at)<julianday(delivery_window_ends_at)),
            CHECK((delivery_window_ends_at GLOB '*Z'
                    OR delivery_window_ends_at GLOB '*+00:00')
                AND (expires_at GLOB '*Z' OR expires_at GLOB '*+00:00')
                AND julianday(expires_at)=julianday(delivery_window_ends_at)),
            UNIQUE(commitment_id, commitment_digest),
            UNIQUE(idempotency_scope, idempotency_key),
            FOREIGN KEY(provider_id, provider_policy_revision)
                REFERENCES compute_provider_versions(provider_id, policy_revision)
                ON DELETE RESTRICT,
            FOREIGN KEY(offer_id, offer_version)
                REFERENCES compute_offer_versions(offer_id, offer_version)
                ON DELETE RESTRICT,
            FOREIGN KEY(pool_id, capacity_epoch, pool_revision)
                REFERENCES compute_capacity_pool_versions(
                    pool_id, capacity_epoch, pool_revision)
                ON DELETE RESTRICT,
            FOREIGN KEY(price_snapshot_id)
                REFERENCES compute_price_snapshots(snapshot_id) ON DELETE RESTRICT,
            FOREIGN KEY(reference_binding_id)
                REFERENCES compute_platform_reference_price_curve_snapshot_bindings(binding_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(claim_id, claim_revision)
                REFERENCES compute_capacity_claim_versions(claim_id, revision)
                ON DELETE RESTRICT,
            FOREIGN KEY(claim_id, claim_digest)
                REFERENCES compute_capacity_claim_versions(claim_id, claim_digest)
                ON DELETE RESTRICT,
            FOREIGN KEY(hold_transaction_id)
                REFERENCES compute_capacity_ledger_transactions(transaction_id)
                ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS compute_capacity_commitment_terminal_receipts (
            terminal_receipt_id TEXT PRIMARY KEY CHECK(
                length(trim(terminal_receipt_id)) BETWEEN 1 AND 200),
            terminal_schema TEXT NOT NULL CHECK(terminal_schema=
                'compute_federation.capacity_commitment_terminal_receipt.v1'),
            terminal_revision INTEGER NOT NULL CHECK(terminal_revision=2),
            terminal_status TEXT NOT NULL CHECK(
                terminal_status IN ('canceled','expired')),
            terminal_receipt_digest TEXT NOT NULL UNIQUE CHECK(
                length(terminal_receipt_digest)=64
                AND terminal_receipt_digest NOT GLOB '*[^0-9a-f]*'),
            terminal_receipt_json TEXT NOT NULL CHECK(
                json_valid(terminal_receipt_json)
                AND json_type(terminal_receipt_json)='object'
                AND length(CAST(terminal_receipt_json AS BLOB))<=1048576),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            commitment_id TEXT NOT NULL UNIQUE,
            commitment_revision INTEGER NOT NULL CHECK(commitment_revision=1),
            commitment_digest TEXT NOT NULL CHECK(
                length(commitment_digest)=64
                AND commitment_digest NOT GLOB '*[^0-9a-f]*'),
            claim_id TEXT NOT NULL,
            prior_claim_revision INTEGER NOT NULL CHECK(prior_claim_revision=1),
            prior_claim_digest TEXT NOT NULL CHECK(
                length(prior_claim_digest)=64
                AND prior_claim_digest NOT GLOB '*[^0-9a-f]*'),
            result_claim_revision INTEGER NOT NULL CHECK(result_claim_revision=2),
            result_claim_digest TEXT NOT NULL CHECK(
                length(result_claim_digest)=64
                AND result_claim_digest NOT GLOB '*[^0-9a-f]*'),
            result_claim_state TEXT NOT NULL CHECK(
                result_claim_state IN ('released','expired')),
            terminal_transaction_id TEXT NOT NULL UNIQUE CHECK(
                length(trim(terminal_transaction_id)) BETWEEN 1 AND 200),
            terminal_transaction_digest TEXT NOT NULL CHECK(
                length(terminal_transaction_digest)=64
                AND terminal_transaction_digest NOT GLOB '*[^0-9a-f]*'),
            terminal_ledger_sequence INTEGER NOT NULL CHECK(
                terminal_ledger_sequence BETWEEN 1 AND 9007199254740991),
            terminal_event_kind TEXT NOT NULL CHECK(
                terminal_event_kind IN ('reservation_released','reservation_expired')),
            causal_transaction_id TEXT NOT NULL,
            actor_kind TEXT NOT NULL CHECK(
                actor_kind IN ('provider_owner','platform_admin')),
            actor_id TEXT NOT NULL CHECK(length(trim(actor_id)) BETWEEN 1 AND 200),
            reason TEXT CHECK(reason IS NULL OR (
                length(trim(reason)) BETWEEN 1 AND 1000 AND reason=trim(reason))),
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
            CHECK(julianday(occurred_at)<=julianday(recorded_at)),
            CHECK((terminal_status='canceled'
                    AND result_claim_state='released'
                    AND terminal_event_kind='reservation_released'
                    AND actor_kind='provider_owner')
                OR (terminal_status='expired'
                    AND result_claim_state='expired'
                    AND terminal_event_kind='reservation_expired'
                    AND actor_kind='platform_admin')),
            UNIQUE(commitment_id, commitment_digest),
            UNIQUE(idempotency_scope, idempotency_key),
            FOREIGN KEY(commitment_id, commitment_digest)
                REFERENCES compute_capacity_commitments(
                    commitment_id, commitment_digest) ON DELETE RESTRICT,
            FOREIGN KEY(claim_id, prior_claim_revision)
                REFERENCES compute_capacity_claim_versions(claim_id, revision)
                ON DELETE RESTRICT,
            FOREIGN KEY(claim_id, prior_claim_digest)
                REFERENCES compute_capacity_claim_versions(claim_id, claim_digest)
                ON DELETE RESTRICT,
            FOREIGN KEY(claim_id, result_claim_revision)
                REFERENCES compute_capacity_claim_versions(claim_id, revision)
                ON DELETE RESTRICT,
            FOREIGN KEY(claim_id, result_claim_digest)
                REFERENCES compute_capacity_claim_versions(claim_id, claim_digest)
                ON DELETE RESTRICT,
            FOREIGN KEY(terminal_transaction_id)
                REFERENCES compute_capacity_ledger_transactions(transaction_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(causal_transaction_id)
                REFERENCES compute_capacity_ledger_transactions(transaction_id)
                ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_compute_capacity_commitments_owner_pool
            ON compute_capacity_commitments(
                owner_account_id, provider_id, pool_id, created_at DESC, commitment_id);
        CREATE INDEX IF NOT EXISTS idx_compute_capacity_commitments_expiry
            ON compute_capacity_commitments(expires_at, commitment_id);
        CREATE INDEX IF NOT EXISTS idx_compute_capacity_commitment_terminal_recorded
            ON compute_capacity_commitment_terminal_receipts(
                recorded_at DESC, terminal_receipt_id);

        CREATE VIEW IF NOT EXISTS compute_capacity_commitment_current AS
        SELECT commitment.commitment_id,
               commitment.commitment_digest,
               commitment.owner_account_id,
               commitment.provider_id,
               commitment.offer_id,
               commitment.offer_version,
               commitment.pool_id,
               commitment.capacity_epoch,
               commitment.delivery_window_id,
               commitment.price_snapshot_id,
               commitment.reference_binding_id,
               commitment.claim_id,
               COALESCE(terminal.terminal_revision, commitment.commitment_revision)
                    AS current_revision,
               COALESCE(terminal.terminal_status, commitment.commitment_status)
                    AS current_status,
               COALESCE(terminal.result_claim_revision, commitment.claim_revision)
                    AS current_claim_revision,
               COALESCE(terminal.result_claim_digest, commitment.claim_digest)
                    AS current_claim_digest,
               terminal.terminal_receipt_id,
               terminal.terminal_receipt_digest,
               commitment.created_at,
               commitment.expires_at,
               terminal.occurred_at AS terminal_occurred_at,
               terminal.recorded_at AS terminal_recorded_at
          FROM compute_capacity_commitments commitment
          LEFT JOIN compute_capacity_commitment_terminal_receipts terminal
            ON terminal.commitment_id=commitment.commitment_id;
        "#,
    )?;
    Ok(())
}
