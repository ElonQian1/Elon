use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_compute_capacity_commitment_claim_source
        BEFORE INSERT ON compute_capacity_commitments
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_capacity_claims claim
              JOIN compute_capacity_claim_versions claim_version
                ON claim_version.claim_id=claim.claim_id
               AND claim_version.revision=NEW.claim_revision
              JOIN compute_capacity_ledger_transactions hold_transaction
                ON hold_transaction.transaction_id=NEW.hold_transaction_id
             WHERE claim.claim_id=NEW.claim_id
               AND claim.claim_digest=NEW.claim_digest
               AND claim.pool_id=NEW.pool_id
               AND claim.capacity_epoch=NEW.capacity_epoch
               AND claim.delivery_window_id=NEW.delivery_window_id
               AND claim.claim_kind='capacity_commitment'
               AND claim.subject_kind='compute_capacity_commitment'
               AND claim.subject_id=NEW.commitment_id
               AND claim.status='held'
               AND claim.revision=1
               AND claim.parent_claim_id IS NULL
               AND claim.created_at=NEW.created_at
               AND claim.updated_at=NEW.created_at
               AND claim.expires_at=NEW.expires_at
               AND claim.terminal_at IS NULL
               AND claim_version.claim_digest=NEW.claim_digest
               AND claim_version.status='held'
               AND claim_version.request_digest=claim.request_digest
               AND json_extract(claim_version.claim_json,'$.schema')=
                    'compute_federation.capacity_claim.v1'
               AND json_extract(claim_version.claim_json,'$.claim_id')=NEW.claim_id
               AND json_extract(claim_version.claim_json,'$.claim_digest')=NEW.claim_digest
               AND json_extract(claim_version.claim_json,'$.claim_kind')='capacity_commitment'
               AND json_extract(claim_version.claim_json,'$.state')='held'
               AND json_extract(claim_version.claim_json,'$.revision')=1
               AND json_type(claim_version.claim_json,'$.parent_claim_id')='null'
               AND json_extract(claim_version.claim_json,'$.subject_kind')=
                    'compute_capacity_commitment'
               AND json_extract(claim_version.claim_json,'$.subject_id')=NEW.commitment_id
               AND json_extract(claim_version.claim_json,'$.idempotency_scope')=
                    claim.idempotency_scope
               AND json_extract(claim_version.claim_json,'$.idempotency_key')=
                    claim.idempotency_key
               AND json_extract(claim_version.claim_json,'$.request_digest')=
                    claim.request_digest
               AND json_extract(claim_version.claim_json,'$.pool.pool_id')=NEW.pool_id
               AND json_extract(claim_version.claim_json,'$.pool.capacity_epoch')=
                    NEW.capacity_epoch
               AND json_extract(claim_version.claim_json,'$.pool.pool_revision')=
                    NEW.pool_revision
               AND json_extract(claim_version.claim_json,'$.pool.pool_digest')=NEW.pool_digest
               AND json_extract(claim_version.claim_json,'$.delivery_window.window_id')=
                    NEW.delivery_window_id
               AND json_extract(claim_version.claim_json,'$.delivery_window.window_digest')=
                    NEW.delivery_window_digest
               AND json_extract(claim_version.claim_json,'$.created_at')=NEW.created_at
               AND json_extract(claim_version.claim_json,'$.updated_at')=NEW.created_at
               AND json_extract(claim_version.claim_json,'$.expires_at')=NEW.expires_at
               AND json_type(claim_version.claim_json,'$.terminal_at')='null'
               AND (SELECT COUNT(*)
                      FROM json_each(claim_version.claim_json,'$.lines'))=
                   (SELECT COUNT(*) FROM compute_capacity_claim_lines line
                     WHERE line.claim_id=NEW.claim_id)
               AND NOT EXISTS (
                    SELECT 1
                      FROM compute_capacity_claim_lines line
                      JOIN compute_capacity_buckets bucket ON bucket.bucket_id=line.bucket_id
                     WHERE line.claim_id=NEW.claim_id
                       AND NOT EXISTS (
                            SELECT 1
                              FROM json_each(claim_version.claim_json,'$.lines') item
                             WHERE json_extract(item.value,'$.line_no')=line.line_no
                               AND json_extract(item.value,'$.bucket.bucket_id')=line.bucket_id
                               AND json_extract(item.value,'$.bucket.bucket_digest')=
                                    bucket.bucket_digest
                               AND json_extract(item.value,'$.bucket.pool.pool_id')=NEW.pool_id
                               AND json_extract(item.value,'$.bucket.pool.capacity_epoch')=
                                    NEW.capacity_epoch
                               AND json_extract(item.value,'$.bucket.pool.pool_revision')=
                                    NEW.pool_revision
                               AND json_extract(item.value,'$.bucket.pool.pool_digest')=
                                    NEW.pool_digest
                               AND json_extract(
                                    item.value,'$.bucket.delivery_window.window_id')=
                                    NEW.delivery_window_id
                               AND json_extract(
                                    item.value,'$.bucket.delivery_window.window_digest')=
                                    NEW.delivery_window_digest
                               AND json_extract(item.value,'$.bucket.meter')=line.meter
                               AND json_extract(item.value,'$.bucket.meter_mode')=
                                    bucket.meter_mode
                               AND json_extract(item.value,'$.bucket.quantum_units')=
                                    bucket.quantum_units
                               AND json_extract(item.value,'$.bucket.meter_policy_digest')=
                                    bucket.meter_policy_digest
                               AND json_extract(item.value,'$.quantity_units')=
                                    line.quantity_units))
               AND hold_transaction.transaction_digest=NEW.hold_transaction_digest
               AND hold_transaction.pool_id=NEW.pool_id
               AND hold_transaction.capacity_epoch=NEW.capacity_epoch
               AND hold_transaction.delivery_window_id=NEW.delivery_window_id
               AND hold_transaction.ledger_sequence=NEW.hold_ledger_sequence
               AND hold_transaction.event_kind=NEW.hold_event_kind
               AND hold_transaction.claim_id=NEW.claim_id
               AND hold_transaction.claim_effect='held'
               AND hold_transaction.claim_effect_key=claim.idempotency_key
               AND hold_transaction.offer_id=NEW.offer_id
               AND hold_transaction.offer_version=NEW.offer_version
               AND hold_transaction.offer_digest=NEW.offer_digest
               AND hold_transaction.job_id IS NULL
               AND hold_transaction.reservation_id IS NULL
               AND hold_transaction.attempt_lease_id IS NULL
               AND hold_transaction.fencing_generation IS NULL
               AND hold_transaction.request_digest=claim.request_digest
               AND hold_transaction.subject_kind='compute_capacity_commitment'
               AND hold_transaction.subject_id=NEW.commitment_id
               AND hold_transaction.causal_transaction_id IS NULL
               AND hold_transaction.recorded_at=NEW.created_at
               AND julianday(hold_transaction.occurred_at) IS NOT NULL
               AND julianday(hold_transaction.occurred_at)<=
                    julianday(hold_transaction.recorded_at)
               AND (hold_transaction.occurred_at GLOB '*Z'
                    OR hold_transaction.occurred_at GLOB '*+00:00')
               AND (SELECT COUNT(*) FROM compute_capacity_ledger_legs leg
                     WHERE leg.transaction_id=NEW.hold_transaction_id)=
                   2*(SELECT COUNT(*) FROM compute_capacity_claim_lines line
                       WHERE line.claim_id=NEW.claim_id)
               AND NOT EXISTS (
                    SELECT 1 FROM compute_capacity_claim_lines line
                     WHERE line.claim_id=NEW.claim_id
                       AND (NOT EXISTS (
                            SELECT 1 FROM compute_capacity_ledger_legs leg
                             WHERE leg.transaction_id=NEW.hold_transaction_id
                               AND leg.line_no=line.line_no AND leg.leg_role='from'
                               AND leg.bucket_id=line.bucket_id AND leg.meter=line.meter
                               AND leg.account='available'
                               AND leg.delta_units=-line.quantity_units)
                         OR NOT EXISTS (
                            SELECT 1 FROM compute_capacity_ledger_legs leg
                             WHERE leg.transaction_id=NEW.hold_transaction_id
                               AND leg.line_no=line.line_no AND leg.leg_role='to'
                               AND leg.bucket_id=line.bucket_id AND leg.meter=line.meter
                               AND leg.account='held'
                               AND leg.delta_units=line.quantity_units)))
        )
        BEGIN
            SELECT RAISE(ABORT, 'capacity commitment lacks exact held Claim and ledger source');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_capacity_commitment_quantity_source
        BEFORE INSERT ON compute_capacity_commitments
        WHEN (SELECT COUNT(*) FROM compute_capacity_claim_lines line
               WHERE line.claim_id=NEW.claim_id)=0
          OR (SELECT COUNT(*) FROM compute_capacity_claim_lines line
               WHERE line.claim_id=NEW.claim_id)<>
             (SELECT COUNT(*)
                FROM compute_offer_versions offer_version,
                     json_each(offer_version.offer_json,'$.capacity') capacity
               WHERE offer_version.offer_id=NEW.offer_id
                 AND offer_version.offer_version=NEW.offer_version
                 AND json_extract(capacity.value,'$.bucket.delivery_window.window_id')=
                      NEW.delivery_window_id
                 AND json_extract(capacity.value,'$.bucket.delivery_window.window_digest')=
                      NEW.delivery_window_digest)
          OR (SELECT COUNT(*) FROM compute_capacity_claim_lines line
               WHERE line.claim_id=NEW.claim_id)<>
             (SELECT COUNT(*)
                FROM compute_price_snapshots snapshot,
                     json_each(snapshot.snapshot_json,'$.components') component
               WHERE snapshot.snapshot_id=NEW.price_snapshot_id)
          OR EXISTS (
                SELECT 1
                  FROM compute_capacity_claim_lines line
                  JOIN compute_capacity_buckets bucket ON bucket.bucket_id=line.bucket_id
                 WHERE line.claim_id=NEW.claim_id
                   AND (line.meter<>bucket.meter
                     OR bucket.pool_id<>NEW.pool_id
                     OR bucket.capacity_epoch<>NEW.capacity_epoch
                     OR bucket.pool_revision<>NEW.pool_revision
                     OR bucket.delivery_window_id<>NEW.delivery_window_id
                     OR bucket.delivery_window_digest<>NEW.delivery_window_digest
                     OR bucket.delivery_window_starts_at<>NEW.delivery_window_starts_at
                     OR bucket.delivery_window_ends_at<>NEW.delivery_window_ends_at
                     OR bucket.status<>'open'
                     OR line.quantity_units<=0
                     OR line.quantity_units%bucket.quantum_units<>0
                     OR NOT EXISTS (
                        SELECT 1
                          FROM compute_offer_versions offer_version,
                               json_each(offer_version.offer_json,'$.capacity') capacity
                         WHERE offer_version.offer_id=NEW.offer_id
                           AND offer_version.offer_version=NEW.offer_version
                           AND json_extract(capacity.value,'$.bucket.bucket_id')=line.bucket_id
                           AND json_extract(capacity.value,'$.bucket.bucket_digest')=
                                bucket.bucket_digest
                           AND json_extract(capacity.value,'$.bucket.delivery_window.window_id')=
                                NEW.delivery_window_id
                           AND json_extract(capacity.value,'$.bucket.delivery_window.window_digest')=
                                NEW.delivery_window_digest
                           AND json_extract(capacity.value,'$.bucket.meter')=line.meter
                           AND line.quantity_units<=
                                json_extract(capacity.value,'$.reservable_units')
                           AND (SELECT COALESCE(SUM(live_line.quantity_units),0)
                                  FROM compute_capacity_claims live_claim
                                  JOIN compute_capacity_claim_lines live_line
                                    ON live_line.claim_id=live_claim.claim_id
                                  JOIN compute_capacity_ledger_transactions live_hold
                                    ON live_hold.claim_id=live_claim.claim_id
                                   AND live_hold.claim_effect='held'
                                 WHERE live_claim.status IN ('held','active')
                                   AND live_claim.claim_kind IN (
                                        'quote_hold','reservation','capacity_commitment')
                                   AND live_line.bucket_id=line.bucket_id
                                   AND live_hold.offer_id=NEW.offer_id)
                                <=json_extract(capacity.value,'$.reservable_units'))
                     OR NOT EXISTS (
                        SELECT 1
                          FROM compute_price_snapshots snapshot,
                               json_each(snapshot.snapshot_json,'$.components') component
                         WHERE snapshot.snapshot_id=NEW.price_snapshot_id
                           AND json_extract(component.value,'$.meter')=line.meter
                           AND json_extract(component.value,'$.unit_size')=
                                bucket.quantum_units
                           AND line.quantity_units<=
                                json_extract(component.value,'$.max_units'))))
        BEGIN
            SELECT RAISE(ABORT, 'capacity commitment quantity source is not exact');
        END;
        "#,
    )?;
    Ok(())
}
