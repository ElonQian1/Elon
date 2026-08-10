use anyhow::{bail, Result};
use rusqlite::Connection;

pub(super) fn replace(conn: &Connection) -> Result<()> {
    let has_legacy_external_source = conn.query_row(
        "SELECT EXISTS(
             SELECT 1 FROM compute_route_authorization_receipts
              WHERE source_kind='external_pool_onboarding'
         )",
        [],
        |row| row.get::<_, bool>(0),
    )?;
    if has_legacy_external_source {
        bail!("EXTERNAL_POOL_ONBOARDING_ROUTE_SOURCE_BACKFILL_REQUIRED");
    }
    conn.execute_batch(
        r#"
        DROP TRIGGER IF EXISTS trg_compute_route_authorization_exact_source;
        CREATE TRIGGER trg_compute_route_authorization_exact_source
        BEFORE INSERT ON compute_route_authorization_receipts
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_route_credential_versions credential
              JOIN compute_route_adapter_versions adapter
                ON adapter.adapter_id=NEW.adapter_id
               AND adapter.adapter_revision=NEW.adapter_revision
              JOIN compute_service_actor_authorizations actor
                ON actor.actor_authorization_id=NEW.actor_authorization_id
             WHERE credential.credential_id=NEW.credential_id
               AND credential.credential_revision=NEW.credential_revision
               AND credential.credential_digest=NEW.credential_digest
               AND credential.provider_id=NEW.provider_id
               AND credential.provider_kind=NEW.provider_kind
               AND credential.provider_owner_account_id=NEW.provider_owner_account_id
               AND credential.route_kind=NEW.route_kind
               AND credential.route_binding_digest=NEW.route_binding_digest
               AND credential.adapter_binding_digest=NEW.adapter_binding_digest
               AND credential.endpoint_id IS NEW.endpoint_id
               AND credential.endpoint_transport IS NEW.endpoint_transport
               AND credential.adapter_registry_digest=NEW.adapter_registry_digest
               AND credential.adapter_release_version=NEW.adapter_release_version
               AND credential.implementation_digest=NEW.implementation_digest
               AND credential.adapter_config_revision=NEW.adapter_config_revision
               AND credential.adapter_config_digest=NEW.adapter_config_digest
               AND credential.expires_at=NEW.credential_expires_at
               AND credential.cleanup_expires_at=NEW.credential_cleanup_expires_at
               AND credential.verification_kind=NEW.verification_kind
               AND credential.verifier_id=NEW.verifier_id
               AND credential.verifier_revision=NEW.verifier_revision
               AND credential.verifier_digest=NEW.verifier_digest
               AND credential.verification_receipt_id=NEW.verification_receipt_id
               AND credential.verification_receipt_digest=NEW.verification_receipt_digest
               AND adapter.adapter_digest=NEW.adapter_registry_digest
               AND actor.actor_authorization_digest=NEW.actor_authorization_digest
               AND actor.provider_id=NEW.provider_id
               AND actor.provider_owner_account_id=NEW.provider_owner_account_id
               AND actor.service_actor_id=NEW.verified_by_service_actor_id
               AND actor.issued_at<=NEW.authenticated_at
               AND NEW.recorded_at<actor.valid_until
               AND EXISTS (
                    SELECT 1 FROM json_each(actor.allowed_route_kinds_json) allowed
                     WHERE allowed.type='text' AND allowed.value=NEW.route_kind
               )
               AND NOT EXISTS (
                    SELECT 1 FROM compute_route_credential_revocations revoked
                     WHERE revoked.credential_id=NEW.credential_id
                       AND revoked.credential_revision=NEW.credential_revision
                       AND revoked.revoked_at<=NEW.authorized_at
               )
        ) OR NOT (
            (NEW.source_kind='provider_activation_application' AND EXISTS (
                SELECT 1 FROM compute_activation_applications source
                 WHERE source.application_id=NEW.source_id
                   AND source.application_digest=NEW.source_digest
                   AND source.provider_id=NEW.provider_id
                   AND source.applied_by_user_id=NEW.approved_by_user_id
            )) OR (NEW.source_kind='provider_recovery_application' AND EXISTS (
                SELECT 1 FROM compute_activation_recovery_applications source
                 WHERE source.recovery_application_id=NEW.source_id
                   AND source.application_digest=NEW.source_digest
                   AND source.provider_id=NEW.provider_id
                   AND source.applied_by_user_id=NEW.approved_by_user_id
            )) OR (NEW.source_kind='external_pool_onboarding'
                AND NEW.provider_kind='external_pool'
                AND EXISTS (
                    SELECT 1
                      FROM compute_external_pool_onboarding_applications source
                      JOIN compute_external_pool_onboarding_reviews review
                        ON review.review_id=source.review_id
                       AND review.request_id=source.request_id
                      JOIN compute_external_pool_onboarding_requests request
                        ON request.request_id=source.request_id
                       AND request.request_digest=source.request_digest
                     WHERE source.application_id=NEW.source_id
                       AND source.application_digest=NEW.source_digest
                       AND source.provider_id=NEW.provider_id
                       AND source.provider_kind=NEW.provider_kind
                       AND source.provider_owner_account_id=NEW.provider_owner_account_id
                       AND source.approved_by_user_id=NEW.approved_by_user_id
                       AND source.approved_by_user_id=source.provider_owner_account_id
                       AND source.reviewed_by_user_id=review.reviewed_by_user_id
                       AND source.review_digest=review.review_digest
                       AND review.decision='approved'
                       AND request.status='applied'
                       AND source.reviewed_by_user_id<>source.provider_owner_account_id
                       AND source.adapter_id=NEW.adapter_id
                       AND source.adapter_release_version=NEW.adapter_release_version
                       AND source.adapter_config_revision=NEW.adapter_config_revision
                       AND source.adapter_config_digest=NEW.adapter_config_digest
                ))
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute route authorization lacks exact source');
        END;
        "#,
    )?;
    Ok(())
}
