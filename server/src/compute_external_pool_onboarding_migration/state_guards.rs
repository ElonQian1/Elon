use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_external_pool_onboarding_request_state_update
        BEFORE UPDATE ON compute_external_pool_onboarding_requests
        WHEN NOT (
            OLD.request_id IS NEW.request_id
            AND OLD.request_schema IS NEW.request_schema
            AND OLD.request_digest IS NEW.request_digest
            AND OLD.request_json IS NEW.request_json
            AND OLD.canonicalization IS NEW.canonicalization
            AND OLD.digest_algorithm IS NEW.digest_algorithm
            AND OLD.target_provider_policy_revision IS NEW.target_provider_policy_revision
            AND OLD.target_provider_digest IS NEW.target_provider_digest
            AND OLD.target_provider_jcs IS NEW.target_provider_jcs
            AND OLD.target_provider_registry_json IS NEW.target_provider_registry_json
            AND OLD.provider_id IS NEW.provider_id
            AND OLD.provider_kind IS NEW.provider_kind
            AND OLD.provider_owner_account_id IS NEW.provider_owner_account_id
            AND OLD.settlement_account_id IS NEW.settlement_account_id
            AND OLD.adapter_id IS NEW.adapter_id
            AND OLD.adapter_release_version IS NEW.adapter_release_version
            AND OLD.adapter_config_revision IS NEW.adapter_config_revision
            AND OLD.adapter_config_digest IS NEW.adapter_config_digest
            AND OLD.non_bearer_credential_ref IS NEW.non_bearer_credential_ref
            AND OLD.credential_hint IS NEW.credential_hint
            AND OLD.external_evidence_ref IS NEW.external_evidence_ref
            AND OLD.external_evidence_sha256 IS NEW.external_evidence_sha256
            AND OLD.confirmation IS NEW.confirmation
            AND OLD.owner_note IS NEW.owner_note
            AND OLD.requested_by_owner_user_id IS NEW.requested_by_owner_user_id
            AND OLD.requested_at IS NEW.requested_at
            AND OLD.idempotency_scope IS NEW.idempotency_scope
            AND OLD.idempotency_key IS NEW.idempotency_key
            AND OLD.created_at IS NEW.created_at
            AND (
                (OLD.status='submitted'
                    AND NEW.status IN ('approved','changes_requested','rejected')
                    AND OLD.reviewed_by_user_id IS NULL AND OLD.reviewed_at IS NULL
                    AND NEW.canceled_by_owner_user_id IS NULL AND NEW.canceled_at IS NULL
                    AND NEW.applied_by_user_id IS NULL AND NEW.applied_at IS NULL
                    AND NEW.updated_at=NEW.reviewed_at
                    AND EXISTS (
                        SELECT 1 FROM compute_external_pool_onboarding_reviews review
                         WHERE review.request_id=OLD.request_id
                           AND review.request_digest=OLD.request_digest
                           AND review.decision=NEW.status
                           AND review.reviewed_by_user_id=NEW.reviewed_by_user_id
                           AND review.reviewed_at=NEW.reviewed_at))
                OR (OLD.status='submitted' AND NEW.status='canceled'
                    AND OLD.reviewed_by_user_id IS NULL AND OLD.reviewed_at IS NULL
                    AND NEW.reviewed_by_user_id IS NULL AND NEW.reviewed_at IS NULL
                    AND NEW.canceled_by_owner_user_id=OLD.provider_owner_account_id
                    AND NEW.canceled_at IS NOT NULL
                    AND OLD.requested_at<=NEW.canceled_at
                    AND NEW.updated_at=NEW.canceled_at
                    AND NEW.applied_by_user_id IS NULL AND NEW.applied_at IS NULL)
                OR (OLD.status='approved' AND NEW.status='applied'
                    AND NEW.reviewed_by_user_id IS OLD.reviewed_by_user_id
                    AND NEW.reviewed_at IS OLD.reviewed_at
                    AND NEW.canceled_by_owner_user_id IS NULL AND NEW.canceled_at IS NULL
                    AND OLD.reviewed_at<=NEW.applied_at
                    AND NEW.updated_at=NEW.applied_at
                    AND EXISTS (
                        SELECT 1 FROM compute_external_pool_onboarding_applications application
                         WHERE application.request_id=OLD.request_id
                           AND application.request_digest=OLD.request_digest
                           AND application.applied_by_user_id=NEW.applied_by_user_id
                           AND application.applied_at=NEW.applied_at))
            )
        )
        BEGIN
            SELECT RAISE(ABORT, 'external pool onboarding request state transition rejected');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_external_pool_onboarding_review_advance_request
        AFTER INSERT ON compute_external_pool_onboarding_reviews
        BEGIN
            UPDATE compute_external_pool_onboarding_requests
               SET status=NEW.decision,
                   reviewed_by_user_id=NEW.reviewed_by_user_id,
                   reviewed_at=NEW.reviewed_at,
                   updated_at=NEW.reviewed_at
             WHERE request_id=NEW.request_id
               AND request_digest=NEW.request_digest
               AND status='submitted';
            SELECT CASE WHEN changes()<>1 THEN RAISE(ABORT,
                'external pool onboarding review did not advance one submitted request') END;
        END;

        CREATE TRIGGER IF NOT EXISTS trg_external_pool_onboarding_application_advance_request
        AFTER INSERT ON compute_external_pool_onboarding_applications
        BEGIN
            UPDATE compute_external_pool_onboarding_requests
               SET status='applied',
                   applied_by_user_id=NEW.applied_by_user_id,
                   applied_at=NEW.applied_at,
                   updated_at=NEW.applied_at
             WHERE request_id=NEW.request_id
               AND request_digest=NEW.request_digest
               AND status='approved'
               AND reviewed_by_user_id=NEW.reviewed_by_user_id;
            SELECT CASE WHEN changes()<>1 THEN RAISE(ABORT,
                'external pool onboarding application did not consume one approval') END;
        END;

        CREATE TRIGGER IF NOT EXISTS trg_external_pool_onboarding_requests_no_delete
        BEFORE DELETE ON compute_external_pool_onboarding_requests
        BEGIN SELECT RAISE(ABORT, 'external pool onboarding requests cannot be deleted'); END;
        CREATE TRIGGER IF NOT EXISTS trg_external_pool_onboarding_reviews_no_update
        BEFORE UPDATE ON compute_external_pool_onboarding_reviews
        BEGIN SELECT RAISE(ABORT, 'external pool onboarding reviews are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_external_pool_onboarding_reviews_no_delete
        BEFORE DELETE ON compute_external_pool_onboarding_reviews
        BEGIN SELECT RAISE(ABORT, 'external pool onboarding reviews are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_external_pool_onboarding_applications_no_update
        BEFORE UPDATE ON compute_external_pool_onboarding_applications
        BEGIN SELECT RAISE(ABORT, 'external pool onboarding applications are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_external_pool_onboarding_applications_no_delete
        BEFORE DELETE ON compute_external_pool_onboarding_applications
        BEGIN SELECT RAISE(ABORT, 'external pool onboarding applications are append-only'); END;
        "#,
    )?;
    Ok(())
}
