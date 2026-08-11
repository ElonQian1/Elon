use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_artifact_source_projection
        BEFORE INSERT ON compute_external_pool_adapter_artifact_source_receipts
        WHEN json_type(NEW.source_receipt_json) IS NOT 'object'
          OR (SELECT COUNT(*) FROM json_each(NEW.source_receipt_json))<>7
          OR EXISTS (
                SELECT 1 FROM json_each(NEW.source_receipt_json)
                 WHERE key NOT IN ('schema','source_receipt_id','source_receipt_digest',
                    'intake_material_digest','canonicalization','digest_algorithm','source'))
          OR json_type(NEW.source_receipt_json,'$.source') IS NOT 'object'
          OR (SELECT COUNT(*) FROM json_each(NEW.source_receipt_json,'$.source'))<>30
          OR EXISTS (
                SELECT 1 FROM json_each(NEW.source_receipt_json,'$.source')
                 WHERE key NOT IN ('admission_id','admission_digest','request_id',
                    'request_digest','request_material_digest','review_id','review_digest',
                    'adapter_id','release_version','candidate_artifact_ref',
                    'declared_implementation_sha256','intake_sha256','reopened_sha256',
                    'artifact_size_bytes','storage_root_kind','storage_namespace',
                    'content_address_algorithm','content_address_digest','custody_state',
                    'intake_kind','evidence_scope','artifact_ref_resolution_effect',
                    'adapter_effect','route_effect','recorded_by_admin_user_id',
                    'intake_confirmation','recorded_at','idempotency_scope',
                    'idempotency_key','created_at'))
          OR json_extract(NEW.source_receipt_json,'$.schema')
                IS NOT NEW.source_receipt_schema
          OR json_extract(NEW.source_receipt_json,'$.source_receipt_id')
                IS NOT NEW.source_receipt_id
          OR json_extract(NEW.source_receipt_json,'$.source_receipt_digest')
                IS NOT NEW.source_receipt_digest
          OR json_extract(NEW.source_receipt_json,'$.intake_material_digest')
                IS NOT NEW.intake_material_digest
          OR json_extract(NEW.source_receipt_json,'$.canonicalization')
                IS NOT NEW.canonicalization
          OR json_extract(NEW.source_receipt_json,'$.digest_algorithm')
                IS NOT NEW.digest_algorithm
          OR json_extract(NEW.source_receipt_json,'$.source.admission_id')
                IS NOT NEW.admission_id
          OR json_extract(NEW.source_receipt_json,'$.source.admission_digest')
                IS NOT NEW.admission_digest
          OR json_extract(NEW.source_receipt_json,'$.source.request_id')
                IS NOT NEW.request_id
          OR json_extract(NEW.source_receipt_json,'$.source.request_digest')
                IS NOT NEW.request_digest
          OR json_extract(NEW.source_receipt_json,
                '$.source.request_material_digest')
                IS NOT NEW.request_material_digest
          OR json_extract(NEW.source_receipt_json,'$.source.review_id')
                IS NOT NEW.review_id
          OR json_extract(NEW.source_receipt_json,'$.source.review_digest')
                IS NOT NEW.review_digest
          OR json_extract(NEW.source_receipt_json,'$.source.adapter_id')
                IS NOT NEW.adapter_id
          OR json_extract(NEW.source_receipt_json,'$.source.release_version')
                IS NOT NEW.release_version
          OR json_extract(NEW.source_receipt_json,
                '$.source.candidate_artifact_ref')
                IS NOT NEW.candidate_artifact_ref
          OR json_extract(NEW.source_receipt_json,
                '$.source.declared_implementation_sha256')
                IS NOT NEW.declared_implementation_sha256
          OR json_extract(NEW.source_receipt_json,'$.source.intake_sha256')
                IS NOT NEW.intake_sha256
          OR json_extract(NEW.source_receipt_json,'$.source.reopened_sha256')
                IS NOT NEW.reopened_sha256
          OR json_extract(NEW.source_receipt_json,'$.source.artifact_size_bytes')
                IS NOT NEW.artifact_size_bytes
          OR json_extract(NEW.source_receipt_json,'$.source.storage_root_kind')
                IS NOT NEW.storage_root_kind
          OR json_extract(NEW.source_receipt_json,'$.source.storage_namespace')
                IS NOT NEW.storage_namespace
          OR json_extract(NEW.source_receipt_json,
                '$.source.content_address_algorithm')
                IS NOT NEW.content_address_algorithm
          OR json_extract(NEW.source_receipt_json,
                '$.source.content_address_digest')
                IS NOT NEW.content_address_digest
          OR json_extract(NEW.source_receipt_json,'$.source.custody_state')
                IS NOT NEW.custody_state
          OR json_extract(NEW.source_receipt_json,'$.source.intake_kind')
                IS NOT NEW.intake_kind
          OR json_extract(NEW.source_receipt_json,'$.source.evidence_scope')
                IS NOT NEW.evidence_scope
          OR json_extract(NEW.source_receipt_json,
                '$.source.artifact_ref_resolution_effect')
                IS NOT NEW.artifact_ref_resolution_effect
          OR json_extract(NEW.source_receipt_json,'$.source.adapter_effect')
                IS NOT NEW.adapter_effect
          OR json_extract(NEW.source_receipt_json,'$.source.route_effect')
                IS NOT NEW.route_effect
          OR json_extract(NEW.source_receipt_json,
                '$.source.recorded_by_admin_user_id')
                IS NOT NEW.recorded_by_admin_user_id
          OR json_extract(NEW.source_receipt_json,'$.source.intake_confirmation')
                IS NOT NEW.intake_confirmation
          OR json_extract(NEW.source_receipt_json,'$.source.recorded_at')
                IS NOT NEW.recorded_at
          OR json_extract(NEW.source_receipt_json,'$.source.idempotency_scope')
                IS NOT NEW.idempotency_scope
          OR json_extract(NEW.source_receipt_json,'$.source.idempotency_key')
                IS NOT NEW.idempotency_key
          OR json_extract(NEW.source_receipt_json,'$.source.created_at')
                IS NOT NEW.created_at
        BEGIN
            SELECT RAISE(ABORT,
                'external pool Adapter artifact source receipt projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_artifact_source_exact_source
        BEFORE INSERT ON compute_external_pool_adapter_artifact_source_receipts
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_external_pool_adapter_release_admissions admission
              JOIN compute_external_pool_adapter_release_requests request
                ON request.request_id=admission.request_id
               AND request.request_digest=admission.request_digest
               AND request.request_material_digest=admission.request_material_digest
              JOIN compute_external_pool_adapter_release_reviews review
                ON review.review_id=admission.review_id
               AND review.review_digest=admission.review_digest
               AND review.request_id=admission.request_id
               AND review.request_digest=admission.request_digest
               AND review.request_material_digest=admission.request_material_digest
             WHERE admission.admission_id=NEW.admission_id
               AND admission.admission_digest=NEW.admission_digest
               AND admission.request_id=NEW.request_id
               AND admission.request_digest=NEW.request_digest
               AND admission.request_material_digest=NEW.request_material_digest
               AND admission.review_id=NEW.review_id
               AND admission.review_digest=NEW.review_digest
               AND admission.adapter_id=NEW.adapter_id
               AND admission.release_version=NEW.release_version
               AND admission.candidate_artifact_ref=NEW.candidate_artifact_ref
               AND admission.declared_implementation_sha256=
                    NEW.declared_implementation_sha256
               AND admission.status='staged'
               AND admission.applied_at<=NEW.recorded_at
               AND request.adapter_id=NEW.adapter_id
               AND request.release_version=NEW.release_version
               AND request.candidate_artifact_ref=NEW.candidate_artifact_ref
               AND request.declared_implementation_sha256=
                    NEW.declared_implementation_sha256
               AND request.status='staged'
               AND request.reviewed_by_admin_user_id=admission.reviewed_by_admin_user_id
               AND request.applied_by_admin_user_id=admission.applied_by_admin_user_id
               AND request.applied_at=admission.applied_at
               AND review.adapter_id=NEW.adapter_id
               AND review.release_version=NEW.release_version
               AND review.decision='approved'
               AND review.reviewed_by_admin_user_id=admission.reviewed_by_admin_user_id
               AND review.reviewed_at=request.reviewed_at
               AND review.reviewed_at<=admission.applied_at
        )
        BEGIN
            SELECT RAISE(ABORT,
                'external pool Adapter artifact source lacks exact staged admission');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_artifact_source_no_update
        BEFORE UPDATE ON compute_external_pool_adapter_artifact_source_receipts
        BEGIN SELECT RAISE(ABORT,
            'external pool Adapter artifact source receipts are append-only'); END;

        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_artifact_source_no_delete
        BEFORE DELETE ON compute_external_pool_adapter_artifact_source_receipts
        BEGIN SELECT RAISE(ABORT,
            'external pool Adapter artifact source receipts are append-only'); END;

        CREATE TRIGGER IF NOT EXISTS trg_external_pool_adapter_artifact_source_no_replace
        BEFORE INSERT ON compute_external_pool_adapter_artifact_source_receipts
        WHEN EXISTS (
            SELECT 1 FROM compute_external_pool_adapter_artifact_source_receipts existing
             WHERE existing.source_receipt_id=NEW.source_receipt_id
                OR existing.source_receipt_digest=NEW.source_receipt_digest
                OR existing.admission_id=NEW.admission_id
                OR (existing.idempotency_scope=NEW.idempotency_scope
                    AND existing.idempotency_key=NEW.idempotency_key)
        )
        BEGIN SELECT RAISE(ABORT,
            'external pool Adapter artifact source receipts cannot be replaced'); END;
        "#,
    )?;
    Ok(())
}
