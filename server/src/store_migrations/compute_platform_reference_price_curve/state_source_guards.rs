use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_batch_state_update
        BEFORE UPDATE ON compute_platform_reference_price_curve_batches
        WHEN NOT (
            OLD.batch_id IS NEW.batch_id AND OLD.batch_schema IS NEW.batch_schema
            AND OLD.batch_digest IS NEW.batch_digest AND OLD.batch_json IS NEW.batch_json
            AND OLD.canonicalization IS NEW.canonicalization
            AND OLD.digest_algorithm IS NEW.digest_algorithm
            AND OLD.batch_material_digest IS NEW.batch_material_digest
            AND OLD.curve_id IS NEW.curve_id AND OLD.curve_version IS NEW.curve_version
            AND OLD.methodology_kind IS NEW.methodology_kind
            AND OLD.valid_from IS NEW.valid_from AND OLD.valid_until IS NEW.valid_until
            AND OLD.quote_ttl_seconds IS NEW.quote_ttl_seconds
            AND OLD.rounding_mode IS NEW.rounding_mode
            AND OLD.entry_count IS NEW.entry_count
            AND OLD.entry_set_digest IS NEW.entry_set_digest
            AND OLD.confirmation IS NEW.confirmation
            AND OLD.submission_note IS NEW.submission_note
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
                        SELECT 1 FROM compute_platform_reference_price_curve_reviews review
                         WHERE review.batch_id=OLD.batch_id
                           AND review.batch_digest=OLD.batch_digest
                           AND review.batch_material_digest=OLD.batch_material_digest
                           AND review.curve_id=OLD.curve_id
                           AND review.curve_version=OLD.curve_version
                           AND review.entry_set_digest=OLD.entry_set_digest
                           AND review.decision=NEW.status
                           AND review.reviewed_by_admin_user_id=NEW.reviewed_by_admin_user_id
                           AND review.reviewed_at=NEW.reviewed_at))
                OR (OLD.status='approved' AND NEW.status='applied'
                    AND NEW.reviewed_by_admin_user_id IS OLD.reviewed_by_admin_user_id
                    AND NEW.reviewed_at IS OLD.reviewed_at
                    AND OLD.applied_by_admin_user_id IS NULL AND OLD.applied_at IS NULL
                    AND NEW.applied_by_admin_user_id IS NOT NULL
                    AND OLD.reviewed_at<=NEW.applied_at
                    AND NEW.updated_at=NEW.applied_at
                    AND EXISTS (
                        SELECT 1 FROM compute_platform_reference_price_curve_applications app
                         WHERE app.batch_id=OLD.batch_id
                           AND app.batch_digest=OLD.batch_digest
                           AND app.batch_material_digest=OLD.batch_material_digest
                           AND app.curve_id=OLD.curve_id
                           AND app.curve_version=OLD.curve_version
                           AND app.reviewed_by_admin_user_id=OLD.reviewed_by_admin_user_id
                           AND app.applied_by_admin_user_id=NEW.applied_by_admin_user_id
                           AND app.applied_at=NEW.applied_at
                           AND app.status='applied'))
            )
        )
        BEGIN
            SELECT RAISE(ABORT, 'platform reference price curve batch transition rejected');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_review_source
        BEFORE INSERT ON compute_platform_reference_price_curve_reviews
        WHEN NOT EXISTS (
            SELECT 1 FROM compute_platform_reference_price_curve_batches batch
             WHERE batch.batch_id=NEW.batch_id
               AND batch.batch_digest=NEW.batch_digest
               AND batch.batch_material_digest=NEW.batch_material_digest
               AND batch.curve_id=NEW.curve_id
               AND batch.curve_version=NEW.curve_version
               AND batch.entry_set_digest=NEW.entry_set_digest
               AND batch.status='submitted'
               AND batch.submitted_by_admin_user_id<>NEW.reviewed_by_admin_user_id
               AND batch.submitted_at<=NEW.reviewed_at
               AND (SELECT COUNT(*)
                      FROM compute_platform_reference_price_curve_entries entry
                     WHERE entry.batch_id=batch.batch_id
                       AND entry.batch_digest=batch.batch_digest)=batch.entry_count
        )
        BEGIN
            SELECT RAISE(ABORT, 'platform reference price curve review lacks exact batch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_review_advance_batch
        AFTER INSERT ON compute_platform_reference_price_curve_reviews
        BEGIN
            UPDATE compute_platform_reference_price_curve_batches
               SET status=NEW.decision,
                   reviewed_by_admin_user_id=NEW.reviewed_by_admin_user_id,
                   reviewed_at=NEW.reviewed_at,
                   updated_at=NEW.reviewed_at
             WHERE batch_id=NEW.batch_id AND batch_digest=NEW.batch_digest
               AND batch_material_digest=NEW.batch_material_digest
               AND status='submitted';
            SELECT CASE WHEN changes()<>1 THEN RAISE(ABORT,
                'platform reference price curve review did not advance one batch') END;
        END;

        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_binding_source
        BEFORE INSERT ON compute_platform_reference_price_curve_snapshot_bindings
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_platform_reference_price_curve_batches batch
              JOIN compute_platform_reference_price_curve_reviews review
                ON review.batch_id=batch.batch_id AND review.batch_digest=batch.batch_digest
              JOIN compute_platform_reference_price_curve_entries entry
                ON entry.batch_id=batch.batch_id AND entry.batch_digest=batch.batch_digest
             WHERE batch.batch_id=NEW.batch_id AND batch.batch_digest=NEW.batch_digest
               AND batch.status='approved'
               AND review.review_id=NEW.review_id AND review.review_digest=NEW.review_digest
               AND review.decision='approved'
               AND entry.entry_id=NEW.entry_id AND entry.entry_digest=NEW.entry_digest
               AND entry.ordinal=NEW.ordinal AND entry.entry_key=NEW.entry_key
               AND batch.curve_id=NEW.curve_id AND batch.curve_version=NEW.curve_version
               AND NEW.source_kind='fallback_curve'
               AND NEW.source_id='platform_reference_curve:' || batch.curve_id
               AND NEW.source_version=batch.curve_version
               AND NEW.source_digest=entry.entry_digest
               AND review.reviewed_at<=NEW.quoted_at
               AND batch.valid_from<=NEW.quoted_at AND NEW.quoted_at<batch.valid_until
               AND NEW.quoted_at<NEW.expires_at AND NEW.expires_at<=batch.valid_until
               AND julianday(NEW.expires_at)-julianday(NEW.quoted_at)
                    <=batch.quote_ttl_seconds/86400.0
        )
        BEGIN
            SELECT RAISE(ABORT, 'platform reference price curve binding lacks exact approval');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_application_source
        BEFORE INSERT ON compute_platform_reference_price_curve_applications
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_platform_reference_price_curve_batches batch
              JOIN compute_platform_reference_price_curve_reviews review
                ON review.batch_id=batch.batch_id AND review.batch_digest=batch.batch_digest
             WHERE batch.batch_id=NEW.batch_id AND batch.batch_digest=NEW.batch_digest
               AND batch.batch_material_digest=NEW.batch_material_digest
               AND batch.status='approved'
               AND batch.curve_id=NEW.curve_id AND batch.curve_version=NEW.curve_version
               AND batch.submitted_by_admin_user_id=NEW.submitted_by_admin_user_id
               AND review.review_id=NEW.review_id AND review.review_digest=NEW.review_digest
               AND review.decision='approved'
               AND review.reviewed_by_admin_user_id=NEW.reviewed_by_admin_user_id
               AND batch.submitted_by_admin_user_id<>review.reviewed_by_admin_user_id
               AND review.reviewed_at<=NEW.applied_at
               AND NEW.binding_count=batch.entry_count
               AND (SELECT COUNT(*)
                      FROM compute_platform_reference_price_curve_snapshot_bindings binding
                     WHERE binding.application_id=NEW.application_id)=NEW.binding_count
               AND NOT EXISTS (
                    SELECT 1 FROM json_each(NEW.binding_digests_json) expected
                     WHERE NOT EXISTS (
                        SELECT 1
                          FROM compute_platform_reference_price_curve_snapshot_bindings binding
                         JOIN compute_price_snapshots snapshot
                            ON snapshot.snapshot_id=binding.snapshot_id
                           AND snapshot.snapshot_digest=binding.snapshot_digest
                         WHERE binding.application_id=NEW.application_id
                           AND binding.batch_id=NEW.batch_id
                           AND binding.batch_digest=NEW.batch_digest
                           AND binding.review_id=NEW.review_id
                           AND binding.review_digest=NEW.review_digest
                           AND binding.curve_id=NEW.curve_id
                           AND binding.curve_version=NEW.curve_version
                           AND binding.quoted_at=NEW.applied_at
                           AND binding.status='snapshot_registered'
                           AND binding.ordinal=CAST(expected.key AS INTEGER)+1
                           AND binding.binding_digest=expected.value))
        )
        BEGIN
            SELECT RAISE(ABORT, 'platform reference price curve application is incomplete');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_application_advance_batch
        AFTER INSERT ON compute_platform_reference_price_curve_applications
        BEGIN
            UPDATE compute_platform_reference_price_curve_batches
               SET status='applied', applied_by_admin_user_id=NEW.applied_by_admin_user_id,
                   applied_at=NEW.applied_at, updated_at=NEW.applied_at
             WHERE batch_id=NEW.batch_id AND batch_digest=NEW.batch_digest
               AND batch_material_digest=NEW.batch_material_digest
               AND status='approved'
               AND reviewed_by_admin_user_id=NEW.reviewed_by_admin_user_id;
            SELECT CASE WHEN changes()<>1 THEN RAISE(ABORT,
                'platform reference price curve application did not consume one approval') END;
        END;

        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_batches_no_replace
        BEFORE INSERT ON compute_platform_reference_price_curve_batches
        WHEN EXISTS (
            SELECT 1 FROM compute_platform_reference_price_curve_batches existing
             WHERE existing.batch_id=NEW.batch_id OR existing.batch_digest=NEW.batch_digest
                OR (existing.curve_id=NEW.curve_id
                    AND existing.curve_version=NEW.curve_version)
                OR (existing.idempotency_scope=NEW.idempotency_scope
                    AND existing.idempotency_key=NEW.idempotency_key))
        BEGIN SELECT RAISE(ABORT, 'platform reference price curve batch cannot replace history'); END;
        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_entries_no_replace
        BEFORE INSERT ON compute_platform_reference_price_curve_entries
        WHEN EXISTS (
            SELECT 1 FROM compute_platform_reference_price_curve_entries existing
             WHERE existing.entry_id=NEW.entry_id OR existing.entry_digest=NEW.entry_digest
                OR (existing.batch_id=NEW.batch_id AND existing.ordinal=NEW.ordinal)
                OR (existing.batch_id=NEW.batch_id AND existing.entry_key=NEW.entry_key)
                OR (existing.batch_id=NEW.batch_id AND existing.offer_id=NEW.offer_id
                    AND existing.offer_version=NEW.offer_version
                    AND existing.delivery_window_id=NEW.delivery_window_id))
        BEGIN SELECT RAISE(ABORT, 'platform reference price curve entry cannot replace history'); END;
        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_reviews_no_replace
        BEFORE INSERT ON compute_platform_reference_price_curve_reviews
        WHEN EXISTS (
            SELECT 1 FROM compute_platform_reference_price_curve_reviews existing
             WHERE existing.review_id=NEW.review_id OR existing.review_digest=NEW.review_digest
                OR existing.batch_id=NEW.batch_id
                OR (existing.idempotency_scope=NEW.idempotency_scope
                    AND existing.idempotency_key=NEW.idempotency_key))
        BEGIN SELECT RAISE(ABORT, 'platform reference price curve review cannot replace history'); END;
        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_applications_no_replace
        BEFORE INSERT ON compute_platform_reference_price_curve_applications
        WHEN EXISTS (
            SELECT 1 FROM compute_platform_reference_price_curve_applications existing
             WHERE existing.application_id=NEW.application_id
                OR existing.application_digest=NEW.application_digest
                OR existing.batch_id=NEW.batch_id OR existing.review_id=NEW.review_id
                OR (existing.curve_id=NEW.curve_id
                    AND existing.curve_version=NEW.curve_version)
                OR (existing.idempotency_scope=NEW.idempotency_scope
                    AND existing.idempotency_key=NEW.idempotency_key))
        BEGIN SELECT RAISE(ABORT, 'platform reference curve application cannot replace history'); END;
        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_bindings_no_replace
        BEFORE INSERT ON compute_platform_reference_price_curve_snapshot_bindings
        WHEN EXISTS (
            SELECT 1 FROM compute_platform_reference_price_curve_snapshot_bindings existing
             WHERE existing.binding_id=NEW.binding_id OR existing.binding_digest=NEW.binding_digest
                OR existing.snapshot_id=NEW.snapshot_id OR existing.snapshot_digest=NEW.snapshot_digest
                OR existing.quote_id=NEW.quote_id
                OR (existing.application_id=NEW.application_id
                    AND existing.ordinal=NEW.ordinal)
                OR (existing.entry_id=NEW.entry_id AND existing.entry_digest=NEW.entry_digest))
        BEGIN SELECT RAISE(ABORT, 'platform reference curve binding cannot replace history'); END;

        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_batches_no_delete
        BEFORE DELETE ON compute_platform_reference_price_curve_batches
        BEGIN SELECT RAISE(ABORT, 'platform reference price curve batches are durable'); END;
        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_entries_no_update
        BEFORE UPDATE ON compute_platform_reference_price_curve_entries
        BEGIN SELECT RAISE(ABORT, 'platform reference price curve entries are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_entries_no_delete
        BEFORE DELETE ON compute_platform_reference_price_curve_entries
        BEGIN SELECT RAISE(ABORT, 'platform reference price curve entries are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_reviews_no_update
        BEFORE UPDATE ON compute_platform_reference_price_curve_reviews
        BEGIN SELECT RAISE(ABORT, 'platform reference price curve reviews are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_reviews_no_delete
        BEFORE DELETE ON compute_platform_reference_price_curve_reviews
        BEGIN SELECT RAISE(ABORT, 'platform reference price curve reviews are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_applications_no_update
        BEFORE UPDATE ON compute_platform_reference_price_curve_applications
        BEGIN SELECT RAISE(ABORT, 'platform reference price curve applications are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_applications_no_delete
        BEFORE DELETE ON compute_platform_reference_price_curve_applications
        BEGIN SELECT RAISE(ABORT, 'platform reference price curve applications are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_bindings_no_update
        BEFORE UPDATE ON compute_platform_reference_price_curve_snapshot_bindings
        BEGIN SELECT RAISE(ABORT, 'platform reference price curve bindings are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS trg_platform_reference_curve_bindings_no_delete
        BEFORE DELETE ON compute_platform_reference_price_curve_snapshot_bindings
        BEGIN SELECT RAISE(ABORT, 'platform reference price curve bindings are immutable'); END;
        "#,
    )?;
    Ok(())
}
