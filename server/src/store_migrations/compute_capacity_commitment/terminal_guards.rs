use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_compute_capacity_commitment_terminal_projection
        BEFORE INSERT ON compute_capacity_commitment_terminal_receipts
        WHEN json_extract(NEW.terminal_receipt_json,'$.schema') IS NOT NEW.terminal_schema
          OR json_extract(NEW.terminal_receipt_json,'$.terminal_receipt_id')
                IS NOT NEW.terminal_receipt_id
          OR json_extract(NEW.terminal_receipt_json,'$.terminal_revision')
                IS NOT NEW.terminal_revision
          OR json_extract(NEW.terminal_receipt_json,'$.terminal_status')
                IS NOT NEW.terminal_status
          OR json_extract(NEW.terminal_receipt_json,'$.terminal_receipt_digest')
                IS NOT NEW.terminal_receipt_digest
          OR json_extract(NEW.terminal_receipt_json,'$.commitment_id')
                IS NOT NEW.commitment_id
          OR json_extract(NEW.terminal_receipt_json,'$.commitment_digest')
                IS NOT NEW.commitment_digest
          OR json_extract(NEW.terminal_receipt_json,'$.claim_id') IS NOT NEW.claim_id
          OR json_extract(NEW.terminal_receipt_json,'$.prior_claim_revision')
                IS NOT NEW.prior_claim_revision
          OR json_extract(NEW.terminal_receipt_json,'$.prior_claim_digest')
                IS NOT NEW.prior_claim_digest
          OR json_extract(NEW.terminal_receipt_json,'$.result_claim_revision')
                IS NOT NEW.result_claim_revision
          OR json_extract(NEW.terminal_receipt_json,'$.result_claim_digest')
                IS NOT NEW.result_claim_digest
          OR json_extract(NEW.terminal_receipt_json,'$.result_claim_state')
                IS NOT NEW.result_claim_state
          OR json_extract(NEW.terminal_receipt_json,'$.ledger.transaction_id')
                IS NOT NEW.terminal_transaction_id
          OR json_extract(NEW.terminal_receipt_json,'$.ledger.transaction_digest')
                IS NOT NEW.terminal_transaction_digest
          OR json_extract(NEW.terminal_receipt_json,'$.ledger.ledger_sequence')
                IS NOT NEW.terminal_ledger_sequence
          OR json_extract(NEW.terminal_receipt_json,'$.ledger.event_kind')
                IS NOT NEW.terminal_event_kind
          OR json_extract(NEW.terminal_receipt_json,'$.ledger.causal_transaction_id')
                IS NOT NEW.causal_transaction_id
          OR json_extract(NEW.terminal_receipt_json,'$.actor_kind') IS NOT NEW.actor_kind
          OR json_extract(NEW.terminal_receipt_json,'$.actor_id') IS NOT NEW.actor_id
          OR json_extract(NEW.terminal_receipt_json,'$.reason') IS NOT NEW.reason
          OR json_extract(NEW.terminal_receipt_json,'$.idempotency_scope')
                IS NOT NEW.idempotency_scope
          OR json_extract(NEW.terminal_receipt_json,'$.idempotency_key')
                IS NOT NEW.idempotency_key
          OR json_extract(NEW.terminal_receipt_json,'$.request_digest') IS NOT NEW.request_digest
          OR json_extract(NEW.terminal_receipt_json,'$.occurred_at') IS NOT NEW.occurred_at
          OR json_extract(NEW.terminal_receipt_json,'$.recorded_at') IS NOT NEW.recorded_at
          OR (SELECT COUNT(*) FROM json_each(NEW.terminal_receipt_json))<>22
          OR EXISTS (
                SELECT 1 FROM json_each(NEW.terminal_receipt_json)
                 WHERE key NOT IN ('schema','terminal_receipt_id','terminal_revision',
                    'terminal_receipt_digest','terminal_status','commitment_id',
                    'commitment_digest','claim_id','prior_claim_revision',
                    'prior_claim_digest','result_claim_revision','result_claim_digest',
                    'result_claim_state','ledger','actor_kind','actor_id','reason',
                    'idempotency_scope','idempotency_key','request_digest',
                    'occurred_at','recorded_at'))
          OR (SELECT COUNT(*)
                FROM json_each(NEW.terminal_receipt_json,'$.ledger'))<>5
          OR EXISTS (
                SELECT 1 FROM json_each(NEW.terminal_receipt_json,'$.ledger')
                 WHERE key NOT IN ('transaction_id','transaction_digest','ledger_sequence',
                    'event_kind','causal_transaction_id'))
        BEGIN
            SELECT RAISE(ABORT, 'capacity commitment terminal JSON projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_capacity_commitment_terminal_source
        BEFORE INSERT ON compute_capacity_commitment_terminal_receipts
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_capacity_commitments commitment
              JOIN compute_capacity_claim_versions prior_claim
                ON prior_claim.claim_id=commitment.claim_id
               AND prior_claim.revision=commitment.claim_revision
              JOIN compute_capacity_claim_versions result_claim
                ON result_claim.claim_id=commitment.claim_id
               AND result_claim.revision=NEW.result_claim_revision
              JOIN compute_capacity_claims claim ON claim.claim_id=commitment.claim_id
              JOIN compute_capacity_ledger_transactions terminal_transaction
                ON terminal_transaction.transaction_id=NEW.terminal_transaction_id
             WHERE commitment.commitment_id=NEW.commitment_id
               AND commitment.commitment_digest=NEW.commitment_digest
               AND commitment.commitment_revision=NEW.commitment_revision
               AND commitment.commitment_status='committed'
               AND commitment.claim_id=NEW.claim_id
               AND commitment.claim_revision=NEW.prior_claim_revision
               AND commitment.claim_digest=NEW.prior_claim_digest
               AND prior_claim.claim_digest=NEW.prior_claim_digest
               AND prior_claim.status='held'
               AND result_claim.claim_digest=NEW.result_claim_digest
               AND result_claim.status=NEW.result_claim_state
               AND result_claim.request_digest=prior_claim.request_digest
               AND claim.claim_digest=NEW.result_claim_digest
               AND claim.pool_id=commitment.pool_id
               AND claim.capacity_epoch=commitment.capacity_epoch
               AND claim.delivery_window_id=commitment.delivery_window_id
               AND claim.claim_kind='capacity_commitment'
               AND claim.subject_kind='compute_capacity_commitment'
               AND claim.subject_id=commitment.commitment_id
               AND claim.status=NEW.result_claim_state
               AND claim.revision=NEW.result_claim_revision
               AND claim.parent_claim_id IS NULL
               AND claim.created_at=commitment.created_at
               AND claim.updated_at=NEW.recorded_at
               AND claim.expires_at=commitment.expires_at
               AND claim.terminal_at=NEW.recorded_at
               AND json_extract(result_claim.claim_json,'$.schema')=
                    'compute_federation.capacity_claim.v1'
               AND json_extract(result_claim.claim_json,'$.claim_id')=NEW.claim_id
               AND json_extract(result_claim.claim_json,'$.claim_digest')=
                    NEW.result_claim_digest
               AND json_extract(result_claim.claim_json,'$.claim_kind')='capacity_commitment'
               AND json_extract(result_claim.claim_json,'$.state')=NEW.result_claim_state
               AND json_extract(result_claim.claim_json,'$.revision')=NEW.result_claim_revision
               AND json_type(result_claim.claim_json,'$.parent_claim_id')='null'
               AND json_extract(result_claim.claim_json,'$.subject_kind')=
                    'compute_capacity_commitment'
               AND json_extract(result_claim.claim_json,'$.subject_id')=NEW.commitment_id
               AND json_extract(result_claim.claim_json,'$.idempotency_scope')=
                    json_extract(prior_claim.claim_json,'$.idempotency_scope')
               AND json_extract(result_claim.claim_json,'$.idempotency_key')=
                    json_extract(prior_claim.claim_json,'$.idempotency_key')
               AND json_extract(result_claim.claim_json,'$.request_digest')=
                    prior_claim.request_digest
               AND json_extract(result_claim.claim_json,'$.pool.pool_id')=commitment.pool_id
               AND json_extract(result_claim.claim_json,'$.pool.capacity_epoch')=
                    commitment.capacity_epoch
               AND json_extract(result_claim.claim_json,'$.pool.pool_revision')=
                    commitment.pool_revision
               AND json_extract(result_claim.claim_json,'$.pool.pool_digest')=
                    commitment.pool_digest
               AND json_extract(result_claim.claim_json,'$.delivery_window.window_id')=
                    commitment.delivery_window_id
               AND json_extract(result_claim.claim_json,'$.delivery_window.window_digest')=
                    commitment.delivery_window_digest
               AND json_extract(result_claim.claim_json,'$.created_at')=commitment.created_at
               AND json_extract(result_claim.claim_json,'$.updated_at')=NEW.recorded_at
               AND json_extract(result_claim.claim_json,'$.expires_at')=commitment.expires_at
               AND json_extract(result_claim.claim_json,'$.terminal_at')=NEW.recorded_at
               AND (SELECT COUNT(*)
                      FROM json_each(result_claim.claim_json,'$.lines'))=
                   (SELECT COUNT(*) FROM compute_capacity_claim_lines line
                     WHERE line.claim_id=NEW.claim_id)
               AND NOT EXISTS (
                    SELECT 1
                      FROM compute_capacity_claim_lines line
                      JOIN compute_capacity_buckets bucket ON bucket.bucket_id=line.bucket_id
                     WHERE line.claim_id=NEW.claim_id
                       AND NOT EXISTS (
                            SELECT 1
                              FROM json_each(result_claim.claim_json,'$.lines') item
                             WHERE json_extract(item.value,'$.line_no')=line.line_no
                               AND json_extract(item.value,'$.bucket.bucket_id')=line.bucket_id
                               AND json_extract(item.value,'$.bucket.bucket_digest')=
                                    bucket.bucket_digest
                               AND json_extract(item.value,'$.bucket.pool.pool_id')=
                                    commitment.pool_id
                               AND json_extract(item.value,'$.bucket.pool.capacity_epoch')=
                                    commitment.capacity_epoch
                               AND json_extract(item.value,'$.bucket.pool.pool_revision')=
                                    commitment.pool_revision
                               AND json_extract(item.value,'$.bucket.pool.pool_digest')=
                                    commitment.pool_digest
                               AND json_extract(
                                    item.value,'$.bucket.delivery_window.window_id')=
                                    commitment.delivery_window_id
                               AND json_extract(
                                    item.value,'$.bucket.delivery_window.window_digest')=
                                    commitment.delivery_window_digest
                               AND json_extract(item.value,'$.bucket.meter')=line.meter
                               AND json_extract(item.value,'$.bucket.meter_mode')=
                                    bucket.meter_mode
                               AND json_extract(item.value,'$.bucket.quantum_units')=
                                    bucket.quantum_units
                               AND json_extract(item.value,'$.bucket.meter_policy_digest')=
                                    bucket.meter_policy_digest
                               AND json_extract(item.value,'$.quantity_units')=
                                    line.quantity_units))
               AND terminal_transaction.transaction_digest=
                    NEW.terminal_transaction_digest
               AND terminal_transaction.pool_id=commitment.pool_id
               AND terminal_transaction.capacity_epoch=commitment.capacity_epoch
               AND terminal_transaction.delivery_window_id=commitment.delivery_window_id
               AND terminal_transaction.ledger_sequence=NEW.terminal_ledger_sequence
               AND terminal_transaction.event_kind=NEW.terminal_event_kind
               AND terminal_transaction.claim_id=NEW.claim_id
               AND terminal_transaction.claim_effect=NEW.result_claim_state
               AND terminal_transaction.claim_effect_key=NEW.idempotency_key
               AND terminal_transaction.offer_id=commitment.offer_id
               AND terminal_transaction.offer_version=commitment.offer_version
               AND terminal_transaction.offer_digest=commitment.offer_digest
               AND terminal_transaction.job_id IS NULL
               AND terminal_transaction.reservation_id IS NULL
               AND terminal_transaction.attempt_lease_id IS NULL
               AND terminal_transaction.fencing_generation IS NULL
               AND terminal_transaction.subject_kind='compute_capacity_commitment'
               AND terminal_transaction.subject_id=NEW.commitment_id
               AND terminal_transaction.causal_transaction_id=commitment.hold_transaction_id
               AND terminal_transaction.idempotency_key=NEW.idempotency_key
               AND terminal_transaction.occurred_at=NEW.occurred_at
               AND terminal_transaction.recorded_at=NEW.recorded_at
               AND NEW.causal_transaction_id=commitment.hold_transaction_id
               AND ((NEW.terminal_status='canceled'
                    AND NEW.actor_id=commitment.owner_account_id
                    AND julianday(NEW.recorded_at)<
                        julianday(commitment.delivery_window_starts_at))
                 OR (NEW.terminal_status='expired'
                    AND NEW.occurred_at=commitment.expires_at
                    AND julianday(NEW.recorded_at)>=julianday(commitment.expires_at)))
               AND (SELECT COUNT(*) FROM compute_capacity_ledger_legs leg
                     WHERE leg.transaction_id=NEW.terminal_transaction_id)=
                   2*(SELECT COUNT(*) FROM compute_capacity_claim_lines line
                       WHERE line.claim_id=NEW.claim_id)
               AND NOT EXISTS (
                    SELECT 1 FROM compute_capacity_claim_lines line
                     WHERE line.claim_id=NEW.claim_id
                       AND (NOT EXISTS (
                            SELECT 1 FROM compute_capacity_ledger_legs leg
                             WHERE leg.transaction_id=NEW.terminal_transaction_id
                               AND leg.line_no=line.line_no AND leg.leg_role='from'
                               AND leg.bucket_id=line.bucket_id AND leg.meter=line.meter
                               AND leg.account='held'
                               AND leg.delta_units=-line.quantity_units)
                         OR NOT EXISTS (
                            SELECT 1 FROM compute_capacity_ledger_legs leg
                             WHERE leg.transaction_id=NEW.terminal_transaction_id
                               AND leg.line_no=line.line_no AND leg.leg_role='to'
                               AND leg.bucket_id=line.bucket_id AND leg.meter=line.meter
                               AND leg.account='available'
                               AND leg.delta_units=line.quantity_units)))
        )
        BEGIN
            SELECT RAISE(ABORT, 'capacity commitment terminal lacks exact Claim and ledger source');
        END;
        "#,
    )?;
    Ok(())
}
