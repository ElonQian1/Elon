use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_compute_route_adapter_version_projection
        BEFORE INSERT ON compute_route_adapter_versions
        WHEN json_extract(NEW.adapter_json,'$.schema') IS NOT NEW.adapter_schema
          OR json_extract(NEW.adapter_json,'$.adapter_id') IS NOT NEW.adapter_id
          OR json_extract(NEW.adapter_json,'$.adapter_revision') IS NOT NEW.adapter_revision
          OR json_extract(NEW.adapter_json,'$.adapter_digest') IS NOT NEW.adapter_digest
          OR json_extract(NEW.adapter_json,'$.canonicalization') IS NOT NEW.canonicalization
          OR json_extract(NEW.adapter_json,'$.digest_algorithm') IS NOT NEW.digest_algorithm
          OR json_extract(NEW.adapter_json,'$.adapter.release_version')
                IS NOT NEW.release_version
          OR json_extract(NEW.adapter_json,'$.adapter.implementation_digest')
                IS NOT NEW.implementation_digest
          OR json_extract(NEW.adapter_json,'$.adapter.route_kind') IS NOT NEW.route_kind
          OR json_extract(NEW.adapter_json,'$.adapter.supported_provider_kinds')
                IS NOT NEW.supported_provider_kinds_json
          OR json_extract(NEW.adapter_json,'$.adapter.credential_verifier.verification_kind')
                IS NOT NEW.credential_verification_kind
          OR json_extract(NEW.adapter_json,'$.adapter.credential_verifier.verifier_id')
                IS NOT NEW.credential_verifier_id
          OR json_extract(NEW.adapter_json,'$.adapter.credential_verifier.verifier_revision')
                IS NOT NEW.credential_verifier_revision
          OR json_extract(NEW.adapter_json,'$.adapter.credential_verifier.verifier_digest')
                IS NOT NEW.credential_verifier_digest
          OR json_extract(NEW.adapter_json,'$.adapter.supported_capabilities')
                IS NOT NEW.supported_capabilities_json
          OR json_extract(NEW.adapter_json,'$.adapter.status') IS NOT NEW.status
          OR json_extract(NEW.adapter_json,'$.adapter.registered_by_service_actor_id')
                IS NOT NEW.registered_by_service_actor_id
          OR json_extract(NEW.adapter_json,'$.adapter.actor_authorization_id')
                IS NOT NEW.actor_authorization_id
          OR json_extract(NEW.adapter_json,'$.adapter.actor_authorization_digest')
                IS NOT NEW.actor_authorization_digest
          OR json_extract(NEW.adapter_json,'$.adapter.registered_at') IS NOT NEW.registered_at
        BEGIN
            SELECT RAISE(ABORT, 'compute route adapter projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_route_adapter_version_source
        BEFORE INSERT ON compute_route_adapter_versions
        WHEN NOT EXISTS (
            SELECT 1 FROM compute_service_actor_authorizations actor
             WHERE actor.actor_authorization_id=NEW.actor_authorization_id
               AND actor.actor_authorization_digest=NEW.actor_authorization_digest
               AND actor.service_actor_id=NEW.registered_by_service_actor_id
               AND actor.issued_at<=NEW.registered_at
               AND NEW.registered_at<actor.valid_until
               AND EXISTS (
                    SELECT 1 FROM json_each(actor.allowed_route_kinds_json) allowed
                     WHERE allowed.type='text' AND allowed.value=NEW.route_kind
               )
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute route adapter lacks exact actor authority');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_route_credential_version_projection
        BEFORE INSERT ON compute_route_credential_versions
        WHEN json_extract(NEW.credential_json,'$.schema') IS NOT NEW.credential_schema
          OR json_extract(NEW.credential_json,'$.credential_id') IS NOT NEW.credential_id
          OR json_extract(NEW.credential_json,'$.credential_revision')
                IS NOT NEW.credential_revision
          OR json_extract(NEW.credential_json,'$.credential_digest')
                IS NOT NEW.credential_digest
          OR json_extract(NEW.credential_json,'$.canonicalization') IS NOT NEW.canonicalization
          OR json_extract(NEW.credential_json,'$.digest_algorithm') IS NOT NEW.digest_algorithm
          OR json_extract(NEW.credential_json,'$.credential.provider.provider_id')
                IS NOT NEW.provider_id
          OR json_extract(NEW.credential_json,'$.credential.provider.provider_kind')
                IS NOT NEW.provider_kind
          OR json_extract(NEW.credential_json,
                '$.credential.provider.provider_owner_account_id')
                IS NOT NEW.provider_owner_account_id
          OR json_extract(NEW.credential_json,'$.credential.route.route_kind')
                IS NOT NEW.route_kind
          OR json_extract(NEW.credential_json,'$.credential.route.route_binding_digest')
                IS NOT NEW.route_binding_digest
          OR json_extract(NEW.credential_json,'$.credential.route.adapter_binding_digest')
                IS NOT NEW.adapter_binding_digest
          OR json_type(NEW.credential_json,'$.credential.route.endpoint_id') IS NULL
          OR json_extract(NEW.credential_json,'$.credential.route.endpoint_id')
                IS NOT NEW.endpoint_id
          OR json_type(NEW.credential_json,'$.credential.route.endpoint_transport') IS NULL
          OR json_extract(NEW.credential_json,'$.credential.route.endpoint_transport')
                IS NOT NEW.endpoint_transport
          OR json_extract(NEW.credential_json,'$.credential.route.adapter.adapter_id')
                IS NOT NEW.adapter_id
          OR json_extract(NEW.credential_json,'$.credential.route.adapter.adapter_revision')
                IS NOT NEW.adapter_revision
          OR json_extract(NEW.credential_json,
                '$.credential.route.adapter.adapter_registry_digest')
                IS NOT NEW.adapter_registry_digest
          OR json_extract(NEW.credential_json,
                '$.credential.route.adapter.adapter_release_version')
                IS NOT NEW.adapter_release_version
          OR json_extract(NEW.credential_json,
                '$.credential.route.adapter.implementation_digest')
                IS NOT NEW.implementation_digest
          OR json_extract(NEW.credential_json,'$.credential.route.adapter.config_revision')
                IS NOT NEW.adapter_config_revision
          OR json_extract(NEW.credential_json,'$.credential.route.adapter.config_digest')
                IS NOT NEW.adapter_config_digest
          OR json_extract(NEW.credential_json,'$.credential.non_bearer_credential_ref')
                IS NOT NEW.non_bearer_credential_ref
          OR json_extract(NEW.credential_json,'$.credential.credential_hint')
                IS NOT NEW.credential_hint
          OR json_extract(NEW.credential_json,'$.credential.verifier.verification_kind')
                IS NOT NEW.verification_kind
          OR json_extract(NEW.credential_json,'$.credential.verifier.verifier_id')
                IS NOT NEW.verifier_id
          OR json_extract(NEW.credential_json,'$.credential.verifier.verifier_revision')
                IS NOT NEW.verifier_revision
          OR json_extract(NEW.credential_json,'$.credential.verifier.verifier_digest')
                IS NOT NEW.verifier_digest
          OR json_extract(NEW.credential_json,'$.credential.verification_receipt_id')
                IS NOT NEW.verification_receipt_id
          OR json_extract(NEW.credential_json,'$.credential.verification_receipt_digest')
                IS NOT NEW.verification_receipt_digest
          OR json_extract(NEW.credential_json,'$.credential.verified_by_service_actor_id')
                IS NOT NEW.verified_by_service_actor_id
          OR json_extract(NEW.credential_json,'$.credential.actor_authorization_id')
                IS NOT NEW.actor_authorization_id
          OR json_extract(NEW.credential_json,'$.credential.actor_authorization_digest')
                IS NOT NEW.actor_authorization_digest
          OR json_extract(NEW.credential_json,'$.credential.authenticated_at')
                IS NOT NEW.authenticated_at
          OR json_extract(NEW.credential_json,'$.credential.expires_at') IS NOT NEW.expires_at
          OR json_extract(NEW.credential_json,'$.credential.cleanup_expires_at')
                IS NOT NEW.cleanup_expires_at
          OR json_extract(NEW.credential_json,'$.credential.recorded_at') IS NOT NEW.recorded_at
        BEGIN
            SELECT RAISE(ABORT, 'compute route credential projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_route_credential_version_source
        BEFORE INSERT ON compute_route_credential_versions
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_providers provider
              JOIN compute_route_adapter_versions adapter
                ON adapter.adapter_id=NEW.adapter_id
               AND adapter.adapter_revision=NEW.adapter_revision
              JOIN compute_service_actor_authorizations actor
                ON actor.actor_authorization_id=NEW.actor_authorization_id
             WHERE provider.provider_id=NEW.provider_id
               AND provider.provider_kind=NEW.provider_kind
               AND provider.owner_account_id=NEW.provider_owner_account_id
               AND adapter.adapter_digest=NEW.adapter_registry_digest
               AND adapter.release_version=NEW.adapter_release_version
               AND adapter.implementation_digest=NEW.implementation_digest
               AND adapter.route_kind=NEW.route_kind
               AND adapter.credential_verification_kind=NEW.verification_kind
               AND adapter.credential_verifier_id=NEW.verifier_id
               AND adapter.credential_verifier_revision=NEW.verifier_revision
               AND adapter.credential_verifier_digest=NEW.verifier_digest
               AND EXISTS (
                    SELECT 1 FROM json_each(adapter.supported_provider_kinds_json) supported
                     WHERE supported.type='text' AND supported.value=NEW.provider_kind
               )
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
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute route credential lacks exact registry source');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_route_credential_revocation_projection
        BEFORE INSERT ON compute_route_credential_revocations
        WHEN json_extract(NEW.revocation_json,'$.schema') IS NOT NEW.revocation_schema
          OR json_extract(NEW.revocation_json,'$.revocation_id') IS NOT NEW.revocation_id
          OR json_extract(NEW.revocation_json,'$.revocation_digest') IS NOT NEW.revocation_digest
          OR json_extract(NEW.revocation_json,'$.canonicalization') IS NOT NEW.canonicalization
          OR json_extract(NEW.revocation_json,'$.digest_algorithm') IS NOT NEW.digest_algorithm
          OR json_extract(NEW.revocation_json,'$.credential_id') IS NOT NEW.credential_id
          OR json_extract(NEW.revocation_json,'$.credential_revision')
                IS NOT NEW.credential_revision
          OR json_extract(NEW.revocation_json,'$.credential_digest') IS NOT NEW.credential_digest
          OR json_extract(NEW.revocation_json,'$.provider_id') IS NOT NEW.provider_id
          OR json_extract(NEW.revocation_json,'$.reason_code') IS NOT NEW.reason_code
          OR json_extract(NEW.revocation_json,'$.revoked_by_service_actor_id')
                IS NOT NEW.revoked_by_service_actor_id
          OR json_extract(NEW.revocation_json,'$.actor_authorization_id')
                IS NOT NEW.actor_authorization_id
          OR json_extract(NEW.revocation_json,'$.actor_authorization_digest')
                IS NOT NEW.actor_authorization_digest
          OR json_extract(NEW.revocation_json,'$.revoked_at') IS NOT NEW.revoked_at
          OR json_extract(NEW.revocation_json,'$.recorded_at') IS NOT NEW.recorded_at
        BEGIN
            SELECT RAISE(ABORT, 'compute route credential revocation projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_route_credential_revocation_source
        BEFORE INSERT ON compute_route_credential_revocations
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_route_credential_versions credential
              JOIN compute_service_actor_authorizations actor
                ON actor.actor_authorization_id=NEW.actor_authorization_id
             WHERE credential.credential_id=NEW.credential_id
               AND credential.credential_revision=NEW.credential_revision
               AND credential.credential_digest=NEW.credential_digest
               AND credential.provider_id=NEW.provider_id
               AND actor.actor_authorization_digest=NEW.actor_authorization_digest
               AND actor.provider_id=NEW.provider_id
               AND actor.service_actor_id=NEW.revoked_by_service_actor_id
               AND actor.issued_at<=NEW.revoked_at AND NEW.recorded_at<actor.valid_until
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute route credential revocation lacks exact source');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_route_authorization_projection
        BEFORE INSERT ON compute_route_authorization_receipts
        WHEN json_extract(NEW.route_authorization_json,'$.schema')
                IS NOT NEW.route_authorization_schema
          OR json_extract(NEW.route_authorization_json,'$.route_authorization_id')
                IS NOT NEW.route_authorization_id
          OR json_extract(NEW.route_authorization_json,'$.route_authorization_revision')
                IS NOT NEW.route_authorization_revision
          OR json_extract(NEW.route_authorization_json,'$.route_authorization_digest')
                IS NOT NEW.route_authorization_digest
          OR json_extract(NEW.route_authorization_json,'$.canonicalization')
                IS NOT NEW.canonicalization
          OR json_extract(NEW.route_authorization_json,'$.digest_algorithm')
                IS NOT NEW.digest_algorithm
          OR json_extract(NEW.route_authorization_json,'$.authorization.provider.provider_id')
                IS NOT NEW.provider_id
          OR json_extract(NEW.route_authorization_json,'$.authorization.provider.provider_kind')
                IS NOT NEW.provider_kind
          OR json_extract(NEW.route_authorization_json,
                '$.authorization.provider.provider_owner_account_id')
                IS NOT NEW.provider_owner_account_id
          OR json_extract(NEW.route_authorization_json,'$.authorization.executor_id')
                IS NOT NEW.executor_id
          OR json_extract(NEW.route_authorization_json,'$.authorization.route.route_kind')
                IS NOT NEW.route_kind
          OR json_extract(NEW.route_authorization_json,
                '$.authorization.route.route_binding_digest') IS NOT NEW.route_binding_digest
          OR json_extract(NEW.route_authorization_json,
                '$.authorization.route.adapter_binding_digest') IS NOT NEW.adapter_binding_digest
          OR json_type(NEW.route_authorization_json,
                '$.authorization.route.endpoint_id') IS NULL
          OR json_extract(NEW.route_authorization_json,'$.authorization.route.endpoint_id')
                IS NOT NEW.endpoint_id
          OR json_type(NEW.route_authorization_json,
                '$.authorization.route.endpoint_transport') IS NULL
          OR json_extract(NEW.route_authorization_json,
                '$.authorization.route.endpoint_transport') IS NOT NEW.endpoint_transport
          OR json_extract(NEW.route_authorization_json,
                '$.authorization.route.adapter.adapter_id') IS NOT NEW.adapter_id
          OR json_extract(NEW.route_authorization_json,
                '$.authorization.route.adapter.adapter_revision') IS NOT NEW.adapter_revision
          OR json_extract(NEW.route_authorization_json,
                '$.authorization.route.adapter.adapter_registry_digest')
                IS NOT NEW.adapter_registry_digest
          OR json_extract(NEW.route_authorization_json,
                '$.authorization.route.adapter.adapter_release_version')
                IS NOT NEW.adapter_release_version
          OR json_extract(NEW.route_authorization_json,
                '$.authorization.route.adapter.implementation_digest')
                IS NOT NEW.implementation_digest
          OR json_extract(NEW.route_authorization_json,
                '$.authorization.route.adapter.config_revision')
                IS NOT NEW.adapter_config_revision
          OR json_extract(NEW.route_authorization_json,
                '$.authorization.route.adapter.config_digest') IS NOT NEW.adapter_config_digest
          OR json_extract(NEW.route_authorization_json,
                '$.authorization.credential.credential_id') IS NOT NEW.credential_id
          OR json_extract(NEW.route_authorization_json,
                '$.authorization.credential.credential_revision') IS NOT NEW.credential_revision
          OR json_extract(NEW.route_authorization_json,
                '$.authorization.credential.credential_digest') IS NOT NEW.credential_digest
          OR json_extract(NEW.route_authorization_json,
                '$.authorization.credential.expires_at') IS NOT NEW.credential_expires_at
          OR json_extract(NEW.route_authorization_json,
                '$.authorization.credential.cleanup_expires_at')
                IS NOT NEW.credential_cleanup_expires_at
          OR json_array_length(json_extract(NEW.route_authorization_json,
                '$.authorization.capabilities')) IS NOT NEW.capability_count
          OR json_extract(NEW.route_authorization_json,'$.authorization.source.source_kind')
                IS NOT NEW.source_kind
          OR json_extract(NEW.route_authorization_json,'$.authorization.source.source_id')
                IS NOT NEW.source_id
          OR json_extract(NEW.route_authorization_json,'$.authorization.source.source_digest')
                IS NOT NEW.source_digest
          OR json_extract(NEW.route_authorization_json,
                '$.authorization.source.approved_by_user_id') IS NOT NEW.approved_by_user_id
          OR json_extract(NEW.route_authorization_json,
                '$.authorization.verifier.verification_kind') IS NOT NEW.verification_kind
          OR json_extract(NEW.route_authorization_json,
                '$.authorization.verifier.verifier_id') IS NOT NEW.verifier_id
          OR json_extract(NEW.route_authorization_json,
                '$.authorization.verifier.verifier_revision') IS NOT NEW.verifier_revision
          OR json_extract(NEW.route_authorization_json,
                '$.authorization.verifier.verifier_digest') IS NOT NEW.verifier_digest
          OR json_extract(NEW.route_authorization_json,'$.authorization.verification_receipt_id')
                IS NOT NEW.verification_receipt_id
          OR json_extract(NEW.route_authorization_json,
                '$.authorization.verification_receipt_digest')
                IS NOT NEW.verification_receipt_digest
          OR json_extract(NEW.route_authorization_json,
                '$.authorization.verified_by_service_actor_id')
                IS NOT NEW.verified_by_service_actor_id
          OR json_extract(NEW.route_authorization_json,'$.authorization.actor_authorization_id')
                IS NOT NEW.actor_authorization_id
          OR json_extract(NEW.route_authorization_json,
                '$.authorization.actor_authorization_digest')
                IS NOT NEW.actor_authorization_digest
          OR json_extract(NEW.route_authorization_json,'$.authorization.authenticated_at')
                IS NOT NEW.authenticated_at
          OR json_extract(NEW.route_authorization_json,'$.authorization.authorized_at')
                IS NOT NEW.authorized_at
          OR json_extract(NEW.route_authorization_json,'$.authorization.expires_at')
                IS NOT NEW.expires_at
          OR json_extract(NEW.route_authorization_json,'$.authorization.cleanup_expires_at')
                IS NOT NEW.cleanup_expires_at
          OR json_extract(NEW.route_authorization_json,'$.authorization.recorded_at')
                IS NOT NEW.recorded_at
        BEGIN
            SELECT RAISE(ABORT, 'compute route authorization projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_route_authorization_exact_source
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
            )) OR (NEW.source_kind='external_pool_onboarding' AND NEW.provider_kind='external_pool'
                AND EXISTS (
                    SELECT 1 FROM compute_activation_applications source
                     WHERE source.application_id=NEW.source_id
                       AND source.application_digest=NEW.source_digest
                       AND source.provider_id=NEW.provider_id
                       AND source.applied_by_user_id=NEW.approved_by_user_id
                ))
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute route authorization lacks exact source');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_route_authorization_seal_projection
        BEFORE INSERT ON compute_route_authorization_seals
        WHEN json_extract(NEW.seal_json,'$.schema') IS NOT NEW.seal_schema
          OR json_extract(NEW.seal_json,'$.seal_id') IS NOT NEW.seal_id
          OR json_extract(NEW.seal_json,'$.seal_digest') IS NOT NEW.seal_digest
          OR json_extract(NEW.seal_json,'$.canonicalization') IS NOT NEW.canonicalization
          OR json_extract(NEW.seal_json,'$.digest_algorithm') IS NOT NEW.digest_algorithm
          OR json_extract(NEW.seal_json,'$.route_authorization_id')
                IS NOT NEW.route_authorization_id
          OR json_extract(NEW.seal_json,'$.route_authorization_revision')
                IS NOT NEW.route_authorization_revision
          OR json_extract(NEW.seal_json,'$.route_authorization_digest')
                IS NOT NEW.route_authorization_digest
          OR json_extract(NEW.seal_json,'$.adapter_id') IS NOT NEW.adapter_id
          OR json_extract(NEW.seal_json,'$.adapter_revision') IS NOT NEW.adapter_revision
          OR json_extract(NEW.seal_json,'$.adapter_registry_digest')
                IS NOT NEW.adapter_registry_digest
          OR json_extract(NEW.seal_json,'$.credential_id') IS NOT NEW.credential_id
          OR json_extract(NEW.seal_json,'$.credential_revision') IS NOT NEW.credential_revision
          OR json_extract(NEW.seal_json,'$.credential_digest') IS NOT NEW.credential_digest
          OR json_extract(NEW.seal_json,'$.capability_count') IS NOT NEW.capability_count
          OR json_extract(NEW.seal_json,'$.capability_set_digest')
                IS NOT NEW.capability_set_digest
          OR json_extract(NEW.seal_json,'$.sealed_at') IS NOT NEW.sealed_at
        BEGIN
            SELECT RAISE(ABORT, 'compute route authorization seal projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_route_authorization_seal_exact
        BEFORE INSERT ON compute_route_authorization_seals
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_route_authorization_receipts route
              JOIN compute_route_adapter_versions adapter
                ON adapter.adapter_id=route.adapter_id
               AND adapter.adapter_revision=route.adapter_revision
              JOIN compute_route_credential_versions credential
                ON credential.credential_id=route.credential_id
               AND credential.credential_revision=route.credential_revision
             WHERE route.route_authorization_id=NEW.route_authorization_id
               AND route.route_authorization_revision=NEW.route_authorization_revision
               AND route.route_authorization_digest=NEW.route_authorization_digest
               AND route.adapter_id=NEW.adapter_id
               AND route.adapter_revision=NEW.adapter_revision
               AND route.adapter_registry_digest=NEW.adapter_registry_digest
               AND route.credential_id=NEW.credential_id
               AND route.credential_revision=NEW.credential_revision
               AND route.credential_digest=NEW.credential_digest
               AND route.capability_count=NEW.capability_count
               AND route.capability_set_digest=NEW.capability_set_digest
               AND adapter.adapter_digest=NEW.adapter_registry_digest
               AND credential.credential_digest=NEW.credential_digest
               AND route.recorded_at<=NEW.sealed_at
               AND (SELECT count(*) FROM compute_route_authorization_capabilities cap
                     WHERE cap.route_authorization_id=NEW.route_authorization_id)=6
               AND NOT EXISTS (
                    SELECT 1 FROM compute_route_authorization_capabilities cap
                     WHERE cap.route_authorization_id=NEW.route_authorization_id
                       AND NOT EXISTS (
                            SELECT 1 FROM json_each(route.route_authorization_json,
                                '$.authorization.capabilities') item
                             WHERE json_extract(item.value,'$.ordinal')=cap.ordinal
                               AND json_extract(item.value,'$.capability_id')=cap.capability_id
                               AND json_extract(item.value,'$.capability_revision')
                                    =cap.capability_revision
                       )
               )
               AND NOT EXISTS (
                    SELECT 1 FROM compute_route_authorization_capabilities cap
                     WHERE cap.route_authorization_id=NEW.route_authorization_id
                       AND NOT EXISTS (
                            SELECT 1 FROM json_each(adapter.supported_capabilities_json) supported
                             WHERE json_extract(supported.value,'$.capability_id')=cap.capability_id
                               AND json_extract(supported.value,'$.capability_revision')
                                    =cap.capability_revision
                       )
               )
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute route authorization seal is incomplete');
        END;
        "#,
    )?;
    Ok(())
}
