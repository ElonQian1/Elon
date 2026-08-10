use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_external_pool_onboarding_requests (
            request_id TEXT PRIMARY KEY CHECK(length(trim(request_id)) BETWEEN 1 AND 160),
            request_schema TEXT NOT NULL CHECK(
                request_schema='compute_federation.external_pool_onboarding_request.v1'),
            request_digest TEXT NOT NULL UNIQUE CHECK(length(request_digest)=64
                AND request_digest NOT GLOB '*[^0-9a-f]*'),
            request_json TEXT NOT NULL CHECK(json_valid(request_json)
                AND length(CAST(request_json AS BLOB))<=524288),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            target_provider_policy_revision INTEGER NOT NULL CHECK(
                target_provider_policy_revision=1),
            target_provider_digest TEXT NOT NULL CHECK(length(target_provider_digest)=64
                AND target_provider_digest NOT GLOB '*[^0-9a-f]*'),
            target_provider_jcs TEXT NOT NULL CHECK(json_valid(target_provider_jcs)
                AND length(CAST(target_provider_jcs AS BLOB))<=524288),
            target_provider_registry_json TEXT NOT NULL CHECK(
                json_valid(target_provider_registry_json)
                AND length(CAST(target_provider_registry_json AS BLOB))<=524288),
            provider_id TEXT NOT NULL CHECK(length(trim(provider_id)) BETWEEN 1 AND 160),
            provider_kind TEXT NOT NULL CHECK(provider_kind='external_pool'),
            provider_owner_account_id TEXT NOT NULL CHECK(
                length(trim(provider_owner_account_id)) BETWEEN 1 AND 160),
            settlement_account_id TEXT NOT NULL CHECK(
                length(trim(settlement_account_id)) BETWEEN 1 AND 160),
            adapter_id TEXT NOT NULL CHECK(length(trim(adapter_id)) BETWEEN 1 AND 160),
            adapter_release_version TEXT NOT NULL CHECK(
                length(trim(adapter_release_version)) BETWEEN 1 AND 80),
            adapter_config_revision INTEGER NOT NULL CHECK(adapter_config_revision>0),
            adapter_config_digest TEXT NOT NULL CHECK(
                length(trim(adapter_config_digest)) BETWEEN 1 AND 512
                AND adapter_config_digest=trim(adapter_config_digest)),
            non_bearer_credential_ref TEXT CHECK(non_bearer_credential_ref IS NULL OR
                (substr(non_bearer_credential_ref,1,10)='vault-ref:'
                 AND length(non_bearer_credential_ref) BETWEEN 11 AND 170
                 AND substr(non_bearer_credential_ref,11) NOT GLOB '*[^0-9A-Za-z._-]*') OR
                (substr(non_bearer_credential_ref,1,12)='gateway-ref:'
                 AND length(non_bearer_credential_ref) BETWEEN 13 AND 172
                 AND substr(non_bearer_credential_ref,13) NOT GLOB '*[^0-9A-Za-z._-]*')),
            credential_hint TEXT CHECK(credential_hint IS NULL OR
                (length(trim(credential_hint)) BETWEEN 1 AND 160
                 AND credential_hint=trim(credential_hint))),
            external_evidence_ref TEXT CHECK(external_evidence_ref IS NULL OR
                (substr(external_evidence_ref,1,13)='evidence-ref:'
                 AND length(external_evidence_ref) BETWEEN 14 AND 173
                 AND substr(external_evidence_ref,14) NOT GLOB '*[^0-9A-Za-z._-]*')),
            external_evidence_sha256 TEXT CHECK(external_evidence_sha256 IS NULL OR
                (length(external_evidence_sha256)=64
                 AND external_evidence_sha256 NOT GLOB '*[^0-9a-f]*')),
            confirmation TEXT NOT NULL CHECK(
                confirmation='confirm_external_pool_onboarding_request'),
            owner_note TEXT NOT NULL CHECK(
                length(owner_note)<=2000 AND owner_note=trim(owner_note)),
            requested_by_owner_user_id TEXT NOT NULL CHECK(
                length(trim(requested_by_owner_user_id)) BETWEEN 1 AND 160),
            requested_at TEXT NOT NULL CHECK(requested_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(requested_at)=30 AND julianday(requested_at) IS NOT NULL),
            status TEXT NOT NULL CHECK(status IN (
                'submitted','approved','changes_requested','rejected','canceled','applied')),
            reviewed_by_user_id TEXT CHECK(reviewed_by_user_id IS NULL
                OR length(trim(reviewed_by_user_id)) BETWEEN 1 AND 160),
            reviewed_at TEXT CHECK(reviewed_at IS NULL OR
                (reviewed_at GLOB '????-??-??T??:??:??.?????????Z'
                 AND length(reviewed_at)=30 AND julianday(reviewed_at) IS NOT NULL)),
            canceled_by_owner_user_id TEXT CHECK(canceled_by_owner_user_id IS NULL
                OR length(trim(canceled_by_owner_user_id)) BETWEEN 1 AND 160),
            canceled_at TEXT CHECK(canceled_at IS NULL OR
                (canceled_at GLOB '????-??-??T??:??:??.?????????Z'
                 AND length(canceled_at)=30 AND julianday(canceled_at) IS NOT NULL)),
            applied_by_user_id TEXT CHECK(applied_by_user_id IS NULL
                OR length(trim(applied_by_user_id)) BETWEEN 1 AND 160),
            applied_at TEXT CHECK(applied_at IS NULL OR
                (applied_at GLOB '????-??-??T??:??:??.?????????Z'
                 AND length(applied_at)=30 AND julianday(applied_at) IS NOT NULL)),
            idempotency_scope TEXT NOT NULL CHECK(
                length(trim(idempotency_scope)) BETWEEN 1 AND 200),
            idempotency_key TEXT NOT NULL CHECK(
                length(trim(idempotency_key)) BETWEEN 1 AND 160),
            created_at TEXT NOT NULL CHECK(created_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(created_at)=30 AND julianday(created_at) IS NOT NULL),
            updated_at TEXT NOT NULL CHECK(updated_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(updated_at)=30 AND julianday(updated_at) IS NOT NULL),
            CHECK(requested_by_owner_user_id=provider_owner_account_id),
            CHECK(created_at=requested_at),
            CHECK((non_bearer_credential_ref IS NULL)=(credential_hint IS NULL)),
            CHECK((external_evidence_ref IS NULL)=(external_evidence_sha256 IS NULL)),
            CHECK(
                (status='submitted' AND updated_at=requested_at
                    AND reviewed_by_user_id IS NULL AND reviewed_at IS NULL
                    AND canceled_by_owner_user_id IS NULL AND canceled_at IS NULL
                    AND applied_by_user_id IS NULL AND applied_at IS NULL)
                OR (status IN ('approved','changes_requested','rejected')
                    AND reviewed_by_user_id IS NOT NULL AND reviewed_at IS NOT NULL
                    AND requested_at<=reviewed_at AND updated_at=reviewed_at
                    AND canceled_by_owner_user_id IS NULL AND canceled_at IS NULL
                    AND applied_by_user_id IS NULL AND applied_at IS NULL)
                OR (status='canceled' AND reviewed_by_user_id IS NULL AND reviewed_at IS NULL
                    AND canceled_by_owner_user_id=provider_owner_account_id
                    AND canceled_at IS NOT NULL AND requested_at<=canceled_at
                    AND updated_at=canceled_at AND applied_by_user_id IS NULL AND applied_at IS NULL)
                OR (status='applied' AND reviewed_by_user_id IS NOT NULL
                    AND reviewed_at IS NOT NULL AND canceled_by_owner_user_id IS NULL
                    AND canceled_at IS NULL AND applied_by_user_id IS NOT NULL
                    AND applied_at IS NOT NULL AND reviewed_at<=applied_at
                    AND updated_at=applied_at)),
            UNIQUE(idempotency_scope, idempotency_key)
        );

        CREATE TABLE IF NOT EXISTS compute_external_pool_onboarding_reviews (
            review_id TEXT PRIMARY KEY CHECK(length(trim(review_id)) BETWEEN 1 AND 160),
            review_schema TEXT NOT NULL CHECK(
                review_schema='compute_federation.external_pool_onboarding_review.v1'),
            review_digest TEXT NOT NULL UNIQUE CHECK(length(review_digest)=64
                AND review_digest NOT GLOB '*[^0-9a-f]*'),
            review_json TEXT NOT NULL CHECK(json_valid(review_json)
                AND length(CAST(review_json AS BLOB))<=262144),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            request_id TEXT NOT NULL UNIQUE,
            request_digest TEXT NOT NULL CHECK(length(request_digest)=64
                AND request_digest NOT GLOB '*[^0-9a-f]*'),
            provider_id TEXT NOT NULL CHECK(length(trim(provider_id)) BETWEEN 1 AND 160),
            provider_owner_account_id TEXT NOT NULL CHECK(
                length(trim(provider_owner_account_id)) BETWEEN 1 AND 160),
            decision TEXT NOT NULL CHECK(
                decision IN ('approved','changes_requested','rejected')),
            review_reason TEXT CHECK(review_reason IS NULL OR
                (length(trim(review_reason)) BETWEEN 1 AND 2000
                 AND review_reason=trim(review_reason))),
            reviewed_by_user_id TEXT NOT NULL CHECK(
                length(trim(reviewed_by_user_id)) BETWEEN 1 AND 160),
            reviewed_at TEXT NOT NULL CHECK(reviewed_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(reviewed_at)=30 AND julianday(reviewed_at) IS NOT NULL),
            idempotency_scope TEXT NOT NULL CHECK(
                length(trim(idempotency_scope)) BETWEEN 1 AND 200),
            idempotency_key TEXT NOT NULL CHECK(
                length(trim(idempotency_key)) BETWEEN 1 AND 160),
            created_at TEXT NOT NULL CHECK(created_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(created_at)=30 AND julianday(created_at) IS NOT NULL),
            CHECK(provider_owner_account_id<>reviewed_by_user_id),
            CHECK(decision='approved' OR review_reason IS NOT NULL),
            CHECK(created_at=reviewed_at),
            UNIQUE(idempotency_scope, idempotency_key),
            FOREIGN KEY(request_id)
                REFERENCES compute_external_pool_onboarding_requests(request_id)
                ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS compute_external_pool_onboarding_applications (
            application_id TEXT PRIMARY KEY CHECK(length(trim(application_id)) BETWEEN 1 AND 160),
            application_schema TEXT NOT NULL CHECK(
                application_schema='compute_federation.external_pool_onboarding_application.v1'),
            application_digest TEXT NOT NULL UNIQUE CHECK(length(application_digest)=64
                AND application_digest NOT GLOB '*[^0-9a-f]*'),
            application_json TEXT NOT NULL CHECK(json_valid(application_json)
                AND length(CAST(application_json AS BLOB))<=524288),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            request_id TEXT NOT NULL UNIQUE,
            request_digest TEXT NOT NULL CHECK(length(request_digest)=64
                AND request_digest NOT GLOB '*[^0-9a-f]*'),
            review_id TEXT NOT NULL UNIQUE,
            review_digest TEXT NOT NULL CHECK(length(review_digest)=64
                AND review_digest NOT GLOB '*[^0-9a-f]*'),
            provider_id TEXT NOT NULL UNIQUE,
            provider_kind TEXT NOT NULL CHECK(provider_kind='external_pool'),
            provider_owner_account_id TEXT NOT NULL CHECK(
                length(trim(provider_owner_account_id)) BETWEEN 1 AND 160),
            settlement_account_id TEXT NOT NULL CHECK(
                length(trim(settlement_account_id)) BETWEEN 1 AND 160),
            target_provider_policy_revision INTEGER NOT NULL CHECK(
                target_provider_policy_revision=1),
            target_provider_digest TEXT NOT NULL CHECK(length(target_provider_digest)=64
                AND target_provider_digest NOT GLOB '*[^0-9a-f]*'),
            target_provider_jcs TEXT NOT NULL CHECK(json_valid(target_provider_jcs)
                AND length(CAST(target_provider_jcs AS BLOB))<=524288),
            target_provider_registry_json TEXT NOT NULL CHECK(
                json_valid(target_provider_registry_json)
                AND length(CAST(target_provider_registry_json AS BLOB))<=524288),
            adapter_id TEXT NOT NULL CHECK(length(trim(adapter_id)) BETWEEN 1 AND 160),
            adapter_release_version TEXT NOT NULL CHECK(
                length(trim(adapter_release_version)) BETWEEN 1 AND 80),
            adapter_config_revision INTEGER NOT NULL CHECK(adapter_config_revision>0),
            adapter_config_digest TEXT NOT NULL CHECK(
                length(trim(adapter_config_digest)) BETWEEN 1 AND 512
                AND adapter_config_digest=trim(adapter_config_digest)),
            non_bearer_credential_ref TEXT CHECK(non_bearer_credential_ref IS NULL OR
                (substr(non_bearer_credential_ref,1,10)='vault-ref:'
                 AND length(non_bearer_credential_ref) BETWEEN 11 AND 170
                 AND substr(non_bearer_credential_ref,11) NOT GLOB '*[^0-9A-Za-z._-]*') OR
                (substr(non_bearer_credential_ref,1,12)='gateway-ref:'
                 AND length(non_bearer_credential_ref) BETWEEN 13 AND 172
                 AND substr(non_bearer_credential_ref,13) NOT GLOB '*[^0-9A-Za-z._-]*')),
            credential_hint TEXT CHECK(credential_hint IS NULL OR
                (length(trim(credential_hint)) BETWEEN 1 AND 160
                 AND credential_hint=trim(credential_hint))),
            external_evidence_ref TEXT CHECK(external_evidence_ref IS NULL OR
                (substr(external_evidence_ref,1,13)='evidence-ref:'
                 AND length(external_evidence_ref) BETWEEN 14 AND 173
                 AND substr(external_evidence_ref,14) NOT GLOB '*[^0-9A-Za-z._-]*')),
            external_evidence_sha256 TEXT CHECK(external_evidence_sha256 IS NULL OR
                (length(external_evidence_sha256)=64
                 AND external_evidence_sha256 NOT GLOB '*[^0-9a-f]*')),
            approved_by_user_id TEXT NOT NULL CHECK(
                approved_by_user_id=provider_owner_account_id),
            reviewed_by_user_id TEXT NOT NULL CHECK(
                length(trim(reviewed_by_user_id)) BETWEEN 1 AND 160
                AND reviewed_by_user_id<>provider_owner_account_id),
            apply_confirmation TEXT NOT NULL CHECK(
                apply_confirmation='confirm_external_pool_onboarding_apply'),
            applied_by_user_id TEXT NOT NULL CHECK(
                length(trim(applied_by_user_id)) BETWEEN 1 AND 160),
            applied_at TEXT NOT NULL CHECK(applied_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(applied_at)=30 AND julianday(applied_at) IS NOT NULL),
            idempotency_scope TEXT NOT NULL CHECK(
                length(trim(idempotency_scope)) BETWEEN 1 AND 200),
            idempotency_key TEXT NOT NULL CHECK(
                length(trim(idempotency_key)) BETWEEN 1 AND 160),
            created_at TEXT NOT NULL CHECK(created_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(created_at)=30 AND julianday(created_at) IS NOT NULL),
            CHECK((non_bearer_credential_ref IS NULL)=(credential_hint IS NULL)),
            CHECK((external_evidence_ref IS NULL)=(external_evidence_sha256 IS NULL)),
            CHECK(created_at=applied_at),
            UNIQUE(idempotency_scope, idempotency_key),
            FOREIGN KEY(request_id)
                REFERENCES compute_external_pool_onboarding_requests(request_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(review_id)
                REFERENCES compute_external_pool_onboarding_reviews(review_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(provider_id) REFERENCES compute_providers(provider_id) ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_external_pool_onboarding_requests_owner
            ON compute_external_pool_onboarding_requests(
                provider_owner_account_id, requested_at DESC, request_id);
        CREATE INDEX IF NOT EXISTS idx_external_pool_onboarding_requests_status
            ON compute_external_pool_onboarding_requests(status, requested_at, request_id);
        CREATE INDEX IF NOT EXISTS idx_external_pool_onboarding_reviews_queue
            ON compute_external_pool_onboarding_reviews(decision, reviewed_at DESC, review_id);
        CREATE INDEX IF NOT EXISTS idx_external_pool_onboarding_applications_owner
            ON compute_external_pool_onboarding_applications(
                provider_owner_account_id, applied_at DESC, application_id);
        "#,
    )?;
    Ok(())
}
