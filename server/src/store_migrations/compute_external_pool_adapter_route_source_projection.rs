use anyhow::{bail, Result};
use rusqlite::{Connection, Transaction, TransactionBehavior};

pub(crate) fn migration_v271(conn: &Connection) -> Result<()> {
    let transaction = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    reject_existing_external_pool_routes(&transaction)?;
    require_v254_fences(&transaction)?;
    replace_exact_source_trigger(&transaction)?;
    transaction.commit()?;
    Ok(())
}

fn reject_existing_external_pool_routes(conn: &Connection) -> Result<()> {
    let exists = conn.query_row(
        "SELECT EXISTS(
             SELECT 1 FROM compute_route_authorization_receipts
              WHERE source_kind='external_pool_onboarding'
                OR provider_kind='external_pool'
         )",
        [],
        |row| row.get::<_, bool>(0),
    )?;
    if exists {
        bail!("V271 refuses existing external_pool route authorization rows");
    }
    Ok(())
}

fn require_v254_fences(conn: &Connection) -> Result<()> {
    let count = conn.query_row(
        r#"SELECT COUNT(*) FROM sqlite_master
            WHERE type='trigger' AND name IN (
              'v254_external_pool_provider_activation_fence',
              'v254_external_pool_provider_insert_active_fence',
              'v254_external_pool_provider_identity_update_fence',
              'v254_external_pool_provider_kind_update_fence',
              'v254_external_pool_provider_version_active_fence',
              'v254_external_pool_candidate_projection_adapter_fence',
              'v254_external_pool_candidate_projection_adapter_version_fence',
              'v254_external_pool_candidate_service_actor_fence',
              'v254_external_pool_route_credential_fence',
              'v254_external_pool_route_authorization_fence',
              'v254_external_pool_route_capability_fence',
              'v254_external_pool_route_seal_fence',
              'v254_external_pool_capacity_pool_insert_active_fence',
              'v254_external_pool_capacity_pool_update_active_fence',
              'v254_external_pool_capacity_pool_version_active_fence',
              'v254_external_pool_offer_insert_market_fence',
              'v254_external_pool_offer_update_market_fence',
              'v254_external_pool_offer_version_market_fence'
            )"#,
        [],
        |row| row.get::<_, i64>(0),
    )?;
    if count != 18 {
        bail!("V271 requires all 18 V254 deny fences");
    }
    Ok(())
}

