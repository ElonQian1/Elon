use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_platform_reference_price_curve_reviews (
            review_id TEXT PRIMARY KEY CHECK(length(trim(review_id)) BETWEEN 1 AND 160),
            review_schema TEXT NOT NULL CHECK(review_schema=
                'compute_federation.platform_reference_price_curve_review.v1'),
            review_digest TEXT NOT NULL UNIQUE CHECK(length(review_digest)=64
                AND review_digest NOT GLOB '*[^0-9a-f]*'),
            review_json TEXT NOT NULL CHECK(json_valid(review_json)
                AND length(CAST(review_json AS BLOB))<=1048576),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            batch_id TEXT NOT NULL UNIQUE,
            batch_digest TEXT NOT NULL CHECK(length(batch_digest)=64
                AND batch_digest NOT GLOB '*[^0-9a-f]*'),
            batch_material_digest TEXT NOT NULL CHECK(length(batch_material_digest)=64
                AND batch_material_digest NOT GLOB '*[^0-9a-f]*'),
            curve_id TEXT NOT NULL CHECK(length(trim(curve_id)) BETWEEN 1 AND 160),
            curve_version INTEGER NOT NULL CHECK(
                curve_version BETWEEN 1 AND 9007199254740991),
            entry_set_digest TEXT NOT NULL CHECK(length(entry_set_digest)=64
                AND entry_set_digest NOT GLOB '*[^0-9a-f]*'),
            decision TEXT NOT NULL CHECK(
                decision IN ('approved','changes_requested','rejected')),
            review_confirmation TEXT NOT NULL CHECK(review_confirmation=
                'confirm_platform_reference_price_curve_review'),
            review_note TEXT CHECK(review_note IS NULL OR
                (length(trim(review_note)) BETWEEN 1 AND 2000
                 AND review_note=trim(review_note))),
            reviewed_by_admin_user_id TEXT NOT NULL CHECK(
                length(trim(reviewed_by_admin_user_id)) BETWEEN 1 AND 160),
            reviewed_at TEXT NOT NULL CHECK(reviewed_at GLOB
                '????-??-??T??:??:??.?????????Z' AND length(reviewed_at)=30
                AND substr(reviewed_at,20,1)='.' AND substr(reviewed_at,30,1)='Z'
                AND julianday(reviewed_at) IS NOT NULL),
            idempotency_scope TEXT NOT NULL CHECK(
                length(trim(idempotency_scope)) BETWEEN 1 AND 200),
            idempotency_key TEXT NOT NULL CHECK(
                length(trim(idempotency_key)) BETWEEN 1 AND 160),
            created_at TEXT NOT NULL,
            CHECK(decision='approved' OR review_note IS NOT NULL),
            CHECK(created_at=reviewed_at),
            UNIQUE(review_id, review_digest),
            UNIQUE(idempotency_scope, idempotency_key),
            FOREIGN KEY(batch_id, batch_digest)
                REFERENCES compute_platform_reference_price_curve_batches(batch_id, batch_digest)
                ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS compute_platform_reference_price_curve_applications (
            application_id TEXT PRIMARY KEY CHECK(
                length(trim(application_id)) BETWEEN 1 AND 160),
            application_schema TEXT NOT NULL CHECK(application_schema=
                'compute_federation.platform_reference_price_curve_application.v1'),
            application_digest TEXT NOT NULL UNIQUE CHECK(length(application_digest)=64
                AND application_digest NOT GLOB '*[^0-9a-f]*'),
            application_json TEXT NOT NULL CHECK(json_valid(application_json)
                AND length(CAST(application_json AS BLOB))<=1048576),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            batch_id TEXT NOT NULL UNIQUE,
            batch_digest TEXT NOT NULL CHECK(length(batch_digest)=64
                AND batch_digest NOT GLOB '*[^0-9a-f]*'),
            batch_material_digest TEXT NOT NULL CHECK(length(batch_material_digest)=64
                AND batch_material_digest NOT GLOB '*[^0-9a-f]*'),
            review_id TEXT NOT NULL UNIQUE,
            review_digest TEXT NOT NULL CHECK(length(review_digest)=64
                AND review_digest NOT GLOB '*[^0-9a-f]*'),
            curve_id TEXT NOT NULL CHECK(length(trim(curve_id)) BETWEEN 1 AND 160),
            curve_version INTEGER NOT NULL CHECK(
                curve_version BETWEEN 1 AND 9007199254740991),
            binding_digests_json TEXT NOT NULL CHECK(json_valid(binding_digests_json)
                AND json_type(binding_digests_json)='array'
                AND json_array_length(binding_digests_json) BETWEEN 1 AND 32
                AND length(CAST(binding_digests_json AS BLOB))<=16384),
            binding_count INTEGER NOT NULL CHECK(binding_count BETWEEN 1 AND 32),
            binding_set_digest TEXT NOT NULL CHECK(length(binding_set_digest)=64
                AND binding_set_digest NOT GLOB '*[^0-9a-f]*'),
            submitted_by_admin_user_id TEXT NOT NULL CHECK(
                length(trim(submitted_by_admin_user_id)) BETWEEN 1 AND 160),
            reviewed_by_admin_user_id TEXT NOT NULL CHECK(
                length(trim(reviewed_by_admin_user_id)) BETWEEN 1 AND 160),
            applied_by_admin_user_id TEXT NOT NULL CHECK(
                length(trim(applied_by_admin_user_id)) BETWEEN 1 AND 160),
            apply_confirmation TEXT NOT NULL CHECK(apply_confirmation=
                'confirm_platform_reference_price_curve_apply'),
            apply_note TEXT NOT NULL CHECK(length(apply_note)<=2000
                AND apply_note=trim(apply_note)),
            applied_at TEXT NOT NULL CHECK(applied_at GLOB
                '????-??-??T??:??:??.?????????Z' AND length(applied_at)=30
                AND substr(applied_at,20,1)='.' AND substr(applied_at,30,1)='Z'
                AND julianday(applied_at) IS NOT NULL),
            status TEXT NOT NULL CHECK(status='applied'),
            idempotency_scope TEXT NOT NULL CHECK(
                length(trim(idempotency_scope)) BETWEEN 1 AND 200),
            idempotency_key TEXT NOT NULL CHECK(
                length(trim(idempotency_key)) BETWEEN 1 AND 160),
            created_at TEXT NOT NULL,
            CHECK(binding_count=json_array_length(binding_digests_json)),
            CHECK(submitted_by_admin_user_id<>reviewed_by_admin_user_id),
            CHECK(created_at=applied_at),
            UNIQUE(application_id, application_digest),
            UNIQUE(curve_id, curve_version),
            UNIQUE(idempotency_scope, idempotency_key),
            FOREIGN KEY(batch_id, batch_digest)
                REFERENCES compute_platform_reference_price_curve_batches(batch_id, batch_digest)
                ON DELETE RESTRICT,
            FOREIGN KEY(review_id, review_digest)
                REFERENCES compute_platform_reference_price_curve_reviews(review_id, review_digest)
                ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS compute_platform_reference_price_curve_snapshot_bindings (
            binding_id TEXT PRIMARY KEY CHECK(length(trim(binding_id)) BETWEEN 1 AND 200),
            binding_schema TEXT NOT NULL CHECK(binding_schema=
                'compute_federation.platform_reference_price_curve_snapshot_binding.v1'),
            binding_digest TEXT NOT NULL UNIQUE CHECK(length(binding_digest)=64
                AND binding_digest NOT GLOB '*[^0-9a-f]*'),
            binding_json TEXT NOT NULL CHECK(json_valid(binding_json)
                AND length(CAST(binding_json AS BLOB))<=1048576),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            application_id TEXT NOT NULL,
            batch_id TEXT NOT NULL,
            batch_digest TEXT NOT NULL CHECK(length(batch_digest)=64
                AND batch_digest NOT GLOB '*[^0-9a-f]*'),
            review_id TEXT NOT NULL,
            review_digest TEXT NOT NULL CHECK(length(review_digest)=64
                AND review_digest NOT GLOB '*[^0-9a-f]*'),
            entry_id TEXT NOT NULL,
            entry_digest TEXT NOT NULL CHECK(length(entry_digest)=64
                AND entry_digest NOT GLOB '*[^0-9a-f]*'),
            ordinal INTEGER NOT NULL CHECK(ordinal BETWEEN 1 AND 32),
            entry_key TEXT NOT NULL CHECK(length(trim(entry_key)) BETWEEN 1 AND 160),
            curve_id TEXT NOT NULL CHECK(length(trim(curve_id)) BETWEEN 1 AND 160),
            curve_version INTEGER NOT NULL CHECK(
                curve_version BETWEEN 1 AND 9007199254740991),
            snapshot_id TEXT NOT NULL UNIQUE CHECK(length(trim(snapshot_id)) BETWEEN 1 AND 200),
            snapshot_digest TEXT NOT NULL UNIQUE CHECK(length(snapshot_digest)=64
                AND snapshot_digest NOT GLOB '*[^0-9a-f]*'),
            quote_id TEXT NOT NULL UNIQUE CHECK(length(trim(quote_id)) BETWEEN 1 AND 200),
            source_kind TEXT NOT NULL CHECK(source_kind='fallback_curve'),
            source_id TEXT NOT NULL CHECK(substr(source_id,1,25)='platform_reference_curve:'
                AND length(source_id) BETWEEN 26 AND 185),
            source_version INTEGER NOT NULL CHECK(
                source_version BETWEEN 1 AND 9007199254740991),
            source_digest TEXT NOT NULL CHECK(length(source_digest)=64
                AND source_digest NOT GLOB '*[^0-9a-f]*'),
            quoted_at TEXT NOT NULL CHECK(quoted_at GLOB
                '????-??-??T??:??:??.?????????Z' AND length(quoted_at)=30
                AND substr(quoted_at,20,1)='.' AND substr(quoted_at,30,1)='Z'
                AND julianday(quoted_at) IS NOT NULL),
            expires_at TEXT NOT NULL CHECK(expires_at GLOB
                '????-??-??T??:??:??.?????????Z' AND length(expires_at)=30
                AND substr(expires_at,20,1)='.' AND substr(expires_at,30,1)='Z'
                AND julianday(expires_at) IS NOT NULL),
            status TEXT NOT NULL CHECK(status='snapshot_registered'),
            created_at TEXT NOT NULL,
            CHECK(quoted_at<expires_at AND created_at=quoted_at),
            CHECK(source_version=curve_version AND source_digest=entry_digest),
            UNIQUE(application_id, ordinal),
            UNIQUE(application_id, binding_digest),
            UNIQUE(entry_id, entry_digest),
            FOREIGN KEY(application_id)
                REFERENCES compute_platform_reference_price_curve_applications(application_id)
                ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
            FOREIGN KEY(batch_id, batch_digest)
                REFERENCES compute_platform_reference_price_curve_batches(batch_id, batch_digest)
                ON DELETE RESTRICT,
            FOREIGN KEY(review_id, review_digest)
                REFERENCES compute_platform_reference_price_curve_reviews(review_id, review_digest)
                ON DELETE RESTRICT,
            FOREIGN KEY(entry_id, entry_digest)
                REFERENCES compute_platform_reference_price_curve_entries(entry_id, entry_digest)
                ON DELETE RESTRICT,
            FOREIGN KEY(snapshot_id) REFERENCES compute_price_snapshots(snapshot_id)
                ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
        );

        CREATE INDEX IF NOT EXISTS idx_platform_reference_curve_reviews_queue
            ON compute_platform_reference_price_curve_reviews(
                decision, reviewed_at DESC, review_id);
        CREATE INDEX IF NOT EXISTS idx_platform_reference_curve_bindings_source
            ON compute_platform_reference_price_curve_snapshot_bindings(
                source_id, source_version, source_digest, binding_id);
        "#,
    )?;
    Ok(())
}
