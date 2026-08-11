use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_release_request_state_update
        BEFORE UPDATE ON compute_external_pool_adapter_release_requests
        WHEN NOT (
            OLD.request_id IS NEW.request_id
            AND OLD.request_schema IS NEW.request_schema
            AND OLD.request_digest IS NEW.request_digest
            AND OLD.request_json IS NEW.request_json
            AND OLD.canonicalization IS NEW.canonicalization
            AND OLD.digest_algorithm IS NEW.digest_algorithm
            AND OLD.request_material_digest IS NEW.request_material_digest
            AND OLD.adapter_id IS NEW.adapter_id
            AND OLD.release_version IS NEW.release_version
            AND OLD.route_kind IS NEW.route_kind
            AND OLD.supported_provider_kinds_json IS NEW.supported_provider_kinds_json
            AND OLD.candidate_artifact_ref IS NEW.candidate_artifact_ref
            AND OLD.declared_implementation_sha256 IS NEW.declared_implementation_sha256
            AND OLD.capabilities_json IS NEW.capabilities_json
            AND OLD.capability_set_digest IS NEW.capability_set_digest
            AND OLD.verifier_verification_kind IS NEW.verifier_verification_kind
            AND OLD.verifier_id IS NEW.verifier_id
            AND OLD.verifier_revision IS NEW.verifier_revision
            AND OLD.verifier_digest IS NEW.verifier_digest
            AND OLD.submit_confirmation IS NEW.submit_confirmation
            AND OLD.submit_note IS NEW.submit_note
            AND OLD.submitted_by_admin_user_id IS NEW.submitted_by_admin_user_id
            AND OLD.submitted_at IS NEW.submitted_at
            AND OLD.idempotency_scope IS NEW.idempotency_scope
            AND OLD.idempotency_key IS NEW.idempotency_key
            AND OLD.created_at IS NEW.created_at
            AND (
                (OLD.status='submitted'
                    AND NEW.status IN ('approved','changes_requested','rejected')
                    AND OLD.reviewed_by_admin_user_id IS NULL AND OLD.reviewed_at IS NULL
                    AND OLD.applied_by_admin_user_id IS NULL AND OLD.applied_at IS NULL
                    AND NEW.reviewed_by_admin_user_id IS NOT NULL
                    AND NEW.reviewed_by_admin_user_id<>OLD.submitted_by_admin_user_id
                    AND OLD.submitted_at<=NEW.reviewed_at
                    AND NEW.applied_by_admin_user_id IS NULL AND NEW.applied_at IS NULL
                    AND NEW.updated_at=NEW.reviewed_at
                    AND EXISTS (
                        SELECT 1 FROM compute_external_pool_adapter_release_reviews review
                         WHERE review.request_id=OLD.request_id
                           AND review.request_digest=OLD.request_digest
                           AND review.request_material_digest=OLD.request_material_digest
                           AND review.adapter_id=OLD.adapter_id
                           AND review.release_version=OLD.release_version
                           AND review.decision=NEW.status
                           AND review.reviewed_by_admin_user_id=
                                NEW.reviewed_by_admin_user_id
                           AND review.reviewed_at=NEW.reviewed_at))
                OR (OLD.status='approved' AND NEW.status='staged'
                    AND NEW.reviewed_by_admin_user_id IS OLD.reviewed_by_admin_user_id
                    AND NEW.reviewed_at IS OLD.reviewed_at
                    AND OLD.applied_by_admin_user_id IS NULL AND OLD.applied_at IS NULL
                    AND NEW.applied_by_admin_user_id IS NOT NULL
                    AND OLD.reviewed_at<=NEW.applied_at
                    AND NEW.updated_at=NEW.applied_at
                    AND EXISTS (
                        SELECT 1 FROM compute_external_pool_adapter_release_admissions admission
                         WHERE admission.request_id=OLD.request_id
                           AND admission.request_digest=OLD.request_digest
                           AND admission.request_material_digest=OLD.request_material_digest
                           AND admission.adapter_id=OLD.adapter_id
                           AND admission.release_version=OLD.release_version
                           AND admission.reviewed_by_admin_user_id=
                                OLD.reviewed_by_admin_user_id
                           AND admission.applied_by_admin_user_id=
                                NEW.applied_by_admin_user_id
                           AND admission.applied_at=NEW.applied_at
                           AND admission.status='staged'))
            )
        )
        BEGIN
            SELECT RAISE(ABORT,
                'external pool Adapter release request state transition rejected');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_release_review_advance_request
        AFTER INSERT ON compute_external_pool_adapter_release_reviews
        BEGIN
            UPDATE compute_external_pool_adapter_release_requests
               SET status=NEW.decision,
                   reviewed_by_admin_user_id=NEW.reviewed_by_admin_user_id,
                   reviewed_at=NEW.reviewed_at,
                   updated_at=NEW.reviewed_at
             WHERE request_id=NEW.request_id
               AND request_digest=NEW.request_digest
               AND request_material_digest=NEW.request_material_digest
               AND adapter_id=NEW.adapter_id
               AND release_version=NEW.release_version
               AND status='submitted';
            SELECT CASE WHEN changes()<>1 THEN RAISE(ABORT,
                'external pool Adapter release review did not advance one request') END;
        END;

        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_release_admission_advance_request
        AFTER INSERT ON compute_external_pool_adapter_release_admissions
        BEGIN
            UPDATE compute_external_pool_adapter_release_requests
               SET status='staged',
                   applied_by_admin_user_id=NEW.applied_by_admin_user_id,
                   applied_at=NEW.applied_at,
                   updated_at=NEW.applied_at
             WHERE request_id=NEW.request_id
               AND request_digest=NEW.request_digest
               AND request_material_digest=NEW.request_material_digest
               AND adapter_id=NEW.adapter_id
               AND release_version=NEW.release_version
               AND status='approved'
               AND reviewed_by_admin_user_id=NEW.reviewed_by_admin_user_id;
            SELECT CASE WHEN changes()<>1 THEN RAISE(ABORT,
                'external pool Adapter release admission did not consume one approval') END;
        END;

        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_release_requests_no_delete
        BEFORE DELETE ON compute_external_pool_adapter_release_requests
        BEGIN SELECT RAISE(ABORT,
            'external pool Adapter release requests cannot be deleted'); END;
        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_release_requests_no_replace
        BEFORE INSERT ON compute_external_pool_adapter_release_requests
        WHEN EXISTS (
            SELECT 1 FROM compute_external_pool_adapter_release_requests existing
             WHERE existing.request_id=NEW.request_id
                OR existing.request_digest=NEW.request_digest
                OR (existing.idempotency_scope=NEW.idempotency_scope
                    AND existing.idempotency_key=NEW.idempotency_key))
        BEGIN SELECT RAISE(ABORT,
            'external pool Adapter release requests cannot be replaced'); END;
        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_release_reviews_no_update
        BEFORE UPDATE ON compute_external_pool_adapter_release_reviews
        BEGIN SELECT RAISE(ABORT,
            'external pool Adapter release reviews are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_release_reviews_no_delete
        BEFORE DELETE ON compute_external_pool_adapter_release_reviews
        BEGIN SELECT RAISE(ABORT,
            'external pool Adapter release reviews are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_release_reviews_no_replace
        BEFORE INSERT ON compute_external_pool_adapter_release_reviews
        WHEN EXISTS (
            SELECT 1 FROM compute_external_pool_adapter_release_reviews existing
             WHERE existing.review_id=NEW.review_id
                OR existing.review_digest=NEW.review_digest
                OR existing.request_id=NEW.request_id
                OR (existing.idempotency_scope=NEW.idempotency_scope
                    AND existing.idempotency_key=NEW.idempotency_key))
        BEGIN SELECT RAISE(ABORT,
            'external pool Adapter release reviews cannot be replaced'); END;
        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_release_admissions_no_update
        BEFORE UPDATE ON compute_external_pool_adapter_release_admissions
        BEGIN SELECT RAISE(ABORT,
            'external pool Adapter release admissions are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_release_admissions_no_delete
        BEFORE DELETE ON compute_external_pool_adapter_release_admissions
        BEGIN SELECT RAISE(ABORT,
            'external pool Adapter release admissions are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_release_admissions_no_replace
        BEFORE INSERT ON compute_external_pool_adapter_release_admissions
        WHEN EXISTS (
            SELECT 1 FROM compute_external_pool_adapter_release_admissions existing
             WHERE existing.admission_id=NEW.admission_id
                OR existing.admission_digest=NEW.admission_digest
                OR existing.request_id=NEW.request_id
                OR existing.review_id=NEW.review_id
                OR (existing.adapter_id=NEW.adapter_id
                    AND existing.release_version=NEW.release_version)
                OR (existing.idempotency_scope=NEW.idempotency_scope
                    AND existing.idempotency_key=NEW.idempotency_key))
        BEGIN SELECT RAISE(ABORT,
            'external pool Adapter release admissions cannot be replaced'); END;
        "#,
    )?;
    Ok(())
}
