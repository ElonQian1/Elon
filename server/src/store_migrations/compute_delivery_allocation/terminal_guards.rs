use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_delivery_allocation_terminal_projection
        BEFORE INSERT ON compute_delivery_allocation_terminal_receipts
        WHEN json_extract(NEW.terminal_receipt_json,'$.schema') IS NOT NEW.terminal_schema
          OR json_extract(NEW.terminal_receipt_json,'$.terminal_receipt_id')
                IS NOT NEW.terminal_receipt_id
          OR json_extract(NEW.terminal_receipt_json,'$.terminal_revision')
                IS NOT NEW.terminal_revision
          OR json_extract(NEW.terminal_receipt_json,'$.terminal_receipt_digest')
                IS NOT NEW.terminal_receipt_digest
          OR json_extract(NEW.terminal_receipt_json,'$.terminal_status')
                IS NOT NEW.terminal_status
          OR json_extract(NEW.terminal_receipt_json,'$.grant_id') IS NOT NEW.grant_id
          OR json_extract(NEW.terminal_receipt_json,'$.grant_digest') IS NOT NEW.grant_digest
          OR json_extract(NEW.terminal_receipt_json,'$.commitment.commitment_id')
                IS NOT NEW.commitment_id
          OR json_extract(NEW.terminal_receipt_json,'$.commitment.commitment_revision')
                IS NOT NEW.commitment_revision
          OR json_extract(NEW.terminal_receipt_json,'$.commitment.commitment_digest')
                IS NOT NEW.commitment_digest
          OR json_extract(NEW.terminal_receipt_json,'$.actor_kind') IS NOT NEW.actor_kind
          OR json_extract(NEW.terminal_receipt_json,'$.actor_id') IS NOT NEW.actor_id
          OR json_extract(NEW.terminal_receipt_json,'$.idempotency_scope')
                IS NOT NEW.idempotency_scope
          OR json_extract(NEW.terminal_receipt_json,'$.idempotency_key')
                IS NOT NEW.idempotency_key
          OR json_extract(NEW.terminal_receipt_json,'$.request_digest') IS NOT NEW.request_digest
          OR json_extract(NEW.terminal_receipt_json,'$.occurred_at') IS NOT NEW.occurred_at
          OR json_extract(NEW.terminal_receipt_json,'$.recorded_at') IS NOT NEW.recorded_at
          OR (SELECT COUNT(*) FROM json_each(NEW.terminal_receipt_json))<>16
          OR (SELECT COUNT(DISTINCT key) FROM json_each(NEW.terminal_receipt_json))<>16
          OR EXISTS (SELECT 1 FROM json_each(NEW.terminal_receipt_json) WHERE key NOT IN (
                'schema','terminal_receipt_id','terminal_revision','terminal_receipt_digest',
                'terminal_status','grant_id','grant_digest','commitment','actor_kind','actor_id',
                'exercise','idempotency_scope','idempotency_key','request_digest',
                'occurred_at','recorded_at'))
          OR json_type(NEW.terminal_receipt_json,'$.commitment') IS NOT 'object'
          OR (SELECT COUNT(*) FROM json_each(
                NEW.terminal_receipt_json,'$.commitment'))<>3
          OR EXISTS (SELECT 1 FROM json_each(
                NEW.terminal_receipt_json,'$.commitment') WHERE key NOT IN (
                'commitment_id','commitment_revision','commitment_digest'))
          OR (NEW.terminal_status='exercised' AND (
                json_type(NEW.terminal_receipt_json,'$.exercise') IS NOT 'object'
                OR json_extract(NEW.terminal_receipt_json,'$.exercise.parent_claim_id')
                    IS NOT NEW.parent_claim_id
                OR json_extract(NEW.terminal_receipt_json,
                    '$.exercise.parent_prior_claim_revision')
                    IS NOT NEW.parent_prior_claim_revision
                OR json_extract(NEW.terminal_receipt_json,
                    '$.exercise.parent_prior_claim_digest') IS NOT NEW.parent_prior_claim_digest
                OR json_extract(NEW.terminal_receipt_json,
                    '$.exercise.parent_result_claim_revision')
                    IS NOT NEW.parent_result_claim_revision
                OR json_extract(NEW.terminal_receipt_json,
                    '$.exercise.parent_result_claim_digest')
                    IS NOT NEW.parent_result_claim_digest
                OR json_extract(NEW.terminal_receipt_json,
                    '$.exercise.parent_result_claim_state') IS NOT NEW.parent_result_claim_state
                OR json_extract(NEW.terminal_receipt_json,
                    '$.exercise.parent_release_ledger.transaction_id')
                    IS NOT NEW.parent_release_transaction_id
                OR json_extract(NEW.terminal_receipt_json,
                    '$.exercise.parent_release_ledger.transaction_digest')
                    IS NOT NEW.parent_release_transaction_digest
                OR json_extract(NEW.terminal_receipt_json,
                    '$.exercise.parent_release_ledger.ledger_sequence')
                    IS NOT NEW.parent_release_ledger_sequence
                OR json_extract(NEW.terminal_receipt_json,
                    '$.exercise.parent_release_ledger.event_kind')
                    IS NOT NEW.parent_release_event_kind
                OR json_extract(NEW.terminal_receipt_json,
                    '$.exercise.parent_release_ledger.causal_transaction_id')
                    IS NOT NEW.parent_release_causal_transaction_id
                OR json_extract(NEW.terminal_receipt_json,
                    '$.exercise.reservation_claim.claim_id') IS NOT NEW.reservation_claim_id
                OR json_extract(NEW.terminal_receipt_json,
                    '$.exercise.reservation_claim.claim_revision')
                    IS NOT NEW.reservation_claim_revision
                OR json_extract(NEW.terminal_receipt_json,
                    '$.exercise.reservation_claim.claim_digest')
                    IS NOT NEW.reservation_claim_digest
                OR json_extract(NEW.terminal_receipt_json,
                    '$.exercise.reservation_claim.parent_claim_id')
                    IS NOT NEW.reservation_parent_claim_id
                OR json_extract(NEW.terminal_receipt_json,
                    '$.exercise.reservation_hold_ledger.transaction_id')
                    IS NOT NEW.reservation_hold_transaction_id
                OR json_extract(NEW.terminal_receipt_json,
                    '$.exercise.reservation_hold_ledger.transaction_digest')
                    IS NOT NEW.reservation_hold_transaction_digest
                OR json_extract(NEW.terminal_receipt_json,
                    '$.exercise.reservation_hold_ledger.ledger_sequence')
                    IS NOT NEW.reservation_hold_ledger_sequence
                OR json_extract(NEW.terminal_receipt_json,
                    '$.exercise.reservation_hold_ledger.event_kind')
                    IS NOT NEW.reservation_hold_event_kind
                OR json_extract(NEW.terminal_receipt_json,
                    '$.exercise.reservation_hold_ledger.causal_transaction_id')
                    IS NOT NEW.reservation_hold_causal_transaction_id
                OR json_extract(NEW.terminal_receipt_json,
                    '$.exercise.reservation.reservation_id') IS NOT NEW.reservation_id
                OR json_extract(NEW.terminal_receipt_json,
                    '$.exercise.reservation.reservation_revision')
                    IS NOT NEW.reservation_revision
                OR json_extract(NEW.terminal_receipt_json,
                    '$.exercise.reservation.reservation_digest') IS NOT NEW.reservation_digest
                OR json_extract(NEW.terminal_receipt_json,'$.exercise.source_job_revision')
                    IS NOT NEW.source_job_revision
                OR json_extract(NEW.terminal_receipt_json,'$.exercise.source_job_digest')
                    IS NOT NEW.source_job_digest
                OR json_extract(NEW.terminal_receipt_json,'$.exercise.reserved_job_revision')
                    IS NOT NEW.reserved_job_revision
                OR json_extract(NEW.terminal_receipt_json,'$.exercise.reserved_job_digest')
                    IS NOT NEW.reserved_job_digest
                OR json_extract(NEW.terminal_receipt_json,'$.exercise.budget_reservation_id')
                    IS NOT NEW.budget_reservation_id
                OR json_extract(NEW.terminal_receipt_json,'$.exercise.reserved_amount_fen')
                    IS NOT NEW.reserved_amount_fen
                OR json_extract(NEW.terminal_receipt_json,
                    '$.exercise.broker_reserve_request_digest')
                    IS NOT NEW.broker_reserve_request_digest
                OR (SELECT COUNT(*) FROM json_each(
                    NEW.terminal_receipt_json,'$.exercise'))<>17
                OR EXISTS (SELECT 1 FROM json_each(
                    NEW.terminal_receipt_json,'$.exercise') WHERE key NOT IN (
                    'parent_claim_id','parent_prior_claim_revision','parent_prior_claim_digest',
                    'parent_result_claim_revision','parent_result_claim_digest',
                    'parent_result_claim_state','parent_release_ledger','reservation_claim',
                    'reservation_hold_ledger','reservation','source_job_revision',
                    'source_job_digest','reserved_job_revision','reserved_job_digest',
                    'budget_reservation_id','reserved_amount_fen',
                    'broker_reserve_request_digest'))
                OR (SELECT COUNT(*) FROM json_each(NEW.terminal_receipt_json,
                    '$.exercise.parent_release_ledger'))<>5
                OR EXISTS (SELECT 1 FROM json_each(NEW.terminal_receipt_json,
                    '$.exercise.parent_release_ledger') WHERE key NOT IN (
                    'transaction_id','transaction_digest','ledger_sequence','event_kind',
                    'causal_transaction_id'))
                OR (SELECT COUNT(*) FROM json_each(NEW.terminal_receipt_json,
                    '$.exercise.reservation_claim'))<>4
                OR EXISTS (SELECT 1 FROM json_each(NEW.terminal_receipt_json,
                    '$.exercise.reservation_claim') WHERE key NOT IN (
                    'claim_id','claim_revision','claim_digest','parent_claim_id'))
                OR (SELECT COUNT(*) FROM json_each(NEW.terminal_receipt_json,
                    '$.exercise.reservation_hold_ledger'))<>5
                OR EXISTS (SELECT 1 FROM json_each(NEW.terminal_receipt_json,
                    '$.exercise.reservation_hold_ledger') WHERE key NOT IN (
                    'transaction_id','transaction_digest','ledger_sequence','event_kind',
                    'causal_transaction_id'))
                OR (SELECT COUNT(*) FROM json_each(NEW.terminal_receipt_json,
                    '$.exercise.reservation'))<>3
                OR EXISTS (SELECT 1 FROM json_each(NEW.terminal_receipt_json,
                    '$.exercise.reservation') WHERE key NOT IN (
                    'reservation_id','reservation_revision','reservation_digest'))))
          OR (NEW.terminal_status IN ('declined','expired')
                AND json_type(NEW.terminal_receipt_json,'$.exercise') IS NOT 'null')
        BEGIN
            SELECT RAISE(ABORT, 'delivery allocation terminal JSON projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_delivery_allocation_terminal_common_source
        BEFORE INSERT ON compute_delivery_allocation_terminal_receipts
        WHEN NOT EXISTS (
            SELECT 1 FROM compute_delivery_allocation_grants grant_row
              JOIN compute_capacity_commitments commitment
                ON commitment.commitment_id=grant_row.commitment_id
               AND commitment.commitment_digest=grant_row.commitment_digest
             WHERE grant_row.grant_id=NEW.grant_id AND grant_row.grant_digest=NEW.grant_digest
               AND grant_row.grant_revision=1 AND grant_row.grant_status='granted'
               AND grant_row.commitment_id=NEW.commitment_id
               AND grant_row.commitment_revision=NEW.commitment_revision
               AND grant_row.commitment_digest=NEW.commitment_digest
               AND commitment.commitment_status='committed'
               AND NOT EXISTS (SELECT 1
                    FROM compute_capacity_commitment_terminal_receipts old_terminal
                   WHERE old_terminal.commitment_id=NEW.commitment_id)
               AND ((NEW.terminal_status IN ('exercised','declined')
                    AND NEW.actor_kind='consumer'
                    AND NEW.actor_id=grant_row.consumer_account_id
                    AND NEW.occurred_at=NEW.recorded_at
                    AND julianday(NEW.recorded_at)<julianday(grant_row.exercise_expires_at))
                 OR (NEW.terminal_status='expired' AND NEW.actor_kind='admin'
                    AND NEW.occurred_at=grant_row.exercise_expires_at
                    AND julianday(NEW.recorded_at)>=julianday(grant_row.exercise_expires_at)))
               AND (NEW.terminal_status='exercised' OR EXISTS (
                    SELECT 1 FROM compute_capacity_claims parent
                     WHERE parent.claim_id=commitment.claim_id AND parent.claim_digest=commitment.claim_digest
                       AND parent.revision=1 AND parent.status='held' AND parent.terminal_at IS NULL
                       AND parent.claim_kind='capacity_commitment' AND parent.subject_kind='compute_capacity_commitment' AND parent.subject_id=commitment.commitment_id AND parent.parent_claim_id IS NULL AND parent.pool_id=commitment.pool_id AND parent.capacity_epoch=commitment.capacity_epoch AND parent.delivery_window_id=commitment.delivery_window_id))
        )
        BEGIN
            SELECT RAISE(ABORT, 'delivery allocation terminal lacks exact active Grant source');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_delivery_allocation_terminal_exercise_source
        BEFORE INSERT ON compute_delivery_allocation_terminal_receipts
        WHEN NEW.terminal_status='exercised' AND NOT EXISTS (
            SELECT 1
              FROM compute_delivery_allocation_grants grant_row
              JOIN compute_capacity_commitments commitment
                ON commitment.commitment_id=grant_row.commitment_id
              JOIN compute_capacity_claim_versions parent_prior
                ON parent_prior.claim_id=NEW.parent_claim_id
               AND parent_prior.revision=NEW.parent_prior_claim_revision
              JOIN compute_capacity_claim_versions parent_result
                ON parent_result.claim_id=NEW.parent_claim_id
               AND parent_result.revision=NEW.parent_result_claim_revision
              JOIN compute_capacity_claims parent ON parent.claim_id=NEW.parent_claim_id
              JOIN compute_capacity_ledger_transactions parent_release
                ON parent_release.transaction_id=NEW.parent_release_transaction_id
              JOIN compute_capacity_claim_versions child_version
                ON child_version.claim_id=NEW.reservation_claim_id
               AND child_version.revision=NEW.reservation_claim_revision
              JOIN compute_capacity_claims child ON child.claim_id=NEW.reservation_claim_id
              JOIN compute_capacity_ledger_transactions child_hold
                ON child_hold.transaction_id=NEW.reservation_hold_transaction_id
              JOIN compute_reservations reservation
                ON reservation.reservation_id=NEW.reservation_id
              JOIN compute_reservation_versions reservation_version
                ON reservation_version.reservation_id=NEW.reservation_id
               AND reservation_version.revision=NEW.reservation_revision
              JOIN compute_jobs job ON job.job_id=grant_row.job_id
              JOIN compute_job_versions source_job
                ON source_job.job_id=grant_row.job_id
               AND source_job.revision=NEW.source_job_revision
              JOIN compute_job_versions reserved_job
                ON reserved_job.job_id=grant_row.job_id
               AND reserved_job.revision=NEW.reserved_job_revision
              JOIN compute_broker_reserve_receipts broker
                ON broker.reservation_id=NEW.reservation_id
              JOIN billing_reservations budget ON budget.id=NEW.budget_reservation_id
             WHERE grant_row.grant_id=NEW.grant_id AND grant_row.grant_digest=NEW.grant_digest
               AND commitment.commitment_id=NEW.commitment_id
               AND commitment.commitment_revision=NEW.commitment_revision
               AND commitment.commitment_digest=NEW.commitment_digest
               AND commitment.claim_id=NEW.parent_claim_id
               AND EXISTS (SELECT 1 FROM compute_providers provider WHERE
                    provider.provider_id=commitment.provider_id AND provider.owner_account_id=commitment.owner_account_id
                    AND provider.status IN ('active','draining'))
               AND EXISTS (SELECT 1 FROM compute_offers offer WHERE
                    offer.offer_id=commitment.offer_id AND offer.provider_id=commitment.provider_id
                    AND offer.capacity_pool_id=commitment.pool_id
                    AND offer.status IN ('active','draining'))
               AND EXISTS (SELECT 1 FROM compute_capacity_pools pool WHERE
                    pool.pool_id=commitment.pool_id AND pool.provider_id=commitment.provider_id AND pool.status IN ('active','draining')
                    AND pool.current_capacity_epoch=commitment.capacity_epoch)
               AND parent_prior.claim_digest=NEW.parent_prior_claim_digest
               AND parent_prior.status='held'
               AND parent_result.claim_digest=NEW.parent_result_claim_digest
               AND parent_result.status='released'
               AND parent.claim_digest=NEW.parent_result_claim_digest
               AND parent.revision=2 AND parent.status='released'
               AND parent.claim_kind='capacity_commitment'
               AND parent.subject_kind='compute_capacity_commitment'
               AND parent.subject_id=NEW.commitment_id AND parent.parent_claim_id IS NULL
               AND parent_release.transaction_digest=NEW.parent_release_transaction_digest
               AND parent_release.ledger_sequence=NEW.parent_release_ledger_sequence
               AND parent_release.event_kind='reservation_released'
               AND parent_release.claim_id=NEW.parent_claim_id
               AND parent_release.claim_effect='released'
               AND parent_release.pool_id=commitment.pool_id
               AND parent_release.capacity_epoch=commitment.capacity_epoch
               AND parent_release.delivery_window_id=commitment.delivery_window_id
               AND parent_release.offer_id=commitment.offer_id
               AND parent_release.offer_version=commitment.offer_version
               AND parent_release.offer_digest=commitment.offer_digest
               AND parent_release.job_id IS NULL AND parent_release.reservation_id IS NULL
               AND parent_release.subject_kind='compute_capacity_commitment'
               AND parent_release.subject_id=NEW.commitment_id
               AND parent_release.causal_transaction_id=commitment.hold_transaction_id
               AND NEW.parent_release_causal_transaction_id=commitment.hold_transaction_id
               AND child_version.claim_digest=NEW.reservation_claim_digest
               AND child_version.status='held'
               AND child.claim_digest=NEW.reservation_claim_digest
               AND child.revision=1 AND child.status='held'
               AND child.claim_kind='reservation' AND child.subject_kind='compute_reservation'
               AND child.subject_id=NEW.reservation_id
               AND child.parent_claim_id=NEW.parent_claim_id
               AND child.pool_id=commitment.pool_id
               AND child.capacity_epoch=commitment.capacity_epoch
               AND child.delivery_window_id=commitment.delivery_window_id
               AND child.terminal_at IS NULL AND child.expires_at=reservation.expires_at
               AND child_hold.transaction_digest=NEW.reservation_hold_transaction_digest
               AND child_hold.ledger_sequence=NEW.reservation_hold_ledger_sequence
               AND child_hold.event_kind='reservation_held'
               AND child_hold.claim_id=NEW.reservation_claim_id
               AND child_hold.claim_effect='held'
               AND child_hold.pool_id=commitment.pool_id
               AND child_hold.capacity_epoch=commitment.capacity_epoch
               AND child_hold.delivery_window_id=commitment.delivery_window_id
               AND child_hold.offer_id=commitment.offer_id
               AND child_hold.offer_version=commitment.offer_version
               AND child_hold.offer_digest=commitment.offer_digest
               AND child_hold.job_id=grant_row.job_id
               AND child_hold.reservation_id=NEW.reservation_id
               AND child_hold.subject_kind='compute_reservation'
               AND child_hold.subject_id=NEW.reservation_id
               AND child_hold.causal_transaction_id=NEW.parent_release_transaction_id
               AND reservation.consumer_account_id=grant_row.consumer_account_id
               AND reservation.current_revision=NEW.reservation_revision
               AND reservation.current_reservation_digest=NEW.reservation_digest
               AND reservation.status='active' AND reservation.job_id=grant_row.job_id
               AND reservation.job_revision=NEW.reserved_job_revision
               AND reservation.job_digest=NEW.reserved_job_digest
               AND reservation.provider_id=commitment.provider_id
               AND reservation.offer_id=commitment.offer_id
               AND reservation.offer_version=commitment.offer_version
               AND reservation.offer_digest=commitment.offer_digest
               AND reservation.price_snapshot_id=commitment.price_snapshot_id
               AND reservation.capacity_claim_id=NEW.reservation_claim_id
               AND reservation.capacity_claim_revision=NEW.reservation_claim_revision
               AND reservation.capacity_claim_digest=NEW.reservation_claim_digest
               AND reservation.consumer_authorization_ref=NEW.budget_reservation_id
               AND reservation.expires_at=json_extract(
                    reserved_job.job_json,'$.workload.deadline_at')
               AND julianday(reservation.expires_at)<=
                    julianday(commitment.delivery_window_ends_at)
               AND reservation_version.reservation_digest=NEW.reservation_digest
               AND reservation_version.status='active'
               AND reservation_version.job_id=grant_row.job_id
               AND reservation_version.job_revision=NEW.reserved_job_revision
               AND reservation_version.job_digest=NEW.reserved_job_digest
               AND reservation_version.capacity_claim_id=NEW.reservation_claim_id
               AND reservation_version.capacity_claim_revision=1
               AND reservation_version.capacity_claim_digest=NEW.reservation_claim_digest
               AND source_job.job_digest=NEW.source_job_digest
               AND source_job.status='quoted'
               AND NEW.source_job_revision=grant_row.job_revision
               AND NEW.source_job_digest=grant_row.job_digest
               AND reserved_job.job_digest=NEW.reserved_job_digest
               AND reserved_job.status='reserved'
               AND job.current_revision=NEW.reserved_job_revision
               AND job.current_job_digest=NEW.reserved_job_digest AND job.status='reserved'
               AND broker.consumer_account_id=grant_row.consumer_account_id
               AND broker.request_digest=NEW.broker_reserve_request_digest
               AND broker.budget_adapter='platform_balance_cny'
               AND broker.budget_reservation_id=NEW.budget_reservation_id
               AND broker.budget_reserved_fen=NEW.reserved_amount_fen
               AND broker.capacity_claim_id=NEW.reservation_claim_id
               AND broker.capacity_claim_revision=1
               AND broker.capacity_claim_digest=NEW.reservation_claim_digest
               AND broker.job_id=grant_row.job_id
               AND broker.source_job_revision=NEW.source_job_revision
               AND broker.source_job_digest=NEW.source_job_digest
               AND broker.reserved_job_revision=NEW.reserved_job_revision
               AND broker.reserved_job_digest=NEW.reserved_job_digest
               AND broker.reservation_revision=NEW.reservation_revision
               AND broker.reservation_digest=NEW.reservation_digest
               AND budget.user_id=grant_row.consumer_account_id
               AND budget.reserved_fen=NEW.reserved_amount_fen
               AND budget.status='reserved'
               AND (SELECT COUNT(*) FROM compute_capacity_claim_lines line
                     WHERE line.claim_id=NEW.parent_claim_id)=
                   (SELECT COUNT(*) FROM compute_capacity_claim_lines line
                     WHERE line.claim_id=NEW.reservation_claim_id)
               AND NOT EXISTS (SELECT 1 FROM compute_capacity_claim_lines parent_line
                    WHERE parent_line.claim_id=NEW.parent_claim_id
                      AND NOT EXISTS (SELECT 1 FROM compute_capacity_claim_lines child_line
                       WHERE child_line.claim_id=NEW.reservation_claim_id
                         AND child_line.line_no=parent_line.line_no
                         AND child_line.bucket_id=parent_line.bucket_id
                         AND child_line.meter=parent_line.meter
                         AND child_line.quantity_units=parent_line.quantity_units))
               AND (SELECT COUNT(*) FROM compute_capacity_ledger_legs leg
                     WHERE leg.transaction_id=NEW.parent_release_transaction_id)=
                   2*(SELECT COUNT(*) FROM compute_capacity_claim_lines line
                       WHERE line.claim_id=NEW.parent_claim_id)
               AND NOT EXISTS (SELECT 1 FROM compute_capacity_claim_lines line
                    WHERE line.claim_id=NEW.parent_claim_id AND (
                      NOT EXISTS (SELECT 1 FROM compute_capacity_ledger_legs leg
                       WHERE leg.transaction_id=NEW.parent_release_transaction_id
                         AND leg.line_no=line.line_no AND leg.leg_role='from'
                         AND leg.bucket_id=line.bucket_id AND leg.meter=line.meter
                         AND leg.account='held' AND leg.delta_units=-line.quantity_units)
                      OR NOT EXISTS (SELECT 1 FROM compute_capacity_ledger_legs leg
                       WHERE leg.transaction_id=NEW.parent_release_transaction_id
                         AND leg.line_no=line.line_no AND leg.leg_role='to'
                         AND leg.bucket_id=line.bucket_id AND leg.meter=line.meter
                         AND leg.account='available' AND leg.delta_units=line.quantity_units)))
               AND (SELECT COUNT(*) FROM compute_capacity_ledger_legs leg
                     WHERE leg.transaction_id=NEW.reservation_hold_transaction_id)=
                   2*(SELECT COUNT(*) FROM compute_capacity_claim_lines line
                       WHERE line.claim_id=NEW.reservation_claim_id)
               AND NOT EXISTS (SELECT 1 FROM compute_capacity_claim_lines line
                    WHERE line.claim_id=NEW.reservation_claim_id AND (
                      NOT EXISTS (SELECT 1 FROM compute_capacity_ledger_legs leg
                       WHERE leg.transaction_id=NEW.reservation_hold_transaction_id
                         AND leg.line_no=line.line_no AND leg.leg_role='from'
                         AND leg.bucket_id=line.bucket_id AND leg.meter=line.meter
                         AND leg.account='available' AND leg.delta_units=-line.quantity_units)
                      OR NOT EXISTS (SELECT 1 FROM compute_capacity_ledger_legs leg
                       WHERE leg.transaction_id=NEW.reservation_hold_transaction_id
                         AND leg.line_no=line.line_no AND leg.leg_role='to'
                         AND leg.bucket_id=line.bucket_id AND leg.meter=line.meter
                         AND leg.account='held' AND leg.delta_units=line.quantity_units)))
        )
        BEGIN
            SELECT RAISE(ABORT, 'delivery allocation exercise lacks exact atomic source');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_capacity_commitment_terminal_allocation_guard
        BEFORE INSERT ON compute_capacity_commitment_terminal_receipts
        WHEN EXISTS (
            SELECT 1 FROM compute_delivery_allocation_grants grant_row
              LEFT JOIN compute_delivery_allocation_terminal_receipts terminal
                ON terminal.grant_id=grant_row.grant_id
             WHERE grant_row.commitment_id=NEW.commitment_id
               AND (terminal.grant_id IS NULL OR terminal.terminal_status='exercised'))
        BEGIN
            SELECT RAISE(ABORT, 'active or exercised allocation blocks commitment terminal');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_delivery_allocation_terminal_no_replace
        BEFORE INSERT ON compute_delivery_allocation_terminal_receipts
        WHEN EXISTS (SELECT 1 FROM compute_delivery_allocation_terminal_receipts existing
             WHERE existing.terminal_receipt_id=NEW.terminal_receipt_id
                OR existing.terminal_receipt_digest=NEW.terminal_receipt_digest
                OR existing.grant_id=NEW.grant_id OR existing.commitment_id=NEW.commitment_id
                OR existing.parent_release_transaction_id=NEW.parent_release_transaction_id
                OR existing.reservation_claim_id=NEW.reservation_claim_id
                OR existing.reservation_hold_transaction_id=NEW.reservation_hold_transaction_id
                OR existing.reservation_id=NEW.reservation_id
                OR existing.budget_reservation_id=NEW.budget_reservation_id
                OR (existing.idempotency_scope=NEW.idempotency_scope
                    AND existing.idempotency_key=NEW.idempotency_key))
        BEGIN
            SELECT RAISE(ABORT, 'delivery allocation terminal cannot replace history');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_delivery_allocation_terminal_no_update
        BEFORE UPDATE ON compute_delivery_allocation_terminal_receipts
        BEGIN
            SELECT RAISE(ABORT, 'delivery allocation terminal receipts are immutable');
        END;
        CREATE TRIGGER IF NOT EXISTS trg_delivery_allocation_terminal_no_delete
        BEFORE DELETE ON compute_delivery_allocation_terminal_receipts
        BEGIN
            SELECT RAISE(ABORT, 'delivery allocation terminal receipts are immutable');
        END;
        "#,
    )?;
    Ok(())
}
