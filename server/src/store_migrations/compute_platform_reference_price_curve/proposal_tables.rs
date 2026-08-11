use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_platform_reference_price_curve_batches (
            batch_id TEXT PRIMARY KEY CHECK(length(trim(batch_id)) BETWEEN 1 AND 160),
            batch_schema TEXT NOT NULL CHECK(batch_schema=
                'compute_federation.platform_reference_price_curve_batch.v1'),
            batch_digest TEXT NOT NULL UNIQUE CHECK(length(batch_digest)=64
                AND batch_digest NOT GLOB '*[^0-9a-f]*'),
            batch_json TEXT NOT NULL CHECK(json_valid(batch_json)
                AND length(CAST(batch_json AS BLOB))<=1048576),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            batch_material_digest TEXT NOT NULL CHECK(length(batch_material_digest)=64
                AND batch_material_digest NOT GLOB '*[^0-9a-f]*'),
            curve_id TEXT NOT NULL CHECK(length(trim(curve_id)) BETWEEN 1 AND 160),
            curve_version INTEGER NOT NULL CHECK(
                curve_version BETWEEN 1 AND 9007199254740991),
            methodology_kind TEXT NOT NULL CHECK(
                methodology_kind='platform_reference_fallback_v1'),
            valid_from TEXT NOT NULL,
            valid_until TEXT NOT NULL,
            quote_ttl_seconds INTEGER NOT NULL CHECK(quote_ttl_seconds BETWEEN 30 AND 3600),
            rounding_mode TEXT NOT NULL CHECK(rounding_mode='half_even'),
            entry_count INTEGER NOT NULL CHECK(entry_count BETWEEN 1 AND 32),
            entry_set_digest TEXT NOT NULL CHECK(length(entry_set_digest)=64
                AND entry_set_digest NOT GLOB '*[^0-9a-f]*'),
            confirmation TEXT NOT NULL CHECK(
                confirmation='confirm_platform_reference_price_curve_batch'),
            submission_note TEXT NOT NULL CHECK(length(submission_note)<=2000
                AND submission_note=trim(submission_note)),
            submitted_by_admin_user_id TEXT NOT NULL CHECK(
                length(trim(submitted_by_admin_user_id)) BETWEEN 1 AND 160),
            submitted_at TEXT NOT NULL,
            status TEXT NOT NULL CHECK(status IN (
                'submitted','approved','changes_requested','rejected','applied')),
            reviewed_by_admin_user_id TEXT CHECK(reviewed_by_admin_user_id IS NULL OR
                length(trim(reviewed_by_admin_user_id)) BETWEEN 1 AND 160),
            reviewed_at TEXT,
            applied_by_admin_user_id TEXT CHECK(applied_by_admin_user_id IS NULL OR
                length(trim(applied_by_admin_user_id)) BETWEEN 1 AND 160),
            applied_at TEXT,
            idempotency_scope TEXT NOT NULL CHECK(
                length(trim(idempotency_scope)) BETWEEN 1 AND 200),
            idempotency_key TEXT NOT NULL CHECK(
                length(trim(idempotency_key)) BETWEEN 1 AND 160),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            CHECK(valid_from GLOB '????-??-??T??:??:??.?????????Z'
                AND length(valid_from)=30 AND substr(valid_from,20,1)='.'
                AND substr(valid_from,30,1)='Z' AND julianday(valid_from) IS NOT NULL),
            CHECK(valid_until GLOB '????-??-??T??:??:??.?????????Z'
                AND length(valid_until)=30 AND substr(valid_until,20,1)='.'
                AND substr(valid_until,30,1)='Z' AND julianday(valid_until) IS NOT NULL),
            CHECK(submitted_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(submitted_at)=30 AND substr(submitted_at,20,1)='.'
                AND substr(submitted_at,30,1)='Z' AND julianday(submitted_at) IS NOT NULL),
            CHECK(reviewed_at IS NULL OR (reviewed_at GLOB
                '????-??-??T??:??:??.?????????Z' AND length(reviewed_at)=30
                AND substr(reviewed_at,20,1)='.' AND substr(reviewed_at,30,1)='Z'
                AND julianday(reviewed_at) IS NOT NULL)),
            CHECK(applied_at IS NULL OR (applied_at GLOB
                '????-??-??T??:??:??.?????????Z' AND length(applied_at)=30
                AND substr(applied_at,20,1)='.' AND substr(applied_at,30,1)='Z'
                AND julianday(applied_at) IS NOT NULL)),
            CHECK(created_at=submitted_at),
            CHECK(submitted_at<=valid_from AND valid_from<valid_until),
            CHECK(julianday(valid_from)+(quote_ttl_seconds/86400.0)<=julianday(valid_until)),
            CHECK(
                (status='submitted' AND updated_at=submitted_at
                    AND reviewed_by_admin_user_id IS NULL AND reviewed_at IS NULL
                    AND applied_by_admin_user_id IS NULL AND applied_at IS NULL)
                OR (status IN ('approved','changes_requested','rejected')
                    AND reviewed_by_admin_user_id IS NOT NULL AND reviewed_at IS NOT NULL
                    AND reviewed_by_admin_user_id<>submitted_by_admin_user_id
                    AND submitted_at<=reviewed_at AND updated_at=reviewed_at
                    AND applied_by_admin_user_id IS NULL AND applied_at IS NULL)
                OR (status='applied' AND reviewed_by_admin_user_id IS NOT NULL
                    AND reviewed_at IS NOT NULL
                    AND reviewed_by_admin_user_id<>submitted_by_admin_user_id
                    AND applied_by_admin_user_id IS NOT NULL AND applied_at IS NOT NULL
                    AND submitted_at<=reviewed_at AND reviewed_at<=applied_at
                    AND updated_at=applied_at)),
            UNIQUE(batch_id, batch_digest),
            UNIQUE(curve_id, curve_version),
            UNIQUE(idempotency_scope, idempotency_key)
        );

        CREATE TABLE IF NOT EXISTS compute_platform_reference_price_curve_entries (
            entry_id TEXT PRIMARY KEY CHECK(length(trim(entry_id)) BETWEEN 1 AND 200),
            entry_schema TEXT NOT NULL CHECK(entry_schema=
                'compute_federation.platform_reference_price_curve_entry.v1'),
            entry_digest TEXT NOT NULL UNIQUE CHECK(length(entry_digest)=64
                AND entry_digest NOT GLOB '*[^0-9a-f]*'),
            entry_json TEXT NOT NULL CHECK(json_valid(entry_json)
                AND length(CAST(entry_json AS BLOB))<=1048576),
            batch_id TEXT NOT NULL,
            batch_digest TEXT NOT NULL CHECK(length(batch_digest)=64
                AND batch_digest NOT GLOB '*[^0-9a-f]*'),
            ordinal INTEGER NOT NULL CHECK(ordinal BETWEEN 1 AND 32),
            entry_key TEXT NOT NULL CHECK(length(trim(entry_key)) BETWEEN 1 AND 160),
            provider_id TEXT NOT NULL CHECK(length(trim(provider_id)) BETWEEN 1 AND 160),
            offer_id TEXT NOT NULL CHECK(length(trim(offer_id)) BETWEEN 1 AND 200),
            offer_version INTEGER NOT NULL CHECK(
                offer_version BETWEEN 1 AND 9007199254740991),
            offer_digest TEXT NOT NULL CHECK(length(offer_digest)=64
                AND offer_digest NOT GLOB '*[^0-9a-f]*'),
            sku_id TEXT NOT NULL CHECK(length(trim(sku_id)) BETWEEN 1 AND 200),
            sku_digest TEXT NOT NULL CHECK(length(sku_digest)=64
                AND sku_digest NOT GLOB '*[^0-9a-f]*'),
            delivery_window_id TEXT NOT NULL CHECK(
                length(trim(delivery_window_id)) BETWEEN 1 AND 200),
            delivery_window_digest TEXT NOT NULL CHECK(length(delivery_window_digest)=64
                AND delivery_window_digest NOT GLOB '*[^0-9a-f]*'),
            pricing_mode TEXT NOT NULL CHECK(pricing_mode IN ('spot','capacity_future')),
            currency TEXT NOT NULL CHECK(currency='CNY'),
            offer_curve_id TEXT CHECK(offer_curve_id IS NULL OR
                length(trim(offer_curve_id)) BETWEEN 1 AND 160),
            offer_curve_version INTEGER CHECK(offer_curve_version IS NULL OR
                offer_curve_version BETWEEN 1 AND 9007199254740991),
            instrument_id TEXT CHECK(instrument_id IS NULL OR
                length(trim(instrument_id)) BETWEEN 1 AND 160),
            components_json TEXT NOT NULL CHECK(json_valid(components_json)
                AND json_type(components_json)='array'
                AND json_array_length(components_json) BETWEEN 1 AND 32
                AND length(CAST(components_json AS BLOB))<=262144),
            fee_rules_json TEXT NOT NULL CHECK(fee_rules_json='[]'),
            consumer_max_amount_micros INTEGER NOT NULL CHECK(
                consumer_max_amount_micros BETWEEN 0 AND 9007199254740991),
            provider_max_amount_micros INTEGER NOT NULL CHECK(
                provider_max_amount_micros BETWEEN 0 AND 9007199254740991
                AND provider_max_amount_micros<=consumer_max_amount_micros),
            created_at TEXT NOT NULL,
            CHECK((offer_curve_id IS NULL)=(offer_curve_version IS NULL)),
            CHECK((pricing_mode='spot' AND instrument_id IS NULL)
                OR (pricing_mode='capacity_future' AND instrument_id IS NOT NULL)),
            CHECK(created_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(created_at)=30 AND substr(created_at,20,1)='.'
                AND substr(created_at,30,1)='Z' AND julianday(created_at) IS NOT NULL),
            UNIQUE(batch_id, ordinal),
            UNIQUE(batch_id, entry_key),
            UNIQUE(batch_id, offer_id, offer_version, delivery_window_id),
            UNIQUE(entry_id, entry_digest),
            FOREIGN KEY(batch_id, batch_digest)
                REFERENCES compute_platform_reference_price_curve_batches(batch_id, batch_digest)
                ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
        );

        CREATE INDEX IF NOT EXISTS idx_platform_reference_curve_batches_status
            ON compute_platform_reference_price_curve_batches(
                status, submitted_at, batch_id);
        CREATE INDEX IF NOT EXISTS idx_platform_reference_curve_batches_curve
            ON compute_platform_reference_price_curve_batches(
                curve_id, curve_version, submitted_at DESC, batch_id);
        CREATE INDEX IF NOT EXISTS idx_platform_reference_curve_entries_offer
            ON compute_platform_reference_price_curve_entries(
                offer_id, offer_version, delivery_window_id, batch_id);
        "#,
    )?;
    Ok(())
}
