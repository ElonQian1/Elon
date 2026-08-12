use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE VIEW IF NOT EXISTS compute_capacity_instrument_historical_exercise_authority AS
        SELECT child.subject_id AS reservation_id,
               child.claim_id AS capacity_claim_id,
               child.revision AS capacity_claim_revision,
               child.claim_digest AS capacity_claim_digest,
               child.created_at AS claim_created_at,
               child.expires_at AS claim_expires_at,
               grant_row.consumer_account_id,
               grant_row.job_id,
               grant_row.job_revision AS source_job_revision,
               grant_row.job_digest AS source_job_digest,
               commitment.provider_id,
               commitment.offer_id,
               commitment.offer_version,
               commitment.offer_digest,
               commitment.price_snapshot_id,
               commitment.instrument_id
          FROM compute_capacity_claims child
          JOIN compute_capacity_claim_versions child_version
            ON child_version.claim_id=child.claim_id
           AND child_version.revision=child.revision
           AND child_version.claim_digest=child.claim_digest
          JOIN compute_capacity_ledger_transactions child_hold
            ON child_hold.claim_id=child.claim_id
           AND child_hold.event_kind='reservation_held'
           AND child_hold.claim_effect='held'
          JOIN compute_capacity_claims parent
            ON parent.claim_id=child.parent_claim_id
          JOIN compute_capacity_claim_versions parent_prior
            ON parent_prior.claim_id=parent.claim_id
           AND parent_prior.revision=1
          JOIN compute_capacity_claim_versions parent_result
            ON parent_result.claim_id=parent.claim_id
           AND parent_result.revision=parent.revision
           AND parent_result.claim_digest=parent.claim_digest
          JOIN compute_capacity_ledger_transactions parent_release
            ON parent_release.transaction_id=child_hold.causal_transaction_id
          JOIN compute_capacity_commitments commitment
            ON commitment.claim_id=parent.claim_id
           AND commitment.claim_revision=parent_prior.revision
           AND commitment.claim_digest=parent_prior.claim_digest
          JOIN compute_delivery_allocation_grants grant_row
            ON grant_row.commitment_id=commitment.commitment_id
           AND grant_row.commitment_revision=commitment.commitment_revision
           AND grant_row.commitment_digest=commitment.commitment_digest
          JOIN compute_job_versions source_job
            ON source_job.job_id=grant_row.job_id
           AND source_job.revision=grant_row.job_revision
           AND source_job.job_digest=grant_row.job_digest
          JOIN compute_price_snapshots snapshot
            ON snapshot.snapshot_id=commitment.price_snapshot_id
           AND snapshot.snapshot_digest=commitment.price_snapshot_digest
           AND snapshot.offer_id=commitment.offer_id
           AND snapshot.offer_version=commitment.offer_version
           AND snapshot.offer_digest=commitment.offer_digest
           AND snapshot.instrument_id=commitment.instrument_id
          JOIN compute_capacity_instrument_offer_adoptions adoption
            ON adoption.offer_id=commitment.offer_id
           AND adoption.offer_version=commitment.offer_version
           AND adoption.offer_digest=commitment.offer_digest
           AND adoption.instrument_id=commitment.instrument_id
          JOIN compute_capacity_instrument_current instrument
            ON instrument.instrument_id=adoption.instrument_id
           AND instrument.instrument_revision=adoption.instrument_revision
           AND instrument.instrument_digest=adoption.instrument_digest
          JOIN compute_offer_versions historical_offer
            ON historical_offer.offer_id=adoption.offer_id
           AND historical_offer.offer_version=adoption.offer_version
           AND historical_offer.offer_digest=adoption.offer_digest
           AND historical_offer.status='active'
          JOIN compute_offers current_offer
            ON current_offer.offer_id=adoption.offer_id
          JOIN compute_offer_versions current_version
            ON current_version.offer_id=current_offer.offer_id
           AND current_version.offer_version=current_offer.current_offer_version
           AND current_version.offer_digest=current_offer.current_offer_digest
           AND current_version.provider_id=current_offer.provider_id
           AND current_version.provider_policy_revision=
               current_offer.current_provider_policy_revision
           AND current_version.provider_digest=current_offer.current_provider_digest
           AND current_version.sku_id=current_offer.sku_id
           AND current_version.sku_digest=current_offer.sku_digest
           AND current_version.capacity_pool_id=current_offer.capacity_pool_id
           AND current_version.status=current_offer.status
           AND current_version.valid_from=current_offer.valid_from
           AND current_version.valid_until=current_offer.valid_until
           AND current_version.created_at=current_offer.current_version_created_at
          LEFT JOIN compute_offer_lifecycle_events drain
            ON drain.offer_id=adoption.offer_id
           AND drain.provider_id=commitment.provider_id
           AND drain.pool_id=commitment.pool_id
           AND drain.previous_status='active'
           AND drain.target_status='draining'
          LEFT JOIN compute_offer_versions drain_source
            ON drain_source.offer_id=drain.offer_id
           AND drain_source.offer_version=drain.previous_offer_version
           AND drain_source.offer_digest=drain.previous_offer_digest
           AND drain_source.provider_id=commitment.provider_id
           AND drain_source.capacity_pool_id=commitment.pool_id
           AND drain_source.sku_id=historical_offer.sku_id
           AND drain_source.sku_digest=historical_offer.sku_digest
           AND drain_source.status='active'
          LEFT JOIN compute_offer_versions drain_target
            ON drain_target.offer_id=drain.offer_id
           AND drain_target.offer_version=drain.target_offer_version
           AND drain_target.offer_digest=drain.target_offer_digest
           AND drain_target.provider_id=commitment.provider_id
           AND drain_target.capacity_pool_id=commitment.pool_id
           AND drain_target.sku_id=historical_offer.sku_id
           AND drain_target.sku_digest=historical_offer.sku_digest
           AND drain_target.status='draining'
         WHERE child.claim_kind='reservation'
           AND child.subject_kind='compute_reservation'
           AND child.status='held' AND child.revision=1 AND child.terminal_at IS NULL
           AND child.expires_at IS NOT NULL
           AND child_version.status='held'
           AND child.pool_id=commitment.pool_id
           AND child.capacity_epoch=commitment.capacity_epoch
           AND child.delivery_window_id=commitment.delivery_window_id
           AND child_hold.pool_id=commitment.pool_id
           AND child_hold.capacity_epoch=commitment.capacity_epoch
           AND child_hold.delivery_window_id=commitment.delivery_window_id
           AND child_hold.offer_id=commitment.offer_id
           AND child_hold.offer_version=commitment.offer_version
           AND child_hold.offer_digest=commitment.offer_digest
           AND child_hold.claim_effect_key=child.idempotency_key
           AND child_hold.job_id=grant_row.job_id
           AND child_hold.reservation_id=child.subject_id
           AND child_hold.subject_kind='compute_reservation'
           AND child_hold.subject_id=child.subject_id
           AND parent.claim_kind='capacity_commitment'
           AND parent.subject_kind='compute_capacity_commitment'
           AND parent.subject_id=commitment.commitment_id
           AND parent.parent_claim_id IS NULL
           AND parent.status='released' AND parent.revision=2
           AND parent.pool_id=commitment.pool_id
           AND parent.capacity_epoch=commitment.capacity_epoch
           AND parent.delivery_window_id=commitment.delivery_window_id
           AND parent_prior.status='held'
           AND parent_result.status='released'
           AND parent_release.claim_id=parent.claim_id
           AND parent_release.event_kind='reservation_released'
           AND parent_release.claim_effect='released'
           AND parent_release.pool_id=commitment.pool_id
           AND parent_release.capacity_epoch=commitment.capacity_epoch
           AND parent_release.delivery_window_id=commitment.delivery_window_id
           AND parent_release.offer_id=commitment.offer_id
           AND parent_release.offer_version=commitment.offer_version
           AND parent_release.offer_digest=commitment.offer_digest
           AND parent_release.job_id IS NULL AND parent_release.reservation_id IS NULL
           AND parent_release.subject_kind='compute_capacity_commitment'
           AND parent_release.subject_id=commitment.commitment_id
           AND parent_release.causal_transaction_id=commitment.hold_transaction_id
           AND parent_release.occurred_at=child_hold.occurred_at
           AND parent_release.recorded_at<=child_hold.recorded_at
           AND source_job.status='quoted'
           AND commitment.commitment_status='committed'
           AND grant_row.grant_status='granted'
           AND julianday(child.created_at)<julianday(grant_row.exercise_expires_at)
           AND NOT EXISTS (
                SELECT 1 FROM compute_delivery_allocation_terminal_receipts terminal
                 WHERE terminal.grant_id=grant_row.grant_id)
           AND NOT EXISTS (
                SELECT 1 FROM compute_capacity_commitment_terminal_receipts terminal
                 WHERE terminal.commitment_id=commitment.commitment_id)
           AND snapshot.pricing_mode='capacity_future'
           AND instrument.current_status='active'
           AND historical_offer.provider_id=commitment.provider_id
           AND historical_offer.capacity_pool_id=commitment.pool_id
           AND current_offer.provider_id=commitment.provider_id
           AND current_offer.capacity_pool_id=commitment.pool_id
           AND current_version.sku_id=historical_offer.sku_id
           AND current_version.sku_digest=historical_offer.sku_digest
           AND (
                (current_offer.status='active'
                 AND current_offer.current_offer_version>adoption.offer_version)
                OR
                (current_offer.status='draining'
                 AND drain.event_id IS NOT NULL
                 AND drain_source.offer_id IS NOT NULL
                 AND drain_target.offer_id IS NOT NULL
                 AND drain.previous_offer_version>=adoption.offer_version
                 AND current_offer.current_offer_version>=drain.target_offer_version
                 AND julianday(adoption.adopted_at)<=julianday(drain.changed_at)))
           AND (SELECT COUNT(*) FROM compute_capacity_claim_lines line
                 WHERE line.claim_id=parent.claim_id)=
               (SELECT COUNT(*) FROM compute_capacity_claim_lines line
                 WHERE line.claim_id=child.claim_id)
           AND NOT EXISTS (
                SELECT 1 FROM compute_capacity_claim_lines parent_line
                 WHERE parent_line.claim_id=parent.claim_id
                   AND NOT EXISTS (
                        SELECT 1 FROM compute_capacity_claim_lines child_line
                         WHERE child_line.claim_id=child.claim_id
                           AND child_line.line_no=parent_line.line_no
                           AND child_line.bucket_id=parent_line.bucket_id
                           AND child_line.meter=parent_line.meter
                           AND child_line.quantity_units=parent_line.quantity_units))
           AND (SELECT COUNT(*) FROM compute_capacity_ledger_legs leg
                 WHERE leg.transaction_id=parent_release.transaction_id)=
               2*(SELECT COUNT(*) FROM compute_capacity_claim_lines line
                   WHERE line.claim_id=parent.claim_id)
           AND NOT EXISTS (
                SELECT 1 FROM compute_capacity_claim_lines line
                 WHERE line.claim_id=parent.claim_id AND (
                    NOT EXISTS (SELECT 1 FROM compute_capacity_ledger_legs leg
                         WHERE leg.transaction_id=parent_release.transaction_id
                           AND leg.line_no=line.line_no AND leg.leg_role='from'
                           AND leg.bucket_id=line.bucket_id AND leg.meter=line.meter
                           AND leg.account='held' AND leg.delta_units=-line.quantity_units)
                    OR NOT EXISTS (SELECT 1 FROM compute_capacity_ledger_legs leg
                         WHERE leg.transaction_id=parent_release.transaction_id
                           AND leg.line_no=line.line_no AND leg.leg_role='to'
                           AND leg.bucket_id=line.bucket_id AND leg.meter=line.meter
                           AND leg.account='available' AND leg.delta_units=line.quantity_units)))
           AND (SELECT COUNT(*) FROM compute_capacity_ledger_legs leg
                 WHERE leg.transaction_id=child_hold.transaction_id)=
               2*(SELECT COUNT(*) FROM compute_capacity_claim_lines line
                   WHERE line.claim_id=child.claim_id)
           AND NOT EXISTS (
                SELECT 1 FROM compute_capacity_claim_lines line
                 WHERE line.claim_id=child.claim_id AND (
                    NOT EXISTS (SELECT 1 FROM compute_capacity_ledger_legs leg
                         WHERE leg.transaction_id=child_hold.transaction_id
                           AND leg.line_no=line.line_no AND leg.leg_role='from'
                           AND leg.bucket_id=line.bucket_id AND leg.meter=line.meter
                           AND leg.account='available' AND leg.delta_units=-line.quantity_units)
                    OR NOT EXISTS (SELECT 1 FROM compute_capacity_ledger_legs leg
                         WHERE leg.transaction_id=child_hold.transaction_id
                           AND leg.line_no=line.line_no AND leg.leg_role='to'
                           AND leg.bucket_id=line.bucket_id AND leg.meter=line.meter
                           AND leg.account='held' AND leg.delta_units=line.quantity_units)));
        "#,
    )?;
    Ok(())
}
