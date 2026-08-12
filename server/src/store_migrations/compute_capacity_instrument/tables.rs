use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_capacity_instruments (
            instrument_id TEXT NOT NULL PRIMARY KEY CHECK(
                length(instrument_id) BETWEEN 1 AND 160 AND instrument_id=trim(instrument_id)),
            instrument_schema TEXT NOT NULL CHECK(instrument_schema=
                'compute_federation.capacity_instrument.v1'),
            instrument_revision INTEGER NOT NULL CHECK(instrument_revision=1),
            instrument_digest TEXT NOT NULL UNIQUE CHECK(
                length(instrument_digest)=64
                AND instrument_digest NOT GLOB '*[^0-9a-f]*'),
            instrument_json TEXT NOT NULL CHECK(
                json_valid(instrument_json) AND json_type(instrument_json)='object'
                AND length(CAST(instrument_json AS BLOB))<=1048576),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            sku_id TEXT NOT NULL CHECK(length(sku_id) BETWEEN 1 AND 160 AND sku_id=trim(sku_id)),
            sku_digest TEXT NOT NULL CHECK(
                length(sku_digest)=64 AND sku_digest NOT GLOB '*[^0-9a-f]*'),
            delivery_window_id TEXT NOT NULL CHECK(
                length(delivery_window_id) BETWEEN 1 AND 160
                AND delivery_window_id=trim(delivery_window_id)),
            delivery_window_digest TEXT NOT NULL CHECK(
                length(delivery_window_digest)=64
                AND delivery_window_digest NOT GLOB '*[^0-9a-f]*'),
            delivery_window_starts_at TEXT NOT NULL CHECK(
                length(delivery_window_starts_at)=30
                AND delivery_window_starts_at GLOB '????-??-??T??:??:??.?????????Z'
                AND julianday(delivery_window_starts_at) IS NOT NULL
            ),
            delivery_window_ends_at TEXT NOT NULL CHECK(
                length(delivery_window_ends_at)=30
                AND delivery_window_ends_at GLOB '????-??-??T??:??:??.?????????Z'
                AND julianday(delivery_window_ends_at) IS NOT NULL
            ),
            contract_units_json TEXT NOT NULL CHECK(
                json_valid(contract_units_json) AND json_type(contract_units_json)='array'
                AND contract_units_json=json(contract_units_json)
                AND json_array_length(contract_units_json) BETWEEN 1 AND 64),
            availability_sla_tier TEXT NOT NULL CHECK(
                length(availability_sla_tier) BETWEEN 1 AND 200
                AND availability_sla_tier=trim(availability_sla_tier)),
            region_or_data_zone TEXT NOT NULL CHECK(
                length(region_or_data_zone) BETWEEN 1 AND 200
                AND region_or_data_zone=trim(region_or_data_zone)),
            verification_tier TEXT NOT NULL CHECK(
                length(verification_tier) BETWEEN 1 AND 200
                AND verification_tier=trim(verification_tier)),
            settlement_currency TEXT NOT NULL CHECK(settlement_currency='CNY'),
            settlement_unit TEXT NOT NULL CHECK(
                settlement_unit='platform_balance_cny_micros'),
            registered_by_admin_user_id TEXT NOT NULL CHECK(
                length(registered_by_admin_user_id) BETWEEN 1 AND 200
                AND registered_by_admin_user_id=trim(registered_by_admin_user_id)),
            confirmation TEXT NOT NULL CHECK(confirmation=
                'confirm_compute_capacity_instrument_registration'),
            idempotency_scope TEXT NOT NULL CHECK(
                length(idempotency_scope) BETWEEN 1 AND 200
                AND idempotency_scope=trim(idempotency_scope)),
            idempotency_key TEXT NOT NULL CHECK(
                length(idempotency_key) BETWEEN 1 AND 200
                AND idempotency_key=trim(idempotency_key)),
            registered_at TEXT NOT NULL CHECK(
                length(registered_at)=30
                AND registered_at GLOB '????-??-??T??:??:??.?????????Z'
                AND julianday(registered_at) IS NOT NULL),
            recorded_at TEXT NOT NULL CHECK(recorded_at=registered_at),
            CHECK(julianday(delivery_window_starts_at)<julianday(delivery_window_ends_at)),
            UNIQUE(idempotency_scope, idempotency_key),
            UNIQUE(instrument_id, instrument_revision, instrument_digest)
        );

        CREATE TABLE IF NOT EXISTS compute_capacity_instrument_activations (
            activation_receipt_id TEXT NOT NULL PRIMARY KEY CHECK(
                length(activation_receipt_id) BETWEEN 1 AND 200
                AND activation_receipt_id=trim(activation_receipt_id)),
            activation_schema TEXT NOT NULL CHECK(activation_schema=
                'compute_federation.capacity_instrument_activation_receipt.v1'),
            activation_receipt_digest TEXT NOT NULL UNIQUE CHECK(
                length(activation_receipt_digest)=64
                AND activation_receipt_digest NOT GLOB '*[^0-9a-f]*'),
            activation_receipt_json TEXT NOT NULL CHECK(
                json_valid(activation_receipt_json)
                AND json_type(activation_receipt_json)='object'
                AND length(CAST(activation_receipt_json AS BLOB))<=262144),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            instrument_id TEXT NOT NULL UNIQUE,
            instrument_revision INTEGER NOT NULL CHECK(instrument_revision=1),
            instrument_digest TEXT NOT NULL CHECK(
                length(instrument_digest)=64
                AND instrument_digest NOT GLOB '*[^0-9a-f]*'),
            activated_by_admin_user_id TEXT NOT NULL CHECK(
                length(activated_by_admin_user_id) BETWEEN 1 AND 200
                AND activated_by_admin_user_id=trim(activated_by_admin_user_id)),
            confirmation TEXT NOT NULL CHECK(confirmation=
                'confirm_compute_capacity_instrument_activation'),
            idempotency_scope TEXT NOT NULL CHECK(
                length(idempotency_scope) BETWEEN 1 AND 200
                AND idempotency_scope=trim(idempotency_scope)),
            idempotency_key TEXT NOT NULL CHECK(
                length(idempotency_key) BETWEEN 1 AND 200
                AND idempotency_key=trim(idempotency_key)),
            activated_at TEXT NOT NULL CHECK(
                length(activated_at)=30
                AND activated_at GLOB '????-??-??T??:??:??.?????????Z'
                AND julianday(activated_at) IS NOT NULL),
            recorded_at TEXT NOT NULL CHECK(recorded_at=activated_at),
            UNIQUE(idempotency_scope, idempotency_key),
            UNIQUE(activation_receipt_id, activation_receipt_digest,
                instrument_id, instrument_revision, instrument_digest),
            FOREIGN KEY(instrument_id, instrument_revision, instrument_digest)
                REFERENCES compute_capacity_instruments(
                    instrument_id, instrument_revision, instrument_digest) ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS compute_capacity_instrument_retirements (
            retirement_receipt_id TEXT NOT NULL PRIMARY KEY CHECK(
                length(retirement_receipt_id) BETWEEN 1 AND 200
                AND retirement_receipt_id=trim(retirement_receipt_id)),
            retirement_schema TEXT NOT NULL CHECK(retirement_schema=
                'compute_federation.capacity_instrument_retirement_receipt.v1'),
            retirement_receipt_digest TEXT NOT NULL UNIQUE CHECK(
                length(retirement_receipt_digest)=64
                AND retirement_receipt_digest NOT GLOB '*[^0-9a-f]*'),
            retirement_receipt_json TEXT NOT NULL CHECK(
                json_valid(retirement_receipt_json)
                AND json_type(retirement_receipt_json)='object'
                AND length(CAST(retirement_receipt_json AS BLOB))<=262144),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            instrument_id TEXT NOT NULL UNIQUE,
            instrument_revision INTEGER NOT NULL CHECK(instrument_revision=1),
            instrument_digest TEXT NOT NULL CHECK(
                length(instrument_digest)=64
                AND instrument_digest NOT GLOB '*[^0-9a-f]*'),
            retired_by_admin_user_id TEXT NOT NULL CHECK(
                length(retired_by_admin_user_id) BETWEEN 1 AND 200
                AND retired_by_admin_user_id=trim(retired_by_admin_user_id)),
            reason TEXT NOT NULL CHECK(
                length(reason) BETWEEN 8 AND 2000 AND reason=trim(reason)),
            confirmation TEXT NOT NULL CHECK(confirmation=
                'confirm_compute_capacity_instrument_retirement'),
            idempotency_scope TEXT NOT NULL CHECK(
                length(idempotency_scope) BETWEEN 1 AND 200
                AND idempotency_scope=trim(idempotency_scope)),
            idempotency_key TEXT NOT NULL CHECK(
                length(idempotency_key) BETWEEN 1 AND 200
                AND idempotency_key=trim(idempotency_key)),
            retired_at TEXT NOT NULL CHECK(
                length(retired_at)=30
                AND retired_at GLOB '????-??-??T??:??:??.?????????Z'
                AND julianday(retired_at) IS NOT NULL),
            recorded_at TEXT NOT NULL CHECK(recorded_at=retired_at),
            UNIQUE(idempotency_scope, idempotency_key),
            FOREIGN KEY(instrument_id, instrument_revision, instrument_digest)
                REFERENCES compute_capacity_instruments(
                    instrument_id, instrument_revision, instrument_digest) ON DELETE RESTRICT
        );

        CREATE UNIQUE INDEX IF NOT EXISTS ux_compute_offer_publications_exact
            ON compute_offer_publications(publication_id, publication_digest);

        CREATE TABLE IF NOT EXISTS compute_capacity_instrument_offer_adoptions (
            adoption_receipt_id TEXT NOT NULL PRIMARY KEY CHECK(
                length(adoption_receipt_id) BETWEEN 1 AND 200
                AND adoption_receipt_id=trim(adoption_receipt_id)),
            adoption_schema TEXT NOT NULL CHECK(adoption_schema=
                'compute_federation.capacity_instrument_offer_adoption_receipt.v1'),
            adoption_receipt_digest TEXT NOT NULL UNIQUE CHECK(
                length(adoption_receipt_digest)=64
                AND adoption_receipt_digest NOT GLOB '*[^0-9a-f]*'),
            adoption_receipt_json TEXT NOT NULL CHECK(
                json_valid(adoption_receipt_json)
                AND json_type(adoption_receipt_json)='object'
                AND length(CAST(adoption_receipt_json AS BLOB))<=524288),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            instrument_id TEXT NOT NULL,
            instrument_revision INTEGER NOT NULL CHECK(instrument_revision=1),
            instrument_digest TEXT NOT NULL CHECK(
                length(instrument_digest)=64
                AND instrument_digest NOT GLOB '*[^0-9a-f]*'),
            offer_id TEXT NOT NULL CHECK(length(offer_id) BETWEEN 1 AND 200
                AND offer_id=trim(offer_id)),
            offer_version INTEGER NOT NULL CHECK(
                offer_version BETWEEN 1 AND 9007199254740991),
            offer_digest TEXT NOT NULL CHECK(
                length(offer_digest)=64 AND offer_digest NOT GLOB '*[^0-9a-f]*'),
            publication_id TEXT NOT NULL UNIQUE,
            publication_digest TEXT NOT NULL CHECK(
                length(publication_digest)=64
                AND publication_digest NOT GLOB '*[^0-9a-f]*'),
            adopted_by_admin_user_id TEXT NOT NULL CHECK(
                length(adopted_by_admin_user_id) BETWEEN 1 AND 200
                AND adopted_by_admin_user_id=trim(adopted_by_admin_user_id)),
            confirmation TEXT NOT NULL CHECK(confirmation=
                'confirm_compute_capacity_instrument_offer_adoption'),
            idempotency_scope TEXT NOT NULL CHECK(
                length(idempotency_scope) BETWEEN 1 AND 200
                AND idempotency_scope=trim(idempotency_scope)),
            idempotency_key TEXT NOT NULL CHECK(
                length(idempotency_key) BETWEEN 1 AND 200
                AND idempotency_key=trim(idempotency_key)),
            adopted_at TEXT NOT NULL CHECK(
                length(adopted_at)=30
                AND adopted_at GLOB '????-??-??T??:??:??.?????????Z'
                AND julianday(adopted_at) IS NOT NULL),
            recorded_at TEXT NOT NULL CHECK(recorded_at=adopted_at),
            UNIQUE(offer_id, offer_version),
            UNIQUE(idempotency_scope, idempotency_key),
            FOREIGN KEY(instrument_id, instrument_revision, instrument_digest)
                REFERENCES compute_capacity_instruments(
                    instrument_id, instrument_revision, instrument_digest) ON DELETE RESTRICT,
            FOREIGN KEY(offer_id, offer_version)
                REFERENCES compute_offer_versions(offer_id, offer_version) ON DELETE RESTRICT,
            FOREIGN KEY(offer_id, offer_digest)
                REFERENCES compute_offer_versions(offer_id, offer_digest) ON DELETE RESTRICT,
            FOREIGN KEY(publication_id, publication_digest)
                REFERENCES compute_offer_publications(publication_id, publication_digest)
                ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_capacity_instrument_sku
            ON compute_capacity_instruments(sku_id, sku_digest, instrument_id);
        CREATE INDEX IF NOT EXISTS idx_capacity_instrument_adoption_instrument
            ON compute_capacity_instrument_offer_adoptions(
                instrument_id, instrument_revision, instrument_digest, offer_id);

        DROP VIEW IF EXISTS compute_capacity_instrument_current;
        CREATE VIEW compute_capacity_instrument_current AS
        SELECT root.*,
               CASE WHEN retired.retirement_receipt_id IS NOT NULL THEN 'retired'
                    WHEN active.activation_receipt_id IS NOT NULL THEN 'active'
                    ELSE 'registered' END AS current_status,
               active.activation_receipt_id, active.activation_receipt_digest,
               active.activated_by_admin_user_id, active.activated_at,
               retired.retirement_receipt_id, retired.retirement_receipt_digest,
               retired.retired_by_admin_user_id, retired.reason AS retirement_reason,
               retired.retired_at
          FROM compute_capacity_instruments root
          LEFT JOIN compute_capacity_instrument_activations active
            ON active.instrument_id=root.instrument_id
           AND active.instrument_revision=root.instrument_revision
           AND active.instrument_digest=root.instrument_digest
          LEFT JOIN compute_capacity_instrument_retirements retired
            ON retired.instrument_id=root.instrument_id
           AND retired.instrument_revision=root.instrument_revision
           AND retired.instrument_digest=root.instrument_digest;
        "#,
    )?;
    Ok(())
}
