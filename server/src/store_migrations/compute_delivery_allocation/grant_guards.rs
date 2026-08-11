use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_delivery_allocation_grant_projection
        BEFORE INSERT ON compute_delivery_allocation_grants
        WHEN json_extract(NEW.grant_json,'$.schema') IS NOT NEW.grant_schema
          OR json_extract(NEW.grant_json,'$.grant_id') IS NOT NEW.grant_id
          OR json_extract(NEW.grant_json,'$.grant_revision') IS NOT NEW.grant_revision
          OR json_extract(NEW.grant_json,'$.grant_digest') IS NOT NEW.grant_digest
          OR json_extract(NEW.grant_json,'$.grant_status') IS NOT NEW.grant_status
          OR json_extract(NEW.grant_json,'$.commitment.commitment_id')
                IS NOT NEW.commitment_id
          OR json_extract(NEW.grant_json,'$.commitment.commitment_revision')
                IS NOT NEW.commitment_revision
          OR json_extract(NEW.grant_json,'$.commitment.commitment_digest')
                IS NOT NEW.commitment_digest
          OR json_extract(NEW.grant_json,'$.provider_owner_account_id')
                IS NOT NEW.provider_owner_account_id
          OR json_extract(NEW.grant_json,'$.consumer_account_id')
                IS NOT NEW.consumer_account_id
          OR (NEW.project_id IS NULL
                AND json_type(NEW.grant_json,'$.project_id') IS NOT 'null')
          OR (NEW.project_id IS NOT NULL
                AND json_extract(NEW.grant_json,'$.project_id') IS NOT NEW.project_id)
          OR json_extract(NEW.grant_json,'$.job.job_id') IS NOT NEW.job_id
          OR json_extract(NEW.grant_json,'$.job.job_revision') IS NOT NEW.job_revision
          OR json_extract(NEW.grant_json,'$.job.job_digest') IS NOT NEW.job_digest
          OR json_extract(NEW.grant_json,'$.exercise_expires_at')
                IS NOT NEW.exercise_expires_at
          OR json_extract(NEW.grant_json,'$.idempotency_scope')
                IS NOT NEW.idempotency_scope
          OR json_extract(NEW.grant_json,'$.idempotency_key') IS NOT NEW.idempotency_key
          OR json_extract(NEW.grant_json,'$.request_digest') IS NOT NEW.request_digest
          OR json_extract(NEW.grant_json,'$.created_at') IS NOT NEW.created_at
          OR (SELECT COUNT(*) FROM json_each(NEW.grant_json))<>15
          OR (SELECT COUNT(DISTINCT key) FROM json_each(NEW.grant_json))<>15
          OR EXISTS (SELECT 1 FROM json_each(NEW.grant_json) WHERE key NOT IN (
                'schema','grant_id','grant_revision','grant_digest','grant_status',
                'commitment','provider_owner_account_id','consumer_account_id','project_id',
                'job','exercise_expires_at','idempotency_scope','idempotency_key',
                'request_digest','created_at'))
          OR json_type(NEW.grant_json,'$.commitment') IS NOT 'object'
          OR (SELECT COUNT(*) FROM json_each(NEW.grant_json,'$.commitment'))<>3
          OR EXISTS (SELECT 1 FROM json_each(NEW.grant_json,'$.commitment')
                WHERE key NOT IN ('commitment_id','commitment_revision','commitment_digest'))
          OR json_type(NEW.grant_json,'$.job') IS NOT 'object'
          OR (SELECT COUNT(*) FROM json_each(NEW.grant_json,'$.job'))<>3
          OR EXISTS (SELECT 1 FROM json_each(NEW.grant_json,'$.job')
                WHERE key NOT IN ('job_id','job_revision','job_digest'))
        BEGIN
            SELECT RAISE(ABORT, 'delivery allocation Grant JSON projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_delivery_allocation_grant_exact_source
        BEFORE INSERT ON compute_delivery_allocation_grants
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_capacity_commitments commitment
              JOIN compute_capacity_claims parent_claim
                ON parent_claim.claim_id=commitment.claim_id
              JOIN compute_capacity_claim_versions parent_version
                ON parent_version.claim_id=parent_claim.claim_id
               AND parent_version.revision=parent_claim.revision
              JOIN compute_providers provider ON provider.provider_id=commitment.provider_id
              JOIN compute_offers offer ON offer.offer_id=commitment.offer_id
              JOIN compute_capacity_pools pool ON pool.pool_id=commitment.pool_id
              JOIN compute_price_snapshots snapshot
                ON snapshot.snapshot_id=commitment.price_snapshot_id
              JOIN compute_jobs job ON job.job_id=NEW.job_id
              JOIN compute_job_versions job_version
                ON job_version.job_id=job.job_id
               AND job_version.revision=job.current_revision
             WHERE commitment.commitment_id=NEW.commitment_id
               AND commitment.commitment_revision=NEW.commitment_revision
               AND commitment.commitment_digest=NEW.commitment_digest
               AND commitment.commitment_status='committed'
               AND commitment.owner_account_id=NEW.provider_owner_account_id
               AND commitment.delivery_window_starts_at=NEW.exercise_expires_at
               AND julianday(commitment.created_at)<=julianday(NEW.created_at)
               AND NOT EXISTS (SELECT 1
                    FROM compute_capacity_commitment_terminal_receipts terminal
                   WHERE terminal.commitment_id=NEW.commitment_id)
               AND parent_claim.claim_digest=commitment.claim_digest
               AND parent_claim.revision=1 AND parent_claim.status='held'
               AND parent_claim.claim_kind='capacity_commitment'
               AND parent_claim.subject_kind='compute_capacity_commitment'
               AND parent_claim.subject_id=commitment.commitment_id
               AND parent_claim.parent_claim_id IS NULL AND parent_claim.terminal_at IS NULL
               AND parent_claim.pool_id=commitment.pool_id
               AND parent_claim.capacity_epoch=commitment.capacity_epoch
               AND parent_claim.delivery_window_id=commitment.delivery_window_id
               AND parent_version.claim_digest=commitment.claim_digest
               AND parent_version.status='held'
               AND provider.owner_account_id=NEW.provider_owner_account_id
               AND provider.status='active'
               AND offer.provider_id=commitment.provider_id
               AND offer.capacity_pool_id=commitment.pool_id AND offer.status='active'
               AND julianday(offer.valid_from)<=julianday(NEW.created_at)
               AND julianday(NEW.created_at)<julianday(offer.valid_until)
               AND pool.provider_id=commitment.provider_id AND pool.status='active'
               AND pool.current_capacity_epoch=commitment.capacity_epoch
               AND snapshot.snapshot_digest=commitment.price_snapshot_digest
               AND snapshot.provider_id=commitment.provider_id
               AND snapshot.offer_id=commitment.offer_id
               AND snapshot.offer_version=commitment.offer_version
               AND snapshot.offer_digest=commitment.offer_digest
               AND snapshot.delivery_window_id=commitment.delivery_window_id
               AND snapshot.delivery_window_digest=commitment.delivery_window_digest
               AND julianday(snapshot.quoted_at)<=julianday(NEW.created_at)
               AND julianday(NEW.created_at)<julianday(snapshot.expires_at)
               AND job.consumer_account_id=NEW.consumer_account_id
               AND job.project_id IS NEW.project_id
               AND job.current_revision=NEW.job_revision
               AND job.current_job_digest=NEW.job_digest AND job.status='quoted'
               AND job.selected_provider_id=commitment.provider_id
               AND job.selected_offer_id=commitment.offer_id
               AND job.selected_offer_version=commitment.offer_version
               AND job.selected_offer_digest=commitment.offer_digest
               AND job.price_snapshot_id=commitment.price_snapshot_id
               AND job.currency='CNY'
               AND job.max_consumer_charge_micros>=snapshot.consumer_max_amount_micros
               AND job_version.job_digest=NEW.job_digest AND job_version.status='quoted'
               AND job_version.selected_provider_id=commitment.provider_id
               AND job_version.selected_offer_id=commitment.offer_id
               AND job_version.selected_offer_version=commitment.offer_version
               AND job_version.selected_offer_digest=commitment.offer_digest
               AND job_version.price_snapshot_id=commitment.price_snapshot_id
               AND json_extract(job_version.job_json,'$.schema')='compute_federation.job.v1'
               AND json_extract(job_version.job_json,'$.job_id')=NEW.job_id
               AND json_extract(job_version.job_json,'$.consumer_account_id')=
                    NEW.consumer_account_id
               AND json_extract(job_version.job_json,'$.project_id') IS NEW.project_id
               AND json_extract(job_version.job_json,'$.status')='quoted'
               AND json_extract(job_version.job_json,'$.selected_offer.provider_id')=
                    commitment.provider_id
               AND json_extract(job_version.job_json,'$.selected_offer.offer_id')=
                    commitment.offer_id
               AND json_extract(job_version.job_json,'$.selected_offer.offer_version')=
                    commitment.offer_version
               AND json_extract(job_version.job_json,'$.selected_offer.offer_digest')=
                    commitment.offer_digest
               AND json_extract(job_version.job_json,'$.price_snapshot_id')=
                    commitment.price_snapshot_id
               AND julianday(json_extract(job_version.job_json,'$.workload.deadline_at'))>
                    julianday(NEW.created_at)
               AND julianday(json_extract(job_version.job_json,'$.workload.deadline_at'))<=
                    julianday(commitment.delivery_window_ends_at)
               AND (SELECT COUNT(*) FROM compute_capacity_claim_lines line
                     WHERE line.claim_id=parent_claim.claim_id)>0
               AND (SELECT COUNT(*) FROM compute_capacity_claim_lines line
                     WHERE line.claim_id=parent_claim.claim_id)=
                   (SELECT COUNT(*) FROM json_each(
                        job_version.job_json,'$.workload.usage_limits'))
               AND NOT EXISTS (
                    SELECT 1 FROM compute_capacity_claim_lines line
                     WHERE line.claim_id=parent_claim.claim_id
                       AND NOT EXISTS (
                            SELECT 1 FROM json_each(
                                job_version.job_json,'$.workload.usage_limits') usage
                             WHERE json_extract(usage.value,'$.meter')=line.meter
                               AND json_extract(usage.value,'$.max_quantity')=
                                    line.quantity_units))
        )
        BEGIN
            SELECT RAISE(ABORT, 'delivery allocation Grant lacks exact live source');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_delivery_allocation_grants_no_replace
        BEFORE INSERT ON compute_delivery_allocation_grants
        WHEN EXISTS (SELECT 1 FROM compute_delivery_allocation_grants existing
             WHERE existing.grant_id=NEW.grant_id
                OR existing.grant_digest=NEW.grant_digest
                OR existing.commitment_id=NEW.commitment_id
                OR existing.job_id=NEW.job_id
                OR (existing.idempotency_scope=NEW.idempotency_scope
                    AND existing.idempotency_key=NEW.idempotency_key))
        BEGIN
            SELECT RAISE(ABORT, 'delivery allocation Grant cannot replace history');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_delivery_allocation_grants_no_update
        BEFORE UPDATE ON compute_delivery_allocation_grants
        BEGIN
            SELECT RAISE(ABORT, 'delivery allocation Grants are immutable');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_delivery_allocation_grants_no_delete
        BEFORE DELETE ON compute_delivery_allocation_grants
        BEGIN
            SELECT RAISE(ABORT, 'delivery allocation Grants are immutable');
        END;
        "#,
    )?;
    Ok(())
}
