use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        DROP VIEW IF EXISTS compute_external_pool_adapter_credential_verification_current;
        CREATE VIEW compute_external_pool_adapter_credential_verification_current AS
        SELECT receipt.credential_verification_receipt_id,
               receipt.credential_verification_receipt_digest,
               CASE WHEN app.application_id IS NOT NULL
                          AND provider.provider_id IS NOT NULL
                          AND admission.admission_id IS NOT NULL
                          AND admission.current_status='staged'
                          AND verifier.key_record_id IS NOT NULL
                          AND verifier.current_status='active'
                          AND julianday(receipt.report_expires_at)>julianday('now')
                    THEN 'verified_current' ELSE 'historical_only' END AS current_status,
               CASE WHEN app.application_id IS NOT NULL THEN 'exact' ELSE 'not_exact' END AS onboarding_status,
               CASE WHEN provider.provider_id IS NOT NULL THEN 'exact_registering' ELSE 'not_current' END AS provider_status,
               COALESCE(admission.current_status,'not_current') AS admission_status,
               COALESCE(verifier.current_status,'not_current') AS verifier_key_status,
               CASE WHEN julianday(receipt.report_expires_at)>julianday('now') THEN 'current' ELSE 'expired' END AS report_validity_status
          FROM compute_external_pool_adapter_credential_verification_receipts receipt
          LEFT JOIN compute_external_pool_onboarding_applications app
            ON app.application_id=receipt.application_id
           AND app.application_digest=receipt.application_digest
           AND app.provider_id=receipt.provider_id
           AND app.target_provider_digest=receipt.provider_digest
          LEFT JOIN compute_providers provider
            ON provider.provider_id=receipt.provider_id
           AND provider.current_policy_revision=receipt.provider_policy_revision
           AND provider.current_provider_digest=receipt.provider_digest
           AND provider.status='registering'
          LEFT JOIN compute_external_pool_adapter_release_admission_current admission
            ON admission.admission_id=receipt.admission_id
           AND admission.admission_digest=receipt.admission_digest
          LEFT JOIN compute_external_pool_adapter_credential_verifier_key_current verifier
            ON verifier.key_record_id=receipt.credential_verifier_key_record_id
           AND verifier.key_record_digest=receipt.credential_verifier_key_record_digest
           AND verifier.verifier_record_id=receipt.credential_verifier_record_id
           AND verifier.verifier_record_digest=receipt.credential_verifier_record_digest
           AND verifier.key_id=receipt.credential_verifier_key_id;
        "#,
    )?;
    Ok(())
}
