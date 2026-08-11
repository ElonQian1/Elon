use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_release_request_projection
        BEFORE INSERT ON compute_external_pool_adapter_release_requests
        WHEN json_extract(NEW.request_json,'$.schema') IS NOT NEW.request_schema
          OR json_extract(NEW.request_json,'$.request_id') IS NOT NEW.request_id
          OR json_extract(NEW.request_json,'$.request_digest') IS NOT NEW.request_digest
          OR json_extract(NEW.request_json,'$.request_material_digest')
                IS NOT NEW.request_material_digest
          OR json_extract(NEW.request_json,'$.canonicalization') IS NOT NEW.canonicalization
          OR json_extract(NEW.request_json,'$.digest_algorithm') IS NOT NEW.digest_algorithm
          OR json_extract(NEW.request_json,'$.request.release.adapter_id') IS NOT NEW.adapter_id
          OR json_extract(NEW.request_json,'$.request.release.release_version')
                IS NOT NEW.release_version
          OR json_extract(NEW.request_json,'$.request.release.route_kind') IS NOT NEW.route_kind
          OR json_extract(NEW.request_json,'$.request.release.supported_provider_kinds')
                IS NOT NEW.supported_provider_kinds_json
          OR json_extract(NEW.request_json,'$.request.release.candidate_artifact_ref')
                IS NOT NEW.candidate_artifact_ref
          OR json_extract(NEW.request_json,'$.request.release.declared_implementation_sha256')
                IS NOT NEW.declared_implementation_sha256
          OR json_extract(NEW.request_json,'$.request.release.supported_capabilities')
                IS NOT NEW.capabilities_json
          OR json_extract(NEW.request_json,'$.request.release.capability_set_digest')
                IS NOT NEW.capability_set_digest
          OR json_extract(NEW.request_json,
                '$.request.release.expected_credential_verifier.verification_kind')
                IS NOT NEW.verifier_verification_kind
          OR json_extract(NEW.request_json,
                '$.request.release.expected_credential_verifier.verifier_id')
                IS NOT NEW.verifier_id
          OR json_extract(NEW.request_json,
                '$.request.release.expected_credential_verifier.verifier_revision')
                IS NOT NEW.verifier_revision
          OR json_extract(NEW.request_json,
                '$.request.release.expected_credential_verifier.verifier_digest')
                IS NOT NEW.verifier_digest
          OR json_extract(NEW.request_json,'$.request.confirmation')
                IS NOT NEW.submit_confirmation
          OR json_extract(NEW.request_json,'$.request.submission_note') IS NOT NEW.submit_note
          OR json_extract(NEW.request_json,'$.request.submitted_by_admin_user_id')
                IS NOT NEW.submitted_by_admin_user_id
          OR json_extract(NEW.request_json,'$.request.submitted_at') IS NOT NEW.submitted_at
          OR json_extract(NEW.request_json,'$.request.idempotency_key')
                IS NOT NEW.idempotency_key
          OR NEW.status IS NOT 'submitted'
          OR NEW.reviewed_by_admin_user_id IS NOT NULL
          OR NEW.reviewed_at IS NOT NULL
          OR NEW.applied_by_admin_user_id IS NOT NULL
          OR NEW.applied_at IS NOT NULL
          OR NEW.created_at IS NOT NEW.submitted_at
          OR NEW.updated_at IS NOT NEW.submitted_at
        BEGIN
            SELECT RAISE(ABORT,
                'external pool Adapter release request projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_release_request_capabilities
        BEFORE INSERT ON compute_external_pool_adapter_release_requests
        WHEN json_type(NEW.capabilities_json,'$') IS NOT 'array'
          OR json_array_length(NEW.capabilities_json)<>6
          OR json_extract(NEW.capabilities_json,'$[0].capability_id')
                IS NOT 'authenticated_ack'
          OR json_extract(NEW.capabilities_json,'$[1].capability_id')
                IS NOT 'authenticated_events'
          OR json_extract(NEW.capabilities_json,'$[2].capability_id')
                IS NOT 'cancel_no_start'
          OR json_extract(NEW.capabilities_json,'$[3].capability_id')
                IS NOT 'idempotent_commit'
          OR json_extract(NEW.capabilities_json,'$[4].capability_id') IS NOT 'prepare'
          OR json_extract(NEW.capabilities_json,'$[5].capability_id') IS NOT 'reconcile'
          OR json_type(NEW.capabilities_json,'$[0].capability_revision') IS NOT 'integer'
          OR json_type(NEW.capabilities_json,'$[1].capability_revision') IS NOT 'integer'
          OR json_type(NEW.capabilities_json,'$[2].capability_revision') IS NOT 'integer'
          OR json_type(NEW.capabilities_json,'$[3].capability_revision') IS NOT 'integer'
          OR json_type(NEW.capabilities_json,'$[4].capability_revision') IS NOT 'integer'
          OR json_type(NEW.capabilities_json,'$[5].capability_revision') IS NOT 'integer'
          OR json_extract(NEW.capabilities_json,'$[0].capability_revision') NOT BETWEEN 1 AND 9007199254740991
          OR json_extract(NEW.capabilities_json,'$[1].capability_revision') NOT BETWEEN 1 AND 9007199254740991
          OR json_extract(NEW.capabilities_json,'$[2].capability_revision') NOT BETWEEN 1 AND 9007199254740991
          OR json_extract(NEW.capabilities_json,'$[3].capability_revision') NOT BETWEEN 1 AND 9007199254740991
          OR json_extract(NEW.capabilities_json,'$[4].capability_revision') NOT BETWEEN 1 AND 9007199254740991
          OR json_extract(NEW.capabilities_json,'$[5].capability_revision') NOT BETWEEN 1 AND 9007199254740991
        BEGIN
            SELECT RAISE(ABORT,
                'external pool Adapter release request capabilities mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_release_review_projection
        BEFORE INSERT ON compute_external_pool_adapter_release_reviews
        WHEN json_extract(NEW.review_json,'$.schema') IS NOT NEW.review_schema
          OR json_extract(NEW.review_json,'$.review_id') IS NOT NEW.review_id
          OR json_extract(NEW.review_json,'$.review_digest') IS NOT NEW.review_digest
          OR json_extract(NEW.review_json,'$.canonicalization') IS NOT NEW.canonicalization
          OR json_extract(NEW.review_json,'$.digest_algorithm') IS NOT NEW.digest_algorithm
          OR json_extract(NEW.review_json,'$.review.request_id') IS NOT NEW.request_id
          OR json_extract(NEW.review_json,'$.review.request_digest') IS NOT NEW.request_digest
          OR json_extract(NEW.review_json,'$.review.request_material_digest')
                IS NOT NEW.request_material_digest
          OR json_extract(NEW.review_json,'$.review.adapter_id') IS NOT NEW.adapter_id
          OR json_extract(NEW.review_json,'$.review.release_version') IS NOT NEW.release_version
          OR json_extract(NEW.review_json,'$.review.decision') IS NOT NEW.decision
          OR json_extract(NEW.review_json,'$.review.review_confirmation')
                IS NOT NEW.review_confirmation
          OR json_extract(NEW.review_json,'$.review.review_note') IS NOT NEW.review_note
          OR json_extract(NEW.review_json,'$.review.reviewed_by_admin_user_id')
                IS NOT NEW.reviewed_by_admin_user_id
          OR json_extract(NEW.review_json,'$.review.reviewed_at') IS NOT NEW.reviewed_at
          OR NOT EXISTS (
                SELECT 1 FROM compute_external_pool_adapter_release_requests request
                 WHERE request.request_id=NEW.request_id
                   AND request.request_digest=NEW.request_digest
                   AND request.request_material_digest=NEW.request_material_digest
                   AND request.adapter_id=NEW.adapter_id
                   AND request.release_version=NEW.release_version
                   AND request.status='submitted'
                   AND request.submitted_by_admin_user_id<>NEW.reviewed_by_admin_user_id
                   AND request.submitted_at<=NEW.reviewed_at)
        BEGIN
            SELECT RAISE(ABORT,
                'external pool Adapter release review lacks exact submitted source');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_release_admission_projection
        BEFORE INSERT ON compute_external_pool_adapter_release_admissions
        WHEN json_extract(NEW.admission_json,'$.schema') IS NOT NEW.admission_schema
          OR json_extract(NEW.admission_json,'$.admission_id') IS NOT NEW.admission_id
          OR json_extract(NEW.admission_json,'$.admission_digest') IS NOT NEW.admission_digest
          OR json_extract(NEW.admission_json,'$.canonicalization') IS NOT NEW.canonicalization
          OR json_extract(NEW.admission_json,'$.digest_algorithm') IS NOT NEW.digest_algorithm
          OR json_extract(NEW.admission_json,'$.admission.request_id') IS NOT NEW.request_id
          OR json_extract(NEW.admission_json,'$.admission.request_digest')
                IS NOT NEW.request_digest
          OR json_extract(NEW.admission_json,'$.admission.request_material_digest')
                IS NOT NEW.request_material_digest
          OR json_extract(NEW.admission_json,'$.admission.review_id') IS NOT NEW.review_id
          OR json_extract(NEW.admission_json,'$.admission.review_digest')
                IS NOT NEW.review_digest
          OR json_extract(NEW.admission_json,'$.admission.adapter_id') IS NOT NEW.adapter_id
          OR json_extract(NEW.admission_json,'$.admission.release_version')
                IS NOT NEW.release_version
          OR json_extract(NEW.admission_json,'$.admission.route_kind') IS NOT NEW.route_kind
          OR json_extract(NEW.admission_json,'$.admission.supported_provider_kinds')
                IS NOT NEW.supported_provider_kinds_json
          OR json_extract(NEW.admission_json,'$.admission.candidate_artifact_ref')
                IS NOT NEW.candidate_artifact_ref
          OR json_extract(NEW.admission_json,'$.admission.declared_implementation_sha256')
                IS NOT NEW.declared_implementation_sha256
          OR json_extract(NEW.admission_json,'$.admission.supported_capabilities')
                IS NOT NEW.capabilities_json
          OR json_extract(NEW.admission_json,'$.admission.capability_set_digest')
                IS NOT NEW.capability_set_digest
          OR json_extract(NEW.admission_json,
                '$.admission.expected_credential_verifier.verification_kind')
                IS NOT NEW.verifier_verification_kind
          OR json_extract(NEW.admission_json,
                '$.admission.expected_credential_verifier.verifier_id') IS NOT NEW.verifier_id
          OR json_extract(NEW.admission_json,
                '$.admission.expected_credential_verifier.verifier_revision')
                IS NOT NEW.verifier_revision
          OR json_extract(NEW.admission_json,
                '$.admission.expected_credential_verifier.verifier_digest')
                IS NOT NEW.verifier_digest
          OR json_extract(NEW.admission_json,'$.admission.submitted_by_admin_user_id')
                IS NOT NEW.submitted_by_admin_user_id
          OR json_extract(NEW.admission_json,'$.admission.reviewed_by_admin_user_id')
                IS NOT NEW.reviewed_by_admin_user_id
          OR json_extract(NEW.admission_json,'$.admission.applied_by_admin_user_id')
                IS NOT NEW.applied_by_admin_user_id
          OR json_extract(NEW.admission_json,'$.admission.apply_confirmation')
                IS NOT NEW.apply_confirmation
          OR json_extract(NEW.admission_json,'$.admission.apply_note') IS NOT NEW.apply_note
          OR json_extract(NEW.admission_json,'$.admission.applied_at') IS NOT NEW.applied_at
          OR json_extract(NEW.admission_json,'$.admission.status') IS NOT NEW.status
        BEGIN
            SELECT RAISE(ABORT,
                'external pool Adapter release admission projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_release_admission_source
        BEFORE INSERT ON compute_external_pool_adapter_release_admissions
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_external_pool_adapter_release_requests request
              JOIN compute_external_pool_adapter_release_reviews review
                ON review.request_id=request.request_id
               AND review.request_digest=request.request_digest
             WHERE request.request_id=NEW.request_id
               AND request.request_digest=NEW.request_digest
               AND request.request_material_digest=NEW.request_material_digest
               AND request.status='approved'
               AND request.adapter_id=NEW.adapter_id
               AND request.release_version=NEW.release_version
               AND request.route_kind=NEW.route_kind
               AND request.supported_provider_kinds_json=NEW.supported_provider_kinds_json
               AND request.candidate_artifact_ref=NEW.candidate_artifact_ref
               AND request.declared_implementation_sha256=NEW.declared_implementation_sha256
               AND request.capabilities_json=NEW.capabilities_json
               AND request.capability_set_digest=NEW.capability_set_digest
               AND request.verifier_verification_kind=NEW.verifier_verification_kind
               AND request.verifier_id=NEW.verifier_id
               AND request.verifier_revision=NEW.verifier_revision
               AND request.verifier_digest=NEW.verifier_digest
               AND request.submitted_by_admin_user_id=NEW.submitted_by_admin_user_id
               AND request.reviewed_by_admin_user_id=NEW.reviewed_by_admin_user_id
               AND review.review_id=NEW.review_id
               AND review.review_digest=NEW.review_digest
               AND review.request_material_digest=NEW.request_material_digest
               AND review.adapter_id=NEW.adapter_id
               AND review.release_version=NEW.release_version
               AND review.decision='approved'
               AND review.reviewed_by_admin_user_id=NEW.reviewed_by_admin_user_id
               AND request.submitted_by_admin_user_id<>review.reviewed_by_admin_user_id
               AND review.reviewed_at=request.reviewed_at
               AND review.reviewed_at<=NEW.applied_at
        ) OR EXISTS (
            SELECT 1 FROM compute_external_pool_adapter_release_admissions admitted
             WHERE admitted.adapter_id=NEW.adapter_id
               AND admitted.release_version=NEW.release_version
        )
        BEGIN
            SELECT RAISE(ABORT,
                'external pool Adapter release admission lacks exact approval');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_release_admission_capabilities
        BEFORE INSERT ON compute_external_pool_adapter_release_admissions
        WHEN json_type(NEW.capabilities_json,'$') IS NOT 'array'
          OR json_array_length(NEW.capabilities_json)<>6
          OR json_extract(NEW.capabilities_json,'$[0].capability_id')
                IS NOT 'authenticated_ack'
          OR json_extract(NEW.capabilities_json,'$[1].capability_id')
                IS NOT 'authenticated_events'
          OR json_extract(NEW.capabilities_json,'$[2].capability_id')
                IS NOT 'cancel_no_start'
          OR json_extract(NEW.capabilities_json,'$[3].capability_id')
                IS NOT 'idempotent_commit'
          OR json_extract(NEW.capabilities_json,'$[4].capability_id') IS NOT 'prepare'
          OR json_extract(NEW.capabilities_json,'$[5].capability_id') IS NOT 'reconcile'
          OR json_type(NEW.capabilities_json,'$[0].capability_revision') IS NOT 'integer'
          OR json_type(NEW.capabilities_json,'$[1].capability_revision') IS NOT 'integer'
          OR json_type(NEW.capabilities_json,'$[2].capability_revision') IS NOT 'integer'
          OR json_type(NEW.capabilities_json,'$[3].capability_revision') IS NOT 'integer'
          OR json_type(NEW.capabilities_json,'$[4].capability_revision') IS NOT 'integer'
          OR json_type(NEW.capabilities_json,'$[5].capability_revision') IS NOT 'integer'
          OR json_extract(NEW.capabilities_json,'$[0].capability_revision') NOT BETWEEN 1 AND 9007199254740991
          OR json_extract(NEW.capabilities_json,'$[1].capability_revision') NOT BETWEEN 1 AND 9007199254740991
          OR json_extract(NEW.capabilities_json,'$[2].capability_revision') NOT BETWEEN 1 AND 9007199254740991
          OR json_extract(NEW.capabilities_json,'$[3].capability_revision') NOT BETWEEN 1 AND 9007199254740991
          OR json_extract(NEW.capabilities_json,'$[4].capability_revision') NOT BETWEEN 1 AND 9007199254740991
          OR json_extract(NEW.capabilities_json,'$[5].capability_revision') NOT BETWEEN 1 AND 9007199254740991
        BEGIN
            SELECT RAISE(ABORT,
                'external pool Adapter release admission capabilities mismatch');
        END;
        "#,
    )?;
    Ok(())
}
