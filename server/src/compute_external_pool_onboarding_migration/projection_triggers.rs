use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_external_pool_onboarding_request_projection
        BEFORE INSERT ON compute_external_pool_onboarding_requests
        WHEN NEW.status IS NOT 'submitted'
          OR NEW.created_at IS NOT NEW.requested_at
          OR NEW.updated_at IS NOT NEW.requested_at
          OR json_extract(NEW.request_json,'$.schema') IS NOT NEW.request_schema
          OR json_extract(NEW.request_json,'$.request_id') IS NOT NEW.request_id
          OR json_extract(NEW.request_json,'$.request_digest') IS NOT NEW.request_digest
          OR json_extract(NEW.request_json,'$.canonicalization') IS NOT NEW.canonicalization
          OR json_extract(NEW.request_json,'$.digest_algorithm') IS NOT NEW.digest_algorithm
          OR json_extract(NEW.request_json,'$.request.target_provider.policy_revision')
                IS NOT NEW.target_provider_policy_revision
          OR json_extract(NEW.request_json,'$.request.target_provider')
                IS NOT json(NEW.target_provider_jcs)
          OR json_extract(NEW.request_json,'$.request.target_provider.provider_id')
                IS NOT NEW.provider_id
          OR json_extract(NEW.request_json,'$.request.target_provider.provider_kind')
                IS NOT NEW.provider_kind
          OR json_extract(NEW.request_json,'$.request.target_provider.owner_account_id')
                IS NOT NEW.provider_owner_account_id
          OR json_extract(NEW.request_json,'$.request.target_provider.settlement_account_id')
                IS NOT NEW.settlement_account_id
          OR json_extract(NEW.request_json,'$.request.adapter_intent.expected_adapter_id')
                IS NOT NEW.adapter_id
          OR json_extract(NEW.request_json,'$.request.adapter_intent.expected_release_version')
                IS NOT NEW.adapter_release_version
          OR json_extract(NEW.request_json,'$.request.adapter_intent.expected_config_revision')
                IS NOT NEW.adapter_config_revision
          OR json_extract(NEW.request_json,'$.request.adapter_intent.expected_config_digest')
                IS NOT NEW.adapter_config_digest
          OR json_type(NEW.request_json,
                '$.request.credential_intent.non_bearer_credential_ref') IS NULL
          OR json_extract(NEW.request_json,
                '$.request.credential_intent.non_bearer_credential_ref')
                IS NOT NEW.non_bearer_credential_ref
          OR json_type(NEW.request_json,'$.request.credential_intent.credential_hint') IS NULL
          OR json_extract(NEW.request_json,'$.request.credential_intent.credential_hint')
                IS NOT NEW.credential_hint
          OR json_type(NEW.request_json,'$.request.external_evidence_ref') IS NULL
          OR json_extract(NEW.request_json,'$.request.external_evidence_ref')
                IS NOT NEW.external_evidence_ref
          OR json_type(NEW.request_json,'$.request.external_evidence_sha256') IS NULL
          OR json_extract(NEW.request_json,'$.request.external_evidence_sha256')
                IS NOT NEW.external_evidence_sha256
          OR json_extract(NEW.request_json,'$.request.requested_by_owner_user_id')
                IS NOT NEW.requested_by_owner_user_id
          OR json_extract(NEW.request_json,'$.request.idempotency_key') IS NOT NEW.idempotency_key
          OR json_extract(NEW.request_json,'$.request.confirmation') IS NOT NEW.confirmation
          OR json_extract(NEW.request_json,'$.request.owner_note') IS NOT NEW.owner_note
          OR json_extract(NEW.request_json,'$.request.submitted_at') IS NOT NEW.requested_at
          OR json_extract(NEW.target_provider_jcs,'$.schema')
                IS NOT 'compute_federation.provider.v1'
          OR json_extract(NEW.target_provider_jcs,'$.provider_id') IS NOT NEW.provider_id
          OR json_extract(NEW.target_provider_jcs,'$.provider_kind') IS NOT 'external_pool'
          OR json_extract(NEW.target_provider_jcs,'$.owner_account_id')
                IS NOT NEW.provider_owner_account_id
          OR json_extract(NEW.target_provider_jcs,'$.settlement_account_id')
                IS NOT NEW.settlement_account_id
          OR json_extract(NEW.target_provider_jcs,'$.status') IS NOT 'registering'
          OR json_extract(NEW.target_provider_jcs,'$.trust_tier') IS NOT 'self_declared'
          OR json_extract(NEW.target_provider_jcs,'$.policy_revision') IS NOT 1
          OR json_extract(NEW.target_provider_jcs,'$.created_at') IS NOT NEW.requested_at
          OR json_extract(NEW.target_provider_jcs,'$.updated_at') IS NOT NEW.requested_at
          OR json_type(NEW.target_provider_jcs,'$.endpoint') IS NOT 'null'
          OR json_extract(NEW.target_provider_jcs,'$.adapter.adapter_id') IS NOT NEW.adapter_id
          OR json_extract(NEW.target_provider_jcs,'$.adapter.adapter_version')
                IS NOT NEW.adapter_release_version
          OR json_extract(NEW.target_provider_jcs,'$.adapter.config_revision')
                IS NOT NEW.adapter_config_revision
          OR json_extract(NEW.target_provider_jcs,'$.adapter.config_digest')
                IS NOT NEW.adapter_config_digest
          OR json_type(NEW.target_provider_jcs,'$.evidence_profile.observed_hardware_digest')
                IS NOT 'null'
          OR json_type(NEW.target_provider_jcs,'$.evidence_profile.verified_hardware_digest')
                IS NOT 'null'
          OR json_type(NEW.target_provider_jcs,'$.evidence_profile.last_observed_at')
                IS NOT 'null'
          OR json_type(NEW.target_provider_jcs,'$.evidence_profile.last_verified_at')
                IS NOT 'null'
        BEGIN
            SELECT RAISE(ABORT, 'external pool onboarding request projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_external_pool_onboarding_review_projection
        BEFORE INSERT ON compute_external_pool_onboarding_reviews
        WHEN json_extract(NEW.review_json,'$.schema') IS NOT NEW.review_schema
          OR json_extract(NEW.review_json,'$.review_id') IS NOT NEW.review_id
          OR json_extract(NEW.review_json,'$.review_digest') IS NOT NEW.review_digest
          OR json_extract(NEW.review_json,'$.canonicalization') IS NOT NEW.canonicalization
          OR json_extract(NEW.review_json,'$.digest_algorithm') IS NOT NEW.digest_algorithm
          OR json_extract(NEW.review_json,'$.review.request_id') IS NOT NEW.request_id
          OR json_extract(NEW.review_json,'$.review.request_digest') IS NOT NEW.request_digest
          OR json_extract(NEW.review_json,'$.review.provider_id') IS NOT NEW.provider_id
          OR json_extract(NEW.review_json,'$.review.provider_owner_account_id')
                IS NOT NEW.provider_owner_account_id
          OR json_extract(NEW.review_json,'$.review.decision') IS NOT NEW.decision
          OR json_type(NEW.review_json,'$.review.review_reason') IS NULL
          OR json_extract(NEW.review_json,'$.review.review_reason') IS NOT NEW.review_reason
          OR json_extract(NEW.review_json,'$.review.reviewed_by_user_id')
                IS NOT NEW.reviewed_by_user_id
          OR json_extract(NEW.review_json,'$.review.reviewed_at') IS NOT NEW.reviewed_at
          OR NOT EXISTS (
                SELECT 1 FROM compute_external_pool_onboarding_requests request
                 WHERE request.request_id=NEW.request_id
                   AND request.request_digest=NEW.request_digest
                   AND request.provider_id=NEW.provider_id
                   AND request.provider_owner_account_id=NEW.provider_owner_account_id
                   AND request.requested_by_owner_user_id=NEW.provider_owner_account_id
                   AND request.status='submitted'
                   AND request.requested_at<=NEW.reviewed_at
                   AND NEW.created_at=NEW.reviewed_at)
        BEGIN
            SELECT RAISE(ABORT, 'external pool onboarding review projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_external_pool_onboarding_application_projection
        BEFORE INSERT ON compute_external_pool_onboarding_applications
        WHEN json_extract(NEW.application_json,'$.schema') IS NOT NEW.application_schema
          OR json_extract(NEW.application_json,'$.application_id') IS NOT NEW.application_id
          OR json_extract(NEW.application_json,'$.application_digest')
                IS NOT NEW.application_digest
          OR json_extract(NEW.application_json,'$.canonicalization') IS NOT NEW.canonicalization
          OR json_extract(NEW.application_json,'$.digest_algorithm') IS NOT NEW.digest_algorithm
          OR json_extract(NEW.application_json,'$.application.request_id') IS NOT NEW.request_id
          OR json_extract(NEW.application_json,'$.application.request_digest')
                IS NOT NEW.request_digest
          OR json_extract(NEW.application_json,'$.application.review_id') IS NOT NEW.review_id
          OR json_extract(NEW.application_json,'$.application.review_digest')
                IS NOT NEW.review_digest
          OR json_extract(NEW.application_json,'$.application.provider_id') IS NOT NEW.provider_id
          OR json_extract(NEW.application_json,'$.application.provider_kind')
                IS NOT NEW.provider_kind
          OR json_extract(NEW.application_json,'$.application.provider_owner_account_id')
                IS NOT NEW.provider_owner_account_id
          OR json_extract(NEW.application_json,'$.application.settlement_account_id')
                IS NOT NEW.settlement_account_id
          OR json_extract(NEW.application_json,
                '$.application.target_provider_policy_revision')
                IS NOT NEW.target_provider_policy_revision
          OR json_extract(NEW.application_json,'$.application.target_provider_digest')
                IS NOT NEW.target_provider_digest
          OR json_extract(NEW.application_json,'$.application.adapter_id') IS NOT NEW.adapter_id
          OR json_extract(NEW.application_json,'$.application.adapter_release_version')
                IS NOT NEW.adapter_release_version
          OR json_extract(NEW.application_json,'$.application.adapter_config_revision')
                IS NOT NEW.adapter_config_revision
          OR json_extract(NEW.application_json,'$.application.adapter_config_digest')
                IS NOT NEW.adapter_config_digest
          OR json_type(NEW.application_json,
                '$.application.non_bearer_credential_ref') IS NULL
          OR json_extract(NEW.application_json,'$.application.non_bearer_credential_ref')
                IS NOT NEW.non_bearer_credential_ref
          OR json_type(NEW.application_json,'$.application.credential_hint') IS NULL
          OR json_extract(NEW.application_json,'$.application.credential_hint')
                IS NOT NEW.credential_hint
          OR json_type(NEW.application_json,'$.application.external_evidence_ref') IS NULL
          OR json_extract(NEW.application_json,'$.application.external_evidence_ref')
                IS NOT NEW.external_evidence_ref
          OR json_type(NEW.application_json,'$.application.external_evidence_sha256') IS NULL
          OR json_extract(NEW.application_json,'$.application.external_evidence_sha256')
                IS NOT NEW.external_evidence_sha256
          OR json_extract(NEW.application_json,'$.application.approved_by_user_id')
                IS NOT NEW.approved_by_user_id
          OR json_extract(NEW.application_json,'$.application.reviewed_by_user_id')
                IS NOT NEW.reviewed_by_user_id
          OR json_extract(NEW.application_json,'$.application.apply_confirmation')
                IS NOT NEW.apply_confirmation
          OR json_extract(NEW.application_json,'$.application.applied_by_user_id')
                IS NOT NEW.applied_by_user_id
          OR json_extract(NEW.application_json,'$.application.applied_at') IS NOT NEW.applied_at
          OR NOT EXISTS (
                SELECT 1
                  FROM compute_external_pool_onboarding_requests request
                  JOIN compute_external_pool_onboarding_reviews review
                    ON review.review_id=NEW.review_id AND review.request_id=request.request_id
                  JOIN compute_providers provider ON provider.provider_id=NEW.provider_id
                  JOIN compute_provider_versions version
                    ON version.provider_id=provider.provider_id
                   AND version.policy_revision=NEW.target_provider_policy_revision
                 WHERE request.request_id=NEW.request_id
                   AND request.request_digest=NEW.request_digest
                   AND request.status='approved'
                   AND request.reviewed_by_user_id=NEW.reviewed_by_user_id
                   AND request.reviewed_at=review.reviewed_at
                   AND review.review_digest=NEW.review_digest
                   AND review.decision='approved'
                   AND review.reviewed_by_user_id=NEW.reviewed_by_user_id
                   AND review.reviewed_at<=NEW.applied_at
                   AND NEW.created_at=NEW.applied_at
                   AND request.provider_id=NEW.provider_id
                   AND request.provider_kind=NEW.provider_kind
                   AND request.provider_owner_account_id=NEW.provider_owner_account_id
                   AND request.settlement_account_id=NEW.settlement_account_id
                   AND request.target_provider_policy_revision=NEW.target_provider_policy_revision
                   AND request.target_provider_digest=NEW.target_provider_digest
                   AND request.target_provider_jcs=NEW.target_provider_jcs
                   AND request.target_provider_registry_json=NEW.target_provider_registry_json
                   AND request.adapter_id=NEW.adapter_id
                   AND request.adapter_release_version=NEW.adapter_release_version
                   AND request.adapter_config_revision=NEW.adapter_config_revision
                   AND request.adapter_config_digest=NEW.adapter_config_digest
                   AND request.non_bearer_credential_ref IS NEW.non_bearer_credential_ref
                   AND request.credential_hint IS NEW.credential_hint
                   AND request.external_evidence_ref IS NEW.external_evidence_ref
                   AND request.external_evidence_sha256 IS NEW.external_evidence_sha256
                   AND provider.provider_kind='external_pool'
                   AND provider.owner_account_id=NEW.provider_owner_account_id
                   AND provider.settlement_account_id=NEW.settlement_account_id
                   AND provider.status='registering' AND provider.trust_tier='self_declared'
                   AND provider.current_policy_revision=1
                   AND provider.current_provider_digest=NEW.target_provider_digest
                   AND version.provider_digest=NEW.target_provider_digest
                   AND version.provider_json=NEW.target_provider_registry_json)
        BEGIN
            SELECT RAISE(ABORT, 'external pool onboarding application projection mismatch');
        END;
        "#,
    )?;
    Ok(())
}
