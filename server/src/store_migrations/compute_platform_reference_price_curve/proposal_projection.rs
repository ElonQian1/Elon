use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_batch_projection
        BEFORE INSERT ON compute_platform_reference_price_curve_batches
        WHEN json_extract(NEW.batch_json,'$.schema') IS NOT NEW.batch_schema
          OR json_extract(NEW.batch_json,'$.batch_id') IS NOT NEW.batch_id
          OR json_extract(NEW.batch_json,'$.batch_digest') IS NOT NEW.batch_digest
          OR json_extract(NEW.batch_json,'$.batch_material_digest')
                IS NOT NEW.batch_material_digest
          OR json_extract(NEW.batch_json,'$.canonicalization') IS NOT NEW.canonicalization
          OR json_extract(NEW.batch_json,'$.digest_algorithm') IS NOT NEW.digest_algorithm
          OR json_extract(NEW.batch_json,'$.batch.curve_id') IS NOT NEW.curve_id
          OR json_extract(NEW.batch_json,'$.batch.curve_version') IS NOT NEW.curve_version
          OR json_extract(NEW.batch_json,'$.batch.methodology_kind')
                IS NOT NEW.methodology_kind
          OR json_extract(NEW.batch_json,'$.batch.valid_from') IS NOT NEW.valid_from
          OR json_extract(NEW.batch_json,'$.batch.valid_until') IS NOT NEW.valid_until
          OR json_extract(NEW.batch_json,'$.batch.quote_ttl_seconds')
                IS NOT NEW.quote_ttl_seconds
          OR json_extract(NEW.batch_json,'$.batch.rounding_mode') IS NOT NEW.rounding_mode
          OR json_type(NEW.batch_json,'$.batch.entries') IS NOT 'array'
          OR json_array_length(json_extract(NEW.batch_json,'$.batch.entries'))
                IS NOT NEW.entry_count
          OR json_extract(NEW.batch_json,'$.batch.entry_set_digest')
                IS NOT NEW.entry_set_digest
          OR json_extract(NEW.batch_json,'$.batch.confirmation') IS NOT NEW.confirmation
          OR json_extract(NEW.batch_json,'$.batch.submission_note') IS NOT NEW.submission_note
          OR json_extract(NEW.batch_json,'$.batch.submitted_by_admin_user_id')
                IS NOT NEW.submitted_by_admin_user_id
          OR json_extract(NEW.batch_json,'$.batch.submitted_at') IS NOT NEW.submitted_at
          OR json_extract(NEW.batch_json,'$.batch.idempotency_key') IS NOT NEW.idempotency_key
          OR NEW.status<>'submitted'
          OR NEW.reviewed_by_admin_user_id IS NOT NULL OR NEW.reviewed_at IS NOT NULL
          OR NEW.applied_by_admin_user_id IS NOT NULL OR NEW.applied_at IS NOT NULL
          OR NEW.created_at<>NEW.submitted_at OR NEW.updated_at<>NEW.submitted_at
          OR (SELECT COUNT(*)
                FROM compute_platform_reference_price_curve_entries entry
               WHERE entry.batch_id=NEW.batch_id
                 AND entry.batch_digest=NEW.batch_digest)<>NEW.entry_count
          OR EXISTS (
                SELECT 1 FROM json_each(NEW.batch_json,'$.batch.entries') expected
                 WHERE NOT EXISTS (
                    SELECT 1 FROM compute_platform_reference_price_curve_entries entry
                     WHERE entry.batch_id=NEW.batch_id
                       AND entry.batch_digest=NEW.batch_digest
                       AND entry.ordinal=CAST(expected.key AS INTEGER)+1
                       AND entry.created_at=NEW.submitted_at
                       AND json_extract(entry.entry_json,'$.entry') IS expected.value))
        BEGIN
            SELECT RAISE(ABORT, 'platform reference price curve batch projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_entry_projection
        BEFORE INSERT ON compute_platform_reference_price_curve_entries
        WHEN json_extract(NEW.entry_json,'$.schema') IS NOT NEW.entry_schema
          OR json_extract(NEW.entry_json,'$.batch_id') IS NOT NEW.batch_id
          OR json_extract(NEW.entry_json,'$.batch_digest') IS NOT NEW.batch_digest
          OR json_extract(NEW.entry_json,'$.entry_id') IS NOT NEW.entry_id
          OR json_extract(NEW.entry_json,'$.entry_digest') IS NOT NEW.entry_digest
          OR json_extract(NEW.entry_json,'$.ordinal') IS NOT NEW.ordinal
          OR json_extract(NEW.entry_json,'$.entry.entry_key') IS NOT NEW.entry_key
          OR json_extract(NEW.entry_json,'$.entry.provider_id') IS NOT NEW.provider_id
          OR json_extract(NEW.entry_json,'$.entry.offer_id') IS NOT NEW.offer_id
          OR json_extract(NEW.entry_json,'$.entry.offer_version') IS NOT NEW.offer_version
          OR json_extract(NEW.entry_json,'$.entry.offer_digest') IS NOT NEW.offer_digest
          OR json_extract(NEW.entry_json,'$.entry.sku_id') IS NOT NEW.sku_id
          OR json_extract(NEW.entry_json,'$.entry.sku_digest') IS NOT NEW.sku_digest
          OR json_extract(NEW.entry_json,'$.entry.delivery_window_id')
                IS NOT NEW.delivery_window_id
          OR json_extract(NEW.entry_json,'$.entry.delivery_window_digest')
                IS NOT NEW.delivery_window_digest
          OR json_extract(NEW.entry_json,'$.entry.pricing_mode') IS NOT NEW.pricing_mode
          OR json_extract(NEW.entry_json,'$.entry.currency') IS NOT NEW.currency
          OR json_extract(NEW.entry_json,'$.entry.offer_curve_id') IS NOT NEW.offer_curve_id
          OR json_extract(NEW.entry_json,'$.entry.offer_curve_version')
                IS NOT NEW.offer_curve_version
          OR json_extract(NEW.entry_json,'$.entry.instrument_id') IS NOT NEW.instrument_id
          OR json_extract(NEW.entry_json,'$.entry.components') IS NOT NEW.components_json
          OR json_extract(NEW.entry_json,'$.entry.fee_rules') IS NOT NEW.fee_rules_json
          OR json_extract(NEW.entry_json,'$.entry.consumer_max_amount_micros')
                IS NOT NEW.consumer_max_amount_micros
          OR json_extract(NEW.entry_json,'$.entry.provider_max_amount_micros')
                IS NOT NEW.provider_max_amount_micros
        BEGIN
            SELECT RAISE(ABORT, 'platform reference price curve entry projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_entry_order
        BEFORE INSERT ON compute_platform_reference_price_curve_entries
        WHEN EXISTS (
            SELECT 1 FROM compute_platform_reference_price_curve_entries existing
             WHERE existing.batch_id=NEW.batch_id
               AND ((existing.ordinal<NEW.ordinal AND existing.entry_key>=NEW.entry_key)
                 OR (existing.ordinal>NEW.ordinal AND existing.entry_key<=NEW.entry_key))
        )
        BEGIN
            SELECT RAISE(ABORT, 'platform reference price curve entries are not ordered');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_entry_components
        BEFORE INSERT ON compute_platform_reference_price_curve_entries
        WHEN EXISTS (
            SELECT 1 FROM json_each(NEW.components_json) component
             WHERE component.type<>'object'
                OR json_type(component.value,'$.meter') IS NOT 'text'
                OR length(trim(json_extract(component.value,'$.meter'))) NOT BETWEEN 1 AND 80
                OR json_type(component.value,'$.unit_size') IS NOT 'integer'
                OR json_extract(component.value,'$.unit_size') NOT BETWEEN 1 AND 9007199254740991
                OR json_type(component.value,'$.consumer_unit_price_micros') IS NOT 'integer'
                OR json_extract(component.value,'$.consumer_unit_price_micros')
                    NOT BETWEEN 0 AND 9007199254740991
                OR json_type(component.value,'$.provider_unit_price_micros') IS NOT 'integer'
                OR json_extract(component.value,'$.provider_unit_price_micros')
                    NOT BETWEEN 0 AND 9007199254740991
                OR json_extract(component.value,'$.provider_unit_price_micros')
                    > json_extract(component.value,'$.consumer_unit_price_micros')
                OR json_type(component.value,'$.max_units') IS NOT 'integer'
                OR json_extract(component.value,'$.max_units') NOT BETWEEN 1 AND 9007199254740991
                OR json_extract(component.value,'$.max_units')
                    % json_extract(component.value,'$.unit_size')<>0
        ) OR EXISTS (
            SELECT 1
              FROM json_each(NEW.components_json) left_component
              JOIN json_each(NEW.components_json) right_component
                ON CAST(left_component.key AS INTEGER)<CAST(right_component.key AS INTEGER)
             WHERE json_extract(left_component.value,'$.meter')
                    IS json_extract(right_component.value,'$.meter')
        )
        BEGIN
            SELECT RAISE(ABORT, 'platform reference price curve components are invalid');
        END;
        "#,
    )?;
    Ok(())
}