fn replace_exact_source_trigger(conn: &Connection) -> Result<()> {
    // This migration maps immutable V221/V249/V254 scalar and JSON projections only. It does not
    // recompute V254's domain-separated compatibility digests or grant activation authority.
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
                AND NEW.route_kind='server_adapter'
                AND EXISTS (
                    SELECT 1
                      FROM compute_external_pool_onboarding_applications source
                      JOIN compute_external_pool_onboarding_reviews review
                        ON review.review_id=source.review_id
                       AND review.request_id=source.request_id
                      JOIN compute_external_pool_onboarding_requests request
                        ON request.request_id=source.request_id
                       AND request.request_digest=source.request_digest
                      JOIN compute_external_pool_adapter_registry_provider_bindings binding
                        ON binding.application_id=source.application_id
                       AND binding.application_digest=source.application_digest
                       AND binding.provider_id=source.provider_id
                       AND binding.provider_owner_account_id=source.provider_owner_account_id
                       AND binding.provider_policy_revision=source.target_provider_policy_revision
                       AND binding.provider_digest=source.target_provider_digest
                       AND binding.adapter_id=source.adapter_id
                       AND binding.release_version=source.adapter_release_version
                       AND binding.adapter_config_revision=source.adapter_config_revision
                       AND binding.adapter_config_digest=source.adapter_config_digest
                      JOIN compute_external_pool_adapter_registry_release_current current_release
                        ON current_release.registry_release_id=binding.registry_release_id
                       AND current_release.registry_release_digest=binding.registry_release_digest
                       AND current_release.current_status='release_current'
                      JOIN compute_external_pool_adapter_registry_releases release
                        ON release.registry_release_id=current_release.registry_release_id
                       AND release.registry_release_digest=current_release.registry_release_digest
                      JOIN compute_route_adapter_versions projected_adapter
                        ON projected_adapter.adapter_id=NEW.adapter_id
                       AND projected_adapter.adapter_revision=NEW.adapter_revision
                       AND projected_adapter.adapter_digest=NEW.adapter_registry_digest
                      JOIN compute_external_pool_provider_activation_candidates candidate
                        ON candidate.provider_binding_id=binding.provider_binding_id
                       AND candidate.provider_binding_digest=binding.provider_binding_digest
                       AND candidate.registry_release_id=binding.registry_release_id
                       AND candidate.registry_release_digest=binding.registry_release_digest
                       AND candidate.installation_receipt_id=binding.installation_receipt_id
                       AND candidate.installation_receipt_digest=binding.installation_receipt_digest
                       AND candidate.installation_content_digest=binding.installation_content_digest
                       AND candidate.route_adapter_projection_id=binding.route_adapter_projection_id
                       AND candidate.provider_id=binding.provider_id
                       AND candidate.provider_owner_account_id=binding.provider_owner_account_id
                       AND candidate.provider_policy_revision=binding.provider_policy_revision
                       AND candidate.provider_digest=binding.provider_digest
                       AND candidate.logical_adapter_id=binding.adapter_id
                       AND candidate.release_version=binding.release_version
                       AND candidate.adapter_config_revision=binding.adapter_config_revision
                       AND candidate.adapter_config_digest=binding.adapter_config_digest
                      JOIN compute_external_pool_provider_activation_delegations delegation
                        ON delegation.delegation_id=candidate.delegation_id
                       AND delegation.delegation_digest=candidate.delegation_digest
                       AND delegation.provider_binding_id=candidate.provider_binding_id
                       AND delegation.provider_binding_digest=candidate.provider_binding_digest
                       AND delegation.sequence=candidate.sequence
                       AND delegation.service_actor_id=candidate.service_actor_id
                      JOIN compute_providers provider
                        ON provider.provider_id=candidate.provider_id
                       AND provider.provider_kind='external_pool'
                       AND provider.owner_account_id=candidate.provider_owner_account_id
                       AND provider.status='registering'
                       AND provider.current_policy_revision=candidate.provider_policy_revision
                       AND provider.current_provider_digest=candidate.provider_digest
                      JOIN compute_provider_versions provider_version
                        ON provider_version.provider_id=provider.provider_id
                       AND provider_version.policy_revision=provider.current_policy_revision
                       AND provider_version.provider_digest=provider.current_provider_digest
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
                       AND binding.route_adapter_projection_id<>source.adapter_id
                       AND binding.route_adapter_projection_id=NEW.adapter_id
                       AND candidate.route_adapter_projection_id=NEW.adapter_id
                       AND candidate.release_version=NEW.adapter_release_version
                       AND candidate.implementation_digest=NEW.implementation_digest
                       AND candidate.adapter_config_revision=NEW.adapter_config_revision
                       AND candidate.adapter_config_digest=NEW.adapter_config_digest
                       AND candidate.service_actor_id=NEW.verified_by_service_actor_id
                       AND candidate.logical_adapter_binding_digest=NEW.route_binding_digest
                       AND candidate.logical_adapter_binding_digest=NEW.adapter_binding_digest
                       AND candidate.provider_status='registering'
                       AND candidate.candidate_status='candidate_current_not_activation_ready'
                       AND candidate.activation_closure_status='activation_closure_not_implemented'
                       AND length(candidate.logical_adapter_binding_digest)=64
                       AND candidate.logical_adapter_binding_digest NOT GLOB '*[^0-9a-f]*'
                       AND length(candidate.logical_projection_compatibility_digest)=64
                       AND candidate.logical_projection_compatibility_digest NOT GLOB '*[^0-9a-f]*'
                       AND json_extract(candidate.candidate_json,'$.candidate.logical_adapter_binding_digest')=candidate.logical_adapter_binding_digest
                       AND json_extract(candidate.candidate_json,'$.candidate.logical_projection_compatibility_digest')=candidate.logical_projection_compatibility_digest
                       AND release.adapter_id=candidate.logical_adapter_id
                       AND release.release_version=candidate.release_version
                       AND release.implementation_digest=candidate.implementation_digest
                       AND release.capability_set_digest=candidate.capability_set_digest
                       AND release.credential_verifier_digest=candidate.credential_verifier_digest
                       AND projected_adapter.supported_capabilities_json=release.supported_capabilities_json
                       AND json_array_length(projected_adapter.supported_capabilities_json)=6
                       AND json_extract(projected_adapter.supported_capabilities_json,'$[0].capability_id')='authenticated_ack'
                       AND json_extract(projected_adapter.supported_capabilities_json,'$[1].capability_id')='authenticated_events'
                       AND json_extract(projected_adapter.supported_capabilities_json,'$[2].capability_id')='cancel_no_start'
                       AND json_extract(projected_adapter.supported_capabilities_json,'$[3].capability_id')='idempotent_commit'
                       AND json_extract(projected_adapter.supported_capabilities_json,'$[4].capability_id')='prepare'
                       AND json_extract(projected_adapter.supported_capabilities_json,'$[5].capability_id')='reconcile'
                       AND delegation.issued_by_owner_user_id=candidate.provider_owner_account_id
                       AND delegation.provider_id=candidate.provider_id
                       AND delegation.provider_owner_account_id=candidate.provider_owner_account_id
                       AND delegation.provider_policy_revision=candidate.provider_policy_revision
                       AND delegation.provider_digest=candidate.provider_digest
                       AND delegation.provider_status='registering'
                       AND delegation.logical_adapter_id=candidate.logical_adapter_id
                       AND delegation.release_version=candidate.release_version
                       AND delegation.adapter_config_revision=candidate.adapter_config_revision
                       AND delegation.adapter_config_digest=candidate.adapter_config_digest
                       AND delegation.service_actor_kind='platform_dispatch_service'
                       AND json_extract(delegation.allowed_route_kinds_json,'$[0]')='server_adapter'
                       AND delegation.issued_at<=NEW.authenticated_at
                       AND candidate.checked_at<=NEW.authenticated_at
                       AND json_extract(provider_version.provider_json,'$.provider_id')=candidate.provider_id
                       AND json_extract(provider_version.provider_json,'$.provider_kind')='external_pool'
                       AND json_extract(provider_version.provider_json,'$.owner_account_id')=candidate.provider_owner_account_id
                       AND json_extract(provider_version.provider_json,'$.status')='registering'
                       AND json_extract(provider_version.provider_json,'$.policy_revision')=candidate.provider_policy_revision
                       AND json_extract(provider_version.provider_json,'$.adapter.adapter_id')=candidate.logical_adapter_id
                       AND json_extract(provider_version.provider_json,'$.adapter.adapter_version')=candidate.release_version
                       AND json_extract(provider_version.provider_json,'$.adapter.config_revision')=candidate.adapter_config_revision
                       AND json_extract(provider_version.provider_json,'$.adapter.config_digest')=candidate.adapter_config_digest
                       AND NOT EXISTS (
                            SELECT 1 FROM compute_external_pool_provider_activation_delegations later
                             WHERE later.provider_binding_id=candidate.provider_binding_id
                               AND later.sequence>candidate.sequence
                       )
                       AND NOT EXISTS (
                            SELECT 1 FROM compute_external_pool_provider_activation_candidates later
                             WHERE later.provider_binding_id=candidate.provider_binding_id
                               AND later.sequence>candidate.sequence
                       )
                       AND NOT EXISTS (
                            SELECT 1 FROM compute_external_pool_provider_activation_delegation_revocations revoked
                             WHERE revoked.delegation_id=candidate.delegation_id
                               AND revoked.delegation_digest=candidate.delegation_digest
                               AND revoked.candidate_id=candidate.candidate_id
                               AND revoked.candidate_digest=candidate.candidate_digest
                       )
                       AND NOT EXISTS (
                            SELECT 1 FROM compute_external_pool_adapter_installation_terminal_receipts terminal
                             WHERE terminal.installation_receipt_id=binding.installation_receipt_id
                               AND terminal.installation_receipt_digest=binding.installation_receipt_digest
                       )
                       AND NOT EXISTS (
                            SELECT 1 FROM compute_external_pool_adapter_adoption_terminal_receipts terminal
                             WHERE terminal.adoption_receipt_id=binding.adoption_receipt_id
                               AND terminal.adoption_receipt_digest=binding.adoption_receipt_digest
                       )
                ))
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute route authorization lacks exact source');
        END;
        "#,
    )?;
    Ok(())
}

#[cfg(test)]
#[path = "compute_external_pool_adapter_route_source_projection/dynamic_tests.rs"]
mod dynamic_tests;
