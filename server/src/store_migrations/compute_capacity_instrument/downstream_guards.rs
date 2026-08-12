use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_snapshot_adoption
        BEFORE INSERT ON compute_price_snapshots
        WHEN NEW.pricing_mode='capacity_future' AND NOT EXISTS (
            SELECT 1
              FROM compute_capacity_instrument_offer_adoptions adoption
              JOIN compute_capacity_instrument_current instrument
                ON instrument.instrument_id=adoption.instrument_id
               AND instrument.instrument_revision=adoption.instrument_revision
               AND instrument.instrument_digest=adoption.instrument_digest
              JOIN compute_offers current_offer
                ON current_offer.offer_id=adoption.offer_id
               AND current_offer.current_offer_version=adoption.offer_version
               AND current_offer.current_offer_digest=adoption.offer_digest
             WHERE adoption.offer_id=NEW.offer_id
               AND adoption.offer_version=NEW.offer_version
               AND adoption.offer_digest=NEW.offer_digest
               AND adoption.instrument_id=NEW.instrument_id
               AND current_offer.status='active'
               AND instrument.current_status='active'
               AND instrument.sku_id=NEW.sku_id AND instrument.sku_digest=NEW.sku_digest
               AND instrument.delivery_window_id=NEW.delivery_window_id
               AND instrument.delivery_window_digest=NEW.delivery_window_digest
               AND json_extract(NEW.snapshot_json,'$.delivery_window.starts_at_utc')=
                    instrument.delivery_window_starts_at
               AND json_extract(NEW.snapshot_json,'$.delivery_window.ends_at_utc')=
                    instrument.delivery_window_ends_at
               AND instrument.settlement_currency=NEW.currency)
        BEGIN
            SELECT RAISE(ABORT, 'capacity future Snapshot lacks exact instrument adoption');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_quoted_job_adoption
        BEFORE INSERT ON compute_job_versions
        WHEN NEW.status='quoted' AND EXISTS (
            SELECT 1 FROM compute_price_snapshots snapshot
             WHERE snapshot.snapshot_id=NEW.price_snapshot_id
               AND snapshot.pricing_mode='capacity_future')
         AND NOT EXISTS (
            SELECT 1 FROM compute_price_snapshots snapshot
              JOIN compute_capacity_instrument_offer_adoptions adoption
                ON adoption.offer_id=snapshot.offer_id
               AND adoption.offer_version=snapshot.offer_version
               AND adoption.offer_digest=snapshot.offer_digest
               AND adoption.instrument_id=snapshot.instrument_id
              JOIN compute_capacity_instrument_current instrument
                ON instrument.instrument_id=adoption.instrument_id
               AND instrument.instrument_revision=adoption.instrument_revision
               AND instrument.instrument_digest=adoption.instrument_digest
              JOIN compute_offers current_offer
                ON current_offer.offer_id=adoption.offer_id
               AND current_offer.current_offer_version=adoption.offer_version
               AND current_offer.current_offer_digest=adoption.offer_digest
             WHERE snapshot.snapshot_id=NEW.price_snapshot_id
               AND snapshot.offer_id=NEW.selected_offer_id
               AND snapshot.offer_version=NEW.selected_offer_version
               AND snapshot.offer_digest=NEW.selected_offer_digest
               AND current_offer.status='active'
               AND instrument.current_status='active')
        BEGIN
            SELECT RAISE(ABORT, 'capacity future quoted Job lacks exact instrument adoption');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_reservation_adoption
        BEFORE INSERT ON compute_reservations
        WHEN NEW.status='pending' AND EXISTS (
            SELECT 1 FROM compute_price_snapshots snapshot
             WHERE snapshot.snapshot_id=NEW.price_snapshot_id
               AND snapshot.pricing_mode='capacity_future')
         AND NOT EXISTS (
            SELECT 1 FROM compute_price_snapshots snapshot
              JOIN compute_capacity_instrument_offer_adoptions adoption
                ON adoption.offer_id=NEW.offer_id
               AND adoption.offer_version=NEW.offer_version
               AND adoption.offer_digest=NEW.offer_digest
               AND adoption.instrument_id=snapshot.instrument_id
              JOIN compute_capacity_instrument_current instrument
                ON instrument.instrument_id=adoption.instrument_id
               AND instrument.instrument_revision=adoption.instrument_revision
               AND instrument.instrument_digest=adoption.instrument_digest
              JOIN compute_offers current_offer
                ON current_offer.offer_id=adoption.offer_id
             WHERE snapshot.snapshot_id=NEW.price_snapshot_id
               AND snapshot.offer_id=NEW.offer_id
               AND snapshot.offer_version=NEW.offer_version
               AND snapshot.offer_digest=NEW.offer_digest
               AND ((current_offer.current_offer_version=adoption.offer_version
                     AND current_offer.current_offer_digest=adoption.offer_digest
                     AND current_offer.status='active')
                    OR EXISTS (
                       SELECT 1
                         FROM compute_capacity_instrument_historical_exercise_authority authority
                        WHERE authority.reservation_id=NEW.reservation_id
                          AND authority.capacity_claim_id=NEW.capacity_claim_id
                          AND authority.capacity_claim_revision=NEW.capacity_claim_revision
                          AND authority.capacity_claim_digest=NEW.capacity_claim_digest
                          AND authority.consumer_account_id=NEW.consumer_account_id
                          AND authority.job_id=NEW.job_id
                          AND authority.source_job_revision=NEW.job_revision
                          AND authority.source_job_digest=NEW.job_digest
                          AND authority.provider_id=NEW.provider_id
                          AND authority.offer_id=NEW.offer_id
                          AND authority.offer_version=NEW.offer_version
                          AND authority.offer_digest=NEW.offer_digest
                          AND authority.price_snapshot_id=NEW.price_snapshot_id
                          AND NEW.created_at=authority.claim_created_at
                          AND NEW.updated_at=authority.claim_created_at
                          AND NEW.expires_at=authority.claim_expires_at))
               AND instrument.current_status='active')
        BEGIN
            SELECT RAISE(ABORT, 'capacity future Reservation lacks exact instrument adoption');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_broker_receipt_adoption
        BEFORE INSERT ON compute_broker_reserve_receipts
        WHEN EXISTS (
            SELECT 1 FROM compute_reservation_versions reservation
              JOIN compute_price_snapshots snapshot
                ON snapshot.snapshot_id=reservation.price_snapshot_id
             WHERE reservation.reservation_id=NEW.reservation_id
               AND reservation.revision=NEW.reservation_revision
               AND snapshot.pricing_mode='capacity_future')
         AND NOT EXISTS (
            SELECT 1 FROM compute_reservation_versions reservation
              JOIN compute_price_snapshots snapshot
                ON snapshot.snapshot_id=reservation.price_snapshot_id
              JOIN compute_capacity_instrument_offer_adoptions adoption
                ON adoption.offer_id=reservation.offer_id
               AND adoption.offer_version=reservation.offer_version
               AND adoption.offer_digest=reservation.offer_digest
               AND adoption.instrument_id=snapshot.instrument_id
              JOIN compute_capacity_instrument_current instrument
                ON instrument.instrument_id=adoption.instrument_id
               AND instrument.instrument_revision=adoption.instrument_revision
               AND instrument.instrument_digest=adoption.instrument_digest
              JOIN compute_offers current_offer
                ON current_offer.offer_id=adoption.offer_id
             WHERE reservation.reservation_id=NEW.reservation_id
               AND reservation.revision=NEW.reservation_revision
               AND reservation.reservation_digest=NEW.reservation_digest
               AND ((current_offer.current_offer_version=adoption.offer_version
                     AND current_offer.current_offer_digest=adoption.offer_digest
                     AND current_offer.status='active')
                    OR EXISTS (
                       SELECT 1
                         FROM compute_capacity_instrument_historical_exercise_authority authority
                         JOIN compute_reservations root
                           ON root.reservation_id=authority.reservation_id
                        WHERE authority.reservation_id=NEW.reservation_id
                          AND authority.capacity_claim_id=NEW.capacity_claim_id
                          AND authority.capacity_claim_revision=NEW.capacity_claim_revision
                          AND authority.capacity_claim_digest=NEW.capacity_claim_digest
                          AND authority.consumer_account_id=NEW.consumer_account_id
                          AND authority.job_id=NEW.job_id
                          AND authority.source_job_revision=NEW.source_job_revision
                          AND authority.source_job_digest=NEW.source_job_digest
                          AND authority.offer_id=reservation.offer_id
                          AND authority.offer_version=reservation.offer_version
                          AND authority.offer_digest=reservation.offer_digest
                          AND authority.price_snapshot_id=reservation.price_snapshot_id
                          AND root.consumer_account_id=authority.consumer_account_id
                          AND root.current_revision=reservation.revision
                          AND root.current_reservation_digest=reservation.reservation_digest
                          AND root.status='active'
                          AND root.job_id=NEW.job_id
                          AND root.job_revision=NEW.reserved_job_revision
                          AND root.job_digest=NEW.reserved_job_digest
                          AND root.capacity_claim_id=authority.capacity_claim_id
                          AND root.capacity_claim_revision=authority.capacity_claim_revision
                          AND root.capacity_claim_digest=authority.capacity_claim_digest))
               AND instrument.current_status='active')
        BEGIN
            SELECT RAISE(ABORT, 'capacity future Broker receipt lacks exact instrument adoption');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_active_reservation_adoption
        BEFORE INSERT ON compute_reservation_versions
        WHEN NEW.status='active' AND EXISTS (
            SELECT 1 FROM compute_price_snapshots snapshot
             WHERE snapshot.snapshot_id=NEW.price_snapshot_id
               AND snapshot.pricing_mode='capacity_future')
         AND NOT EXISTS (
            SELECT 1 FROM compute_price_snapshots snapshot
              JOIN compute_capacity_instrument_offer_adoptions adoption
                ON adoption.offer_id=NEW.offer_id
               AND adoption.offer_version=NEW.offer_version
               AND adoption.offer_digest=NEW.offer_digest
               AND adoption.instrument_id=snapshot.instrument_id
              JOIN compute_capacity_instrument_current instrument
                ON instrument.instrument_id=adoption.instrument_id
               AND instrument.instrument_revision=adoption.instrument_revision
               AND instrument.instrument_digest=adoption.instrument_digest
              JOIN compute_offers current_offer
                ON current_offer.offer_id=adoption.offer_id
             WHERE snapshot.snapshot_id=NEW.price_snapshot_id
               AND snapshot.offer_id=NEW.offer_id
               AND snapshot.offer_version=NEW.offer_version
               AND snapshot.offer_digest=NEW.offer_digest
               AND ((current_offer.current_offer_version=adoption.offer_version
                     AND current_offer.current_offer_digest=adoption.offer_digest
                     AND current_offer.status='active')
                    OR EXISTS (
                       SELECT 1
                         FROM compute_capacity_instrument_historical_exercise_authority authority
                         JOIN compute_reservations root
                           ON root.reservation_id=authority.reservation_id
                        WHERE authority.reservation_id=NEW.reservation_id
                          AND authority.capacity_claim_id=NEW.capacity_claim_id
                          AND authority.capacity_claim_revision=NEW.capacity_claim_revision
                          AND authority.capacity_claim_digest=NEW.capacity_claim_digest
                          AND authority.job_id=NEW.job_id
                          AND authority.provider_id=NEW.provider_id
                          AND authority.offer_id=NEW.offer_id
                          AND authority.offer_version=NEW.offer_version
                          AND authority.offer_digest=NEW.offer_digest
                          AND authority.price_snapshot_id=NEW.price_snapshot_id
                          AND root.consumer_account_id=authority.consumer_account_id
                          AND root.current_revision=1 AND root.status='pending'
                          AND root.job_id=authority.job_id
                          AND root.job_revision=authority.source_job_revision
                          AND root.job_digest=authority.source_job_digest
                          AND root.capacity_claim_id=authority.capacity_claim_id
                          AND root.capacity_claim_revision=authority.capacity_claim_revision
                          AND root.capacity_claim_digest=authority.capacity_claim_digest
                          AND NEW.revision=root.current_revision+1
                          AND NEW.job_revision=root.job_revision+1
                          AND EXISTS (
                               SELECT 1 FROM compute_job_versions reserved_job
                                WHERE reserved_job.job_id=NEW.job_id
                                  AND reserved_job.revision=NEW.job_revision
                                  AND reserved_job.job_digest=NEW.job_digest
                                  AND reserved_job.status='reserved')))
               AND instrument.current_status='active')
        BEGIN
            SELECT RAISE(ABORT, 'capacity future active Reservation lacks exact instrument adoption');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_commitment_adoption
        BEFORE INSERT ON compute_capacity_commitments
        WHEN NOT EXISTS (
            SELECT 1 FROM compute_price_snapshots snapshot
              JOIN compute_capacity_instrument_offer_adoptions adoption
                ON adoption.offer_id=NEW.offer_id
               AND adoption.offer_version=NEW.offer_version
               AND adoption.offer_digest=NEW.offer_digest
               AND adoption.instrument_id=NEW.instrument_id
              JOIN compute_capacity_instrument_current instrument
                ON instrument.instrument_id=adoption.instrument_id
               AND instrument.instrument_revision=adoption.instrument_revision
               AND instrument.instrument_digest=adoption.instrument_digest
             WHERE snapshot.snapshot_id=NEW.price_snapshot_id
               AND snapshot.snapshot_digest=NEW.price_snapshot_digest
               AND snapshot.pricing_mode='capacity_future'
               AND snapshot.offer_id=NEW.offer_id
               AND snapshot.offer_version=NEW.offer_version
               AND snapshot.offer_digest=NEW.offer_digest
               AND snapshot.instrument_id=NEW.instrument_id
               AND instrument.current_status='active'
               AND instrument.delivery_window_id=NEW.delivery_window_id
               AND instrument.delivery_window_digest=NEW.delivery_window_digest
               AND instrument.delivery_window_starts_at=NEW.delivery_window_starts_at
               AND instrument.delivery_window_ends_at=NEW.delivery_window_ends_at
               AND (SELECT COUNT(*)
                      FROM compute_capacity_claim_lines line
                     WHERE line.claim_id=NEW.claim_id)=
                   json_array_length(instrument.contract_units_json)
               AND NOT EXISTS (
                    SELECT 1
                      FROM json_each(instrument.contract_units_json) unit
                     WHERE NOT EXISTS (
                        SELECT 1
                          FROM compute_capacity_claim_lines line
                         WHERE line.claim_id=NEW.claim_id
                           AND line.meter=json_extract(unit.value,'$.meter')
                           AND line.quantity_units%
                                json_extract(unit.value,'$.quantity_units')=0))
               AND (SELECT MIN(
                        line.quantity_units/
                        json_extract(unit.value,'$.quantity_units'))
                      FROM compute_capacity_claim_lines line
                      JOIN json_each(instrument.contract_units_json) unit
                        ON json_extract(unit.value,'$.meter')=line.meter
                     WHERE line.claim_id=NEW.claim_id)=
                   (SELECT MAX(
                        line.quantity_units/
                        json_extract(unit.value,'$.quantity_units'))
                      FROM compute_capacity_claim_lines line
                      JOIN json_each(instrument.contract_units_json) unit
                        ON json_extract(unit.value,'$.meter')=line.meter
                     WHERE line.claim_id=NEW.claim_id))
        BEGIN
            SELECT RAISE(ABORT, 'CapacityCommitment lacks exact instrument adoption');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_grant_adoption
        BEFORE INSERT ON compute_delivery_allocation_grants
        WHEN NOT EXISTS (
            SELECT 1 FROM compute_capacity_commitments commitment
              JOIN compute_capacity_instrument_offer_adoptions adoption
                ON adoption.offer_id=commitment.offer_id
               AND adoption.offer_version=commitment.offer_version
               AND adoption.offer_digest=commitment.offer_digest
               AND adoption.instrument_id=commitment.instrument_id
              JOIN compute_capacity_instrument_current instrument
                ON instrument.instrument_id=adoption.instrument_id
               AND instrument.instrument_revision=adoption.instrument_revision
               AND instrument.instrument_digest=adoption.instrument_digest
              JOIN compute_offers current_offer
                ON current_offer.offer_id=adoption.offer_id
               AND current_offer.current_offer_version=adoption.offer_version
               AND current_offer.current_offer_digest=adoption.offer_digest
             WHERE commitment.commitment_id=NEW.commitment_id
               AND commitment.commitment_revision=NEW.commitment_revision
               AND commitment.commitment_digest=NEW.commitment_digest
               AND current_offer.status='active'
               AND instrument.current_status='active')
        BEGIN
            SELECT RAISE(ABORT, 'DeliveryAllocation Grant lacks exact instrument adoption');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_exercise_adoption
        BEFORE INSERT ON compute_delivery_allocation_terminal_receipts
        WHEN NEW.terminal_status='exercised' AND NOT EXISTS (
            SELECT 1 FROM compute_capacity_commitments commitment
              JOIN compute_capacity_instrument_offer_adoptions adoption
                ON adoption.offer_id=commitment.offer_id
               AND adoption.offer_version=commitment.offer_version
               AND adoption.offer_digest=commitment.offer_digest
               AND adoption.instrument_id=commitment.instrument_id
              JOIN compute_capacity_instrument_current instrument
                ON instrument.instrument_id=adoption.instrument_id
               AND instrument.instrument_revision=adoption.instrument_revision
               AND instrument.instrument_digest=adoption.instrument_digest
             WHERE commitment.commitment_id=NEW.commitment_id
               AND commitment.commitment_revision=NEW.commitment_revision
               AND commitment.commitment_digest=NEW.commitment_digest
               AND instrument.current_status='active')
        BEGIN
            SELECT RAISE(ABORT, 'DeliveryAllocation exercise lacks exact instrument adoption');
        END;
        "#,
    )?;
    Ok(())
}
