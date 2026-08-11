use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_compute_capacity_commitment_projection
        BEFORE INSERT ON compute_capacity_commitments
        WHEN json_extract(NEW.commitment_json,'$.schema') IS NOT NEW.commitment_schema
          OR json_extract(NEW.commitment_json,'$.commitment_id') IS NOT NEW.commitment_id
          OR json_extract(NEW.commitment_json,'$.commitment_revision')
                IS NOT NEW.commitment_revision
          OR json_extract(NEW.commitment_json,'$.commitment_status')
                IS NOT NEW.commitment_status
          OR json_extract(NEW.commitment_json,'$.commitment_digest')
                IS NOT NEW.commitment_digest
          OR json_extract(NEW.commitment_json,'$.owner_account_id') IS NOT NEW.owner_account_id
          OR json_extract(NEW.commitment_json,'$.provider.provider_id') IS NOT NEW.provider_id
          OR json_extract(NEW.commitment_json,'$.provider.policy_revision')
                IS NOT NEW.provider_policy_revision
          OR json_extract(NEW.commitment_json,'$.provider.provider_digest')
                IS NOT NEW.provider_digest
          OR json_extract(NEW.commitment_json,'$.offer.offer_id') IS NOT NEW.offer_id
          OR json_extract(NEW.commitment_json,'$.offer.offer_version') IS NOT NEW.offer_version
          OR json_extract(NEW.commitment_json,'$.offer.offer_digest') IS NOT NEW.offer_digest
          OR json_extract(NEW.commitment_json,'$.pool.pool_id') IS NOT NEW.pool_id
          OR json_extract(NEW.commitment_json,'$.pool.capacity_epoch') IS NOT NEW.capacity_epoch
          OR json_extract(NEW.commitment_json,'$.pool.pool_revision') IS NOT NEW.pool_revision
          OR json_extract(NEW.commitment_json,'$.pool.pool_digest') IS NOT NEW.pool_digest
          OR json_extract(NEW.commitment_json,'$.delivery_window.binding.window_id')
                IS NOT NEW.delivery_window_id
          OR json_extract(NEW.commitment_json,'$.delivery_window.binding.window_digest')
                IS NOT NEW.delivery_window_digest
          OR json_extract(NEW.commitment_json,'$.delivery_window.starts_at_utc')
                IS NOT NEW.delivery_window_starts_at
          OR json_extract(NEW.commitment_json,'$.delivery_window.ends_at_utc')
                IS NOT NEW.delivery_window_ends_at
          OR json_extract(NEW.commitment_json,'$.price_snapshot_id')
                IS NOT NEW.price_snapshot_id
          OR json_extract(NEW.commitment_json,'$.price_snapshot_digest')
                IS NOT NEW.price_snapshot_digest
          OR json_extract(NEW.commitment_json,'$.reference_binding.binding_id')
                IS NOT NEW.reference_binding_id
          OR json_extract(NEW.commitment_json,'$.reference_binding.binding_digest')
                IS NOT NEW.reference_binding_digest
          OR json_extract(NEW.commitment_json,'$.instrument_id') IS NOT NEW.instrument_id
          OR json_extract(NEW.commitment_json,'$.claim.claim_id') IS NOT NEW.claim_id
          OR json_extract(NEW.commitment_json,'$.claim.claim_revision') IS NOT NEW.claim_revision
          OR json_extract(NEW.commitment_json,'$.claim.claim_digest') IS NOT NEW.claim_digest
          OR json_extract(NEW.commitment_json,'$.creation_ledger.transaction_id')
                IS NOT NEW.hold_transaction_id
          OR json_extract(NEW.commitment_json,'$.creation_ledger.transaction_digest')
                IS NOT NEW.hold_transaction_digest
          OR json_extract(NEW.commitment_json,'$.creation_ledger.ledger_sequence')
                IS NOT NEW.hold_ledger_sequence
          OR json_extract(NEW.commitment_json,'$.creation_ledger.event_kind')
                IS NOT NEW.hold_event_kind
          OR json_type(NEW.commitment_json,'$.creation_ledger.causal_transaction_id')
                IS NOT 'null'
          OR json_extract(NEW.commitment_json,'$.idempotency_scope')
                IS NOT NEW.idempotency_scope
          OR json_extract(NEW.commitment_json,'$.idempotency_key') IS NOT NEW.idempotency_key
          OR json_extract(NEW.commitment_json,'$.request_digest') IS NOT NEW.request_digest
          OR json_extract(NEW.commitment_json,'$.created_at') IS NOT NEW.created_at
          OR json_extract(NEW.commitment_json,'$.expires_at') IS NOT NEW.expires_at
          OR (SELECT COUNT(*) FROM json_each(NEW.commitment_json))<>21
          OR EXISTS (
                SELECT 1 FROM json_each(NEW.commitment_json)
                 WHERE key NOT IN ('schema','commitment_id','commitment_revision',
                    'commitment_digest','commitment_status','owner_account_id','provider',
                    'offer','pool','delivery_window','price_snapshot_id',
                    'price_snapshot_digest','reference_binding','instrument_id','claim',
                    'creation_ledger','idempotency_scope','idempotency_key','request_digest',
                    'created_at','expires_at'))
          OR (SELECT COUNT(*)
                FROM json_each(NEW.commitment_json,'$.provider'))<>3
          OR EXISTS (
                SELECT 1 FROM json_each(NEW.commitment_json,'$.provider')
                 WHERE key NOT IN ('provider_id','policy_revision','provider_digest'))
          OR (SELECT COUNT(*)
                FROM json_each(NEW.commitment_json,'$.offer'))<>3
          OR EXISTS (
                SELECT 1 FROM json_each(NEW.commitment_json,'$.offer')
                 WHERE key NOT IN ('offer_id','offer_version','offer_digest'))
          OR (SELECT COUNT(*)
                FROM json_each(NEW.commitment_json,'$.pool'))<>4
          OR EXISTS (
                SELECT 1 FROM json_each(NEW.commitment_json,'$.pool')
                 WHERE key NOT IN ('pool_id','capacity_epoch','pool_revision','pool_digest'))
          OR (SELECT COUNT(*)
                FROM json_each(NEW.commitment_json,'$.delivery_window'))<>3
          OR EXISTS (
                SELECT 1 FROM json_each(NEW.commitment_json,'$.delivery_window')
                 WHERE key NOT IN ('binding','starts_at_utc','ends_at_utc'))
          OR (SELECT COUNT(*)
                FROM json_each(NEW.commitment_json,'$.delivery_window.binding'))<>2
          OR EXISTS (
                SELECT 1 FROM json_each(NEW.commitment_json,'$.delivery_window.binding')
                 WHERE key NOT IN ('window_id','window_digest'))
          OR (SELECT COUNT(*)
                FROM json_each(NEW.commitment_json,'$.reference_binding'))<>2
          OR EXISTS (
                SELECT 1 FROM json_each(NEW.commitment_json,'$.reference_binding')
                 WHERE key NOT IN ('binding_id','binding_digest'))
          OR (SELECT COUNT(*)
                FROM json_each(NEW.commitment_json,'$.claim'))<>3
          OR EXISTS (
                SELECT 1 FROM json_each(NEW.commitment_json,'$.claim')
                 WHERE key NOT IN ('claim_id','claim_revision','claim_digest'))
          OR (SELECT COUNT(*)
                FROM json_each(NEW.commitment_json,'$.creation_ledger'))<>5
          OR EXISTS (
                SELECT 1 FROM json_each(NEW.commitment_json,'$.creation_ledger')
                 WHERE key NOT IN ('transaction_id','transaction_digest',
                    'ledger_sequence','event_kind','causal_transaction_id'))
        BEGIN
            SELECT RAISE(ABORT, 'capacity commitment JSON projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_capacity_commitment_market_source
        BEFORE INSERT ON compute_capacity_commitments
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_providers provider
              JOIN compute_provider_versions provider_version
                ON provider_version.provider_id=provider.provider_id
               AND provider_version.policy_revision=provider.current_policy_revision
              JOIN compute_offers offer ON offer.offer_id=NEW.offer_id
              JOIN compute_offer_versions offer_version
                ON offer_version.offer_id=offer.offer_id
               AND offer_version.offer_version=offer.current_offer_version
              JOIN compute_capacity_pools pool ON pool.pool_id=NEW.pool_id
              JOIN compute_capacity_pool_versions pool_version
                ON pool_version.pool_id=pool.pool_id
               AND pool_version.capacity_epoch=pool.current_capacity_epoch
               AND pool_version.pool_revision=NEW.pool_revision
             WHERE provider.provider_id=NEW.provider_id
               AND provider.owner_account_id=NEW.owner_account_id
               AND provider.provider_kind<>'external_pool'
               AND provider.status='active'
               AND provider.current_policy_revision=NEW.provider_policy_revision
               AND provider.current_provider_digest=NEW.provider_digest
               AND provider_version.provider_digest=NEW.provider_digest
               AND json_extract(provider_version.provider_json,'$.schema')=
                    'compute_federation.provider.v1'
               AND json_extract(provider_version.provider_json,'$.provider_id')=NEW.provider_id
               AND json_extract(provider_version.provider_json,'$.owner_account_id')=
                    NEW.owner_account_id
               AND json_extract(provider_version.provider_json,'$.provider_kind')=
                    provider.provider_kind
               AND json_extract(provider_version.provider_json,'$.status')='active'
               AND json_extract(provider_version.provider_json,'$.policy_revision')=
                    NEW.provider_policy_revision
               AND offer.provider_id=NEW.provider_id
               AND offer.capacity_pool_id=NEW.pool_id
               AND offer.status='active'
               AND offer.current_offer_version=NEW.offer_version
               AND offer.current_offer_digest=NEW.offer_digest
               AND offer.current_provider_policy_revision=NEW.provider_policy_revision
               AND offer.current_provider_digest=NEW.provider_digest
               AND offer_version.offer_digest=NEW.offer_digest
               AND offer_version.provider_id=NEW.provider_id
               AND offer_version.provider_policy_revision=NEW.provider_policy_revision
               AND offer_version.provider_digest=NEW.provider_digest
               AND offer_version.capacity_pool_id=NEW.pool_id
               AND offer_version.capacity_epoch=NEW.capacity_epoch
               AND offer_version.pool_revision=NEW.pool_revision
               AND offer_version.pool_digest=NEW.pool_digest
               AND offer_version.status='active'
               AND julianday(offer.valid_from)<=julianday(NEW.created_at)
               AND julianday(NEW.created_at)<julianday(offer.valid_until)
               AND json_extract(offer_version.offer_json,'$.schema')=
                    'compute_federation.offer.v1'
               AND json_extract(offer_version.offer_json,'$.offer_id')=NEW.offer_id
               AND json_extract(offer_version.offer_json,'$.offer_version')=NEW.offer_version
               AND json_extract(offer_version.offer_json,'$.offer_digest')=NEW.offer_digest
               AND json_extract(offer_version.offer_json,'$.provider_id')=NEW.provider_id
               AND json_extract(offer_version.offer_json,'$.status')='active'
               AND json_extract(offer_version.offer_json,'$.capacity_pool.pool_id')=NEW.pool_id
               AND json_extract(offer_version.offer_json,'$.capacity_pool.capacity_epoch')=
                    NEW.capacity_epoch
               AND json_extract(offer_version.offer_json,'$.capacity_pool.pool_revision')=
                    NEW.pool_revision
               AND json_extract(offer_version.offer_json,'$.capacity_pool.pool_digest')=
                    NEW.pool_digest
               AND json_extract(offer_version.offer_json,'$.price_terms.pricing_mode')=
                    'capacity_future'
               AND json_extract(offer_version.offer_json,'$.price_terms.instrument_id')=
                    NEW.instrument_id
               AND EXISTS (
                    SELECT 1 FROM json_each(
                        offer_version.offer_json,'$.delivery_windows') window
                     WHERE json_extract(window.value,'$.binding.window_id')=
                            NEW.delivery_window_id
                       AND json_extract(window.value,'$.binding.window_digest')=
                            NEW.delivery_window_digest
                       AND json_extract(window.value,'$.starts_at_utc')=
                            NEW.delivery_window_starts_at
                       AND json_extract(window.value,'$.ends_at_utc')=
                            NEW.delivery_window_ends_at)
               AND pool.provider_id=NEW.provider_id
               AND pool.status='active'
               AND pool.current_capacity_epoch=NEW.capacity_epoch
               AND pool_version.pool_digest=NEW.pool_digest
               AND NOT EXISTS (
                    SELECT 1 FROM compute_capacity_pool_versions later
                     WHERE later.pool_id=NEW.pool_id
                       AND later.capacity_epoch=NEW.capacity_epoch
                       AND later.pool_revision>NEW.pool_revision)
        )
        BEGIN
            SELECT RAISE(ABORT, 'capacity commitment lacks exact current market source');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_capacity_commitment_price_source
        BEFORE INSERT ON compute_capacity_commitments
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_price_snapshots snapshot
              JOIN compute_platform_reference_price_curve_snapshot_bindings binding
                ON binding.binding_id=NEW.reference_binding_id
              JOIN compute_platform_reference_price_curve_applications application
                ON application.application_id=binding.application_id
              JOIN compute_platform_reference_price_curve_reviews review
                ON review.review_id=binding.review_id
               AND review.review_digest=binding.review_digest
              JOIN compute_platform_reference_price_curve_batches batch
                ON batch.batch_id=binding.batch_id
               AND batch.batch_digest=binding.batch_digest
             WHERE snapshot.snapshot_id=NEW.price_snapshot_id
               AND snapshot.snapshot_digest=NEW.price_snapshot_digest
               AND snapshot.pricing_mode='capacity_future'
               AND snapshot.provider_id=NEW.provider_id
               AND snapshot.offer_id=NEW.offer_id
               AND snapshot.offer_version=NEW.offer_version
               AND snapshot.offer_digest=NEW.offer_digest
               AND snapshot.sku_id=(
                    SELECT offer_version.sku_id
                      FROM compute_offer_versions offer_version
                     WHERE offer_version.offer_id=NEW.offer_id
                       AND offer_version.offer_version=NEW.offer_version)
               AND snapshot.sku_digest=(
                    SELECT offer_version.sku_digest
                      FROM compute_offer_versions offer_version
                     WHERE offer_version.offer_id=NEW.offer_id
                       AND offer_version.offer_version=NEW.offer_version)
               AND snapshot.delivery_window_id=NEW.delivery_window_id
               AND snapshot.delivery_window_digest=NEW.delivery_window_digest
               AND snapshot.currency='CNY'
               AND snapshot.trade_id IS NULL
               AND snapshot.instrument_id=NEW.instrument_id
               AND snapshot.price_source_kind='fallback_curve'
               AND julianday(snapshot.quoted_at)<=julianday(NEW.created_at)
               AND julianday(NEW.created_at)<julianday(snapshot.expires_at)
               AND json_extract(snapshot.snapshot_json,'$.schema')=
                    'compute_federation.price_snapshot.v1'
               AND json_extract(snapshot.snapshot_json,'$.snapshot_id')=NEW.price_snapshot_id
               AND json_extract(snapshot.snapshot_json,'$.snapshot_digest')=
                    NEW.price_snapshot_digest
               AND json_extract(snapshot.snapshot_json,'$.pricing_mode')='capacity_future'
               AND json_extract(snapshot.snapshot_json,'$.provider_id')=NEW.provider_id
               AND json_extract(snapshot.snapshot_json,'$.offer_id')=NEW.offer_id
               AND json_extract(snapshot.snapshot_json,'$.offer_version')=NEW.offer_version
               AND json_extract(snapshot.snapshot_json,'$.offer_digest')=NEW.offer_digest
               AND json_extract(snapshot.snapshot_json,'$.delivery_window.binding.window_id')=
                    NEW.delivery_window_id
               AND json_extract(snapshot.snapshot_json,'$.delivery_window.binding.window_digest')=
                    NEW.delivery_window_digest
               AND json_extract(snapshot.snapshot_json,'$.delivery_window.starts_at_utc')=
                    NEW.delivery_window_starts_at
               AND json_extract(snapshot.snapshot_json,'$.delivery_window.ends_at_utc')=
                    NEW.delivery_window_ends_at
               AND json_extract(snapshot.snapshot_json,'$.currency')='CNY'
               AND json_type(snapshot.snapshot_json,'$.trade_id')='null'
               AND json_extract(snapshot.snapshot_json,'$.instrument_id')=NEW.instrument_id
               AND json_extract(snapshot.snapshot_json,'$.price_source.sample_count')=0
               AND binding.binding_digest=NEW.reference_binding_digest
               AND binding.snapshot_id=NEW.price_snapshot_id
               AND binding.snapshot_digest=NEW.price_snapshot_digest
               AND binding.source_kind='fallback_curve'
               AND binding.source_id=snapshot.price_source_id
               AND binding.source_version=snapshot.price_source_version
               AND binding.source_digest=snapshot.price_source_digest
               AND binding.quoted_at=snapshot.quoted_at
               AND binding.expires_at=snapshot.expires_at
               AND binding.status='snapshot_registered'
               AND application.status='applied'
               AND review.decision='approved'
               AND batch.status='applied'
        )
        BEGIN
            SELECT RAISE(ABORT, 'capacity commitment lacks exact applied price source');
        END;
        "#,
    )?;
    Ok(())
}
