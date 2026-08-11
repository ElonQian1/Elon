use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_external_pool_adapter_release_requests (
            request_id TEXT PRIMARY KEY CHECK(length(trim(request_id)) BETWEEN 1 AND 160),
            request_schema TEXT NOT NULL CHECK(request_schema=
                'compute_federation.external_pool_adapter_release_request.v1'),
            request_digest TEXT NOT NULL UNIQUE CHECK(length(request_digest)=64
                AND request_digest NOT GLOB '*[^0-9a-f]*'),
            request_json TEXT NOT NULL CHECK(json_valid(request_json)
                AND length(CAST(request_json AS BLOB))<=524288),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            request_material_digest TEXT NOT NULL CHECK(length(request_material_digest)=64
                AND request_material_digest NOT GLOB '*[^0-9a-f]*'),
            adapter_id TEXT NOT NULL CHECK(length(trim(adapter_id)) BETWEEN 1 AND 160),
            release_version TEXT NOT NULL CHECK(
                length(trim(release_version)) BETWEEN 1 AND 80),
            route_kind TEXT NOT NULL CHECK(route_kind='server_adapter'),
            supported_provider_kinds_json TEXT NOT NULL CHECK(
                supported_provider_kinds_json='["external_pool"]'),
            candidate_artifact_ref TEXT NOT NULL CHECK(
                substr(candidate_artifact_ref,1,13)='artifact-ref:'
                AND length(candidate_artifact_ref) BETWEEN 14 AND 173
                AND substr(candidate_artifact_ref,14) NOT GLOB '*[^0-9A-Za-z._-]*'),
            declared_implementation_sha256 TEXT NOT NULL CHECK(
                length(declared_implementation_sha256)=64
                AND declared_implementation_sha256 NOT GLOB '*[^0-9a-f]*'),
            capabilities_json TEXT NOT NULL CHECK(json_valid(capabilities_json)
                AND json_array_length(capabilities_json)=6
                AND length(CAST(capabilities_json AS BLOB))<=16384),
            capability_set_digest TEXT NOT NULL CHECK(length(capability_set_digest)=64
                AND capability_set_digest NOT GLOB '*[^0-9a-f]*'),
            verifier_verification_kind TEXT NOT NULL CHECK(
                length(trim(verifier_verification_kind)) BETWEEN 1 AND 80),
            verifier_id TEXT NOT NULL CHECK(length(trim(verifier_id)) BETWEEN 1 AND 160),
            verifier_revision INTEGER NOT NULL CHECK(
                verifier_revision BETWEEN 1 AND 9007199254740991),
            verifier_digest TEXT NOT NULL CHECK(length(verifier_digest)=64
                AND verifier_digest NOT GLOB '*[^0-9a-f]*'),
            submit_confirmation TEXT NOT NULL CHECK(submit_confirmation=
                'confirm_external_pool_adapter_release_request'),
            submit_note TEXT NOT NULL CHECK(
                length(submit_note)<=2000 AND submit_note=trim(submit_note)),
            submitted_by_admin_user_id TEXT NOT NULL CHECK(
                length(trim(submitted_by_admin_user_id)) BETWEEN 1 AND 160),
            submitted_at TEXT NOT NULL CHECK(submitted_at GLOB
                '????-??-??T??:??:??.?????????Z' AND length(submitted_at)=30
                AND substr(submitted_at,20,1)='.' AND substr(submitted_at,30,1)='Z'
                AND julianday(submitted_at) IS NOT NULL),
            status TEXT NOT NULL CHECK(status IN (
                'submitted','approved','changes_requested','rejected','staged')),
            reviewed_by_admin_user_id TEXT CHECK(reviewed_by_admin_user_id IS NULL OR
                length(trim(reviewed_by_admin_user_id)) BETWEEN 1 AND 160),
            reviewed_at TEXT CHECK(reviewed_at IS NULL OR (reviewed_at GLOB
                '????-??-??T??:??:??.?????????Z' AND length(reviewed_at)=30
                AND substr(reviewed_at,20,1)='.' AND substr(reviewed_at,30,1)='Z'
                AND julianday(reviewed_at) IS NOT NULL)),
            applied_by_admin_user_id TEXT CHECK(applied_by_admin_user_id IS NULL OR
                length(trim(applied_by_admin_user_id)) BETWEEN 1 AND 160),
            applied_at TEXT CHECK(applied_at IS NULL OR (applied_at GLOB
                '????-??-??T??:??:??.?????????Z' AND length(applied_at)=30
                AND substr(applied_at,20,1)='.' AND substr(applied_at,30,1)='Z'
                AND julianday(applied_at) IS NOT NULL)),
            idempotency_scope TEXT NOT NULL CHECK(
                length(trim(idempotency_scope)) BETWEEN 1 AND 200),
            idempotency_key TEXT NOT NULL CHECK(
                length(trim(idempotency_key)) BETWEEN 1 AND 160),
            created_at TEXT NOT NULL CHECK(created_at GLOB
                '????-??-??T??:??:??.?????????Z' AND length(created_at)=30
                AND substr(created_at,20,1)='.' AND substr(created_at,30,1)='Z'
                AND julianday(created_at) IS NOT NULL),
            updated_at TEXT NOT NULL CHECK(updated_at GLOB
                '????-??-??T??:??:??.?????????Z' AND length(updated_at)=30
                AND substr(updated_at,20,1)='.' AND substr(updated_at,30,1)='Z'
                AND julianday(updated_at) IS NOT NULL),
            CHECK(created_at=submitted_at),
            CHECK(
                (status='submitted' AND updated_at=submitted_at
                    AND reviewed_by_admin_user_id IS NULL AND reviewed_at IS NULL
                    AND applied_by_admin_user_id IS NULL AND applied_at IS NULL)
                OR (status IN ('approved','changes_requested','rejected')
                    AND reviewed_by_admin_user_id IS NOT NULL AND reviewed_at IS NOT NULL
                    AND reviewed_by_admin_user_id<>submitted_by_admin_user_id
                    AND submitted_at<=reviewed_at AND updated_at=reviewed_at
                    AND applied_by_admin_user_id IS NULL AND applied_at IS NULL)
                OR (status='staged' AND reviewed_by_admin_user_id IS NOT NULL
                    AND reviewed_at IS NOT NULL
                    AND reviewed_by_admin_user_id<>submitted_by_admin_user_id
                    AND applied_by_admin_user_id IS NOT NULL AND applied_at IS NOT NULL
                    AND submitted_at<=reviewed_at AND reviewed_at<=applied_at
                    AND updated_at=applied_at)),
            UNIQUE(idempotency_scope, idempotency_key)
        );

        CREATE TABLE IF NOT EXISTS compute_external_pool_adapter_release_reviews (
            review_id TEXT PRIMARY KEY CHECK(length(trim(review_id)) BETWEEN 1 AND 160),
            review_schema TEXT NOT NULL CHECK(review_schema=
                'compute_federation.external_pool_adapter_release_review.v1'),
            review_digest TEXT NOT NULL UNIQUE CHECK(length(review_digest)=64
                AND review_digest NOT GLOB '*[^0-9a-f]*'),
            review_json TEXT NOT NULL CHECK(json_valid(review_json)
                AND length(CAST(review_json AS BLOB))<=262144),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            request_id TEXT NOT NULL UNIQUE,
            request_digest TEXT NOT NULL CHECK(length(request_digest)=64
                AND request_digest NOT GLOB '*[^0-9a-f]*'),
            request_material_digest TEXT NOT NULL CHECK(length(request_material_digest)=64
                AND request_material_digest NOT GLOB '*[^0-9a-f]*'),
            adapter_id TEXT NOT NULL CHECK(length(trim(adapter_id)) BETWEEN 1 AND 160),
            release_version TEXT NOT NULL CHECK(
                length(trim(release_version)) BETWEEN 1 AND 80),
            decision TEXT NOT NULL CHECK(
                decision IN ('approved','changes_requested','rejected')),
            review_confirmation TEXT NOT NULL CHECK(review_confirmation=
                'confirm_external_pool_adapter_release_review'),
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
            created_at TEXT NOT NULL CHECK(created_at GLOB
                '????-??-??T??:??:??.?????????Z' AND length(created_at)=30
                AND substr(created_at,20,1)='.' AND substr(created_at,30,1)='Z'
                AND julianday(created_at) IS NOT NULL),
            CHECK(decision='approved' OR review_note IS NOT NULL),
            CHECK(created_at=reviewed_at),
            UNIQUE(idempotency_scope, idempotency_key),
            FOREIGN KEY(request_id)
                REFERENCES compute_external_pool_adapter_release_requests(request_id)
                ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS compute_external_pool_adapter_release_admissions (
            admission_id TEXT PRIMARY KEY CHECK(length(trim(admission_id)) BETWEEN 1 AND 160),
            admission_schema TEXT NOT NULL CHECK(admission_schema=
                'compute_federation.external_pool_adapter_release_admission.v1'),
            admission_digest TEXT NOT NULL UNIQUE CHECK(length(admission_digest)=64
                AND admission_digest NOT GLOB '*[^0-9a-f]*'),
            admission_json TEXT NOT NULL CHECK(json_valid(admission_json)
                AND length(CAST(admission_json AS BLOB))<=524288),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            request_id TEXT NOT NULL UNIQUE,
            request_digest TEXT NOT NULL CHECK(length(request_digest)=64
                AND request_digest NOT GLOB '*[^0-9a-f]*'),
            request_material_digest TEXT NOT NULL CHECK(length(request_material_digest)=64
                AND request_material_digest NOT GLOB '*[^0-9a-f]*'),
            review_id TEXT NOT NULL UNIQUE,
            review_digest TEXT NOT NULL CHECK(length(review_digest)=64
                AND review_digest NOT GLOB '*[^0-9a-f]*'),
            adapter_id TEXT NOT NULL CHECK(length(trim(adapter_id)) BETWEEN 1 AND 160),
            release_version TEXT NOT NULL CHECK(
                length(trim(release_version)) BETWEEN 1 AND 80),
            route_kind TEXT NOT NULL CHECK(route_kind='server_adapter'),
            supported_provider_kinds_json TEXT NOT NULL CHECK(
                supported_provider_kinds_json='["external_pool"]'),
            candidate_artifact_ref TEXT NOT NULL CHECK(
                substr(candidate_artifact_ref,1,13)='artifact-ref:'
                AND length(candidate_artifact_ref) BETWEEN 14 AND 173
                AND substr(candidate_artifact_ref,14) NOT GLOB '*[^0-9A-Za-z._-]*'),
            declared_implementation_sha256 TEXT NOT NULL CHECK(
                length(declared_implementation_sha256)=64
                AND declared_implementation_sha256 NOT GLOB '*[^0-9a-f]*'),
            capabilities_json TEXT NOT NULL CHECK(json_valid(capabilities_json)
                AND json_array_length(capabilities_json)=6
                AND length(CAST(capabilities_json AS BLOB))<=16384),
            capability_set_digest TEXT NOT NULL CHECK(length(capability_set_digest)=64
                AND capability_set_digest NOT GLOB '*[^0-9a-f]*'),
            verifier_verification_kind TEXT NOT NULL CHECK(
                length(trim(verifier_verification_kind)) BETWEEN 1 AND 80),
            verifier_id TEXT NOT NULL CHECK(length(trim(verifier_id)) BETWEEN 1 AND 160),
            verifier_revision INTEGER NOT NULL CHECK(
                verifier_revision BETWEEN 1 AND 9007199254740991),
            verifier_digest TEXT NOT NULL CHECK(length(verifier_digest)=64
                AND verifier_digest NOT GLOB '*[^0-9a-f]*'),
            submitted_by_admin_user_id TEXT NOT NULL CHECK(
                length(trim(submitted_by_admin_user_id)) BETWEEN 1 AND 160),
            reviewed_by_admin_user_id TEXT NOT NULL CHECK(
                length(trim(reviewed_by_admin_user_id)) BETWEEN 1 AND 160),
            applied_by_admin_user_id TEXT NOT NULL CHECK(
                length(trim(applied_by_admin_user_id)) BETWEEN 1 AND 160),
            apply_confirmation TEXT NOT NULL CHECK(apply_confirmation=
                'confirm_external_pool_adapter_release_stage'),
            apply_note TEXT NOT NULL CHECK(
                length(apply_note)<=2000 AND apply_note=trim(apply_note)),
            applied_at TEXT NOT NULL CHECK(applied_at GLOB
                '????-??-??T??:??:??.?????????Z' AND length(applied_at)=30
                AND substr(applied_at,20,1)='.' AND substr(applied_at,30,1)='Z'
                AND julianday(applied_at) IS NOT NULL),
            status TEXT NOT NULL CHECK(status='staged'),
            idempotency_scope TEXT NOT NULL CHECK(
                length(trim(idempotency_scope)) BETWEEN 1 AND 200),
            idempotency_key TEXT NOT NULL CHECK(
                length(trim(idempotency_key)) BETWEEN 1 AND 160),
            created_at TEXT NOT NULL CHECK(created_at GLOB
                '????-??-??T??:??:??.?????????Z' AND length(created_at)=30
                AND substr(created_at,20,1)='.' AND substr(created_at,30,1)='Z'
                AND julianday(created_at) IS NOT NULL),
            CHECK(submitted_by_admin_user_id<>reviewed_by_admin_user_id),
            CHECK(created_at=applied_at),
            UNIQUE(adapter_id, release_version),
            UNIQUE(idempotency_scope, idempotency_key),
            FOREIGN KEY(request_id)
                REFERENCES compute_external_pool_adapter_release_requests(request_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(review_id)
                REFERENCES compute_external_pool_adapter_release_reviews(review_id)
                ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_external_pool_adapter_release_requests_submitter
            ON compute_external_pool_adapter_release_requests(
                submitted_by_admin_user_id, submitted_at DESC, request_id);
        CREATE INDEX IF NOT EXISTS idx_external_pool_adapter_release_requests_status
            ON compute_external_pool_adapter_release_requests(
                status, submitted_at, request_id);
        CREATE INDEX IF NOT EXISTS idx_external_pool_adapter_release_reviews_queue
            ON compute_external_pool_adapter_release_reviews(
                decision, reviewed_at DESC, review_id);
        CREATE INDEX IF NOT EXISTS idx_external_pool_adapter_release_admissions_time
            ON compute_external_pool_adapter_release_admissions(
                applied_at DESC, admission_id);
        "#,
    )?;
    Ok(())
}
