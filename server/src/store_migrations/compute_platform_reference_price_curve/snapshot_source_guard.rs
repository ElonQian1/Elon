use anyhow::{bail, Result};
use rusqlite::Connection;

pub(super) fn reject_legacy_reserved_sources(conn: &Connection) -> Result<()> {
    let reserved_exists: i64 = conn.query_row(
        "SELECT EXISTS (
             SELECT 1 FROM compute_price_snapshots
              WHERE price_source_id GLOB 'platform_reference_curve:*'
         )",
        [],
        |row| row.get(0),
    )?;
    if reserved_exists != 0 {
        bail!("legacy Price Snapshot already occupies the platform reference curve namespace");
    }
    Ok(())
}

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_snapshot_source
        BEFORE INSERT ON compute_price_snapshots
        WHEN NEW.price_source_id GLOB 'platform_reference_curve:*'
         AND NOT EXISTS (
            SELECT 1
              FROM compute_platform_reference_price_curve_snapshot_bindings binding
              JOIN compute_platform_reference_price_curve_entries entry
                ON entry.entry_id=binding.entry_id
               AND entry.entry_digest=binding.entry_digest
              JOIN compute_platform_reference_price_curve_batches batch
                ON batch.batch_id=binding.batch_id
               AND batch.batch_digest=binding.batch_digest
              JOIN compute_platform_reference_price_curve_reviews review
                ON review.review_id=binding.review_id
               AND review.review_digest=binding.review_digest
              JOIN compute_providers current_provider
                ON current_provider.provider_id=entry.provider_id
              JOIN compute_provider_versions current_provider_version
                ON current_provider_version.provider_id=current_provider.provider_id
               AND current_provider_version.policy_revision=
                    current_provider.current_policy_revision
               AND current_provider_version.provider_digest=
                    current_provider.current_provider_digest
              JOIN compute_offers current_offer
                ON current_offer.offer_id=entry.offer_id
               AND current_offer.current_offer_version=entry.offer_version
               AND current_offer.current_offer_digest=entry.offer_digest
              JOIN compute_offer_versions offer_version
                ON offer_version.offer_id=entry.offer_id
               AND offer_version.offer_version=entry.offer_version
               AND offer_version.offer_digest=entry.offer_digest
             WHERE binding.snapshot_id=NEW.snapshot_id
               AND binding.snapshot_digest=NEW.snapshot_digest
               AND binding.quote_id=NEW.quote_id
               AND binding.batch_id=batch.batch_id
               AND binding.entry_id=entry.entry_id
               AND binding.ordinal=entry.ordinal
               AND binding.entry_key=entry.entry_key
               AND binding.curve_id=batch.curve_id
               AND binding.curve_version=batch.curve_version
               AND binding.source_kind='fallback_curve'
               AND binding.source_id='platform_reference_curve:' || batch.curve_id
               AND binding.source_version=batch.curve_version
               AND binding.source_digest=entry.entry_digest
               AND binding.quoted_at=NEW.quoted_at
               AND binding.expires_at=NEW.expires_at
               AND binding.status='snapshot_registered'
               AND batch.status='approved'
               AND review.batch_id=batch.batch_id
               AND review.batch_digest=batch.batch_digest
               AND review.batch_material_digest=batch.batch_material_digest
               AND review.decision='approved'
               AND current_provider.status='active'
               AND json_extract(current_provider_version.provider_json,'$.provider_id')
                    IS current_provider.provider_id
               AND json_extract(current_provider_version.provider_json,'$.policy_revision')
                    IS current_provider.current_policy_revision
               AND json_extract(current_provider_version.provider_json,'$.status') IS 'active'
               AND current_offer.provider_id=entry.provider_id
               AND current_offer.current_provider_policy_revision=
                    current_provider.current_policy_revision
               AND current_offer.current_provider_digest=
                    current_provider.current_provider_digest
               AND current_offer.sku_id=entry.sku_id
               AND current_offer.sku_digest=entry.sku_digest
               AND current_offer.status='active'
               AND offer_version.provider_id=entry.provider_id
               AND offer_version.provider_policy_revision=
                    current_provider.current_policy_revision
               AND offer_version.provider_digest=current_provider.current_provider_digest
               AND offer_version.sku_id=entry.sku_id
               AND offer_version.sku_digest=entry.sku_digest
               AND offer_version.status='active'
               AND json_extract(offer_version.offer_json,'$.offer_id') IS entry.offer_id
               AND json_extract(offer_version.offer_json,'$.offer_version')
                    IS entry.offer_version
               AND json_extract(offer_version.offer_json,'$.offer_digest') IS entry.offer_digest
               AND json_extract(offer_version.offer_json,'$.provider_id') IS entry.provider_id
               AND json_extract(offer_version.offer_json,'$.status') IS 'active'
               AND json_extract(offer_version.offer_json,'$.sku.sku_id') IS entry.sku_id
               AND json_extract(offer_version.offer_json,'$.sku.sku_digest') IS entry.sku_digest
               AND json_extract(offer_version.offer_json,'$.valid_from')
                    IS offer_version.valid_from
               AND json_extract(offer_version.offer_json,'$.valid_until')
                    IS offer_version.valid_until
               AND julianday(offer_version.valid_from)<=julianday(NEW.quoted_at)
               AND julianday(NEW.expires_at)<=julianday(offer_version.valid_until)
               AND julianday(NEW.expires_at)<=julianday(json_extract(
                    offer_version.offer_json,'$.price_terms.valid_until'))
               AND json_extract(offer_version.offer_json,'$.price_terms.pricing_mode')
                    IS entry.pricing_mode
               AND json_extract(offer_version.offer_json,'$.price_terms.currency')
                    IS entry.currency
               AND json_extract(offer_version.offer_json,'$.price_terms.curve_id')
                    IS entry.offer_curve_id
               AND json_extract(offer_version.offer_json,'$.price_terms.curve_version')
                    IS entry.offer_curve_version
               AND json_extract(offer_version.offer_json,'$.price_terms.instrument_id')
                    IS entry.instrument_id
               AND json_type(offer_version.offer_json,'$.price_terms.components')='array'
               AND json_array_length(json_extract(
                    offer_version.offer_json,'$.price_terms.components'))
                    =json_array_length(entry.components_json)
               AND NOT EXISTS (
                    SELECT 1
                      FROM json_each(entry.components_json) expected_component
                      LEFT JOIN json_each(
                           offer_version.offer_json,'$.price_terms.components') offer_component
                        ON offer_component.key=expected_component.key
                     WHERE offer_component.key IS NULL OR offer_component.type<>'object'
                        OR json_extract(offer_component.value,'$.meter')
                             IS NOT json_extract(expected_component.value,'$.meter')
                        OR json_extract(offer_component.value,'$.unit_size')
                             IS NOT json_extract(expected_component.value,'$.unit_size')
                        OR json_extract(offer_component.value,'$.consumer_unit_price_micros')
                             IS NOT json_extract(
                                expected_component.value,'$.consumer_unit_price_micros')
                        OR json_extract(offer_component.value,'$.provider_unit_price_micros')
                             IS NOT json_extract(
                                expected_component.value,'$.provider_unit_price_micros')
                        OR json_extract(offer_component.value,'$.max_units')
                             IS NOT json_extract(expected_component.value,'$.max_units'))
               AND json_extract(offer_version.offer_json,'$.price_terms.fee_rules')
                    IS entry.fee_rules_json
               AND EXISTS (
                    SELECT 1 FROM json_each(
                        offer_version.offer_json,'$.delivery_windows') window
                     WHERE json_extract(window.value,'$.binding.window_id')
                            IS entry.delivery_window_id
                       AND json_extract(window.value,'$.binding.window_digest')
                            IS entry.delivery_window_digest
                       AND json_extract(NEW.snapshot_json,
                            '$.delivery_window.starts_at_utc')
                            IS json_extract(window.value,'$.starts_at_utc')
                       AND json_extract(NEW.snapshot_json,
                            '$.delivery_window.ends_at_utc')
                            IS json_extract(window.value,'$.ends_at_utc'))
               AND NEW.price_source_kind='fallback_curve'
               AND NEW.price_source_id=binding.source_id
               AND NEW.price_source_version=binding.source_version
               AND NEW.price_source_digest=binding.source_digest
               AND NEW.trade_id IS NULL
               AND NEW.pricing_mode=entry.pricing_mode
               AND NEW.sku_id=entry.sku_id AND NEW.sku_digest=entry.sku_digest
               AND NEW.provider_id=entry.provider_id
               AND NEW.offer_id=entry.offer_id
               AND NEW.offer_version=entry.offer_version
               AND NEW.offer_digest=entry.offer_digest
               AND NEW.delivery_window_id=entry.delivery_window_id
               AND NEW.delivery_window_digest=entry.delivery_window_digest
               AND NEW.currency=entry.currency
               AND NEW.consumer_max_amount_micros=entry.consumer_max_amount_micros
               AND NEW.provider_max_amount_micros=entry.provider_max_amount_micros
               AND NEW.instrument_id IS entry.instrument_id
               AND json_extract(NEW.snapshot_json,'$.schema')
                    IS 'compute_federation.price_snapshot.v1'
               AND json_extract(NEW.snapshot_json,'$.snapshot_id') IS NEW.snapshot_id
               AND json_extract(NEW.snapshot_json,'$.snapshot_digest') IS NEW.snapshot_digest
               AND json_extract(NEW.snapshot_json,'$.quote_id') IS NEW.quote_id
               AND json_extract(NEW.snapshot_json,'$.pricing_mode') IS NEW.pricing_mode
               AND json_extract(NEW.snapshot_json,'$.sku')
                    IS json_extract(offer_version.offer_json,'$.sku')
               AND json_extract(NEW.snapshot_json,'$.sku.sku_id') IS NEW.sku_id
               AND json_extract(NEW.snapshot_json,'$.sku.sku_digest') IS NEW.sku_digest
               AND json_extract(NEW.snapshot_json,'$.provider_id') IS NEW.provider_id
               AND json_extract(NEW.snapshot_json,'$.offer_id') IS NEW.offer_id
               AND json_extract(NEW.snapshot_json,'$.offer_version') IS NEW.offer_version
               AND json_extract(NEW.snapshot_json,'$.offer_digest') IS NEW.offer_digest
               AND json_extract(NEW.snapshot_json,'$.delivery_window.binding.window_id')
                    IS NEW.delivery_window_id
               AND json_extract(NEW.snapshot_json,'$.delivery_window.binding.window_digest')
                    IS NEW.delivery_window_digest
               AND json_extract(NEW.snapshot_json,'$.currency') IS NEW.currency
               AND json_extract(NEW.snapshot_json,'$.consumer_max_amount_micros')
                    IS NEW.consumer_max_amount_micros
               AND json_extract(NEW.snapshot_json,'$.provider_max_amount_micros')
                    IS NEW.provider_max_amount_micros
               AND json_extract(NEW.snapshot_json,'$.instrument_id') IS NEW.instrument_id
               AND json_type(NEW.snapshot_json,'$.components')='array'
               AND json_array_length(json_extract(NEW.snapshot_json,'$.components'))
                    =json_array_length(entry.components_json)
               AND NOT EXISTS (
                    SELECT 1
                      FROM json_each(entry.components_json) expected_component
                      LEFT JOIN json_each(NEW.snapshot_json,'$.components') snapshot_component
                        ON snapshot_component.key=expected_component.key
                     WHERE snapshot_component.key IS NULL OR snapshot_component.type<>'object'
                        OR json_extract(snapshot_component.value,'$.meter')
                             IS NOT json_extract(expected_component.value,'$.meter')
                        OR json_extract(snapshot_component.value,'$.unit_size')
                             IS NOT json_extract(expected_component.value,'$.unit_size')
                        OR json_extract(snapshot_component.value,'$.consumer_unit_price_micros')
                             IS NOT json_extract(
                                expected_component.value,'$.consumer_unit_price_micros')
                        OR json_extract(snapshot_component.value,'$.provider_unit_price_micros')
                             IS NOT json_extract(
                                expected_component.value,'$.provider_unit_price_micros')
                        OR json_extract(snapshot_component.value,'$.max_units')
                             IS NOT json_extract(expected_component.value,'$.max_units'))
               AND json_extract(NEW.snapshot_json,'$.fee_rules') IS entry.fee_rules_json
               AND json_extract(NEW.snapshot_json,'$.rounding_mode') IS batch.rounding_mode
               AND json_type(NEW.snapshot_json,'$.trade_id') IS 'null'
               AND json_extract(NEW.snapshot_json,'$.quoted_at') IS binding.quoted_at
               AND json_extract(NEW.snapshot_json,'$.expires_at') IS binding.expires_at
               AND json_extract(NEW.snapshot_json,'$.price_source.source_kind')
                    IS binding.source_kind
               AND json_extract(NEW.snapshot_json,'$.price_source.source_id')
                    IS binding.source_id
               AND json_extract(NEW.snapshot_json,'$.price_source.source_version')
                    IS binding.source_version
               AND json_extract(NEW.snapshot_json,'$.price_source.observation_window_end')
                    IS binding.quoted_at
               AND json_extract(NEW.snapshot_json,'$.price_source.observation_window_start')
                    GLOB '????-??-??T??:??:??.?????????Z'
               AND length(json_extract(
                    NEW.snapshot_json,'$.price_source.observation_window_start'))=30
               AND strftime(
                    '%Y-%m-%dT%H:%M:%S',
                    substr(json_extract(
                        NEW.snapshot_json,'$.price_source.observation_window_start'),1,19),
                    '+1 second'
               ) IS substr(binding.quoted_at,1,19)
               AND substr(json_extract(
                    NEW.snapshot_json,'$.price_source.observation_window_start'),20,10)
                    IS substr(binding.quoted_at,20,10)
               AND json_extract(NEW.snapshot_json,'$.price_source.sample_count') IS 0
               AND json_extract(NEW.snapshot_json,'$.price_source.source_digest')
                    IS binding.source_digest
        )
        BEGIN
            SELECT RAISE(ABORT,
                'platform reference curve Snapshot lacks exact approved source');
        END;
        "#,
    )?;
    Ok(())
}
