use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_review_projection
        BEFORE INSERT ON compute_platform_reference_price_curve_reviews
        WHEN json_extract(NEW.review_json,'$.schema') IS NOT NEW.review_schema
          OR json_extract(NEW.review_json,'$.review_id') IS NOT NEW.review_id
          OR json_extract(NEW.review_json,'$.review_digest') IS NOT NEW.review_digest
          OR json_extract(NEW.review_json,'$.canonicalization') IS NOT NEW.canonicalization
          OR json_extract(NEW.review_json,'$.digest_algorithm') IS NOT NEW.digest_algorithm
          OR json_extract(NEW.review_json,'$.review.batch_id') IS NOT NEW.batch_id
          OR json_extract(NEW.review_json,'$.review.batch_digest') IS NOT NEW.batch_digest
          OR json_extract(NEW.review_json,'$.review.batch_material_digest')
                IS NOT NEW.batch_material_digest
          OR json_extract(NEW.review_json,'$.review.curve_id') IS NOT NEW.curve_id
          OR json_extract(NEW.review_json,'$.review.curve_version') IS NOT NEW.curve_version
          OR json_extract(NEW.review_json,'$.review.entry_set_digest')
                IS NOT NEW.entry_set_digest
          OR json_extract(NEW.review_json,'$.review.decision') IS NOT NEW.decision
          OR json_extract(NEW.review_json,'$.review.review_confirmation')
                IS NOT NEW.review_confirmation
          OR json_extract(NEW.review_json,'$.review.review_note') IS NOT NEW.review_note
          OR json_extract(NEW.review_json,'$.review.reviewed_by_admin_user_id')
                IS NOT NEW.reviewed_by_admin_user_id
          OR json_extract(NEW.review_json,'$.review.reviewed_at') IS NOT NEW.reviewed_at
          OR NEW.created_at<>NEW.reviewed_at
        BEGIN
            SELECT RAISE(ABORT, 'platform reference price curve review projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_application_projection
        BEFORE INSERT ON compute_platform_reference_price_curve_applications
        WHEN json_extract(NEW.application_json,'$.schema') IS NOT NEW.application_schema
          OR json_extract(NEW.application_json,'$.application_id') IS NOT NEW.application_id
          OR json_extract(NEW.application_json,'$.application_digest')
                IS NOT NEW.application_digest
          OR json_extract(NEW.application_json,'$.canonicalization')
                IS NOT NEW.canonicalization
          OR json_extract(NEW.application_json,'$.digest_algorithm')
                IS NOT NEW.digest_algorithm
          OR json_extract(NEW.application_json,'$.application.batch_id') IS NOT NEW.batch_id
          OR json_extract(NEW.application_json,'$.application.batch_digest')
                IS NOT NEW.batch_digest
          OR json_extract(NEW.application_json,'$.application.batch_material_digest')
                IS NOT NEW.batch_material_digest
          OR json_extract(NEW.application_json,'$.application.review_id') IS NOT NEW.review_id
          OR json_extract(NEW.application_json,'$.application.review_digest')
                IS NOT NEW.review_digest
          OR json_extract(NEW.application_json,'$.application.curve_id') IS NOT NEW.curve_id
          OR json_extract(NEW.application_json,'$.application.curve_version')
                IS NOT NEW.curve_version
          OR json_extract(NEW.application_json,'$.application.binding_digests')
                IS NOT NEW.binding_digests_json
          OR json_array_length(json_extract(
                NEW.application_json,'$.application.binding_digests')) IS NOT NEW.binding_count
          OR json_extract(NEW.application_json,'$.application.binding_set_digest')
                IS NOT NEW.binding_set_digest
          OR json_extract(NEW.application_json,'$.application.submitted_by_admin_user_id')
                IS NOT NEW.submitted_by_admin_user_id
          OR json_extract(NEW.application_json,'$.application.reviewed_by_admin_user_id')
                IS NOT NEW.reviewed_by_admin_user_id
          OR json_extract(NEW.application_json,'$.application.applied_by_admin_user_id')
                IS NOT NEW.applied_by_admin_user_id
          OR json_extract(NEW.application_json,'$.application.apply_confirmation')
                IS NOT NEW.apply_confirmation
          OR json_extract(NEW.application_json,'$.application.apply_note') IS NOT NEW.apply_note
          OR json_extract(NEW.application_json,'$.application.applied_at') IS NOT NEW.applied_at
          OR json_extract(NEW.application_json,'$.application.status') IS NOT NEW.status
          OR NEW.created_at<>NEW.applied_at
        BEGIN
            SELECT RAISE(ABORT, 'platform reference price curve application projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_binding_projection
        BEFORE INSERT ON compute_platform_reference_price_curve_snapshot_bindings
        WHEN json_extract(NEW.binding_json,'$.schema') IS NOT NEW.binding_schema
          OR json_extract(NEW.binding_json,'$.binding_id') IS NOT NEW.binding_id
          OR json_extract(NEW.binding_json,'$.binding_digest') IS NOT NEW.binding_digest
          OR json_extract(NEW.binding_json,'$.canonicalization') IS NOT NEW.canonicalization
          OR json_extract(NEW.binding_json,'$.digest_algorithm') IS NOT NEW.digest_algorithm
          OR json_extract(NEW.binding_json,'$.binding.application_id') IS NOT NEW.application_id
          OR json_extract(NEW.binding_json,'$.binding.batch_id') IS NOT NEW.batch_id
          OR json_extract(NEW.binding_json,'$.binding.batch_digest') IS NOT NEW.batch_digest
          OR json_extract(NEW.binding_json,'$.binding.review_id') IS NOT NEW.review_id
          OR json_extract(NEW.binding_json,'$.binding.review_digest') IS NOT NEW.review_digest
          OR json_extract(NEW.binding_json,'$.binding.entry_id') IS NOT NEW.entry_id
          OR json_extract(NEW.binding_json,'$.binding.entry_digest') IS NOT NEW.entry_digest
          OR json_extract(NEW.binding_json,'$.binding.ordinal') IS NOT NEW.ordinal
          OR json_extract(NEW.binding_json,'$.binding.entry_key') IS NOT NEW.entry_key
          OR json_extract(NEW.binding_json,'$.binding.curve_id') IS NOT NEW.curve_id
          OR json_extract(NEW.binding_json,'$.binding.curve_version') IS NOT NEW.curve_version
          OR json_extract(NEW.binding_json,'$.binding.snapshot_id') IS NOT NEW.snapshot_id
          OR json_extract(NEW.binding_json,'$.binding.snapshot_digest') IS NOT NEW.snapshot_digest
          OR json_extract(NEW.binding_json,'$.binding.quote_id') IS NOT NEW.quote_id
          OR json_extract(NEW.binding_json,'$.binding.source_kind') IS NOT NEW.source_kind
          OR json_extract(NEW.binding_json,'$.binding.source_id') IS NOT NEW.source_id
          OR json_extract(NEW.binding_json,'$.binding.source_version') IS NOT NEW.source_version
          OR json_extract(NEW.binding_json,'$.binding.source_digest') IS NOT NEW.source_digest
          OR json_extract(NEW.binding_json,'$.binding.quoted_at') IS NOT NEW.quoted_at
          OR json_extract(NEW.binding_json,'$.binding.expires_at') IS NOT NEW.expires_at
          OR json_extract(NEW.binding_json,'$.binding.status') IS NOT NEW.status
          OR NEW.created_at<>NEW.quoted_at
        BEGIN
            SELECT RAISE(ABORT, 'platform reference price curve binding projection mismatch');
        END;
        "#,
    )?;
    Ok(())
}
